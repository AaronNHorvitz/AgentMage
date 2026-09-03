//! Protected installed-file ownership for approved memory bundles and portable exports.

use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Component, Path, PathBuf};

use agentmage_capability_knowledge::{MemoryMarkdownBundle, MemoryMarkdownFile};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const STATE_FILE: &str = ".agentmage-memory-state.json";
const BACKUP_DIR: &str = ".agentmage-memory-last-good";
const MAX_FILES: usize = 100_001;
const MAX_FILE_BYTES: usize = 1024 * 1024;
const MAX_EXPORT_BYTES: usize = 256 * 1024 * 1024;

type ProjectionFiles = Vec<(String, Vec<u8>)>;

/// Closed installed-memory publication result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryFileDisposition {
    /// The exact approved bundle replaced the governed installed projection.
    Committed,
    /// Concurrent state was preserved and the proposal was retained separately.
    ConflictPreserved,
    /// A corrupt or interrupted installed projection was restored from last-good bytes.
    Restored,
}

/// Content-free receipt for an installed-memory operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryFileReceipt {
    /// Closed operation outcome.
    pub disposition: MemoryFileDisposition,
    /// Digest of the exact installed or preserved bundle.
    pub bundle_sha256: String,
    /// Exact governed file count.
    pub file_count: u64,
    /// Fixed false network marker.
    pub network_used: bool,
    /// Fixed false automatic-decision marker.
    pub automatic_decision: bool,
}

/// Content-free protected-memory failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryFileError {
    /// Root, path, identifier, digest, bundle, or export input was invalid.
    InvalidInput,
    /// A symbolic link or non-regular governed object was observed.
    UnsafeFilesystem,
    /// Installed bytes did not match their committed manifest or last-good copy.
    CorruptState,
    /// A bounded filesystem operation failed.
    Filesystem,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InstalledState {
    schema_version: u16,
    bundle_sha256: String,
    files: Vec<InstalledFile>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InstalledFile {
    relative_path: String,
    content_sha256: String,
    byte_count: u64,
}

/// Filesystem owner rooted at one explicitly selected local memory directory.
pub struct MemoryFileRuntime {
    root: PathBuf,
}

impl MemoryFileRuntime {
    /// Admits an existing, non-symbolic-link directory as the complete memory-file root.
    pub fn open(root: PathBuf) -> Result<Self, MemoryFileError> {
        let metadata = fs::symlink_metadata(&root).map_err(|_| MemoryFileError::Filesystem)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(MemoryFileError::UnsafeFilesystem);
        }
        Ok(Self { root })
    }

    /// Publishes one exact approved bundle or preserves a concurrent proposal as a conflict.
    pub fn apply_bundle(
        &self,
        bundle: &MemoryMarkdownBundle,
        expected_bundle_sha256: Option<&str>,
        transaction_id: &str,
    ) -> Result<MemoryFileReceipt, MemoryFileError> {
        validate_identifier(transaction_id)?;
        let (state, files) = state_for_bundle(bundle)?;
        let current = self.read_state_optional()?;
        if expected_bundle_sha256 != current.as_ref().map(|value| value.bundle_sha256.as_str()) {
            let conflict_root = self.root.join("Conflicts").join(transaction_id);
            fs::create_dir_all(self.root.join("Conflicts"))
                .map_err(|_| MemoryFileError::Filesystem)?;
            self.write_new_projection(&conflict_root, &files, &state)?;
            return Ok(receipt(MemoryFileDisposition::ConflictPreserved, &state));
        }

        let stage = self
            .root
            .join(format!(".agentmage-memory-stage-{transaction_id}"));
        self.write_new_projection(&stage, &files, &state)?;
        if let Some(current) = &current {
            self.snapshot_last_good(current)?;
        }
        self.install_staged(&stage, current.as_ref(), &state)?;
        fs::remove_dir_all(&stage).map_err(|_| MemoryFileError::Filesystem)?;
        Ok(receipt(MemoryFileDisposition::Committed, &state))
    }

    /// Verifies installed bytes and restores the last complete projection after corruption.
    pub fn recover(&self) -> Result<MemoryFileReceipt, MemoryFileError> {
        let current = self.read_state().ok().flatten();
        if let Some(state) = &current
            && self.verify_projection(&self.root, state).is_ok()
        {
            return Ok(receipt(MemoryFileDisposition::Restored, state));
        }
        let backup = self.root.join(BACKUP_DIR);
        let backup_state = read_state_at(&backup)?.ok_or(MemoryFileError::CorruptState)?;
        self.verify_projection(&backup, &backup_state)?;
        self.install_from_projection(&backup, current.as_ref(), &backup_state)?;
        Ok(receipt(MemoryFileDisposition::Restored, &backup_state))
    }

    /// Writes exact authenticated portable-export bytes with private mode on Unix.
    pub fn write_portable_export(
        &self,
        relative_name: &str,
        bytes: &[u8],
        expected_sha256: &str,
    ) -> Result<MemoryFileReceipt, MemoryFileError> {
        validate_single_name(relative_name)?;
        if bytes.is_empty() || bytes.len() > MAX_EXPORT_BYTES || sha256(bytes) != expected_sha256 {
            return Err(MemoryFileError::InvalidInput);
        }
        let target = self.root.join(relative_name);
        write_new_file(&target, bytes)?;
        Ok(MemoryFileReceipt {
            disposition: MemoryFileDisposition::Committed,
            bundle_sha256: expected_sha256.to_owned(),
            file_count: 1,
            network_used: false,
            automatic_decision: false,
        })
    }

    /// Reads exact portable-export bytes only when their expected digest matches.
    pub fn read_portable_export(
        &self,
        relative_name: &str,
        expected_sha256: &str,
    ) -> Result<Vec<u8>, MemoryFileError> {
        validate_single_name(relative_name)?;
        if !valid_sha256(expected_sha256) {
            return Err(MemoryFileError::InvalidInput);
        }
        let path = self.root.join(relative_name);
        ensure_regular(&path)?;
        let bytes = fs::read(path).map_err(|_| MemoryFileError::Filesystem)?;
        if bytes.is_empty() || bytes.len() > MAX_EXPORT_BYTES || sha256(&bytes) != expected_sha256 {
            return Err(MemoryFileError::CorruptState);
        }
        Ok(bytes)
    }

    fn read_state_optional(&self) -> Result<Option<InstalledState>, MemoryFileError> {
        read_state_at(&self.root)
    }

    fn read_state(&self) -> Result<Option<InstalledState>, MemoryFileError> {
        self.read_state_optional()
    }

    fn verify_projection(
        &self,
        root: &Path,
        state: &InstalledState,
    ) -> Result<(), MemoryFileError> {
        validate_state(state)?;
        for file in &state.files {
            let path = joined_safe(root, &file.relative_path)?;
            ensure_regular(&path)?;
            let bytes = fs::read(path).map_err(|_| MemoryFileError::Filesystem)?;
            if bytes.len() as u64 != file.byte_count || sha256(&bytes) != file.content_sha256 {
                return Err(MemoryFileError::CorruptState);
            }
        }
        Ok(())
    }

    fn write_new_projection(
        &self,
        root: &Path,
        files: &[(String, Vec<u8>)],
        state: &InstalledState,
    ) -> Result<(), MemoryFileError> {
        fs::create_dir(root).map_err(|_| MemoryFileError::Filesystem)?;
        for (relative, bytes) in files {
            let target = joined_safe(root, relative)?;
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|_| MemoryFileError::Filesystem)?;
            }
            write_new_file(&target, bytes)?;
        }
        let state_bytes = serde_json::to_vec(state).map_err(|_| MemoryFileError::InvalidInput)?;
        write_new_file(&root.join(STATE_FILE), &state_bytes)
    }

    fn snapshot_last_good(&self, current: &InstalledState) -> Result<(), MemoryFileError> {
        self.verify_projection(&self.root, current)?;
        let backup = self.root.join(BACKUP_DIR);
        if backup.exists() {
            fs::remove_dir_all(&backup).map_err(|_| MemoryFileError::Filesystem)?;
        }
        fs::create_dir(&backup).map_err(|_| MemoryFileError::Filesystem)?;
        for file in &current.files {
            let source = joined_safe(&self.root, &file.relative_path)?;
            let target = joined_safe(&backup, &file.relative_path)?;
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|_| MemoryFileError::Filesystem)?;
            }
            fs::copy(source, target).map_err(|_| MemoryFileError::Filesystem)?;
        }
        fs::copy(self.root.join(STATE_FILE), backup.join(STATE_FILE))
            .map_err(|_| MemoryFileError::Filesystem)?;
        Ok(())
    }

    fn install_staged(
        &self,
        stage: &Path,
        current: Option<&InstalledState>,
        next: &InstalledState,
    ) -> Result<(), MemoryFileError> {
        self.install_from_projection(stage, current, next)
    }

    fn install_from_projection(
        &self,
        source_root: &Path,
        current: Option<&InstalledState>,
        next: &InstalledState,
    ) -> Result<(), MemoryFileError> {
        let next_paths = next
            .files
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect::<BTreeSet<_>>();
        for file in &next.files {
            let source = joined_safe(source_root, &file.relative_path)?;
            ensure_regular(&source)?;
            let target = joined_safe(&self.root, &file.relative_path)?;
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|_| MemoryFileError::Filesystem)?;
            }
            let temporary = target.with_extension("agentmage-new");
            if temporary.exists() {
                fs::remove_file(&temporary).map_err(|_| MemoryFileError::Filesystem)?;
            }
            fs::copy(source, &temporary).map_err(|_| MemoryFileError::Filesystem)?;
            fs::rename(temporary, target).map_err(|_| MemoryFileError::Filesystem)?;
        }
        if let Some(current) = current {
            for file in &current.files {
                if !next_paths.contains(file.relative_path.as_str()) {
                    let stale = joined_safe(&self.root, &file.relative_path)?;
                    if stale.exists() {
                        ensure_regular(&stale)?;
                        fs::remove_file(stale).map_err(|_| MemoryFileError::Filesystem)?;
                    }
                }
            }
        }
        let state_bytes = serde_json::to_vec(next).map_err(|_| MemoryFileError::InvalidInput)?;
        let temporary_state = self.root.join(".agentmage-memory-state.new");
        if temporary_state.exists() {
            fs::remove_file(&temporary_state).map_err(|_| MemoryFileError::Filesystem)?;
        }
        write_new_file(&temporary_state, &state_bytes)?;
        fs::rename(temporary_state, self.root.join(STATE_FILE))
            .map_err(|_| MemoryFileError::Filesystem)?;
        self.verify_projection(&self.root, next)
    }
}

fn state_for_bundle(
    bundle: &MemoryMarkdownBundle,
) -> Result<(InstalledState, ProjectionFiles), MemoryFileError> {
    if !valid_sha256(&bundle.bundle_sha256)
        || bundle.write_enabled
        || bundle.topics.len().saturating_add(1) > MAX_FILES
        || bundle_digest(&bundle.index, &bundle.topics) != bundle.bundle_sha256
    {
        return Err(MemoryFileError::InvalidInput);
    }
    let mut inputs = Vec::with_capacity(bundle.topics.len() + 1);
    inputs.push(&bundle.index);
    inputs.extend(bundle.topics.iter());
    let mut seen = BTreeSet::new();
    let mut files = Vec::with_capacity(inputs.len());
    let mut state_files = Vec::with_capacity(inputs.len());
    for file in inputs {
        validate_markdown_file(file)?;
        if !seen.insert(file.relative_path.clone()) {
            return Err(MemoryFileError::InvalidInput);
        }
        let bytes = file.markdown.as_bytes().to_vec();
        state_files.push(InstalledFile {
            relative_path: file.relative_path.clone(),
            content_sha256: file.content_sha256.clone(),
            byte_count: bytes.len() as u64,
        });
        files.push((file.relative_path.clone(), bytes));
    }
    state_files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok((
        InstalledState {
            schema_version: 1,
            bundle_sha256: bundle.bundle_sha256.clone(),
            files: state_files,
        },
        files,
    ))
}

fn bundle_digest(index: &MemoryMarkdownFile, topics: &[MemoryMarkdownFile]) -> String {
    let mut material = Vec::new();
    material.extend_from_slice(index.relative_path.as_bytes());
    material.extend_from_slice(index.content_sha256.as_bytes());
    for topic in topics {
        material.extend_from_slice(topic.relative_path.as_bytes());
        material.extend_from_slice(topic.content_sha256.as_bytes());
    }
    sha256(&material)
}

fn validate_markdown_file(file: &MemoryMarkdownFile) -> Result<(), MemoryFileError> {
    if file.write_enabled
        || file.markdown.is_empty()
        || file.markdown.len() > MAX_FILE_BYTES
        || sha256(file.markdown.as_bytes()) != file.content_sha256
    {
        return Err(MemoryFileError::InvalidInput);
    }
    joined_safe(Path::new("."), &file.relative_path).map(|_| ())
}

fn validate_state(state: &InstalledState) -> Result<(), MemoryFileError> {
    if state.schema_version != 1
        || !valid_sha256(&state.bundle_sha256)
        || state.files.is_empty()
        || state.files.len() > MAX_FILES
    {
        return Err(MemoryFileError::CorruptState);
    }
    let mut seen = BTreeSet::new();
    for file in &state.files {
        if !seen.insert(&file.relative_path)
            || !valid_sha256(&file.content_sha256)
            || file.byte_count == 0
            || file.byte_count > MAX_FILE_BYTES as u64
            || joined_safe(Path::new("."), &file.relative_path).is_err()
        {
            return Err(MemoryFileError::CorruptState);
        }
    }
    Ok(())
}

fn read_state_at(root: &Path) -> Result<Option<InstalledState>, MemoryFileError> {
    let path = root.join(STATE_FILE);
    if !path.exists() {
        return Ok(None);
    }
    ensure_regular(&path)?;
    let bytes = fs::read(path).map_err(|_| MemoryFileError::Filesystem)?;
    let state = serde_json::from_slice(&bytes).map_err(|_| MemoryFileError::CorruptState)?;
    validate_state(&state)?;
    Ok(Some(state))
}

fn joined_safe(root: &Path, relative: &str) -> Result<PathBuf, MemoryFileError> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(MemoryFileError::InvalidInput);
    }
    Ok(root.join(path))
}

fn validate_single_name(value: &str) -> Result<(), MemoryFileError> {
    let path = Path::new(value);
    if value.is_empty()
        || value.len() > 255
        || path.components().count() != 1
        || !matches!(path.components().next(), Some(Component::Normal(_)))
    {
        return Err(MemoryFileError::InvalidInput);
    }
    Ok(())
}

fn validate_identifier(value: &str) -> Result<(), MemoryFileError> {
    if value.is_empty()
        || value.len() > 128
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
    {
        return Err(MemoryFileError::InvalidInput);
    }
    Ok(())
}

fn ensure_regular(path: &Path) -> Result<(), MemoryFileError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| MemoryFileError::Filesystem)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(MemoryFileError::UnsafeFilesystem);
    }
    Ok(())
}

fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), MemoryFileError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|_| MemoryFileError::Filesystem)?;
    file.write_all(bytes)
        .map_err(|_| MemoryFileError::Filesystem)?;
    file.sync_all().map_err(|_| MemoryFileError::Filesystem)
}

fn receipt(disposition: MemoryFileDisposition, state: &InstalledState) -> MemoryFileReceipt {
    MemoryFileReceipt {
        disposition,
        bundle_sha256: state.bundle_sha256.clone(),
        file_count: state.files.len() as u64,
        network_used: false,
        automatic_decision: false,
    }
}

fn sha256(bytes: &[u8]) -> String {
    let mut value = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        use std::fmt::Write as _;
        write!(&mut value, "{byte:02x}").expect("writing to String cannot fail");
    }
    value
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "agentmage-memory-{name}-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("old fixture removed");
        }
        fs::create_dir(&root).expect("fixture root");
        root
    }

    fn file(path: &str, markdown: &str) -> MemoryMarkdownFile {
        MemoryMarkdownFile {
            relative_path: path.to_owned(),
            markdown: markdown.to_owned(),
            content_sha256: sha256(markdown.as_bytes()),
            write_enabled: false,
        }
    }

    fn bundle(version: &str) -> MemoryMarkdownBundle {
        let index = file("MEMORY.md", &format!("# Memory {version}\n"));
        let topic = file(
            "Memory/memory-one.md",
            &format!("---\nstatus: current\n---\n# Topic {version}\n"),
        );
        let bundle_sha256 = bundle_digest(&index, std::slice::from_ref(&topic));
        MemoryMarkdownBundle {
            bundle_sha256,
            index,
            topics: vec![topic],
            write_enabled: false,
        }
    }

    #[test]
    fn interrupted_or_corrupt_projection_restores_exact_last_good_bytes() {
        let root = temporary_root("recover");
        let runtime = MemoryFileRuntime::open(root.clone()).expect("runtime");
        let first = bundle("one");
        runtime
            .apply_bundle(&first, None, "first")
            .expect("first apply");
        let second = bundle("two");
        runtime
            .apply_bundle(&second, Some(&first.bundle_sha256), "second")
            .expect("second apply");
        fs::write(root.join("MEMORY.md"), b"corrupt\n").expect("simulate interrupted write");
        let receipt = runtime.recover().expect("restore last good");
        assert_eq!(receipt.disposition, MemoryFileDisposition::Restored);
        assert_eq!(receipt.bundle_sha256, first.bundle_sha256);
        assert_eq!(
            fs::read(root.join("MEMORY.md")).expect("restored"),
            b"# Memory one\n"
        );
        fs::write(root.join(STATE_FILE), b"corrupt-state\n").expect("corrupt state");
        assert_eq!(
            runtime
                .recover()
                .expect("restore corrupt manifest")
                .bundle_sha256,
            first.bundle_sha256
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn simultaneous_edit_preserves_current_and_complete_conflict_bundle() {
        let root = temporary_root("conflict");
        let runtime = MemoryFileRuntime::open(root.clone()).expect("runtime");
        let current = bundle("current");
        runtime
            .apply_bundle(&current, None, "current")
            .expect("current apply");
        let proposed = bundle("proposed");
        let receipt = runtime
            .apply_bundle(&proposed, Some(&"9".repeat(64)), "simultaneous")
            .expect("conflict preserved");
        assert_eq!(
            receipt.disposition,
            MemoryFileDisposition::ConflictPreserved
        );
        assert_eq!(
            fs::read(root.join("MEMORY.md")).expect("current"),
            b"# Memory current\n"
        );
        assert_eq!(
            fs::read(root.join("Conflicts/simultaneous/MEMORY.md")).expect("proposal"),
            b"# Memory proposed\n"
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn exact_encrypted_export_migrates_between_machine_roots_without_path_authority() {
        let source_root = temporary_root("machine-a");
        let target_root = temporary_root("machine-b");
        let source = MemoryFileRuntime::open(source_root.clone()).expect("source");
        let target = MemoryFileRuntime::open(target_root.clone()).expect("target");
        let encrypted = b"synthetic-authenticated-memory-envelope";
        let digest = sha256(encrypted);
        source
            .write_portable_export("memory.age", encrypted, &digest)
            .expect("write export");
        let transferred = source
            .read_portable_export("memory.age", &digest)
            .expect("read export");
        target
            .write_portable_export("memory.age", &transferred, &digest)
            .expect("write migrated export");
        assert_eq!(
            target
                .read_portable_export("memory.age", &digest)
                .expect("read migrated export"),
            encrypted
        );
        assert_eq!(
            target.read_portable_export("../memory.age", &digest),
            Err(MemoryFileError::InvalidInput)
        );
        fs::remove_dir_all(source_root).expect("cleanup source");
        fs::remove_dir_all(target_root).expect("cleanup target");
    }
}
