// SPDX-License-Identifier: Apache-2.0
//! Creation lineage retained only across a verified native transaction and publication.

use super::*;
use crate::{LinuxControlledFilesystemDriver, LinuxFilesystemDriverLimits};
use agentmage_kernel_engine::filesystem_control::{
    ControlledFilesystemDriver, FilesystemApplyAuthorization, FilesystemApplyReport,
    FilesystemDriverError, FilesystemOperationKind, FilesystemOperationObservation,
    FilesystemOperationStatus, FilesystemPlan, FilesystemRestoreAuthorization,
    FilesystemRestoreReport, FilesystemTransactionOutcome, FilesystemTransactionResult,
    verify_filesystem_receipts,
};
use std::sync::MutexGuard;

struct CreationAttempt {
    transaction: String,
    consumed_grant: String,
    // operation -> (path, postimage, mode)
    operations: BTreeMap<String, (String, String, u32)>,
}

/// Native creation driver with the same live owner as structured edits. It can
/// neither issue grants nor adopt a receipt. Only opaque kernel authorization
/// can dispatch, and only successful publication can make files transferable.
pub struct LinuxCodingRuntimeCreateDriver<'a> {
    native: LinuxControlledFilesystemDriver<'a>,
    owner: Option<MutexGuard<'a, LinuxCodingRepository>>,
    attempt: Option<CreationAttempt>,
    applied: bool,
}

impl runtime::LinuxCodingWorktreeLease {
    /// Locks the live owner through native creation and terminal publication.
    /// Other filesystem operations are refused by this creation-only wrapper.
    pub fn create_driver<'a>(
        &'a self,
        workspace: &'a LinuxAuthorizedWorkspace,
        limits: LinuxFilesystemDriverLimits,
    ) -> Result<LinuxCodingRuntimeCreateDriver<'a>> {
        let owner = self
            .owner
            .try_lock()
            .map_err(|_| err("code.runtime.busy"))?;
        let created = owner.verify_created()?;
        if workspace.root_path_sha256() != created.held.root_path_sha256() {
            return Err(err("code.runtime.scope"));
        }
        workspace
            .revalidate()
            .map_err(|_| err("code.runtime.scope"))?;
        Ok(LinuxCodingRuntimeCreateDriver {
            native: LinuxControlledFilesystemDriver::new(workspace, limits),
            owner: Some(owner),
            attempt: None,
            applied: false,
        })
    }
}

impl<'a> LinuxCodingRuntimeCreateDriver<'a> {
    /// Preserves ordinary native operation when no creation owner is attached.
    #[must_use]
    pub fn unowned(
        workspace: &'a LinuxAuthorizedWorkspace,
        limits: LinuxFilesystemDriverLimits,
    ) -> Self {
        Self {
            native: LinuxControlledFilesystemDriver::new(workspace, limits),
            owner: None,
            attempt: None,
            applied: false,
        }
    }

    /// Call only after terminal publication. A matching freshly verified commit
    /// records absent-before lineage; failure or drop retains no such authority.
    pub fn complete(&mut self, result: &FilesystemTransactionResult) -> Result<()> {
        let Some(owner) = &mut self.owner else {
            return Ok(());
        };
        let attempt = self.attempt.take();
        if result.outcome != FilesystemTransactionOutcome::Committed {
            return Ok(());
        }
        let attempt = attempt.ok_or(err("code.runtime.create-unverified"))?;
        if !self.applied
            || result.transaction_id != attempt.transaction
            || result.grant_consumption.consumed_grant_sha256 != attempt.consumed_grant
            || verify_filesystem_receipts(&result.receipts).is_err()
            || result.receipts.iter().any(|r| {
                r.transaction_id != attempt.transaction
                    || r.grant_id != result.grant_consumption.grant_id
            })
        {
            return Err(err("code.runtime.create-unverified"));
        }
        let verified = result
            .receipts
            .iter()
            .filter(|r| r.status == FilesystemOperationStatus::Verified)
            .map(|r| {
                if r.kind != FilesystemOperationKind::Create
                    || r.source_path.is_some()
                    || r.source_sha256 != "0".repeat(64)
                {
                    return Err(err("code.runtime.create-unverified"));
                }
                Ok((
                    r.operation_id.clone(),
                    (
                        r.destination_path
                            .clone()
                            .ok_or(err("code.runtime.create-unverified"))?,
                        r.postimage_sha256.clone(),
                        r.destination_mode
                            .ok_or(err("code.runtime.create-unverified"))?,
                    ),
                ))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        if verified != attempt.operations {
            return Err(err("code.runtime.create-unverified"));
        }
        owner.verify_created()?;
        let created = owner
            .created
            .as_mut()
            .ok_or(err("code.runtime.worktree-required"))?;
        for (_, (path, after_sha256, mode)) in attempt.operations {
            created.edits.insert(
                path,
                AppliedEdit {
                    before_sha256: "0".repeat(64),
                    after_sha256,
                    created_mode: Some(mode),
                },
            );
        }
        self.applied = false;
        Ok(())
    }
}

impl ControlledFilesystemDriver for LinuxCodingRuntimeCreateDriver<'_> {
    fn observe(
        &mut self,
        plan: &FilesystemPlan,
    ) -> std::result::Result<Vec<FilesystemOperationObservation>, FilesystemDriverError> {
        if let Some(owner) = &self.owner {
            owner
                .verify_created()
                .map_err(|_| FilesystemDriverError::ObservationUnavailable)?;
        }
        self.native.observe(plan)
    }
    fn apply(&mut self, authorization: FilesystemApplyAuthorization<'_>) -> FilesystemApplyReport {
        if let Some(owner) = &mut self.owner {
            let candidate = (|| -> Result<CreationAttempt> {
                if self.attempt.is_some() {
                    return Err(err("code.runtime.create-repeated"));
                }
                let created = owner.verify_created()?;
                let mut operations = BTreeMap::new();
                let mut paths = BTreeSet::new();
                for op in authorization.plan().operations() {
                    if op.kind() != FilesystemOperationKind::Create {
                        return Err(err("code.runtime.create-shape"));
                    }
                    let path = op
                        .destination_path()
                        .ok_or(err("code.runtime.create-shape"))?;
                    if created.edits.contains_key(path)
                        || !paths.insert(path.to_owned())
                        || operations
                            .insert(
                                op.operation_id().to_owned(),
                                (
                                    path.to_owned(),
                                    op.postimage_sha256()
                                        .ok_or(err("code.runtime.create-shape"))?
                                        .to_owned(),
                                    op.destination_mode()
                                        .ok_or(err("code.runtime.create-shape"))?,
                                ),
                            )
                            .is_some()
                    {
                        return Err(err("code.runtime.create-shape"));
                    }
                }
                if paths.is_empty() || paths.len() + created.edits.len() > 16 {
                    return Err(err("code.edit.lineage-limit"));
                }
                Ok(CreationAttempt {
                    transaction: authorization.transaction_id().to_owned(),
                    consumed_grant: authorization.consumed_grant_sha256().to_owned(),
                    operations,
                })
            })();
            let attempt = match candidate {
                Ok(a) => a,
                Err(e) => {
                    return FilesystemApplyReport {
                        atomic: false,
                        applied_indexes: vec![],
                        failure_index: Some(0),
                        failure_code: Some(e.code().into()),
                        uncertain: false,
                    };
                }
            };
            owner.command = None;
            owner.edit = None;
            owner.apply = None;
            owner.applied_write = None;
            // No prior path is eligible here, but do not retain it if a future
            // caller changes validation. Publication is the only insertion site.
            for (path, _, _) in attempt.operations.values() {
                owner
                    .created
                    .as_mut()
                    .expect("verified owner")
                    .edits
                    .remove(path);
            }
            self.attempt = Some(attempt);
        }
        let report = self.native.apply(authorization);
        self.applied = !report.uncertain
            && report.failure_index.is_none()
            && report.failure_code.is_none()
            && self.attempt.as_ref().is_none_or(|a| {
                report.applied_indexes == (0..a.operations.len() as u32).collect::<Vec<_>>()
            });
        report
    }
    fn restore(
        &mut self,
        authorization: FilesystemRestoreAuthorization<'_>,
    ) -> FilesystemRestoreReport {
        self.applied = false;
        self.native.restore(authorization)
    }
}
