#![recursion_limit = "256"]
#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Fedora and Ubuntu platform path adapter.

mod command_runner;
mod configuration_store;
mod filesystem_control;
mod inventory;
mod ipc;
mod lifecycle;
mod local_commit;
mod platform;
mod repository_safety;
mod runtime_artifact_crypto;
mod runtime_artifact_store;
mod sandbox;
mod secret_service;
mod security_controls;
mod strict_local;
mod write_transaction;

pub use command_runner::{
    LinuxBoundedCommandExecutor, LinuxCommandManifest, LinuxCommandRunnerError,
    LinuxCommandRunnerErrorKind,
};
pub use configuration_store::{
    LinuxConfigurationEffectDriver, LinuxConfigurationEffectOutput,
    LinuxConfigurationEffectRequest, LinuxConfigurationError, LinuxConfigurationErrorKind,
    LinuxConfigurationStore, open_linux_configuration_store,
};
pub use filesystem_control::{LinuxControlledFilesystemDriver, LinuxFilesystemDriverLimits};
pub use inventory::{
    LinuxDeclaredListener, LinuxDeclaredNetworkRule, LinuxDeclaredProcess, LinuxDeclaredSocket,
    LinuxDeclaredTool, LinuxDeclaredWritable, LinuxInventoryError, LinuxInventoryErrorKind,
    LinuxInventoryScope, LinuxInventoryTarget, LinuxListenerBoundaryError,
    LinuxListenerBoundaryErrorKind, LinuxListenerBoundaryReceipt, LinuxProcessIdentityBinding,
    LinuxSessionBoundaryError, LinuxSessionBoundaryErrorKind, LinuxSessionBoundaryManifest,
    LinuxSessionBoundaryReport, LinuxSessionInventory, LinuxSessionInventoryCollector,
    LinuxSessionListenerPolicy, LinuxSessionProcessObservation, LinuxSocketObservation,
    LinuxSocketProtocol, LinuxSocketState, LinuxWritableObservation, LinuxWritableTargetClass,
};
pub use ipc::{
    LINUX_IPC_PROTOCOL_VERSION, LinuxAuthenticatedIpcSession, LinuxAuthenticatedPeer,
    LinuxHandshakeRequest, LinuxHostIpcEndpoint, LinuxIpcAuthenticator, LinuxIpcError,
    LinuxIpcErrorKind, LinuxLaunchCredentials, LinuxPeerIdentity, observe_linux_process_identity,
};
pub use lifecycle::{
    LinuxOperationalKeyLifecycleError, LinuxOperationalKeyLifecycleErrorKind,
    LinuxOperationalKeyProvisionReceipt, provision_linux_operational_key,
    rotate_linux_operational_key,
};
pub use local_commit::{
    LinuxLocalCommitError, LinuxLocalCommitErrorKind, LinuxLocalCommitExecutor, LinuxOpenPgpSigner,
};
pub use platform::{
    LinuxAuthorityOpenError, LinuxAuthorityRuntime, LinuxPlatformAdapter,
    LinuxPlatformDiscoveryError, LinuxPlatformDiscoveryErrorKind, open_linux_authority,
    open_linux_bootstrap_ipc, open_linux_host_ipc, resolve_linux_workspace_object,
    select_linux_workspace,
};
#[cfg(feature = "test-support")]
pub use platform::{
    open_test_linux_authority, resolve_test_linux_workspace_object, select_test_linux_workspace,
};
pub use repository_safety::{
    LinuxBoundedRepositoryInspectionExecutor, LinuxGitArtifact, LinuxRepositoryCollector,
    LinuxRepositoryError, LinuxRepositoryErrorKind, LinuxRepositoryExecutor, LinuxRepositoryScope,
    linux_repository_path_sha256,
};
pub use runtime_artifact_store::{LinuxRuntimeArtifactPayloadStore, LinuxRuntimeArtifactStaged};
pub use sandbox::{
    LinuxReadOnlyToolEffectDriver, LinuxReadOnlyToolInput, LinuxSandboxEffectDriver,
    LinuxSandboxError, LinuxSandboxErrorKind, LinuxSandboxLimits, LinuxSandboxManifest,
    LinuxSandboxOperation, LinuxSandboxResult, LinuxSandboxRunner, LinuxWorkerRuntimeFile,
};
pub use secret_service::{
    LinuxOperationalStoreKeyProvider, LinuxSecretEffectDriver, LinuxSecretEffectOutput,
    LinuxSecretEffectRequest, LinuxSecretKey, LinuxSecretOperation, LinuxSecretReceipt,
    LinuxSecretService, LinuxSecretServiceError, LinuxSecretServiceErrorKind,
    LinuxSecretServiceManifest, LinuxSecretValue,
};
pub use strict_local::{
    LinuxStrictLocalRoot, LinuxStrictLocalRootError, LinuxStrictLocalRootErrorKind,
    LinuxStrictLocalRootInspector, classify_linux_filesystem_magic,
};
pub use write_transaction::{LinuxAtomicWriteDriver, LinuxAtomicWriteDriverLimits};

use std::fmt;
use std::fs;
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path};

use agentmage_kernel_contracts::{
    AdapterInstanceId, AuthorizedWorkspaceHandle, DisplayFileLink, DisplayLinkErrorKind,
    FilePreimage, HeldWorkspaceObject, HeldWorkspaceRoot, PathAdapterError, PathAdapterErrorKind,
    PathPlatform, PathResolutionIntent, PlatformPathAdapter, WorkspaceAuthorizationId, WorkspaceId,
    WorkspaceObjectIdentity, WorkspaceObjectKind, WorkspacePath,
};
use rustix::fd::OwnedFd;
use rustix::fs::{
    AtFlags, Dir, FileType, Mode, OFlags, ResolveFlags, Stat, StatxFlags, fstat, open, openat,
    openat2, statx,
};
use rustix::io::{Errno, pread};
use sha2::{Digest, Sha256};

/// Stable component identity used by diagnostics and build verification.
pub const COMPONENT_ID: &str = "platform-linux";

/// Default maximum number of bytes included in one exact path preimage.
pub const DEFAULT_MAX_PREIMAGE_BYTES: u64 = 64 * 1024 * 1024;

const HASH_BUFFER_BYTES: usize = 64 * 1024;
/// Default and hard maximum number of names in one exact directory observation.
pub const MAX_DIRECTORY_OBSERVATION_NAMES: usize = 4_096;
/// Default and hard maximum UTF-8 bytes in one exact directory-name observation.
pub const MAX_DIRECTORY_OBSERVATION_BYTES: usize = 1024 * 1024;

const STRICT_RESOLVE_FLAGS: ResolveFlags = ResolveFlags::BENEATH
    .union(ResolveFlags::NO_SYMLINKS)
    .union(ResolveFlags::NO_MAGICLINKS)
    .union(ResolveFlags::NO_XDEV);

/// Returns the identity of the contracts implemented by this adapter.
#[must_use]
pub const fn contract_component_id() -> &'static str {
    agentmage_kernel_contracts::COMPONENT_ID
}

/// Returns the identity of the kernel mediation boundary used by effect drivers.
#[must_use]
pub const fn mediation_component_id() -> &'static str {
    agentmage_kernel_engine::COMPONENT_ID
}

/// Linux path adapter with an exact instance identity and bounded hash limit.
#[derive(Debug)]
pub struct LinuxPathAdapter {
    adapter_instance_id: AdapterInstanceId,
    max_preimage_bytes: u64,
    resolver_preference: ResolverPreference,
}

/// Security mechanism selected for one Linux path resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxResolutionStrategy {
    /// Linux `openat2` with beneath, no-symlink, no-magic-link, and no-mount-crossing rules.
    OpenAt2,
    /// Descriptor-relative component walk with verified `statx` mount identities.
    VerifiedDescriptorWalk,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResolverPreference {
    Auto,
    VerifiedDescriptorWalk,
}

impl LinuxPathAdapter {
    /// Creates an unprivileged Linux path adapter without authorizing a workspace.
    #[must_use]
    pub const fn new(adapter_instance_id: AdapterInstanceId, max_preimage_bytes: u64) -> Self {
        Self {
            adapter_instance_id,
            max_preimage_bytes,
            resolver_preference: ResolverPreference::Auto,
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
    root_identity: WorkspaceObjectIdentity,
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

impl HeldWorkspaceRoot for LinuxAuthorizedWorkspace {
    fn root_identity(&self) -> &WorkspaceObjectIdentity {
        &self.root_identity
    }
}

impl LinuxAuthorizedWorkspace {
    /// Revalidates the continuously held workspace-root descriptor and identity.
    pub fn revalidate(&self) -> Result<(), PathAdapterError> {
        let root_now = snapshot(&self.root_descriptor, None)?;
        if !self.root_snapshot.same_object(&root_now) || self.root_snapshot.mode != root_now.mode {
            return Err(adapter_error(PathAdapterErrorKind::MountChanged, None));
        }
        Ok(())
    }

    pub(crate) fn reopen_root_directory(&self) -> Result<OwnedFd, PathAdapterError> {
        self.revalidate()?;
        let descriptor = openat(
            &self.root_descriptor,
            ".",
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| adapter_error(PathAdapterErrorKind::PlatformFailure, None))?;
        let observed = snapshot(&descriptor, None)?;
        if !self.root_snapshot.same_object(&observed) || self.root_snapshot.mode != observed.mode {
            return Err(adapter_error(PathAdapterErrorKind::MountChanged, None));
        }
        Ok(descriptor)
    }

    /// Observes a bounded sorted name projection from the continuously held root.
    pub fn observe_root_names(
        &self,
        maximum_names: usize,
        maximum_name_bytes: usize,
    ) -> Result<Vec<String>, PathAdapterError> {
        let descriptor = self.reopen_root_directory()?;
        let names = observe_directory_names(&descriptor, maximum_names, maximum_name_bytes)?;
        self.revalidate()?;
        Ok(names)
    }
}

fn authorize_workspace_root(
    root: &Path,
    workspace_id: WorkspaceId,
    authorization_id: WorkspaceAuthorizationId,
    adapter_instance_id: AdapterInstanceId,
) -> Result<LinuxAuthorizedWorkspace, PathAdapterError> {
    if !root.is_absolute() {
        return Err(adapter_error(PathAdapterErrorKind::UnsafeComponent, None));
    }
    let mut descriptor = open(
        "/",
        OFlags::PATH | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| adapter_error(PathAdapterErrorKind::PlatformFailure, None))?;
    let mut index = 0;
    for component in root.components() {
        match component {
            Component::RootDir => {}
            Component::Normal(name) => {
                descriptor = openat(
                    &descriptor,
                    name,
                    OFlags::PATH | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    Mode::empty(),
                )
                .map_err(|error| open_error(error, index))?;
                index += 1;
            }
            Component::CurDir | Component::ParentDir | Component::Prefix(_) => {
                return Err(adapter_error(
                    PathAdapterErrorKind::UnsafeComponent,
                    Some(index),
                ));
            }
        }
    }
    let root_snapshot = snapshot(&descriptor, None)?;
    if root_snapshot.file_type != FileType::Directory {
        return Err(adapter_error(
            PathAdapterErrorKind::ObjectKindMismatch,
            None,
        ));
    }
    let root_identity = workspace_root_identity(&root_snapshot);
    Ok(LinuxAuthorizedWorkspace {
        workspace_id,
        authorization_id,
        adapter_instance_id,
        root_descriptor: descriptor,
        root_snapshot,
        root_identity,
    })
}

fn workspace_root_identity(snapshot: &LinuxStatSnapshot) -> WorkspaceObjectIdentity {
    WorkspaceObjectIdentity::new(
        PathPlatform::Linux,
        identity_digest(b"agentmage.linux.mount.v1", snapshot),
        identity_digest(b"agentmage.linux.workspace-root.v1", snapshot),
    )
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
    resolution_strategy: LinuxResolutionStrategy,
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
            .field("resolution_strategy", &self.resolution_strategy)
            .finish_non_exhaustive()
    }
}

impl LinuxHeldObject {
    /// Returns the mechanism that enforced this path resolution.
    #[must_use]
    pub const fn resolution_strategy(&self) -> LinuxResolutionStrategy {
        self.resolution_strategy
    }

    /// Generates a non-authoritative absolute file link after held-object revalidation.
    pub fn display_link(&self, line: Option<u32>) -> Result<DisplayFileLink, PathAdapterError> {
        if line == Some(0) {
            return Err(adapter_error(PathAdapterErrorKind::UnsafeComponent, None));
        }
        self.revalidate()?;
        let descriptor_link = format!("/proc/self/fd/{}", self.object_descriptor.as_raw_fd());
        let absolute = fs::read_link(descriptor_link)
            .map_err(|_| adapter_error(PathAdapterErrorKind::PlatformFailure, None))?;
        if !absolute.is_absolute() {
            return Err(adapter_error(PathAdapterErrorKind::PlatformFailure, None));
        }
        let file_uri = encode_file_uri(&absolute);
        DisplayFileLink::new(file_uri, line, self.object_identity.clone()).map_err(|error| {
            let kind = if error.kind() == DisplayLinkErrorKind::OversizedUri {
                PathAdapterErrorKind::ResourceLimitExceeded
            } else {
                PathAdapterErrorKind::PlatformFailure
            };
            adapter_error(kind, None)
        })
    }

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

    /// Reads the complete exact held-file preimage through its descriptor.
    pub fn read_exact_bytes(&self) -> Result<Vec<u8>, PathAdapterError> {
        if self.object_kind != WorkspaceObjectKind::RegularFile
            || !matches!(
                self.intent,
                PathResolutionIntent::ReadFile | PathResolutionIntent::ContentHash
            )
        {
            return Err(adapter_error(
                PathAdapterErrorKind::ObjectKindMismatch,
                None,
            ));
        }
        self.revalidate()?;
        let expected = self
            .preimage
            .as_ref()
            .ok_or_else(|| adapter_error(PathAdapterErrorKind::IdentityChanged, None))?;
        let length = usize::try_from(expected.byte_len())
            .map_err(|_| adapter_error(PathAdapterErrorKind::ResourceLimitExceeded, None))?;
        let mut bytes = vec![0_u8; length];
        let mut offset = 0_usize;
        while offset < bytes.len() {
            let count = pread(&self.object_descriptor, &mut bytes[offset..], offset as u64)
                .map_err(|_| adapter_error(PathAdapterErrorKind::PlatformFailure, None))?;
            if count == 0 {
                return Err(adapter_error(PathAdapterErrorKind::IdentityChanged, None));
            }
            offset = offset
                .checked_add(count)
                .ok_or_else(|| adapter_error(PathAdapterErrorKind::ResourceLimitExceeded, None))?;
        }
        let observed: [u8; 32] = Sha256::digest(&bytes).into();
        if &observed != expected.content_sha256() {
            return Err(adapter_error(PathAdapterErrorKind::IdentityChanged, None));
        }
        self.revalidate()?;
        Ok(bytes)
    }

    /// Observes a bounded sorted name projection from one held directory descriptor.
    pub fn observe_directory_names(
        &self,
        maximum_names: usize,
        maximum_name_bytes: usize,
    ) -> Result<Vec<String>, PathAdapterError> {
        if self.object_kind != WorkspaceObjectKind::Directory
            || self.intent != PathResolutionIntent::ReadDirectory
        {
            return Err(adapter_error(
                PathAdapterErrorKind::ObjectKindMismatch,
                None,
            ));
        }
        self.revalidate()?;
        let names =
            observe_directory_names(&self.object_descriptor, maximum_names, maximum_name_bytes)?;
        self.revalidate()?;
        Ok(names)
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
        let resolution_strategy = select_strategy(
            &workspace.root_descriptor,
            &root_now,
            self.resolver_preference,
        )?;
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
            let next = open_relative(
                resolution_strategy,
                &current_directory,
                component.as_str(),
                OFlags::PATH | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                index,
            )?;
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
            if !same_mount(&root_now, &next_snapshot, resolution_strategy) {
                return Err(adapter_error(
                    PathAdapterErrorKind::MountChanged,
                    Some(index),
                ));
            }
            current_directory = next;
        }

        let final_component = &path.components()[last_index];
        let candidate = open_relative(
            resolution_strategy,
            &current_directory,
            final_component.as_str(),
            OFlags::PATH | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            last_index,
        )?;
        let candidate_snapshot = snapshot(&candidate, Some(last_index))?;
        if candidate_snapshot.file_type == FileType::Symlink {
            return Err(adapter_error(
                PathAdapterErrorKind::SymbolicLink,
                Some(last_index),
            ));
        }
        if !same_mount(&root_now, &candidate_snapshot, resolution_strategy) {
            return Err(adapter_error(
                PathAdapterErrorKind::MountChanged,
                Some(last_index),
            ));
        }
        let object_kind = admitted_kind(&candidate_snapshot, intent, last_index)?;

        let object_descriptor = match intent {
            PathResolutionIntent::Metadata => candidate,
            PathResolutionIntent::ReadDirectory => reopen_and_compare(
                resolution_strategy,
                &current_directory,
                final_component.as_str(),
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                &candidate_snapshot,
                last_index,
            )?,
            PathResolutionIntent::ReadFile | PathResolutionIntent::ContentHash => {
                reopen_and_compare(
                    resolution_strategy,
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
            resolution_strategy,
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
    mount_id: Option<u64>,
}

impl LinuxStatSnapshot {
    fn from_stat(stat: &Stat, mount_id: Option<u64>) -> Self {
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
            mount_id,
        }
    }

    fn same_object(&self, other: &Self) -> bool {
        self.device == other.device
            && self.inode == other.inode
            && self.file_type == other.file_type
            && self.mount_id == other.mount_id
    }
}

fn snapshot(
    descriptor: &OwnedFd,
    component_index: Option<usize>,
) -> Result<LinuxStatSnapshot, PathAdapterError> {
    let stat = fstat(descriptor)
        .map_err(|_| adapter_error(PathAdapterErrorKind::PlatformFailure, component_index))?;
    let mount_id = descriptor_mount_id(descriptor, component_index)?;
    Ok(LinuxStatSnapshot::from_stat(&stat, mount_id))
}

fn descriptor_mount_id(
    descriptor: &OwnedFd,
    component_index: Option<usize>,
) -> Result<Option<u64>, PathAdapterError> {
    match statx(
        descriptor,
        "",
        AtFlags::EMPTY_PATH | AtFlags::NO_AUTOMOUNT,
        StatxFlags::MNT_ID,
    ) {
        Ok(observed) => Ok(StatxFlags::from_bits_retain(observed.stx_mask)
            .contains(StatxFlags::MNT_ID)
            .then_some(observed.stx_mnt_id)),
        Err(Errno::NOSYS | Errno::INVAL | Errno::PERM | Errno::ACCESS) => Ok(None),
        Err(_) => Err(adapter_error(
            PathAdapterErrorKind::PlatformFailure,
            component_index,
        )),
    }
}

fn select_strategy(
    root: &OwnedFd,
    root_snapshot: &LinuxStatSnapshot,
    preference: ResolverPreference,
) -> Result<LinuxResolutionStrategy, PathAdapterError> {
    if preference == ResolverPreference::VerifiedDescriptorWalk {
        return verified_fallback(root_snapshot);
    }

    match openat2(
        root,
        ".",
        OFlags::PATH | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
        STRICT_RESOLVE_FLAGS,
    ) {
        Ok(probe) => {
            let probe_snapshot = snapshot(&probe, None)?;
            if !root_snapshot.same_object(&probe_snapshot) {
                return Err(adapter_error(PathAdapterErrorKind::MountChanged, None));
            }
            Ok(LinuxResolutionStrategy::OpenAt2)
        }
        Err(Errno::NOSYS | Errno::INVAL | Errno::PERM | Errno::ACCESS) => Err(adapter_error(
            PathAdapterErrorKind::UnsupportedPrimitive,
            None,
        )),
        Err(_) => Err(adapter_error(PathAdapterErrorKind::PlatformFailure, None)),
    }
}

pub(crate) fn strict_descriptor_paths_available() -> bool {
    let root = match open(
        "/",
        OFlags::PATH | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(root) => root,
        Err(_) => return false,
    };
    let root_snapshot = match snapshot(&root, None) {
        Ok(snapshot) => snapshot,
        Err(_) => return false,
    };
    matches!(
        select_strategy(&root, &root_snapshot, ResolverPreference::Auto),
        Ok(LinuxResolutionStrategy::OpenAt2)
    )
}

fn verified_fallback(
    root_snapshot: &LinuxStatSnapshot,
) -> Result<LinuxResolutionStrategy, PathAdapterError> {
    root_snapshot
        .mount_id
        .map(|_| LinuxResolutionStrategy::VerifiedDescriptorWalk)
        .ok_or_else(|| adapter_error(PathAdapterErrorKind::UnsupportedPrimitive, None))
}

fn open_relative(
    strategy: LinuxResolutionStrategy,
    parent: &OwnedFd,
    component: &str,
    flags: OFlags,
    component_index: usize,
) -> Result<OwnedFd, PathAdapterError> {
    let result = match strategy {
        LinuxResolutionStrategy::OpenAt2 => openat2(
            parent,
            component,
            flags,
            Mode::empty(),
            STRICT_RESOLVE_FLAGS,
        ),
        LinuxResolutionStrategy::VerifiedDescriptorWalk => {
            openat(parent, component, flags, Mode::empty())
        }
    };
    result.map_err(|error| {
        if strategy == LinuxResolutionStrategy::OpenAt2
            && matches!(error, Errno::NOSYS | Errno::INVAL | Errno::PERM)
        {
            adapter_error(
                PathAdapterErrorKind::UnsupportedPrimitive,
                Some(component_index),
            )
        } else {
            open_error(error, component_index)
        }
    })
}

fn same_mount(
    root: &LinuxStatSnapshot,
    candidate: &LinuxStatSnapshot,
    strategy: LinuxResolutionStrategy,
) -> bool {
    match (root.mount_id, candidate.mount_id) {
        (Some(root_id), Some(candidate_id)) => root_id == candidate_id,
        (None, None) => {
            strategy == LinuxResolutionStrategy::OpenAt2 && root.device == candidate.device
        }
        _ => false,
    }
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
    strategy: LinuxResolutionStrategy,
    parent: &OwnedFd,
    component: &str,
    flags: OFlags,
    candidate: &LinuxStatSnapshot,
    component_index: usize,
) -> Result<OwnedFd, PathAdapterError> {
    let descriptor = open_relative(strategy, parent, component, flags, component_index)?;
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

fn observe_directory_names(
    descriptor: &OwnedFd,
    maximum_names: usize,
    maximum_name_bytes: usize,
) -> Result<Vec<String>, PathAdapterError> {
    if maximum_names == 0
        || maximum_names > MAX_DIRECTORY_OBSERVATION_NAMES
        || maximum_name_bytes == 0
        || maximum_name_bytes > MAX_DIRECTORY_OBSERVATION_BYTES
    {
        return Err(adapter_error(
            PathAdapterErrorKind::ResourceLimitExceeded,
            None,
        ));
    }
    let entries = Dir::read_from(descriptor)
        .map_err(|_| adapter_error(PathAdapterErrorKind::PlatformFailure, None))?;
    let mut names = Vec::new();
    let mut total_bytes = 0_usize;
    for entry in entries {
        let entry =
            entry.map_err(|_| adapter_error(PathAdapterErrorKind::PlatformFailure, None))?;
        let bytes = entry.file_name().to_bytes();
        if matches!(bytes, b"." | b"..") {
            continue;
        }
        if names.len() >= maximum_names {
            return Err(adapter_error(
                PathAdapterErrorKind::ResourceLimitExceeded,
                None,
            ));
        }
        total_bytes = total_bytes
            .checked_add(bytes.len())
            .filter(|value| *value <= maximum_name_bytes)
            .ok_or_else(|| adapter_error(PathAdapterErrorKind::ResourceLimitExceeded, None))?;
        names.push(
            std::str::from_utf8(bytes)
                .map_err(|_| adapter_error(PathAdapterErrorKind::UnsafeComponent, None))?
                .to_owned(),
        );
    }
    names.sort_unstable();
    if names.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(adapter_error(PathAdapterErrorKind::PlatformFailure, None));
    }
    Ok(names)
}

fn identity_digest(domain: &[u8], snapshot: &LinuxStatSnapshot) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update((domain.len() as u64).to_be_bytes());
    digest.update(domain);
    digest.update(snapshot.device.to_be_bytes());
    digest.update(snapshot.inode.to_be_bytes());
    digest.update(snapshot.mode.to_be_bytes());
    match snapshot.mount_id {
        Some(mount_id) => {
            digest.update([1]);
            digest.update(mount_id.to_be_bytes());
        }
        None => digest.update([0]),
    }
    digest.finalize().into()
}

fn encode_file_uri(path: &Path) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let bytes = path.as_os_str().as_bytes();
    let mut encoded = String::with_capacity(7 + bytes.len());
    encoded.push_str("file://");
    for byte in bytes {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(*byte));
        } else {
            encoded.push('%');
            encoded.push(char::from(HEX[(byte >> 4) as usize]));
            encoded.push(char::from(HEX[(byte & 0x0f) as usize]));
        }
    }
    encoded
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
    use std::process::Command;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::thread;

    use agentmage_kernel_contracts::{
        AdapterInstanceId, HeldWorkspaceObject, HeldWorkspaceRoot, PathAdapterErrorKind,
        PathResolutionIntent, PlatformPathAdapter, WorkspaceAuthorizationId, WorkspaceId,
        WorkspaceObjectKind, WorkspacePath, WorkspacePathErrorKind,
    };
    use rustix::fs::{Mode, OFlags, open, openat2};
    use rustix::io::Errno;
    use sha2::{Digest, Sha256};

    use super::{
        COMPONENT_ID, DEFAULT_MAX_PREIMAGE_BYTES, LinuxAuthorizedWorkspace, LinuxPathAdapter,
        LinuxResolutionStrategy, LinuxStatSnapshot, ResolverPreference, STRICT_RESOLVE_FLAGS,
        contract_component_id, mediation_component_id, same_mount, select_strategy,
        verified_fallback,
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

    fn fallback_adapter() -> LinuxPathAdapter {
        LinuxPathAdapter {
            adapter_instance_id: AdapterInstanceId::from_raw("adapter-linux-0001"),
            max_preimage_bytes: DEFAULT_MAX_PREIMAGE_BYTES,
            resolver_preference: ResolverPreference::VerifiedDescriptorWalk,
        }
    }

    fn authorize_for_test(root: &Path) -> LinuxAuthorizedWorkspace {
        let root_descriptor = open(
            root,
            OFlags::PATH | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .expect("test root opens");
        let root_snapshot = super::snapshot(&root_descriptor, None).expect("test root stats");
        let root_identity = super::workspace_root_identity(&root_snapshot);
        LinuxAuthorizedWorkspace {
            workspace_id: WorkspaceId::from_raw("workspace-0001"),
            authorization_id: WorkspaceAuthorizationId::from_raw("authorization-0001"),
            adapter_instance_id: AdapterInstanceId::from_raw("adapter-linux-0001"),
            root_descriptor,
            root_snapshot,
            root_identity,
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
    fn adapter_depends_inward_on_contracts_and_kernel_mediation() {
        assert_eq!(COMPONENT_ID, "platform-linux");
        assert_eq!(contract_component_id(), "kernel-contracts");
        assert_eq!(mediation_component_id(), "kernel-engine");
    }

    #[test]
    fn logical_fixture_has_equivalent_linux_policy_semantics() {
        let fixture = include_bytes!("../../../fixtures/paths/v1/logical-input.txt");
        let test = TestDirectory::new();
        let root = test.path.join("workspace");
        fs::create_dir_all(root.join("docs")).expect("logical fixture directory creates");
        fs::write(root.join("docs/logical-input.txt"), fixture).expect("logical fixture writes");
        let workspace = authorize_for_test(&root);
        let candidate = path(&["docs", "logical-input.txt"]);
        let held = adapter()
            .resolve(&workspace, &candidate, PathResolutionIntent::ContentHash)
            .expect("Linux adapter admits the shared logical fixture");
        let expected: [u8; 32] = Sha256::digest(fixture).into();
        assert_eq!(held.workspace_path(), &candidate);
        assert_eq!(held.intent(), PathResolutionIntent::ContentHash);
        assert_eq!(held.object_kind(), WorkspaceObjectKind::RegularFile);
        assert_eq!(
            held.object_identity().platform(),
            agentmage_kernel_contracts::PathPlatform::Linux
        );
        assert_eq!(held.preimage().expect("exact preimage").byte_len(), 28);
        assert_eq!(
            held.preimage().expect("exact preimage").content_sha256(),
            &expected
        );
        held.revalidate().expect("logical fixture remains current");
    }

    #[test]
    fn held_workspace_root_survives_child_changes_and_retains_exact_identity() {
        let test = TestDirectory::new();
        let root = test.path.join("workspace");
        fs::create_dir(&root).expect("workspace creates");
        let workspace = authorize_for_test(&root);
        let identity = workspace.root_identity().clone();

        fs::write(root.join("new-child.txt"), b"changed contents").expect("child write succeeds");

        workspace.revalidate().expect("held root remains current");
        assert_eq!(workspace.root_identity(), &identity);
    }

    #[test]
    fn held_workspace_root_observes_sorted_bounded_child_names() {
        let test = TestDirectory::new();
        let root = test.path.join("workspace");
        fs::create_dir(&root).expect("workspace creates");
        fs::write(root.join("zeta.txt"), b"zeta\n").expect("zeta fixture writes");
        fs::create_dir(root.join("alpha")).expect("alpha fixture creates");
        let workspace = authorize_for_test(&root);

        assert_eq!(
            workspace
                .observe_root_names(2, 32)
                .expect("root observation succeeds"),
            ["alpha".to_owned(), "zeta.txt".to_owned()]
        );
        let error = workspace
            .observe_root_names(1, 32)
            .expect_err("name limit rejects complete observation");
        assert_eq!(error.kind(), PathAdapterErrorKind::ResourceLimitExceeded);
        let error = workspace
            .observe_root_names(2, 0)
            .expect_err("zero byte limit rejects");
        assert_eq!(error.kind(), PathAdapterErrorKind::ResourceLimitExceeded);
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
        assert!(matches!(
            held.resolution_strategy(),
            LinuxResolutionStrategy::OpenAt2 | LinuxResolutionStrategy::VerifiedDescriptorWalk
        ));

        fs::rename(root.join("docs/input.txt"), root.join("docs/moved.txt"))
            .expect("held file renames");
        fs::write(root.join("docs/input.txt"), b"replacement bytes\n").expect("replacement writes");
        held.revalidate()
            .expect("held descriptor remains the validated original object");
    }

    #[test]
    fn held_file_reads_exact_bytes_and_rejects_post_resolution_mutation() {
        let test = TestDirectory::new();
        let root = test.path.join("workspace");
        fs::create_dir(&root).expect("workspace creates");
        let original = b"exact held bytes\n";
        fs::write(root.join("input.txt"), original).expect("fixture writes");
        let workspace = authorize_for_test(&root);
        let held = adapter()
            .resolve(
                &workspace,
                &path(&["input.txt"]),
                PathResolutionIntent::ContentHash,
            )
            .expect("held file resolves");

        assert_eq!(held.read_exact_bytes().expect("exact bytes read"), original);
        fs::write(root.join("input.txt"), b"mutated held bytes\n").expect("fixture mutates");
        let error = held
            .read_exact_bytes()
            .expect_err("mutation invalidates exact read");
        assert_eq!(error.kind(), PathAdapterErrorKind::IdentityChanged);
    }

    #[test]
    fn held_directory_observes_names_and_rejects_file_read_semantics() {
        let test = TestDirectory::new();
        let root = test.path.join("workspace");
        fs::create_dir_all(root.join("docs")).expect("fixture directories create");
        fs::write(root.join("docs/beta.txt"), b"beta\n").expect("beta fixture writes");
        fs::write(root.join("docs/alpha.txt"), b"alpha\n").expect("alpha fixture writes");
        let workspace = authorize_for_test(&root);
        let held = adapter()
            .resolve(
                &workspace,
                &path(&["docs"]),
                PathResolutionIntent::ReadDirectory,
            )
            .expect("held directory resolves");

        assert_eq!(
            held.observe_directory_names(2, 32)
                .expect("directory observation succeeds"),
            ["alpha.txt".to_owned(), "beta.txt".to_owned()]
        );
        let error = held
            .read_exact_bytes()
            .expect_err("directory cannot be read as a file");
        assert_eq!(error.kind(), PathAdapterErrorKind::ObjectKindMismatch);
    }

    #[test]
    fn held_directory_observation_rejects_post_resolution_mutation() {
        let test = TestDirectory::new();
        let root = test.path.join("workspace");
        fs::create_dir_all(root.join("docs")).expect("fixture directories create");
        fs::write(root.join("docs/original.txt"), b"original\n").expect("fixture writes");
        let workspace = authorize_for_test(&root);
        let held = adapter()
            .resolve(
                &workspace,
                &path(&["docs"]),
                PathResolutionIntent::ReadDirectory,
            )
            .expect("held directory resolves");

        fs::write(root.join("docs/added.txt"), b"added\n").expect("directory mutates");
        let error = held
            .observe_directory_names(8, 128)
            .expect_err("directory mutation invalidates observation");
        assert_eq!(error.kind(), PathAdapterErrorKind::IdentityChanged);
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
    fn verified_mount_id_fallback_is_explicit_and_missing_mount_id_fails_closed() {
        let test = TestDirectory::new();
        let root = test.path.join("workspace");
        fs::create_dir(&root).expect("workspace creates");
        fs::write(root.join("input.txt"), b"fallback\n").expect("fixture writes");
        symlink("input.txt", root.join("link.txt")).expect("fallback symlink creates");
        let workspace = authorize_for_test(&root);
        let held = fallback_adapter()
            .resolve(
                &workspace,
                &path(&["input.txt"]),
                PathResolutionIntent::ReadFile,
            )
            .expect("verified descriptor fallback resolves");
        assert_eq!(
            held.resolution_strategy(),
            LinuxResolutionStrategy::VerifiedDescriptorWalk
        );
        let symlink_error = fallback_adapter()
            .resolve(
                &workspace,
                &path(&["link.txt"]),
                PathResolutionIntent::Metadata,
            )
            .expect_err("fallback symlink rejects");
        assert_eq!(symlink_error.kind(), PathAdapterErrorKind::SymbolicLink);

        let mut unavailable = workspace.root_snapshot.clone();
        unavailable.mount_id = None;
        let error = verified_fallback(&unavailable).expect_err("unverified fallback rejects");
        assert_eq!(error.kind(), PathAdapterErrorKind::UnsupportedPrimitive);
        assert_eq!(error.component_index(), None);
    }

    #[test]
    fn automatic_strategy_matches_strict_openat2_probe_and_mount_ids_cannot_drift() {
        let test = TestDirectory::new();
        let root = test.path.join("workspace");
        fs::create_dir(&root).expect("workspace creates");
        let workspace = authorize_for_test(&root);
        let direct_probe = openat2(
            &workspace.root_descriptor,
            ".",
            OFlags::PATH | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
            STRICT_RESOLVE_FLAGS,
        );
        let selected = select_strategy(
            &workspace.root_descriptor,
            &workspace.root_snapshot,
            ResolverPreference::Auto,
        );
        match direct_probe {
            Ok(_) => assert_eq!(
                selected.expect("supported openat2 selects"),
                LinuxResolutionStrategy::OpenAt2
            ),
            Err(Errno::NOSYS | Errno::INVAL | Errno::PERM | Errno::ACCESS) => assert_eq!(
                selected
                    .expect_err("unsupported openat2 fails closed")
                    .kind(),
                PathAdapterErrorKind::UnsupportedPrimitive
            ),
            Err(_) => assert_eq!(
                selected
                    .expect_err("failed openat2 probe fails closed")
                    .kind(),
                PathAdapterErrorKind::PlatformFailure
            ),
        }

        let root_snapshot = workspace.root_snapshot.clone();
        let mut changed_mount = root_snapshot.clone();
        changed_mount.mount_id = changed_mount.mount_id.map(|mount_id| mount_id + 1);
        assert!(!same_mount(
            &root_snapshot,
            &changed_mount,
            LinuxResolutionStrategy::VerifiedDescriptorWalk,
        ));
        assert!(!same_mount(
            &root_snapshot,
            &changed_mount,
            LinuxResolutionStrategy::OpenAt2,
        ));
    }

    #[test]
    fn concurrent_symlink_replacement_never_changes_held_file_authority() {
        let test = TestDirectory::new();
        let root = test.path.join("workspace");
        fs::create_dir(&root).expect("workspace creates");
        let outside = test.path.join("outside-secret.txt");
        fs::write(&outside, b"outside secret bytes\n").expect("outside fixture writes");
        let safe = b"inside workspace bytes\n";
        fs::write(root.join("target.txt"), safe).expect("inside fixture writes");
        let workspace = authorize_for_test(&root);
        let candidate = path(&["target.txt"]);
        let expected: [u8; 32] = Sha256::digest(safe).into();

        let initial = adapter()
            .resolve(&workspace, &candidate, PathResolutionIntent::ContentHash)
            .expect("initial safe target resolves");
        assert_eq!(
            initial
                .preimage()
                .expect("initial preimage")
                .content_sha256(),
            &expected
        );

        let complete = Arc::new(AtomicBool::new(false));
        let attacker_complete = Arc::clone(&complete);
        let attacker_root = root.clone();
        let attacker_outside = outside.clone();
        let attacker = thread::spawn(move || {
            for index in 0..256 {
                let safe_staging = attacker_root.join(format!(".safe-{index}"));
                fs::write(&safe_staging, safe).expect("safe staging writes");
                fs::rename(&safe_staging, attacker_root.join("target.txt"))
                    .expect("safe staging replaces target");

                let link_staging = attacker_root.join(format!(".link-{index}"));
                symlink(&attacker_outside, &link_staging).expect("link staging creates");
                fs::rename(&link_staging, attacker_root.join("target.txt"))
                    .expect("link staging replaces target");
            }
            attacker_complete.store(true, Ordering::Release);
        });

        let mut attempts = 0_u32;
        let mut admitted_safe = 0_u32;
        while attempts < 512 || !complete.load(Ordering::Acquire) {
            attempts += 1;
            match adapter().resolve(&workspace, &candidate, PathResolutionIntent::ContentHash) {
                Ok(held) => {
                    assert_eq!(
                        held.preimage().expect("held preimage").content_sha256(),
                        &expected
                    );
                    if let Err(error) = held.revalidate() {
                        assert_eq!(error.kind(), PathAdapterErrorKind::IdentityChanged);
                    }
                    admitted_safe += 1;
                }
                Err(error) => assert!(matches!(
                    error.kind(),
                    PathAdapterErrorKind::SymbolicLink
                        | PathAdapterErrorKind::NotFound
                        | PathAdapterErrorKind::ObjectKindMismatch
                        | PathAdapterErrorKind::IdentityChanged
                        | PathAdapterErrorKind::PlatformFailure
                )),
            }
        }
        attacker.join().expect("attacker thread joins");
        assert!(attempts >= 512);
        assert!(admitted_safe <= attempts);
        if let Err(error) = initial.revalidate() {
            assert_eq!(error.kind(), PathAdapterErrorKind::IdentityChanged);
        }
    }

    #[test]
    #[ignore = "requires an isolated user and mount namespace"]
    fn isolated_bind_mount_swap_is_rejected_as_mount_changed() {
        let test = TestDirectory::new();
        let root = test.path.join("workspace");
        let mount_point = root.join("mounted");
        let outside = test.path.join("outside");
        fs::create_dir_all(&mount_point).expect("workspace mount point creates");
        fs::create_dir(&outside).expect("outside directory creates");
        fs::write(outside.join("secret.txt"), b"outside mount bytes\n")
            .expect("outside mount fixture writes");
        let workspace = authorize_for_test(&root);

        let mounted = Command::new("mount")
            .args(["--bind"])
            .arg(&outside)
            .arg(&mount_point)
            .status()
            .expect("mount command starts");
        assert!(mounted.success(), "isolated bind mount must succeed");

        let result = adapter().resolve(
            &workspace,
            &path(&["mounted", "secret.txt"]),
            PathResolutionIntent::ContentHash,
        );

        let unmounted = Command::new("umount")
            .arg(&mount_point)
            .status()
            .expect("umount command starts");
        assert!(unmounted.success(), "isolated bind mount must be removed");
        let error = result.expect_err("cross-mount path must fail closed");
        assert_eq!(error.kind(), PathAdapterErrorKind::MountChanged);
        assert_eq!(error.component_index(), Some(0));
    }

    #[test]
    fn display_links_are_encoded_redacted_and_rejected_as_workspace_authority() {
        let test = TestDirectory::new();
        let root = test.path.join("workspace");
        fs::create_dir(&root).expect("workspace creates");
        fs::write(root.join("My Note#1%.txt"), b"display\n").expect("fixture writes");
        let workspace = authorize_for_test(&root);
        let held = adapter()
            .resolve(
                &workspace,
                &path(&["My Note#1%.txt"]),
                PathResolutionIntent::ReadFile,
            )
            .expect("display fixture resolves");
        let link = held.display_link(Some(7)).expect("display link generates");
        assert!(link.file_uri().starts_with("file:///"));
        assert!(link.file_uri().ends_with("/My%20Note%231%25.txt"));
        assert_eq!(link.line(), Some(7));
        assert!(link.render_target().ends_with("/My%20Note%231%25.txt#L7"));
        assert_eq!(link.object_identity(), held.object_identity());
        assert!(!format!("{link:?}").contains("My%20Note"));

        let replay = WorkspacePath::new(WorkspaceId::from_raw("workspace-0001"), [link.file_uri()])
            .expect_err("display URI cannot become a workspace component");
        assert_eq!(replay.kind(), WorkspacePathErrorKind::ColonInComponent);
        let rendered_replay = WorkspacePath::new(
            WorkspaceId::from_raw("workspace-0001"),
            [link.render_target()],
        )
        .expect_err("rendered display target cannot become workspace authority");
        assert_eq!(
            rendered_replay.kind(),
            WorkspacePathErrorKind::ColonInComponent
        );
        let zero_line = held
            .display_link(Some(0))
            .expect_err("zero display line rejects");
        assert_eq!(zero_line.kind(), PathAdapterErrorKind::UnsafeComponent);
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
            mount_id: Some(9),
        };
        let mut changed = original.clone();
        changed.changed_nanoseconds += 1;
        assert!(original.same_object(&changed));
        assert_ne!(original, changed);
    }
}
