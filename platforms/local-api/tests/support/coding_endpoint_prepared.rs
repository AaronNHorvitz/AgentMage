// SPDX-License-Identifier: MIT OR Apache-2.0
//! Synthetic creation/endpoint, real native edits and original HTTP ownership.
use super::*;
use agentmage_host::coding_endpoint_prepared_startup::{
    CodingEndpointPreparedKind, CodingEndpointPreparedStartup,
};
use agentmage_kernel_engine::instruction_provenance::{
    GuidanceConstraint, GuidanceConstraintKind, InstructionDiscoveryInput,
    InstructionEvidenceLedger, InstructionLocation, InstructionScope, InstructionSourceKind,
    InstructionTrustDisposition, build_instruction_ledger, record_instruction_discovery,
    record_instruction_read, record_instruction_trust_decision,
};
use agentmage_local_api::{CodingEndpointPreparedHttpRun, serve_coding_endpoint_prepared};
use agentmage_platform_linux::{
    LinuxCodingReplacement, LinuxCodingRepository, linux_coding_endpoint_edit_registry,
};
use std::{collections::BTreeMap, fs, os::unix::fs::PermissionsExt, path::PathBuf};

#[path = "coding_endpoint_strict_return.rs"]
mod strict_return;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "agentmage-prepared-http-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        for dir in ["state", "original", "owned", "owned/worktree"] {
            fs::create_dir(root.join(dir)).unwrap();
            fs::set_permissions(root.join(dir), fs::Permissions::from_mode(0o700)).unwrap();
        }
        for dir in ["original", "owned/worktree"] {
            fs::write(root.join(dir).join("one.py"), "before\n").unwrap();
        }
        Self(root)
    }
    fn owner(&self) -> LinuxCodingRepository {
        LinuxCodingRepository::open(
            &self.0.join("state"),
            b"synthetic-prepared-http-state-secret",
            1000,
        )
        .unwrap()
        .with_test_created_worktree(&self.0.join("original"), &self.0.join("owned"))
        .unwrap()
    }
    fn prepare(&self, owner: &mut LinuxCodingRepository, content: &str) {
        let paths = vec!["one.py".into()];
        let status = owner.inspect_apply().unwrap();
        let scope = owner
            .consented_created_sources(status["worktree_root_sha256"].as_str().unwrap(), &paths)
            .unwrap();
        let sources = scope.read(&paths).unwrap();
        owner
            .prepare_edit(
                scope,
                &[LinuxCodingReplacement {
                    path: "one.py",
                    before_sha256: &digest(sources[0].content.as_bytes()),
                    content,
                }],
                "Synthetic prepared HTTP edit",
                1000,
            )
            .unwrap();
    }
    fn assert_bytes(&self, worktree: &[u8]) {
        assert_eq!(
            fs::read(self.0.join("original/one.py")).unwrap(),
            b"before\n"
        );
        assert_eq!(
            fs::read(self.0.join("owned/worktree/one.py")).unwrap(),
            worktree
        );
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let result = fs::remove_dir_all(&self.0);
        if !thread::panicking() {
            result.unwrap();
        }
    }
}

struct Control;
impl ModelOperationControl for Control {
    fn remaining_ms(&self) -> Result<std::num::NonZeroU64, ModelOperationStop> {
        Ok(std::num::NonZeroU64::new(60000).unwrap())
    }
}
struct SharedClock(Arc<AtomicUsize>);
impl RuntimeClock for SharedClock {
    fn now_epoch_ms(&mut self) -> Result<u64, RuntimePortFailure> {
        Ok(1000 + self.0.fetch_add(1, Ordering::SeqCst) as u64)
    }
}
fn derived(prefix: &str, run: &RuntimeRunId, sequence: u32) -> String {
    format!(
        "{prefix}:{}",
        &digest(format!("{}:{sequence}", run.as_str()).as_bytes())[..32]
    )
}

fn ledger(request: &RuntimeEndpointRunRequest) -> InstructionEvidenceLedger {
    let discovery = record_instruction_discovery(InstructionDiscoveryInput {
        discovery_id: "synthetic-prepared-discovery".into(),
        source_kind: InstructionSourceKind::RepositoryInstruction,
        location: InstructionLocation {
            workspace_path: Some(
                WorkspacePath::new(request.workspace_id.clone(), ["AGENTS.md"]).unwrap(),
            ),
            source_identity_sha256: "c".repeat(64),
            revision_sha256: "d".repeat(64),
            line_start: Some(1),
            line_end: Some(1),
        },
        workspace_manifest_sha256: request.workspace_snapshot_sha256.clone(),
        freshness_sha256: "e".repeat(64),
    })
    .unwrap();
    let read = record_instruction_read(
        &discovery,
        &"e".repeat(64),
        b"Synthetic exclusion record, not an actual instruction file.",
    )
    .unwrap();
    let decision = record_instruction_trust_decision(
        "synthetic-prepared-trust".into(),
        &read,
        InstructionScope {
            workspace_id: request.workspace_id.clone(),
            path: WorkspaceScopePath::new(request.workspace_id.clone(), ["private"]).unwrap(),
        },
        100,
        InstructionTrustDisposition::TrustForNarrowing,
        vec![GuidanceConstraint {
            kind: GuidanceConstraintKind::ExcludeScope,
            target: "private".into(),
        }],
        "f".repeat(64),
    )
    .unwrap();
    build_instruction_ledger(
        request.workspace_snapshot_sha256.clone(),
        vec![discovery],
        vec![read.clone()],
        vec![decision],
        &BTreeMap::from([(read.record_sha256, "e".repeat(64))]),
    )
    .unwrap()
}

fn prepared_input(
    mut owner: LinuxCodingRepository,
    endpoint: LocalChatEndpoint,
    id: RuntimeRunId,
    session: SessionId,
    clock: Arc<AtomicUsize>,
    model_edit: Option<&str>,
) -> (
    CodingEndpointPreparedHttpRun<Verifier, SharedClock>,
    RuntimeApprovalChallenge,
    String,
) {
    let input = compose(id.clone(), session, endpoint, Arc::new(AtomicUsize::new(0)));
    let registry = linux_coding_endpoint_edit_registry().unwrap();
    let definition = registry.list_tools()[0];
    let mut request = input.request;
    request.mode = RuntimeSessionMode::ControlledWrite;
    request.workspace_id = WorkspaceId::from_raw("local-coding-worktree-create");
    request.work_packet.mutable_files = vec!["one.py".into()];
    request.work_packet.protected_files = vec!["private".into()];
    request.visible_tools = runtime_tool_references(&registry).unwrap();
    request.tool_catalog_sha256 =
        runtime_tool_catalog_sha256(&request.tool_catalog_id, &request.visible_tools).unwrap();
    let request = seal_runtime_endpoint_run_request(request).unwrap();
    let ledger = ledger(&request);
    let startup =
        CodingEndpointPreparedStartup::prepare(request, CodingEndpointPreparedKind::Edit, &ledger)
            .unwrap();
    let control = Arc::new(Control);
    let mut consented_sources = vec![];
    if let Some(answer) = model_edit {
        use agentmage_capability_repository_map::coding_proposal::{
            CodingSource, prepare_coding_proposal,
        };
        // Explicit fixture read AND generation disclosure consent. This extra
        // phase is separate from endpoint coordinator accounting below. The
        // socket peer is synthetic, never an actual model or task grade.
        let paths = vec!["one.py".into()];
        let status = owner.inspect_apply().unwrap();
        let sources = owner
            .consented_created_sources(status["worktree_root_sha256"].as_str().unwrap(), &paths)
            .unwrap()
            .read(&paths)
            .unwrap()
            .into_iter()
            .map(|s| CodingSource {
                path: s.path,
                content: s.content,
            })
            .collect::<Vec<_>>();
        let context =
            prepare_coding_proposal(&startup.request().task.objective, &sources, 2048).unwrap();
        let connection = match &input.connection {
            CodingEndpointConnection::Shared {
                transport,
                approval,
            } => CodingEndpointConnection::Shared {
                transport: transport.clone(),
                approval: approval.clone(),
            },
            CodingEndpointConnection::Borrowed { .. } => {
                panic!("fixture must retain configuration")
            }
        };
        let mut streamed = String::new();
        let generated = startup
            .prepare_edit_generation(connection, context, true)
            .unwrap()
            .execute(control.as_ref(), |s| streamed.push_str(s));
        assert_eq!(streamed, answer);
        assert!(generated.result().is_ok());
        assert_eq!(generated.usage().dispatch_attempts, 1);
        generated
            .prepare_preview(&startup, &mut owner, control.as_ref(), 1000)
            .unwrap();
        consented_sources = sources;
    }
    let args = json!({"preview":owner.inspect_edit().unwrap()["pending"]});
    let bytes = serde_json::to_vec(&args).unwrap();
    let call = ToolCall {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_call_id: ToolCallId::from_raw(derived("endpoint-call", &id, 1)),
        correlation_id: CorrelationId::from_raw(derived("correlation", &id, 0)),
        action_id: runtime_action_id(&id, 1),
        tool_id: definition.tool_id.clone(),
        tool_version: definition.tool_version.clone(),
        arguments: ContractPayload {
            schema: definition.input_schema.clone(),
            media_type: "application/json".into(),
            sha256: digest(&bytes),
            bytes,
        },
    };
    let proposal = json!({"tool_call":{"tool_id":call.tool_id,"tool_version":call.tool_version,"arguments":args}}).to_string();
    let prepared = startup
        .bind(
            owner,
            RuntimeTurnId::from_raw(derived("turn", &id, 1)),
            RuntimeOperationId::from_raw(derived("operation", &id, 1)),
            call,
        )
        .ok()
        .unwrap();
    let challenge = prepared.challenge().clone();
    (
        CodingEndpointPreparedHttpRun {
            prepared,
            connection: input.connection,
            control,
            clock: SharedClock(clock),
            verifier: input.verifier,
            sources: consented_sources,
            source_budget: 2048,
            include_usage: true,
        },
        challenge,
        proposal,
    )
}

#[test]
fn endpoint_prepared_service_native_edit_returns_original_lineage_and_needs_new_approval() {
    let fixture = Fixture::new();
    let (upstream, endpoint) = upstream();
    let (proposal_tx, proposal_rx) = mpsc::sync_channel::<String>(1);
    let model = thread::spawn(move || {
        for (old, new) in [("before\n", "first\n"), ("first\n", "second\n")] {
            let mut socket = accept(&upstream);
            let generation = read_upstream(&mut socket);
            assert_eq!(
                generation["messages"][0]["content"],
                agentmage_capability_repository_map::coding_proposal::CODING_PROPOSAL_INSTRUCTION
            );
            let source: Value =
                serde_json::from_str(generation["messages"][1]["content"].as_str().unwrap())
                    .unwrap();
            assert_eq!(source["excerpts"][0]["text"], old);
            respond(
                &mut socket,
                &json!({"edits":[{"path":"one.py","old":old,"new":new}]}).to_string(),
            );
            drop(socket);
            let proposal = proposal_rx.recv_timeout(Duration::from_secs(10)).unwrap();
            let mut socket = accept(&upstream);
            read_upstream(&mut socket);
            respond(&mut socket, &proposal);
            drop(socket);
            let mut socket = accept(&upstream);
            let observed = read_upstream(&mut socket);
            assert!(observed.to_string().contains("write_result"));
            respond(&mut socket, "synthetic unverified completion");
        }
    });
    let mut owner = fixture.owner();
    let clock = Arc::new(AtomicUsize::new(0));
    let mut prior: Option<(Value, Value)> = None;
    for (iteration, content) in [(1, "first\n"), (2, "second\n")] {
        let old = if iteration == 1 {
            "before\n"
        } else {
            "first\n"
        };
        let model_edit = json!({"edits":[{"path":"one.py","old":old,"new":content}]}).to_string();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let service_stop = stop.clone();
        let endpoint = endpoint.clone();
        let clock = clock.clone();
        let proposal_tx = proposal_tx.clone();
        let (tx, rx) = mpsc::sync_channel(1);
        let (challenge_tx, challenge_rx) = mpsc::sync_channel(1);
        let worker = thread::spawn(move || {
            let result = serve_coding_endpoint_prepared(
                listener,
                ApiTokens::new(CLIENT.into(), CONTROL.into()).unwrap(),
                service_stop,
                CodingEndpointObservationAccess::ControlOnly,
                |id, session| {
                    let (input, challenge, proposal) =
                        prepared_input(owner, endpoint, id, session, clock, Some(&model_edit));
                    assert_eq!(input.sources.len(), 1);
                    assert_eq!(input.sources[0].path, "one.py");
                    challenge_tx.send(challenge).unwrap();
                    proposal_tx.send(proposal).unwrap();
                    Ok(input)
                },
            )?;
            let _ = tx.send(result);
            Ok(())
        });
        let api = Api {
            address,
            stop,
            worker: Some(worker),
        };
        let expected_challenge = challenge_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        let request = api.request();
        // No implicit model disclosure or execution during admission.
        fixture.assert_bytes(if iteration == 1 {
            b"before\n"
        } else {
            b"first\n"
        });
        assert_eq!(api.advance(&request, Value::Null, false).0, 403);
        let (code, pending) = api.advance(&request, Value::Null, true);
        assert_eq!(code, 200);
        assert_eq!(pending["challenge"], json!(expected_challenge));
        assert_eq!(api.bind("release", &request).0, 409);
        if let Some((old_request, old_approval)) = prior {
            assert_ne!(request["run_id"], old_request["run_id"]);
            assert_eq!(api.advance(&old_request, Value::Null, true).0, 409);
            assert_eq!(api.advance(&request, old_approval, true).0, 409);
        }
        let approval = json!({"schema_version":CONTRACT_SCHEMA_VERSION,"run_id":pending["challenge"]["run_id"],
            "approval_id":pending["challenge"]["approval_id"],"disposition":RuntimeApprovalDisposition::Allow,
            "challenge_sha256":pending["challenge"]["challenge_sha256"],
            "grant_id":pending["challenge"]["proposed_grant_id"],"selection":null});
        let (code, result) = api.advance(&request, approval.clone(), true);
        assert_eq!(code, 200);
        assert_eq!(result["outcome"]["outcome"]["state"], "FAILED");
        fixture.assert_bytes(content.as_bytes());
        assert!(matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
        assert_eq!(api.bind("release", &request).0, 200);
        drop(api); // joins every HTTP worker before returning the original owner
        owner = rx.recv_timeout(Duration::from_secs(1)).unwrap().unwrap();
        let edit = owner.inspect_edit().unwrap();
        assert!(edit["pending"].is_null());
        assert_eq!(edit["latest"]["outcome"], "committed");
        let receipts = edit["latest"]["write_result"]["receipts"]
            .as_array()
            .unwrap();
        assert_eq!(receipts.last().unwrap()["status"], "verified");
        assert!(
            receipts
                .iter()
                .all(|receipt| receipt["grant_id"] == json!(expected_challenge.proposed_grant_id))
        );
        let apply = owner.inspect_apply().unwrap();
        assert_eq!(apply["available_paths"], json!(["one.py"]));
        prior = Some((request, approval));
    }
    assert!(clock.load(Ordering::SeqCst) > 4);
    model.join().unwrap();
    // Separate original-checkout read consent in this fixture. Preparing its
    // diff must retain the FIRST preimage through both native endpoint edits.
    // This is an inert preview, never approval to apply to the original.
    let roots = owner.inspect_apply().unwrap();
    let preview = owner
        .prepare_apply(
            &["one.py".into()],
            "Inspect the two-edit lineage",
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
    fixture.assert_bytes(b"second\n");
}

#[test]
fn endpoint_prepared_service_refuses_excluded_sources_and_foreign_identity_before_serving() {
    for selection in ["private/secret.py", "../escape.py", "foreign-run"] {
        let fixture = Fixture::new();
        let mut owner = fixture.owner();
        fixture.prepare(&mut owner, "proposed\n");
        let (upstream, endpoint) = upstream();
        let clock = Arc::new(AtomicUsize::new(0));
        let factory_calls = AtomicUsize::new(0);
        let result = serve_coding_endpoint_prepared(
            TcpListener::bind("127.0.0.1:0").unwrap(),
            ApiTokens::new(CLIENT.into(), CONTROL.into()).unwrap(),
            Arc::new(AtomicBool::new(false)),
            CodingEndpointObservationAccess::ControlOnly,
            |id, session| {
                factory_calls.fetch_add(1, Ordering::SeqCst);
                let id = if selection == "foreign-run" {
                    RuntimeRunId::from_raw("foreign-prepared-run")
                } else {
                    id
                };
                let (mut input, _, _) =
                    prepared_input(owner, endpoint, id, session, clock.clone(), None);
                if selection != "foreign-run" {
                    input.sources = vec![
                        agentmage_capability_repository_map::coding_proposal::CodingSource {
                            path: selection.into(),
                            content: "synthetic source must never leave the process".into(),
                        },
                    ];
                }
                Ok(input)
            },
        );
        assert!(matches!(result, Err(ApiError::Configuration)));
        assert_eq!(factory_calls.load(Ordering::SeqCst), 1);
        assert_eq!(clock.load(Ordering::SeqCst), 0);
        fixture.assert_bytes(b"before\n");
        assert!(matches!(upstream.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
    }
}

#[test]
fn endpoint_prepared_service_stop_without_release_does_not_return_a_native_owner() {
    let fixture = Fixture::new();
    let mut owner = fixture.owner();
    fixture.prepare(&mut owner, "proposed\n");
    let (upstream, endpoint) = upstream();
    let clock = Arc::new(AtomicUsize::new(0));
    let stop = Arc::new(AtomicBool::new(false));
    let result = serve_coding_endpoint_prepared(
        TcpListener::bind("127.0.0.1:0").unwrap(),
        ApiTokens::new(CLIENT.into(), CONTROL.into()).unwrap(),
        stop.clone(),
        CodingEndpointObservationAccess::ControlOnly,
        |id, session| {
            let input = prepared_input(owner, endpoint, id, session, clock.clone(), None).0;
            stop.store(true, Ordering::Release);
            Ok(input)
        },
    )
    .unwrap();
    assert!(result.is_none());
    assert_eq!(clock.load(Ordering::SeqCst), 0);
    fixture.assert_bytes(b"before\n");
    assert!(matches!(upstream.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
}

fn strict_input(
    owner: LinuxCodingRepository,
    endpoint: LocalChatEndpoint,
    id: RuntimeRunId,
    session: SessionId,
    clock: Arc<AtomicUsize>,
) -> agentmage_local_api::CodingEndpointStrictHttpRun<Verifier, SharedClock> {
    let input = compose(id, session, endpoint, Arc::new(AtomicUsize::new(0)));
    let registry = linux_coding_endpoint_edit_registry().unwrap();
    let mut request = input.request;
    request.mode = RuntimeSessionMode::ControlledWrite;
    request.workspace_id = WorkspaceId::from_raw("local-coding-worktree-create");
    request.work_packet.mutable_files = vec!["one.py".into()];
    request.work_packet.protected_files = vec!["private".into()];
    request.visible_tools = runtime_tool_references(&registry).unwrap();
    request.tool_catalog_sha256 =
        runtime_tool_catalog_sha256(&request.tool_catalog_id, &request.visible_tools).unwrap();
    request.limits.max_model_calls = 2;
    let request = seal_runtime_endpoint_run_request(request).unwrap();
    let evidence = ledger(&request);
    agentmage_local_api::CodingEndpointStrictHttpRun {
        startup: CodingEndpointPreparedStartup::prepare(
            request,
            CodingEndpointPreparedKind::Edit,
            &evidence,
        )
        .unwrap(),
        repository: owner,
        connection: input.connection,
        control: Arc::new(Control),
        clock: SharedClock(clock),
        verifier: input.verifier,
        paths: vec!["one.py".into()],
        source_budget: 2048,
        include_usage: true,
    }
}

fn approval(challenge: &Value, allow: bool) -> Value {
    json!({"schema_version":CONTRACT_SCHEMA_VERSION,"run_id":challenge["run_id"],
        "approval_id":challenge["approval_id"],"disposition":if allow {RuntimeApprovalDisposition::Allow} else {RuntimeApprovalDisposition::Deny},
        "challenge_sha256":challenge["challenge_sha256"],
        "grant_id":if allow {challenge["proposed_grant_id"].clone()} else {Value::Null},"selection":null})
}

#[test]
fn endpoint_strict_service_fresh_sources_exact_native_approvals_and_same_run_budget() {
    use agentmage_capability_repository_map::coding_proposal::{
        CODING_PROPOSAL_INSTRUCTION, CodingSource, prepare_coding_proposal,
    };
    for case in [
        "allow",
        "deny",
        "cancel",
        "malformed",
        "stale",
        "budget",
        "expiry",
    ] {
        let fixture = Fixture::new();
        let owner = fixture.owner();
        let (upstream, endpoint) = upstream();
        let observed = Arc::new(AtomicUsize::new(0));
        let calls = observed.clone();
        let (objective_tx, objective_rx) = mpsc::sync_channel::<String>(1);
        let peer = thread::spawn(move || {
            let objective = objective_rx.recv_timeout(Duration::from_secs(10)).unwrap();
            let count = if case == "allow" { 2 } else { 1 };
            for (old, new) in [("before\n", "first\n"), ("first\n", "second\n")]
                .into_iter()
                .take(count)
            {
                let mut socket = accept(&upstream);
                let body = read_upstream(&mut socket);
                assert_eq!(body["messages"][0]["content"], CODING_PROPOSAL_INSTRUCTION);
                let context = prepare_coding_proposal(
                    &objective,
                    &[CodingSource {
                        path: "one.py".into(),
                        content: old.into(),
                    }],
                    2048,
                )
                .unwrap();
                assert_eq!(body["messages"][1]["content"], context.user_message());
                calls.fetch_add(1, Ordering::SeqCst);
                respond(
                    &mut socket,
                    &if case == "malformed" {
                        "untrusted completion claim".into()
                    } else {
                        json!({"edits":[{"path":"one.py","old":old,"new":new}]}).to_string()
                    },
                );
            }
            upstream
        });
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let service_stop = stop.clone();
        let clock = Arc::new(AtomicUsize::new(0));
        let shared_clock = clock.clone();
        let worker = thread::spawn(move || {
            agentmage_local_api::serve_coding_endpoint_strict_edits(
                listener,
                ApiTokens::new(CLIENT.into(), CONTROL.into()).unwrap(),
                service_stop,
                CodingEndpointObservationAccess::ControlOnly,
                |id, session| {
                    let mut input = strict_input(owner, endpoint, id, session, shared_clock);
                    if case == "budget" {
                        // Same sealed original run, one model call for its lifetime.
                        let mut request = input.startup.request().clone();
                        request.limits.max_model_calls = 1;
                        let request = seal_runtime_endpoint_run_request(request).unwrap();
                        let evidence = ledger(&request);
                        input.startup = CodingEndpointPreparedStartup::prepare(
                            request,
                            CodingEndpointPreparedKind::Edit,
                            &evidence,
                        )
                        .unwrap();
                    }
                    Ok(input)
                },
            )
        });
        let api = Api {
            address,
            stop,
            worker: Some(worker),
        };
        let request = api.request();
        objective_tx
            .send(request["task"]["objective"].as_str().unwrap().into())
            .unwrap();
        assert_eq!(
            api.call("GET", "/v1/code/endpoint/request", CLIENT, None).0,
            403
        );
        assert_eq!(api.advance(&request, Value::Null, false).0, 403);
        assert_eq!(observed.load(Ordering::SeqCst), 0);
        assert_eq!(clock.load(Ordering::SeqCst), 0);
        fixture.assert_bytes(b"before\n");
        let (code, mut result) = api.advance(&request, Value::Null, true);
        assert_eq!(code, 200);
        if case != "malformed" {
            assert_eq!(result["state"], "awaiting_approval");
            assert_eq!(observed.load(Ordering::SeqCst), 1);
            assert_eq!(api.bind("release", &request).0, 409);
            fixture.assert_bytes(b"before\n");
            let first = approval(&result["challenge"], true);
            let mut tampered = first.clone();
            tampered["challenge_sha256"] = json!("0".repeat(64));
            assert_eq!(api.advance(&request, tampered, true).0, 409);
            fixture.assert_bytes(b"before\n");
            if case == "stale" {
                fs::write(fixture.0.join("owned/worktree/one.py"), "external\n").unwrap();
            }
            if case == "expiry" {
                let elapsed = request["limits"]["max_elapsed_ms"].as_u64().unwrap() + 1000;
                let expiry = result["challenge"]["expires_at_epoch_ms"].as_u64().unwrap();
                clock.store((elapsed.max(expiry) - 1000) as usize, Ordering::SeqCst);
                assert_eq!(api.advance(&request, first.clone(), true).0, 409);
                fixture.assert_bytes(b"before\n");
                result = api.advance(&request, Value::Null, true).1;
            } else if case == "cancel" {
                assert_eq!(api.bind("cancel", &request).0, 200);
                result = api.advance(&request, Value::Null, false).1;
            } else {
                let decision = approval(&result["challenge"], case != "deny");
                let (status, next) = api.advance(&request, decision, true);
                if case == "stale" {
                    assert_eq!(status, 409);
                    assert_eq!(next, json!({"error":"code.endpoint.refused"}));
                    fixture.assert_bytes(b"external\n");
                    assert_eq!(
                        api.advance(&request, Value::Null, true),
                        (409, json!({"error":"code.endpoint.faulted"}))
                    );
                    assert_eq!(api.bind("release", &request).0, 200);
                    drop(api);
                    let upstream = peer.join().unwrap();
                    assert!(
                        matches!(upstream.accept(), Err(e) if e.kind()==std::io::ErrorKind::WouldBlock)
                    );
                    continue;
                } else {
                    assert_eq!(status, 200);
                    result = next;
                }
            }
            if case == "allow" {
                fixture.assert_bytes(b"first\n");
                assert_eq!(result["state"], "awaiting_approval");
                assert_ne!(result["challenge"]["approval_id"], first["approval_id"]);
                assert_eq!(observed.load(Ordering::SeqCst), 2);
                assert_eq!(api.advance(&request, first, true).0, 409);
                fixture.assert_bytes(b"first\n");
                result = api
                    .advance(&request, approval(&result["challenge"], true), true)
                    .1;
            }
        }
        assert_eq!(result["state"], "terminal", "case={case}: {result}");
        assert_eq!(
            result["outcome"]["outcome"]["state"],
            match case {
                "deny" => "DECLINED",
                "cancel" => "CANCELLED",
                "allow" | "budget" | "expiry" => "EXHAUSTED",
                _ => "FAILED",
            }
        );
        assert_eq!(
            result["outcome"]["outcome"]["model_call_count"],
            if case == "allow" { 2 } else { 1 }
        );
        assert!(result["outcome"]["outcome"]["output"].is_null());
        assert_eq!(
            result["outcome"]["outcome"]["tool_call_count"],
            match case {
                "allow" => 2,
                "budget" | "deny" | "cancel" | "expiry" => 1,
                _ => 0,
            }
        );
        fixture.assert_bytes(match case {
            "allow" => b"second\n",
            "budget" => b"first\n",
            "stale" => b"external\n",
            _ => b"before\n",
        });
        assert_eq!(api.advance(&request, Value::Null, true).1, result);
        // Reconstruct the full immutable event sequence using ordinary paging.
        let mut sequence = agentmage_kernel_engine::runtime_event::RuntimeEventSequence::new();
        let mut index = 0;
        loop {
            let (code, page) = api.call("POST", "/v1/code/endpoint/events", CONTROL, Some(json!({"run_id":request["run_id"],"request_sha256":request["request_sha256"],"after":index})));
            assert_eq!(code, 200);
            for event in page["events"].as_array().unwrap() {
                sequence
                    .push(&serde_json::from_value(event.clone()).unwrap())
                    .unwrap();
            }
            index = page["next"].as_u64().unwrap();
            if index == page["total"].as_u64().unwrap() {
                break;
            }
        }
        assert_eq!(api.bind("release", &request).0, 200);
        assert_eq!(api.advance(&request, Value::Null, true).0, 409);
        drop(api);
        let upstream = peer.join().unwrap();
        assert!(matches!(upstream.accept(), Err(e) if e.kind()==std::io::ErrorKind::WouldBlock));
    }
}

#[test]
fn endpoint_strict_service_rejects_foreign_factory_or_excluded_sources_before_io() {
    for case in [
        "foreign-run",
        "foreign-session",
        "excluded",
        "outside",
        "zero-budget",
        "empty",
        "foreign-endpoint",
    ] {
        let fixture = Fixture::new();
        let owner = fixture.owner();
        let (upstream, endpoint) = upstream();
        let clock = Arc::new(AtomicUsize::new(0));
        let result = agentmage_local_api::serve_coding_endpoint_strict_edits(
            TcpListener::bind("127.0.0.1:0").unwrap(),
            ApiTokens::new(CLIENT.into(), CONTROL.into()).unwrap(),
            Arc::new(AtomicBool::new(false)),
            CodingEndpointObservationAccess::ControlOnly,
            |id, session| {
                let mut input = strict_input(
                    owner,
                    endpoint,
                    if case == "foreign-run" {
                        RuntimeRunId::from_raw("foreign")
                    } else {
                        id
                    },
                    if case == "foreign-session" {
                        SessionId::from_raw("foreign")
                    } else {
                        session
                    },
                    clock.clone(),
                );
                match case {
                    "excluded" => input.paths = vec!["private/missing.py".into()],
                    "outside" => input.paths = vec!["../outside.py".into()],
                    "zero-budget" => input.source_budget = 0,
                    "empty" => input.paths.clear(),
                    "foreign-endpoint" => {
                        let mut request = input.startup.request().clone();
                        request.model_selection.endpoint = "http://127.0.0.1:1/v1".into();
                        let request = seal_runtime_endpoint_run_request(request).unwrap();
                        let evidence = ledger(&request);
                        input.startup = CodingEndpointPreparedStartup::prepare(
                            request,
                            CodingEndpointPreparedKind::Edit,
                            &evidence,
                        )
                        .unwrap();
                    }
                    _ => (),
                }
                Ok(input)
            },
        );
        assert!(
            matches!(result, Err(ApiError::Configuration)),
            "case={case}"
        );
        assert_eq!(clock.load(Ordering::SeqCst), 0);
        fixture.assert_bytes(b"before\n");
        assert!(matches!(upstream.accept(),Err(e) if e.kind()==std::io::ErrorKind::WouldBlock));
    }
}

#[test]
fn endpoint_edit_test_service_keeps_disclosure_consent_and_fixed_test_approval_in_one_run() {
    use agentmage_host::coding_endpoint_edit_previews::CodingEndpointValidation;
    use agentmage_kernel_engine::{
        command_runner::{CommandBounds, CommandRisk, CommandSpec, CommandWorkingDirectory},
        validation_template::ValidationKind,
    };
    use agentmage_platform_linux::linux_coding_endpoint_coding_registry;
    for invalid_ack in [false, true] {
        let fixture = Fixture::new();
        let owner = fixture.owner();
        let (upstream, endpoint) = upstream();
        let observed = Arc::new(AtomicUsize::new(0));
        let calls = observed.clone();
        let peer = thread::spawn(move || {
            let mut socket = accept(&upstream);
            let body = read_upstream(&mut socket);
            assert_eq!(
                body["messages"][0]["content"],
                agentmage_capability_repository_map::coding_proposal::CODING_PROPOSAL_INSTRUCTION
            );
            calls.fetch_add(1, Ordering::SeqCst);
            respond(
                &mut socket,
                &json!({"edits":[{"path":"one.py","old":"before\n","new":"first\n"}]}).to_string(),
            );
            drop(socket); // Finish the first SSE response before the next request.
            let mut socket = accept(&upstream);
            let body = read_upstream(&mut socket);
            assert!(
                body["messages"][0]["content"]
                    .as_str()
                    .unwrap()
                    .starts_with("Return exactly {\"validate\":true}")
            );
            calls.fetch_add(1, Ordering::SeqCst);
            respond(
                &mut socket,
                if invalid_ack {
                    "{\"validate\":true,\"command\":\"foreign\"}"
                } else {
                    "{\"validate\":true}"
                },
            );
            upstream
        });
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let service_stop = stop.clone();
        let worker = thread::spawn(move || {
            agentmage_local_api::serve_coding_endpoint_edits_and_tests(
                listener,
                ApiTokens::new(CLIENT.into(), CONTROL.into()).unwrap(),
                service_stop,
                CodingEndpointObservationAccess::ControlOnly,
                |id, session| {
                    // This positive fixture has no read exclusions. The original
                    // edit-only fixture's exclusion still refuses combined admission.
                    let source = compose(id, session, endpoint, Arc::new(AtomicUsize::new(0)));
                    let registry = linux_coding_endpoint_coding_registry().unwrap();
                    let mut request = source.request;
                    request.mode = RuntimeSessionMode::ControlledWrite;
                    request.workspace_id = WorkspaceId::from_raw("local-coding-worktree-create");
                    request.work_packet.mutable_files = vec!["one.py".into()];
                    request.work_packet.protected_files = vec!["private".into()];
                    request.visible_tools = runtime_tool_references(&registry).unwrap();
                    request.tool_catalog_sha256 = runtime_tool_catalog_sha256(
                        &request.tool_catalog_id,
                        &request.visible_tools,
                    )
                    .unwrap();
                    request.limits.max_model_calls = 2;
                    let request = seal_runtime_endpoint_run_request(request).unwrap();
                    assert!(matches!(
                        CodingEndpointPreparedStartup::prepare(request.clone(), CodingEndpointPreparedKind::EditAndTest, &ledger(&request)),
                        Err(agentmage_host::coding_endpoint_startup::CodingEndpointStartupError::Source)
                    ));
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
                        verifier: source.verifier,
                        paths: vec!["one.py".into()],
                        source_budget: 2048,
                        include_usage: true,
                    };
                    let command = CommandSpec::seal(
                        "fixed-http-test",
                        "1.0.0",
                        "/usr/bin/true",
                        digest(&fs::read("/usr/bin/true").unwrap()),
                        vec![],
                        CommandWorkingDirectory::OwnedWorktree,
                        Default::default(),
                        CommandRisk::Moderate,
                        CommandBounds::new(1000, 1024, 1024, 16 * 1024 * 1024, 1, 100).unwrap(),
                    )
                    .unwrap();
                    Ok((
                        input,
                        CodingEndpointValidation {
                            command,
                            kind: ValidationKind::Unit,
                            minimum_test_count: 1,
                            output_format: Default::default(),
                        },
                    ))
                },
            )
        });
        let api = Api {
            address,
            stop,
            worker: Some(worker),
        };
        let request = api.request();
        assert_eq!(request["visible_tools"].as_array().unwrap().len(), 2);
        assert_eq!(api.advance(&request, Value::Null, false).0, 403);
        assert_eq!(observed.load(Ordering::SeqCst), 0);
        let (status, result) = api.advance(&request, Value::Null, true);
        assert_eq!(status, 200);
        assert_eq!(result["state"], "awaiting_approval");
        fixture.assert_bytes(b"before\n");
        let original = approval(&result["challenge"], true);
        assert_eq!(api.advance(&request, original.clone(), false).0, 403);
        fixture.assert_bytes(b"before\n");
        let (status, mut result) = api.advance(&request, original.clone(), true);
        assert_eq!(status, 200);
        fixture.assert_bytes(b"first\n");
        assert_eq!(observed.load(Ordering::SeqCst), 2);
        if result["state"] == "awaiting_approval" {
            assert!(!invalid_ack);
            assert_eq!(
                result["challenge"]["presentation"]["tool_id"],
                "agentmage.code.run-prepared-command"
            );
            assert_eq!(api.advance(&request, original, true).0, 409);
            result = api
                .advance(&request, approval(&result["challenge"], false), true)
                .1;
        }
        assert_eq!(result["state"], "terminal");
        assert!(matches!(
            result["outcome"]["outcome"]["state"].as_str(),
            Some("FAILED" | "DECLINED")
        ));
        assert_eq!(result["outcome"]["outcome"]["model_call_count"], 2);
        assert!(result["outcome"]["outcome"]["output"].is_null());
        assert_eq!(api.advance(&request, Value::Null, true).1, result);
        assert_eq!(api.bind("release", &request).0, 200);
        drop(api);
        let upstream = peer.join().unwrap();
        assert!(matches!(upstream.accept(),Err(e) if e.kind()==std::io::ErrorKind::WouldBlock));
    }
}
