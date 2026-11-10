// SPDX-License-Identifier: Apache-2.0
//! Retain original source creation proof only across checked native completion.

use super::*;
use agentmage_kernel_engine::filesystem_control::{
    FilesystemOperationKind, FilesystemOperationStatus, FilesystemTransactionOutcome,
    FilesystemTransactionResult, render_filesystem_preview, verify_filesystem_receipts,
};
use agentmage_kernel_engine::review_packet::CommitChangePurpose;

impl LinuxCodingAppliedWrite {
    pub(crate) fn from_native_creation(
        root_sha256: &str,
        transaction_id: &str,
        plan: FilesystemPlan,
        approval: &FilesystemApprovalReceipt,
        result: &FilesystemTransactionResult,
    ) -> Result<Option<Self>, LinuxCodingWriteError> {
        use LinuxCodingWriteError::Transaction;
        let preview = render_filesystem_preview(&plan).map_err(|_| Transaction)?;
        let consumption = &result.grant_consumption;
        let issued = digest(&to_canonical_json(&approval.grant).map_err(|_| Transaction)?);
        let binding = digest(
            &serde_json::to_vec(&(
                approval.grant.grant_id.as_str(),
                plan.plan_sha256(),
                preview.preview_sha256.as_str(),
                &approval.permitted_verification,
                OperationBinding::new(GrantOperation::WorkspaceWrite),
            ))
            .map_err(|_| Transaction)?,
        );
        if result.outcome != FilesystemTransactionOutcome::Committed
            || result.transaction_id != transaction_id
            || approval.plan_sha256 != plan.plan_sha256()
            || approval.preview_sha256 != preview.preview_sha256
            || approval.permitted_verification != plan.permitted_verification()
            || approval.binding_sha256 != binding
            || consumption.grant_id != approval.grant.grant_id
            || consumption.issued_revision != approval.grant.revision
            || consumption.issued_grant_sha256 != issued
            || consumption.consumed_revision
                != consumption
                    .issued_revision
                    .checked_add(1)
                    .ok_or(Transaction)?
            || verify_filesystem_receipts(&result.receipts).is_err()
        {
            return Err(Transaction);
        }
        let expected = plan
            .operations()
            .iter()
            .map(|op| {
                if op.kind() != FilesystemOperationKind::Create {
                    return Err(Transaction);
                }
                Ok((
                    op.operation_id(),
                    (
                        op.destination_path().ok_or(Transaction)?,
                        op.postimage_sha256().ok_or(Transaction)?,
                        op.destination_mode().ok_or(Transaction)?,
                    ),
                ))
            })
            .collect::<Result<std::collections::BTreeMap<_, _>, _>>()?;
        let mut verified = BTreeSet::new();
        for receipt in &result.receipts {
            if receipt.transaction_id != transaction_id
                || receipt.grant_id != consumption.grant_id
                || receipt.kind != FilesystemOperationKind::Create
                || receipt.source_path.is_some()
                || receipt.source_sha256 != "0".repeat(64)
                || receipt.source_mode.is_some()
                || expected.get(receipt.operation_id.as_str())
                    != Some(&(
                        receipt.destination_path.as_deref().ok_or(Transaction)?,
                        receipt.postimage_sha256.as_str(),
                        receipt.destination_mode.ok_or(Transaction)?,
                    ))
            {
                return Err(Transaction);
            }
            if receipt.status == FilesystemOperationStatus::Verified
                && !verified.insert(receipt.operation_id.as_str())
            {
                return Err(Transaction);
            }
        }
        if verified != expected.keys().copied().collect() {
            return Err(Transaction);
        }
        let application = &result.receipts.last().ok_or(Transaction)?.receipt_sha256;
        let Ok(projection) = capture_reviewable_creation(
            &plan,
            plan.operations()
                .iter()
                .map(|op| ChangePurposeAssignment {
                    operation_id: op.operation_id().into(),
                    purpose: CommitChangePurpose::Behavior,
                })
                .collect(),
            &approval.binding_sha256,
            application,
        ) else {
            // Other already-supported native creations still retain transfer
            // lineage. They cannot supply bounded source-review proof or return
            // earlier review history, and are never relabeled as source code.
            return Ok(None);
        };
        Ok(Some(Self {
            writes: vec![NativeWrite {
                root_sha256: root_sha256.into(),
                diff_bytes: projection
                    .files()
                    .iter()
                    .map(|f| f.complete_diff.len())
                    .sum(),
                changes: NativeChanges::Create(plan),
                approval_sha256: approval.binding_sha256.clone(),
                application_sha256: application.clone(),
            }],
        }))
    }
}
