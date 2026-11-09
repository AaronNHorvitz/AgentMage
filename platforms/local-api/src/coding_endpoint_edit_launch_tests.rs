// SPDX-License-Identifier: Apache-2.0
//! Synthetic launch metadata and actual native refusal; no model/task score.
use super::*;
use crate::endpoint_launch_fixture as fixture;
use agentmage_kernel_contracts::*;
use agentmage_kernel_engine::{
    instruction_provenance::build_instruction_ledger,
    runtime_coordinator::{runtime_tool_catalog_sha256, seal_runtime_endpoint_run_request},
    runtime_loop::runtime_tool_references,
};
use fixture::{Fixture, private_file};
use std::{collections::BTreeMap, fs};

fn launch(f: &Fixture) -> Value {
    let owned = f.root.join("owned");
    if !owned.exists() {
        fs::create_dir(&owned).unwrap();
        fs::set_permissions(&owned, std::os::unix::fs::PermissionsExt::from_mode(0o700)).unwrap();
    }
    let mut request: RuntimeEndpointRunRequest =
        serde_json::from_value(f.launch["request"].clone()).unwrap();
    request.mode = RuntimeSessionMode::ControlledWrite;
    request.workspace_id = WorkspaceId::from_raw("local-coding-worktree-create");
    request.work_packet.mutable_files = vec!["sentinel".into()];
    request.work_packet.protected_files.clear();
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
    json!({"version":1,"state_root":f.root.join("state"),"repository_root":f.root.join("disposable/workspace"),
        "owned_root":f.root.join("owned"),"base_commit":"a".repeat(40),"credential_file":null,
        "request":request,"instructions":instructions,"paths":["sentinel"],"source_budget":4096})
}
fn write(f: &Fixture, value: &Value) -> PathBuf {
    let path = f.root.join("edit.json");
    private_file(&path, &serde_json::to_vec(value).unwrap());
    path
}
fn prepared(value: Value) -> Result<CodingEndpointPreparedStartup, ApiError> {
    let launch = serde_json::from_value(value).unwrap();
    prepare(
        &launch,
        RuntimeRunId::from_raw("fresh-edit-run"),
        SessionId::from_raw("fresh-edit-session"),
    )
}

#[test]
fn edit_launch_prepares_original_narrowed_request_without_effects_or_sources() {
    let f = Fixture::new();
    let value = launch(&f);
    let input = load(&write(&f, &value)).unwrap();
    let startup = prepare(
        &input,
        RuntimeRunId::from_raw("fresh-edit-run"),
        SessionId::from_raw("fresh-edit-session"),
    )
    .unwrap();
    assert_eq!(startup.request().run_id.as_str(), "fresh-edit-run");
    assert_eq!(startup.request().session_id.as_str(), "fresh-edit-session");
    assert_eq!(startup.request().work_packet.mutable_files, ["sentinel"]);
    assert_ne!(
        startup.request().request_sha256,
        input.request.request_sha256
    );
    assert!(
        startup
            .request()
            .task
            .constraints
            .last()
            .unwrap()
            .starts_with("Trusted repository narrowing policy SHA-256: ")
    );
    assert!(startup.request().model_selection == input.request.model_selection);
    assert_eq!(fs::read_dir(f.root.join("state")).unwrap().count(), 0);
    assert_eq!(fs::read_dir(f.root.join("owned")).unwrap().count(), 0);
    assert_eq!(
        fs::read(f.root.join("disposable/workspace/sentinel")).unwrap(),
        b"synthetic source\n"
    );
}

#[test]
fn edit_launch_refuses_foreign_or_unusable_metadata_before_native_inspection() {
    let f = Fixture::new();
    let value = launch(&f);
    for (name, invalid) in [
        ("paths", json!([])),
        ("paths", json!(["../sentinel"])),
        ("paths", json!(["sentinel", "sentinel"])),
        ("source_budget", json!(0)),
        ("source_budget", json!(32769)),
    ] {
        let mut bad = value.clone();
        bad[name] = invalid;
        assert!(prepared(bad).is_err());
    }
    for mutation in [0, 1, 2, 3] {
        let mut bad = value.clone();
        let mut request: RuntimeEndpointRunRequest =
            serde_json::from_value(bad["request"].clone()).unwrap();
        match mutation {
            0 => request.workspace_id = WorkspaceId::from_raw("foreign-owner-label"),
            1 => request.mode = RuntimeSessionMode::EphemeralReadOnly,
            2 => request.model_selection.endpoint = "https://remote.invalid/v1".into(),
            _ => request.visible_tools.clear(),
        }
        match seal_runtime_endpoint_run_request(request) {
            Ok(request) => {
                bad["request"] = json!(request);
                assert!(prepared(bad).is_err());
            }
            Err(_) => assert_eq!(mutation, 3, "only the empty catalog fails sealing"),
        }
    }
    let mut bad = value.clone();
    bad["request"]["request_sha256"] = json!("0".repeat(64));
    assert!(prepared(bad).is_err());
    let mut bad = value;
    bad["instructions"]["workspace_manifest_sha256"] = json!("0".repeat(64));
    assert!(prepared(bad).is_err());
    assert_eq!(fs::read_dir(f.root.join("state")).unwrap().count(), 0);
}

#[test]
fn edit_launch_private_closed_schema_reuses_original_no_alias_reader() {
    let f = Fixture::new();
    let value = launch(&f);
    assert!(
        load(&f.write()).is_err(),
        "read-only launch cannot activate edits"
    );
    let path = write(&f, &value);
    assert!(load(&path).is_ok());
    for (name, invalid) in [
        ("version", json!(2)),
        ("unknown", json!(true)),
        ("instructions", json!([])),
        ("request", json!([])),
    ] {
        let mut bad = value.clone();
        bad[name] = invalid;
        assert!(load(&write(&f, &bad)).is_err());
    }
    write(&f, &value);
    std::os::unix::fs::symlink(&path, f.root.join("alias")).unwrap();
    assert!(load(&f.root.join("alias")).is_err());
    fs::set_permissions(&path, std::os::unix::fs::PermissionsExt::from_mode(0o644)).unwrap();
    assert!(load(&path).is_err());
}

#[test]
fn edit_creation_decision_requires_exact_digest_newline_and_bounded_input() {
    let digest = "a".repeat(64);
    assert_eq!(
        creation_decision(&mut format!("approve {digest}\n").as_bytes(), &digest),
        Ok(true)
    );
    assert_eq!(
        creation_decision(&mut b"deny\n".as_slice(), &digest),
        Ok(false)
    );
    for input in [
        format!("approve {}\n", "b".repeat(64)),
        "approve\n".into(),
        "yes\n".into(),
        format!("approve {digest}"),
        format!("approve {digest}\r\n"),
        format!("{}\n", "x".repeat(1000)),
        String::new(),
    ] {
        assert!(creation_decision(&mut input.as_bytes(), &digest).is_err());
    }
    assert!(creation_decision(&mut b"deny\n".as_slice(), "invalid").is_err());
}

#[test]
fn edit_launch_control_retains_original_deadline_and_shutdown() {
    let stop = Arc::new(AtomicBool::new(false));
    let control = RunControl {
        stop: stop.clone(),
        deadline: ProposalDispatchControl {
            started: Instant::now(),
            total: Duration::from_secs(1),
        },
    };
    assert!(control.remaining_ms().unwrap().get() <= 1000);
    stop.store(true, Ordering::Release);
    assert_eq!(control.remaining_ms(), Err(ModelOperationStop::Cancelled));
    stop.store(false, Ordering::Release);
    let expired = RunControl {
        stop,
        deadline: ProposalDispatchControl {
            started: Instant::now() - Duration::from_secs(2),
            total: Duration::from_secs(1),
        },
    };
    assert_eq!(expired.remaining_ms(), Err(ModelOperationStop::TimedOut));
}

#[test]
fn edit_creation_terminal_io_observes_shutdown_expiry_and_restores_flags() {
    use std::os::unix::net::UnixStream;
    let digest = "a".repeat(64);
    for expiry in [false, true] {
        let (input, _peer) = UnixStream::pair().unwrap();
        let observer = input.try_clone().unwrap();
        let flags = rustix::fs::fcntl_getfl(&observer).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let mut reader = ApprovalInput::new(input, stop.clone()).unwrap();
        assert!(
            rustix::fs::fcntl_getfl(&observer)
                .unwrap()
                .contains(OFlags::NONBLOCK)
        );
        if expiry {
            reader.deadline = Instant::now();
        } else {
            stop.store(true, Ordering::Release);
        }
        assert!(creation_decision(&mut reader, &digest).is_err());
        drop(reader);
        assert_eq!(rustix::fs::fcntl_getfl(&observer).unwrap(), flags);
    }
    let (input, mut peer) = UnixStream::pair().unwrap();
    let observer = input.try_clone().unwrap();
    let flags = rustix::fs::fcntl_getfl(&observer).unwrap();
    peer.write_all(format!("approve {digest}\n").as_bytes())
        .unwrap();
    let mut reader = ApprovalInput::new(input, Arc::new(AtomicBool::new(false))).unwrap();
    assert_eq!(creation_decision(&mut reader, &digest), Ok(true));
    drop(reader);
    assert_eq!(rustix::fs::fcntl_getfl(&observer).unwrap(), flags);
}

#[test]
fn edit_launch_refuses_overlapping_or_aliased_roots_before_state_open() {
    let f = Fixture::new();
    let value = launch(&f);
    for field in ["state_root", "owned_root"] {
        let mut bad = value.clone();
        bad[field] = bad["repository_root"].clone();
        assert!(prepared(bad).is_err());
    }
    let alias = f.root.join("aliased-repository");
    std::os::unix::fs::symlink(f.root.join("disposable/workspace"), &alias).unwrap();
    let mut bad = value;
    bad["repository_root"] = json!(alias);
    assert!(prepared(bad).is_err());
    assert_eq!(fs::read_dir(f.root.join("state")).unwrap().count(), 0);
}
