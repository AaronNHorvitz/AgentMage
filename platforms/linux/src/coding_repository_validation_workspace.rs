// SPDX-License-Identifier: Apache-2.0
//! Bounded whole-worktree observations inside the original command admission.

use super::*;
use agentmage_kernel_engine::command_runner::{
    BoundedCommandExecutor, CommandExecutionFailure, CommandLaunchPermit, CommandPlatformResult,
    CommandReceipt, SealedCommandWorkspaceInput,
};
use agentmage_kernel_engine::propagation::EffectCancellationObservation;

pub(in crate::coding_repository) use crate::command_runner::worktree::WorkspaceObservation;

pub(in crate::coding_repository) struct ValidationWorkspaceBinding {
    observation: WorkspaceObservation,
    creation_receipt: String,
    validation_receipt: String,
    command_receipt: String,
    sealed_input: Option<SealedCommandWorkspaceInput>,
}

impl ValidationWorkspaceBinding {
    pub(in crate::coding_repository) fn new(
        observation: WorkspaceObservation,
        creation: &str,
        receipt: &str,
        command: &CommandReceipt,
    ) -> Self {
        Self {
            observation,
            creation_receipt: creation.into(),
            validation_receipt: receipt.into(),
            command_receipt: command.receipt_sha256.clone(),
            sealed_input: command.sealed_workspace_input.clone(),
        }
    }

    pub(in crate::coding_repository) fn inspect(
        &self,
        created: &CreatedWorktree,
        receipt: &str,
    ) -> Value {
        let matches = self.matches_current(created, receipt);
        let sealed_matches = self.sealed_matches_observation();
        json!({"version":1,"scope":"whole-worktree-observation",
            "validation_receipt_sha256":self.validation_receipt,
            "command_receipt_sha256":self.command_receipt,
            "sealed_workspace_input":self.sealed_input,
            "sealed_input_matches_observation":sealed_matches,
            "creation_receipt_sha256":self.creation_receipt,
            "worktree_root_sha256":self.observation.root,
            "observation_sha256":self.observation.digest,
            "entries":self.observation.entries,"file_bytes":self.observation.bytes,
            "matches_current_observation":matches,"atomic_snapshot":false,
            "independent_task_grade":false})
    }

    pub(in crate::coding_repository) fn permits_completion_summary(
        &self,
        created: &CreatedWorktree,
        receipt: &str,
        command: &CommandReceipt,
    ) -> bool {
        self.command_receipt == command.receipt_sha256
            && self.sealed_input == command.sealed_workspace_input
            && self.sealed_matches_observation()
            && self.matches_current(created, receipt)
    }

    fn matches_current(&self, created: &CreatedWorktree, receipt: &str) -> bool {
        self.creation_receipt == created.creation_receipt_sha256
            && self.validation_receipt == receipt
            && WorkspaceObservation::capture(&created.held)
                .is_some_and(|now| now == self.observation)
    }

    fn sealed_matches_observation(&self) -> bool {
        self.sealed_input.as_ref().is_some_and(|input| {
            input.worktree_root_sha256 == self.observation.root
                && input.observation_sha256 == self.observation.digest
                && u64::from(input.files) + u64::from(input.directories) - 1
                    == self.observation.entries as u64
                && input.file_bytes == self.observation.bytes
        })
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
