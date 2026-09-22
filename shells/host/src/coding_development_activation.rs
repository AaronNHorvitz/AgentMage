//! Explicit disposable-workspace activation for executable coding development only.

use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use agentmage_kernel_engine::operational_store::{
    OperationalStoreKeyError, OperationalStoreKeyProvider,
};

/// Exact activation identity; this value is never accepted by production startup.
pub const CODING_DEVELOPMENT_ACTIVATION: &str = "coding-development-v1";
const MARKER_NAME: &str = ".agentmage-development-workspace";
const MAX_MARKER_BYTES: u64 = 4096;
const DEVELOPMENT_KEY_NAME: &str = "operational-store-development-v1.key";

/// Stable content-free activation refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodingDevelopmentActivationError {
    /// A root was relative, aliased, nested unsafely, or had the wrong owner/mode.
    RootDenied,
    /// The workspace was not an exact private descendant of the disposable root.
    WorkspaceDenied,
    /// The owned marker was absent, substituted, linked, malformed, or stale.
    MarkerDenied,
    /// The selected directory was not a private ordinary Git worktree.
    GitWorktreeDenied,
}

impl CodingDevelopmentActivationError {
    /// Returns one stable redacted diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::RootDenied => "coding.development.root-denied",
            Self::WorkspaceDenied => "coding.development.workspace-denied",
            Self::MarkerDenied => "coding.development.marker-denied",
            Self::GitWorktreeDenied => "coding.development.git-worktree-denied",
        }
    }
}

impl fmt::Display for CodingDevelopmentActivationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for CodingDevelopmentActivationError {}

/// Continuously revalidatable identities for one development-only coding launch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodingDevelopmentActivation {
    state_root: PathBuf,
    disposable_root: PathBuf,
    workspace_root: PathBuf,
    marker_sha256: String,
    marker_epoch_ms: u64,
}

impl CodingDevelopmentActivation {
    /// Validates exact private roots, the ordinary Git worktree, and its path-bound marker.
    pub fn validate(
        state_root: &Path,
        disposable_root: &Path,
        workspace_root: &Path,
    ) -> Result<Self, CodingDevelopmentActivationError> {
        let state_root = private_canonical_directory(state_root)
            .map_err(|_| CodingDevelopmentActivationError::RootDenied)?;
        let disposable_root = private_canonical_directory(disposable_root)
            .map_err(|_| CodingDevelopmentActivationError::RootDenied)?;
        let workspace_root = private_canonical_directory(workspace_root)
            .map_err(|_| CodingDevelopmentActivationError::WorkspaceDenied)?;
        if workspace_root == disposable_root
            || !workspace_root.starts_with(&disposable_root)
            || state_root.starts_with(&disposable_root)
            || disposable_root.starts_with(&state_root)
        {
            return Err(CodingDevelopmentActivationError::WorkspaceDenied);
        }
        verify_git_directory(&workspace_root)?;
        let marker = workspace_root.join(MARKER_NAME);
        let metadata = fs::symlink_metadata(&marker)
            .map_err(|_| CodingDevelopmentActivationError::MarkerDenied)?;
        if !metadata.file_type().is_file()
            || metadata.uid() != rustix::process::getuid().as_raw()
            || metadata.mode() & 0o777 != 0o600
            || metadata.nlink() != 1
            || metadata.len() == 0
            || metadata.len() > MAX_MARKER_BYTES
        {
            return Err(CodingDevelopmentActivationError::MarkerDenied);
        }
        let marker_epoch_ms = u64::try_from(metadata.mtime())
            .ok()
            .and_then(|seconds| seconds.checked_mul(1_000))
            .and_then(|millis| {
                u64::try_from(metadata.mtime_nsec())
                    .ok()
                    .and_then(|nanos| millis.checked_add(nanos / 1_000_000))
            })
            .filter(|value| *value > 0)
            .ok_or(CodingDevelopmentActivationError::MarkerDenied)?;
        let bytes =
            fs::read(&marker).map_err(|_| CodingDevelopmentActivationError::MarkerDenied)?;
        let expected = marker_contents(&workspace_root)?;
        if bytes != expected {
            return Err(CodingDevelopmentActivationError::MarkerDenied);
        }
        Ok(Self {
            state_root,
            disposable_root,
            workspace_root,
            marker_sha256: hex(&Sha256::digest(bytes)),
            marker_epoch_ms,
        })
    }

    /// Revalidates every development activation input before an effect-bearing phase.
    pub fn revalidate(&self) -> Result<(), CodingDevelopmentActivationError> {
        let current = Self::validate(
            &self.state_root,
            &self.disposable_root,
            &self.workspace_root,
        )?;
        if current != *self {
            return Err(CodingDevelopmentActivationError::MarkerDenied);
        }
        Ok(())
    }

    /// Returns the exact private development state root.
    #[must_use]
    pub fn state_root(&self) -> &Path {
        &self.state_root
    }

    /// Returns the exact disposable parent root.
    #[must_use]
    pub fn disposable_root(&self) -> &Path {
        &self.disposable_root
    }

    /// Returns the exact marked Git worktree.
    #[must_use]
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    /// Returns the digest of the exact activation marker.
    #[must_use]
    pub fn marker_sha256(&self) -> &str {
        &self.marker_sha256
    }

    /// Returns the marker-bound start time used for stable development-session retention.
    #[must_use]
    pub const fn marker_epoch_ms(&self) -> u64 {
        self.marker_epoch_ms
    }
}

/// Returns the exact marker bytes an explicit setup command must install with mode `0600`.
pub fn coding_development_marker(
    workspace_root: &Path,
) -> Result<Vec<u8>, CodingDevelopmentActivationError> {
    let workspace = workspace_root
        .canonicalize()
        .map_err(|_| CodingDevelopmentActivationError::WorkspaceDenied)?;
    marker_contents(&workspace)
}

/// File-backed 256-bit key restricted to the separate coding-development state root.
pub struct CodingDevelopmentKeyProvider {
    key: [u8; 32],
}

impl fmt::Debug for CodingDevelopmentKeyProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CodingDevelopmentKeyProvider")
            .field("key", &"redacted")
            .finish()
    }
}

impl CodingDevelopmentKeyProvider {
    /// Opens or creates the exact private development-only key file.
    pub fn open(
        activation: &CodingDevelopmentActivation,
    ) -> Result<Self, CodingDevelopmentActivationError> {
        activation.revalidate()?;
        let path = activation.state_root.join(DEVELOPMENT_KEY_NAME);
        let mut key = [0_u8; 32];
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
        {
            Ok(mut file) => {
                let count =
                    rustix::rand::getrandom(&mut key, rustix::rand::GetRandomFlags::empty())
                        .map_err(|_| CodingDevelopmentActivationError::RootDenied)?;
                if count != key.len() || key == [0; 32] {
                    key.fill(0);
                    return Err(CodingDevelopmentActivationError::RootDenied);
                }
                file.write_all(&key)
                    .and_then(|()| file.sync_all())
                    .map_err(|_| CodingDevelopmentActivationError::RootDenied)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let mut file = OpenOptions::new()
                    .read(true)
                    .open(&path)
                    .map_err(|_| CodingDevelopmentActivationError::RootDenied)?;
                file.read_exact(&mut key)
                    .map_err(|_| CodingDevelopmentActivationError::RootDenied)?;
                let mut extra = [0_u8; 1];
                if file.read(&mut extra).ok() != Some(0) || key == [0; 32] {
                    key.fill(0);
                    return Err(CodingDevelopmentActivationError::RootDenied);
                }
            }
            Err(_) => return Err(CodingDevelopmentActivationError::RootDenied),
        }
        let metadata =
            fs::symlink_metadata(path).map_err(|_| CodingDevelopmentActivationError::RootDenied)?;
        if !metadata.file_type().is_file()
            || metadata.uid() != rustix::process::getuid().as_raw()
            || metadata.mode() & 0o777 != 0o600
            || metadata.nlink() != 1
            || metadata.len() != 32
        {
            key.fill(0);
            return Err(CodingDevelopmentActivationError::RootDenied);
        }
        Ok(Self { key })
    }
}

impl OperationalStoreKeyProvider for CodingDevelopmentKeyProvider {
    fn with_key<T>(
        &mut self,
        operation: impl FnOnce(&[u8]) -> T,
    ) -> Result<T, OperationalStoreKeyError> {
        Ok(operation(&self.key))
    }
}

impl Drop for CodingDevelopmentKeyProvider {
    fn drop(&mut self) {
        self.key.fill(0);
    }
}

fn marker_contents(workspace: &Path) -> Result<Vec<u8>, CodingDevelopmentActivationError> {
    let workspace = workspace
        .to_str()
        .filter(|value| !value.contains('\0') && !value.contains('\n'))
        .ok_or(CodingDevelopmentActivationError::MarkerDenied)?;
    Ok(format!("activation={CODING_DEVELOPMENT_ACTIVATION}\nworkspace={workspace}\n").into_bytes())
}

fn private_canonical_directory(path: &Path) -> Result<PathBuf, ()> {
    if !path.is_absolute() {
        return Err(());
    }
    let canonical = path.canonicalize().map_err(|_| ())?;
    if canonical != path {
        return Err(());
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| ())?;
    if !metadata.is_dir()
        || metadata.uid() != rustix::process::getuid().as_raw()
        || metadata.mode() & 0o777 != 0o700
    {
        return Err(());
    }
    Ok(canonical)
}

fn verify_git_directory(workspace: &Path) -> Result<(), CodingDevelopmentActivationError> {
    let git = workspace.join(".git");
    let git_metadata = fs::symlink_metadata(&git)
        .map_err(|_| CodingDevelopmentActivationError::GitWorktreeDenied)?;
    let head = git.join("HEAD");
    let head_metadata = fs::symlink_metadata(head)
        .map_err(|_| CodingDevelopmentActivationError::GitWorktreeDenied)?;
    if !git_metadata.is_dir()
        || git_metadata.uid() != rustix::process::getuid().as_raw()
        || git_metadata.mode() & 0o022 != 0
        || !head_metadata.is_file()
        || head_metadata.uid() != rustix::process::getuid().as_raw()
        || head_metadata.nlink() != 1
    {
        return Err(CodingDevelopmentActivationError::GitWorktreeDenied);
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static NEXT: AtomicU64 = AtomicU64::new(1);

    #[test]
    fn exact_private_marked_git_workspace_is_the_only_activation() {
        let base = std::env::temp_dir().join(format!(
            "agentmage-coding-activation-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let state = base.join("state");
        let disposable = base.join("disposable");
        let workspace = disposable.join("worktree");
        for path in [
            &base,
            &state,
            &disposable,
            &workspace,
            &workspace.join(".git"),
        ] {
            fs::create_dir(path).expect("create directory");
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).expect("private mode");
        }
        fs::write(workspace.join(".git/HEAD"), b"ref: refs/heads/main\n").expect("HEAD");
        let marker = workspace.join(MARKER_NAME);
        fs::write(
            &marker,
            coding_development_marker(&workspace).expect("marker"),
        )
        .expect("write");
        fs::set_permissions(&marker, fs::Permissions::from_mode(0o600)).expect("marker mode");

        let activation = CodingDevelopmentActivation::validate(&state, &disposable, &workspace)
            .expect("activation");
        activation.revalidate().expect("revalidate");
        assert_eq!(activation.workspace_root(), workspace);

        fs::write(
            &marker,
            b"activation=coding-development-v1\nworkspace=/wrong\n",
        )
        .expect("mutate marker");
        assert_eq!(
            activation.revalidate(),
            Err(CodingDevelopmentActivationError::MarkerDenied)
        );
        fs::remove_dir_all(base).expect("cleanup");
    }
}
