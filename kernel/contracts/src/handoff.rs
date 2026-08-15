//! Local-only, reviewable manual handoff contracts.

/// Sensitivity visible for one proposed handoff entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffSensitivity {
    /// Public content that requires no additional acknowledgment.
    Public,
    /// Content supplied directly by the user.
    UserProvided,
    /// Permitted non-public content.
    NonPublic,
    /// Content that cannot enter a handoff packet.
    Prohibited,
}

/// Whether an entry is direct source material or an explicit inference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffEntryKind {
    /// Direct bounded source excerpt.
    SourceExcerpt,
    /// Explicitly labeled inferred content.
    Inference,
}

/// Redaction disposition applied before review.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffEntryDisposition {
    /// The exact excerpt may be shown in the review and packet.
    Include,
    /// Only a fixed redaction marker may be shown.
    Redacted,
    /// The entry cannot enter a preview or packet.
    Prohibited,
}

/// Destination disclosed to the user without conferring delivery authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffDestinationClass {
    /// Content may be manually moved by the user to a separate Codex interface.
    ManualCodexInterface,
}

/// One exact source, inference, or redaction disclosed before rendering.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandoffDisclosureEntry {
    /// Stable entry identity within the draft.
    pub entry_id: String,
    /// Stable source identity.
    pub source_id: String,
    /// Display-only relative file name or logical source label.
    pub display_source: String,
    /// Exact range, fragment, or logical excerpt identity.
    pub fragment: String,
    /// Exact reviewed excerpt or fixed redaction marker.
    pub excerpt: String,
    /// Digest of the complete observed source object.
    pub content_sha256: String,
    /// Direct-source or inference label.
    pub kind: HandoffEntryKind,
    /// Visible sensitivity label.
    pub sensitivity: HandoffSensitivity,
    /// Include, redact, or prohibit disposition.
    pub disposition: HandoffEntryDisposition,
    /// Ordered stable redaction codes.
    pub redactions: Vec<String>,
    /// Whether the source was hidden from ordinary workspace visibility.
    pub hidden: bool,
    /// Whether the entry is directly related to the declared objective.
    pub related: bool,
    /// Digest of all preceding entry fields.
    pub entry_sha256: String,
}

/// Complete local input proposed for a manual handoff.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandoffDraft {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable handoff identity.
    pub handoff_id: String,
    /// Current workspace-state digest.
    pub workspace_state_sha256: String,
    /// Current governing policy digest.
    pub policy_sha256: String,
    /// Current redaction-policy digest.
    pub redaction_policy_sha256: String,
    /// Exact objective.
    pub objective: String,
    /// Ordered acceptance criteria.
    pub acceptance_criteria: Vec<String>,
    /// Ordered authority, behavior, and output constraints.
    pub constraints: Vec<String>,
    /// Exact included source and inference disclosures.
    pub entries: Vec<HandoffDisclosureEntry>,
    /// Ordered, visible exclusions.
    pub exclusions: Vec<String>,
    /// Ordered unresolved questions.
    pub unresolved_questions: Vec<String>,
    /// Non-authoritative destination disclosure.
    pub destination: HandoffDestinationClass,
}

/// Content-addressed manifest for one exact rendered packet.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandoffPacketManifest {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact handoff identity.
    pub handoff_id: String,
    /// Digest of the source draft.
    pub draft_sha256: String,
    /// Ordered entry digests.
    pub entry_sha256: Vec<String>,
    /// Digest of exact UTF-8 packet bytes.
    pub packet_sha256: String,
    /// Exact UTF-8 packet size.
    pub packet_bytes: u64,
    /// Destination shown to the user.
    pub destination: HandoffDestinationClass,
    /// Whether permitted non-public content requires acknowledgment.
    pub acknowledgment_required: bool,
    /// Always false because AgentMage cannot deliver a handoff.
    pub delivered: bool,
    /// Digest of all preceding manifest fields.
    pub manifest_sha256: String,
}

/// Mandatory exact-content review shown before local rendering completes.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandoffReview {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact preview identity.
    pub preview_id: String,
    /// Exact packet text the user may later transfer manually.
    pub packet_markdown: String,
    /// Exact packet manifest.
    pub manifest: HandoffPacketManifest,
    /// Explicit local-only statement.
    pub local_only_notice: String,
    /// Logical expiry instant in milliseconds.
    pub expires_at_ms: u64,
    /// Digest bound to every review field and the required acknowledgment state.
    pub confirmation_sha256: String,
}

/// Prohibited delivery or interface-control action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandoffProhibitedAction {
    /// Invoke Codex directly.
    CodexInvocation,
    /// Activate or control another tab.
    TabActivation,
    /// Populate another Chat interface.
    ChatPopulation,
    /// Write packet content to the clipboard.
    ClipboardWrite,
    /// Launch a URI or external handler.
    UriLaunch,
    /// Deliver through a local model endpoint.
    LocalRuntimeDelivery,
    /// Deliver through a raw runtime endpoint.
    RawRuntimeDelivery,
    /// Contact any network destination.
    NetworkCall,
    /// Submit the packet automatically.
    AutomaticSubmission,
    /// Upload packet content to any service or destination.
    FileUpload,
    /// Control a browser to deliver or populate packet content.
    BrowserControl,
    /// Deliver later from a schedule or timer.
    ScheduledDelivery,
    /// Route delivery through another model, tool, connector, or workflow.
    RoutedDelivery,
    /// Reuse standing consent instead of exact current user review.
    StandingConsentDelivery,
}

/// Local-only terminal outcome for a review or prohibited action attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalHandoffOutcome {
    /// The current reviewed packet was rendered locally.
    Rendered,
    /// Rendering was cancelled.
    Cancelled,
    /// A stale, prohibited, or unacknowledged operation was denied.
    Denied,
}

/// Content-free local receipt that never claims external delivery.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalHandoffReceipt {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact attempt identity.
    pub attempt_id: String,
    /// Exact handoff identity when a review existed.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub handoff_id: Option<String>,
    /// Terminal local outcome.
    pub outcome: LocalHandoffOutcome,
    /// Stable result or denial code.
    pub result_code: String,
    /// Prohibited action when one was attempted.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub prohibited_action: Option<HandoffProhibitedAction>,
    /// Exact packet digest when local rendering succeeded.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub packet_sha256: Option<String>,
    /// Always false; retained to prevent ambiguous delivery claims.
    pub external_delivery_attempted: bool,
    /// Digest of all preceding receipt fields.
    pub receipt_sha256: String,
}

/// Locally rendered packet, exact manifest, and no-delivery receipt.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderedHandoff {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact packet text previously reviewed.
    pub packet_markdown: String,
    /// Exact packet manifest previously reviewed.
    pub manifest: HandoffPacketManifest,
    /// Local-only terminal receipt.
    pub receipt: LocalHandoffReceipt,
}
