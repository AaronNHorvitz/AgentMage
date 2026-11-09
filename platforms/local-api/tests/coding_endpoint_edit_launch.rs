// SPDX-License-Identifier: Apache-2.0
//! Actual executable refusals; native Git trust is preserved, never substituted.
#[path = "support/endpoint_launch.rs"]
mod fixture;
use agentmage_kernel_contracts::*;
use agentmage_kernel_engine::{
    instruction_provenance::build_instruction_ledger,
    runtime_coordinator::{runtime_tool_catalog_sha256, seal_runtime_endpoint_run_request},
    runtime_loop::runtime_tool_references,
};
use agentmage_local_api::{ApiError, ApiTokens, serve_coding_endpoint_edit_development};
use fixture::{Fixture, private_file};
use serde_json::json;
use std::{
    collections::BTreeMap,
    fs,
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    process::{Command, Stdio},
    sync::{Arc, atomic::AtomicBool},
};

#[test]
fn executable_strict_edit_launch_requires_explicit_inspection_consent() {
    for approval in [None, Some("--approve-workspace-state-and-endpoint")] {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_agentmage-local-api"));
        cmd.args([
            "serve-endpoint-edit-development",
            "127.0.0.1:0",
            "/missing-auth",
            "/missing-launch",
        ]);
        if let Some(approval) = approval {
            cmd.arg(approval);
        }
        let result = cmd.stdin(Stdio::null()).output().unwrap();
        assert_eq!(result.status.code(), Some(1));
        assert!(String::from_utf8(result.stderr).unwrap().contains("Usage:"));
        assert!(result.stdout.is_empty());
    }
}

#[test]
fn strict_edit_launch_refuses_listener_and_stop_before_private_configuration() {
    for (address, stopping) in [("0.0.0.0:0", false), ("127.0.0.1:0", true)] {
        assert_eq!(
            serve_coding_endpoint_edit_development(
                TcpListener::bind(address).unwrap(),
                ApiTokens::new("a".repeat(64), "b".repeat(64)).unwrap(),
                Arc::new(AtomicBool::new(stopping)),
                std::path::Path::new("/missing-launch"),
            ),
            Err(ApiError::Configuration)
        );
    }
}

#[test]
fn executable_strict_edit_launch_reaches_actual_native_refusal_before_credentials_or_model() {
    let f = Fixture::new();
    let read_launch = f.write();
    let owned = f.root.join("owned");
    fs::create_dir(&owned).unwrap();
    fs::set_permissions(&owned, fs::Permissions::from_mode(0o700)).unwrap();
    let model = TcpListener::bind("127.0.0.1:0").unwrap();
    model.set_nonblocking(true).unwrap();
    let mut request: RuntimeEndpointRunRequest =
        serde_json::from_value(f.launch["request"].clone()).unwrap();
    request.mode = RuntimeSessionMode::ControlledWrite;
    request.workspace_id = WorkspaceId::from_raw("local-coding-worktree-create");
    request.work_packet.mutable_files = vec!["sentinel".into()];
    request.work_packet.protected_files.clear();
    request.model_selection.endpoint = format!("http://{}/v1", model.local_addr().unwrap());
    let registry = agentmage_platform_linux::linux_coding_endpoint_edit_registry().unwrap();
    request.visible_tools = runtime_tool_references(&registry).unwrap();
    request.tool_catalog_sha256 =
        runtime_tool_catalog_sha256(&request.tool_catalog_id, &request.visible_tools).unwrap();
    let request = seal_runtime_endpoint_run_request(request).unwrap();
    let instructions = build_instruction_ledger(
        request.workspace_snapshot_sha256.clone(),
        vec![],
        vec![],
        vec![],
        &BTreeMap::new(),
    )
    .unwrap();
    let value = json!({"version":1,"state_root":f.root.join("state"),"repository_root":f.root.join("disposable/workspace"),
        "owned_root":owned,"base_commit":"a".repeat(40),"credential_file":f.root.join("must-not-read"),
        "request":request,"instructions":instructions,"paths":["sentinel"],"source_budget":4096});
    let launch = f.root.join("edit.json");
    private_file(&launch, &serde_json::to_vec(&value).unwrap());
    let auth = f.root.join("auth");
    ApiTokens::initialize(&auth).unwrap();
    let legacy = Command::new(env!("CARGO_BIN_EXE_agentmage-local-api"))
        .args(["serve-endpoint-edit-development", "127.0.0.1:0"])
        .arg(&auth)
        .arg(read_launch)
        .arg("--approve-inspection-state-and-endpoint")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(legacy.status.code(), Some(1));
    assert!(legacy.stdout.is_empty());
    assert!(
        !String::from_utf8(legacy.stderr)
            .unwrap()
            .contains("code.endpoint.edit-startup:")
    );
    assert_eq!(fs::read_dir(f.root.join("state")).unwrap().count(), 0);
    let result = Command::new(env!("CARGO_BIN_EXE_agentmage-local-api"))
        .args(["serve-endpoint-edit-development", "127.0.0.1:0"])
        .arg(auth)
        .arg(launch)
        .arg("--approve-inspection-state-and-endpoint")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    let stderr = String::from_utf8(result.stderr).unwrap();
    assert!(
        stderr.contains("code.endpoint.edit-startup: linux.repository.git_artifact.invalid"),
        "{stderr}"
    );
    assert!(!stderr.contains("must-not-read"));
    assert!(
        result.stdout.is_empty(),
        "no creation preview or service has been admitted"
    );
    assert_eq!(
        model.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert_eq!(fs::read_dir(owned).unwrap().count(), 0);
    assert_eq!(
        fs::read(f.root.join("disposable/workspace/sentinel")).unwrap(),
        b"synthetic source\n"
    );
    assert_eq!(
        fs::read(f.root.join("disposable/workspace/.git/HEAD")).unwrap(),
        b"ref: refs/heads/main\n"
    );
    // Opening encrypted state was explicitly authorized. No native creation,
    // model run, exact approval or repository mutation succeeded in this lane.
}
