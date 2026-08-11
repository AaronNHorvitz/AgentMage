#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Fedora and Ubuntu platform path adapter.

use std::fmt;

use agentmage_kernel_contracts::{
    AdapterInstanceId, AuthorizedWorkspaceHandle, FilePreimage, HeldWorkspaceObject,
    PathAdapterError, PathAdapterErrorKind, PathPlatform, PathResolutionIntent,
    PlatformPathAdapter, WorkspaceAuthorizationId, WorkspaceId, WorkspaceObjectIdentity,
    WorkspaceObjectKind, WorkspacePath,
};
use rustix::fd::OwnedFd;
use rustix::fs::{FileType, Mode, OFlags, Stat, fstat, openat};
use rustix::io::{Errno, pread};
use sha2::{Digest, Sha256};

/// Stable component identity used by diagnostics and build verification.
pub const COMPONENT_ID: &str = "platform-linux";

/// Default maximum number of bytes included in one exact path preimage.
pub const DEFAULT_MAX_PREIMAGE_BYTES: u64 = 64 * 1024 * 1024;

const HASH_BUFFER_BYTES: usize = 64 * 1024;

/// Returns the identity of the contracts implemented by this adapter.
#[must_use]
pub const fn contract_component_id() -> &'static str {
    agentmage_kernel_contracts::COMPONENT_ID
}

/// Linux path adapter with an exact instance identity and bounded hash limit.
#[derive(Debug)]
pub struct LinuxPathAdapter {
    adapter_instance_id: AdapterInstanceId,
    max_preimage_bytes: u64,
}

impl LinuxPathAdapter {
    /// Creates an unprivileged Linux path adapter without authorizing a workspace.
    #[must_use]
    pub const fn new(adapter_instance_id: AdapterInstanceId, max_preimage_bytes: u64) -> Self {
        Self {
            adapter_instance_id,
            max_preimage_bytes,
        }
    }

    /// Returns the configured exact-preimage byte limit.
    #[must_use]
    pub const fn max_preimage_bytes(&self) -> u64 {
        self.max_preimage_bytes
    }
}

/// Adapter-owned authorization for one already user-approved Linux workspace.
///
/// Construction remains private until the workspace-selection boundary is implemented.
pub struct LinuxAuthorizedWorkspace {
    workspace_id: WorkspaceId,
    authorization_id: WorkspaceAuthorizationId,
    adapter_instance_id: AdapterInstanceId,
    root_descriptor: OwnedFd,
    root_snapshot: LinuxStatSnapshot,
}

impl fmt::Debug for LinuxAuthorizedWorkspace {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxAuthorizedWorkspace")
            .field("workspace_id", &self.workspace_id)
            .field("authorization_id", &self.authorization_id)
            .field("adapter_instance_id", &self.adapter_instance_id)
            .finish_non_exhaustive()
    }
}

impl AuthorizedWorkspaceHandle for LinuxAuthorizedWorkspace {
    fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    fn authorization_id(&self) -> &WorkspaceAuthorizationId {
        &self.authorization_id
    }

    fn adapter_instance_id(&self) -> &AdapterInstanceId {
        &self.adapter_instance_id
    }

    fn platform(&self) -> PathPlatform {
        PathPlatform::Linux
    }
}

/// Linux object whose workspace root and resolved object descriptors remain held.
pub struct LinuxHeldObject {
    workspace_path: WorkspacePath,
    authorization_id: WorkspaceAuthorizationId,
    adapter_instance_id: AdapterInstanceId,
    intent: PathResolutionIntent,
    object_kind: WorkspaceObjectKind,
    object_identity: WorkspaceObjectIdentity,
    preimage: Option<FilePreimage>,
    root_descriptor: OwnedFd,
    root_snapshot: LinuxStatSnapshot,
    object_descriptor: OwnedFd,
    object_snapshot: LinuxStatSnapshot,
    max_preimage_bytes: u64,
}

impl fmt::Debug for LinuxHeldObject {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxHeldObject")
            .field("authorization_id", &self.authorization_id)
            .field("adapter_instance_id", &self.adapter_instance_id)
            .field("intent", &self.intent)
            .field("object_kind", &self.object_kind)
            .field("has_preimage", &self.preimage.is_some())
            .finish_non_exhaustive()
    }
}

impl LinuxHeldObject {
    /// Revalidates the continuously held root and object identities.
    pub fn revalidate(&self) -> Result<(), PathAdapterError> {
        let root_now = snapshot(&self.root_descriptor, None)?;
        if !self.root_snapshot.same_object(&root_now) {
            return Err(adapter_error(PathAdapterErrorKind::MountChanged, None));
        }
        let object_now = snapshot(&self.object_descriptor, None)?;
        if !self.object_snapshot.same_object(&object_now)
            || self.object_snapshot.link_count != object_now.link_count
            || self.object_snapshot.mode != object_now.mode
        {
            return Err(adapter_error(PathAdapterErrorKind::IdentityChanged, None));
        }
        if let Some(expected) = &self.preimage {
            let observed = hash_preimage(
                &self.object_descriptor,
                &object_now,
                self.max_preimage_bytes,
                None,
            )?;
            if &observed != expected {
                return Err(adapter_error(PathAdapterErrorKind::IdentityChanged, None));
            }
        } else if self.object_snapshot != object_now {
            return Err(adapter_error(PathAdapterErrorKind::IdentityChanged, None));
        }
        Ok(())
    }
}

impl HeldWorkspaceObject for LinuxHeldObject {
    fn workspace_path(&self) -> &WorkspacePath {
        &self.workspace_path
    }

    fn authorization_id(&self) -> &WorkspaceAuthorizationId {
        &self.authorization_id
    }

    fn adapter_instance_id(&self) -> &AdapterInstanceId {
        &self.adapter_instance_id
    }

    fn intent(&self) -> PathResolutionIntent {
        self.intent
    }

    fn object_kind(&self) -> WorkspaceObjectKind {
        self.object_kind
    }

    fn object_identity(&self) -> &WorkspaceObjectIdentity {
        &self.object_identity
    }

    fn preimage(&self) -> Option<&FilePreimage> {
        self.preimage.as_ref()
    }
}

impl PlatformPathAdapter for LinuxPathAdapter {
    type WorkspaceHandle = LinuxAuthorizedWorkspace;
    type HeldObject = LinuxHeldObject;

    fn adapter_instance_id(&self) -> &AdapterInstanceId {
        &self.adapter_instance_id
    }

    fn platform(&self) -> PathPlatform {
        PathPlatform::Linux
    }

    fn resolve(
        &self,
        workspace: &Self::WorkspaceHandle,
        path: &WorkspacePath,
        intent: PathResolutionIntent,
    ) -> Result<Self::HeldObject, PathAdapterError> {
        if workspace.adapter_instance_id() != self.adapter_instance_id() {
            return Err(adapter_error(PathAdapterErrorKind::ForeignHandle, None));
        }
        if workspace.workspace_id() != path.workspace_id() {
            return Err(adapter_error(PathAdapterErrorKind::WorkspaceMismatch, None));
        }

        let root_now = snapshot(&workspace.root_descriptor, None)?;
        if !workspace.root_snapshot.same_object(&root_now)
            || root_now.file_type != FileType::Directory
        {
            return Err(adapter_error(PathAdapterErrorKind::MountChanged, None));
        }
        let held_root = workspace
            .root_descriptor
            .try_clone()
            .map_err(|_| adapter_error(PathAdapterErrorKind::PlatformFailure, None))?;
        let mut current_directory = workspace
            .root_descriptor
            .try_clone()
            .map_err(|_| adapter_error(PathAdapterErrorKind::PlatformFailure, None))?;
        let last_index = path.components().len() - 1;

        for (index, component) in path.components()[..last_index].iter().enumerate() {
            let next = openat(
                &current_directory,
                component.as_str(),
                OFlags::PATH | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|error| open_error(error, index))?;
            let next_snapshot = snapshot(&next, Some(index))?;
            if next_snapshot.file_type == FileType::Symlink {
                return Err(adapter_error(
                    PathAdapterErrorKind::SymbolicLink,
                    Some(index),
                ));
            }
            if next_snapshot.file_type != FileType::Directory {
                return Err(adapter_error(
                    PathAdapterErrorKind::ObjectKindMismatch,
                    Some(index),
                ));
            }
            if next_snapshot.device != root_now.device {
                return Err(adapter_error(
                    PathAdapterErrorKind::MountChanged,
                    Some(index),
                ));
            }
            current_directory = next;
        }

        let final_component = &path.components()[last_index];
        let candidate = openat(
            &current_directory,
            final_component.as_str(),
            OFlags::PATH | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|error| open_error(error, last_index))?;
        let candidate_snapshot = snapshot(&candidate, Some(last_index))?;
        if candidate_snapshot.file_type == FileType::Symlink {
            return Err(adapter_error(
                PathAdapterErrorKind::SymbolicLink,
                Some(last_index),
            ));
        }
        if candidate_snapshot.device != root_now.device {
            return Err(adapter_error(
                PathAdapterErrorKind::MountChanged,
                Some(last_index),
            ));
        }
        let object_kind = admitted_kind(&candidate_snapshot, intent, last_index)?;

        let object_descriptor = match intent {
            PathResolutionIntent::Metadata => candidate,
            PathResolutionIntent::ReadDirectory => reopen_and_compare(
                &current_directory,
                final_component.as_str(),
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                &candidate_snapshot,
                last_index,
            )?,
            PathResolutionIntent::ReadFile | PathResolutionIntent::ContentHash => {
                reopen_and_compare(
                    &current_directory,
                    final_component.as_str(),
                    OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    &candidate_snapshot,
                    last_index,
                )?
            }
        };
        let object_snapshot = snapshot(&object_descriptor, Some(last_index))?;
        let preimage = if matches!(
            intent,
            PathResolutionIntent::ReadFile | PathResolutionIntent::ContentHash
        ) {
            Some(hash_preimage(
                &object_descriptor,
                &object_snapshot,
                self.max_preimage_bytes,
                Some(last_index),
            )?)
        } else {
            None
        };
        let object_identity = WorkspaceObjectIdentity::new(
            PathPlatform::Linux,
            identity_digest(b"agentmage.linux.mount.v1", &root_now),
            identity_digest(b"agentmage.linux.object.v1", &object_snapshot),
        );

        Ok(LinuxHeldObject {
            workspace_path: path.clone(),
            authorization_id: workspace.authorization_id().clone(),
            adapter_instance_id: workspace.adapter_instance_id().clone(),
            intent,
            object_kind,
            object_identity,
            preimage,
            root_descriptor: held_root,
            root_snapshot: root_now,
            object_descriptor,
            object_snapshot,
            max_preimage_bytes: self.max_preimage_bytes,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LinuxStatSnapshot {
    device: u64,
    inode: u64,
    link_count: u64,
    mode: u32,
    size: i64,
    modified_seconds: i64,
    modified_nanoseconds: u64,
    changed_seconds: i64,
    changed_nanoseconds: u64,
    file_type: FileType,
}

impl LinuxStatSnapshot {
    fn from_stat(stat: &Stat) -> Self {
        Self {
            device: stat.st_dev,
            inode: stat.st_ino,
            link_count: stat.st_nlink,
            mode: stat.st_mode,
            size: stat.st_size,
            modified_seconds: stat.st_mtime,
            modified_nanoseconds: stat.st_mtime_nsec,
            changed_seconds: stat.st_ctime,
            changed_nanoseconds: stat.st_ctime_nsec,
            file_type: FileType::from_raw_mode(stat.st_mode),
        }
    }

    fn same_object(&self, other: &Self) -> bool {
        self.device == other.device
            && self.inode == other.inode
            && self.file_type == other.file_type
    }
}

fn snapshot(
    descriptor: &OwnedFd,
    component_index: Option<usize>,
) -> Result<LinuxStatSnapshot, PathAdapterError> {
    fstat(descriptor)
        .map(|stat| LinuxStatSnapshot::from_stat(&stat))
        .map_err(|_| adapter_error(PathAdapterErrorKind::PlatformFailure, component_index))
}

fn admitted_kind(
    snapshot: &LinuxStatSnapshot,
    intent: PathResolutionIntent,
    component_index: usize,
) -> Result<WorkspaceObjectKind, PathAdapterError> {
    let kind = match snapshot.file_type {
        FileType::RegularFile => WorkspaceObjectKind::RegularFile,
        FileType::Directory => WorkspaceObjectKind::Directory,
        FileType::Symlink => {
            return Err(adapter_error(
                PathAdapterErrorKind::SymbolicLink,
                Some(component_index),
            ));
        }
        _ => {
            return Err(adapter_error(
                PathAdapterErrorKind::ObjectKindMismatch,
                Some(component_index),
            ));
        }
    };
    if snapshot.file_type == FileType::RegularFile && snapshot.link_count > 1 {
        return Err(adapter_error(
            PathAdapterErrorKind::HardLink,
            Some(component_index),
        ));
    }
    let compatible = matches!(intent, PathResolutionIntent::Metadata)
        || matches!(
            (intent, kind),
            (
                PathResolutionIntent::ReadFile | PathResolutionIntent::ContentHash,
                WorkspaceObjectKind::RegularFile
            ) | (
                PathResolutionIntent::ReadDirectory,
                WorkspaceObjectKind::Directory
            )
        );
    if !compatible {
        return Err(adapter_error(
            PathAdapterErrorKind::ObjectKindMismatch,
            Some(component_index),
        ));
    }
    Ok(kind)
}

fn reopen_and_compare(
    parent: &OwnedFd,
    component: &str,
    flags: OFlags,
    candidate: &LinuxStatSnapshot,
    component_index: usize,
) -> Result<OwnedFd, PathAdapterError> {
    let descriptor = openat(parent, component, flags, Mode::empty())
        .map_err(|error| open_error(error, component_index))?;
    let reopened = snapshot(&descriptor, Some(component_index))?;
    if &reopened != candidate {
        return Err(adapter_error(
            PathAdapterErrorKind::IdentityChanged,
            Some(component_index),
        ));
    }
    Ok(descriptor)
}

fn hash_preimage(
    descriptor: &OwnedFd,
    expected: &LinuxStatSnapshot,
    max_bytes: u64,
    component_index: Option<usize>,
) -> Result<FilePreimage, PathAdapterError> {
    if expected.size < 0 || expected.size.cast_unsigned() > max_bytes {
        return Err(adapter_error(
            PathAdapterErrorKind::ResourceLimitExceeded,
            component_index,
        ));
    }
    let mut digest = Sha256::new();
    let mut byte_len = 0_u64;
    let mut buffer = [0_u8; HASH_BUFFER_BYTES];
    loop {
        let read_count = pread(descriptor, &mut buffer, byte_len)
            .map_err(|_| adapter_error(PathAdapterErrorKind::PlatformFailure, component_index))?;
        if read_count == 0 {
            break;
        }
        byte_len = byte_len.checked_add(read_count as u64).ok_or_else(|| {
            adapter_error(PathAdapterErrorKind::ResourceLimitExceeded, component_index)
        })?;
        if byte_len > max_bytes {
            return Err(adapter_error(
                PathAdapterErrorKind::ResourceLimitExceeded,
                component_index,
            ));
        }
        digest.update(&buffer[..read_count]);
    }
    let observed = snapshot(descriptor, component_index)?;
    if &observed != expected || byte_len != expected.size.cast_unsigned() {
        return Err(adapter_error(
            PathAdapterErrorKind::IdentityChanged,
            component_index,
        ));
    }
    Ok(FilePreimage::new(byte_len, digest.finalize().into()))
}

fn identity_digest(domain: &[u8], snapshot: &LinuxStatSnapshot) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update((domain.len() as u64).to_be_bytes());
    digest.update(domain);
    digest.update(snapshot.device.to_be_bytes());
    digest.update(snapshot.inode.to_be_bytes());
    digest.update(snapshot.mode.to_be_bytes());
    digest.finalize().into()
}

fn open_error(error: Errno, component_index: usize) -> PathAdapterError {
    let kind = if error == Errno::LOOP {
        PathAdapterErrorKind::SymbolicLink
    } else if error == Errno::NOENT {
        PathAdapterErrorKind::NotFound
    } else if error == Errno::ACCESS || error == Errno::PERM {
        PathAdapterErrorKind::PermissionDenied
    } else if error == Errno::XDEV {
        PathAdapterErrorKind::MountChanged
    } else if error == Errno::NOTDIR || error == Errno::ISDIR {
        PathAdapterErrorKind::ObjectKindMismatch
    } else {
        PathAdapterErrorKind::PlatformFailure
    };
    adapter_error(kind, Some(component_index))
}

const fn adapter_error(
    kind: PathAdapterErrorKind,
    component_index: Option<usize>,
) -> PathAdapterError {
    PathAdapterError::new(kind, component_index)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::{
        AdapterInstanceId, HeldWorkspaceObject, PathAdapterErrorKind, PathResolutionIntent,
        PlatformPathAdapter, WorkspaceAuthorizationId, WorkspaceId, WorkspaceObjectKind,
        WorkspacePath,
    };
    use rustix::fs::{Mode, OFlags, open};
    use sha2::{Digest, Sha256};

    use super::{
        COMPONENT_ID, DEFAULT_MAX_PREIMAGE_BYTES, LinuxAuthorizedWorkspace, LinuxPathAdapter,
        LinuxStatSnapshot, contract_component_id,
    };

    static TEMP_ID: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory {
        path: PathBuf,
    }

    impl TestDirectory {
        fn new() -> Self {
            let id = TEMP_ID.fetch_add(1, Ordering::SeqCst);
            let path = std::env::temp_dir()
                .join(format!("agentmage-linux-path-{}-{id}", std::process::id()));
            fs::create_dir(&path).expect("temporary test directory creates");
            Self { path }
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.path).expect("temporary test directory removes");
        }
    }

    fn adapter() -> LinuxPathAdapter {
        LinuxPathAdapter::new(
            AdapterInstanceId::from_raw("adapter-linux-0001"),
            DEFAULT_MAX_PREIMAGE_BYTES,
        )
    }

    fn authorize_for_test(root: &Path) -> LinuxAuthorizedWorkspace {
        let root_descriptor = open(
            root,
            OFlags::PATH | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .expect("test root opens");
        let root_snapshot = super::snapshot(&root_descriptor, None).expect("test root stats");
        LinuxAuthorizedWorkspace {
            workspace_id: WorkspaceId::from_raw("workspace-0001"),
            authorization_id: WorkspaceAuthorizationId::from_raw("authorization-0001"),
            adapter_instance_id: AdapterInstanceId::from_raw("adapter-linux-0001"),
            root_descriptor,
            root_snapshot,
        }
    }

    fn path(components: &[&str]) -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-0001"),
            components.iter().copied(),
        )
        .expect("canonical test path")
    }

    #[test]
    fn adapter_depends_only_on_contracts() {
        assert_eq!(COMPONENT_ID, "platform-linux");
        assert_eq!(contract_component_id(), "kernel-contracts");
    }

    #[test]
    fn descriptor_walk_holds_the_original_file_and_exact_preimage_across_replacement() {
        let test = TestDirectory::new();
        let root = test.path.join("workspace");
        fs::create_dir_all(root.join("docs")).expect("fixture directories create");
        let original = b"held original bytes\n";
        fs::write(root.join("docs/input.txt"), original).expect("fixture file writes");
        let workspace = authorize_for_test(&root);

        let held = adapter()
            .resolve(
                &workspace,
                &path(&["docs", "input.txt"]),
                PathResolutionIntent::ContentHash,
            )
            .expect("descriptor-relative hash resolves");
        let expected: [u8; 32] = Sha256::digest(original).into();
        assert_eq!(held.object_kind(), WorkspaceObjectKind::RegularFile);
        assert_eq!(
            held.preimage().expect("preimage").byte_len(),
            original.len() as u64
        );
        assert_eq!(
            held.preimage().expect("preimage").content_sha256(),
            &expected
        );
        assert_eq!(
            held.object_identity().platform(),
            agentmage_kernel_contracts::PathPlatform::Linux
        );

        fs::rename(root.join("docs/input.txt"), root.join("docs/moved.txt"))
            .expect("held file renames");
        fs::write(root.join("docs/input.txt"), b"replacement bytes\n").expect("replacement writes");
        held.revalidate()
            .expect("held descriptor remains the validated original object");
    }

    #[test]
    fn symlink_hard_link_special_kind_and_resource_limit_fail_closed() {
        let test = TestDirectory::new();
        let root = test.path.join("workspace");
        let outside = test.path.join("outside");
        fs::create_dir_all(root.join("docs")).expect("workspace creates");
        fs::create_dir(&outside).expect("outside creates");
        fs::write(outside.join("secret.txt"), b"outside\n").expect("outside file writes");
        symlink(&outside, root.join("docs/link-out")).expect("symlink creates");
        fs::write(root.join("docs/original.txt"), b"linked\n").expect("original writes");
        fs::hard_link(
            root.join("docs/original.txt"),
            root.join("docs/hard-link.txt"),
        )
        .expect("hard link creates");
        fs::write(root.join("docs/large.txt"), b"too large").expect("large file writes");
        let workspace = authorize_for_test(&root);

        for (candidate, intent, expected) in [
            (
                path(&["docs", "link-out", "secret.txt"]),
                PathResolutionIntent::ReadFile,
                PathAdapterErrorKind::SymbolicLink,
            ),
            (
                path(&["docs", "hard-link.txt"]),
                PathResolutionIntent::ReadFile,
                PathAdapterErrorKind::HardLink,
            ),
            (
                path(&["docs"]),
                PathResolutionIntent::ReadFile,
                PathAdapterErrorKind::ObjectKindMismatch,
            ),
        ] {
            let error = adapter()
                .resolve(&workspace, &candidate, intent)
                .expect_err("unsafe fixture rejects");
            assert_eq!(error.kind(), expected);
            assert!(error.component_index().is_some());
        }

        let limited = LinuxPathAdapter::new(AdapterInstanceId::from_raw("adapter-linux-0001"), 4);
        let error = limited
            .resolve(
                &workspace,
                &path(&["docs", "large.txt"]),
                PathResolutionIntent::ContentHash,
            )
            .expect_err("oversized hash rejects");
        assert_eq!(error.kind(), PathAdapterErrorKind::ResourceLimitExceeded);
    }

    #[test]
    fn affinity_precedes_observation_and_post_resolution_mutation_is_detected() {
        let test = TestDirectory::new();
        let root = test.path.join("workspace");
        fs::create_dir(&root).expect("workspace creates");
        fs::write(root.join("input.txt"), b"before\n").expect("fixture writes");
        let workspace = authorize_for_test(&root);
        let mismatched =
            WorkspacePath::new(WorkspaceId::from_raw("workspace-0002"), ["missing.txt"])
                .expect("mismatched path constructs");
        let error = adapter()
            .resolve(&workspace, &mismatched, PathResolutionIntent::Metadata)
            .expect_err("workspace mismatch rejects before missing-path observation");
        assert_eq!(error.kind(), PathAdapterErrorKind::WorkspaceMismatch);
        assert_eq!(error.component_index(), None);

        let held = adapter()
            .resolve(
                &workspace,
                &path(&["input.txt"]),
                PathResolutionIntent::ReadFile,
            )
            .expect("file resolves");
        fs::write(root.join("input.txt"), b"changed bytes\n").expect("fixture mutates");
        let changed = held.revalidate().expect_err("identity mutation rejects");
        assert_eq!(changed.kind(), PathAdapterErrorKind::IdentityChanged);
    }

    #[test]
    fn snapshot_comparison_includes_content_change_indicators() {
        let original = LinuxStatSnapshot {
            device: 1,
            inode: 2,
            link_count: 1,
            mode: 3,
            size: 4,
            modified_seconds: 5,
            modified_nanoseconds: 6,
            changed_seconds: 7,
            changed_nanoseconds: 8,
            file_type: rustix::fs::FileType::RegularFile,
        };
        let mut changed = original.clone();
        changed.changed_nanoseconds += 1;
        assert!(original.same_object(&changed));
        assert_ne!(original, changed);
    }
}
