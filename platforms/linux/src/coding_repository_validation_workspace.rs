// SPDX-License-Identifier: Apache-2.0
//! Bounded whole-worktree observations inside the original command admission.

use super::*;
use agentmage_kernel_contracts::{HeldWorkspaceObject, WorkspaceObjectKind};
use agentmage_kernel_engine::command_runner::{
    BoundedCommandExecutor, CommandExecutionFailure, CommandLaunchPermit, CommandPlatformResult,
};
use agentmage_kernel_engine::propagation::EffectCancellationObservation;

const MAX_ENTRIES: usize = 4096;
const MAX_DIRECTORIES: usize = 256;
const MAX_DEPTH: usize = 16;
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 64 * 1024 * 1024;

#[derive(PartialEq, Eq)]
pub(in crate::coding_repository) struct WorkspaceObservation {
    root: String,
    digest: String,
    entries: usize,
    bytes: u64,
}

impl WorkspaceObservation {
    // All names count, including hidden/generated objects and the local Git
    // pointer. Nothing is followed outside the held root or silently excluded.
    fn capture(workspace: &LinuxAuthorizedWorkspace) -> Option<Self> {
        let deadline = Instant::now() + Duration::from_secs(2);
        let first = Self::scan(workspace, deadline)?;
        (first == Self::scan(workspace, deadline)?).then_some(first)
    }

    fn scan(workspace: &LinuxAuthorizedWorkspace, deadline: Instant) -> Option<Self> {
        workspace.revalidate().ok()?;
        let root_before = crate::snapshot(&workspace.root_descriptor, None).ok()?;
        let adapter = LinuxPathAdapter::new(workspace.adapter_instance_id.clone(), MAX_FILE_BYTES);
        let mut pending = vec![Vec::<String>::new()];
        let mut entries = BTreeMap::new();
        let mut bytes = 0_u64;
        let mut directories = 0;
        while let Some(components) = pending.pop() {
            if Instant::now() >= deadline || components.len() > MAX_DEPTH {
                return None;
            }
            directories += 1;
            if directories > MAX_DIRECTORIES {
                return None;
            }
            let directory = if components.is_empty() {
                None
            } else {
                let path =
                    WorkspacePath::new(workspace.workspace_id.clone(), components.clone()).ok()?;
                Some(
                    adapter
                        .resolve(workspace, &path, PathResolutionIntent::ReadDirectory)
                        .ok()?,
                )
            };
            let names = match &directory {
                None => workspace.observe_root_names(MAX_ENTRIES, 256 * 1024),
                Some(directory) => directory.observe_directory_names(MAX_ENTRIES, 256 * 1024),
            }
            .ok()?;
            for name in names {
                if Instant::now() >= deadline || entries.len() >= MAX_ENTRIES {
                    return None;
                }
                let mut child = components.clone();
                child.push(name);
                let path =
                    WorkspacePath::new(workspace.workspace_id.clone(), child.clone()).ok()?;
                let object = adapter
                    .resolve(workspace, &path, PathResolutionIntent::Metadata)
                    .ok()?;
                let object = match object.object_kind() {
                    WorkspaceObjectKind::Directory => {
                        pending.push(child.clone());
                        object
                    }
                    WorkspaceObjectKind::RegularFile => {
                        let size = u64::try_from(object.object_snapshot.size).ok()?;
                        bytes = bytes.checked_add(size)?;
                        if bytes > MAX_TOTAL_BYTES {
                            return None;
                        }
                        let file = adapter
                            .resolve(workspace, &path, PathResolutionIntent::ContentHash)
                            .ok()?;
                        // Refuse a replacement between metadata and content reads.
                        if file.object_snapshot != object.object_snapshot {
                            return None;
                        }
                        file.revalidate().ok()?;
                        file
                    }
                };
                let stat = &object.object_snapshot;
                entries.insert(
                    child,
                    json!({
                        "device":stat.device,"inode":stat.inode,"links":stat.link_count,
                        "mode":stat.mode,"size":stat.size,"mount":stat.mount_id,
                        "mtime":[stat.modified_seconds,stat.modified_nanoseconds],
                        "ctime":[stat.changed_seconds,stat.changed_nanoseconds],
                        "preimage":object.preimage(),
                    }),
                );
            }
            if let Some(directory) = directory {
                directory.revalidate().ok()?;
            }
        }
        // Directory timestamps detect additions/removals, including transient
        // replacements. File timestamps include ctime, not just caller-set mtime.
        let root_after = crate::snapshot(&workspace.root_descriptor, None).ok()?;
        if root_before != root_after || Instant::now() >= deadline {
            return None;
        }
        let material = json!({"version":1,"root":workspace.root_path_sha256(),
            "root_stat":{"device":root_before.device,"inode":root_before.inode,
                "links":root_before.link_count,"mode":root_before.mode,
                "size":root_before.size,"mount":root_before.mount_id,
                "mtime":[root_before.modified_seconds,root_before.modified_nanoseconds],
                "ctime":[root_before.changed_seconds,root_before.changed_nanoseconds]},
            "entries":entries.into_iter().collect::<Vec<_>>()});
        Some(Self {
            root: workspace.root_path_sha256().into(),
            digest: hash(&serde_json::to_vec(&material).ok()?),
            entries: material["entries"].as_array()?.len(),
            bytes,
        })
    }
}

pub(in crate::coding_repository) struct ValidationWorkspaceBinding {
    observation: WorkspaceObservation,
    creation_receipt: String,
    validation_receipt: String,
}

impl ValidationWorkspaceBinding {
    pub(in crate::coding_repository) fn new(
        observation: WorkspaceObservation,
        creation: &str,
        receipt: &str,
    ) -> Self {
        Self {
            observation,
            creation_receipt: creation.into(),
            validation_receipt: receipt.into(),
        }
    }

    pub(in crate::coding_repository) fn inspect(
        &self,
        created: &CreatedWorktree,
        receipt: &str,
    ) -> Value {
        let matches = self.creation_receipt == created.creation_receipt_sha256
            && self.validation_receipt == receipt
            && WorkspaceObservation::capture(&created.held)
                .is_some_and(|now| now == self.observation);
        json!({"version":1,"scope":"whole-worktree-observation",
            "validation_receipt_sha256":self.validation_receipt,
            "creation_receipt_sha256":self.creation_receipt,
            "worktree_root_sha256":self.observation.root,
            "observation_sha256":self.observation.digest,
            "entries":self.observation.entries,"file_bytes":self.observation.bytes,
            "matches_current_observation":matches,"atomic_snapshot":false,
            "independent_task_grade":false})
    }
}

/// Delegates the original permit once. Observations cannot issue authority or
/// substitute a command, executor, working directory, result or cancellation.
pub(in crate::coding_repository) struct ValidationObservedExecutor<'a, E> {
    pub(in crate::coding_repository) native: E,
    pub(in crate::coding_repository) enabled: bool,
    pub(in crate::coding_repository) observation: &'a mut Option<WorkspaceObservation>,
}

impl<E: BoundedCommandExecutor<WorkingDirectory = LinuxAuthorizedWorkspace>> BoundedCommandExecutor
    for ValidationObservedExecutor<'_, E>
{
    type WorkingDirectory = LinuxAuthorizedWorkspace;
    fn execute(
        &mut self,
        permit: CommandLaunchPermit<'_>,
        workspace: &LinuxAuthorizedWorkspace,
        cancellation: &dyn EffectCancellationObservation,
    ) -> std::result::Result<CommandPlatformResult, CommandExecutionFailure> {
        let before = self
            .enabled
            .then(|| WorkspaceObservation::capture(workspace))
            .flatten();
        let result = self.native.execute(permit, workspace, cancellation);
        if let Some(before) = before
            && result.is_ok()
            && WorkspaceObservation::capture(workspace).is_some_and(|after| before == after)
        {
            *self.observation = Some(before);
        }
        result
    }
}
