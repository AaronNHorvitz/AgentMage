//! Canonical encrypted conversation storage behind the kernel boundary.

use std::fmt::Write as _;

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, ConversationId, ConversationRecord, ConversationStatus,
    ConversationTurn, ConversationTurnId, DataSensitivity, WorkspaceId, from_json,
    to_canonical_json,
};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use sha2::{Digest, Sha256};

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
                    branch_from_turn_id, current_turn_id, pinned, persistence_enabled,
                    record_sha256, record_json
                 ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                    ?13, NULL, ?14, ?15, ?16, ?17
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
                Some(text) => search_preview(&conversation, &history.turns, text),
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
        Ok(ConversationHistory {
            conversation,
            turns,
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
    None
}

fn bounded_preview(value: &str) -> String {
    value.chars().take(MAX_RESULT_PREVIEW_CHARS).collect()
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
               AND current_turn_id IS ?14 AND pinned=?15 AND persistence_enabled=?16",
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
        CONTRACT_SCHEMA_VERSION, CloudSynchronizationMarker, ConversationAttachmentReference,
        ConversationId, ConversationRecord, ConversationStatus, ConversationTurn,
        ConversationTurnId, ConversationTurnRole, DataSensitivity, GrantId, ModelProfileId,
        ReceiptId, SessionCheckpointId, StorageFilesystemClass, StrictLocalStorageObservation,
        WorkspaceId,
    };

    use super::{ConversationLibraryError, ConversationQuery, sha256};
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
}
