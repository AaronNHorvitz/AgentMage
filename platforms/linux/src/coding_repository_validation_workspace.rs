// SPDX-License-Identifier: MIT OR Apache-2.0
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

#[cfg(test)]
mod tests {
    use super::*;
    use agentmage_kernel_contracts::OperationOutcome;
    use agentmage_kernel_engine::{
        command_runner::CommandTermination, propagation::CancellationToken,
    };
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn completion_binding_independently_refuses_command_and_sealed_input_substitution() {
        let root = std::env::temp_dir().join(format!(
            "agentmage-binding-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for side in ["original", "owned/worktree"] {
            fs::create_dir_all(root.join(side)).unwrap();
        }
        fs::write(root.join("owned/worktree/source.py"), b"source\n").unwrap();
        let created = CreatedWorktree {
            original_path: root.join("original"),
            original: held_root(&root.join("original")).unwrap(),
            path: root.join("owned/worktree"),
            held: held_root(&root.join("owned/worktree")).unwrap(),
            owned: held_root(&root.join("owned")).unwrap(),
            creation_receipt_sha256: hash(b"synthetic creation"),
            git_creation: None,
            edits: BTreeMap::new(),
        };
        let observation = WorkspaceObservation::capture(&created.held).unwrap();
        let sealed = crate::command_runner::worktree::capture_input_for_test(
            &created.held,
            &CancellationToken::root(
                BoundaryKind::Shell,
                TaskId::from_raw("synthetic-binding"),
                CorrelationId::from_raw("synthetic-binding"),
            ),
        )
        .unwrap();
        // Inert synthetic receipt: tests only the binding's independent equality
        // clauses, never command validation, native launch or acceptance.
        let command = CommandReceipt {
            sealed_workspace_input: Some(sealed),
            schema_version: 1,
            authority_transaction_id: "synthetic-transaction".into(),
            operation_attempt_id: "synthetic-operation".into(),
            command_attempt_id: "synthetic-command".into(),
            template_id: "synthetic-template".into(),
            template_version: "1.0.0".into(),
            spec_sha256: hash(b"spec"),
            request_sha256: hash(b"request"),
            preview_sha256: hash(b"preview"),
            termination: CommandTermination::Exited,
            outcome: OperationOutcome::Succeeded,
            exit_code: Some(0),
            signal: None,
            stdout_sha256: hash(b""),
            stdout_total_bytes: 0,
            stdout_retained_bytes: 0,
            stdout_truncated: false,
            stderr_sha256: hash(b""),
            stderr_total_bytes: 0,
            stderr_retained_bytes: 0,
            stderr_truncated: false,
            elapsed_ms: 1,
            resource_usage: None,
            descendants_terminated: true,
            platform_code: "synthetic.binding".into(),
            receipt_sha256: hash(b"command receipt"),
        };
        let receipt = hash(b"validation receipt");
        let binding = ValidationWorkspaceBinding::new(
            observation,
            &created.creation_receipt_sha256,
            &receipt,
            &command,
        );
        assert!(binding.permits_completion_summary(&created, &receipt, &command));
        for substitute_seal in [false, true] {
            let mut substituted = command.clone();
            if substitute_seal {
                substituted
                    .sealed_workspace_input
                    .as_mut()
                    .unwrap()
                    .projection_sha256 = hash(b"foreign projection");
                assert_eq!(binding.command_receipt, substituted.receipt_sha256);
            } else {
                substituted.receipt_sha256 = hash(b"foreign command receipt");
                assert_eq!(binding.sealed_input, substituted.sealed_workspace_input);
            }
            // Every other conjunct still passes, isolating the changed clause.
            assert!(binding.sealed_matches_observation());
            assert!(binding.matches_current(&created, &receipt));
            assert!(!binding.permits_completion_summary(&created, &receipt, &substituted));
        }
        assert!(binding.permits_completion_summary(&created, &receipt, &command));
        fs::remove_dir_all(root).unwrap();
    }
}
