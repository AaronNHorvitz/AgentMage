// SPDX-License-Identifier: Apache-2.0
//! Synthetic complete packets and approvals; no native effect or human consent.

use super::*;
use crate::review_packet::{
    ReviewFindingInput, ReviewMode, commit_packet_for_test, normalize_review_findings,
};

fn cancellation() -> CancellationToken {
    CancellationToken::root(
        agentmage_kernel_contracts::BoundaryKind::Kernel,
        agentmage_kernel_contracts::TaskId::from_raw("fixture-task"),
        agentmage_kernel_contracts::CorrelationId::from_raw("fixture-correlation"),
    )
}

fn packet(
    severity: Option<FindingSeverity>,
    confidence: u16,
    duplicate: bool,
) -> LocalReviewPacket {
    let findings = severity.map_or_else(Vec::new, |severity| {
        (0..if duplicate { 2 } else { 1 })
            .map(|index| ReviewFindingInput {
                finding_id: format!("fixture-finding-{index}"),
                mode: ReviewMode::Correctness,
                code: "review.correctness.boundary".into(),
                path: "src/lib.rs".into(),
                line: 7,
                severity,
                confidence_bps: confidence,
                evidence_sha256: "1".repeat(64),
                rationale_code: "fixture.boundary".into(),
            })
            .collect()
    });
    commit_packet_for_test(
        normalize_review_findings(vec![ReviewMode::Correctness], findings).unwrap(),
    )
}

// Deliberately assemble the plan without plan_local_commit: a caller can hash
// its own inert plan, but that must not bypass complete-packet admission.
fn inputs(packet: &LocalReviewPacket) -> (LocalCommitPlan, RepositoryPreservationManifest) {
    let mut before: RepositoryPreservationManifest = serde_json::from_str(include_str!(
        "../../../schemas/runtime/examples/repository-preservation-manifest.valid.json"
    ))
    .unwrap();
    before
        .repository_sha256
        .clone_from(&packet.repository_sha256);
    let before = RepositoryPreservationManifest::seal(before).unwrap();
    let signer = PinnedCommitSigner::seal(PinnedCommitSigner {
        schema_version: 0,
        signer_id: "synthetic-signer".into(),
        kind: CommitSignerKind::OpenPgp,
        source: SignerInspectionSource::ExternalKeyring,
        signer_path_sha256: "1".repeat(64),
        signer_executable_sha256: "2".repeat(64),
        key_fingerprint: "1234567890abcdef1234567890abcdef12345678".into(),
        public_key_sha256: "3".repeat(64),
        inspection_evidence_sha256: "4".repeat(64),
        repository_selected_program_used: false,
        unsigned_fallback: false,
        signer_sha256: String::new(),
    })
    .unwrap();
    let identity = CommitIdentity {
        name: "Fixture".into(),
        email: "fixture@example.test".into(),
    };
    let group = &packet.commit_plan.groups[0];
    let mut plan = LocalCommitPlan {
        schema_version: 1,
        commit_transaction_id: "fixture-commit".into(),
        repository_sha256: packet.repository_sha256.clone(),
        review_packet_sha256: packet.packet_sha256.clone(),
        candidate_tree_receipt_sha256: "a".repeat(64),
        commit_group_sha256: group.group_sha256.clone(),
        parent_object: packet.base_object.clone(),
        candidate_tree: "b".repeat(40),
        message: group.proposed_message.clone(),
        message_sha256: sha256_hex(group.proposed_message.as_bytes()),
        author: identity.clone(),
        committer: identity.clone(),
        identity_sha256: canonical_sha256(&(identity.clone(), identity)).unwrap(),
        timestamp_epoch_seconds: 1_787_000_000,
        timezone_offset_minutes: 0,
        signer,
        task_branch: "refs/heads/agentmage/tasks/fixture".into(),
        preservation_manifest_sha256: before.manifest_sha256.clone(),
        user_index_sha256: before.index_sha256.clone(),
        automatic_commit: false,
        push_authority: false,
        merge_authority: false,
        release_authority: false,
        history_rewrite_authority: false,
        reset_authority: false,
        discard_authority: false,
        force_authority: false,
        network_authority: false,
        plan_sha256: ZERO_SHA256.into(),
    };
    reseal(&mut plan);
    (plan, before)
}

fn reseal(plan: &mut LocalCommitPlan) {
    plan.plan_sha256 = ZERO_SHA256.into();
    plan.plan_sha256 = canonical_sha256(plan).unwrap();
    verify_local_commit_plan(plan).unwrap();
}

fn decision() -> ManualCommitApprovalDecision {
    ManualCommitApprovalDecision {
        approval_id: "synthetic-approval".into(),
        approval_channel_sha256: "d".repeat(64),
        approved_at_epoch_ms: 10_000,
        expires_at_epoch_ms: 20_000,
        human_confirmed: true,
    }
}

#[test]
fn preparation_refuses_material_findings_even_with_forged_valid_plan_and_approval() {
    for severity in [
        FindingSeverity::Info,
        FindingSeverity::Low,
        FindingSeverity::Medium,
        FindingSeverity::High,
        FindingSeverity::Critical,
    ] {
        for confidence in [0, 6999, 7000, 10000] {
            for duplicate in [false, true] {
                let packet = packet(Some(severity), confidence, duplicate);
                assert!(verify_review_packet(&packet));
                assert_eq!(
                    packet.review_results.suppressed.len(),
                    usize::from(duplicate) + usize::from(confidence < 7000)
                );
                let (plan, _) = inputs(&packet);
                let raw = record_manual_commit_approval(&plan, decision()).unwrap();
                verify_manual_commit_approval(&plan, &raw, 15_000).unwrap();
                let blocked = matches!(
                    severity,
                    FindingSeverity::Medium | FindingSeverity::High | FindingSeverity::Critical
                );
                let recorded = record_reviewed_manual_commit_approval(&plan, &packet, decision());
                let verified = verify_reviewed_manual_commit_approval(&plan, &packet, &raw, 15_000);
                let prepared = PreparedLocalCommit::new(plan, packet, raw, 15_000);
                assert_eq!(
                    recorded.is_err(),
                    blocked,
                    "{severity:?}/{confidence}/{duplicate}"
                );
                assert_eq!(
                    verified.is_err(),
                    blocked,
                    "{severity:?}/{confidence}/{duplicate}"
                );
                assert_eq!(
                    prepared.is_err(),
                    blocked,
                    "{severity:?}/{confidence}/{duplicate}"
                );
                if blocked {
                    assert_eq!(recorded.unwrap_err(), LocalCommitError::ReviewMismatch);
                    assert_eq!(verified.unwrap_err(), LocalCommitError::ReviewMismatch);
                    assert_eq!(prepared.unwrap_err(), LocalCommitError::ReviewMismatch);
                }
            }
        }
    }
}

#[test]
fn preparation_refuses_packet_and_resealed_commit_binding_drift() {
    for field in [
        "packet",
        "repository",
        "parent",
        "group",
        "message",
        "integrity",
    ] {
        let mut packet = packet(None, 0, false);
        let (mut plan, _) = inputs(&packet);
        match field {
            "packet" => plan.review_packet_sha256 = "a".repeat(64),
            "repository" => plan.repository_sha256 = "a".repeat(64),
            "parent" => plan.parent_object = "a".repeat(40),
            "group" => plan.commit_group_sha256 = "a".repeat(64),
            "message" => {
                plan.message.push_str(" changed");
                plan.message_sha256 = sha256_hex(plan.message.as_bytes());
            }
            "integrity" => packet.objective.push_str(" unsealed"),
            _ => unreachable!(),
        }
        reseal(&mut plan);
        let raw = record_manual_commit_approval(&plan, decision()).unwrap();
        verify_manual_commit_approval(&plan, &raw, 15_000).unwrap();
        assert_eq!(
            record_reviewed_manual_commit_approval(&plan, &packet, decision()),
            Err(LocalCommitError::ReviewMismatch),
            "{field}"
        );
        assert_eq!(
            verify_reviewed_manual_commit_approval(&plan, &packet, &raw, 15_000),
            Err(LocalCommitError::ReviewMismatch),
            "{field}"
        );
        assert_eq!(
            PreparedLocalCommit::new(plan, packet, raw, 15_000).unwrap_err(),
            LocalCommitError::ReviewMismatch,
            "{field}"
        );
    }
}

#[test]
fn preparation_retains_exact_packet_canonical_request_and_expiry_checks() {
    let mut packet = packet(None, 0, false);
    let (plan, before) = inputs(&packet);
    let approval = record_reviewed_manual_commit_approval(&plan, &packet, decision()).unwrap();
    let prepared =
        PreparedLocalCommit::new(plan.clone(), packet.clone(), approval.clone(), 15_000).unwrap();
    assert_eq!(
        sha256_hex(prepared.request_bytes()),
        prepared.request_sha256()
    );
    assert_eq!(prepared.plan(), &plan);
    assert_eq!(prepared.packet, packet);
    packet.objective.push_str(" changed external clone");
    assert!(prepared.verify_at(15_000).is_ok());
    assert!(!format!("{prepared:?}").contains(&prepared.packet.objective));
    for now in [9_999, 20_000] {
        assert_eq!(
            prepared.verify_at(now),
            Err(LocalCommitError::ApprovalDenied)
        );
        assert_eq!(
            PreparedLocalCommit::new(plan.clone(), prepared.packet.clone(), approval.clone(), now)
                .unwrap_err(),
            LocalCommitError::ApprovalDenied
        );
        assert_eq!(
            LocalCommitEffectDriver::new(
                (),
                (),
                prepared.clone(),
                before.clone(),
                cancellation(),
                FixedClock(now),
            )
            .unwrap_err(),
            LocalCommitError::ApprovalDenied
        );
    }
    assert!(
        LocalCommitEffectDriver::new((), (), prepared, before, cancellation(), FixedClock(15_000),)
            .is_ok()
    );
}

#[test]
fn preparation_and_driver_recheck_retained_packet_before_launch() {
    for field in ["integrity", "material", "binding"] {
        let packet = packet(None, 0, false);
        let (plan, before) = inputs(&packet);
        let approval = record_reviewed_manual_commit_approval(&plan, &packet, decision()).unwrap();
        let mut prepared = PreparedLocalCommit::new(plan, packet, approval, 15_000).unwrap();
        match field {
            "integrity" => prepared.packet.objective.push_str(" unsealed"),
            "material" => {
                prepared.packet = self::packet(Some(FindingSeverity::Medium), 0, false);
                prepared
                    .plan
                    .review_packet_sha256
                    .clone_from(&prepared.packet.packet_sha256);
                reseal(&mut prepared.plan);
                prepared.approval =
                    record_manual_commit_approval(&prepared.plan, decision()).unwrap();
            }
            "binding" => {
                prepared.plan.message.push_str(" changed");
                prepared.plan.message_sha256 = sha256_hex(prepared.plan.message.as_bytes());
                reseal(&mut prepared.plan);
                prepared.approval =
                    record_manual_commit_approval(&prepared.plan, decision()).unwrap();
            }
            _ => unreachable!(),
        }
        // Only this module-private test can alter retained fields. This is the
        // same guard invoked at actual dispatch, not an authorized effect run.
        assert_eq!(
            prepared.verify_at(15_000),
            Err(LocalCommitError::ReviewMismatch)
        );
        assert_eq!(
            LocalCommitEffectDriver::new(
                (),
                (),
                prepared,
                before,
                cancellation(),
                FixedClock(15_000),
            )
            .unwrap_err(),
            LocalCommitError::ReviewMismatch,
            "{field}"
        );
    }
}

struct FixedClock(u64);

impl RuntimeClock for FixedClock {
    fn now_epoch_ms(&mut self) -> Result<u64, crate::runtime_loop::RuntimePortFailure> {
        Ok(self.0)
    }
}

struct SequenceClock(
    std::collections::VecDeque<Result<u64, crate::runtime_loop::RuntimePortFailure>>,
);

impl RuntimeClock for SequenceClock {
    fn now_epoch_ms(&mut self) -> Result<u64, crate::runtime_loop::RuntimePortFailure> {
        self.0.pop_front().expect("unexpected clock sample")
    }
}

#[derive(Default)]
struct SyntheticExecutor {
    launches: usize,
}

impl BoundedLocalCommitExecutor for SyntheticExecutor {
    fn execute(
        &mut self,
        permit: LocalCommitLaunchPermit<'_>,
        cancellation: &CancellationToken,
    ) -> LocalCommitPlatformResult {
        assert!(!cancellation.is_cancelled());
        self.launches += 1;
        let plan = permit.plan();
        let before = permit.before();
        let mut after = before.clone();
        after.object_database_sha256 = "a".repeat(64);
        after.agentmage_refs_sha256 = "b".repeat(64);
        let after = RepositoryPreservationManifest::seal(after).unwrap();
        let object = "f".repeat(40);
        LocalCommitPlatformResult {
            outcome: OperationOutcome::Succeeded,
            before: before.clone(),
            after,
            commit_object: Some(object.clone()),
            observed_parent: Some(plan.parent_object.clone()),
            observed_tree: Some(plan.candidate_tree.clone()),
            observed_message_sha256: Some(plan.message_sha256.clone()),
            observed_identity_sha256: Some(plan.identity_sha256.clone()),
            observed_signer_fingerprint: Some(plan.signer.key_fingerprint.clone()),
            signature_verified: true,
            task_branch_before: Some(plan.parent_object.clone()),
            task_branch_after: Some(object),
            user_index_after_sha256: before.index_sha256.clone(),
            remotes_after_sha256: before.remotes_sha256.clone(),
            hooks_executed: false,
            filters_executed: false,
            repository_selected_program_used: false,
            network_used: false,
            cleanup_verified: true,
            platform_code: "fixture.commit.succeeded".into(),
        }
    }
}

pub(super) fn prepared_fixture() -> (PreparedLocalCommit, RepositoryPreservationManifest) {
    let packet = packet(None, 0, false);
    let (plan, before) = inputs(&packet);
    let approval = record_reviewed_manual_commit_approval(&plan, &packet, decision()).unwrap();
    (
        PreparedLocalCommit::new(plan, packet, approval, 15_000).unwrap(),
        before,
    )
}

#[test]
fn commit_dispatch_samples_clock_and_refuses_expired_approval_or_consumed_grant() {
    use crate::authority_transaction::tests::{commit_held_fixture, dispatch_commit_fixture};
    use crate::runtime_loop::RuntimePortFailure;
    for now in [
        Ok(15_000),
        Ok(19_999),
        Ok(20_000),
        Ok(30_000),
        Ok(14_999),
        Ok(0),
        Err(RuntimePortFailure::Unavailable),
        Err(RuntimePortFailure::TimedOut),
    ] {
        let (prepared, before) = prepared_fixture();
        let arguments = prepared.request_bytes().to_vec();
        let clock = SequenceClock([Ok(15_000), now].into());
        let mut driver = LocalCommitEffectDriver::new(
            SyntheticExecutor::default(),
            commit_held_fixture(),
            prepared,
            before,
            cancellation(),
            clock,
        )
        .unwrap();
        let receipt = dispatch_commit_fixture(&arguments, &mut driver);
        let allowed = matches!(now, Ok(15_000 | 19_999));
        assert_eq!(
            receipt.outcome == OperationOutcome::Succeeded,
            allowed,
            "{now:?}"
        );
        assert_eq!(driver.take_receipt().is_some(), allowed, "{now:?}");
        assert_eq!(
            driver.take_error(),
            if allowed {
                None
            } else {
                Some(LocalCommitError::ApprovalDenied)
            },
            "{now:?}"
        );
        assert!(driver.clock.0.is_empty());
        assert_eq!(driver.into_executor().launches, usize::from(allowed));
    }
    // A forward clock from construction can still precede grant consumption.
    let (prepared, before) = prepared_fixture();
    let arguments = prepared.request_bytes().to_vec();
    let mut driver = LocalCommitEffectDriver::new(
        SyntheticExecutor::default(),
        commit_held_fixture(),
        prepared,
        before,
        cancellation(),
        SequenceClock([Ok(10_000), Ok(14_999)].into()),
    )
    .unwrap();
    let receipt = dispatch_commit_fixture(&arguments, &mut driver);
    assert_ne!(receipt.outcome, OperationOutcome::Succeeded);
    assert_eq!(driver.take_error(), Some(LocalCommitError::ApprovalDenied));
    assert!(driver.take_receipt().is_none());
    assert_eq!(driver.into_executor().launches, 0);
    // Separate the grant's shorter lifetime from manual-approval expiry.
    let (mut prepared, before) = prepared_fixture();
    let mut longer = decision();
    longer.expires_at_epoch_ms = 40_000;
    prepared.approval =
        record_reviewed_manual_commit_approval(&prepared.plan, &prepared.packet, longer).unwrap();
    let prepared =
        PreparedLocalCommit::new(prepared.plan, prepared.packet, prepared.approval, 15_000)
            .unwrap();
    let arguments = prepared.request_bytes().to_vec();
    for now in [29_999, 30_000] {
        let mut driver = LocalCommitEffectDriver::new(
            SyntheticExecutor::default(),
            commit_held_fixture(),
            prepared.clone(),
            before.clone(),
            cancellation(),
            SequenceClock([Ok(15_000), Ok(now)].into()),
        )
        .unwrap();
        let receipt = dispatch_commit_fixture(&arguments, &mut driver);
        assert_eq!(receipt.outcome == OperationOutcome::Succeeded, now < 30_000);
        assert_eq!(driver.take_receipt().is_some(), now < 30_000);
        assert_eq!(driver.into_executor().launches, usize::from(now < 30_000));
    }
}

#[test]
fn commit_dispatch_rechecks_packet_and_call_bindings_after_real_consumption() {
    use crate::authority_transaction::tests::{commit_held_fixture, dispatch_commit_fixture};
    for field in ["integrity", "material", "binding", "arguments", "cancelled"] {
        let (prepared, before) = prepared_fixture();
        let mut arguments = prepared.request_bytes().to_vec();
        let mut driver = LocalCommitEffectDriver::new(
            SyntheticExecutor::default(),
            commit_held_fixture(),
            prepared,
            before,
            cancellation(),
            SequenceClock([Ok(15_000), Ok(15_001)].into()),
        )
        .unwrap();
        match field {
            "integrity" => driver.prepared.packet.objective.push_str(" unsealed"),
            "material" => {
                driver.prepared.packet = packet(Some(FindingSeverity::Medium), 0, false);
                driver.prepared.plan.review_packet_sha256 =
                    driver.prepared.packet.packet_sha256.clone();
                reseal(&mut driver.prepared.plan);
                driver.prepared.approval =
                    record_manual_commit_approval(&driver.prepared.plan, decision()).unwrap();
            }
            "binding" => {
                driver.prepared.plan.message.push_str(" changed");
                driver.prepared.plan.message_sha256 =
                    sha256_hex(driver.prepared.plan.message.as_bytes());
                reseal(&mut driver.prepared.plan);
                driver.prepared.approval =
                    record_manual_commit_approval(&driver.prepared.plan, decision()).unwrap();
            }
            "arguments" => arguments = b"{}".to_vec(),
            "cancelled" => {
                driver
                    .cancellation
                    .cancel(agentmage_kernel_contracts::CancellationSignal {
                        schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                        cancellation_id: agentmage_kernel_contracts::CancellationId::from_raw(
                            "fixture-cancel",
                        ),
                        task_id: agentmage_kernel_contracts::TaskId::from_raw("fixture-task"),
                        correlation_id: agentmage_kernel_contracts::CorrelationId::from_raw(
                            "fixture-correlation",
                        ),
                        requested_by: agentmage_kernel_contracts::BoundaryKind::Kernel,
                        reason: agentmage_kernel_contracts::CancellationReason::UserRequested,
                    })
                    .unwrap();
            }
            _ => unreachable!(),
        }
        let receipt = dispatch_commit_fixture(&arguments, &mut driver);
        assert_ne!(receipt.outcome, OperationOutcome::Succeeded, "{field}");
        assert!(driver.take_receipt().is_none());
        assert_eq!(
            driver.take_error(),
            Some(if field == "cancelled" {
                LocalCommitError::ResultDenied
            } else {
                LocalCommitError::ApprovalDenied
            }),
            "{field}"
        );
        assert_eq!(driver.into_executor().launches, 0, "{field}");
    }
}

#[test]
fn commit_driver_refuses_unavailable_construction_clock() {
    let (prepared, before) = prepared_fixture();
    assert_eq!(
        LocalCommitEffectDriver::new(
            (),
            (),
            prepared,
            before,
            cancellation(),
            SequenceClock([Err(crate::runtime_loop::RuntimePortFailure::Unavailable)].into())
        )
        .unwrap_err(),
        LocalCommitError::ApprovalDenied
    );
}

#[test]
fn commit_driver_is_single_use_after_success_uncertainty_and_prelaunch_refusal() {
    use crate::authority_transaction::tests::{commit_held_fixture, dispatch_commit_fixture_as};
    use crate::runtime_loop::RuntimePortFailure;

    struct Executor {
        launches: usize,
        uncertain: bool,
    }
    impl BoundedLocalCommitExecutor for Executor {
        fn execute(
            &mut self,
            permit: LocalCommitLaunchPermit<'_>,
            token: &CancellationToken,
        ) -> LocalCommitPlatformResult {
            self.launches += 1;
            let mut result = SyntheticExecutor::default().execute(permit, token);
            result.cleanup_verified = !self.uncertain;
            self.uncertain = false;
            result
        }
    }
    for field in [
        "success",
        "uncertain",
        "clock",
        "expiry",
        "arguments",
        "packet",
        "cancelled",
    ] {
        let (prepared, before) = prepared_fixture();
        let original = prepared.clone();
        let arguments = prepared.request_bytes().to_vec();
        let first_time = match field {
            "clock" => Err(RuntimePortFailure::Unavailable),
            "expiry" => Ok(20_000),
            _ => Ok(15_001),
        };
        let mut driver = LocalCommitEffectDriver::new(
            Executor {
                launches: 0,
                uncertain: field == "uncertain",
            },
            commit_held_fixture(),
            prepared,
            before,
            cancellation(),
            SequenceClock([Ok(15_000), first_time, Ok(15_002)].into()),
        )
        .unwrap();
        if field == "packet" {
            driver.prepared.packet.objective.push_str(" unsealed");
        }
        if field == "cancelled" {
            driver
                .cancellation
                .cancel(agentmage_kernel_contracts::CancellationSignal {
                    schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                    cancellation_id: agentmage_kernel_contracts::CancellationId::from_raw(
                        "one-shot-cancel",
                    ),
                    task_id: agentmage_kernel_contracts::TaskId::from_raw("fixture-task"),
                    correlation_id: agentmage_kernel_contracts::CorrelationId::from_raw(
                        "fixture-correlation",
                    ),
                    requested_by: agentmage_kernel_contracts::BoundaryKind::Kernel,
                    reason: agentmage_kernel_contracts::CancellationReason::UserRequested,
                })
                .unwrap();
        }
        let first = dispatch_commit_fixture_as(
            if field == "arguments" {
                b"{}"
            } else {
                &arguments
            },
            &mut driver,
            GrantOperation::GitCommit,
            "commit-first",
            "attempt-first",
        );
        assert_eq!(
            first.outcome == OperationOutcome::Succeeded,
            field == "success",
            "{field}"
        );
        assert_eq!(driver.clock.0.len(), 1);
        let original_receipt = driver.receipt.clone();
        let original_error = driver.error;
        // Repair every first-entry refusal; the next grant must STILL refuse.
        // These module-private substitutions cannot be performed by callers.
        driver.prepared = original;
        driver.cancellation = cancellation();
        for (transaction, attempt) in [
            ("commit-second", "attempt-second"),
            ("commit-third", "attempt-third"),
        ] {
            let repeated = dispatch_commit_fixture_as(
                &arguments,
                &mut driver,
                GrantOperation::GitCommit,
                transaction,
                attempt,
            );
            assert_ne!(repeated.outcome, OperationOutcome::Succeeded, "{field}");
            assert_eq!(repeated.authority_transaction_id.as_str(), transaction);
            assert_eq!(repeated.operation_attempt_id.as_str(), attempt);
            assert_eq!(
                driver.clock.0.len(),
                1,
                "reentry must precede clock sampling"
            );
            assert_eq!(
                driver.take_receipt(),
                if transaction == "commit-second" {
                    original_receipt.clone()
                } else {
                    None
                }
            );
            assert_eq!(
                driver.take_error(),
                if transaction == "commit-second" {
                    original_error
                } else {
                    None
                }
            );
        }
        if let Some(receipt) = original_receipt {
            assert_eq!(
                receipt.authority_transaction_id,
                first.authority_transaction_id.as_str()
            );
            assert_eq!(
                receipt.operation_attempt_id,
                first.operation_attempt_id.as_str()
            );
        }
        assert_eq!(
            original_error,
            match field {
                "success" => None,
                "uncertain" | "cancelled" => Some(LocalCommitError::ResultDenied),
                _ => Some(LocalCommitError::ApprovalDenied),
            }
        );
        assert_eq!(
            driver.into_executor().launches,
            usize::from(matches!(field, "success" | "uncertain")),
            "{field}"
        );
    }
}

#[test]
fn commit_driver_cannot_relaunch_after_executor_unwinds() {
    use crate::authority_transaction::tests::{commit_held_fixture, dispatch_commit_fixture_as};
    struct Panics(usize);
    impl BoundedLocalCommitExecutor for Panics {
        fn execute(
            &mut self,
            _: LocalCommitLaunchPermit<'_>,
            _: &CancellationToken,
        ) -> LocalCommitPlatformResult {
            self.0 += 1;
            panic!("synthetic executor interruption; effects unknown");
        }
    }
    let (prepared, before) = prepared_fixture();
    let arguments = prepared.request_bytes().to_vec();
    let mut driver = LocalCommitEffectDriver::new(
        Panics(0),
        commit_held_fixture(),
        prepared,
        before,
        cancellation(),
        SequenceClock([Ok(15_000), Ok(15_001)].into()),
    )
    .unwrap();
    let interrupted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        dispatch_commit_fixture_as(
            &arguments,
            &mut driver,
            GrantOperation::GitCommit,
            "unwind-first",
            "unwind-first-attempt",
        )
    }));
    assert!(interrupted.is_err());
    let repeated = dispatch_commit_fixture_as(
        &arguments,
        &mut driver,
        GrantOperation::GitCommit,
        "unwind-second",
        "unwind-second-attempt",
    );
    assert_ne!(repeated.outcome, OperationOutcome::Succeeded);
    assert!(driver.take_receipt().is_none());
    assert!(driver.take_error().is_none());
    assert!(driver.clock.0.is_empty());
    assert_eq!(driver.into_executor().0, 1);
}

#[test]
fn commit_driver_latches_before_a_dispatch_clock_unwind() {
    use crate::authority_transaction::tests::{commit_held_fixture, dispatch_commit_fixture};
    struct Panics(u8);
    impl RuntimeClock for Panics {
        fn now_epoch_ms(&mut self) -> Result<u64, crate::runtime_loop::RuntimePortFailure> {
            self.0 += 1;
            if self.0 == 1 {
                Ok(15_000)
            } else {
                panic!("synthetic clock unavailable")
            }
        }
    }
    let (prepared, before) = prepared_fixture();
    let arguments = prepared.request_bytes().to_vec();
    let mut driver = LocalCommitEffectDriver::new(
        SyntheticExecutor::default(),
        commit_held_fixture(),
        prepared,
        before,
        cancellation(),
        Panics(0),
    )
    .unwrap();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            dispatch_commit_fixture(&arguments, &mut driver)
        }))
        .is_err()
    );
    assert_ne!(
        dispatch_commit_fixture(&arguments, &mut driver).outcome,
        OperationOutcome::Succeeded
    );
    assert_eq!(driver.clock.0, 2);
    assert!(driver.take_receipt().is_none());
    assert!(driver.take_error().is_none());
    assert_eq!(driver.into_executor().launches, 0);
}

#[test]
fn commit_dispatch_refuses_isolated_clock_rollback_with_live_grant_and_approval() {
    use crate::authority_transaction::tests::{commit_held_fixture, dispatch_commit_fixture};
    for now in [15_500, 16_000] {
        let (prepared, before) = prepared_fixture();
        let arguments = prepared.request_bytes().to_vec();
        let mut driver = LocalCommitEffectDriver::new(
            SyntheticExecutor::default(),
            commit_held_fixture(),
            prepared,
            before,
            cancellation(),
            SequenceClock([Ok(16_000), Ok(now)].into()),
        )
        .unwrap();
        let receipt = dispatch_commit_fixture(&arguments, &mut driver);
        assert_eq!(
            receipt.outcome == OperationOutcome::Succeeded,
            now == 16_000
        );
        assert_eq!(driver.take_receipt().is_some(), now == 16_000);
        assert_eq!(
            driver.take_error(),
            if now == 16_000 {
                None
            } else {
                Some(LocalCommitError::ApprovalDenied)
            }
        );
        assert_eq!(driver.into_executor().launches, usize::from(now == 16_000));
    }
}

#[test]
fn commit_dispatch_refuses_other_operation_or_other_held_scope() {
    use crate::authority_transaction::tests::{
        commit_held_fixture, commit_other_held_fixture, dispatch_commit_fixture_as,
    };
    let (prepared, before) = prepared_fixture();
    let arguments = prepared.request_bytes().to_vec();
    let mut driver = LocalCommitEffectDriver::new(
        SyntheticExecutor::default(),
        commit_other_held_fixture(),
        prepared,
        before,
        cancellation(),
        SequenceClock([Ok(15_000), Ok(15_001)].into()),
    )
    .unwrap();
    assert_ne!(
        dispatch_commit_fixture_as(
            &arguments,
            &mut driver,
            GrantOperation::GitCommit,
            "scope-first",
            "scope-attempt"
        )
        .outcome,
        OperationOutcome::Succeeded
    );
    assert_eq!(driver.take_error(), Some(LocalCommitError::ApprovalDenied));
    assert!(driver.take_receipt().is_none());
    assert_eq!(driver.into_executor().launches, 0);
    for operation in [GrantOperation::WorkspaceWrite, GrantOperation::GitCommit] {
        let (prepared, before) = prepared_fixture();
        let mut driver = LocalCommitEffectDriver::new(
            SyntheticExecutor::default(),
            commit_held_fixture(),
            prepared,
            before,
            cancellation(),
            SequenceClock([Ok(15_000), Ok(15_001)].into()),
        )
        .unwrap();
        let receipt = dispatch_commit_fixture_as(
            &arguments,
            &mut driver,
            operation,
            "operation-first",
            "operation-attempt",
        );
        let allowed = operation == GrantOperation::GitCommit;
        assert_eq!(receipt.outcome == OperationOutcome::Succeeded, allowed);
        assert_eq!(driver.take_receipt().is_some(), allowed);
        assert_eq!(
            driver.take_error(),
            if allowed {
                None
            } else {
                Some(LocalCommitError::ApprovalDenied)
            }
        );
        assert_eq!(driver.into_executor().launches, usize::from(allowed));
    }
}

#[test]
fn reconciliation_requires_retained_packet_bindings_findings_and_approval() {
    for field in ["integrity", "material", "binding", "approval", "expiry"] {
        let (mut prepared, before) = prepared_fixture();
        let result = SyntheticExecutor::default().execute(
            LocalCommitLaunchPermit {
                plan: &prepared.plan,
                before: &before,
            },
            &cancellation(),
        );
        match field {
            "integrity" => prepared.packet.objective.push_str(" unsealed"),
            "material" => {
                prepared.packet = packet(Some(FindingSeverity::Medium), 0, false);
                prepared
                    .plan
                    .review_packet_sha256
                    .clone_from(&prepared.packet.packet_sha256);
                reseal(&mut prepared.plan);
                prepared.approval =
                    record_manual_commit_approval(&prepared.plan, decision()).unwrap();
            }
            "binding" => {
                prepared.plan.review_packet_sha256 = "e".repeat(64);
                reseal(&mut prepared.plan);
                prepared.approval =
                    record_manual_commit_approval(&prepared.plan, decision()).unwrap();
            }
            "approval" => prepared.approval.receipt_sha256 = "e".repeat(64),
            "expiry" => (),
            _ => unreachable!(),
        }
        let now = if field == "expiry" { 20_000 } else { 15_000 };
        assert_eq!(
            reconcile_local_commit(
                &prepared,
                result,
                "authority-fixture",
                "attempt-fixture",
                now
            ),
            Err(if matches!(field, "approval" | "expiry") {
                LocalCommitError::ApprovalDenied
            } else {
                LocalCommitError::ReviewMismatch
            }),
            "{field}"
        );
    }
}

#[test]
fn commit_dispatch_never_emits_success_receipt_for_unproved_platform_result() {
    use crate::authority_transaction::tests::{commit_held_fixture, dispatch_commit_fixture};
    struct ResultExecutor {
        field: &'static str,
        launches: usize,
    }
    impl BoundedLocalCommitExecutor for ResultExecutor {
        fn execute(
            &mut self,
            permit: LocalCommitLaunchPermit<'_>,
            token: &CancellationToken,
        ) -> LocalCommitPlatformResult {
            self.launches += 1;
            let mut result = SyntheticExecutor::default().execute(permit, token);
            match self.field {
                "valid" => (),
                "failed" => result.outcome = OperationOutcome::Failed,
                "cancelled" => result.outcome = OperationOutcome::Cancelled,
                "before" => result.before.manifest_sha256 = "e".repeat(64),
                "after" => result.after.manifest_sha256 = "e".repeat(64),
                "preservation" => {
                    result.after.local_branches_sha256 = "e".repeat(64);
                    result.after = RepositoryPreservationManifest::seal(result.after).unwrap();
                }
                "object" => result.commit_object = None,
                "parent" => result.observed_parent = None,
                "tree" => result.observed_tree = None,
                "message" => result.observed_message_sha256 = None,
                "identity" => result.observed_identity_sha256 = None,
                "signer" => result.observed_signer_fingerprint = None,
                "signature" => result.signature_verified = false,
                "branch-before" => result.task_branch_before = None,
                "branch-after" => result.task_branch_after = None,
                "index" => result.user_index_after_sha256 = "e".repeat(64),
                "remotes" => result.remotes_after_sha256 = "e".repeat(64),
                "hooks" => result.hooks_executed = true,
                "filters" => result.filters_executed = true,
                "program" => result.repository_selected_program_used = true,
                "network" => result.network_used = true,
                "cleanup" => result.cleanup_verified = false,
                "code" => result.platform_code = "untrusted output with spaces".into(),
                _ => unreachable!(),
            }
            result
        }
    }
    for field in [
        "valid",
        "failed",
        "cancelled",
        "before",
        "after",
        "preservation",
        "object",
        "parent",
        "tree",
        "message",
        "identity",
        "signer",
        "signature",
        "branch-before",
        "branch-after",
        "index",
        "remotes",
        "hooks",
        "filters",
        "program",
        "network",
        "cleanup",
        "code",
    ] {
        let (prepared, before) = prepared_fixture();
        let arguments = prepared.request_bytes().to_vec();
        let mut driver = LocalCommitEffectDriver::new(
            ResultExecutor { field, launches: 0 },
            commit_held_fixture(),
            prepared,
            before,
            cancellation(),
            SequenceClock([Ok(15_000), Ok(15_001)].into()),
        )
        .unwrap();
        let authority = dispatch_commit_fixture(&arguments, &mut driver);
        let valid = field == "valid";
        assert_eq!(
            authority.outcome == OperationOutcome::Succeeded,
            valid,
            "{field}"
        );
        let receipt = driver.take_receipt();
        assert_eq!(receipt.is_some(), valid, "{field}");
        if let Some(receipt) = receipt {
            assert_eq!(
                receipt.authority_transaction_id,
                authority.authority_transaction_id.as_str()
            );
            assert_eq!(
                receipt.operation_attempt_id,
                authority.operation_attempt_id.as_str()
            );
        }
        assert!(driver.take_receipt().is_none());
        assert_eq!(
            driver.take_error(),
            if valid {
                None
            } else {
                Some(LocalCommitError::ResultDenied)
            },
            "{field}"
        );
        assert_eq!(driver.into_executor().launches, 1, "{field}");
    }
}
