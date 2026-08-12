//! Fail-closed verification of one installed package payload.

use std::fs;
use std::io::Read;
use std::os::fd::OwnedFd;
use std::path::{Component, Path, PathBuf};

use rustix::fs::{FileType, Mode, OFlags, fstat, open, openat};
use rustix::io::dup;
use serde::Deserialize;
use sha2::{Digest, Sha256};

const MANIFEST_PATH: &str = "usr/share/agentmage/package-manifest.json";
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAX_FILE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_FILES: usize = 32;
const REQUIRED_FILES: [&str; 3] = [
    "usr/libexec/agentmage/agentmage-host",
    "usr/share/agentmage/agentmage.vsix",
    "usr/share/licenses/agentmage/LICENSE",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackageClass {
    SignedRelease,
    UnsignedCandidate,
}

impl PackageClass {
    const fn status(self) -> Result<&'static str, PackageVerificationError> {
        match self {
            Self::SignedRelease => Err(PackageVerificationError::ReleaseVerificationUnavailable),
            Self::UnsignedCandidate => Ok("unsigned-candidate"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackageVerificationError {
    RootDenied,
    ManifestDenied,
    ManifestInvalid,
    StatusDenied,
    ReleaseVerificationUnavailable,
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
    files: Vec<PackageFile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PackageFile {
    path: String,
    sha256: String,
    size: u64,
    mode: u32,
}

pub fn verify_package_root(
    root: &std::ffi::OsStr,
    expected: PackageClass,
) -> Result<(), PackageVerificationError> {
    let expected_status = expected.status()?;
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
    validate_manifest(&manifest, expected_status)?;
    for record in &manifest.files {
        verify_file(&root, record)?;
    }
    Ok(())
}

fn validate_manifest(
    manifest: &PackageManifest,
    expected_status: &str,
) -> Result<(), PackageVerificationError> {
    if manifest.schema_version != 1
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

    use serde_json::json;
    use sha2::{Digest, Sha256};

    use super::{PackageClass, PackageVerificationError, verify_package_root};

    static TEMP_ID: AtomicU64 = AtomicU64::new(1);

    #[test]
    fn exact_candidate_payload_verifies_and_mutation_fails_closed() {
        let root = complete_fixture_root("unsigned-candidate");
        let payload = root.join("usr/libexec/agentmage/agentmage-host");
        verify_package_root(root.as_os_str(), PackageClass::UnsignedCandidate)
            .expect("candidate verifies");
        fs::write(&payload, b"changed-host").expect("mutation");
        assert_eq!(
            verify_package_root(root.as_os_str(), PackageClass::UnsignedCandidate),
            Err(PackageVerificationError::FileMismatch)
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn candidate_cannot_satisfy_signed_release_mode() {
        let root = complete_fixture_root("unsigned-candidate");
        assert_eq!(
            verify_package_root(root.as_os_str(), PackageClass::SignedRelease),
            Err(PackageVerificationError::ReleaseVerificationUnavailable)
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn relabeled_manifest_cannot_enable_unimplemented_release_verification() {
        let root = complete_fixture_root("signed-release");
        assert_eq!(
            verify_package_root(root.as_os_str(), PackageClass::SignedRelease),
            Err(PackageVerificationError::ReleaseVerificationUnavailable)
        );
        fs::remove_dir_all(root).expect("cleanup");
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
            verify_package_root(root.as_os_str(), PackageClass::UnsignedCandidate),
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
            verify_package_root(root.as_os_str(), PackageClass::UnsignedCandidate),
            Err(PackageVerificationError::FileDenied)
        );
        fs::remove_dir_all(root).expect("cleanup root");
        fs::remove_dir_all(outside).expect("cleanup outside");
    }

    fn complete_fixture_root(status: &str) -> PathBuf {
        let root = fixture_root();
        let files = [
            (
                "usr/libexec/agentmage/agentmage-host",
                b"host".as_slice(),
                0o755,
            ),
            (
                "usr/share/agentmage/agentmage.vsix",
                b"vsix".as_slice(),
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
        fs::write(
            manifest,
            serde_json::to_vec(&json!({
                "schema_version": 1,
                "record_type": "agentmage-package-manifest",
                "status": status,
                "package_id": "fixture",
                "files": records
            }))
            .expect("manifest bytes"),
        )
        .expect("manifest");
        root
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
        let path = std::env::temp_dir().join(format!(
            "agentmage-package-verifier-{}-{}",
            std::process::id(),
            TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("fixture root");
        path
    }

    use std::path::{Path, PathBuf};
}
