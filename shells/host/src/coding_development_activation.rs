//! Explicit disposable-workspace activation for executable coding development only.

use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use agentmage_platform_linux::LinuxLinkedRepository;
use sha2::{Digest, Sha256};

use agentmage_kernel_engine::operational_store::{
    OperationalStoreKeyError, OperationalStoreKeyProvider,
};

/// Exact activation identity; this value is never accepted by production startup.
pub const CODING_DEVELOPMENT_ACTIVATION: &str = "coding-development-v1";
pub(crate) const MARKER_NAME: &str = ".agentmage-development-workspace";
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
    /// The selected directory was not an admitted ordinary or linked Git worktree.
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
    linked_repository: Option<LinuxLinkedRepository>,
}

impl CodingDevelopmentActivation {
    /// Validates exact private roots, Git metadata, and the explicit path-bound
    /// marker. Linked worktrees require additional exact metadata marker fields.
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
        let mut marker_file = OpenOptions::new()
            .read(true)
            .custom_flags(
                (rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32,
            )
            .open(&marker)
            .map_err(|_| CodingDevelopmentActivationError::MarkerDenied)?;
        let opened = marker_file
            .metadata()
            .map_err(|_| CodingDevelopmentActivationError::MarkerDenied)?;
        if (
            opened.dev(),
            opened.ino(),
            opened.mode(),
            opened.len(),
            opened.nlink(),
        ) != (
            metadata.dev(),
            metadata.ino(),
            metadata.mode(),
            metadata.len(),
            metadata.nlink(),
        ) {
            return Err(CodingDevelopmentActivationError::MarkerDenied);
        }
        let mut bytes = Vec::new();
        Read::by_ref(&mut marker_file)
            .take(MAX_MARKER_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| CodingDevelopmentActivationError::MarkerDenied)?;
        if bytes.len() as u64 != metadata.len() || bytes.len() as u64 > MAX_MARKER_BYTES {
            return Err(CodingDevelopmentActivationError::MarkerDenied);
        }
        // Resolve external metadata only after a valid private marker names
        // its exact paths. A workspace-only or malformed marker cannot expand
        // read scope by following an untrusted .git pointer.
        let linked_repository = linked_repository_for_marker(&workspace_root, &bytes)?;
        let expected = marker_contents(&workspace_root, linked_repository.as_ref())?;
        if bytes != expected {
            return Err(CodingDevelopmentActivationError::MarkerDenied);
        }
        Ok(Self {
            state_root,
            disposable_root,
            workspace_root,
            marker_sha256: hex(&Sha256::digest(bytes)),
            marker_epoch_ms,
            linked_repository,
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

    /// Metadata explicitly included in this activation's marker, if linked.
    /// This is read-only scope, never manual worktree-creation provenance.
    #[must_use]
    pub fn linked_repository(&self) -> Option<&LinuxLinkedRepository> {
        self.linked_repository.as_ref()
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
    verify_git_directory(&workspace)?;
    marker_contents(&workspace, None)
}

/// Displays linked setup bytes only for the two explicitly selected metadata
/// roots. An untrusted workspace pointer cannot redirect these reads elsewhere.
pub fn coding_development_linked_marker(
    workspace_root: &Path,
    git_metadata: &Path,
    git_common: &Path,
) -> Result<Vec<u8>, CodingDevelopmentActivationError> {
    let linked = LinuxLinkedRepository::verify(workspace_root, git_metadata, git_common)
        .map_err(|_| CodingDevelopmentActivationError::GitWorktreeDenied)?;
    marker_contents(workspace_root, Some(&linked))
}

fn linked_repository_for_marker(
    workspace: &Path,
    bytes: &[u8],
) -> Result<Option<LinuxLinkedRepository>, CodingDevelopmentActivationError> {
    let denied = CodingDevelopmentActivationError::MarkerDenied;
    let prefix = marker_contents(workspace, None)?;
    if bytes == prefix {
        if fs::symlink_metadata(workspace.join(".git")).is_ok_and(|m| m.is_file()) {
            return Err(denied);
        }
        verify_git_directory(workspace)?;
        return Ok(None);
    }
    let suffix = bytes.strip_prefix(prefix.as_slice()).ok_or(denied)?;
    let fields: Vec<_> = std::str::from_utf8(suffix)
        .map_err(|_| denied)?
        .split_terminator('\n')
        .collect();
    let [git, common, digest] = fields.as_slice() else {
        return Err(denied);
    };
    let git = git.strip_prefix("git-metadata=").ok_or(denied)?;
    let common = common.strip_prefix("git-common=").ok_or(denied)?;
    let digest = digest.strip_prefix("metadata-sha256=").ok_or(denied)?;
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(denied);
    }
    LinuxLinkedRepository::verify(workspace, Path::new(git), Path::new(common))
        .map(Some)
        .map_err(|_| CodingDevelopmentActivationError::GitWorktreeDenied)
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
        Self::open_in_state_root(&activation.state_root)
    }

    /// Opens or creates the development-only key file of another validated
    /// development activation's private state root (Decision 0150). The caller
    /// revalidates its activation first.
    pub fn open_in_state_root(state_root: &Path) -> Result<Self, CodingDevelopmentActivationError> {
        let path = state_root.join(DEVELOPMENT_KEY_NAME);
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

fn marker_contents(
    workspace: &Path,
    linked: Option<&LinuxLinkedRepository>,
) -> Result<Vec<u8>, CodingDevelopmentActivationError> {
    let workspace = workspace
        .to_str()
        .filter(|value| !value.contains('\0') && !value.contains('\n'))
        .ok_or(CodingDevelopmentActivationError::MarkerDenied)?;
    let mut marker = format!("activation={CODING_DEVELOPMENT_ACTIVATION}\nworkspace={workspace}\n");
    if let Some(linked) = linked {
        marker.push_str(&format!(
            "git-metadata={}\ngit-common={}\nmetadata-sha256={}\n",
            linked
                .git_directory()
                .to_str()
                .ok_or(CodingDevelopmentActivationError::MarkerDenied)?,
            linked
                .common_directory()
                .to_str()
                .ok_or(CodingDevelopmentActivationError::MarkerDenied)?,
            linked.identity_sha256()
        ));
    }
    if marker.len() > MAX_MARKER_BYTES as usize {
        return Err(CodingDevelopmentActivationError::MarkerDenied);
    }
    Ok(marker.into_bytes())
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
    if !git_metadata.is_dir()
        || git_metadata.uid() != rustix::process::getuid().as_raw()
        || git_metadata.mode() & 0o022 != 0
    {
        return Err(CodingDevelopmentActivationError::GitWorktreeDenied);
    }
    let head = git.join("HEAD");
    let head_metadata = fs::symlink_metadata(head)
        .map_err(|_| CodingDevelopmentActivationError::GitWorktreeDenied)?;
    if !head_metadata.is_file()
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

    #[test]
    fn real_linked_worktree_requires_metadata_marker_and_refuses_substitution() {
        // Git is used only to create this disposable fixture. This does not
        // bypass or exercise the product's pinned executable/worker boundary.
        let started = std::time::Instant::now();
        let base = std::env::temp_dir().join(format!(
            "agentmage-linked-activation-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let state = base.join("state");
        let disposable = base.join("disposable");
        let original = base.join("original");
        let workspace = disposable.join("worktree");
        for path in [&base, &state, &disposable, &original] {
            fs::create_dir(path).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        for args in [
            vec!["init", "--quiet", "--initial-branch=main"],
            vec![
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--quiet",
                "--allow-empty",
                "-m",
                "fixture",
            ],
            vec![
                "worktree",
                "add",
                "--quiet",
                "-b",
                "agentmage/tasks/linked-fixture",
                workspace.to_str().unwrap(),
                "HEAD",
            ],
        ] {
            let output = std::process::Command::new("/usr/bin/git")
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("HOME", &base)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .env("LANG", "C")
                .arg("-C")
                .arg(&original)
                .args(args)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "fixture Git: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        fs::set_permissions(&workspace, fs::Permissions::from_mode(0o700)).unwrap();
        let marker = workspace.join(MARKER_NAME);
        fs::write(
            &marker,
            format!(
                "activation={CODING_DEVELOPMENT_ACTIVATION}\nworkspace={}\n",
                workspace.display()
            ),
        )
        .unwrap();
        fs::set_permissions(&marker, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(
            CodingDevelopmentActivation::validate(&state, &disposable, &workspace),
            Err(CodingDevelopmentActivationError::MarkerDenied)
        );
        let git_metadata = original.join(".git/worktrees/worktree");
        let parked = original.join(".git/worktrees/parked");
        fs::rename(&git_metadata, &parked).unwrap();
        assert_eq!(
            CodingDevelopmentActivation::validate(&state, &disposable, &workspace),
            Err(CodingDevelopmentActivationError::MarkerDenied),
            "legacy marker must refuse before following unavailable external metadata"
        );
        fs::rename(&parked, &git_metadata).unwrap();
        let contents =
            coding_development_linked_marker(&workspace, &git_metadata, &original.join(".git"))
                .unwrap();
        assert!(String::from_utf8_lossy(&contents).contains("\ngit-common="));
        assert!(String::from_utf8_lossy(&contents).contains("\nmetadata-sha256="));
        fs::write(&marker, &contents).unwrap();
        let activation =
            CodingDevelopmentActivation::validate(&state, &disposable, &workspace).unwrap();
        activation.revalidate().unwrap();
        let linked = activation.linked_repository().unwrap();
        assert_eq!(linked.worktree_root(), workspace);
        assert_eq!(linked.common_directory(), original.join(".git"));
        let management = state.join("repository-management");
        fs::create_dir(&management).unwrap();
        fs::set_permissions(&management, fs::Permissions::from_mode(0o700)).unwrap();
        agentmage_platform_linux::LinuxRepositoryScope::verify_linked_read_only(
            linked,
            &management,
        )
        .unwrap();
        let pointer = workspace.join(".git");
        let saved = workspace.join("saved-pointer");
        fs::rename(&pointer, &saved).unwrap();
        fs::write(&pointer, fs::read(&saved).unwrap()).unwrap();
        assert_eq!(
            activation.revalidate(),
            Err(CodingDevelopmentActivationError::MarkerDenied)
        );
        // The earlier marker cannot authorize a same-byte replacement. Fresh
        // setup is necessary; neither path nor old digest restores activation.
        assert_eq!(
            CodingDevelopmentActivation::validate(&state, &disposable, &workspace),
            Err(CodingDevelopmentActivationError::MarkerDenied)
        );
        eprintln!(
            "real Git linked activation component: created=1 activated=1 legacy_marker_refused=1 replacement_refused=1 elapsed_us={}",
            started.elapsed().as_micros()
        );
        fs::remove_dir_all(&base).unwrap();
    }
}
