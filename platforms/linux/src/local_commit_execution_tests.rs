// SPDX-License-Identifier: MIT OR Apache-2.0
//! Executor ordering with synthetic outcomes, never native or approval proof.

use super::*;
use agentmage_kernel_contracts::{
    BoundaryKind, CONTRACT_SCHEMA_VERSION, CancellationId, CancellationReason, CancellationSignal,
    CorrelationId, TaskId,
};

const ORDER: [&str; 7] = [
    "branch",
    "sign",
    "write",
    "read",
    "verify",
    "update",
    "postflight",
];

struct SyntheticSession {
    plan: LocalCommitPlan,
    after: RepositoryPreservationManifest,
    calls: Vec<&'static str>,
    fail: Option<&'static str>,
    signer_error: LinuxLocalCommitErrorKind,
    raw: Option<Vec<u8>>,
    stored: Option<(Vec<u8>, String)>,
    cancellation: CancellationToken,
    deadline: Instant,
    stop_after: Option<&'static str>,
    expire: bool,
}

impl SyntheticSession {
    fn new(width: usize) -> Self {
        let mut plan = super::super::tests::synthetic_plan(&hash(b"executor fixture"));
        plan.parent_object = "b".repeat(width);
        plan.candidate_tree = "c".repeat(width);
        plan.plan_sha256 = "0".repeat(64);
        plan.plan_sha256 = hash(&serde_json::to_vec(&plan).unwrap());
        let mut after = before();
        // Distinct final observations must be forwarded, not replaced with the
        // preflight manifest when publishing the platform result.
        after.index_sha256 = hash(b"observed final index");
        after.remotes_sha256 = hash(b"observed final remotes");
        Self {
            plan,
            after,
            calls: Vec::new(),
            fail: None,
            signer_error: LinuxLocalCommitErrorKind::VerificationFailed,
            raw: None,
            stored: None,
            cancellation: token(),
            deadline: Instant::now() + Duration::from_secs(30),
            stop_after: None,
            expire: false,
        }
    }

    fn step(&mut self, name: &'static str) -> Result<(), ()> {
        self.calls.push(name);
        if self.stop_after == Some(name) {
            self.stop_control();
        }
        if self.fail == Some(name) {
            Err(())
        } else {
            Ok(())
        }
    }
    fn stop_control(&mut self) {
        if self.expire {
            self.deadline = Instant::now();
        } else {
            cancel(&self.cancellation);
        }
    }
    fn run(&mut self) -> LocalCommitPlatformResult {
        run_with(&self.plan.clone(), before(), self)
    }
}

impl Session for SyntheticSession {
    fn control_active(&self) -> bool {
        control_active(&self.cancellation, self.deadline)
    }
    fn branch_before(&mut self) -> Result<(), ()> {
        self.step("branch")
    }
    fn sign(&mut self) -> Result<packet::VerifiedCommit, LinuxLocalCommitError> {
        self.step("sign").map_err(|()| error(self.signer_error))?;
        Ok(packet::tests::fixture_verified(&self.plan))
    }
    fn write(&mut self, signed: &packet::VerifiedCommit) -> Result<String, ()> {
        self.step("write")?;
        // The runner must forward the exact opaque value returned by signing,
        // not reconstruct bytes or substitute an object identity.
        let (raw, object) = signed.for_plan(&self.plan).unwrap();
        self.stored = Some((raw.to_vec(), object.into()));
        Ok(object.into())
    }
    fn read(&mut self, object: &str) -> Result<Vec<u8>, ()> {
        self.step("read")?;
        let (raw, stored_object) = self.stored.as_ref().expect("write precedes read");
        assert_eq!(object, stored_object);
        Ok(self.raw.clone().unwrap_or_else(|| raw.clone()))
    }
    fn verify(&mut self, raw: &[u8]) -> Result<(), LinuxLocalCommitError> {
        self.step("verify").map_err(|()| error(self.signer_error))?;
        assert_eq!(raw, self.stored.as_ref().unwrap().0);
        Ok(())
    }
    fn update(&mut self) -> Result<(), ()> {
        self.step("update")
    }
    fn postflight(&mut self) -> Result<RepositoryPreservationManifest, ()> {
        self.step("postflight")?;
        Ok(self.after.clone())
    }
}

fn token() -> CancellationToken {
    CancellationToken::root(
        BoundaryKind::Kernel,
        TaskId::from_raw("commit-control"),
        CorrelationId::from_raw("commit-control"),
    )
}

fn cancel(token: &CancellationToken) {
    token
        .cancel(CancellationSignal {
            schema_version: CONTRACT_SCHEMA_VERSION,
            cancellation_id: CancellationId::from_raw("cancel-commit-control"),
            task_id: TaskId::from_raw("commit-control"),
            correlation_id: CorrelationId::from_raw("commit-control"),
            reason: CancellationReason::UserRequested,
            requested_by: BoundaryKind::Kernel,
        })
        .unwrap();
}

#[test]
fn commit_execution_cancellation_stops_before_every_next_stage_and_after_postflight() {
    assert_stopped_boundaries(false);
}

#[test]
fn commit_execution_expiry_stops_before_every_next_stage_and_after_postflight() {
    assert_stopped_boundaries(true);
}

fn assert_stopped_boundaries(expire: bool) {
    for width in [40, 64] {
        for completed in 0..=ORDER.len() {
            let mut s = SyntheticSession::new(width);
            s.expire = expire;
            if completed == 0 {
                s.stop_control();
            } else {
                s.stop_after = Some(ORDER[completed - 1]);
            }
            assert_uncertain(&s.run(), "linux.git.commit.control_stopped");
            assert_eq!(s.calls, ORDER[..completed]);
            assert_eq!(s.stored.is_some(), completed >= 3);
            assert_eq!(s.calls.contains(&"update"), completed >= 6);
        }
    }
}

#[test]
fn commit_execution_preflight_checks_cancellation_before_and_after_collection() {
    let t = token();
    cancel(&t);
    assert!(
        preflight(&t, || panic!(
            "cancelled preflight must not inspect Git or files"
        ))
        .is_err()
    );

    let t = token();
    let propagated = t.clone();
    assert!(
        preflight(&t, || {
            cancel(&propagated);
            Ok(before())
        })
        .is_err()
    );

    let t = token();
    assert_eq!(preflight(&t, || Ok(before())).unwrap(), before());
    assert!(control_active(&t, Instant::now() + Duration::from_secs(30)));
    assert!(!control_active(&t, Instant::now()));
    cancel(&t);
    assert!(!control_active(
        &t,
        Instant::now() + Duration::from_secs(30)
    ));
}

fn before() -> RepositoryPreservationManifest {
    serde_json::from_str(include_str!(
        "../../../schemas/runtime/examples/repository-preservation-manifest.valid.json"
    ))
    .unwrap()
}

fn assert_uncertain(result: &LocalCommitPlatformResult, code: &str) {
    assert_eq!(result.outcome, OperationOutcome::Failed);
    assert_eq!(result.platform_code, code);
    assert!(!result.cleanup_verified);
    assert!(!result.signature_verified);
    assert_eq!(result.before, before());
    assert_eq!(result.after, before());
    assert!(result.commit_object.is_none());
    assert!(result.observed_parent.is_none());
    assert!(result.observed_tree.is_none());
    assert!(result.observed_message_sha256.is_none());
    assert!(result.observed_identity_sha256.is_none());
    assert!(result.observed_signer_fingerprint.is_none());
    assert!(result.task_branch_before.is_none());
    assert!(result.task_branch_after.is_none());
    assert_eq!(result.user_index_after_sha256, before().index_sha256);
    assert_eq!(result.remotes_after_sha256, before().remotes_sha256);
    assert!(!result.network_used);
    assert!(!result.hooks_executed);
    assert!(!result.filters_executed);
    assert!(!result.repository_selected_program_used);
}

#[test]
fn commit_execution_orders_verified_storage_readback_reverification_cas_and_postflight() {
    for width in [40, 64] {
        let mut s = SyntheticSession::new(width);
        let result = s.run();
        assert_eq!(s.calls, ORDER);
        let object = s.stored.as_ref().unwrap().1.clone();
        assert_eq!(object.len(), width);
        assert_eq!(result.outcome, OperationOutcome::Succeeded);
        assert_eq!(result.platform_code, "linux.git.commit.succeeded");
        assert_eq!(result.before, before());
        assert_eq!(result.after, s.after);
        assert_eq!(result.commit_object, Some(object.clone()));
        assert_eq!(result.observed_parent, Some(s.plan.parent_object.clone()));
        assert_eq!(result.observed_tree, Some(s.plan.candidate_tree.clone()));
        assert_eq!(
            result.observed_message_sha256,
            Some(s.plan.message_sha256.clone())
        );
        assert_eq!(
            result.observed_identity_sha256,
            Some(s.plan.identity_sha256.clone())
        );
        assert_eq!(
            result.observed_signer_fingerprint,
            Some(s.plan.signer.key_fingerprint.clone())
        );
        assert_eq!(result.task_branch_before, Some(s.plan.parent_object));
        assert_eq!(result.task_branch_after, Some(object));
        assert_eq!(result.user_index_after_sha256, s.after.index_sha256);
        assert_eq!(result.remotes_after_sha256, s.after.remotes_sha256);
        assert!(result.signature_verified && result.cleanup_verified);
        assert!(!result.network_used);
        assert!(!result.hooks_executed);
        assert!(!result.filters_executed);
        assert!(!result.repository_selected_program_used);
    }
}

#[test]
fn commit_execution_stops_every_failed_stage_without_retry_or_success_claims() {
    let codes = [
        "linux.git.commit.branch_stale",
        "linux.git.commit.creation_failed",
        "linux.git.commit.storage_failed",
        "linux.git.commit.verification_failed",
        "linux.git.commit.verification_failed",
        "linux.git.commit.compare_and_swap_failed",
        "linux.git.commit.postflight",
    ];
    for width in [40, 64] {
        for (i, &stage) in ORDER.iter().enumerate() {
            let mut s = SyntheticSession::new(width);
            s.fail = Some(stage);
            assert_uncertain(&s.run(), codes[i]);
            assert_eq!(s.calls, ORDER[..=i], "must stop at {stage}");
            if i < 5 {
                assert!(!s.calls.contains(&"update"));
            }
        }
    }
}

#[test]
fn commit_execution_refuses_corrupt_readback_before_verifier_and_ref_update() {
    for width in [40, 64] {
        let p = SyntheticSession::new(width).plan;
        let verified = packet::tests::fixture_verified(&p);
        let raw = verified.for_plan(&p).unwrap().0;
        let mut changed_tree = raw.to_vec();
        changed_tree[5] = b'd';
        let mut changed_message = raw.to_vec();
        *changed_message.last_mut().unwrap() = b'!';
        for bad in [
            Vec::new(),
            raw[..raw.len() - 1].to_vec(),
            changed_tree,
            changed_message,
        ] {
            let mut s = SyntheticSession::new(width);
            s.raw = Some(bad);
            assert_uncertain(&s.run(), "linux.git.commit.verification_failed");
            assert_eq!(s.calls, ORDER[..4]);
            assert!(!s.calls.contains(&"verify"));
            assert!(!s.calls.contains(&"update"));
        }
    }
}

#[test]
fn commit_execution_signature_errors_after_storage_never_reach_cas_and_keep_uncertainty() {
    for width in [40, 64] {
        for stage in ["sign", "verify"] {
            for kind in [
                LinuxLocalCommitErrorKind::ArtifactDenied,
                LinuxLocalCommitErrorKind::ScopeDenied,
                LinuxLocalCommitErrorKind::PlanDenied,
                LinuxLocalCommitErrorKind::ExecutionFailed,
                LinuxLocalCommitErrorKind::CleanupUncertain,
                LinuxLocalCommitErrorKind::VerificationFailed,
            ] {
                let mut s = SyntheticSession::new(width);
                s.fail = Some(stage);
                s.signer_error = kind;
                let code = if kind == LinuxLocalCommitErrorKind::CleanupUncertain {
                    "linux.git.commit.signer_cleanup_uncertain"
                } else if stage == "sign" {
                    "linux.git.commit.creation_failed"
                } else {
                    "linux.git.commit.verification_failed"
                };
                assert_uncertain(&s.run(), code);
                assert_eq!(s.calls, ORDER[..if stage == "sign" { 2 } else { 5 }]);
                assert!(!s.calls.contains(&"update"));
                assert_eq!(s.stored.is_some(), stage == "verify");
            }
        }
    }
}
