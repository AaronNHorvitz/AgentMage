//! Descriptor-relative Linux driver for exact controlled-write transactions.

use std::fmt::Write as _;
use std::fs::File;
use std::io::Write;

use agentmage_kernel_contracts::{GrantTarget, PlatformPathAdapter};
use agentmage_kernel_engine::write_approval::{CurrentWriteTarget, ShadowChangeSet};
use agentmage_kernel_engine::write_transaction::{
    AtomicWriteDriver, WriteApplyAuthorization, WriteApplyReport, WriteDriverError,
    WriteRestoreAuthorization, WriteRestoreReport,
};
use rustix::fd::OwnedFd;
use rustix::fs::{
    AtFlags, FileType, Mode, OFlags, RenameFlags, fchmod, fstat, fsync, openat, renameat_with,
    unlinkat,
};
use rustix::io::{Errno, pread};
use sha2::{Digest, Sha256};

use crate::{
    LinuxAuthorizedWorkspace, LinuxHeldObject, LinuxPathAdapter, LinuxStatSnapshot, open_relative,
    same_mount, select_strategy, snapshot,
};

const HASH_BUFFER_BYTES: usize = 64 * 1024;
const FAILURE_APPLY: &str = "write.linux.apply_failed";
const FAILURE_RESTORE: &str = "write.linux.restore_failed";
const FAILURE_UNCERTAIN: &str = "write.linux.state_uncertain";

/// Closed resource limits for one Linux controlled-write driver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxAtomicWriteDriverLimits {
    /// Maximum number of exact files in one transaction.
    pub maximum_operations: usize,
    /// Maximum bytes admitted for one preimage or postimage.
    pub maximum_file_bytes: u64,
    /// Maximum aggregate preimage plus postimage bytes.
    pub maximum_transaction_bytes: u64,
}

impl Default for LinuxAtomicWriteDriverLimits {
    fn default() -> Self {
        Self {
            maximum_operations: 128,
            maximum_file_bytes: 64 * 1024 * 1024,
            maximum_transaction_bytes: 128 * 1024 * 1024,
        }
    }
}

/// Linux platform driver that can act only through Sprint 36's opaque authorization.
pub struct LinuxAtomicWriteDriver<'workspace> {
    workspace: &'workspace LinuxAuthorizedWorkspace,
    limits: LinuxAtomicWriteDriverLimits,
}

impl std::fmt::Debug for LinuxAtomicWriteDriver<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LinuxAtomicWriteDriver")
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}

impl<'workspace> LinuxAtomicWriteDriver<'workspace> {
    /// Creates an inert driver for one already authorized workspace.
    #[must_use]
    pub const fn new(
        workspace: &'workspace LinuxAuthorizedWorkspace,
        limits: LinuxAtomicWriteDriverLimits,
    ) -> Self {
        Self { workspace, limits }
    }

    fn adapter(&self) -> LinuxPathAdapter {
        LinuxPathAdapter::new(
            self.workspace.adapter_instance_id.clone(),
            self.limits.maximum_file_bytes,
        )
    }

    fn validate_limits(&self, change_set: &ShadowChangeSet) -> Result<(), WriteDriverError> {
        if change_set.operations().is_empty()
            || change_set.operations().len() > self.limits.maximum_operations
        {
            return Err(WriteDriverError::ApplyFailed);
        }
        let mut aggregate = 0_u64;
        for operation in change_set.operations() {
            let before = u64::try_from(operation.preimage_bytes().len())
                .map_err(|_| WriteDriverError::ApplyFailed)?;
            let after = u64::try_from(operation.proposed_bytes().len())
                .map_err(|_| WriteDriverError::ApplyFailed)?;
            if before > self.limits.maximum_file_bytes || after > self.limits.maximum_file_bytes {
                return Err(WriteDriverError::ApplyFailed);
            }
            aggregate = aggregate
                .checked_add(before)
                .and_then(|value| value.checked_add(after))
                .ok_or(WriteDriverError::ApplyFailed)?;
        }
        if aggregate > self.limits.maximum_transaction_bytes {
            return Err(WriteDriverError::ApplyFailed);
        }
        Ok(())
    }

    fn observe_all(
        &self,
        change_set: &ShadowChangeSet,
    ) -> Result<Vec<CurrentWriteTarget>, WriteDriverError> {
        self.validate_limits(change_set)?;
        change_set
            .operations()
            .iter()
            .map(|operation| {
                let path = operation
                    .target()
                    .workspace_path()
                    .ok_or(WriteDriverError::ObservationUnavailable)?;
                let held = self
                    .adapter()
                    .resolve(
                        self.workspace,
                        path,
                        agentmage_kernel_contracts::PathResolutionIntent::ReadFile,
                    )
                    .map_err(|_| WriteDriverError::ObservationUnavailable)?;
                let bytes = read_held_bytes(&held, self.limits.maximum_file_bytes)?;
                let target = GrantTarget::held_object(&held)
                    .map_err(|_| WriteDriverError::ObservationUnavailable)?;
                Ok(CurrentWriteTarget { target, bytes })
            })
            .collect()
    }

    pub(super) fn replace_exact(
        &self,
        transaction_id: &str,
        index: usize,
        target: &GrantTarget,
        expected: &[u8],
        replacement: &[u8],
        purpose: &str,
    ) -> ReplaceOutcome {
        let Some(path) = target.workspace_path() else {
            return ReplaceOutcome::NoChange;
        };
        let adapter = self.adapter();
        let held = match adapter.resolve(
            self.workspace,
            path,
            agentmage_kernel_contracts::PathResolutionIntent::ReadFile,
        ) {
            Ok(value) => value,
            Err(_) => return ReplaceOutcome::NoChange,
        };
        let current_target = match GrantTarget::held_object(&held) {
            Ok(value) => value,
            Err(_) => return ReplaceOutcome::NoChange,
        };
        let current = match read_held_bytes(&held, self.limits.maximum_file_bytes) {
            Ok(value) => value,
            Err(_) => return ReplaceOutcome::NoChange,
        };
        if &current_target != target || current != expected {
            return ReplaceOutcome::NoChange;
        }

        let (directory, target_name) = match open_parent(self.workspace, path.components()) {
            Ok(value) => value,
            Err(_) => return ReplaceOutcome::NoChange,
        };
        let original_snapshot = match snapshot(&held.object_descriptor, None) {
            Ok(value) => value,
            Err(_) => return ReplaceOutcome::NoChange,
        };
        let temporary = temporary_name(transaction_id, index, purpose, replacement);
        let staged = match stage_file(
            &directory,
            &temporary,
            replacement,
            original_snapshot.mode & 0o7777,
        ) {
            Ok(value) => value,
            Err(_) => return ReplaceOutcome::NoChange,
        };
        drop(staged);

        let fresh = adapter.resolve(
            self.workspace,
            path,
            agentmage_kernel_contracts::PathResolutionIntent::ReadFile,
        );
        if !fresh.as_ref().is_ok_and(|value| {
            GrantTarget::held_object(value).as_ref() == Ok(target)
                && read_held_bytes(value, self.limits.maximum_file_bytes).as_deref() == Ok(expected)
        }) {
            let _ = remove_staged(&directory, &temporary);
            return ReplaceOutcome::NoChange;
        }

        if renameat_with(
            &directory,
            temporary.as_str(),
            &directory,
            target_name.as_str(),
            RenameFlags::EXCHANGE,
        )
        .is_err()
        {
            let _ = remove_staged(&directory, &temporary);
            return ReplaceOutcome::NoChange;
        }

        let displaced_matches = open_regular(&directory, &temporary).is_ok_and(|displaced| {
            snapshot(&displaced, None)
                .is_ok_and(|value| stable_replacement_identity(&original_snapshot, &value))
                && read_descriptor(&displaced, self.limits.maximum_file_bytes).as_deref()
                    == Ok(expected)
        });
        if !displaced_matches {
            return if exchange_back(&directory, &temporary, &target_name).is_ok() {
                ReplaceOutcome::NoChange
            } else {
                ReplaceOutcome::Uncertain
            };
        }
        if fsync(&directory).is_err()
            || unlinkat(&directory, temporary.as_str(), AtFlags::empty()).is_err()
            || fsync(&directory).is_err()
        {
            return ReplaceOutcome::Uncertain;
        }
        ReplaceOutcome::Applied
    }
}

impl AtomicWriteDriver for LinuxAtomicWriteDriver<'_> {
    fn observe(
        &mut self,
        change_set: &ShadowChangeSet,
    ) -> Result<Vec<CurrentWriteTarget>, WriteDriverError> {
        self.observe_all(change_set)
    }

    fn apply(&mut self, authorization: WriteApplyAuthorization<'_>) -> WriteApplyReport {
        if self.validate_limits(authorization.change_set()).is_err() {
            return known_failure(0, FAILURE_APPLY);
        }
        let mut applied_indexes = Vec::new();
        for (index, operation) in authorization.change_set().operations().iter().enumerate() {
            match self.replace_exact(
                authorization.transaction_id(),
                index,
                operation.target(),
                operation.preimage_bytes(),
                operation.proposed_bytes(),
                "apply",
            ) {
                ReplaceOutcome::Applied => {
                    let Ok(index) = u32::try_from(index) else {
                        return uncertain_report(FAILURE_UNCERTAIN);
                    };
                    applied_indexes.push(index);
                }
                ReplaceOutcome::NoChange => {
                    let Ok(failure_index) = u32::try_from(index) else {
                        return uncertain_report(FAILURE_UNCERTAIN);
                    };
                    return WriteApplyReport {
                        atomic: false,
                        applied_indexes,
                        failure_index: Some(failure_index),
                        failure_code: Some(FAILURE_APPLY.to_owned()),
                        uncertain: false,
                    };
                }
                ReplaceOutcome::Uncertain => return uncertain_report(FAILURE_UNCERTAIN),
            }
        }
        WriteApplyReport {
            atomic: authorization.change_set().operations().len() == 1,
            applied_indexes,
            failure_index: None,
            failure_code: None,
            uncertain: false,
        }
    }

    fn restore(&mut self, authorization: WriteRestoreAuthorization<'_>) -> WriteRestoreReport {
        let mut restored_indexes = Vec::new();
        for raw_index in authorization.restore_indexes() {
            let Ok(index) = usize::try_from(*raw_index) else {
                return uncertain_restore();
            };
            let Some(operation) = authorization.change_set().operations().get(index) else {
                return uncertain_restore();
            };
            let current = match self.observe_all(authorization.change_set()) {
                Ok(values) => values,
                Err(_) => return uncertain_restore(),
            };
            let Some(current_target) = current.get(index).map(|value| &value.target) else {
                return uncertain_restore();
            };
            match self.replace_exact(
                authorization.transaction_id(),
                index,
                current_target,
                operation.proposed_bytes(),
                operation.preimage_bytes(),
                "restore",
            ) {
                ReplaceOutcome::Applied => restored_indexes.push(*raw_index),
                ReplaceOutcome::NoChange | ReplaceOutcome::Uncertain => {
                    return WriteRestoreReport {
                        restored_indexes,
                        failure_code: Some(FAILURE_RESTORE.to_owned()),
                        uncertain: true,
                    };
                }
            }
        }
        WriteRestoreReport {
            restored_indexes,
            failure_code: None,
            uncertain: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ReplaceOutcome {
    Applied,
    NoChange,
    Uncertain,
}

fn stable_replacement_identity(expected: &LinuxStatSnapshot, observed: &LinuxStatSnapshot) -> bool {
    expected.same_object(observed)
        && expected.link_count == observed.link_count
        && expected.mode == observed.mode
        && expected.size == observed.size
}

fn known_failure(index: u32, code: &str) -> WriteApplyReport {
    WriteApplyReport {
        atomic: false,
        applied_indexes: Vec::new(),
        failure_index: Some(index),
        failure_code: Some(code.to_owned()),
        uncertain: false,
    }
}

fn uncertain_report(code: &str) -> WriteApplyReport {
    WriteApplyReport {
        atomic: false,
        applied_indexes: Vec::new(),
        failure_index: None,
        failure_code: Some(code.to_owned()),
        uncertain: true,
    }
}

fn uncertain_restore() -> WriteRestoreReport {
    WriteRestoreReport {
        restored_indexes: Vec::new(),
        failure_code: Some(FAILURE_RESTORE.to_owned()),
        uncertain: true,
    }
}

pub(super) fn open_parent(
    workspace: &LinuxAuthorizedWorkspace,
    components: &[agentmage_kernel_contracts::WorkspacePathComponent],
) -> Result<(OwnedFd, String), WriteDriverError> {
    let Some((name, parents)) = components.split_last() else {
        return Err(WriteDriverError::ApplyFailed);
    };
    let root_now =
        snapshot(&workspace.root_descriptor, None).map_err(|_| WriteDriverError::ApplyFailed)?;
    if !workspace.root_snapshot.same_object(&root_now) || root_now.file_type != FileType::Directory
    {
        return Err(WriteDriverError::ApplyFailed);
    }
    let strategy = select_strategy(
        &workspace.root_descriptor,
        &root_now,
        crate::ResolverPreference::Auto,
    )
    .map_err(|_| WriteDriverError::ApplyFailed)?;
    let mut directory = workspace
        .root_descriptor
        .try_clone()
        .map_err(|_| WriteDriverError::ApplyFailed)?;
    for (index, component) in parents.iter().enumerate() {
        let next = open_relative(
            strategy,
            &directory,
            component.as_str(),
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            index,
        )
        .map_err(|_| WriteDriverError::ApplyFailed)?;
        let current = snapshot(&next, Some(index)).map_err(|_| WriteDriverError::ApplyFailed)?;
        if current.file_type != FileType::Directory || !same_mount(&root_now, &current, strategy) {
            return Err(WriteDriverError::ApplyFailed);
        }
        directory = next;
    }
    let io_directory = open_relative(
        strategy,
        &directory,
        ".",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        parents.len(),
    )
    .map_err(|_| WriteDriverError::ApplyFailed)?;
    Ok((io_directory, name.as_str().to_owned()))
}

pub(super) fn stage_file(
    directory: &OwnedFd,
    name: &str,
    bytes: &[u8],
    raw_mode: u32,
) -> Result<OwnedFd, WriteDriverError> {
    let descriptor = openat(
        directory,
        name,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
    )
    .map_err(|_| WriteDriverError::ApplyFailed)?;
    fchmod(&descriptor, Mode::from_raw_mode(raw_mode)).map_err(|_| {
        let _ = unlinkat(directory, name, AtFlags::empty());
        WriteDriverError::ApplyFailed
    })?;
    let write_descriptor = match descriptor.try_clone() {
        Ok(value) => value,
        Err(_) => {
            let _ = remove_staged(directory, name);
            return Err(WriteDriverError::ApplyFailed);
        }
    };
    let mut file = File::from(write_descriptor);
    if file.write_all(bytes).is_err() || file.sync_all().is_err() {
        drop(file);
        let _ = unlinkat(directory, name, AtFlags::empty());
        return Err(WriteDriverError::ApplyFailed);
    }
    Ok(descriptor)
}

fn exchange_back(directory: &OwnedFd, temporary: &str, target: &str) -> Result<(), ()> {
    renameat_with(
        directory,
        temporary,
        directory,
        target,
        RenameFlags::EXCHANGE,
    )
    .map_err(|_| ())?;
    fsync(directory).map_err(|_| ())?;
    remove_staged(directory, temporary)
}

pub(super) fn remove_staged(directory: &OwnedFd, name: &str) -> Result<(), ()> {
    match unlinkat(directory, name, AtFlags::empty()) {
        Ok(()) | Err(Errno::NOENT) => fsync(directory).map_err(|_| ()),
        Err(_) => Err(()),
    }
}

pub(super) fn open_regular(directory: &OwnedFd, name: &str) -> Result<OwnedFd, WriteDriverError> {
    let descriptor = openat(
        directory,
        name,
        OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| WriteDriverError::ObservationUnavailable)?;
    let stat = fstat(&descriptor).map_err(|_| WriteDriverError::ObservationUnavailable)?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile || stat.st_nlink != 1 {
        return Err(WriteDriverError::ObservationUnavailable);
    }
    Ok(descriptor)
}

pub(super) fn read_held_bytes(
    held: &LinuxHeldObject,
    maximum: u64,
) -> Result<Vec<u8>, WriteDriverError> {
    held.revalidate()
        .map_err(|_| WriteDriverError::ObservationUnavailable)?;
    let bytes = read_descriptor(&held.object_descriptor, maximum)?;
    held.revalidate()
        .map_err(|_| WriteDriverError::ObservationUnavailable)?;
    Ok(bytes)
}

pub(super) fn read_descriptor(
    descriptor: &OwnedFd,
    maximum: u64,
) -> Result<Vec<u8>, WriteDriverError> {
    let stat = fstat(descriptor).map_err(|_| WriteDriverError::ObservationUnavailable)?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile
        || stat.st_nlink != 1
        || stat.st_size < 0
        || stat.st_size.cast_unsigned() > maximum
    {
        return Err(WriteDriverError::ObservationUnavailable);
    }
    let mut bytes = Vec::with_capacity(
        usize::try_from(stat.st_size).map_err(|_| WriteDriverError::ObservationUnavailable)?,
    );
    let mut offset = 0_u64;
    let mut buffer = [0_u8; HASH_BUFFER_BYTES];
    loop {
        let count = pread(descriptor, &mut buffer, offset)
            .map_err(|_| WriteDriverError::ObservationUnavailable)?;
        if count == 0 {
            break;
        }
        offset = offset
            .checked_add(count as u64)
            .ok_or(WriteDriverError::ObservationUnavailable)?;
        if offset > maximum {
            return Err(WriteDriverError::ObservationUnavailable);
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
    let after = fstat(descriptor).map_err(|_| WriteDriverError::ObservationUnavailable)?;
    if stat.st_dev != after.st_dev
        || stat.st_ino != after.st_ino
        || stat.st_size != after.st_size
        || stat.st_mtime != after.st_mtime
        || stat.st_mtime_nsec != after.st_mtime_nsec
        || stat.st_ctime != after.st_ctime
        || stat.st_ctime_nsec != after.st_ctime_nsec
        || offset != stat.st_size.cast_unsigned()
    {
        return Err(WriteDriverError::ObservationUnavailable);
    }
    Ok(bytes)
}

pub(super) fn temporary_name(
    transaction_id: &str,
    index: usize,
    purpose: &str,
    bytes: &[u8],
) -> String {
    let mut digest = Sha256::new();
    digest.update(b"agentmage.linux.write.temporary.v1");
    digest.update((transaction_id.len() as u64).to_be_bytes());
    digest.update(transaction_id.as_bytes());
    digest.update((index as u64).to_be_bytes());
    digest.update((purpose.len() as u64).to_be_bytes());
    digest.update(purpose.as_bytes());
    digest.update(Sha256::digest(bytes));
    let encoded = hex_digest(&digest.finalize());
    format!(".agentmage-write-{}.tmp", &encoded[..32])
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::{
        ActionId, ActionKind, ActorId, AdapterInstanceId, ApprovalId, DataSensitivity, GrantId,
        GrantNonce, GrantOperation, GrantTarget, OperationBinding, PathAdapterErrorKind,
        PathResolutionIntent, PlatformPathAdapter, SessionId, TaskId, ToolId,
        WorkspaceAuthorizationId, WorkspaceId, WorkspacePath, WorkspaceScopePath,
    };
    use agentmage_kernel_engine::grants::{GrantIssuer, SessionReadGrantRequest};
    use agentmage_kernel_engine::policy::{
        PolicyDocument, PolicyEngine, ScopeRules, ToolPolicyBinding,
    };
    use agentmage_kernel_engine::write_approval::{
        ShadowChangeSet, ShadowChangeSetRequest, ShadowWriteDraft, WriteApprovalDecision,
        WriteApprovalReceipt, WriteArtifactClass, WriteChangeScope, WriteGrantRequest,
        WriteLineEndings, WriteReviewNarrative, WriteSyntax, build_shadow_change_set,
        issue_write_grant, render_write_preview,
    };
    use agentmage_kernel_engine::write_transaction::{
        AtomicWriteDriver, WriteTransactionError, WriteTransactionOutcome, WriteTransactionRequest,
        execute_write_transaction,
    };
    use sha2::{Digest, Sha256};

    use super::{LinuxAtomicWriteDriver, LinuxAtomicWriteDriverLimits, temporary_name};
    use crate::{LinuxAuthorizedWorkspace, LinuxPathAdapter, authorize_workspace_root};

    static TEMP_ID: AtomicU64 = AtomicU64::new(1);
    const TRANSACTION_ID: &str = "linux-write-transaction-0001";

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let id = TEMP_ID.fetch_add(1, Ordering::SeqCst);
            let path = std::env::temp_dir()
                .join(format!("agentmage-linux-write-{}-{id}", std::process::id()));
            fs::create_dir(&path).expect("temporary write root");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("temporary write root removed");
        }
    }

    struct Fixture {
        _root: TestDirectory,
        workspace: LinuxAuthorizedWorkspace,
        issuer: GrantIssuer,
        policy: PolicyEngine,
        change_set: ShadowChangeSet,
        approval: WriteApprovalReceipt,
        paths: Vec<PathBuf>,
        preimages: Vec<Vec<u8>>,
        postimages: Vec<Vec<u8>>,
    }

    fn rules<T: Ord>(values: impl IntoIterator<Item = T>) -> ScopeRules<T> {
        ScopeRules {
            allowed: values.into_iter().collect(),
            denied: BTreeSet::new(),
        }
    }

    fn hex_sha256(bytes: &[u8]) -> String {
        super::hex_digest(&Sha256::digest(bytes))
    }

    fn review() -> WriteReviewNarrative {
        WriteReviewNarrative {
            rationale: "Apply exact Linux fixture postimages".to_owned(),
            behavior_change: "Fixture files contain reviewed values".to_owned(),
            verification_plan: vec!["Run focused Linux write tests".to_owned()],
            risks: vec!["A fixture may expect its previous bytes".to_owned()],
            rollback: "Restore exact retained preimages in a new transaction".to_owned(),
            unverified_assumptions: vec!["No external fixture consumer was inspected".to_owned()],
        }
    }

    fn fixture(operation_count: usize) -> Fixture {
        let root = TestDirectory::new();
        fs::create_dir(root.path().join("src")).expect("fixture parent");
        let workspace_id = WorkspaceId::from_raw("workspace-linux-write");
        let authorization_id = WorkspaceAuthorizationId::from_raw("authorization-linux-write");
        let adapter_id = AdapterInstanceId::from_raw("adapter-linux-write");
        let workspace = authorize_workspace_root(
            root.path(),
            workspace_id.clone(),
            authorization_id,
            adapter_id.clone(),
        )
        .expect("fixture workspace");
        let adapter = LinuxPathAdapter::new(adapter_id, 1024 * 1024);
        let mut paths = Vec::new();
        let mut preimages = Vec::new();
        let mut postimages = Vec::new();
        let mut targets = Vec::new();
        for index in 0..operation_count {
            let absolute = root
                .path()
                .join("src")
                .join(format!("fixture-{index}.json"));
            let before = format!("{{\"value\":{index}}}\n").into_bytes();
            let after = format!("{{\"value\":{}}}\n", index + 10).into_bytes();
            fs::write(&absolute, &before).expect("fixture preimage");
            fs::set_permissions(&absolute, fs::Permissions::from_mode(0o640))
                .expect("fixture mode");
            let path = WorkspacePath::new(
                workspace_id.clone(),
                ["src".to_owned(), format!("fixture-{index}.json")],
            )
            .expect("fixture path");
            let held = adapter
                .resolve(
                    &workspace,
                    &path,
                    agentmage_kernel_contracts::PathResolutionIntent::ReadFile,
                )
                .expect("fixture target");
            targets.push(GrantTarget::held_object(&held).expect("grant target"));
            paths.push(absolute);
            preimages.push(before);
            postimages.push(after);
        }

        let action_id = ActionId::from_raw("action-linux-write");
        let tool_id = ToolId::from_raw("workspace.linux-write");
        let policy = PolicyEngine::new(PolicyDocument {
            schema_version: 1,
            revision: 1,
            actors: rules([ActorId::from_raw("actor-local")]),
            tasks: rules([TaskId::from_raw("task-linux-write")]),
            actions: rules([action_id.clone()]),
            tools: rules([ToolPolicyBinding {
                tool_id: tool_id.clone(),
                tool_version: "1.0.0".to_owned(),
            }]),
            operations: rules([OperationBinding::new(GrantOperation::WorkspaceWrite)]),
            targets: rules(targets.clone()),
            denied_argument_sha256s: BTreeSet::new(),
            denied_preimage_sha256s: BTreeSet::new(),
            network_scopes: ScopeRules::deny_all(),
            credential_scopes: ScopeRules::deny_all(),
            publication_scopes: ScopeRules::deny_all(),
        })
        .expect("fixture policy");
        let mut issuer = GrantIssuer::new();
        let parent_target = GrantTarget::workspace_scope(
            &workspace,
            WorkspaceScopePath::new(workspace_id.clone(), Vec::<String>::new())
                .expect("root scope"),
        )
        .expect("parent target");
        let excluded_target = GrantTarget::workspace_scope(
            &workspace,
            WorkspaceScopePath::new(workspace_id, ["private"]).expect("excluded scope"),
        )
        .expect("excluded target");
        let parent = issuer
            .issue_session_read(SessionReadGrantRequest {
                grant_id: GrantId::from_raw("grant-parent-linux-write"),
                actor_id: ActorId::from_raw("actor-local"),
                session_id: SessionId::from_raw("session-linux-write"),
                task_id: TaskId::from_raw("task-linux-write"),
                targets: vec![parent_target],
                excluded_targets: vec![excluded_target],
                sensitivity: DataSensitivity::Restricted,
                issued_at_epoch_ms: 1_000,
                expires_at_epoch_ms: 100_000,
                nonce: GrantNonce::from_raw("nonce-parent-linux-write"),
                maximum_derived_operations: 2,
                preview_sha256: "a".repeat(64),
                policy_sha256: policy.policy_sha256().to_owned(),
            })
            .expect("parent grant");
        let drafts = targets
            .into_iter()
            .zip(&preimages)
            .zip(&postimages)
            .enumerate()
            .map(|(index, ((target, before), after))| ShadowWriteDraft {
                operation_id: format!("linux-write-operation-{index}"),
                target,
                observed_bytes: before.clone(),
                proposed_bytes: after.clone(),
                expected_postimage_sha256: hex_sha256(after),
                artifact_class: WriteArtifactClass::Configuration,
                syntax: WriteSyntax::Json,
                line_endings: WriteLineEndings::Lf,
                generated_file: false,
                allow_generated_file: false,
            })
            .collect();
        let change_set = build_shadow_change_set(
            &parent,
            ShadowChangeSetRequest {
                change_set_id: "change-set-linux-write".to_owned(),
                observed_at_epoch_ms: 2_000,
                intent_sha256: "1".repeat(64),
                plan_sha256: "2".repeat(64),
                scope: WriteChangeScope::Minimal,
                expanded_scope_approval_sha256: None,
                review_hooks: Vec::new(),
                operations: drafts,
                review: review(),
                permitted_verification: vec!["cargo-test-linux-write".to_owned()],
            },
        )
        .expect("change set");
        let preview = render_write_preview(&change_set).expect("write preview");
        let approval = issue_write_grant(
            &mut issuer,
            &change_set,
            &preview,
            &WriteApprovalDecision {
                approval_id: ApprovalId::from_raw("approval-linux-write"),
                approved_change_set_sha256: preview.change_set_sha256.clone(),
                approved_preview_sha256: preview.preview_sha256.clone(),
                approved_at_epoch_ms: 3_000,
                expires_at_epoch_ms: 30_000,
                permitted_verification: preview.permitted_verification.clone(),
                user_confirmed: true,
            },
            WriteGrantRequest {
                parent_grant_id: parent.grant_id,
                grant_id: GrantId::from_raw("grant-linux-write"),
                action_id,
                action_kind: ActionKind::DeterministicTool,
                tool_id,
                tool_version: "1.0.0".to_owned(),
                nonce: GrantNonce::from_raw("nonce-linux-write"),
                policy_sha256: policy.policy_sha256().to_owned(),
            },
        )
        .expect("write approval");
        Fixture {
            _root: root,
            workspace,
            issuer,
            policy,
            change_set,
            approval,
            paths,
            preimages,
            postimages,
        }
    }

    fn execute(fixture: &mut Fixture) -> Result<WriteTransactionOutcome, WriteTransactionError> {
        let mut driver = LinuxAtomicWriteDriver::new(
            &fixture.workspace,
            LinuxAtomicWriteDriverLimits {
                maximum_file_bytes: 1024 * 1024,
                maximum_transaction_bytes: 4 * 1024 * 1024,
                ..LinuxAtomicWriteDriverLimits::default()
            },
        );
        execute_write_transaction(
            &mut fixture.issuer,
            &fixture.policy,
            &fixture.change_set,
            &fixture.approval,
            WriteTransactionRequest {
                transaction_id: TRANSACTION_ID.to_owned(),
                now_epoch_ms: 4_000,
            },
            &mut driver,
        )
        .map(|result| result.outcome)
    }

    #[test]
    fn one_file_exchange_commits_exact_bytes_and_preserves_mode() {
        let mut fixture = fixture(1);
        let neighboring = fixture._root.path().join("src/neighbor.txt");
        fs::write(&neighboring, b"untouched\n").expect("neighbor fixture");

        assert_eq!(
            execute(&mut fixture),
            Ok(WriteTransactionOutcome::Committed)
        );
        assert_eq!(
            fs::read(&fixture.paths[0]).expect("postimage"),
            fixture.postimages[0]
        );
        assert_eq!(
            fs::metadata(&fixture.paths[0]).expect("metadata").mode() & 0o777,
            0o640
        );
        assert_eq!(fs::read(neighboring).expect("neighbor"), b"untouched\n");
        assert!(
            fs::read_dir(fixture._root.path().join("src"))
                .expect("parent listing")
                .all(|entry| !entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".agentmage-write-"))
        );
    }

    #[test]
    fn stale_preimage_is_denied_before_consumption_and_change() {
        let mut fixture = fixture(1);
        fs::write(&fixture.paths[0], b"later user work\n").expect("concurrent change");

        assert_eq!(
            execute(&mut fixture),
            Err(WriteTransactionError::PreapplyDenied)
        );
        assert_eq!(
            fs::read(&fixture.paths[0]).expect("later bytes"),
            b"later user work\n"
        );
    }

    #[test]
    fn second_file_collision_restores_first_and_preserves_unrelated_state() {
        let mut fixture = fixture(2);
        let collision_name = temporary_name(
            TRANSACTION_ID,
            1,
            "apply",
            fixture.change_set.operations()[1].proposed_bytes(),
        );
        let collision = fixture._root.path().join("src").join(collision_name);
        fs::write(&collision, b"unrelated collision owner\n").expect("collision fixture");

        assert_eq!(execute(&mut fixture), Ok(WriteTransactionOutcome::Restored));
        for ((path, before), after) in fixture
            .paths
            .iter()
            .zip(&fixture.preimages)
            .zip(&fixture.postimages)
        {
            assert_eq!(fs::read(path).expect("restored bytes"), *before);
            assert_ne!(fs::read(path).expect("not postimage"), *after);
        }
        assert_eq!(
            fs::read(collision).expect("collision retained"),
            b"unrelated collision owner\n"
        );
    }

    #[test]
    fn symlink_hardlink_and_tight_limits_fail_without_effect() {
        let fixture = fixture(1);
        let symlink_path = fixture._root.path().join("src/link.json");
        symlink(&fixture.paths[0], &symlink_path).expect("symlink fixture");
        let hardlink_path = fixture._root.path().join("src/hard.json");
        fs::hard_link(&fixture.paths[0], &hardlink_path).expect("hardlink fixture");
        let adapter = LinuxPathAdapter::new(
            AdapterInstanceId::from_raw("adapter-linux-write"),
            1024 * 1024,
        );
        let link = WorkspacePath::new(
            WorkspaceId::from_raw("workspace-linux-write"),
            ["src", "link.json"],
        )
        .expect("link path");
        let hard = WorkspacePath::new(
            WorkspaceId::from_raw("workspace-linux-write"),
            ["src", "hard.json"],
        )
        .expect("hard-link path");
        assert_eq!(
            adapter
                .resolve(&fixture.workspace, &link, PathResolutionIntent::ReadFile)
                .expect_err("symlink denied")
                .kind(),
            PathAdapterErrorKind::SymbolicLink
        );
        assert_eq!(
            adapter
                .resolve(&fixture.workspace, &hard, PathResolutionIntent::ReadFile)
                .expect_err("hard link denied")
                .kind(),
            PathAdapterErrorKind::HardLink
        );

        let mut driver = LinuxAtomicWriteDriver::new(
            &fixture.workspace,
            LinuxAtomicWriteDriverLimits {
                maximum_operations: 0,
                ..LinuxAtomicWriteDriverLimits::default()
            },
        );
        assert_eq!(
            driver.observe(&fixture.change_set),
            Err(agentmage_kernel_engine::write_transaction::WriteDriverError::ApplyFailed)
        );
        assert_eq!(
            fs::read(&fixture.paths[0]).expect("original bytes"),
            fixture.preimages[0]
        );
        assert!(
            fs::symlink_metadata(symlink_path)
                .expect("symlink metadata")
                .file_type()
                .is_symlink()
        );
        assert_eq!(
            fs::metadata(hardlink_path)
                .expect("hardlink metadata")
                .nlink(),
            2
        );
    }
}
