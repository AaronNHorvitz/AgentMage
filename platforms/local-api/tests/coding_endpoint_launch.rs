// SPDX-License-Identifier: Apache-2.0
//! Executable native-admission refusal, not a successful worker/model workflow.
#[path = "support/endpoint_launch.rs"]
mod fixture;
use agentmage_kernel_contracts::RuntimeEndpointRunRequest;
use agentmage_kernel_engine::runtime_coordinator::seal_runtime_endpoint_run_request;
use agentmage_local_api::{ApiError, ApiTokens, serve_coding_endpoint_read_development};
use fixture::Fixture;
use std::{
    fs,
    net::TcpListener,
    process::{Command, Stdio},
    sync::{Arc, atomic::AtomicBool},
};

#[test]
fn executable_endpoint_launch_requires_explicit_consent_before_reading_configuration() {
    let result = Command::new(env!("CARGO_BIN_EXE_agentmage-local-api"))
        .args([
            "serve-endpoint-read-development",
            "127.0.0.1:0",
            "/missing-auth",
            "/missing-launch",
        ])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8(result.stderr).unwrap().contains("Usage:"));
    assert!(result.stdout.is_empty());
}

#[test]
fn executable_endpoint_launch_reaches_original_native_refusal_without_model_or_state_effects() {
    let mut f = Fixture::new();
    let model = TcpListener::bind("127.0.0.1:0").unwrap();
    model.set_nonblocking(true).unwrap();
    let mut request: RuntimeEndpointRunRequest =
        serde_json::from_value(f.launch["request"].clone()).unwrap();
    request.model_selection.endpoint = format!("http://{}/v1", model.local_addr().unwrap());
    f.launch["request"] =
        serde_json::to_value(seal_runtime_endpoint_run_request(request).unwrap()).unwrap();
    // If opened, this absent credential would fail before the native diagnostic.
    f.launch["credential_file"] = serde_json::json!(f.root.join("must-not-read"));
    let launch = f.write();
    let auth = f.root.join("auth");
    ApiTokens::initialize(&auth).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_agentmage-local-api"))
        .args(["serve-endpoint-read-development", "127.0.0.1:0"])
        .arg(&auth)
        .arg(&launch)
        .arg("--approve-workspace-state-and-endpoint")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    let stderr = String::from_utf8(result.stderr).unwrap();
    assert!(
        stderr.contains("code.endpoint.native-admission: linux.sandbox.manifest.invalid"),
        "{stderr}"
    );
    assert!(!stderr.contains("must-not-read"));
    assert_eq!(
        model.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert_eq!(fs::read_dir(f.root.join("state")).unwrap().count(), 0);
    assert_eq!(
        fs::read(f.root.join("disposable/workspace/sentinel")).unwrap(),
        b"synthetic source\n"
    );
}

#[test]
fn endpoint_launch_listener_and_stop_checks_precede_private_configuration_access() {
    for (address, stopping) in [("0.0.0.0:0", false), ("127.0.0.1:0", true)] {
        let listener = TcpListener::bind(address).unwrap();
        let tokens = ApiTokens::new("a".repeat(64), "b".repeat(64)).unwrap();
        assert_eq!(
            serve_coding_endpoint_read_development(
                listener,
                tokens,
                Arc::new(AtomicBool::new(stopping)),
                std::path::Path::new("/missing-launch")
            )
            .err(),
            Some(ApiError::Configuration)
        );
    }
}

#[test]
fn executable_sealed_sibling_selection_refuses_an_arbitrary_worker_before_effects() {
    let mut f = Fixture::new();
    let model = TcpListener::bind("127.0.0.1:0").unwrap();
    model.set_nonblocking(true).unwrap();
    let mut request: RuntimeEndpointRunRequest =
        serde_json::from_value(f.launch["request"].clone()).unwrap();
    request.model_selection.endpoint = format!("http://{}/v1", model.local_addr().unwrap());
    f.launch["request"] =
        serde_json::to_value(seal_runtime_endpoint_run_request(request).unwrap()).unwrap();
    f.launch["worker_trust"] = serde_json::json!("sealed_sibling_development");
    // Its basename matches, but the launch file cannot select this unrelated
    // path as a development trust root. No root-owned fallback is permitted.
    f.launch["credential_file"] = serde_json::json!(f.root.join("must-not-read"));
    let launch = f.write();
    let auth = f.root.join("auth");
    ApiTokens::initialize(&auth).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_agentmage-local-api"))
        .args(["serve-endpoint-read-development", "127.0.0.1:0"])
        .arg(&auth)
        .arg(&launch)
        .arg("--approve-workspace-state-and-endpoint")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    let stderr = String::from_utf8(result.stderr).unwrap();
    assert!(
        stderr.contains("code.endpoint.native-admission: linux.sandbox.manifest.invalid"),
        "{stderr}"
    );
    assert!(!stderr.contains("must-not-read"));
    assert!(result.stdout.is_empty());
    assert_eq!(
        model.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert_eq!(fs::read_dir(f.root.join("state")).unwrap().count(), 0);
    assert_eq!(
        fs::read(f.root.join("agentmage-read-only-worker")).unwrap(),
        b"synthetic refused worker\n"
    );
    assert_eq!(
        fs::read(f.root.join("disposable/workspace/sentinel")).unwrap(),
        b"synthetic source\n"
    );
}
