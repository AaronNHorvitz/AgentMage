// SPDX-License-Identifier: Apache-2.0
//! Process-held worktree ownership shared with the existing native coordinator.

use super::*;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::coding_workspace::LinuxCodingAppliedWrite;
use crate::{LinuxAtomicWriteDriver, LinuxAtomicWriteDriverLimits};
use agentmage_kernel_engine::write_approval::{
    CurrentWriteTarget, ShadowChangeSet, WriteApprovalReceipt,
};
use agentmage_kernel_engine::write_transaction::{
    AtomicWriteDriver, WriteApplyAuthorization, WriteApplyReport, WriteDriverError,
    WriteOperationStatus, WriteRestoreAuthorization, WriteRestoreReport, WriteTransactionOutcome,
    WriteTransactionResult, verify_write_receipts,
};

/// Nonserializable creation authority. Only consuming a live repository owner
/// with a successfully created worktree can construct it. Clones share one
/// serialized owner; none can recover manual authority while another is live.
#[derive(Clone)]
pub struct LinuxCodingWorktreeLease {
    pub(super) owner: Arc<Mutex<LinuxCodingRepository>>,
    root_sha256: String,
    recovery_sha256: String,
}

impl LinuxCodingRepository {
    /// Transfers this live owner, after explicit handoff consent, to the native
    /// coordinator. The displayed root must match before inspecting roots.
    /// Returns the owner intact on refusal. Clears pending manual decisions.
    /// No source, model, command or write permission is issued. A marker or caller receipt cannot create this lease.
    #[allow(clippy::result_large_err)] // Refusal must preserve the live authority owner.
    pub fn into_runtime_worktree(
        mut self,
        expected_root_sha256: &str,
    ) -> std::result::Result<LinuxCodingWorktreeLease, (Self, LinuxCodingRepositoryError)> {
        let check = (|| {
            let created = self
                .created
                .as_ref()
                .ok_or(err("code.runtime.worktree-required"))?;
            if created.held.root_path_sha256() != expected_root_sha256 {
                return Err(err("code.runtime.scope"));
            }
            self.verify_created()?;
            Ok(())
        })();
        if let Err(error) = check {
            return Err((self, error));
        }
        // Mint a fresh identity for this handoff, including reattachment of the
        // same created root. Randomness failure returns the owner untouched.
        let mut nonce = [0_u8; 32];
        if rustix::rand::getrandom(&mut nonce, rustix::rand::GetRandomFlags::empty()) != Ok(32) {
            return Err((self, err("code.runtime.recovery-identity")));
        }
        let root_sha256 = expected_root_sha256.to_owned();
        let recovery_sha256 = hash(&nonce);
        self.pending = None;
        self.command = None;
        self.validation = None;
        self.edit = None;
        self.apply = None;
        Ok(LinuxCodingWorktreeLease {
            owner: Arc::new(Mutex::new(self)),
            root_sha256,
            recovery_sha256,
        })
    }
}

impl LinuxCodingWorktreeLease {
    /// Content-free identity for explicit recovery consent. Stable across clones
    /// and busy recovery; changes on every new handoff, even for the same root.
    /// These digests confer no authority and cannot construct a lease. Available
    /// without locking the owner so poisoned ownership can still be discarded
    /// through the ordinary fail-closed recovery path.
    #[must_use]
    pub fn recovery_binding(&self) -> (&str, &str) {
        (&self.root_sha256, &self.recovery_sha256)
    }

    /// Refuses consent for another owner or an earlier handoff of the same root.
    /// This only checks identity; it never grants or recovers ownership.
    pub fn check_recovery_binding(&self, root: &str, recovery: &str) -> Result<()> {
        if self.recovery_binding() != (root, recovery) {
            return Err(err("code.runtime.recovery-stale"));
        }
        Ok(())
    }

    /// Recovers a held recovery slot without losing its lease on a busy retry.
    /// Only a live creation lease can populate the slot; no path is adopted.
    /// Success consumes the slot, including the ordinary poison invalidation.
    pub fn recover_repository(slot: &mut Option<Self>) -> Result<LinuxCodingRepository> {
        let lease = slot.take().ok_or(err("code.runtime.worktree-required"))?;
        match lease.try_into_repository() {
            Ok(owner) => Ok(owner),
            Err(lease) => {
                *slot = Some(lease);
                Err(err("code.runtime.owner-held"))
            }
        }
    }

    /// Checks a canonical private runtime state root is separate from the manual
    /// authority store and every held repository root. No store is opened here.
    pub fn check_runtime_state_root(&self, root: &Path) -> Result<()> {
        let owner = self
            .owner
            .try_lock()
            .map_err(|_| err("code.runtime.busy"))?;
        let created = owner.verify_created()?;
        if !root.is_absolute()
            || [&owner.state_root, &created.original_path, &created.path]
                .iter()
                .any(|held| root.starts_with(held) || held.starts_with(root))
        {
            return Err(err("code.runtime.state-scope"));
        }
        Ok(())
    }

    /// Revalidates the exact activation path against held creation authority.
    /// Reads directory metadata only. It never adopts a path or receipt.
    pub fn check_workspace(&self, root: &Path) -> Result<()> {
        let owner = self
            .owner
            .try_lock()
            .map_err(|_| err("code.runtime.busy"))?;
        let created = owner
            .created
            .as_ref()
            .ok_or(err("code.runtime.worktree-required"))?;
        if root != created.path {
            return Err(err("code.runtime.scope"));
        }
        owner.verify_created()?;
        Ok(())
    }

    /// Recovers the same manual owner only after every runtime reference is
    /// released. Busy ownership is returned intact for later explicit recovery.
    /// A poisoned owner fails closed; it cannot recover transferable edits.
    pub fn try_into_repository(self) -> std::result::Result<LinuxCodingRepository, Self> {
        match Arc::try_unwrap(self.owner) {
            Ok(mutex) => match mutex.into_inner() {
                Ok(owner) => Ok(owner),
                Err(poison) => {
                    let mut owner = poison.into_inner();
                    owner.created = None;
                    owner.pending = None;
                    owner.command = None;
                    owner.validation = None;
                    owner.edit = None;
                    owner.apply = None;
                    owner.applied_write = None;
                    Ok(owner)
                }
            },
            Err(owner) => Err(Self {
                owner,
                root_sha256: self.root_sha256,
                recovery_sha256: self.recovery_sha256,
            }),
        }
    }

    /// Wraps the unchanged native atomic writer. The workspace descriptor must
    /// match the held created root. The lock lasts through final reconciliation;
    /// concurrent writes refuse rather than block or interleave lineage.
    pub fn write_driver<'a>(
        &'a self,
        workspace: &'a LinuxAuthorizedWorkspace,
        limits: LinuxAtomicWriteDriverLimits,
    ) -> Result<LinuxCodingRuntimeWriteDriver<'a>> {
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
        Ok(LinuxCodingRuntimeWriteDriver {
            native: LinuxAtomicWriteDriver::new(workspace, limits),
            owner: Some(owner),
            attempted: None,
            successful_apply: false,
        })
    }
}

struct Attempt {
    transaction_id: String,
    consumed_grant_sha256: String,
    edits: BTreeMap<String, AppliedEdit>,
    operations: BTreeMap<String, (String, String)>,
    changes: ShadowChangeSet,
    approval: WriteApprovalReceipt,
}

/// Native writer with optional live creation provenance. It cannot issue an
/// approval: application still requires the kernel's opaque consumed grant.
/// Only a successful actual native apply and checked committed result establish
/// lineage. Drop, failed completion and restoration leave attempted paths
/// ineligible for transfer to the original checkout.
pub struct LinuxCodingRuntimeWriteDriver<'a> {
    native: LinuxAtomicWriteDriver<'a>,
    owner: Option<MutexGuard<'a, LinuxCodingRepository>>,
    attempted: Option<Attempt>,
    successful_apply: bool,
}

impl<'a> LinuxCodingRuntimeWriteDriver<'a> {
    /// Preserves the ordinary native writer when no owned worktree is attached.
    pub fn unowned(
        workspace: &'a LinuxAuthorizedWorkspace,
        limits: LinuxAtomicWriteDriverLimits,
    ) -> Self {
        Self {
            native: LinuxAtomicWriteDriver::new(workspace, limits),
            owner: None,
            attempted: None,
            successful_apply: false,
        }
    }

    /// Records only the matching, freshly verified transaction. Call after the
    /// coordinator's terminal publication succeeds. This grants no apply to the
    /// original: that still needs its complete diff and separate exact approval.
    pub fn complete(&mut self, result: &WriteTransactionResult) -> Result<()> {
        let Some(owner) = &mut self.owner else {
            return Ok(());
        };
        let attempt = self.attempted.take();
        if result.outcome != WriteTransactionOutcome::Committed {
            return Ok(());
        }
        let attempt = attempt.ok_or(err("code.runtime.write-unverified"))?;
        if !self.successful_apply
            || result.transaction_id != attempt.transaction_id
            || result.grant_consumption.consumed_grant_sha256 != attempt.consumed_grant_sha256
            || verify_write_receipts(&result.receipts).is_err()
        {
            return Err(err("code.runtime.write-unverified"));
        }
        let verified = result
            .receipts
            .iter()
            .filter(|r| r.status == WriteOperationStatus::Verified)
            .map(|r| {
                (
                    r.operation_id.clone(),
                    (r.preimage_sha256.clone(), r.postimage_sha256.clone()),
                )
            })
            .collect::<BTreeMap<_, _>>();
        if verified != attempt.operations
            || result.receipts.iter().any(|r| {
                r.transaction_id != attempt.transaction_id
                    || r.grant_id != result.grant_consumption.grant_id
            })
        {
            return Err(err("code.runtime.write-unverified"));
        }
        owner.verify_created()?;
        let proof = LinuxCodingAppliedWrite::from_native_result(
            owner
                .created
                .as_ref()
                .ok_or(err("code.runtime.worktree-required"))?
                .held
                .root_path_sha256(),
            &attempt.transaction_id,
            attempt.changes.clone(),
            &attempt.approval,
            result,
        )
        .map_err(|_| err("code.runtime.write-unverified"))?;
        let created = owner
            .created
            .as_mut()
            .ok_or(err("code.runtime.worktree-required"))?;
        created.edits.extend(attempt.edits.clone());
        // A single transaction is reviewable only if it covers the complete
        // first-preimage/current-postimage lineage. Never invent an approval
        // spanning earlier retries or edits on other paths.
        if attempt.changes.operations().len() == created.edits.len()
            && attempt.changes.operations().iter().all(|op| {
                created.edits.get(op.path()).is_some_and(|edit| {
                    edit.created_mode.is_none()
                        && edit.before_sha256 == op.preimage_sha256()
                        && edit.after_sha256 == op.expected_postimage_sha256()
                })
            })
        {
            owner.applied_write = Some(proof);
        }
        self.successful_apply = false;
        Ok(())
    }
}

impl AtomicWriteDriver for LinuxCodingRuntimeWriteDriver<'_> {
    fn observe(
        &mut self,
        changes: &ShadowChangeSet,
    ) -> std::result::Result<Vec<CurrentWriteTarget>, WriteDriverError> {
        if let Some(owner) = &self.owner {
            owner
                .verify_created()
                .map_err(|_| WriteDriverError::ObservationUnavailable)?;
        }
        self.native.observe(changes)
    }

    fn apply(&mut self, authorization: WriteApplyAuthorization<'_>) -> WriteApplyReport {
        if let Some(owner) = &mut self.owner {
            let candidate = (|| -> Result<Attempt> {
                if self.attempted.is_some() {
                    return Err(err("code.runtime.write-repeated"));
                }
                let created = owner.verify_created()?;
                let mut edits = BTreeMap::new();
                let mut operations = BTreeMap::new();
                for change in authorization.change_set().operations() {
                    let path = change.path();
                    let prior = created.edits.get(path);
                    if prior.is_some_and(|p| p.after_sha256 != change.preimage_sha256()) {
                        return Err(err("code.edit.lineage"));
                    }
                    if edits
                        .insert(
                            path.to_owned(),
                            AppliedEdit {
                                before_sha256: prior.map_or_else(
                                    || change.preimage_sha256().to_owned(),
                                    |p| p.before_sha256.clone(),
                                ),
                                after_sha256: change.expected_postimage_sha256().to_owned(),
                                created_mode: prior.and_then(|p| p.created_mode),
                            },
                        )
                        .is_some()
                        || operations
                            .insert(
                                change.operation_id().to_owned(),
                                (
                                    change.preimage_sha256().to_owned(),
                                    change.expected_postimage_sha256().to_owned(),
                                ),
                            )
                            .is_some()
                    {
                        return Err(err("code.runtime.write-shape"));
                    }
                }
                if edits.is_empty()
                    || created
                        .edits
                        .keys()
                        .chain(edits.keys())
                        .collect::<BTreeSet<_>>()
                        .len()
                        > 16
                {
                    return Err(err("code.edit.lineage-limit"));
                }
                Ok(Attempt {
                    transaction_id: authorization.transaction_id().to_owned(),
                    consumed_grant_sha256: authorization.consumed_grant_sha256().to_owned(),
                    edits,
                    operations,
                    changes: authorization.change_set().clone(),
                    approval: authorization.approval().clone(),
                })
            })();
            let attempt = match candidate {
                Ok(attempt) => attempt,
                Err(error) => {
                    return WriteApplyReport {
                        atomic: false,
                        applied_indexes: vec![],
                        failure_index: Some(0),
                        failure_code: Some(error.code().to_owned()),
                        uncertain: false,
                    };
                }
            };
            owner.command = None;
            owner.edit = None;
            owner.apply = None;
            // Historical proof is retired before dispatch. Only this actual
            // native transaction plus terminal publication can replace it.
            owner.applied_write = None;
            let created = owner.created.as_mut().expect("verified created worktree");
            for path in attempt.edits.keys() {
                created.edits.remove(path);
            }
            self.attempted = Some(attempt);
        }
        let report = self.native.apply(authorization);
        self.successful_apply = !report.uncertain
            && report.failure_index.is_none()
            && report.failure_code.is_none()
            && self.attempted.as_ref().is_none_or(|attempt| {
                report.applied_indexes == (0..attempt.operations.len() as u32).collect::<Vec<_>>()
            });
        report
    }

    fn restore(&mut self, authorization: WriteRestoreAuthorization<'_>) -> WriteRestoreReport {
        self.successful_apply = false;
        self.native.restore(authorization)
    }
}
