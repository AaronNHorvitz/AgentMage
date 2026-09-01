//! Durable append-only recovery for authority-free frontier import transactions.

use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAX_CHECKPOINT_BYTES: usize = 64 * 1024;
const MAX_CHECKPOINT_GENERATIONS: usize = 4;
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Stable failure while sealing or persisting a frontier-import checkpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrontierImportRecoveryError {
    /// A checkpoint was malformed, corrupt, forked, or phase-inconsistent.
    InvalidCheckpoint,
    /// The exact append-only local storage operation failed.
    StorageFailure,
}

impl FrontierImportRecoveryError {
    /// Stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidCheckpoint => "frontier.import-recovery.checkpoint-invalid",
            Self::StorageFailure => "frontier.import-recovery.storage-failed",
        }
    }
}

impl std::fmt::Display for FrontierImportRecoveryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for FrontierImportRecoveryError {}

/// Monotonic durable phase of one import transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrontierImportPhase {
    /// The exact manifest passed its closed parser.
    Parsed,
    /// Imported artifacts and citations were revalidated against current state.
    Revalidated,
    /// Every eligible step entered one normal local proposal flow.
    Routed,
    /// The no-effect round-trip receipt was sealed.
    Completed,
}

/// Content-free, hash-chained durable checkpoint for one import transaction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontierImportCheckpoint {
    /// Stable host transaction identity.
    pub transaction_id: String,
    /// Monotonic checkpoint generation, beginning at one.
    pub generation: u32,
    /// Last fully committed phase.
    pub phase: FrontierImportPhase,
    /// Exact request packet digest.
    pub request_packet_sha256: String,
    /// Exact return-manifest digest.
    pub manifest_sha256: String,
    /// Current-state digest once local revalidation completed.
    pub current_state_sha256: Option<String>,
    /// Local revalidation report digest once available.
    pub report_sha256: Option<String>,
    /// Deterministic digest of all local-flow tickets once routed.
    pub tickets_sha256: Option<String>,
    /// Terminal round-trip receipt digest once completed.
    pub receipt_sha256: Option<String>,
    /// Previous checkpoint digest, absent only for generation one.
    pub previous_checkpoint_sha256: Option<String>,
    /// Fixed false: checkpoints carry no execution authority.
    pub execution_authority: bool,
    /// Fixed zero: checkpoint publication applies no product effect.
    pub applied_effect_count: u32,
    /// Canonical digest with this field zeroed.
    pub checkpoint_sha256: String,
}

/// Seals one complete checkpoint after validating its closed phase shape.
pub fn seal_frontier_import_checkpoint(
    mut checkpoint: FrontierImportCheckpoint,
) -> Result<FrontierImportCheckpoint, FrontierImportRecoveryError> {
    checkpoint.checkpoint_sha256 = ZERO_SHA256.to_owned();
    validate_checkpoint_shape(&checkpoint)?;
    checkpoint.checkpoint_sha256 = checkpoint_digest(&checkpoint)?;
    verify_frontier_import_checkpoint(&checkpoint)?;
    Ok(checkpoint)
}

/// Verifies one checkpoint's closed shape and canonical digest.
pub fn verify_frontier_import_checkpoint(
    checkpoint: &FrontierImportCheckpoint,
) -> Result<(), FrontierImportRecoveryError> {
    validate_checkpoint_shape(checkpoint)?;
    if !valid_sha256(&checkpoint.checkpoint_sha256)
        || checkpoint.checkpoint_sha256 != checkpoint_digest(checkpoint)?
    {
        return Err(FrontierImportRecoveryError::InvalidCheckpoint);
    }
    Ok(())
}

/// Append-only atomic checkpoint store beneath one caller-owned local directory.
///
/// Every generation is a separate content-addressed file. Reopen verifies the complete chain and
/// refuses symlinks, gaps, forks, malformed names, tampering, or a phase/generation mismatch.
#[derive(Clone, Debug)]
pub struct DirectoryFrontierImportCheckpointStore {
    root: PathBuf,
}

impl DirectoryFrontierImportCheckpointStore {
    /// Opens or creates one exact checkpoint directory without following a pre-existing symlink.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, FrontierImportRecoveryError> {
        let root = root.as_ref();
        match fs::symlink_metadata(root) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(FrontierImportRecoveryError::StorageFailure);
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(root).map_err(|_| FrontierImportRecoveryError::StorageFailure)?;
            }
            Err(_) => return Err(FrontierImportRecoveryError::StorageFailure),
        }
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    /// Loads and verifies the latest complete generation for one exact transaction.
    pub fn load_latest(
        &self,
        transaction_id: &str,
    ) -> Result<Option<FrontierImportCheckpoint>, FrontierImportRecoveryError> {
        Ok(self.checkpoints(transaction_id)?.pop())
    }

    /// Atomically appends one exact next generation.
    pub fn commit(
        &mut self,
        checkpoint: &FrontierImportCheckpoint,
    ) -> Result<(), FrontierImportRecoveryError> {
        verify_frontier_import_checkpoint(checkpoint)?;
        let current = self.load_latest(&checkpoint.transaction_id)?;
        let expected_generation = current
            .as_ref()
            .map_or(1, |value| value.generation.saturating_add(1));
        let expected_previous = current
            .as_ref()
            .map(|value| value.checkpoint_sha256.as_str());
        if checkpoint.generation != expected_generation
            || checkpoint.previous_checkpoint_sha256.as_deref() != expected_previous
            || checkpoint.generation as usize > MAX_CHECKPOINT_GENERATIONS
        {
            return Err(FrontierImportRecoveryError::InvalidCheckpoint);
        }
        let bytes = serde_json::to_vec(checkpoint)
            .map_err(|_| FrontierImportRecoveryError::InvalidCheckpoint)?;
        if bytes.len() > MAX_CHECKPOINT_BYTES {
            return Err(FrontierImportRecoveryError::InvalidCheckpoint);
        }
        let final_path = self.root.join(checkpoint_filename(checkpoint));
        let pending_path = self.root.join(format!(
            ".{}.{}.pending",
            raw_sha256(checkpoint.transaction_id.as_bytes()),
            checkpoint.checkpoint_sha256
        ));
        let result = (|| {
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&pending_path)
            {
                Ok(mut pending) => {
                    pending.write_all(&bytes)?;
                    pending.sync_all()?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    if fs::read(&pending_path)? != bytes {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "frontier checkpoint pending file mismatch",
                        ));
                    }
                }
                Err(error) => return Err(error),
            }
            fs::rename(&pending_path, &final_path)?;
            File::open(&self.root)?.sync_all()?;
            Ok::<(), std::io::Error>(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&pending_path);
            return Err(FrontierImportRecoveryError::StorageFailure);
        }
        Ok(())
    }

    fn checkpoints(
        &self,
        transaction_id: &str,
    ) -> Result<Vec<FrontierImportCheckpoint>, FrontierImportRecoveryError> {
        if !valid_identifier(transaction_id) {
            return Err(FrontierImportRecoveryError::InvalidCheckpoint);
        }
        let prefix = format!("{}.", raw_sha256(transaction_id.as_bytes()));
        let mut checkpoints = Vec::new();
        for entry in
            fs::read_dir(&self.root).map_err(|_| FrontierImportRecoveryError::StorageFailure)?
        {
            let entry = entry.map_err(|_| FrontierImportRecoveryError::StorageFailure)?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| FrontierImportRecoveryError::InvalidCheckpoint)?;
            if !name.starts_with(&prefix) {
                continue;
            }
            let file_type = entry
                .file_type()
                .map_err(|_| FrontierImportRecoveryError::StorageFailure)?;
            if file_type.is_symlink() || !file_type.is_file() || !name.ends_with(".json") {
                return Err(FrontierImportRecoveryError::InvalidCheckpoint);
            }
            if checkpoints.len() >= MAX_CHECKPOINT_GENERATIONS {
                return Err(FrontierImportRecoveryError::InvalidCheckpoint);
            }
            let bytes =
                fs::read(entry.path()).map_err(|_| FrontierImportRecoveryError::StorageFailure)?;
            if bytes.is_empty() || bytes.len() > MAX_CHECKPOINT_BYTES {
                return Err(FrontierImportRecoveryError::InvalidCheckpoint);
            }
            let checkpoint: FrontierImportCheckpoint = serde_json::from_slice(&bytes)
                .map_err(|_| FrontierImportRecoveryError::InvalidCheckpoint)?;
            verify_frontier_import_checkpoint(&checkpoint)?;
            if checkpoint.transaction_id != transaction_id
                || name != checkpoint_filename(&checkpoint)
            {
                return Err(FrontierImportRecoveryError::InvalidCheckpoint);
            }
            checkpoints.push(checkpoint);
        }
        checkpoints.sort_by_key(|checkpoint| checkpoint.generation);
        for (index, checkpoint) in checkpoints.iter().enumerate() {
            let expected_generation = u32::try_from(index + 1)
                .map_err(|_| FrontierImportRecoveryError::InvalidCheckpoint)?;
            let expected_phase = match expected_generation {
                1 => FrontierImportPhase::Parsed,
                2 => FrontierImportPhase::Revalidated,
                3 => FrontierImportPhase::Routed,
                4 => FrontierImportPhase::Completed,
                _ => return Err(FrontierImportRecoveryError::InvalidCheckpoint),
            };
            let expected_previous = index
                .checked_sub(1)
                .map(|previous| checkpoints[previous].checkpoint_sha256.as_str());
            if checkpoint.generation != expected_generation
                || checkpoint.phase != expected_phase
                || checkpoint.previous_checkpoint_sha256.as_deref() != expected_previous
            {
                return Err(FrontierImportRecoveryError::InvalidCheckpoint);
            }
        }
        Ok(checkpoints)
    }
}

fn validate_checkpoint_shape(
    checkpoint: &FrontierImportCheckpoint,
) -> Result<(), FrontierImportRecoveryError> {
    let phase_fields_valid = match checkpoint.phase {
        FrontierImportPhase::Parsed => {
            checkpoint.current_state_sha256.is_none()
                && checkpoint.report_sha256.is_none()
                && checkpoint.tickets_sha256.is_none()
                && checkpoint.receipt_sha256.is_none()
        }
        FrontierImportPhase::Revalidated => {
            checkpoint.current_state_sha256.is_some()
                && checkpoint.report_sha256.is_some()
                && checkpoint.tickets_sha256.is_none()
                && checkpoint.receipt_sha256.is_none()
        }
        FrontierImportPhase::Routed => {
            checkpoint.current_state_sha256.is_some()
                && checkpoint.report_sha256.is_some()
                && checkpoint.tickets_sha256.is_some()
                && checkpoint.receipt_sha256.is_none()
        }
        FrontierImportPhase::Completed => {
            checkpoint.current_state_sha256.is_some()
                && checkpoint.report_sha256.is_some()
                && checkpoint.tickets_sha256.is_some()
                && checkpoint.receipt_sha256.is_some()
        }
    };
    if !valid_identifier(&checkpoint.transaction_id)
        || checkpoint.generation == 0
        || !valid_sha256(&checkpoint.request_packet_sha256)
        || !valid_sha256(&checkpoint.manifest_sha256)
        || checkpoint
            .current_state_sha256
            .as_deref()
            .is_some_and(|value| !valid_sha256(value))
        || checkpoint
            .report_sha256
            .as_deref()
            .is_some_and(|value| !valid_sha256(value))
        || checkpoint
            .tickets_sha256
            .as_deref()
            .is_some_and(|value| !valid_sha256(value))
        || checkpoint
            .receipt_sha256
            .as_deref()
            .is_some_and(|value| !valid_sha256(value))
        || checkpoint
            .previous_checkpoint_sha256
            .as_deref()
            .is_some_and(|value| !valid_sha256(value))
        || checkpoint.generation == 1 && checkpoint.previous_checkpoint_sha256.is_some()
        || checkpoint.generation > 1 && checkpoint.previous_checkpoint_sha256.is_none()
        || !phase_fields_valid
        || checkpoint.execution_authority
        || checkpoint.applied_effect_count != 0
    {
        return Err(FrontierImportRecoveryError::InvalidCheckpoint);
    }
    Ok(())
}

fn checkpoint_digest(
    checkpoint: &FrontierImportCheckpoint,
) -> Result<String, FrontierImportRecoveryError> {
    let mut unsigned = checkpoint.clone();
    unsigned.checkpoint_sha256 = ZERO_SHA256.to_owned();
    serde_json::to_vec(&("agentmage-frontier-import-checkpoint-v1", unsigned))
        .map(|bytes| raw_sha256(&bytes))
        .map_err(|_| FrontierImportRecoveryError::InvalidCheckpoint)
}

fn checkpoint_filename(checkpoint: &FrontierImportCheckpoint) -> String {
    format!(
        "{}.{:08}.{}.json",
        raw_sha256(checkpoint.transaction_id.as_bytes()),
        checkpoint.generation,
        checkpoint.checkpoint_sha256
    )
}

fn raw_sha256(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to a string cannot fail");
    }
    output
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
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
