//! Closed terminal and headless-client contracts with no direct authority or native effects.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};

use agentmage_kernel_contracts::GrantOperation;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// First supported thin-client protocol version.
pub const THIN_CLIENT_PROTOCOL_VERSION: u16 = 1;

/// Maximum encoded request accepted from any client surface.
pub const MAX_THIN_CLIENT_REQUEST_BYTES: usize = 64 * 1024;

/// Maximum encoded event accepted from the canonical host stream.
pub const MAX_THIN_CLIENT_EVENT_BYTES: usize = 1024 * 1024;

/// Maximum cumulative encoded output retained for one request.
pub const MAX_THIN_CLIENT_OUTPUT_BYTES: u64 = 4 * 1024 * 1024;

const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_TEXT_BYTES: usize = 64 * 1024;
const MAX_QUERY_BYTES: usize = 4 * 1024;
const MAX_STATUS_ITEMS: usize = 64;
const MAX_EVENT_COUNT: usize = 4_096;
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Stable failure class exposed as a documented process exit code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum ClientExitCode {
    /// Operation completed and its terminal receipt verified.
    Success = 0,
    /// Command-line or protocol input was malformed.
    InvalidInput = 2,
    /// Required authority was absent, stale, ambiguous, or mismatched.
    AuthorityDenied = 3,
    /// Policy denied the exact request.
    PolicyDenied = 4,
    /// The authenticated local host or required dependency was unavailable.
    ServiceUnavailable = 5,
    /// The exact request was cancelled.
    Cancelled = 6,
    /// Output or another declared resource exceeded its bound.
    ResourceBound = 7,
    /// The operation ended without enough evidence to claim success or failure.
    Uncertain = 8,
    /// A versioned client, policy, or event contract changed incompatibly.
    ProtocolMismatch = 9,
}

impl ClientExitCode {
    /// Returns the stable numeric process exit code.
    #[must_use]
    pub const fn process_code(self) -> u8 {
        self as u8
    }
}

/// Stable redacted thin-client failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThinClientError {
    /// Encoded input or output exceeded a declared byte or item bound.
    SizeExceeded,
    /// JSON or a closed command/event shape was malformed.
    Malformed,
    /// A client or event version was unsupported.
    VersionMismatch,
    /// An identifier, date, digest, command argument, or status field was invalid.
    InvalidValue,
    /// Headless authority was absent, stale, interactive-only, or not exact.
    AuthorityDenied,
    /// A request or event stream was replayed, reordered, or resumed ambiguously.
    ReplayDenied,
    /// The transport closed, failed, or returned an invalid stream.
    TransportFailed,
    /// Cancellation was observed before a terminal verified event.
    Cancelled,
}

impl ThinClientError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::SizeExceeded => "client.bound.exceeded",
            Self::Malformed => "client.protocol.malformed",
            Self::VersionMismatch => "client.protocol.version_mismatch",
            Self::InvalidValue => "client.protocol.value_invalid",
            Self::AuthorityDenied => "client.authority.denied",
            Self::ReplayDenied => "client.replay.denied",
            Self::TransportFailed => "client.transport.failed",
            Self::Cancelled => "client.cancelled",
        }
    }

    /// Maps this failure to the stable process exit contract.
    #[must_use]
    pub const fn exit_code(self) -> ClientExitCode {
        match self {
            Self::SizeExceeded => ClientExitCode::ResourceBound,
            Self::Malformed | Self::InvalidValue => ClientExitCode::InvalidInput,
            Self::VersionMismatch => ClientExitCode::ProtocolMismatch,
            Self::AuthorityDenied | Self::ReplayDenied => ClientExitCode::AuthorityDenied,
            Self::TransportFailed => ClientExitCode::ServiceUnavailable,
            Self::Cancelled => ClientExitCode::Cancelled,
        }
    }
}

impl fmt::Display for ThinClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for ThinClientError {}

/// Closed client surface; every variant is transport/display only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientSurface {
    /// Native Visual Studio Code Chat surface.
    NativeChat,
    /// Human-operated terminal chat and command surface.
    InteractiveCli,
    /// Noninteractive versioned JSON interface.
    Json,
    /// Local software-development-kit client.
    Sdk,
    /// Agent Client Protocol compatible local client.
    Acp,
}

impl ClientSurface {
    /// Reports whether this surface may present a live human approval prompt.
    #[must_use]
    pub const fn has_interactive_approval(self) -> bool {
        matches!(self, Self::NativeChat | Self::InteractiveCli)
    }
}

/// Closed conversation command family.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConversationClientCommand {
    /// List conversations within optional inclusive local dates.
    List {
        /// Optional first local date in `YYYY-MM-DD` form.
        from: Option<String>,
        /// Optional last local date in `YYYY-MM-DD` form.
        to: Option<String>,
    },
    /// Search retained conversations using bounded local text.
    Search {
        /// Exact search query.
        query: String,
    },
    /// Show one conversation without changing active state.
    Show {
        /// Stable conversation identity.
        conversation_id: String,
    },
    /// Open one conversation in the current interactive surface.
    Open {
        /// Stable conversation identity.
        conversation_id: String,
    },
    /// Resume from the current terminal checkpoint.
    Resume {
        /// Stable conversation identity.
        conversation_id: String,
    },
    /// Branch from one exact immutable turn without changing the original.
    Branch {
        /// Stable source conversation identity.
        conversation_id: String,
        /// Exact immutable turn identity.
        turn_id: String,
    },
}

/// Closed local Markdown/vault command family.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum VaultClientCommand {
    /// Search an already approved local Markdown scope.
    Search {
        /// Exact bounded query.
        query: String,
    },
    /// Show one note by stable identity.
    NoteShow {
        /// Stable note identity.
        note_id: String,
    },
    /// List outgoing links for one note.
    Links {
        /// Stable note identity.
        note_id: String,
    },
    /// List backlinks for one note.
    Backlinks {
        /// Stable note identity.
        note_id: String,
    },
    /// Show the canonical local task projection.
    Tasks,
}

/// Closed operational command family exposed by terminal and headless clients.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum OperationalClientCommand {
    /// Publish a safe-boundary local checkpoint through the kernel.
    Checkpoint,
    /// Prepare or render a separately reviewed local handoff.
    Handoff,
    /// Show content-free canonical audit records.
    Audit,
    /// Inspect one memory item or the visible memory inventory.
    MemoryInspect {
        /// Optional stable memory identity.
        memory_id: Option<String>,
    },
    /// Propose one explicit memory correction for separate approval.
    MemoryCorrect {
        /// Stable memory identity.
        memory_id: String,
        /// Exact proposed replacement text.
        replacement: String,
    },
    /// Prepare an explicit derived export.
    Export {
        /// Closed export profile identity.
        profile: String,
    },
    /// Validate an explicit import manifest without ambient discovery.
    Import {
        /// Digest of the exact user-supplied import manifest.
        manifest_sha256: String,
    },
    /// Return local doctor and diagnostics state without network use.
    Diagnostics,
}

/// Complete closed client command family.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientCommand {
    /// Submit one bounded user message through the canonical runtime.
    Chat {
        /// Exact user-authored message.
        message: String,
    },
    /// Access canonical local conversation operations.
    Conversations {
        /// Exact conversation action.
        action: ConversationClientCommand,
    },
    /// Access approved local Markdown knowledge operations.
    Vault {
        /// Exact vault action.
        action: VaultClientCommand,
    },
    /// Access checkpoint, handoff, audit, memory, transfer, or diagnostic operations.
    Operations {
        /// Exact operational action.
        action: OperationalClientCommand,
    },
}

impl ClientCommand {
    /// Returns the one exact grant operation required by this command.
    #[must_use]
    pub const fn required_operation(&self) -> GrantOperation {
        match self {
            Self::Chat { .. } => GrantOperation::ModelInference,
            Self::Conversations { action } => match action {
                ConversationClientCommand::Branch { .. } => GrantOperation::WorkspaceWrite,
                ConversationClientCommand::Open { .. }
                | ConversationClientCommand::Resume { .. } => GrantOperation::DatabaseRead,
                ConversationClientCommand::List { .. }
                | ConversationClientCommand::Search { .. }
                | ConversationClientCommand::Show { .. } => GrantOperation::DatabaseRead,
            },
            Self::Vault { .. } => GrantOperation::WorkspaceRead,
            Self::Operations { action } => match action {
                OperationalClientCommand::MemoryCorrect { .. }
                | OperationalClientCommand::Export { .. }
                | OperationalClientCommand::Import { .. }
                | OperationalClientCommand::Checkpoint => GrantOperation::WorkspaceWrite,
                OperationalClientCommand::Handoff => GrantOperation::DraftCreate,
                OperationalClientCommand::Audit
                | OperationalClientCommand::MemoryInspect { .. }
                | OperationalClientCommand::Diagnostics => GrantOperation::DatabaseRead,
            },
        }
    }

    /// Verifies that every command argument satisfies its closed semantic bounds.
    pub fn verify(&self) -> Result<(), ThinClientError> {
        if self.validate() {
            Ok(())
        } else {
            Err(ThinClientError::InvalidValue)
        }
    }

    fn validate(&self) -> bool {
        match self {
            Self::Chat { message } => valid_text(message, MAX_TEXT_BYTES),
            Self::Conversations { action } => match action {
                ConversationClientCommand::List { from, to } => {
                    from.as_deref().is_none_or(valid_date)
                        && to.as_deref().is_none_or(valid_date)
                        && match (from, to) {
                            (Some(from), Some(to)) => from <= to,
                            _ => true,
                        }
                }
                ConversationClientCommand::Search { query } => valid_text(query, MAX_QUERY_BYTES),
                ConversationClientCommand::Show { conversation_id }
                | ConversationClientCommand::Open { conversation_id }
                | ConversationClientCommand::Resume { conversation_id } => {
                    valid_identifier(conversation_id)
                }
                ConversationClientCommand::Branch {
                    conversation_id,
                    turn_id,
                } => valid_identifier(conversation_id) && valid_identifier(turn_id),
            },
            Self::Vault { action } => match action {
                VaultClientCommand::Search { query } => valid_text(query, MAX_QUERY_BYTES),
                VaultClientCommand::NoteShow { note_id }
                | VaultClientCommand::Links { note_id }
                | VaultClientCommand::Backlinks { note_id } => valid_identifier(note_id),
                VaultClientCommand::Tasks => true,
            },
            Self::Operations { action } => match action {
                OperationalClientCommand::MemoryInspect { memory_id } => {
                    memory_id.as_deref().is_none_or(valid_identifier)
                }
                OperationalClientCommand::MemoryCorrect {
                    memory_id,
                    replacement,
                } => valid_identifier(memory_id) && valid_text(replacement, MAX_TEXT_BYTES),
                OperationalClientCommand::Export { profile } => valid_identifier(profile),
                OperationalClientCommand::Import { manifest_sha256 } => {
                    valid_sha256(manifest_sha256)
                }
                OperationalClientCommand::Checkpoint
                | OperationalClientCommand::Handoff
                | OperationalClientCommand::Audit
                | OperationalClientCommand::Diagnostics => true,
            },
        }
    }
}

/// Exact visible status shared by every first-party client.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientStatusSnapshot {
    /// Exact active workspace identity.
    pub workspace_id: String,
    /// Exact selected model profile, absent when no model is active.
    pub model_profile_id: Option<String>,
    /// Exact permission-profile identity.
    pub permission_profile_id: String,
    /// Current conversation identity, when one is active.
    pub conversation_id: Option<String>,
    /// Current plan step identity, when one is active.
    pub plan_step_id: Option<String>,
    /// Bounded user-visible writable-root labels.
    pub writable_roots: Vec<String>,
    /// True when the canonical runtime has no external-network authority.
    pub offline: bool,
    /// Canonical digest of this status with this field zeroed.
    pub status_sha256: String,
}

impl ClientStatusSnapshot {
    /// Validates and seals an exact status snapshot.
    pub fn seal(mut self) -> Result<Self, ThinClientError> {
        self.status_sha256 = ZERO_SHA256.to_owned();
        if !self.valid() {
            return Err(ThinClientError::InvalidValue);
        }
        self.status_sha256 = canonical_sha256(&self)?;
        Ok(self)
    }

    /// Verifies the current status fields and canonical digest.
    #[must_use]
    pub fn verify(&self) -> bool {
        let mut canonical = self.clone();
        canonical.status_sha256 = ZERO_SHA256.to_owned();
        self.valid() && canonical_sha256(&canonical).as_deref() == Ok(self.status_sha256.as_str())
    }

    fn valid(&self) -> bool {
        valid_identifier(&self.workspace_id)
            && self
                .model_profile_id
                .as_deref()
                .is_none_or(valid_identifier)
            && valid_identifier(&self.permission_profile_id)
            && self.conversation_id.as_deref().is_none_or(valid_identifier)
            && self.plan_step_id.as_deref().is_none_or(valid_identifier)
            && self.writable_roots.len() <= MAX_STATUS_ITEMS
            && self
                .writable_roots
                .iter()
                .all(|root| valid_text(root, MAX_QUERY_BYTES) && !root.contains('\0'))
            && valid_sha256(&self.status_sha256)
    }
}

/// Exact predeclared authority presented by a noninteractive client.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PredeclaredClientGrant {
    /// Stable kernel-issued grant identity.
    pub grant_id: String,
    /// Exact operation admitted by the grant.
    pub operation: GrantOperation,
    /// Exact current policy identity.
    pub policy_sha256: String,
    /// Exact kernel-operation digest bound to the grant.
    pub arguments_sha256: String,
    /// Nonce digest; raw nonce material is never carried by this interface.
    pub nonce_sha256: String,
    /// Trusted issuance time.
    pub issued_at_epoch_ms: u64,
    /// Trusted exclusive expiration time.
    pub expires_at_epoch_ms: u64,
    /// Grants presented through this protocol are always one use.
    pub single_use: bool,
}

/// Exact authority form visible to a thin client.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientAuthority {
    /// Live human confirmation is available only in native Chat or interactive CLI.
    Interactive {
        /// Digest of the exact protected approval channel.
        approval_channel_sha256: String,
    },
    /// One predeclared bounded expiring kernel grant.
    Predeclared {
        /// Exact grant envelope.
        grant: PredeclaredClientGrant,
    },
}

/// Resume cursor for an already-started canonical event stream.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientResumeCursor {
    /// Stable canonical stream identity.
    pub stream_id: String,
    /// Last fully verified event sequence held by the client.
    pub after_sequence: u64,
    /// Digest of that exact last event.
    pub last_event_sha256: String,
}

/// Versioned exact request shared by native Chat, CLI, JSON, SDK, and ACP clients.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThinClientRequest {
    /// Closed protocol version.
    pub schema_version: u16,
    /// Stable non-replayable request identity.
    pub request_id: String,
    /// Display/transport surface, never an authority source.
    pub surface: ClientSurface,
    /// Exact current status rendered by the client.
    pub status: ClientStatusSnapshot,
    /// Exact closed command.
    pub command: ClientCommand,
    /// Interactive or predeclared authority presentation.
    pub authority: ClientAuthority,
    /// Exact current policy identity.
    pub policy_sha256: String,
    /// Stable cancellation identity.
    pub cancellation_id: String,
    /// Per-event byte ceiling.
    pub max_event_bytes: u64,
    /// Cumulative event byte ceiling.
    pub max_output_bytes: u64,
    /// Optional cursor that can only resume an existing operation.
    pub resume: Option<ClientResumeCursor>,
    /// Digest of the surface-independent kernel operation.
    pub kernel_request_sha256: String,
    /// Canonical digest of this request with this field zeroed.
    pub request_sha256: String,
}

impl ThinClientRequest {
    /// Seals and validates a request at one trusted kernel-clock instant.
    pub fn seal(mut self, now_epoch_ms: u64) -> Result<Self, ThinClientError> {
        self.schema_version = THIN_CLIENT_PROTOCOL_VERSION;
        self.kernel_request_sha256 = kernel_operation_sha256(
            &self.status.workspace_id,
            &self.command,
            &self.policy_sha256,
        )?;
        self.request_sha256 = ZERO_SHA256.to_owned();
        self.validate(now_epoch_ms)?;
        self.request_sha256 = canonical_sha256(&self)?;
        Ok(self)
    }

    /// Verifies all request bindings at one trusted kernel-clock instant.
    pub fn verify(&self, now_epoch_ms: u64) -> Result<(), ThinClientError> {
        self.validate(now_epoch_ms)?;
        let mut canonical = self.clone();
        canonical.request_sha256 = ZERO_SHA256.to_owned();
        if canonical_sha256(&canonical).as_deref() != Ok(self.request_sha256.as_str()) {
            return Err(ThinClientError::InvalidValue);
        }
        Ok(())
    }

    fn validate(&self, now_epoch_ms: u64) -> Result<(), ThinClientError> {
        if self.schema_version != THIN_CLIENT_PROTOCOL_VERSION {
            return Err(ThinClientError::VersionMismatch);
        }
        if !valid_identifier(&self.request_id)
            || !self.status.verify()
            || !self.command.validate()
            || !valid_sha256(&self.policy_sha256)
            || !valid_identifier(&self.cancellation_id)
            || self.max_event_bytes == 0
            || self.max_event_bytes > MAX_THIN_CLIENT_EVENT_BYTES as u64
            || self.max_output_bytes < self.max_event_bytes
            || self.max_output_bytes > MAX_THIN_CLIENT_OUTPUT_BYTES
            || !valid_sha256(&self.kernel_request_sha256)
            || !valid_sha256(&self.request_sha256)
        {
            return Err(ThinClientError::InvalidValue);
        }
        if kernel_operation_sha256(
            &self.status.workspace_id,
            &self.command,
            &self.policy_sha256,
        )
        .as_deref()
            != Ok(self.kernel_request_sha256.as_str())
        {
            return Err(ThinClientError::InvalidValue);
        }
        if let Some(cursor) = &self.resume
            && (!valid_identifier(&cursor.stream_id) || !valid_sha256(&cursor.last_event_sha256))
        {
            return Err(ThinClientError::InvalidValue);
        }
        match &self.authority {
            ClientAuthority::Interactive {
                approval_channel_sha256,
            } => {
                if !self.surface.has_interactive_approval()
                    || !valid_sha256(approval_channel_sha256)
                {
                    return Err(ThinClientError::AuthorityDenied);
                }
            }
            ClientAuthority::Predeclared { grant } => {
                if !valid_identifier(&grant.grant_id)
                    || grant.operation != self.command.required_operation()
                    || grant.policy_sha256 != self.policy_sha256
                    || grant.arguments_sha256 != self.kernel_request_sha256
                    || !valid_sha256(&grant.nonce_sha256)
                    || !grant.single_use
                    || grant.issued_at_epoch_ms > now_epoch_ms
                    || now_epoch_ms >= grant.expires_at_epoch_ms
                    || grant.issued_at_epoch_ms >= grant.expires_at_epoch_ms
                {
                    return Err(ThinClientError::AuthorityDenied);
                }
            }
        }
        Ok(())
    }
}

/// Computes the exact surface-independent kernel operation digest.
pub fn kernel_operation_sha256(
    workspace_id: &str,
    command: &ClientCommand,
    policy_sha256: &str,
) -> Result<String, ThinClientError> {
    if !valid_identifier(workspace_id) || !command.validate() || !valid_sha256(policy_sha256) {
        return Err(ThinClientError::InvalidValue);
    }
    canonical_sha256(&(
        THIN_CLIENT_PROTOCOL_VERSION,
        workspace_id,
        command,
        policy_sha256,
    ))
}

/// Stable content channel for a bounded visible event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientContentChannel {
    /// Assistant or command result content.
    Content,
    /// Current status or progress narration.
    Progress,
    /// Exact non-authoritative preview.
    Preview,
    /// Exact reviewed diff.
    Diff,
    /// Source citation display.
    Citation,
    /// Bounded user-visible error narration.
    Error,
}

/// Closed event payload emitted only by the canonical host.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
pub enum ThinClientEventKind {
    /// Canonical operation was admitted or an existing stream was resumed.
    Started,
    /// Exact current status projection.
    Status {
        /// Hash-verified status.
        status: ClientStatusSnapshot,
    },
    /// Bounded visible text with an exact content digest.
    Content {
        /// Display channel.
        channel: ClientContentChannel,
        /// Bounded text.
        text: String,
        /// Digest of the exact text bytes.
        text_sha256: String,
    },
    /// Interactive approval must occur through the protected native channel.
    ApprovalRequired {
        /// Exact operation awaiting approval.
        operation: GrantOperation,
        /// Digest of the complete preview.
        preview_sha256: String,
        /// Exclusive approval expiration.
        expires_at_epoch_ms: u64,
    },
    /// Content-free terminal or intermediate receipt projection.
    Receipt {
        /// Stable receipt identity.
        receipt_id: String,
        /// Digest of the exact canonical receipt.
        receipt_sha256: String,
        /// Stable terminal or partial outcome.
        outcome: String,
    },
    /// Operation completed with an exact canonical final-state digest.
    Completed {
        /// Exact final state digest.
        final_state_sha256: String,
    },
    /// Operation failed closed without a completed effect claim.
    Denied {
        /// Stable content-free denial code.
        code: String,
    },
    /// Operation observed cancellation without a success claim.
    Cancelled {
        /// Stable content-free cancellation code.
        code: String,
    },
}

impl ThinClientEventKind {
    const fn terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed { .. } | Self::Denied { .. } | Self::Cancelled { .. }
        )
    }

    fn valid(&self) -> bool {
        match self {
            Self::Started => true,
            Self::Status { status } => status.verify(),
            Self::Content {
                text, text_sha256, ..
            } => valid_text(text, MAX_TEXT_BYTES) && sha256_hex(text.as_bytes()) == *text_sha256,
            Self::ApprovalRequired {
                preview_sha256,
                expires_at_epoch_ms,
                ..
            } => valid_sha256(preview_sha256) && *expires_at_epoch_ms > 0,
            Self::Receipt {
                receipt_id,
                receipt_sha256,
                outcome,
            } => {
                valid_identifier(receipt_id)
                    && valid_sha256(receipt_sha256)
                    && valid_identifier(outcome)
            }
            Self::Completed { final_state_sha256 } => valid_sha256(final_state_sha256),
            Self::Denied { code } | Self::Cancelled { code } => valid_code(code),
        }
    }
}

/// One hash-bound event in a canonical resumable stream.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThinClientEvent {
    /// Closed protocol version.
    pub schema_version: u16,
    /// Exact owning request identity.
    pub request_id: String,
    /// Stable canonical stream identity.
    pub stream_id: String,
    /// Strictly increasing zero-based event sequence.
    pub sequence: u64,
    /// Digest of the surface-independent kernel operation.
    pub kernel_request_sha256: String,
    /// Exact policy identity used by the operation.
    pub policy_sha256: String,
    /// Cumulative encoded event bytes through this event.
    pub cumulative_output_bytes: u64,
    /// Closed event payload.
    pub kind: ThinClientEventKind,
    /// Digest of the prior event or zeroes for sequence zero.
    pub previous_event_sha256: String,
    /// Canonical digest of this event with this field zeroed.
    pub event_sha256: String,
}

impl ThinClientEvent {
    /// Seals one event after the caller computes its cumulative encoded-byte count.
    pub fn seal(mut self) -> Result<Self, ThinClientError> {
        self.schema_version = THIN_CLIENT_PROTOCOL_VERSION;
        self.event_sha256 = ZERO_SHA256.to_owned();
        if !self.valid() {
            return Err(ThinClientError::InvalidValue);
        }
        self.event_sha256 = canonical_sha256(&self)?;
        Ok(self)
    }

    /// Verifies this event without trusting its producer.
    #[must_use]
    pub fn verify(&self) -> bool {
        let mut canonical = self.clone();
        canonical.event_sha256 = ZERO_SHA256.to_owned();
        self.valid() && canonical_sha256(&canonical).as_deref() == Ok(self.event_sha256.as_str())
    }

    fn valid(&self) -> bool {
        self.schema_version == THIN_CLIENT_PROTOCOL_VERSION
            && valid_identifier(&self.request_id)
            && valid_identifier(&self.stream_id)
            && valid_sha256(&self.kernel_request_sha256)
            && valid_sha256(&self.policy_sha256)
            && self.cumulative_output_bytes > 0
            && self.kind.valid()
            && valid_sha256(&self.previous_event_sha256)
            && valid_sha256(&self.event_sha256)
    }
}

/// Verified terminal event stream returned by a thin client.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedClientStream {
    /// Exact verified events in sequence order.
    pub events: Vec<ThinClientEvent>,
    /// Stable terminal process exit code.
    pub exit_code: ClientExitCode,
}

/// Verifies framing, ordering, bounds, hashes, authority behavior, and terminal state.
pub fn verify_event_stream(
    request: &ThinClientRequest,
    encoded_events: &[Vec<u8>],
) -> Result<VerifiedClientStream, ThinClientError> {
    if encoded_events.is_empty() || encoded_events.len() > MAX_EVENT_COUNT {
        return Err(ThinClientError::Malformed);
    }
    let mut events = Vec::with_capacity(encoded_events.len());
    let mut total = 0_u64;
    let mut previous = ZERO_SHA256.to_owned();
    let mut receipt_seen = false;
    for (index, encoded) in encoded_events.iter().enumerate() {
        if encoded.is_empty()
            || encoded.len() > request.max_event_bytes as usize
            || encoded.len() > MAX_THIN_CLIENT_EVENT_BYTES
        {
            return Err(ThinClientError::SizeExceeded);
        }
        total = total
            .checked_add(encoded.len() as u64)
            .filter(|value| *value <= request.max_output_bytes)
            .ok_or(ThinClientError::SizeExceeded)?;
        let event: ThinClientEvent =
            serde_json::from_slice(encoded).map_err(|_| ThinClientError::Malformed)?;
        if !event.verify()
            || event.request_id != request.request_id
            || event.kernel_request_sha256 != request.kernel_request_sha256
            || event.policy_sha256 != request.policy_sha256
            || event.sequence != index as u64
            || event.previous_event_sha256 != previous
            || event.cumulative_output_bytes != total
            || (index == 0 && !matches!(event.kind, ThinClientEventKind::Started))
            || (event.kind.terminal() && index + 1 != encoded_events.len())
            || (!event.kind.terminal() && index + 1 == encoded_events.len())
            || (!request.surface.has_interactive_approval()
                && matches!(event.kind, ThinClientEventKind::ApprovalRequired { .. }))
        {
            return Err(ThinClientError::Malformed);
        }
        if matches!(event.kind, ThinClientEventKind::Receipt { .. }) {
            receipt_seen = true;
        }
        previous.clone_from(&event.event_sha256);
        events.push(event);
    }
    let terminal = &events.last().ok_or(ThinClientError::Malformed)?.kind;
    let exit_code = match terminal {
        ThinClientEventKind::Completed { .. } if receipt_seen => ClientExitCode::Success,
        ThinClientEventKind::Completed { .. } => return Err(ThinClientError::Malformed),
        ThinClientEventKind::Denied { .. } => ClientExitCode::PolicyDenied,
        ThinClientEventKind::Cancelled { .. } => ClientExitCode::Cancelled,
        _ => return Err(ThinClientError::Malformed),
    };
    Ok(VerifiedClientStream { events, exit_code })
}

/// Thread-safe cancellation signal that carries no operation authority.
#[derive(Debug)]
pub struct ClientCancellation {
    cancellation_id: String,
    cancelled: AtomicBool,
}

impl ClientCancellation {
    /// Creates one cancellation signal bound to an exact request identity.
    pub fn new(cancellation_id: impl Into<String>) -> Result<Self, ThinClientError> {
        let cancellation_id = cancellation_id.into();
        if !valid_identifier(&cancellation_id) {
            return Err(ThinClientError::InvalidValue);
        }
        Ok(Self {
            cancellation_id,
            cancelled: AtomicBool::new(false),
        })
    }

    /// Requests deterministic cancellation.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    /// Reports whether cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    /// Returns the exact cancellation identity.
    #[must_use]
    pub fn cancellation_id(&self) -> &str {
        &self.cancellation_id
    }
}

/// Transport-only boundary implemented by an authenticated local host bridge.
pub trait ThinClientTransport {
    /// Exchanges one exact request for bounded encoded canonical events.
    fn exchange(
        &mut self,
        request: &[u8],
        cancellation: &ClientCancellation,
    ) -> Result<Vec<Vec<u8>>, ThinClientError>;
}

/// Local request replay and resume guard; it never grants operation authority.
#[derive(Debug, Default)]
pub struct ClientReplayGuard {
    requests: BTreeMap<String, String>,
    cursors: BTreeMap<String, (u64, String)>,
}

impl ClientReplayGuard {
    /// Admits one fresh request or an exact forward-only resume cursor.
    pub fn admit(&mut self, request: &ThinClientRequest) -> Result<(), ThinClientError> {
        match (&request.resume, self.requests.get(&request.request_id)) {
            (None, None) => {
                self.requests.insert(
                    request.request_id.clone(),
                    request.kernel_request_sha256.clone(),
                );
                Ok(())
            }
            (None, Some(_)) | (Some(_), None) => Err(ThinClientError::ReplayDenied),
            (Some(cursor), Some(kernel_sha256)) => {
                if kernel_sha256 != &request.kernel_request_sha256 {
                    return Err(ThinClientError::ReplayDenied);
                }
                if let Some((sequence, digest)) = self.cursors.get(&cursor.stream_id)
                    && (cursor.after_sequence < *sequence
                        || (cursor.after_sequence == *sequence
                            && cursor.last_event_sha256 != *digest))
                {
                    return Err(ThinClientError::ReplayDenied);
                }
                self.cursors.insert(
                    cursor.stream_id.clone(),
                    (cursor.after_sequence, cursor.last_event_sha256.clone()),
                );
                Ok(())
            }
        }
    }
}

/// Thin transport/display client shared by CLI, JSON, SDK, and ACP adapters.
pub struct ThinKernelClient<T> {
    surface: ClientSurface,
    transport: T,
    replay: ClientReplayGuard,
}

impl<T> ThinKernelClient<T>
where
    T: ThinClientTransport,
{
    /// Creates one inert thin client around an authenticated local transport.
    #[must_use]
    pub const fn new(surface: ClientSurface, transport: T) -> Self {
        Self {
            surface,
            transport,
            replay: ClientReplayGuard {
                requests: BTreeMap::new(),
                cursors: BTreeMap::new(),
            },
        }
    }

    /// Executes one exact request without direct storage, tool, model, connector, or secret access.
    pub fn execute(
        &mut self,
        request: &ThinClientRequest,
        now_epoch_ms: u64,
        cancellation: &ClientCancellation,
    ) -> Result<VerifiedClientStream, ThinClientError> {
        if request.surface != self.surface
            || request.cancellation_id != cancellation.cancellation_id()
        {
            return Err(ThinClientError::InvalidValue);
        }
        request.verify(now_epoch_ms)?;
        self.replay.admit(request)?;
        if cancellation.is_cancelled() {
            return Err(ThinClientError::Cancelled);
        }
        let encoded = serde_json::to_vec(request).map_err(|_| ThinClientError::Malformed)?;
        if encoded.len() > MAX_THIN_CLIENT_REQUEST_BYTES {
            return Err(ThinClientError::SizeExceeded);
        }
        let events = self.transport.exchange(&encoded, cancellation)?;
        if cancellation.is_cancelled() {
            return Err(ThinClientError::Cancelled);
        }
        verify_event_stream(request, &events)
    }

    /// Returns the transport after the client is no longer needed.
    #[must_use]
    pub fn into_transport(self) -> T {
        self.transport
    }
}

/// Encodes one request only after complete validation.
pub fn encode_thin_client_request(
    request: &ThinClientRequest,
    now_epoch_ms: u64,
) -> Result<Vec<u8>, ThinClientError> {
    request.verify(now_epoch_ms)?;
    let bytes = serde_json::to_vec(request).map_err(|_| ThinClientError::Malformed)?;
    if bytes.len() > MAX_THIN_CLIENT_REQUEST_BYTES {
        return Err(ThinClientError::SizeExceeded);
    }
    Ok(bytes)
}

/// Parses one request without deriving, minting, or widening authority.
pub fn parse_thin_client_request(
    bytes: &[u8],
    now_epoch_ms: u64,
) -> Result<ThinClientRequest, ThinClientError> {
    if bytes.is_empty() || bytes.len() > MAX_THIN_CLIENT_REQUEST_BYTES {
        return Err(ThinClientError::SizeExceeded);
    }
    let request: ThinClientRequest =
        serde_json::from_slice(bytes).map_err(|_| ThinClientError::Malformed)?;
    request.verify(now_epoch_ms)?;
    Ok(request)
}

fn canonical_sha256(value: &impl Serialize) -> Result<String, ThinClientError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| ThinClientError::Malformed)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn valid_code(value: &str) -> bool {
    valid_identifier(value) && value.contains('.')
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn valid_text(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty() && value.len() <= maximum && !value.contains('\0')
}

fn valid_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
        && (1..=12).contains(&((bytes[5] - b'0') * 10 + bytes[6] - b'0'))
        && (1..=31).contains(&((bytes[8] - b'0') * 10 + bytes[9] - b'0'))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    const NOW: u64 = 10_000;

    fn hash(character: char) -> String {
        character.to_string().repeat(64)
    }

    fn status() -> ClientStatusSnapshot {
        ClientStatusSnapshot {
            workspace_id: "workspace-0001".to_owned(),
            model_profile_id: Some("model-profile-0001".to_owned()),
            permission_profile_id: "permission-standard".to_owned(),
            conversation_id: Some("conversation-0001".to_owned()),
            plan_step_id: Some("plan-step-0001".to_owned()),
            writable_roots: vec!["workspace".to_owned()],
            offline: true,
            status_sha256: ZERO_SHA256.to_owned(),
        }
        .seal()
        .expect("status")
    }

    fn command() -> ClientCommand {
        ClientCommand::Conversations {
            action: ConversationClientCommand::Search {
                query: "exact local query".to_owned(),
            },
        }
    }

    fn request(surface: ClientSurface, request_id: &str) -> ThinClientRequest {
        let command = command();
        let status = status();
        let policy = hash('a');
        let operation = kernel_operation_sha256(&status.workspace_id, &command, &policy)
            .expect("kernel operation");
        ThinClientRequest {
            schema_version: 0,
            request_id: request_id.to_owned(),
            surface,
            status,
            command,
            authority: ClientAuthority::Predeclared {
                grant: PredeclaredClientGrant {
                    grant_id: "grant-0001".to_owned(),
                    operation: GrantOperation::DatabaseRead,
                    policy_sha256: policy.clone(),
                    arguments_sha256: operation,
                    nonce_sha256: hash('b'),
                    issued_at_epoch_ms: NOW - 1,
                    expires_at_epoch_ms: NOW + 1,
                    single_use: true,
                },
            },
            policy_sha256: policy,
            cancellation_id: format!("cancel-{request_id}"),
            max_event_bytes: MAX_THIN_CLIENT_EVENT_BYTES as u64,
            max_output_bytes: MAX_THIN_CLIENT_OUTPUT_BYTES,
            resume: None,
            kernel_request_sha256: ZERO_SHA256.to_owned(),
            request_sha256: ZERO_SHA256.to_owned(),
        }
        .seal(NOW)
        .expect("request")
    }

    fn event(
        request: &ThinClientRequest,
        sequence: u64,
        previous: &str,
        cumulative: u64,
        kind: ThinClientEventKind,
    ) -> ThinClientEvent {
        ThinClientEvent {
            schema_version: 0,
            request_id: request.request_id.clone(),
            stream_id: "stream-0001".to_owned(),
            sequence,
            kernel_request_sha256: request.kernel_request_sha256.clone(),
            policy_sha256: request.policy_sha256.clone(),
            cumulative_output_bytes: cumulative,
            kind,
            previous_event_sha256: previous.to_owned(),
            event_sha256: ZERO_SHA256.to_owned(),
        }
        .seal()
        .expect("event")
    }

    fn encoded_stream(request: &ThinClientRequest) -> Vec<Vec<u8>> {
        let first = event(request, 0, ZERO_SHA256, 1, ThinClientEventKind::Started);
        let first_bytes = serde_json::to_vec(&first).expect("first bytes");
        let receipt = event(
            request,
            1,
            &first.event_sha256,
            (first_bytes.len() + 1) as u64,
            ThinClientEventKind::Receipt {
                receipt_id: "receipt-0001".to_owned(),
                receipt_sha256: hash('c'),
                outcome: "succeeded".to_owned(),
            },
        );
        let receipt_bytes = serde_json::to_vec(&receipt).expect("receipt bytes");
        let completed = event(
            request,
            2,
            &receipt.event_sha256,
            (first_bytes.len() + receipt_bytes.len() + 1) as u64,
            ThinClientEventKind::Completed {
                final_state_sha256: hash('d'),
            },
        );
        // Cumulative bytes are encoded fields, so seal again after exact lengths stabilize.
        let mut total = 0_u64;
        let mut previous = ZERO_SHA256.to_owned();
        [first.kind, receipt.kind, completed.kind]
            .into_iter()
            .enumerate()
            .map(|(index, kind)| {
                let mut candidate_total = total + 512;
                loop {
                    let candidate = event(
                        request,
                        index as u64,
                        &previous,
                        candidate_total,
                        kind.clone(),
                    );
                    let bytes = serde_json::to_vec(&candidate).expect("event bytes");
                    let exact = total + bytes.len() as u64;
                    if exact == candidate_total {
                        previous = candidate.event_sha256;
                        total = exact;
                        break bytes;
                    }
                    candidate_total = exact;
                }
            })
            .collect()
    }

    #[derive(Default)]
    struct FixtureTransport {
        calls: usize,
    }

    impl ThinClientTransport for FixtureTransport {
        fn exchange(
            &mut self,
            request: &[u8],
            _cancellation: &ClientCancellation,
        ) -> Result<Vec<Vec<u8>>, ThinClientError> {
            self.calls += 1;
            let request = parse_thin_client_request(request, NOW)?;
            Ok(encoded_stream(&request))
        }
    }

    #[test]
    fn all_surfaces_share_one_kernel_operation_and_terminal_result() {
        let mut kernel_hashes = BTreeSet::new();
        let mut terminal_hashes = BTreeSet::new();
        for surface in [
            ClientSurface::NativeChat,
            ClientSurface::InteractiveCli,
            ClientSurface::Json,
            ClientSurface::Sdk,
            ClientSurface::Acp,
        ] {
            let request = request(surface, &format!("request-{surface:?}"));
            kernel_hashes.insert(request.kernel_request_sha256.clone());
            let cancellation =
                ClientCancellation::new(&request.cancellation_id).expect("cancellation");
            let mut client = ThinKernelClient::new(surface, FixtureTransport::default());
            let result = client
                .execute(&request, NOW, &cancellation)
                .expect("thin client result");
            assert_eq!(result.exit_code, ClientExitCode::Success);
            let ThinClientEventKind::Completed { final_state_sha256 } =
                &result.events.last().expect("terminal").kind
            else {
                panic!("expected completion");
            };
            terminal_hashes.insert(final_state_sha256.clone());
        }
        assert_eq!(kernel_hashes.len(), 1);
        assert_eq!(terminal_hashes.len(), 1);
    }

    #[test]
    fn headless_authority_is_exact_expiring_and_never_interactive() {
        let surfaces = [ClientSurface::Json, ClientSurface::Sdk, ClientSurface::Acp];
        for surface in surfaces {
            let exact = request(surface, "request-authority");
            assert!(exact.verify(NOW).is_ok());
            let mut interactive = exact.clone();
            interactive.authority = ClientAuthority::Interactive {
                approval_channel_sha256: hash('e'),
            };
            interactive.request_sha256 = ZERO_SHA256.to_owned();
            interactive.request_sha256 = canonical_sha256(&interactive).expect("digest");
            assert_eq!(
                interactive.verify(NOW),
                Err(ThinClientError::AuthorityDenied)
            );

            for mutation in ["operation", "policy", "arguments", "expiry", "reuse"] {
                let mut changed = exact.clone();
                let ClientAuthority::Predeclared { grant } = &mut changed.authority else {
                    unreachable!();
                };
                match mutation {
                    "operation" => grant.operation = GrantOperation::GitPush,
                    "policy" => grant.policy_sha256 = hash('f'),
                    "arguments" => grant.arguments_sha256 = hash('f'),
                    "expiry" => grant.expires_at_epoch_ms = NOW,
                    "reuse" => grant.single_use = false,
                    _ => unreachable!(),
                }
                changed.request_sha256 = ZERO_SHA256.to_owned();
                changed.request_sha256 = canonical_sha256(&changed).expect("digest");
                assert_eq!(changed.verify(NOW), Err(ThinClientError::AuthorityDenied));
            }
        }
    }

    #[test]
    fn malformed_oversized_reordered_partial_and_hidden_approval_streams_fail() {
        let request = request(ClientSurface::Json, "request-stream");
        let exact = encoded_stream(&request);
        assert!(verify_event_stream(&request, &exact).is_ok());

        let mut reordered = exact.clone();
        reordered.swap(1, 2);
        assert_eq!(
            verify_event_stream(&request, &reordered),
            Err(ThinClientError::Malformed)
        );
        assert_eq!(
            verify_event_stream(&request, &exact[..2]),
            Err(ThinClientError::Malformed)
        );
        let mut oversized = exact.clone();
        oversized[1] = vec![b'x'; MAX_THIN_CLIENT_EVENT_BYTES + 1];
        assert_eq!(
            verify_event_stream(&request, &oversized),
            Err(ThinClientError::SizeExceeded)
        );
        let approval = event(
            &request,
            1,
            ZERO_SHA256,
            1,
            ThinClientEventKind::ApprovalRequired {
                operation: GrantOperation::DatabaseRead,
                preview_sha256: hash('f'),
                expires_at_epoch_ms: NOW + 1,
            },
        );
        let approval = vec![
            exact[0].clone(),
            serde_json::to_vec(&approval).expect("approval"),
            exact[2].clone(),
        ];
        assert_eq!(
            verify_event_stream(&request, &approval),
            Err(ThinClientError::Malformed)
        );
    }

    #[test]
    fn replay_resume_cancellation_and_transport_failure_do_not_duplicate_launch() {
        let initial_request = request(ClientSurface::Json, "request-replay");
        let cancellation =
            ClientCancellation::new(&initial_request.cancellation_id).expect("cancellation");
        let mut client = ThinKernelClient::new(ClientSurface::Json, FixtureTransport::default());
        assert!(client.execute(&initial_request, NOW, &cancellation).is_ok());
        assert_eq!(
            client.execute(&initial_request, NOW, &cancellation),
            Err(ThinClientError::ReplayDenied)
        );
        assert_eq!(client.into_transport().calls, 1);

        let cancelled_request = request(ClientSurface::Json, "request-cancelled");
        let cancelled =
            ClientCancellation::new(&cancelled_request.cancellation_id).expect("cancellation");
        cancelled.cancel();
        let mut client = ThinKernelClient::new(ClientSurface::Json, FixtureTransport::default());
        assert_eq!(
            client.execute(&cancelled_request, NOW, &cancelled),
            Err(ThinClientError::Cancelled)
        );
        assert_eq!(client.into_transport().calls, 0);
    }

    #[test]
    fn closed_request_rejects_unknown_fields_and_every_command_family_is_bounded() {
        let exact = request(ClientSurface::Json, "request-closed");
        let bytes = encode_thin_client_request(&exact, NOW).expect("encoded request");
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON");
        value["direct_storage"] = serde_json::json!(true);
        assert_eq!(
            parse_thin_client_request(&serde_json::to_vec(&value).expect("mutated JSON"), NOW,),
            Err(ThinClientError::Malformed)
        );

        let commands = [
            ClientCommand::Chat {
                message: "hello".to_owned(),
            },
            ClientCommand::Conversations {
                action: ConversationClientCommand::Branch {
                    conversation_id: "conversation-0001".to_owned(),
                    turn_id: "turn-0001".to_owned(),
                },
            },
            ClientCommand::Vault {
                action: VaultClientCommand::Backlinks {
                    note_id: "note-0001".to_owned(),
                },
            },
            ClientCommand::Operations {
                action: OperationalClientCommand::MemoryCorrect {
                    memory_id: "memory-0001".to_owned(),
                    replacement: "corrected".to_owned(),
                },
            },
        ];
        assert!(commands.iter().all(ClientCommand::validate));
        assert_eq!(
            BTreeSet::from_iter(commands.iter().map(ClientCommand::required_operation)),
            BTreeSet::from([
                GrantOperation::WorkspaceRead,
                GrantOperation::WorkspaceWrite,
                GrantOperation::ModelInference,
            ])
        );
    }
}
