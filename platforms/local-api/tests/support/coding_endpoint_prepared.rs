// SPDX-License-Identifier: Apache-2.0
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
        let scope = owner.select_created_sources(&paths).unwrap();
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
        fs::remove_dir_all(&self.0).unwrap();
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
    owner: LinuxCodingRepository,
    endpoint: LocalChatEndpoint,
    id: RuntimeRunId,
    session: SessionId,
    clock: Arc<AtomicUsize>,
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
            control: Arc::new(Control),
            clock: SharedClock(clock),
            verifier: input.verifier,
            sources: vec![],
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
        for _ in 0..2 {
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
        fixture.prepare(&mut owner, content);
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
                        prepared_input(owner, endpoint, id, session, clock);
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
                let (mut input, _, _) = prepared_input(owner, endpoint, id, session, clock.clone());
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
            let input = prepared_input(owner, endpoint, id, session, clock.clone()).0;
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
