// SPDX-License-Identifier: Apache-2.0
//! Actual file edits/seals with explicitly synthetic Git and process observations.

use super::*;
use agentmage_kernel_engine::review_packet::{
    ChangePurposeAssignment, CommitChangePurpose, FindingSeverity, ReviewPacketInput,
    build_logical_commit_plan, build_review_packet, normalize_review_findings,
    verify_review_packet,
};
use std::cell::Cell;
use std::fs;
use std::os::unix::fs::PermissionsExt;

fn base_file(sha256: String, mode: CommitFileMode) -> Option<BaseFilePreimage> {
    Some(BaseFilePreimage {
        sha256,
        mode,
        bytes: Vec::new(),
    })
}

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
        base_preimages: owner
            .created
            .as_ref()
            .unwrap()
            .edits
            .iter()
            .map(|(path, edit)| {
                (
                    path.clone(),
                    (edit.before_sha256 != "0".repeat(64)).then(|| BaseFilePreimage {
                        sha256: edit.before_sha256.clone(),
                        mode: CommitFileMode::Regular,
                        bytes: fs::read(owner.created.as_ref().unwrap().original_path.join(path))
                            .unwrap(),
                    }),
                )
            })
            .collect(),
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
    packet_from_projection(owner, change_set)
}

fn packet_from_projection(
    owner: &LinuxCodingRepository,
    change_set: ReviewableChangeSet,
) -> LocalReviewPacket {
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
    let status = owner.inspect_apply().unwrap();
    let review = owner
        .consented_commit_review(
            status["repository_root_sha256"].as_str().unwrap(),
            status["worktree_root_sha256"].as_str().unwrap(),
        )
        .unwrap();
    let result = review.plan_candidate_tree(
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
fn candidate_review_scope_matches_both_selections_before_inspecting_unavailable_roots() {
    for root in ["repo", "owned/worktree"] {
        let (f, owner, changes) = command::tests::candidate_review_fixture();
        let status = owner.inspect_apply().unwrap();
        let original = status["repository_root_sha256"].as_str().unwrap();
        let tree = status["worktree_root_sha256"].as_str().unwrap();
        let receipts = owner.inspect().unwrap()["receipts"].clone();
        assert!(owner.check_completion_validation().is_ok());
        let validation_history = owner.inspect_validation().unwrap()["history"].clone();
        let expected = packet(&owner, &changes).change_set;
        fs::rename(f.root.join(root), f.root.join("displaced-root")).unwrap();
        for (selected_original, selected_tree) in [
            ("", tree),
            (original, ""),
            ("foreign", tree),
            (original, "foreign"),
            (tree, original),
        ] {
            assert_eq!(
                owner
                    .consented_commit_review(selected_original, selected_tree)
                    .err()
                    .unwrap()
                    .code(),
                "code.commit.read-scope",
                "{root}: {selected_original} / {selected_tree}"
            );
            assert_eq!(owner.inspect().unwrap()["receipts"], receipts);
            assert_eq!(owner.inspect_apply().unwrap(), status);
        }
        // Equal path digests do not adopt an unavailable root either.
        assert!(owner.consented_commit_review(original, tree).is_err());
        fs::rename(f.root.join("displaced-root"), f.root.join(root)).unwrap();
        let review = owner.consented_commit_review(original, tree).unwrap();
        assert_eq!(
            review.capture_applied_write_review(purposes()).unwrap(),
            expected
        );
        // The validation seal observes the worktree, not the original
        // checkout. Moving the worktree changes that observation; restoring
        // its directory permits historical projection but cannot revive the
        // old completion-validation binding. Original-only moves leave that
        // separate worktree observation intact.
        if root == "owned/worktree" {
            assert_eq!(
                owner.check_completion_validation().unwrap_err().code(),
                "code.completion.source-binding"
            );
        } else {
            assert!(owner.check_completion_validation().is_ok());
        }
        assert_eq!(
            owner.inspect_validation().unwrap()["history"],
            validation_history
        );
        assert_eq!(owner.inspect().unwrap()["receipts"], receipts);
        assert_eq!(owner.inspect_apply().unwrap(), status);
    }
}

#[test]
fn candidate_review_scope_rechecks_replaced_roots_on_each_borrowed_use() {
    for root in ["repo", "owned", "owned/worktree"] {
        let (f, owner, changes) = command::tests::candidate_review_fixture();
        let packet = packet(&owner, &changes);
        let status = owner.inspect_apply().unwrap();
        let receipts = owner.inspect().unwrap()["receipts"].clone();
        let review = owner
            .consented_commit_review(
                status["repository_root_sha256"].as_str().unwrap(),
                status["worktree_root_sha256"].as_str().unwrap(),
            )
            .unwrap();
        assert_eq!(
            review.capture_applied_write_review(purposes()).unwrap(),
            packet.change_set
        );
        fs::rename(f.root.join(root), f.root.join("displaced-root")).unwrap();
        fs::create_dir(f.root.join(root)).unwrap();
        fs::set_permissions(f.root.join(root), fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(f.root.join(root).join("retain"), b"unrelated replacement").unwrap();
        let expected_error = if root == "repo" {
            "code.apply.scope"
        } else {
            "code.command.scope"
        };
        assert_eq!(
            review
                .capture_applied_write_review(purposes())
                .unwrap_err()
                .code(),
            expected_error
        );
        assert_eq!(
            review
                .plan_candidate_tree(&packet, &packet.commit_plan.groups[0].group_id, "scoped")
                .unwrap_err()
                .code(),
            expected_error
        );
        // Replacement data and the displaced original remain; neither a
        // matching path nor an old view authorizes adoption or cleanup.
        assert_eq!(
            fs::read(f.root.join(root).join("retain")).unwrap(),
            b"unrelated replacement"
        );
        assert!(f.root.join("displaced-root").is_dir());
        assert_eq!(owner.inspect().unwrap()["receipts"], receipts);
        assert_eq!(owner.inspect_apply().unwrap(), status);
    }
}

#[test]
fn candidate_review_scope_cannot_adopt_another_owner_or_restore_missing_creation() {
    let (f, mut owner, _) = command::tests::candidate_review_fixture();
    let (_other, mut other_owner, _) = command::tests::candidate_review_fixture();
    let status = owner.inspect_apply().unwrap();
    let other_status = other_owner.inspect_apply().unwrap();
    assert!(
        other_owner
            .consented_commit_review(
                status["repository_root_sha256"].as_str().unwrap(),
                status["worktree_root_sha256"].as_str().unwrap(),
            )
            .is_err()
    );
    // Synthetic test-only substitution stands for a later separately approved
    // worktree creation. The public API cannot import a CreatedWorktree.
    owner.created = other_owner.created.take();
    assert_eq!(
        owner
            .consented_commit_review(
                status["repository_root_sha256"].as_str().unwrap(),
                status["worktree_root_sha256"].as_str().unwrap(),
            )
            .err()
            .unwrap()
            .code(),
        "code.commit.read-scope"
    );
    let fresh = owner
        .consented_commit_review(
            other_status["repository_root_sha256"].as_str().unwrap(),
            other_status["worktree_root_sha256"].as_str().unwrap(),
        )
        .unwrap();
    assert_eq!(
        fresh
            .capture_applied_write_review(purposes())
            .unwrap_err()
            .code(),
        "code.commit.write-provenance"
    );
    owner.created = None;
    assert_eq!(
        owner.consented_commit_review("", "").err().unwrap().code(),
        "code.command.worktree-required"
    );
    assert_eq!(
        fs::read(f.root.join("repo/sentinel")).unwrap(),
        b"preserve human work"
    );
}

#[test]
fn candidate_review_refuses_dirty_untracked_missing_and_extra_base_preimages() {
    let (f, owner, changes) = command::tests::candidate_review_fixture();
    let packet = packet(&owner, &changes);
    for variant in ["dirty", "untracked", "empty", "missing", "extra"] {
        let calls = Cell::new(0);
        let result = plan(&owner, &packet, || {
            calls.set(calls.get() + 1);
            let mut observed = observation(&owner);
            match variant {
                "dirty" => {
                    observed.base_preimages.insert(
                        "sentinel".into(),
                        base_file(hash(b"different immutable base"), CommitFileMode::Regular),
                    );
                }
                "untracked" => {
                    observed.base_preimages.insert("sentinel".into(), None);
                }
                "empty" => {
                    observed.base_preimages.insert(
                        "sentinel".into(),
                        base_file(hash(b""), CommitFileMode::Regular),
                    );
                }
                "missing" => {
                    observed.base_preimages.clear();
                }
                "extra" => {
                    observed.base_preimages.insert("other.py".into(), None);
                }
                _ => unreachable!(),
            }
            Ok(observed)
        });
        assert_eq!(
            result.unwrap_err().code(),
            "code.commit.base-preimage",
            "{variant}"
        );
        assert_eq!(calls.get(), 1);
    }
    assert_eq!(
        fs::read(f.root.join("repo/sentinel")).unwrap(),
        b"preserve human work"
    );
    assert!(!f.root.join("owned/candidate-index-review-build").exists());
}

#[test]
fn candidate_review_refuses_changed_immutable_base_observation_before_releasing_plan() {
    let (f, owner, changes) = command::tests::candidate_review_fixture();
    let packet = packet(&owner, &changes);
    let calls = Cell::new(0);
    let result = plan(&owner, &packet, || {
        calls.set(calls.get() + 1);
        let mut observed = observation(&owner);
        if calls.get() == 2 {
            observed.base_preimages.insert("sentinel".into(), None);
        }
        Ok(observed)
    });
    assert_eq!(result.unwrap_err().code(), "code.commit.git-drift");
    assert_eq!(calls.get(), 2);
    assert!(!f.root.join("owned/candidate-index-review-build").exists());
}

#[test]
fn candidate_review_derives_git_modes_from_observed_owner_executability() {
    for (permissions, expected) in [
        (0o600, CommitFileMode::Regular),
        (0o644, CommitFileMode::Regular),
        (0o610, CommitFileMode::Regular),
        (0o700, CommitFileMode::Executable),
        (0o755, CommitFileMode::Executable),
    ] {
        let (_f, owner, changes) = command::tests::candidate_review_fixture_with_mode(permissions);
        let packet = packet(&owner, &changes);
        let result = plan(&owner, &packet, || {
            let mut observed = observation(&owner);
            observed
                .base_preimages
                .get_mut("sentinel")
                .unwrap()
                .as_mut()
                .unwrap()
                .mode = expected;
            Ok(observed)
        })
        .unwrap();
        assert_eq!(result.files[0].mode, expected);
    }
}

#[test]
fn candidate_review_refuses_existing_mode_changes_even_with_approved_bytes_and_sealed_tests() {
    for (current, base) in [
        (0o700, CommitFileMode::Regular),
        (0o600, CommitFileMode::Executable),
    ] {
        // The write and validation are real/native and synthetic respectively,
        // as in the other owner fixtures. Only immutable Git observations are
        // supplied by this private seam. Correct bytes cannot authorize chmod.
        let (f, owner, changes) = command::tests::candidate_review_fixture_with_mode(current);
        let packet = packet(&owner, &changes);
        let calls = Cell::new(0);
        let result = plan(&owner, &reseal(packet), || {
            calls.set(calls.get() + 1);
            let mut observed = observation(&owner);
            observed
                .base_preimages
                .get_mut("sentinel")
                .unwrap()
                .as_mut()
                .unwrap()
                .mode = base;
            Ok(observed)
        });
        assert_eq!(result.unwrap_err().code(), "code.commit.base-preimage");
        assert_eq!(calls.get(), 1);
        owner.check_completion_validation().unwrap();
        assert_eq!(
            fs::read(f.root.join("repo/sentinel")).unwrap(),
            b"preserve human work"
        );
        assert_eq!(
            fs::read(f.root.join("owned/worktree/sentinel")).unwrap(),
            b"reviewed change"
        );
        assert!(!f.root.join("owned/candidate-index-review-build").exists());
    }
}

#[test]
fn candidate_review_rechecks_base_and_source_modes_before_releasing_plan() {
    for changed in ["base", "source"] {
        let (f, owner, changes) = command::tests::candidate_review_fixture();
        let packet = packet(&owner, &changes);
        let calls = Cell::new(0);
        let result = plan(&owner, &packet, || {
            calls.set(calls.get() + 1);
            let mut observed = observation(&owner);
            if calls.get() == 2 {
                if changed == "base" {
                    observed
                        .base_preimages
                        .get_mut("sentinel")
                        .unwrap()
                        .as_mut()
                        .unwrap()
                        .mode = CommitFileMode::Executable;
                } else {
                    fs::set_permissions(
                        f.root.join("owned/worktree/sentinel"),
                        fs::Permissions::from_mode(0o700),
                    )
                    .unwrap();
                }
            }
            Ok(observed)
        });
        assert_eq!(calls.get(), 2);
        assert_eq!(
            result.unwrap_err().code(),
            if changed == "base" {
                "code.commit.git-drift"
            } else {
                "code.completion.source-binding"
            }
        );
        assert!(!f.root.join("owned/candidate-index-review-build").exists());
        assert_eq!(
            fs::read(f.root.join("repo/sentinel")).unwrap(),
            b"preserve human work"
        );
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
fn candidate_review_blocks_suppressed_material_findings_before_any_source_or_git_read() {
    use agentmage_kernel_engine::review_packet::{ReviewFindingInput, ReviewMode};
    let (f, owner, changes) = command::tests::candidate_review_fixture();
    let original = packet(&owner, &changes);
    let mut validation = owner.inspect_validation().unwrap();
    let records = owner.inspect().unwrap()["receipts"].clone();
    assert!(plan(&owner, &original, || Ok(observation(&owner))).is_ok());
    let selected = owner.inspect_apply().unwrap();
    let review = owner
        .consented_commit_review(
            selected["repository_root_sha256"].as_str().unwrap(),
            selected["worktree_root_sha256"].as_str().unwrap(),
        )
        .unwrap();
    // Even an unavailable source must not conceal the review refusal behind
    // a filesystem error. The entire normalized result is checked first.
    fs::rename(f.root.join("owned/worktree"), f.root.join("displaced")).unwrap();
    for severity in [
        FindingSeverity::Medium,
        FindingSeverity::High,
        FindingSeverity::Critical,
    ] {
        for confidence_bps in [0, 6999] {
            for duplicate in [false, true] {
                let mut inputs = vec![ReviewFindingInput {
                    finding_id: "original-material-finding".into(),
                    mode: ReviewMode::Correctness,
                    code: "fixture.review".into(),
                    path: "sentinel".into(),
                    line: 1,
                    severity,
                    confidence_bps,
                    evidence_sha256: hash(b"synthetic finding evidence"),
                    rationale_code: "fixture.rationale".into(),
                }];
                if duplicate {
                    let mut second = inputs[0].clone();
                    second.finding_id = "duplicate-material-finding".into();
                    inputs.push(second);
                }
                let mut p = original.clone();
                p.review_results =
                    normalize_review_findings(vec![ReviewMode::Correctness], inputs).unwrap();
                assert!(p.review_results.findings.is_empty());
                assert_eq!(
                    p.review_results.suppressed.len(),
                    if duplicate { 2 } else { 1 }
                );
                let p = reseal(p);
                assert!(verify_review_packet(&p));
                assert_eq!(p.blocking_finding_count, 0); // Existing display count, not admission.
                assert_eq!(
                    review
                        .plan_candidate_tree(&p, &p.commit_plan.groups[0].group_id, "review-build")
                        .unwrap_err()
                        .code(),
                    "code.commit.review-packet"
                );
                assert_eq!(
                    plan(&owner, &p, || panic!("Git must not run"))
                        .unwrap_err()
                        .code(),
                    "code.commit.review-packet",
                    "{severity:?} / {confidence_bps} / duplicate={duplicate}"
                );
            }
        }
    }
    fs::rename(f.root.join("displaced"), f.root.join("owned/worktree")).unwrap();
    assert_eq!(owner.inspect().unwrap()["receipts"], records);
    // Renaming changes the root metadata in the sealed source observation.
    // History is preserved; that external drift must not regain readiness.
    validation["workspace_binding"]["matches_current_observation"] = json!(false);
    assert_eq!(owner.inspect_validation().unwrap(), validation);
    assert_eq!(
        owner.check_completion_validation().unwrap_err().code(),
        "code.completion.source-binding"
    );
    assert_eq!(
        review
            .capture_applied_write_review(
                original
                    .change_set
                    .files()
                    .iter()
                    .map(|f| ChangePurposeAssignment {
                        operation_id: f.operation_id.clone(),
                        purpose: f.purpose
                    })
                    .collect()
            )
            .unwrap(),
        original.change_set
    );
    assert_eq!(
        fs::read(f.root.join("repo/sentinel")).unwrap(),
        b"preserve human work"
    );
    assert!(!f.root.join("owned/candidate-index-review-build").exists());
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
fn applied_review_refuses_later_transaction_without_retained_original_proof() {
    let (f, mut owner, _) = command::tests::candidate_review_fixture();
    // Losing the original process-only proof cannot be repaired by the next
    // transaction, even when the aggregate edit lineage remains in memory.
    owner.applied_write = None;
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

#[test]
fn write_history_candidate_plan_uses_complete_original_chain_and_fresh_validation() {
    let (f, owner, _) =
        command::tests::candidate_review_fixture_before_validation(0o644, |owner| {
            assert!(owner.check_completion_validation().is_err());
            let scope = owner.select_created_sources(&["sentinel".into()]).unwrap();
            let preview = owner
                .prepare_edit(
                    scope,
                    &[crate::LinuxCodingReplacement {
                        path: "sentinel",
                        before_sha256: &hash(b"reviewed change"),
                        content: "reviewed retry",
                    }],
                    "second approved native edit",
                    1000,
                )
                .unwrap();
            assert_eq!(
                owner
                    .decide_edit(preview["preview_sha256"].as_str().unwrap(), true, 1000)
                    .unwrap()["outcome"],
                "committed"
            );
            assert!(owner.check_completion_validation().is_err());
        });
    let projection = owner
        .capture_applied_write_review(vec![ChangePurposeAssignment {
            operation_id: "history-0".into(),
            purpose: CommitChangePurpose::Behavior,
        }])
        .unwrap();
    owner.check_completion_validation().unwrap();
    let p = packet_from_projection(&owner, projection);
    assert_eq!(p.schema_version, 2);
    assert!(verify_review_packet(&p));
    println!(
        "write_history_packet_json={}",
        serde_json::to_string(&p).unwrap()
    );
    let calls = Cell::new(0);
    let result = plan(&owner, &p, || {
        calls.set(calls.get() + 1);
        Ok(observation(&owner))
    })
    .unwrap();
    assert_eq!(calls.get(), 2);
    assert_eq!(result.files.len(), 1);
    assert_eq!(
        p.change_set.files()[0].preimage_sha256,
        hash(b"preserve human work")
    );
    assert_eq!(
        p.change_set.files()[0].postimage_sha256,
        hash(b"reviewed retry")
    );
    assert_eq!(
        fs::read(f.root.join("repo/sentinel")).unwrap(),
        b"preserve human work"
    );
    assert!(!f.root.join("owned/candidate-index-review-build").exists());
}

#[test]
fn creation_review_candidate_plan_binds_mixed_original_history_and_creation_modes() {
    let (f, owner, _) = command::tests::candidate_review_fixture_transform_before_validation(
        0o644,
        |owner| {
            let root = owner.created.as_ref().unwrap().path.clone();
            let binding = owner
                .created
                .as_ref()
                .unwrap()
                .held
                .root_path_sha256()
                .to_owned();
            let lease = match owner.into_runtime_worktree(&binding) {
                Ok(lease) => lease,
                Err(_) => panic!("original owner handoff failed"),
            };
            let paths = vec!["new.py".into(), "other.py".into()];
            let scope = LinuxCodingReadScope::select_targets(&root, &paths).unwrap();
            let result = scope
                .prepare_create(
                    &[
                        ("new.py", "created first\n", 0o640),
                        ("other.py", "executable\n", 0o700),
                    ],
                    "Create exact source files",
                    1000,
                )
                .unwrap()
                .apply_with_owned_driver(&scope, &lease, 1000, true, None)
                .unwrap();
            assert_eq!(result.outcome, agentmage_kernel_engine::filesystem_control::FilesystemTransactionOutcome::Committed);
            let mut owner = match lease.try_into_repository() {
                Ok(owner) => owner,
                Err(_) => panic!("creation driver must release owner"),
            };
            let scope = owner.select_created_sources(&["new.py".into()]).unwrap();
            let preview = owner
                .prepare_edit(
                    scope,
                    &[crate::LinuxCodingReplacement {
                        path: "new.py",
                        before_sha256: &hash(b"created first\n"),
                        content: "created final\n",
                    }],
                    "Retry created source",
                    1000,
                )
                .unwrap();
            assert_eq!(
                owner
                    .decide_edit(preview["preview_sha256"].as_str().unwrap(), true, 1000)
                    .unwrap()["outcome"],
                "committed"
            );
            owner
        },
    );
    let projection = owner
        .capture_applied_write_review(
            (0..3)
                .map(|i| ChangePurposeAssignment {
                    operation_id: format!("history-{i}"),
                    purpose: CommitChangePurpose::Behavior,
                })
                .collect(),
        )
        .unwrap();
    let packet = packet_from_projection(&owner, projection);
    assert!(verify_review_packet(&packet));
    println!(
        "creation_review_packet_json={}",
        serde_json::to_string(&packet).unwrap()
    );
    let calls = Cell::new(0);
    let candidate = plan(&owner, &packet, || {
        calls.set(calls.get() + 1);
        Ok(observation(&owner))
    })
    .unwrap();
    assert_eq!(calls.get(), 2);
    assert_eq!(candidate.files.len(), 3);
    for (i, (path, mode, preimage, content)) in [
        (
            "new.py",
            CommitFileMode::Regular,
            "0".repeat(64),
            b"created final\n".as_slice(),
        ),
        (
            "other.py",
            CommitFileMode::Executable,
            "0".repeat(64),
            b"executable\n".as_slice(),
        ),
        (
            "sentinel",
            CommitFileMode::Regular,
            hash(b"preserve human work"),
            b"reviewed change".as_slice(),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(candidate.files[i].path, path);
        assert_eq!(candidate.files[i].mode, mode);
        assert_eq!(candidate.files[i].postimage_sha256, hash(content));
        assert_eq!(packet.change_set.files()[i].preimage_sha256, preimage);
    }
    assert!(!candidate.network_authority && !candidate.hooks_enabled && !candidate.filters_enabled);
    let mut checker = CandidateChecker(|input: LinuxCodingCandidateReviewEvidence<'_>| {
        let sources = input.sources();
        assert_eq!(sources.len(), 3);
        assert_eq!(sources[0].path(), "new.py");
        assert_eq!(sources[0].preimage(), None);
        assert_eq!(sources[0].postimage(), b"created final\n");
        assert_eq!(sources[0].mode(), CommitFileMode::Regular);
        assert_eq!(sources[1].path(), "other.py");
        assert_eq!(sources[1].preimage(), None);
        assert_eq!(sources[1].postimage(), b"executable\n");
        assert_eq!(sources[1].mode(), CommitFileMode::Executable);
        assert_eq!(
            sources[2].preimage(),
            Some(b"preserve human work".as_slice())
        );
        assert_eq!(sources[2].postimage(), b"reviewed change");
        Ok(())
    });
    selected_review(&owner)
        .verify_candidate_tree_with(
            &packet,
            &packet.commit_plan.groups[0].group_id,
            "review-mixed-sources",
            &mut checker,
            &review_token(),
            || Ok(observation(&owner)),
        )
        .unwrap();
    let split_projection = owner
        .capture_applied_write_review(
            (0..3)
                .map(|i| ChangePurposeAssignment {
                    operation_id: format!("history-{i}"),
                    purpose: if i == 1 {
                        CommitChangePurpose::Formatting
                    } else {
                        CommitChangePurpose::Behavior
                    },
                })
                .collect(),
        )
        .unwrap();
    let split_packet = packet_from_projection(&owner, split_projection);
    assert_eq!(split_packet.commit_plan.groups.len(), 2);
    let split = selected_review(&owner)
        .verify_candidate_tree_with(
            &split_packet,
            &split_packet.commit_plan.groups[0].group_id,
            "review-split-sources",
            &mut checker,
            &review_token(),
            || Ok(observation(&owner)),
        )
        .unwrap();
    assert_eq!(split.candidate().files.len(), 2);
    // Absence is required at the immutable base, not just at the original
    // filesystem creation dispatch. Even an empty base blob is occupied.
    for occupied in [hash(b"base content\n"), hash(b""), "0".repeat(64)] {
        let result = plan(&owner, &packet, || {
            let mut observed = observation(&owner);
            observed.base_preimages.insert(
                "new.py".into(),
                base_file(occupied.clone(), CommitFileMode::Regular),
            );
            Ok(observed)
        });
        assert_eq!(result.unwrap_err().code(), "code.commit.base-preimage");
    }
    assert!(!f.root.join("repo/new.py").exists());
    assert!(!f.root.join("repo/other.py").exists());
    assert_eq!(
        fs::read(f.root.join("repo/sentinel")).unwrap(),
        b"preserve human work"
    );
    assert!(!f.root.join("owned/candidate-index-review-build").exists());
    fs::set_permissions(
        f.root.join("owned/worktree/other.py"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    assert_eq!(
        plan(&owner, &packet, || panic!(
            "mode drift must refuse before Git"
        ))
        .unwrap_err()
        .code(),
        "code.commit.write-provenance"
    );
    fs::set_permissions(
        f.root.join("owned/worktree/other.py"),
        fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    fs::write(f.root.join("owned/worktree/new.py"), b"foreign source\n").unwrap();
    assert_eq!(
        plan(&owner, &packet, || panic!(
            "source drift must refuse before Git"
        ))
        .unwrap_err()
        .code(),
        "code.commit.write-provenance"
    );
}

// This explicit test-only checker is synthetic acceptance, not an installed
// independent reviewer, native admission or a model/coding score.
struct CandidateChecker<F>(F);
impl<F> LinuxCodingCandidateReviewPort for CandidateChecker<F>
where
    F: FnMut(
        LinuxCodingCandidateReviewEvidence<'_>,
    ) -> std::result::Result<(), LinuxCodingCandidateReviewFailure>,
{
    fn verify_candidate(
        &mut self,
        input: LinuxCodingCandidateReviewEvidence<'_>,
    ) -> std::result::Result<(), LinuxCodingCandidateReviewFailure> {
        (self.0)(input)
    }
}

fn review_token() -> CancellationToken {
    CancellationToken::root(
        BoundaryKind::Shell,
        TaskId::from_raw("candidate-review"),
        CorrelationId::from_raw("candidate-review"),
    )
}
fn cancel_review(token: &CancellationToken) {
    token
        .cancel(agentmage_kernel_contracts::CancellationSignal {
            schema_version: CONTRACT_SCHEMA_VERSION,
            cancellation_id: agentmage_kernel_contracts::CancellationId::from_raw("review-cancel"),
            correlation_id: CorrelationId::from_raw("candidate-review"),
            task_id: TaskId::from_raw("candidate-review"),
            reason: agentmage_kernel_contracts::CancellationReason::UserRequested,
            requested_by: BoundaryKind::Shell,
        })
        .unwrap();
}
fn selected_review(owner: &LinuxCodingRepository) -> LinuxCodingCommitReview<'_> {
    let status = owner.inspect_apply().unwrap();
    owner
        .consented_commit_review(
            status["repository_root_sha256"].as_str().unwrap(),
            status["worktree_root_sha256"].as_str().unwrap(),
        )
        .unwrap()
}

#[test]
fn candidate_checker_receives_original_records_and_releases_only_rechecked_inert_plan() {
    let (f, owner, changes) = command::tests::candidate_review_fixture();
    let p = packet(&owner, &changes);
    let review = selected_review(&owner);
    let calls = Cell::new(0);
    let checked = Cell::new(0);
    let receipts = owner.inspect().unwrap()["receipts"].clone();
    let validation = owner.inspect_validation().unwrap();
    let mut checker = CandidateChecker(|input: LinuxCodingCandidateReviewEvidence<'_>| {
        assert_eq!(calls.get(), 2);
        checked.set(checked.get() + 1);
        assert!(std::ptr::eq(input.packet(), &p));
        assert!(std::ptr::eq(
            input.creation(),
            owner.created_git_identity().unwrap()
        ));
        input.validation().assert_original_for_test(&owner);
        assert_eq!(
            input.validation().creation_receipt_sha256(),
            input.creation().authority_receipt_sha256()
        );
        assert_eq!(input.candidate().review_packet_sha256, p.packet_sha256);
        let sources = input.sources();
        assert_eq!(sources.len(), 1);
        assert_eq!(
            sources[0].operation_id(),
            p.change_set.files()[0].operation_id
        );
        assert_eq!(sources[0].path(), "sentinel");
        assert_eq!(
            sources[0].preimage(),
            Some(b"preserve human work".as_slice())
        );
        assert_eq!(sources[0].postimage(), b"reviewed change");
        assert_eq!(sources[0].mode(), CommitFileMode::Regular);
        assert_eq!(
            input.candidate().parent_object,
            input.creation().base_object()
        );
        assert_eq!(
            input.creation().branch_ref(),
            "refs/heads/agentmage/tasks/component"
        );
        Ok(())
    });
    let value = review
        .verify_candidate_tree_with(
            &p,
            &p.commit_plan.groups[0].group_id,
            "checked-candidate",
            &mut checker,
            &review_token(),
            || {
                calls.set(calls.get() + 1);
                Ok(observation(&owner))
            },
        )
        .unwrap();
    assert_eq!(calls.get(), 4);
    assert_eq!(checked.get(), 1);
    assert!(std::ptr::eq(value.packet(), &p));
    assert!(std::ptr::eq(value._owner, &owner));
    assert_eq!(value.candidate().review_packet_sha256, p.packet_sha256);
    assert!(!value.candidate().network_authority);
    assert_eq!(owner.inspect().unwrap()["receipts"], receipts);
    assert_eq!(owner.inspect_validation().unwrap(), validation);
    assert_eq!(
        fs::read(f.root.join("repo/sentinel")).unwrap(),
        b"preserve human work"
    );
    assert_eq!(
        fs::read(f.root.join("owned/worktree/sentinel")).unwrap(),
        b"reviewed change"
    );
    assert!(
        !f.root
            .join("owned/candidate-index-checked-candidate")
            .exists()
    );
}

#[test]
fn candidate_checker_failures_stop_before_postcheck_and_invalid_packets_never_invoke_checker() {
    for (failure, code) in [
        (
            LinuxCodingCandidateReviewFailure::Rejected,
            "code.commit.review-rejected",
        ),
        (
            LinuxCodingCandidateReviewFailure::Unavailable,
            "code.commit.review-unavailable",
        ),
    ] {
        let (f, owner, changes) = command::tests::candidate_review_fixture();
        let p = packet(&owner, &changes);
        let review = selected_review(&owner);
        let calls = Cell::new(0);
        let mut checker = CandidateChecker(|_: LinuxCodingCandidateReviewEvidence<'_>| {
            // A post-check would now produce a different error. The checker
            // failure must stop immediately without more filesystem/Git reads.
            fs::rename(f.root.join("owned/worktree"), f.root.join("moved-worktree")).unwrap();
            Err(failure)
        });
        let result = review.verify_candidate_tree_with(
            &p,
            &p.commit_plan.groups[0].group_id,
            "checker-refusal",
            &mut checker,
            &review_token(),
            || {
                calls.set(calls.get() + 1);
                Ok(observation(&owner))
            },
        );
        assert_eq!(result.err().unwrap().code(), code);
        assert_eq!(calls.get(), 2);
        assert_eq!(
            fs::read(f.root.join("repo/sentinel")).unwrap(),
            b"preserve human work"
        );
    }
    let (_f, owner, changes) = command::tests::candidate_review_fixture();
    let mut p = packet(&owner, &changes);
    p.packet_sha256 = hash(b"forged caller packet");
    let mut checker = CandidateChecker(|_: LinuxCodingCandidateReviewEvidence<'_>| {
        panic!("invalid packet must not reach a reviewer")
    });
    assert_eq!(
        selected_review(&owner)
            .verify_candidate_tree_with(
                &p,
                &p.commit_plan.groups[0].group_id,
                "checker-invalid",
                &mut checker,
                &review_token(),
                || panic!("invalid packet must not reach Git"),
            )
            .err()
            .unwrap()
            .code(),
        "code.commit.review-packet"
    );
}

#[test]
fn candidate_checker_success_cannot_hide_source_modes_or_root_drift() {
    for drift in ["source", "mode", "worktree", "original"] {
        let (f, owner, changes) = command::tests::candidate_review_fixture();
        let p = packet(&owner, &changes);
        let review = selected_review(&owner);
        let receipts = owner.inspect().unwrap()["receipts"].clone();
        let history = owner.inspect_validation().unwrap()["history"].clone();
        let calls = Cell::new(0);
        let mut checker = CandidateChecker(|_: LinuxCodingCandidateReviewEvidence<'_>| {
            match drift {
                "source" => {
                    fs::write(f.root.join("owned/worktree/sentinel"), "unreviewed").unwrap()
                }
                "mode" => fs::set_permissions(
                    f.root.join("owned/worktree/sentinel"),
                    fs::Permissions::from_mode(0o700),
                )
                .unwrap(),
                "worktree" => {
                    fs::rename(f.root.join("owned/worktree"), f.root.join("moved-worktree"))
                        .unwrap()
                }
                "original" => {
                    fs::rename(f.root.join("repo"), f.root.join("moved-original")).unwrap()
                }
                _ => unreachable!(),
            }
            Ok(())
        });
        assert!(
            review
                .verify_candidate_tree_with(
                    &p,
                    &p.commit_plan.groups[0].group_id,
                    "checker-drift",
                    &mut checker,
                    &review_token(),
                    || {
                        calls.set(calls.get() + 1);
                        Ok(observation(&owner))
                    },
                )
                .is_err(),
            "{drift}"
        );
        assert_eq!(
            calls.get(),
            2,
            "drift must stop before post-check Git: {drift}"
        );
        assert_eq!(owner.inspect().unwrap()["receipts"], receipts);
        assert_eq!(owner.inspect_validation().unwrap()["history"], history);
        assert!(owner.applied_write.is_some());
        assert!(!f.root.join("owned/candidate-index-checker-drift").exists());
    }
}

#[test]
fn candidate_checker_compares_complete_git_observations_across_the_checker_call() {
    for drift in [
        "manifest",
        "head",
        "branch",
        "linked",
        "inventory",
        "preimages",
    ] {
        for at in [2, 3, 4] {
            let (_f, owner, changes) = command::tests::candidate_review_fixture();
            let p = packet(&owner, &changes);
            let review = selected_review(&owner);
            let calls = Cell::new(0);
            let checked = Cell::new(0);
            let mut checker = CandidateChecker(|_: LinuxCodingCandidateReviewEvidence<'_>| {
                checked.set(checked.get() + 1);
                Ok(())
            });
            let result = review.verify_candidate_tree_with(
                &p,
                &p.commit_plan.groups[0].group_id,
                "checker-git-drift",
                &mut checker,
                &review_token(),
                || {
                    calls.set(calls.get() + 1);
                    let mut observed = observation(&owner);
                    if calls.get() >= at {
                        match drift {
                            "manifest" => {
                                observed.original.index_sha256 = hash(b"changed index");
                                observed.original =
                                    RepositoryPreservationManifest::seal(observed.original)
                                        .unwrap();
                            }
                            "head" => observed.task_head = "a".repeat(40),
                            "branch" => observed.task_branch = Some("refs/heads/foreign".into()),
                            "linked" => {
                                observed.linked_identity = hash(b"different linked metadata")
                            }
                            "inventory" => {
                                observed.task_inventory_sha256 = hash(b"changed task inventory")
                            }
                            "preimages" => observed.base_preimages.clear(),
                            _ => unreachable!(),
                        }
                    }
                    Ok(observed)
                },
            );
            assert_eq!(
                result.err().unwrap().code(),
                "code.commit.review-drift",
                "{drift} at {at}"
            );
            assert_eq!(calls.get(), at);
            assert_eq!(checked.get(), u32::from(at > 2));
        }
    }
}

#[test]
fn candidate_checker_cancellation_stops_each_observation_and_checker_boundary() {
    for stage in [
        "initial",
        "observe-1",
        "observe-2",
        "checker",
        "observe-3",
        "observe-4",
    ] {
        let (_f, owner, changes) = command::tests::candidate_review_fixture();
        let p = packet(&owner, &changes);
        let review = selected_review(&owner);
        let token = review_token();
        if stage == "initial" {
            cancel_review(&token);
        }
        let calls = Cell::new(0);
        let checked = Cell::new(0);
        let mut checker = CandidateChecker(|_: LinuxCodingCandidateReviewEvidence<'_>| {
            checked.set(checked.get() + 1);
            if stage == "checker" {
                cancel_review(&token);
            }
            Ok(())
        });
        let result = review.verify_candidate_tree_with(
            &p,
            &p.commit_plan.groups[0].group_id,
            "checker-cancel",
            &mut checker,
            &token,
            || {
                calls.set(calls.get() + 1);
                if stage == format!("observe-{}", calls.get()) {
                    cancel_review(&token);
                }
                Ok(observation(&owner))
            },
        );
        assert_eq!(
            result.err().unwrap().code(),
            "code.commit.review-cancelled",
            "{stage}"
        );
        let (expected_calls, expected_checks) = match stage {
            "initial" => (0, 0),
            "observe-1" => (1, 0),
            "observe-2" => (2, 0),
            "checker" => (2, 1),
            "observe-3" => (3, 1),
            "observe-4" => (4, 1),
            _ => unreachable!(),
        };
        assert_eq!(
            (calls.get(), checked.get()),
            (expected_calls, expected_checks),
            "{stage}"
        );
    }
}

#[test]
fn candidate_checker_public_native_path_keeps_artifact_admission_and_does_not_invoke_checker() {
    let (f, owner, changes) = command::tests::candidate_review_fixture();
    let p = packet(&owner, &changes);
    let mut checker = CandidateChecker(|_: LinuxCodingCandidateReviewEvidence<'_>| {
        panic!("synthetic repository or unadmitted Git cannot reach the checker")
    });
    let result = selected_review(&owner).verify_candidate_tree(
        &p,
        &p.commit_plan.groups[0].group_id,
        "checker-native",
        &mut checker,
        &review_token(),
    );
    let code = result.err().unwrap().code();
    if let Err(refusal) = LinuxGitArtifact::verify("/usr/bin/git") {
        assert_eq!(code, refusal.kind().code());
    }
    assert!(!f.root.join("owned/candidate-index-checker-native").exists());
    assert_eq!(
        fs::read(f.root.join("repo/sentinel")).unwrap(),
        b"preserve human work"
    );
}

#[test]
fn candidate_checker_refuses_hash_only_base_proofs_before_invoking_reviewer() {
    for bytes in [b"".as_slice(), b"different base", b"preserve human work\0"] {
        let (f, owner, changes) = command::tests::candidate_review_fixture();
        let p = packet(&owner, &changes);
        let calls = Cell::new(0);
        let mut checker = CandidateChecker(|_: LinuxCodingCandidateReviewEvidence<'_>| {
            panic!("a digest without its exact captured base bytes cannot reach review")
        });
        let result = selected_review(&owner).verify_candidate_tree_with(
            &p,
            &p.commit_plan.groups[0].group_id,
            "review-base-content",
            &mut checker,
            &review_token(),
            || {
                calls.set(calls.get() + 1);
                let mut value = observation(&owner);
                value
                    .base_preimages
                    .get_mut("sentinel")
                    .unwrap()
                    .as_mut()
                    .unwrap()
                    .bytes = bytes.to_vec();
                Ok(value)
            },
        );
        assert_eq!(result.err().unwrap().code(), "code.commit.review-source");
        assert_eq!(calls.get(), 2);
        assert!(
            !f.root
                .join("owned/candidate-index-review-base-content")
                .exists()
        );
    }
}

#[test]
fn candidate_review_source_capture_refuses_changed_postimage_and_missing_base() {
    for case in ["postimage", "missing", "absence", "extra", "base-bytes"] {
        let (f, owner, changes) = command::tests::candidate_review_fixture();
        let p = packet(&owner, &changes);
        let mut observed = observation(&owner);
        match case {
            "postimage" => {
                fs::write(f.root.join("owned/worktree/sentinel"), b"external edit").unwrap()
            }
            "missing" => {
                observed.base_preimages.clear();
            }
            "absence" => {
                observed.base_preimages.insert("sentinel".into(), None);
            }
            "extra" => {
                observed.base_preimages.insert("other.py".into(), None);
            }
            "base-bytes" => {
                observed
                    .base_preimages
                    .get_mut("sentinel")
                    .unwrap()
                    .as_mut()
                    .unwrap()
                    .bytes
                    .clear();
            }
            _ => unreachable!(),
        }
        assert!(
            owner
                .capture_candidate_review_sources(&p, &observed)
                .is_err(),
            "{case}"
        );
        assert_eq!(
            fs::read(f.root.join("repo/sentinel")).unwrap(),
            b"preserve human work"
        );
    }
}

#[test]
fn candidate_checker_distinguishes_existing_empty_base_source_from_absence() {
    let (f, owner, changes) =
        command::tests::candidate_review_fixture_from_base(0o644, b"", |owner| owner);
    let packet = packet(&owner, &changes);
    assert_eq!(packet.change_set.files()[0].preimage_sha256, hash(b""));
    let calls = Cell::new(0);
    let mut checker = CandidateChecker(|input: LinuxCodingCandidateReviewEvidence<'_>| {
        calls.set(calls.get() + 1);
        let sources = input.sources();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].path(), "sentinel");
        assert_eq!(sources[0].preimage(), Some(b"".as_slice()));
        assert_eq!(sources[0].postimage(), b"reviewed change");
        assert_eq!(sources[0].mode(), CommitFileMode::Regular);
        Ok(())
    });
    let review = selected_review(&owner);
    review
        .verify_candidate_tree_with(
            &packet,
            &packet.commit_plan.groups[0].group_id,
            "empty-base-review",
            &mut checker,
            &review_token(),
            || Ok(observation(&owner)),
        )
        .unwrap();
    assert_eq!(calls.get(), 1);
    let result = review.verify_candidate_tree_with(
        &packet,
        &packet.commit_plan.groups[0].group_id,
        "absent-base-review",
        &mut checker,
        &review_token(),
        || {
            let mut observed = observation(&owner);
            observed.base_preimages.insert("sentinel".into(), None);
            Ok(observed)
        },
    );
    assert!(result.is_err());
    assert_eq!(calls.get(), 1, "an absent base cannot reach the checker");
    assert_eq!(fs::read(f.root.join("repo/sentinel")).unwrap(), b"");
    assert_eq!(
        fs::read(f.root.join("owned/worktree/sentinel")).unwrap(),
        b"reviewed change"
    );
    for id in ["empty-base-review", "absent-base-review"] {
        assert!(!f.root.join(format!("owned/candidate-index-{id}")).exists());
    }
}

#[test]
fn candidate_native_observation_precancel_refuses_before_opening_displaced_roots() {
    let (f, owner, changes) = command::tests::candidate_review_fixture();
    let packet = packet(&owner, &changes);
    let token = review_token();
    cancel_review(&token);
    // Call the real native observation wrapper, not the outer review seam.
    // Precancellation must beat held-root and installed-artifact admission.
    fs::rename(f.root.join("repo"), f.root.join("saved-repo")).unwrap();
    assert_eq!(
        owner
            .observe_commit_git(&packet, Some(&token))
            .err()
            .unwrap()
            .code(),
        "code.commit.review-cancelled"
    );
    assert!(owner.observe_commit_git(&packet, None).is_err());
    assert_eq!(
        fs::read(f.root.join("saved-repo/sentinel")).unwrap(),
        b"preserve human work"
    );
    assert_eq!(
        fs::read(f.root.join("owned/worktree/sentinel")).unwrap(),
        b"reviewed change"
    );
}
