#![forbid(unsafe_code)]

use std::process::{Command, Stdio};

use agentmage_platform_linux_inference::BOUNDARY_DESCRIPTION;

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
