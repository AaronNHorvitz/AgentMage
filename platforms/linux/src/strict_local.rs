//! Descriptor-held Linux strict-local data-root inspection.

use std::fmt;
use std::os::fd::AsRawFd;
use std::path::{Component, Path, PathBuf};

use agentmage_kernel_contracts::{
    CloudSynchronizationMarker, StorageFilesystemClass, StrictLocalStorageObservation,
};
use agentmage_kernel_engine::strict_local::{
    StrictLocalStorageDecision, StrictLocalStorageDenial, evaluate_storage,
};
use rustix::fd::OwnedFd;
use rustix::fs::{
    AtFlags, FileType, Mode, OFlags, StatxFlags, fstat, fstatfs, open, openat, statx,
};
use rustix::io::{Errno, fcntl_dupfd_cloexec};
use rustix::process::getuid;
use sha2::{Digest, Sha256};

const EXT_FAMILY_MAGIC: u64 = 0x0000_ef53;
const XFS_MAGIC: u64 = 0x5846_5342;
const BTRFS_MAGIC: u64 = 0x9123_683e;
const TMPFS_MAGIC: u64 = 0x0102_1994;
const RAMFS_MAGIC: u64 = 0x8584_58f6;
const F2FS_MAGIC: u64 = 0xf2f5_2010;
const EROFS_MAGIC: u64 = 0xe0f5_e1e2;
const ZFS_MAGIC: u64 = 0x2fc1_2fc1;
const MSDOS_MAGIC: u64 = 0x0000_4d44;
const EXFAT_MAGIC: u64 = 0x2011_bab0;
const NTFS3_MAGIC: u64 = 0x7366_746e;

const NFS_MAGIC: u64 = 0x0000_6969;
const CIFS_MAGIC: u64 = 0xff53_4d42;
const NINE_P_MAGIC: u64 = 0x0102_1997;
const AFS_MAGIC: u64 = 0x5346_414f;
const CEPH_MAGIC: u64 = 0x00c3_6400;
const NCP_MAGIC: u64 = 0x0000_564c;
const CODA_MAGIC: u64 = 0x7375_7245;
const FUSE_MAGIC: u64 = 0x6573_5546;

const ROOT_SENTINELS: [(&str, CloudSynchronizationMarker); 3] = [
    (".stfolder", CloudSynchronizationMarker::Syncthing),
    (".dropbox.cache", CloudSynchronizationMarker::Dropbox),
    (
        ".agentmage-cloud-synchronized",
        CloudSynchronizationMarker::OtherKnownMarker,
    ),
];

/// Stable failure class for Linux strict-local root inspection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxStrictLocalRootErrorKind {
    /// Empty paths are not inspectable roots.
    EmptyPath,
    /// Only an explicit absolute path can become a state root.
    RelativePath,
    /// The path contains a current, parent, or platform-prefix component.
    UnsafeComponent,
    /// A component cannot be conservatively matched against sync markers.
    NonUtf8Component,
    /// A symbolic-link component was encountered.
    SymbolicLink,
    /// A path component is not a directory.
    NotDirectory,
    /// A path component could not be opened.
    OpenFailed,
    /// Required descriptor or filesystem metadata was unavailable.
    MetadataUnavailable,
    /// The root is not owned by the current user.
    ForeignOwner,
    /// The root grants any group or other permission.
    UnsafeMode,
    /// The held root identity or filesystem changed.
    IdentityChanged,
    /// A known cloud-synchronization marker was observed.
    CloudSynchronized,
    /// The root resides on a network or remote filesystem.
    RemoteFilesystem,
    /// The root resides on a conservatively rejected userspace filesystem.
    FuseFilesystem,
    /// The root filesystem could not be admitted safely.
    UnknownFilesystem,
}

/// Content-free Linux strict-local root inspection failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxStrictLocalRootError {
    kind: LinuxStrictLocalRootErrorKind,
    component_index: Option<usize>,
}

impl LinuxStrictLocalRootError {
    /// Returns the stable failure class.
    #[must_use]
    pub const fn kind(self) -> LinuxStrictLocalRootErrorKind {
        self.kind
    }

    /// Returns the failing normal-component index without exposing its name.
    #[must_use]
    pub const fn component_index(self) -> Option<usize> {
        self.component_index
    }
}

impl fmt::Display for LinuxStrictLocalRootError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            LinuxStrictLocalRootErrorKind::EmptyPath => "strict_local_root.empty_path",
            LinuxStrictLocalRootErrorKind::RelativePath => "strict_local_root.relative_path",
            LinuxStrictLocalRootErrorKind::UnsafeComponent => "strict_local_root.unsafe_component",
            LinuxStrictLocalRootErrorKind::NonUtf8Component => {
                "strict_local_root.non_utf8_component"
            }
            LinuxStrictLocalRootErrorKind::SymbolicLink => "strict_local_root.symbolic_link",
            LinuxStrictLocalRootErrorKind::NotDirectory => "strict_local_root.not_directory",
            LinuxStrictLocalRootErrorKind::OpenFailed => "strict_local_root.open_failed",
            LinuxStrictLocalRootErrorKind::MetadataUnavailable => {
                "strict_local_root.metadata_unavailable"
            }
            LinuxStrictLocalRootErrorKind::ForeignOwner => "strict_local_root.foreign_owner",
            LinuxStrictLocalRootErrorKind::UnsafeMode => "strict_local_root.unsafe_mode",
            LinuxStrictLocalRootErrorKind::IdentityChanged => "strict_local_root.identity_changed",
            LinuxStrictLocalRootErrorKind::CloudSynchronized => {
                "strict_local_root.cloud_synchronized"
            }
            LinuxStrictLocalRootErrorKind::RemoteFilesystem => {
                "strict_local_root.remote_filesystem"
            }
            LinuxStrictLocalRootErrorKind::FuseFilesystem => "strict_local_root.fuse_filesystem",
            LinuxStrictLocalRootErrorKind::UnknownFilesystem => {
                "strict_local_root.unknown_filesystem"
            }
        })
    }
}

impl std::error::Error for LinuxStrictLocalRootError {}

/// Stateless Linux strict-local root inspector.
#[derive(Clone, Copy, Debug, Default)]
pub struct LinuxStrictLocalRootInspector;

impl LinuxStrictLocalRootInspector {
    /// Resolves one absolute directory without following symbolic links and holds its identity.
    pub fn inspect(path: &Path) -> Result<LinuxStrictLocalRoot, LinuxStrictLocalRootError> {
        if path.as_os_str().is_empty() {
            return Err(error(LinuxStrictLocalRootErrorKind::EmptyPath, None));
        }
        if !path.is_absolute() {
            return Err(error(LinuxStrictLocalRootErrorKind::RelativePath, None));
        }

        let mut descriptor = open(
            "/",
            OFlags::PATH | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| error(LinuxStrictLocalRootErrorKind::OpenFailed, None))?;
        let mut synchronization_marker = None;
        let mut normal_index = 0;

        for component in path.components() {
            match component {
                Component::RootDir => {}
                Component::Normal(name) => {
                    let name_text = name.to_str().ok_or_else(|| {
                        error(
                            LinuxStrictLocalRootErrorKind::NonUtf8Component,
                            Some(normal_index),
                        )
                    })?;
                    synchronization_marker = synchronization_marker
                        .or_else(|| synchronization_marker_for_component(name_text));
                    descriptor = open_directory_component(&descriptor, name, normal_index)?;
                    normal_index += 1;
                }
                Component::CurDir | Component::ParentDir | Component::Prefix(_) => {
                    return Err(error(
                        LinuxStrictLocalRootErrorKind::UnsafeComponent,
                        Some(normal_index),
                    ));
                }
            }
        }

        synchronization_marker = synchronization_marker.or(detect_root_sentinel(&descriptor)?);
        let metadata = root_metadata(&descriptor)?;
        if metadata.owner != getuid().as_raw() {
            return Err(error(LinuxStrictLocalRootErrorKind::ForeignOwner, None));
        }
        if metadata.mode & 0o077 != 0 {
            return Err(error(LinuxStrictLocalRootErrorKind::UnsafeMode, None));
        }
        let observation = StrictLocalStorageObservation {
            filesystem: classify_linux_filesystem_magic(metadata.filesystem_magic),
            synchronization_marker,
            root_identity_sha256: root_identity_digest(&metadata),
            symlink_free: true,
        };
        Ok(LinuxStrictLocalRoot {
            descriptor,
            metadata,
            observation,
        })
    }
}

/// A continuously held Linux data-root identity and content-free observation.
pub struct LinuxStrictLocalRoot {
    descriptor: OwnedFd,
    metadata: RootMetadata,
    observation: StrictLocalStorageObservation,
}

impl LinuxStrictLocalRoot {
    /// Returns the content-free observation for kernel policy evaluation.
    #[must_use]
    pub const fn observation(&self) -> &StrictLocalStorageObservation {
        &self.observation
    }

    /// Duplicates the held root descriptor for a later descriptor-relative store boundary.
    pub fn duplicate_descriptor(&self) -> Result<OwnedFd, LinuxStrictLocalRootError> {
        fcntl_dupfd_cloexec(&self.descriptor, 3)
            .map_err(|_| error(LinuxStrictLocalRootErrorKind::OpenFailed, None))
    }

    /// Opens the held directory for descriptor-relative I/O and directory synchronization.
    pub(crate) fn duplicate_io_descriptor(&self) -> Result<OwnedFd, LinuxStrictLocalRootError> {
        let descriptor = openat(
            &self.descriptor,
            ".",
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| error(LinuxStrictLocalRootErrorKind::OpenFailed, None))?;
        if root_metadata(&descriptor)? != self.metadata {
            return Err(error(LinuxStrictLocalRootErrorKind::IdentityChanged, None));
        }
        Ok(descriptor)
    }

    /// Revalidates object identity, filesystem class, and root-level sync sentinels.
    pub fn revalidate(&self) -> Result<(), LinuxStrictLocalRootError> {
        let current = root_metadata(&self.descriptor)?;
        if current != self.metadata
            || root_identity_digest(&current) != self.observation.root_identity_sha256
        {
            return Err(error(LinuxStrictLocalRootErrorKind::IdentityChanged, None));
        }
        match evaluate_storage(&self.observation) {
            StrictLocalStorageDecision::Eligible => {}
            StrictLocalStorageDecision::Reject { reason } => {
                return Err(error(storage_denial_kind(reason), None));
            }
        }
        if detect_root_sentinel(&self.descriptor)? != self.observation.synchronization_marker {
            return Err(error(LinuxStrictLocalRootErrorKind::IdentityChanged, None));
        }
        Ok(())
    }

    /// Returns the fixed authority-database path through the continuously held root.
    pub(crate) fn authority_database_path(&self) -> PathBuf {
        PathBuf::from(format!(
            "/proc/self/fd/{}/authority.db",
            self.descriptor.as_raw_fd()
        ))
    }
}

impl fmt::Debug for LinuxStrictLocalRoot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxStrictLocalRoot")
            .field("filesystem", &self.observation.filesystem)
            .field(
                "synchronization_marker",
                &self.observation.synchronization_marker,
            )
            .field("root_identity", &"sha256:[REDACTED]")
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RootMetadata {
    device: u64,
    inode: u64,
    mount_id: Option<u64>,
    filesystem_magic: u64,
    owner: u32,
    mode: u32,
}

fn open_directory_component(
    parent: &OwnedFd,
    name: &std::ffi::OsStr,
    component_index: usize,
) -> Result<OwnedFd, LinuxStrictLocalRootError> {
    let descriptor = openat(
        parent,
        name,
        OFlags::PATH | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|failure| open_error(failure, component_index))?;
    let stat = fstat(&descriptor).map_err(|_| {
        error(
            LinuxStrictLocalRootErrorKind::MetadataUnavailable,
            Some(component_index),
        )
    })?;
    match FileType::from_raw_mode(stat.st_mode) {
        FileType::Directory => Ok(descriptor),
        FileType::Symlink => Err(error(
            LinuxStrictLocalRootErrorKind::SymbolicLink,
            Some(component_index),
        )),
        _ => Err(error(
            LinuxStrictLocalRootErrorKind::NotDirectory,
            Some(component_index),
        )),
    }
}

fn root_metadata(descriptor: &OwnedFd) -> Result<RootMetadata, LinuxStrictLocalRootError> {
    let stat = fstat(descriptor)
        .map_err(|_| error(LinuxStrictLocalRootErrorKind::MetadataUnavailable, None))?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::Directory {
        return Err(error(LinuxStrictLocalRootErrorKind::NotDirectory, None));
    }
    let filesystem = fstatfs(descriptor)
        .map_err(|_| error(LinuxStrictLocalRootErrorKind::MetadataUnavailable, None))?;
    let mount_id = match statx(
        descriptor,
        "",
        AtFlags::EMPTY_PATH | AtFlags::NO_AUTOMOUNT,
        StatxFlags::MNT_ID,
    ) {
        Ok(observed)
            if StatxFlags::from_bits_retain(observed.stx_mask).contains(StatxFlags::MNT_ID) =>
        {
            Some(observed.stx_mnt_id)
        }
        Ok(_) | Err(Errno::NOSYS | Errno::INVAL | Errno::PERM | Errno::ACCESS) => None,
        Err(_) => {
            return Err(error(
                LinuxStrictLocalRootErrorKind::MetadataUnavailable,
                None,
            ));
        }
    };
    Ok(RootMetadata {
        device: stat.st_dev,
        inode: stat.st_ino,
        mount_id,
        filesystem_magic: filesystem.f_type as u64,
        owner: stat.st_uid,
        mode: stat.st_mode & 0o777,
    })
}

fn root_identity_digest(metadata: &RootMetadata) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(b"agentmage.strict-local-root.v1\0");
    digest.update(metadata.device.to_le_bytes());
    digest.update(metadata.inode.to_le_bytes());
    digest.update(metadata.filesystem_magic.to_le_bytes());
    digest.update(metadata.owner.to_le_bytes());
    digest.update(metadata.mode.to_le_bytes());
    match metadata.mount_id {
        Some(mount_id) => {
            digest.update([1]);
            digest.update(mount_id.to_le_bytes());
        }
        None => digest.update([0]),
    }
    digest.finalize().into()
}

fn detect_root_sentinel(
    descriptor: &OwnedFd,
) -> Result<Option<CloudSynchronizationMarker>, LinuxStrictLocalRootError> {
    for (name, marker) in ROOT_SENTINELS {
        match openat(
            descriptor,
            name,
            OFlags::PATH | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        ) {
            Ok(_) => return Ok(Some(marker)),
            Err(Errno::NOENT) => {}
            Err(_) => {
                return Err(error(
                    LinuxStrictLocalRootErrorKind::MetadataUnavailable,
                    None,
                ));
            }
        }
    }
    Ok(None)
}

fn synchronization_marker_for_component(name: &str) -> Option<CloudSynchronizationMarker> {
    let canonical: String = name
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|character| character.to_ascii_lowercase())
        .collect();
    if canonical.starts_with("dropbox") {
        Some(CloudSynchronizationMarker::Dropbox)
    } else if canonical.starts_with("onedrive") {
        Some(CloudSynchronizationMarker::OneDrive)
    } else if canonical.starts_with("googledrive") {
        Some(CloudSynchronizationMarker::GoogleDrive)
    } else if canonical.starts_with("nextcloud") {
        Some(CloudSynchronizationMarker::Nextcloud)
    } else if canonical.starts_with("owncloud") {
        Some(CloudSynchronizationMarker::OwnCloud)
    } else if canonical.starts_with("iclouddrive") || canonical == "mobiledocuments" {
        Some(CloudSynchronizationMarker::ICloudDrive)
    } else if canonical.starts_with("syncthing") {
        Some(CloudSynchronizationMarker::Syncthing)
    } else if matches!(
        canonical.as_str(),
        "box"
            | "boxdrive"
            | "boxsync"
            | "mega"
            | "megasync"
            | "pcloud"
            | "pclouddrive"
            | "protondrive"
            | "tresorit"
    ) {
        Some(CloudSynchronizationMarker::OtherKnownMarker)
    } else {
        None
    }
}

const fn storage_denial_kind(denial: StrictLocalStorageDenial) -> LinuxStrictLocalRootErrorKind {
    match denial {
        StrictLocalStorageDenial::InvalidRootIdentity | StrictLocalStorageDenial::SymbolicLink => {
            LinuxStrictLocalRootErrorKind::IdentityChanged
        }
        StrictLocalStorageDenial::CloudSynchronized => {
            LinuxStrictLocalRootErrorKind::CloudSynchronized
        }
        StrictLocalStorageDenial::RemoteFilesystem => {
            LinuxStrictLocalRootErrorKind::RemoteFilesystem
        }
        StrictLocalStorageDenial::FuseFilesystem => LinuxStrictLocalRootErrorKind::FuseFilesystem,
        StrictLocalStorageDenial::UnknownFilesystem => {
            LinuxStrictLocalRootErrorKind::UnknownFilesystem
        }
    }
}

/// Maps a Linux filesystem magic number to the closed strict-local class.
#[must_use]
pub const fn classify_linux_filesystem_magic(magic: u64) -> StorageFilesystemClass {
    match magic {
        EXT_FAMILY_MAGIC | XFS_MAGIC | BTRFS_MAGIC | TMPFS_MAGIC | RAMFS_MAGIC | F2FS_MAGIC
        | EROFS_MAGIC | ZFS_MAGIC | MSDOS_MAGIC | EXFAT_MAGIC | NTFS3_MAGIC => {
            StorageFilesystemClass::Local
        }
        NFS_MAGIC | CIFS_MAGIC | NINE_P_MAGIC | AFS_MAGIC | CEPH_MAGIC | NCP_MAGIC | CODA_MAGIC => {
            StorageFilesystemClass::Remote
        }
        FUSE_MAGIC => StorageFilesystemClass::Fuse,
        _ => StorageFilesystemClass::Unknown,
    }
}

fn open_error(failure: Errno, component_index: usize) -> LinuxStrictLocalRootError {
    let kind = match failure {
        Errno::LOOP => LinuxStrictLocalRootErrorKind::SymbolicLink,
        Errno::NOTDIR => LinuxStrictLocalRootErrorKind::NotDirectory,
        _ => LinuxStrictLocalRootErrorKind::OpenFailed,
    };
    error(kind, Some(component_index))
}

const fn error(
    kind: LinuxStrictLocalRootErrorKind,
    component_index: Option<usize>,
) -> LinuxStrictLocalRootError {
    LinuxStrictLocalRootError {
        kind,
        component_index,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::fd::AsRawFd;
    use std::os::unix::ffi::OsStringExt;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::{CloudSynchronizationMarker, StorageFilesystemClass};
    use agentmage_kernel_engine::strict_local::StrictLocalStorageDenial;

    use super::{
        AFS_MAGIC, BTRFS_MAGIC, CEPH_MAGIC, CIFS_MAGIC, CODA_MAGIC, EXFAT_MAGIC, EXT_FAMILY_MAGIC,
        F2FS_MAGIC, FUSE_MAGIC, LinuxStrictLocalRootErrorKind, LinuxStrictLocalRootInspector,
        MSDOS_MAGIC, NCP_MAGIC, NFS_MAGIC, NINE_P_MAGIC, NTFS3_MAGIC, RAMFS_MAGIC, TMPFS_MAGIC,
        XFS_MAGIC, ZFS_MAGIC, classify_linux_filesystem_magic, storage_denial_kind,
        synchronization_marker_for_component,
    };

    static TEMP_ID: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let id = TEMP_ID.fetch_add(1, Ordering::SeqCst);
            let path = std::env::temp_dir().join(format!(
                "agentmage-strict-local-{label}-{}-{id}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("test root creates");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
                .expect("test root is private");
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("test root removes");
        }
    }

    #[test]
    fn strict_local_state_root_filesystem_magic_matrix_is_closed_and_conservative() {
        for magic in [
            EXT_FAMILY_MAGIC,
            XFS_MAGIC,
            BTRFS_MAGIC,
            TMPFS_MAGIC,
            RAMFS_MAGIC,
            F2FS_MAGIC,
            ZFS_MAGIC,
            MSDOS_MAGIC,
            EXFAT_MAGIC,
            NTFS3_MAGIC,
        ] {
            assert_eq!(
                classify_linux_filesystem_magic(magic),
                StorageFilesystemClass::Local
            );
        }
        for magic in [
            NFS_MAGIC,
            CIFS_MAGIC,
            NINE_P_MAGIC,
            AFS_MAGIC,
            CEPH_MAGIC,
            NCP_MAGIC,
            CODA_MAGIC,
        ] {
            assert_eq!(
                classify_linux_filesystem_magic(magic),
                StorageFilesystemClass::Remote
            );
        }
        assert_eq!(
            classify_linux_filesystem_magic(FUSE_MAGIC),
            StorageFilesystemClass::Fuse
        );
        assert_eq!(
            classify_linux_filesystem_magic(0xdead_beef),
            StorageFilesystemClass::Unknown
        );
    }

    #[test]
    fn strict_local_state_root_provider_names_and_sentinels_are_rejected() {
        assert_eq!(
            synchronization_marker_for_component("Google Drive"),
            Some(CloudSynchronizationMarker::GoogleDrive)
        );
        assert_eq!(
            synchronization_marker_for_component("OneDrive - Example Organization"),
            Some(CloudSynchronizationMarker::OneDrive)
        );
        for name in [
            "Box Drive",
            "MEGAsync",
            "pCloud Drive",
            "Proton Drive",
            "Tresorit",
        ] {
            assert_eq!(
                synchronization_marker_for_component(name),
                Some(CloudSynchronizationMarker::OtherKnownMarker)
            );
        }
        assert_eq!(
            synchronization_marker_for_component("ordinary-project"),
            None
        );

        let test = TestDirectory::new("sentinel");
        fs::create_dir(test.0.join(".stfolder")).expect("sentinel creates");
        let held = LinuxStrictLocalRootInspector::inspect(&test.0).expect("root inspects");
        assert_eq!(
            held.observation().synchronization_marker,
            Some(CloudSynchronizationMarker::Syncthing)
        );
        assert!(!format!("{held:?}").contains(test.0.to_string_lossy().as_ref()));
        assert_eq!(
            held.revalidate()
                .expect_err("synchronized root rejects")
                .kind(),
            LinuxStrictLocalRootErrorKind::CloudSynchronized
        );

        let provider = TestDirectory::new("provider-parent");
        let synchronized = provider.0.join("Google Drive");
        fs::create_dir(&synchronized).expect("provider directory creates");
        fs::set_permissions(&synchronized, fs::Permissions::from_mode(0o700))
            .expect("provider directory private");
        let held =
            LinuxStrictLocalRootInspector::inspect(&synchronized).expect("provider root observes");
        assert_eq!(
            held.revalidate().expect_err("provider root rejects").kind(),
            LinuxStrictLocalRootErrorKind::CloudSynchronized
        );
    }

    #[test]
    fn strict_local_state_root_kernel_denials_map_to_exact_platform_refusals() {
        for (denial, expected) in [
            (
                StrictLocalStorageDenial::InvalidRootIdentity,
                LinuxStrictLocalRootErrorKind::IdentityChanged,
            ),
            (
                StrictLocalStorageDenial::SymbolicLink,
                LinuxStrictLocalRootErrorKind::IdentityChanged,
            ),
            (
                StrictLocalStorageDenial::CloudSynchronized,
                LinuxStrictLocalRootErrorKind::CloudSynchronized,
            ),
            (
                StrictLocalStorageDenial::RemoteFilesystem,
                LinuxStrictLocalRootErrorKind::RemoteFilesystem,
            ),
            (
                StrictLocalStorageDenial::FuseFilesystem,
                LinuxStrictLocalRootErrorKind::FuseFilesystem,
            ),
            (
                StrictLocalStorageDenial::UnknownFilesystem,
                LinuxStrictLocalRootErrorKind::UnknownFilesystem,
            ),
        ] {
            assert_eq!(storage_denial_kind(denial), expected);
        }
    }

    #[test]
    fn inspection_holds_and_revalidates_a_local_directory_identity() {
        let test = TestDirectory::new("local");
        let held = LinuxStrictLocalRootInspector::inspect(&test.0).expect("root inspects");
        assert!(matches!(
            held.observation().filesystem,
            StorageFilesystemClass::Local | StorageFilesystemClass::Unknown
        ));
        assert_ne!(held.observation().root_identity_sha256, [0; 32]);
        match held.observation().filesystem {
            StorageFilesystemClass::Local => held.revalidate().expect("held root revalidates"),
            StorageFilesystemClass::Unknown => assert_eq!(
                held.revalidate().expect_err("unknown root rejects").kind(),
                LinuxStrictLocalRootErrorKind::UnknownFilesystem
            ),
            _ => panic!("temporary test root unexpectedly used a non-local filesystem"),
        }
        let duplicate = held.duplicate_descriptor().expect("descriptor duplicates");
        assert!(duplicate.as_raw_fd() >= 3);
    }

    #[test]
    fn public_mode_is_rejected_and_a_mode_change_invalidates_a_held_root() {
        let test = TestDirectory::new("mode");
        fs::set_permissions(&test.0, fs::Permissions::from_mode(0o750)).expect("weaken root mode");
        assert_eq!(
            LinuxStrictLocalRootInspector::inspect(&test.0)
                .expect_err("public root rejects")
                .kind(),
            LinuxStrictLocalRootErrorKind::UnsafeMode
        );
        fs::set_permissions(&test.0, fs::Permissions::from_mode(0o700)).expect("restore root mode");
        let held = LinuxStrictLocalRootInspector::inspect(&test.0).expect("private root");
        fs::set_permissions(&test.0, fs::Permissions::from_mode(0o750))
            .expect("mutate held root mode");
        assert_eq!(
            held.revalidate().expect_err("mode mutation rejects").kind(),
            LinuxStrictLocalRootErrorKind::IdentityChanged
        );
    }

    #[test]
    fn symlink_and_non_utf8_components_fail_closed() {
        let test = TestDirectory::new("unsafe");
        let target = test.0.join("target");
        let link = test.0.join("link");
        fs::create_dir(&target).expect("target creates");
        symlink(&target, &link).expect("symlink creates");
        assert_eq!(
            LinuxStrictLocalRootInspector::inspect(&link)
                .expect_err("symlink rejects")
                .kind(),
            LinuxStrictLocalRootErrorKind::SymbolicLink
        );

        let invalid = std::ffi::OsString::from_vec(vec![0xff, 0xfe]);
        let invalid_path = test.0.join(invalid);
        fs::create_dir(&invalid_path).expect("non-UTF8 directory creates");
        assert_eq!(
            LinuxStrictLocalRootInspector::inspect(&invalid_path)
                .expect_err("non-UTF8 path rejects")
                .kind(),
            LinuxStrictLocalRootErrorKind::NonUtf8Component
        );
    }

    #[test]
    fn strict_local_state_root_adding_a_sync_sentinel_invalidates_the_held_root() {
        let test = TestDirectory::new("mutation");
        let held = LinuxStrictLocalRootInspector::inspect(&test.0).expect("root inspects");
        fs::create_dir(test.0.join(".stfolder")).expect("sentinel creates");
        assert_eq!(
            held.revalidate()
                .expect_err("sentinel change rejects")
                .kind(),
            LinuxStrictLocalRootErrorKind::IdentityChanged
        );
    }
}
