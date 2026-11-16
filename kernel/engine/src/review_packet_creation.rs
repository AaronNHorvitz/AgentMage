// SPDX-License-Identifier: MIT OR Apache-2.0
//! Authority-free source creation projection for the existing review contract.

use super::*;
use crate::filesystem_control::{
    FileClassification, FilesystemOperationKind, FilesystemPlan, render_filesystem_preview,
};

/// Projects an exact text-source creation plan, preserving the original plan,
/// operation and receipt identities. Zero preimage means absent, distinct from
/// an existing empty file. Integrity alone never authenticates native execution.
/// This creates neither a grant nor Git authority. Other filesystem operations,
/// binary content and non-source classifications are deliberately refused.
pub fn capture_reviewable_creation(
    plan: &FilesystemPlan,
    purposes: Vec<ChangePurposeAssignment>,
    approval_sha256: &str,
    application_sha256: &str,
) -> Result<ReviewableChangeSet, ReviewPacketError> {
    use ReviewPacketError::ChangeSetMismatch;
    render_filesystem_preview(plan).map_err(|_| ChangeSetMismatch)?;
    let count = purposes.len();
    let purposes = purposes
        .into_iter()
        .map(|p| (p.operation_id, p.purpose))
        .collect::<BTreeMap<_, _>>();
    if purposes.len() != count || count != plan.operations().len() {
        return Err(ChangeSetMismatch);
    }
    let mut files = Vec::with_capacity(count);
    for op in plan.operations() {
        if op.kind() != FilesystemOperationKind::Create
            || op.classification() != Some(FileClassification::SourceCode)
        {
            return Err(ChangeSetMismatch);
        }
        let content = std::str::from_utf8(op.postimage_bytes()).map_err(|_| ChangeSetMismatch)?;
        if content.is_empty() || content.contains('\0') {
            return Err(ChangeSetMismatch);
        }
        let path = op.destination_path().ok_or(ChangeSetMismatch)?;
        let postimage = op.postimage_sha256().ok_or(ChangeSetMismatch)?;
        let absent = "0".repeat(64);
        files.push(ReviewChangedFile {
            operation_id: op.operation_id().into(),
            path: path.into(),
            preimage_sha256: absent.clone(),
            postimage_sha256: postimage.into(),
            artifact_class: ReviewArtifactClass::Code,
            purpose: *purposes.get(op.operation_id()).ok_or(ChangeSetMismatch)?,
            generated: false,
            complete_diff: format!(
                "--- {path} sha256:{absent}\n+++ {path} sha256:{postimage}\n@@ exact-complete-postimage @@\n-\"\"\n+{content:?}\n"
            ),
            operation_sha256: op.operation_sha256().into(),
        });
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let result = ReviewableChangeSet {
        change_set_id: plan.plan_id().into(),
        change_set_sha256: plan.plan_sha256().into(),
        intent_sha256: sha256_json(plan.review())?,
        plan_sha256: plan.plan_sha256().into(),
        write_approval_receipt_sha256: Some(approval_sha256.into()),
        write_application_receipt_sha256: Some(application_sha256.into()),
        write_history: vec![],
        scope: WriteChangeScope::Minimal,
        review_hooks: vec![WriteReviewHook::Security],
        files_sha256: sha256_json(&files)?,
        files,
        write_authority: false,
    };
    if !verify_single_change_set(&result) {
        return Err(ChangeSetMismatch);
    }
    Ok(result)
}
