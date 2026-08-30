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
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub byte_length: Option<u64>,
    /// Exact SHA-256 only when captured.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
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
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub source: Option<CanonicalArtifactReference>,
    /// Ordered derivative transformation identities.
    pub transformation_ids: Vec<String>,
    /// Bounded visible warnings.
    pub warnings: Vec<String>,
    /// Stable terminal error code.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
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
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
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

/// Logical owner family for one retained source artifact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalSourceOwnerClass {
    /// The owning local session releases the source when it ends.
    Session,
    /// The owning task releases the source when it reaches a terminal state.
    Task,
    /// The single originating request releases the source when the turn ends.
    Request,
}

/// Closed retention decision for one logical source artifact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalSourceRetentionClass {
    /// Default behavior; source bytes never reach durable storage.
    MemoryOnly,
    /// Explicit policy approved encrypted persistence for resume or retention.
    PolicyPersisted,
    /// The owner released the logical reference.
    Released,
    /// Canonical metadata records completed payload deletion.
    Deleted,
}

/// Payload encryption state for one logical source artifact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalSourceEncryptionState {
    /// No durable payload exists for this source.
    NotPersisted,
    /// The durable payload is encrypted at rest by the runtime artifact backend.
    EncryptedAtRest,
}

/// Current lifecycle state for one logical source-retention record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalSourceRetentionLifecycle {
    /// The logical reference is current.
    Active,
    /// The reference is retained for inspection but returns no payload bytes.
    Quarantined,
    /// The owner released the reference.
    Released,
    /// Canonical metadata records completed payload deletion.
    Deleted,
}

/// Existing physical payload family reused without widening `RuntimeArtifactKind`.
///
/// Every variant mirrors one existing [`crate::RuntimeArtifactKind`] variant. A logical
/// source artifact never introduces a new physical family and never creates a second
/// physical store.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalPhysicalArtifactKind {
    /// Mirrors `RuntimeArtifactKind::Patch`.
    Patch,
    /// Mirrors `RuntimeArtifactKind::StandardOutput`.
    StandardOutput,
    /// Mirrors `RuntimeArtifactKind::StandardError`.
    StandardError,
    /// Mirrors `RuntimeArtifactKind::TestLog`.
    TestLog,
    /// Mirrors `RuntimeArtifactKind::GeneratedFile`.
    GeneratedFile,
    /// Mirrors `RuntimeArtifactKind::Report`.
    Report,
    /// Mirrors `RuntimeArtifactKind::ModelOutput`.
    ModelOutput,
}

/// Binding from one logical source artifact to the single existing physical backend.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalPhysicalArtifactBinding {
    /// Existing physical payload family; never a new source-specific kind.
    pub artifact_kind: CanonicalPhysicalArtifactKind,
    /// Physical runtime artifact identity owned by the existing backend.
    pub artifact_id: String,
    /// Lowercase SHA-256 content address of the encrypted payload.
    pub payload_sha256: String,
    /// Exact durable payload size.
    pub byte_length: u64,
}

/// Logical ownership and retention for one source artifact.
///
/// The record carries no absolute path and no original URI. Protected metadata is
/// represented only by [`Self::protected_metadata_sha256`] and never becomes
/// model-visible.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalSourceRetention {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable retention-record identity.
    pub retention_id: String,
    /// Logical source artifact governed by this record.
    pub source_artifact_id: String,
    /// Originating request.
    pub request_id: String,
    /// Governing authority identity.
    pub authority_id: String,
    /// Logical owner family.
    pub owner_class: CanonicalSourceOwnerClass,
    /// Logical owner identity; knowledge of this identity grants no access.
    pub owner_id: String,
    /// Closed retention decision.
    pub retention_class: CanonicalSourceRetentionClass,
    /// Deterministic policy that approved persistence; null when memory-only.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub retention_policy_id: Option<String>,
    /// Trusted expiry for a persisted payload; null when memory-only.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub retention_expires_at: Option<String>,
    /// Binding to the existing backend; null unless the payload is persisted.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub physical_binding: Option<CanonicalPhysicalArtifactBinding>,
    /// Durable payload encryption state.
    pub encryption_state: CanonicalSourceEncryptionState,
    /// Digest of protected path and URI metadata; the values themselves never appear.
    pub protected_metadata_sha256: String,
    /// Current lifecycle state.
    pub lifecycle_state: CanonicalSourceRetentionLifecycle,
    /// Stable content-free code required for every non-active lifecycle state.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub reason_code: Option<String>,
    /// Trusted record time.
    pub recorded_at: String,
    /// Digest of the canonical record with this field zeroed.
    pub source_retention_sha256: String,
}

/// Coordinate space a source locator is authoritative for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalSourceLocatorKind {
    /// Authoritative byte offsets in the exact source bytes.
    Byte,
    /// Authoritative line offsets in a text source.
    Line,
    /// Authoritative page number in a paginated document.
    Page,
    /// Authoritative worksheet within a spreadsheet.
    Sheet,
    /// Authoritative cell within a named worksheet.
    Cell,
    /// Authoritative pixel region within an image.
    ImageRegion,
    /// Authoritative extracted structural section.
    Section,
}

/// Whether a locator names a real position and why it may not.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalSourceAvailabilityState {
    /// The exact position is known and the content behind it was read completely.
    Complete,
    /// The position is known and only part of the content behind it was read.
    Partial,
    /// The position is known and the content behind it was cut at a declared bound.
    Truncated,
    /// The content could not be decrypted, so no position may be claimed.
    Encrypted,
    /// The format is not supported, so no position may be claimed.
    Unsupported,
    /// The content could not be reached, so no position may be claimed.
    Unavailable,
}

/// One-indexed cell coordinate inside a named worksheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalCellReference {
    /// One-indexed row.
    pub sheet_row: u64,
    /// One-indexed column.
    pub sheet_column: u64,
}

/// Pixel region inside an image source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalImageRegion {
    /// Left pixel offset.
    pub origin_x: u64,
    /// Top pixel offset.
    pub origin_y: u64,
    /// Region width in pixels; never zero.
    pub width: u64,
    /// Region height in pixels; never zero.
    pub height: u64,
}

/// One inclusive-exclusive line range in a text source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalLineRange {
    /// First included line offset.
    pub start_line: u64,
    /// First excluded line offset.
    pub end_line_exclusive: u64,
}

/// Authoritative position of extracted content inside one source artifact.
///
/// A locator carries exactly the payload of its declared
/// [`CanonicalSourceLocatorKind`]. An unresolved availability state carries no payload
/// at all, so encrypted, unsupported, and unavailable content can never be reported
/// with an invented position.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalSourceLocator {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable locator identity.
    pub locator_id: String,
    /// Source artifact this locator addresses.
    pub source_artifact_id: String,
    /// Provenance record that admitted this locator.
    pub provenance_id: String,
    /// Extraction that produced the locator, when one exists.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub extraction_id: Option<String>,
    /// Authoritative coordinate space.
    pub locator_kind: CanonicalSourceLocatorKind,
    /// Whether the position is known, and why it may not be.
    pub availability_state: CanonicalSourceAvailabilityState,
    /// Byte payload; present only for a resolved byte locator.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub byte_range: Option<CanonicalByteRange>,
    /// Line payload; present only for a resolved line locator.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub line_range: Option<CanonicalLineRange>,
    /// Page payload; present only for a resolved page locator.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub page_number: Option<u64>,
    /// Worksheet payload; present for a resolved sheet or cell locator.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub sheet_name: Option<String>,
    /// Cell payload; present only for a resolved cell locator.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub cell_reference: Option<CanonicalCellReference>,
    /// Image payload; present only for a resolved image-region locator.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub image_region: Option<CanonicalImageRegion>,
    /// Section payload; present only for a resolved section locator.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub section_id: Option<String>,
    /// Stable content-free code required for every non-complete state.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub reason_code: Option<String>,
    /// Trusted observation time.
    pub observed_at: String,
    /// Digest of the canonical locator with this field zeroed.
    pub locator_sha256: String,
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

/// Current version of the closed effect-class taxonomy.
pub const EFFECT_CLASS_TAXONOMY_VERSION: u16 = 1;

/// Effect class of one workflow step.
///
/// This class describes repeatability and external state semantics only. It is
/// independent of both [`crate::AuthorityClass`] and [`crate::ToolRiskLevel`];
/// neither authority nor review risk can infer or widen an effect class.
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

/// Current version of the closed supervisory failure taxonomy.
pub const WORKFLOW_FAILURE_TAXONOMY_VERSION: u16 = 1;

/// Supervisory failure class used to choose one conservative default disposition.
///
/// This taxonomy is deliberately separate from [`CanonicalFailureClass`], which
/// records lower-level executor and provider causes. A low-level cause cannot mint
/// retry authority or bypass the supervisory disposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalWorkflowFailureClass {
    /// Input cannot be parsed or validated against its closed contract.
    MalformedInput,
    /// A deterministic preflight did not admit the attempt.
    Preflight,
    /// Current policy denied the requested operation.
    Policy,
    /// Required approval is absent, denied, expired, or stale.
    Approval,
    /// A declared dependency is not currently satisfied.
    Dependency,
    /// A classified transient condition may be eligible for a fresh attempt.
    Transient,
    /// Current state conflicts with the attempt's verified preconditions.
    Conflict,
    /// The attempt exceeded its declared time bound.
    Timeout,
    /// The user or trusted runtime cancelled the attempt.
    Cancellation,
    /// The supervised worker or runtime crashed.
    Crash,
    /// Whether an attempted effect occurred cannot be established.
    UncertainEffect,
    /// Deterministic verification did not establish the required postcondition.
    Verification,
    /// A declared resource ceiling was exhausted.
    Resource,
    /// A bounded internal invariant failed without a more specific class.
    Internal,
}

/// Conservative action selected from a supervisory failure class before any
/// operation-specific retry eligibility is considered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalFailureDisposition {
    /// Refuse before dispatch and return a typed diagnostic.
    RejectBeforeDispatch,
    /// Remain blocked by current policy.
    BlockedByPolicy,
    /// Wait for a new exact approval; no prior approval is reusable.
    AwaitFreshApproval,
    /// Wait for the named dependency to become current.
    AwaitDependency,
    /// Permit later policy to evaluate a fresh attempt; this is not execution authority.
    EligibleFreshAttempt,
    /// Reconcile current state before choosing any next action.
    ReconcileThenDecide,
    /// Terminate as user/runtime cancellation.
    TerminalCancelled,
    /// Terminate with one structured failure diagnosis.
    TerminalFailure,
    /// Terminate with the exact exhausted resource diagnosis.
    TerminalResourceExhausted,
}

impl CanonicalWorkflowFailureClass {
    /// Every supervisory failure class in stable taxonomy order.
    pub const ALL: [Self; 14] = [
        Self::MalformedInput,
        Self::Preflight,
        Self::Policy,
        Self::Approval,
        Self::Dependency,
        Self::Transient,
        Self::Conflict,
        Self::Timeout,
        Self::Cancellation,
        Self::Crash,
        Self::UncertainEffect,
        Self::Verification,
        Self::Resource,
        Self::Internal,
    ];

    /// Exact conservative disposition selected without model interpretation.
    #[must_use]
    pub const fn default_disposition(self) -> CanonicalFailureDisposition {
        match self {
            Self::MalformedInput | Self::Preflight => {
                CanonicalFailureDisposition::RejectBeforeDispatch
            }
            Self::Policy => CanonicalFailureDisposition::BlockedByPolicy,
            Self::Approval => CanonicalFailureDisposition::AwaitFreshApproval,
            Self::Dependency => CanonicalFailureDisposition::AwaitDependency,
            Self::Transient => CanonicalFailureDisposition::EligibleFreshAttempt,
            Self::Conflict | Self::Timeout | Self::Crash | Self::UncertainEffect => {
                CanonicalFailureDisposition::ReconcileThenDecide
            }
            Self::Cancellation => CanonicalFailureDisposition::TerminalCancelled,
            Self::Verification | Self::Internal => CanonicalFailureDisposition::TerminalFailure,
            Self::Resource => CanonicalFailureDisposition::TerminalResourceExhausted,
        }
    }
}

impl CanonicalEffectClass {
    /// Every effect class in stable taxonomy order.
    pub const ALL: [Self; 7] = [
        Self::ReadOnly,
        Self::IdempotentWrite,
        Self::Conditional,
        Self::NonIdempotent,
        Self::Destructive,
        Self::External,
        Self::Unknown,
    ];

    /// Retry classes this effect class admits.
    ///
    /// This is the closed effect/retry matrix from Decision 0042. The match is
    /// exhaustive, so adding an effect class without deciding its retry rule stops
    /// compiling rather than defaulting to a permissive one.
    #[must_use]
    pub const fn permitted_retry_classes(self) -> &'static [CanonicalRetryClass] {
        match self {
            Self::ReadOnly => &[
                CanonicalRetryClass::Never,
                CanonicalRetryClass::RecoverableRead,
            ],
            Self::IdempotentWrite | Self::Conditional => &[
                CanonicalRetryClass::Never,
                CanonicalRetryClass::ConditionalAfterReconciliation,
            ],
            Self::NonIdempotent | Self::Destructive | Self::External | Self::Unknown => &[
                CanonicalRetryClass::Never,
                CanonicalRetryClass::UserDecisionRequired,
            ],
        }
    }

    /// Whether the runtime may open a fresh attempt on its own authority.
    #[must_use]
    pub const fn permits_automatic_retry(self) -> bool {
        match self {
            Self::ReadOnly | Self::IdempotentWrite | Self::Conditional => true,
            Self::NonIdempotent | Self::Destructive | Self::External | Self::Unknown => false,
        }
    }

    /// Whether a recorded approval is mandatory before any attempt.
    ///
    /// [`Self::Unknown`] is included deliberately: an unclassified effect fails toward
    /// approval rather than past it.
    #[must_use]
    pub const fn requires_approval(self) -> bool {
        match self {
            Self::NonIdempotent | Self::Destructive | Self::External | Self::Unknown => true,
            Self::ReadOnly | Self::IdempotentWrite | Self::Conditional => false,
        }
    }
}

impl CanonicalRetryClass {
    /// Whether this class lets the runtime retry without a human decision.
    #[must_use]
    pub const fn is_automatic(self) -> bool {
        match self {
            Self::RecoverableRead | Self::ConditionalAfterReconciliation => true,
            Self::Never | Self::UserDecisionRequired => false,
        }
    }
}

/// Whether one plan step needs a recorded approval, and how often.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalApprovalRequirement {
    /// The step may run without an approval record.
    NotRequired,
    /// One approval covers the step.
    RequiredOnce,
    /// Every attempt needs its own fresh, narrow approval.
    RequiredPerAttempt,
}

/// How a step proves that a further attempt cannot duplicate an effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalIdempotencyRequirement {
    /// The step changes nothing, so an idempotency key would be a false claim.
    NotApplicable,
    /// A verified idempotency key is required before a further attempt.
    Required,
    /// The desired end state is verified instead of carrying a key.
    VerifiedDesiredState,
}

/// How one plan step is permitted to reach completion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalVerificationRequirement {
    /// Completion resolves only to current verifier evidence.
    VerifierEvidenceRequired,
    /// Policy records an explicit permitted deferral with its reason.
    PolicyDeferred,
}

/// What a terminal diagnostic for this step is allowed to disclose.
///
/// Neither variant admits model prose, prompts, credentials, or environment values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalDiagnosticDisclosure {
    /// Deterministic content-free codes only.
    ContentFreeCodes,
    /// Deterministic codes plus a reference to a separately classified artifact.
    ContentFreeCodesWithArtifactReference,
}

/// Execution policy for one existing plan step.
///
/// This is a companion record, not a replacement. It is keyed by the existing
/// [`crate::PlanStepId`] and carries execution policy only: the step's description,
/// ordinal, dependencies, and state stay in [`crate::PlanStep`], arguments stay in the
/// tool call, authority stays in the capability grant, observed outcome stays in the
/// operation receipt, and completion stays in verified completion. The record names the
/// eight policy identities that govern the step and restates none of those surfaces.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalStepExecutionPolicy {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable policy identity.
    pub policy_id: String,
    /// Plan the governed step belongs to.
    pub plan_id: crate::PlanId,
    /// The existing plan step this policy governs.
    pub plan_step_id: crate::PlanStepId,
    /// Exact plan revision that proposed the step, so a replanned step cannot silently
    /// inherit a policy written for different work.
    pub plan_revision: u32,
    /// Preflight policy identity.
    pub preflight_policy_id: String,
    /// Deterministic preflights that must return typed facts before an attempt.
    pub required_preflight_ids: Vec<String>,
    /// Side-effect policy identity.
    pub side_effect_policy_id: String,
    /// Declared effect class, reused from the existing closed family.
    pub effect_class: CanonicalEffectClass,
    /// Approval policy identity.
    pub approval_policy_id: String,
    /// Whether an approval must exist, and how often.
    pub approval_requirement: CanonicalApprovalRequirement,
    /// Idempotency policy identity.
    pub idempotency_policy_id: String,
    /// How a further attempt proves it cannot duplicate an effect.
    pub idempotency_key_requirement: CanonicalIdempotencyRequirement,
    /// Verifier policy identity.
    pub verifier_policy_id: String,
    /// How this step is permitted to reach completion.
    pub verification_requirement: CanonicalVerificationRequirement,
    /// Verifiers whose current evidence completion resolves to.
    pub required_verifier_ids: Vec<String>,
    /// Stable content-free code required for a recorded completion deferral.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub deferral_reason_code: Option<String>,
    /// Retry policy identity.
    pub retry_policy_id: String,
    /// Declared retry class, reused from the existing closed family.
    pub retry_class: CanonicalRetryClass,
    /// Budget policy identity.
    pub budget_policy_id: String,
    /// Exact step budgets, reused from the existing closed budget contract.
    pub budgets: CanonicalExecutionBudgets,
    /// Diagnostic policy identity.
    pub diagnostic_policy_id: String,
    /// What a terminal diagnostic for this step may disclose.
    pub diagnostic_disclosure: CanonicalDiagnosticDisclosure,
    /// Trusted record time.
    pub recorded_at: String,
    /// Digest of the canonical policy with this field zeroed.
    pub policy_sha256: String,
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
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub model_role: Option<String>,
    /// Optional tool identity.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
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
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub active_step_id: Option<String>,
    /// Completed steps.
    pub completed_step_ids: Vec<String>,
    /// Attempt identities.
    pub attempt_ids: Vec<String>,
    /// Digest of consumed budgets.
    pub consumed_budget_sha256: String,
    /// Terminal result identity only after terminal transition.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
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
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub exit_code: Option<i32>,
    /// Process signal description when applicable.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub signal: Option<String>,
    /// Exact stdout artifact when present.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub stdout: Option<CanonicalArtifactReference>,
    /// Exact stderr artifact when present.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
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
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
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
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub diagnostic_code: Option<String>,
    /// Safe visible next action on non-success.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub safe_next_action: Option<String>,
    /// Must equal `agentmage-runtime-verifier`.
    pub established_by: String,
    /// Digest of this result with this field zeroed.
    pub result_sha256: String,
}

/// The single authority permitted to establish execution truth.
///
/// The enum has exactly one variant, so no record in this family can attribute an
/// execution fact to a model, a client, or any other source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CanonicalExecutionAuthority {
    /// The AgentMage runtime.
    #[serde(rename = "agentmage-runtime")]
    AgentmageRuntime,
}

/// One bounded workflow execution.
///
/// The envelope references the existing [`crate::Plan`] by identity and revision and
/// never restates the plan's steps.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalWorkflowExecution {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable execution identity.
    pub execution_id: String,
    /// Workflow being executed.
    pub workflow_id: String,
    /// Workflow version.
    pub workflow_version: u64,
    /// Digest of the immutable definition this execution runs.
    pub definition_sha256: String,
    /// Existing plan this execution serves.
    pub plan_id: crate::PlanId,
    /// Exact plan revision.
    pub plan_revision: u32,
    /// Owning task.
    pub task_id: crate::TaskId,
    /// Owning session.
    pub session_id: crate::SessionId,
    /// Digest of the policy set in force.
    pub policy_sha256: String,
    /// Current lifecycle state, reused from the existing closed family.
    pub lifecycle: CanonicalWorkflowLifecycle,
    /// Monotonic sequence for ordering.
    pub sequence: u64,
    /// Trusted start time.
    pub started_at: String,
    /// Authority that established this record.
    pub established_by: CanonicalExecutionAuthority,
    /// Digest of this execution with this field zeroed.
    pub execution_sha256: String,
}

/// Lifecycle state of one bounded step execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalStepExecutionState {
    /// Dependencies are satisfied.
    Ready,
    /// Deterministic preflights are running.
    Preflighting,
    /// A required approval is outstanding.
    AwaitingApproval,
    /// An attempt is in progress.
    Running,
    /// Verifier evidence is being collected.
    Verifying,
    /// A possible effect is being reconciled.
    Reconciling,
    /// The step completed with current verifier evidence.
    Completed,
    /// Completion was deferred under an explicit recorded policy.
    Deferred,
    /// The step is visibly blocked.
    Blocked,
    /// The step failed.
    Failed,
    /// The step was cancelled.
    Cancelled,
}

/// One bounded step execution, keyed to the existing plan step.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalStepExecution {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable step-execution identity.
    pub step_execution_id: String,
    /// Owning workflow execution.
    pub execution_id: String,
    /// The existing plan step being executed.
    pub plan_step_id: crate::PlanStepId,
    /// Companion policy governing this step.
    pub policy_id: String,
    /// Current step-execution state.
    pub state: CanonicalStepExecutionState,
    /// Attempts opened for this step, in order.
    pub attempt_ids: Vec<String>,
    /// Verifications bound to this step.
    pub verification_ids: Vec<String>,
    /// Monotonic sequence for ordering.
    pub sequence: u64,
    /// Trusted start time.
    pub started_at: String,
    /// Trusted end time once the step is terminal.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub ended_at: Option<String>,
    /// Authority that established this record.
    pub established_by: CanonicalExecutionAuthority,
    /// Digest of this step execution with this field zeroed.
    pub step_execution_sha256: String,
}

/// Outcome of validating one proposed call.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalCallValidationState {
    /// The proposed call validated as authored.
    Validated,
    /// One permitted targeted repair was applied before validation succeeded.
    RepairedThenValidated,
    /// The call was rejected and never became an attempt.
    Rejected,
}

/// One validated call proposed for a step.
///
/// The envelope binds the existing [`crate::ToolCall`] by identity and carries only the
/// digest of its validated arguments, never the argument values.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalCallEnvelope {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable call identity.
    pub call_id: String,
    /// Owning step execution.
    pub step_execution_id: String,
    /// Existing tool call this envelope admits.
    pub tool_call_id: crate::ToolCallId,
    /// Tool identity.
    pub tool_id: String,
    /// Exact tool version.
    pub tool_version: String,
    /// Digest of the exact tool schema used to validate.
    pub tool_schema_sha256: String,
    /// Digest of the validated arguments; absent for a rejected call.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub validated_arguments_sha256: Option<String>,
    /// Validation outcome.
    pub validation_state: CanonicalCallValidationState,
    /// Permitted targeted repairs applied; zero unless the call was repaired.
    pub repair_count: u32,
    /// Stable content-free code required for a rejected call.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub rejection_reason_code: Option<String>,
    /// Trusted proposal time.
    pub proposed_at: String,
    /// Authority that established this record.
    pub established_by: CanonicalExecutionAuthority,
    /// Digest of this call with this field zeroed.
    pub call_sha256: String,
}

/// Terminal or in-flight state of one operation attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalAttemptState {
    /// The attempt is in progress.
    Started,
    /// The attempt succeeded and resolved to an executor receipt.
    Succeeded,
    /// The attempt failed.
    Failed,
    /// Current policy denied the attempt.
    Denied,
    /// The attempt was cancelled.
    Cancelled,
    /// The attempt timed out.
    TimedOut,
    /// The attempt may have produced an effect that is not yet confirmed.
    Uncertain,
}

/// One operation attempt.
///
/// The attempt binds the existing capability grant and operation receipt by identity and
/// digest and never restates their contents. Attempt ordinals are one-based and strictly
/// increasing, so a further attempt is always a new attempt and never a replay of an
/// earlier one.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalOperationAttempt {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable attempt identity.
    pub attempt_id: String,
    /// Call this attempt executes.
    pub call_id: String,
    /// Owning step execution.
    pub step_execution_id: String,
    /// One-based attempt ordinal.
    pub attempt_ordinal: u32,
    /// Attempt this one follows; absent only for the first attempt.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub supersedes_attempt_id: Option<String>,
    /// Existing capability grant consumed by this attempt.
    pub grant_id: crate::GrantId,
    /// Digest of the exact consumed grant.
    pub grant_sha256: String,
    /// Trusted executor identity.
    pub executor_identity: String,
    /// Sandbox identity when the attempt ran confined.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub sandbox_identity: Option<String>,
    /// Current attempt state.
    pub state: CanonicalAttemptState,
    /// Trusted start time.
    pub started_at: String,
    /// Trusted end time once the attempt is terminal.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub ended_at: Option<String>,
    /// Existing operation receipt this attempt resolved to.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub receipt_id: Option<crate::ReceiptId>,
    /// Digest of the exact receipt.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub receipt_sha256: Option<String>,
    /// Tool observation recorded for this attempt.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub observation_id: Option<String>,
    /// Authority that established this record.
    pub established_by: CanonicalExecutionAuthority,
    /// Digest of this attempt with this field zeroed.
    pub attempt_sha256: String,
}

/// Binds one verification to the step execution it judges.
///
/// The verdict itself stays in the existing verification result; this envelope carries
/// only the binding.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalVerificationEnvelope {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable verification-binding identity.
    pub verification_id: String,
    /// Step execution being verified.
    pub step_execution_id: String,
    /// Attempt being verified, when the verification targets one.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub attempt_id: Option<String>,
    /// Existing verification result carrying the verdict.
    pub verification_result_id: String,
    /// Verifier policy that required this verification.
    pub verifier_policy_id: String,
    /// Whether completion depends on this verification.
    pub required: bool,
    /// Whether the evidence is current for the observed state.
    pub current: bool,
    /// Trusted observation time.
    pub observed_at: String,
    /// Authority that established this record.
    pub established_by: CanonicalExecutionAuthority,
    /// Digest of this binding with this field zeroed.
    pub verification_sha256: String,
}

/// Deterministic failure family observed for one attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalFailureClass {
    /// Transport-level failure.
    Transport,
    /// Rate limiting.
    Rate,
    /// Operation exceeded its time bound.
    Timeout,
    /// Executor or child process crashed.
    Crash,
    /// A required service was unavailable.
    UnavailableService,
    /// A required command was absent.
    MissingCommand,
    /// Arguments were invalid.
    InvalidArguments,
    /// Authentication failed.
    Authentication,
    /// Permission was insufficient.
    Permission,
    /// Current policy denied the operation.
    PolicyDenial,
    /// A deterministic verifier failed.
    DeterministicVerificationFailure,
    /// Model output could not be parsed into a valid call.
    MalformedModelOutput,
    /// Context exceeded its budget.
    ContextOverflow,
    /// The user rejected the operation.
    UserRejection,
}

impl CanonicalFailureClass {
    /// Whether this failure family is classified transient.
    ///
    /// Only a transient failure may be reopened by the runtime on its own authority.
    /// The match is exhaustive, so a new failure family must decide this explicitly.
    #[must_use]
    pub const fn is_transient(self) -> bool {
        match self {
            Self::Transport | Self::Rate | Self::Timeout | Self::UnavailableService => true,
            Self::Crash
            | Self::MissingCommand
            | Self::InvalidArguments
            | Self::Authentication
            | Self::Permission
            | Self::PolicyDenial
            | Self::DeterministicVerificationFailure
            | Self::MalformedModelOutput
            | Self::ContextOverflow
            | Self::UserRejection => false,
        }
    }
}

/// What the runtime decided after a non-success.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalRecoveryAction {
    /// Open a fresh attempt. Never a replay of the previous one.
    RetryNewAttempt,
    /// Reconcile the observed state before deciding anything else.
    ReconcileThenDecide,
    /// Request the approval this effect requires.
    RequestApproval,
    /// Ask the user for a decision.
    RequestUserDecision,
    /// Replan the work.
    Replan,
    /// Defer under an explicit recorded policy.
    Defer,
    /// Stop with a terminal diagnostic.
    Stop,
}

/// The runtime's recorded decision after a non-success.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalRecoveryDecision {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable decision identity.
    pub decision_id: String,
    /// Step execution this decision governs.
    pub step_execution_id: String,
    /// Attempt whose outcome prompted the decision.
    pub attempt_id: String,
    /// Companion policy in force.
    pub policy_id: String,
    /// Observed failure family.
    pub failure_class: CanonicalFailureClass,
    /// Whether the attempt may have produced an effect that is not yet confirmed.
    pub uncertain_outcome: bool,
    /// Decided action.
    pub decision: CanonicalRecoveryAction,
    /// Stable content-free reason code.
    pub reason_code: String,
    /// Trusted decision time.
    pub decided_at: String,
    /// Authority that established this record.
    pub established_by: CanonicalExecutionAuthority,
    /// Digest of this decision with this field zeroed.
    pub decision_sha256: String,
}

/// One content-free terminal diagnostic.
///
/// The record names deterministic codes and artifact references. It admits no message,
/// model prose, prompt, credential, or environment value.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalTerminalDiagnostic {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable diagnostic identity.
    pub diagnostic_id: String,
    /// Workflow execution this diagnostic terminates.
    pub execution_id: String,
    /// Existing terminal result this diagnostic explains.
    pub terminal_result_id: String,
    /// Deterministic diagnostic code.
    pub diagnostic_code: String,
    /// Observed failure family; absent for a successful terminal outcome.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub failure_class: Option<CanonicalFailureClass>,
    /// Deterministic code for a safe next action; never free prose.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub safe_next_action_code: Option<String>,
    /// Separately classified evidence artifacts.
    pub evidence_artifact_ids: Vec<String>,
    /// What this diagnostic is permitted to disclose.
    pub disclosure: CanonicalDiagnosticDisclosure,
    /// Trusted report time.
    pub reported_at: String,
    /// Authority that established this record.
    pub established_by: CanonicalExecutionAuthority,
    /// Digest of this diagnostic with this field zeroed.
    pub diagnostic_sha256: String,
}

/// Admits exactly one successor attempt after a prior attempt.
///
/// This is the compatibility boundary that keeps the existing zero-hidden-retry
/// behavior valid. A permitted retry is a new attempt: it carries a new attempt, call,
/// tool-call, and grant identity, and a new approval when policy requires one per
/// attempt. Nothing is replayed. The record carries no receipt, because the successor
/// attempt has not run.
///
/// A runtime that performs no retry emits no admission at all, so the current
/// zero-retry behavior stays conformant without change.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalRetryAdmission {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable admission identity.
    pub admission_id: String,
    /// Step execution this admission belongs to.
    pub step_execution_id: String,
    /// Companion policy in force.
    pub policy_id: String,
    /// Recovery decision that permitted this retry.
    pub decision_id: String,
    /// Attempt being followed.
    pub prior_attempt_id: String,
    /// One-based ordinal of the attempt being followed.
    pub prior_attempt_ordinal: u32,
    /// Call the prior attempt executed.
    pub prior_call_id: String,
    /// Tool call the prior attempt executed.
    pub prior_tool_call_id: crate::ToolCallId,
    /// Grant the prior attempt consumed.
    pub prior_grant_id: crate::GrantId,
    /// Newly opened attempt.
    pub successor_attempt_id: String,
    /// One-based ordinal of the newly opened attempt.
    pub successor_attempt_ordinal: u32,
    /// Newly issued call.
    pub successor_call_id: String,
    /// Newly issued tool call.
    pub successor_tool_call_id: crate::ToolCallId,
    /// Newly issued grant.
    pub successor_grant_id: crate::GrantId,
    /// Approval requirement carried by the governing policy.
    pub approval_requirement: CanonicalApprovalRequirement,
    /// Approval covering the successor attempt when policy requires one per attempt.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub successor_approval_id: Option<String>,
    /// Whether deterministic reconciliation is required before the successor runs.
    pub reconciliation_required: bool,
    /// Whether that reconciliation completed.
    pub reconciled: bool,
    /// Trusted admission time.
    pub admitted_at: String,
    /// Authority that established this record.
    pub established_by: CanonicalExecutionAuthority,
    /// Digest of this admission with this field zeroed.
    pub admission_sha256: String,
}

impl CanonicalRetryAdmission {
    /// Whether this admission opens a genuinely new attempt rather than replaying one.
    ///
    /// Every identity that could carry an already-executed effect forward must differ
    /// between the prior attempt and the successor, the ordinals must form an ordered
    /// chain, required reconciliation must have completed, and a per-attempt approval
    /// must name the successor's own approval.
    #[must_use]
    pub fn opens_new_attempt(&self) -> bool {
        if self.prior_attempt_id == self.successor_attempt_id {
            return false;
        }
        if self.prior_call_id == self.successor_call_id {
            return false;
        }
        if self.prior_tool_call_id == self.successor_tool_call_id {
            return false;
        }
        if self.prior_grant_id == self.successor_grant_id {
            return false;
        }
        if self.successor_attempt_ordinal != self.prior_attempt_ordinal.saturating_add(1) {
            return false;
        }
        if self.reconciliation_required && !self.reconciled {
            return false;
        }
        if matches!(
            self.approval_requirement,
            CanonicalApprovalRequirement::RequiredPerAttempt
        ) {
            return self.successor_approval_id.is_some();
        }
        true
    }
}

#[cfg(test)]
mod source_retention_tests {
    use super::{
        CanonicalPhysicalArtifactBinding, CanonicalPhysicalArtifactKind,
        CanonicalSourceEncryptionState, CanonicalSourceOwnerClass, CanonicalSourceRetention,
        CanonicalSourceRetentionClass, CanonicalSourceRetentionLifecycle,
    };
    use crate::RuntimeArtifactKind;

    fn memory_only() -> CanonicalSourceRetention {
        CanonicalSourceRetention {
            schema_version: 1,
            retention_id: "retention-1".to_owned(),
            source_artifact_id: "source-1".to_owned(),
            request_id: "request-1".to_owned(),
            authority_id: "authority-1".to_owned(),
            owner_class: CanonicalSourceOwnerClass::Session,
            owner_id: "session-1".to_owned(),
            retention_class: CanonicalSourceRetentionClass::MemoryOnly,
            retention_policy_id: None,
            retention_expires_at: None,
            physical_binding: None,
            encryption_state: CanonicalSourceEncryptionState::NotPersisted,
            protected_metadata_sha256: "c".repeat(64),
            lifecycle_state: CanonicalSourceRetentionLifecycle::Active,
            reason_code: None,
            recorded_at: "2026-08-25T12:00:00Z".to_owned(),
            source_retention_sha256: "a".repeat(64),
        }
    }

    /// Every physical family a retained source may name must already exist in the
    /// kernel-owned `RuntimeArtifactKind`. Adding a source-specific variant to either
    /// enum without the other fails this mapping.
    #[test]
    fn physical_kinds_mirror_the_existing_runtime_artifact_kind() {
        let pairs = [
            (
                CanonicalPhysicalArtifactKind::Patch,
                RuntimeArtifactKind::Patch,
            ),
            (
                CanonicalPhysicalArtifactKind::StandardOutput,
                RuntimeArtifactKind::StandardOutput,
            ),
            (
                CanonicalPhysicalArtifactKind::StandardError,
                RuntimeArtifactKind::StandardError,
            ),
            (
                CanonicalPhysicalArtifactKind::TestLog,
                RuntimeArtifactKind::TestLog,
            ),
            (
                CanonicalPhysicalArtifactKind::GeneratedFile,
                RuntimeArtifactKind::GeneratedFile,
            ),
            (
                CanonicalPhysicalArtifactKind::Report,
                RuntimeArtifactKind::Report,
            ),
            (
                CanonicalPhysicalArtifactKind::ModelOutput,
                RuntimeArtifactKind::ModelOutput,
            ),
        ];
        assert_eq!(pairs.len(), 7);
        for (logical, physical) in pairs {
            assert_eq!(
                serde_json::to_string(&logical).expect("logical kind serializes"),
                serde_json::to_string(&physical).expect("physical kind serializes"),
                "logical source kinds must not diverge from RuntimeArtifactKind",
            );
        }
        // Exhaustive match: a new RuntimeArtifactKind variant stops compiling here.
        for physical in pairs.map(|(_, physical)| physical) {
            let _mapped = match physical {
                RuntimeArtifactKind::Patch => CanonicalPhysicalArtifactKind::Patch,
                RuntimeArtifactKind::StandardOutput => {
                    CanonicalPhysicalArtifactKind::StandardOutput
                }
                RuntimeArtifactKind::StandardError => CanonicalPhysicalArtifactKind::StandardError,
                RuntimeArtifactKind::TestLog => CanonicalPhysicalArtifactKind::TestLog,
                RuntimeArtifactKind::GeneratedFile => CanonicalPhysicalArtifactKind::GeneratedFile,
                RuntimeArtifactKind::Report => CanonicalPhysicalArtifactKind::Report,
                RuntimeArtifactKind::ModelOutput => CanonicalPhysicalArtifactKind::ModelOutput,
            };
        }
    }

    #[test]
    fn memory_only_retention_round_trips_without_durable_binding() {
        let record = memory_only();
        let encoded = serde_json::to_string(&record).expect("record serializes");
        let decoded: CanonicalSourceRetention =
            serde_json::from_str(&encoded).expect("record deserializes");
        assert_eq!(decoded, record);
        assert!(decoded.physical_binding.is_none());
    }

    #[test]
    fn persisted_retention_round_trips_with_an_existing_physical_family() {
        let mut record = memory_only();
        record.retention_class = CanonicalSourceRetentionClass::PolicyPersisted;
        record.retention_policy_id = Some("policy-1".to_owned());
        record.retention_expires_at = Some("2026-09-25T12:00:00Z".to_owned());
        record.encryption_state = CanonicalSourceEncryptionState::EncryptedAtRest;
        record.physical_binding = Some(CanonicalPhysicalArtifactBinding {
            artifact_kind: CanonicalPhysicalArtifactKind::GeneratedFile,
            artifact_id: "artifact-1".to_owned(),
            payload_sha256: "b".repeat(64),
            byte_length: 4096,
        });
        let encoded = serde_json::to_string(&record).expect("record serializes");
        assert!(encoded.contains("\"generated_file\""));
        let decoded: CanonicalSourceRetention =
            serde_json::from_str(&encoded).expect("record deserializes");
        assert_eq!(decoded, record);
    }

    #[test]
    fn unknown_and_missing_fields_are_rejected() {
        let encoded = serde_json::to_string(&memory_only()).expect("record serializes");
        let widened = encoded.replace(
            "\"retention_id\"",
            "\"source_absolute_path\":\"/home/user/secret.txt\",\"retention_id\"",
        );
        assert!(
            serde_json::from_str::<CanonicalSourceRetention>(&widened).is_err(),
            "an absolute path must never deserialize into the retention record",
        );
        let dropped = encoded.replace("\"reason_code\":null,", "");
        assert!(
            serde_json::from_str::<CanonicalSourceRetention>(&dropped).is_err(),
            "a required optional field must be present and explicit",
        );
    }

    #[test]
    fn source_specific_physical_kinds_do_not_deserialize() {
        for widened in ["source_payload", "source_artifact", "attachment"] {
            let candidate = format!("\"{widened}\"");
            assert!(
                serde_json::from_str::<CanonicalPhysicalArtifactKind>(&candidate).is_err(),
                "{widened} must not be an admitted physical family",
            );
        }
    }
}

#[cfg(test)]
mod source_locator_tests {
    use super::{
        CanonicalByteRange, CanonicalCellReference, CanonicalImageRegion, CanonicalLineRange,
        CanonicalSourceAvailabilityState, CanonicalSourceLocator, CanonicalSourceLocatorKind,
    };

    fn unresolved(
        kind: CanonicalSourceLocatorKind,
        availability_state: CanonicalSourceAvailabilityState,
    ) -> CanonicalSourceLocator {
        CanonicalSourceLocator {
            schema_version: 1,
            locator_id: "locator-1".to_owned(),
            source_artifact_id: "source-1".to_owned(),
            provenance_id: "provenance-1".to_owned(),
            extraction_id: Some("extraction-1".to_owned()),
            locator_kind: kind,
            availability_state,
            byte_range: None,
            line_range: None,
            page_number: None,
            sheet_name: None,
            cell_reference: None,
            image_region: None,
            section_id: None,
            reason_code: Some("payload_encrypted".to_owned()),
            observed_at: "2026-08-25T12:00:00Z".to_owned(),
            locator_sha256: "a".repeat(64),
        }
    }

    #[test]
    fn every_coordinate_space_round_trips_with_its_own_payload() {
        let mut byte_locator = unresolved(
            CanonicalSourceLocatorKind::Byte,
            CanonicalSourceAvailabilityState::Complete,
        );
        byte_locator.reason_code = None;
        byte_locator.byte_range = Some(CanonicalByteRange {
            start_byte: 0,
            end_byte_exclusive: 128,
        });
        let mut cell_locator = unresolved(
            CanonicalSourceLocatorKind::Cell,
            CanonicalSourceAvailabilityState::Complete,
        );
        cell_locator.reason_code = None;
        cell_locator.sheet_name = Some("Q3 Summary".to_owned());
        cell_locator.cell_reference = Some(CanonicalCellReference {
            sheet_row: 4,
            sheet_column: 7,
        });
        let mut image_locator = unresolved(
            CanonicalSourceLocatorKind::ImageRegion,
            CanonicalSourceAvailabilityState::Truncated,
        );
        image_locator.image_region = Some(CanonicalImageRegion {
            origin_x: 10,
            origin_y: 20,
            width: 640,
            height: 480,
        });
        let mut line_locator = unresolved(
            CanonicalSourceLocatorKind::Line,
            CanonicalSourceAvailabilityState::Partial,
        );
        line_locator.line_range = Some(CanonicalLineRange {
            start_line: 0,
            end_line_exclusive: 12,
        });
        for record in [byte_locator, cell_locator, image_locator, line_locator] {
            let encoded = serde_json::to_string(&record).expect("locator serializes");
            let decoded: CanonicalSourceLocator =
                serde_json::from_str(&encoded).expect("locator deserializes");
            assert_eq!(decoded, record);
        }
    }

    #[test]
    fn unresolved_states_round_trip_without_any_position() {
        for state in [
            CanonicalSourceAvailabilityState::Encrypted,
            CanonicalSourceAvailabilityState::Unsupported,
            CanonicalSourceAvailabilityState::Unavailable,
        ] {
            let record = unresolved(CanonicalSourceLocatorKind::Page, state);
            let encoded = serde_json::to_string(&record).expect("locator serializes");
            let decoded: CanonicalSourceLocator =
                serde_json::from_str(&encoded).expect("locator deserializes");
            assert_eq!(decoded, record);
            assert!(decoded.byte_range.is_none());
            assert!(decoded.page_number.is_none());
            assert!(decoded.section_id.is_none());
        }
    }

    #[test]
    fn unknown_and_missing_fields_are_rejected() {
        let encoded = serde_json::to_string(&unresolved(
            CanonicalSourceLocatorKind::Byte,
            CanonicalSourceAvailabilityState::Unavailable,
        ))
        .expect("locator serializes");
        let widened = encoded.replace(
            "\"locator_id\"",
            "\"source_absolute_path\":\"/home/user/report.pdf\",\"locator_id\"",
        );
        assert!(
            serde_json::from_str::<CanonicalSourceLocator>(&widened).is_err(),
            "an absolute path must never deserialize into a locator",
        );
        let dropped = encoded.replace("\"section_id\":null,", "");
        assert!(
            serde_json::from_str::<CanonicalSourceLocator>(&dropped).is_err(),
            "a required optional payload field must be present and explicit",
        );
    }

    #[test]
    fn unknown_locator_kinds_and_states_do_not_deserialize() {
        for candidate in ["\"paragraph\"", "\"token\"", "\"offset\""] {
            assert!(
                serde_json::from_str::<CanonicalSourceLocatorKind>(candidate).is_err(),
                "{candidate} must not be an admitted coordinate space",
            );
        }
        for candidate in ["\"redacted\"", "\"missing\"", "\"stale\""] {
            assert!(
                serde_json::from_str::<CanonicalSourceAvailabilityState>(candidate).is_err(),
                "{candidate} must not be an admitted availability state",
            );
        }
    }
}

#[cfg(test)]
mod step_execution_policy_tests {
    use super::{
        CanonicalApprovalRequirement, CanonicalDiagnosticDisclosure, CanonicalEffectClass,
        CanonicalExecutionBudgets, CanonicalFailureDisposition, CanonicalIdempotencyRequirement,
        CanonicalRetryClass, CanonicalStepExecutionPolicy, CanonicalVerificationRequirement,
        CanonicalWorkflowFailureClass,
    };
    use crate::{
        AuthorityClass, EvidenceKind, PlanId, PlanStep, PlanStepId, PlanStepState, ToolRiskLevel,
    };

    const ALL_RETRY_CLASSES: [CanonicalRetryClass; 4] = [
        CanonicalRetryClass::Never,
        CanonicalRetryClass::RecoverableRead,
        CanonicalRetryClass::ConditionalAfterReconciliation,
        CanonicalRetryClass::UserDecisionRequired,
    ];

    fn budgets() -> CanonicalExecutionBudgets {
        CanonicalExecutionBudgets {
            turns: 4,
            tokens: 32_000,
            duration_ms: 60_000,
            tool_calls: 8,
            attempts: 1,
            no_progress_events: 2,
            output_bytes: 1_048_576,
            memory_bytes: 268_435_456,
            cost_minor_units: 0,
        }
    }

    fn policy_for(plan_step_id: PlanStepId) -> CanonicalStepExecutionPolicy {
        CanonicalStepExecutionPolicy {
            schema_version: 1,
            policy_id: "policy-1".to_owned(),
            plan_id: PlanId::from_raw("plan-0001"),
            plan_step_id,
            plan_revision: 3,
            preflight_policy_id: "preflight-policy-1".to_owned(),
            required_preflight_ids: vec![
                "workspace-trust".to_owned(),
                "repository-clean".to_owned(),
            ],
            side_effect_policy_id: "side-effect-policy-1".to_owned(),
            effect_class: CanonicalEffectClass::ReadOnly,
            approval_policy_id: "approval-policy-1".to_owned(),
            approval_requirement: CanonicalApprovalRequirement::NotRequired,
            idempotency_policy_id: "idempotency-policy-1".to_owned(),
            idempotency_key_requirement: CanonicalIdempotencyRequirement::NotApplicable,
            verifier_policy_id: "verifier-policy-1".to_owned(),
            verification_requirement: CanonicalVerificationRequirement::VerifierEvidenceRequired,
            required_verifier_ids: vec!["exit-status".to_owned()],
            deferral_reason_code: None,
            retry_policy_id: "retry-policy-1".to_owned(),
            retry_class: CanonicalRetryClass::Never,
            budget_policy_id: "budget-policy-1".to_owned(),
            budgets: budgets(),
            diagnostic_policy_id: "diagnostic-policy-1".to_owned(),
            diagnostic_disclosure: CanonicalDiagnosticDisclosure::ContentFreeCodes,
            recorded_at: "2026-08-26T12:00:00Z".to_owned(),
            policy_sha256: "a".repeat(64),
        }
    }

    fn plan_step() -> PlanStep {
        PlanStep {
            plan_step_id: PlanStepId::from_raw("plan-0001:step:0002"),
            ordinal: 2,
            description: "Regenerate the traceability report".to_owned(),
            depends_on: vec![PlanStepId::from_raw("plan-0001:step:0001")],
            expected_evidence: vec![EvidenceKind::Validation],
            state: PlanStepState::Ready,
        }
    }

    #[test]
    fn policy_is_keyed_to_the_existing_plan_step_identity() {
        let step = plan_step();
        let policy = policy_for(step.plan_step_id.clone());
        assert_eq!(policy.plan_step_id, step.plan_step_id);
        let encoded = serde_json::to_string(&policy).expect("policy serializes");
        let decoded: CanonicalStepExecutionPolicy =
            serde_json::from_str(&encoded).expect("policy deserializes");
        assert_eq!(decoded, policy);
        // The existing identity crosses the wire as its own bare value, so the companion
        // record introduces no second step-identity encoding.
        let value: serde_json::Value = serde_json::from_str(&encoded).expect("policy is an object");
        assert_eq!(
            value["plan_step_id"],
            serde_json::Value::String(step.plan_step_id.as_str().to_owned()),
        );
    }

    #[test]
    fn the_policy_restates_no_plan_step_tool_grant_receipt_or_completion_state() {
        let value =
            serde_json::to_value(policy_for(plan_step().plan_step_id)).expect("policy serializes");
        let object = value.as_object().expect("policy is an object");
        assert!(
            object.contains_key("plan_step_id"),
            "the policy is keyed to the plan step"
        );
        // Each of these belongs to a contract this record must never replace.
        for owned_elsewhere in [
            "description",
            "ordinal",
            "depends_on",
            "expected_evidence",
            "state",
            "arguments",
            "tool_arguments",
            "grant_id",
            "capability_grant",
            "receipt_id",
            "exit_status",
            "changed_resources",
            "completed",
            "verified_completion",
        ] {
            assert!(
                !object.contains_key(owned_elsewhere),
                "{owned_elsewhere} belongs to an existing contract and must stay there",
            );
        }
        // Policy identities are content-free: no path, URI, command, or credential surface.
        for key in object.keys() {
            for forbidden in [
                "path",
                "uri",
                "url",
                "command",
                "secret",
                "token",
                "credential",
            ] {
                assert!(
                    !key.contains(forbidden),
                    "policy field {key} must not expose a {forbidden} surface",
                );
            }
        }
    }

    #[test]
    fn every_effect_class_admits_only_its_own_retry_classes() {
        for effect in CanonicalEffectClass::ALL {
            let permitted = effect.permitted_retry_classes();
            assert!(
                permitted.contains(&CanonicalRetryClass::Never),
                "{effect:?} must always admit declining a further attempt",
            );
            for retry in ALL_RETRY_CLASSES {
                if permitted.contains(&retry) && retry.is_automatic() {
                    assert!(
                        effect.permits_automatic_retry(),
                        "{effect:?} admits automatic {retry:?} but forbids automatic retry",
                    );
                }
            }
            // An effect that fails toward approval is never retried by the runtime alone.
            if effect.requires_approval() {
                assert!(
                    !effect.permits_automatic_retry(),
                    "{effect:?} must not self-retry"
                );
                for retry in permitted {
                    assert!(
                        !retry.is_automatic(),
                        "{effect:?} must not admit automatic {retry:?}",
                    );
                }
            }
        }
        assert!(CanonicalEffectClass::Unknown.requires_approval());
        assert!(!CanonicalEffectClass::Unknown.permits_automatic_retry());
        assert!(CanonicalEffectClass::Destructive.requires_approval());
        assert!(CanonicalEffectClass::External.requires_approval());
    }

    #[test]
    fn effect_taxonomy_is_closed_versioned_and_independent_of_authority_and_risk() {
        let expected = [
            (CanonicalEffectClass::ReadOnly, "read_only"),
            (CanonicalEffectClass::IdempotentWrite, "idempotent_write"),
            (CanonicalEffectClass::Conditional, "conditional"),
            (CanonicalEffectClass::NonIdempotent, "non_idempotent"),
            (CanonicalEffectClass::Destructive, "destructive"),
            (CanonicalEffectClass::External, "external"),
            (CanonicalEffectClass::Unknown, "unknown"),
        ];
        assert_eq!(super::EFFECT_CLASS_TAXONOMY_VERSION, 1);
        assert_eq!(
            CanonicalEffectClass::ALL,
            expected.map(|(effect, _)| effect)
        );

        for (effect, name) in expected {
            let encoded = serde_json::to_string(&effect).expect("effect class serializes");
            assert_eq!(encoded, format!("\"{name}\""));
            assert_eq!(
                serde_json::from_str::<CanonicalEffectClass>(&encoded)
                    .expect("effect class deserializes"),
                effect,
            );
            for authority in AuthorityClass::ALL {
                for risk in [
                    ToolRiskLevel::Low,
                    ToolRiskLevel::Moderate,
                    ToolRiskLevel::High,
                    ToolRiskLevel::Critical,
                ] {
                    let independent = (effect, authority, risk);
                    let round_trip: (CanonicalEffectClass, AuthorityClass, ToolRiskLevel) =
                        serde_json::from_str(
                            &serde_json::to_string(&independent)
                                .expect("independent classifications serialize"),
                        )
                        .expect("independent classifications deserialize");
                    assert_eq!(round_trip, independent);
                    assert_eq!(
                        round_trip.0.permitted_retry_classes(),
                        effect.permitted_retry_classes()
                    );
                    assert_eq!(round_trip.0.requires_approval(), effect.requires_approval());
                }
            }
        }

        for rejected in ["custom", "*", "inherited", "model_created", ""] {
            assert!(
                serde_json::from_value::<CanonicalEffectClass>(serde_json::json!(rejected))
                    .is_err(),
                "unsupported effect class {rejected:?} must fail closed",
            );
        }
    }

    #[test]
    fn workflow_failure_taxonomy_has_one_exact_conservative_default_per_class() {
        use CanonicalFailureDisposition as Disposition;
        use CanonicalWorkflowFailureClass as Failure;

        let expected = [
            (
                Failure::MalformedInput,
                "malformed_input",
                Disposition::RejectBeforeDispatch,
            ),
            (
                Failure::Preflight,
                "preflight",
                Disposition::RejectBeforeDispatch,
            ),
            (Failure::Policy, "policy", Disposition::BlockedByPolicy),
            (
                Failure::Approval,
                "approval",
                Disposition::AwaitFreshApproval,
            ),
            (
                Failure::Dependency,
                "dependency",
                Disposition::AwaitDependency,
            ),
            (
                Failure::Transient,
                "transient",
                Disposition::EligibleFreshAttempt,
            ),
            (
                Failure::Conflict,
                "conflict",
                Disposition::ReconcileThenDecide,
            ),
            (
                Failure::Timeout,
                "timeout",
                Disposition::ReconcileThenDecide,
            ),
            (
                Failure::Cancellation,
                "cancellation",
                Disposition::TerminalCancelled,
            ),
            (Failure::Crash, "crash", Disposition::ReconcileThenDecide),
            (
                Failure::UncertainEffect,
                "uncertain_effect",
                Disposition::ReconcileThenDecide,
            ),
            (
                Failure::Verification,
                "verification",
                Disposition::TerminalFailure,
            ),
            (
                Failure::Resource,
                "resource",
                Disposition::TerminalResourceExhausted,
            ),
            (Failure::Internal, "internal", Disposition::TerminalFailure),
        ];
        assert_eq!(super::WORKFLOW_FAILURE_TAXONOMY_VERSION, 1);
        assert_eq!(
            CanonicalWorkflowFailureClass::ALL,
            expected.map(|(class, _, _)| class)
        );
        for (class, name, disposition) in expected {
            assert_eq!(class.default_disposition(), disposition);
            let encoded = serde_json::to_string(&class).expect("failure class serializes");
            assert_eq!(encoded, format!("\"{name}\""));
            assert_eq!(
                serde_json::from_str::<CanonicalWorkflowFailureClass>(&encoded)
                    .expect("failure class deserializes"),
                class,
            );
        }
        assert_eq!(
            CanonicalWorkflowFailureClass::Transient.default_disposition(),
            Disposition::EligibleFreshAttempt,
        );
        for class in [
            Failure::Conflict,
            Failure::Timeout,
            Failure::Crash,
            Failure::UncertainEffect,
        ] {
            assert_eq!(
                class.default_disposition(),
                Disposition::ReconcileThenDecide
            );
        }
        for rejected in ["custom", "*", "inherited", "model_created", "unknown", ""] {
            assert!(
                serde_json::from_value::<CanonicalWorkflowFailureClass>(serde_json::json!(
                    rejected
                ))
                .is_err(),
                "unsupported failure class {rejected:?} must fail closed",
            );
        }
    }

    #[test]
    fn unknown_and_missing_fields_are_rejected() {
        let encoded = serde_json::to_string(&policy_for(plan_step().plan_step_id))
            .expect("policy serializes");
        let widened = encoded.replace(
            "\"policy_id\"",
            "\"command_line\":\"rm -rf /\",\"policy_id\"",
        );
        assert!(
            serde_json::from_str::<CanonicalStepExecutionPolicy>(&widened).is_err(),
            "a command line must never deserialize into a step policy",
        );
        let dropped = encoded.replace("\"deferral_reason_code\":null,", "");
        assert!(
            serde_json::from_str::<CanonicalStepExecutionPolicy>(&dropped).is_err(),
            "a required optional field must be present and explicit",
        );
    }

    #[test]
    fn unknown_requirement_values_do_not_deserialize() {
        for candidate in ["\"optional\"", "\"best_effort\"", "\"required\""] {
            assert!(
                serde_json::from_str::<CanonicalApprovalRequirement>(candidate).is_err(),
                "{candidate} must not be an admitted approval requirement",
            );
        }
        for candidate in ["\"assumed\"", "\"skipped\"", "\"none\""] {
            assert!(
                serde_json::from_str::<CanonicalIdempotencyRequirement>(candidate).is_err(),
                "{candidate} must not be an admitted idempotency requirement",
            );
        }
        for candidate in ["\"model_asserted\"", "\"self_reported\"", "\"assumed\""] {
            assert!(
                serde_json::from_str::<CanonicalVerificationRequirement>(candidate).is_err(),
                "{candidate} must not be an admitted verification requirement",
            );
        }
        for candidate in ["\"full_transcript\"", "\"model_prose\"", "\"raw_output\""] {
            assert!(
                serde_json::from_str::<CanonicalDiagnosticDisclosure>(candidate).is_err(),
                "{candidate} must not be an admitted diagnostic disclosure",
            );
        }
    }
}

#[cfg(test)]
mod execution_envelope_tests {
    use super::{
        CanonicalAttemptState, CanonicalCallEnvelope, CanonicalCallValidationState,
        CanonicalDiagnosticDisclosure, CanonicalExecutionAuthority, CanonicalFailureClass,
        CanonicalOperationAttempt, CanonicalRecoveryAction, CanonicalRecoveryDecision,
        CanonicalStepExecution, CanonicalStepExecutionState, CanonicalTerminalDiagnostic,
        CanonicalVerificationEnvelope, CanonicalWorkflowExecution, CanonicalWorkflowLifecycle,
    };
    use crate::{GrantId, PlanId, PlanStepId, ReceiptId, SessionId, TaskId, ToolCallId};

    const ALL_FAILURE_CLASSES: [CanonicalFailureClass; 14] = [
        CanonicalFailureClass::Transport,
        CanonicalFailureClass::Rate,
        CanonicalFailureClass::Timeout,
        CanonicalFailureClass::Crash,
        CanonicalFailureClass::UnavailableService,
        CanonicalFailureClass::MissingCommand,
        CanonicalFailureClass::InvalidArguments,
        CanonicalFailureClass::Authentication,
        CanonicalFailureClass::Permission,
        CanonicalFailureClass::PolicyDenial,
        CanonicalFailureClass::DeterministicVerificationFailure,
        CanonicalFailureClass::MalformedModelOutput,
        CanonicalFailureClass::ContextOverflow,
        CanonicalFailureClass::UserRejection,
    ];

    fn digest() -> String {
        "a".repeat(64)
    }

    fn workflow_execution() -> CanonicalWorkflowExecution {
        CanonicalWorkflowExecution {
            schema_version: 1,
            execution_id: "execution-1".to_owned(),
            workflow_id: "workflow-1".to_owned(),
            workflow_version: 1,
            definition_sha256: digest(),
            plan_id: PlanId::from_raw("plan-0001"),
            plan_revision: 3,
            task_id: TaskId::from_raw("task-0001"),
            session_id: SessionId::from_raw("session-0001"),
            policy_sha256: digest(),
            lifecycle: CanonicalWorkflowLifecycle::Running,
            sequence: 7,
            started_at: "2026-08-26T12:00:00Z".to_owned(),
            established_by: CanonicalExecutionAuthority::AgentmageRuntime,
            execution_sha256: digest(),
        }
    }

    fn step_execution() -> CanonicalStepExecution {
        CanonicalStepExecution {
            schema_version: 1,
            step_execution_id: "step-execution-1".to_owned(),
            execution_id: "execution-1".to_owned(),
            plan_step_id: PlanStepId::from_raw("plan-0001:step:0002"),
            policy_id: "policy-1".to_owned(),
            state: CanonicalStepExecutionState::Running,
            attempt_ids: vec!["attempt-1".to_owned()],
            verification_ids: vec!["verification-1".to_owned()],
            sequence: 2,
            started_at: "2026-08-26T12:00:00Z".to_owned(),
            ended_at: None,
            established_by: CanonicalExecutionAuthority::AgentmageRuntime,
            step_execution_sha256: digest(),
        }
    }

    fn call_envelope() -> CanonicalCallEnvelope {
        CanonicalCallEnvelope {
            schema_version: 1,
            call_id: "call-1".to_owned(),
            step_execution_id: "step-execution-1".to_owned(),
            tool_call_id: ToolCallId::from_raw("tool-call-1"),
            tool_id: "cargo-test".to_owned(),
            tool_version: "1.0.0".to_owned(),
            tool_schema_sha256: digest(),
            validated_arguments_sha256: Some(digest()),
            validation_state: CanonicalCallValidationState::Validated,
            repair_count: 0,
            rejection_reason_code: None,
            proposed_at: "2026-08-26T12:00:00Z".to_owned(),
            established_by: CanonicalExecutionAuthority::AgentmageRuntime,
            call_sha256: digest(),
        }
    }

    fn operation_attempt() -> CanonicalOperationAttempt {
        CanonicalOperationAttempt {
            schema_version: 1,
            attempt_id: "attempt-1".to_owned(),
            call_id: "call-1".to_owned(),
            step_execution_id: "step-execution-1".to_owned(),
            attempt_ordinal: 1,
            supersedes_attempt_id: None,
            grant_id: GrantId::from_raw("grant-1"),
            grant_sha256: digest(),
            executor_identity: "executor-1".to_owned(),
            sandbox_identity: Some("sandbox-1".to_owned()),
            state: CanonicalAttemptState::Started,
            started_at: "2026-08-26T12:00:00Z".to_owned(),
            ended_at: None,
            receipt_id: None,
            receipt_sha256: None,
            observation_id: None,
            established_by: CanonicalExecutionAuthority::AgentmageRuntime,
            attempt_sha256: digest(),
        }
    }

    fn verification_envelope() -> CanonicalVerificationEnvelope {
        CanonicalVerificationEnvelope {
            schema_version: 1,
            verification_id: "verification-1".to_owned(),
            step_execution_id: "step-execution-1".to_owned(),
            attempt_id: Some("attempt-1".to_owned()),
            verification_result_id: "verification-result-1".to_owned(),
            verifier_policy_id: "verifier-policy-1".to_owned(),
            required: true,
            current: true,
            observed_at: "2026-08-26T12:00:00Z".to_owned(),
            established_by: CanonicalExecutionAuthority::AgentmageRuntime,
            verification_sha256: digest(),
        }
    }

    fn recovery_decision() -> CanonicalRecoveryDecision {
        CanonicalRecoveryDecision {
            schema_version: 1,
            decision_id: "decision-1".to_owned(),
            step_execution_id: "step-execution-1".to_owned(),
            attempt_id: "attempt-1".to_owned(),
            policy_id: "policy-1".to_owned(),
            failure_class: CanonicalFailureClass::Timeout,
            uncertain_outcome: false,
            decision: CanonicalRecoveryAction::RetryNewAttempt,
            reason_code: "transient_timeout".to_owned(),
            decided_at: "2026-08-26T12:00:00Z".to_owned(),
            established_by: CanonicalExecutionAuthority::AgentmageRuntime,
            decision_sha256: digest(),
        }
    }

    fn terminal_diagnostic() -> CanonicalTerminalDiagnostic {
        CanonicalTerminalDiagnostic {
            schema_version: 1,
            diagnostic_id: "diagnostic-1".to_owned(),
            execution_id: "execution-1".to_owned(),
            terminal_result_id: "terminal-result-1".to_owned(),
            diagnostic_code: "verifier_evidence_absent".to_owned(),
            failure_class: Some(CanonicalFailureClass::DeterministicVerificationFailure),
            safe_next_action_code: Some("rerun_verifier".to_owned()),
            evidence_artifact_ids: vec!["artifact-1".to_owned()],
            disclosure: CanonicalDiagnosticDisclosure::ContentFreeCodesWithArtifactReference,
            reported_at: "2026-08-26T12:00:00Z".to_owned(),
            established_by: CanonicalExecutionAuthority::AgentmageRuntime,
            diagnostic_sha256: digest(),
        }
    }

    /// Every envelope in the family, encoded once for the shared boundary assertions.
    fn encoded_family() -> Vec<(&'static str, String)> {
        vec![
            (
                "workflow-execution",
                serde_json::to_string(&workflow_execution()).expect("serializes"),
            ),
            (
                "step-execution",
                serde_json::to_string(&step_execution()).expect("serializes"),
            ),
            (
                "call-envelope",
                serde_json::to_string(&call_envelope()).expect("serializes"),
            ),
            (
                "operation-attempt",
                serde_json::to_string(&operation_attempt()).expect("serializes"),
            ),
            (
                "verification-envelope",
                serde_json::to_string(&verification_envelope()).expect("serializes"),
            ),
            (
                "recovery-decision",
                serde_json::to_string(&recovery_decision()).expect("serializes"),
            ),
            (
                "terminal-diagnostic",
                serde_json::to_string(&terminal_diagnostic()).expect("serializes"),
            ),
        ]
    }

    #[test]
    fn every_envelope_round_trips_and_is_established_only_by_the_runtime() {
        assert_eq!(encoded_family().len(), 7);
        assert_eq!(
            serde_json::to_string(&CanonicalExecutionAuthority::AgentmageRuntime)
                .expect("authority serializes"),
            "\"agentmage-runtime\"",
        );
        for candidate in ["\"model\"", "\"client\"", "\"user\"", "\"extension\""] {
            assert!(
                serde_json::from_str::<CanonicalExecutionAuthority>(candidate).is_err(),
                "{candidate} must never establish execution truth",
            );
        }
        assert_eq!(
            serde_json::from_str::<CanonicalWorkflowExecution>(&encoded_family()[0].1)
                .expect("deserializes"),
            workflow_execution(),
        );
        assert_eq!(
            serde_json::from_str::<CanonicalOperationAttempt>(&encoded_family()[3].1)
                .expect("deserializes"),
            operation_attempt(),
        );
        assert_eq!(
            serde_json::from_str::<CanonicalRecoveryDecision>(&encoded_family()[5].1)
                .expect("deserializes"),
            recovery_decision(),
        );
    }

    #[test]
    fn no_envelope_restates_content_the_protected_contracts_own() {
        // Each of these belongs to Plan, ToolCall, CapabilityGrant, OperationReceipt, or
        // VerifiedCompletion. The family reaches those by identity and digest only.
        for (name, encoded) in encoded_family() {
            let value: serde_json::Value =
                serde_json::from_str(&encoded).expect("envelope is an object");
            let object = value.as_object().expect("envelope is an object");
            for owned_elsewhere in [
                "steps",
                "description",
                "ordinal",
                "depends_on",
                "expected_evidence",
                "arguments",
                "tool_arguments",
                "parameters",
                "scope",
                "capability",
                "granted_operations",
                "exit_code",
                "stdout",
                "stderr",
                "resource_usage",
                "preserved_invariants",
                "observed_evidence_sha256s",
            ] {
                assert!(
                    !object.contains_key(owned_elsewhere),
                    "{name} must not restate {owned_elsewhere}",
                );
            }
            for key in object.keys() {
                for forbidden in ["absolute_path", "secret", "credential", "prompt"] {
                    assert!(
                        !key.contains(forbidden),
                        "{name} field {key} must not expose a {forbidden} surface",
                    );
                }
            }
        }
    }

    #[test]
    fn the_protected_contracts_stay_reachable_by_their_own_identity_types() {
        // The envelopes key off the existing identity types, so a second identity
        // encoding for a protected contract cannot appear without changing these fields.
        let execution = workflow_execution();
        assert_eq!(execution.plan_id, PlanId::from_raw("plan-0001"));
        assert_eq!(
            step_execution().plan_step_id,
            PlanStepId::from_raw("plan-0001:step:0002"),
        );
        assert_eq!(
            call_envelope().tool_call_id,
            ToolCallId::from_raw("tool-call-1")
        );
        assert_eq!(operation_attempt().grant_id, GrantId::from_raw("grant-1"));
        let mut succeeded = operation_attempt();
        succeeded.state = CanonicalAttemptState::Succeeded;
        succeeded.ended_at = Some("2026-08-26T12:05:00Z".to_owned());
        succeeded.receipt_id = Some(ReceiptId::from_raw("receipt-1"));
        succeeded.receipt_sha256 = Some(digest());
        let encoded = serde_json::to_string(&succeeded).expect("serializes");
        let decoded: CanonicalOperationAttempt =
            serde_json::from_str(&encoded).expect("deserializes");
        assert_eq!(decoded.receipt_id, Some(ReceiptId::from_raw("receipt-1")));
    }

    #[test]
    fn only_a_classified_transient_failure_is_transient() {
        let mut transient = 0_usize;
        for failure in ALL_FAILURE_CLASSES {
            if failure.is_transient() {
                transient += 1;
            }
        }
        assert_eq!(transient, 4, "exactly four failure families are transient");
        for failure in [
            CanonicalFailureClass::Transport,
            CanonicalFailureClass::Rate,
            CanonicalFailureClass::Timeout,
            CanonicalFailureClass::UnavailableService,
        ] {
            assert!(failure.is_transient(), "{failure:?} must be transient");
        }
        // A destructive or user-owned outcome is never reopened by the runtime alone.
        for failure in [
            CanonicalFailureClass::UserRejection,
            CanonicalFailureClass::PolicyDenial,
            CanonicalFailureClass::Permission,
            CanonicalFailureClass::DeterministicVerificationFailure,
            CanonicalFailureClass::Crash,
        ] {
            assert!(!failure.is_transient(), "{failure:?} must not be transient");
        }
    }

    #[test]
    fn unknown_and_missing_envelope_fields_are_rejected() {
        let encoded = serde_json::to_string(&operation_attempt()).expect("serializes");
        let widened = encoded.replace(
            "\"attempt_id\"",
            "\"replayed_from\":\"attempt-0\",\"attempt_id\"",
        );
        assert!(
            serde_json::from_str::<CanonicalOperationAttempt>(&widened).is_err(),
            "a replay field must never deserialize into an attempt",
        );
        let dropped = encoded.replace("\"receipt_sha256\":null,", "");
        assert!(
            serde_json::from_str::<CanonicalOperationAttempt>(&dropped).is_err(),
            "a required optional field must be present and explicit",
        );
        let diagnostic = serde_json::to_string(&terminal_diagnostic()).expect("serializes");
        let with_prose = diagnostic.replace(
            "\"diagnostic_code\"",
            "\"message\":\"the model thinks it worked\",\"diagnostic_code\"",
        );
        assert!(
            serde_json::from_str::<CanonicalTerminalDiagnostic>(&with_prose).is_err(),
            "a terminal diagnostic must never carry prose",
        );
    }

    #[test]
    fn unknown_envelope_states_do_not_deserialize() {
        for candidate in ["\"replayed\"", "\"retried\"", "\"resumed\""] {
            assert!(
                serde_json::from_str::<CanonicalAttemptState>(candidate).is_err(),
                "{candidate} must not be an admitted attempt state",
            );
        }
        for candidate in ["\"replay\"", "\"retry_same_attempt\"", "\"ignore\""] {
            assert!(
                serde_json::from_str::<CanonicalRecoveryAction>(candidate).is_err(),
                "{candidate} must not be an admitted recovery action",
            );
        }
        for candidate in ["\"unlucky\"", "\"flaky\"", "\"unknown\""] {
            assert!(
                serde_json::from_str::<CanonicalFailureClass>(candidate).is_err(),
                "{candidate} must not be an admitted failure class",
            );
        }
        for candidate in ["\"finished\"", "\"replaying\"", "\"skipped\""] {
            assert!(
                serde_json::from_str::<CanonicalStepExecutionState>(candidate).is_err(),
                "{candidate} must not be an admitted step-execution state",
            );
        }
    }
}

#[cfg(test)]
mod retry_admission_tests {
    use super::{
        CanonicalApprovalRequirement, CanonicalExecutionAuthority, CanonicalRetryAdmission,
    };
    use crate::{GrantId, ToolCallId};

    fn admission() -> CanonicalRetryAdmission {
        CanonicalRetryAdmission {
            schema_version: 1,
            admission_id: "admission-1".to_owned(),
            step_execution_id: "step-execution-1".to_owned(),
            policy_id: "policy-1".to_owned(),
            decision_id: "decision-1".to_owned(),
            prior_attempt_id: "attempt-1".to_owned(),
            prior_attempt_ordinal: 1,
            prior_call_id: "call-1".to_owned(),
            prior_tool_call_id: ToolCallId::from_raw("tool-call-1"),
            prior_grant_id: GrantId::from_raw("grant-1"),
            successor_attempt_id: "attempt-2".to_owned(),
            successor_attempt_ordinal: 2,
            successor_call_id: "call-2".to_owned(),
            successor_tool_call_id: ToolCallId::from_raw("tool-call-2"),
            successor_grant_id: GrantId::from_raw("grant-2"),
            approval_requirement: CanonicalApprovalRequirement::NotRequired,
            successor_approval_id: None,
            reconciliation_required: false,
            reconciled: false,
            admitted_at: "2026-08-26T12:00:00Z".to_owned(),
            established_by: CanonicalExecutionAuthority::AgentmageRuntime,
            admission_sha256: "a".repeat(64),
        }
    }

    #[test]
    fn a_permitted_retry_is_a_new_attempt_and_round_trips() {
        let record = admission();
        assert!(record.opens_new_attempt());
        let encoded = serde_json::to_string(&record).expect("admission serializes");
        let decoded: CanonicalRetryAdmission =
            serde_json::from_str(&encoded).expect("admission deserializes");
        assert_eq!(decoded, record);
        assert!(decoded.opens_new_attempt());
    }

    #[test]
    fn reusing_any_prior_identity_is_a_replay_and_is_refused() {
        // Each of these would carry an already-executed effect forward into the retry.
        let mut replayed_attempt = admission();
        replayed_attempt.successor_attempt_id = replayed_attempt.prior_attempt_id.clone();
        assert!(
            !replayed_attempt.opens_new_attempt(),
            "attempt identity replayed"
        );

        let mut replayed_call = admission();
        replayed_call.successor_call_id = replayed_call.prior_call_id.clone();
        assert!(!replayed_call.opens_new_attempt(), "call identity replayed");

        let mut replayed_tool_call = admission();
        replayed_tool_call.successor_tool_call_id = replayed_tool_call.prior_tool_call_id.clone();
        assert!(
            !replayed_tool_call.opens_new_attempt(),
            "tool call replayed"
        );

        let mut replayed_grant = admission();
        replayed_grant.successor_grant_id = replayed_grant.prior_grant_id.clone();
        assert!(
            !replayed_grant.opens_new_attempt(),
            "consumed grant replayed"
        );
    }

    #[test]
    fn attempts_form_an_ordered_chain_and_honour_approval_and_reconciliation() {
        for ordinal in [1_u32, 2, 3, 99] {
            let mut record = admission();
            record.successor_attempt_ordinal = ordinal;
            assert_eq!(
                record.opens_new_attempt(),
                ordinal == 2,
                "successor ordinal {ordinal} must follow prior ordinal 1 by exactly one",
            );
        }
        let mut unreconciled = admission();
        unreconciled.reconciliation_required = true;
        assert!(
            !unreconciled.opens_new_attempt(),
            "a retry requiring reconciliation must reconcile first",
        );
        unreconciled.reconciled = true;
        assert!(unreconciled.opens_new_attempt());

        let mut per_attempt = admission();
        per_attempt.approval_requirement = CanonicalApprovalRequirement::RequiredPerAttempt;
        assert!(
            !per_attempt.opens_new_attempt(),
            "a prior approval never covers a later attempt",
        );
        per_attempt.successor_approval_id = Some("approval-2".to_owned());
        assert!(per_attempt.opens_new_attempt());
    }

    #[test]
    fn an_admission_carries_no_receipt_and_no_replay_surface() {
        let value = serde_json::to_value(admission()).expect("admission serializes");
        let object = value.as_object().expect("admission is an object");
        assert!(
            object.contains_key("decision_id"),
            "a retry needs a recorded decision"
        );
        for key in object.keys() {
            assert!(
                !key.contains("receipt"),
                "{key} must not present an outcome"
            );
            assert!(!key.contains("replay"), "{key} must not name a replay");
        }
        let encoded = serde_json::to_string(&admission()).expect("admission serializes");
        let widened = encoded.replace(
            "\"admission_id\"",
            "\"replay_of_receipt_id\":\"receipt-1\",\"admission_id\"",
        );
        assert!(
            serde_json::from_str::<CanonicalRetryAdmission>(&widened).is_err(),
            "a replayed receipt must never deserialize into an admission",
        );
        let dropped = encoded.replace("\"successor_approval_id\":null,", "");
        assert!(
            serde_json::from_str::<CanonicalRetryAdmission>(&dropped).is_err(),
            "a required optional field must be present and explicit",
        );
    }
}
