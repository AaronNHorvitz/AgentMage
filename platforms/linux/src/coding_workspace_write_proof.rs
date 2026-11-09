// SPDX-License-Identifier: Apache-2.0
//! Historical proof retained only from this module's actual native transaction.

use super::*;
use agentmage_kernel_contracts::to_canonical_json;
use agentmage_kernel_engine::review_packet::{
    ChangePurposeAssignment, ReviewableChangeSet, capture_reviewable_change_set,
};
use agentmage_kernel_engine::write_transaction::{WriteOperationStatus, verify_write_receipts};

// No Clone, Serialize, Deserialize or public constructor. Even a valid external
// receipt chain is not a substitute for the original native execution boundary.
pub(crate) struct LinuxCodingAppliedWrite {
    root_sha256: String,
    changes: ShadowChangeSet,
    approval_sha256: String,
    application_sha256: String,
}

impl LinuxCodingAppliedWrite {
    pub(super) fn from_native_result(
        root_sha256: &str,
        changes: ShadowChangeSet,
        approval: &WriteApprovalReceipt,
        result: &WriteTransactionResult,
    ) -> Result<Self, LinuxCodingWriteError> {
        use LinuxCodingWriteError::Transaction;
        let issued = digest(&to_canonical_json(&approval.grant).map_err(|_| Transaction)?);
        let binding = digest(
            &serde_json::to_vec(&(
                approval.grant.grant_id.as_str(),
                changes.change_set_sha256(),
                approval.preview_sha256.as_str(),
                &approval.permitted_verification,
            ))
            .map_err(|_| Transaction)?,
        );
        let consumption = &result.grant_consumption;
        if result.outcome != WriteTransactionOutcome::Committed
            || approval.change_set_sha256 != changes.change_set_sha256()
            || result.transaction_id != format!("transaction-{}", changes.change_set_id())
            || consumption.grant_id != approval.grant.grant_id
            || consumption.issued_revision != approval.grant.revision
            || consumption.issued_grant_sha256 != issued
            || consumption.consumed_revision
                != consumption
                    .issued_revision
                    .checked_add(1)
                    .ok_or(Transaction)?
            || approval.binding_sha256 != binding
            || verify_write_receipts(&result.receipts).is_err()
        {
            return Err(Transaction);
        }
        let expected = changes
            .operations()
            .iter()
            .map(|op| {
                (
                    op.operation_id(),
                    (op.preimage_sha256(), op.expected_postimage_sha256()),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let verified = result
            .receipts
            .iter()
            .filter(|r| r.status == WriteOperationStatus::Verified)
            .map(|r| {
                (
                    r.operation_id.as_str(),
                    (r.preimage_sha256.as_str(), r.postimage_sha256.as_str()),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        if expected != verified
            || result.receipts.iter().any(|r| {
                r.transaction_id != result.transaction_id
                    || r.grant_id != consumption.grant_id
                    || expected.get(r.operation_id.as_str())
                        != Some(&(r.preimage_sha256.as_str(), r.postimage_sha256.as_str()))
            })
        {
            return Err(Transaction);
        }
        Ok(Self {
            root_sha256: root_sha256.into(),
            changes,
            approval_sha256: approval.binding_sha256.clone(),
            // The last digest binds the entire checked application receipt chain.
            application_sha256: result
                .receipts
                .last()
                .ok_or(Transaction)?
                .receipt_sha256
                .clone(),
        })
    }

    pub(crate) fn root_sha256(&self) -> &str {
        &self.root_sha256
    }

    pub(crate) fn project(
        &self,
        purposes: Vec<ChangePurposeAssignment>,
    ) -> Result<ReviewableChangeSet, LinuxCodingWriteError> {
        capture_reviewable_change_set(
            &self.changes,
            purposes,
            &self.approval_sha256,
            &self.application_sha256,
        )
        .map_err(|_| LinuxCodingWriteError::Input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);

    struct Fixture(std::path::PathBuf);
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn native_write_proof_correlates_original_approval_and_rejects_result_substitutions() {
        let started = Instant::now();
        let f = Fixture(std::env::temp_dir().join(format!(
            "agentmage-write-proof-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )));
        fs::create_dir(&f.0).unwrap();
        fs::set_permissions(&f.0, fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(f.0.join("source.rs"), b"original").unwrap();
        let scope = LinuxCodingReadScope::select(&f.0, &["source.rs".into()]).unwrap();
        let mut pending = scope
            .prepare_write(
                &[LinuxCodingReplacement {
                    path: "source.rs",
                    before_sha256: &digest(b"original"),
                    content: "approved",
                }],
                "original native proof test",
                1000,
            )
            .unwrap();
        let changes = pending.change_set.clone();
        let preview = pending.preview.preview_sha256.clone();
        let approval = pending
            .approve(changes.change_set_sha256(), &preview, 1000)
            .unwrap()
            .clone();
        let mut driver =
            LinuxAtomicWriteDriver::new(&scope.workspace, LinuxAtomicWriteDriverLimits::default());
        let result = pending
            .execute(
                &scope,
                changes.change_set_sha256(),
                &preview,
                1000,
                &mut driver,
            )
            .unwrap();
        let proof = LinuxCodingAppliedWrite::from_native_result(
            scope.root_sha256(),
            changes.clone(),
            &approval,
            &result,
        )
        .unwrap();
        let purposes = || {
            vec![ChangePurposeAssignment {
                operation_id: "edit-0".into(),
                purpose: agentmage_kernel_engine::review_packet::CommitChangePurpose::Behavior,
            }]
        };
        let projected = serde_json::to_value(proof.project(purposes()).unwrap()).unwrap();
        assert_eq!(
            projected["write_approval_receipt_sha256"],
            approval.binding_sha256
        );
        assert_eq!(
            projected["write_application_receipt_sha256"],
            result.receipts.last().unwrap().receipt_sha256
        );
        for which in [
            "transaction",
            "grant",
            "issued",
            "revision",
            "receipts",
            "outcome",
            "approval",
            "binding",
        ] {
            let mut bad = result.clone();
            let mut bad_approval = approval.clone();
            match which {
                "transaction" => bad.transaction_id.push_str("-other"),
                "grant" => bad.grant_consumption.grant_id = GrantId::from_raw("foreign-grant"),
                "issued" => bad.grant_consumption.issued_grant_sha256 = digest(b"other grant"),
                "revision" => bad.grant_consumption.consumed_revision += 1,
                "receipts" => bad.receipts[0].receipt_sha256 = digest(b"other receipt"),
                "outcome" => bad.outcome = WriteTransactionOutcome::Uncertain,
                "approval" => bad_approval.change_set_sha256 = digest(b"other changes"),
                "binding" => bad_approval.binding_sha256 = digest(b"other binding"),
                _ => unreachable!(),
            }
            assert!(
                LinuxCodingAppliedWrite::from_native_result(
                    scope.root_sha256(),
                    changes.clone(),
                    &bad_approval,
                    &bad
                )
                .is_err(),
                "{which}"
            );
        }
        assert_eq!(fs::read(f.0.join("source.rs")).unwrap(), b"approved");
        println!(
            "native_write_proof_component_us={}",
            started.elapsed().as_micros()
        );
    }
}
