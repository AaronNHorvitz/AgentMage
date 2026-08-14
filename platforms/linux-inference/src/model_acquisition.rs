//! Non-acquiring model review and machine-fit preflight.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use agentmage_kernel_contracts::{
    ExactModelProfile, ModelLifecycleState, ModelModality, ModelProfileId, ModelRuntimeIdentity,
    PlatformArchitecture, PlatformFamily,
};
use sha2::{Digest, Sha256};

const GIB: u64 = 1024 * 1024 * 1024;
const MAX_ARTIFACT_BYTES: u64 = 64 * GIB;
const INSTALL_RESERVE_BYTES: u64 = GIB;

/// Exact host facts used before any model source is opened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelAcquisitionHost {
    /// Observed platform family.
    pub platform: PlatformFamily,
    /// Observed processor architecture.
    pub architecture: PlatformArchitecture,
    /// Total system memory available to the machine.
    pub system_memory_bytes: u64,
    /// Total compatible accelerator memory.
    pub accelerator_memory_bytes: u64,
    /// Free bytes in the separately selected private model store.
    pub model_store_available_bytes: u64,
    /// Maximum context requested for the first install self-check.
    pub requested_context_tokens: u32,
    /// Exact runtime observed as locally installed and package-verified.
    pub runtime: Option<ModelRuntimeIdentity>,
}

/// Stable reason an exact candidate cannot begin acquisition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModelAcquisitionBlocker {
    /// Candidate profile structure or lifecycle is not installable.
    ProfilePolicy,
    /// Artifact format or byte identity is outside the initial importer contract.
    ArtifactPolicy,
    /// License, publisher, or lineage information is incomplete.
    ProvenancePolicy,
    /// No exact hardware envelope matches the observed platform and architecture.
    Platform,
    /// Required runtime is absent or differs from the exact profile.
    Runtime,
    /// Requested context exceeds the exact profile.
    Context,
    /// System memory is below the matching envelope.
    SystemMemory,
    /// Accelerator memory is below the matching envelope.
    AcceleratorMemory,
    /// Private model-store capacity cannot retain staging plus active bytes.
    Disk,
}

impl ModelAcquisitionBlocker {
    /// Returns the stable content-free blocker code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ProfilePolicy => "model.install.profile-policy",
            Self::ArtifactPolicy => "model.install.artifact-policy",
            Self::ProvenancePolicy => "model.install.provenance-policy",
            Self::Platform => "model.install.platform",
            Self::Runtime => "model.install.runtime",
            Self::Context => "model.install.context",
            Self::SystemMemory => "model.install.system-memory",
            Self::AcceleratorMemory => "model.install.accelerator-memory",
            Self::Disk => "model.install.disk",
        }
    }
}

/// Exact non-acquiring preflight disposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelAcquisitionDisposition {
    /// Every policy and machine-fit prerequisite passed.
    Eligible,
    /// Candidate identity, provenance, artifact, runtime, or context policy blocked.
    Blocked,
    /// Candidate is policy-eligible but this machine lacks a required resource.
    BlockedHardware,
}

/// Review record rendered before a source or destination can be selected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelAcquisitionReview {
    /// Exact candidate profile.
    pub profile_id: ModelProfileId,
    /// Exact profile manifest digest.
    pub manifest_sha256: String,
    /// Display-only exact profile label.
    pub display_name: String,
    /// Publisher controlling the selected artifact.
    pub publisher: String,
    /// Publisher-control jurisdiction or policy label.
    pub publisher_control: String,
    /// Ordered source-to-artifact lineage.
    pub lineage: Vec<String>,
    /// Exact SPDX expression.
    pub license_spdx: String,
    /// Exact license and use-term digest.
    pub license_terms_sha256: String,
    /// Exact immutable source revision.
    pub source_revision: String,
    /// Exact artifact format.
    pub artifact_format: String,
    /// Exact artifact byte count.
    pub artifact_bytes: u64,
    /// Exact artifact digest.
    pub artifact_sha256: String,
    /// Exact quantization label.
    pub quantization: String,
    /// Exact required runtime.
    pub runtime: ModelRuntimeIdentity,
    /// Maximum admitted context for this profile.
    pub maximum_context_tokens: u32,
    /// Minimum system memory from the matching hardware envelope, when present.
    pub minimum_system_memory_bytes: Option<u64>,
    /// Minimum accelerator memory from the matching envelope, when present.
    pub minimum_accelerator_memory_bytes: Option<u64>,
    /// Required private-store capacity for staging and active copies plus reserve.
    pub minimum_model_store_bytes: u64,
}

/// Complete non-acquiring result with ordered reasons and review material.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelAcquisitionPreflight {
    /// Exact review material displayed before acquisition.
    pub review: ModelAcquisitionReview,
    /// Truthful current disposition.
    pub disposition: ModelAcquisitionDisposition,
    /// Stable blockers in policy-first order.
    pub blockers: Vec<ModelAcquisitionBlocker>,
    /// No source was opened by this operation.
    pub source_opened: bool,
    /// No destination was changed by this operation.
    pub destination_changed: bool,
}

/// Terminal result of one local import attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelImportDisposition {
    /// Exact verified bytes are staged but remain non-selectable.
    VerifiedStaged,
    /// Cancellation removed all staging bytes before activation.
    Cancelled,
    /// Invalid bytes were retained under a non-selectable quarantine name.
    Quarantined,
}

/// Content-free terminal receipt for one local import attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelImportReceipt {
    /// Exact profile requested by the user.
    pub profile_id: ModelProfileId,
    /// Exact expected artifact digest.
    pub expected_sha256: String,
    /// Observed digest when the complete expected byte count was read.
    pub observed_sha256: Option<String>,
    /// Number of source bytes copied before the terminal result.
    pub copied_bytes: u64,
    /// Terminal lifecycle result.
    pub disposition: ModelImportDisposition,
    /// Relative activated or quarantine filename, when retained.
    pub retained_name: Option<String>,
    /// Whether pre/post source identity remained unchanged.
    pub source_unchanged: bool,
    /// Importer had no workspace authority.
    pub workspace_available: bool,
    /// Importer had no session authority.
    pub session_available: bool,
    /// Importer had no inference authority.
    pub inference_available: bool,
    /// Importer had no tool authority.
    pub tool_available: bool,
}

/// Stable fail-closed local-import error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelImportError {
    /// Preflight did not exactly authorize this profile.
    PreflightMismatch,
    /// Source is linked, executable, non-regular, missing, or changed.
    SourceInvalid,
    /// Private model-store identity or permissions are invalid.
    StoreInvalid,
    /// A staging, active, or quarantine identity already exists.
    DestinationOccupied,
    /// A bounded read, write, synchronization, link, or cleanup step failed.
    Filesystem,
}

impl ModelImportError {
    /// Returns a stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::PreflightMismatch => "model.import.preflight-mismatch",
            Self::SourceInvalid => "model.import.source-invalid",
            Self::StoreInvalid => "model.import.store-invalid",
            Self::DestinationOccupied => "model.import.destination-occupied",
            Self::Filesystem => "model.import.filesystem",
        }
    }
}

impl std::fmt::Display for ModelImportError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for ModelImportError {}

/// Builds an exact review and evaluates machine fit without acquiring bytes.
#[must_use]
pub fn preflight_model_acquisition(
    profile: &ExactModelProfile,
    host: &ModelAcquisitionHost,
) -> ModelAcquisitionPreflight {
    let minimum_model_store_bytes = profile
        .artifact
        .bytes
        .saturating_mul(2)
        .saturating_add(INSTALL_RESERVE_BYTES);
    let envelope = profile
        .hardware
        .iter()
        .find(|item| item.platform == host.platform && item.architecture == host.architecture);
    let review = ModelAcquisitionReview {
        profile_id: profile.profile_id.clone(),
        manifest_sha256: profile.manifest_sha256.clone(),
        display_name: profile.display_name.clone(),
        publisher: profile.artifact.publisher.clone(),
        publisher_control: profile.publisher_control.clone(),
        lineage: profile.lineage.clone(),
        license_spdx: profile.license_spdx.clone(),
        license_terms_sha256: profile.license_terms_sha256.clone(),
        source_revision: profile.artifact.source_revision.clone(),
        artifact_format: profile.artifact.format.clone(),
        artifact_bytes: profile.artifact.bytes,
        artifact_sha256: profile.artifact.sha256.clone(),
        quantization: profile.quantization.clone(),
        runtime: profile.runtime.clone(),
        maximum_context_tokens: profile.context.max_context_tokens,
        minimum_system_memory_bytes: envelope.map(|item| item.minimum_system_memory_bytes),
        minimum_accelerator_memory_bytes: envelope
            .map(|item| item.minimum_accelerator_memory_bytes),
        minimum_model_store_bytes,
    };
    let mut blockers = Vec::new();
    if profile.schema_version != 2
        || profile.lifecycle != ModelLifecycleState::Candidate
        || profile.enabled
        || profile.automatic_fallback
        || profile.modalities != [ModelModality::Text]
        || !valid_sha256(&profile.manifest_sha256)
    {
        blockers.push(ModelAcquisitionBlocker::ProfilePolicy);
    }
    if profile.artifact.format != "GGUF"
        || profile.artifact.bytes == 0
        || profile.artifact.bytes > MAX_ARTIFACT_BYTES
        || !valid_sha256(&profile.artifact.sha256)
    {
        blockers.push(ModelAcquisitionBlocker::ArtifactPolicy);
    }
    if profile.artifact.publisher.is_empty()
        || profile.publisher_control.is_empty()
        || profile.lineage.is_empty()
        || profile.license_spdx.is_empty()
        || !valid_sha256(&profile.license_terms_sha256)
        || profile.artifact.source_revision.is_empty()
    {
        blockers.push(ModelAcquisitionBlocker::ProvenancePolicy);
    }
    if envelope.is_none() {
        blockers.push(ModelAcquisitionBlocker::Platform);
    }
    if host.runtime.as_ref() != Some(&profile.runtime) {
        blockers.push(ModelAcquisitionBlocker::Runtime);
    }
    if host.requested_context_tokens == 0
        || host.requested_context_tokens > profile.context.max_context_tokens
    {
        blockers.push(ModelAcquisitionBlocker::Context);
    }
    if envelope.is_some_and(|item| host.system_memory_bytes < item.minimum_system_memory_bytes) {
        blockers.push(ModelAcquisitionBlocker::SystemMemory);
    }
    if envelope
        .is_some_and(|item| host.accelerator_memory_bytes < item.minimum_accelerator_memory_bytes)
    {
        blockers.push(ModelAcquisitionBlocker::AcceleratorMemory);
    }
    if host.model_store_available_bytes < minimum_model_store_bytes {
        blockers.push(ModelAcquisitionBlocker::Disk);
    }
    let policy_blocked = blockers.iter().any(|blocker| {
        matches!(
            blocker,
            ModelAcquisitionBlocker::ProfilePolicy
                | ModelAcquisitionBlocker::ArtifactPolicy
                | ModelAcquisitionBlocker::ProvenancePolicy
                | ModelAcquisitionBlocker::Platform
                | ModelAcquisitionBlocker::Runtime
                | ModelAcquisitionBlocker::Context
        )
    });
    let disposition = if blockers.is_empty() {
        ModelAcquisitionDisposition::Eligible
    } else if policy_blocked {
        ModelAcquisitionDisposition::Blocked
    } else {
        ModelAcquisitionDisposition::BlockedHardware
    };
    ModelAcquisitionPreflight {
        review,
        disposition,
        blockers,
        source_opened: false,
        destination_changed: false,
    }
}

/// Imports one user-selected local GGUF after exact non-acquiring preflight.
///
/// The source is opened read-only. The destination root must already be an
/// owner-only directory. Activation never replaces an existing identity.
pub fn import_local_model(
    profile: &ExactModelProfile,
    preflight: &ModelAcquisitionPreflight,
    source: &Path,
    store: &Path,
    mut cancelled: impl FnMut() -> bool,
) -> Result<ModelImportReceipt, ModelImportError> {
    if preflight.disposition != ModelAcquisitionDisposition::Eligible
        || preflight.review.profile_id != profile.profile_id
        || preflight.review.manifest_sha256 != profile.manifest_sha256
        || preflight.review.artifact_bytes != profile.artifact.bytes
        || preflight.review.artifact_sha256 != profile.artifact.sha256
        || preflight.source_opened
        || preflight.destination_changed
    {
        return Err(ModelImportError::PreflightMismatch);
    }
    let store = open_store(store)?;
    let before = source_identity(source)?;
    let mut input = File::open(source).map_err(|_| ModelImportError::SourceInvalid)?;
    if source_identity_from_metadata(
        &input
            .metadata()
            .map_err(|_| ModelImportError::SourceInvalid)?,
    )? != before
    {
        return Err(ModelImportError::SourceInvalid);
    }
    let staging_name = format!(".staging-{}.part", profile.artifact.sha256);
    let verified_name = format!(".verified-import-{}.gguf", profile.artifact.sha256);
    let staging = store.held_path.join(&staging_name);
    let verified = store.held_path.join(&verified_name);
    if staging.exists() || verified.exists() {
        return Err(ModelImportError::DestinationOccupied);
    }
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&staging)
        .map_err(|_| ModelImportError::DestinationOccupied)?;
    let mut guard = StagingGuard::new(staging.clone());
    let mut digest = Sha256::new();
    let mut copied = 0_u64;
    let mut header = [0_u8; 4];
    let mut header_count = 0_usize;
    let mut buffer = vec![0_u8; 4 * 1024 * 1024];
    while copied < profile.artifact.bytes {
        if cancelled() {
            drop(output);
            guard.remove()?;
            return Ok(receipt(
                profile,
                None,
                copied,
                ModelImportDisposition::Cancelled,
                None,
                source_identity(source).ok().as_ref() == Some(&before),
            ));
        }
        let remaining = profile.artifact.bytes - copied;
        let limit = usize::try_from(remaining.min(buffer.len() as u64))
            .map_err(|_| ModelImportError::Filesystem)?;
        let count = input
            .read(&mut buffer[..limit])
            .map_err(|_| ModelImportError::Filesystem)?;
        if count == 0 {
            break;
        }
        if header_count < header.len() {
            let take = (header.len() - header_count).min(count);
            header[header_count..header_count + take].copy_from_slice(&buffer[..take]);
            header_count += take;
        }
        output
            .write_all(&buffer[..count])
            .map_err(|_| ModelImportError::Filesystem)?;
        digest.update(&buffer[..count]);
        copied = copied
            .checked_add(u64::try_from(count).map_err(|_| ModelImportError::Filesystem)?)
            .ok_or(ModelImportError::Filesystem)?;
    }
    let mut trailing = [0_u8; 1];
    let oversized = input
        .read(&mut trailing)
        .map_err(|_| ModelImportError::Filesystem)?
        != 0;
    output
        .sync_all()
        .map_err(|_| ModelImportError::Filesystem)?;
    drop(output);
    let after = source_identity(source).map_err(|_| ModelImportError::SourceInvalid)?;
    let source_unchanged = before == after;
    if !source_unchanged {
        return Err(ModelImportError::SourceInvalid);
    }
    let observed = lowercase_hex(&digest.finalize());
    let exact = copied == profile.artifact.bytes
        && !oversized
        && header_count == header.len()
        && &header == b"GGUF"
        && observed == profile.artifact.sha256;
    if !exact {
        let quarantine_name = format!(".quarantine-{}-{observed}.gguf", profile.artifact.sha256);
        let quarantine = store.held_path.join(&quarantine_name);
        retain_without_overwrite(&staging, &quarantine)?;
        guard.disarm();
        store.sync()?;
        return Ok(receipt(
            profile,
            Some(observed),
            copied,
            ModelImportDisposition::Quarantined,
            Some(quarantine_name),
            true,
        ));
    }
    retain_without_overwrite(&staging, &verified)?;
    guard.disarm();
    store.sync()?;
    Ok(receipt(
        profile,
        Some(observed),
        copied,
        ModelImportDisposition::VerifiedStaged,
        Some(verified_name),
        true,
    ))
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SourceIdentity {
    device: u64,
    inode: u64,
    bytes: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

fn source_identity(path: &Path) -> Result<SourceIdentity, ModelImportError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| ModelImportError::SourceInvalid)?;
    source_identity_from_metadata(&metadata)
}

fn source_identity_from_metadata(
    metadata: &fs::Metadata,
) -> Result<SourceIdentity, ModelImportError> {
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.nlink() != 1
        || metadata.permissions().mode() & 0o111 != 0
    {
        return Err(ModelImportError::SourceInvalid);
    }
    Ok(SourceIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        bytes: metadata.len(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    })
}

pub(super) struct StoreHandle {
    pub(super) directory: File,
    pub(super) held_path: PathBuf,
}

impl StoreHandle {
    pub(super) fn sync(&self) -> Result<(), ModelImportError> {
        self.directory
            .sync_all()
            .map_err(|_| ModelImportError::Filesystem)
    }
}

pub(super) fn open_store(path: &Path) -> Result<StoreHandle, ModelImportError> {
    let before = fs::symlink_metadata(path).map_err(|_| ModelImportError::StoreInvalid)?;
    if !valid_store_metadata(&before) {
        return Err(ModelImportError::StoreInvalid);
    }
    let directory = File::open(path).map_err(|_| ModelImportError::StoreInvalid)?;
    let opened = directory
        .metadata()
        .map_err(|_| ModelImportError::StoreInvalid)?;
    if !valid_store_metadata(&opened)
        || opened.dev() != before.dev()
        || opened.ino() != before.ino()
    {
        return Err(ModelImportError::StoreInvalid);
    }
    let held_path = PathBuf::from(format!("/proc/self/fd/{}", directory.as_raw_fd()));
    Ok(StoreHandle {
        directory,
        held_path,
    })
}

fn valid_store_metadata(metadata: &fs::Metadata) -> bool {
    metadata.is_dir()
        && !metadata.file_type().is_symlink()
        && metadata.permissions().mode() & 0o777 == 0o700
        && metadata.uid() == rustix::process::geteuid().as_raw()
}

pub(super) fn retain_without_overwrite(
    staging: &Path,
    destination: &Path,
) -> Result<(), ModelImportError> {
    fs::hard_link(staging, destination).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            ModelImportError::DestinationOccupied
        } else {
            ModelImportError::Filesystem
        }
    })?;
    fs::remove_file(staging).map_err(|_| ModelImportError::Filesystem)
}

fn receipt(
    profile: &ExactModelProfile,
    observed_sha256: Option<String>,
    copied_bytes: u64,
    disposition: ModelImportDisposition,
    retained_name: Option<String>,
    source_unchanged: bool,
) -> ModelImportReceipt {
    ModelImportReceipt {
        profile_id: profile.profile_id.clone(),
        expected_sha256: profile.artifact.sha256.clone(),
        observed_sha256,
        copied_bytes,
        disposition,
        retained_name,
        source_unchanged,
        workspace_available: false,
        session_available: false,
        inference_available: false,
        tool_available: false,
    }
}

struct StagingGuard {
    path: PathBuf,
    armed: bool,
}

impl StagingGuard {
    const fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn remove(&mut self) -> Result<(), ModelImportError> {
        fs::remove_file(&self.path).map_err(|_| ModelImportError::Filesystem)?;
        self.armed = false;
        Ok(())
    }

    const fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for StagingGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn lowercase_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::{ExactModelProfile, ModelLifecycleState, PlatformFamily};
    use serde_json::Value;
    use sha2::Digest;

    use super::{
        GIB, ModelAcquisitionBlocker, ModelAcquisitionDisposition, ModelAcquisitionHost,
        ModelImportDisposition, import_local_model, lowercase_hex, preflight_model_acquisition,
    };

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "agentmage-model-import-{}-{}",
                std::process::id(),
                NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).expect("create fixture root");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("private root");
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn profile() -> ExactModelProfile {
        let catalog: Value = serde_json::from_str(include_str!(
            "../../../model-profiles/exact-profile-catalog.json"
        ))
        .expect("catalog JSON");
        serde_json::from_value(
            catalog["profiles"]
                .as_array()
                .expect("profiles")
                .iter()
                .find(|value| {
                    value["profile_id"]
                        == "muse-glimmer-30b-q4-k-m-text-8k-fedora-diagnostic-repeatability"
                })
                .expect("profile")
                .clone(),
        )
        .expect("exact profile")
    }

    fn host(profile: &ExactModelProfile) -> ModelAcquisitionHost {
        ModelAcquisitionHost {
            platform: PlatformFamily::Fedora,
            architecture: profile.runtime.architecture,
            system_memory_bytes: 32 * GIB,
            accelerator_memory_bytes: 20 * GIB,
            model_store_available_bytes: profile.artifact.bytes * 2 + GIB,
            requested_context_tokens: 8192,
            runtime: Some(profile.runtime.clone()),
        }
    }

    fn import_profile(bytes: &[u8]) -> ExactModelProfile {
        let mut profile = profile();
        profile.artifact.bytes = bytes.len() as u64;
        profile.artifact.sha256 = lowercase_hex(&sha2::Sha256::digest(bytes));
        profile
    }

    fn write_source(root: &Path, bytes: &[u8]) -> PathBuf {
        let path = root.join("selected.gguf");
        fs::write(&path, bytes).expect("source fixture");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).expect("source mode");
        path
    }

    #[test]
    fn exact_boundary_is_eligible_and_review_is_complete_without_io() {
        let profile = profile();
        let result = preflight_model_acquisition(&profile, &host(&profile));
        assert_eq!(result.disposition, ModelAcquisitionDisposition::Eligible);
        assert!(result.blockers.is_empty());
        assert!(!result.source_opened);
        assert!(!result.destination_changed);
        assert_eq!(result.review.profile_id, profile.profile_id);
        assert_eq!(result.review.lineage, profile.lineage);
        assert_eq!(result.review.artifact_sha256, profile.artifact.sha256);
        assert_eq!(result.review.runtime, profile.runtime);
    }

    #[test]
    fn each_hardware_boundary_below_at_and_above_is_exact() {
        let profile = profile();
        let exact = host(&profile);
        for (blocker, mutate) in [
            (
                ModelAcquisitionBlocker::SystemMemory,
                (|value: &mut ModelAcquisitionHost| value.system_memory_bytes -= 1)
                    as fn(&mut ModelAcquisitionHost),
            ),
            (
                ModelAcquisitionBlocker::AcceleratorMemory,
                (|value: &mut ModelAcquisitionHost| value.accelerator_memory_bytes -= 1)
                    as fn(&mut ModelAcquisitionHost),
            ),
            (
                ModelAcquisitionBlocker::Disk,
                (|value: &mut ModelAcquisitionHost| value.model_store_available_bytes -= 1)
                    as fn(&mut ModelAcquisitionHost),
            ),
        ] {
            let mut below = exact.clone();
            mutate(&mut below);
            let result = preflight_model_acquisition(&profile, &below);
            assert_eq!(
                result.disposition,
                ModelAcquisitionDisposition::BlockedHardware
            );
            assert_eq!(result.blockers, [blocker]);
        }
        let at = preflight_model_acquisition(&profile, &exact);
        assert_eq!(at.disposition, ModelAcquisitionDisposition::Eligible);
        let mut above = exact;
        above.system_memory_bytes += 1;
        above.accelerator_memory_bytes += 1;
        above.model_store_available_bytes += 1;
        assert_eq!(
            preflight_model_acquisition(&profile, &above).disposition,
            ModelAcquisitionDisposition::Eligible
        );
    }

    #[test]
    fn platform_runtime_context_profile_and_artifact_drift_block_before_io() {
        let profile = profile();
        let mut changed_host = host(&profile);
        changed_host.platform = PlatformFamily::Ubuntu;
        changed_host.runtime = None;
        changed_host.requested_context_tokens = 8193;
        let result = preflight_model_acquisition(&profile, &changed_host);
        assert_eq!(result.disposition, ModelAcquisitionDisposition::Blocked);
        assert_eq!(
            result.blockers,
            [
                ModelAcquisitionBlocker::Platform,
                ModelAcquisitionBlocker::Runtime,
                ModelAcquisitionBlocker::Context,
            ]
        );
        assert!(!result.source_opened);
        assert!(!result.destination_changed);

        let mut changed_profile = profile;
        changed_profile.lifecycle = ModelLifecycleState::Evaluating;
        changed_profile.artifact.format = "executable".to_owned();
        changed_profile.license_terms_sha256 = "invalid".to_owned();
        let result = preflight_model_acquisition(&changed_profile, &host(&changed_profile));
        assert_eq!(result.disposition, ModelAcquisitionDisposition::Blocked);
        assert_eq!(
            &result.blockers[..3],
            [
                ModelAcquisitionBlocker::ProfilePolicy,
                ModelAcquisitionBlocker::ArtifactPolicy,
                ModelAcquisitionBlocker::ProvenancePolicy,
            ]
        );
    }

    #[test]
    fn exact_local_import_stages_without_activation_source_mutation_or_authority() {
        let directory = TestDirectory::new();
        let bytes = b"GGUFexact-model-fixture";
        let source = write_source(&directory.0, bytes);
        let store = directory.0.join("store");
        fs::create_dir(&store).expect("store");
        fs::set_permissions(&store, fs::Permissions::from_mode(0o700)).expect("store mode");
        let profile = import_profile(bytes);
        let preflight = preflight_model_acquisition(&profile, &host(&profile));
        let before = fs::read(&source).expect("source preimage");
        let receipt = import_local_model(&profile, &preflight, &source, &store, || false)
            .expect("exact import");
        assert_eq!(receipt.disposition, ModelImportDisposition::VerifiedStaged);
        assert!(receipt.source_unchanged);
        assert!(!receipt.workspace_available);
        assert!(!receipt.session_available);
        assert!(!receipt.inference_available);
        assert!(!receipt.tool_available);
        assert_eq!(fs::read(&source).expect("source after"), before);
        let verified = store.join(receipt.retained_name.expect("verified name"));
        assert_eq!(fs::read(verified).expect("verified bytes"), bytes);
        assert_eq!(fs::read_dir(store).expect("store entries").count(), 1);
    }

    #[test]
    fn corrupt_and_oversized_inputs_quarantine_without_activation() {
        for source_bytes in [
            b"BAD!exact-model-fixture".to_vec(),
            b"GGUFexact-model-fixture-extra".to_vec(),
        ] {
            let directory = TestDirectory::new();
            let expected = b"GGUFexact-model-fixture";
            let source = write_source(&directory.0, &source_bytes);
            let store = directory.0.join("store");
            fs::create_dir(&store).expect("store");
            fs::set_permissions(&store, fs::Permissions::from_mode(0o700)).expect("store mode");
            let profile = import_profile(expected);
            let preflight = preflight_model_acquisition(&profile, &host(&profile));
            let receipt = import_local_model(&profile, &preflight, &source, &store, || false)
                .expect("quarantine result");
            assert_eq!(receipt.disposition, ModelImportDisposition::Quarantined);
            assert!(
                receipt
                    .retained_name
                    .as_deref()
                    .is_some_and(|name| name.starts_with(".quarantine-"))
            );
            assert_eq!(fs::read_dir(store).expect("store entries").count(), 1);
        }
    }

    #[test]
    fn cancellation_removes_staging_and_link_or_mode_attacks_fail_closed() {
        let directory = TestDirectory::new();
        let bytes = b"GGUFexact-model-fixture";
        let source = write_source(&directory.0, bytes);
        let store = directory.0.join("store");
        fs::create_dir(&store).expect("store");
        fs::set_permissions(&store, fs::Permissions::from_mode(0o700)).expect("store mode");
        let profile = import_profile(bytes);
        let preflight = preflight_model_acquisition(&profile, &host(&profile));
        let receipt = import_local_model(&profile, &preflight, &source, &store, || true)
            .expect("cancelled import");
        assert_eq!(receipt.disposition, ModelImportDisposition::Cancelled);
        assert_eq!(fs::read_dir(&store).expect("empty store").count(), 0);

        fs::set_permissions(&source, fs::Permissions::from_mode(0o700)).expect("executable source");
        assert!(import_local_model(&profile, &preflight, &source, &store, || false).is_err());
        fs::set_permissions(&source, fs::Permissions::from_mode(0o600)).expect("source mode");
        let linked = directory.0.join("linked.gguf");
        fs::hard_link(&source, &linked).expect("hard link attack");
        assert!(import_local_model(&profile, &preflight, &source, &store, || false).is_err());
        assert_eq!(fs::read_dir(store).expect("empty store").count(), 0);
    }
}
