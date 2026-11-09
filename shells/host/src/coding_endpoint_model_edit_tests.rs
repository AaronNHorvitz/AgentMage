// SPDX-License-Identifier: Apache-2.0
//! Strict untrusted proposals, synthetic creation and original native previews.
use super::*;
use crate::coding_endpoint_prepared_startup::CodingEndpointEditProposalError as Refusal;
use agentmage_capability_repository_map::coding_proposal::{
    CodingProposalError, prepare_coding_proposal,
};
use serde_json::{Value, json};

#[path = "coding_endpoint_edit_generation_tests.rs"]
mod generation_tests;

fn owner(f: &Fixture) -> LinuxCodingRepository {
    for dir in [
        "state",
        "original",
        "owned",
        "owned/worktree",
        "owned/worktree/private",
    ] {
        fs::create_dir(f.root.join(dir)).unwrap();
        fs::set_permissions(f.root.join(dir), fs::Permissions::from_mode(0o700)).unwrap();
    }
    for (path, bytes) in [
        ("one.py", "one\n"),
        ("two.py", "two\n"),
        ("private/secret.py", "secret\n"),
    ] {
        fs::write(f.root.join("owned/worktree").join(path), bytes).unwrap();
    }
    LinuxCodingRepository::open(
        &f.root.join("state"),
        b"synthetic-strict-proposal-test-secret",
        1000,
    )
    .unwrap()
    .with_test_created_worktree(&f.root.join("original"), &f.root.join("owned"))
    .unwrap()
}
fn startup(f: &Fixture, protected: &[&str], ledger: &InstructionEvidenceLedger) -> Startup {
    let mut request = controlled(f, Kind::Edit);
    request.workspace_id = WorkspaceId::from_raw("local-coding-worktree-create");
    request.work_packet.mutable_files =
        vec!["one.py".into(), "two.py".into(), "private/secret.py".into()];
    request
        .work_packet
        .mutable_files
        .retain(|path| !protected.contains(&path.as_str()));
    request.work_packet.protected_files = protected.iter().map(|s| (*s).into()).collect();
    Startup::prepare(
        seal_runtime_endpoint_run_request(request).unwrap(),
        Kind::Edit,
        ledger,
    )
    .unwrap()
}
fn proposal_context(
    task: &str,
    sources: &[(&str, &str)],
) -> agentmage_capability_repository_map::coding_proposal::CodingProposalContext {
    prepare_coding_proposal(
        task,
        &sources
            .iter()
            .map(|(path, content)| CodingSource {
                path: (*path).into(),
                content: (*content).into(),
            })
            .collect::<Vec<_>>(),
        2048,
    )
    .unwrap()
}
fn edit(path: &str, old: &str, new: &str) -> Value {
    json!({"path":path,"old":old,"new":new})
}

#[test]
fn endpoint_model_edit_prepares_complete_strict_multifile_diff_without_writing() {
    let f = Fixture::new();
    let mut owner = owner(&f);
    let startup = startup(&f, &[], &f.empty_ledger());
    let context = proposal_context(
        &startup.request().task.objective,
        &[("one.py", "one\n"), ("two.py", "two\n")],
    );
    assert_eq!(context.task(), startup.request().task.objective);
    let preview = startup
        .prepare_edit_proposal(
            &mut owner,
            &context,
            &json!({"edits":[edit("two.py","two","second"),edit("one.py","one","first")]})
                .to_string(),
            1000,
        )
        .unwrap();
    assert_eq!(preview["operation"], "edit-worktree");
    assert_eq!(
        preview["write_preview"]["files"],
        json!(["one.py", "two.py"])
    );
    assert_eq!(owner.inspect_edit().unwrap()["pending"], preview);
    assert!(owner.inspect_edit().unwrap()["latest"].is_null());
    assert_eq!(
        fs::read_to_string(f.root.join("owned/worktree/one.py")).unwrap(),
        "one\n"
    );
    assert_eq!(
        fs::read_to_string(f.root.join("owned/worktree/two.py")).unwrap(),
        "two\n"
    );
    assert!(
        owner.inspect_apply().unwrap()["available_paths"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    // Exact native denial retires this complete preview without a write.
    assert_eq!(
        owner
            .decide_edit(preview["preview_sha256"].as_str().unwrap(), false, 1001)
            .unwrap()["outcome"],
        "denied"
    );
    assert!(owner.inspect_edit().unwrap()["pending"].is_null());
    assert!(
        owner
            .decide_edit(preview["preview_sha256"].as_str().unwrap(), true, 1002)
            .is_err()
    );
    assert_eq!(
        fs::read_to_string(f.root.join("owned/worktree/one.py")).unwrap(),
        "one\n"
    );
}

#[test]
fn endpoint_model_edit_refuses_invalid_or_stale_proposals_without_replacing_preview() {
    let f = Fixture::new();
    let mut owner = owner(&f);
    let startup = startup(&f, &[], &f.empty_ledger());
    let context = proposal_context(
        &startup.request().task.objective,
        &[("one.py", "one\n"), ("two.py", "two\n")],
    );
    let good = json!({"edits":[edit("one.py","one","first")]}).to_string();
    let prior = startup
        .prepare_edit_proposal(&mut owner, &context, &good, 1000)
        .unwrap();
    let invalid = [
        (
            "{\"edits\":[],\"edits\":[]}".into(),
            CodingProposalError::Format,
        ),
        (
            "{\"edits\":[],\"execute\":true}".into(),
            CodingProposalError::Format,
        ),
        (
            "[[[\"one.py\",\"one\",\"first\"]]]".into(),
            CodingProposalError::Format,
        ),
        (format!("```json\n{good}\n```"), CodingProposalError::Format),
        (
            json!({"edits":[edit("one.py","one","first"),edit("two.py","absent","second")]})
                .to_string(),
            CodingProposalError::Edit,
        ),
        (
            json!({"edits":[edit("one.py","one","first"),edit("one.py","one","second")]})
                .to_string(),
            CodingProposalError::Edit,
        ),
        (
            json!({"edits":[edit("../escape.py","one","first")]}).to_string(),
            CodingProposalError::Edit,
        ),
    ];
    for (answer, error) in invalid {
        assert_eq!(
            startup.prepare_edit_proposal(&mut owner, &context, &answer, 1001),
            Err(Refusal::Proposal(error))
        );
        assert_eq!(owner.inspect_edit().unwrap()["pending"], prior);
    }
    // Even an UNEDITED context file must still match its complete old snapshot.
    fs::write(f.root.join("owned/worktree/two.py"), "changed by person\n").unwrap();
    assert_eq!(
        startup.prepare_edit_proposal(&mut owner, &context, &good, 1002),
        Err(Refusal::Proposal(CodingProposalError::Stale))
    );
    assert_eq!(owner.inspect_edit().unwrap()["pending"], prior);
    assert_eq!(
        fs::read_to_string(f.root.join("owned/worktree/one.py")).unwrap(),
        "one\n"
    );
}

#[test]
fn endpoint_model_edit_checks_task_exclusions_and_targets_before_replacing_preview() {
    let mut f = Fixture::new();
    f.request.workspace_id = WorkspaceId::from_raw("local-coding-worktree-create");
    f.request = seal_runtime_endpoint_run_request(f.request.clone()).unwrap();
    let mut owner = owner(&f);
    let startup = startup(&f, &["two.py", "private"], &f.empty_ledger());
    let task = &startup.request().task.objective;
    let context = proposal_context(
        task,
        &[
            ("one.py", "one\n"),
            ("two.py", "two\n"),
            ("private/secret.py", "secret\n"),
        ],
    );
    let good = json!({"edits":[edit("one.py","one","first")]}).to_string();
    // Protected files may supply consented context but cannot be edited.
    let prior = startup
        .prepare_edit_proposal(&mut owner, &context, &good, 1000)
        .unwrap();
    for path in ["two.py", "private/secret.py"] {
        let old = if path == "two.py" { "two" } else { "secret" };
        let answer =
            json!({"edits":[edit("one.py","one","first"),edit(path,old,"forbidden")]}).to_string();
        assert_eq!(
            startup.prepare_edit_proposal(&mut owner, &context, &answer, 1001),
            Err(Refusal::Scope)
        );
        assert_eq!(owner.inspect_edit().unwrap()["pending"], prior);
    }
    let foreign_task = proposal_context("A different task", &[("one.py", "one\n")]);
    assert_eq!(
        startup.prepare_edit_proposal(&mut owner, &foreign_task, &good, 1001),
        Err(Refusal::Scope)
    );
    // Excluded file does not exist: a native read would refuse Native. Scope
    // refusal proves exclusion precedes selection/reading of that path.
    let mut request = startup.request().clone();
    request.workspace_id = f.request.workspace_id.clone();
    let narrowed = Startup::prepare(
        seal_runtime_endpoint_run_request(request).unwrap(),
        Kind::Edit,
        &f.ledger(vec![constraint(
            GuidanceConstraintKind::ExcludeScope,
            "private",
        )]),
    )
    .unwrap();
    let excluded = proposal_context(task, &[("private/missing.py", "missing\n")]);
    assert_eq!(
        narrowed.prepare_edit_proposal(&mut owner, &excluded, &good, 1001),
        Err(Refusal::Scope)
    );
    assert_eq!(owner.inspect_edit().unwrap()["pending"], prior);
}

#[test]
fn endpoint_model_edit_refuses_nonedit_unlisted_and_native_source_substitutions() {
    let f = Fixture::new();
    let mut owner = owner(&f);
    let startup = startup(&f, &[], &f.empty_ledger());
    let context = proposal_context(&startup.request().task.objective, &[("one.py", "one\n")]);
    let answer = json!({"edits":[edit("one.py","one","first")]}).to_string();
    let prior = startup
        .prepare_edit_proposal(&mut owner, &context, &answer, 1000)
        .unwrap();

    let command = Startup::prepare(
        controlled(&f, Kind::Command),
        Kind::Command,
        &f.empty_ledger(),
    )
    .unwrap();
    assert_eq!(
        command.prepare_edit_proposal(&mut owner, &context, &answer, 1001),
        Err(Refusal::Scope)
    );

    let mut request = startup.request().clone();
    request.work_packet.mutable_files = vec!["two.py".into()];
    let unlisted = Startup::prepare(
        seal_runtime_endpoint_run_request(request).unwrap(),
        Kind::Edit,
        &f.empty_ledger(),
    )
    .unwrap();
    assert_eq!(
        unlisted.prepare_edit_proposal(&mut owner, &context, &answer, 1001),
        Err(Refusal::Scope)
    );
    assert_eq!(owner.inspect_edit().unwrap()["pending"], prior);

    // No caller path can substitute an arbitrary outside file for the held
    // selected preimage. A symlink refuses before proposal/native preparation.
    let file = f.root.join("owned/worktree/one.py");
    fs::remove_file(&file).unwrap();
    std::os::unix::fs::symlink(f.root.join("sentinel"), &file).unwrap();
    assert_eq!(
        startup.prepare_edit_proposal(&mut owner, &context, &answer, 1001),
        Err(Refusal::Native)
    );
    assert_eq!(owner.inspect_edit().unwrap()["pending"], prior);
    assert_eq!(
        fs::read(f.root.join("sentinel")).unwrap(),
        b"unchanged synthetic source"
    );
}

#[test]
fn endpoint_model_edit_refuses_foreign_workspace_before_reads_or_preview_replacement() {
    let f = Fixture::new();
    let mut repository = owner(&f);
    let startup = startup(&f, &[], &f.empty_ledger());
    let context = proposal_context(&startup.request().task.objective, &[("one.py", "one\n")]);
    let answer = json!({"edits":[edit("one.py","one","first")]}).to_string();
    let prior = startup
        .prepare_edit_proposal(&mut repository, &context, &answer, 1000)
        .unwrap();
    assert_eq!(
        repository.created_workspace_id(),
        Some(&startup.request().workspace_id)
    );
    let mut request = startup.request().clone();
    request.workspace_id = WorkspaceId::from_raw("foreign-workspace");
    let foreign = Startup::prepare(
        seal_runtime_endpoint_run_request(request).unwrap(),
        Kind::Edit,
        &f.empty_ledger(),
    )
    .unwrap();
    // All snapshots and the proposal are valid. Previously this replaced the
    // original owner's preview despite the foreign sealed workspace label.
    assert_eq!(
        foreign.prepare_edit_proposal(&mut repository, &context, &answer, 1001),
        Err(Refusal::Scope)
    );
    assert_eq!(repository.inspect_edit().unwrap()["pending"], prior);
    assert_eq!(
        fs::read(f.root.join("owned/worktree/one.py")).unwrap(),
        b"one\n"
    );
    fs::remove_file(f.root.join("owned/worktree/one.py")).unwrap();
    // Even an unreadable selection still refuses Scope. Native would mean
    // selection/reading was reached; the matching startup proves that path.
    assert_eq!(
        foreign.prepare_edit_proposal(&mut repository, &context, &answer, 1002),
        Err(Refusal::Scope)
    );
    assert_eq!(
        startup.prepare_edit_proposal(&mut repository, &context, &answer, 1002),
        Err(Refusal::Native)
    );
    assert_eq!(repository.inspect_edit().unwrap()["pending"], prior);
    assert!(repository.inspect_edit().unwrap()["latest"].is_null());
}

#[test]
fn endpoint_model_edit_workspace_label_cannot_adopt_missing_or_replaced_owner() {
    let f = Fixture::new();
    let mut repository = owner(&f);
    let startup = startup(&f, &[], &f.empty_ledger());
    let context = proposal_context(&startup.request().task.objective, &[("one.py", "one\n")]);
    let answer = json!({"edits":[edit("one.py","one","first")]}).to_string();
    let prior = startup
        .prepare_edit_proposal(&mut repository, &context, &answer, 1000)
        .unwrap();
    let state = f.root.join("uncreated-state");
    fs::create_dir(&state).unwrap();
    fs::set_permissions(&state, fs::Permissions::from_mode(0o700)).unwrap();
    let mut uncreated =
        LinuxCodingRepository::open(&state, b"synthetic-uncreated-workspace-test-secret", 1000)
            .unwrap();
    assert_eq!(uncreated.created_workspace_id(), None);
    assert_eq!(
        startup.prepare_edit_proposal(&mut uncreated, &context, &answer, 1001),
        Err(Refusal::Scope)
    );
    assert!(uncreated.inspect_edit().unwrap()["pending"].is_null());
    // A matching in-memory label is deliberately insufficient. Replacing the
    // directory at the same path must still fail the original held-root check.
    let worktree = f.root.join("owned/worktree");
    let displaced = f.root.join("owned/displaced");
    fs::rename(&worktree, &displaced).unwrap();
    fs::create_dir(&worktree).unwrap();
    fs::set_permissions(&worktree, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(worktree.join("one.py"), "one\n").unwrap();
    assert_eq!(
        repository.created_workspace_id(),
        Some(&startup.request().workspace_id)
    );
    assert_eq!(
        startup.prepare_edit_proposal(&mut repository, &context, &answer, 1002),
        Err(Refusal::Native)
    );
    assert_eq!(repository.inspect_edit().unwrap()["pending"], prior);
    assert_eq!(fs::read(displaced.join("one.py")).unwrap(), b"one\n");
    assert_eq!(fs::read(worktree.join("one.py")).unwrap(), b"one\n");
}
