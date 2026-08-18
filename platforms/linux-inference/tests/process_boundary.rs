#![forbid(unsafe_code)]

use std::process::{Command, Stdio};

use agentmage_platform_linux_inference::{BOUNDARY_DESCRIPTION, MODEL_INSTALLER_SELF_CHECK};
use serde_json::{Value, json};

const DOCKER_GUARD_SELF_CHECK: &[u8] = b"{\"accepted_operations\":[\"serve-one-session\",\"self-check\"],\"authority\":\"guarded-inference-transport-only\",\"component_id\":\"agentmage-docker-guard\",\"docker_control\":false,\"enabled\":false,\"network_egress\":false,\"protocol_version\":1,\"raw_target\":\"private-namespace-loopback-only\",\"sessions\":1}\n";
const DOCKER_COLLECTOR_SELF_CHECK: &[u8] = b"{\"accepted_operations\":[\"observe\",\"self-check\",\"validate-observation-stdin\"],\"authority\":\"docker-topology-observation-only\",\"component_id\":\"agentmage-docker-topology-collector\",\"docker_mutation\":false,\"enabled\":false,\"network_egress\":false,\"preflight_contract_version\":3,\"protocol_version\":2}\n";

#[test]
fn packaged_process_self_check_is_exact_and_inactive() {
    let output = Command::new(env!("CARGO_BIN_EXE_agentmage-native-inference"))
        .arg("--self-check")
        .stdin(Stdio::null())
        .output()
        .expect("adapter executes");
    assert!(output.status.success());
    assert_eq!(output.stdout, BOUNDARY_DESCRIPTION);
    assert!(output.stderr.is_empty());
}

#[test]
fn packaged_process_refuses_inference_and_ambient_arguments() {
    for arguments in [
        vec![],
        vec!["--serve"],
        vec!["--model", "/tmp/model.gguf"],
        vec!["--self-check", "extra"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_agentmage-native-inference"))
            .args(arguments)
            .stdin(Stdio::null())
            .output()
            .expect("adapter executes");
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert_eq!(
            output.stderr,
            b"agentmage.native-inference.operation-unavailable-no-admitted-profile\n"
        );
    }
}

#[test]
fn packaged_model_installer_is_one_shot_inactive_and_content_free() {
    let output = Command::new(env!("CARGO_BIN_EXE_agentmage-model-installer"))
        .arg("--self-check")
        .env("AGENTMAGE_TEST_SECRET_CANARY", "must-not-appear")
        .stdin(Stdio::null())
        .output()
        .expect("installer executes and exits");
    assert!(output.status.success());
    assert_eq!(output.stdout, MODEL_INSTALLER_SELF_CHECK);
    assert!(output.stderr.is_empty());
    assert!(
        !output
            .stdout
            .windows(15)
            .any(|value| value == b"must-not-appear")
    );
}

#[test]
fn packaged_model_installer_preflight_is_exact_and_non_acquiring() {
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
        .expect("hardware envelope");
    let artifact_bytes = profile["artifact"]["bytes"]
        .as_u64()
        .expect("artifact bytes");
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
    let mut child = Command::new(env!("CARGO_BIN_EXE_agentmage-model-installer"))
        .arg("--preflight-stdin")
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("installer starts");
    serde_json::to_writer(child.stdin.as_mut().expect("stdin"), &request).expect("request");
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("installer exits");
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let response: Value = serde_json::from_slice(&output.stdout).expect("response");
    assert_eq!(response["operation"], "preflight");
    assert_eq!(response["preflight"]["disposition"], "eligible");
    assert_eq!(response["preflight"]["source_opened"], false);
    assert_eq!(response["preflight"]["destination_changed"], false);
}

#[test]
fn packaged_model_installer_refuses_lifecycle_and_ambient_arguments() {
    for arguments in [
        vec![],
        vec!["--install"],
        vec!["--source", "/tmp/model.gguf"],
        vec!["--self-check", "extra"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_agentmage-model-installer"))
            .args(arguments)
            .stdin(Stdio::null())
            .output()
            .expect("installer executes and exits");
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert_eq!(output.stderr, b"model.installer.operation-unavailable\n");
    }
}

#[test]
fn packaged_docker_guard_self_check_is_exact_and_inactive() {
    let output = Command::new(env!("CARGO_BIN_EXE_agentmage-docker-guard"))
        .arg("--self-check")
        .stdin(Stdio::null())
        .output()
        .expect("guard executes");
    assert!(output.status.success());
    assert_eq!(output.stdout, DOCKER_GUARD_SELF_CHECK);
    assert!(output.stderr.is_empty());
}

#[test]
fn packaged_docker_guard_refuses_arguments_and_incomplete_bootstrap() {
    for arguments in [vec![], vec!["--serve", "extra"], vec!["--socket", "/tmp/x"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_agentmage-docker-guard"))
            .args(arguments)
            .stdin(Stdio::null())
            .output()
            .expect("guard executes");
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert_eq!(output.stderr, b"docker-guard.service.bootstrap\n");
    }
    let output = Command::new(env!("CARGO_BIN_EXE_agentmage-docker-guard"))
        .arg("--serve")
        .stdin(Stdio::null())
        .output()
        .expect("guard executes");
    assert!(!output.status.success());
    assert_eq!(output.stderr, b"docker-guard.service.bootstrap\n");
}

#[test]
fn packaged_docker_collector_self_check_is_exact_and_inactive() {
    let output = Command::new(env!("CARGO_BIN_EXE_agentmage-docker-topology-collector"))
        .arg("--self-check")
        .stdin(Stdio::null())
        .output()
        .expect("collector executes");
    assert!(output.status.success());
    assert_eq!(output.stdout, DOCKER_COLLECTOR_SELF_CHECK);
    assert!(output.stderr.is_empty());
}

#[test]
fn packaged_docker_collector_refuses_arguments_and_empty_input() {
    for arguments in [
        vec![],
        vec!["--validate-observation-stdin", "extra"],
        vec!["--observe", "extra"],
        vec!["--docker", "x"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_agentmage-docker-topology-collector"))
            .args(arguments)
            .stdin(Stdio::null())
            .output()
            .expect("collector executes");
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert_eq!(output.stderr, b"docker-collector.input\n");
    }
    let output = Command::new(env!("CARGO_BIN_EXE_agentmage-docker-topology-collector"))
        .arg("--validate-observation-stdin")
        .stdin(Stdio::null())
        .output()
        .expect("collector executes");
    assert!(!output.status.success());
    assert_eq!(output.stderr, b"docker-collector.administrator-identity\n");

    let output = Command::new(env!("CARGO_BIN_EXE_agentmage-docker-topology-collector"))
        .arg("--observe")
        .stdin(Stdio::null())
        .output()
        .expect("collector executes");
    assert!(!output.status.success());
    assert_eq!(output.stderr, b"docker-collector.administrator-identity\n");
}
