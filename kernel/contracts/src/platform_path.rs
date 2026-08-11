//! Platform path-adapter contracts around authorized workspace handles.

use std::fmt;

use crate::{AdapterInstanceId, WorkspaceAuthorizationId, WorkspaceId, WorkspacePath};

/// Closed platform family reported by one selected path adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PathPlatform {
    /// Deterministic in-memory adapter used only by contract tests.
    DeterministicFake,
    /// Fedora or Ubuntu Linux adapter.
    Linux,
    /// Apple Silicon macOS adapter.
    MacOs,
}

/// Closed read-only purpose for resolving one canonical workspace path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathResolutionIntent {
    /// Observe metadata without reading file content.
    Metadata,
    /// Hold one regular file for bounded reading.
    ReadFile,
    /// Hold one directory for bounded enumeration.
    ReadDirectory,
    /// Hold one regular file while computing its exact content digest.
    ContentHash,
}

/// Closed filesystem-object kind admitted by a read-only path adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceObjectKind {
    /// A regular file held for metadata, reading, or hashing.
    RegularFile,
    /// A directory held for metadata or bounded enumeration.
    Directory,
}

/// Content-free digest evidence for one platform filesystem identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceObjectIdentity {
    platform: PathPlatform,
    mount_identity_sha256: [u8; 32],
    object_identity_sha256: [u8; 32],
}

impl WorkspaceObjectIdentity {
    /// Creates evidence from adapter-computed, domain-separated identity digests.
    #[must_use]
    pub const fn new(
        platform: PathPlatform,
        mount_identity_sha256: [u8; 32],
        object_identity_sha256: [u8; 32],
    ) -> Self {
        Self {
            platform,
            mount_identity_sha256,
            object_identity_sha256,
        }
    }

    /// Returns the platform family that produced this identity.
    #[must_use]
    pub const fn platform(&self) -> PathPlatform {
        self.platform
    }

    /// Returns the exact mount-identity digest without native identifiers.
    #[must_use]
    pub const fn mount_identity_sha256(&self) -> &[u8; 32] {
        &self.mount_identity_sha256
    }

    /// Returns the exact object-identity digest without native identifiers.
    #[must_use]
    pub const fn object_identity_sha256(&self) -> &[u8; 32] {
        &self.object_identity_sha256
    }
}

/// Exact bounded content preimage computed from one continuously held file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilePreimage {
    byte_len: u64,
    content_sha256: [u8; 32],
}

impl FilePreimage {
    /// Creates exact preimage evidence from the hashed byte count and digest.
    #[must_use]
    pub const fn new(byte_len: u64, content_sha256: [u8; 32]) -> Self {
        Self {
            byte_len,
            content_sha256,
        }
    }

    /// Returns the number of bytes included in the digest.
    #[must_use]
    pub const fn byte_len(&self) -> u64 {
        self.byte_len
    }

    /// Returns the exact binary SHA-256 digest.
    #[must_use]
    pub const fn content_sha256(&self) -> &[u8; 32] {
        &self.content_sha256
    }
}

/// Stable, content-free path-adapter failure class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathAdapterErrorKind {
    /// The path and authorized handle name different workspaces.
    WorkspaceMismatch,
    /// The handle did not originate from the selected adapter instance.
    ForeignHandle,
    /// The selected workspace authorization is no longer current.
    StaleAuthorization,
    /// A platform primitive required for safe resolution is unavailable.
    UnsupportedPrimitive,
    /// A path component is unsafe for the platform boundary.
    UnsafeComponent,
    /// Resolution encountered a symbolic link.
    SymbolicLink,
    /// Resolution encountered an alias-like platform redirect.
    Alias,
    /// Resolution encountered a multiply linked regular file.
    HardLink,
    /// The held object identity changed during validation.
    IdentityChanged,
    /// The workspace mount identity changed during validation.
    MountChanged,
    /// The requested object does not exist beneath the workspace.
    NotFound,
    /// The object kind is incompatible with the requested read intent.
    ObjectKindMismatch,
    /// A bounded read or hash would exceed the configured resource limit.
    ResourceLimitExceeded,
    /// The operating system denied the bounded operation.
    PermissionDenied,
    /// A bounded platform operation failed without safe additional detail.
    PlatformFailure,
}

impl PathAdapterErrorKind {
    /// Returns the stable redacted code for this failure class.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::WorkspaceMismatch => "path.adapter.workspace_mismatch",
            Self::ForeignHandle => "path.adapter.foreign_handle",
            Self::StaleAuthorization => "path.adapter.stale_authorization",
            Self::UnsupportedPrimitive => "path.adapter.unsupported_primitive",
            Self::UnsafeComponent => "path.adapter.unsafe_component",
            Self::SymbolicLink => "path.adapter.symbolic_link",
            Self::Alias => "path.adapter.alias",
            Self::HardLink => "path.adapter.hard_link",
            Self::IdentityChanged => "path.adapter.identity_changed",
            Self::MountChanged => "path.adapter.mount_changed",
            Self::NotFound => "path.adapter.not_found",
            Self::ObjectKindMismatch => "path.adapter.object_kind_mismatch",
            Self::ResourceLimitExceeded => "path.adapter.resource_limit_exceeded",
            Self::PermissionDenied => "path.adapter.permission_denied",
            Self::PlatformFailure => "path.adapter.platform_failure",
        }
    }
}

/// Content-free error returned by a platform path adapter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathAdapterError {
    kind: PathAdapterErrorKind,
    component_index: Option<usize>,
}

impl PathAdapterError {
    /// Creates a bounded adapter error without retaining path or operating-system text.
    #[must_use]
    pub const fn new(kind: PathAdapterErrorKind, component_index: Option<usize>) -> Self {
        Self {
            kind,
            component_index,
        }
    }

    /// Returns the stable failure class.
    #[must_use]
    pub const fn kind(&self) -> PathAdapterErrorKind {
        self.kind
    }

    /// Returns only the unsafe component index, never component content.
    #[must_use]
    pub const fn component_index(&self) -> Option<usize> {
        self.component_index
    }
}

impl fmt::Display for PathAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.component_index {
            Some(index) => write!(formatter, "{} at component {index}", self.kind.code()),
            None => formatter.write_str(self.kind.code()),
        }
    }
}

impl std::error::Error for PathAdapterError {}

/// Adapter-owned proof that one workspace selection remains authorized.
///
/// Implementations hold native authorization state privately. The kernel can inspect
/// only stable identities and cannot construct a platform handle through this trait.
pub trait AuthorizedWorkspaceHandle: fmt::Debug + Send + Sync {
    /// Returns the exact approved workspace identity.
    fn workspace_id(&self) -> &WorkspaceId;

    /// Returns the unique authorization event identity.
    fn authorization_id(&self) -> &WorkspaceAuthorizationId;

    /// Returns the selected adapter-instance identity.
    fn adapter_instance_id(&self) -> &AdapterInstanceId;

    /// Returns the platform family that owns the native authorization.
    fn platform(&self) -> PathPlatform;
}

/// Adapter-owned object held continuously from path validation through use.
pub trait HeldWorkspaceObject: fmt::Debug + Send {
    /// Returns the exact canonical workspace path used for resolution.
    fn workspace_path(&self) -> &WorkspacePath;

    /// Returns the authorization event under which this object was held.
    fn authorization_id(&self) -> &WorkspaceAuthorizationId;

    /// Returns the adapter instance that owns the held native object.
    fn adapter_instance_id(&self) -> &AdapterInstanceId;

    /// Returns the read-only intent validated for this held object.
    fn intent(&self) -> PathResolutionIntent;

    /// Returns the validated filesystem-object kind.
    fn object_kind(&self) -> WorkspaceObjectKind;

    /// Returns the platform-scoped mount and object identity evidence.
    fn object_identity(&self) -> &WorkspaceObjectIdentity;

    /// Returns an exact preimage when regular-file reading or hashing was requested.
    fn preimage(&self) -> Option<&FilePreimage>;
}

/// Shared contract implemented by each platform's secure path adapter.
///
/// Associated types prevent handles and held objects from one adapter implementation
/// from being passed to another implementation without an explicit conversion that this
/// contract does not provide.
pub trait PlatformPathAdapter: fmt::Debug + Send + Sync {
    /// Native authorization type created by this adapter.
    type WorkspaceHandle: AuthorizedWorkspaceHandle;
    /// Native object type held by this adapter from validation through use.
    type HeldObject: HeldWorkspaceObject;

    /// Returns the selected adapter-instance identity.
    fn adapter_instance_id(&self) -> &AdapterInstanceId;

    /// Returns the adapter's platform family.
    fn platform(&self) -> PathPlatform;

    /// Resolves and holds one canonical path beneath an authorized workspace.
    fn resolve(
        &self,
        workspace: &Self::WorkspaceHandle,
        path: &WorkspacePath,
        intent: PathResolutionIntent,
    ) -> Result<Self::HeldObject, PathAdapterError>;
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{
        AuthorizedWorkspaceHandle, FilePreimage, HeldWorkspaceObject, PathAdapterError,
        PathAdapterErrorKind, PathPlatform, PathResolutionIntent, PlatformPathAdapter,
        WorkspaceObjectIdentity, WorkspaceObjectKind,
    };
    use crate::{AdapterInstanceId, WorkspaceAuthorizationId, WorkspaceId, WorkspacePath};

    #[derive(Debug)]
    struct FakeHandle {
        workspace_id: WorkspaceId,
        authorization_id: WorkspaceAuthorizationId,
        adapter_instance_id: AdapterInstanceId,
    }

    impl AuthorizedWorkspaceHandle for FakeHandle {
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
            PathPlatform::DeterministicFake
        }
    }

    #[derive(Debug)]
    struct FakeHeldObject {
        path: WorkspacePath,
        authorization_id: WorkspaceAuthorizationId,
        adapter_instance_id: AdapterInstanceId,
        intent: PathResolutionIntent,
        object_identity: WorkspaceObjectIdentity,
    }

    impl HeldWorkspaceObject for FakeHeldObject {
        fn workspace_path(&self) -> &WorkspacePath {
            &self.path
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
            WorkspaceObjectKind::RegularFile
        }

        fn object_identity(&self) -> &WorkspaceObjectIdentity {
            &self.object_identity
        }

        fn preimage(&self) -> Option<&FilePreimage> {
            None
        }
    }

    #[derive(Debug)]
    struct FakeAdapter {
        adapter_instance_id: AdapterInstanceId,
        observations: AtomicUsize,
    }

    impl PlatformPathAdapter for FakeAdapter {
        type WorkspaceHandle = FakeHandle;
        type HeldObject = FakeHeldObject;

        fn adapter_instance_id(&self) -> &AdapterInstanceId {
            &self.adapter_instance_id
        }

        fn platform(&self) -> PathPlatform {
            PathPlatform::DeterministicFake
        }

        fn resolve(
            &self,
            workspace: &Self::WorkspaceHandle,
            path: &WorkspacePath,
            intent: PathResolutionIntent,
        ) -> Result<Self::HeldObject, PathAdapterError> {
            if workspace.adapter_instance_id() != self.adapter_instance_id() {
                return Err(PathAdapterError::new(
                    PathAdapterErrorKind::ForeignHandle,
                    None,
                ));
            }
            if workspace.workspace_id() != path.workspace_id() {
                return Err(PathAdapterError::new(
                    PathAdapterErrorKind::WorkspaceMismatch,
                    None,
                ));
            }
            self.observations.fetch_add(1, Ordering::SeqCst);
            Ok(FakeHeldObject {
                path: path.clone(),
                authorization_id: workspace.authorization_id().clone(),
                adapter_instance_id: workspace.adapter_instance_id().clone(),
                intent,
                object_identity: WorkspaceObjectIdentity::new(
                    PathPlatform::DeterministicFake,
                    [1; 32],
                    [2; 32],
                ),
            })
        }
    }

    fn adapter() -> FakeAdapter {
        FakeAdapter {
            adapter_instance_id: AdapterInstanceId::from_raw("adapter-fake-0001"),
            observations: AtomicUsize::new(0),
        }
    }

    fn handle() -> FakeHandle {
        FakeHandle {
            workspace_id: WorkspaceId::from_raw("workspace-0001"),
            authorization_id: WorkspaceAuthorizationId::from_raw("authorization-0001"),
            adapter_instance_id: AdapterInstanceId::from_raw("adapter-fake-0001"),
        }
    }

    #[test]
    fn adapter_contract_binds_handle_path_platform_and_intent() {
        let adapter = adapter();
        let handle = handle();
        let path = WorkspacePath::new(
            WorkspaceId::from_raw("workspace-0001"),
            ["fixtures", "input.txt"],
        )
        .expect("canonical path");
        let held = adapter
            .resolve(&handle, &path, PathResolutionIntent::ReadFile)
            .expect("fake resolution");
        assert_eq!(adapter.platform(), PathPlatform::DeterministicFake);
        assert_eq!(handle.platform(), PathPlatform::DeterministicFake);
        assert_eq!(held.workspace_path(), &path);
        assert_eq!(held.authorization_id(), handle.authorization_id());
        assert_eq!(held.adapter_instance_id(), adapter.adapter_instance_id());
        assert_eq!(held.intent(), PathResolutionIntent::ReadFile);
        assert_eq!(held.object_kind(), WorkspaceObjectKind::RegularFile);
        assert_eq!(held.object_identity().mount_identity_sha256(), &[1; 32]);
        assert!(held.preimage().is_none());
        assert_eq!(adapter.observations.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn mismatched_workspace_and_foreign_handle_fail_before_observation() {
        let adapter = adapter();
        let path = WorkspacePath::new(WorkspaceId::from_raw("workspace-0002"), ["input.txt"])
            .expect("canonical path");
        let workspace_error = adapter
            .resolve(&handle(), &path, PathResolutionIntent::Metadata)
            .expect_err("workspace mismatch");
        assert_eq!(
            workspace_error.kind(),
            PathAdapterErrorKind::WorkspaceMismatch
        );

        let mut foreign = handle();
        foreign.adapter_instance_id = AdapterInstanceId::from_raw("adapter-foreign-0001");
        let foreign_error = adapter
            .resolve(&foreign, &path, PathResolutionIntent::Metadata)
            .expect_err("foreign handle");
        assert_eq!(foreign_error.kind(), PathAdapterErrorKind::ForeignHandle);
        assert_eq!(adapter.observations.load(Ordering::SeqCst), 0);
    }
}
