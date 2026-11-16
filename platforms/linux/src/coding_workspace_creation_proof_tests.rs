// SPDX-License-Identifier: MIT OR Apache-2.0
//! Actual native creation with original opaque authorization, without publication.

use super::*;
use crate::coding_workspace::LinuxCodingAppliedWrite;
use agentmage_kernel_engine::filesystem_control::FilesystemApprovalReceipt;
use agentmage_kernel_engine::review_packet::{ChangePurposeAssignment, CommitChangePurpose};
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

struct Capture<'a> {
    native: LinuxControlledFilesystemDriver<'a>,
    original: Option<(FilesystemPlan, FilesystemApprovalReceipt, String)>,
}
impl ControlledFilesystemDriver for Capture<'_> {
    fn observe(
        &mut self,
        plan: &FilesystemPlan,
    ) -> Result<Vec<FilesystemOperationObservation>, FilesystemDriverError> {
        self.native.observe(plan)
    }
    fn apply(&mut self, auth: FilesystemApplyAuthorization<'_>) -> FilesystemApplyReport {
        assert!(self.original.is_none());
        self.original = Some((
            auth.plan().clone(),
            auth.approval().clone(),
            auth.transaction_id().into(),
        ));
        self.native.apply(auth)
    }
    fn restore(&mut self, auth: FilesystemRestoreAuthorization<'_>) -> FilesystemRestoreReport {
        self.native.restore(auth)
    }
}

#[test]
fn creation_review_correlates_original_native_approval_and_all_creation_receipts() {
    let f = Fixture(std::env::temp_dir().join(format!(
        "agentmage-creation-review-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )));
    fs::create_dir(&f.0).unwrap();
    fs::set_permissions(&f.0, fs::Permissions::from_mode(0o700)).unwrap();
    let paths = vec!["new.py".into(), "other.py".into()];
    let scope = LinuxCodingReadScope::select_targets(&f.0, &paths).unwrap();
    let pending = scope
        .prepare_create(
            &[
                ("new.py", "created\n", 0o640),
                ("other.py", "executable\n", 0o700),
            ],
            "Original source creation component",
            1000,
        )
        .unwrap();
    let mut driver = Capture {
        native: LinuxControlledFilesystemDriver::new(&scope.workspace, limits()),
        original: None,
    };
    let result = pending.execute(&scope, 1000, &mut driver).unwrap();
    let (plan, approval, transaction) = driver.original.take().unwrap();
    let proof = LinuxCodingAppliedWrite::from_native_creation(
        scope.root_sha256(),
        &transaction,
        plan.clone(),
        &approval,
        &result,
    )
    .unwrap()
    .unwrap();
    let projection = proof
        .project(
            (0..2)
                .map(|i| ChangePurposeAssignment {
                    operation_id: format!("create-{i}"),
                    purpose: CommitChangePurpose::Behavior,
                })
                .collect(),
        )
        .unwrap();
    let value = serde_json::to_value(&projection).unwrap();
    assert_eq!(value["change_set_sha256"], plan.plan_sha256());
    assert_eq!(
        value["write_approval_receipt_sha256"],
        approval.binding_sha256
    );
    assert_eq!(
        value["write_application_receipt_sha256"],
        result.receipts.last().unwrap().receipt_sha256
    );
    for which in [
        "transaction",
        "grant",
        "issued",
        "revision",
        "consumed-revision",
        "receipt",
        "outcome",
        "plan",
        "preview",
        "verification",
        "binding",
        "grant-bytes",
        "missing-operation",
    ] {
        let mut bad = result.clone();
        let mut bad_approval = approval.clone();
        match which {
            "transaction" => bad.transaction_id.push_str("foreign"),
            "grant" => bad.grant_consumption.grant_id = GrantId::from_raw("foreign"),
            "issued" => bad.grant_consumption.issued_grant_sha256 = "a".repeat(64),
            "revision" => bad.grant_consumption.issued_revision += 1,
            "consumed-revision" => bad.grant_consumption.consumed_revision += 1,
            "receipt" => bad.receipts.last_mut().unwrap().receipt_sha256 = "b".repeat(64),
            "outcome" => bad.outcome = FilesystemTransactionOutcome::Uncertain,
            "plan" => bad_approval.plan_sha256 = "c".repeat(64),
            "preview" => bad_approval.preview_sha256 = "d".repeat(64),
            "verification" => bad_approval.permitted_verification.clear(),
            "binding" => bad_approval.binding_sha256 = "e".repeat(64),
            "grant-bytes" => bad_approval.grant.expires_at_epoch_ms += 1,
            "missing-operation" => bad.receipts.retain(|r| r.operation_id != "create-1"),
            _ => unreachable!(),
        }
        assert!(
            LinuxCodingAppliedWrite::from_native_creation(
                scope.root_sha256(),
                &transaction,
                plan.clone(),
                &bad_approval,
                &bad
            )
            .is_err(),
            "{which}"
        );
    }
    for (path, bytes, mode) in [
        ("new.py", b"created\n".as_slice(), 0o640),
        ("other.py", b"executable\n".as_slice(), 0o700),
    ] {
        assert_eq!(fs::read(f.0.join(path)).unwrap(), bytes);
        assert_eq!(
            fs::metadata(f.0.join(path)).unwrap().permissions().mode() & 0o7777,
            mode
        );
    }
}

#[test]
fn creation_review_unsupported_native_creations_remain_successful_without_source_proof() {
    for (content, classification) in [
        (
            b"documentation\n".to_vec(),
            FileClassification::Documentation,
        ),
        (vec![0xff], FileClassification::SourceCode),
        (b"nul\0data".to_vec(), FileClassification::SourceCode),
    ] {
        let f = Fixture(std::env::temp_dir().join(format!(
            "agentmage-creation-review-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )));
        fs::create_dir(&f.0).unwrap();
        fs::set_permissions(&f.0, fs::Permissions::from_mode(0o700)).unwrap();
        let scope = LinuxCodingReadScope::select_targets(&f.0, &["new.py".into()]).unwrap();
        let mut pending = scope
            .prepare_create(
                &[("new.py", "placeholder", 0o600)],
                "Non-source native creation",
                1000,
            )
            .unwrap();
        let original = &pending.plan.operations()[0];
        // Build a real separate approved plan. No receipt/approval field is
        // edited to make unsupported content appear to be an original source.
        pending.plan = build_filesystem_plan(
            pending.issuer.current(&pending.parent).unwrap(),
            FilesystemPlanRequest {
                plan_id: pending.plan.plan_id().into(),
                observed_at_epoch_ms: 1000,
                operations: vec![FilesystemOperationDraft::Create {
                    operation_id: original.operation_id().into(),
                    destination: NewDestinationDraft {
                        parent: original.destination_parent().unwrap().clone(),
                        path: scope.paths["new.py"].clone(),
                        observed_sibling_names: pending.sibling_names[0].clone(),
                    },
                    content: content.clone(),
                    mode: 0o600,
                    classification,
                }],
                review: pending.plan.review().clone(),
                permitted_verification: pending.plan.permitted_verification().to_vec(),
            },
        )
        .unwrap();
        pending.preview = render_filesystem_preview(&pending.plan).unwrap();
        let mut driver = Capture {
            native: LinuxControlledFilesystemDriver::new(&scope.workspace, limits()),
            original: None,
        };
        let result = pending.execute(&scope, 1000, &mut driver).unwrap();
        let (plan, approval, transaction) = driver.original.take().unwrap();
        assert_eq!(result.outcome, FilesystemTransactionOutcome::Committed);
        assert!(
            LinuxCodingAppliedWrite::from_native_creation(
                scope.root_sha256(),
                &transaction,
                plan,
                &approval,
                &result
            )
            .unwrap()
            .is_none()
        );
        assert_eq!(fs::read(f.0.join("new.py")).unwrap(), content);
    }
}
