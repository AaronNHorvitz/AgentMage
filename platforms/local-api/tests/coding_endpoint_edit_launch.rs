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
use agentmage_local_api::{
    ApiError, ApiTokens, serve_coding_endpoint_coding_development,
    serve_coding_endpoint_coding_review_development, serve_coding_endpoint_edit_development,
    serve_coding_endpoint_edit_review_development,
};
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
    for command in [
        "serve-endpoint-edit-development",
        "serve-endpoint-coding-development",
        "serve-endpoint-edit-review-development",
        "serve-endpoint-coding-review-development",
    ] {
        for approval in [None, Some("--approve-workspace-state-and-endpoint")] {
            let mut cmd = Command::new(env!("CARGO_BIN_EXE_agentmage-local-api"));
            cmd.args([command, "127.0.0.1:0", "/missing-auth", "/missing-launch"]);
            if let Some(approval) = approval {
                cmd.arg(approval);
            }
            let result = cmd.stdin(Stdio::null()).output().unwrap();
            assert_eq!(result.status.code(), Some(1));
            assert!(String::from_utf8(result.stderr).unwrap().contains("Usage:"));
            assert!(result.stdout.is_empty());
        }
    }
}

#[test]
fn strict_edit_launch_refuses_listener_and_stop_before_private_configuration() {
    for serve in [
        serve_coding_endpoint_edit_development,
        serve_coding_endpoint_coding_development,
        serve_coding_endpoint_edit_review_development,
        serve_coding_endpoint_coding_review_development,
    ] {
        for (address, stopping) in [("0.0.0.0:0", false), ("127.0.0.1:0", true)] {
            assert_eq!(
                serve(
                    TcpListener::bind(address).unwrap(),
                    ApiTokens::new("a".repeat(64), "b".repeat(64)).unwrap(),
                    Arc::new(AtomicBool::new(stopping)),
                    std::path::Path::new("/missing-launch"),
                ),
                Err(ApiError::Configuration)
            );
        }
    }
}

#[test]
fn executable_strict_edit_launch_reaches_actual_native_refusal_before_credentials_or_model() {
    exercise_native_refusal(false, false);
}

#[test]
fn executable_coding_launch_reaches_actual_native_refusal_before_credentials_or_model() {
    exercise_native_refusal(true, false);
}

#[test]
fn executable_checked_coding_launch_preserves_original_native_refusal() {
    exercise_native_refusal(true, true);
}

fn exercise_native_refusal(coding: bool, checked: bool) {
    exercise_native_refusal_command(coding, checked, false, false);
    exercise_native_refusal_command(coding, checked, true, false);
}

#[test]
fn executable_observation_sharing_keeps_all_strict_launchers_native_refusals() {
    for (coding, checked) in [(false, false), (true, false), (true, true)] {
        for review in [false, true] {
            exercise_native_refusal_command(coding, checked, review, true);
        }
    }
}

fn exercise_native_refusal_command(coding: bool, checked: bool, review: bool, share: bool) {
    let command = if review && coding {
        "serve-endpoint-coding-review-development"
    } else if review {
        "serve-endpoint-edit-review-development"
    } else if coding {
        "serve-endpoint-coding-development"
    } else {
        "serve-endpoint-edit-development"
    };
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
    let registry = if coding {
        agentmage_platform_linux::linux_coding_endpoint_coding_registry()
    } else {
        agentmage_platform_linux::linux_coding_endpoint_edit_registry()
    }
    .unwrap();
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
    let mut value = json!({"version":1,"state_root":f.root.join("state"),"repository_root":f.root.join("disposable/workspace"),
        "owned_root":owned,"base_commit":"a".repeat(40),"credential_file":f.root.join("must-not-read"),
        "request":request,"instructions":instructions,"paths":["sentinel"],"source_budget":4096});
    if coding {
        use agentmage_kernel_engine::{
            command_runner::{CommandBounds, CommandRisk, CommandSpec, CommandWorkingDirectory},
            validation_template::ValidationParserKind,
        };
        // Synthetic executable digest; no test process can be admitted here.
        let spec = CommandSpec::seal(
            "fixed-test",
            "1.0.0",
            "/usr/bin/true",
            "a".repeat(64),
            vec![],
            CommandWorkingDirectory::OwnedWorktree,
            BTreeMap::new(),
            CommandRisk::Moderate,
            CommandBounds::new(1000, 1024, 1024, 128 * 1024 * 1024, 8, 100).unwrap(),
        )
        .unwrap();
        value["validation"] = json!({"command":spec,"kind":"unit","minimum_test_count":1,"output_format":ValidationParserKind::AgentMageJsonV1});
    }
    if checked {
        use agentmage_host::{
            coding_endpoint_edit_previews::CodingEndpointValidation,
            coding_endpoint_fixed_test_verifier::CodingEndpointFixedTestCheck,
        };
        value["validation"]["output_format"] = json!("python_unittest_text_v1");
        let definition = CodingEndpointValidation {
            command: serde_json::from_value(value["validation"]["command"].clone()).unwrap(),
            kind: agentmage_kernel_engine::validation_template::ValidationKind::Unit,
            minimum_test_count: 1,
            output_format: agentmage_platform_linux::LinuxCodingTestFormat::PythonUnittestTextV1,
        };
        let check = CodingEndpointFixedTestCheck::new(&definition).unwrap();
        let mut request: RuntimeEndpointRunRequest =
            serde_json::from_value(value["request"].clone()).unwrap();
        request.task.acceptance_criteria = vec![check.criterion()];
        request.work_packet.acceptance_checks = vec![check.criterion()];
        request.work_packet.required_capability_class = AuthorityClass::Execute;
        value["request"] = json!(seal_runtime_endpoint_run_request(request).unwrap());
        value["completion_check"] = json!("fixed_test_run_v1");
    }
    if share {
        value["share_run_observations"] = json!(true);
    }
    let launch = f.root.join("edit.json");
    private_file(&launch, &serde_json::to_vec(&value).unwrap());
    let auth = f.root.join("auth");
    ApiTokens::initialize(&auth).unwrap();
    let legacy = Command::new(env!("CARGO_BIN_EXE_agentmage-local-api"))
        .args([command, "127.0.0.1:0"])
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
    if coding {
        let mut invalid = value.clone();
        invalid["validation"]["minimum_test_count"] = json!(0);
        private_file(&launch, &serde_json::to_vec(&invalid).unwrap());
        let result = Command::new(env!("CARGO_BIN_EXE_agentmage-local-api"))
            .args([command, "127.0.0.1:0"])
            .arg(&auth)
            .arg(&launch)
            .arg("--approve-inspection-state-and-endpoint")
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(1));
        assert!(result.stdout.is_empty());
        assert!(
            !String::from_utf8(result.stderr)
                .unwrap()
                .contains("code.endpoint.edit-startup:")
        );
        assert_eq!(fs::read_dir(f.root.join("state")).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&owned).unwrap().count(), 0);
        assert_eq!(
            model.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        private_file(&launch, &serde_json::to_vec(&value).unwrap());
    }
    if checked {
        let mut invalid = value.clone();
        invalid["validation"]["output_format"] = json!("agent_mage_json_v1");
        private_file(&launch, &serde_json::to_vec(&invalid).unwrap());
        let result = Command::new(env!("CARGO_BIN_EXE_agentmage-local-api"))
            .args([command, "127.0.0.1:0"])
            .arg(&auth)
            .arg(&launch)
            .arg("--approve-inspection-state-and-endpoint")
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(1));
        assert!(result.stdout.is_empty());
        assert!(
            !String::from_utf8(result.stderr)
                .unwrap()
                .contains("code.endpoint.edit-startup:")
        );
        assert_eq!(fs::read_dir(f.root.join("state")).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&owned).unwrap().count(), 0);
        assert_eq!(
            model.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        private_file(&launch, &serde_json::to_vec(&value).unwrap());
    }
    let result = Command::new(env!("CARGO_BIN_EXE_agentmage-local-api"))
        .args([command, "127.0.0.1:0"])
        .arg(auth)
        .arg(launch)
        .arg("--approve-inspection-state-and-endpoint")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    let stderr = String::from_utf8(result.stderr).unwrap();
    assert_eq!(
        stderr.contains("Run observations shared with client.token holders:"),
        share
    );
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

#[test]
fn checked_service_refuses_listener_and_stop_before_calling_factory() {
    use agentmage_host::{
        coding_development_runtime::OsRuntimeClock,
        coding_endpoint_fixed_test_verifier::CodingEndpointFixedTestVerifier,
    };
    use agentmage_local_api::{
        CodingEndpointObservationAccess, serve_coding_endpoint_edits_and_checked_tests,
    };
    for (address, stopping) in [("0.0.0.0:0", false), ("127.0.0.1:0", true)] {
        assert_eq!(
            serve_coding_endpoint_edits_and_checked_tests::<
                CodingEndpointFixedTestVerifier,
                OsRuntimeClock,
            >(
                TcpListener::bind(address).unwrap(),
                ApiTokens::new("a".repeat(64), "b".repeat(64)).unwrap(),
                Arc::new(AtomicBool::new(stopping)),
                CodingEndpointObservationAccess::ControlOnly,
                |_, _| panic!("invalid listener/stop must precede checked factory"),
            ),
            Err(ApiError::Configuration)
        );
    }
}
