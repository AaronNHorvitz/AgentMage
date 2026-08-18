//! Crash-safe activation, rollback, and recovery for verified model artifacts.

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use agentmage_kernel_contracts::{ExactModelProfile, ModelProfileId};
use rustix::fs::{FlockOperation, Mode, OFlags, flock, open, openat};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::model_acquisition::{
    ModelImportError, StoreHandle, open_store, retain_without_overwrite,
};

const ACTIVE_MANIFEST_NAME: &str = "active-model.json";
const INSTALLER_LOCK_NAME: &str = ".installer.lock";
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
const MAX_STORE_ENTRIES: usize = 4096;

/// Exact content-free malware and format scan result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelArtifactScanReport {
    /// Exact profile scanned.
    pub profile_id: ModelProfileId,
    /// Exact artifact digest scanned.
    pub artifact_sha256: String,
    /// Exact scanner policy identity.
    pub scanner_policy_sha256: String,
    /// GGUF structure and declared bounds were valid.
    pub format_valid: bool,
    /// No executable-bearing payload was identified.
    pub executable_payload_detected: bool,
    /// No scanner policy indicator was identified.
    pub malware_indicator_detected: bool,
    /// Scanner completed its entire required policy.
    pub complete: bool,
}

/// Exact content-free install self-test result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelInstallSelfTestReport {
    /// Exact profile exercised.
    pub profile_id: ModelProfileId,
    /// Exact manifest exercised.
    pub manifest_sha256: String,
    /// Exact artifact exercised.
    pub artifact_sha256: String,
    /// Exact runtime exercised.
    pub runtime_sha256: String,
    /// Runtime loaded the artifact successfully.
    pub loaded: bool,
    /// Runtime reached exact ready state.
    pub ready: bool,
    /// Runtime unloaded and reaped all children.
    pub unloaded: bool,
    /// Runtime observed no network authority.
    pub network_available: bool,
    /// Runtime observed no workspace authority.
    pub workspace_available: bool,
    /// Runtime observed no session authority.
    pub session_available: bool,
    /// Runtime observed no inference grant outside the self-test.
    pub unrelated_inference_available: bool,
    /// Runtime observed no tool authority.
    pub tool_available: bool,
}

/// Trusted verifier boundary used before selection can change.
pub trait ModelInstallVerifier {
    /// Runs the complete deterministic format and malware policy.
    fn scan(
        &mut self,
        artifact: &Path,
        profile: &ExactModelProfile,
    ) -> Option<ModelArtifactScanReport>;

    /// Loads, checks readiness, unloads, and reports exact isolation.
    fn load_unload(
        &mut self,
        artifact: &Path,
        profile: &ExactModelProfile,
    ) -> Option<ModelInstallSelfTestReport>;
}

/// Durable active-model record. Presence is necessary but not sufficient for product selection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActiveModelRecord {
    /// Exact profile identity.
    pub profile_id: ModelProfileId,
    /// Exact profile manifest digest.
    pub manifest_sha256: String,
    /// Exact content-addressed artifact filename.
    pub artifact_name: String,
    /// Exact artifact digest.
    pub artifact_sha256: String,
    /// Exact runtime digest used by the passing self-test.
    pub runtime_sha256: String,
    /// Exact scanner policy used by the passing scan.
    pub scanner_policy_sha256: String,
}

/// Durable activation manifest with at most one rollback predecessor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActiveModelManifest {
    /// Closed manifest schema version.
    pub schema_version: u16,
    /// Monotonic local generation.
    pub generation: u64,
    /// Current exact active record.
    pub active: ActiveModelRecord,
    /// Previous exact record retained for one-step rollback.
    pub previous: Option<ActiveModelRecord>,
}

/// Observable activation transition for crash-recovery verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModelActivationStage {
    /// Exact bytes and source staging identity passed.
    ArtifactVerified,
    /// Complete scanner policy passed.
    ScanPassed,
    /// Exact isolated load/unload self-test passed.
    SelfTestPassed,
    /// Content-addressed artifact was promoted but not yet selected.
    ArtifactPromoted,
    /// Next manifest was durably written but not yet renamed.
    ManifestPrepared,
    /// Active manifest rename and directory sync completed.
    ManifestCommitted,
}

/// Observable rollback transition for crash-recovery verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModelRollbackStage {
    /// The predecessor and its retained artifact were revalidated.
    PredecessorVerified,
    /// The rollback manifest was durably written but not yet renamed.
    ManifestPrepared,
    /// The rollback manifest rename and directory sync completed.
    ManifestCommitted,
}

/// Terminal activation disposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelActivationDisposition {
    /// New exact record became active after every gate passed.
    Activated,
    /// Scan or self-test rejected staging and preserved prior selection.
    RejectedPreservedPrevious,
    /// Prior exact record became active again.
    RolledBack,
}

/// Content-free receipt for activation or rollback.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelActivationReceipt {
    /// Terminal disposition.
    pub disposition: ModelActivationDisposition,
    /// Exact resulting generation, or unchanged generation after rejection.
    pub generation: u64,
    /// Exact resulting active profile, when one exists.
    pub active_profile_id: Option<ModelProfileId>,
    /// Exact rejected profile, when rejection occurred.
    pub rejected_profile_id: Option<ModelProfileId>,
    /// Relative retained quarantine name, when rejection occurred.
    pub quarantine_name: Option<String>,
    /// Installer had no workspace authority.
    pub workspace_available: bool,
    /// Installer had no session authority.
    pub session_available: bool,
    /// Installer had no inference authority beyond the exact self-test.
    pub unrelated_inference_available: bool,
    /// Installer had no tool authority.
    pub tool_available: bool,
    /// Installer had no network authority.
    pub network_available: bool,
}

/// Bounded recovery result after interrupted lifecycle work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelStoreRecoveryReceipt {
    /// Exact active profile retained after reconciliation.
    pub active_profile_id: Option<ModelProfileId>,
    /// Relative temporary names removed in sorted order.
    pub removed_names: Vec<String>,
    /// Relative quarantine and resumable names retained in sorted order.
    pub retained_inactive_names: Vec<String>,
    /// No installer process or network authority remains.
    pub installer_authority_present: bool,
}

/// Stable fail-closed lifecycle error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelInstallLifecycleError {
    /// Store, staged object, active manifest, or content identity was unsafe.
    UnsafeState,
    /// Another installer owns the exclusive lifecycle lock.
    Busy,
    /// Verified staging did not belong to the exact profile.
    StagingMismatch,
    /// A destination identity already existed with different content.
    Conflict,
    /// A deterministic interruption fixture stopped at the named stage.
    Interrupted(ModelActivationStage),
    /// A deterministic interruption fixture stopped at the named rollback stage.
    RollbackInterrupted(ModelRollbackStage),
    /// No previous exact record was available for rollback.
    RollbackUnavailable,
    /// A bounded filesystem operation failed.
    Filesystem,
}

impl ModelInstallLifecycleError {
    /// Returns a stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::UnsafeState => "model.install-lifecycle.unsafe-state",
            Self::Busy => "model.install-lifecycle.busy",
            Self::StagingMismatch => "model.install-lifecycle.staging-mismatch",
            Self::Conflict => "model.install-lifecycle.conflict",
            Self::Interrupted(_) | Self::RollbackInterrupted(_) => {
                "model.install-lifecycle.interrupted"
            }
            Self::RollbackUnavailable => "model.install-lifecycle.rollback-unavailable",
            Self::Filesystem => "model.install-lifecycle.filesystem",
        }
    }
}

impl std::fmt::Display for ModelInstallLifecycleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for ModelInstallLifecycleError {}

/// Scans, self-tests, and atomically selects one exact verified staging object.
pub fn activate_verified_model(
    profile: &ExactModelProfile,
    store: &Path,
    verified_name: &str,
    verifier: &mut impl ModelInstallVerifier,
    mut interrupt: impl FnMut(ModelActivationStage) -> bool,
) -> Result<ModelActivationReceipt, ModelInstallLifecycleError> {
    let store = open_store(store).map_err(map_store_error)?;
    let _lock = acquire_lock(&store)?;
    let prior = read_manifest(&store.held_path)?;
    let verified = validate_staged(profile, &store.held_path, verified_name)?;
    stop_if_requested(&mut interrupt, ModelActivationStage::ArtifactVerified)?;

    let scan = verifier
        .scan(&verified, profile)
        .filter(|report| valid_scan(report, profile));
    let Some(scan) = scan else {
        return reject_staging(profile, &store.held_path, &verified, prior.as_ref(), "scan");
    };
    stop_if_requested(&mut interrupt, ModelActivationStage::ScanPassed)?;
    let self_test = verifier
        .load_unload(&verified, profile)
        .filter(|report| valid_self_test(report, profile));
    if self_test.is_none() {
        return reject_staging(
            profile,
            &store.held_path,
            &verified,
            prior.as_ref(),
            "self-test",
        );
    }
    stop_if_requested(&mut interrupt, ModelActivationStage::SelfTestPassed)?;

    let installed_name = format!("{}.gguf", profile.artifact.sha256);
    let installed = store.held_path.join(&installed_name);
    if installed.exists() {
        if !exact_artifact(&installed, profile)? {
            return Err(ModelInstallLifecycleError::Conflict);
        }
        fs::remove_file(&verified).map_err(|_| ModelInstallLifecycleError::Filesystem)?;
    } else {
        retain_without_overwrite(&verified, &installed).map_err(map_retain_error)?;
    }
    if !exact_artifact(&installed, profile)? {
        return Err(ModelInstallLifecycleError::UnsafeState);
    }
    stop_if_requested(&mut interrupt, ModelActivationStage::ArtifactPromoted)?;

    let generation = prior
        .as_ref()
        .map_or(1, |manifest| manifest.generation.saturating_add(1));
    let active = ActiveModelRecord {
        profile_id: profile.profile_id.clone(),
        manifest_sha256: profile.manifest_sha256.clone(),
        artifact_name: installed_name,
        artifact_sha256: profile.artifact.sha256.clone(),
        runtime_sha256: profile.runtime.runtime_sha256.clone(),
        scanner_policy_sha256: scan.scanner_policy_sha256,
    };
    let next = ActiveModelManifest {
        schema_version: 1,
        generation,
        previous: prior.as_ref().map(|manifest| manifest.active.clone()),
        active,
    };
    prepare_manifest(&store.held_path, &next)?;
    stop_if_requested(&mut interrupt, ModelActivationStage::ManifestPrepared)?;
    commit_prepared_manifest(&store.held_path, generation)?;
    store.sync().map_err(map_store_error)?;
    stop_if_requested(&mut interrupt, ModelActivationStage::ManifestCommitted)?;
    Ok(activation_receipt(
        ModelActivationDisposition::Activated,
        generation,
        Some(profile.profile_id.clone()),
        None,
        None,
    ))
}

/// Revalidates and atomically restores the one retained predecessor.
pub fn rollback_active_model(
    store: &Path,
    verifier: impl FnMut(&ActiveModelRecord, &Path) -> bool,
) -> Result<ModelActivationReceipt, ModelInstallLifecycleError> {
    rollback_active_model_with_interruption(store, verifier, |_| false)
}

/// Revalidates and rolls back with deterministic crash boundaries for verification.
pub fn rollback_active_model_with_interruption(
    store: &Path,
    mut verifier: impl FnMut(&ActiveModelRecord, &Path) -> bool,
    mut interrupt: impl FnMut(ModelRollbackStage) -> bool,
) -> Result<ModelActivationReceipt, ModelInstallLifecycleError> {
    let store = open_store(store).map_err(map_store_error)?;
    let _lock = acquire_lock(&store)?;
    let current =
        read_manifest(&store.held_path)?.ok_or(ModelInstallLifecycleError::RollbackUnavailable)?;
    let previous = current
        .previous
        .clone()
        .ok_or(ModelInstallLifecycleError::RollbackUnavailable)?;
    let artifact = store.held_path.join(&previous.artifact_name);
    if !valid_record_artifact(&previous, &artifact)? || !verifier(&previous, &artifact) {
        return Err(ModelInstallLifecycleError::UnsafeState);
    }
    stop_rollback_if_requested(&mut interrupt, ModelRollbackStage::PredecessorVerified)?;
    let generation = current.generation.saturating_add(1);
    let next = ActiveModelManifest {
        schema_version: 1,
        generation,
        active: previous.clone(),
        previous: None,
    };
    prepare_manifest(&store.held_path, &next)?;
    stop_rollback_if_requested(&mut interrupt, ModelRollbackStage::ManifestPrepared)?;
    commit_prepared_manifest(&store.held_path, generation)?;
    store.sync().map_err(map_store_error)?;
    stop_rollback_if_requested(&mut interrupt, ModelRollbackStage::ManifestCommitted)?;
    Ok(activation_receipt(
        ModelActivationDisposition::RolledBack,
        generation,
        Some(previous.profile_id),
        None,
        None,
    ))
}

/// Reconciles interrupted lifecycle residue without activating any artifact.
pub fn recover_model_store(
    store: &Path,
) -> Result<ModelStoreRecoveryReceipt, ModelInstallLifecycleError> {
    let store = open_store(store).map_err(map_store_error)?;
    let _lock = acquire_lock(&store)?;
    let manifest = read_manifest(&store.held_path)?;
    let mut referenced = BTreeSet::new();
    if let Some(manifest) = &manifest {
        referenced.insert(manifest.active.artifact_name.clone());
        if let Some(previous) = &manifest.previous {
            referenced.insert(previous.artifact_name.clone());
        }
    }
    let mut removed_names = Vec::new();
    let mut retained_inactive_names = Vec::new();
    let entries =
        fs::read_dir(&store.held_path).map_err(|_| ModelInstallLifecycleError::Filesystem)?;
    for (index, entry) in entries.enumerate() {
        if index >= MAX_STORE_ENTRIES {
            return Err(ModelInstallLifecycleError::UnsafeState);
        }
        let entry = entry.map_err(|_| ModelInstallLifecycleError::Filesystem)?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| ModelInstallLifecycleError::UnsafeState)?;
        let path = entry.path();
        let metadata =
            fs::symlink_metadata(&path).map_err(|_| ModelInstallLifecycleError::UnsafeState)?;
        if name == ACTIVE_MANIFEST_NAME || name == INSTALLER_LOCK_NAME || referenced.contains(&name)
        {
            continue;
        }
        if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.nlink() != 1 {
            return Err(ModelInstallLifecycleError::UnsafeState);
        }
        if name.starts_with(".active-model-") && name.ends_with(".next")
            || name.starts_with(".staging-")
            || is_unreferenced_installed(&name)
        {
            fs::remove_file(path).map_err(|_| ModelInstallLifecycleError::Filesystem)?;
            removed_names.push(name);
        } else if name.starts_with(".download-")
            || name.starts_with(".verified-")
            || name.starts_with(".quarantine-")
        {
            retained_inactive_names.push(name);
        }
    }
    removed_names.sort();
    retained_inactive_names.sort();
    store.sync().map_err(map_store_error)?;
    Ok(ModelStoreRecoveryReceipt {
        active_profile_id: manifest.map(|value| value.active.profile_id),
        removed_names,
        retained_inactive_names,
        installer_authority_present: false,
    })
}

/// Reads and validates the exact durable selection without changing it.
pub fn read_active_model_manifest(
    store: &Path,
) -> Result<Option<ActiveModelManifest>, ModelInstallLifecycleError> {
    let store = open_store(store).map_err(map_store_error)?;
    read_manifest(&store.held_path)
}

fn validate_staged(
    profile: &ExactModelProfile,
    store: &Path,
    verified_name: &str,
) -> Result<PathBuf, ModelInstallLifecycleError> {
    let import_name = format!(".verified-import-{}.gguf", profile.artifact.sha256);
    let download_name = format!(".verified-download-{}.gguf", profile.artifact.sha256);
    if verified_name != import_name && verified_name != download_name {
        return Err(ModelInstallLifecycleError::StagingMismatch);
    }
    let path = store.join(verified_name);
    if exact_artifact(&path, profile)? {
        Ok(path)
    } else {
        Err(ModelInstallLifecycleError::StagingMismatch)
    }
}

fn exact_artifact(
    path: &Path,
    profile: &ExactModelProfile,
) -> Result<bool, ModelInstallLifecycleError> {
    let file = open_nofollow_read(path)?;
    let metadata = file
        .metadata()
        .map_err(|_| ModelInstallLifecycleError::UnsafeState)?;
    if !metadata.is_file()
        || metadata.nlink() != 1
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o777 != 0o600
        || metadata.len() != profile.artifact.bytes
    {
        return Ok(false);
    }
    let (digest, header) = hash_artifact(file, profile.artifact.bytes)?;
    Ok(header == *b"GGUF" && digest == profile.artifact.sha256)
}

fn valid_record_artifact(
    record: &ActiveModelRecord,
    path: &Path,
) -> Result<bool, ModelInstallLifecycleError> {
    if record.artifact_name != format!("{}.gguf", record.artifact_sha256) {
        return Ok(false);
    }
    let file = open_nofollow_read(path)?;
    let metadata = file
        .metadata()
        .map_err(|_| ModelInstallLifecycleError::UnsafeState)?;
    if !metadata.is_file()
        || metadata.nlink() != 1
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o777 != 0o600
    {
        return Ok(false);
    }
    let (digest, header) = hash_artifact(file, metadata.len())?;
    Ok(header == *b"GGUF" && digest == record.artifact_sha256)
}

fn hash_artifact(
    mut file: File,
    expected_bytes: u64,
) -> Result<(String, [u8; 4]), ModelInstallLifecycleError> {
    let mut digest = Sha256::new();
    let mut header = [0_u8; 4];
    let mut read_bytes = 0_u64;
    let mut buffer = vec![0_u8; 4 * 1024 * 1024];
    while read_bytes < expected_bytes {
        let limit = usize::try_from((expected_bytes - read_bytes).min(buffer.len() as u64))
            .map_err(|_| ModelInstallLifecycleError::Filesystem)?;
        let count = file
            .read(&mut buffer[..limit])
            .map_err(|_| ModelInstallLifecycleError::Filesystem)?;
        if count == 0 {
            return Ok((String::new(), header));
        }
        if read_bytes < 4 {
            let start =
                usize::try_from(read_bytes).map_err(|_| ModelInstallLifecycleError::Filesystem)?;
            let take = (4 - start).min(count);
            header[start..start + take].copy_from_slice(&buffer[..take]);
        }
        digest.update(&buffer[..count]);
        read_bytes += u64::try_from(count).map_err(|_| ModelInstallLifecycleError::Filesystem)?;
    }
    let mut trailing = [0_u8; 1];
    if file
        .read(&mut trailing)
        .map_err(|_| ModelInstallLifecycleError::Filesystem)?
        != 0
    {
        return Ok((String::new(), header));
    }
    Ok((lower_hex(&digest.finalize()), header))
}

fn valid_scan(report: &ModelArtifactScanReport, profile: &ExactModelProfile) -> bool {
    report.profile_id == profile.profile_id
        && report.artifact_sha256 == profile.artifact.sha256
        && report.scanner_policy_sha256.len() == 64
        && report.scanner_policy_sha256.bytes().all(is_lower_hex)
        && report.format_valid
        && !report.executable_payload_detected
        && !report.malware_indicator_detected
        && report.complete
}

fn valid_self_test(report: &ModelInstallSelfTestReport, profile: &ExactModelProfile) -> bool {
    report.profile_id == profile.profile_id
        && report.manifest_sha256 == profile.manifest_sha256
        && report.artifact_sha256 == profile.artifact.sha256
        && report.runtime_sha256 == profile.runtime.runtime_sha256
        && report.loaded
        && report.ready
        && report.unloaded
        && !report.network_available
        && !report.workspace_available
        && !report.session_available
        && !report.unrelated_inference_available
        && !report.tool_available
}

fn reject_staging(
    profile: &ExactModelProfile,
    store: &Path,
    verified: &Path,
    prior: Option<&ActiveModelManifest>,
    reason: &str,
) -> Result<ModelActivationReceipt, ModelInstallLifecycleError> {
    let quarantine_name = format!(
        ".quarantine-activation-{}-{reason}.gguf",
        profile.artifact.sha256
    );
    retain_without_overwrite(verified, &store.join(&quarantine_name)).map_err(map_retain_error)?;
    Ok(activation_receipt(
        ModelActivationDisposition::RejectedPreservedPrevious,
        prior.map_or(0, |manifest| manifest.generation),
        prior.map(|manifest| manifest.active.profile_id.clone()),
        Some(profile.profile_id.clone()),
        Some(quarantine_name),
    ))
}

fn acquire_lock(store: &StoreHandle) -> Result<File, ModelInstallLifecycleError> {
    let descriptor = openat(
        &store.directory,
        INSTALLER_LOCK_NAME,
        OFlags::RDWR | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    )
    .map_err(|_| ModelInstallLifecycleError::UnsafeState)?;
    let lock = File::from(descriptor);
    let metadata = lock
        .metadata()
        .map_err(|_| ModelInstallLifecycleError::UnsafeState)?;
    if !metadata.is_file()
        || metadata.nlink() != 1
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o777 != 0o600
    {
        return Err(ModelInstallLifecycleError::UnsafeState);
    }
    flock(&lock, FlockOperation::NonBlockingLockExclusive)
        .map_err(|_| ModelInstallLifecycleError::Busy)?;
    Ok(lock)
}

fn read_manifest(store: &Path) -> Result<Option<ActiveModelManifest>, ModelInstallLifecycleError> {
    let path = store.join(ACTIVE_MANIFEST_NAME);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(ModelInstallLifecycleError::UnsafeState),
    };
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.nlink() != 1
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o777 != 0o600
        || metadata.len() > MAX_MANIFEST_BYTES
    {
        return Err(ModelInstallLifecycleError::UnsafeState);
    }
    let file = open_nofollow_read(&path)?;
    let mut bytes = Vec::new();
    file.take(MAX_MANIFEST_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ModelInstallLifecycleError::UnsafeState)?;
    if bytes.len() as u64 > MAX_MANIFEST_BYTES {
        return Err(ModelInstallLifecycleError::UnsafeState);
    }
    let manifest: ActiveModelManifest =
        serde_json::from_slice(&bytes).map_err(|_| ModelInstallLifecycleError::UnsafeState)?;
    if manifest.schema_version != 1 || manifest.generation == 0 {
        return Err(ModelInstallLifecycleError::UnsafeState);
    }
    if !valid_record_artifact(
        &manifest.active,
        &store.join(&manifest.active.artifact_name),
    )? || manifest.previous.as_ref().is_some_and(|record| {
        !valid_record_artifact(record, &store.join(&record.artifact_name)).unwrap_or(false)
    }) {
        return Err(ModelInstallLifecycleError::UnsafeState);
    }
    Ok(Some(manifest))
}

fn prepare_manifest(
    store: &Path,
    manifest: &ActiveModelManifest,
) -> Result<(), ModelInstallLifecycleError> {
    let name = format!(".active-model-{}.next", manifest.generation);
    let path = store.join(name);
    let bytes = serde_json::to_vec(manifest).map_err(|_| ModelInstallLifecycleError::Filesystem)?;
    if bytes.len() as u64 > MAX_MANIFEST_BYTES {
        return Err(ModelInstallLifecycleError::UnsafeState);
    }
    let descriptor = open(
        &path,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    )
    .map_err(|_| ModelInstallLifecycleError::Conflict)?;
    let mut file = File::from(descriptor);
    file.write_all(&bytes)
        .map_err(|_| ModelInstallLifecycleError::Filesystem)?;
    file.sync_all()
        .map_err(|_| ModelInstallLifecycleError::Filesystem)
}

fn commit_prepared_manifest(
    store: &Path,
    generation: u64,
) -> Result<(), ModelInstallLifecycleError> {
    let prepared = store.join(format!(".active-model-{generation}.next"));
    let active = store.join(ACTIVE_MANIFEST_NAME);
    fs::rename(prepared, active).map_err(|_| ModelInstallLifecycleError::Filesystem)
}

fn open_nofollow_read(path: &Path) -> Result<File, ModelInstallLifecycleError> {
    let descriptor = open(
        path,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| ModelInstallLifecycleError::UnsafeState)?;
    Ok(File::from(descriptor))
}

fn stop_if_requested(
    interrupt: &mut impl FnMut(ModelActivationStage) -> bool,
    stage: ModelActivationStage,
) -> Result<(), ModelInstallLifecycleError> {
    if interrupt(stage) {
        Err(ModelInstallLifecycleError::Interrupted(stage))
    } else {
        Ok(())
    }
}

fn stop_rollback_if_requested(
    interrupt: &mut impl FnMut(ModelRollbackStage) -> bool,
    stage: ModelRollbackStage,
) -> Result<(), ModelInstallLifecycleError> {
    if interrupt(stage) {
        Err(ModelInstallLifecycleError::RollbackInterrupted(stage))
    } else {
        Ok(())
    }
}

fn activation_receipt(
    disposition: ModelActivationDisposition,
    generation: u64,
    active_profile_id: Option<ModelProfileId>,
    rejected_profile_id: Option<ModelProfileId>,
    quarantine_name: Option<String>,
) -> ModelActivationReceipt {
    ModelActivationReceipt {
        disposition,
        generation,
        active_profile_id,
        rejected_profile_id,
        quarantine_name,
        workspace_available: false,
        session_available: false,
        unrelated_inference_available: false,
        tool_available: false,
        network_available: false,
    }
}

fn is_unreferenced_installed(name: &str) -> bool {
    name.len() == 69 && name.ends_with(".gguf") && name[..64].bytes().all(is_lower_hex)
}

fn is_lower_hex(byte: u8) -> bool {
    byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
}

fn lower_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn map_store_error(_: ModelImportError) -> ModelInstallLifecycleError {
    ModelInstallLifecycleError::UnsafeState
}

fn map_retain_error(error: ModelImportError) -> ModelInstallLifecycleError {
    if error == ModelImportError::DestinationOccupied {
        ModelInstallLifecycleError::Conflict
    } else {
        ModelInstallLifecycleError::Filesystem
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::ExactModelProfile;
    use serde_json::Value;
    use sha2::Digest;

    use super::{
        ModelActivationDisposition, ModelActivationStage, ModelArtifactScanReport,
        ModelInstallLifecycleError, ModelInstallSelfTestReport, ModelInstallVerifier,
        ModelRollbackStage, acquire_lock, activate_verified_model, lower_hex,
        read_active_model_manifest, recover_model_store, rollback_active_model,
        rollback_active_model_with_interruption,
    };
    use crate::model_acquisition::open_store;

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "agentmage-model-lifecycle-{}-{}",
                std::process::id(),
                NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).expect("store");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("mode");
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    struct Verifier {
        scan_passes: bool,
        self_test_passes: bool,
    }

    impl ModelInstallVerifier for Verifier {
        fn scan(
            &mut self,
            _artifact: &Path,
            profile: &ExactModelProfile,
        ) -> Option<ModelArtifactScanReport> {
            Some(ModelArtifactScanReport {
                profile_id: profile.profile_id.clone(),
                artifact_sha256: profile.artifact.sha256.clone(),
                scanner_policy_sha256: "a".repeat(64),
                format_valid: self.scan_passes,
                executable_payload_detected: false,
                malware_indicator_detected: false,
                complete: true,
            })
        }

        fn load_unload(
            &mut self,
            _artifact: &Path,
            profile: &ExactModelProfile,
        ) -> Option<ModelInstallSelfTestReport> {
            Some(ModelInstallSelfTestReport {
                profile_id: profile.profile_id.clone(),
                manifest_sha256: profile.manifest_sha256.clone(),
                artifact_sha256: profile.artifact.sha256.clone(),
                runtime_sha256: profile.runtime.runtime_sha256.clone(),
                loaded: self.self_test_passes,
                ready: self.self_test_passes,
                unloaded: self.self_test_passes,
                network_available: false,
                workspace_available: false,
                session_available: false,
                unrelated_inference_available: false,
                tool_available: false,
            })
        }
    }

    fn profile(bytes: &[u8], suffix: &str) -> ExactModelProfile {
        let catalog: Value = serde_json::from_str(include_str!(
            "../../../model-profiles/exact-profile-catalog.json"
        ))
        .expect("catalog");
        let mut profile: ExactModelProfile =
            serde_json::from_value(catalog["profiles"].as_array().expect("profiles")[0].clone())
                .expect("profile");
        profile.profile_id =
            agentmage_kernel_contracts::ModelProfileId::from_raw(format!("fixture-{suffix}"));
        profile.manifest_sha256 = lower_hex(&sha2::Sha256::digest(suffix.as_bytes()));
        profile.artifact.bytes = bytes.len() as u64;
        profile.artifact.sha256 = lower_hex(&sha2::Sha256::digest(bytes));
        profile
    }

    fn stage_fixture(store: &Path, profile: &ExactModelProfile, bytes: &[u8]) -> String {
        let name = format!(".verified-import-{}.gguf", profile.artifact.sha256);
        fs::write(store.join(&name), bytes).expect("stage");
        fs::set_permissions(store.join(&name), fs::Permissions::from_mode(0o600)).expect("mode");
        name
    }

    #[test]
    fn activation_requires_scan_and_self_test_then_preserves_predecessor_for_rollback() {
        let directory = TestDirectory::new();
        let first_bytes = b"GGUFfirst-model";
        let first = profile(first_bytes, "first");
        let first_name = stage_fixture(&directory.0, &first, first_bytes);
        let receipt = activate_verified_model(
            &first,
            &directory.0,
            &first_name,
            &mut Verifier {
                scan_passes: true,
                self_test_passes: true,
            },
            |_| false,
        )
        .expect("first activation");
        assert_eq!(receipt.disposition, ModelActivationDisposition::Activated);

        let second_bytes = b"GGUFsecond-model";
        let second = profile(second_bytes, "second");
        let second_name = stage_fixture(&directory.0, &second, second_bytes);
        activate_verified_model(
            &second,
            &directory.0,
            &second_name,
            &mut Verifier {
                scan_passes: true,
                self_test_passes: true,
            },
            |_| false,
        )
        .expect("second activation");
        let manifest = read_active_model_manifest(&directory.0)
            .expect("read")
            .expect("manifest");
        assert_eq!(manifest.active.profile_id, second.profile_id);
        assert_eq!(
            manifest.previous.expect("previous").profile_id,
            first.profile_id
        );

        let rollback = rollback_active_model(&directory.0, |_, _| true).expect("rollback");
        assert_eq!(rollback.disposition, ModelActivationDisposition::RolledBack);
        assert_eq!(rollback.active_profile_id, Some(first.profile_id));
    }

    #[test]
    fn failed_scan_quarantines_candidate_and_preserves_active_manifest() {
        let directory = TestDirectory::new();
        let current_bytes = b"GGUFcurrent-model";
        let current = profile(current_bytes, "current");
        let current_name = stage_fixture(&directory.0, &current, current_bytes);
        activate_verified_model(
            &current,
            &directory.0,
            &current_name,
            &mut Verifier {
                scan_passes: true,
                self_test_passes: true,
            },
            |_| false,
        )
        .expect("current");
        let rejected_bytes = b"GGUFrejected-model";
        let rejected = profile(rejected_bytes, "rejected");
        let rejected_name = stage_fixture(&directory.0, &rejected, rejected_bytes);
        let receipt = activate_verified_model(
            &rejected,
            &directory.0,
            &rejected_name,
            &mut Verifier {
                scan_passes: false,
                self_test_passes: true,
            },
            |_| false,
        )
        .expect("rejection receipt");
        assert_eq!(
            receipt.disposition,
            ModelActivationDisposition::RejectedPreservedPrevious
        );
        assert_eq!(receipt.active_profile_id, Some(current.profile_id.clone()));
        assert_eq!(
            read_active_model_manifest(&directory.0)
                .expect("read")
                .expect("manifest")
                .active
                .profile_id,
            current.profile_id
        );
    }

    #[test]
    fn interruption_at_each_stage_recovers_to_prior_or_fully_committed_state() {
        for stage in [
            ModelActivationStage::ArtifactVerified,
            ModelActivationStage::ScanPassed,
            ModelActivationStage::SelfTestPassed,
            ModelActivationStage::ArtifactPromoted,
            ModelActivationStage::ManifestPrepared,
            ModelActivationStage::ManifestCommitted,
        ] {
            let directory = TestDirectory::new();
            let bytes = b"GGUFcrash-fixture";
            let profile = profile(bytes, "crash");
            let name = stage_fixture(&directory.0, &profile, bytes);
            let result = activate_verified_model(
                &profile,
                &directory.0,
                &name,
                &mut Verifier {
                    scan_passes: true,
                    self_test_passes: true,
                },
                |current| current == stage,
            );
            assert!(result.is_err());
            let recovery = recover_model_store(&directory.0).expect("recover");
            let manifest = read_active_model_manifest(&directory.0).expect("read");
            if stage == ModelActivationStage::ManifestCommitted {
                assert_eq!(
                    manifest.expect("committed").active.profile_id,
                    profile.profile_id
                );
            } else {
                assert!(manifest.is_none());
            }
            assert!(!recovery.installer_authority_present);
        }
    }

    #[test]
    fn rollback_interruption_recovers_current_or_verified_predecessor() {
        for stage in [
            ModelRollbackStage::PredecessorVerified,
            ModelRollbackStage::ManifestPrepared,
            ModelRollbackStage::ManifestCommitted,
        ] {
            let directory = TestDirectory::new();
            let first_bytes = b"GGUFrollback-crash-first";
            let first = profile(first_bytes, "rollback-crash-first");
            let first_name = stage_fixture(&directory.0, &first, first_bytes);
            activate_verified_model(
                &first,
                &directory.0,
                &first_name,
                &mut Verifier {
                    scan_passes: true,
                    self_test_passes: true,
                },
                |_| false,
            )
            .expect("first activation");
            let second_bytes = b"GGUFrollback-crash-second";
            let second = profile(second_bytes, "rollback-crash-second");
            let second_name = stage_fixture(&directory.0, &second, second_bytes);
            activate_verified_model(
                &second,
                &directory.0,
                &second_name,
                &mut Verifier {
                    scan_passes: true,
                    self_test_passes: true,
                },
                |_| false,
            )
            .expect("second activation");

            let result = rollback_active_model_with_interruption(
                &directory.0,
                |_, _| true,
                |current| current == stage,
            );
            assert_eq!(
                result.expect_err("interruption"),
                ModelInstallLifecycleError::RollbackInterrupted(stage)
            );
            let recovery = recover_model_store(&directory.0).expect("recover");
            let active = read_active_model_manifest(&directory.0)
                .expect("read")
                .expect("active")
                .active
                .profile_id;
            if stage == ModelRollbackStage::ManifestCommitted {
                assert_eq!(active, first.profile_id);
            } else {
                assert_eq!(active, second.profile_id);
            }
            assert!(!recovery.installer_authority_present);
        }
    }

    #[test]
    fn cleanup_recovery_is_idempotent_after_partial_progress() {
        let directory = TestDirectory::new();
        let installed = format!("{}.gguf", "a".repeat(64));
        for name in [
            ".staging-first.part",
            ".staging-second.part",
            installed.as_str(),
        ] {
            fs::write(directory.0.join(name), b"residue").expect("residue");
            fs::set_permissions(directory.0.join(name), fs::Permissions::from_mode(0o600))
                .expect("mode");
        }
        fs::remove_file(directory.0.join(".staging-first.part")).expect("partial cleanup");

        let first = recover_model_store(&directory.0).expect("resume cleanup");
        assert_eq!(
            first.removed_names,
            [".staging-second.part".to_owned(), installed]
        );
        let second = recover_model_store(&directory.0).expect("repeat cleanup");
        assert!(second.removed_names.is_empty());
        assert!(second.retained_inactive_names.is_empty());
        assert!(!second.installer_authority_present);
    }

    #[test]
    fn symlinked_staging_and_lock_objects_fail_closed() {
        let directory = TestDirectory::new();
        let bytes = b"GGUFsymlink-fixture";
        let profile = profile(bytes, "symlink");
        let outside = directory.0.join("outside.gguf");
        fs::write(&outside, bytes).expect("outside");
        fs::set_permissions(&outside, fs::Permissions::from_mode(0o600)).expect("outside mode");
        let staged_name = format!(".verified-import-{}.gguf", profile.artifact.sha256);
        std::os::unix::fs::symlink(&outside, directory.0.join(&staged_name)).expect("stage link");
        assert_eq!(
            activate_verified_model(
                &profile,
                &directory.0,
                &staged_name,
                &mut Verifier {
                    scan_passes: true,
                    self_test_passes: true,
                },
                |_| false,
            )
            .expect_err("symlink must fail"),
            ModelInstallLifecycleError::UnsafeState
        );
        fs::remove_file(directory.0.join(&staged_name)).expect("remove stage link");
        fs::remove_file(directory.0.join(".installer.lock")).expect("remove prior lock");
        std::os::unix::fs::symlink(&outside, directory.0.join(".installer.lock"))
            .expect("lock link");
        let store = open_store(&directory.0).expect("store");
        assert_eq!(
            acquire_lock(&store).expect_err("lock link must fail"),
            ModelInstallLifecycleError::UnsafeState
        );
    }

    #[test]
    fn installer_lock_is_exclusive() {
        let directory = TestDirectory::new();
        let store = open_store(&directory.0).expect("store");
        let first = acquire_lock(&store).expect("first lock");
        assert_eq!(
            acquire_lock(&store).expect_err("second lock must fail"),
            ModelInstallLifecycleError::Busy
        );
        drop(first);
        acquire_lock(&store).expect("lock after release");
    }
}
