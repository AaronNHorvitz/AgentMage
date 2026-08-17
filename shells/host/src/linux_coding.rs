//! Linux descriptor binding for one immutable native coding-session profile.

use std::fmt::Write;
use std::path::Path;

use agentmage_capability_repository_map::RepositoryMap;
use agentmage_kernel_contracts::{
    AuthorizedWorkspaceHandle, GrantTarget, HeldWorkspaceObject, PathResolutionIntent,
    WorkspaceAuthorizationId, WorkspaceObjectKind, WorkspacePath,
};
use agentmage_kernel_engine::platform_startup::VerifiedPlatformAdapter;
use agentmage_platform_linux::{
    LinuxAuthorizedWorkspace, LinuxHeldObject, LinuxPlatformAdapter, linux_repository_path_sha256,
    resolve_linux_workspace_object, select_linux_workspace,
};

use crate::{
    coding_operation::{
        NativeCodingOperationPlanner, NativeCodingTargetPlan, PreparedNativeCodingOperation,
    },
    coding_projection::CodingRepositoryProjection,
    coding_session::CodingSessionProfile,
};

/// Stable content-free refusal while binding a coding profile to Linux descriptors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxCodingBindingError {
    /// The actual directory does not equal the profile's owned-worktree identity.
    WorktreeDenied,
    /// The repository map does not equal the profile's frozen repository snapshot.
    ProjectionDenied,
    /// The model call could not produce one valid profile-bound operation plan.
    OperationDenied,
    /// One exact path, object kind, identity, or preimage could not be held.
    TargetDenied,
}

impl LinuxCodingBindingError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::WorktreeDenied => "runtime.linux-coding.worktree-denied",
            Self::ProjectionDenied => "runtime.linux-coding.projection-denied",
            Self::OperationDenied => "runtime.linux-coding.operation-denied",
            Self::TargetDenied => "runtime.linux-coding.target-denied",
        }
    }
}

/// Continuously held Linux objects associated with one inert coding operation.
pub enum LinuxCodingTargetBinding {
    /// Exact path-ordered objects for one bounded read-only worker.
    ReadProjection {
        /// Continuously held exact Linux objects.
        held: Vec<LinuxHeldObject>,
        /// Canonical grant targets derived from those objects.
        targets: Vec<GrantTarget>,
    },
    /// Exact continuously held worktree root; the owning session retains its descriptor.
    OwnedWorktreeRoot {
        /// Canonical root operation target.
        target: GrantTarget,
    },
    /// One continuously held existing regular file.
    ExistingFile {
        /// Continuously held exact Linux file.
        held: LinuxHeldObject,
        /// Canonical operation target derived from the file.
        target: GrantTarget,
    },
    /// One continuously held non-root parent, or the session-held root, for direct-child creation.
    DestinationParent {
        /// Continuously held directory when the parent is below the root.
        held_parent: Option<LinuxHeldObject>,
        /// Canonical parent operation target.
        target: GrantTarget,
        /// Exact direct-child destination that must remain absent.
        destination: WorkspacePath,
        /// Model-observed parent projection digest awaiting trusted reconciliation.
        expected_parent_sha256: String,
    },
}

impl std::fmt::Debug for LinuxCodingTargetBinding {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (kind, targets) = match self {
            Self::ReadProjection { targets, .. } => ("read_projection", targets.len()),
            Self::OwnedWorktreeRoot { .. } => ("owned_worktree_root", 1),
            Self::ExistingFile { .. } => ("existing_file", 1),
            Self::DestinationParent { .. } => ("destination_parent", 1),
        };
        formatter
            .debug_struct("LinuxCodingTargetBinding")
            .field("kind", &kind)
            .field("target_count", &targets)
            .finish_non_exhaustive()
    }
}

impl LinuxCodingTargetBinding {
    /// Returns the number of exact operation targets retained by this binding.
    #[must_use]
    pub fn target_count(&self) -> usize {
        match self {
            Self::ReadProjection { targets, .. } => targets.len(),
            Self::OwnedWorktreeRoot { .. }
            | Self::ExistingFile { .. }
            | Self::DestinationParent { .. } => 1,
        }
    }

    /// Returns one exact operation target by stable index.
    #[must_use]
    pub fn target(&self, index: usize) -> Option<&GrantTarget> {
        match self {
            Self::ReadProjection { targets, .. } => targets.get(index),
            Self::OwnedWorktreeRoot { target }
            | Self::ExistingFile { target, .. }
            | Self::DestinationParent { target, .. } => (index == 0).then_some(target),
        }
    }
}

/// One profile-bound native operation with all required Linux descriptors held open.
#[derive(Debug)]
pub struct PreparedLinuxCodingOperation<'workspace> {
    operation: PreparedNativeCodingOperation,
    binding: LinuxCodingTargetBinding,
    workspace: &'workspace LinuxAuthorizedWorkspace,
}

impl<'workspace> PreparedLinuxCodingOperation<'workspace> {
    /// Returns the profile-bound authority-free operation plan.
    #[must_use]
    pub const fn operation(&self) -> &PreparedNativeCodingOperation {
        &self.operation
    }

    /// Returns the exact continuously held Linux target binding.
    #[must_use]
    pub const fn binding(&self) -> &LinuxCodingTargetBinding {
        &self.binding
    }

    /// Returns the continuously held worktree root that owns every relative binding.
    #[must_use]
    pub const fn workspace(&self) -> &LinuxAuthorizedWorkspace {
        self.workspace
    }
}

/// One verified Linux owned-worktree projection for an immutable coding profile.
pub struct LinuxCodingWorkspace<'session, 'platform> {
    platform: &'platform VerifiedPlatformAdapter<LinuxPlatformAdapter>,
    profile: &'session CodingSessionProfile,
    projection: CodingRepositoryProjection,
    workspace: LinuxAuthorizedWorkspace,
}

impl std::fmt::Debug for LinuxCodingWorkspace<'_, '_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LinuxCodingWorkspace")
            .field("profile_id", &self.profile.profile_id())
            .field("worktree_id", &self.profile.worktree().worktree_id)
            .field("workspace", &self.workspace)
            .finish_non_exhaustive()
    }
}

impl<'session, 'platform> LinuxCodingWorkspace<'session, 'platform> {
    /// Verifies and holds the exact actual worktree selected by one coding profile.
    pub fn bind(
        platform: &'platform VerifiedPlatformAdapter<LinuxPlatformAdapter>,
        profile: &'session CodingSessionProfile,
        repository_map: RepositoryMap,
        worktree_root: &Path,
        authorization_id: WorkspaceAuthorizationId,
    ) -> Result<Self, LinuxCodingBindingError> {
        verify_worktree_path(
            profile.worktree().worktree_path_sha256.as_str(),
            worktree_root,
        )?;
        let projection = CodingRepositoryProjection::for_profile(profile, repository_map)
            .map_err(|_| LinuxCodingBindingError::ProjectionDenied)?;
        let workspace = select_linux_workspace(
            platform,
            worktree_root,
            profile.write_scope().workspace_id().clone(),
            authorization_id,
        )
        .map_err(|_| LinuxCodingBindingError::WorktreeDenied)?;
        workspace
            .revalidate()
            .map_err(|_| LinuxCodingBindingError::WorktreeDenied)?;
        GrantTarget::held_workspace_root(&workspace)
            .map_err(|_| LinuxCodingBindingError::WorktreeDenied)?;
        Ok(Self {
            platform,
            profile,
            projection,
            workspace,
        })
    }

    /// Returns the continuously held exact worktree root without granting its use.
    #[must_use]
    pub const fn workspace(&self) -> &LinuxAuthorizedWorkspace {
        &self.workspace
    }

    /// Plans one model call and resolves every required Linux object without executing it.
    pub fn prepare(
        &self,
        call: &agentmage_kernel_contracts::ToolCall,
    ) -> Result<PreparedLinuxCodingOperation<'_>, LinuxCodingBindingError> {
        self.workspace
            .revalidate()
            .map_err(|_| LinuxCodingBindingError::WorktreeDenied)?;
        let operation = NativeCodingOperationPlanner::new(self.profile, &self.projection)
            .prepare(call)
            .map_err(|_| LinuxCodingBindingError::OperationDenied)?;
        let binding = bind_target(&self.workspace, operation.target(), |path, intent| {
            resolve_linux_workspace_object(self.platform, &self.workspace, path, intent)
        })?;
        self.workspace
            .revalidate()
            .map_err(|_| LinuxCodingBindingError::TargetDenied)?;
        Ok(PreparedLinuxCodingOperation {
            operation,
            binding,
            workspace: &self.workspace,
        })
    }
}

fn verify_worktree_path(
    expected_sha256: &str,
    worktree_root: &Path,
) -> Result<(), LinuxCodingBindingError> {
    if !is_sha256(expected_sha256)
        || linux_repository_path_sha256(worktree_root).as_deref() != Ok(expected_sha256)
    {
        return Err(LinuxCodingBindingError::WorktreeDenied);
    }
    Ok(())
}

fn bind_target(
    workspace: &LinuxAuthorizedWorkspace,
    plan: &NativeCodingTargetPlan,
    mut resolve: impl FnMut(
        &WorkspacePath,
        PathResolutionIntent,
    )
        -> Result<LinuxHeldObject, agentmage_kernel_contracts::PathAdapterError>,
) -> Result<LinuxCodingTargetBinding, LinuxCodingBindingError> {
    match plan {
        NativeCodingTargetPlan::ReadProjection { objects, .. } => {
            if objects.is_empty() {
                return Err(LinuxCodingBindingError::TargetDenied);
            }
            let mut held = Vec::with_capacity(objects.len());
            let mut targets = Vec::with_capacity(objects.len());
            for object in objects {
                let intent = match object.object_kind {
                    WorkspaceObjectKind::RegularFile => PathResolutionIntent::ReadFile,
                    WorkspaceObjectKind::Directory => PathResolutionIntent::ReadDirectory,
                };
                let candidate = resolve(&object.path, intent)
                    .map_err(|_| LinuxCodingBindingError::TargetDenied)?;
                if candidate.object_kind() != object.object_kind {
                    return Err(LinuxCodingBindingError::TargetDenied);
                }
                targets.push(
                    GrantTarget::held_object(&candidate)
                        .map_err(|_| LinuxCodingBindingError::TargetDenied)?,
                );
                held.push(candidate);
            }
            Ok(LinuxCodingTargetBinding::ReadProjection { held, targets })
        }
        NativeCodingTargetPlan::OwnedWorktreeRoot => {
            let target = GrantTarget::held_workspace_root(workspace)
                .map_err(|_| LinuxCodingBindingError::TargetDenied)?;
            Ok(LinuxCodingTargetBinding::OwnedWorktreeRoot { target })
        }
        NativeCodingTargetPlan::ExistingFile {
            path,
            expected_preimage_sha256,
        } => {
            let held = resolve(path, PathResolutionIntent::ReadFile)
                .map_err(|_| LinuxCodingBindingError::TargetDenied)?;
            let target = GrantTarget::held_object(&held)
                .map_err(|_| LinuxCodingBindingError::TargetDenied)?;
            if target.object_kind() != Some(WorkspaceObjectKind::RegularFile)
                || target
                    .preimage()
                    .map(|preimage| hex(preimage.content_sha256()))
                    .as_deref()
                    != Some(expected_preimage_sha256)
            {
                return Err(LinuxCodingBindingError::TargetDenied);
            }
            Ok(LinuxCodingTargetBinding::ExistingFile { held, target })
        }
        NativeCodingTargetPlan::DestinationParent {
            parent,
            destination,
            expected_parent_sha256,
        } => {
            let parent_components = parent.as_ref().map_or(&[][..], |path| path.components());
            if destination.workspace_id() != workspace.workspace_id()
                || destination.components().len() != parent_components.len() + 1
                || !destination.components().starts_with(parent_components)
            {
                return Err(LinuxCodingBindingError::TargetDenied);
            }
            let (held_parent, target) = if let Some(parent) = parent {
                let held = resolve(parent, PathResolutionIntent::ReadDirectory)
                    .map_err(|_| LinuxCodingBindingError::TargetDenied)?;
                let target = GrantTarget::held_object(&held)
                    .map_err(|_| LinuxCodingBindingError::TargetDenied)?;
                (Some(held), target)
            } else {
                (
                    None,
                    GrantTarget::held_workspace_root(workspace)
                        .map_err(|_| LinuxCodingBindingError::TargetDenied)?,
                )
            };
            Ok(LinuxCodingTargetBinding::DestinationParent {
                held_parent,
                target,
                destination: destination.clone(),
                expected_parent_sha256: expected_parent_sha256.clone(),
            })
        }
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::{
        AdapterInstanceId, WorkspaceId, WorkspaceObjectKind, WorkspacePath,
    };
    use agentmage_platform_linux::{
        resolve_test_linux_workspace_object, select_test_linux_workspace,
    };
    use sha2::{Digest, Sha256};

    use super::*;
    use crate::coding_projection::CodingProjectionObject;

    static TEMP_ID: AtomicU64 = AtomicU64::new(1);

    #[test]
    fn story_48_2_linux_binding_holds_projection_root_file_and_parent_targets() {
        let fixture = Fixture::new();
        let workspace_id = WorkspaceId::from_raw("workspace-linux-coding");
        let adapter_id = AdapterInstanceId::from_raw("adapter-linux-coding");
        let workspace = select_test_linux_workspace(
            &fixture.root,
            workspace_id.clone(),
            WorkspaceAuthorizationId::from_raw("authorization-linux-coding"),
            adapter_id.clone(),
        )
        .expect("test workspace");
        let resolver = |path: &WorkspacePath, intent: PathResolutionIntent| {
            resolve_test_linux_workspace_object(&workspace, adapter_id.clone(), path, intent)
        };

        let read = bind_target(
            &workspace,
            &NativeCodingTargetPlan::ReadProjection {
                objects: vec![
                    CodingProjectionObject {
                        path: path(&workspace_id, &["src"]),
                        object_kind: WorkspaceObjectKind::Directory,
                    },
                    CodingProjectionObject {
                        path: path(&workspace_id, &["src", "lib.rs"]),
                        object_kind: WorkspaceObjectKind::RegularFile,
                    },
                ],
                projection_sha256: "a".repeat(64),
            },
            resolver,
        )
        .expect("read binding");
        assert_eq!(read.target_count(), 2);

        let root = bind_target(
            &workspace,
            &NativeCodingTargetPlan::OwnedWorktreeRoot,
            |path, intent| {
                resolve_test_linux_workspace_object(&workspace, adapter_id.clone(), path, intent)
            },
        )
        .expect("root binding");
        assert_eq!(root.target_count(), 1);
        assert!(
            root.target(0)
                .is_some_and(|target| target.workspace_path().is_none())
        );

        let file = bind_target(
            &workspace,
            &NativeCodingTargetPlan::ExistingFile {
                path: path(&workspace_id, &["src", "lib.rs"]),
                expected_preimage_sha256: sha256_hex(b"pub fn run() {}\n"),
            },
            |path, intent| {
                resolve_test_linux_workspace_object(&workspace, adapter_id.clone(), path, intent)
            },
        )
        .expect("file binding");
        assert_eq!(file.target_count(), 1);

        let parent = bind_target(
            &workspace,
            &NativeCodingTargetPlan::DestinationParent {
                parent: Some(path(&workspace_id, &["src"])),
                destination: path(&workspace_id, &["src", "new.rs"]),
                expected_parent_sha256: "b".repeat(64),
            },
            |path, intent| {
                resolve_test_linux_workspace_object(&workspace, adapter_id.clone(), path, intent)
            },
        )
        .expect("parent binding");
        assert_eq!(parent.target_count(), 1);
    }

    #[test]
    fn story_48_2_linux_binding_rejects_wrong_path_preimage_and_parent_relationship() {
        let fixture = Fixture::new();
        let actual_sha256 = linux_repository_path_sha256(&fixture.root).expect("path identity");
        assert!(verify_worktree_path(&actual_sha256, &fixture.root).is_ok());
        assert_eq!(
            verify_worktree_path(&"f".repeat(64), &fixture.root),
            Err(LinuxCodingBindingError::WorktreeDenied)
        );

        let workspace_id = WorkspaceId::from_raw("workspace-linux-coding");
        let adapter_id = AdapterInstanceId::from_raw("adapter-linux-coding");
        let workspace = select_test_linux_workspace(
            &fixture.root,
            workspace_id.clone(),
            WorkspaceAuthorizationId::from_raw("authorization-linux-coding"),
            adapter_id.clone(),
        )
        .expect("test workspace");
        let bind = |plan: &NativeCodingTargetPlan| {
            bind_target(&workspace, plan, |path, intent| {
                resolve_test_linux_workspace_object(&workspace, adapter_id.clone(), path, intent)
            })
        };
        assert!(matches!(
            bind(&NativeCodingTargetPlan::ExistingFile {
                path: path(&workspace_id, &["src", "lib.rs"]),
                expected_preimage_sha256: "0".repeat(64),
            }),
            Err(LinuxCodingBindingError::TargetDenied)
        ));
        assert!(matches!(
            bind(&NativeCodingTargetPlan::DestinationParent {
                parent: Some(path(&workspace_id, &["src"])),
                destination: path(&workspace_id, &["other", "new.rs"]),
                expected_parent_sha256: "b".repeat(64),
            }),
            Err(LinuxCodingBindingError::TargetDenied)
        ));
    }

    fn path(workspace_id: &WorkspaceId, components: &[&str]) -> WorkspacePath {
        WorkspacePath::new(workspace_id.clone(), components.iter().copied()).expect("fixture path")
    }

    fn sha256_hex(bytes: &[u8]) -> String {
        hex(&Sha256::digest(bytes))
    }

    struct Fixture {
        root: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "agentmage-linux-coding-{}-{}",
                std::process::id(),
                TEMP_ID.fetch_add(1, Ordering::SeqCst)
            ));
            fs::create_dir_all(root.join("src")).expect("fixture directories");
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700))
                .expect("private fixture root");
            fs::write(root.join("src/lib.rs"), b"pub fn run() {}\n").expect("fixture file");
            Self { root }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
