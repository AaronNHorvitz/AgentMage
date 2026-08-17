//! Linux descriptor binding for one immutable native coding-session profile.

use std::fmt::Write;
use std::path::Path;

use agentmage_capability_repository_map::{RepositoryMap, StructuredFileChangePlan};
use agentmage_kernel_contracts::{
    AuthorizedWorkspaceHandle, GrantTarget, HeldWorkspaceObject, PathResolutionIntent,
    WorkspaceAuthorizationId, WorkspaceObjectKind, WorkspacePath,
};
use agentmage_kernel_engine::{
    filesystem_control::FilesystemOperationDraft, platform_startup::VerifiedPlatformAdapter,
};
use agentmage_platform_linux::{
    LinuxAuthorizedWorkspace, LinuxHeldObject, LinuxPlatformAdapter,
    MAX_DIRECTORY_OBSERVATION_BYTES, MAX_DIRECTORY_OBSERVATION_NAMES, linux_repository_path_sha256,
    resolve_linux_workspace_object, select_linux_workspace,
};

#[cfg(test)]
use agentmage_kernel_contracts::AdapterInstanceId;

use crate::{
    coding_changes::{
        CodingWriteScope, bind_structured_patch_proposal,
        controlled_create_parent_observation_sha256, prepare_controlled_file_creation,
    },
    coding_dispatch::PreparedNativeCodingCall,
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
        /// Trusted sorted bounded sibling-name observation.
        observed_sibling_names: Vec<String>,
    },
}

/// Trusted authority-free write material composed from one model proposal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinuxCodingWriteDraft {
    /// Existing structured-edit plan bound to exact descriptor-read preimage bytes.
    StructuredPatch(StructuredFileChangePlan),
    /// Existing controlled-filesystem creation draft bound to a held parent observation.
    ControlledCreate(FilesystemOperationDraft),
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
    write_draft: Option<LinuxCodingWriteDraft>,
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

    /// Returns trusted write material when this operation proposes a filesystem change.
    #[must_use]
    pub const fn write_draft(&self) -> Option<&LinuxCodingWriteDraft> {
        self.write_draft.as_ref()
    }

    /// Returns the continuously held worktree root that owns every relative binding.
    #[must_use]
    pub const fn workspace(&self) -> &LinuxAuthorizedWorkspace {
        self.workspace
    }

    /// Revalidates every held target and exact directory observation before authority use.
    pub fn revalidate(&self) -> Result<(), LinuxCodingBindingError> {
        self.workspace
            .revalidate()
            .map_err(|_| LinuxCodingBindingError::TargetDenied)?;
        revalidate_binding(self.workspace, &self.binding)?;
        self.workspace
            .revalidate()
            .map_err(|_| LinuxCodingBindingError::TargetDenied)
    }

    /// Separates a retained operation into the exact inert plan and held execution material.
    pub(crate) fn into_parts(
        self,
    ) -> (
        PreparedNativeCodingOperation,
        LinuxCodingTargetBinding,
        Option<LinuxCodingWriteDraft>,
        &'workspace LinuxAuthorizedWorkspace,
    ) {
        (
            self.operation,
            self.binding,
            self.write_draft,
            self.workspace,
        )
    }
}

/// One verified Linux owned-worktree projection for an immutable coding profile.
pub struct LinuxCodingWorkspace<'session, 'platform> {
    platform: LinuxCodingPlatform<'platform>,
    profile: &'session CodingSessionProfile,
    projection: CodingRepositoryProjection,
    workspace: LinuxAuthorizedWorkspace,
}

enum LinuxCodingPlatform<'platform> {
    Verified(&'platform VerifiedPlatformAdapter<LinuxPlatformAdapter>),
    #[cfg(test)]
    Test(AdapterInstanceId),
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
            platform: LinuxCodingPlatform::Verified(platform),
            profile,
            projection,
            workspace,
        })
    }

    #[cfg(test)]
    pub(crate) fn bind_test(
        profile: &'session CodingSessionProfile,
        repository_map: RepositoryMap,
        worktree_root: &Path,
        authorization_id: WorkspaceAuthorizationId,
        adapter_instance_id: AdapterInstanceId,
    ) -> Result<Self, LinuxCodingBindingError> {
        verify_worktree_path(
            profile.worktree().worktree_path_sha256.as_str(),
            worktree_root,
        )?;
        let projection = CodingRepositoryProjection::for_profile(profile, repository_map)
            .map_err(|_| LinuxCodingBindingError::ProjectionDenied)?;
        let workspace = agentmage_platform_linux::select_test_linux_workspace(
            worktree_root,
            profile.write_scope().workspace_id().clone(),
            authorization_id,
            adapter_instance_id.clone(),
        )
        .map_err(|_| LinuxCodingBindingError::WorktreeDenied)?;
        workspace
            .revalidate()
            .map_err(|_| LinuxCodingBindingError::WorktreeDenied)?;
        GrantTarget::held_workspace_root(&workspace)
            .map_err(|_| LinuxCodingBindingError::WorktreeDenied)?;
        Ok(Self {
            platform: LinuxCodingPlatform::Test(adapter_instance_id),
            profile,
            projection,
            workspace,
        })
    }

    /// Returns the immutable profile whose repository and tool identities are held here.
    #[must_use]
    pub const fn profile(&self) -> &CodingSessionProfile {
        self.profile
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
        let binding = bind_target(
            &self.workspace,
            operation.target(),
            |path, intent| match &self.platform {
                LinuxCodingPlatform::Verified(platform) => {
                    resolve_linux_workspace_object(platform, &self.workspace, path, intent)
                }
                #[cfg(test)]
                LinuxCodingPlatform::Test(adapter_instance_id) => {
                    agentmage_platform_linux::resolve_test_linux_workspace_object(
                        &self.workspace,
                        adapter_instance_id.clone(),
                        path,
                        intent,
                    )
                }
            },
        )?;
        let write_draft = compose_write_draft(
            self.profile.write_scope(),
            operation.prepared(),
            operation.target(),
            &binding,
        )?;
        let prepared = PreparedLinuxCodingOperation {
            operation,
            binding,
            write_draft,
            workspace: &self.workspace,
        };
        prepared.revalidate()?;
        Ok(prepared)
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
            let (held_parent, target, observed_sibling_names) = if let Some(parent) = parent {
                let held = resolve(parent, PathResolutionIntent::ReadDirectory)
                    .map_err(|_| LinuxCodingBindingError::TargetDenied)?;
                let target = GrantTarget::held_object(&held)
                    .map_err(|_| LinuxCodingBindingError::TargetDenied)?;
                let names = held
                    .observe_directory_names(
                        MAX_DIRECTORY_OBSERVATION_NAMES,
                        MAX_DIRECTORY_OBSERVATION_BYTES,
                    )
                    .map_err(|_| LinuxCodingBindingError::TargetDenied)?;
                (Some(held), target, names)
            } else {
                let names = workspace
                    .observe_root_names(
                        MAX_DIRECTORY_OBSERVATION_NAMES,
                        MAX_DIRECTORY_OBSERVATION_BYTES,
                    )
                    .map_err(|_| LinuxCodingBindingError::TargetDenied)?;
                (
                    None,
                    GrantTarget::held_workspace_root(workspace)
                        .map_err(|_| LinuxCodingBindingError::TargetDenied)?,
                    names,
                )
            };
            if controlled_create_parent_observation_sha256(&target, &observed_sibling_names)
                .as_deref()
                != Ok(expected_parent_sha256)
            {
                return Err(LinuxCodingBindingError::TargetDenied);
            }
            Ok(LinuxCodingTargetBinding::DestinationParent {
                held_parent,
                target,
                destination: destination.clone(),
                observed_sibling_names,
            })
        }
    }
}

fn compose_write_draft(
    scope: &CodingWriteScope,
    prepared: &PreparedNativeCodingCall,
    target_plan: &NativeCodingTargetPlan,
    binding: &LinuxCodingTargetBinding,
) -> Result<Option<LinuxCodingWriteDraft>, LinuxCodingBindingError> {
    match (prepared, target_plan, binding) {
        (
            PreparedNativeCodingCall::StructuredPatch { proposal },
            NativeCodingTargetPlan::ExistingFile {
                path,
                expected_preimage_sha256,
            },
            LinuxCodingTargetBinding::ExistingFile { held, target },
        ) if held.workspace_path() == path
            && target.workspace_path() == Some(path)
            && proposal.expected_preimage_sha256 == *expected_preimage_sha256 =>
        {
            let preimage = held
                .read_exact_bytes()
                .map_err(|_| LinuxCodingBindingError::TargetDenied)?;
            bind_structured_patch_proposal(scope, proposal.clone(), preimage)
                .map(LinuxCodingWriteDraft::StructuredPatch)
                .map(Some)
                .map_err(|_| LinuxCodingBindingError::TargetDenied)
        }
        (
            PreparedNativeCodingCall::ControlledCreate { proposal },
            NativeCodingTargetPlan::DestinationParent {
                destination,
                expected_parent_sha256,
                ..
            },
            LinuxCodingTargetBinding::DestinationParent {
                target,
                destination: bound_destination,
                observed_sibling_names,
                ..
            },
        ) if destination == bound_destination
            && proposal.expected_parent_sha256 == *expected_parent_sha256 =>
        {
            prepare_controlled_file_creation(
                scope,
                proposal.clone(),
                target.clone(),
                observed_sibling_names.clone(),
            )
            .map(LinuxCodingWriteDraft::ControlledCreate)
            .map(Some)
            .map_err(|_| LinuxCodingBindingError::TargetDenied)
        }
        (
            PreparedNativeCodingCall::ReadOnly { .. },
            NativeCodingTargetPlan::ReadProjection { .. },
            LinuxCodingTargetBinding::ReadProjection { .. },
        )
        | (
            PreparedNativeCodingCall::GitInspection { .. }
            | PreparedNativeCodingCall::Command { .. }
            | PreparedNativeCodingCall::Validation { .. },
            NativeCodingTargetPlan::OwnedWorktreeRoot,
            LinuxCodingTargetBinding::OwnedWorktreeRoot { .. },
        ) => Ok(None),
        _ => Err(LinuxCodingBindingError::TargetDenied),
    }
}

fn revalidate_binding(
    workspace: &LinuxAuthorizedWorkspace,
    binding: &LinuxCodingTargetBinding,
) -> Result<(), LinuxCodingBindingError> {
    match binding {
        LinuxCodingTargetBinding::ReadProjection { held, .. } => held
            .iter()
            .try_for_each(|object| object.revalidate())
            .map_err(|_| LinuxCodingBindingError::TargetDenied),
        LinuxCodingTargetBinding::OwnedWorktreeRoot { .. } => Ok(()),
        LinuxCodingTargetBinding::ExistingFile { held, .. } => held
            .revalidate()
            .map_err(|_| LinuxCodingBindingError::TargetDenied),
        LinuxCodingTargetBinding::DestinationParent {
            held_parent,
            observed_sibling_names,
            ..
        } => {
            let current = if let Some(parent) = held_parent {
                parent.observe_directory_names(
                    MAX_DIRECTORY_OBSERVATION_NAMES,
                    MAX_DIRECTORY_OBSERVATION_BYTES,
                )
            } else {
                workspace.observe_root_names(
                    MAX_DIRECTORY_OBSERVATION_NAMES,
                    MAX_DIRECTORY_OBSERVATION_BYTES,
                )
            }
            .map_err(|_| LinuxCodingBindingError::TargetDenied)?;
            (current == *observed_sibling_names)
                .then_some(())
                .ok_or(LinuxCodingBindingError::TargetDenied)
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

    use agentmage_capability_repository_map::{
        StructuredArtifactClass, StructuredEdit, StructuredLanguage,
    };
    use agentmage_kernel_contracts::{
        AdapterInstanceId, WorkspaceId, WorkspaceObjectKind, WorkspacePath,
    };
    use agentmage_platform_linux::{
        resolve_test_linux_workspace_object, select_test_linux_workspace,
    };
    use sha2::{Digest, Sha256};

    use super::*;
    use crate::{
        coding_changes::{
            ControlledFileClassification, ControlledFileCreationProposal, StructuredPatchProposal,
        },
        coding_projection::CodingProjectionObject,
    };

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

        let parent_path = path(&workspace_id, &["src"]);
        let held_parent = resolve_test_linux_workspace_object(
            &workspace,
            adapter_id.clone(),
            &parent_path,
            PathResolutionIntent::ReadDirectory,
        )
        .expect("parent observation resolves");
        let parent_target = GrantTarget::held_object(&held_parent).expect("parent target");
        let sibling_names = held_parent
            .observe_directory_names(
                MAX_DIRECTORY_OBSERVATION_NAMES,
                MAX_DIRECTORY_OBSERVATION_BYTES,
            )
            .expect("sibling names");
        let expected_parent_sha256 =
            controlled_create_parent_observation_sha256(&parent_target, &sibling_names)
                .expect("parent observation digest");
        let parent = bind_target(
            &workspace,
            &NativeCodingTargetPlan::DestinationParent {
                parent: Some(parent_path),
                destination: path(&workspace_id, &["src", "new.rs"]),
                expected_parent_sha256,
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

    #[test]
    fn story_48_2_linux_composes_exact_patch_and_controlled_create_drafts() {
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
        let scope = CodingWriteScope::new(workspace_id.clone(), vec![vec!["src".to_owned()]])
            .expect("write scope");

        let patch_proposal = StructuredPatchProposal {
            schema_version: 1,
            change_id: "change-linux-coding".to_owned(),
            path: vec!["src".to_owned(), "lib.rs".to_owned()],
            expected_preimage_sha256: sha256_hex(b"pub fn run() {}\n"),
            intent_sha256: "a".repeat(64),
            change_plan_sha256: "b".repeat(64),
            language: StructuredLanguage::Rust,
            artifact_class: StructuredArtifactClass::Code,
            edits: vec![StructuredEdit::RenameIdentifier {
                edit_id: "edit-linux-coding".to_owned(),
                old: "run".to_owned(),
                replacement: "execute".to_owned(),
            }],
            additional_review_hooks: Vec::new(),
            generated: false,
            allow_generated: false,
        };
        let patch_target = NativeCodingTargetPlan::ExistingFile {
            path: path(&workspace_id, &["src", "lib.rs"]),
            expected_preimage_sha256: patch_proposal.expected_preimage_sha256.clone(),
        };
        let patch_binding = bind_target(&workspace, &patch_target, |path, intent| {
            resolve_test_linux_workspace_object(&workspace, adapter_id.clone(), path, intent)
        })
        .expect("patch target binds");
        let patch_draft = compose_write_draft(
            &scope,
            &PreparedNativeCodingCall::StructuredPatch {
                proposal: patch_proposal,
            },
            &patch_target,
            &patch_binding,
        )
        .expect("patch composes")
        .expect("patch write draft");
        let LinuxCodingWriteDraft::StructuredPatch(plan) = patch_draft else {
            panic!("wrong patch draft");
        };
        assert_eq!(plan.postimage(), b"pub fn execute() {}\n");

        let parent_path = path(&workspace_id, &["src"]);
        let held_parent = resolve_test_linux_workspace_object(
            &workspace,
            adapter_id.clone(),
            &parent_path,
            PathResolutionIntent::ReadDirectory,
        )
        .expect("parent resolves");
        let parent_target = GrantTarget::held_object(&held_parent).expect("parent target");
        let sibling_names = held_parent
            .observe_directory_names(
                MAX_DIRECTORY_OBSERVATION_NAMES,
                MAX_DIRECTORY_OBSERVATION_BYTES,
            )
            .expect("sibling names");
        let expected_parent_sha256 =
            controlled_create_parent_observation_sha256(&parent_target, &sibling_names)
                .expect("parent digest");
        let create_proposal = ControlledFileCreationProposal {
            schema_version: 1,
            creation_id: "create-linux-coding".to_owned(),
            path: vec!["src".to_owned(), "new.rs".to_owned()],
            content: "pub fn added() {}\n".to_owned(),
            mode: 0o644,
            classification: ControlledFileClassification::SourceCode,
            intent_sha256: "c".repeat(64),
            change_plan_sha256: "d".repeat(64),
            expected_parent_sha256: expected_parent_sha256.clone(),
        };
        let create_target = NativeCodingTargetPlan::DestinationParent {
            parent: Some(parent_path),
            destination: path(&workspace_id, &["src", "new.rs"]),
            expected_parent_sha256,
        };
        let create_binding = bind_target(&workspace, &create_target, |path, intent| {
            resolve_test_linux_workspace_object(&workspace, adapter_id.clone(), path, intent)
        })
        .expect("create target binds");
        let create_draft = compose_write_draft(
            &scope,
            &PreparedNativeCodingCall::ControlledCreate {
                proposal: create_proposal,
            },
            &create_target,
            &create_binding,
        )
        .expect("create composes")
        .expect("create write draft");
        assert!(matches!(
            create_draft,
            LinuxCodingWriteDraft::ControlledCreate(FilesystemOperationDraft::Create { .. })
        ));
    }

    #[test]
    fn story_48_2_linux_create_binding_detects_stale_root_observation() {
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
        let root_target = GrantTarget::held_workspace_root(&workspace).expect("root target");
        let sibling_names = workspace
            .observe_root_names(
                MAX_DIRECTORY_OBSERVATION_NAMES,
                MAX_DIRECTORY_OBSERVATION_BYTES,
            )
            .expect("root names");
        let expected_parent_sha256 =
            controlled_create_parent_observation_sha256(&root_target, &sibling_names)
                .expect("root digest");
        let binding = bind_target(
            &workspace,
            &NativeCodingTargetPlan::DestinationParent {
                parent: None,
                destination: path(&workspace_id, &["README.md"]),
                expected_parent_sha256,
            },
            |path, intent| {
                resolve_test_linux_workspace_object(&workspace, adapter_id.clone(), path, intent)
            },
        )
        .expect("root destination binds");

        fs::write(fixture.root.join("LICENSE"), b"license\n").expect("root mutates");
        assert_eq!(
            revalidate_binding(&workspace, &binding),
            Err(LinuxCodingBindingError::TargetDenied)
        );
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
