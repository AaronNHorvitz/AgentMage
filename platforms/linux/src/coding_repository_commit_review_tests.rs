// SPDX-License-Identifier: Apache-2.0
//! Actual file edits/seals with explicitly synthetic Git and process observations.

use super::*;
use agentmage_kernel_engine::review_packet::{
    ChangePurposeAssignment, CommitChangePurpose, ReviewPacketInput, build_logical_commit_plan,
    build_review_packet, normalize_review_findings,
};
use std::cell::Cell;
use std::fs;
use std::os::unix::fs::PermissionsExt;

fn observation(owner: &LinuxCodingRepository) -> GitObservation {
    let identity = owner.created_git_identity().unwrap();
    let mut original = crate::coding_repository::tests::manifest();
    original.repository_sha256 = owner
        .created
        .as_ref()
        .unwrap()
        .original
        .root_path_sha256()
        .into();
    GitObservation {
        original: RepositoryPreservationManifest::seal(original).unwrap(),
        task_head: identity.base_object().into(),
        task_branch: Some(identity.branch_ref().into()),
        linked_identity: hash(b"synthetic linked metadata"),
        task_inventory_sha256: hash(b"synthetic task inventory"),
    }
}

fn packet(
    owner: &LinuxCodingRepository,
    changes: &agentmage_kernel_engine::write_approval::ShadowChangeSet,
) -> LocalReviewPacket {
    let change_set = owner
        .capture_applied_write_review(
            changes
                .operations()
                .iter()
                .map(|op| ChangePurposeAssignment {
                    operation_id: op.operation_id().into(),
                    purpose: CommitChangePurpose::Behavior,
                })
                .collect(),
        )
        .unwrap();
    let evidence = owner.completion_validation_evidence().unwrap();
    let last = evidence.attempts().last().unwrap();
    let validation = bind_validation_evidence(
        evidence.template(),
        &last.prepared,
        &last.command,
        &last.validation,
    )
    .unwrap();
    let observed = observation(owner);
    build_review_packet(ReviewPacketInput {
        packet_id: "synthetic-candidate-review".into(),
        repository_sha256: observed.original.repository_sha256,
        base_object: observed.task_head,
        candidate_snapshot_sha256: WorkspaceObservation::capture(
            &owner.created.as_ref().unwrap().held,
        )
        .unwrap()
        .digest,
        preservation_manifest_sha256: observed.original.manifest_sha256,
        objective: "Review the exact edited source".into(),
        behavior_delta: "Replace the selected fixture text".into(),
        commit_plan: build_logical_commit_plan(&change_set, vec![]).unwrap(),
        change_set,
        validations: vec![validation],
        checks_not_run: vec![],
        review_results: normalize_review_findings(
            vec![agentmage_kernel_engine::review_packet::ReviewMode::Correctness],
            vec![],
        )
        .unwrap(),
        evidence_artifacts: vec![],
        rollback: "A separately approved reverse edit is required".into(),
    })
    .unwrap()
}

// Resealing an inert packet models a caller with a legitimate typed packet
// builder. It must not circumvent the live-owner correlations.
fn reseal(p: LocalReviewPacket) -> LocalReviewPacket {
    build_review_packet(ReviewPacketInput {
        packet_id: p.packet_id,
        repository_sha256: p.repository_sha256,
        base_object: p.base_object,
        candidate_snapshot_sha256: p.candidate_snapshot_sha256,
        preservation_manifest_sha256: p.preservation_manifest_sha256,
        objective: p.objective,
        behavior_delta: p.behavior_delta,
        change_set: p.change_set,
        validations: p.validations,
        checks_not_run: p.checks_not_run,
        review_results: p.review_results,
        evidence_artifacts: p.evidence_artifacts,
        commit_plan: p.commit_plan,
        rollback: p.rollback,
    })
    .unwrap()
}

fn plan(
    owner: &LinuxCodingRepository,
    packet: &LocalReviewPacket,
    observe: impl FnMut() -> Result<GitObservation>,
) -> Result<CandidateTreePlan> {
    owner.plan_reviewed_candidate_tree_with(
        packet,
        &packet.commit_plan.groups[0].group_id,
        "review-build",
        observe,
    )
}

#[test]
fn candidate_review_correlates_live_edit_sealed_validation_and_both_git_observations() {
    let (f, owner, changes) = command::tests::candidate_review_fixture();
    fs::create_dir(f.root.join("repo/.git/unused")).unwrap();
    let packet = packet(&owner, &changes);
    let calls = Cell::new(0);
    let result = plan(&owner, &packet, || {
        calls.set(calls.get() + 1);
        Ok(observation(&owner))
    })
    .unwrap();
    assert_eq!(calls.get(), 2);
    assert_eq!(result.review_packet_sha256, packet.packet_sha256);
    assert_eq!(
        result.parent_object,
        owner.created_git_identity().unwrap().base_object()
    );
    assert_eq!(
        result.preservation_manifest_sha256,
        packet.preservation_manifest_sha256
    );
    assert_eq!(
        result.user_index_sha256,
        observation(&owner).original.index_sha256
    );
    assert_eq!(result.files.len(), 1);
    assert_eq!(result.files[0].path, "sentinel");
    assert_eq!(result.files[0].postimage_sha256, hash(b"reviewed change"));
    assert_eq!(result.files[0].mode, CommitFileMode::Regular);
    assert!(
        !result.network_authority
            && !result.hooks_enabled
            && !result.filters_enabled
            && !result.repository_program_selection_enabled
    );
    agentmage_kernel_engine::local_commit::verify_candidate_tree_plan(&result).unwrap();
    assert_eq!(
        fs::read(f.root.join("repo/sentinel")).unwrap(),
        b"preserve human work"
    );
    assert!(!f.root.join("owned/candidate-index-review-build").exists());
    assert_eq!(owner.created.as_ref().unwrap().edits.len(), 1);
}

#[test]
fn candidate_review_native_entry_retains_exact_artifact_refusal_without_git_effects() {
    let (f, owner, changes) = command::tests::candidate_review_fixture();
    let packet = packet(&owner, &changes);
    let result = owner.plan_reviewed_candidate_tree(
        &packet,
        &packet.commit_plan.groups[0].group_id,
        "native-review-build",
    );
    if let Err(refusal) = LinuxGitArtifact::verify("/usr/bin/git") {
        assert_eq!(result.unwrap_err().code(), refusal.kind().code());
        println!("native candidate planning not measured: exact system Git trust refusal retained");
    } else {
        // Safe system Git alone cannot make this synthetic metadata directory a
        // real repository. It must still refuse; this is not native admission.
        assert!(result.is_err());
        println!("native candidate planning not measured: synthetic repository metadata refused");
    }
    assert!(
        !f.root
            .join("owned/candidate-index-native-review-build")
            .exists()
    );
    assert_eq!(
        fs::read(f.root.join("repo/sentinel")).unwrap(),
        b"preserve human work"
    );
    assert_eq!(
        fs::read(f.root.join("owned/worktree/sentinel")).unwrap(),
        b"reviewed change"
    );
}

#[test]
fn candidate_review_derives_git_modes_from_observed_owner_executability() {
    for (permissions, expected) in [
        (0o600, CommitFileMode::Regular),
        (0o700, CommitFileMode::Executable),
        (0o755, CommitFileMode::Executable),
    ] {
        let (_f, owner, changes) = command::tests::candidate_review_fixture_with_mode(permissions);
        let packet = packet(&owner, &changes);
        let result = plan(&owner, &packet, || Ok(observation(&owner))).unwrap();
        assert_eq!(result.files[0].mode, expected);
    }
}

#[test]
fn candidate_review_refuses_resealed_packet_substitutions_before_git_reads() {
    let (_f, owner, changes) = command::tests::candidate_review_fixture();
    let original = packet(&owner, &changes);
    for which in ["repository", "base", "snapshot", "validation", "integrity"] {
        let mut p = original.clone();
        match which {
            "repository" => p.repository_sha256 = hash(b"other repository"),
            "base" => p.base_object = "2".repeat(40),
            "snapshot" => p.candidate_snapshot_sha256 = hash(b"other snapshot"),
            "validation" => p.validations.clear(),
            "integrity" => p.objective.push_str(" changed"),
            _ => unreachable!(),
        }
        if which != "integrity" {
            p = reseal(p);
        }
        assert!(
            plan(&owner, &p, || panic!("Git must not run for {which}")).is_err(),
            "{which}"
        );
    }
}

#[test]
fn candidate_review_blocks_visible_medium_high_critical_findings_without_inventing_review_authority()
 {
    use agentmage_kernel_engine::review_packet::{ReviewFindingInput, ReviewMode};
    let (_f, owner, changes) = command::tests::candidate_review_fixture();
    let original = packet(&owner, &changes);
    for severity in [
        FindingSeverity::Low,
        FindingSeverity::Medium,
        FindingSeverity::High,
        FindingSeverity::Critical,
    ] {
        let mut p = original.clone();
        p.review_results = normalize_review_findings(
            vec![ReviewMode::Correctness],
            vec![ReviewFindingInput {
                finding_id: "fixture-finding".into(),
                mode: ReviewMode::Correctness,
                code: "fixture.review".into(),
                path: "sentinel".into(),
                line: 1,
                severity,
                confidence_bps: 9000,
                evidence_sha256: hash(b"synthetic finding evidence"),
                rationale_code: "fixture.rationale".into(),
            }],
        )
        .unwrap();
        let p = reseal(p);
        let calls = Cell::new(0);
        let result = plan(&owner, &p, || {
            calls.set(calls.get() + 1);
            Ok(observation(&owner))
        });
        if severity == FindingSeverity::Low {
            assert!(result.is_ok());
            assert_eq!(calls.get(), 2);
        } else {
            assert_eq!(result.unwrap_err().code(), "code.commit.review-packet");
            assert_eq!(calls.get(), 0);
        }
        // Rejection leaves every retained record intact and does not approve a
        // different packet or a subsequent operation.
        assert_eq!(owner.created.as_ref().unwrap().edits.len(), 1);
        owner.check_completion_validation().unwrap();
    }
}

#[test]
fn candidate_review_refuses_git_ref_manifest_hazards_and_midreview_drift() {
    let (_f, owner, changes) = command::tests::candidate_review_fixture();
    let packet = packet(&owner, &changes);
    for which in [
        "branch",
        "detached",
        "head",
        "repository",
        "preservation",
        "invalid-manifest",
        "shallow",
        "partial",
        "hazard",
    ] {
        let result = plan(&owner, &packet, || {
            let mut o = observation(&owner);
            match which {
                "branch" => o.task_branch = Some("refs/heads/main".into()),
                "detached" => o.task_branch = None,
                "head" => o.task_head = "2".repeat(40),
                "repository" => o.original.repository_sha256 = hash(b"other repository"),
                "preservation" => o.original.index_sha256 = hash(b"other index"),
                "invalid-manifest" => o.original.manifest_sha256 = hash(b"invalid manifest"),
                "shallow" => o.original.shallow = true,
                "partial" => o.original.partial = true,
                "hazard" => o.original.hazardous_configuration = true,
                _ => unreachable!(),
            }
            if which != "invalid-manifest" {
                o.original = RepositoryPreservationManifest::seal(o.original).unwrap();
            }
            Ok(o)
        });
        assert_eq!(
            result.unwrap_err().code(),
            "code.commit.git-state",
            "{which}"
        );
    }
    for which in [
        "branch",
        "head",
        "index",
        "linked",
        "inventory",
        "read-error",
    ] {
        let calls = Cell::new(0);
        let result = plan(&owner, &packet, || {
            calls.set(calls.get() + 1);
            let mut o = observation(&owner);
            if calls.get() == 2 {
                match which {
                    "branch" => o.task_branch = None,
                    "head" => o.task_head = "2".repeat(40),
                    "index" => {
                        o.original.index_sha256 = hash(b"later index");
                        o.original = RepositoryPreservationManifest::seal(o.original).unwrap();
                    }
                    "linked" => o.linked_identity = hash(b"different linked metadata"),
                    "inventory" => {
                        o.task_inventory_sha256 = hash(b"different task index/inventory")
                    }
                    "read-error" => return Err(err("synthetic.read-failure")),
                    _ => unreachable!(),
                }
            }
            Ok(o)
        });
        assert_eq!(calls.get(), 2);
        assert_eq!(
            result.unwrap_err().code(),
            if which == "read-error" {
                "synthetic.read-failure"
            } else {
                "code.commit.git-drift"
            }
        );
    }
}

#[test]
fn candidate_review_rechecks_sources_after_git_and_keeps_native_authority_absent() {
    let (f, owner, changes) = command::tests::candidate_review_fixture();
    let packet = packet(&owner, &changes);
    let result = plan(&owner, &packet, || {
        fs::write(
            f.root.join("owned/worktree/sentinel"),
            b"unreviewed later change",
        )
        .unwrap();
        Ok(observation(&owner))
    });
    assert!(result.is_err());
    assert_eq!(
        fs::read(f.root.join("repo/sentinel")).unwrap(),
        b"preserve human work"
    );
    assert!(!f.root.join("owned/candidate-index-review-build").exists());
}

#[test]
fn candidate_review_refuses_changed_held_roots_and_invalid_group_or_index_choices() {
    for root in ["repo", "owned", "owned/worktree"] {
        let (f, owner, changes) = command::tests::candidate_review_fixture();
        let packet = packet(&owner, &changes);
        let calls = Cell::new(0);
        let result = plan(&owner, &packet, || {
            let observed = observation(&owner);
            calls.set(calls.get() + 1);
            if calls.get() == 2 {
                fs::rename(f.root.join(root), f.root.join("displaced-root")).unwrap();
                fs::create_dir(f.root.join(root)).unwrap();
                fs::set_permissions(f.root.join(root), fs::Permissions::from_mode(0o700)).unwrap();
            }
            Ok(observed)
        });
        assert!(result.is_err(), "{root}");
        assert_eq!(calls.get(), 2);
    }
    let (f, owner, changes) = command::tests::candidate_review_fixture();
    let packet = packet(&owner, &changes);
    let group = &packet.commit_plan.groups[0].group_id;
    for (group, build) in [
        ("unknown-group", "review-build"),
        (group.as_str(), "../escape"),
        (group.as_str(), ""),
    ] {
        let result = owner
            .plan_reviewed_candidate_tree_with(&packet, group, build, || Ok(observation(&owner)));
        assert!(result.is_err());
    }
    assert!(!f.root.join("owned/candidate-index-review-build").exists());
    assert!(!f.root.join("escape").exists());
}

#[test]
fn candidate_review_rejects_missing_owner_lineage_newfiles_and_unsafe_modes() {
    for which in [
        "identity",
        "lineage",
        "preimage",
        "postimage",
        "creation",
        "mode",
        "pending",
    ] {
        let (f, mut owner, changes) = command::tests::candidate_review_fixture();
        let packet = packet(&owner, &changes);
        let created = owner.created.as_mut().unwrap();
        match which {
            "identity" => created.git_creation = None,
            "lineage" => created.edits.clear(),
            "preimage" => {
                created.edits.get_mut("sentinel").unwrap().before_sha256 = hash(b"foreign preimage")
            }
            "postimage" => {
                created.edits.get_mut("sentinel").unwrap().after_sha256 = hash(b"foreign postimage")
            }
            "creation" => created.edits.get_mut("sentinel").unwrap().created_mode = Some(0o644),
            "mode" => fs::set_permissions(
                f.root.join("owned/worktree/sentinel"),
                fs::Permissions::from_mode(0o600),
            )
            .unwrap(),
            "pending" => {
                let scope = owner.select_created_sources(&["sentinel".into()]).unwrap();
                owner
                    .prepare_edit(
                        scope,
                        &[crate::LinuxCodingReplacement {
                            path: "sentinel",
                            before_sha256: &hash(b"reviewed change"),
                            content: "pending",
                        }],
                        "pending fixture",
                        1000,
                    )
                    .unwrap();
            }
            _ => unreachable!(),
        }
        assert!(
            plan(&owner, &packet, || panic!("Git must not run for {which}")).is_err(),
            "{which}"
        );
    }
}

fn purposes() -> Vec<ChangePurposeAssignment> {
    vec![ChangePurposeAssignment {
        operation_id: "edit-0".into(),
        purpose: CommitChangePurpose::Behavior,
    }]
}

#[test]
fn applied_review_uses_actual_native_receipt_chain_and_rejects_resealed_caller_hashes() {
    use agentmage_kernel_engine::review_packet::capture_reviewable_change_set;
    let (f, owner, changes) = command::tests::candidate_review_fixture();
    let original = packet(&owner, &changes);
    let projection = serde_json::to_value(&original.change_set).unwrap();
    let result = owner.inspect_edit().unwrap();
    let receipts = result["latest"]["write_result"]["receipts"]
        .as_array()
        .unwrap();
    assert_eq!(
        projection["write_application_receipt_sha256"],
        receipts.last().unwrap()["receipt_sha256"]
    );
    assert_eq!(receipts.last().unwrap()["status"], "verified");
    assert_eq!(projection["change_set_sha256"], changes.change_set_sha256());
    for which in ["approval", "application", "both"] {
        let mut p = original.clone();
        p.change_set = capture_reviewable_change_set(
            &changes,
            purposes(),
            if which == "application" {
                projection["write_approval_receipt_sha256"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            } else {
                hash(b"caller approval")
            },
            if which == "approval" {
                projection["write_application_receipt_sha256"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            } else {
                hash(b"caller application")
            },
        )
        .unwrap();
        p.commit_plan = build_logical_commit_plan(&p.change_set, vec![]).unwrap();
        let p = reseal(p);
        assert!(verify_review_packet(&p));
        assert_eq!(
            plan(&owner, &p, || panic!("Git must not run for {which}"))
                .unwrap_err()
                .code(),
            "code.commit.write-provenance"
        );
    }
    assert_eq!(
        fs::read(f.root.join("repo/sentinel")).unwrap(),
        b"preserve human work"
    );
    assert!(!f.root.join("owned/candidate-index-review-build").exists());
}

#[test]
fn applied_review_requires_same_owner_not_copied_records_or_missing_proof() {
    let (f, mut owner, changes) = command::tests::candidate_review_fixture();
    let packet = packet(&owner, &changes);
    owner.applied_write = None;
    assert_eq!(
        plan(&owner, &packet, || panic!("no Git without proof"))
            .unwrap_err()
            .code(),
        "code.commit.write-provenance"
    );
    let result = owner.edit_result.clone();
    let created = owner.created.as_ref().unwrap();
    let edits = created.edits.clone();
    let creation_receipt_sha256 = created.creation_receipt_sha256.clone();
    // Reopening is tested after the original authority-store owner is dropped.
    // Its exclusive live lock must never be bypassed by a fixture.
    drop(owner);
    let mut reopened = f.open();
    reopened.created = Some(CreatedWorktree {
        original_path: f.root.join("repo"),
        original: held_root(&f.root.join("repo")).unwrap(),
        path: f.root.join("owned/worktree"),
        held: held_root(&f.root.join("owned/worktree")).unwrap(),
        owned: held_root(&f.root.join("owned")).unwrap(),
        creation_receipt_sha256,
        git_creation: None,
        edits,
    });
    reopened.edit_result = result;
    assert_eq!(
        reopened
            .capture_applied_write_review(purposes())
            .unwrap_err()
            .code(),
        "code.commit.write-provenance"
    );
}

#[test]
fn applied_review_refuses_later_transaction_without_complete_original_preimage() {
    let (f, mut owner, _) = command::tests::candidate_review_fixture();
    let scope = owner.select_created_sources(&["sentinel".into()]).unwrap();
    let p = owner
        .prepare_edit(
            scope,
            &[crate::LinuxCodingReplacement {
                path: "sentinel",
                before_sha256: &hash(b"reviewed change"),
                content: "second change",
            }],
            "second exact edit",
            1000,
        )
        .unwrap();
    assert_eq!(
        owner
            .decide_edit(p["preview_sha256"].as_str().unwrap(), true, 1000)
            .unwrap()["outcome"],
        "committed"
    );
    assert!(owner.applied_write.is_some());
    assert_eq!(
        owner
            .capture_applied_write_review(purposes())
            .unwrap_err()
            .code(),
        "code.commit.write-provenance"
    );
    assert_eq!(
        fs::read(f.root.join("owned/worktree/sentinel")).unwrap(),
        b"second change"
    );
    assert_eq!(
        fs::read(f.root.join("repo/sentinel")).unwrap(),
        b"preserve human work"
    );
}

#[test]
fn applied_review_denial_preserves_proof_but_failed_native_attempt_retires_it() {
    let (f, mut owner, _) = command::tests::candidate_review_fixture();
    let original = owner.capture_applied_write_review(purposes()).unwrap();
    for approve in [false, true] {
        let scope = owner.select_created_sources(&["sentinel".into()]).unwrap();
        let p = owner
            .prepare_edit(
                scope,
                &[crate::LinuxCodingReplacement {
                    path: "sentinel",
                    before_sha256: &hash(b"reviewed change"),
                    content: "never approved before drift",
                }],
                "exact edit",
                1000,
            )
            .unwrap();
        let digest = p["preview_sha256"].as_str().unwrap();
        if approve {
            fs::write(f.root.join("owned/worktree/sentinel"), b"external change").unwrap();
            assert!(owner.decide_edit(digest, true, 1000).is_err());
            assert!(owner.applied_write.is_none());
            fs::write(f.root.join("owned/worktree/sentinel"), b"reviewed change").unwrap();
            assert!(owner.capture_applied_write_review(purposes()).is_err());
        } else {
            assert_eq!(
                owner.decide_edit(digest, false, 1000).unwrap()["outcome"],
                "denied"
            );
            assert_eq!(
                owner.capture_applied_write_review(purposes()).unwrap(),
                original
            );
        }
    }
    assert_eq!(
        fs::read(f.root.join("repo/sentinel")).unwrap(),
        b"preserve human work"
    );
}

#[test]
fn applied_review_rechecks_current_bytes_and_all_held_roots() {
    for which in ["source", "original", "worktree", "owned", "scope"] {
        let (f, mut owner, _) = command::tests::candidate_review_fixture();
        match which {
            "source" => {
                fs::write(f.root.join("owned/worktree/sentinel"), b"external bytes").unwrap()
            }
            "scope" => {
                let (_other_fixture, mut other, _) = command::tests::candidate_review_fixture();
                owner.applied_write = other.applied_write.take();
            }
            other => {
                let path = match other {
                    "original" => "repo",
                    "worktree" => "owned/worktree",
                    _ => "owned",
                };
                fs::rename(f.root.join(path), f.root.join("saved-root")).unwrap();
                fs::create_dir(f.root.join(path)).unwrap();
            }
        }
        assert!(
            owner.capture_applied_write_review(purposes()).is_err(),
            "{which}"
        );
    }
}
