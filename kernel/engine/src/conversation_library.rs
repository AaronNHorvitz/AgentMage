//! Canonical encrypted conversation storage behind the kernel boundary.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use agentmage_kernel_contracts::{
    ApprovalId, CONTRACT_SCHEMA_VERSION, ConversationCompactionId, ConversationCompactionRecord,
    ConversationId, ConversationRecord, ConversationRetention, ConversationRetentionKind,
    ConversationStatus, ConversationTurn, ConversationTurnId, DataSensitivity, SessionCheckpoint,
    SessionCheckpointId, WorkspaceId, from_json, to_canonical_json,
};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use sha2::{Digest, Sha256};

use agentmage_kernel_contracts::ResumeDriftDimension;

use crate::context_management::{ResumeDirective, ResumeObservation, revalidate_resume};
use crate::operational_store::OperationalStore;

const MAX_TITLE_BYTES: usize = 512;
const MAX_TIMEZONE_BYTES: usize = 128;
const MAX_PROJECT_BYTES: usize = 256;
const MAX_TAGS: usize = 128;
const MAX_TAG_BYTES: usize = 128;
const MAX_TURN_TEXT_BYTES: usize = 1024 * 1024;
const MAX_REFERENCES: usize = 512;
const MAX_REFERENCE_ID_BYTES: usize = 256;
const MAX_ATTACHMENT_NAME_BYTES: usize = 512;
const MAX_MEDIA_TYPE_BYTES: usize = 256;
const MAX_QUERY_BYTES: usize = 512;
const MAX_QUERY_RESULTS: u32 = 1_000;
const MAX_CONVERSATION_SCAN: usize = 100_000;
const MAX_RESULT_PREVIEW_CHARS: usize = 256;

/// Stable content-free conversation storage failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConversationLibraryError {
    /// A contract value is malformed, out of bounds, or internally inconsistent.
    InvalidInput,
    /// The stable identity already exists.
    DuplicateIdentity,
    /// The requested conversation or turn does not exist.
    NotFound,
    /// The mutation conflicts with the current immutable timeline head.
    Conflict,
    /// Exact text was supplied when conversation persistence was disabled.
    PersistenceDisabled,
    /// SQLite rejected an atomic storage operation.
    StorageFailed,
    /// A canonical row or normalized projection failed hash or shape verification.
    IntegrityFailure,
}

impl ConversationLibraryError {
    /// Returns a stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "conversation.input.invalid",
            Self::DuplicateIdentity => "conversation.identity.duplicate",
            Self::NotFound => "conversation.identity.not_found",
            Self::Conflict => "conversation.timeline.conflict",
            Self::PersistenceDisabled => "conversation.persistence.disabled",
            Self::StorageFailed => "conversation.storage.failed",
            Self::IntegrityFailure => "conversation.integrity.failed",
        }
    }
}

impl std::fmt::Display for ConversationLibraryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for ConversationLibraryError {}

/// Content-free result of one atomic canonical conversation mutation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversationMutationReceipt {
    /// Stable conversation identity.
    pub conversation_id: ConversationId,
    /// Immutable turn identity when a turn was appended.
    pub turn_id: Option<ConversationTurnId>,
    /// Current one-based turn ordinal after the mutation.
    pub current_ordinal: u64,
    /// Digest of the current canonical conversation record.
    pub conversation_sha256: String,
    /// Digest of the appended canonical turn, when present.
    pub turn_sha256: Option<String>,
    /// Fixed true marker: this API writes only through the encrypted canonical store.
    pub encrypted_canonical_store: bool,
}

/// Closed bounded local conversation query.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ConversationQuery {
    /// Inclusive local start date in `YYYY-MM-DD` form.
    pub from_local_date: Option<String>,
    /// Inclusive local end date in `YYYY-MM-DD` form.
    pub to_local_date: Option<String>,
    /// Case-insensitive literal matched against title and retained turn metadata/content.
    pub text: Option<String>,
    /// Exact workspace identity.
    pub workspace_id: Option<WorkspaceId>,
    /// Exact project identity.
    pub project_id: Option<String>,
    /// Exact model profile identity.
    pub model_profile_id: Option<agentmage_kernel_contracts::ModelProfileId>,
    /// Exact lifecycle state.
    pub status: Option<ConversationStatus>,
    /// Exact tag.
    pub tag: Option<String>,
    /// Optional pinned-state filter.
    pub pinned: Option<bool>,
    /// Whether archived records may appear without an explicit archived status filter.
    pub include_archived: bool,
    /// Maximum result count.
    pub limit: u32,
}

/// Bounded content-minimized conversation search result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversationSearchHit {
    /// Stable conversation identity.
    pub conversation_id: ConversationId,
    /// Current title.
    pub title: String,
    /// Current local date.
    pub local_date: String,
    /// Exact workspace identity.
    pub workspace_id: WorkspaceId,
    /// Optional project identity.
    pub project_id: Option<String>,
    /// Exact model profile identity.
    pub model_profile_id: agentmage_kernel_contracts::ModelProfileId,
    /// Current lifecycle state.
    pub status: ConversationStatus,
    /// Current pinned state.
    pub pinned: bool,
    /// Number of immutable turns.
    pub turn_count: u64,
    /// Bounded matching preview, or none when no text filter was requested.
    pub matching_preview: Option<String>,
}

/// Complete read-only immutable timeline for one conversation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversationHistory {
    /// Hash-verified canonical conversation metadata.
    pub conversation: ConversationRecord,
    /// Hash-verified turns in strict ordinal order.
    pub turns: Vec<ConversationTurn>,
    /// Checked append-only compactions in creation order.
    pub compactions: Vec<ConversationCompactionRecord>,
    /// Fixed marker showing this view cannot mutate canonical state.
    pub read_only: bool,
}

/// Local parent and child relationships without graph-database authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversationRelationships {
    /// Selected conversation identity.
    pub conversation_id: ConversationId,
    /// Original parent conversation, when selected record is a branch.
    pub parent_conversation_id: Option<ConversationId>,
    /// Exact parent turn, when selected record is a branch.
    pub branch_from_turn_id: Option<ConversationTurnId>,
    /// Direct child branches in stable identity order.
    pub child_conversation_ids: Vec<ConversationId>,
}

/// Closed user-visible metadata change class.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConversationMetadataChange {
    /// Replace the bounded title.
    Rename(String),
    /// Set the exact pinned state.
    SetPinned(bool),
    /// Move the conversation to the archived lifecycle state.
    Archive,
    /// Replace all tags with one stable sorted set.
    ReplaceTags(Vec<String>),
    /// Replace the visible retention rule.
    SetRetention(ConversationRetention),
}

/// Closed content-free metadata change identity retained in previews and receipts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConversationMetadataChangeKind {
    /// Title replacement.
    Rename,
    /// Pin-state replacement.
    SetPinned,
    /// Archive transition.
    Archive,
    /// Tag-set replacement.
    ReplaceTags,
    /// Retention-policy replacement.
    SetRetention,
}

/// Exact compare-and-swap preview for one local conversation metadata change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversationChangePreview {
    /// Stable conversation identity.
    pub conversation_id: ConversationId,
    /// Digest of the exact current canonical record.
    pub expected_record_sha256: String,
    /// Closed change class.
    pub change_kind: ConversationMetadataChangeKind,
    /// Complete proposed replacement shown to the user.
    pub proposed_record: ConversationRecord,
    /// Digest of the complete proposed canonical record.
    pub proposed_record_sha256: String,
    /// Digest binding identity, current state, proposed state, and change class.
    pub preview_sha256: String,
    /// Fixed false marker: constructing a preview performs no write.
    pub applied: bool,
}

/// Exact deletion preview containing only identities, hashes, and record counts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversationDeletionPreview {
    /// Stable conversation identity.
    pub conversation_id: ConversationId,
    /// Digest of the exact current canonical conversation record.
    pub expected_record_sha256: String,
    /// Number of immutable turns that would be deleted.
    pub turn_count: u64,
    /// Number of append-only checked compactions that would be deleted.
    pub compaction_count: u64,
    /// Number of attachment references that would be deleted.
    pub attachment_reference_count: u64,
    /// Number of grant, receipt, checkpoint, citation, and source references deleted.
    pub evidence_reference_count: u64,
    /// Direct child branches that prevent deletion until separately handled.
    pub blocking_child_conversation_ids: Vec<ConversationId>,
    /// Digest of the complete preview.
    pub preview_sha256: String,
    /// Fixed false marker: constructing a preview performs no write.
    pub applied: bool,
}

/// Explicit user approval bound to one exact deletion preview.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversationDeletionApproval {
    /// Stable approval identity produced by the guarded approval boundary.
    pub approval_id: ApprovalId,
    /// Exact deletion preview digest approved by the user.
    pub approved_preview_sha256: String,
    /// Digest of the explicit user decision evidence.
    pub decision_sha256: String,
    /// Exact trusted approval time as Unix epoch milliseconds.
    pub approved_at_epoch_ms: u64,
    /// Explicit confirmation marker; false approvals are inert.
    pub user_confirmed: bool,
}

/// Content-free result of one approved canonical conversation deletion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversationDeletionReceipt {
    /// Digest of the deleted stable conversation identity.
    pub conversation_id_sha256: String,
    /// Exact approved preview digest.
    pub preview_sha256: String,
    /// Digest of the approval identity.
    pub approval_id_sha256: String,
    /// Number of immutable turns deleted.
    pub deleted_turn_count: u64,
    /// Number of checked compactions deleted.
    pub deleted_compaction_count: u64,
    /// Fixed true marker after one atomic committed deletion.
    pub deleted: bool,
}

/// Read-only checkpoint and drift result for resuming one exact conversation turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversationResumeReview {
    /// Conversation selected for continuation.
    pub conversation_id: ConversationId,
    /// Exact historical turn owning the selected checkpoint.
    pub turn_id: ConversationTurnId,
    /// Hash-verified metadata-only checkpoint.
    pub checkpoint: SessionCheckpoint,
    /// Current-versus-recorded drift result from the shared resume engine.
    pub directive: ResumeDirective,
    /// Digest of the immutable source history prefix through the selected turn.
    pub source_history_sha256: String,
    /// Fixed false marker: review never resumes work or creates a branch.
    pub resumed: bool,
}

/// Exact no-write preview for a visible branch from one historical turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversationBranchPreview {
    /// Source checkpoint and current drift result.
    pub resume_review: ConversationResumeReview,
    /// Complete proposed empty child conversation.
    pub proposed_conversation: ConversationRecord,
    /// Digest of the proposed canonical child record.
    pub proposed_record_sha256: String,
    /// Digest binding source history, checkpoint, drift state, and proposed child.
    pub preview_sha256: String,
    /// Fixed false marker: constructing a branch preview performs no write.
    pub applied: bool,
}

/// Content-free result of storing one checked append-only compaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversationCompactionReceipt {
    /// Stable conversation identity.
    pub conversation_id: ConversationId,
    /// Stable compaction identity.
    pub compaction_id: ConversationCompactionId,
    /// Digest of the canonical compaction record.
    pub record_sha256: String,
    /// Digest of the unchanged source history prefix.
    pub source_history_sha256: String,
    /// Fixed true marker after atomic canonical storage.
    pub stored: bool,
    /// Fixed false marker: source turns are never rewritten by compaction.
    pub source_turns_rewritten: bool,
}

impl OperationalStore {
    /// Creates one canonical conversation with no turns in the encrypted store.
    pub fn create_conversation(
        &mut self,
        conversation: &ConversationRecord,
    ) -> Result<ConversationMutationReceipt, ConversationLibraryError> {
        validate_conversation(conversation, true)?;
        if let (Some(parent_id), Some(branch_turn_id)) = (
            &conversation.parent_conversation_id,
            &conversation.branch_from_turn_id,
        ) {
            let branch_turn = self
                .conversation_turn(branch_turn_id)?
                .ok_or(ConversationLibraryError::NotFound)?;
            if &branch_turn.conversation_id != parent_id {
                return Err(ConversationLibraryError::Conflict);
            }
        }
        let record_json =
            to_canonical_json(conversation).map_err(|_| ConversationLibraryError::InvalidInput)?;
        let record_sha256 = sha256(&record_json);
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
        let inserted = transaction
            .execute(
                "INSERT OR IGNORE INTO conversations(
                    conversation_id, title, sensitivity, created_at_epoch_ms,
                    updated_at_epoch_ms, local_date, local_timezone, workspace_id,
                    project_id, model_profile_id, status, parent_conversation_id,
                    branch_from_turn_id, current_turn_id, retention_kind,
                    retention_expires_at_epoch_ms, retention_policy_sha256, pinned,
                    persistence_enabled, record_sha256, record_json
                 ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                    ?13, NULL, ?14, ?15, ?16, ?17, ?18, ?19, ?20
                 )",
                params![
                    conversation.conversation_id.as_str(),
                    &conversation.title,
                    sensitivity_code(conversation.sensitivity),
                    as_sql_integer(conversation.created_at_epoch_ms)?,
                    as_sql_integer(conversation.updated_at_epoch_ms)?,
                    &conversation.local_date,
                    &conversation.local_timezone,
                    conversation.workspace_id.as_str(),
                    conversation.project_id.as_deref(),
                    conversation.model_profile_id.as_str(),
                    status_code(conversation.status),
                    conversation
                        .parent_conversation_id
                        .as_ref()
                        .map(ConversationId::as_str),
                    conversation
                        .branch_from_turn_id
                        .as_ref()
                        .map(ConversationTurnId::as_str),
                    retention_code(conversation.retention.kind),
                    conversation
                        .retention
                        .expires_at_epoch_ms
                        .map(as_sql_integer)
                        .transpose()?,
                    &conversation.retention.policy_sha256,
                    i64::from(conversation.pinned),
                    i64::from(conversation.persistence_enabled),
                    &record_sha256,
                    &record_json,
                ],
            )
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
        if inserted != 1 {
            return Err(ConversationLibraryError::DuplicateIdentity);
        }
        for tag in &conversation.tags {
            transaction
                .execute(
                    "INSERT INTO conversation_tags(conversation_id, tag) VALUES (?1, ?2)",
                    params![conversation.conversation_id.as_str(), tag],
                )
                .map_err(|_| ConversationLibraryError::StorageFailed)?;
        }
        transaction
            .commit()
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
        Ok(ConversationMutationReceipt {
            conversation_id: conversation.conversation_id.clone(),
            turn_id: None,
            current_ordinal: 0,
            conversation_sha256: record_sha256,
            turn_sha256: None,
            encrypted_canonical_store: true,
        })
    }

    /// Appends one immutable turn and atomically advances the conversation head.
    pub fn append_conversation_turn(
        &mut self,
        turn: &ConversationTurn,
    ) -> Result<ConversationMutationReceipt, ConversationLibraryError> {
        validate_turn(turn)?;
        let mut conversation = self
            .conversation(&turn.conversation_id)?
            .ok_or(ConversationLibraryError::NotFound)?;
        if conversation.status != ConversationStatus::Active {
            return Err(ConversationLibraryError::Conflict);
        }
        if !conversation.persistence_enabled && turn.text.is_some() {
            return Err(ConversationLibraryError::PersistenceDisabled);
        }
        if turn.created_at_epoch_ms < conversation.updated_at_epoch_ms
            || turn.local_date < conversation.local_date
        {
            return Err(ConversationLibraryError::Conflict);
        }
        let expected_ordinal = self.current_turn_ordinal(&turn.conversation_id)? + 1;
        if turn.ordinal != expected_ordinal {
            return Err(ConversationLibraryError::Conflict);
        }
        let turn_json =
            to_canonical_json(turn).map_err(|_| ConversationLibraryError::InvalidInput)?;
        let turn_sha256 = sha256(&turn_json);
        conversation.current_turn_id = Some(turn.turn_id.clone());
        conversation.updated_at_epoch_ms = turn.created_at_epoch_ms;
        conversation.local_date.clone_from(&turn.local_date);
        conversation.sensitivity =
            strictest_sensitivity(conversation.sensitivity, turn.sensitivity);
        let conversation_json =
            to_canonical_json(&conversation).map_err(|_| ConversationLibraryError::InvalidInput)?;
        let conversation_sha256 = sha256(&conversation_json);

        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
        transaction
            .execute(
                "INSERT INTO conversation_turns(
                    turn_id, conversation_id, ordinal, role, sensitivity,
                    created_at_epoch_ms, local_date, text_sha256, record_sha256, record_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    turn.turn_id.as_str(),
                    turn.conversation_id.as_str(),
                    as_sql_integer(turn.ordinal)?,
                    role_code(turn.role),
                    sensitivity_code(turn.sensitivity),
                    as_sql_integer(turn.created_at_epoch_ms)?,
                    &turn.local_date,
                    &turn.text_sha256,
                    &turn_sha256,
                    &turn_json,
                ],
            )
            .map_err(classify_insert_error)?;
        insert_turn_references(&transaction, turn)?;
        let advanced = transaction
            .execute(
                "UPDATE conversations
                 SET updated_at_epoch_ms=?1, local_date=?2, sensitivity=?3,
                     current_turn_id=?4, record_sha256=?5, record_json=?6
                 WHERE conversation_id=?7 AND current_turn_id IS ?8",
                params![
                    as_sql_integer(conversation.updated_at_epoch_ms)?,
                    &conversation.local_date,
                    sensitivity_code(conversation.sensitivity),
                    turn.turn_id.as_str(),
                    &conversation_sha256,
                    &conversation_json,
                    turn.conversation_id.as_str(),
                    conversation_head_before(turn.ordinal, &transaction, &turn.conversation_id)?,
                ],
            )
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
        if advanced != 1 {
            return Err(ConversationLibraryError::Conflict);
        }
        transaction
            .commit()
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
        Ok(ConversationMutationReceipt {
            conversation_id: turn.conversation_id.clone(),
            turn_id: Some(turn.turn_id.clone()),
            current_ordinal: turn.ordinal,
            conversation_sha256,
            turn_sha256: Some(turn_sha256),
            encrypted_canonical_store: true,
        })
    }

    /// Loads and hash-verifies one canonical conversation.
    pub fn conversation(
        &self,
        conversation_id: &ConversationId,
    ) -> Result<Option<ConversationRecord>, ConversationLibraryError> {
        let row = self
            .connection
            .query_row(
                "SELECT record_json, record_sha256 FROM conversations WHERE conversation_id=?1",
                [conversation_id.as_str()],
                |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
        let Some((bytes, expected_sha256)) = row else {
            return Ok(None);
        };
        if sha256(&bytes) != expected_sha256 {
            return Err(ConversationLibraryError::IntegrityFailure);
        }
        let record: ConversationRecord =
            from_json(&bytes).map_err(|_| ConversationLibraryError::IntegrityFailure)?;
        validate_conversation(&record, false)
            .map_err(|_| ConversationLibraryError::IntegrityFailure)?;
        if &record.conversation_id != conversation_id {
            return Err(ConversationLibraryError::IntegrityFailure);
        }
        verify_conversation_projection(&self.connection, &record)?;
        Ok(Some(record))
    }

    /// Loads and hash-verifies one immutable canonical turn.
    pub fn conversation_turn(
        &self,
        turn_id: &ConversationTurnId,
    ) -> Result<Option<ConversationTurn>, ConversationLibraryError> {
        let row = self
            .connection
            .query_row(
                "SELECT record_json, record_sha256 FROM conversation_turns WHERE turn_id=?1",
                [turn_id.as_str()],
                |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
        let Some((bytes, expected_sha256)) = row else {
            return Ok(None);
        };
        if sha256(&bytes) != expected_sha256 {
            return Err(ConversationLibraryError::IntegrityFailure);
        }
        let turn: ConversationTurn =
            from_json(&bytes).map_err(|_| ConversationLibraryError::IntegrityFailure)?;
        validate_turn(&turn).map_err(|_| ConversationLibraryError::IntegrityFailure)?;
        if &turn.turn_id != turn_id {
            return Err(ConversationLibraryError::IntegrityFailure);
        }
        verify_turn_projection(&self.connection, &turn)?;
        Ok(Some(turn))
    }

    /// Loads and verifies one append-only checked compaction against original turns.
    pub fn conversation_compaction(
        &self,
        compaction_id: &ConversationCompactionId,
    ) -> Result<Option<ConversationCompactionRecord>, ConversationLibraryError> {
        let row = self
            .connection
            .query_row(
                "SELECT record_sha256, record_json FROM conversation_compactions
                 WHERE compaction_id=?1",
                [compaction_id.as_str()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?)),
            )
            .optional()
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
        let Some((expected_sha256, bytes)) = row else {
            return Ok(None);
        };
        if sha256(&bytes) != expected_sha256 {
            return Err(ConversationLibraryError::IntegrityFailure);
        }
        let record: ConversationCompactionRecord =
            from_json(&bytes).map_err(|_| ConversationLibraryError::IntegrityFailure)?;
        if &record.compaction_id != compaction_id {
            return Err(ConversationLibraryError::IntegrityFailure);
        }
        self.validate_compaction_against_source(&record)
            .map_err(|_| ConversationLibraryError::IntegrityFailure)?;
        Ok(Some(record))
    }

    /// Stores one checked compaction without replacing or deleting original turns.
    pub fn append_conversation_compaction(
        &mut self,
        record: &ConversationCompactionRecord,
    ) -> Result<ConversationCompactionReceipt, ConversationLibraryError> {
        self.validate_compaction_against_source(record)?;
        let record_json =
            to_canonical_json(record).map_err(|_| ConversationLibraryError::InvalidInput)?;
        let record_sha256 = sha256(&record_json);
        let through_turn = self
            .conversation_turn(&record.through_turn_id)?
            .ok_or(ConversationLibraryError::NotFound)?;
        let source_history_sha256 =
            self.source_history_sha256(&record.conversation_id, through_turn.ordinal)?;
        let inserted = self
            .connection
            .execute(
                "INSERT OR IGNORE INTO conversation_compactions(
                    compaction_id, conversation_id, through_turn_id, summary_id,
                    created_at_epoch_ms, source_hash_set_sha256, record_sha256, record_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    record.compaction_id.as_str(),
                    record.conversation_id.as_str(),
                    record.through_turn_id.as_str(),
                    record.summary.summary_id.as_str(),
                    as_sql_integer(record.created_at_epoch_ms)?,
                    &record.source_hash_set_sha256,
                    &record_sha256,
                    &record_json,
                ],
            )
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
        if inserted != 1 {
            return Err(ConversationLibraryError::DuplicateIdentity);
        }
        Ok(ConversationCompactionReceipt {
            conversation_id: record.conversation_id.clone(),
            compaction_id: record.compaction_id.clone(),
            record_sha256,
            source_history_sha256,
            stored: true,
            source_turns_rewritten: false,
        })
    }

    /// Searches canonical local conversations using bounded exact filters and literal text.
    pub fn search_conversations(
        &self,
        query: &ConversationQuery,
    ) -> Result<Vec<ConversationSearchHit>, ConversationLibraryError> {
        validate_query(query)?;
        let identities = self
            .connection
            .prepare(
                "SELECT conversation_id FROM conversations
                 ORDER BY pinned DESC, updated_at_epoch_ms DESC, conversation_id ASC",
            )
            .and_then(|mut statement| {
                statement
                    .query_map([], |row| row.get::<_, String>(0))?
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
        if identities.len() > MAX_CONVERSATION_SCAN {
            return Err(ConversationLibraryError::IntegrityFailure);
        }
        let normalized_text = query.text.as_ref().map(|value| value.to_lowercase());
        let mut results = Vec::new();
        for identity in identities {
            let conversation_id = ConversationId::from_raw(identity);
            let conversation = self
                .conversation(&conversation_id)?
                .ok_or(ConversationLibraryError::IntegrityFailure)?;
            if !conversation_matches(&conversation, query) {
                continue;
            }
            let history = self.conversation_history(&conversation_id)?;
            let matching_preview = match &normalized_text {
                Some(text) => {
                    search_preview(&conversation, &history.turns, &history.compactions, text)
                }
                None => None,
            };
            if normalized_text.is_some() && matching_preview.is_none() {
                continue;
            }
            results.push(ConversationSearchHit {
                conversation_id: conversation.conversation_id,
                title: conversation.title,
                local_date: conversation.local_date,
                workspace_id: conversation.workspace_id,
                project_id: conversation.project_id,
                model_profile_id: conversation.model_profile_id,
                status: conversation.status,
                pinned: conversation.pinned,
                turn_count: history.turns.len() as u64,
                matching_preview,
            });
            if results.len() == query.limit as usize {
                break;
            }
        }
        Ok(results)
    }

    /// Loads one complete hash-verified conversation in read-only timeline order.
    pub fn conversation_history(
        &self,
        conversation_id: &ConversationId,
    ) -> Result<ConversationHistory, ConversationLibraryError> {
        let conversation = self
            .conversation(conversation_id)?
            .ok_or(ConversationLibraryError::NotFound)?;
        let turn_ids = query_strings(
            &self.connection,
            "SELECT turn_id FROM conversation_turns
             WHERE conversation_id=?1 ORDER BY ordinal",
            conversation_id.as_str(),
        )?;
        let mut turns = Vec::with_capacity(turn_ids.len());
        for (index, turn_id) in turn_ids.into_iter().enumerate() {
            let turn_id = ConversationTurnId::from_raw(turn_id);
            let turn = self
                .conversation_turn(&turn_id)?
                .ok_or(ConversationLibraryError::IntegrityFailure)?;
            if turn.conversation_id != *conversation_id || turn.ordinal != index as u64 + 1 {
                return Err(ConversationLibraryError::IntegrityFailure);
            }
            turns.push(turn);
        }
        let expected_head = turns.last().map(|turn| turn.turn_id.clone());
        if conversation.current_turn_id != expected_head {
            return Err(ConversationLibraryError::IntegrityFailure);
        }
        let compaction_ids = query_strings(
            &self.connection,
            "SELECT compaction_id FROM conversation_compactions
             WHERE conversation_id=?1 ORDER BY created_at_epoch_ms, compaction_id",
            conversation_id.as_str(),
        )?;
        let mut compactions = Vec::with_capacity(compaction_ids.len());
        for compaction_id in compaction_ids {
            compactions.push(
                self.conversation_compaction(&ConversationCompactionId::from_raw(compaction_id))?
                    .ok_or(ConversationLibraryError::IntegrityFailure)?,
            );
        }
        Ok(ConversationHistory {
            conversation,
            turns,
            compactions,
            read_only: true,
        })
    }

    /// Returns verified direct parent and child branch identities.
    pub fn conversation_relationships(
        &self,
        conversation_id: &ConversationId,
    ) -> Result<ConversationRelationships, ConversationLibraryError> {
        let conversation = self
            .conversation(conversation_id)?
            .ok_or(ConversationLibraryError::NotFound)?;
        let children = query_strings(
            &self.connection,
            "SELECT conversation_id FROM conversations
             WHERE parent_conversation_id=?1 ORDER BY conversation_id",
            conversation_id.as_str(),
        )?
        .into_iter()
        .map(ConversationId::from_raw)
        .collect::<Vec<_>>();
        for child_id in &children {
            let child = self
                .conversation(child_id)?
                .ok_or(ConversationLibraryError::IntegrityFailure)?;
            if child.parent_conversation_id.as_ref() != Some(conversation_id) {
                return Err(ConversationLibraryError::IntegrityFailure);
            }
        }
        Ok(ConversationRelationships {
            conversation_id: conversation.conversation_id,
            parent_conversation_id: conversation.parent_conversation_id,
            branch_from_turn_id: conversation.branch_from_turn_id,
            child_conversation_ids: children,
        })
    }

    /// Reviews the latest safe checkpoint before resuming a conversation in place.
    pub fn review_latest_conversation_resume(
        &self,
        conversation_id: &ConversationId,
        current: &ResumeObservation,
    ) -> Result<ConversationResumeReview, ConversationLibraryError> {
        let turn_id = self
            .connection
            .query_row(
                "SELECT turn.turn_id FROM conversation_turns AS turn
                 JOIN conversation_turn_checkpoints AS checkpoint
                   ON checkpoint.turn_id=turn.turn_id
                 WHERE turn.conversation_id=?1
                 ORDER BY turn.ordinal DESC LIMIT 1",
                [conversation_id.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| ConversationLibraryError::StorageFailed)?
            .ok_or(ConversationLibraryError::NotFound)?;
        self.review_conversation_resume_from_turn(
            conversation_id,
            &ConversationTurnId::from_raw(turn_id),
            current,
        )
    }

    /// Reviews one exact historical turn checkpoint without changing the original timeline.
    pub fn review_conversation_resume_from_turn(
        &self,
        conversation_id: &ConversationId,
        turn_id: &ConversationTurnId,
        current: &ResumeObservation,
    ) -> Result<ConversationResumeReview, ConversationLibraryError> {
        let turn = self
            .conversation_turn(turn_id)?
            .ok_or(ConversationLibraryError::NotFound)?;
        if &turn.conversation_id != conversation_id {
            return Err(ConversationLibraryError::Conflict);
        }
        let checkpoint_id = turn
            .checkpoint_id
            .as_ref()
            .ok_or(ConversationLibraryError::NotFound)?;
        let checkpoint = self.load_conversation_checkpoint(checkpoint_id)?;
        let directive = revalidate_resume(&checkpoint, current)
            .map_err(|_| ConversationLibraryError::IntegrityFailure)?;
        Ok(ConversationResumeReview {
            conversation_id: conversation_id.clone(),
            turn_id: turn_id.clone(),
            checkpoint,
            directive,
            source_history_sha256: self.source_history_sha256(conversation_id, turn.ordinal)?,
            resumed: false,
        })
    }

    /// Constructs a no-write branch preview from one exact historical turn.
    pub fn preview_conversation_branch(
        &self,
        source_conversation_id: &ConversationId,
        source_turn_id: &ConversationTurnId,
        proposed_conversation: ConversationRecord,
        current: &ResumeObservation,
    ) -> Result<ConversationBranchPreview, ConversationLibraryError> {
        if proposed_conversation.parent_conversation_id.as_ref() != Some(source_conversation_id)
            || proposed_conversation.branch_from_turn_id.as_ref() != Some(source_turn_id)
            || proposed_conversation.current_turn_id.is_some()
            || proposed_conversation.status != ConversationStatus::Active
        {
            return Err(ConversationLibraryError::InvalidInput);
        }
        validate_conversation(&proposed_conversation, true)?;
        if self
            .conversation(&proposed_conversation.conversation_id)?
            .is_some()
        {
            return Err(ConversationLibraryError::DuplicateIdentity);
        }
        let resume_review = self.review_conversation_resume_from_turn(
            source_conversation_id,
            source_turn_id,
            current,
        )?;
        let proposed_record_sha256 = canonical_record_sha256(&proposed_conversation)?;
        let preview_sha256 = branch_preview_digest(&resume_review, &proposed_record_sha256);
        Ok(ConversationBranchPreview {
            resume_review,
            proposed_conversation,
            proposed_record_sha256,
            preview_sha256,
            applied: false,
        })
    }

    /// Creates one visible child branch only when the preview and current state remain exact.
    pub fn apply_conversation_branch(
        &mut self,
        preview: &ConversationBranchPreview,
        current: &ResumeObservation,
    ) -> Result<ConversationMutationReceipt, ConversationLibraryError> {
        validate_branch_preview(preview)?;
        let fresh = self.preview_conversation_branch(
            &preview.resume_review.conversation_id,
            &preview.resume_review.turn_id,
            preview.proposed_conversation.clone(),
            current,
        )?;
        if &fresh != preview || preview.resume_review.directive != ResumeDirective::Continue {
            return Err(ConversationLibraryError::Conflict);
        }
        self.create_conversation(&preview.proposed_conversation)
    }

    /// Constructs an exact metadata change preview without writing canonical state.
    pub fn preview_conversation_change(
        &self,
        conversation_id: &ConversationId,
        change: ConversationMetadataChange,
        changed_at_epoch_ms: u64,
        changed_local_date: String,
    ) -> Result<ConversationChangePreview, ConversationLibraryError> {
        let current = self
            .conversation(conversation_id)?
            .ok_or(ConversationLibraryError::NotFound)?;
        if changed_at_epoch_ms < current.updated_at_epoch_ms
            || !valid_local_date(&changed_local_date)
            || changed_local_date < current.local_date
        {
            return Err(ConversationLibraryError::Conflict);
        }
        let expected_record_sha256 = canonical_record_sha256(&current)?;
        let change_kind = metadata_change_kind(&change);
        let mut proposed = current;
        match change {
            ConversationMetadataChange::Rename(title) => proposed.title = title,
            ConversationMetadataChange::SetPinned(pinned) => proposed.pinned = pinned,
            ConversationMetadataChange::Archive => proposed.status = ConversationStatus::Archived,
            ConversationMetadataChange::ReplaceTags(tags) => proposed.tags = tags,
            ConversationMetadataChange::SetRetention(retention) => {
                proposed.retention = retention;
            }
        }
        proposed.updated_at_epoch_ms = changed_at_epoch_ms;
        proposed.local_date = changed_local_date;
        validate_conversation(&proposed, false)?;
        let proposed_record_sha256 = canonical_record_sha256(&proposed)?;
        let preview_sha256 = change_preview_digest(
            conversation_id,
            &expected_record_sha256,
            &proposed_record_sha256,
            change_kind,
        );
        Ok(ConversationChangePreview {
            conversation_id: conversation_id.clone(),
            expected_record_sha256,
            change_kind,
            proposed_record: proposed,
            proposed_record_sha256,
            preview_sha256,
            applied: false,
        })
    }

    /// Applies one unchanged compare-and-swap metadata preview atomically.
    pub fn apply_conversation_change(
        &mut self,
        preview: &ConversationChangePreview,
    ) -> Result<ConversationMutationReceipt, ConversationLibraryError> {
        validate_change_preview(preview)?;
        let current = self
            .conversation(&preview.conversation_id)?
            .ok_or(ConversationLibraryError::NotFound)?;
        if canonical_record_sha256(&current)? != preview.expected_record_sha256
            || !same_immutable_conversation_fields(&current, &preview.proposed_record)
        {
            return Err(ConversationLibraryError::Conflict);
        }
        let proposed_json = to_canonical_json(&preview.proposed_record)
            .map_err(|_| ConversationLibraryError::InvalidInput)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
        let changed = transaction
            .execute(
                "UPDATE conversations SET
                    title=?1, sensitivity=?2, updated_at_epoch_ms=?3, local_date=?4,
                    model_profile_id=?5, status=?6, retention_kind=?7,
                    retention_expires_at_epoch_ms=?8, retention_policy_sha256=?9,
                    pinned=?10, persistence_enabled=?11, record_sha256=?12, record_json=?13
                 WHERE conversation_id=?14 AND record_sha256=?15",
                params![
                    &preview.proposed_record.title,
                    sensitivity_code(preview.proposed_record.sensitivity),
                    as_sql_integer(preview.proposed_record.updated_at_epoch_ms)?,
                    &preview.proposed_record.local_date,
                    preview.proposed_record.model_profile_id.as_str(),
                    status_code(preview.proposed_record.status),
                    retention_code(preview.proposed_record.retention.kind),
                    preview
                        .proposed_record
                        .retention
                        .expires_at_epoch_ms
                        .map(as_sql_integer)
                        .transpose()?,
                    &preview.proposed_record.retention.policy_sha256,
                    i64::from(preview.proposed_record.pinned),
                    i64::from(preview.proposed_record.persistence_enabled),
                    &preview.proposed_record_sha256,
                    &proposed_json,
                    preview.conversation_id.as_str(),
                    &preview.expected_record_sha256,
                ],
            )
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
        if changed != 1 {
            return Err(ConversationLibraryError::Conflict);
        }
        transaction
            .execute(
                "DELETE FROM conversation_tags WHERE conversation_id=?1",
                [preview.conversation_id.as_str()],
            )
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
        for tag in &preview.proposed_record.tags {
            transaction
                .execute(
                    "INSERT INTO conversation_tags(conversation_id, tag) VALUES (?1, ?2)",
                    params![preview.conversation_id.as_str(), tag],
                )
                .map_err(|_| ConversationLibraryError::StorageFailed)?;
        }
        transaction
            .commit()
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
        Ok(ConversationMutationReceipt {
            conversation_id: preview.conversation_id.clone(),
            turn_id: None,
            current_ordinal: self.current_turn_ordinal(&preview.conversation_id)?,
            conversation_sha256: preview.proposed_record_sha256.clone(),
            turn_sha256: None,
            encrypted_canonical_store: true,
        })
    }

    /// Constructs an exact content-free deletion preview without changing state.
    pub fn preview_conversation_deletion(
        &self,
        conversation_id: &ConversationId,
    ) -> Result<ConversationDeletionPreview, ConversationLibraryError> {
        let conversation = self
            .conversation(conversation_id)?
            .ok_or(ConversationLibraryError::NotFound)?;
        let relationships = self.conversation_relationships(conversation_id)?;
        let turn_count = count_rows(
            &self.connection,
            "SELECT COUNT(*) FROM conversation_turns WHERE conversation_id=?1",
            conversation_id.as_str(),
        )?;
        let compaction_count = count_rows(
            &self.connection,
            "SELECT COUNT(*) FROM conversation_compactions WHERE conversation_id=?1",
            conversation_id.as_str(),
        )?;
        let attachment_reference_count = count_joined_rows(
            &self.connection,
            "conversation_turn_attachments",
            conversation_id.as_str(),
        )?;
        let mut evidence_reference_count = 0_u64;
        for table in [
            "conversation_turn_grants",
            "conversation_turn_receipts",
            "conversation_turn_checkpoints",
            "conversation_turn_citations",
            "conversation_turn_sources",
        ] {
            evidence_reference_count = evidence_reference_count
                .checked_add(count_joined_rows(
                    &self.connection,
                    table,
                    conversation_id.as_str(),
                )?)
                .ok_or(ConversationLibraryError::IntegrityFailure)?;
        }
        let expected_record_sha256 = canonical_record_sha256(&conversation)?;
        let preview_sha256 = deletion_preview_digest(
            conversation_id,
            &expected_record_sha256,
            turn_count,
            compaction_count,
            attachment_reference_count,
            evidence_reference_count,
            &relationships.child_conversation_ids,
        );
        Ok(ConversationDeletionPreview {
            conversation_id: conversation_id.clone(),
            expected_record_sha256,
            turn_count,
            compaction_count,
            attachment_reference_count,
            evidence_reference_count,
            blocking_child_conversation_ids: relationships.child_conversation_ids,
            preview_sha256,
            applied: false,
        })
    }

    /// Applies one exact leaf-conversation deletion after separately bound user approval.
    pub fn delete_conversation(
        &mut self,
        preview: &ConversationDeletionPreview,
        approval: &ConversationDeletionApproval,
    ) -> Result<ConversationDeletionReceipt, ConversationLibraryError> {
        validate_deletion_preview(preview)?;
        validate_deletion_approval(preview, approval)?;
        if !preview.blocking_child_conversation_ids.is_empty() {
            return Err(ConversationLibraryError::Conflict);
        }
        let current_preview = self.preview_conversation_deletion(&preview.conversation_id)?;
        if &current_preview != preview {
            return Err(ConversationLibraryError::Conflict);
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
        let deleted_compactions = transaction
            .execute(
                "DELETE FROM conversation_compactions WHERE conversation_id=?1",
                [preview.conversation_id.as_str()],
            )
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
        if deleted_compactions as u64 != preview.compaction_count {
            return Err(ConversationLibraryError::Conflict);
        }
        transaction
            .execute(
                "UPDATE conversations SET current_turn_id=NULL WHERE conversation_id=?1 AND record_sha256=?2",
                params![
                    preview.conversation_id.as_str(),
                    &preview.expected_record_sha256,
                ],
            )
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
        let deleted_turns = transaction
            .execute(
                "DELETE FROM conversation_turns WHERE conversation_id=?1",
                [preview.conversation_id.as_str()],
            )
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
        if deleted_turns as u64 != preview.turn_count {
            return Err(ConversationLibraryError::Conflict);
        }
        let deleted_conversation = transaction
            .execute(
                "DELETE FROM conversations WHERE conversation_id=?1 AND record_sha256=?2",
                params![
                    preview.conversation_id.as_str(),
                    &preview.expected_record_sha256,
                ],
            )
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
        if deleted_conversation != 1 {
            return Err(ConversationLibraryError::Conflict);
        }
        transaction
            .commit()
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
        Ok(ConversationDeletionReceipt {
            conversation_id_sha256: sha256(preview.conversation_id.as_str().as_bytes()),
            preview_sha256: preview.preview_sha256.clone(),
            approval_id_sha256: sha256(approval.approval_id.as_str().as_bytes()),
            deleted_turn_count: preview.turn_count,
            deleted_compaction_count: preview.compaction_count,
            deleted: true,
        })
    }

    fn current_turn_ordinal(
        &self,
        conversation_id: &ConversationId,
    ) -> Result<u64, ConversationLibraryError> {
        let value: i64 = self
            .connection
            .query_row(
                "SELECT COALESCE(MAX(ordinal), 0) FROM conversation_turns WHERE conversation_id=?1",
                [conversation_id.as_str()],
                |row| row.get(0),
            )
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
        u64::try_from(value).map_err(|_| ConversationLibraryError::IntegrityFailure)
    }

    fn load_conversation_checkpoint(
        &self,
        checkpoint_id: &SessionCheckpointId,
    ) -> Result<SessionCheckpoint, ConversationLibraryError> {
        let row = self
            .connection
            .query_row(
                "SELECT checkpoint_sha256, record_json FROM session_checkpoints
                 WHERE checkpoint_id=?1",
                [checkpoint_id.as_str()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?)),
            )
            .optional()
            .map_err(|_| ConversationLibraryError::StorageFailed)?
            .ok_or(ConversationLibraryError::NotFound)?;
        let checkpoint: SessionCheckpoint =
            from_json(&row.1).map_err(|_| ConversationLibraryError::IntegrityFailure)?;
        if checkpoint.checkpoint_id != *checkpoint_id
            || checkpoint.checkpoint_sha256 != row.0
            || checkpoint.ephemeral
            || crate::context_management::verify_checkpoint(&checkpoint).is_err()
        {
            return Err(ConversationLibraryError::IntegrityFailure);
        }
        Ok(checkpoint)
    }

    fn source_history_sha256(
        &self,
        conversation_id: &ConversationId,
        through_ordinal: u64,
    ) -> Result<String, ConversationLibraryError> {
        let history = self.conversation_history(conversation_id)?;
        if through_ordinal == 0 || through_ordinal > history.turns.len() as u64 {
            return Err(ConversationLibraryError::Conflict);
        }
        let mut value = String::from("conversation-history-v1\n");
        for turn in history.turns.iter().take(through_ordinal as usize) {
            writeln!(
                &mut value,
                "{} {}",
                turn.turn_id.as_str(),
                sha256(
                    &to_canonical_json(turn)
                        .map_err(|_| ConversationLibraryError::IntegrityFailure)?
                )
            )
            .expect("writing to a String cannot fail");
        }
        Ok(sha256(value.as_bytes()))
    }

    fn validate_compaction_against_source(
        &self,
        record: &ConversationCompactionRecord,
    ) -> Result<(), ConversationLibraryError> {
        if record.schema_version != CONTRACT_SCHEMA_VERSION
            || !valid_prefixed_id(record.compaction_id.as_str(), "compaction-")
            || !valid_prefixed_id(record.conversation_id.as_str(), "conversation-")
            || !valid_prefixed_id(record.through_turn_id.as_str(), "turn-")
            || record.source_turn_ids.is_empty()
            || record.source_turn_ids.len() > MAX_REFERENCES
            || record.source_turn_ids.last() != Some(&record.through_turn_id)
            || record.source_turn_ids.iter().collect::<BTreeSet<_>>().len()
                != record.source_turn_ids.len()
            || record.source_sha256.len() > MAX_REFERENCES
            || !strict_unique_text(&record.source_sha256, 64)
            || !record.source_sha256.iter().all(|value| valid_sha256(value))
            || source_hash_set_digest(&record.source_sha256) != record.source_hash_set_sha256
            || record.summary.source_set_sha256 != record.source_hash_set_sha256
            || crate::context_management::evaluate_checked_summary(&record.summary)
                != Ok(crate::context_management::SummaryUseDecision::UseSummary)
        {
            return Err(ConversationLibraryError::InvalidInput);
        }
        let history = self.conversation_history_without_compactions(&record.conversation_id)?;
        if record.source_turn_ids.len() > history.len()
            || history
                .iter()
                .take(record.source_turn_ids.len())
                .map(|turn| &turn.turn_id)
                .ne(record.source_turn_ids.iter())
        {
            return Err(ConversationLibraryError::Conflict);
        }
        let source_turns = &history[..record.source_turn_ids.len()];
        let citations = source_turns
            .iter()
            .flat_map(|turn| turn.citation_ids.iter().cloned())
            .collect::<BTreeSet<_>>();
        let receipts = source_turns
            .iter()
            .flat_map(|turn| {
                turn.receipt_ids
                    .iter()
                    .map(|value| value.as_str().to_owned())
            })
            .collect::<BTreeSet<_>>();
        let sources = source_turns
            .iter()
            .flat_map(|turn| turn.source_sha256.iter().cloned())
            .collect::<BTreeSet<_>>();
        if citations
            != record
                .summary
                .citation_ids
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>()
            || receipts
                != record
                    .summary
                    .receipt_ids
                    .iter()
                    .map(|value| value.as_str().to_owned())
                    .collect::<BTreeSet<_>>()
            || sources
                != record
                    .source_sha256
                    .iter()
                    .cloned()
                    .collect::<BTreeSet<_>>()
        {
            return Err(ConversationLibraryError::Conflict);
        }
        Ok(())
    }

    fn conversation_history_without_compactions(
        &self,
        conversation_id: &ConversationId,
    ) -> Result<Vec<ConversationTurn>, ConversationLibraryError> {
        if self.conversation(conversation_id)?.is_none() {
            return Err(ConversationLibraryError::NotFound);
        }
        let turn_ids = query_strings(
            &self.connection,
            "SELECT turn_id FROM conversation_turns
             WHERE conversation_id=?1 ORDER BY ordinal",
            conversation_id.as_str(),
        )?;
        let mut turns = Vec::with_capacity(turn_ids.len());
        for (index, turn_id) in turn_ids.into_iter().enumerate() {
            let turn = self
                .conversation_turn(&ConversationTurnId::from_raw(turn_id))?
                .ok_or(ConversationLibraryError::IntegrityFailure)?;
            if turn.ordinal != index as u64 + 1 {
                return Err(ConversationLibraryError::IntegrityFailure);
            }
            turns.push(turn);
        }
        Ok(turns)
    }
}

fn metadata_change_kind(change: &ConversationMetadataChange) -> ConversationMetadataChangeKind {
    match change {
        ConversationMetadataChange::Rename(_) => ConversationMetadataChangeKind::Rename,
        ConversationMetadataChange::SetPinned(_) => ConversationMetadataChangeKind::SetPinned,
        ConversationMetadataChange::Archive => ConversationMetadataChangeKind::Archive,
        ConversationMetadataChange::ReplaceTags(_) => ConversationMetadataChangeKind::ReplaceTags,
        ConversationMetadataChange::SetRetention(_) => ConversationMetadataChangeKind::SetRetention,
    }
}

fn branch_preview_digest(
    review: &ConversationResumeReview,
    proposed_record_sha256: &str,
) -> String {
    let mut value = format!(
        "conversation-branch-v1\n{}\n{}\n{}\n{}\n{}\n",
        review.conversation_id.as_str(),
        review.turn_id.as_str(),
        review.checkpoint.checkpoint_sha256,
        review.source_history_sha256,
        proposed_record_sha256,
    );
    match &review.directive {
        ResumeDirective::Continue => value.push_str("continue\n"),
        ResumeDirective::DecisionRequired { drift } => {
            value.push_str("decision_required\n");
            for dimension in drift {
                writeln!(&mut value, "{}", drift_dimension_code(*dimension))
                    .expect("writing to a String cannot fail");
            }
        }
    }
    sha256(value.as_bytes())
}

fn validate_branch_preview(
    preview: &ConversationBranchPreview,
) -> Result<(), ConversationLibraryError> {
    validate_conversation(&preview.proposed_conversation, true)?;
    if preview.applied
        || preview.resume_review.resumed
        || preview
            .proposed_conversation
            .parent_conversation_id
            .as_ref()
            != Some(&preview.resume_review.conversation_id)
        || preview.proposed_conversation.branch_from_turn_id.as_ref()
            != Some(&preview.resume_review.turn_id)
        || canonical_record_sha256(&preview.proposed_conversation)?
            != preview.proposed_record_sha256
        || branch_preview_digest(&preview.resume_review, &preview.proposed_record_sha256)
            != preview.preview_sha256
    {
        return Err(ConversationLibraryError::InvalidInput);
    }
    Ok(())
}

const fn drift_dimension_code(dimension: ResumeDriftDimension) -> &'static str {
    match dimension {
        ResumeDriftDimension::Workspace => "workspace",
        ResumeDriftDimension::File => "file",
        ResumeDriftDimension::Instruction => "instruction",
        ResumeDriftDimension::Branch => "branch",
        ResumeDriftDimension::RepositoryMap => "repository_map",
        ResumeDriftDimension::Citation => "citation",
        ResumeDriftDimension::Model => "model",
        ResumeDriftDimension::Permission => "permission",
        ResumeDriftDimension::Policy => "policy",
    }
}

fn metadata_change_code(kind: ConversationMetadataChangeKind) -> &'static str {
    match kind {
        ConversationMetadataChangeKind::Rename => "rename",
        ConversationMetadataChangeKind::SetPinned => "set_pinned",
        ConversationMetadataChangeKind::Archive => "archive",
        ConversationMetadataChangeKind::ReplaceTags => "replace_tags",
        ConversationMetadataChangeKind::SetRetention => "set_retention",
    }
}

fn canonical_record_sha256(
    record: &ConversationRecord,
) -> Result<String, ConversationLibraryError> {
    to_canonical_json(record)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| ConversationLibraryError::InvalidInput)
}

fn change_preview_digest(
    conversation_id: &ConversationId,
    expected_record_sha256: &str,
    proposed_record_sha256: &str,
    change_kind: ConversationMetadataChangeKind,
) -> String {
    sha256(
        format!(
            "conversation-change-v1\n{}\n{}\n{}\n{}\n",
            conversation_id.as_str(),
            expected_record_sha256,
            proposed_record_sha256,
            metadata_change_code(change_kind),
        )
        .as_bytes(),
    )
}

fn validate_change_preview(
    preview: &ConversationChangePreview,
) -> Result<(), ConversationLibraryError> {
    validate_conversation(&preview.proposed_record, false)?;
    if preview.applied
        || preview.proposed_record.conversation_id != preview.conversation_id
        || !valid_sha256(&preview.expected_record_sha256)
        || canonical_record_sha256(&preview.proposed_record)? != preview.proposed_record_sha256
        || change_preview_digest(
            &preview.conversation_id,
            &preview.expected_record_sha256,
            &preview.proposed_record_sha256,
            preview.change_kind,
        ) != preview.preview_sha256
    {
        return Err(ConversationLibraryError::InvalidInput);
    }
    Ok(())
}

fn same_immutable_conversation_fields(
    current: &ConversationRecord,
    proposed: &ConversationRecord,
) -> bool {
    current.schema_version == proposed.schema_version
        && current.conversation_id == proposed.conversation_id
        && current.created_at_epoch_ms == proposed.created_at_epoch_ms
        && current.local_timezone == proposed.local_timezone
        && current.workspace_id == proposed.workspace_id
        && current.project_id == proposed.project_id
        && current.model_profile_id == proposed.model_profile_id
        && current.parent_conversation_id == proposed.parent_conversation_id
        && current.branch_from_turn_id == proposed.branch_from_turn_id
        && current.current_turn_id == proposed.current_turn_id
        && current.persistence_enabled == proposed.persistence_enabled
        && current.sensitivity == proposed.sensitivity
}

fn deletion_preview_digest(
    conversation_id: &ConversationId,
    expected_record_sha256: &str,
    turn_count: u64,
    compaction_count: u64,
    attachment_reference_count: u64,
    evidence_reference_count: u64,
    children: &[ConversationId],
) -> String {
    let mut value = format!(
        "conversation-delete-v1\n{}\n{}\n{}\n{}\n{}\n{}\n",
        conversation_id.as_str(),
        expected_record_sha256,
        turn_count,
        compaction_count,
        attachment_reference_count,
        evidence_reference_count,
    );
    for child in children {
        writeln!(&mut value, "{}", child.as_str()).expect("writing to a String cannot fail");
    }
    sha256(value.as_bytes())
}

fn validate_deletion_preview(
    preview: &ConversationDeletionPreview,
) -> Result<(), ConversationLibraryError> {
    if preview.applied
        || !valid_prefixed_id(preview.conversation_id.as_str(), "conversation-")
        || !valid_sha256(&preview.expected_record_sha256)
        || !strict_sorted_ids(
            preview
                .blocking_child_conversation_ids
                .iter()
                .map(ConversationId::as_str),
        )
        || deletion_preview_digest(
            &preview.conversation_id,
            &preview.expected_record_sha256,
            preview.turn_count,
            preview.compaction_count,
            preview.attachment_reference_count,
            preview.evidence_reference_count,
            &preview.blocking_child_conversation_ids,
        ) != preview.preview_sha256
    {
        return Err(ConversationLibraryError::InvalidInput);
    }
    Ok(())
}

fn validate_deletion_approval(
    preview: &ConversationDeletionPreview,
    approval: &ConversationDeletionApproval,
) -> Result<(), ConversationLibraryError> {
    if !approval.user_confirmed
        || approval.approved_at_epoch_ms == 0
        || !valid_prefixed_id(approval.approval_id.as_str(), "approval-")
        || approval.approved_preview_sha256 != preview.preview_sha256
        || !valid_sha256(&approval.decision_sha256)
    {
        return Err(ConversationLibraryError::InvalidInput);
    }
    Ok(())
}

fn count_rows(
    connection: &rusqlite::Connection,
    sql: &str,
    identity: &str,
) -> Result<u64, ConversationLibraryError> {
    let count: i64 = connection
        .query_row(sql, [identity], |row| row.get(0))
        .map_err(|_| ConversationLibraryError::StorageFailed)?;
    u64::try_from(count).map_err(|_| ConversationLibraryError::IntegrityFailure)
}

fn count_joined_rows(
    connection: &rusqlite::Connection,
    table: &str,
    conversation_id: &str,
) -> Result<u64, ConversationLibraryError> {
    let sql = match table {
        "conversation_turn_attachments" => {
            "SELECT COUNT(*) FROM conversation_turn_attachments AS reference
             JOIN conversation_turns AS turn ON turn.turn_id=reference.turn_id
             WHERE turn.conversation_id=?1"
        }
        "conversation_turn_grants" => {
            "SELECT COUNT(*) FROM conversation_turn_grants AS reference
             JOIN conversation_turns AS turn ON turn.turn_id=reference.turn_id
             WHERE turn.conversation_id=?1"
        }
        "conversation_turn_receipts" => {
            "SELECT COUNT(*) FROM conversation_turn_receipts AS reference
             JOIN conversation_turns AS turn ON turn.turn_id=reference.turn_id
             WHERE turn.conversation_id=?1"
        }
        "conversation_turn_checkpoints" => {
            "SELECT COUNT(*) FROM conversation_turn_checkpoints AS reference
             JOIN conversation_turns AS turn ON turn.turn_id=reference.turn_id
             WHERE turn.conversation_id=?1"
        }
        "conversation_turn_citations" => {
            "SELECT COUNT(*) FROM conversation_turn_citations AS reference
             JOIN conversation_turns AS turn ON turn.turn_id=reference.turn_id
             WHERE turn.conversation_id=?1"
        }
        "conversation_turn_sources" => {
            "SELECT COUNT(*) FROM conversation_turn_sources AS reference
             JOIN conversation_turns AS turn ON turn.turn_id=reference.turn_id
             WHERE turn.conversation_id=?1"
        }
        _ => return Err(ConversationLibraryError::InvalidInput),
    };
    count_rows(connection, sql, conversation_id)
}

fn validate_query(query: &ConversationQuery) -> Result<(), ConversationLibraryError> {
    if query.limit == 0
        || query.limit > MAX_QUERY_RESULTS
        || query
            .from_local_date
            .as_deref()
            .is_some_and(|value| !valid_local_date(value))
        || query
            .to_local_date
            .as_deref()
            .is_some_and(|value| !valid_local_date(value))
        || query.from_local_date > query.to_local_date && query.to_local_date.is_some()
        || query
            .text
            .as_deref()
            .is_some_and(|value| !valid_bounded_text(value, MAX_QUERY_BYTES))
        || query
            .workspace_id
            .as_ref()
            .is_some_and(|value| !valid_prefixed_id(value.as_str(), "workspace-"))
        || query
            .project_id
            .as_deref()
            .is_some_and(|value| !valid_bounded_text(value, MAX_PROJECT_BYTES))
        || query
            .model_profile_id
            .as_ref()
            .is_some_and(|value| !valid_prefixed_id(value.as_str(), "model-"))
        || query
            .tag
            .as_deref()
            .is_some_and(|value| !valid_bounded_text(value, MAX_TAG_BYTES))
    {
        return Err(ConversationLibraryError::InvalidInput);
    }
    Ok(())
}

fn conversation_matches(conversation: &ConversationRecord, query: &ConversationQuery) -> bool {
    query
        .from_local_date
        .as_ref()
        .is_none_or(|value| &conversation.local_date >= value)
        && query
            .to_local_date
            .as_ref()
            .is_none_or(|value| &conversation.local_date <= value)
        && query
            .workspace_id
            .as_ref()
            .is_none_or(|value| &conversation.workspace_id == value)
        && query
            .project_id
            .as_ref()
            .is_none_or(|value| conversation.project_id.as_ref() == Some(value))
        && query
            .model_profile_id
            .as_ref()
            .is_none_or(|value| &conversation.model_profile_id == value)
        && query
            .status
            .is_none_or(|value| conversation.status == value)
        && query
            .tag
            .as_ref()
            .is_none_or(|value| conversation.tags.contains(value))
        && query
            .pinned
            .is_none_or(|value| conversation.pinned == value)
        && (query.include_archived
            || query.status == Some(ConversationStatus::Archived)
            || conversation.status != ConversationStatus::Archived)
}

fn search_preview(
    conversation: &ConversationRecord,
    turns: &[ConversationTurn],
    compactions: &[ConversationCompactionRecord],
    normalized_query: &str,
) -> Option<String> {
    if conversation.title.to_lowercase().contains(normalized_query) {
        return Some(bounded_preview(&conversation.title));
    }
    for turn in turns {
        if let Some(text) = &turn.text
            && text.to_lowercase().contains(normalized_query)
        {
            return Some(bounded_preview(text));
        }
        for attachment in &turn.attachments {
            if attachment
                .display_name
                .to_lowercase()
                .contains(normalized_query)
                || attachment
                    .media_type
                    .to_lowercase()
                    .contains(normalized_query)
            {
                return Some(bounded_preview(&attachment.display_name));
            }
        }
        for identity in turn
            .citation_ids
            .iter()
            .map(String::as_str)
            .chain(turn.grant_ids.iter().map(|value| value.as_str()))
            .chain(turn.receipt_ids.iter().map(|value| value.as_str()))
        {
            if identity.to_lowercase().contains(normalized_query) {
                return Some(bounded_preview(identity));
            }
        }
    }
    for compaction in compactions {
        if compaction
            .summary
            .summary
            .to_lowercase()
            .contains(normalized_query)
        {
            return Some(bounded_preview(&compaction.summary.summary));
        }
        for value in compaction
            .summary
            .paths
            .iter()
            .chain(&compaction.summary.errors)
            .chain(&compaction.summary.identifiers)
            .chain(&compaction.summary.commands)
            .chain(&compaction.summary.decisions)
            .chain(&compaction.summary.unresolved_questions)
        {
            if value.to_lowercase().contains(normalized_query) {
                return Some(bounded_preview(value));
            }
        }
    }
    None
}

fn bounded_preview(value: &str) -> String {
    value.chars().take(MAX_RESULT_PREVIEW_CHARS).collect()
}

fn source_hash_set_digest(source_sha256: &[String]) -> String {
    let mut value = String::from("conversation-source-hashes-v1\n");
    for source in source_sha256 {
        writeln!(&mut value, "{source}").expect("writing to a String cannot fail");
    }
    sha256(value.as_bytes())
}

fn insert_turn_references(
    transaction: &rusqlite::Transaction<'_>,
    turn: &ConversationTurn,
) -> Result<(), ConversationLibraryError> {
    for attachment in &turn.attachments {
        transaction
            .execute(
                "INSERT INTO conversation_turn_attachments(turn_id, reference_id, content_sha256)
                 VALUES (?1, ?2, ?3)",
                params![
                    turn.turn_id.as_str(),
                    &attachment.reference_id,
                    &attachment.content_sha256,
                ],
            )
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
    }
    for grant_id in &turn.grant_ids {
        transaction
            .execute(
                "INSERT INTO conversation_turn_grants(turn_id, grant_id) VALUES (?1, ?2)",
                params![turn.turn_id.as_str(), grant_id.as_str()],
            )
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
    }
    for receipt_id in &turn.receipt_ids {
        transaction
            .execute(
                "INSERT INTO conversation_turn_receipts(turn_id, receipt_id) VALUES (?1, ?2)",
                params![turn.turn_id.as_str(), receipt_id.as_str()],
            )
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
    }
    if let Some(checkpoint_id) = &turn.checkpoint_id {
        transaction
            .execute(
                "INSERT INTO conversation_turn_checkpoints(turn_id, checkpoint_id) VALUES (?1, ?2)",
                params![turn.turn_id.as_str(), checkpoint_id.as_str()],
            )
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
    }
    for citation_id in &turn.citation_ids {
        transaction
            .execute(
                "INSERT INTO conversation_turn_citations(turn_id, citation_id) VALUES (?1, ?2)",
                params![turn.turn_id.as_str(), citation_id],
            )
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
    }
    for source_sha256 in &turn.source_sha256 {
        transaction
            .execute(
                "INSERT INTO conversation_turn_sources(turn_id, source_sha256) VALUES (?1, ?2)",
                params![turn.turn_id.as_str(), source_sha256],
            )
            .map_err(|_| ConversationLibraryError::StorageFailed)?;
    }
    Ok(())
}

fn conversation_head_before(
    ordinal: u64,
    transaction: &rusqlite::Transaction<'_>,
    conversation_id: &ConversationId,
) -> Result<Option<String>, ConversationLibraryError> {
    if ordinal == 1 {
        return Ok(None);
    }
    transaction
        .query_row(
            "SELECT turn_id FROM conversation_turns
             WHERE conversation_id=?1 AND ordinal=?2",
            params![conversation_id.as_str(), as_sql_integer(ordinal - 1)?],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| ConversationLibraryError::StorageFailed)
}

fn validate_conversation(
    conversation: &ConversationRecord,
    creating: bool,
) -> Result<(), ConversationLibraryError> {
    if conversation.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_prefixed_id(conversation.conversation_id.as_str(), "conversation-")
        || conversation.title.trim().is_empty()
        || conversation.title.len() > MAX_TITLE_BYTES
        || conversation.updated_at_epoch_ms < conversation.created_at_epoch_ms
        || !valid_local_date(&conversation.local_date)
        || !valid_bounded_text(&conversation.local_timezone, MAX_TIMEZONE_BYTES)
        || !valid_prefixed_id(conversation.workspace_id.as_str(), "workspace-")
        || !valid_prefixed_id(conversation.model_profile_id.as_str(), "model-")
        || conversation
            .project_id
            .as_deref()
            .is_some_and(|value| !valid_bounded_text(value, MAX_PROJECT_BYTES))
        || conversation.tags.len() > MAX_TAGS
        || !strict_unique_text(&conversation.tags, MAX_TAG_BYTES)
        || !valid_sha256(&conversation.retention.policy_sha256)
        || match conversation.retention.kind {
            ConversationRetentionKind::Session | ConversationRetentionKind::UntilExpiration => {
                conversation
                    .retention
                    .expires_at_epoch_ms
                    .is_none_or(|expires| expires <= conversation.updated_at_epoch_ms)
            }
            ConversationRetentionKind::UserHold => {
                conversation.retention.expires_at_epoch_ms.is_some()
            }
        }
        || conversation.parent_conversation_id.is_some()
            != conversation.branch_from_turn_id.is_some()
        || conversation
            .parent_conversation_id
            .as_ref()
            .is_some_and(|parent| parent == &conversation.conversation_id)
        || conversation
            .parent_conversation_id
            .as_ref()
            .is_some_and(|value| !valid_prefixed_id(value.as_str(), "conversation-"))
        || conversation
            .branch_from_turn_id
            .as_ref()
            .is_some_and(|value| !valid_prefixed_id(value.as_str(), "turn-"))
        || conversation
            .current_turn_id
            .as_ref()
            .is_some_and(|value| !valid_prefixed_id(value.as_str(), "turn-"))
        || (creating && conversation.current_turn_id.is_some())
    {
        return Err(ConversationLibraryError::InvalidInput);
    }
    Ok(())
}

fn validate_turn(turn: &ConversationTurn) -> Result<(), ConversationLibraryError> {
    let text_matches = turn.text.as_ref().is_none_or(|text| {
        !text.is_empty()
            && text.len() <= MAX_TURN_TEXT_BYTES
            && sha256(text.as_bytes()) == turn.text_sha256
    });
    if turn.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_prefixed_id(turn.turn_id.as_str(), "turn-")
        || !valid_prefixed_id(turn.conversation_id.as_str(), "conversation-")
        || turn.ordinal == 0
        || !valid_local_date(&turn.local_date)
        || !valid_sha256(&turn.text_sha256)
        || !text_matches
        || turn.attachments.len() > MAX_REFERENCES
        || turn.grant_ids.len() > MAX_REFERENCES
        || turn.receipt_ids.len() > MAX_REFERENCES
        || turn.citation_ids.len() > MAX_REFERENCES
        || turn.source_sha256.len() > MAX_REFERENCES
        || !strict_unique_text(&turn.citation_ids, MAX_REFERENCE_ID_BYTES)
        || !strict_unique_text(&turn.source_sha256, 64)
        || !turn.source_sha256.iter().all(|value| valid_sha256(value))
        || !strict_sorted_ids(turn.grant_ids.iter().map(|value| value.as_str()))
        || !strict_sorted_ids(turn.receipt_ids.iter().map(|value| value.as_str()))
        || !valid_attachments(turn)
        || turn
            .checkpoint_id
            .as_ref()
            .is_some_and(|value| !valid_prefixed_id(value.as_str(), "checkpoint-"))
    {
        return Err(ConversationLibraryError::InvalidInput);
    }
    Ok(())
}

fn valid_attachments(turn: &ConversationTurn) -> bool {
    turn.attachments.iter().all(|attachment| {
        valid_bounded_text(&attachment.reference_id, MAX_REFERENCE_ID_BYTES)
            && valid_bounded_text(&attachment.display_name, MAX_ATTACHMENT_NAME_BYTES)
            && valid_sha256(&attachment.content_sha256)
            && valid_bounded_text(&attachment.media_type, MAX_MEDIA_TYPE_BYTES)
    }) && strict_sorted_ids(
        turn.attachments
            .iter()
            .map(|attachment| attachment.reference_id.as_str()),
    )
}

fn strict_unique_text(values: &[String], max_bytes: usize) -> bool {
    values.windows(2).all(|window| window[0] < window[1])
        && values
            .iter()
            .all(|value| valid_bounded_text(value, max_bytes))
}

fn strict_sorted_ids<'a>(values: impl Iterator<Item = &'a str>) -> bool {
    let values = values.collect::<Vec<_>>();
    values.len() <= MAX_REFERENCES
        && values
            .iter()
            .all(|value| valid_bounded_text(value, MAX_REFERENCE_ID_BYTES))
        && values.windows(2).all(|window| window[0] < window[1])
}

fn verify_conversation_projection(
    connection: &rusqlite::Connection,
    record: &ConversationRecord,
) -> Result<(), ConversationLibraryError> {
    let matches: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM conversations
             WHERE conversation_id=?1 AND title=?2 AND sensitivity=?3
               AND created_at_epoch_ms=?4 AND updated_at_epoch_ms=?5
               AND local_date=?6 AND local_timezone=?7 AND workspace_id=?8
               AND project_id IS ?9 AND model_profile_id=?10 AND status=?11
               AND parent_conversation_id IS ?12 AND branch_from_turn_id IS ?13
               AND current_turn_id IS ?14 AND retention_kind=?15
               AND retention_expires_at_epoch_ms IS ?16 AND retention_policy_sha256=?17
               AND pinned=?18 AND persistence_enabled=?19",
            params![
                record.conversation_id.as_str(),
                &record.title,
                sensitivity_code(record.sensitivity),
                as_sql_integer(record.created_at_epoch_ms)?,
                as_sql_integer(record.updated_at_epoch_ms)?,
                &record.local_date,
                &record.local_timezone,
                record.workspace_id.as_str(),
                record.project_id.as_deref(),
                record.model_profile_id.as_str(),
                status_code(record.status),
                record
                    .parent_conversation_id
                    .as_ref()
                    .map(ConversationId::as_str),
                record
                    .branch_from_turn_id
                    .as_ref()
                    .map(ConversationTurnId::as_str),
                record
                    .current_turn_id
                    .as_ref()
                    .map(ConversationTurnId::as_str),
                retention_code(record.retention.kind),
                record
                    .retention
                    .expires_at_epoch_ms
                    .map(as_sql_integer)
                    .transpose()?,
                &record.retention.policy_sha256,
                i64::from(record.pinned),
                i64::from(record.persistence_enabled),
            ],
            |row| row.get(0),
        )
        .map_err(|_| ConversationLibraryError::IntegrityFailure)?;
    let tags = query_strings(
        connection,
        "SELECT tag FROM conversation_tags WHERE conversation_id=?1 ORDER BY tag",
        record.conversation_id.as_str(),
    )?;
    if matches != 1 || tags != record.tags {
        return Err(ConversationLibraryError::IntegrityFailure);
    }
    Ok(())
}

fn verify_turn_projection(
    connection: &rusqlite::Connection,
    turn: &ConversationTurn,
) -> Result<(), ConversationLibraryError> {
    let scalar_matches: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM conversation_turns
             WHERE turn_id=?1 AND conversation_id=?2 AND ordinal=?3 AND role=?4
               AND sensitivity=?5 AND created_at_epoch_ms=?6 AND local_date=?7
               AND text_sha256=?8",
            params![
                turn.turn_id.as_str(),
                turn.conversation_id.as_str(),
                as_sql_integer(turn.ordinal)?,
                role_code(turn.role),
                sensitivity_code(turn.sensitivity),
                as_sql_integer(turn.created_at_epoch_ms)?,
                &turn.local_date,
                &turn.text_sha256,
            ],
            |row| row.get(0),
        )
        .map_err(|_| ConversationLibraryError::IntegrityFailure)?;
    let attachment_rows = query_pairs(
        connection,
        "SELECT reference_id, content_sha256 FROM conversation_turn_attachments
         WHERE turn_id=?1 ORDER BY reference_id",
        turn.turn_id.as_str(),
    )?;
    let expected_attachments = turn
        .attachments
        .iter()
        .map(|value| (value.reference_id.clone(), value.content_sha256.clone()))
        .collect::<Vec<_>>();
    let grants = query_strings(
        connection,
        "SELECT grant_id FROM conversation_turn_grants WHERE turn_id=?1 ORDER BY grant_id",
        turn.turn_id.as_str(),
    )?;
    let receipts = query_strings(
        connection,
        "SELECT receipt_id FROM conversation_turn_receipts WHERE turn_id=?1 ORDER BY receipt_id",
        turn.turn_id.as_str(),
    )?;
    let checkpoints = query_strings(
        connection,
        "SELECT checkpoint_id FROM conversation_turn_checkpoints WHERE turn_id=?1 ORDER BY checkpoint_id",
        turn.turn_id.as_str(),
    )?;
    let citations = query_strings(
        connection,
        "SELECT citation_id FROM conversation_turn_citations WHERE turn_id=?1 ORDER BY citation_id",
        turn.turn_id.as_str(),
    )?;
    let sources = query_strings(
        connection,
        "SELECT source_sha256 FROM conversation_turn_sources WHERE turn_id=?1 ORDER BY source_sha256",
        turn.turn_id.as_str(),
    )?;
    let expected_grants = turn
        .grant_ids
        .iter()
        .map(|value| value.as_str().to_owned())
        .collect::<Vec<_>>();
    let expected_receipts = turn
        .receipt_ids
        .iter()
        .map(|value| value.as_str().to_owned())
        .collect::<Vec<_>>();
    let expected_checkpoint = turn
        .checkpoint_id
        .as_ref()
        .map(|value| vec![value.as_str().to_owned()])
        .unwrap_or_default();
    if scalar_matches != 1
        || attachment_rows != expected_attachments
        || grants != expected_grants
        || receipts != expected_receipts
        || checkpoints != expected_checkpoint
        || citations != turn.citation_ids
        || sources != turn.source_sha256
    {
        return Err(ConversationLibraryError::IntegrityFailure);
    }
    Ok(())
}

fn query_strings(
    connection: &rusqlite::Connection,
    sql: &str,
    identity: &str,
) -> Result<Vec<String>, ConversationLibraryError> {
    connection
        .prepare(sql)
        .and_then(|mut statement| {
            statement
                .query_map([identity], |row| row.get(0))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| ConversationLibraryError::IntegrityFailure)
}

fn query_pairs(
    connection: &rusqlite::Connection,
    sql: &str,
    identity: &str,
) -> Result<Vec<(String, String)>, ConversationLibraryError> {
    connection
        .prepare(sql)
        .and_then(|mut statement| {
            statement
                .query_map([identity], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|_| ConversationLibraryError::IntegrityFailure)
}

fn valid_bounded_text(value: &str, max_bytes: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max_bytes && !value.contains('\0')
}

fn valid_prefixed_id(value: &str, prefix: &str) -> bool {
    value.starts_with(prefix)
        && value.len() <= MAX_REFERENCE_ID_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.')
        })
}

fn valid_local_date(value: &str) -> bool {
    value.len() == 10
        && value.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 4 | 7) {
                byte == b'-'
            } else {
                byte.is_ascii_digit()
            }
        })
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn status_code(status: ConversationStatus) -> &'static str {
    match status {
        ConversationStatus::Active => "active",
        ConversationStatus::Completed => "completed",
        ConversationStatus::Cancelled => "cancelled",
        ConversationStatus::Failed => "failed",
        ConversationStatus::Archived => "archived",
    }
}

fn retention_code(kind: ConversationRetentionKind) -> &'static str {
    match kind {
        ConversationRetentionKind::Session => "session",
        ConversationRetentionKind::UntilExpiration => "until_expiration",
        ConversationRetentionKind::UserHold => "user_hold",
    }
}

fn role_code(role: agentmage_kernel_contracts::ConversationTurnRole) -> &'static str {
    match role {
        agentmage_kernel_contracts::ConversationTurnRole::System => "system",
        agentmage_kernel_contracts::ConversationTurnRole::User => "user",
        agentmage_kernel_contracts::ConversationTurnRole::Assistant => "assistant",
        agentmage_kernel_contracts::ConversationTurnRole::Tool => "tool",
    }
}

fn sensitivity_code(sensitivity: DataSensitivity) -> &'static str {
    match sensitivity {
        DataSensitivity::Ephemeral => "ephemeral",
        DataSensitivity::Operational => "operational",
        DataSensitivity::Durable => "durable",
        DataSensitivity::Restricted => "restricted",
    }
}

fn strictest_sensitivity(left: DataSensitivity, right: DataSensitivity) -> DataSensitivity {
    if sensitivity_rank(left) >= sensitivity_rank(right) {
        left
    } else {
        right
    }
}

const fn sensitivity_rank(value: DataSensitivity) -> u8 {
    match value {
        DataSensitivity::Ephemeral => 0,
        DataSensitivity::Operational => 1,
        DataSensitivity::Durable => 2,
        DataSensitivity::Restricted => 3,
    }
}

fn as_sql_integer(value: u64) -> Result<i64, ConversationLibraryError> {
    i64::try_from(value).map_err(|_| ConversationLibraryError::InvalidInput)
}

fn classify_insert_error(error: rusqlite::Error) -> ConversationLibraryError {
    match error {
        rusqlite::Error::SqliteFailure(ref failure, _)
            if failure.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY
                || failure.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE =>
        {
            ConversationLibraryError::DuplicateIdentity
        }
        _ => ConversationLibraryError::StorageFailed,
    }
}

fn sha256(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::{
        ApprovalId, CONTRACT_SCHEMA_VERSION, CheckedContextSummary, CheckedSummaryState,
        CheckpointFileIdentity, CloudSynchronizationMarker, ContextSummaryId,
        ConversationAttachmentReference, ConversationCompactionId, ConversationCompactionRecord,
        ConversationId, ConversationRecord, ConversationRetention, ConversationRetentionKind,
        ConversationStatus, ConversationTurn, ConversationTurnId, ConversationTurnRole,
        DataSensitivity, EvidenceId, GrantId, ModelProfileId, PlanId, PlanStepId, PolicyId,
        ReceiptId, RepositorySnapshotId, SessionCheckpoint, SessionCheckpointId, SessionId,
        StorageFilesystemClass, StrictLocalStorageObservation, TaskId, WorkspaceId,
        to_canonical_json,
    };

    use super::{
        ConversationDeletionApproval, ConversationLibraryError, ConversationMetadataChange,
        ConversationQuery, sha256, source_hash_set_digest,
    };
    use crate::context_management::{ResumeDirective, ResumeObservation, finalize_checkpoint};
    use crate::operational_store::{
        OperationalStore, OperationalStoreKeyError, OperationalStoreKeyProvider,
    };

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    struct TestKey([u8; 32]);

    impl OperationalStoreKeyProvider for TestKey {
        fn with_key<T>(
            &mut self,
            operation: impl FnOnce(&[u8]) -> T,
        ) -> Result<T, OperationalStoreKeyError> {
            Ok(operation(&self.0))
        }
    }

    fn observation() -> StrictLocalStorageObservation {
        StrictLocalStorageObservation {
            filesystem: StorageFilesystemClass::Local,
            synchronization_marker: None::<CloudSynchronizationMarker>,
            root_identity_sha256: [31; 32],
            symlink_free: true,
        }
    }

    fn store() -> (std::path::PathBuf, std::path::PathBuf, OperationalStore) {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "agentmage-conversation-library-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&directory).expect("temporary directory");
        let path = directory.join("canonical.db");
        let store = OperationalStore::open(&path, &observation(), &mut TestKey([41; 32]))
            .expect("encrypted store");
        (directory, path, store)
    }

    fn conversation(persistence_enabled: bool) -> ConversationRecord {
        ConversationRecord {
            schema_version: CONTRACT_SCHEMA_VERSION,
            conversation_id: ConversationId::from_raw("conversation-alpha"),
            title: "Alpha review".to_owned(),
            sensitivity: DataSensitivity::Operational,
            created_at_epoch_ms: 1_800_000_000_000,
            updated_at_epoch_ms: 1_800_000_000_000,
            local_date: "2027-01-15".to_owned(),
            local_timezone: "America/Chicago".to_owned(),
            workspace_id: WorkspaceId::from_raw("workspace-alpha"),
            project_id: Some("project-alpha".to_owned()),
            model_profile_id: ModelProfileId::from_raw("model-local-alpha"),
            status: ConversationStatus::Active,
            parent_conversation_id: None,
            branch_from_turn_id: None,
            current_turn_id: None,
            tags: vec!["audit".to_owned(), "local".to_owned()],
            retention: ConversationRetention {
                kind: ConversationRetentionKind::Session,
                expires_at_epoch_ms: Some(1_800_086_400_000),
                policy_sha256: "c".repeat(64),
            },
            pinned: false,
            persistence_enabled,
        }
    }

    fn turn(ordinal: u64, text: Option<&str>) -> ConversationTurn {
        ConversationTurn {
            schema_version: CONTRACT_SCHEMA_VERSION,
            turn_id: ConversationTurnId::from_raw(format!("turn-alpha-{ordinal}")),
            conversation_id: ConversationId::from_raw("conversation-alpha"),
            ordinal,
            role: ConversationTurnRole::User,
            sensitivity: DataSensitivity::Operational,
            created_at_epoch_ms: 1_800_000_000_000 + ordinal,
            local_date: "2027-01-15".to_owned(),
            text: text.map(str::to_owned),
            text_sha256: sha256(text.unwrap_or("redacted").as_bytes()),
            attachments: vec![ConversationAttachmentReference {
                reference_id: format!("attachment-{ordinal}"),
                display_name: "fixture.txt".to_owned(),
                content_sha256: "a".repeat(64),
                media_type: "text/plain".to_owned(),
            }],
            grant_ids: vec![GrantId::from_raw(format!("grant-{ordinal}"))],
            receipt_ids: vec![ReceiptId::from_raw(format!("receipt-{ordinal}"))],
            checkpoint_id: Some(SessionCheckpointId::from_raw(format!(
                "checkpoint-{ordinal}"
            ))),
            citation_ids: vec![format!("citation-{ordinal}")],
            source_sha256: vec!["b".repeat(64)],
        }
    }

    fn checkpoint() -> SessionCheckpoint {
        finalize_checkpoint(SessionCheckpoint {
            schema_version: CONTRACT_SCHEMA_VERSION,
            checkpoint_id: SessionCheckpointId::from_raw("checkpoint-1"),
            session_id: SessionId::from_raw("session-conversation-1"),
            task_id: TaskId::from_raw("task-conversation-1"),
            objective_sha256: "1".repeat(64),
            plan_id: PlanId::from_raw("plan-conversation-1"),
            plan_revision: 1,
            plan_step_id: PlanStepId::from_raw("step-conversation-1"),
            next_action_sha256: "2".repeat(64),
            workspace_id: WorkspaceId::from_raw("workspace-alpha"),
            workspace_state_sha256: "3".repeat(64),
            repository_snapshot_id: RepositorySnapshotId::from_raw("repository-conversation-1"),
            repository_branch: "main".to_owned(),
            repository_map_sha256: "4".repeat(64),
            files: vec![CheckpointFileIdentity {
                object_id: "file-alpha".to_owned(),
                content_sha256: "5".repeat(64),
                observed_revision: "revision-1".to_owned(),
            }],
            instruction_sha256: "6".repeat(64),
            permission_profile_id: "permission-alpha".to_owned(),
            permission_profile_sha256: "7".repeat(64),
            policy_id: PolicyId::from_raw("policy-alpha"),
            policy_sha256: "8".repeat(64),
            model_profile_id: ModelProfileId::from_raw("model-local-alpha"),
            model_manifest_sha256: "9".repeat(64),
            model_runtime_sha256: "a".repeat(64),
            evidence_ids: vec![EvidenceId::from_raw("evidence-alpha")],
            citation_set_sha256: "b".repeat(64),
            blockers: Vec::new(),
            context_packet_sha256: "c".repeat(64),
            action_id: None,
            action_state: None,
            consumed_grant_id: None,
            receipt_id: None,
            receipt_sha256: None,
            ephemeral: false,
            checkpoint_sha256: "0".repeat(64),
        })
        .expect("checkpoint finalizes")
    }

    fn seed_checkpoint(store: &OperationalStore, checkpoint: &SessionCheckpoint) {
        let bytes = to_canonical_json(checkpoint).expect("checkpoint serializes");
        store
            .connection
            .execute(
                "INSERT INTO checkpoints(generation, state_sha256, session_checkpoint_sha256)
                 VALUES (1, ?1, ?2)",
                rusqlite::params!["0".repeat(64), &checkpoint.checkpoint_sha256],
            )
            .expect("checkpoint generation seeds");
        store
            .connection
            .execute(
                "INSERT INTO session_checkpoints(
                    generation, checkpoint_id, checkpoint_sha256, record_json
                 ) VALUES (1, ?1, ?2, ?3)",
                rusqlite::params![
                    checkpoint.checkpoint_id.as_str(),
                    &checkpoint.checkpoint_sha256,
                    bytes,
                ],
            )
            .expect("session checkpoint seeds");
    }

    fn resume_observation(checkpoint: &SessionCheckpoint) -> ResumeObservation {
        ResumeObservation {
            workspace_id: checkpoint.workspace_id.as_str().to_owned(),
            workspace_state_sha256: checkpoint.workspace_state_sha256.clone(),
            files: checkpoint.files.clone(),
            instruction_sha256: checkpoint.instruction_sha256.clone(),
            repository_branch: checkpoint.repository_branch.clone(),
            repository_map_sha256: checkpoint.repository_map_sha256.clone(),
            citation_set_sha256: checkpoint.citation_set_sha256.clone(),
            model_profile_id: checkpoint.model_profile_id.as_str().to_owned(),
            model_manifest_sha256: checkpoint.model_manifest_sha256.clone(),
            model_runtime_sha256: checkpoint.model_runtime_sha256.clone(),
            permission_profile_id: checkpoint.permission_profile_id.clone(),
            permission_profile_sha256: checkpoint.permission_profile_sha256.clone(),
            policy_id: checkpoint.policy_id.as_str().to_owned(),
            policy_sha256: checkpoint.policy_sha256.clone(),
        }
    }

    fn compaction(first: &ConversationTurn) -> ConversationCompactionRecord {
        let source_sha256 = first.source_sha256.clone();
        let source_hash_set_sha256 = source_hash_set_digest(&source_sha256);
        ConversationCompactionRecord {
            schema_version: CONTRACT_SCHEMA_VERSION,
            compaction_id: ConversationCompactionId::from_raw("compaction-alpha-1"),
            conversation_id: first.conversation_id.clone(),
            through_turn_id: first.turn_id.clone(),
            source_turn_ids: vec![first.turn_id.clone()],
            summary: CheckedContextSummary {
                schema_version: CONTRACT_SCHEMA_VERSION,
                summary_id: ContextSummaryId::from_raw("summary-alpha-1"),
                state: CheckedSummaryState::Current,
                summary: "Checked Bayesian continuity summary".to_owned(),
                paths: Vec::new(),
                errors: Vec::new(),
                identifiers: Vec::new(),
                commands: Vec::new(),
                decisions: Vec::new(),
                unresolved_questions: Vec::new(),
                evidence_ids: Vec::new(),
                citation_ids: first.citation_ids.clone(),
                receipt_ids: first.receipt_ids.clone(),
                source_set_sha256: source_hash_set_sha256.clone(),
            },
            source_sha256,
            source_hash_set_sha256,
            created_at_epoch_ms: first.created_at_epoch_ms + 10,
        }
    }

    #[test]
    fn encrypted_store_round_trip_retains_metadata_and_reference_identities() {
        let (directory, path, mut store) = store();
        let conversation = conversation(true);
        let first = turn(1, Some("local conversation canary"));
        let created = store
            .create_conversation(&conversation)
            .expect("conversation creates");
        assert!(created.encrypted_canonical_store);
        let appended = store
            .append_conversation_turn(&first)
            .expect("turn appends");
        assert_eq!(appended.current_ordinal, 1);
        assert_eq!(
            store.conversation_turn(&first.turn_id),
            Ok(Some(first.clone()))
        );
        let loaded = store
            .conversation(&conversation.conversation_id)
            .expect("conversation reads")
            .expect("conversation exists");
        assert_eq!(loaded.current_turn_id, Some(first.turn_id));
        drop(store);
        let bytes = fs::read(&path).expect("encrypted bytes");
        assert!(!bytes.starts_with(b"SQLite format 3"));
        assert!(
            !bytes
                .windows(25)
                .any(|window| window == b"local conversation canary")
        );
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn duplicate_and_out_of_order_turns_leave_the_timeline_unchanged() {
        let (directory, _path, mut store) = store();
        let conversation = conversation(true);
        store
            .create_conversation(&conversation)
            .expect("conversation creates");
        let second = turn(2, Some("out of order"));
        assert_eq!(
            store.append_conversation_turn(&second),
            Err(ConversationLibraryError::Conflict)
        );
        let first = turn(1, Some("first"));
        store
            .append_conversation_turn(&first)
            .expect("first turn appends");
        assert_eq!(
            store.append_conversation_turn(&first),
            Err(ConversationLibraryError::Conflict)
        );
        assert_eq!(store.conversation_turn(&second.turn_id), Ok(None));
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn disabled_persistence_rejects_exact_text_but_accepts_hash_only_turns() {
        let (directory, _path, mut store) = store();
        store
            .create_conversation(&conversation(false))
            .expect("metadata-only conversation creates");
        assert_eq!(
            store.append_conversation_turn(&turn(1, Some("must not persist"))),
            Err(ConversationLibraryError::PersistenceDisabled)
        );
        let hash_only = turn(1, None);
        store
            .append_conversation_turn(&hash_only)
            .expect("hash-only turn appends");
        assert_eq!(
            store
                .conversation_turn(&hash_only.turn_id)
                .expect("turn reads")
                .expect("turn exists")
                .text,
            None
        );
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn malformed_hashes_duplicate_identity_and_cross_parent_branches_fail_closed() {
        let (directory, _path, mut store) = store();
        let original = conversation(true);
        store
            .create_conversation(&original)
            .expect("conversation creates");
        assert_eq!(
            store.create_conversation(&original),
            Err(ConversationLibraryError::DuplicateIdentity)
        );
        let mut malformed = turn(1, Some("exact"));
        malformed.text_sha256 = "0".repeat(64);
        assert_eq!(
            store.append_conversation_turn(&malformed),
            Err(ConversationLibraryError::InvalidInput)
        );
        let first = turn(1, Some("exact"));
        store
            .append_conversation_turn(&first)
            .expect("turn appends");
        let mut branch = conversation(true);
        branch.conversation_id = ConversationId::from_raw("conversation-branch");
        branch.parent_conversation_id = Some(ConversationId::from_raw("conversation-missing"));
        branch.branch_from_turn_id = Some(first.turn_id);
        assert_eq!(
            store.create_conversation(&branch),
            Err(ConversationLibraryError::Conflict)
        );
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn canonical_row_tampering_is_detected_before_returning_content() {
        let (directory, _path, mut store) = store();
        let conversation = conversation(true);
        store
            .create_conversation(&conversation)
            .expect("conversation creates");
        store
            .connection
            .execute(
                "UPDATE conversations SET record_json=X'7b7d' WHERE conversation_id=?1",
                [conversation.conversation_id.as_str()],
            )
            .expect("tamper fixture");
        assert_eq!(
            store.conversation(&conversation.conversation_id),
            Err(ConversationLibraryError::IntegrityFailure)
        );
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn normalized_projection_tampering_is_detected_before_search_or_history_use() {
        let (directory, _path, mut store) = store();
        let conversation = conversation(true);
        let first = turn(1, Some("projection fixture"));
        store
            .create_conversation(&conversation)
            .expect("conversation creates");
        store
            .append_conversation_turn(&first)
            .expect("turn appends");
        store
            .connection
            .execute(
                "DELETE FROM conversation_turn_citations WHERE turn_id=?1",
                [first.turn_id.as_str()],
            )
            .expect("projection tamper fixture");
        assert_eq!(
            store.conversation_turn(&first.turn_id),
            Err(ConversationLibraryError::IntegrityFailure)
        );
        store
            .connection
            .execute(
                "DELETE FROM conversation_tags WHERE conversation_id=?1",
                [conversation.conversation_id.as_str()],
            )
            .expect("tag tamper fixture");
        assert_eq!(
            store.conversation(&conversation.conversation_id),
            Err(ConversationLibraryError::IntegrityFailure)
        );
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn bounded_search_combines_exact_filters_with_local_literal_content_matching() {
        let (directory, _path, mut store) = store();
        let alpha = conversation(true);
        store.create_conversation(&alpha).expect("alpha creates");
        store
            .append_conversation_turn(&turn(1, Some("Review the Bayesian model")))
            .expect("alpha turn");

        let mut archived = conversation(true);
        archived.conversation_id = ConversationId::from_raw("conversation-archived");
        archived.title = "Archived Bayesian notes".to_owned();
        archived.status = ConversationStatus::Archived;
        archived.local_date = "2027-01-16".to_owned();
        archived.created_at_epoch_ms += 10;
        archived.updated_at_epoch_ms += 10;
        store
            .create_conversation(&archived)
            .expect("archive creates");

        let query = ConversationQuery {
            from_local_date: Some("2027-01-15".to_owned()),
            to_local_date: Some("2027-01-15".to_owned()),
            text: Some("BAYESIAN".to_owned()),
            workspace_id: Some(WorkspaceId::from_raw("workspace-alpha")),
            project_id: Some("project-alpha".to_owned()),
            model_profile_id: Some(ModelProfileId::from_raw("model-local-alpha")),
            status: Some(ConversationStatus::Active),
            tag: Some("audit".to_owned()),
            pinned: Some(false),
            include_archived: false,
            limit: 10,
        };
        let hits = store.search_conversations(&query).expect("search succeeds");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].conversation_id, alpha.conversation_id);
        assert_eq!(hits[0].turn_count, 1);
        assert_eq!(
            hits[0].matching_preview.as_deref(),
            Some("Review the Bayesian model")
        );
        let archived_hidden = store
            .search_conversations(&ConversationQuery {
                text: Some("Bayesian".to_owned()),
                limit: 10,
                ..ConversationQuery::default()
            })
            .expect("default excludes archived");
        assert_eq!(archived_hidden.len(), 1);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn history_and_relationship_views_preserve_original_timeline() {
        let (directory, _path, mut store) = store();
        let original = conversation(true);
        let first = turn(1, Some("original first turn"));
        store
            .create_conversation(&original)
            .expect("original creates");
        store
            .append_conversation_turn(&first)
            .expect("original turn");
        let mut branch = conversation(true);
        branch.conversation_id = ConversationId::from_raw("conversation-branch");
        branch.title = "Branch".to_owned();
        branch.parent_conversation_id = Some(original.conversation_id.clone());
        branch.branch_from_turn_id = Some(first.turn_id.clone());
        store
            .create_conversation(&branch)
            .expect("valid branch creates");

        let history = store
            .conversation_history(&original.conversation_id)
            .expect("history reads");
        assert!(history.read_only);
        assert_eq!(history.turns, vec![first.clone()]);
        let original_relationships = store
            .conversation_relationships(&original.conversation_id)
            .expect("original relationships");
        assert_eq!(
            original_relationships.child_conversation_ids,
            vec![branch.conversation_id.clone()]
        );
        let branch_relationships = store
            .conversation_relationships(&branch.conversation_id)
            .expect("branch relationships");
        assert_eq!(
            branch_relationships.parent_conversation_id,
            Some(original.conversation_id)
        );
        assert_eq!(
            branch_relationships.branch_from_turn_id,
            Some(first.turn_id)
        );
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn invalid_search_bounds_fail_without_read_or_mutation() {
        let (directory, _path, mut store) = store();
        let conversation = conversation(true);
        store
            .create_conversation(&conversation)
            .expect("conversation creates");
        let invalid = ConversationQuery {
            from_local_date: Some("2027-02-01".to_owned()),
            to_local_date: Some("2027-01-01".to_owned()),
            limit: 1,
            ..ConversationQuery::default()
        };
        assert_eq!(
            store.search_conversations(&invalid),
            Err(ConversationLibraryError::InvalidInput)
        );
        assert_eq!(
            store
                .conversation_history(&conversation.conversation_id)
                .expect("history remains readable")
                .turns,
            Vec::new()
        );
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn metadata_changes_are_previewed_compare_and_swap_and_retention_visible() {
        let (directory, _path, mut store) = store();
        let conversation = conversation(true);
        store
            .create_conversation(&conversation)
            .expect("conversation creates");
        let rename = store
            .preview_conversation_change(
                &conversation.conversation_id,
                ConversationMetadataChange::Rename("Renamed review".to_owned()),
                conversation.updated_at_epoch_ms + 1,
                conversation.local_date.clone(),
            )
            .expect("rename previews");
        assert!(!rename.applied);
        assert_eq!(
            store
                .conversation(&conversation.conversation_id)
                .expect("conversation reads")
                .expect("conversation exists")
                .title,
            "Alpha review"
        );
        store
            .apply_conversation_change(&rename)
            .expect("rename applies");
        assert_eq!(
            store
                .conversation(&conversation.conversation_id)
                .expect("conversation reads")
                .expect("conversation exists")
                .title,
            "Renamed review"
        );
        assert_eq!(
            store.apply_conversation_change(&rename),
            Err(ConversationLibraryError::Conflict)
        );

        let tags = store
            .preview_conversation_change(
                &conversation.conversation_id,
                ConversationMetadataChange::ReplaceTags(vec![
                    "branch".to_owned(),
                    "review".to_owned(),
                ]),
                conversation.updated_at_epoch_ms + 2,
                conversation.local_date.clone(),
            )
            .expect("tags preview");
        store.apply_conversation_change(&tags).expect("tags apply");
        let retention = ConversationRetention {
            kind: ConversationRetentionKind::UserHold,
            expires_at_epoch_ms: None,
            policy_sha256: "d".repeat(64),
        };
        let hold = store
            .preview_conversation_change(
                &conversation.conversation_id,
                ConversationMetadataChange::SetRetention(retention.clone()),
                conversation.updated_at_epoch_ms + 3,
                conversation.local_date.clone(),
            )
            .expect("hold previews");
        store
            .apply_conversation_change(&hold)
            .expect("hold applies");
        let pin = store
            .preview_conversation_change(
                &conversation.conversation_id,
                ConversationMetadataChange::SetPinned(true),
                conversation.updated_at_epoch_ms + 4,
                conversation.local_date.clone(),
            )
            .expect("pin previews");
        store.apply_conversation_change(&pin).expect("pin applies");
        let archive = store
            .preview_conversation_change(
                &conversation.conversation_id,
                ConversationMetadataChange::Archive,
                conversation.updated_at_epoch_ms + 5,
                conversation.local_date.clone(),
            )
            .expect("archive previews");
        store
            .apply_conversation_change(&archive)
            .expect("archive applies");
        let loaded = store
            .conversation(&conversation.conversation_id)
            .expect("conversation reads")
            .expect("conversation exists");
        assert_eq!(loaded.tags, vec!["branch", "review"]);
        assert_eq!(loaded.retention, retention);
        assert!(loaded.pinned);
        assert_eq!(loaded.status, ConversationStatus::Archived);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn deletion_requires_exact_fresh_confirmation_and_removes_one_leaf_atomically() {
        let (directory, _path, mut store) = store();
        let conversation = conversation(true);
        let first = turn(1, Some("delete only after approval"));
        store
            .create_conversation(&conversation)
            .expect("conversation creates");
        store
            .append_conversation_turn(&first)
            .expect("turn appends");
        let preview = store
            .preview_conversation_deletion(&conversation.conversation_id)
            .expect("deletion previews");
        assert_eq!(preview.turn_count, 1);
        assert_eq!(preview.attachment_reference_count, 1);
        assert_eq!(preview.evidence_reference_count, 5);
        let denied = ConversationDeletionApproval {
            approval_id: ApprovalId::from_raw("approval-delete-1"),
            approved_preview_sha256: preview.preview_sha256.clone(),
            decision_sha256: "e".repeat(64),
            approved_at_epoch_ms: 1_800_000_000_100,
            user_confirmed: false,
        };
        assert_eq!(
            store.delete_conversation(&preview, &denied),
            Err(ConversationLibraryError::InvalidInput)
        );
        assert!(
            store
                .conversation(&conversation.conversation_id)
                .expect("conversation reads")
                .is_some()
        );
        let approval = ConversationDeletionApproval {
            user_confirmed: true,
            ..denied
        };
        let receipt = store
            .delete_conversation(&preview, &approval)
            .expect("approved deletion commits");
        assert!(receipt.deleted);
        assert_eq!(receipt.deleted_turn_count, 1);
        assert_eq!(store.conversation(&conversation.conversation_id), Ok(None));
        assert_eq!(store.conversation_turn(&first.turn_id), Ok(None));
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn child_branch_and_stale_preview_block_deletion_without_partial_effect() {
        let (directory, _path, mut store) = store();
        let original = conversation(true);
        let first = turn(1, Some("original"));
        store
            .create_conversation(&original)
            .expect("original creates");
        store
            .append_conversation_turn(&first)
            .expect("turn appends");
        let stale = store
            .preview_conversation_deletion(&original.conversation_id)
            .expect("initial preview");
        let mut branch = conversation(true);
        branch.conversation_id = ConversationId::from_raw("conversation-child");
        branch.parent_conversation_id = Some(original.conversation_id.clone());
        branch.branch_from_turn_id = Some(first.turn_id);
        store.create_conversation(&branch).expect("child creates");
        let approval = ConversationDeletionApproval {
            approval_id: ApprovalId::from_raw("approval-delete-parent"),
            approved_preview_sha256: stale.preview_sha256.clone(),
            decision_sha256: "f".repeat(64),
            approved_at_epoch_ms: 1_800_000_000_200,
            user_confirmed: true,
        };
        assert_eq!(
            store.delete_conversation(&stale, &approval),
            Err(ConversationLibraryError::Conflict)
        );
        let current = store
            .preview_conversation_deletion(&original.conversation_id)
            .expect("current preview");
        assert_eq!(
            current.blocking_child_conversation_ids,
            vec![branch.conversation_id]
        );
        assert!(
            store
                .conversation(&original.conversation_id)
                .expect("parent reads")
                .is_some()
        );
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn latest_resume_and_exact_branch_reuse_shared_drift_checks_without_rewriting_source() {
        let (directory, _path, mut store) = store();
        let original = conversation(true);
        let first = turn(1, Some("immutable source turn"));
        store
            .create_conversation(&original)
            .expect("original creates");
        store
            .append_conversation_turn(&first)
            .expect("source turn appends");
        let checkpoint = checkpoint();
        seed_checkpoint(&store, &checkpoint);
        let observation = resume_observation(&checkpoint);
        let latest = store
            .review_latest_conversation_resume(&original.conversation_id, &observation)
            .expect("latest resume reviews");
        assert_eq!(latest.turn_id, first.turn_id);
        assert_eq!(latest.directive, ResumeDirective::Continue);
        assert!(!latest.resumed);

        let mut branch = conversation(true);
        branch.conversation_id = ConversationId::from_raw("conversation-exact-branch");
        branch.title = "Exact branch".to_owned();
        branch.parent_conversation_id = Some(original.conversation_id.clone());
        branch.branch_from_turn_id = Some(first.turn_id.clone());
        branch.created_at_epoch_ms += 100;
        branch.updated_at_epoch_ms += 100;
        let preview = store
            .preview_conversation_branch(
                &original.conversation_id,
                &first.turn_id,
                branch.clone(),
                &observation,
            )
            .expect("branch previews");
        assert!(!preview.applied);
        store
            .apply_conversation_branch(&preview, &observation)
            .expect("branch applies");
        let source_history = store
            .conversation_history(&original.conversation_id)
            .expect("source history remains");
        assert_eq!(source_history.turns, vec![first.clone()]);
        let branch_history = store
            .conversation_history(&branch.conversation_id)
            .expect("branch history reads");
        assert!(branch_history.turns.is_empty());
        assert_eq!(
            branch_history.conversation.branch_from_turn_id,
            Some(first.turn_id)
        );
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn resume_drift_requires_a_decision_and_prevents_branch_creation() {
        let (directory, _path, mut store) = store();
        let original = conversation(true);
        let first = turn(1, Some("drift source"));
        store
            .create_conversation(&original)
            .expect("original creates");
        store
            .append_conversation_turn(&first)
            .expect("turn appends");
        let checkpoint = checkpoint();
        seed_checkpoint(&store, &checkpoint);
        let mut drifted = resume_observation(&checkpoint);
        drifted.instruction_sha256 = "d".repeat(64);
        let review = store
            .review_conversation_resume_from_turn(
                &original.conversation_id,
                &first.turn_id,
                &drifted,
            )
            .expect("drift reviews");
        assert!(matches!(
            review.directive,
            ResumeDirective::DecisionRequired { .. }
        ));
        let mut branch = conversation(true);
        branch.conversation_id = ConversationId::from_raw("conversation-drift-branch");
        branch.parent_conversation_id = Some(original.conversation_id.clone());
        branch.branch_from_turn_id = Some(first.turn_id.clone());
        branch.created_at_epoch_ms += 100;
        branch.updated_at_epoch_ms += 100;
        let preview = store
            .preview_conversation_branch(
                &original.conversation_id,
                &first.turn_id,
                branch.clone(),
                &drifted,
            )
            .expect("drifted preview remains visible");
        assert_eq!(
            store.apply_conversation_branch(&preview, &drifted),
            Err(ConversationLibraryError::Conflict)
        );
        assert_eq!(store.conversation(&branch.conversation_id), Ok(None));
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn stale_branch_preview_and_missing_checkpoint_fail_without_child_creation() {
        let (directory, _path, mut store) = store();
        let original = conversation(true);
        let first = turn(1, Some("checkpoint source"));
        store
            .create_conversation(&original)
            .expect("original creates");
        store
            .append_conversation_turn(&first)
            .expect("turn appends");
        let checkpoint = checkpoint();
        let observation = resume_observation(&checkpoint);
        assert_eq!(
            store.review_latest_conversation_resume(&original.conversation_id, &observation),
            Err(ConversationLibraryError::NotFound)
        );
        seed_checkpoint(&store, &checkpoint);
        let mut branch = conversation(true);
        branch.conversation_id = ConversationId::from_raw("conversation-stale-branch");
        branch.parent_conversation_id = Some(original.conversation_id.clone());
        branch.branch_from_turn_id = Some(first.turn_id.clone());
        branch.created_at_epoch_ms += 100;
        branch.updated_at_epoch_ms += 100;
        let preview = store
            .preview_conversation_branch(
                &original.conversation_id,
                &first.turn_id,
                branch.clone(),
                &observation,
            )
            .expect("branch previews");
        store
            .connection
            .execute(
                "UPDATE session_checkpoints SET record_json=X'7b7d' WHERE checkpoint_id=?1",
                [checkpoint.checkpoint_id.as_str()],
            )
            .expect("checkpoint tamper fixture");
        assert_eq!(
            store.apply_conversation_branch(&preview, &observation),
            Err(ConversationLibraryError::IntegrityFailure)
        );
        assert_eq!(store.conversation(&branch.conversation_id), Ok(None));
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn checked_compaction_preserves_all_references_and_original_turns() {
        let (directory, _path, mut store) = store();
        let conversation = conversation(true);
        let first = turn(1, Some("full original evidence remains"));
        store
            .create_conversation(&conversation)
            .expect("conversation creates");
        store
            .append_conversation_turn(&first)
            .expect("turn appends");
        let compaction = compaction(&first);
        let receipt = store
            .append_conversation_compaction(&compaction)
            .expect("compaction appends");
        assert!(receipt.stored);
        assert!(!receipt.source_turns_rewritten);
        assert_eq!(
            store.conversation_turn(&first.turn_id),
            Ok(Some(first.clone()))
        );
        let history = store
            .conversation_history(&conversation.conversation_id)
            .expect("history reads");
        assert_eq!(history.turns, vec![first]);
        assert_eq!(history.compactions, vec![compaction.clone()]);
        let hits = store
            .search_conversations(&ConversationQuery {
                text: Some("continuity summary".to_owned()),
                limit: 10,
                ..ConversationQuery::default()
            })
            .expect("summary search");
        assert_eq!(hits.len(), 1);
        assert_eq!(
            store.append_conversation_compaction(&compaction),
            Err(ConversationLibraryError::DuplicateIdentity)
        );
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn compaction_missing_citation_receipt_or_source_hash_is_rejected_without_state_change() {
        let (directory, _path, mut store) = store();
        let conversation = conversation(true);
        let first = turn(1, Some("source evidence"));
        store
            .create_conversation(&conversation)
            .expect("conversation creates");
        store
            .append_conversation_turn(&first)
            .expect("turn appends");
        let mut missing_citation = compaction(&first);
        missing_citation.summary.citation_ids.clear();
        assert_eq!(
            store.append_conversation_compaction(&missing_citation),
            Err(ConversationLibraryError::Conflict)
        );
        let mut missing_receipt = compaction(&first);
        missing_receipt.summary.receipt_ids.clear();
        assert_eq!(
            store.append_conversation_compaction(&missing_receipt),
            Err(ConversationLibraryError::Conflict)
        );
        let mut missing_source = compaction(&first);
        missing_source.source_sha256.clear();
        missing_source.source_hash_set_sha256 = source_hash_set_digest(&[]);
        missing_source.summary.source_set_sha256 = missing_source.source_hash_set_sha256.clone();
        assert_eq!(
            store.append_conversation_compaction(&missing_source),
            Err(ConversationLibraryError::Conflict)
        );
        assert!(
            store
                .conversation_history(&conversation.conversation_id)
                .expect("history reads")
                .compactions
                .is_empty()
        );
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn deletion_preview_counts_and_atomically_removes_checked_compactions() {
        let (directory, _path, mut store) = store();
        let conversation = conversation(true);
        let first = turn(1, Some("compacted deletion"));
        store
            .create_conversation(&conversation)
            .expect("conversation creates");
        store
            .append_conversation_turn(&first)
            .expect("turn appends");
        store
            .append_conversation_compaction(&compaction(&first))
            .expect("compaction appends");
        let preview = store
            .preview_conversation_deletion(&conversation.conversation_id)
            .expect("deletion previews");
        assert_eq!(preview.compaction_count, 1);
        let approval = ConversationDeletionApproval {
            approval_id: ApprovalId::from_raw("approval-compacted-delete"),
            approved_preview_sha256: preview.preview_sha256.clone(),
            decision_sha256: "f".repeat(64),
            approved_at_epoch_ms: first.created_at_epoch_ms + 100,
            user_confirmed: true,
        };
        let receipt = store
            .delete_conversation(&preview, &approval)
            .expect("deletion commits");
        assert_eq!(receipt.deleted_compaction_count, 1);
        fs::remove_dir_all(directory).expect("cleanup");
    }
}
