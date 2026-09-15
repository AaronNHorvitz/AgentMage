#![forbid(unsafe_code)]

//! Observes the packaged installer process while it runs and after it is
//! terminated to prove the one-shot boundary never opens sockets, never
//! retains process-visible network state, and closes its stdio pipes with
//! bounded residue when interrupted at any input phase.

use std::fs;
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

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

fn socket_fd_targets(pid: u32) -> Vec<String> {
    let fd_directory = format!("/proc/{pid}/fd");
    let mut targets = Vec::new();
    let Ok(entries) = fs::read_dir(&fd_directory) else {
        return targets;
    };
    for entry in entries.flatten() {
        if let Ok(target) = fs::read_link(entry.path()) {
            let text = target.to_string_lossy().into_owned();
            if text.starts_with("socket:") || text.starts_with("anon_inode:[socket]") {
                targets.push(text);
            }
        }
    }
    targets
}

fn spawn_installer(argument: &str, stdin_mode: Stdio) -> Child {
    Command::new(env!("CARGO_BIN_EXE_agentmage-model-installer"))
        .arg(argument)
        .env_clear()
        .stdin(stdin_mode)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("installer spawns")
}

fn sample_sockets_until_exit(child: &mut Child) -> Vec<String> {
    let pid = child.id();
    let deadline = Instant::now() + Duration::from_millis(500);
    let mut observed: Vec<String> = Vec::new();
    loop {
        for target in socket_fd_targets(pid) {
            if !observed.contains(&target) {
                observed.push(target);
            }
        }
        if child
            .try_wait()
            .expect("child status readable")
            .is_some()
        {
            break;
        }
        if Instant::now() > deadline {
            break;
        }
        thread::sleep(Duration::from_millis(2));
    }
    observed
}

#[test]
fn self_check_boundary_never_opens_sockets() {
    let mut child = spawn_installer("--self-check", Stdio::null());
    let observed = sample_sockets_until_exit(&mut child);
    let output = child.wait_with_output().expect("reap");
    assert!(output.status.success());
    assert_eq!(output.stdout, MODEL_INSTALLER_SELF_CHECK);
    assert!(output.stderr.is_empty());
    assert!(
        observed.is_empty(),
        "self-check must not open transport sockets: {observed:?}"
    );
}

#[test]
fn preflight_boundary_never_opens_sockets() {
    let bytes = preflight_request_bytes();
    let mut child = spawn_installer("--preflight-stdin", Stdio::piped());
    let pre_write = socket_fd_targets(child.id());
    {
        let mut stdin = child.stdin.take().expect("stdin");
        stdin.write_all(&bytes).expect("write request");
    }
    let observed = sample_sockets_until_exit(&mut child);
    let output = child.wait_with_output().expect("reap");
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let response: Value = serde_json::from_slice(&output.stdout).expect("response");
    assert_eq!(response["operation"], "preflight");
    assert_eq!(response["preflight"]["source_opened"], false);
    assert_eq!(response["preflight"]["destination_changed"], false);
    assert!(
        pre_write.is_empty(),
        "preflight must not open sockets before reading input: {pre_write:?}"
    );
    assert!(
        observed.is_empty(),
        "preflight must not open transport sockets: {observed:?}"
    );
}

#[test]
fn terminated_preflight_closes_pipes_and_never_opens_sockets() {
    let bytes = preflight_request_bytes();
    let mut child = spawn_installer("--preflight-stdin", Stdio::piped());
    {
        let mut stdin = child.stdin.take().expect("stdin");
        stdin.write_all(&bytes[..bytes.len() / 2]).expect("partial");
    }
    // Observe once while the process is still alive to prove no sockets exist
    // even mid-read; then terminate and confirm pipes drain empty.
    let mid = socket_fd_targets(child.id());
    child.kill().expect("terminate");
    let output = child.wait_with_output().expect("reap after termination");
    assert!(!output.status.success());
    assert!(
        output.stdout.is_empty(),
        "terminated preflight must not commit stdout bytes"
    );
    assert!(
        output.stderr.is_empty(),
        "terminated preflight must not commit stderr bytes"
    );
    assert!(
        mid.is_empty(),
        "terminated preflight must not have opened transport sockets: {mid:?}"
    );
}

#[test]
fn refused_arguments_never_open_sockets_and_close_stdio_cleanly() {
    for argument in [
        "--download",
        "--import",
        "--stage",
        "--verify",
        "--activate",
        "--rollback",
        "--cleanup",
    ] {
        let mut child = spawn_installer(argument, Stdio::null());
        let observed = sample_sockets_until_exit(&mut child);
        let output = child.wait_with_output().expect("reap");
        assert!(!output.status.success(), "{argument} must fail closed");
        assert!(
            output.stdout.is_empty(),
            "{argument} must not commit stdout"
        );
        assert_eq!(
            output.stderr, b"model.installer.operation-unavailable\n",
            "{argument} must emit the stable refusal code"
        );
        assert!(
            observed.is_empty(),
            "{argument} must not open transport sockets: {observed:?}"
        );
    }
}

#[test]
fn back_to_back_terminated_and_committed_runs_share_no_transport_state() {
    for _ in 0..4 {
        let mut child = spawn_installer("--preflight-stdin", Stdio::piped());
        drop(child.stdin.take());
        let output = child.wait_with_output().expect("reap");
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert_eq!(output.stderr, b"model.installer.input-invalid\n");
    }
    // A committed preflight after a burst of closed-stdin refusals must still
    // emit an exact, socket-free response.
    let bytes = preflight_request_bytes();
    let mut child = spawn_installer("--preflight-stdin", Stdio::piped());
    {
        let mut stdin = child.stdin.take().expect("stdin");
        stdin.write_all(&bytes).expect("write");
    }
    let observed = sample_sockets_until_exit(&mut child);
    let output = child.wait_with_output().expect("reap");
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let response: Value = serde_json::from_slice(&output.stdout).expect("response");
    assert_eq!(response["preflight"]["disposition"], "eligible");
    assert!(
        observed.is_empty(),
        "committed preflight after refusals must not open sockets: {observed:?}"
    );
}
