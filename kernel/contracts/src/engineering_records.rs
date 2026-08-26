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

impl CanonicalEffectClass {
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
        CanonicalExecutionBudgets, CanonicalIdempotencyRequirement, CanonicalRetryClass,
        CanonicalStepExecutionPolicy, CanonicalVerificationRequirement,
    };
    use crate::{EvidenceKind, PlanId, PlanStep, PlanStepId, PlanStepState};

    const ALL_EFFECT_CLASSES: [CanonicalEffectClass; 7] = [
        CanonicalEffectClass::ReadOnly,
        CanonicalEffectClass::IdempotentWrite,
        CanonicalEffectClass::Conditional,
        CanonicalEffectClass::NonIdempotent,
        CanonicalEffectClass::Destructive,
        CanonicalEffectClass::External,
        CanonicalEffectClass::Unknown,
    ];

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
        for effect in ALL_EFFECT_CLASSES {
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
