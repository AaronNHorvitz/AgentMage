// SPDX-License-Identifier: Apache-2.0
//! One terminal from retained coordinator through saved record and exact diff.
//! Synthetic creation/model; real HTTP, permissions, native edits and transfer.
use super::*;

fn archive(fixture: &Fixture) -> std::path::PathBuf {
    let archive = fixture.0.join("records");
    fs::create_dir(&archive).unwrap();
    fs::set_permissions(&archive, fs::Permissions::from_mode(0o700)).unwrap();
    archive
}

fn run(
    api: &Api,
    fixture: &Fixture,
    request: &Value,
    archive: &std::path::Path,
    seconds: &str,
    scenario: &str,
) -> Value {
    terminal_command(
        api.address,
        fixture,
        scenario,
        &[
            "endpoint-run-apply-saved",
            archive.to_str().unwrap(),
            seconds,
            request["run_id"].as_str().unwrap(),
            request["request_sha256"].as_str().unwrap(),
            fixture.0.join("original").to_str().unwrap(),
            fixture.0.join("owned/worktree").to_str().unwrap(),
            "Review the complete diff",
            "one.py",
        ],
    )
}

fn saved(archive: &std::path::Path, request: &Value) -> Value {
    let path = archive.join(format!(
        "{}.json",
        request["request_sha256"].as_str().unwrap()
    ));
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o7777,
        0o600
    );
    let bytes = fs::read(path).unwrap();
    let record: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(record["request"], *request);
    let mut sequence = agentmage_kernel_engine::runtime_event::RuntimeEventSequence::new();
    for event in record["events"].as_array().unwrap() {
        sequence
            .push(&serde_json::from_value(event.clone()).unwrap())
            .unwrap();
    }
    assert!(sequence.is_terminal());
    record
}

#[test]
fn guided_transfer_runs_original_native_edit_saves_failure_then_applies_only_exact_diff() {
    let fixture = Fixture::new();
    let archive = archive(&fixture);
    let (api, upstream) = start(&fixture, false);
    let peer = thread::spawn(move || {
        let mut first = accept(&upstream);
        let body = read_upstream(&mut first);
        assert!(body.to_string().contains("first"));
        respond(
            &mut first,
            &json!({"edits":[{"path":"one.py","old":"first\n","new":"second\n"}]}).to_string(),
        );
        drop(first);
        let mut second = accept(&upstream);
        let body = read_upstream(&mut second);
        assert!(body.to_string().contains("second"));
        // A genuine failed synthetic model response, never a coding task pass.
        respond(&mut second, "malformed strict edit response");
        drop(second);
        upstream
    });
    let request = api.request();
    let output = run(&api, &fixture, &request, &archive, "30", "allow");
    assert_eq!(output["status"], 0, "{output}");
    let prompts = output["prompts"].as_array().unwrap();
    assert_eq!(prompts.len(), 7, "{output}");
    for (prompt, prefix) in prompts.iter().zip([
        "share ",
        "allow ",
        "share ",
        "save-record ",
        "release ",
        "read ",
        "apply-original ",
    ]) {
        assert!(prompt.as_str().unwrap().starts_with(prefix), "{output}");
    }
    let text = output["transcript"].as_str().unwrap();
    assert!(text.contains("FAILED") && text.contains("service-observation-only"));
    assert!(text.contains("\"original_checkout_tested\": false"));
    assert!(text.contains("\"git_commit_created\": false"));
    assert!(text.find("\"saved\": true").unwrap() < text.find("Type exactly release ").unwrap());
    assert!(
        text.find("\"write_preview\"").unwrap()
            < text.find("Type exactly apply-original ").unwrap()
    );
    let record = saved(&archive, &request);
    assert_eq!(
        record["events"].as_array().unwrap().last().unwrap()["kind"]["state"],
        "FAILED"
    );
    assert_eq!(
        fs::read(fixture.0.join("original/one.py")).unwrap(),
        b"second\n"
    );
    assert_eq!(
        fs::read(fixture.0.join("owned/worktree/one.py")).unwrap(),
        b"second\n"
    );
    assert_eq!(
        fs::read(fixture.0.join("original/human")).unwrap(),
        b"human sentinel"
    );
    assert_eq!(
        fs::read(fixture.0.join("original/.git/index")).unwrap(),
        b"index sentinel"
    );
    assert_eq!(api.bind("events", &request).0, 409);
    let status = api
        .call("GET", "/v1/code/repository/apply", CONTROL, None)
        .1;
    assert_eq!(status["available_paths"], json!([]));
    assert_eq!(status["latest"]["outcome"], "committed");
    drop(api);
    assert_eq!(saved(&archive, &request), record);
    assert!(
        matches!(peer.join().unwrap().accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock)
    );
}

#[test]
fn guided_transfer_declines_every_phase_on_both_original_review_services() {
    for checked in [false, true] {
        for decision in 1..=5 {
            let fixture = Fixture::new();
            let archive = archive(&fixture);
            let (api, upstream) = start(&fixture, checked);
            let request = api.request();
            cancel(&api, &request);
            let output = run(
                &api,
                &fixture,
                &request,
                &archive,
                "30",
                &format!("decline-{decision}"),
            );
            assert_ne!(output["status"], 0, "{output}");
            assert_eq!(output["prompts"].as_array().unwrap().len(), decision);
            preserved(&fixture, b"before\n");
            assert_eq!(
                fs::read_dir(&archive).unwrap().count(),
                usize::from(decision > 2)
            );
            if decision > 2 {
                assert_eq!(
                    saved(&archive, &request)["events"]
                        .as_array()
                        .unwrap()
                        .last()
                        .unwrap()["kind"]["state"],
                    "CANCELLED"
                );
            }
            assert_eq!(
                api.bind("events", &request).0,
                if decision <= 3 { 200 } else { 409 }
            );
            if decision <= 3 {
                assert_eq!(
                    api.call("GET", "/v1/code/repository/apply", CONTROL, None)
                        .0,
                    428
                );
            } else {
                let state = api
                    .call("GET", "/v1/code/repository/apply", CONTROL, None)
                    .1;
                assert_eq!(state["pending"].is_null(), decision == 4);
                assert!(state["latest"].is_null());
                assert_eq!(state["available_paths"], json!(["one.py"]));
            }
            drop(api);
            assert!(
                matches!(upstream.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock)
            );
        }
    }
}

#[test]
fn guided_transfer_one_deadline_and_signal_guard_cover_saving_and_release() {
    for scenario in ["delay-each", "interrupt-release"] {
        let fixture = Fixture::new();
        let archive = archive(&fixture);
        let (api, upstream) = start(&fixture, false);
        let request = api.request();
        cancel(&api, &request);
        let output = run(
            &api,
            &fixture,
            &request,
            &archive,
            if scenario == "delay-each" { "2" } else { "30" },
            scenario,
        );
        assert_ne!(output["status"], 0, "{output}");
        let saving = scenario == "interrupt-release";
        assert_eq!(
            output["prompts"].as_array().unwrap().len(),
            if saving { 3 } else { 2 }
        );
        assert_eq!(fs::read_dir(&archive).unwrap().count(), usize::from(saving));
        assert_eq!(api.bind("events", &request).0, 200);
        assert_eq!(
            api.call("GET", "/v1/code/repository/apply", CONTROL, None)
                .0,
            428
        );
        preserved(&fixture, b"before\n");
        drop(api);
        assert!(matches!(upstream.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
    }
}

#[test]
fn saved_transfer_refuses_archives_inside_either_checkout_before_disclosure_or_release() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    let (api, upstream) = start(&fixture, false);
    let request = api.request();
    let external = archive(&fixture);
    for path in [
        "original/records",
        "original/.git/records",
        "owned/worktree/records",
    ] {
        fs::create_dir(fixture.0.join(path)).unwrap();
        fs::set_permissions(fixture.0.join(path), fs::Permissions::from_mode(0o700)).unwrap();
    }
    symlink(&external, fixture.0.join("alias")).unwrap();
    symlink(
        fixture.0.join("original/records"),
        fixture.0.join("inside-alias"),
    )
    .unwrap();
    for command in ["endpoint-original-apply-saved", "endpoint-run-apply-saved"] {
        for path in [
            "original",
            "original/records",
            "original/.git/records",
            "owned/worktree",
            "owned/worktree/records",
            "alias",
            "inside-alias",
            "records/.",
            "missing",
        ] {
            let output = terminal_command(
                api.address,
                &fixture,
                "allow",
                &[
                    command,
                    fixture.0.join(path).to_str().unwrap(),
                    "30",
                    request["run_id"].as_str().unwrap(),
                    request["request_sha256"].as_str().unwrap(),
                    fixture.0.join("original").to_str().unwrap(),
                    fixture.0.join("owned/worktree").to_str().unwrap(),
                    "Read exact changes",
                    "one.py",
                ],
            );
            assert_ne!(output["status"], 0, "{command} {path}: {output}");
            assert_eq!(output["prompts"], json!([]));
            assert_eq!(api.bind("events", &request).0, 200);
            assert_eq!(
                api.call("GET", "/v1/code/repository/apply", CONTROL, None)
                    .0,
                428
            );
            preserved(&fixture, b"before\n");
            assert_eq!(
                fs::read_dir(fixture.0.join("original/records"))
                    .unwrap()
                    .count(),
                0
            );
            assert_eq!(
                fs::read_dir(fixture.0.join("owned/worktree/records"))
                    .unwrap()
                    .count(),
                0
            );
        }
    }
    drop(api);
    assert!(matches!(upstream.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
}
