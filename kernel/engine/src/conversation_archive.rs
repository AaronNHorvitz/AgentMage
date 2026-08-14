//! Separately keyed private conversation archives and controlled lifecycle operations.

use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::Read as _;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use agentmage_kernel_contracts::{
    ApprovalId, CONTRACT_SCHEMA_VERSION, ConversationId, StrictLocalStorageObservation,
    to_canonical_json,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::conversation_library::ConversationLibraryError;
use crate::operational_store::{
    OperationalStore, OperationalStoreError, OperationalStoreKeyProvider, RestoreCandidateReceipt,
};
use crate::strict_local::{StrictLocalStorageDecision, evaluate_storage};

const MAX_ARCHIVE_CONVERSATIONS: usize = 100_000;
const MAX_ARCHIVE_ID_BYTES: usize = 128;

/// Stable content-free private-archive failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConversationArchiveError {
    /// A request, manifest, preview, approval, or retention value is malformed.
    InvalidInput,
    /// Current encrypted bytes or canonical inventory differ from the reviewed state.
    IntegrityFailure,
    /// The selected storage location is not an admitted strict-local private file.
    StorageRejected,
    /// The separately scoped archive key is unavailable or incorrect.
    KeyUnavailable,
    /// A retained hold or future expiry currently prevents deletion.
    RetentionBlocked,
    /// The reviewed state became stale before a requested lifecycle operation.
    StaleReview,
    /// An encrypted backup, restore, or file lifecycle operation failed.
    OperationFailed,
}

impl ConversationArchiveError {
    /// Returns a stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "conversation_archive.input.invalid",
            Self::IntegrityFailure => "conversation_archive.integrity.failed",
            Self::StorageRejected => "conversation_archive.storage.rejected",
            Self::KeyUnavailable => "conversation_archive.key.unavailable",
            Self::RetentionBlocked => "conversation_archive.retention.blocked",
            Self::StaleReview => "conversation_archive.review.stale",
            Self::OperationFailed => "conversation_archive.operation.failed",
        }
    }
}

impl std::fmt::Display for ConversationArchiveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for ConversationArchiveError {}

/// Visible lifecycle policy for one encrypted private archive.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConversationArchiveRetention {
    /// Earliest trusted time at which deletion may proceed, when no hold is active.
    pub delete_after_epoch_ms: u64,
    /// Explicit hold that blocks deletion until a reviewed retention revision clears it.
    pub user_hold: bool,
    /// Digest of the policy or user decision establishing this lifecycle rule.
    pub policy_sha256: String,
}

/// Exact request for one separately keyed snapshot of all canonical conversations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversationArchiveRequest {
    /// Stable archive identity.
    pub archive_id: String,
    /// Trusted creation time as Unix epoch milliseconds.
    pub created_at_epoch_ms: u64,
    /// Exact sorted conversation identities the user reviewed for inclusion.
    pub expected_conversation_ids: Vec<ConversationId>,
    /// Initial visible retention rule.
    pub retention: ConversationArchiveRetention,
}

/// Content-free manifest binding encrypted bytes to the reviewed canonical inventory.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ConversationArchiveManifest {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable archive identity.
    pub archive_id: String,
    /// Monotonic manifest revision; encrypted snapshot bytes remain immutable.
    pub revision: u64,
    /// Trusted archive creation time.
    pub created_at_epoch_ms: u64,
    /// SQLCipher schema copied into the archive.
    pub encrypted_store_schema_version: u32,
    /// Canonical authority generation copied into the archive.
    pub canonical_generation: u64,
    /// Exact sorted conversation identities included in the snapshot.
    pub conversation_ids: Vec<ConversationId>,
    /// Number of immutable conversation turns in the archive.
    pub turn_count: u64,
    /// Number of checked compaction records in the archive.
    pub compaction_count: u64,
    /// Digest of the complete hash-verified conversation inventory.
    pub source_inventory_sha256: String,
    /// SHA-256 of the separately keyed encrypted archive file.
    pub encrypted_file_sha256: String,
    /// Exact encrypted byte count.
    pub encrypted_file_bytes: u64,
    /// Current visible lifecycle rule.
    pub retention: ConversationArchiveRetention,
    /// Digest of every preceding manifest field.
    pub manifest_sha256: String,
}

/// Read-only proof that encrypted bytes reopen to the exact reviewed inventory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversationArchiveInspection {
    /// Stable archive identity.
    pub archive_id: String,
    /// Exact manifest digest inspected.
    pub manifest_sha256: String,
    /// Exact encrypted file digest inspected.
    pub encrypted_file_sha256: String,
    /// Number of verified conversations.
    pub conversation_count: u64,
    /// Number of verified immutable turns.
    pub turn_count: u64,
    /// Number of verified checked compactions.
    pub compaction_count: u64,
    /// Fixed true marker after SQLCipher and canonical inventory verification.
    pub verified: bool,
    /// Fixed false marker: inspection never restores or changes live authority.
    pub restored: bool,
}

/// Compare-and-swap preview for a content-free archive-retention revision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversationArchiveRetentionPreview {
    /// Stable archive identity.
    pub archive_id: String,
    /// Exact current manifest digest.
    pub expected_manifest_sha256: String,
    /// Complete proposed replacement manifest.
    pub proposed_manifest: ConversationArchiveManifest,
    /// Digest binding current and proposed state.
    pub preview_sha256: String,
    /// Fixed false marker: preview construction changes nothing.
    pub applied: bool,
}

/// Exact deletion preview over one immutable encrypted archive file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversationArchiveDeletionPreview {
    /// Stable archive identity.
    pub archive_id: String,
    /// Exact current manifest digest.
    pub expected_manifest_sha256: String,
    /// Exact encrypted file digest.
    pub encrypted_file_sha256: String,
    /// Exact encrypted byte count.
    pub encrypted_file_bytes: u64,
    /// Whether the current retention rule blocks deletion.
    pub retention_blocked: bool,
    /// Digest binding all preview fields.
    pub preview_sha256: String,
    /// Fixed false marker: preview construction removes nothing.
    pub deleted: bool,
}

/// Explicit approval bound to one exact archive deletion preview.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversationArchiveDeletionApproval {
    /// Stable approval identity produced by the guarded approval boundary.
    pub approval_id: ApprovalId,
    /// Exact deletion preview digest approved by the user.
    pub approved_preview_sha256: String,
    /// Digest of the explicit user decision evidence.
    pub decision_sha256: String,
    /// Trusted approval time as Unix epoch milliseconds.
    pub approved_at_epoch_ms: u64,
    /// Explicit confirmation marker; false approvals are inert.
    pub user_confirmed: bool,
}

/// Content-free result of one approved archive deletion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversationArchiveDeletionReceipt {
    /// Digest of the stable archive identity.
    pub archive_id_sha256: String,
    /// Exact approved preview digest.
    pub preview_sha256: String,
    /// Digest of the approval identity.
    pub approval_id_sha256: String,
    /// Exact encrypted bytes removed.
    pub deleted_file_bytes: u64,
    /// Fixed true marker after the selected archive file is absent.
    pub deleted: bool,
    /// Always false because storage media overwrite is not claimed.
    pub physical_overwrite_claim: bool,
}

/// Content-free result of restoring one archive into a fresh candidate store.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConversationArchiveRestoreReceipt {
    /// Stable archive identity.
    pub archive_id: String,
    /// Exact source manifest digest.
    pub manifest_sha256: String,
    /// Underlying SQLCipher fresh-candidate receipt.
    pub candidate: RestoreCandidateReceipt,
    /// Fixed true marker after restored canonical inventory verification.
    pub inventory_verified: bool,
    /// Fixed false marker: the live store was not replaced.
    pub live_store_replaced: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ArchiveInventory {
    conversation_ids: Vec<ConversationId>,
    turn_count: u64,
    compaction_count: u64,
    inventory_sha256: String,
}

impl OperationalStore {
    /// Creates a separately keyed encrypted archive after exact inventory review.
    pub fn create_conversation_archive<P: OperationalStoreKeyProvider>(
        &self,
        destination: &Path,
        observation: &StrictLocalStorageObservation,
        provider: &mut P,
        request: &ConversationArchiveRequest,
    ) -> Result<ConversationArchiveManifest, ConversationArchiveError> {
        validate_archive_request(request)?;
        let inventory = self.conversation_archive_inventory()?;
        if inventory.conversation_ids != request.expected_conversation_ids {
            return Err(ConversationArchiveError::StaleReview);
        }
        let receipt = self
            .backup(destination, observation, provider)
            .map_err(map_store_error)?;
        let encrypted_file_bytes = private_file_size(destination)?;
        let mut manifest = ConversationArchiveManifest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            archive_id: request.archive_id.clone(),
            revision: 1,
            created_at_epoch_ms: request.created_at_epoch_ms,
            encrypted_store_schema_version: receipt.schema_version,
            canonical_generation: receipt.generation,
            conversation_ids: inventory.conversation_ids,
            turn_count: inventory.turn_count,
            compaction_count: inventory.compaction_count,
            source_inventory_sha256: inventory.inventory_sha256,
            encrypted_file_sha256: receipt.encrypted_file_sha256,
            encrypted_file_bytes,
            retention: request.retention.clone(),
            manifest_sha256: String::new(),
        };
        manifest.manifest_sha256 = archive_manifest_digest(&manifest)?;
        validate_archive_manifest(&manifest)?;
        Ok(manifest)
    }

    /// Reopens and verifies an encrypted archive without restoring live authority.
    pub fn inspect_conversation_archive<P: OperationalStoreKeyProvider>(
        archive: &Path,
        observation: &StrictLocalStorageObservation,
        provider: &mut P,
        manifest: &ConversationArchiveManifest,
    ) -> Result<ConversationArchiveInspection, ConversationArchiveError> {
        validate_archive_manifest(manifest)?;
        verify_archive_file(archive, observation, manifest)?;
        let store = Self::open(archive, observation, provider).map_err(map_store_error)?;
        let inventory = store.conversation_archive_inventory()?;
        verify_inventory_matches_manifest(&inventory, manifest)?;
        drop(store);
        Ok(ConversationArchiveInspection {
            archive_id: manifest.archive_id.clone(),
            manifest_sha256: manifest.manifest_sha256.clone(),
            encrypted_file_sha256: manifest.encrypted_file_sha256.clone(),
            conversation_count: manifest.conversation_ids.len() as u64,
            turn_count: manifest.turn_count,
            compaction_count: manifest.compaction_count,
            verified: true,
            restored: false,
        })
    }

    /// Restores one verified archive into a separately keyed fresh candidate only.
    pub fn restore_conversation_archive_to_fresh_candidate<
        SP: OperationalStoreKeyProvider,
        DP: OperationalStoreKeyProvider,
    >(
        archive: &Path,
        archive_observation: &StrictLocalStorageObservation,
        archive_provider: &mut SP,
        manifest: &ConversationArchiveManifest,
        destination: &Path,
        destination_observation: &StrictLocalStorageObservation,
        destination_provider: &mut DP,
    ) -> Result<ConversationArchiveRestoreReceipt, ConversationArchiveError> {
        let _ = Self::inspect_conversation_archive(
            archive,
            archive_observation,
            archive_provider,
            manifest,
        )?;
        let candidate = Self::restore_to_fresh_candidate(
            archive,
            archive_observation,
            archive_provider,
            destination,
            destination_observation,
            destination_provider,
        )
        .map_err(map_store_error)?;
        let restored = Self::open(destination, destination_observation, destination_provider)
            .map_err(map_store_error)?;
        let inventory = restored.conversation_archive_inventory()?;
        verify_inventory_matches_manifest(&inventory, manifest)?;
        drop(restored);
        Ok(ConversationArchiveRestoreReceipt {
            archive_id: manifest.archive_id.clone(),
            manifest_sha256: manifest.manifest_sha256.clone(),
            candidate,
            inventory_verified: true,
            live_store_replaced: false,
        })
    }

    fn conversation_archive_inventory(&self) -> Result<ArchiveInventory, ConversationArchiveError> {
        let mut statement = self
            .connection
            .prepare("SELECT conversation_id FROM conversations ORDER BY conversation_id")
            .map_err(|_| ConversationArchiveError::IntegrityFailure)?;
        let identities = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|_| ConversationArchiveError::IntegrityFailure)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ConversationArchiveError::IntegrityFailure)?;
        if identities.len() > MAX_ARCHIVE_CONVERSATIONS {
            return Err(ConversationArchiveError::IntegrityFailure);
        }
        let mut conversation_ids = Vec::with_capacity(identities.len());
        let mut turn_count = 0_u64;
        let mut compaction_count = 0_u64;
        let mut digest = Sha256::new();
        digest.update(b"agentmage-conversation-archive-inventory-v1\n");
        for identity in identities {
            let conversation_id = ConversationId::from_raw(identity);
            let history = self
                .conversation_history(&conversation_id)
                .map_err(map_conversation_error)?;
            turn_count = turn_count
                .checked_add(history.turns.len() as u64)
                .ok_or(ConversationArchiveError::IntegrityFailure)?;
            compaction_count = compaction_count
                .checked_add(history.compactions.len() as u64)
                .ok_or(ConversationArchiveError::IntegrityFailure)?;
            digest.update(
                to_canonical_json(&history.conversation)
                    .map_err(|_| ConversationArchiveError::IntegrityFailure)?,
            );
            for turn in &history.turns {
                digest.update(
                    to_canonical_json(turn)
                        .map_err(|_| ConversationArchiveError::IntegrityFailure)?,
                );
            }
            for compaction in &history.compactions {
                digest.update(
                    to_canonical_json(compaction)
                        .map_err(|_| ConversationArchiveError::IntegrityFailure)?,
                );
            }
            conversation_ids.push(conversation_id);
        }
        Ok(ArchiveInventory {
            conversation_ids,
            turn_count,
            compaction_count,
            inventory_sha256: hex_digest(&digest.finalize()),
        })
    }
}

/// Builds an exact no-write preview for changing only archive retention metadata.
pub fn preview_conversation_archive_retention(
    current: &ConversationArchiveManifest,
    proposed_retention: ConversationArchiveRetention,
) -> Result<ConversationArchiveRetentionPreview, ConversationArchiveError> {
    validate_archive_manifest(current)?;
    validate_retention(&proposed_retention, current.created_at_epoch_ms)?;
    let mut proposed = current.clone();
    proposed.revision = proposed
        .revision
        .checked_add(1)
        .ok_or(ConversationArchiveError::InvalidInput)?;
    proposed.retention = proposed_retention;
    proposed.manifest_sha256.clear();
    proposed.manifest_sha256 = archive_manifest_digest(&proposed)?;
    let preview_sha256 = digest(&(
        "agentmage-conversation-archive-retention-preview-v1",
        &current.manifest_sha256,
        &proposed.manifest_sha256,
    ))?;
    Ok(ConversationArchiveRetentionPreview {
        archive_id: current.archive_id.clone(),
        expected_manifest_sha256: current.manifest_sha256.clone(),
        proposed_manifest: proposed,
        preview_sha256,
        applied: false,
    })
}

/// Applies a retention revision only when the current manifest still matches the preview.
pub fn apply_conversation_archive_retention(
    current: &ConversationArchiveManifest,
    preview: &ConversationArchiveRetentionPreview,
) -> Result<ConversationArchiveManifest, ConversationArchiveError> {
    let fresh = preview_conversation_archive_retention(
        current,
        preview.proposed_manifest.retention.clone(),
    )?;
    if &fresh != preview {
        return Err(ConversationArchiveError::StaleReview);
    }
    Ok(preview.proposed_manifest.clone())
}

/// Builds an exact no-write preview for deleting one selected encrypted archive.
pub fn preview_conversation_archive_deletion(
    archive: &Path,
    observation: &StrictLocalStorageObservation,
    manifest: &ConversationArchiveManifest,
    now_epoch_ms: u64,
) -> Result<ConversationArchiveDeletionPreview, ConversationArchiveError> {
    if now_epoch_ms == 0 {
        return Err(ConversationArchiveError::InvalidInput);
    }
    validate_archive_manifest(manifest)?;
    verify_archive_file(archive, observation, manifest)?;
    let retention_blocked =
        manifest.retention.user_hold || now_epoch_ms < manifest.retention.delete_after_epoch_ms;
    let preview_sha256 = digest(&(
        "agentmage-conversation-archive-deletion-preview-v1",
        &manifest.archive_id,
        &manifest.manifest_sha256,
        &manifest.encrypted_file_sha256,
        manifest.encrypted_file_bytes,
        retention_blocked,
    ))?;
    Ok(ConversationArchiveDeletionPreview {
        archive_id: manifest.archive_id.clone(),
        expected_manifest_sha256: manifest.manifest_sha256.clone(),
        encrypted_file_sha256: manifest.encrypted_file_sha256.clone(),
        encrypted_file_bytes: manifest.encrypted_file_bytes,
        retention_blocked,
        preview_sha256,
        deleted: false,
    })
}

/// Deletes one exact encrypted archive after retention release and bound approval.
pub fn delete_conversation_archive(
    archive: &Path,
    observation: &StrictLocalStorageObservation,
    manifest: &ConversationArchiveManifest,
    preview: &ConversationArchiveDeletionPreview,
    approval: &ConversationArchiveDeletionApproval,
    now_epoch_ms: u64,
) -> Result<ConversationArchiveDeletionReceipt, ConversationArchiveError> {
    let fresh =
        preview_conversation_archive_deletion(archive, observation, manifest, now_epoch_ms)?;
    if &fresh != preview
        || approval.approved_preview_sha256 != preview.preview_sha256
        || !valid_identifier(approval.approval_id.as_str())
        || !valid_sha256(&approval.decision_sha256)
        || approval.approved_at_epoch_ms == 0
        || approval.approved_at_epoch_ms > now_epoch_ms
        || !approval.user_confirmed
    {
        return Err(ConversationArchiveError::StaleReview);
    }
    if preview.retention_blocked {
        return Err(ConversationArchiveError::RetentionBlocked);
    }
    fs::remove_file(archive).map_err(|_| ConversationArchiveError::OperationFailed)?;
    if archive.exists() {
        return Err(ConversationArchiveError::OperationFailed);
    }
    Ok(ConversationArchiveDeletionReceipt {
        archive_id_sha256: sha256(manifest.archive_id.as_bytes()),
        preview_sha256: preview.preview_sha256.clone(),
        approval_id_sha256: sha256(approval.approval_id.as_str().as_bytes()),
        deleted_file_bytes: preview.encrypted_file_bytes,
        deleted: true,
        physical_overwrite_claim: false,
    })
}

fn verify_archive_file(
    archive: &Path,
    observation: &StrictLocalStorageObservation,
    manifest: &ConversationArchiveManifest,
) -> Result<(), ConversationArchiveError> {
    if evaluate_storage(observation) != StrictLocalStorageDecision::Eligible {
        return Err(ConversationArchiveError::StorageRejected);
    }
    if private_file_size(archive)? != manifest.encrypted_file_bytes
        || private_file_sha256(archive)? != manifest.encrypted_file_sha256
    {
        return Err(ConversationArchiveError::IntegrityFailure);
    }
    Ok(())
}

fn private_file_size(path: &Path) -> Result<u64, ConversationArchiveError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| ConversationArchiveError::OperationFailed)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.mode() & 0o077 != 0 {
        return Err(ConversationArchiveError::StorageRejected);
    }
    Ok(metadata.len())
}

fn private_file_sha256(path: &Path) -> Result<String, ConversationArchiveError> {
    let _ = private_file_size(path)?;
    let mut file = OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|_| ConversationArchiveError::OperationFailed)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| ConversationArchiveError::OperationFailed)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(hex_digest(&digest.finalize()))
}

fn verify_inventory_matches_manifest(
    inventory: &ArchiveInventory,
    manifest: &ConversationArchiveManifest,
) -> Result<(), ConversationArchiveError> {
    if inventory.conversation_ids != manifest.conversation_ids
        || inventory.turn_count != manifest.turn_count
        || inventory.compaction_count != manifest.compaction_count
        || inventory.inventory_sha256 != manifest.source_inventory_sha256
    {
        return Err(ConversationArchiveError::IntegrityFailure);
    }
    Ok(())
}

fn validate_archive_request(
    request: &ConversationArchiveRequest,
) -> Result<(), ConversationArchiveError> {
    if !valid_identifier(&request.archive_id) || request.created_at_epoch_ms == 0 {
        return Err(ConversationArchiveError::InvalidInput);
    }
    validate_retention(&request.retention, request.created_at_epoch_ms)?;
    validate_sorted_conversation_ids(&request.expected_conversation_ids)
}

fn validate_archive_manifest(
    manifest: &ConversationArchiveManifest,
) -> Result<(), ConversationArchiveError> {
    if manifest.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_identifier(&manifest.archive_id)
        || manifest.revision == 0
        || manifest.created_at_epoch_ms == 0
        || manifest.encrypted_store_schema_version == 0
        || manifest.encrypted_file_bytes == 0
        || !valid_sha256(&manifest.source_inventory_sha256)
        || !valid_sha256(&manifest.encrypted_file_sha256)
        || !valid_sha256(&manifest.manifest_sha256)
    {
        return Err(ConversationArchiveError::InvalidInput);
    }
    validate_sorted_conversation_ids(&manifest.conversation_ids)?;
    validate_retention(&manifest.retention, manifest.created_at_epoch_ms)?;
    if manifest.manifest_sha256 != archive_manifest_digest(manifest)? {
        return Err(ConversationArchiveError::IntegrityFailure);
    }
    Ok(())
}

fn validate_sorted_conversation_ids(
    values: &[ConversationId],
) -> Result<(), ConversationArchiveError> {
    if values.len() > MAX_ARCHIVE_CONVERSATIONS
        || values.iter().any(|value| !valid_identifier(value.as_str()))
        || values.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(ConversationArchiveError::InvalidInput);
    }
    let unique = values
        .iter()
        .map(ConversationId::as_str)
        .collect::<BTreeSet<_>>();
    if unique.len() != values.len() {
        return Err(ConversationArchiveError::InvalidInput);
    }
    Ok(())
}

fn validate_retention(
    retention: &ConversationArchiveRetention,
    created_at_epoch_ms: u64,
) -> Result<(), ConversationArchiveError> {
    if retention.delete_after_epoch_ms < created_at_epoch_ms
        || !valid_sha256(&retention.policy_sha256)
    {
        return Err(ConversationArchiveError::InvalidInput);
    }
    Ok(())
}

fn archive_manifest_digest(
    manifest: &ConversationArchiveManifest,
) -> Result<String, ConversationArchiveError> {
    digest(&(
        "agentmage-conversation-archive-manifest-v1",
        manifest.schema_version,
        &manifest.archive_id,
        manifest.revision,
        manifest.created_at_epoch_ms,
        manifest.encrypted_store_schema_version,
        manifest.canonical_generation,
        &manifest.conversation_ids,
        manifest.turn_count,
        manifest.compaction_count,
        &manifest.source_inventory_sha256,
        &manifest.encrypted_file_sha256,
        manifest.encrypted_file_bytes,
        &manifest.retention,
    ))
}

fn digest<T: Serialize>(value: &T) -> Result<String, ConversationArchiveError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| ConversationArchiveError::InvalidInput)
}

fn sha256(value: &[u8]) -> String {
    hex_digest(&Sha256::digest(value))
}

fn hex_digest(value: &[u8]) -> String {
    let mut encoded = String::with_capacity(value.len() * 2);
    for byte in value {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to a string cannot fail");
    }
    encoded
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ARCHIVE_ID_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn map_store_error(error: OperationalStoreError) -> ConversationArchiveError {
    match error {
        OperationalStoreError::StorageRejected => ConversationArchiveError::StorageRejected,
        OperationalStoreError::KeyUnavailable | OperationalStoreError::InvalidKey => {
            ConversationArchiveError::KeyUnavailable
        }
        OperationalStoreError::IntegrityFailure => ConversationArchiveError::IntegrityFailure,
        _ => ConversationArchiveError::OperationFailed,
    }
}

fn map_conversation_error(error: ConversationLibraryError) -> ConversationArchiveError {
    match error {
        ConversationLibraryError::IntegrityFailure => ConversationArchiveError::IntegrityFailure,
        _ => ConversationArchiveError::OperationFailed,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::{
        CloudSynchronizationMarker, ConversationRecord, ConversationRetention,
        ConversationRetentionKind, ConversationStatus, ConversationTurn, ConversationTurnId,
        ConversationTurnRole, DataSensitivity, ModelProfileId, StorageFilesystemClass, WorkspaceId,
    };

    use super::*;
    use crate::operational_store::{OperationalStoreKeyError, OperationalStoreKeyProvider};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);
    const CREATED: u64 = 1_800_000_000_000;
    const CANARY: &str = "private archive plaintext canary";

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

    fn fixture() -> (std::path::PathBuf, OperationalStore, ConversationRecord) {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "agentmage-conversation-archive-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&directory).expect("temporary directory");
        let mut store = OperationalStore::open(
            &directory.join("canonical.db"),
            &observation(),
            &mut TestKey([51; 32]),
        )
        .expect("encrypted store");
        let conversation = ConversationRecord {
            schema_version: CONTRACT_SCHEMA_VERSION,
            conversation_id: ConversationId::from_raw("conversation-archive-alpha"),
            title: "Private archive fixture".to_owned(),
            sensitivity: DataSensitivity::Operational,
            created_at_epoch_ms: CREATED,
            updated_at_epoch_ms: CREATED,
            local_date: "2027-01-15".to_owned(),
            local_timezone: "America/Chicago".to_owned(),
            workspace_id: WorkspaceId::from_raw("workspace-archive-alpha"),
            project_id: Some("project-archive-alpha".to_owned()),
            model_profile_id: ModelProfileId::from_raw("model-local-alpha"),
            status: ConversationStatus::Active,
            parent_conversation_id: None,
            branch_from_turn_id: None,
            current_turn_id: None,
            tags: vec!["archive".to_owned()],
            retention: ConversationRetention {
                kind: ConversationRetentionKind::Session,
                expires_at_epoch_ms: Some(CREATED + 86_400_000),
                policy_sha256: "a".repeat(64),
            },
            pinned: false,
            persistence_enabled: true,
        };
        store
            .create_conversation(&conversation)
            .expect("conversation creates");
        store
            .append_conversation_turn(&ConversationTurn {
                schema_version: CONTRACT_SCHEMA_VERSION,
                turn_id: ConversationTurnId::from_raw("turn-archive-alpha-1"),
                conversation_id: conversation.conversation_id.clone(),
                ordinal: 1,
                role: ConversationTurnRole::User,
                sensitivity: DataSensitivity::Operational,
                created_at_epoch_ms: CREATED + 1,
                local_date: "2027-01-15".to_owned(),
                text: Some(CANARY.to_owned()),
                text_sha256: sha256(CANARY.as_bytes()),
                attachments: Vec::new(),
                grant_ids: Vec::new(),
                receipt_ids: Vec::new(),
                checkpoint_id: None,
                citation_ids: vec!["citation-archive-alpha".to_owned()],
                source_sha256: vec!["b".repeat(64)],
            })
            .expect("turn appends");
        (directory, store, conversation)
    }

    fn request(conversation: &ConversationRecord) -> ConversationArchiveRequest {
        ConversationArchiveRequest {
            archive_id: "archive-alpha-0001".to_owned(),
            created_at_epoch_ms: CREATED + 10,
            expected_conversation_ids: vec![conversation.conversation_id.clone()],
            retention: ConversationArchiveRetention {
                delete_after_epoch_ms: CREATED + 20,
                user_hold: true,
                policy_sha256: "c".repeat(64),
            },
        }
    }

    #[test]
    fn encrypted_archive_inspection_and_fresh_restore_preserve_exact_inventory() {
        let (directory, store, conversation) = fixture();
        let archive = directory.join("private-archive.db");
        let restored = directory.join("restored-candidate.db");
        let manifest = store
            .create_conversation_archive(
                &archive,
                &observation(),
                &mut TestKey([52; 32]),
                &request(&conversation),
            )
            .expect("archive creates");
        assert_eq!(
            manifest.conversation_ids,
            vec![conversation.conversation_id.clone()]
        );
        assert_eq!(manifest.turn_count, 1);
        let bytes = fs::read(&archive).expect("archive bytes");
        assert!(!bytes.starts_with(b"SQLite format 3"));
        assert!(
            !bytes
                .windows(CANARY.len())
                .any(|part| part == CANARY.as_bytes())
        );

        let inspected = OperationalStore::inspect_conversation_archive(
            &archive,
            &observation(),
            &mut TestKey([52; 32]),
            &manifest,
        )
        .expect("archive inspects");
        assert!(inspected.verified);
        assert!(!inspected.restored);

        let receipt = OperationalStore::restore_conversation_archive_to_fresh_candidate(
            &archive,
            &observation(),
            &mut TestKey([52; 32]),
            &manifest,
            &restored,
            &observation(),
            &mut TestKey([53; 32]),
        )
        .expect("archive restores to fresh candidate");
        assert!(receipt.inventory_verified);
        assert!(!receipt.live_store_replaced);
        let candidate = OperationalStore::open(&restored, &observation(), &mut TestKey([53; 32]))
            .expect("candidate opens");
        assert_eq!(
            candidate
                .conversation_history(&conversation.conversation_id)
                .expect("history restored")
                .turns[0]
                .text
                .as_deref(),
            Some(CANARY)
        );
        drop(candidate);
        drop(store);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn stale_inventory_wrong_key_and_corrupt_ciphertext_fail_closed() {
        let (directory, store, conversation) = fixture();
        let stale_archive = directory.join("stale.db");
        let mut stale = request(&conversation);
        stale.expected_conversation_ids.clear();
        assert_eq!(
            store
                .create_conversation_archive(
                    &stale_archive,
                    &observation(),
                    &mut TestKey([54; 32]),
                    &stale,
                )
                .expect_err("unreviewed inventory must fail"),
            ConversationArchiveError::StaleReview
        );
        assert!(!stale_archive.exists());

        let archive = directory.join("private.db");
        let manifest = store
            .create_conversation_archive(
                &archive,
                &observation(),
                &mut TestKey([55; 32]),
                &request(&conversation),
            )
            .expect("archive creates");
        assert!(
            OperationalStore::inspect_conversation_archive(
                &archive,
                &observation(),
                &mut TestKey([99; 32]),
                &manifest,
            )
            .is_err()
        );

        let mut bytes = fs::read(&archive).expect("archive bytes");
        let midpoint = bytes.len() / 2;
        bytes[midpoint] ^= 0x5a;
        fs::write(&archive, bytes).expect("corrupt archive");
        assert_eq!(
            OperationalStore::inspect_conversation_archive(
                &archive,
                &observation(),
                &mut TestKey([55; 32]),
                &manifest,
            )
            .expect_err("corruption must fail"),
            ConversationArchiveError::IntegrityFailure
        );
        drop(store);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn retention_revision_and_bound_approval_control_archive_deletion() {
        let (directory, store, conversation) = fixture();
        let archive = directory.join("private.db");
        let manifest = store
            .create_conversation_archive(
                &archive,
                &observation(),
                &mut TestKey([56; 32]),
                &request(&conversation),
            )
            .expect("archive creates");
        let blocked = preview_conversation_archive_deletion(
            &archive,
            &observation(),
            &manifest,
            CREATED + 30,
        )
        .expect("blocked preview");
        assert!(blocked.retention_blocked);
        let blocked_approval = ConversationArchiveDeletionApproval {
            approval_id: ApprovalId::from_raw("approval-archive-blocked"),
            approved_preview_sha256: blocked.preview_sha256.clone(),
            decision_sha256: "d".repeat(64),
            approved_at_epoch_ms: CREATED + 29,
            user_confirmed: true,
        };
        assert_eq!(
            delete_conversation_archive(
                &archive,
                &observation(),
                &manifest,
                &blocked,
                &blocked_approval,
                CREATED + 30,
            )
            .expect_err("hold blocks deletion"),
            ConversationArchiveError::RetentionBlocked
        );

        let retention_preview = preview_conversation_archive_retention(
            &manifest,
            ConversationArchiveRetention {
                delete_after_epoch_ms: CREATED + 20,
                user_hold: false,
                policy_sha256: "e".repeat(64),
            },
        )
        .expect("retention previews");
        let revised = apply_conversation_archive_retention(&manifest, &retention_preview)
            .expect("retention applies");
        assert_eq!(revised.revision, 2);
        assert_eq!(
            apply_conversation_archive_retention(&revised, &retention_preview)
                .expect_err("retention preview cannot replay"),
            ConversationArchiveError::StaleReview
        );

        let preview =
            preview_conversation_archive_deletion(&archive, &observation(), &revised, CREATED + 30)
                .expect("deletion previews");
        assert!(!preview.retention_blocked);
        let approval = ConversationArchiveDeletionApproval {
            approval_id: ApprovalId::from_raw("approval-archive-delete"),
            approved_preview_sha256: preview.preview_sha256.clone(),
            decision_sha256: "f".repeat(64),
            approved_at_epoch_ms: CREATED + 29,
            user_confirmed: true,
        };
        let receipt = delete_conversation_archive(
            &archive,
            &observation(),
            &revised,
            &preview,
            &approval,
            CREATED + 30,
        )
        .expect("approved deletion commits");
        assert!(receipt.deleted);
        assert!(!receipt.physical_overwrite_claim);
        assert!(!archive.exists());
        drop(store);
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn archive_creation_rejects_nonlocal_storage_without_side_effect() {
        let (directory, store, conversation) = fixture();
        let destination = directory.join("remote.db");
        let remote = StrictLocalStorageObservation {
            filesystem: StorageFilesystemClass::Remote,
            ..observation()
        };
        assert_eq!(
            store
                .create_conversation_archive(
                    &destination,
                    &remote,
                    &mut TestKey([57; 32]),
                    &request(&conversation),
                )
                .expect_err("remote storage must fail"),
            ConversationArchiveError::StorageRejected
        );
        assert!(!destination.exists());
        drop(store);
        fs::remove_dir_all(directory).expect("cleanup");
    }
}
