// SPDX-License-Identifier: MIT OR Apache-2.0
//! Synthetic creation/model; real HTTP, native edits and original-owner return.
use super::*;

#[path = "coding_endpoint_original_review.rs"]
mod original_review;
use agentmage_host::coding_endpoint_edit_previews::{
    CodingEndpointAcceptancePort, CodingEndpointValidation,
};
use agentmage_kernel_engine::{
    command_runner::{CommandBounds, CommandRisk, CommandSpec, CommandWorkingDirectory},
    validation_template::ValidationKind,
};
use agentmage_platform_linux::LinuxCodingValidationEvidence;

struct MustNotCheck(VerifierId);
impl CodingEndpointAcceptancePort for MustNotCheck {
    fn verifier_id(&self) -> &VerifierId {
        &self.0
    }
    fn verify_coding_completion(
        &mut self,
        _: RuntimeCompletionVerificationInput<'_>,
        _: LinuxCodingValidationEvidence<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        panic!("cancelled or declined run cannot reach acceptance")
    }
}

fn checked_input(
    owner: LinuxCodingRepository,
    endpoint: LocalChatEndpoint,
    id: RuntimeRunId,
    session: SessionId,
) -> (
    agentmage_local_api::CodingEndpointStrictHttpRun<MustNotCheck, SharedClock>,
    CodingEndpointValidation,
) {
    let source = compose(id, session, endpoint, Arc::new(AtomicUsize::new(0)));
    let registry = agentmage_platform_linux::linux_coding_endpoint_coding_registry().unwrap();
    let mut request = source.request;
    request.mode = RuntimeSessionMode::ControlledWrite;
    request.workspace_id = WorkspaceId::from_raw("local-coding-worktree-create");
    request.work_packet.mutable_files = vec!["one.py".into()];
    request.work_packet.protected_files = vec!["private".into()];
    request.visible_tools = runtime_tool_references(&registry).unwrap();
    request.tool_catalog_sha256 =
        runtime_tool_catalog_sha256(&request.tool_catalog_id, &request.visible_tools).unwrap();
    let request = seal_runtime_endpoint_run_request(request).unwrap();
    let evidence = build_instruction_ledger(
        request.workspace_snapshot_sha256.clone(),
        vec![],
        vec![],
        vec![],
        &BTreeMap::new(),
    )
    .unwrap();
    let input = agentmage_local_api::CodingEndpointStrictHttpRun {
        startup: CodingEndpointPreparedStartup::prepare(
            request,
            CodingEndpointPreparedKind::EditAndTest,
            &evidence,
        )
        .unwrap(),
        repository: owner,
        connection: source.connection,
        control: Arc::new(Control),
        clock: SharedClock(Arc::new(AtomicUsize::new(0))),
        verifier: MustNotCheck(VerifierId::from_raw("synthetic-return-no-acceptance")),
        paths: vec!["one.py".into()],
        source_budget: 2048,
        include_usage: true,
    };
    let validation = CodingEndpointValidation {
        command: CommandSpec::seal(
            "fixed-return-test",
            "1.0.0",
            "/usr/bin/true",
            digest(&fs::read("/usr/bin/true").unwrap()),
            vec![],
            CommandWorkingDirectory::OwnedWorktree,
            Default::default(),
            CommandRisk::Moderate,
            CommandBounds::new(1000, 1024, 1024, 16 * 1024 * 1024, 1, 100).unwrap(),
        )
        .unwrap(),
        kind: ValidationKind::Unit,
        minimum_test_count: 1,
        output_format: Default::default(),
    };
    (input, validation)
}

#[test]
fn endpoint_strict_return_preserves_approved_lineage_and_abandons_denied_preview() {
    let fixture = Fixture::new();
    let owner = fixture.owner();
    let roots = owner.inspect_apply().unwrap();
    let (upstream, endpoint) = upstream();
    let peer = thread::spawn(move || {
        for (old, new) in [("before\n", "first\n"), ("first\n", "denied\n")] {
            let mut socket = accept(&upstream);
            let body = read_upstream(&mut socket);
            assert!(
                body["messages"][1]["content"]
                    .as_str()
                    .unwrap()
                    .contains(old.trim())
            );
            respond(
                &mut socket,
                &json!({"edits":[{"path":"one.py","old":old,"new":new}]}).to_string(),
            );
        }
        upstream
    });
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = stop.clone();
    let (tx, rx) = mpsc::sync_channel(1);
    let worker = thread::spawn(move || {
        let returned =
            agentmage_local_api::serve_coding_endpoint_strict_edits_returning_repository(
                listener,
                ApiTokens::new(CLIENT.into(), CONTROL.into()).unwrap(),
                worker_stop,
                CodingEndpointObservationAccess::DelegateObservations,
                |id, session| {
                    Ok(strict_input(
                        owner,
                        endpoint,
                        id,
                        session,
                        Arc::new(AtomicUsize::new(0)),
                    ))
                },
            )?;
        let _ = tx.send(returned);
        Ok(())
    });
    let api = Api {
        address,
        stop,
        worker: Some(worker),
    };
    let request = api.request();
    assert_eq!(
        api.bind("release", &request).1["error"],
        "code.endpoint.still-running"
    );
    let (code, first) = api.advance(&request, Value::Null, true);
    assert_eq!(code, 200);
    assert_eq!(first["state"], "awaiting_approval");
    let first_approval = approval(&first["challenge"], true);
    let (code, second) = api.advance(&request, first_approval.clone(), true);
    assert_eq!(code, 200);
    assert_eq!(second["state"], "awaiting_approval");
    fixture.assert_bytes(b"first\n");
    assert_eq!(api.advance(&request, first_approval, true).0, 409);
    assert_eq!(
        api.bind("release", &request).1["error"],
        "code.endpoint.still-running"
    );
    let (code, terminal) = api.advance(&request, approval(&second["challenge"], false), true);
    assert_eq!(code, 200);
    assert_eq!(terminal["outcome"]["outcome"]["state"], "DECLINED");
    assert_eq!(terminal["outcome"]["outcome"]["model_call_count"], 2);
    let events = api.bind("events", &request);
    assert_eq!(events.0, 200);
    assert_eq!(
        api.call(
            "POST",
            "/v1/code/endpoint/release",
            CLIENT,
            Some(json!({"run_id":request["run_id"],"request_sha256":request["request_sha256"]}))
        )
        .0,
        403
    );
    let mut foreign = request.clone();
    foreign["request_sha256"] = json!("0".repeat(64));
    assert_eq!(api.bind("release", &foreign).0, 409);
    assert_eq!(api.bind("events", &request), events);
    assert_eq!(
        api.bind("release", &request),
        (200, json!({"released":true}))
    );
    assert!(matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
    assert_eq!(
        api.bind("release", &request).1["error"],
        "code.endpoint.not-held"
    );
    drop(api); // all workers join before ownership leaves the service
    let mut owner = rx.recv_timeout(Duration::from_secs(1)).unwrap().unwrap();
    let edit = owner.inspect_edit().unwrap();
    assert!(edit["pending"].is_null());
    assert_eq!(edit["latest"]["outcome"], "denied");
    let apply = owner.inspect_apply().unwrap();
    assert_eq!(
        apply["repository_root_sha256"],
        roots["repository_root_sha256"]
    );
    assert_eq!(apply["worktree_root_sha256"], roots["worktree_root_sha256"]);
    assert_eq!(apply["available_paths"], json!(["one.py"]));
    let preview = owner
        .prepare_apply(
            &["one.py".into()],
            "Inspect retained strict-edit lineage",
            roots["repository_root_sha256"].as_str().unwrap(),
            roots["worktree_root_sha256"].as_str().unwrap(),
            2000,
        )
        .unwrap();
    assert!(
        preview["preview_sha256"]
            .as_str()
            .is_some_and(|s| s.len() == 64)
    );
    fixture.assert_bytes(b"first\n"); // diff inspection never approves original writes
    assert!(
        matches!(peer.join().unwrap().accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock)
    );
}

#[test]
fn endpoint_checked_return_requires_release_and_drops_original_verifier_references() {
    for case in [
        "stop",
        "cancel-no-release",
        "cancel-release",
        "deny-release",
        "ready-preview-release",
    ] {
        let fixture = Fixture::new();
        let mut owner = fixture.owner();
        if case == "ready-preview-release" {
            fixture.prepare(&mut owner, "unexecuted manual preview\n");
            assert!(!owner.inspect_edit().unwrap()["pending"].is_null());
        }
        let roots = owner.inspect_apply().unwrap();
        let (upstream, endpoint) = upstream();
        let peer = thread::spawn(move || {
            if case == "deny-release" {
                let mut socket = accept(&upstream);
                read_upstream(&mut socket);
                respond(
                    &mut socket,
                    &json!({"edits":[{"path":"one.py","old":"before\n","new":"denied\n"}]})
                        .to_string(),
                );
            }
            upstream
        });
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let (tx, rx) = mpsc::sync_channel(1);
        let worker = thread::spawn(move || {
            let result = agentmage_local_api::serve_coding_endpoint_edits_and_checked_tests_returning_repository(
                listener, ApiTokens::new(CLIENT.into(), CONTROL.into()).unwrap(), worker_stop,
                CodingEndpointObservationAccess::ControlOnly,
                |id, session| Ok(checked_input(owner, endpoint, id, session)),
            )?;
            let _ = tx.send(result);
            Ok(())
        });
        let api = Api {
            address,
            stop,
            worker: Some(worker),
        };
        let request = api.request();
        if case == "deny-release" {
            let (code, pending) = api.advance(&request, Value::Null, true);
            assert_eq!(code, 200);
            assert_eq!(pending["state"], "awaiting_approval");
            let terminal = api.advance(&request, approval(&pending["challenge"], false), true);
            assert_eq!(terminal.0, 200);
            assert_eq!(terminal.1["outcome"]["outcome"]["state"], "DECLINED");
        } else if case != "stop" {
            assert_eq!(api.bind("cancel", &request).0, 200);
            let terminal = api.advance(&request, Value::Null, false);
            assert_eq!(terminal.0, 200);
            assert_eq!(terminal.1["outcome"]["outcome"]["state"], "CANCELLED");
        }
        if case.ends_with("-release") && case != "cancel-no-release" {
            assert_eq!(api.bind("release", &request).0, 200);
        }
        assert!(matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
        drop(api);
        let returned = rx.recv_timeout(Duration::from_secs(1)).unwrap();
        if matches!(
            case,
            "cancel-release" | "deny-release" | "ready-preview-release"
        ) {
            let owner = returned.expect("original port and verifier references must be dropped");
            assert!(owner.inspect_edit().unwrap()["pending"].is_null());
            let apply = owner.inspect_apply().unwrap();
            assert_eq!(apply["worktree_root_sha256"], roots["worktree_root_sha256"]);
            assert_eq!(apply["available_paths"], json!([]));
        } else {
            assert!(returned.is_none());
        }
        fixture.assert_bytes(b"before\n");
        assert!(
            matches!(peer.join().unwrap().accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock)
        );
    }
}
