// SPDX-License-Identifier: MIT OR Apache-2.0
//! Actual terminal with synthetic setup, model, write and command replies.
//! The peer never creates a worktree or establishes native ownership.

use super::*;
use agentmage_kernel_engine::repository_safety::{
    RepositoryPreservationManifest, plan_worktree_create,
};
use std::os::unix::{ffi::OsStrExt, fs::PermissionsExt};

#[path = "coding_transfer_workflow.rs"]
mod transfer;

fn setup(f: &Fixture) -> (Script, Value) {
    for name in ["repo", "owned"] {
        fs::create_dir(f.root.join(name)).unwrap();
        fs::set_permissions(f.root.join(name), fs::Permissions::from_mode(0o700)).unwrap();
    }
    fs::write(f.root.join("repo/sentinel"), b"original human work").unwrap();
    let mut script = Script::new_at(&f.root.join("owned/worktree"));
    let identity = |name: &str| {
        agentmage_platform_linux::discover_linux_coding_map(&f.root.join(name), "sentinel", || {
            false
        })
        .unwrap()
        .root_sha256
    };
    let manifest = RepositoryPreservationManifest::seal(RepositoryPreservationManifest {
        schema_version: 0,
        repository_sha256: hash(b"repo"),
        common_directory_sha256: hash(b"common"),
        object_format: "sha1".into(),
        safe_ownership: true,
        head_object: Some("1".repeat(40)),
        current_branch: Some("refs/heads/main".into()),
        upstream: None,
        detached: false,
        index_sha256: hash(b"index"),
        path_dispositions_sha256: hash(b"paths"),
        untracked_count: 1,
        ignored_count: 0,
        local_branches_sha256: hash(b"local"),
        remote_tracking_refs_sha256: hash(b"remote"),
        tags_sha256: hash(b"tags"),
        notes_sha256: hash(b"notes"),
        stash_sha256: hash(b"stash"),
        replacement_refs_sha256: hash(b"replacements"),
        reflogs_sha256: hash(b"reflogs"),
        agentmage_refs_sha256: hash(b"agentmage"),
        worktrees_sha256: hash(b"worktrees"),
        submodules_sha256: hash(b"submodules"),
        lfs_sha256: hash(b"lfs"),
        object_database_sha256: hash(b"objects"),
        configuration_sha256: hash(b"config"),
        hooks_sha256: hash(b"hooks"),
        content_drivers_sha256: hash(b"drivers"),
        remotes_sha256: hash(b"remotes"),
        operations_sha256: hash(b"operations"),
        shallow: false,
        partial: false,
        hazardous_configuration: false,
        manifest_sha256: String::new(),
    })
    .unwrap();
    let plan = plan_worktree_create(
        "synthetic-setup",
        "addition",
        &"1".repeat(40),
        &agentmage_platform_linux::linux_repository_path_sha256(f.root.join("repo")).unwrap(),
        &hash(f.root.join("owned/worktree").as_os_str().as_bytes()),
        &manifest,
    )
    .unwrap();
    let prepared = json!({"operation":"create-worktree","plan":plan,
        "preview_sha256":"a".repeat(64),"repository_root_sha256":identity("repo"),
        "owned_root_sha256":identity("owned")});
    script.push("POST /v1/code/repository/prepare", prepared.clone());
    script.push("GET /v1/code/repository", json!({"pending":prepared}));
    script.push("POST /v1/code/repository/decide", json!({"outcome":"succeeded",
        "repository_root_sha256":prepared["repository_root_sha256"],"worktree_root_sha256":script.root,
        "receipt":{"receipt_sha256":"e".repeat(64),"outcome":"succeeded"},
        "repository_receipt":{"plan_sha256":plan.plan_sha256,"outcome":"succeeded"}}));
    script.push("GET /v1/status", json!({"busy":false}));
    script.push(
        "POST /v1/code/map",
        json!({"root_sha256":script.root,
        "entries":[{"path":"calc.py","score":1},{"path":"untouched.py","score":0}]}),
    );
    script.push("GET /v1/settings", script.settings.clone());
    script.push(
        "PUT /v1/code/workspace",
        json!({"selected":true,"workspace_revision":1,
        "workspace_selection_id":"synthetic-selection"}),
    );
    script.push("GET /v1/code/workspace", script.selected.clone());
    (script, prepared)
}

fn run(
    f: &Fixture,
    script: Script,
    t: &ValidationTemplate,
    answer: &str,
    seconds: &str,
) -> (Value, Vec<Value>, Vec<String>) {
    run_command(f, script, t, answer, seconds, "code-repository")
}

fn run_command(
    f: &Fixture,
    script: Script,
    t: &ValidationTemplate,
    answer: &str,
    seconds: &str,
    command: &str,
) -> (Value, Vec<Value>, Vec<String>) {
    let peer = f.peer(script.replies);
    let started = Instant::now();
    let output = f.tty(
        peer.address,
        &[
            command,
            seconds,
            f.root.join("repo").to_str().unwrap(),
            f.root.join("owned").to_str().unwrap(),
            "addition",
            &"1".repeat(40),
            "calc",
            "synthetic-model",
            "4096",
            "128",
            "Fix addition",
            &definition(t),
        ],
        answer,
    );
    eprintln!(
        "synthetic repository-setup workflow elapsed_us={}",
        started.elapsed().as_micros()
    );
    (output, peer.finish(), script.routes)
}

fn assert_prefix(calls: &[Value], routes: &[String], count: usize) {
    assert_eq!(
        calls
            .iter()
            .map(|call| call["route"].as_str().unwrap())
            .collect::<Vec<_>>(),
        routes[..count]
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
    );
}

#[test]
fn repository_workflow_connects_setup_and_original_failed_test_repair_cycle() {
    let f = Fixture::new();
    let (mut script, prepared) = setup(&f);
    let root = script.root.clone();
    let settings = script.settings.clone();
    let t = template();
    script.start(&t);
    let prior = script.attempt(&t, 1, None, "assertion_failed");
    script.attempt(&t, 2, Some(&prior), "passed");
    let (out, calls, routes) = run(&f, script, &t, "approve", "30");
    assert_eq!(out["status"], 0, "{out}");
    assert_eq!(out["answered"], 11, "{out}");
    assert_prefix(&calls, &routes, routes.len());
    assert_eq!(
        calls[0]["body"],
        json!({"root":f.root.join("repo"),"owned_root":f.root.join("owned"),
        "task_id":"addition","base_commit":"1".repeat(40),"approve_inspection":true})
    );
    assert_eq!(
        calls[2]["body"],
        json!({"preview_sha256":prepared["preview_sha256"],"approve":true})
    );
    assert_eq!(
        calls[4]["body"],
        json!({"root":f.root.join("owned/worktree"),"query":"calc","approve_discovery":true})
    );
    assert_eq!(
        calls[6]["body"],
        json!({"root":f.root.join("owned/worktree"),"paths":["calc.py"],
        "approve_read":true,"approve_model_context":true,"settings_revision":settings["settings_revision"],
        "settings_endpoint":settings["endpoint"],"expected_root_sha256":root})
    );
    let transcript = out["transcript"].as_str().unwrap();
    for challenge in [
        "inspect ",
        "create ",
        "discover ",
        "share ",
        "read ",
        "edit ",
        "test ",
        "share-failure ",
    ] {
        assert!(
            transcript.contains(&format!("Type exactly {challenge}")),
            "{out}"
        );
    }
    assert_eq!(
        fs::read(f.root.join("repo/sentinel")).unwrap(),
        b"original human work"
    );
    assert_eq!(
        fs::read(f.root.join("owned/worktree/calc.py")).unwrap(),
        b"return a - b\n"
    );
}

#[test]
fn repository_workflow_declines_each_setup_approval_without_next_dispatch() {
    for (prompt, count) in [(0, 0), (1, 2), (2, 4), (3, 6)] {
        let f = Fixture::new();
        let (script, _) = setup(&f);
        let (out, calls, routes) = run(&f, script, &template(), &format!("deny-at-{prompt}"), "30");
        assert_eq!(out["status"], 1, "{out}");
        assert_eq!(out["answered"], prompt + 1, "{out}");
        assert_prefix(&calls, &routes, count);
    }
}

#[test]
fn repository_workflow_refuses_replaced_setup_and_handoff_bindings() {
    for field in [
        "prepare-operation",
        "prepare-branch",
        "prepare-base",
        "prepare-plan",
        "pending",
        "created-repository",
        "created-plan",
        "map-root",
        "accepted-selection",
        "accepted-revision",
        "selected-paths",
        "handoff-selection",
        "handoff-settings",
        "handoff-root",
        "handoff-creation",
    ] {
        let f = Fixture::new();
        let (mut script, _) = setup(&f);
        let t = template();
        script.start(&t);
        script.attempt(&t, 1, None, "passed");
        let (index, count, answers) = match field {
            "prepare-operation" => {
                script.replies[0]["operation"] = json!("other");
                (0, 1, 1)
            }
            "prepare-branch" | "prepare-base" => {
                let mut plan: agentmage_kernel_engine::repository_safety::RepositoryOperationPlan =
                    serde_json::from_value(script.replies[0]["plan"].clone()).unwrap();
                // Build another internally valid plan, so exact request pins matter.
                if field == "prepare-branch" {
                    plan.branch_ref = Some("refs/heads/agentmage/tasks/other".into());
                    let argument = plan.invocations[0]
                        .arguments
                        .iter_mut()
                        .find(|arg| arg.as_str() == "agentmage/tasks/addition")
                        .unwrap();
                    *argument = "agentmage/tasks/other".into();
                } else {
                    plan.expected_new_object = Some("2".repeat(40));
                    *plan.invocations[0].arguments.last_mut().unwrap() = "2".repeat(40);
                }
                plan.invocations[0].invocation_sha256 = "0".repeat(64);
                plan.invocations[0].invocation_sha256 =
                    hash(&serde_json::to_vec(&plan.invocations[0]).unwrap());
                plan.plan_sha256 = "0".repeat(64);
                plan.plan_sha256 = hash(&serde_json::to_vec(&plan).unwrap());
                plan.verify().unwrap();
                script.replies[0]["plan"] = json!(plan);
                (0, 1, 1)
            }
            "prepare-plan" => {
                script.replies[0]["plan"]["plan_sha256"] = json!("f".repeat(64));
                (0, 1, 1)
            }
            "pending" => {
                script.replies[1]["pending"]["warning"] = json!("replacement");
                (1, 2, 1)
            }
            "created-repository" => {
                script.replies[2]["repository_root_sha256"] = json!("f".repeat(64));
                (2, 3, 2)
            }
            "created-plan" => {
                script.replies[2]["repository_receipt"]["plan_sha256"] = json!("f".repeat(64));
                (2, 3, 2)
            }
            "map-root" => {
                script.replies[4]["root_sha256"] = json!("f".repeat(64));
                (4, 5, 3)
            }
            "accepted-selection" => {
                script.replies[6]["workspace_selection_id"] = json!("replacement");
                (6, 8, 4)
            }
            "accepted-revision" => {
                script.replies[6]["workspace_revision"] = json!(2);
                (6, 8, 4)
            }
            "selected-paths" => {
                script.replies[7]["paths"] = json!(["untouched.py"]);
                (7, 8, 4)
            }
            "handoff-selection" => {
                script.replies[8]["workspace_selection_id"] = json!("replacement");
                (8, 10, 4)
            }
            "handoff-settings" => {
                script.replies[9]["settings_revision"] = json!(2);
                (9, 10, 4)
            }
            "handoff-root" => {
                script.replies[10]["worktree_root_sha256"] = json!("f".repeat(64));
                (10, 11, 4)
            }
            "handoff-creation" => {
                script.replies[12]["creation_receipt_sha256"] = json!("f".repeat(64));
                (12, 13, 4)
            }
            _ => unreachable!(),
        };
        assert!(script.replies[index].is_object());
        let (out, calls, routes) = run(&f, script, &t, "approve", "30");
        assert_eq!(out["status"], 1, "{field}: {out}");
        assert_eq!(out["answered"], answers, "{field}: {out}");
        assert_prefix(&calls, &routes, count);
    }
}

#[test]
fn repository_workflow_stops_after_lost_creation_or_selection_reply() {
    for index in [2, 6] {
        let f = Fixture::new();
        let (mut script, _) = setup(&f);
        script.replies[index] = json!({"test_peer_close":true});
        let (out, calls, routes) = run(&f, script, &template(), "approve", "30");
        assert_eq!(out["status"], 1, "{out}");
        assert_prefix(&calls, &routes, index + 1);
        assert!(
            out["transcript"]
                .as_str()
                .unwrap()
                .contains("inspect service records and disk")
        );
    }
}

#[test]
fn repository_workflow_shares_one_deadline_and_signal_latch_across_setup() {
    for prompt in 0..4 {
        for action in ["wait", "SIGINT", "SIGTERM", "SIGHUP"] {
            let f = Fixture::new();
            let (script, _) = setup(&f);
            let (out, calls, routes) = run(
                &f,
                script,
                &template(),
                &format!("workflow-stop:{prompt}:{action}"),
                if action == "wait" { "1" } else { "30" },
            );
            assert_eq!(out["status"], 1, "{prompt}/{action}: {out}");
            assert_prefix(&calls, &routes, [0, 2, 4, 6][prompt]);
        }
    }
}

#[test]
fn repository_workflow_does_not_read_selected_source_before_share_consent() {
    let f = Fixture::new();
    let (script, _) = setup(&f);
    // The synthetic map names a file that no longer exists. Until source-read
    // consent the client must still present that exact selection for review.
    fs::remove_file(f.root.join("owned/worktree/calc.py")).unwrap();
    let (out, calls, routes) = run(&f, script, &template(), "deny-at-3", "30");
    assert_eq!(out["status"], 1, "{out}");
    assert_eq!(out["answered"], 4, "{out}");
    assert_prefix(&calls, &routes, 6);
    assert!(
        out["transcript"]
            .as_str()
            .unwrap()
            .contains("Type exactly share 1")
    );
}

#[test]
fn repository_workflow_does_not_renew_setup_budget_at_guided_handoff() {
    let f = Fixture::new();
    let (mut script, _) = setup(&f);
    let t = template();
    script.start(&t);
    script.attempt(&t, 1, None, "passed");
    // Spend .9 s at inspection, then leave the first guided read unanswered.
    // A renewed 2 s budget would take about 3 s. The original ends near 2 s.
    let (out, calls, routes) = run(&f, script, &t, "repository-clock", "2");
    assert_eq!(out["status"], 1, "{out}");
    assert_eq!(out["answered"], 5, "{out}");
    assert!(
        (1.9..2.7).contains(&out["seconds"].as_f64().unwrap()),
        "{out}"
    );
    let stopped = routes
        .iter()
        .position(|r| r.starts_with("GET /v1/code/repository/edit "))
        .unwrap();
    // The second edit inspection, during proposal preparation, precedes read
    // consent. No edit prepare or any later effect may follow the expiration.
    let last = routes
        .iter()
        .enumerate()
        .skip(stopped + 1)
        .find(|(_, r)| r.starts_with("GET /v1/code/repository/edit "))
        .unwrap()
        .0;
    assert_prefix(&calls, &routes, last + 1);
}

#[test]
fn repository_workflow_refuses_invalid_complete_inputs_and_piped_approvals() {
    let f = Fixture::new();
    let t = template();
    let valid = vec![
        "code-repository".to_owned(),
        "30".into(),
        f.root.join("repo").to_str().unwrap().into(),
        f.root.join("owned").to_str().unwrap().into(),
        "addition".into(),
        "1".repeat(40),
        "calc".into(),
        "synthetic-model".into(),
        "4096".into(),
        "128".into(),
        "Fix addition".into(),
        definition(&t),
    ];
    for (index, invalid) in [
        (1, "0"),
        (1, "3601"),
        (1, "-1"),
        (1, "x"),
        (2, "relative"),
        (3, "/"),
        (4, "../task"),
        (5, "main"),
        (6, "!!!"),
        (6, "calc\n"),
        (7, ""),
        (8, "2047"),
        (9, "0"),
        (10, " "),
        (11, "{}"),
    ] {
        let peer = f.peer(vec![]);
        let mut args = valid.clone();
        args[index] = invalid.into();
        let args = args.iter().map(String::as_str).collect::<Vec<_>>();
        let out = f.tty(peer.address, &args, "approve");
        assert_eq!(out["status"], 1, "{index}/{invalid}: {out}");
        assert_eq!(out["answered"], 0, "{out}");
        assert!(peer.finish().is_empty());
        assert!(
            !out["transcript"]
                .as_str()
                .unwrap()
                .contains("Guided workflow stopped"),
            "{out}"
        );
    }
    let peer = f.peer(vec![]);
    let args = valid.iter().map(String::as_str).collect::<Vec<_>>();
    assert!(!f.cli(peer.address, &args).status.success());
    assert!(peer.finish().is_empty());
}
