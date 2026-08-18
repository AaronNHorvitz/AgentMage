//! Fail-closed verification of one installed package payload.

use std::fs;
use std::io::Read;
use std::os::fd::OwnedFd;
use std::path::{Component, Path, PathBuf};

use ed25519_dalek::{Signature, VerifyingKey};
use rustix::fs::{FileType, Mode, OFlags, fstat, open, openat};
use rustix::io::dup;
use serde::Deserialize;
use sha2::{Digest, Sha256};

const MANIFEST_PATH: &str = "usr/share/agentmage/package-manifest.json";
const RELEASE_SIGNATURE_DOMAIN: &[u8] = b"agentmage.package-manifest.v2\0";
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAX_FILE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_FILES: usize = 32;
const REQUIRED_FILES: [&str; 9] = [
    "usr/libexec/agentmage/agentmage-docker-guard",
    "usr/libexec/agentmage/agentmage-docker-topology-collector",
    "usr/libexec/agentmage/agentmage-host",
    "usr/libexec/agentmage/agentmage-model-installer",
    "usr/libexec/agentmage/agentmage-native-inference",
    "usr/libexec/agentmage/agentmage-read-only-worker",
    "usr/share/agentmage/agentmage.vsix",
    "usr/share/agentmage/model-profiles/exact-profile-catalog.json",
    "usr/share/licenses/agentmage/LICENSE",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackageVerificationError {
    RootDenied,
    ManifestDenied,
    ManifestInvalid,
    StatusDenied,
    ReleaseVerificationUnavailable,
    SignatureDenied,
    SignatureInvalid,
    TrustRootDenied,
    TrustRootInvalid,
    FileDenied,
    FileMismatch,
}

impl PackageVerificationError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::RootDenied => "agentmage.package.root_denied",
            Self::ManifestDenied => "agentmage.package.manifest_denied",
            Self::ManifestInvalid => "agentmage.package.manifest_invalid",
            Self::StatusDenied => "agentmage.package.status_denied",
            Self::ReleaseVerificationUnavailable => {
                "agentmage.package.release_verification_unavailable"
            }
            Self::SignatureDenied => "agentmage.package.signature_denied",
            Self::SignatureInvalid => "agentmage.package.signature_invalid",
            Self::TrustRootDenied => "agentmage.package.trust_root_denied",
            Self::TrustRootInvalid => "agentmage.package.trust_root_invalid",
            Self::FileDenied => "agentmage.package.file_denied",
            Self::FileMismatch => "agentmage.package.file_mismatch",
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PackageManifest {
    schema_version: u16,
    record_type: String,
    status: String,
    package_id: String,
    release_sequence: Option<u64>,
    files: Vec<PackageFile>,
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PackageFile {
    path: String,
    sha256: String,
    size: u64,
    mode: u32,
}

/// Descriptor-held proof that one exact signed package root and every declared payload verified.
pub(crate) struct VerifiedPackageRoot {
    root: OwnedFd,
    files: Vec<PackageFile>,
    manifest_sha256: String,
}

impl VerifiedPackageRoot {
    /// Re-reads one manifest-bound payload beneath the continuously held package root.
    pub(crate) fn read_verified_file(
        &self,
        relative: &str,
        limit: u64,
    ) -> Result<Vec<u8>, PackageVerificationError> {
        let record = self
            .files
            .iter()
            .find(|record| record.path == relative)
            .ok_or(PackageVerificationError::FileDenied)?;
        if record.size > limit {
            return Err(PackageVerificationError::FileDenied);
        }
        let path =
            normalized_relative_path(relative).ok_or(PackageVerificationError::ManifestInvalid)?;
        let (bytes, metadata) = read_bounded_beneath_with_metadata(&self.root, &path, limit)
            .map_err(|_| PackageVerificationError::FileDenied)?;
        if metadata.st_size < 0
            || metadata.st_size as u64 != record.size
            || metadata.st_mode & 0o777 != record.mode
            || metadata.st_nlink != 1
            || lowercase_hex(&Sha256::digest(&bytes)) != record.sha256
        {
            return Err(PackageVerificationError::FileMismatch);
        }
        Ok(bytes)
    }

    /// Digest of the exact signed package manifest bytes.
    pub(crate) fn manifest_sha256(&self) -> &str {
        &self.manifest_sha256
    }
}

pub fn verify_package_candidate_root(
    root: &std::ffi::OsStr,
) -> Result<(), PackageVerificationError> {
    let root = open(
        Path::new(root),
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| PackageVerificationError::RootDenied)?;
    let manifest_bytes = read_bounded_beneath(&root, Path::new(MANIFEST_PATH), MAX_MANIFEST_BYTES)
        .map_err(|_| PackageVerificationError::ManifestDenied)?;
    let manifest: PackageManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|_| PackageVerificationError::ManifestInvalid)?;
    validate_manifest(&manifest, "unsigned-candidate")?;
    for record in &manifest.files {
        verify_file(&root, record)?;
    }
    Ok(())
}

pub fn verify_signed_package_root(
    root: &std::ffi::OsStr,
    signature_path: &std::ffi::OsStr,
    public_key_path: &std::ffi::OsStr,
) -> Result<(), PackageVerificationError> {
    verify_signed_package_root_with_evidence(root, signature_path, public_key_path).map(|_| ())
}

/// Verifies a signed release and retains the descriptor-held proof for trusted composition.
pub(crate) fn verify_signed_package_root_with_evidence(
    root: &std::ffi::OsStr,
    signature_path: &std::ffi::OsStr,
    public_key_path: &std::ffi::OsStr,
) -> Result<VerifiedPackageRoot, PackageVerificationError> {
    let root = open(
        Path::new(root),
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| PackageVerificationError::RootDenied)?;
    let manifest_bytes = read_bounded_beneath(&root, Path::new(MANIFEST_PATH), MAX_MANIFEST_BYTES)
        .map_err(|_| PackageVerificationError::ManifestDenied)?;
    let signature = read_exact_regular(Path::new(signature_path), 64)
        .map_err(|_| PackageVerificationError::SignatureDenied)?;
    let public_key = read_exact_regular(Path::new(public_key_path), 32)
        .map_err(|_| PackageVerificationError::TrustRootDenied)?;
    let signature_bytes: [u8; 64] = signature
        .try_into()
        .map_err(|_| PackageVerificationError::SignatureInvalid)?;
    let public_key_bytes: [u8; 32] = public_key
        .try_into()
        .map_err(|_| PackageVerificationError::TrustRootInvalid)?;
    let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)
        .map_err(|_| PackageVerificationError::TrustRootInvalid)?;
    let mut signed = Vec::with_capacity(RELEASE_SIGNATURE_DOMAIN.len() + manifest_bytes.len());
    signed.extend_from_slice(RELEASE_SIGNATURE_DOMAIN);
    signed.extend_from_slice(&manifest_bytes);
    verifying_key
        .verify_strict(&signed, &Signature::from_bytes(&signature_bytes))
        .map_err(|_| PackageVerificationError::SignatureInvalid)?;
    let manifest: PackageManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|_| PackageVerificationError::ManifestInvalid)?;
    validate_manifest(&manifest, "signed-release")?;
    for record in &manifest.files {
        verify_file(&root, record)?;
    }
    Ok(VerifiedPackageRoot {
        root,
        files: manifest.files,
        manifest_sha256: lowercase_hex(&Sha256::digest(&manifest_bytes)),
    })
}

fn validate_manifest(
    manifest: &PackageManifest,
    expected_status: &str,
) -> Result<(), PackageVerificationError> {
    let valid_class = match expected_status {
        "unsigned-candidate" => manifest.schema_version == 1 && manifest.release_sequence.is_none(),
        "signed-release" => {
            manifest.schema_version == 2 && manifest.release_sequence.is_some_and(|value| value > 0)
        }
        _ => false,
    };
    if !valid_class
        || manifest.record_type != "agentmage-package-manifest"
        || manifest.package_id.is_empty()
        || manifest.package_id.len() > 128
        || manifest.files.is_empty()
        || manifest.files.len() > MAX_FILES
    {
        return Err(PackageVerificationError::ManifestInvalid);
    }
    if manifest.status != expected_status {
        return Err(PackageVerificationError::StatusDenied);
    }
    let mut paths: Vec<&str> = manifest
        .files
        .iter()
        .map(|record| record.path.as_str())
        .collect();
    let original = paths.clone();
    paths.sort_unstable();
    paths.dedup();
    if paths != original || paths != REQUIRED_FILES {
        return Err(PackageVerificationError::ManifestInvalid);
    }
    Ok(())
}

fn read_exact_regular(path: &Path, expected: usize) -> Result<Vec<u8>, std::io::Error> {
    let descriptor = open(
        path,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(std::io::Error::other)?;
    let before = fstat(&descriptor).map_err(std::io::Error::other)?;
    if FileType::from_raw_mode(before.st_mode) != FileType::RegularFile
        || before.st_size != expected as i64
        || before.st_nlink != 1
    {
        return Err(std::io::Error::other(
            "exact single-link regular file required",
        ));
    }
    let mut bytes = Vec::with_capacity(expected);
    let file = fs::File::from(descriptor);
    (&file).take(expected as u64 + 1).read_to_end(&mut bytes)?;
    let after = fstat(&file).map_err(std::io::Error::other)?;
    if bytes.len() != expected
        || before.st_dev != after.st_dev
        || before.st_ino != after.st_ino
        || before.st_mode != after.st_mode
        || before.st_size != after.st_size
        || before.st_nlink != after.st_nlink
    {
        return Err(std::io::Error::other(
            "verification input changed during read",
        ));
    }
    Ok(bytes)
}

fn verify_file(root: &OwnedFd, record: &PackageFile) -> Result<(), PackageVerificationError> {
    let relative =
        normalized_relative_path(&record.path).ok_or(PackageVerificationError::ManifestInvalid)?;
    if record.size > MAX_FILE_BYTES || !valid_sha256(&record.sha256) || record.mode & !0o777 != 0 {
        return Err(PackageVerificationError::ManifestInvalid);
    }
    let (bytes, metadata) = read_bounded_beneath_with_metadata(root, &relative, MAX_FILE_BYTES)
        .map_err(|_| PackageVerificationError::FileDenied)?;
    if metadata.st_size < 0
        || metadata.st_size as u64 != record.size
        || metadata.st_mode & 0o777 != record.mode
        || metadata.st_nlink != 1
    {
        return Err(PackageVerificationError::FileMismatch);
    }
    let actual = lowercase_hex(&Sha256::digest(bytes));
    if actual != record.sha256 {
        return Err(PackageVerificationError::FileMismatch);
    }
    Ok(())
}

fn normalized_relative_path(value: &str) -> Option<PathBuf> {
    let path = Path::new(value);
    if value.is_empty() || path.is_absolute() {
        return None;
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => normalized.push(value),
            _ => return None,
        }
    }
    (normalized.as_os_str() == path.as_os_str()).then_some(normalized)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
}

fn read_bounded_beneath(
    root: &OwnedFd,
    path: &Path,
    limit: u64,
) -> Result<Vec<u8>, std::io::Error> {
    read_bounded_beneath_with_metadata(root, path, limit).map(|(bytes, _)| bytes)
}

fn read_bounded_beneath_with_metadata(
    root: &OwnedFd,
    path: &Path,
    limit: u64,
) -> Result<(Vec<u8>, rustix::fs::Stat), std::io::Error> {
    let normalized = normalized_relative_path(path.to_str().unwrap_or_default())
        .ok_or_else(|| std::io::Error::other("normalized relative path required"))?;
    let components: Vec<_> = normalized.components().collect();
    let (last, parents) = components
        .split_last()
        .ok_or_else(|| std::io::Error::other("nonempty path required"))?;
    let mut directory = dup(root).map_err(std::io::Error::other)?;
    for component in parents {
        let Component::Normal(name) = component else {
            return Err(std::io::Error::other("normal path component required"));
        };
        directory = openat(
            &directory,
            *name,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(std::io::Error::other)?;
    }
    let Component::Normal(name) = last else {
        return Err(std::io::Error::other("normal final component required"));
    };
    let descriptor = openat(
        &directory,
        *name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(std::io::Error::other)?;
    let before = fstat(&descriptor).map_err(std::io::Error::other)?;
    if FileType::from_raw_mode(before.st_mode) != FileType::RegularFile
        || before.st_size < 0
        || before.st_size as u64 > limit
    {
        return Err(std::io::Error::other("bounded regular file required"));
    }
    let mut bytes = Vec::with_capacity(before.st_size as usize);
    let file = fs::File::from(descriptor);
    (&file).take(limit + 1).read_to_end(&mut bytes)?;
    let after = fstat(&file).map_err(std::io::Error::other)?;
    if bytes.len() as i64 != before.st_size
        || before.st_dev != after.st_dev
        || before.st_ino != after.st_ino
        || before.st_mode != after.st_mode
        || before.st_size != after.st_size
        || before.st_nlink != after.st_nlink
    {
        return Err(std::io::Error::other("file changed during read"));
    }
    Ok((bytes, after))
}

fn lowercase_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(DIGITS[usize::from(byte >> 4)]));
        encoded.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    encoded
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};

    use ed25519_dalek::{Signer, SigningKey};
    use serde_json::{Value, json};
    use sha2::{Digest, Sha256};

    use super::{
        PackageVerificationError, RELEASE_SIGNATURE_DOMAIN, verify_package_candidate_root,
        verify_signed_package_root,
    };

    static TEMP_ID: AtomicU64 = AtomicU64::new(1);

    #[test]
    fn exact_candidate_payload_verifies_and_mutation_fails_closed() {
        let root = complete_fixture_root("unsigned-candidate");
        let payload = root.join("usr/libexec/agentmage/agentmage-host");
        verify_package_candidate_root(root.as_os_str()).expect("candidate verifies");
        fs::write(&payload, b"changed-host").expect("mutation");
        assert_eq!(
            verify_package_candidate_root(root.as_os_str()),
            Err(PackageVerificationError::FileMismatch)
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn candidate_cannot_satisfy_signed_release_mode_even_with_a_valid_signature() {
        let root = complete_fixture_root("unsigned-candidate");
        let (signature, public_key) = sign_fixture(&root, [6; 32]);
        assert_eq!(
            verify_signed_package_root(
                root.as_os_str(),
                signature.as_os_str(),
                public_key.as_os_str(),
            ),
            Err(PackageVerificationError::ManifestInvalid)
        );
        fs::remove_dir_all(root).expect("cleanup");
        fs::remove_file(signature).expect("cleanup signature");
        fs::remove_file(public_key).expect("cleanup key");
    }

    #[test]
    fn exact_signed_release_verifies_with_external_trust_root() {
        let root = complete_fixture_root("signed-release");
        let (signature, public_key) = sign_fixture(&root, [7; 32]);
        verify_signed_package_root(
            root.as_os_str(),
            signature.as_os_str(),
            public_key.as_os_str(),
        )
        .expect("signed release verifies");
        fs::remove_dir_all(root).expect("cleanup root");
        fs::remove_file(signature).expect("cleanup signature");
        fs::remove_file(public_key).expect("cleanup key");
    }

    #[test]
    fn signed_release_proof_loads_the_exact_management_only_model_catalog() {
        let root = complete_fixture_root("signed-release");
        let (signature, public_key) = sign_fixture(&root, [17; 32]);
        let package = super::verify_signed_package_root_with_evidence(
            root.as_os_str(),
            signature.as_os_str(),
            public_key.as_os_str(),
        )
        .expect("signed package proof");
        let snapshot =
            crate::model_catalog_bootstrap::load_signed_model_picker_snapshot(&package, 1_000)
                .expect("signed catalog projection");
        assert!(!snapshot.entries.is_empty());
        assert!(snapshot.catalog_signature_verified);
        assert!(snapshot.entries.iter().all(|entry| {
            entry.disposition == agentmage_kernel_contracts::ModelPickerDisposition::ManagementOnly
        }));
        fs::write(
            root.join("usr/share/agentmage/model-profiles/exact-profile-catalog.json"),
            b"{}",
        )
        .expect("mutate catalog after package verification");
        assert_eq!(
            crate::model_catalog_bootstrap::load_signed_model_picker_snapshot(&package, 1_001),
            Err(crate::model_catalog_bootstrap::ModelCatalogBootstrapError::PackageBinding)
        );
        fs::remove_dir_all(root).expect("cleanup root");
        fs::remove_file(signature).expect("cleanup signature");
        fs::remove_file(public_key).expect("cleanup key");
    }

    #[test]
    fn signed_release_rejects_manifest_mutation_and_wrong_trust_root() {
        let root = complete_fixture_root("signed-release");
        let (signature, public_key) = sign_fixture(&root, [8; 32]);
        let wrong_key = fixture_path("wrong-key");
        fs::write(
            &wrong_key,
            SigningKey::from_bytes(&[9; 32]).verifying_key().as_bytes(),
        )
        .expect("wrong key");
        assert_eq!(
            verify_signed_package_root(
                root.as_os_str(),
                signature.as_os_str(),
                wrong_key.as_os_str(),
            ),
            Err(PackageVerificationError::SignatureInvalid)
        );
        let manifest = root.join(super::MANIFEST_PATH);
        let mut value: Value =
            serde_json::from_slice(&fs::read(&manifest).expect("manifest")).expect("manifest json");
        value["release_sequence"] = json!(2);
        fs::write(
            &manifest,
            serde_json::to_vec(&value).expect("mutated manifest"),
        )
        .expect("write mutation");
        assert_eq!(
            verify_signed_package_root(
                root.as_os_str(),
                signature.as_os_str(),
                public_key.as_os_str(),
            ),
            Err(PackageVerificationError::SignatureInvalid)
        );
        fs::remove_dir_all(root).expect("cleanup root");
        fs::remove_file(signature).expect("cleanup signature");
        fs::remove_file(public_key).expect("cleanup key");
        fs::remove_file(wrong_key).expect("cleanup wrong key");
    }

    #[test]
    fn relabeled_v1_manifest_cannot_enable_release_verification() {
        let root = complete_fixture_root("unsigned-candidate");
        let manifest = root.join(super::MANIFEST_PATH);
        let mut value: Value =
            serde_json::from_slice(&fs::read(&manifest).expect("manifest")).expect("manifest json");
        value["status"] = json!("signed-release");
        fs::write(
            &manifest,
            serde_json::to_vec(&value).expect("relabeled manifest"),
        )
        .expect("write relabel");
        let (signature, public_key) = sign_fixture(&root, [5; 32]);
        assert_eq!(
            verify_signed_package_root(
                root.as_os_str(),
                signature.as_os_str(),
                public_key.as_os_str(),
            ),
            Err(PackageVerificationError::ManifestInvalid)
        );
        fs::remove_dir_all(root).expect("cleanup");
        fs::remove_file(signature).expect("cleanup signature");
        fs::remove_file(public_key).expect("cleanup key");
    }

    #[test]
    fn candidate_manifest_must_bind_the_complete_payload_set() {
        let root = fixture_root();
        let payload = root.join("usr/libexec/agentmage/agentmage-host");
        fs::create_dir_all(payload.parent().expect("payload parent")).expect("payload parent");
        fs::write(&payload, b"candidate-host").expect("payload");
        fs::set_permissions(&payload, fs::Permissions::from_mode(0o755)).expect("mode");
        write_manifest(&root, "unsigned-candidate", &payload);
        assert_eq!(
            verify_package_candidate_root(root.as_os_str()),
            Err(PackageVerificationError::ManifestInvalid)
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn candidate_manifest_cannot_omit_the_model_installer_boundary() {
        let root = complete_fixture_root("unsigned-candidate");
        let manifest = root.join(super::MANIFEST_PATH);
        let mut value: Value =
            serde_json::from_slice(&fs::read(&manifest).expect("manifest")).expect("manifest json");
        value["files"]
            .as_array_mut()
            .expect("files")
            .retain(|record| record["path"] != "usr/libexec/agentmage/agentmage-model-installer");
        fs::write(
            &manifest,
            serde_json::to_vec(&value).expect("mutated manifest"),
        )
        .expect("manifest mutation");
        assert_eq!(
            verify_package_candidate_root(root.as_os_str()),
            Err(PackageVerificationError::ManifestInvalid)
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn candidate_manifest_cannot_omit_the_read_only_worker_boundary() {
        let root = complete_fixture_root("unsigned-candidate");
        let manifest = root.join(super::MANIFEST_PATH);
        let mut value: Value =
            serde_json::from_slice(&fs::read(&manifest).expect("manifest")).expect("manifest json");
        value["files"]
            .as_array_mut()
            .expect("files")
            .retain(|record| record["path"] != "usr/libexec/agentmage/agentmage-read-only-worker");
        fs::write(
            &manifest,
            serde_json::to_vec(&value).expect("mutated manifest"),
        )
        .expect("manifest mutation");
        assert_eq!(
            verify_package_candidate_root(root.as_os_str()),
            Err(PackageVerificationError::ManifestInvalid)
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn symlinked_payload_parent_fails_closed() {
        let root = complete_fixture_root("unsigned-candidate");
        let outside = fixture_root();
        let outside_payload = outside.join("agentmage-host");
        fs::write(&outside_payload, b"candidate-host").expect("outside payload");
        fs::set_permissions(&outside_payload, fs::Permissions::from_mode(0o755)).expect("mode");
        let libexec = root.join("usr/libexec");
        fs::remove_dir_all(libexec.join("agentmage")).expect("remove package directory");
        std::os::unix::fs::symlink(&outside, libexec.join("agentmage")).expect("parent symlink");
        assert_eq!(
            verify_package_candidate_root(root.as_os_str()),
            Err(PackageVerificationError::FileDenied)
        );
        fs::remove_dir_all(root).expect("cleanup root");
        fs::remove_dir_all(outside).expect("cleanup outside");
    }

    fn complete_fixture_root(status: &str) -> PathBuf {
        let root = fixture_root();
        let files = [
            (
                "usr/libexec/agentmage/agentmage-docker-guard",
                b"docker-guard".as_slice(),
                0o755,
            ),
            (
                "usr/libexec/agentmage/agentmage-docker-topology-collector",
                b"docker-collector".as_slice(),
                0o755,
            ),
            (
                "usr/libexec/agentmage/agentmage-host",
                b"host".as_slice(),
                0o755,
            ),
            (
                "usr/libexec/agentmage/agentmage-model-installer",
                b"model-installer".as_slice(),
                0o755,
            ),
            (
                "usr/libexec/agentmage/agentmage-native-inference",
                b"inference-adapter".as_slice(),
                0o755,
            ),
            (
                "usr/libexec/agentmage/agentmage-read-only-worker",
                b"read-only-worker".as_slice(),
                0o755,
            ),
            (
                "usr/share/agentmage/agentmage.vsix",
                b"vsix".as_slice(),
                0o644,
            ),
            (
                "usr/share/agentmage/model-profiles/exact-profile-catalog.json",
                include_bytes!("../../../model-profiles/exact-profile-catalog.json").as_slice(),
                0o644,
            ),
            (
                "usr/share/licenses/agentmage/LICENSE",
                b"license".as_slice(),
                0o644,
            ),
        ];
        let records: Vec<_> = files
            .iter()
            .map(|(relative, bytes, mode)| {
                let path = root.join(relative);
                fs::create_dir_all(path.parent().expect("parent")).expect("parent");
                fs::write(&path, bytes).expect("payload");
                fs::set_permissions(&path, fs::Permissions::from_mode(*mode)).expect("mode");
                json!({
                    "path": relative,
                    "sha256": super::lowercase_hex(&Sha256::digest(bytes)),
                    "size": bytes.len(),
                    "mode": mode
                })
            })
            .collect();
        let manifest = root.join(super::MANIFEST_PATH);
        fs::create_dir_all(manifest.parent().expect("manifest parent")).expect("manifest parent");
        let mut value = json!({
            "schema_version": if status == "signed-release" { 2 } else { 1 },
            "record_type": "agentmage-package-manifest",
            "status": status,
            "package_id": "fixture",
            "files": records
        });
        if status == "signed-release" {
            value["release_sequence"] = json!(1);
        }
        fs::write(
            manifest,
            serde_json::to_vec(&value).expect("manifest bytes"),
        )
        .expect("manifest");
        root
    }

    fn sign_fixture(root: &Path, seed: [u8; 32]) -> (PathBuf, PathBuf) {
        let signing_key = SigningKey::from_bytes(&seed);
        let manifest = fs::read(root.join(super::MANIFEST_PATH)).expect("manifest");
        let mut signed = RELEASE_SIGNATURE_DOMAIN.to_vec();
        signed.extend_from_slice(&manifest);
        let signature = fixture_path("signature");
        let public_key = fixture_path("public-key");
        fs::write(&signature, signing_key.sign(&signed).to_bytes()).expect("signature");
        fs::write(&public_key, signing_key.verifying_key().as_bytes()).expect("public key");
        (signature, public_key)
    }

    fn write_manifest(root: &Path, status: &str, payload: &Path) {
        let manifest = root.join("usr/share/agentmage/package-manifest.json");
        fs::create_dir_all(manifest.parent().expect("manifest parent")).expect("manifest parent");
        let bytes = fs::read(payload).expect("payload bytes");
        let value = json!({
            "schema_version": 1,
            "record_type": "agentmage-package-manifest",
            "status": status,
            "package_id": "fixture",
            "files": [{
                "path": "usr/libexec/agentmage/agentmage-host",
                "sha256": super::lowercase_hex(&Sha256::digest(&bytes)),
                "size": bytes.len(),
                "mode": 493
            }]
        });
        fs::write(
            manifest,
            serde_json::to_vec(&value).expect("manifest bytes"),
        )
        .expect("manifest");
    }

    fn fixture_root() -> PathBuf {
        let path = fixture_path("root");
        fs::create_dir(&path).expect("fixture root");
        path
    }

    fn fixture_path(kind: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "agentmage-package-verifier-{kind}-{}-{}",
            std::process::id(),
            TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ))
    }

    use std::path::{Path, PathBuf};
}
