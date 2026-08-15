//! Content-free write privacy, checkpoint, recovery, and audit coordination.

use std::collections::BTreeSet;
use std::fmt::Write;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::persistence::{SecretFindingClass, detect_secret_classes};

const SCHEMA_VERSION: u16 = 1;
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const REDACTED_VALUE: &str = "[REDACTED]";
const MAX_ITEMS: usize = 256;
const MAX_TEXT_BYTES: usize = 4_096;

/// Stable failure from write recovery coordination.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteRecoveryError {
    /// An identifier, digest, count, timestamp, or state combination was malformed.
    InvalidInput,
    /// A checkpoint transition or hash chain was inconsistent.
    CheckpointIntegrity,
    /// A terminal state attempted to claim completion without required proof.
    AmbiguousCompletion,
    /// A cleanup attempt repeated an already terminal staging decision.
    CleanupReplay,
    /// Canonical serialization failed.
    SerializationFailed,
}

impl WriteRecoveryError {
    /// Returns one stable, content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "write_recovery.input.invalid",
            Self::CheckpointIntegrity => "write_recovery.checkpoint.integrity",
            Self::AmbiguousCompletion => "write_recovery.completion.ambiguous",
            Self::CleanupReplay => "write_recovery.cleanup.replay",
            Self::SerializationFailed => "write_recovery.serialization.failed",
        }
    }
}

/// Write-adjacent trust boundary that must scan and classify content first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WritePrivacyBoundary {
    /// User-visible proposed change.
    Preview,
    /// Temporary application material.
    Staging,
    /// Durable operation evidence.
    Receipt,
    /// Content selected for model inference.
    ModelContext,
    /// Durable application state.
    Persistence,
    /// Recovery preimage or backup material.
    Backup,
    /// Local diagnostic output.
    Diagnostic,
    /// User-directed portable output.
    Export,
    /// Operational event output.
    Log,
    /// Recovery checkpoint metadata.
    Checkpoint,
    /// Error presentation or propagation.
    Error,
}

impl WritePrivacyBoundary {
    /// Every closed write-adjacent boundary.
    pub const ALL: [Self; 11] = [
        Self::Preview,
        Self::Staging,
        Self::Receipt,
        Self::ModelContext,
        Self::Persistence,
        Self::Backup,
        Self::Diagnostic,
        Self::Export,
        Self::Log,
        Self::Checkpoint,
        Self::Error,
    ];
}

/// Trusted sensitivity assigned before a field crosses a write boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteFieldSensitivity {
    /// Non-secret metadata may remain visible after deterministic scanning.
    PublicMetadata,
    /// Private user content must be replaced with a fixed marker.
    PrivateExcerpt,
    /// Credential material must be replaced with a fixed marker.
    Credential,
}

/// Borrowed field submitted to the write privacy gate.
pub struct WriteBoundaryField<'a> {
    /// Stable field label, which must itself contain no secret material.
    pub name: &'a str,
    /// Candidate bytes held only for this classification call.
    pub value: &'a [u8],
    /// Trusted sensitivity classification.
    pub sensitivity: WriteFieldSensitivity,
}

/// Stable reason a field was replaced without retaining removed content.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteRedactionReason {
    /// Caller declared private user content.
    DeclaredPrivate,
    /// Caller declared credential material.
    DeclaredCredential,
    /// The candidate was not valid UTF-8 metadata.
    NonUtf8,
    /// The deterministic secret detector found a known class.
    SecretClass(String),
}

/// Sanitized field that contains either admitted metadata or a fixed marker.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SanitizedWriteField {
    /// Stable field name.
    pub name: String,
    /// Admitted UTF-8 metadata or the fixed redaction marker.
    pub value: String,
    /// Ordered reasons; empty only when the value was admitted.
    pub redaction_reasons: Vec<WriteRedactionReason>,
}

/// Content-free proof that every supplied field was classified.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WriteBoundaryScanReceipt {
    /// Contract schema version.
    pub schema_version: u16,
    /// Boundary receiving the sanitized result.
    pub boundary: WritePrivacyBoundary,
    /// Number of fields inspected.
    pub inspected_fields: u16,
    /// Number of fields replaced.
    pub redacted_fields: u16,
    /// Ordered unique secret classes, without values or value digests.
    pub secret_classes: Vec<String>,
    /// Digest of the sanitized output only.
    pub sanitized_output_sha256: String,
    /// Digest of this receipt with this field zeroed.
    pub receipt_sha256: String,
}

/// Sanitized output and its content-free classification receipt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriteBoundaryScanResult {
    /// Sanitized fields safe for the requested boundary.
    pub fields: Vec<SanitizedWriteField>,
    /// Content-free classification receipt.
    pub receipt: WriteBoundaryScanReceipt,
}

/// Scans and sanitizes every field before one write-adjacent boundary.
pub fn sanitize_write_boundary(
    boundary: WritePrivacyBoundary,
    fields: &[WriteBoundaryField<'_>],
) -> Result<WriteBoundaryScanResult, WriteRecoveryError> {
    if fields.len() > MAX_ITEMS {
        return Err(WriteRecoveryError::InvalidInput);
    }
    let mut names = BTreeSet::new();
    let mut secret_classes = BTreeSet::new();
    let mut sanitized = Vec::with_capacity(fields.len());
    for field in fields {
        if !valid_label(field.name) || !names.insert(field.name) {
            return Err(WriteRecoveryError::InvalidInput);
        }
        let findings = detect_secret_classes(field.name, field.value);
        let mut reasons = Vec::new();
        match field.sensitivity {
            WriteFieldSensitivity::PublicMetadata => {}
            WriteFieldSensitivity::PrivateExcerpt => {
                reasons.push(WriteRedactionReason::DeclaredPrivate);
            }
            WriteFieldSensitivity::Credential => {
                reasons.push(WriteRedactionReason::DeclaredCredential);
            }
        }
        for finding in findings {
            let code = secret_class_code(finding).to_owned();
            secret_classes.insert(code.clone());
            reasons.push(WriteRedactionReason::SecretClass(code));
        }
        let value = if reasons.is_empty() {
            match std::str::from_utf8(field.value) {
                Ok(value) if value.len() <= MAX_TEXT_BYTES => value.to_owned(),
                _ => {
                    reasons.push(WriteRedactionReason::NonUtf8);
                    REDACTED_VALUE.to_owned()
                }
            }
        } else {
            REDACTED_VALUE.to_owned()
        };
        reasons.sort();
        reasons.dedup();
        sanitized.push(SanitizedWriteField {
            name: field.name.to_owned(),
            value,
            redaction_reasons: reasons,
        });
    }
    let redacted_fields = sanitized
        .iter()
        .filter(|field| !field.redaction_reasons.is_empty())
        .count();
    let sanitized_output_sha256 = canonical_sha256(&sanitized)?;
    let mut receipt = WriteBoundaryScanReceipt {
        schema_version: SCHEMA_VERSION,
        boundary,
        inspected_fields: u16::try_from(sanitized.len())
            .map_err(|_| WriteRecoveryError::InvalidInput)?,
        redacted_fields: u16::try_from(redacted_fields)
            .map_err(|_| WriteRecoveryError::InvalidInput)?,
        secret_classes: secret_classes.into_iter().collect(),
        sanitized_output_sha256,
        receipt_sha256: ZERO_SHA256.to_owned(),
    };
    receipt.receipt_sha256 = canonical_sha256(&receipt)?;
    Ok(WriteBoundaryScanResult {
        fields: sanitized,
        receipt,
    })
}

/// Closed write transaction phase retained across restart.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteCheckpointPhase {
    /// No grant has been consumed and no effect may have started.
    BeforeTransaction,
    /// Exact single-use authority was consumed.
    GrantConsumed,
    /// Approved bytes are being staged.
    Staging,
    /// Canonical application may be in progress.
    Applying,
    /// Canonical postimages were freshly verified.
    CanonicalVerified,
    /// Derived-index publication may be in progress.
    IndexUpdating,
    /// Every declared derived-index update was verified.
    IndexVerified,
    /// Terminal receipt persistence may be in progress.
    ReceiptPersisting,
    /// Terminal receipt chain was verified.
    ReceiptPersisted,
    /// Exact approved restoration may be in progress.
    RollbackPending,
    /// No canonical state changed.
    FailedNoChange,
    /// Exact approved preimages were restored.
    Restored,
    /// State cannot be reconciled safely without review.
    Uncertain,
    /// Staging cleanup remains separately required.
    CleanupPending,
    /// Postimages, receipts, indexes, and cleanup were verified.
    Complete,
}

impl WriteCheckpointPhase {
    /// Every closed intermediate and terminal phase.
    pub const ALL: [Self; 15] = [
        Self::BeforeTransaction,
        Self::GrantConsumed,
        Self::Staging,
        Self::Applying,
        Self::CanonicalVerified,
        Self::IndexUpdating,
        Self::IndexVerified,
        Self::ReceiptPersisting,
        Self::ReceiptPersisted,
        Self::RollbackPending,
        Self::FailedNoChange,
        Self::Restored,
        Self::Uncertain,
        Self::CleanupPending,
        Self::Complete,
    ];

    /// Reports whether no future write replay is allowed from this checkpoint.
    #[must_use]
    pub const fn prevents_replay(self) -> bool {
        !matches!(self, Self::BeforeTransaction)
    }

    const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::FailedNoChange | Self::Restored | Self::Uncertain | Self::Complete
        )
    }
}

/// Cleanup state bound into a write checkpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteCleanupState {
    /// No staging item was created.
    NotRequired,
    /// Attributable staging items still require a separate cleanup decision.
    Pending,
    /// Every attributable staging item was verified absent.
    Clean,
    /// A conflict or unavailable root prevents safe cleanup.
    Blocked,
}

/// Input used to build one hash-chained write-aware checkpoint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriteAwareCheckpointInput {
    /// Stable checkpoint identity.
    pub checkpoint_id: String,
    /// Stable transaction correlation identity.
    pub transaction_id: String,
    /// Exact canonical action identity.
    pub action_id: String,
    /// Current write phase.
    pub phase: WriteCheckpointPhase,
    /// Consumed single-use grant, when consumption occurred.
    pub consumed_grant_id: Option<String>,
    /// Head of the file-operation receipt chain, when receipts exist.
    pub file_receipt_head_sha256: Option<String>,
    /// Number of file-operation receipts.
    pub file_receipt_count: u32,
    /// Digest of exact evidence identities bound to the action.
    pub evidence_set_sha256: String,
    /// Digest of declared index updates, when applicable.
    pub index_update_sha256: Option<String>,
    /// Digest of the next canonical session checkpoint.
    pub next_session_checkpoint_sha256: String,
    /// Privacy-gate receipt covering checkpoint metadata.
    pub secret_scan_receipt_sha256: String,
    /// Digest of the complete staging inventory.
    pub staging_inventory_sha256: String,
    /// Number of attributable staging items.
    pub staging_item_count: u32,
    /// Exclusive retention boundary for staging material, or zero when absent.
    pub retention_expires_at_epoch_ms: u64,
    /// Whether canonical postimages were freshly verified.
    pub canonical_postimages_verified: bool,
    /// Whether the complete file receipt chain was verified.
    pub receipt_chain_verified: bool,
    /// Whether every declared index update was freshly verified.
    pub index_verified: bool,
    /// Whether exact approved preimages were restored.
    pub rollback_verified: bool,
    /// Current staging cleanup state.
    pub cleanup_state: WriteCleanupState,
    /// Stable content-free failure code.
    pub failure_code: Option<String>,
    /// Kernel-clock checkpoint time.
    pub occurred_at_epoch_ms: u64,
}

/// Metadata-only state-change checkpoint carrying no file content or authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WriteAwareCheckpoint {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable checkpoint identity.
    pub checkpoint_id: String,
    /// Stable transaction correlation identity.
    pub transaction_id: String,
    /// Exact canonical action identity.
    pub action_id: String,
    /// Monotonic sequence within the transaction.
    pub sequence: u32,
    /// Current write phase.
    pub phase: WriteCheckpointPhase,
    /// Consumed single-use grant, when consumption occurred.
    pub consumed_grant_id: Option<String>,
    /// Head of the file-operation receipt chain, when receipts exist.
    pub file_receipt_head_sha256: Option<String>,
    /// Number of file-operation receipts.
    pub file_receipt_count: u32,
    /// Digest of exact evidence identities bound to the action.
    pub evidence_set_sha256: String,
    /// Digest of declared index updates, when applicable.
    pub index_update_sha256: Option<String>,
    /// Digest of the next canonical session checkpoint.
    pub next_session_checkpoint_sha256: String,
    /// Privacy-gate receipt covering checkpoint metadata.
    pub secret_scan_receipt_sha256: String,
    /// Digest of the complete staging inventory.
    pub staging_inventory_sha256: String,
    /// Number of attributable staging items.
    pub staging_item_count: u32,
    /// Exclusive retention boundary for staging material, or zero when absent.
    pub retention_expires_at_epoch_ms: u64,
    /// Whether canonical postimages were freshly verified.
    pub canonical_postimages_verified: bool,
    /// Whether the complete file receipt chain was verified.
    pub receipt_chain_verified: bool,
    /// Whether every declared index update was freshly verified.
    pub index_verified: bool,
    /// Whether exact approved preimages were restored.
    pub rollback_verified: bool,
    /// Current staging cleanup state.
    pub cleanup_state: WriteCleanupState,
    /// Stable content-free failure code.
    pub failure_code: Option<String>,
    /// Kernel-clock checkpoint time.
    pub occurred_at_epoch_ms: u64,
    /// Previous checkpoint digest or zeroes for the first checkpoint.
    pub previous_checkpoint_sha256: String,
    /// Digest of this checkpoint with this field zeroed.
    pub checkpoint_sha256: String,
}

/// Builds one validated checkpoint linked to the prior transaction checkpoint.
pub fn build_write_checkpoint(
    input: WriteAwareCheckpointInput,
    previous: Option<&WriteAwareCheckpoint>,
) -> Result<WriteAwareCheckpoint, WriteRecoveryError> {
    validate_checkpoint_input(&input)?;
    let (sequence, previous_checkpoint_sha256) = match previous {
        Some(previous) => {
            verify_write_checkpoint(previous)?;
            if previous.transaction_id != input.transaction_id
                || previous.action_id != input.action_id
                || previous.phase.is_terminal()
                || !allowed_transition(previous.phase, input.phase)
                || input.occurred_at_epoch_ms < previous.occurred_at_epoch_ms
            {
                return Err(WriteRecoveryError::CheckpointIntegrity);
            }
            (
                previous
                    .sequence
                    .checked_add(1)
                    .ok_or(WriteRecoveryError::CheckpointIntegrity)?,
                previous.checkpoint_sha256.clone(),
            )
        }
        None => {
            if input.phase != WriteCheckpointPhase::BeforeTransaction {
                return Err(WriteRecoveryError::CheckpointIntegrity);
            }
            (0, ZERO_SHA256.to_owned())
        }
    };
    let mut checkpoint = WriteAwareCheckpoint {
        schema_version: SCHEMA_VERSION,
        checkpoint_id: input.checkpoint_id,
        transaction_id: input.transaction_id,
        action_id: input.action_id,
        sequence,
        phase: input.phase,
        consumed_grant_id: input.consumed_grant_id,
        file_receipt_head_sha256: input.file_receipt_head_sha256,
        file_receipt_count: input.file_receipt_count,
        evidence_set_sha256: input.evidence_set_sha256,
        index_update_sha256: input.index_update_sha256,
        next_session_checkpoint_sha256: input.next_session_checkpoint_sha256,
        secret_scan_receipt_sha256: input.secret_scan_receipt_sha256,
        staging_inventory_sha256: input.staging_inventory_sha256,
        staging_item_count: input.staging_item_count,
        retention_expires_at_epoch_ms: input.retention_expires_at_epoch_ms,
        canonical_postimages_verified: input.canonical_postimages_verified,
        receipt_chain_verified: input.receipt_chain_verified,
        index_verified: input.index_verified,
        rollback_verified: input.rollback_verified,
        cleanup_state: input.cleanup_state,
        failure_code: input.failure_code,
        occurred_at_epoch_ms: input.occurred_at_epoch_ms,
        previous_checkpoint_sha256,
        checkpoint_sha256: ZERO_SHA256.to_owned(),
    };
    checkpoint.checkpoint_sha256 = canonical_sha256(&checkpoint)?;
    Ok(checkpoint)
}

/// Verifies one checkpoint's closed invariants and self-digest.
pub fn verify_write_checkpoint(
    checkpoint: &WriteAwareCheckpoint,
) -> Result<(), WriteRecoveryError> {
    let input = WriteAwareCheckpointInput {
        checkpoint_id: checkpoint.checkpoint_id.clone(),
        transaction_id: checkpoint.transaction_id.clone(),
        action_id: checkpoint.action_id.clone(),
        phase: checkpoint.phase,
        consumed_grant_id: checkpoint.consumed_grant_id.clone(),
        file_receipt_head_sha256: checkpoint.file_receipt_head_sha256.clone(),
        file_receipt_count: checkpoint.file_receipt_count,
        evidence_set_sha256: checkpoint.evidence_set_sha256.clone(),
        index_update_sha256: checkpoint.index_update_sha256.clone(),
        next_session_checkpoint_sha256: checkpoint.next_session_checkpoint_sha256.clone(),
        secret_scan_receipt_sha256: checkpoint.secret_scan_receipt_sha256.clone(),
        staging_inventory_sha256: checkpoint.staging_inventory_sha256.clone(),
        staging_item_count: checkpoint.staging_item_count,
        retention_expires_at_epoch_ms: checkpoint.retention_expires_at_epoch_ms,
        canonical_postimages_verified: checkpoint.canonical_postimages_verified,
        receipt_chain_verified: checkpoint.receipt_chain_verified,
        index_verified: checkpoint.index_verified,
        rollback_verified: checkpoint.rollback_verified,
        cleanup_state: checkpoint.cleanup_state,
        failure_code: checkpoint.failure_code.clone(),
        occurred_at_epoch_ms: checkpoint.occurred_at_epoch_ms,
    };
    validate_checkpoint_input(&input)?;
    if checkpoint.schema_version != SCHEMA_VERSION
        || (checkpoint.sequence == 0) != (checkpoint.previous_checkpoint_sha256 == ZERO_SHA256)
        || (checkpoint.sequence > 0 && !valid_sha256(&checkpoint.previous_checkpoint_sha256))
        || !valid_sha256(&checkpoint.checkpoint_sha256)
    {
        return Err(WriteRecoveryError::CheckpointIntegrity);
    }
    let mut candidate = checkpoint.clone();
    candidate.checkpoint_sha256 = ZERO_SHA256.to_owned();
    if canonical_sha256(&candidate)? != checkpoint.checkpoint_sha256 {
        return Err(WriteRecoveryError::CheckpointIntegrity);
    }
    Ok(())
}

/// Verifies a complete ordered checkpoint chain with no duplicate identities.
pub fn verify_write_checkpoint_chain(
    checkpoints: &[WriteAwareCheckpoint],
) -> Result<(), WriteRecoveryError> {
    if checkpoints.is_empty() || checkpoints.len() > MAX_ITEMS {
        return Err(WriteRecoveryError::CheckpointIntegrity);
    }
    let mut identities = BTreeSet::new();
    for (index, checkpoint) in checkpoints.iter().enumerate() {
        verify_write_checkpoint(checkpoint)?;
        if checkpoint.sequence != index as u32
            || !identities.insert(checkpoint.checkpoint_id.as_str())
            || (index == 0 && checkpoint.phase != WriteCheckpointPhase::BeforeTransaction)
            || (index > 0
                && (checkpoint.transaction_id != checkpoints[0].transaction_id
                    || checkpoint.action_id != checkpoints[0].action_id
                    || checkpoint.previous_checkpoint_sha256
                        != checkpoints[index - 1].checkpoint_sha256
                    || !allowed_transition(checkpoints[index - 1].phase, checkpoint.phase)))
        {
            return Err(WriteRecoveryError::CheckpointIntegrity);
        }
    }
    Ok(())
}

/// Fresh, content-free observations used to choose a restart instruction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriteRecoveryObservation {
    /// Whether the approved workspace root identity still matches.
    pub root_identity_matches: bool,
    /// Whether an external edit or another session changed a target.
    pub concurrent_change_detected: bool,
    /// Whether the encrypted secret store is currently available.
    pub secret_store_available: bool,
    /// Whether the canonical postimages now match the approved transaction.
    pub canonical_postimages_match: bool,
    /// Whether the terminal receipt chain is present and valid.
    pub receipt_chain_verified: bool,
    /// Whether every declared derived index matches canonical bytes.
    pub index_verified: bool,
    /// Number of attributable staging objects still present.
    pub staging_items_present: u32,
    /// Whether the staging retention deadline has elapsed.
    pub staging_retention_expired: bool,
}

/// Closed, non-authoritative recovery instruction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteRecoveryInstruction {
    /// Build a fresh proposal because no grant was ever consumed.
    BuildFreshProposal,
    /// Restore the approved workspace or stop using this transaction.
    RestoreWorkspaceIdentity,
    /// Preserve both states and request human conflict review.
    PreserveConflictForReview,
    /// Restore access to the encrypted secret store before continuing.
    RestoreSecretStore,
    /// Freshly observe canonical postimages without applying again.
    VerifyCanonicalState,
    /// Publish only the declared derived index through separate authority.
    PublishDerivedIndex,
    /// Persist and verify only the content-free terminal receipt.
    PersistTerminalReceipt,
    /// Propose separately receipted removal of attributable staging.
    ProposeStagingCleanup,
    /// Stop because the consumed authority or resulting state is uncertain.
    BlockUncertainState,
    /// No further recovery action is required.
    NoActionComplete,
}

/// Deterministic recovery decision that never carries effect authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WriteRecoveryDecision {
    /// Selected closed instruction.
    pub instruction: WriteRecoveryInstruction,
    /// Stable explanation code.
    pub reason_code: String,
    /// A completed or possibly completed write must never be replayed.
    pub repeat_completed_write: bool,
    /// Any effect requires a separate fresh grant.
    pub requires_separate_grant: bool,
}

/// Selects one deterministic restart instruction without performing an effect.
pub fn plan_write_recovery(
    checkpoint: &WriteAwareCheckpoint,
    observation: &WriteRecoveryObservation,
) -> Result<WriteRecoveryDecision, WriteRecoveryError> {
    verify_write_checkpoint(checkpoint)?;
    let (instruction, reason_code) = if !observation.secret_store_available {
        (
            WriteRecoveryInstruction::RestoreSecretStore,
            "write_recovery.secret_store.unavailable",
        )
    } else if !observation.root_identity_matches {
        (
            WriteRecoveryInstruction::RestoreWorkspaceIdentity,
            "write_recovery.workspace.moved",
        )
    } else if observation.concurrent_change_detected {
        (
            WriteRecoveryInstruction::PreserveConflictForReview,
            "write_recovery.concurrent_change.preserved",
        )
    } else if checkpoint.phase == WriteCheckpointPhase::BeforeTransaction {
        (
            WriteRecoveryInstruction::BuildFreshProposal,
            "write_recovery.before_grant.fresh_proposal",
        )
    } else if checkpoint.phase == WriteCheckpointPhase::Uncertain {
        (
            WriteRecoveryInstruction::BlockUncertainState,
            "write_recovery.state.uncertain",
        )
    } else if !observation.canonical_postimages_match
        && !matches!(
            checkpoint.phase,
            WriteCheckpointPhase::FailedNoChange | WriteCheckpointPhase::Restored
        )
    {
        (
            WriteRecoveryInstruction::VerifyCanonicalState,
            "write_recovery.canonical.verify_only",
        )
    } else if !observation.index_verified && checkpoint.index_update_sha256.is_some() {
        (
            WriteRecoveryInstruction::PublishDerivedIndex,
            "write_recovery.index.publish_separately",
        )
    } else if !observation.receipt_chain_verified {
        (
            WriteRecoveryInstruction::PersistTerminalReceipt,
            "write_recovery.receipt.persist_separately",
        )
    } else if observation.staging_items_present > 0 {
        (
            WriteRecoveryInstruction::ProposeStagingCleanup,
            if observation.staging_retention_expired {
                "write_recovery.staging.expired"
            } else {
                "write_recovery.staging.present"
            },
        )
    } else {
        (
            WriteRecoveryInstruction::NoActionComplete,
            "write_recovery.verified.complete",
        )
    };
    Ok(WriteRecoveryDecision {
        instruction,
        reason_code: reason_code.to_owned(),
        repeat_completed_write: false,
        requires_separate_grant: !matches!(
            instruction,
            WriteRecoveryInstruction::NoActionComplete
                | WriteRecoveryInstruction::BlockUncertainState
                | WriteRecoveryInstruction::PreserveConflictForReview
        ),
    })
}

/// Staging-object lifecycle visible to content-free recovery diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StagingObjectState {
    /// The owning transaction may still need the object.
    Active,
    /// No nonterminal owning transaction exists.
    Orphaned,
    /// A separate cleanup approval exists.
    CleanupApproved,
    /// Absence was verified after cleanup.
    Cleaned,
    /// A conflict or integrity failure requires review.
    Quarantined,
}

/// Content-free inventory identity for one temporary write object.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StagingInventoryItem {
    /// Stable staging identity, never a raw path.
    pub staging_id: String,
    /// Owning transaction identity.
    pub transaction_id: String,
    /// Owning action identity.
    pub action_id: String,
    /// Digest of the staged bytes.
    pub content_sha256: String,
    /// Creation time.
    pub created_at_epoch_ms: u64,
    /// Exclusive retention boundary.
    pub expires_at_epoch_ms: u64,
    /// Recorded lifecycle state.
    pub state: StagingObjectState,
}

/// Content-free orphan and cleanup diagnostic.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StagingDiagnostic {
    /// Stable staging identity.
    pub staging_id: String,
    /// Owning transaction identity.
    pub transaction_id: String,
    /// Whether the object is attributable to a known checkpoint chain.
    pub attributable: bool,
    /// Whether no live transaction owns the object.
    pub orphaned: bool,
    /// Whether its retention deadline elapsed.
    pub expired: bool,
    /// Whether a separate cleanup proposal may be built.
    pub cleanup_eligible: bool,
    /// Stable diagnostic code.
    pub reason_code: String,
}

/// Diagnoses staging inventory without opening, changing, or deleting any object.
pub fn diagnose_staging_inventory(
    now_epoch_ms: u64,
    checkpoints: &[WriteAwareCheckpoint],
    inventory: &[StagingInventoryItem],
) -> Result<Vec<StagingDiagnostic>, WriteRecoveryError> {
    if inventory.len() > MAX_ITEMS || checkpoints.len() > MAX_ITEMS {
        return Err(WriteRecoveryError::InvalidInput);
    }
    let mut staging_ids = BTreeSet::new();
    let mut known_transactions = BTreeSet::new();
    let mut live_transactions = BTreeSet::new();
    for checkpoint in checkpoints {
        verify_write_checkpoint(checkpoint)?;
        known_transactions.insert(checkpoint.transaction_id.as_str());
        if !checkpoint.phase.is_terminal() {
            live_transactions.insert(checkpoint.transaction_id.as_str());
        }
    }
    let mut diagnostics = Vec::with_capacity(inventory.len());
    for item in inventory {
        if !valid_identifier(&item.staging_id)
            || !valid_identifier(&item.transaction_id)
            || !valid_identifier(&item.action_id)
            || !valid_sha256(&item.content_sha256)
            || item.created_at_epoch_ms >= item.expires_at_epoch_ms
            || !staging_ids.insert(item.staging_id.as_str())
        {
            return Err(WriteRecoveryError::InvalidInput);
        }
        let attributable = known_transactions.contains(item.transaction_id.as_str());
        let orphaned = !live_transactions.contains(item.transaction_id.as_str());
        let expired = now_epoch_ms >= item.expires_at_epoch_ms;
        let cleanup_eligible = attributable
            && orphaned
            && !matches!(
                item.state,
                StagingObjectState::Cleaned | StagingObjectState::Quarantined
            );
        let reason_code = if !attributable {
            "write_recovery.staging.owner_unknown"
        } else if item.state == StagingObjectState::Quarantined {
            "write_recovery.staging.quarantined"
        } else if !orphaned {
            "write_recovery.staging.active"
        } else if expired {
            "write_recovery.staging.orphan_expired"
        } else {
            "write_recovery.staging.orphan_retained"
        };
        diagnostics.push(StagingDiagnostic {
            staging_id: item.staging_id.clone(),
            transaction_id: item.transaction_id.clone(),
            attributable,
            orphaned,
            expired,
            cleanup_eligible,
            reason_code: reason_code.to_owned(),
        });
    }
    Ok(diagnostics)
}

/// Terminal separately receipted cleanup disposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StagingCleanupDisposition {
    /// Object absence was freshly verified.
    Removed,
    /// A conflict was preserved without removal.
    PreservedConflict,
    /// Cleanup failed and the object remains attributable.
    Failed,
}

/// Content-free receipt for one staging cleanup decision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StagingCleanupReceipt {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable cleanup action identity.
    pub cleanup_id: String,
    /// Stable staging identity.
    pub staging_id: String,
    /// Owning transaction identity.
    pub transaction_id: String,
    /// Terminal cleanup disposition.
    pub disposition: StagingCleanupDisposition,
    /// Digest of the complete inventory before the separately authorized attempt.
    pub inventory_before_sha256: String,
    /// Digest of the complete inventory after the attempt.
    pub inventory_after_sha256: String,
    /// Kernel-clock event time.
    pub occurred_at_epoch_ms: u64,
    /// Previous cleanup receipt digest or zeroes.
    pub previous_receipt_sha256: String,
    /// Digest of this receipt with this field zeroed.
    pub receipt_sha256: String,
}

/// Complete content-free input for one terminal staging cleanup result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StagingCleanupRecordInput {
    /// Stable cleanup action identity.
    pub cleanup_id: String,
    /// Stable staging identity.
    pub staging_id: String,
    /// Owning transaction identity.
    pub transaction_id: String,
    /// Terminal cleanup disposition.
    pub disposition: StagingCleanupDisposition,
    /// Digest of the complete inventory before the separately authorized attempt.
    pub inventory_before_sha256: String,
    /// Digest of the complete inventory after the attempt.
    pub inventory_after_sha256: String,
    /// Kernel-clock event time.
    pub occurred_at_epoch_ms: u64,
}

/// Memory-only ledger preventing repeated cleanup receipt publication.
#[derive(Default)]
pub struct StagingCleanupLedger {
    receipts: Vec<StagingCleanupReceipt>,
    terminal_staging_ids: BTreeSet<String>,
}

impl StagingCleanupLedger {
    /// Creates an empty cleanup ledger.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            receipts: Vec::new(),
            terminal_staging_ids: BTreeSet::new(),
        }
    }

    /// Returns the complete ordered content-free receipt chain.
    #[must_use]
    pub fn receipts(&self) -> &[StagingCleanupReceipt] {
        &self.receipts
    }

    /// Records one terminal result; this method performs no cleanup effect.
    pub fn record(
        &mut self,
        input: StagingCleanupRecordInput,
    ) -> Result<StagingCleanupReceipt, WriteRecoveryError> {
        if !valid_identifier(&input.cleanup_id)
            || !valid_identifier(&input.staging_id)
            || !valid_identifier(&input.transaction_id)
            || !valid_sha256(&input.inventory_before_sha256)
            || !valid_sha256(&input.inventory_after_sha256)
            || self.terminal_staging_ids.contains(&input.staging_id)
        {
            return Err(if self.terminal_staging_ids.contains(&input.staging_id) {
                WriteRecoveryError::CleanupReplay
            } else {
                WriteRecoveryError::InvalidInput
            });
        }
        let previous_receipt_sha256 = self.receipts.last().map_or_else(
            || ZERO_SHA256.to_owned(),
            |receipt| receipt.receipt_sha256.clone(),
        );
        let mut receipt = StagingCleanupReceipt {
            schema_version: SCHEMA_VERSION,
            cleanup_id: input.cleanup_id,
            staging_id: input.staging_id.clone(),
            transaction_id: input.transaction_id,
            disposition: input.disposition,
            inventory_before_sha256: input.inventory_before_sha256,
            inventory_after_sha256: input.inventory_after_sha256,
            occurred_at_epoch_ms: input.occurred_at_epoch_ms,
            previous_receipt_sha256,
            receipt_sha256: ZERO_SHA256.to_owned(),
        };
        receipt.receipt_sha256 = canonical_sha256(&receipt)?;
        self.terminal_staging_ids.insert(input.staging_id);
        self.receipts.push(receipt.clone());
        Ok(receipt)
    }
}

/// Closed validation state for one human-readable audit operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteAuditValidation {
    /// Postimage was freshly verified.
    Verified,
    /// No canonical change occurred.
    NoChange,
    /// Exact approved preimage was restored.
    Restored,
    /// State remains uncertain and must not be called complete.
    Uncertain,
}

/// Borrowed operation used to build a redacted audit summary.
pub struct WriteAuditOperationInput<'a> {
    /// Canonical workspace-relative file path.
    pub file: &'a str,
    /// Stable operation kind.
    pub operation: &'a str,
    /// Exact approved preimage digest.
    pub preimage_sha256: &'a str,
    /// Exact expected postimage digest.
    pub postimage_sha256: &'a str,
    /// Terminal validation result.
    pub validation: WriteAuditValidation,
    /// Stable failure code, when present.
    pub failure_code: Option<&'a str>,
    /// Stable rollback status code.
    pub rollback_status: &'a str,
}

/// One sanitized operation in a state-change audit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WriteAuditOperation {
    /// Exact path or fixed redaction marker.
    pub file: String,
    /// Stable operation kind.
    pub operation: String,
    /// Exact approved preimage digest.
    pub preimage_sha256: String,
    /// Exact expected postimage digest.
    pub postimage_sha256: String,
    /// Terminal validation result.
    pub validation: WriteAuditValidation,
    /// Stable failure code, when present.
    pub failure_code: Option<String>,
    /// Stable rollback status code.
    pub rollback_status: String,
}

/// Redacted, human-readable state-change audit summary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateChangeAuditSummary {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable transaction identity.
    pub transaction_id: String,
    /// Exact action identity.
    pub action_id: String,
    /// Head of the verified write checkpoint chain.
    pub checkpoint_sha256: String,
    /// Ordered sanitized operation summaries.
    pub operations: Vec<WriteAuditOperation>,
    /// Number of path or failure fields replaced by the privacy gate.
    pub redacted_fields: u16,
    /// Stable overall validation code.
    pub overall_status: String,
    /// Human-readable report generated only from sanitized fields.
    pub report: String,
    /// Digest of this summary with this field zeroed.
    pub audit_sha256: String,
}

/// Builds a deterministic audit report without retaining file content or removed values.
pub fn build_state_change_audit(
    checkpoint: &WriteAwareCheckpoint,
    operations: &[WriteAuditOperationInput<'_>],
) -> Result<StateChangeAuditSummary, WriteRecoveryError> {
    verify_write_checkpoint(checkpoint)?;
    if operations.is_empty() || operations.len() > MAX_ITEMS {
        return Err(WriteRecoveryError::InvalidInput);
    }
    let mut sanitized_operations = Vec::with_capacity(operations.len());
    let mut redacted_fields = 0_u16;
    for operation in operations {
        if !valid_label(operation.operation)
            || !valid_sha256(operation.preimage_sha256)
            || !valid_sha256(operation.postimage_sha256)
            || !valid_code(operation.rollback_status)
            || operation
                .failure_code
                .is_some_and(|value| !valid_code(value))
        {
            return Err(WriteRecoveryError::InvalidInput);
        }
        let fields = [
            WriteBoundaryField {
                name: "file",
                value: operation.file.as_bytes(),
                sensitivity: WriteFieldSensitivity::PublicMetadata,
            },
            WriteBoundaryField {
                name: "failure_code",
                value: operation.failure_code.unwrap_or("").as_bytes(),
                sensitivity: WriteFieldSensitivity::PublicMetadata,
            },
        ];
        let scan = sanitize_write_boundary(WritePrivacyBoundary::Diagnostic, &fields)?;
        redacted_fields = redacted_fields
            .checked_add(scan.receipt.redacted_fields)
            .ok_or(WriteRecoveryError::InvalidInput)?;
        sanitized_operations.push(WriteAuditOperation {
            file: scan.fields[0].value.clone(),
            operation: operation.operation.to_owned(),
            preimage_sha256: operation.preimage_sha256.to_owned(),
            postimage_sha256: operation.postimage_sha256.to_owned(),
            validation: operation.validation,
            failure_code: operation.failure_code.map(|_| scan.fields[1].value.clone()),
            rollback_status: operation.rollback_status.to_owned(),
        });
    }
    let overall_status = match checkpoint.phase {
        WriteCheckpointPhase::Complete => "verified_complete",
        WriteCheckpointPhase::Restored => "verified_restored",
        WriteCheckpointPhase::FailedNoChange => "verified_no_change",
        WriteCheckpointPhase::Uncertain => "blocked_uncertain",
        _ => "in_progress",
    }
    .to_owned();
    let report = render_audit_report(
        &checkpoint.transaction_id,
        &checkpoint.action_id,
        &overall_status,
        &sanitized_operations,
    );
    let mut summary = StateChangeAuditSummary {
        schema_version: SCHEMA_VERSION,
        transaction_id: checkpoint.transaction_id.clone(),
        action_id: checkpoint.action_id.clone(),
        checkpoint_sha256: checkpoint.checkpoint_sha256.clone(),
        operations: sanitized_operations,
        redacted_fields,
        overall_status,
        report,
        audit_sha256: ZERO_SHA256.to_owned(),
    };
    summary.audit_sha256 = canonical_sha256(&summary)?;
    Ok(summary)
}

fn validate_checkpoint_input(input: &WriteAwareCheckpointInput) -> Result<(), WriteRecoveryError> {
    if !valid_identifier(&input.checkpoint_id)
        || !valid_identifier(&input.transaction_id)
        || !valid_identifier(&input.action_id)
        || input
            .consumed_grant_id
            .as_deref()
            .is_some_and(|value| !valid_identifier(value))
        || !valid_sha256(&input.evidence_set_sha256)
        || !valid_sha256(&input.next_session_checkpoint_sha256)
        || !valid_sha256(&input.secret_scan_receipt_sha256)
        || !valid_sha256(&input.staging_inventory_sha256)
        || input
            .file_receipt_head_sha256
            .as_deref()
            .is_some_and(|value| !valid_sha256(value))
        || input
            .index_update_sha256
            .as_deref()
            .is_some_and(|value| !valid_sha256(value))
        || input
            .failure_code
            .as_deref()
            .is_some_and(|value| !valid_code(value))
        || (input.file_receipt_count == 0) != input.file_receipt_head_sha256.is_none()
        || (input.staging_item_count == 0) != (input.retention_expires_at_epoch_ms == 0)
    {
        return Err(WriteRecoveryError::InvalidInput);
    }
    let consumed = input.consumed_grant_id.is_some();
    if (input.phase == WriteCheckpointPhase::BeforeTransaction && consumed)
        || (input.phase != WriteCheckpointPhase::BeforeTransaction && !consumed)
        || (input.index_verified && input.index_update_sha256.is_none())
        || (input.rollback_verified && input.phase != WriteCheckpointPhase::Restored)
    {
        return Err(WriteRecoveryError::AmbiguousCompletion);
    }
    if matches!(
        input.phase,
        WriteCheckpointPhase::CanonicalVerified
            | WriteCheckpointPhase::IndexUpdating
            | WriteCheckpointPhase::IndexVerified
            | WriteCheckpointPhase::ReceiptPersisting
            | WriteCheckpointPhase::ReceiptPersisted
            | WriteCheckpointPhase::CleanupPending
            | WriteCheckpointPhase::Complete
    ) && (!input.canonical_postimages_verified || input.file_receipt_count == 0)
    {
        return Err(WriteRecoveryError::AmbiguousCompletion);
    }
    if matches!(
        input.phase,
        WriteCheckpointPhase::ReceiptPersisted
            | WriteCheckpointPhase::FailedNoChange
            | WriteCheckpointPhase::Restored
            | WriteCheckpointPhase::CleanupPending
            | WriteCheckpointPhase::Complete
    ) && !input.receipt_chain_verified
    {
        return Err(WriteRecoveryError::AmbiguousCompletion);
    }
    if input.phase == WriteCheckpointPhase::IndexVerified && !input.index_verified {
        return Err(WriteRecoveryError::AmbiguousCompletion);
    }
    if input.phase == WriteCheckpointPhase::Restored && !input.rollback_verified {
        return Err(WriteRecoveryError::AmbiguousCompletion);
    }
    if input.phase == WriteCheckpointPhase::Complete
        && (!input.canonical_postimages_verified
            || !input.receipt_chain_verified
            || (input.index_update_sha256.is_some() && !input.index_verified)
            || !matches!(
                input.cleanup_state,
                WriteCleanupState::Clean | WriteCleanupState::NotRequired
            ))
    {
        return Err(WriteRecoveryError::AmbiguousCompletion);
    }
    if input.phase == WriteCheckpointPhase::CleanupPending
        && (input.staging_item_count == 0 || input.cleanup_state != WriteCleanupState::Pending)
    {
        return Err(WriteRecoveryError::AmbiguousCompletion);
    }
    Ok(())
}

const fn allowed_transition(from: WriteCheckpointPhase, to: WriteCheckpointPhase) -> bool {
    use WriteCheckpointPhase as P;
    matches!(
        (from, to),
        (P::BeforeTransaction, P::GrantConsumed | P::FailedNoChange)
            | (
                P::GrantConsumed,
                P::Staging | P::Applying | P::FailedNoChange | P::Uncertain
            )
            | (
                P::Staging,
                P::Applying | P::FailedNoChange | P::RollbackPending | P::Uncertain
            )
            | (
                P::Applying,
                P::CanonicalVerified | P::RollbackPending | P::Uncertain
            )
            | (
                P::CanonicalVerified,
                P::IndexUpdating | P::ReceiptPersisting | P::CleanupPending
            )
            | (P::IndexUpdating, P::IndexVerified | P::Uncertain)
            | (P::IndexVerified, P::ReceiptPersisting | P::CleanupPending)
            | (P::ReceiptPersisting, P::ReceiptPersisted | P::Uncertain)
            | (P::ReceiptPersisted, P::CleanupPending | P::Complete)
            | (P::RollbackPending, P::Restored | P::Uncertain)
            | (P::CleanupPending, P::Complete | P::Uncertain)
    )
}

fn render_audit_report(
    transaction_id: &str,
    action_id: &str,
    overall_status: &str,
    operations: &[WriteAuditOperation],
) -> String {
    let mut report = format!(
        "State-change audit\nTransaction: {transaction_id}\nAction: {action_id}\nStatus: {overall_status}\n"
    );
    for operation in operations {
        let failure = operation.failure_code.as_deref().unwrap_or("none");
        writeln!(
            report,
            "File: {} | Operation: {} | Validation: {:?} | Failure: {} | Rollback: {}",
            operation.file,
            operation.operation,
            operation.validation,
            failure,
            operation.rollback_status
        )
        .expect("writing to String cannot fail");
    }
    report
}

const fn secret_class_code(class: SecretFindingClass) -> &'static str {
    match class {
        SecretFindingClass::CredentialField => "credential_field",
        SecretFindingClass::PrivateKey => "private_key",
        SecretFindingClass::BearerCredential => "bearer_credential",
        SecretFindingClass::ProviderToken => "provider_token",
        SecretFindingClass::CloudAccessKey => "cloud_access_key",
        SecretFindingClass::EmbeddedUriCredential => "embedded_uri_credential",
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':'))
}

fn valid_label(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_TEXT_BYTES
        && !value.chars().any(char::is_control)
        && detect_secret_classes("label", value.as_bytes()).is_empty()
}

fn valid_code(value: &str) -> bool {
    valid_identifier(value)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn canonical_sha256(value: &impl Serialize) -> Result<String, WriteRecoveryError> {
    let bytes = serde_json::to_vec(value).map_err(|_| WriteRecoveryError::SerializationFailed)?;
    Ok(sha256_hex(&bytes))
}

fn sha256_hex(value: &[u8]) -> String {
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(byte: char) -> String {
        byte.to_string().repeat(64)
    }

    fn input(phase: WriteCheckpointPhase) -> WriteAwareCheckpointInput {
        let consumed = phase != WriteCheckpointPhase::BeforeTransaction;
        let postimage = matches!(
            phase,
            WriteCheckpointPhase::CanonicalVerified
                | WriteCheckpointPhase::IndexUpdating
                | WriteCheckpointPhase::IndexVerified
                | WriteCheckpointPhase::ReceiptPersisting
                | WriteCheckpointPhase::ReceiptPersisted
                | WriteCheckpointPhase::CleanupPending
                | WriteCheckpointPhase::Complete
        );
        let receipt_verified = matches!(
            phase,
            WriteCheckpointPhase::ReceiptPersisted
                | WriteCheckpointPhase::FailedNoChange
                | WriteCheckpointPhase::Restored
                | WriteCheckpointPhase::CleanupPending
                | WriteCheckpointPhase::Complete
        );
        let has_receipt = postimage || receipt_verified;
        let has_index = matches!(
            phase,
            WriteCheckpointPhase::IndexUpdating
                | WriteCheckpointPhase::IndexVerified
                | WriteCheckpointPhase::ReceiptPersisting
                | WriteCheckpointPhase::ReceiptPersisted
                | WriteCheckpointPhase::CleanupPending
                | WriteCheckpointPhase::Complete
        );
        let staging = phase == WriteCheckpointPhase::CleanupPending;
        WriteAwareCheckpointInput {
            checkpoint_id: format!("checkpoint-{phase:?}"),
            transaction_id: "transaction-1".to_owned(),
            action_id: "action-1".to_owned(),
            phase,
            consumed_grant_id: consumed.then(|| "grant-1".to_owned()),
            file_receipt_head_sha256: has_receipt.then(|| hash('a')),
            file_receipt_count: u32::from(has_receipt),
            evidence_set_sha256: hash('b'),
            index_update_sha256: has_index.then(|| hash('c')),
            next_session_checkpoint_sha256: hash('d'),
            secret_scan_receipt_sha256: hash('e'),
            staging_inventory_sha256: hash('f'),
            staging_item_count: u32::from(staging),
            retention_expires_at_epoch_ms: if staging { 9_000 } else { 0 },
            canonical_postimages_verified: postimage,
            receipt_chain_verified: receipt_verified,
            index_verified: matches!(
                phase,
                WriteCheckpointPhase::IndexVerified
                    | WriteCheckpointPhase::ReceiptPersisting
                    | WriteCheckpointPhase::ReceiptPersisted
                    | WriteCheckpointPhase::CleanupPending
                    | WriteCheckpointPhase::Complete
            ),
            rollback_verified: phase == WriteCheckpointPhase::Restored,
            cleanup_state: if staging {
                WriteCleanupState::Pending
            } else if phase == WriteCheckpointPhase::Complete {
                WriteCleanupState::Clean
            } else {
                WriteCleanupState::NotRequired
            },
            failure_code: matches!(
                phase,
                WriteCheckpointPhase::FailedNoChange
                    | WriteCheckpointPhase::Restored
                    | WriteCheckpointPhase::Uncertain
            )
            .then(|| "write.failed".to_owned()),
            occurred_at_epoch_ms: 1_000,
        }
    }

    fn standalone(phase: WriteCheckpointPhase) -> WriteAwareCheckpoint {
        let mut value = input(phase);
        if phase != WriteCheckpointPhase::BeforeTransaction {
            value.phase = WriteCheckpointPhase::BeforeTransaction;
            value.consumed_grant_id = None;
            value.file_receipt_head_sha256 = None;
            value.file_receipt_count = 0;
            value.index_update_sha256 = None;
            value.canonical_postimages_verified = false;
            value.receipt_chain_verified = false;
            value.index_verified = false;
            value.rollback_verified = false;
            value.failure_code = None;
            value.checkpoint_id = "checkpoint-before".to_owned();
        }
        let first = build_write_checkpoint(value, None).expect("first checkpoint");
        if phase == WriteCheckpointPhase::BeforeTransaction {
            first
        } else {
            let mut next = input(phase);
            next.occurred_at_epoch_ms = 2_000;
            // The unit needs only a structurally valid checkpoint; direct transitions are
            // exercised separately by the complete chain test.
            let mut checkpoint = WriteAwareCheckpoint {
                schema_version: SCHEMA_VERSION,
                checkpoint_id: next.checkpoint_id,
                transaction_id: next.transaction_id,
                action_id: next.action_id,
                sequence: 1,
                phase: next.phase,
                consumed_grant_id: next.consumed_grant_id,
                file_receipt_head_sha256: next.file_receipt_head_sha256,
                file_receipt_count: next.file_receipt_count,
                evidence_set_sha256: next.evidence_set_sha256,
                index_update_sha256: next.index_update_sha256,
                next_session_checkpoint_sha256: next.next_session_checkpoint_sha256,
                secret_scan_receipt_sha256: next.secret_scan_receipt_sha256,
                staging_inventory_sha256: next.staging_inventory_sha256,
                staging_item_count: next.staging_item_count,
                retention_expires_at_epoch_ms: next.retention_expires_at_epoch_ms,
                canonical_postimages_verified: next.canonical_postimages_verified,
                receipt_chain_verified: next.receipt_chain_verified,
                index_verified: next.index_verified,
                rollback_verified: next.rollback_verified,
                cleanup_state: next.cleanup_state,
                failure_code: next.failure_code,
                occurred_at_epoch_ms: next.occurred_at_epoch_ms,
                previous_checkpoint_sha256: first.checkpoint_sha256,
                checkpoint_sha256: ZERO_SHA256.to_owned(),
            };
            checkpoint.checkpoint_sha256 = canonical_sha256(&checkpoint).expect("hash");
            verify_write_checkpoint(&checkpoint).expect("valid standalone checkpoint");
            checkpoint
        }
    }

    #[test]
    fn privacy_gate_covers_every_boundary_without_retaining_removed_values() {
        const TOKEN: &str = concat!("gh", "p_abcdefghijklmnopqrstuvwxyz1234567890");
        for boundary in WritePrivacyBoundary::ALL {
            let result = sanitize_write_boundary(
                boundary,
                &[
                    WriteBoundaryField {
                        name: "path",
                        value: b"notes/project.md",
                        sensitivity: WriteFieldSensitivity::PublicMetadata,
                    },
                    WriteBoundaryField {
                        name: "private_excerpt",
                        value: b"private user paragraph",
                        sensitivity: WriteFieldSensitivity::PrivateExcerpt,
                    },
                    WriteBoundaryField {
                        name: "provider_value",
                        value: TOKEN.as_bytes(),
                        sensitivity: WriteFieldSensitivity::PublicMetadata,
                    },
                ],
            )
            .expect("scan");
            let encoded = serde_json::to_string(&(result.fields, result.receipt)).expect("json");
            assert!(encoded.contains("notes/project.md"));
            assert!(!encoded.contains("private user paragraph"));
            assert!(!encoded.contains(TOKEN));
            assert_eq!(encoded.matches(REDACTED_VALUE).count(), 2);
        }
    }

    #[test]
    fn checkpoint_schema_accepts_every_state_and_rejects_ambiguous_completion() {
        for phase in WriteCheckpointPhase::ALL {
            verify_write_checkpoint(&standalone(phase)).expect("phase contract");
        }
        let mut incomplete = input(WriteCheckpointPhase::Complete);
        incomplete.receipt_chain_verified = false;
        assert_eq!(
            validate_checkpoint_input(&incomplete),
            Err(WriteRecoveryError::AmbiguousCompletion)
        );
    }

    #[test]
    fn checkpoint_chain_detects_mutation_and_terminal_extension() {
        let mut first_input = input(WriteCheckpointPhase::BeforeTransaction);
        first_input.checkpoint_id = "checkpoint-0".to_owned();
        let first = build_write_checkpoint(first_input, None).expect("first");
        let mut consumed_input = input(WriteCheckpointPhase::GrantConsumed);
        consumed_input.checkpoint_id = "checkpoint-1".to_owned();
        consumed_input.occurred_at_epoch_ms = 2_000;
        let consumed = build_write_checkpoint(consumed_input, Some(&first)).expect("consumed");
        let mut failed_input = input(WriteCheckpointPhase::FailedNoChange);
        failed_input.checkpoint_id = "checkpoint-2".to_owned();
        failed_input.occurred_at_epoch_ms = 3_000;
        let failed = build_write_checkpoint(failed_input, Some(&consumed)).expect("failed");
        verify_write_checkpoint_chain(&[first.clone(), consumed.clone(), failed.clone()])
            .expect("chain");
        assert_eq!(
            build_write_checkpoint(input(WriteCheckpointPhase::Complete), Some(&failed)),
            Err(WriteRecoveryError::CheckpointIntegrity)
        );
        let mut changed = consumed;
        changed.action_id = "action-drift".to_owned();
        assert_eq!(
            verify_write_checkpoint_chain(&[first, changed, failed]),
            Err(WriteRecoveryError::CheckpointIntegrity)
        );
    }

    #[test]
    fn restart_matrix_never_replays_a_consumed_or_completed_write() {
        let base = WriteRecoveryObservation {
            root_identity_matches: true,
            concurrent_change_detected: false,
            secret_store_available: true,
            canonical_postimages_match: true,
            receipt_chain_verified: true,
            index_verified: true,
            staging_items_present: 0,
            staging_retention_expired: false,
        };
        let cases = [
            (
                WriteCheckpointPhase::BeforeTransaction,
                base.clone(),
                WriteRecoveryInstruction::BuildFreshProposal,
            ),
            (
                WriteCheckpointPhase::Uncertain,
                base.clone(),
                WriteRecoveryInstruction::BlockUncertainState,
            ),
            (
                WriteCheckpointPhase::Complete,
                WriteRecoveryObservation {
                    concurrent_change_detected: true,
                    ..base.clone()
                },
                WriteRecoveryInstruction::PreserveConflictForReview,
            ),
            (
                WriteCheckpointPhase::Complete,
                WriteRecoveryObservation {
                    secret_store_available: false,
                    ..base.clone()
                },
                WriteRecoveryInstruction::RestoreSecretStore,
            ),
            (
                WriteCheckpointPhase::Complete,
                WriteRecoveryObservation {
                    root_identity_matches: false,
                    ..base.clone()
                },
                WriteRecoveryInstruction::RestoreWorkspaceIdentity,
            ),
            (
                WriteCheckpointPhase::Complete,
                WriteRecoveryObservation {
                    staging_items_present: 1,
                    staging_retention_expired: true,
                    ..base.clone()
                },
                WriteRecoveryInstruction::ProposeStagingCleanup,
            ),
            (
                WriteCheckpointPhase::Complete,
                base,
                WriteRecoveryInstruction::NoActionComplete,
            ),
        ];
        for (phase, observation, expected) in cases {
            let decision = plan_write_recovery(&standalone(phase), &observation).expect("decision");
            assert_eq!(decision.instruction, expected);
            assert!(!decision.repeat_completed_write);
        }
    }

    #[test]
    fn orphan_inventory_is_attributable_and_cleanup_is_single_use() {
        let complete = standalone(WriteCheckpointPhase::Complete);
        let inventory = [StagingInventoryItem {
            staging_id: "staging-1".to_owned(),
            transaction_id: complete.transaction_id.clone(),
            action_id: complete.action_id.clone(),
            content_sha256: hash('a'),
            created_at_epoch_ms: 1_000,
            expires_at_epoch_ms: 2_000,
            state: StagingObjectState::Orphaned,
        }];
        let diagnostics =
            diagnose_staging_inventory(3_000, &[complete], &inventory).expect("diagnostics");
        assert!(diagnostics[0].attributable);
        assert!(diagnostics[0].orphaned);
        assert!(diagnostics[0].expired);
        assert!(diagnostics[0].cleanup_eligible);

        let mut ledger = StagingCleanupLedger::new();
        ledger
            .record(StagingCleanupRecordInput {
                cleanup_id: "cleanup-1".to_owned(),
                staging_id: "staging-1".to_owned(),
                transaction_id: "transaction-1".to_owned(),
                disposition: StagingCleanupDisposition::Removed,
                inventory_before_sha256: hash('b'),
                inventory_after_sha256: hash('c'),
                occurred_at_epoch_ms: 3_100,
            })
            .expect("receipt");
        assert_eq!(
            ledger.record(StagingCleanupRecordInput {
                cleanup_id: "cleanup-2".to_owned(),
                staging_id: "staging-1".to_owned(),
                transaction_id: "transaction-1".to_owned(),
                disposition: StagingCleanupDisposition::Removed,
                inventory_before_sha256: hash('c'),
                inventory_after_sha256: hash('c'),
                occurred_at_epoch_ms: 3_200,
            }),
            Err(WriteRecoveryError::CleanupReplay)
        );
    }

    #[test]
    fn audit_reports_exact_safe_files_and_redacts_secret_like_paths() {
        const TOKEN: &str = concat!("gh", "p_abcdefghijklmnopqrstuvwxyz1234567890");
        let checkpoint = standalone(WriteCheckpointPhase::Complete);
        let operations = [
            WriteAuditOperationInput {
                file: "src/lib.rs",
                operation: "replace",
                preimage_sha256: &hash('a'),
                postimage_sha256: &hash('b'),
                validation: WriteAuditValidation::Verified,
                failure_code: None,
                rollback_status: "not_required",
            },
            WriteAuditOperationInput {
                file: TOKEN,
                operation: "create",
                preimage_sha256: ZERO_SHA256,
                postimage_sha256: &hash('c'),
                validation: WriteAuditValidation::Verified,
                failure_code: None,
                rollback_status: "not_required",
            },
        ];
        let audit = build_state_change_audit(&checkpoint, &operations).expect("audit");
        let encoded = serde_json::to_string(&audit).expect("json");
        assert!(audit.report.contains("src/lib.rs"));
        assert!(!encoded.contains(TOKEN));
        assert_eq!(audit.redacted_fields, 1);
        assert!(valid_sha256(&audit.audit_sha256));
    }
}
