#![forbid(unsafe_code)]

//! Exercises actual process termination of the packaged one-shot installer at
//! every input, drain, and refusal boundary and asserts bounded residue,
//! content-free stdout, and stable follow-up self-check equivalence.

use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use agentmage_platform_linux_inference::MODEL_INSTALLER_SELF_CHECK;
use serde_json::{Value, json};

fn preflight_request_bytes() -> Vec<u8> {
    let catalog: Value = serde_json::from_str(include_str!(
        "../../../model-profiles/exact-profile-catalog.json"
    ))
    .expect("catalog");
    let profile = catalog["profiles"]
        .as_array()
        .expect("profiles")
        .iter()
        .find(|value| {
            value["profile_id"] == "muse-glimmer-30b-q4-k-m-text-8k-fedora-diagnostic-repeatability"
        })
        .expect("profile")
        .clone();
    let envelope = profile["hardware"]
        .as_array()
        .expect("hardware")
        .iter()
        .find(|value| {
            value["platform"] == profile["runtime"]["platform"]
                && value["architecture"] == profile["runtime"]["architecture"]
        })
        .expect("envelope");
    let artifact_bytes = profile["artifact"]["bytes"].as_u64().expect("artifact bytes");
    let request = json!({
        "schema_version": 1,
        "host": {
            "platform": profile["runtime"]["platform"],
            "architecture": profile["runtime"]["architecture"],
            "system_memory_bytes": envelope["minimum_system_memory_bytes"],
            "accelerator_memory_bytes": envelope["minimum_accelerator_memory_bytes"],
            "model_store_available_bytes": artifact_bytes.saturating_mul(2).saturating_add(1 << 30),
            "requested_context_tokens": 8_192,
            "runtime": profile["runtime"],
        },
        "profile": profile,
    });
    serde_json::to_vec(&request).expect("request bytes")
}

fn spawn_preflight() -> Child {
    Command::new(env!("CARGO_BIN_EXE_agentmage-model-installer"))
        .arg("--preflight-stdin")
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("installer spawns")
}

fn exact_self_check() {
    let follow_up = Command::new(env!("CARGO_BIN_EXE_agentmage-model-installer"))
        .arg("--self-check")
        .env_clear()
        .stdin(Stdio::null())
        .output()
        .expect("self-check runs");
    assert!(follow_up.status.success());
    assert_eq!(follow_up.stdout, MODEL_INSTALLER_SELF_CHECK);
    assert!(follow_up.stderr.is_empty());
}

#[test]
fn termination_before_any_stdin_bytes_commits_no_output() {
    let mut child = spawn_preflight();
    child.kill().expect("terminate");
    let output = child.wait_with_output().expect("reap");
    assert!(!output.status.success(), "signal-terminated must not succeed");
    assert!(
        output.stdout.is_empty(),
        "no preflight bytes may be committed before termination"
    );
    assert!(
        output.stderr.is_empty(),
        "no diagnostic bytes may be committed before termination"
    );
    exact_self_check();
}

#[test]
fn termination_after_partial_stdin_bytes_commits_no_output() {
    let bytes = preflight_request_bytes();
    let split = bytes.len() / 3;
    let mut child = spawn_preflight();
    {
        let mut stdin = child.stdin.take().expect("stdin");
        stdin.write_all(&bytes[..split]).expect("partial write");
    }
    thread::sleep(Duration::from_millis(20));
    child.kill().expect("terminate");
    let output = child.wait_with_output().expect("reap");
    assert!(!output.status.success());
    assert!(
        output.stdout.is_empty(),
        "partial preflight input must not commit any response bytes"
    );
    assert!(output.stderr.is_empty());
    exact_self_check();
}

#[test]
fn termination_after_full_stdin_before_drain_commits_no_output() {
    let bytes = preflight_request_bytes();
    let mut child = spawn_preflight();
    {
        let mut stdin = child.stdin.take().expect("stdin");
        stdin.write_all(&bytes).expect("full write");
    }
    child.kill().expect("terminate before drain");
    let output = child.wait_with_output().expect("reap");
    assert!(!output.status.success());
    assert!(
        output.stdout.is_empty(),
        "response bytes must not appear when the child was killed before serialising"
    );
    assert!(output.stderr.is_empty());
    exact_self_check();
}

#[test]
fn closed_stdin_without_bytes_returns_bounded_content_free_error() {
    let mut child = spawn_preflight();
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("reap");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, b"model.installer.input-invalid\n");
    exact_self_check();
}

#[test]
fn repeated_terminations_leave_no_lingering_state() {
    for _ in 0..8 {
        let mut child = spawn_preflight();
        child.kill().expect("terminate");
        let output = child.wait_with_output().expect("reap");
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
    exact_self_check();
    let bytes = preflight_request_bytes();
    let mut child = spawn_preflight();
    {
        let mut stdin = child.stdin.take().expect("stdin");
        stdin.write_all(&bytes).expect("write");
    }
    let output = child.wait_with_output().expect("reap");
    assert!(output.status.success());
    let response: Value =
        serde_json::from_slice(&output.stdout).expect("response after termination cycles");
    assert_eq!(response["operation"], "preflight");
    assert_eq!(response["preflight"]["disposition"], "eligible");
    assert_eq!(response["preflight"]["source_opened"], false);
    assert_eq!(response["preflight"]["destination_changed"], false);
}

#[test]
fn refused_argument_shape_exits_with_stable_error_and_no_output() {
    for arguments in [
        vec![],
        vec!["--download"],
        vec!["--import"],
        vec!["--stage"],
        vec!["--verify"],
        vec!["--activate"],
        vec!["--rollback"],
        vec!["--cleanup"],
        vec!["--self-check", "extra"],
        vec!["--preflight-stdin", "extra"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_agentmage-model-installer"))
            .args(&arguments)
            .env_clear()
            .stdin(Stdio::null())
            .output()
            .expect("installer executes and exits");
        assert!(
            !output.status.success(),
            "arguments {arguments:?} must fail closed"
        );
        assert!(
            output.stdout.is_empty(),
            "arguments {arguments:?} must not commit stdout"
        );
        assert_eq!(
            output.stderr,
            b"model.installer.operation-unavailable\n",
            "arguments {arguments:?} must emit the stable content-free refusal"
        );
    }
    exact_self_check();
}
