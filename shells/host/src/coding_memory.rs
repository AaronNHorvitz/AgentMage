//! Memory through the development catalog host (Decision 0131).
//!
//! The catalog host keeps one memory catalog for its state root, as an owner
//! state in its operational store. Each operation decodes and re-verifies the
//! stored catalog, applies one transition of the knowledge component and
//! commits the next state under the revision it read. A memory is created
//! only by the person's explicit invocation and cites a kept documentation
//! pack file as its evidence; the existing candidate policy decides whether
//! it may be approved. Revoking one item, or every item of one workspace that
//! cites a source, keeps the content for inspection and deletion and removes
//! the items from every load. Items of other workspaces are never touched.
//! Every refusal is closed and content-free and changes nothing. Nothing here
//! opens a network connection or grants authority.

use std::fmt::Write as _;

use agentmage_capability_knowledge::{
    MemoryCandidate, MemoryCandidateClass, MemoryCatalog, MemoryError, MemoryId, MemoryItem,
    MemoryItemStatus, MemoryPortableError, MemoryScope, MemorySourceRevocation, MemoryType,
    UserMemoryDecision, decode_memory_catalog_state, encode_memory_catalog_state,
    evaluate_memory_candidate, resolve_memory_candidate,
};
use agentmage_kernel_contracts::{
    DataSensitivity, EvidenceId, EvidenceKind, EvidenceReference, WorkspaceId,
};
use agentmage_kernel_engine::doc_pack_store::{DocPackStoreError, DurableDocPackCatalog};
use agentmage_kernel_engine::owner_state_store::{
    DurableOwnerStates, OwnerStateName, OwnerStateStoreError,
};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};

use agentmage_capability_knowledge::DocPackVersion;

const MAX_CONTENT_BYTES: usize = 16 * 1024;
const MAX_LABEL_BYTES: usize = 128;
const MAX_OBJECT_BYTES: usize = 256;
const MAX_PATH_BYTES: usize = 4_096;

/// An optional member that must still be present, as `null` when absent.
fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

/// Memory type a person may state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryTypeView {
    /// A source-backed fact.
    Semantic,
    /// A stated preference.
    Preference,
    /// A reviewed procedure.
    Procedural,
    /// A source-backed record of an event.
    Episodic,
}

impl MemoryTypeView {
    const fn kernel(self) -> MemoryType {
        match self {
            Self::Semantic => MemoryType::Semantic,
            Self::Preference => MemoryType::Preference,
            Self::Procedural => MemoryType::Procedural,
            Self::Episodic => MemoryType::Episodic,
        }
    }

    const fn of(value: MemoryType) -> Option<Self> {
        match value {
            MemoryType::Semantic => Some(Self::Semantic),
            MemoryType::Preference => Some(Self::Preference),
            MemoryType::Procedural => Some(Self::Procedural),
            MemoryType::Episodic => Some(Self::Episodic),
            MemoryType::Working => None,
        }
    }

    /// Parses a command-line type.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "semantic" => Some(Self::Semantic),
            "preference" => Some(Self::Preference),
            "procedural" => Some(Self::Procedural),
            "episodic" => Some(Self::Episodic),
            _ => None,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Semantic => "semantic",
            Self::Preference => "preference",
            Self::Procedural => "procedural",
            Self::Episodic => "episodic",
        }
    }
}

/// A kept documentation pack file a memory cites.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryCitation {
    /// Pack identity.
    pub pack_id: String,
    /// Kept version.
    pub version: DocPackVersion,
    /// File path inside the pack.
    pub path: String,
}

/// One memory request of a catalog client. Every member is required and no
/// other member is admitted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "kebab-case", deny_unknown_fields)]
pub enum MemoryRequest {
    /// The person states one memory citing a kept documentation pack file.
    Remember {
        /// The memory's text.
        content: String,
        /// Its type.
        memory_type: MemoryTypeView,
        /// The workspace label it belongs to.
        workspace_id: String,
        /// The kept file it cites.
        citation: MemoryCitation,
    },
    /// Every item, or every item of one workspace.
    List {
        /// One workspace, or every workspace.
        #[serde(deserialize_with = "required_option")]
        workspace_id: Option<String>,
    },
    /// Revokes one item.
    Revoke {
        /// The item.
        memory_id: String,
    },
    /// Revokes every revocable item of one workspace citing a source.
    RevokeSource {
        /// The workspace.
        workspace_id: String,
        /// The cited source.
        source_id: String,
        /// One object of the source, or every object.
        #[serde(deserialize_with = "required_option")]
        object_id: Option<String>,
    },
    /// Deletes one item, keeping a tombstone without its text.
    Delete {
        /// The item.
        memory_id: String,
    },
}

/// One cited source of an item.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemorySourceView {
    /// Source identity.
    pub source_id: String,
    /// Object identity within the source.
    pub object_id: String,
    /// Digest of the cited content.
    pub content_sha256: String,
}

/// Lifecycle state of an item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryStatusView {
    /// Approved and current.
    Approved,
    /// Rejected.
    Rejected,
    /// Superseded by another item.
    Superseded,
    /// Past its expiry.
    Expired,
    /// Held against expiry and deletion.
    Hold,
    /// Deleted; only a tombstone remains.
    Deleted,
    /// Revoked; its text is kept for inspection and deletion.
    Revoked,
    /// A candidate without a decision.
    Candidate,
}

impl MemoryStatusView {
    const fn of(value: MemoryItemStatus) -> Self {
        match value {
            MemoryItemStatus::Approved => Self::Approved,
            MemoryItemStatus::Rejected => Self::Rejected,
            MemoryItemStatus::Superseded => Self::Superseded,
            MemoryItemStatus::Expired => Self::Expired,
            MemoryItemStatus::Hold => Self::Hold,
            MemoryItemStatus::Deleted => Self::Deleted,
            MemoryItemStatus::Revoked => Self::Revoked,
            MemoryItemStatus::Candidate => Self::Candidate,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Superseded => "superseded",
            Self::Expired => "expired",
            Self::Hold => "hold",
            Self::Deleted => "deleted",
            Self::Revoked => "revoked",
            Self::Candidate => "candidate",
        }
    }
}

/// One item as the person sees it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryItemView {
    /// Identity.
    pub memory_id: String,
    /// Type, when the person may state it.
    #[serde(deserialize_with = "required_option")]
    pub memory_type: Option<MemoryTypeView>,
    /// Workspace label.
    pub workspace_id: String,
    /// Lifecycle state.
    pub status: MemoryStatusView,
    /// The text; none for a tombstone.
    #[serde(deserialize_with = "required_option")]
    pub content: Option<String>,
    /// Cited sources.
    pub sources: Vec<MemorySourceView>,
    /// When it was created.
    pub created_at: String,
    /// When it was last decided.
    pub decided_at: String,
}

/// The receipt of one transition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryTransitionView {
    /// Catalog revision after the transition.
    pub catalog_revision: u64,
    /// Digest of the catalog after the transition.
    pub catalog_sha256: String,
    /// Digest of the person's decision.
    pub decision_sha256: String,
    /// The items the transition changed.
    pub memory_ids: Vec<String>,
}

/// Why a memory candidate was refused by the policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MemoryCandidateRefusal {
    /// Secret, restricted, sensitive or machine-specific content.
    Prohibited,
    /// Temporary context.
    Temporary,
    /// Unresolved or low-confidence.
    Unresolved,
    /// Contradicts a current item.
    Contradiction,
}

/// Content-free reason a memory request was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MemoryRefusal {
    /// A value is malformed or out of bounds.
    InvalidInput,
    /// The policy refused the candidate.
    Candidate(MemoryCandidateRefusal),
    /// The cited pack version or file is not kept.
    CitationNotFound,
    /// The cited path cannot be named portably.
    CitationNotPortable,
    /// No such item, or no item cites the source in the workspace.
    NotFound,
    /// The item's state does not allow the transition.
    InvalidTransition,
    /// The catalog would exceed its bound.
    ResourceLimit,
    /// The host has no clock reading.
    ClockUnavailable,
    /// The store could not be opened, read or written.
    StoreUnavailable,
    /// The catalog changed underneath this operation.
    StoreConflict,
    /// The stored catalog failed its checks.
    StoreIntegrity,
}

impl MemoryRefusal {
    /// Stable content-free code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "memory.invalid-input",
            Self::Candidate(MemoryCandidateRefusal::Prohibited) => "memory.candidate.prohibited",
            Self::Candidate(MemoryCandidateRefusal::Temporary) => "memory.candidate.temporary",
            Self::Candidate(MemoryCandidateRefusal::Unresolved) => "memory.candidate.unresolved",
            Self::Candidate(MemoryCandidateRefusal::Contradiction) => {
                "memory.candidate.contradiction"
            }
            Self::CitationNotFound => "memory.citation-not-found",
            Self::CitationNotPortable => "memory.citation-not-portable",
            Self::NotFound => "memory.not-found",
            Self::InvalidTransition => "memory.invalid-transition",
            Self::ResourceLimit => "memory.resource-limit",
            Self::ClockUnavailable => "memory.clock-unavailable",
            Self::StoreUnavailable => "memory.store-unavailable",
            Self::StoreConflict => "memory.store-conflict",
            Self::StoreIntegrity => "memory.store-integrity",
        }
    }

    const fn of(error: MemoryError) -> Self {
        match error {
            MemoryError::NotFound => Self::NotFound,
            MemoryError::InvalidTransition => Self::InvalidTransition,
            MemoryError::ResourceLimit => Self::ResourceLimit,
            MemoryError::ProhibitedContent => Self::Candidate(MemoryCandidateRefusal::Prohibited),
            MemoryError::IneligibleCandidate => Self::Candidate(MemoryCandidateRefusal::Unresolved),
            MemoryError::InvalidInput
            | MemoryError::DecisionDrift
            | MemoryError::DuplicateIdentity => Self::InvalidInput,
        }
    }

    const fn of_store(error: OwnerStateStoreError) -> Self {
        match error {
            OwnerStateStoreError::Stale => Self::StoreConflict,
            OwnerStateStoreError::Integrity => Self::StoreIntegrity,
            OwnerStateStoreError::ResourceLimit => Self::ResourceLimit,
            OwnerStateStoreError::InvalidInput
            | OwnerStateStoreError::Storage
            | OwnerStateStoreError::Unavailable => Self::StoreUnavailable,
        }
    }
}

/// One memory answer of the catalog host. Every member is required and no
/// other member is admitted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "answer", rename_all = "kebab-case", deny_unknown_fields)]
pub enum MemoryAnswer {
    /// The memory was approved and kept.
    Remembered {
        /// The new item.
        item: MemoryItemView,
        /// The transition.
        receipt: MemoryTransitionView,
    },
    /// The requested items.
    Listed {
        /// The items, by identity.
        items: Vec<MemoryItemView>,
        /// Catalog revision.
        catalog_revision: u64,
    },
    /// The item was revoked, or the source's items in the workspace were.
    Revoked {
        /// The transition.
        receipt: MemoryTransitionView,
    },
    /// The item was deleted.
    Deleted {
        /// The transition.
        receipt: MemoryTransitionView,
    },
    /// The request was refused and changed nothing.
    Refused {
        /// Why.
        refusal: MemoryRefusal,
    },
}

/// The store handles a memory operation uses.
pub struct MemoryStores {
    /// The owner states that keep the memory catalog.
    pub states: DurableOwnerStates,
    /// The documentation catalog citations resolve in.
    pub doc_packs: DurableDocPackCatalog,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

/// The digest of the person's decision: the SHA-256 of the exact request in
/// its wire form.
#[must_use]
pub fn memory_decision_sha256(request: &MemoryRequest) -> Option<String> {
    serde_json::to_vec(request)
        .ok()
        .map(|bytes| sha256_hex(&bytes))
}

/// Whether an answer acknowledges exactly the request that was sent: the
/// matching kind, the decision digest of that request, the item it named or
/// remembered, and only the workspace it named.
#[must_use]
pub fn memory_answer_acknowledges(request: &MemoryRequest, answer: &MemoryAnswer) -> bool {
    let decision = memory_decision_sha256(request);
    let names = |receipt: &MemoryTransitionView, memory_id: &str| {
        Some(&receipt.decision_sha256) == decision.as_ref()
            && receipt.memory_ids.len() == 1
            && receipt.memory_ids[0] == memory_id
    };
    match (request, answer) {
        (_, MemoryAnswer::Refused { .. }) => true,
        (
            MemoryRequest::Remember {
                content,
                memory_type,
                workspace_id,
                ..
            },
            MemoryAnswer::Remembered { item, receipt },
        ) => {
            names(receipt, &item.memory_id)
                && item.content.as_ref() == Some(content)
                && item.memory_type == Some(*memory_type)
                && item.workspace_id == *workspace_id
                && item.status == MemoryStatusView::Approved
                && item.sources.len() == 1
        }
        (MemoryRequest::List { workspace_id }, MemoryAnswer::Listed { items, .. }) => {
            items.iter().all(|item| {
                workspace_id
                    .as_ref()
                    .is_none_or(|workspace| item.workspace_id == *workspace)
            })
        }
        (MemoryRequest::Revoke { memory_id }, MemoryAnswer::Revoked { receipt }) => {
            names(receipt, memory_id)
        }
        (MemoryRequest::Delete { memory_id }, MemoryAnswer::Deleted { receipt }) => {
            names(receipt, memory_id)
        }
        (MemoryRequest::RevokeSource { .. }, MemoryAnswer::Revoked { receipt }) => {
            Some(&receipt.decision_sha256) == decision.as_ref() && !receipt.memory_ids.is_empty()
        }
        _ => false,
    }
}

/// The UTC timestamp `YYYY-MM-DDTHH:MM:SSZ` of a Unix epoch time in
/// milliseconds.
#[must_use]
pub fn memory_timestamp(epoch_ms: u64) -> Option<String> {
    let date = crate::coding_doc_packs::iso_date_of_epoch_ms(epoch_ms)?;
    let seconds = (epoch_ms / 1_000) % 86_400;
    Some(format!(
        "{date}T{:02}:{:02}:{:02}Z",
        seconds / 3_600,
        seconds / 60 % 60,
        seconds % 60
    ))
}

/// Whether a value is a portable label: lowercase letters, digits, hyphens,
/// underscores, dots and colons, at most 128 bytes.
#[must_use]
pub fn portable_memory_label(value: &str) -> bool {
    portable_memory_identifier(value, MAX_LABEL_BYTES)
}

/// Whether a value is a portable object identity of at most 256 bytes.
#[must_use]
pub fn portable_memory_object(value: &str) -> bool {
    portable_memory_identifier(value, MAX_OBJECT_BYTES)
}

fn portable_memory_identifier(value: &str, max_bytes: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_bytes
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'-' | b'_' | b'.' | b':')
        })
}

/// Whether a value is a memory identity.
#[must_use]
pub fn memory_identity(value: &str) -> bool {
    MemoryId::parse(value).is_ok()
}

/// Whether a value is text a person may remember: not blank, no control
/// characters, at most 16 KiB.
#[must_use]
pub fn memory_text(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_CONTENT_BYTES
        && !value.chars().any(char::is_control)
}

/// The source identity of a documentation pack version.
#[must_use]
pub fn doc_pack_source_id(pack_id: &str, version: DocPackVersion) -> String {
    format!(
        "doc-pack:{pack_id}:{}.{}.{}",
        version.major, version.minor, version.patch
    )
}

/// The object identity of a pack file: `path:` and its components joined by
/// colons, when every component is portable.
#[must_use]
pub fn doc_pack_object_id(path: &str) -> Option<String> {
    let components = path.split('/').collect::<Vec<_>>();
    let portable = components.iter().all(|component| {
        !component.is_empty()
            && *component != "."
            && *component != ".."
            && component.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'-' | b'_' | b'.')
            })
    });
    let object = format!("path:{}", components.join(":"));
    (portable && object.len() <= MAX_OBJECT_BYTES).then_some(object)
}

fn item_view(item: &MemoryItem) -> MemoryItemView {
    MemoryItemView {
        memory_id: item.memory_id.as_str().to_owned(),
        memory_type: MemoryTypeView::of(item.memory_type),
        workspace_id: item.scope.workspace_id.as_str().to_owned(),
        status: MemoryStatusView::of(item.status),
        content: item.content.clone(),
        sources: item
            .evidence
            .iter()
            .map(|evidence| MemorySourceView {
                source_id: evidence.source_id.clone(),
                object_id: evidence.object_id.clone(),
                content_sha256: evidence.content_sha256.clone(),
            })
            .collect(),
        created_at: item.created_at.clone(),
        decided_at: item.decided_at.clone(),
    }
}

/// Answers one memory request. `open` opens the store for the operation;
/// `now_epoch_ms` is the host's clock and `memory_id` the identity a new item
/// takes, if the host has them.
pub fn answer_memory(
    request: &MemoryRequest,
    open: &mut dyn FnMut() -> Result<MemoryStores, MemoryRefusal>,
    now_epoch_ms: Option<u64>,
    memory_id: Option<String>,
) -> MemoryAnswer {
    match decide(request, open, now_epoch_ms, memory_id) {
        Ok(answer) => answer,
        Err(refusal) => MemoryAnswer::Refused { refusal },
    }
}

fn decide(
    request: &MemoryRequest,
    open: &mut dyn FnMut() -> Result<MemoryStores, MemoryRefusal>,
    now_epoch_ms: Option<u64>,
    memory_id: Option<String>,
) -> Result<MemoryAnswer, MemoryRefusal> {
    validate(request)?;
    let now = now_epoch_ms
        .and_then(memory_timestamp)
        .ok_or(MemoryRefusal::ClockUnavailable)?;
    let decision_sha256 = memory_decision_sha256(request).ok_or(MemoryRefusal::InvalidInput)?;
    let stores = open()?;
    let stored = stores
        .states
        .load(OwnerStateName::MemoryCatalog)
        .map_err(MemoryRefusal::of_store)?;
    let mut catalog = if stored.revision == 0 {
        MemoryCatalog::new()
    } else {
        decode_memory_catalog_state(&stored.state).map_err(|_| MemoryRefusal::StoreIntegrity)?
    };
    let (answer, changed) = match request {
        MemoryRequest::Remember {
            content,
            memory_type,
            workspace_id,
            citation,
        } => {
            let evidence = resolve_citation(&stores.doc_packs, citation)?;
            let memory_id = memory_id
                .and_then(|memory_id| MemoryId::parse(memory_id).ok())
                .ok_or(MemoryRefusal::StoreUnavailable)?;
            let candidate = MemoryCandidate {
                memory_id,
                memory_type: memory_type.kernel(),
                scope: MemoryScope {
                    workspace_id: WorkspaceId::from_raw(workspace_id.clone()),
                    project_id: None,
                    conversation_id: None,
                },
                content: content.clone(),
                fact_key: None,
                tags: Vec::new(),
                links: Vec::new(),
                evidence: vec![evidence],
                sensitivity: DataSensitivity::Durable,
                // The person states the memory: full confidence.
                confidence_bps: 10_000,
                created_at: now.clone(),
                expires_at: None,
                inferred_sensitive: false,
                model_requested_promotion: false,
            };
            let policy = evaluate_memory_candidate(&candidate, &catalog.items())
                .map_err(MemoryRefusal::of)?;
            if !policy.eligible_for_user_decision {
                return Err(MemoryRefusal::Candidate(match policy.classification {
                    MemoryCandidateClass::Prohibited => MemoryCandidateRefusal::Prohibited,
                    MemoryCandidateClass::TemporaryContext => MemoryCandidateRefusal::Temporary,
                    MemoryCandidateClass::Contradiction => MemoryCandidateRefusal::Contradiction,
                    _ => MemoryCandidateRefusal::Unresolved,
                }));
            }
            let item = resolve_memory_candidate(
                &candidate,
                &policy,
                UserMemoryDecision::Approve,
                decision_sha256.clone(),
                now,
            )
            .map_err(MemoryRefusal::of)?;
            let view = item_view(&item);
            let receipt = catalog.insert(item).map_err(MemoryRefusal::of)?;
            let answer = MemoryAnswer::Remembered {
                item: view.clone(),
                receipt: MemoryTransitionView {
                    catalog_revision: receipt.revision,
                    catalog_sha256: receipt.catalog_sha256,
                    decision_sha256,
                    memory_ids: vec![view.memory_id],
                },
            };
            (answer, true)
        }
        MemoryRequest::List { workspace_id } => {
            let items = catalog
                .items()
                .iter()
                .filter(|item| {
                    workspace_id
                        .as_ref()
                        .is_none_or(|workspace| item.scope.workspace_id.as_str() == workspace)
                })
                .map(item_view)
                .collect();
            let answer = MemoryAnswer::Listed {
                items,
                catalog_revision: catalog.inspect().revision,
            };
            (answer, false)
        }
        MemoryRequest::Revoke { memory_id } => {
            let memory_id =
                MemoryId::parse(memory_id.clone()).map_err(|_| MemoryRefusal::InvalidInput)?;
            let receipt = catalog
                .revoke(&memory_id, decision_sha256.clone(), now)
                .map_err(MemoryRefusal::of)?;
            let answer = MemoryAnswer::Revoked {
                receipt: MemoryTransitionView {
                    catalog_revision: receipt.revision,
                    catalog_sha256: receipt.catalog_sha256,
                    decision_sha256,
                    memory_ids: vec![memory_id.as_str().to_owned()],
                },
            };
            (answer, true)
        }
        MemoryRequest::RevokeSource {
            workspace_id,
            source_id,
            object_id,
        } => {
            let before = catalog.items();
            let receipt = catalog
                .revoke_source(
                    &MemorySourceRevocation {
                        workspace_id: WorkspaceId::from_raw(workspace_id.clone()),
                        source_id: source_id.clone(),
                        object_id: object_id.clone(),
                    },
                    decision_sha256.clone(),
                    now,
                )
                .map_err(MemoryRefusal::of)?;
            let after = catalog.items();
            let memory_ids = after
                .iter()
                .zip(&before)
                .filter(|(after, before)| after.status != before.status)
                .map(|(after, _)| after.memory_id.as_str().to_owned())
                .collect();
            let answer = MemoryAnswer::Revoked {
                receipt: MemoryTransitionView {
                    catalog_revision: receipt.revision,
                    catalog_sha256: receipt.catalog_sha256,
                    decision_sha256,
                    memory_ids,
                },
            };
            (answer, true)
        }
        MemoryRequest::Delete { memory_id } => {
            let memory_id =
                MemoryId::parse(memory_id.clone()).map_err(|_| MemoryRefusal::InvalidInput)?;
            let receipt = catalog
                .delete(&memory_id, decision_sha256.clone(), now)
                .map_err(MemoryRefusal::of)?;
            let answer = MemoryAnswer::Deleted {
                receipt: MemoryTransitionView {
                    catalog_revision: receipt.revision,
                    catalog_sha256: receipt.catalog_sha256,
                    decision_sha256,
                    memory_ids: vec![memory_id.as_str().to_owned()],
                },
            };
            (answer, true)
        }
    };
    if changed {
        // The portable rules refuse machine paths and credentials the
        // candidate policy does not look for; nothing is stored.
        let state = encode_memory_catalog_state(&catalog).map_err(|error| match error {
            MemoryPortableError::NonPortableData => {
                MemoryRefusal::Candidate(MemoryCandidateRefusal::Prohibited)
            }
            MemoryPortableError::ResourceLimit => MemoryRefusal::ResourceLimit,
            _ => MemoryRefusal::InvalidInput,
        })?;
        stores
            .states
            .commit(OwnerStateName::MemoryCatalog, stored.revision, &state)
            .map_err(MemoryRefusal::of_store)?;
    }
    Ok(answer)
}

/// Shapes and bounds of a request, before any store is opened.
fn validate(request: &MemoryRequest) -> Result<(), MemoryRefusal> {
    let valid = match request {
        MemoryRequest::Remember {
            content,
            workspace_id,
            citation,
            ..
        } => {
            memory_text(content)
                && portable_memory_label(workspace_id)
                && crate::coding_doc_packs::plain_doc_pack_id(&citation.pack_id)
                && !citation.path.is_empty()
                && citation.path.len() <= MAX_PATH_BYTES
        }
        MemoryRequest::List { workspace_id } => {
            workspace_id.as_deref().is_none_or(portable_memory_label)
        }
        MemoryRequest::Revoke { memory_id } | MemoryRequest::Delete { memory_id } => {
            memory_identity(memory_id)
        }
        MemoryRequest::RevokeSource {
            workspace_id,
            source_id,
            object_id,
        } => {
            portable_memory_label(workspace_id)
                && portable_memory_label(source_id)
                && object_id.as_deref().is_none_or(portable_memory_object)
        }
    };
    if valid {
        Ok(())
    } else {
        Err(MemoryRefusal::InvalidInput)
    }
}

/// The evidence of a kept documentation pack file: its source, its portable
/// object identity and its digest.
fn resolve_citation(
    doc_packs: &DurableDocPackCatalog,
    citation: &MemoryCitation,
) -> Result<EvidenceReference, MemoryRefusal> {
    let object_id = doc_pack_object_id(&citation.path).ok_or(MemoryRefusal::CitationNotPortable)?;
    let stored = doc_packs.load().map_err(|error| match error {
        DocPackStoreError::Integrity => MemoryRefusal::StoreIntegrity,
        _ => MemoryRefusal::StoreUnavailable,
    })?;
    let version = stored
        .contents
        .versions
        .iter()
        .find(|version| {
            version.key.pack_id == citation.pack_id
                && version.key.major == citation.version.major
                && version.key.minor == citation.version.minor
                && version.key.patch == citation.version.patch
        })
        .ok_or(MemoryRefusal::CitationNotFound)?;
    let file = version
        .files
        .iter()
        .find(|file| file.path == citation.path)
        .ok_or(MemoryRefusal::CitationNotFound)?;
    let source_id = doc_pack_source_id(&citation.pack_id, citation.version);
    if !portable_memory_label(&source_id) {
        return Err(MemoryRefusal::CitationNotPortable);
    }
    Ok(EvidenceReference {
        schema_version: 1,
        evidence_id: EvidenceId::from_raw(format!("evidence-{}", &file.sha256[..16])),
        kind: EvidenceKind::Document,
        source_id,
        object_id,
        fragment: None,
        content_sha256: file.sha256.clone(),
        observed_revision: Some(format!("manifest:{}", version.manifest_sha256)),
    })
}

/// Text of an item for a terminal, escaped.
fn escaped(text: &str) -> String {
    text.escape_debug().to_string()
}

fn render_item(item: &MemoryItemView, json: bool) -> String {
    if json {
        return format!(
            "{}\n",
            serde_json::json!({"type": "memory_item", "item": item})
        );
    }
    let mut output = format!(
        "memory {} [{}] {} in {}; created {}; decided {}\n",
        item.memory_id,
        item.status.name(),
        item.memory_type.map_or("other", MemoryTypeView::name),
        item.workspace_id,
        item.created_at,
        item.decided_at
    );
    match &item.content {
        Some(content) => {
            let _ = writeln!(output, "  | {}", escaped(content));
        }
        None => output.push_str("  | (no text kept)\n"),
    }
    for source in &item.sources {
        let _ = writeln!(
            output,
            "  cites {} {} ({})",
            source.source_id,
            source.object_id,
            &source.content_sha256[..source.content_sha256.len().min(12)]
        );
    }
    output
}

fn render_receipt(kind: &str, receipt: &MemoryTransitionView, json: bool) -> String {
    if json {
        format!(
            "{}\n",
            serde_json::json!({"type": format!("memory_{kind}"), "receipt": receipt})
        )
    } else {
        format!(
            "memory {kind}: {}; catalog revision {}; decision {}\n",
            if receipt.memory_ids.is_empty() {
                "no item".to_owned()
            } else {
                receipt.memory_ids.join(", ")
            },
            receipt.catalog_revision,
            &receipt.decision_sha256[..receipt.decision_sha256.len().min(12)]
        )
    }
}

/// Standard output for one answer. A refusal is written to standard error by
/// the caller.
#[must_use]
pub fn render_memory_answer(answer: &MemoryAnswer, json: bool) -> String {
    match answer {
        MemoryAnswer::Remembered { item, receipt } => {
            render_item(item, json) + &render_receipt("remembered", receipt, json)
        }
        MemoryAnswer::Listed {
            items,
            catalog_revision,
        } => {
            let mut output = items
                .iter()
                .map(|item| render_item(item, json))
                .collect::<String>();
            if json {
                let _ = writeln!(
                    output,
                    "{}",
                    serde_json::json!({"type": "memory_list", "items": items.len(), "catalog_revision": catalog_revision})
                );
            } else {
                let _ = writeln!(
                    output,
                    "memory: {} items; catalog revision {catalog_revision}",
                    items.len()
                );
            }
            output
        }
        MemoryAnswer::Revoked { receipt } => render_receipt("revoked", receipt, json),
        MemoryAnswer::Deleted { receipt } => render_receipt("deleted", receipt, json),
        MemoryAnswer::Refused { .. } => String::new(),
    }
}

/// One refusal line for standard error.
#[must_use]
pub fn render_memory_refusal(refusal: MemoryRefusal, json: bool) -> String {
    if json {
        format!(
            "{}\n",
            serde_json::json!({"type": "memory_refused", "code": refusal.code()})
        )
    } else {
        format!("memory refused: {}\n", refusal.code())
    }
}

/// The closed process exit class of a refusal.
#[must_use]
pub const fn memory_refusal_exit(refusal: MemoryRefusal) -> crate::headless::ClientExitCode {
    use crate::headless::ClientExitCode;
    match refusal {
        MemoryRefusal::InvalidInput | MemoryRefusal::CitationNotPortable => {
            ClientExitCode::InvalidInput
        }
        MemoryRefusal::Candidate(_)
        | MemoryRefusal::CitationNotFound
        | MemoryRefusal::NotFound
        | MemoryRefusal::InvalidTransition => ClientExitCode::PolicyDenied,
        MemoryRefusal::ResourceLimit => ClientExitCode::ResourceBound,
        MemoryRefusal::ClockUnavailable
        | MemoryRefusal::StoreUnavailable
        | MemoryRefusal::StoreConflict
        | MemoryRefusal::StoreIntegrity => ClientExitCode::ServiceUnavailable,
    }
}

/// Parses a citation `PACK@MAJOR.MINOR.PATCH:PATH`.
#[must_use]
pub fn parse_memory_citation(value: &str) -> Option<MemoryCitation> {
    let (pack_id, rest) = value.split_once('@')?;
    let (version, path) = rest.split_once(':')?;
    let version = crate::coding_doc_packs::parse_doc_pack_version(version)?;
    (crate::coding_doc_packs::plain_doc_pack_id(pack_id)
        && !path.is_empty()
        && path.len() <= MAX_PATH_BYTES)
        .then(|| MemoryCitation {
            pack_id: pack_id.to_owned(),
            version,
            path: path.to_owned(),
        })
}

#[cfg(all(
    test,
    target_os = "linux",
    feature = "source-artifacts",
    feature = "workflow-supervisor"
))]
#[path = "coding_memory_tests.rs"]
mod tests;
