//! Canonical local conversation and immutable turn contracts.

use crate::{
    CONTRACT_SCHEMA_VERSION, ConversationId, ConversationTurnId, DataSensitivity, GrantId,
    ModelProfileId, ReceiptId, SessionCheckpointId, WorkspaceId,
};

/// Closed lifecycle state for one local conversation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationStatus {
    /// New turns may be appended.
    Active,
    /// Work reached a normal terminal boundary.
    Completed,
    /// Work was explicitly cancelled.
    Cancelled,
    /// Work stopped with a recorded failure.
    Failed,
    /// Conversation is hidden from the default active view but remains retained.
    Archived,
}

/// Closed author class for one immutable conversation turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationTurnRole {
    /// Effective system or product instruction content.
    System,
    /// User-authored content.
    User,
    /// Model-authored content.
    Assistant,
    /// Tool result content.
    Tool,
}

/// Closed visible retention class for one local conversation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationRetentionKind {
    /// Retain until the exact session-policy expiration.
    Session,
    /// Retain until an explicit expiration selected by policy or user.
    UntilExpiration,
    /// Retain under an explicit user hold without automatic expiry.
    UserHold,
}

/// Visible retention policy carried by canonical conversation metadata.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationRetention {
    /// Closed retention class.
    pub kind: ConversationRetentionKind,
    /// Exact UTC expiration for expiring classes; absent only for a user hold.
    pub expires_at_epoch_ms: Option<u64>,
    /// Digest of the exact policy or explicit decision establishing this rule.
    pub policy_sha256: String,
}

/// Content-addressed attachment identity; attachment bytes are never embedded here.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationAttachmentReference {
    /// Stable attachment reference identity.
    pub reference_id: String,
    /// User-visible bounded file name, not a machine-specific path.
    pub display_name: String,
    /// Lowercase SHA-256 digest of the referenced bytes.
    pub content_sha256: String,
    /// Bounded media type supplied by the trusted attachment boundary.
    pub media_type: String,
}

/// Canonical metadata for one encrypted local conversation.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationRecord {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable conversation identity.
    pub conversation_id: ConversationId,
    /// Bounded user-visible title.
    pub title: String,
    /// Strictest sensitivity assigned to retained conversation metadata or turns.
    pub sensitivity: DataSensitivity,
    /// UTC creation time as Unix epoch milliseconds.
    pub created_at_epoch_ms: u64,
    /// UTC last-update time as Unix epoch milliseconds.
    pub updated_at_epoch_ms: u64,
    /// Local calendar date in `YYYY-MM-DD` form for deterministic date filtering.
    pub local_date: String,
    /// IANA timezone identity used to derive local date fields.
    pub local_timezone: String,
    /// Exact approved workspace identity.
    pub workspace_id: WorkspaceId,
    /// Optional bounded project identity.
    pub project_id: Option<String>,
    /// Exact model profile selected for the current turn.
    pub model_profile_id: ModelProfileId,
    /// Current lifecycle state.
    pub status: ConversationStatus,
    /// Original conversation when this record is a branch.
    pub parent_conversation_id: Option<ConversationId>,
    /// Exact immutable parent turn when this record is a historical branch.
    pub branch_from_turn_id: Option<ConversationTurnId>,
    /// Latest immutable turn, or none before the first turn.
    pub current_turn_id: Option<ConversationTurnId>,
    /// Stable, deduplicated search tags.
    pub tags: Vec<String>,
    /// Visible retention policy.
    pub retention: ConversationRetention,
    /// Whether the user pinned this conversation.
    pub pinned: bool,
    /// Whether exact turn text may be retained for this conversation.
    pub persistence_enabled: bool,
}

impl ConversationRecord {
    /// Creates the first supported schema marker for callers constructing a record.
    #[must_use]
    pub const fn current_schema_version() -> u16 {
        CONTRACT_SCHEMA_VERSION
    }
}

/// One immutable persisted turn and its content-addressed evidence relationships.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationTurn {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable turn identity.
    pub turn_id: ConversationTurnId,
    /// Owning conversation identity.
    pub conversation_id: ConversationId,
    /// Strictly increasing one-based ordinal within the conversation.
    pub ordinal: u64,
    /// Closed author class.
    pub role: ConversationTurnRole,
    /// Turn-level sensitivity assigned before persistence.
    pub sensitivity: DataSensitivity,
    /// UTC creation time as Unix epoch milliseconds.
    pub created_at_epoch_ms: u64,
    /// Local calendar date in `YYYY-MM-DD` form.
    pub local_date: String,
    /// Exact text only when persistence was enabled and policy admitted it.
    pub text: Option<String>,
    /// Digest of the original text, including when exact text is not retained.
    pub text_sha256: String,
    /// Attachment identities and hashes; attachment contents remain external.
    pub attachments: Vec<ConversationAttachmentReference>,
    /// Capability grants referenced by this turn.
    pub grant_ids: Vec<GrantId>,
    /// Terminal receipts referenced by this turn.
    pub receipt_ids: Vec<ReceiptId>,
    /// Safe-boundary checkpoint published for this turn, when present.
    pub checkpoint_id: Option<SessionCheckpointId>,
    /// Stable citation identities referenced by this turn or its checked summary.
    pub citation_ids: Vec<String>,
    /// Content hashes for original source evidence referenced by this turn.
    pub source_sha256: Vec<String>,
}
