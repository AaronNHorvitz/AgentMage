//! Separately authorized, resumable model-artifact download contract.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;

use agentmage_kernel_contracts::{ExactModelProfile, ModelProfileId};
use sha2::{Digest, Sha256};

use crate::model_acquisition::{
    ModelAcquisitionDisposition, ModelAcquisitionPreflight, ModelImportError, open_store,
    retain_without_overwrite,
};

const COPY_BUFFER_BYTES: usize = 4 * 1024 * 1024;
const MAX_SOURCE_URI_BYTES: usize = 4096;
const MAX_RETRIES: u8 = 3;

/// Immutable remote identity approved for one exact acquisition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelDownloadIdentity {
    /// Exact immutable HTTPS source URI after discovery and before transport.
    pub source_uri: String,
    /// Exact first-party revision embedded in the source identity.
    pub source_revision: String,
    /// Exact response byte count.
    pub bytes: u64,
    /// Strong immutable entity tag, represented as lowercase SHA-256.
    pub etag_sha256: String,
    /// Whether byte-range resume is supported.
    pub accepts_ranges: bool,
}

/// One-use user authorization for a bounded model acquisition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelDownloadAuthorization {
    /// Exact profile approved by the user.
    pub profile_id: ModelProfileId,
    /// Exact profile manifest approved by the user.
    pub manifest_sha256: String,
    /// Exact immutable transport identity approved by the user.
    pub source: ModelDownloadIdentity,
    /// Exact accepted license/use-term digest.
    pub accepted_license_terms_sha256: String,
    /// Content-free one-use network grant identifier.
    pub network_grant_id: String,
    /// Explicit user confirmation for this one operation.
    pub approved: bool,
}

/// A bounded byte source implemented by a separately sandboxed transport.
pub trait BoundedModelDownloadSource {
    /// Returns the exact immutable identity observed by the transport.
    fn identity(&self) -> ModelDownloadIdentity;

    /// Reads bytes beginning at the exact offset without redirecting.
    fn read_at(&mut self, offset: u64, output: &mut [u8]) -> Result<usize, ModelDownloadReadError>;
}

/// Stable transport failure class used for bounded retry decisions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelDownloadReadError {
    /// A transient transport failure may be retried within the fixed budget.
    Retriable,
    /// A permanent transport or identity failure cannot be retried.
    Permanent,
}

/// Terminal disposition of one bounded download attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelDownloadDisposition {
    /// Exact verified bytes are staged but remain non-selectable.
    VerifiedStaged,
    /// A bounded transient failure retained a resumable partial file.
    ResumablePartial,
    /// Explicit cancellation removed every partial byte.
    Cancelled,
    /// A permanent transport failure removed every partial byte.
    FailedCleaned,
    /// Invalid or incompatible bytes were retained under a quarantine name.
    Quarantined,
}

/// Content-free terminal receipt for one bounded download attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelDownloadReceipt {
    /// Exact profile requested by the user.
    pub profile_id: ModelProfileId,
    /// Exact expected artifact digest.
    pub expected_sha256: String,
    /// Observed digest when all expected bytes were read.
    pub observed_sha256: Option<String>,
    /// Existing partial offset used for this attempt.
    pub resumed_from_bytes: u64,
    /// Total bytes retained at the terminal state.
    pub retained_bytes: u64,
    /// Number of retriable transport failures consumed.
    pub retries: u8,
    /// Terminal lifecycle result.
    pub disposition: ModelDownloadDisposition,
    /// Relative retained filename, when present.
    pub retained_name: Option<String>,
    /// Installer had no workspace authority.
    pub workspace_available: bool,
    /// Installer had no session authority.
    pub session_available: bool,
    /// Installer had no inference authority.
    pub inference_available: bool,
    /// Installer had no tool authority.
    pub tool_available: bool,
    /// Installer had only the one-use model-source network grant.
    pub unrelated_network_available: bool,
}

/// Stable fail-closed download error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelDownloadError {
    /// Preflight, approval, license, source, or profile identity did not match.
    AuthorizationMismatch,
    /// Existing staging state is linked, executable, owned by another user, or oversized.
    StagingInvalid,
    /// A retained identity already exists.
    DestinationOccupied,
    /// A permanent source failure occurred.
    SourceUnavailable,
    /// A bounded local filesystem operation failed.
    Filesystem,
}

impl ModelDownloadError {
    /// Returns a stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::AuthorizationMismatch => "model.download.authorization-mismatch",
            Self::StagingInvalid => "model.download.staging-invalid",
            Self::DestinationOccupied => "model.download.destination-occupied",
            Self::SourceUnavailable => "model.download.source-unavailable",
            Self::Filesystem => "model.download.filesystem",
        }
    }
}

impl std::fmt::Display for ModelDownloadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for ModelDownloadError {}

/// Downloads one exact immutable artifact into non-selectable verified staging.
///
/// The transport receives only the exact source identity and one-use network
/// authority. This function cannot activate a model and never accepts redirects,
/// mutable identities, unknown lengths, or unbounded retries.
pub fn download_model_artifact(
    profile: &ExactModelProfile,
    preflight: &ModelAcquisitionPreflight,
    authorization: &ModelDownloadAuthorization,
    source: &mut impl BoundedModelDownloadSource,
    store: &Path,
    mut cancelled: impl FnMut() -> bool,
) -> Result<ModelDownloadReceipt, ModelDownloadError> {
    let initial_identity = source.identity();
    validate_authorization(profile, preflight, authorization, &initial_identity)?;
    let store = open_store(store).map_err(map_store_error)?;
    let part_name = format!(".download-{}.part", profile.artifact.sha256);
    let verified_name = format!(".verified-download-{}.gguf", profile.artifact.sha256);
    let part = store.held_path.join(&part_name);
    let verified = store.held_path.join(&verified_name);
    if verified.exists() {
        return Err(ModelDownloadError::DestinationOccupied);
    }

    let (mut output, resumed_from, mut digest, mut header) =
        open_or_resume(&part, profile.artifact.bytes)?;
    let mut retained = resumed_from;
    let mut retries = 0_u8;
    let mut buffer = vec![0_u8; COPY_BUFFER_BYTES];
    while retained < profile.artifact.bytes {
        if cancelled() {
            drop(output);
            fs::remove_file(&part).map_err(|_| ModelDownloadError::Filesystem)?;
            store.sync().map_err(map_store_error)?;
            return Ok(receipt(
                profile,
                resumed_from,
                0,
                retries,
                ModelDownloadDisposition::Cancelled,
                None,
                None,
            ));
        }
        let remaining = profile.artifact.bytes - retained;
        let limit = usize::try_from(remaining.min(buffer.len() as u64))
            .map_err(|_| ModelDownloadError::Filesystem)?;
        match source.read_at(retained, &mut buffer[..limit]) {
            Ok(0) => {
                output
                    .sync_all()
                    .map_err(|_| ModelDownloadError::Filesystem)?;
                drop(output);
                return quarantine(
                    profile,
                    &store.held_path,
                    &part,
                    resumed_from,
                    retained,
                    retries,
                    None,
                );
            }
            Ok(count) => {
                if count > limit {
                    return Err(ModelDownloadError::SourceUnavailable);
                }
                if retained < 4 {
                    let start =
                        usize::try_from(retained).map_err(|_| ModelDownloadError::Filesystem)?;
                    let take = (4 - start).min(count);
                    header[start..start + take].copy_from_slice(&buffer[..take]);
                }
                output
                    .write_all(&buffer[..count])
                    .map_err(|_| ModelDownloadError::Filesystem)?;
                digest.update(&buffer[..count]);
                retained = retained
                    .checked_add(u64::try_from(count).map_err(|_| ModelDownloadError::Filesystem)?)
                    .ok_or(ModelDownloadError::Filesystem)?;
            }
            Err(ModelDownloadReadError::Retriable) if retries < MAX_RETRIES => {
                retries += 1;
            }
            Err(ModelDownloadReadError::Retriable) => {
                output
                    .sync_all()
                    .map_err(|_| ModelDownloadError::Filesystem)?;
                store.sync().map_err(map_store_error)?;
                return Ok(receipt(
                    profile,
                    resumed_from,
                    retained,
                    retries,
                    ModelDownloadDisposition::ResumablePartial,
                    Some(part_name),
                    None,
                ));
            }
            Err(ModelDownloadReadError::Permanent) => {
                drop(output);
                fs::remove_file(&part).map_err(|_| ModelDownloadError::Filesystem)?;
                store.sync().map_err(map_store_error)?;
                return Ok(receipt(
                    profile,
                    resumed_from,
                    0,
                    retries,
                    ModelDownloadDisposition::FailedCleaned,
                    None,
                    None,
                ));
            }
        }
    }
    output
        .sync_all()
        .map_err(|_| ModelDownloadError::Filesystem)?;
    drop(output);
    if source.identity() != initial_identity {
        return quarantine(
            profile,
            &store.held_path,
            &part,
            resumed_from,
            retained,
            retries,
            None,
        );
    }
    let observed = lower_hex(&digest.finalize());
    if header != *b"GGUF" || observed != profile.artifact.sha256 {
        return quarantine(
            profile,
            &store.held_path,
            &part,
            resumed_from,
            retained,
            retries,
            Some(observed),
        );
    }
    retain_without_overwrite(&part, &verified).map_err(map_retain_error)?;
    store.sync().map_err(map_store_error)?;
    Ok(receipt(
        profile,
        resumed_from,
        retained,
        retries,
        ModelDownloadDisposition::VerifiedStaged,
        Some(verified_name),
        Some(observed),
    ))
}

fn validate_authorization(
    profile: &ExactModelProfile,
    preflight: &ModelAcquisitionPreflight,
    authorization: &ModelDownloadAuthorization,
    observed: &ModelDownloadIdentity,
) -> Result<(), ModelDownloadError> {
    let source = &authorization.source;
    let exact = preflight.disposition == ModelAcquisitionDisposition::Eligible
        && preflight.review.profile_id == profile.profile_id
        && preflight.review.manifest_sha256 == profile.manifest_sha256
        && !preflight.source_opened
        && !preflight.destination_changed
        && authorization.approved
        && authorization.profile_id == profile.profile_id
        && authorization.manifest_sha256 == profile.manifest_sha256
        && authorization.accepted_license_terms_sha256 == profile.license_terms_sha256
        && !authorization.network_grant_id.is_empty()
        && source == observed
        && source.source_uri.starts_with("https://")
        && source.source_uri.len() <= MAX_SOURCE_URI_BYTES
        && !source.source_uri.contains(['#', '\r', '\n'])
        && source.source_revision == profile.artifact.source_revision
        && source.bytes == profile.artifact.bytes
        && source.etag_sha256 == profile.artifact.sha256
        && source.accepts_ranges;
    if exact {
        Ok(())
    } else {
        Err(ModelDownloadError::AuthorizationMismatch)
    }
}

fn open_or_resume(
    part: &Path,
    expected_bytes: u64,
) -> Result<(File, u64, Sha256, [u8; 4]), ModelDownloadError> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).mode(0o600);
    let mut file = options
        .open(part)
        .map_err(|_| ModelDownloadError::Filesystem)?;
    let metadata = file
        .metadata()
        .map_err(|_| ModelDownloadError::StagingInvalid)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.nlink() != 1
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || metadata.permissions().mode() & 0o777 != 0o600
        || metadata.len() > expected_bytes
    {
        return Err(ModelDownloadError::StagingInvalid);
    }
    let resumed = metadata.len();
    let mut digest = Sha256::new();
    let mut header = [0_u8; 4];
    let mut read_bytes = 0_u64;
    let mut buffer = vec![0_u8; COPY_BUFFER_BYTES];
    file.seek(SeekFrom::Start(0))
        .map_err(|_| ModelDownloadError::Filesystem)?;
    while read_bytes < resumed {
        let remaining = resumed - read_bytes;
        let limit = usize::try_from(remaining.min(buffer.len() as u64))
            .map_err(|_| ModelDownloadError::Filesystem)?;
        let count = file
            .read(&mut buffer[..limit])
            .map_err(|_| ModelDownloadError::Filesystem)?;
        if count == 0 {
            return Err(ModelDownloadError::StagingInvalid);
        }
        if read_bytes < 4 {
            let start = usize::try_from(read_bytes).map_err(|_| ModelDownloadError::Filesystem)?;
            let take = (4 - start).min(count);
            header[start..start + take].copy_from_slice(&buffer[..take]);
        }
        digest.update(&buffer[..count]);
        read_bytes += u64::try_from(count).map_err(|_| ModelDownloadError::Filesystem)?;
    }
    file.seek(SeekFrom::End(0))
        .map_err(|_| ModelDownloadError::Filesystem)?;
    Ok((file, resumed, digest, header))
}

fn quarantine(
    profile: &ExactModelProfile,
    store: &Path,
    part: &Path,
    resumed_from: u64,
    retained: u64,
    retries: u8,
    observed: Option<String>,
) -> Result<ModelDownloadReceipt, ModelDownloadError> {
    let suffix = observed.as_deref().unwrap_or("incomplete");
    let name = format!(
        ".quarantine-download-{}-{suffix}.gguf",
        profile.artifact.sha256
    );
    retain_without_overwrite(part, &store.join(&name)).map_err(map_retain_error)?;
    Ok(receipt(
        profile,
        resumed_from,
        retained,
        retries,
        ModelDownloadDisposition::Quarantined,
        Some(name),
        observed,
    ))
}

fn receipt(
    profile: &ExactModelProfile,
    resumed_from_bytes: u64,
    retained_bytes: u64,
    retries: u8,
    disposition: ModelDownloadDisposition,
    retained_name: Option<String>,
    observed_sha256: Option<String>,
) -> ModelDownloadReceipt {
    ModelDownloadReceipt {
        profile_id: profile.profile_id.clone(),
        expected_sha256: profile.artifact.sha256.clone(),
        observed_sha256,
        resumed_from_bytes,
        retained_bytes,
        retries,
        disposition,
        retained_name,
        workspace_available: false,
        session_available: false,
        inference_available: false,
        tool_available: false,
        unrelated_network_available: false,
    }
}

fn lower_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn map_store_error(_: ModelImportError) -> ModelDownloadError {
    ModelDownloadError::Filesystem
}

fn map_retain_error(error: ModelImportError) -> ModelDownloadError {
    if error == ModelImportError::DestinationOccupied {
        ModelDownloadError::DestinationOccupied
    } else {
        ModelDownloadError::Filesystem
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::{ExactModelProfile, PlatformFamily};
    use serde_json::Value;
    use sha2::Digest;

    use super::{
        BoundedModelDownloadSource, ModelDownloadAuthorization, ModelDownloadDisposition,
        ModelDownloadIdentity, ModelDownloadReadError, download_model_artifact, lower_hex,
    };
    use crate::{ModelAcquisitionHost, preflight_model_acquisition};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "agentmage-model-download-{}-{}",
                std::process::id(),
                NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).expect("fixture root");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("private root");
            Self(path)
        }

        fn store(&self) -> PathBuf {
            let store = self.0.join("store");
            fs::create_dir(&store).expect("store");
            fs::set_permissions(&store, fs::Permissions::from_mode(0o700)).expect("store mode");
            store
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    struct MemorySource {
        identity: ModelDownloadIdentity,
        bytes: Vec<u8>,
        transient_failures: u8,
        permanent: bool,
    }

    impl BoundedModelDownloadSource for MemorySource {
        fn identity(&self) -> ModelDownloadIdentity {
            self.identity.clone()
        }

        fn read_at(
            &mut self,
            offset: u64,
            output: &mut [u8],
        ) -> Result<usize, ModelDownloadReadError> {
            if self.permanent {
                return Err(ModelDownloadReadError::Permanent);
            }
            if self.transient_failures > 0 {
                self.transient_failures -= 1;
                return Err(ModelDownloadReadError::Retriable);
            }
            let start = usize::try_from(offset).expect("fixture offset");
            if start >= self.bytes.len() {
                return Ok(0);
            }
            let count = output.len().min(self.bytes.len() - start);
            output[..count].copy_from_slice(&self.bytes[start..start + count]);
            Ok(count)
        }
    }

    fn profile(bytes: &[u8]) -> ExactModelProfile {
        let catalog: Value = serde_json::from_str(include_str!(
            "../../../model-profiles/exact-profile-catalog.json"
        ))
        .expect("catalog");
        let mut profile: ExactModelProfile = serde_json::from_value(
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
        .expect("exact profile");
        profile.artifact.bytes = bytes.len() as u64;
        profile.artifact.sha256 = lower_hex(&sha2::Sha256::digest(bytes));
        profile
    }

    fn preflight(profile: &ExactModelProfile) -> crate::ModelAcquisitionPreflight {
        preflight_model_acquisition(
            profile,
            &ModelAcquisitionHost {
                platform: PlatformFamily::Fedora,
                architecture: profile.runtime.architecture,
                system_memory_bytes: 32 * 1024 * 1024 * 1024,
                accelerator_memory_bytes: 20 * 1024 * 1024 * 1024,
                model_store_available_bytes: 80 * 1024 * 1024 * 1024,
                requested_context_tokens: 8192,
                runtime: Some(profile.runtime.clone()),
            },
        )
    }

    fn source_and_authorization(
        profile: &ExactModelProfile,
        bytes: &[u8],
    ) -> (MemorySource, ModelDownloadAuthorization) {
        let identity = ModelDownloadIdentity {
            source_uri: format!(
                "https://first-party.invalid/resolve/{}/model.gguf",
                profile.artifact.source_revision
            ),
            source_revision: profile.artifact.source_revision.clone(),
            bytes: profile.artifact.bytes,
            etag_sha256: profile.artifact.sha256.clone(),
            accepts_ranges: true,
        };
        (
            MemorySource {
                identity: identity.clone(),
                bytes: bytes.to_vec(),
                transient_failures: 0,
                permanent: false,
            },
            ModelDownloadAuthorization {
                profile_id: profile.profile_id.clone(),
                manifest_sha256: profile.manifest_sha256.clone(),
                source: identity,
                accepted_license_terms_sha256: profile.license_terms_sha256.clone(),
                network_grant_id: "one-use-fixture-grant".to_owned(),
                approved: true,
            },
        )
    }

    #[test]
    fn exact_download_is_verified_staged_without_activation_or_authority() {
        let directory = TestDirectory::new();
        let store = directory.store();
        let bytes = b"GGUFverified-download";
        let profile = profile(bytes);
        let (mut source, authorization) = source_and_authorization(&profile, bytes);
        let receipt = download_model_artifact(
            &profile,
            &preflight(&profile),
            &authorization,
            &mut source,
            &store,
            || false,
        )
        .expect("verified download");
        assert_eq!(
            receipt.disposition,
            ModelDownloadDisposition::VerifiedStaged
        );
        assert!(!receipt.workspace_available);
        assert!(!receipt.session_available);
        assert!(!receipt.inference_available);
        assert!(!receipt.tool_available);
        assert!(!receipt.unrelated_network_available);
        let retained = receipt.retained_name.expect("verified name");
        assert!(retained.starts_with(".verified-download-"));
        assert_eq!(fs::read(store.join(retained)).expect("bytes"), bytes);
    }

    #[test]
    fn exhausted_retry_is_resumable_and_next_attempt_completes() {
        let directory = TestDirectory::new();
        let store = directory.store();
        let bytes = b"GGUFresumable-download";
        let profile = profile(bytes);
        let (mut source, authorization) = source_and_authorization(&profile, bytes);
        source.transient_failures = 4;
        let first = download_model_artifact(
            &profile,
            &preflight(&profile),
            &authorization,
            &mut source,
            &store,
            || false,
        )
        .expect("partial receipt");
        assert_eq!(
            first.disposition,
            ModelDownloadDisposition::ResumablePartial
        );
        let mut resumed = MemorySource {
            transient_failures: 0,
            ..source
        };
        let second = download_model_artifact(
            &profile,
            &preflight(&profile),
            &authorization,
            &mut resumed,
            &store,
            || false,
        )
        .expect("resume");
        assert_eq!(second.disposition, ModelDownloadDisposition::VerifiedStaged);
    }

    #[test]
    fn cancellation_removes_partial_and_bad_bytes_quarantine() {
        let directory = TestDirectory::new();
        let store = directory.store();
        let expected = b"GGUFexpected-download";
        let profile = profile(expected);
        let (mut source, authorization) = source_and_authorization(&profile, expected);
        let cancelled = download_model_artifact(
            &profile,
            &preflight(&profile),
            &authorization,
            &mut source,
            &store,
            || true,
        )
        .expect("cancel");
        assert_eq!(cancelled.disposition, ModelDownloadDisposition::Cancelled);
        assert_eq!(fs::read_dir(&store).expect("empty").count(), 0);

        let (mut corrupt, authorization) = source_and_authorization(&profile, expected);
        corrupt.bytes = b"BAD!wrong-model-bytes".to_vec();
        let quarantined = download_model_artifact(
            &profile,
            &preflight(&profile),
            &authorization,
            &mut corrupt,
            &store,
            || false,
        )
        .expect("quarantine");
        assert_eq!(
            quarantined.disposition,
            ModelDownloadDisposition::Quarantined
        );
        assert!(
            quarantined
                .retained_name
                .as_deref()
                .is_some_and(|name| name.starts_with(".quarantine-download-"))
        );
    }

    #[test]
    fn permanent_transport_failure_removes_partial_and_returns_receipt() {
        let directory = TestDirectory::new();
        let store = directory.store();
        let bytes = b"GGUFpermanent-failure";
        let profile = profile(bytes);
        let (mut source, authorization) = source_and_authorization(&profile, bytes);
        source.permanent = true;
        let receipt = download_model_artifact(
            &profile,
            &preflight(&profile),
            &authorization,
            &mut source,
            &store,
            || false,
        )
        .expect("failure receipt");
        assert_eq!(receipt.disposition, ModelDownloadDisposition::FailedCleaned);
        assert_eq!(receipt.retained_bytes, 0);
        assert_eq!(fs::read_dir(store).expect("empty").count(), 0);
    }

    #[test]
    fn drifted_source_license_or_approval_cannot_create_staging() {
        for mutation in 0..3 {
            let directory = TestDirectory::new();
            let store = directory.store();
            let bytes = b"GGUFauthorization-fixture";
            let profile = profile(bytes);
            let (mut source, mut authorization) = source_and_authorization(&profile, bytes);
            match mutation {
                0 => authorization.approved = false,
                1 => authorization.accepted_license_terms_sha256 = "0".repeat(64),
                _ => source.identity.source_revision = "changed".to_owned(),
            }
            assert!(
                download_model_artifact(
                    &profile,
                    &preflight(&profile),
                    &authorization,
                    &mut source,
                    &store,
                    || false,
                )
                .is_err()
            );
            assert_eq!(fs::read_dir(store).expect("empty").count(), 0);
        }
    }
}
