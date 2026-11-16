// SPDX-License-Identifier: MIT OR Apache-2.0
//! Synthetic collection/worker/clock outcomes; no native admission evidence.

use super::*;
use agentmage_kernel_contracts::{BoundaryKind, CorrelationId, TaskId};
use agentmage_kernel_engine::local_commit::{
    CandidateTreeFilePlan, CommitFileMode, reconcile_candidate_tree,
};
use std::cell::Cell;

const ORDER: [&str; 7] = [
    "before", "start", "read", "add", "write", "cleanup", "after",
];

struct SyntheticSession {
    before: RepositoryPreservationManifest,
    after: RepositoryPreservationManifest,
    plan: CandidateTreePlan,
    deadline: Instant,
    now: Cell<Instant>,
    cancellation: CancellationToken,
    calls: Vec<&'static str>,
    fail: Option<&'static str>,
    stop_after: Option<&'static str>,
    cancel: bool,
    checks: Cell<usize>,
    expire_on_check: Option<usize>,
}

impl SyntheticSession {
    fn new(width: usize) -> Self {
        let mut before: RepositoryPreservationManifest = serde_json::from_str(include_str!(
            "../../../schemas/runtime/examples/repository-preservation-manifest.valid.json"
        ))
        .unwrap();
        before.object_format = if width == 40 { "sha1" } else { "sha256" }.into();
        before.head_object = Some("a".repeat(width));
        before = RepositoryPreservationManifest::seal(before).unwrap();
        let mut after = before.clone();
        after.object_database_sha256 = hash(b"synthetic new candidate objects");
        after = RepositoryPreservationManifest::seal(after).unwrap();
        let mut plan = CandidateTreePlan {
            schema_version: 1,
            candidate_build_id: "complete-budget-test".into(),
            repository_sha256: before.repository_sha256.clone(),
            review_packet_sha256: hash(b"synthetic review"),
            change_set_sha256: hash(b"synthetic changes"),
            commit_group_sha256: hash(b"synthetic group"),
            parent_object: "a".repeat(width),
            temporary_index_path_sha256: hash(b"synthetic scratch path"),
            user_index_sha256: before.index_sha256.clone(),
            preservation_manifest_sha256: before.manifest_sha256.clone(),
            files: vec![CandidateTreeFilePlan {
                operation_id: "selected-source".into(),
                path: "src/code.rs".into(),
                postimage_sha256: hash(b"postimage"),
                mode: CommitFileMode::Regular,
            }],
            hooks_enabled: false,
            filters_enabled: false,
            repository_program_selection_enabled: false,
            network_authority: false,
            plan_sha256: "0".repeat(64),
        };
        plan.plan_sha256 = hash(&serde_json::to_vec(&plan).unwrap());
        let now = Instant::now();
        Self {
            before,
            after,
            plan,
            deadline: now + Duration::from_secs(30),
            now: Cell::new(now),
            cancellation: CancellationToken::root(
                BoundaryKind::Kernel,
                TaskId::from_raw("complete-candidate-test"),
                CorrelationId::from_raw("complete-candidate-test"),
            ),
            calls: vec![],
            fail: None,
            stop_after: None,
            cancel: false,
            checks: Cell::new(0),
            expire_on_check: None,
        }
    }

    fn step(&mut self, name: &'static str) -> Result<(), ()> {
        self.calls.push(name);
        if self.stop_after == Some(name) {
            if self.cancel {
                self.cancellation
                    .cancel(agentmage_kernel_contracts::CancellationSignal {
                        schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                        cancellation_id: agentmage_kernel_contracts::CancellationId::from_raw(
                            "candidate-test-cancel",
                        ),
                        task_id: TaskId::from_raw("complete-candidate-test"),
                        correlation_id: CorrelationId::from_raw("complete-candidate-test"),
                        requested_by: BoundaryKind::Kernel,
                        reason: agentmage_kernel_contracts::CancellationReason::UserRequested,
                    })
                    .unwrap();
            } else {
                // Controlled monotonic time, no short real-time success window.
                self.now.set(self.deadline);
            }
        }
        if self.fail == Some(name) {
            Err(())
        } else {
            Ok(())
        }
    }

    fn run(&mut self) -> Result<CandidateTreePlatformResult, LinuxLocalCommitError> {
        self.run_images(vec![
            LinuxApprovedPostimage::new("selected-source", b"postimage".to_vec()).unwrap(),
        ])
    }

    fn run_images(
        &mut self,
        images: Vec<LinuxApprovedPostimage>,
    ) -> Result<CandidateTreePlatformResult, LinuxLocalCommitError> {
        run_with(&self.plan.clone(), images, self.deadline, self)
    }
}

impl Session for SyntheticSession {
    fn check(&self) -> Result<(), LinuxLocalCommitError> {
        let checks = self.checks.get() + 1;
        self.checks.set(checks);
        if self.expire_on_check == Some(checks) {
            self.now.set(self.deadline);
        }
        check_at(&self.cancellation, self.deadline, self.now.get())
    }

    fn collect(&mut self, deadline: Instant) -> Result<RepositoryPreservationManifest, ()> {
        assert_eq!(
            deadline, self.deadline,
            "collection must inherit the original deadline"
        );
        if self.calls.is_empty() {
            self.step("before")?;
            Ok(self.before.clone())
        } else {
            self.step("after")?;
            Ok(self.after.clone())
        }
    }

    fn start(&mut self, plan: &CandidateTreePlan, deadline: Instant) -> Result<(), ()> {
        assert_eq!(
            deadline, self.deadline,
            "scratch setup must not renew the budget"
        );
        assert_eq!(plan, &self.plan);
        self.step("start")
    }

    fn read_tree(&mut self) -> Result<(), ()> {
        self.step("read")
    }

    fn add(&mut self, operation: &str, bytes: &[u8]) -> Result<String, ()> {
        assert_eq!(operation, "selected-source");
        assert_eq!(bytes, b"postimage");
        self.step("add")?;
        Ok("b".repeat(self.plan.parent_object.len()))
    }

    fn write_tree(&mut self) -> Result<String, ()> {
        self.step("write")?;
        Ok("c".repeat(self.plan.parent_object.len()))
    }

    fn cleanup(&mut self) -> bool {
        self.step("cleanup").is_ok()
    }
}

#[test]
fn complete_candidate_budget_preserves_order_exact_inputs_and_reconciled_results() {
    for width in [40, 64] {
        let mut s = SyntheticSession::new(width);
        let result = s.run().unwrap();
        assert_eq!(s.calls, ORDER);
        assert_eq!(result.before, s.before);
        assert_eq!(result.after, s.after);
        assert!(result.cleanup_verified);
        let receipt = reconcile_candidate_tree(&s.plan, result).unwrap();
        assert_eq!(receipt.candidate_tree, "c".repeat(width));
        assert_eq!(receipt.blobs[0].blob_object, "b".repeat(width));
        assert!(!receipt.ref_update_authority);
    }
}

#[test]
fn complete_candidate_budget_stops_at_every_boundary_but_always_cleans_started_scratch() {
    for cancel in [false, true] {
        for (i, stage) in ORDER.iter().enumerate() {
            let mut s = SyntheticSession::new(40);
            s.stop_after = Some(stage);
            s.cancel = cancel;
            assert_eq!(
                s.run().unwrap_err().kind(),
                LinuxLocalCommitErrorKind::ExecutionFailed
            );
            let mut expected = ORDER[..=i].to_vec();
            if (1..5).contains(&i) {
                expected.push("cleanup");
            }
            assert_eq!(s.calls, expected, "{cancel} after {stage}");
            assert_eq!(s.cancellation.is_cancelled(), cancel);
        }
    }
}

#[test]
fn complete_candidate_budget_refuses_phase_failures_and_bad_postimages_without_success() {
    for (i, stage) in ORDER.iter().enumerate() {
        let mut s = SyntheticSession::new(40);
        s.fail = Some(stage);
        assert!(s.run().is_err(), "{stage}");
        let mut expected = ORDER[..=i].to_vec();
        if (2..5).contains(&i) {
            expected.extend(["cleanup", "after"]);
        } else if i == 5 {
            expected.push("after");
        }
        assert_eq!(s.calls, expected, "{stage}");
    }
    for kind in [
        "missing",
        "duplicate",
        "wrong-operation",
        "wrong-bytes",
        "manifest",
    ] {
        let mut s = SyntheticSession::new(40);
        let mut images =
            vec![LinuxApprovedPostimage::new("selected-source", b"postimage".to_vec()).unwrap()];
        match kind {
            "missing" => images.clear(),
            "duplicate" => images.push(
                LinuxApprovedPostimage::new("selected-source", b"postimage".to_vec()).unwrap(),
            ),
            "wrong-operation" => images[0].operation_id = "another".into(),
            "wrong-bytes" => images[0].bytes.push(b'!'),
            "manifest" => s.before.manifest_sha256 = hash(b"another manifest"),
            _ => unreachable!(),
        }
        assert_eq!(
            s.run_images(images).unwrap_err().kind(),
            LinuxLocalCommitErrorKind::PlanDenied
        );
        assert_eq!(s.calls, ["before"], "{kind}");
    }
}

#[test]
fn complete_candidate_budget_checks_expired_entry_late_postflight_and_chunked_hashing() {
    let mut s = SyntheticSession::new(40);
    s.now.set(s.deadline);
    assert_eq!(
        s.run().unwrap_err().kind(),
        LinuxLocalCommitErrorKind::ExecutionFailed
    );
    assert!(s.calls.is_empty());
    // Hash checkpoints scale with bounded chunks, including before/after an
    // empty postimage. No temporary file or worker is involved in hashing.
    for length in [0, 1, HASH_CHUNK, HASH_CHUNK + 1, 3 * HASH_CHUNK] {
        let bytes = vec![b'x'; length];
        let s = SyntheticSession::new(40);
        assert_eq!(checked_hash(&bytes, &s).unwrap(), hash(&bytes));
        assert_eq!(s.checks.get(), length.div_ceil(HASH_CHUNK) + 2);
        for checkpoint in 1..=s.checks.get() {
            let mut stopped = SyntheticSession::new(40);
            stopped.expire_on_check = Some(checkpoint);
            assert!(checked_hash(&bytes, &stopped).is_err());
            assert_eq!(stopped.checks.get(), checkpoint);
            assert!(stopped.calls.is_empty());
        }
    }
    // Entry time below expiry passes, exclusive expiry refuses without ever
    // cancelling the original token. The full sequence above exercises a late
    // successful postflight at exactly expiry and refuses it after cleanup.
    let s = SyntheticSession::new(40);
    assert!(
        check_at(
            &s.cancellation,
            s.deadline,
            s.deadline - Duration::from_nanos(1)
        )
        .is_ok()
    );
    assert!(check_at(&s.cancellation, s.deadline, s.deadline).is_err());
    assert!(!s.cancellation.is_cancelled());
}
