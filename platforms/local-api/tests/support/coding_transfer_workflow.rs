// SPDX-License-Identifier: MIT OR Apache-2.0
//! Synthetic API observations through the actual terminal, never native effects.

use super::*;

fn transfer(script: &mut Script, setup: &Value) -> usize {
    let start = script.routes.len();
    script.push("GET /v1/status", json!({"busy":false}));
    let state = json!({"repository_root_sha256":setup["repository_root_sha256"],
        "worktree_root_sha256":script.root,"available_paths":["calc.py"],"pending":null});
    script.push("GET /v1/code/repository/apply", state.clone());
    script.push("GET /v1/code/repository/apply", state);
    let preview = json!({"operation":"apply-to-original",
        "repository_root_sha256":setup["repository_root_sha256"],
        "worktree_root_sha256":script.root,"creation_receipt_sha256":"e".repeat(64),
        "preview_sha256":"f".repeat(64),"write_preview":{"diff":"synthetic original diff"}});
    script.push("POST /v1/code/repository/apply/prepare", preview.clone());
    script.push("GET /v1/code/repository/apply", json!({"pending":preview}));
    script.push(
        "POST /v1/code/repository/apply/decide",
        json!({"outcome":"committed",
        "preview_sha256":preview["preview_sha256"],
        "creation_receipt_sha256":preview["creation_receipt_sha256"]}),
    );
    start
}

fn run_transfer(
    f: &Fixture,
    script: Script,
    t: &ValidationTemplate,
    answer: &str,
    seconds: &str,
) -> (Value, Vec<Value>, Vec<String>) {
    run_command(f, script, t, answer, seconds, "code-repository-apply")
}

#[test]
fn transfer_connects_setup_repair_pass_and_two_exact_original_approvals() {
    let f = Fixture::new();
    let (mut script, setup) = setup(&f);
    let t = template();
    script.start(&t);
    let receipt = script.attempt(&t, 1, None, "assertion_failed");
    script.attempt(&t, 2, Some(&receipt), "passed");
    let start = transfer(&mut script, &setup);
    let tree = script.root.clone();
    let (out, calls, routes) = run_transfer(&f, script, &t, "approve", "30");
    assert_eq!(out["status"], 0, "{out}");
    assert_eq!(out["answered"], 13, "{out}");
    assert_prefix(&calls, &routes, routes.len());
    assert_eq!(
        calls[start + 3]["body"],
        json!({"paths":["calc.py"],"task":"Fix addition",
        "approve_read":true,"expected_repository_root_sha256":setup["repository_root_sha256"],
        "expected_worktree_root_sha256":tree})
    );
    assert_eq!(
        calls.last().unwrap()["body"],
        json!({"approve":true,"preview_sha256":"f".repeat(64)})
    );
    let transcript = out["transcript"].as_str().unwrap();
    assert!(transcript.contains("Type exactly apply-original "), "{out}");
    assert!(transcript.contains("guided-original-transfer"), "{out}");
    assert!(
        transcript.contains("\"original_checkout_tested\": false"),
        "{out}"
    );
    // The peer simulates replies, not writes. Preserve all original fixture bytes.
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
fn transfer_requires_a_full_passing_worktree_receipt() {
    for statuses in [
        vec!["assertion_failed"; 3],
        vec!["zero_tests"],
        vec!["flaky"],
    ] {
        let f = Fixture::new();
        let (mut script, setup) = setup(&f);
        let t = template();
        script.start(&t);
        let mut prior = None;
        for (index, status) in statuses.into_iter().enumerate() {
            prior = Some(script.attempt(&t, index as u64 + 1, prior.as_deref(), status));
        }
        transfer(&mut script, &setup);
        let (out, calls, routes) = run_transfer(&f, script, &t, "approve", "30");
        assert_ne!(out["status"], 0, "{out}");
        assert_prefix(&calls, &routes, calls.len());
        assert!(
            !calls
                .iter()
                .any(|c| c["route"].as_str().unwrap().contains("/repository/apply")),
            "{out}"
        );
    }
}

#[test]
fn transfer_denial_stops_before_original_read_or_write_dispatch() {
    for (prompt, suffix) in [(7, 3), (8, 5)] {
        let f = Fixture::new();
        let (mut script, setup) = setup(&f);
        let t = template();
        script.start(&t);
        script.attempt(&t, 1, None, "passed");
        let start = transfer(&mut script, &setup);
        let (out, calls, routes) = run_transfer(&f, script, &t, &format!("deny-at-{prompt}"), "30");
        assert_ne!(out["status"], 0, "{out}");
        assert_eq!(out["answered"], prompt + 1, "{out}");
        assert_prefix(&calls, &routes, start + suffix);
    }
}

#[test]
fn transfer_refuses_changed_roots_paths_creation_and_original_preview() {
    for case in [
        "first-original",
        "first-tree",
        "pending",
        "outside",
        "empty",
        "duplicate",
        "second-original",
        "second-tree",
        "prepare-operation",
        "prepare-original",
        "prepare-tree",
        "prepare-creation",
        "prepare-digest",
        "pending-digest",
        "pending-diff",
    ] {
        let f = Fixture::new();
        let (mut script, setup) = setup(&f);
        let t = template();
        script.start(&t);
        script.attempt(&t, 1, None, "passed");
        let start = transfer(&mut script, &setup);
        let (offset, field, value) = match case {
            "first-original" => (1, "repository_root_sha256", json!("9".repeat(64))),
            "first-tree" => (1, "worktree_root_sha256", json!("9".repeat(64))),
            "pending" => (1, "pending", json!({"unrelated":true})),
            "outside" => (1, "available_paths", json!(["sentinel"])),
            "empty" => (1, "available_paths", json!([])),
            "duplicate" => (1, "available_paths", json!(["calc.py", "calc.py"])),
            "second-original" => (2, "repository_root_sha256", json!("9".repeat(64))),
            "second-tree" => (2, "worktree_root_sha256", json!("9".repeat(64))),
            "prepare-operation" => (3, "operation", json!("edit-worktree")),
            "prepare-original" => (3, "repository_root_sha256", json!("9".repeat(64))),
            "prepare-tree" => (3, "worktree_root_sha256", json!("9".repeat(64))),
            "prepare-creation" => (3, "creation_receipt_sha256", json!("9".repeat(64))),
            "prepare-digest" => (3, "preview_sha256", json!("invalid")),
            "pending-digest" => (4, "preview_sha256", json!("9".repeat(64))),
            "pending-diff" => (4, "write_preview", json!({"diff":"replacement"})),
            _ => unreachable!(),
        };
        if offset == 4 {
            script.replies[start + offset]["pending"][field] = value;
        } else {
            script.replies[start + offset][field] = value;
        }
        let (out, calls, routes) = run_transfer(&f, script, &t, "approve", "30");
        assert_ne!(out["status"], 0, "{case}: {out}");
        assert_eq!(
            out["answered"],
            if offset < 3 { 7 } else { 8 },
            "{case}: {out}"
        );
        assert_prefix(&calls, &routes, start + offset + 1);
    }
}

#[test]
fn transfer_never_repeats_lost_effect_replies_or_accepts_substituted_results() {
    for case in [
        "prepare-lost",
        "decide-lost",
        "result-preview",
        "result-creation",
        "result-outcome",
    ] {
        let f = Fixture::new();
        let (mut script, setup) = setup(&f);
        let t = template();
        script.start(&t);
        script.attempt(&t, 1, None, "passed");
        let start = transfer(&mut script, &setup);
        let offset = if case == "prepare-lost" { 3 } else { 5 };
        if case.ends_with("lost") {
            script.replies[start + offset] = json!({"test_peer_close":true});
        } else {
            let field = match case {
                "result-preview" => "preview_sha256",
                "result-creation" => "creation_receipt_sha256",
                _ => "outcome",
            };
            script.replies[start + offset][field] = json!("9".repeat(64));
        }
        let (out, calls, routes) = run_transfer(&f, script, &t, "approve", "30");
        assert_ne!(out["status"], 0, "{case}: {out}");
        assert_prefix(&calls, &routes, start + offset + 1);
        assert!(
            !out["transcript"]
                .as_str()
                .unwrap()
                .contains("guided-original-transfer"),
            "{out}"
        );
    }
}

#[test]
fn transfer_approvals_obey_original_deadline_and_every_stop_signal() {
    for prompt in [7, 8] {
        for action in ["wait", "SIGINT", "SIGTERM", "SIGHUP"] {
            let f = Fixture::new();
            let (mut script, setup) = setup(&f);
            let t = template();
            script.start(&t);
            script.attempt(&t, 1, None, "passed");
            let start = transfer(&mut script, &setup);
            let (out, calls, routes) = run_transfer(
                &f,
                script,
                &t,
                &format!("workflow-stop:{prompt}:{action}"),
                if action == "wait" { "1" } else { "30" },
            );
            assert_ne!(out["status"], 0, "{out}");
            assert_eq!(out["answered"], prompt + 1, "{out}");
            assert!(out["seconds"].as_f64().unwrap() < 3.0, "{out}");
            assert_prefix(&calls, &routes, start + if prompt == 7 { 3 } else { 5 });
        }
    }
}

#[test]
fn transfer_does_not_renew_the_setup_clock() {
    let f = Fixture::new();
    let (mut script, setup) = setup(&f);
    let t = template();
    script.start(&t);
    script.attempt(&t, 1, None, "passed");
    let start = transfer(&mut script, &setup);
    let (out, calls, routes) = run_transfer(&f, script, &t, "transfer-clock", "2");
    assert_ne!(out["status"], 0, "{out}");
    assert_eq!(out["answered"], 8, "{out}");
    assert!(out["seconds"].as_f64().unwrap() >= 1.8, "{out}");
    assert!(out["seconds"].as_f64().unwrap() < 2.8, "{out}");
    assert_prefix(&calls, &routes, start + 3);
}

#[test]
fn transfer_refuses_piped_execution_and_invalid_budgets_before_requests() {
    let t = template();
    for seconds in ["0", "3601", "not-a-number"] {
        let f = Fixture::new();
        let (script, _) = setup(&f);
        let (out, calls, _) = run_transfer(&f, script, &t, "approve", seconds);
        assert_ne!(out["status"], 0, "{out}");
        assert_eq!(out["answered"], 0, "{out}");
        assert!(calls.is_empty());
    }
    let f = Fixture::new();
    let peer = f.peer(vec![]);
    let out = f.cli(
        peer.address,
        &[
            "code-repository-apply",
            "30",
            f.root.join("repo").to_str().unwrap(),
            f.root.join("owned").to_str().unwrap(),
            "addition",
            &"1".repeat(40),
            "calc",
            "synthetic-model",
            "4096",
            "128",
            "Fix addition",
            &definition(&t),
        ],
    );
    assert!(!out.status.success(), "{out:?}");
    assert!(peer.finish().is_empty());
}
