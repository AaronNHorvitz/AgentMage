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
                now,
            )
            .unwrap_err(),
            LocalCommitError::ApprovalDenied
        );
    }
    assert!(
        LocalCommitEffectDriver::new((), (), prepared, before, cancellation(), 15_000,).is_ok()
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
            LocalCommitEffectDriver::new((), (), prepared, before, cancellation(), 15_000,)
                .unwrap_err(),
            LocalCommitError::ReviewMismatch,
            "{field}"
        );
    }
}
