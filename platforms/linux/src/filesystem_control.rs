//! Descriptor-relative Linux driver for controlled filesystem operations.

use agentmage_kernel_contracts::{
    GrantTarget, PathAdapterErrorKind, PathResolutionIntent, PlatformPathAdapter, WorkspacePath,
};
use agentmage_kernel_engine::filesystem_control::{
    ControlledFilesystemDriver, FilesystemApplyAuthorization, FilesystemApplyReport,
    FilesystemDriverError, FilesystemOperation, FilesystemOperationKind,
    FilesystemOperationObservation, FilesystemPlan, FilesystemRestoreAuthorization,
    FilesystemRestoreReport, ObservedFilesystemEntry,
};
use rustix::fs::{AtFlags, Dir, RenameFlags, fstat, fsync, renameat_with, unlinkat};

use crate::{
    LinuxAuthorizedWorkspace, LinuxPathAdapter,
    write_transaction::{
        LinuxAtomicWriteDriver, LinuxAtomicWriteDriverLimits, ReplaceOutcome, held_file_matches,
        open_parent, parent_is_current, read_held_bytes, remove_staged, stage_file, temporary_name,
    },
};

const FAILURE_APPLY: &str = "filesystem.linux.apply_failed";
const FAILURE_RESTORE: &str = "filesystem.linux.restore_failed";
const FAILURE_UNCERTAIN: &str = "filesystem.linux.state_uncertain";

/// Closed resource limits for one Linux controlled-filesystem driver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxFilesystemDriverLimits {
    /// Maximum operations in one transaction.
    pub maximum_operations: usize,
    /// Maximum bytes admitted for one source or postimage.
    pub maximum_file_bytes: u64,
    /// Maximum aggregate source plus postimage bytes.
    pub maximum_transaction_bytes: u64,
    /// Maximum names admitted from one destination directory.
    pub maximum_sibling_names: usize,
    /// Maximum aggregate UTF-8 bytes admitted from one destination directory.
    pub maximum_sibling_name_bytes: usize,
}

impl Default for LinuxFilesystemDriverLimits {
    fn default() -> Self {
        Self {
            maximum_operations: 128,
            maximum_file_bytes: 64 * 1024 * 1024,
            maximum_transaction_bytes: 128 * 1024 * 1024,
            maximum_sibling_names: 4_096,
            maximum_sibling_name_bytes: 1024 * 1024,
        }
    }
}

/// Linux driver that can mutate only through the kernel's opaque filesystem authorization.
pub struct LinuxControlledFilesystemDriver<'workspace> {
    workspace: &'workspace LinuxAuthorizedWorkspace,
    limits: LinuxFilesystemDriverLimits,
    #[cfg(test)]
    race_hook: Option<Box<dyn FnMut(FilesystemLifecycleEvent)>>,
}

impl std::fmt::Debug for LinuxControlledFilesystemDriver<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LinuxControlledFilesystemDriver")
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}

impl<'workspace> LinuxControlledFilesystemDriver<'workspace> {
    /// Creates an inert driver for one already authorized workspace.
    #[must_use]
    pub const fn new(
        workspace: &'workspace LinuxAuthorizedWorkspace,
        limits: LinuxFilesystemDriverLimits,
    ) -> Self {
        Self {
            workspace,
            limits,
            #[cfg(test)]
            race_hook: None,
        }
    }

    fn adapter(&self) -> LinuxPathAdapter {
        LinuxPathAdapter::new(
            self.workspace.adapter_instance_id.clone(),
            self.limits.maximum_file_bytes,
        )
    }

    fn replacement_driver(&self) -> LinuxAtomicWriteDriver<'workspace> {
        LinuxAtomicWriteDriver::new(
            self.workspace,
            LinuxAtomicWriteDriverLimits {
                maximum_operations: self.limits.maximum_operations,
                maximum_file_bytes: self.limits.maximum_file_bytes,
                maximum_transaction_bytes: self.limits.maximum_transaction_bytes,
            },
        )
    }

    fn validate_limits(&self, plan: &FilesystemPlan) -> Result<(), FilesystemDriverError> {
        if plan.operations().is_empty() || plan.operations().len() > self.limits.maximum_operations
        {
            return Err(FilesystemDriverError::ApplyFailed);
        }
        let mut aggregate = 0_u64;
        for operation in plan.operations() {
            let source = u64::try_from(operation.source_bytes().len())
                .map_err(|_| FilesystemDriverError::ApplyFailed)?;
            let postimage = u64::try_from(operation.postimage_bytes().len())
                .map_err(|_| FilesystemDriverError::ApplyFailed)?;
            if source > self.limits.maximum_file_bytes || postimage > self.limits.maximum_file_bytes
            {
                return Err(FilesystemDriverError::ApplyFailed);
            }
            aggregate = aggregate
                .checked_add(source)
                .and_then(|value| value.checked_add(postimage))
                .ok_or(FilesystemDriverError::ApplyFailed)?;
        }
        if aggregate > self.limits.maximum_transaction_bytes {
            return Err(FilesystemDriverError::ApplyFailed);
        }
        Ok(())
    }

    fn observe_all(
        &self,
        plan: &FilesystemPlan,
    ) -> Result<Vec<FilesystemOperationObservation>, FilesystemDriverError> {
        self.validate_limits(plan)?;
        plan.operations()
            .iter()
            .map(|operation| self.observe_operation(operation))
            .collect()
    }

    fn observe_operation(
        &self,
        operation: &FilesystemOperation,
    ) -> Result<FilesystemOperationObservation, FilesystemDriverError> {
        let source = operation
            .source()
            .map(|target| {
                let path = target
                    .workspace_path()
                    .ok_or(FilesystemDriverError::ObservationUnavailable)?;
                self.observe_optional_file(path)
            })
            .transpose()?
            .flatten();
        let Some(expected_parent) = operation.destination_parent() else {
            return Ok(FilesystemOperationObservation {
                source,
                destination_parent: None,
                destination_sibling_names: Vec::new(),
                destination: None,
            });
        };
        let (current_parent, destination_sibling_names) =
            if let Some(parent_path) = expected_parent.workspace_path() {
                let held_parent = self
                    .adapter()
                    .resolve(
                        self.workspace,
                        parent_path,
                        PathResolutionIntent::ReadDirectory,
                    )
                    .map_err(|_| FilesystemDriverError::ObservationUnavailable)?;
                let current_parent = GrantTarget::held_object(&held_parent)
                    .map_err(|_| FilesystemDriverError::ObservationUnavailable)?;
                let sibling_names = self.list_siblings(&held_parent.object_descriptor)?;
                held_parent
                    .revalidate()
                    .map_err(|_| FilesystemDriverError::ObservationUnavailable)?;
                (current_parent, sibling_names)
            } else {
                let current_parent = GrantTarget::held_workspace_root(self.workspace)
                    .map_err(|_| FilesystemDriverError::ObservationUnavailable)?;
                let descriptor = self
                    .workspace
                    .reopen_root_directory()
                    .map_err(|_| FilesystemDriverError::ObservationUnavailable)?;
                let sibling_names = self.list_siblings(&descriptor)?;
                self.workspace
                    .revalidate()
                    .map_err(|_| FilesystemDriverError::ObservationUnavailable)?;
                (current_parent, sibling_names)
            };
        if &current_parent != expected_parent {
            return Err(FilesystemDriverError::ObservationUnavailable);
        }
        let destination_path = destination_path(operation)?;
        let destination = self.observe_optional_file(&destination_path)?;
        Ok(FilesystemOperationObservation {
            source,
            destination_parent: Some(current_parent),
            destination_sibling_names,
            destination,
        })
    }

    fn observe_optional_file(
        &self,
        path: &WorkspacePath,
    ) -> Result<Option<ObservedFilesystemEntry>, FilesystemDriverError> {
        let held =
            match self
                .adapter()
                .resolve(self.workspace, path, PathResolutionIntent::ReadFile)
            {
                Ok(value) => value,
                Err(error) if error.kind() == PathAdapterErrorKind::NotFound => return Ok(None),
                Err(_) => return Err(FilesystemDriverError::ObservationUnavailable),
            };
        let bytes = read_held_bytes(&held, self.limits.maximum_file_bytes)
            .map_err(|_| FilesystemDriverError::ObservationUnavailable)?;
        let stat = fstat(&held.object_descriptor)
            .map_err(|_| FilesystemDriverError::ObservationUnavailable)?;
        let target = GrantTarget::held_object(&held)
            .map_err(|_| FilesystemDriverError::ObservationUnavailable)?;
        Ok(Some(ObservedFilesystemEntry {
            target,
            bytes,
            mode: stat.st_mode & 0o777,
        }))
    }

    fn list_siblings(
        &self,
        directory: &rustix::fd::OwnedFd,
    ) -> Result<Vec<String>, FilesystemDriverError> {
        let entries =
            Dir::read_from(directory).map_err(|_| FilesystemDriverError::ObservationUnavailable)?;
        let mut names = Vec::new();
        let mut total_bytes = 0_usize;
        for entry in entries {
            let entry = entry.map_err(|_| FilesystemDriverError::ObservationUnavailable)?;
            let bytes = entry.file_name().to_bytes();
            if matches!(bytes, b"." | b"..") {
                continue;
            }
            if names.len() >= self.limits.maximum_sibling_names {
                return Err(FilesystemDriverError::ObservationUnavailable);
            }
            total_bytes = total_bytes
                .checked_add(bytes.len())
                .filter(|value| *value <= self.limits.maximum_sibling_name_bytes)
                .ok_or(FilesystemDriverError::ObservationUnavailable)?;
            names.push(
                std::str::from_utf8(bytes)
                    .map_err(|_| FilesystemDriverError::ObservationUnavailable)?
                    .to_owned(),
            );
        }
        names.sort_unstable();
        Ok(names)
    }

    fn apply_operation(
        &mut self,
        transaction_id: &str,
        index: usize,
        operation: &FilesystemOperation,
    ) -> EffectOutcome {
        match operation.kind() {
            FilesystemOperationKind::Create => {
                self.create_exact(transaction_id, index, operation, "create")
            }
            FilesystemOperationKind::ExactPatch => {
                let Some(target) = operation.source() else {
                    return EffectOutcome::NoChange;
                };
                match self.replacement_driver().replace_exact(
                    transaction_id,
                    index,
                    target,
                    operation.source_bytes(),
                    operation.postimage_bytes(),
                    "patch",
                ) {
                    ReplaceOutcome::Applied => EffectOutcome::Applied,
                    ReplaceOutcome::NoChange => EffectOutcome::NoChange,
                    ReplaceOutcome::Uncertain => EffectOutcome::Uncertain,
                }
            }
            FilesystemOperationKind::Copy => {
                if !self.source_is_exact(operation) {
                    return EffectOutcome::NoChange;
                }
                self.create_exact(transaction_id, index, operation, "copy")
            }
            FilesystemOperationKind::Move | FilesystemOperationKind::TrashDelete => {
                self.move_exact(operation, false)
            }
        }
    }

    fn create_exact(
        &mut self,
        transaction_id: &str,
        index: usize,
        operation: &FilesystemOperation,
        purpose: &str,
    ) -> EffectOutcome {
        let Some(expected_parent) = operation.destination_parent() else {
            return EffectOutcome::NoChange;
        };
        let destination = match destination_path(operation) {
            Ok(value) => value,
            Err(_) => return EffectOutcome::NoChange,
        };
        if self.observe_optional_file(&destination).ok() != Some(None) {
            return EffectOutcome::NoChange;
        }
        if !self.destination_parent_is_exact(expected_parent) {
            return EffectOutcome::NoChange;
        }
        #[cfg(test)]
        self.run_race_hook(purpose, FilesystemRaceBoundary::AfterInitialObservation);
        let (directory, name) = match open_parent(self.workspace, destination.components()) {
            Ok(value) => value,
            Err(_) => return EffectOutcome::NoChange,
        };
        #[cfg(test)]
        self.run_race_hook(purpose, FilesystemRaceBoundary::AfterParentOpened);
        if !self.held_destination_parent_is_current(expected_parent, &destination, &directory) {
            return EffectOutcome::NoChange;
        }
        let temporary = temporary_name(transaction_id, index, purpose, operation.postimage_bytes());
        let mode = operation.destination_mode().unwrap_or(u32::MAX);
        #[cfg(test)]
        self.run_race_hook(purpose, FilesystemRaceBoundary::BeforeStaging);
        if !self.held_destination_parent_is_current(expected_parent, &destination, &directory) {
            return EffectOutcome::NoChange;
        }
        let staged = match stage_file(&directory, &temporary, operation.postimage_bytes(), mode) {
            Ok(value) => value,
            Err(_) => return EffectOutcome::NoChange,
        };
        let staged_snapshot = match crate::snapshot(&staged, None) {
            Ok(value) => value,
            Err(_) => {
                drop(staged);
                let _ = remove_staged(&directory, &temporary);
                return EffectOutcome::NoChange;
            }
        };
        drop(staged);
        #[cfg(test)]
        self.run_race_hook(purpose, FilesystemRaceBoundary::AfterStaging);
        #[cfg(test)]
        self.run_race_hook(purpose, FilesystemRaceBoundary::BeforeCommit);
        if !self.held_destination_parent_is_current(expected_parent, &destination, &directory)
            || self.observe_optional_file(&destination).ok() != Some(None)
        {
            let _ = remove_staged(&directory, &temporary);
            return EffectOutcome::NoChange;
        }
        if renameat_with(
            &directory,
            temporary.as_str(),
            &directory,
            name.as_str(),
            RenameFlags::NOREPLACE,
        )
        .is_err()
        {
            let _ = remove_staged(&directory, &temporary);
            return EffectOutcome::NoChange;
        }
        #[cfg(test)]
        self.run_race_hook(purpose, FilesystemRaceBoundary::AfterCommit);
        if !parent_is_current(self.workspace, destination.components(), &directory) {
            return remove_created_entry(
                &directory,
                &name,
                &staged_snapshot,
                operation.postimage_bytes(),
                self.limits.maximum_file_bytes,
            );
        }
        if fsync(&directory).is_err() {
            return EffectOutcome::Uncertain;
        }
        #[cfg(test)]
        self.run_race_hook(purpose, FilesystemRaceBoundary::AfterCommitDurable);
        if !parent_is_current(self.workspace, destination.components(), &directory) {
            return remove_created_entry(
                &directory,
                &name,
                &staged_snapshot,
                operation.postimage_bytes(),
                self.limits.maximum_file_bytes,
            );
        }
        if !self.entry_matches(&destination, operation.postimage_bytes(), mode) {
            return EffectOutcome::Uncertain;
        }
        #[cfg(test)]
        self.run_race_hook(purpose, FilesystemRaceBoundary::AfterVerification);
        if parent_is_current(self.workspace, destination.components(), &directory) {
            EffectOutcome::Applied
        } else {
            remove_created_entry(
                &directory,
                &name,
                &staged_snapshot,
                operation.postimage_bytes(),
                self.limits.maximum_file_bytes,
            )
        }
    }

    fn move_exact(&self, operation: &FilesystemOperation, reverse: bool) -> EffectOutcome {
        let Some(source_target) = operation.source() else {
            return EffectOutcome::NoChange;
        };
        let Some(source_path) = source_target.workspace_path() else {
            return EffectOutcome::NoChange;
        };
        let destination_path = match destination_path(operation) {
            Ok(value) => value,
            Err(_) => return EffectOutcome::NoChange,
        };
        let (from_path, to_path) = if reverse {
            (&destination_path, source_path)
        } else {
            (source_path, &destination_path)
        };
        let expected_bytes = operation.postimage_bytes();
        let expected_mode = operation.destination_mode().unwrap_or(u32::MAX);
        let source_matches = if reverse {
            self.entry_matches(from_path, expected_bytes, expected_mode)
        } else {
            self.source_is_exact(operation)
        };
        if !source_matches || self.observe_optional_file(to_path).ok() != Some(None) {
            return EffectOutcome::NoChange;
        }
        if !reverse
            && !operation
                .destination_parent()
                .is_some_and(|parent| self.destination_parent_is_exact(parent))
        {
            return EffectOutcome::NoChange;
        }
        let (from_directory, from_name) = match open_parent(self.workspace, from_path.components())
        {
            Ok(value) => value,
            Err(_) => return EffectOutcome::NoChange,
        };
        let (to_directory, to_name) = match open_parent(self.workspace, to_path.components()) {
            Ok(value) => value,
            Err(_) => return EffectOutcome::NoChange,
        };
        let from_stat = match fstat(&from_directory) {
            Ok(value) => value,
            Err(_) => return EffectOutcome::NoChange,
        };
        let to_stat = match fstat(&to_directory) {
            Ok(value) => value,
            Err(_) => return EffectOutcome::NoChange,
        };
        if from_stat.st_dev != to_stat.st_dev {
            return EffectOutcome::NoChange;
        }
        if renameat_with(
            &from_directory,
            from_name.as_str(),
            &to_directory,
            to_name.as_str(),
            RenameFlags::NOREPLACE,
        )
        .is_err()
        {
            return EffectOutcome::NoChange;
        }
        if fsync(&from_directory).is_err() || fsync(&to_directory).is_err() {
            return EffectOutcome::Uncertain;
        }
        if self.observe_optional_file(from_path).ok() == Some(None)
            && self.entry_matches(to_path, expected_bytes, expected_mode)
        {
            EffectOutcome::Applied
        } else {
            EffectOutcome::Uncertain
        }
    }

    fn source_is_exact(&self, operation: &FilesystemOperation) -> bool {
        let Some(target) = operation.source() else {
            return false;
        };
        let Some(path) = target.workspace_path() else {
            return false;
        };
        self.observe_optional_file(path)
            .ok()
            .flatten()
            .is_some_and(|entry| {
                entry.target == *target
                    && entry.bytes == operation.source_bytes()
                    && Some(entry.mode) == operation.source_mode()
            })
    }

    fn destination_parent_is_exact(&self, expected: &GrantTarget) -> bool {
        match expected.workspace_path() {
            Some(path) => {
                self.adapter()
                    .resolve(self.workspace, path, PathResolutionIntent::ReadDirectory)
                    .ok()
                    .and_then(|held| GrantTarget::held_object(&held).ok())
                    .as_ref()
                    == Some(expected)
            }
            None => {
                self.workspace.revalidate().is_ok()
                    && GrantTarget::held_workspace_root(self.workspace).as_ref() == Ok(expected)
            }
        }
    }

    fn held_destination_parent_is_current(
        &self,
        expected: &GrantTarget,
        destination: &WorkspacePath,
        held_directory: &rustix::fd::OwnedFd,
    ) -> bool {
        self.destination_parent_is_exact(expected)
            && parent_is_current(self.workspace, destination.components(), held_directory)
    }

    fn entry_matches(&self, path: &WorkspacePath, expected: &[u8], mode: u32) -> bool {
        self.observe_optional_file(path)
            .ok()
            .flatten()
            .is_some_and(|entry| entry.bytes == expected && entry.mode == mode)
    }

    fn remove_exact_destination(&self, operation: &FilesystemOperation) -> EffectOutcome {
        let destination = match destination_path(operation) {
            Ok(value) => value,
            Err(_) => return EffectOutcome::NoChange,
        };
        let mode = operation.destination_mode().unwrap_or(u32::MAX);
        if !self.entry_matches(&destination, operation.postimage_bytes(), mode) {
            return EffectOutcome::NoChange;
        }
        let (directory, name) = match open_parent(self.workspace, destination.components()) {
            Ok(value) => value,
            Err(_) => return EffectOutcome::NoChange,
        };
        if unlinkat(&directory, name.as_str(), AtFlags::empty()).is_err() {
            return EffectOutcome::NoChange;
        }
        if fsync(&directory).is_err() {
            return EffectOutcome::Uncertain;
        }
        if self.observe_optional_file(&destination).ok() == Some(None) {
            EffectOutcome::Applied
        } else {
            EffectOutcome::Uncertain
        }
    }

    fn restore_operation(
        &mut self,
        transaction_id: &str,
        index: usize,
        operation: &FilesystemOperation,
    ) -> EffectOutcome {
        match operation.kind() {
            FilesystemOperationKind::Create | FilesystemOperationKind::Copy => {
                self.remove_exact_destination(operation)
            }
            FilesystemOperationKind::ExactPatch => {
                let Some(path) = operation.source().and_then(GrantTarget::workspace_path) else {
                    return EffectOutcome::NoChange;
                };
                let Some(current) = self.observe_optional_file(path).ok().flatten() else {
                    return EffectOutcome::NoChange;
                };
                match self.replacement_driver().replace_exact(
                    transaction_id,
                    index,
                    &current.target,
                    operation.postimage_bytes(),
                    operation.source_bytes(),
                    "restore-patch",
                ) {
                    ReplaceOutcome::Applied => EffectOutcome::Applied,
                    ReplaceOutcome::NoChange => EffectOutcome::NoChange,
                    ReplaceOutcome::Uncertain => EffectOutcome::Uncertain,
                }
            }
            FilesystemOperationKind::Move | FilesystemOperationKind::TrashDelete => {
                self.move_exact(operation, true)
            }
        }
    }

    #[cfg(test)]
    fn with_race_hook(mut self, hook: impl FnMut(FilesystemLifecycleEvent) + 'static) -> Self {
        self.race_hook = Some(Box::new(hook));
        self
    }

    #[cfg(test)]
    fn run_race_hook(&mut self, purpose: &str, boundary: FilesystemRaceBoundary) {
        if let Some(hook) = self.race_hook.as_mut() {
            hook(FilesystemLifecycleEvent {
                purpose: purpose.to_owned(),
                boundary,
            });
        }
    }
}

impl ControlledFilesystemDriver for LinuxControlledFilesystemDriver<'_> {
    fn observe(
        &mut self,
        plan: &FilesystemPlan,
    ) -> Result<Vec<FilesystemOperationObservation>, FilesystemDriverError> {
        self.observe_all(plan)
    }

    fn apply(&mut self, authorization: FilesystemApplyAuthorization<'_>) -> FilesystemApplyReport {
        if self.validate_limits(authorization.plan()).is_err() {
            return known_failure(0, FAILURE_APPLY);
        }
        let mut applied_indexes = Vec::new();
        for (index, operation) in authorization.plan().operations().iter().enumerate() {
            match self.apply_operation(authorization.transaction_id(), index, operation) {
                EffectOutcome::Applied => {
                    let Ok(index) = u32::try_from(index) else {
                        return uncertain_report(FAILURE_UNCERTAIN);
                    };
                    applied_indexes.push(index);
                }
                EffectOutcome::NoChange => {
                    let Ok(failure_index) = u32::try_from(index) else {
                        return uncertain_report(FAILURE_UNCERTAIN);
                    };
                    return FilesystemApplyReport {
                        atomic: false,
                        applied_indexes,
                        failure_index: Some(failure_index),
                        failure_code: Some(FAILURE_APPLY.to_owned()),
                        uncertain: false,
                    };
                }
                EffectOutcome::Uncertain => return uncertain_report(FAILURE_UNCERTAIN),
            }
        }
        FilesystemApplyReport {
            atomic: authorization.plan().operations().len() == 1,
            applied_indexes,
            failure_index: None,
            failure_code: None,
            uncertain: false,
        }
    }

    fn restore(
        &mut self,
        authorization: FilesystemRestoreAuthorization<'_>,
    ) -> FilesystemRestoreReport {
        let mut restored_indexes = Vec::new();
        for raw_index in authorization.restore_indexes() {
            let Ok(index) = usize::try_from(*raw_index) else {
                return uncertain_restore();
            };
            let Some(operation) = authorization.plan().operations().get(index) else {
                return uncertain_restore();
            };
            match self.restore_operation(authorization.transaction_id(), index, operation) {
                EffectOutcome::Applied => restored_indexes.push(*raw_index),
                EffectOutcome::NoChange | EffectOutcome::Uncertain => {
                    return FilesystemRestoreReport {
                        restored_indexes,
                        failure_code: Some(FAILURE_RESTORE.to_owned()),
                        uncertain: true,
                    };
                }
            }
        }
        FilesystemRestoreReport {
            restored_indexes,
            failure_code: None,
            uncertain: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EffectOutcome {
    Applied,
    NoChange,
    Uncertain,
}

fn remove_created_entry(
    directory: &rustix::fd::OwnedFd,
    name: &str,
    expected_snapshot: &crate::LinuxStatSnapshot,
    expected_bytes: &[u8],
    maximum_bytes: u64,
) -> EffectOutcome {
    if !held_file_matches(
        directory,
        name,
        expected_snapshot,
        expected_bytes,
        maximum_bytes,
    ) {
        return EffectOutcome::Uncertain;
    }
    if unlinkat(directory, name, AtFlags::empty()).is_err() || fsync(directory).is_err() {
        return EffectOutcome::Uncertain;
    }
    EffectOutcome::NoChange
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FilesystemRaceBoundary {
    AfterInitialObservation,
    AfterParentOpened,
    BeforeStaging,
    AfterStaging,
    BeforeCommit,
    AfterCommit,
    AfterCommitDurable,
    AfterVerification,
}

#[cfg(test)]
impl FilesystemRaceBoundary {
    const CREATE_ALL: [Self; 8] = [
        Self::AfterInitialObservation,
        Self::AfterParentOpened,
        Self::BeforeStaging,
        Self::AfterStaging,
        Self::BeforeCommit,
        Self::AfterCommit,
        Self::AfterCommitDurable,
        Self::AfterVerification,
    ];
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
struct FilesystemLifecycleEvent {
    purpose: String,
    boundary: FilesystemRaceBoundary,
}

fn destination_path(
    operation: &FilesystemOperation,
) -> Result<WorkspacePath, FilesystemDriverError> {
    let parent = operation
        .destination_parent()
        .ok_or(FilesystemDriverError::ObservationUnavailable)?;
    let display = operation
        .destination_path()
        .ok_or(FilesystemDriverError::ObservationUnavailable)?;
    let path = WorkspacePath::new(parent.workspace_id().clone(), display.split('/'))
        .map_err(|_| FilesystemDriverError::ObservationUnavailable)?;
    let canonical_display = path
        .components()
        .iter()
        .map(|component| component.as_str())
        .collect::<Vec<_>>()
        .join("/");
    if canonical_display != display
        || path.components().len() != parent.path_components().len() + 1
        || !path.components().starts_with(parent.path_components())
    {
        return Err(FilesystemDriverError::ObservationUnavailable);
    }
    Ok(path)
}

fn known_failure(index: u32, code: &str) -> FilesystemApplyReport {
    FilesystemApplyReport {
        atomic: false,
        applied_indexes: Vec::new(),
        failure_index: Some(index),
        failure_code: Some(code.to_owned()),
        uncertain: false,
    }
}

fn uncertain_report(code: &str) -> FilesystemApplyReport {
    FilesystemApplyReport {
        atomic: false,
        applied_indexes: Vec::new(),
        failure_index: None,
        failure_code: Some(code.to_owned()),
        uncertain: true,
    }
}

fn uncertain_restore() -> FilesystemRestoreReport {
    FilesystemRestoreReport {
        restored_indexes: Vec::new(),
        failure_code: Some(FAILURE_RESTORE.to_owned()),
        uncertain: true,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::collections::BTreeSet;
    use std::fs;
    use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
    use std::os::unix::net::UnixListener;
    use std::path::{Path, PathBuf};
    use std::rc::Rc;
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::{
        ActionId, ActionKind, ActorId, AdapterInstanceId, ApprovalId, DataSensitivity, GrantId,
        GrantNonce, GrantOperation, GrantTarget, OperationBinding, PathResolutionIntent,
        PlatformPathAdapter, SessionId, TaskId, ToolId, WorkspaceAuthorizationId, WorkspaceId,
        WorkspacePath, WorkspaceScopePath,
    };
    use agentmage_kernel_engine::filesystem_control::{
        ControlledFilesystemDriver, ExistingSourceDraft, ExistingWorkDisposition,
        FileClassification, FilesystemApprovalDecision, FilesystemApprovalReceipt,
        FilesystemGrantRequest, FilesystemOperationDraft, FilesystemPlan, FilesystemPlanRequest,
        FilesystemTransactionError, FilesystemTransactionOutcome, FilesystemTransactionRequest,
        NewDestinationDraft, StructuredPatch, StructuredPatchHunk, build_filesystem_plan,
        execute_filesystem_transaction, issue_filesystem_grant, render_filesystem_preview,
    };
    use agentmage_kernel_engine::grants::{GrantIssuer, SessionReadGrantRequest};
    use agentmage_kernel_engine::policy::{
        PolicyDocument, PolicyEngine, ScopeRules, ToolPolicyBinding,
    };
    use agentmage_kernel_engine::write_approval::WriteReviewNarrative;
    use rustix::fs::{CWD, Mode, mkfifoat};
    use sha2::{Digest, Sha256};

    use super::{
        FilesystemLifecycleEvent, FilesystemRaceBoundary, LinuxControlledFilesystemDriver,
        LinuxFilesystemDriverLimits, temporary_name,
    };
    use crate::{LinuxAuthorizedWorkspace, LinuxPathAdapter, authorize_workspace_root};

    static TEMP_ID: AtomicU64 = AtomicU64::new(1);
    const TRANSACTION_ID: &str = "linux-filesystem-transaction-0001";

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let id = TEMP_ID.fetch_add(1, Ordering::SeqCst);
            let path = std::env::temp_dir().join(format!(
                "agentmage-linux-filesystem-{}-{id}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("temporary filesystem root");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("temporary filesystem root removed");
        }
    }

    struct Fixture {
        root: TestDirectory,
        workspace: LinuxAuthorizedWorkspace,
        issuer: GrantIssuer,
        policy: PolicyEngine,
        plan: FilesystemPlan,
        approval: FilesystemApprovalReceipt,
    }

    fn rules<T: Ord>(values: impl IntoIterator<Item = T>) -> ScopeRules<T> {
        ScopeRules {
            allowed: values.into_iter().collect(),
            denied: BTreeSet::new(),
        }
    }

    fn review() -> WriteReviewNarrative {
        WriteReviewNarrative {
            rationale: "Apply exact Linux filesystem operations".to_owned(),
            behavior_change: "Only reviewed fixture paths change".to_owned(),
            verification_plan: vec!["Run focused Linux filesystem tests".to_owned()],
            risks: vec!["A fixture may depend on its prior location".to_owned()],
            rollback: "Restore exact approved pre-state".to_owned(),
            unverified_assumptions: vec!["No external fixture consumer was inspected".to_owned()],
        }
    }

    fn source_draft(
        adapter: &LinuxPathAdapter,
        workspace: &LinuxAuthorizedWorkspace,
        workspace_id: &WorkspaceId,
        components: &[&str],
        disposition: ExistingWorkDisposition,
    ) -> ExistingSourceDraft {
        let path = WorkspacePath::new(workspace_id.clone(), components.iter().copied())
            .expect("source path");
        let held = adapter
            .resolve(workspace, &path, PathResolutionIntent::ReadFile)
            .expect("held source");
        let absolute = workspace_absolute(workspace, components);
        ExistingSourceDraft {
            target: GrantTarget::held_object(&held).expect("source target"),
            observed_bytes: fs::read(&absolute).expect("source bytes"),
            work_disposition: disposition,
            mode: fs::metadata(absolute).expect("source metadata").mode() & 0o777,
        }
    }

    fn destination_draft(
        adapter: &LinuxPathAdapter,
        workspace: &LinuxAuthorizedWorkspace,
        workspace_id: &WorkspaceId,
        parent_components: &[&str],
        name: &str,
    ) -> NewDestinationDraft {
        let parent_path =
            WorkspacePath::new(workspace_id.clone(), parent_components.iter().copied())
                .expect("destination parent path");
        let held = adapter
            .resolve(workspace, &parent_path, PathResolutionIntent::ReadDirectory)
            .expect("held destination parent");
        let mut components = parent_components
            .iter()
            .map(|component| (*component).to_owned())
            .collect::<Vec<_>>();
        components.push(name.to_owned());
        let absolute_parent = workspace_absolute(workspace, parent_components);
        let mut siblings = fs::read_dir(absolute_parent)
            .expect("destination listing")
            .map(|entry| {
                entry
                    .expect("destination entry")
                    .file_name()
                    .into_string()
                    .expect("UTF-8 fixture name")
            })
            .collect::<Vec<_>>();
        siblings.sort_unstable();
        NewDestinationDraft {
            parent: GrantTarget::held_object(&held).expect("destination parent target"),
            path: WorkspacePath::new(workspace_id.clone(), components).expect("destination path"),
            observed_sibling_names: siblings,
        }
    }

    fn root_destination_draft(
        workspace: &LinuxAuthorizedWorkspace,
        workspace_id: &WorkspaceId,
        name: &str,
    ) -> NewDestinationDraft {
        let mut siblings = fs::read_dir(workspace_absolute(workspace, &[]))
            .expect("root destination listing")
            .map(|entry| {
                entry
                    .expect("root destination entry")
                    .file_name()
                    .into_string()
                    .expect("UTF-8 fixture name")
            })
            .collect::<Vec<_>>();
        siblings.sort_unstable();
        NewDestinationDraft {
            parent: GrantTarget::held_workspace_root(workspace).expect("held root target"),
            path: WorkspacePath::new(workspace_id.clone(), [name]).expect("root destination path"),
            observed_sibling_names: siblings,
        }
    }

    fn workspace_absolute(workspace: &LinuxAuthorizedWorkspace, components: &[&str]) -> PathBuf {
        let root = fs::read_link(format!(
            "/proc/self/fd/{}",
            std::os::fd::AsRawFd::as_raw_fd(&workspace.root_descriptor)
        ))
        .expect("workspace root link");
        components
            .iter()
            .fold(root, |path, component| path.join(component))
    }

    fn patch() -> Vec<u8> {
        serde_json::to_vec(&StructuredPatch {
            schema_version: 1,
            hunks: vec![StructuredPatchHunk {
                old_start_line: 2,
                old_lines: vec!["beta\n".to_owned()],
                new_lines: vec!["changed\n".to_owned()],
            }],
        })
        .expect("patch JSON")
    }

    fn hex_sha256(bytes: &[u8]) -> String {
        let digest = Sha256::digest(bytes);
        digest.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    fn fixture(delete: bool) -> Fixture {
        let root = TestDirectory::new();
        for directory in ["src", "new", "copies", "moved", "trash"] {
            fs::create_dir(root.path().join(directory)).expect("fixture directory");
        }
        for (name, bytes) in [
            ("patch.txt", b"alpha\nbeta\ngamma\n".as_slice()),
            ("copy.txt", b"copy\n".as_slice()),
            ("move.txt", b"move\n".as_slice()),
            ("obsolete.txt", b"obsolete\n".as_slice()),
        ] {
            let path = root.path().join("src").join(name);
            fs::write(&path, bytes).expect("source fixture");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).expect("source mode");
        }

        let workspace_id = WorkspaceId::from_raw("workspace-linux-filesystem");
        let adapter_id = AdapterInstanceId::from_raw("adapter-linux-filesystem");
        let workspace = authorize_workspace_root(
            root.path(),
            workspace_id.clone(),
            WorkspaceAuthorizationId::from_raw("authorization-linux-filesystem"),
            adapter_id.clone(),
        )
        .expect("fixture workspace");
        let adapter = LinuxPathAdapter::new(adapter_id, 1024 * 1024);
        let drafts = if delete {
            vec![FilesystemOperationDraft::TrashDelete {
                operation_id: "linux-operation-trash".to_owned(),
                source: source_draft(
                    &adapter,
                    &workspace,
                    &workspace_id,
                    &["src", "obsolete.txt"],
                    ExistingWorkDisposition::Clean,
                ),
                trash_destination: destination_draft(
                    &adapter,
                    &workspace,
                    &workspace_id,
                    &["trash"],
                    "obsolete.txt",
                ),
            }]
        } else {
            vec![
                FilesystemOperationDraft::Create {
                    operation_id: "linux-operation-create".to_owned(),
                    destination: root_destination_draft(&workspace, &workspace_id, "created.txt"),
                    content: b"created\n".to_vec(),
                    mode: 0o600,
                    classification: FileClassification::Documentation,
                },
                FilesystemOperationDraft::ExactPatch {
                    operation_id: "linux-operation-patch".to_owned(),
                    source: source_draft(
                        &adapter,
                        &workspace,
                        &workspace_id,
                        &["src", "patch.txt"],
                        ExistingWorkDisposition::Clean,
                    ),
                    patch_json: patch(),
                    expected_postimage_sha256: hex_sha256(b"alpha\nchanged\ngamma\n"),
                },
                FilesystemOperationDraft::Copy {
                    operation_id: "linux-operation-copy".to_owned(),
                    source: source_draft(
                        &adapter,
                        &workspace,
                        &workspace_id,
                        &["src", "copy.txt"],
                        ExistingWorkDisposition::Clean,
                    ),
                    destination: destination_draft(
                        &adapter,
                        &workspace,
                        &workspace_id,
                        &["copies"],
                        "copy.txt",
                    ),
                    classification: FileClassification::Data,
                },
                FilesystemOperationDraft::Move {
                    operation_id: "linux-operation-move".to_owned(),
                    source: source_draft(
                        &adapter,
                        &workspace,
                        &workspace_id,
                        &["src", "move.txt"],
                        ExistingWorkDisposition::OwnedByCurrentTask,
                    ),
                    destination: destination_draft(
                        &adapter,
                        &workspace,
                        &workspace_id,
                        &["moved"],
                        "move.txt",
                    ),
                },
            ]
        };
        let policy_targets = drafts
            .iter()
            .flat_map(|draft| match draft {
                FilesystemOperationDraft::Create { destination, .. } => {
                    vec![destination.parent.clone()]
                }
                FilesystemOperationDraft::ExactPatch { source, .. } => {
                    vec![source.target.clone()]
                }
                FilesystemOperationDraft::Copy {
                    source,
                    destination,
                    ..
                }
                | FilesystemOperationDraft::Move {
                    source,
                    destination,
                    ..
                } => vec![source.target.clone(), destination.parent.clone()],
                FilesystemOperationDraft::TrashDelete {
                    source,
                    trash_destination,
                    ..
                } => vec![source.target.clone(), trash_destination.parent.clone()],
            })
            .collect::<Vec<_>>();
        let operation = OperationBinding::new(if delete {
            GrantOperation::WorkspaceDelete
        } else {
            GrantOperation::WorkspaceWrite
        });
        let action_id = ActionId::from_raw("action-linux-filesystem");
        let tool_id = ToolId::from_raw("workspace.linux-filesystem");
        let policy = PolicyEngine::new(PolicyDocument {
            schema_version: 1,
            revision: 1,
            actors: rules([ActorId::from_raw("actor-local")]),
            tasks: rules([TaskId::from_raw("task-linux-filesystem")]),
            actions: rules([action_id.clone()]),
            tools: rules([ToolPolicyBinding {
                tool_id: tool_id.clone(),
                tool_version: "1.0.0".to_owned(),
            }]),
            operations: rules([operation]),
            targets: rules(policy_targets),
            denied_argument_sha256s: BTreeSet::new(),
            denied_preimage_sha256s: BTreeSet::new(),
            network_scopes: ScopeRules::deny_all(),
            credential_scopes: ScopeRules::deny_all(),
            publication_scopes: ScopeRules::deny_all(),
        })
        .expect("filesystem policy");
        let mut issuer = GrantIssuer::new();
        let parent = issuer
            .issue_session_read(SessionReadGrantRequest {
                grant_id: GrantId::from_raw("grant-parent-linux-filesystem"),
                actor_id: ActorId::from_raw("actor-local"),
                session_id: SessionId::from_raw("session-linux-filesystem"),
                task_id: TaskId::from_raw("task-linux-filesystem"),
                targets: vec![
                    GrantTarget::workspace_scope(
                        &workspace,
                        WorkspaceScopePath::new(workspace_id.clone(), Vec::<String>::new())
                            .expect("root scope"),
                    )
                    .expect("root target"),
                ],
                excluded_targets: Vec::new(),
                sensitivity: DataSensitivity::Restricted,
                issued_at_epoch_ms: 1_000,
                expires_at_epoch_ms: 100_000,
                nonce: GrantNonce::from_raw("nonce-parent-linux-filesystem"),
                maximum_derived_operations: 2,
                preview_sha256: "a".repeat(64),
                policy_sha256: policy.policy_sha256().to_owned(),
            })
            .expect("parent grant");
        let plan = build_filesystem_plan(
            &parent,
            FilesystemPlanRequest {
                plan_id: "linux-filesystem-plan".to_owned(),
                observed_at_epoch_ms: 2_000,
                operations: drafts,
                review: review(),
                permitted_verification: vec!["cargo-test-linux-filesystem".to_owned()],
            },
        )
        .expect("filesystem plan");
        let preview = render_filesystem_preview(&plan).expect("filesystem preview");
        let approval = issue_filesystem_grant(
            &mut issuer,
            &plan,
            &preview,
            &FilesystemApprovalDecision {
                approval_id: ApprovalId::from_raw("approval-linux-filesystem"),
                approved_plan_sha256: preview.plan_sha256.clone(),
                approved_preview_sha256: preview.preview_sha256.clone(),
                approved_at_epoch_ms: 3_000,
                expires_at_epoch_ms: 30_000,
                permitted_verification: preview.permitted_verification.clone(),
                user_confirmed: true,
                high_risk_delete_confirmed: delete,
            },
            FilesystemGrantRequest {
                parent_grant_id: parent.grant_id,
                grant_id: GrantId::from_raw("grant-linux-filesystem"),
                action_id,
                action_kind: ActionKind::DeterministicTool,
                tool_id,
                tool_version: "1.0.0".to_owned(),
                nonce: GrantNonce::from_raw("nonce-linux-filesystem"),
                policy_sha256: policy.policy_sha256().to_owned(),
            },
        )
        .expect("filesystem approval");
        Fixture {
            root,
            workspace,
            issuer,
            policy,
            plan,
            approval,
        }
    }

    fn execute(
        fixture: &mut Fixture,
    ) -> Result<FilesystemTransactionOutcome, FilesystemTransactionError> {
        let mut driver = LinuxControlledFilesystemDriver::new(
            &fixture.workspace,
            LinuxFilesystemDriverLimits {
                maximum_file_bytes: 1024 * 1024,
                maximum_transaction_bytes: 4 * 1024 * 1024,
                ..LinuxFilesystemDriverLimits::default()
            },
        );
        execute_filesystem_transaction(
            &mut fixture.issuer,
            &fixture.policy,
            &fixture.plan,
            &fixture.approval,
            FilesystemTransactionRequest {
                transaction_id: TRANSACTION_ID.to_owned(),
                now_epoch_ms: 4_000,
                cancelled_before_consume: false,
            },
            &mut driver,
        )
        .map(|result| result.outcome)
    }

    #[test]
    fn native_create_patch_copy_move_and_trash_commit_exact_state() {
        let mut regular = fixture(false);
        let neighbor = regular.root.path().join("new/neighbor.txt");
        fs::write(&neighbor, b"untouched\n").expect("neighbor fixture");
        assert_eq!(
            execute(&mut regular),
            Ok(FilesystemTransactionOutcome::Committed)
        );
        assert_eq!(
            fs::read(regular.root.path().join("created.txt")).expect("created bytes"),
            b"created\n"
        );
        assert_eq!(
            fs::metadata(regular.root.path().join("created.txt"))
                .expect("created metadata")
                .mode()
                & 0o777,
            0o600
        );
        assert_eq!(
            fs::read(regular.root.path().join("src/patch.txt")).expect("patched bytes"),
            b"alpha\nchanged\ngamma\n"
        );
        assert_eq!(
            fs::read(regular.root.path().join("src/copy.txt")).expect("copy source"),
            b"copy\n"
        );
        assert_eq!(
            fs::read(regular.root.path().join("copies/copy.txt")).expect("copy destination"),
            b"copy\n"
        );
        assert!(!regular.root.path().join("src/move.txt").exists());
        assert_eq!(
            fs::read(regular.root.path().join("moved/move.txt")).expect("move destination"),
            b"move\n"
        );
        assert_eq!(fs::read(neighbor).expect("neighbor"), b"untouched\n");

        let mut delete = fixture(true);
        assert_eq!(
            execute(&mut delete),
            Ok(FilesystemTransactionOutcome::Committed)
        );
        assert!(!delete.root.path().join("src/obsolete.txt").exists());
        assert_eq!(
            fs::read(delete.root.path().join("trash/obsolete.txt")).expect("trash destination"),
            b"obsolete\n"
        );
    }

    #[test]
    fn native_collision_symlink_hardlink_and_limits_fail_without_effect() {
        let mut collision = fixture(false);
        fs::write(collision.root.path().join("CREATED.TXT"), b"owner\n").expect("case collision");
        assert_eq!(
            execute(&mut collision),
            Err(FilesystemTransactionError::PreapplyDenied)
        );
        assert!(!collision.root.path().join("created.txt").exists());

        let mut symlink_fixture = fixture(false);
        let outside = symlink_fixture.root.path().join("outside.txt");
        fs::write(&outside, b"outside\n").expect("outside fixture");
        symlink(&outside, symlink_fixture.root.path().join("created.txt"))
            .expect("destination symlink");
        assert_eq!(
            execute(&mut symlink_fixture),
            Err(FilesystemTransactionError::ObservationFailed)
        );
        assert_eq!(fs::read(outside).expect("outside unchanged"), b"outside\n");

        let mut hardlink_fixture = fixture(false);
        fs::hard_link(
            hardlink_fixture.root.path().join("src/copy.txt"),
            hardlink_fixture.root.path().join("src/copy-hard.txt"),
        )
        .expect("hardlink fixture");
        assert_eq!(
            execute(&mut hardlink_fixture),
            Err(FilesystemTransactionError::ObservationFailed)
        );
        assert_eq!(
            fs::metadata(hardlink_fixture.root.path().join("src/copy.txt"))
                .expect("hardlink metadata")
                .nlink(),
            2
        );

        let limited = fixture(false);
        let mut driver = LinuxControlledFilesystemDriver::new(
            &limited.workspace,
            LinuxFilesystemDriverLimits {
                maximum_operations: 1,
                ..LinuxFilesystemDriverLimits::default()
            },
        );
        assert_eq!(
            ControlledFilesystemDriver::observe(&mut driver, &limited.plan),
            Err(agentmage_kernel_engine::filesystem_control::FilesystemDriverError::ApplyFailed)
        );
    }

    #[test]
    fn native_later_staging_failure_restores_prior_create_exactly() {
        let mut fixture = fixture(false);
        let patch_operation = &fixture.plan.operations()[1];
        let collision_name = temporary_name(
            TRANSACTION_ID,
            1,
            "patch",
            patch_operation.postimage_bytes(),
        );
        let collision = fixture.root.path().join("src").join(collision_name);
        fs::write(&collision, b"unrelated temp owner\n").expect("staging collision");

        assert_eq!(
            execute(&mut fixture),
            Ok(FilesystemTransactionOutcome::Restored)
        );
        assert!(!fixture.root.path().join("new/created.txt").exists());
        assert_eq!(
            fs::read(fixture.root.path().join("src/patch.txt")).expect("original patch source"),
            b"alpha\nbeta\ngamma\n"
        );
        assert_eq!(
            fs::read(collision).expect("collision owner preserved"),
            b"unrelated temp owner\n"
        );
    }

    #[test]
    fn s_030_ut01_missing_and_wrong_type_matrix_fails_before_effect() {
        for source in ["patch.txt", "copy.txt", "move.txt"] {
            let mut missing = fixture(false);
            fs::remove_file(missing.root.path().join("src").join(source))
                .expect("remove source fixture");
            assert_eq!(
                execute(&mut missing),
                Err(FilesystemTransactionError::PreapplyDenied),
                "missing {source}"
            );
            assert!(!missing.root.path().join("created.txt").exists());

            let mut wrong_type = fixture(false);
            let source_path = wrong_type.root.path().join("src").join(source);
            fs::remove_file(&source_path).expect("remove regular source");
            fs::create_dir(&source_path).expect("replace source with directory");
            assert_eq!(
                execute(&mut wrong_type),
                Err(FilesystemTransactionError::ObservationFailed),
                "wrong-type {source}"
            );
            assert!(source_path.is_dir());
            assert!(!wrong_type.root.path().join("created.txt").exists());
        }

        for parent in ["copies", "moved"] {
            let mut missing = fixture(false);
            fs::remove_dir(missing.root.path().join(parent)).expect("remove destination parent");
            assert_eq!(
                execute(&mut missing),
                Err(FilesystemTransactionError::ObservationFailed),
                "missing parent {parent}"
            );
            assert!(!missing.root.path().join("created.txt").exists());

            let mut wrong_type = fixture(false);
            let parent_path = wrong_type.root.path().join(parent);
            fs::remove_dir(&parent_path).expect("remove directory parent");
            fs::write(&parent_path, b"wrong parent type\n").expect("write parent file");
            assert_eq!(
                execute(&mut wrong_type),
                Err(FilesystemTransactionError::ObservationFailed),
                "wrong-type parent {parent}"
            );
            assert_eq!(
                fs::read(parent_path).expect("wrong parent preserved"),
                b"wrong parent type\n"
            );
        }

        for destination in ["created.txt", "copies/copy.txt", "moved/move.txt"] {
            let mut wrong_type = fixture(false);
            let destination_path = wrong_type.root.path().join(destination);
            fs::create_dir(&destination_path).expect("create directory destination");
            assert_eq!(
                execute(&mut wrong_type),
                Err(FilesystemTransactionError::ObservationFailed),
                "wrong-type destination {destination}"
            );
            assert!(destination_path.is_dir());
        }

        let mut missing_delete = fixture(true);
        fs::remove_file(missing_delete.root.path().join("src/obsolete.txt"))
            .expect("remove delete source");
        assert_eq!(
            execute(&mut missing_delete),
            Err(FilesystemTransactionError::PreapplyDenied)
        );
        assert!(
            !missing_delete
                .root
                .path()
                .join("trash/obsolete.txt")
                .exists()
        );

        let mut wrong_delete_source = fixture(true);
        let delete_source = wrong_delete_source.root.path().join("src/obsolete.txt");
        fs::remove_file(&delete_source).expect("remove delete source file");
        fs::create_dir(&delete_source).expect("replace delete source with directory");
        assert_eq!(
            execute(&mut wrong_delete_source),
            Err(FilesystemTransactionError::ObservationFailed)
        );
        assert!(delete_source.is_dir());

        let mut missing_trash = fixture(true);
        fs::remove_dir(missing_trash.root.path().join("trash")).expect("remove trash parent");
        assert_eq!(
            execute(&mut missing_trash),
            Err(FilesystemTransactionError::ObservationFailed)
        );
        assert!(missing_trash.root.path().join("src/obsolete.txt").exists());

        let mut wrong_trash_destination = fixture(true);
        let trash_destination = wrong_trash_destination
            .root
            .path()
            .join("trash/obsolete.txt");
        fs::create_dir(&trash_destination).expect("create directory trash destination");
        assert_eq!(
            execute(&mut wrong_trash_destination),
            Err(FilesystemTransactionError::ObservationFailed)
        );
        assert!(trash_destination.is_dir());
        assert!(
            wrong_trash_destination
                .root
                .path()
                .join("src/obsolete.txt")
                .exists()
        );
    }

    #[test]
    fn s_030_st01_socket_and_fifo_substitutions_are_refused_and_preserved() {
        for source in ["patch.txt", "copy.txt", "move.txt"] {
            let mut socket_fixture = fixture(false);
            let source_path = socket_fixture.root.path().join("src").join(source);
            fs::remove_file(&source_path).expect("remove regular source");
            let socket = UnixListener::bind(&source_path).expect("bind source socket");
            assert_eq!(
                execute(&mut socket_fixture),
                Err(FilesystemTransactionError::ObservationFailed),
                "socket source {source}"
            );
            assert!(source_path.exists());
            assert!(!socket_fixture.root.path().join("created.txt").exists());
            drop(socket);

            let mut fifo_fixture = fixture(false);
            let source_path = fifo_fixture.root.path().join("src").join(source);
            fs::remove_file(&source_path).expect("remove regular source");
            mkfifoat(CWD, &source_path, Mode::RUSR | Mode::WUSR).expect("create source FIFO");
            assert_eq!(
                execute(&mut fifo_fixture),
                Err(FilesystemTransactionError::ObservationFailed),
                "FIFO source {source}"
            );
            assert!(source_path.exists());
            assert!(!fifo_fixture.root.path().join("created.txt").exists());
        }

        for parent in ["copies", "moved"] {
            let mut socket_fixture = fixture(false);
            let parent_path = socket_fixture.root.path().join(parent);
            fs::remove_dir(&parent_path).expect("remove directory parent");
            let socket = UnixListener::bind(&parent_path).expect("bind parent socket");
            assert_eq!(
                execute(&mut socket_fixture),
                Err(FilesystemTransactionError::ObservationFailed),
                "socket parent {parent}"
            );
            assert!(parent_path.exists());
            drop(socket);

            let mut fifo_fixture = fixture(false);
            let parent_path = fifo_fixture.root.path().join(parent);
            fs::remove_dir(&parent_path).expect("remove directory parent");
            mkfifoat(CWD, &parent_path, Mode::RUSR | Mode::WUSR).expect("create parent FIFO");
            assert_eq!(
                execute(&mut fifo_fixture),
                Err(FilesystemTransactionError::ObservationFailed),
                "FIFO parent {parent}"
            );
            assert!(parent_path.exists());
        }

        for destination in ["created.txt", "copies/copy.txt", "moved/move.txt"] {
            let mut socket_fixture = fixture(false);
            let destination_path = socket_fixture.root.path().join(destination);
            let socket = UnixListener::bind(&destination_path).expect("bind destination socket");
            assert_eq!(
                execute(&mut socket_fixture),
                Err(FilesystemTransactionError::ObservationFailed),
                "socket destination {destination}"
            );
            assert!(destination_path.exists());
            drop(socket);

            let mut fifo_fixture = fixture(false);
            let destination_path = fifo_fixture.root.path().join(destination);
            mkfifoat(CWD, &destination_path, Mode::RUSR | Mode::WUSR)
                .expect("create destination FIFO");
            assert_eq!(
                execute(&mut fifo_fixture),
                Err(FilesystemTransactionError::ObservationFailed),
                "FIFO destination {destination}"
            );
            assert!(destination_path.exists());
        }
    }

    #[test]
    fn s_030_st01_copy_parent_rename_at_every_boundary_preserves_both_owners() {
        for boundary in FilesystemRaceBoundary::CREATE_ALL {
            let mut fixture = fixture(false);
            let canonical_parent = fixture.root.path().join("copies");
            let authorized_parent = fixture.root.path().join("copies-authorized");
            let fired = Rc::new(Cell::new(false));
            let hook_fired = Rc::clone(&fired);
            let hook_canonical = canonical_parent.clone();
            let hook_authorized = authorized_parent.clone();
            let mut driver = LinuxControlledFilesystemDriver::new(
                &fixture.workspace,
                LinuxFilesystemDriverLimits {
                    maximum_file_bytes: 1024 * 1024,
                    maximum_transaction_bytes: 4 * 1024 * 1024,
                    ..LinuxFilesystemDriverLimits::default()
                },
            )
            .with_race_hook(move |event: FilesystemLifecycleEvent| {
                if event.purpose == "copy"
                    && event.boundary == boundary
                    && !hook_fired.replace(true)
                {
                    fs::rename(&hook_canonical, &hook_authorized)
                        .expect("rename authorized copy parent");
                    fs::create_dir(&hook_canonical).expect("create replacement copy parent");
                    fs::write(hook_canonical.join("owner.txt"), b"competing owner\n")
                        .expect("write competing owner");
                }
            });
            let outcome = execute_filesystem_transaction(
                &mut fixture.issuer,
                &fixture.policy,
                &fixture.plan,
                &fixture.approval,
                FilesystemTransactionRequest {
                    transaction_id: TRANSACTION_ID.to_owned(),
                    now_epoch_ms: 4_000,
                    cancelled_before_consume: false,
                },
                &mut driver,
            )
            .map(|result| result.outcome);

            assert!(fired.get(), "hook did not fire at {boundary:?}");
            assert!(
                matches!(
                    outcome,
                    Ok(FilesystemTransactionOutcome::Restored)
                        | Ok(FilesystemTransactionOutcome::Uncertain)
                ),
                "unexpected transaction outcome at {boundary:?}: {outcome:?}"
            );
            assert_eq!(
                fs::read(canonical_parent.join("owner.txt")).expect("competing owner preserved"),
                b"competing owner\n",
                "canonical replacement changed at {boundary:?}"
            );
            assert_eq!(
                fs::read_dir(&authorized_parent)
                    .expect("authorized parent listing")
                    .count(),
                0,
                "authorized parent retained an effect at {boundary:?}"
            );
            assert_eq!(
                fs::read(fixture.root.path().join("src/copy.txt")).expect("copy source"),
                b"copy\n"
            );
            assert!(!fixture.root.path().join("created.txt").exists());
            assert_eq!(
                fs::read(fixture.root.path().join("src/patch.txt")).expect("patch restored"),
                b"alpha\nbeta\ngamma\n"
            );
        }
    }
}
