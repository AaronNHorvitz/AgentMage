// SPDX-License-Identifier: Apache-2.0
//! Real HTTP/endpoint sockets with synthetic tools and a rejecting verifier.
//! No native filesystem, grant, model quality or coding success is claimed.

use agentmage_host::{
    coding_endpoint::CodingEndpointConnection, runtime_tools::read_only_runtime_registry,
};
use agentmage_kernel_contracts::*;
use agentmage_kernel_engine::{
    runtime_coordinator::{runtime_tool_catalog_sha256, seal_runtime_endpoint_run_request},
    runtime_loop::*,
};
use agentmage_local_api::{
    ApiError, ApiTokens, CodingEndpointHttpRun, CodingEndpointObservationAccess,
    serve_admitted_coding_endpoint, serve_coding_endpoint,
    serve_coding_endpoint_with_observation_access,
};

#[path = "support/coding_endpoint_mcp.rs"]
mod endpoint_mcp;
#[path = "support/coding_endpoint_record.rs"]
mod endpoint_record;
#[path = "support/coding_endpoint_tool_return.rs"]
mod endpoint_tool_return;
use agentmage_platform_linux_inference::{
    local_chat::LocalChatEndpoint,
    local_chat_transport::{LocalChatApproval, LocalChatTransport, TransportLimits},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const CLIENT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const CONTROL: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[test]
fn endpoint_terminal_inspects_cancels_and_pages_the_actual_retained_http_owner() {
    use std::{
        fs,
        process::{Command, Stdio},
    };
    let executions = Arc::new(AtomicUsize::new(0));
    let model = TcpListener::bind("127.0.0.1:0").unwrap();
    model.set_nonblocking(true).unwrap();
    let endpoint =
        LocalChatEndpoint::parse(&format!("http://{}/v1", model.local_addr().unwrap())).unwrap();
    let api = Api::new(endpoint, executions.clone());
    // Fresh private auth directory; only synthetic fixed tokens go in it.
    let root = std::env::temp_dir().join(format!(
        "agentmage-endpoint-terminal-http-{}",
        std::process::id()
    ));
    ApiTokens::initialize(&root).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    let _cleanup = Cleanup(root.clone());
    fs::write(root.join("client.token"), CLIENT).unwrap();
    fs::write(root.join("control.token"), CONTROL).unwrap();
    let cli = |args: &[&str]| {
        let output = Command::new(env!("CARGO_BIN_EXE_agentmage-code"))
            .arg(api.address.to_string())
            .arg(&root)
            .args(args)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    };
    let selected = cli(&["endpoint"]);
    let r = selected["request"].clone();
    let run = r["run_id"].as_str().unwrap();
    let hash = r["request_sha256"].as_str().unwrap();
    assert_eq!(selected["retention"], "process-only");
    assert_eq!(
        cli(&["endpoint-events", run, hash, "0"])["page"]["total"],
        0
    );
    assert_eq!(
        cli(&["endpoint-cancel", run, hash]),
        json!({"cancellation":"requested"})
    );
    assert_eq!(api.advance(&r, Value::Null, false).0, 200);
    let mut after = 0;
    loop {
        let observed = cli(&["endpoint-events", run, hash, &after.to_string()]);
        let page = &observed["page"];
        assert_eq!(observed["index_is_resume_cursor"], false);
        assert_eq!(page["events"].as_array().unwrap().len(), 1);
        assert_eq!(page["events"][0]["run_id"], run);
        after = page["next"].as_u64().unwrap();
        if page["next"] == page["total"] {
            break;
        }
    }
    assert!(after > 0);
    assert_eq!(executions.load(Ordering::SeqCst), 0);
    assert!(matches!(model.accept(),Err(e) if e.kind()==std::io::ErrorKind::WouldBlock));
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
struct Clock;
impl RuntimeClock for Clock {
    fn now_epoch_ms(&mut self) -> Result<u64, RuntimePortFailure> {
        Ok(1000)
    }
}
struct Boundary(Arc<AtomicUsize>);
impl RuntimeToolBoundary for Boundary {
    fn evaluate(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimeOperationId,
        _: &ToolDefinition,
        _: &ToolCall,
        _: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        panic!("native family")
    }
    fn resolve(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimeApprovalChallenge,
        _: &RuntimeApprovalResponse,
        _: &ToolDefinition,
        _: &ToolCall,
        _: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        panic!("native family")
    }
    fn execute(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimePermissionEvaluation,
        _: &ToolDefinition,
        _: &ToolCall,
        _: Option<&dyn ModelCancellationProbe>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        panic!("native family")
    }
    fn evaluate_scoped(
        &mut self,
        scope: RuntimeRunScope<'_>,
        _: &RuntimeOperationId,
        _: &ToolDefinition,
        call: &ToolCall,
        _: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        assert!(scope.endpoint_request().is_some());
        Ok(RuntimePermissionEvaluation::Ask {
            approval_id: ApprovalId::from_raw("synthetic-approval"),
            grant_id: GrantId::from_raw("synthetic-grant"),
            preview_sha256: call.arguments.sha256.clone(),
            expires_at_epoch_ms: 100_000,
        })
    }
    fn resolve_scoped(
        &mut self,
        scope: RuntimeRunScope<'_>,
        _: &RuntimeApprovalChallenge,
        response: &RuntimeApprovalResponse,
        _: &ToolDefinition,
        call: &ToolCall,
        _: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        assert!(scope.endpoint_request().is_some());
        assert_eq!(response.disposition, RuntimeApprovalDisposition::Allow);
        Ok(RuntimePermissionEvaluation::Allow {
            approval_id: response.approval_id.clone(),
            grant_id: response.grant_id.clone().unwrap(),
            preview_sha256: call.arguments.sha256.clone(),
            expires_at_epoch_ms: 100_000,
            decision_sha256: digest(b"synthetic decision"),
            authority_sha256: digest(b"synthetic authority"),
        })
    }
    fn execute_scoped(
        &mut self,
        scope: RuntimeRunScope<'_>,
        evaluation: &RuntimePermissionEvaluation,
        definition: &ToolDefinition,
        call: &ToolCall,
        _: Option<&dyn ModelCancellationProbe>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        assert!(scope.endpoint_request().is_some());
        assert!(matches!(
            evaluation,
            RuntimePermissionEvaluation::Allow { .. }
        ));
        self.0.fetch_add(1, Ordering::SeqCst);
        // An intentionally synthetic observation. This is not a native receipt.
        let bytes = b"{\"synthetic_observation\":\"original retained boundary\"}".to_vec();
        Ok(RuntimeToolExecution {
            receipt_id: ReceiptId::from_raw("synthetic-receipt"),
            receipt_sha256: digest(&bytes),
            result: ToolResult {
                schema_version: CONTRACT_SCHEMA_VERSION,
                tool_call_id: call.tool_call_id.clone(),
                correlation_id: call.correlation_id.clone(),
                outcome: OperationOutcome::Succeeded,
                output: Some(ContractPayload {
                    schema: definition.output_schema.clone(),
                    media_type: "application/json".into(),
                    sha256: digest(&bytes),
                    bytes: bytes.clone(),
                }),
                validation_issues: vec![],
                evidence: vec![EvidenceReference {
                    schema_version: CONTRACT_SCHEMA_VERSION,
                    evidence_id: EvidenceId::from_raw("synthetic-evidence"),
                    kind: EvidenceKind::ToolOutput,
                    source_id: "synthetic-source".into(),
                    object_id: "synthetic-object".into(),
                    fragment: None,
                    content_sha256: digest(&bytes),
                    observed_revision: Some(scope.repository_snapshot_id().as_str().into()),
                }],
                error: None,
                elapsed_ms: 1,
                state_change: StateChange::NotChanged,
            },
            result_output_kind: Some(RuntimeArtifactKind::Report),
            artifact_candidates: vec![],
        })
    }
}
struct Verifier(VerifierId);
impl RuntimeVerifierPort for Verifier {
    fn verifier_id(&self) -> &VerifierId {
        &self.0
    }
    fn verify(
        &mut self,
        _: RuntimeVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        panic!("native family")
    }
    fn verify_completion(
        &mut self,
        input: RuntimeCompletionVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        assert!(input.request.endpoint_request().is_some());
        assert_eq!(input.tool_results.len(), 1);
        Ok(VerifierCandidate {
            schema_version: CONTRACT_SCHEMA_VERSION,
            verifier_record_id: VerifierRecordId::from_raw("synthetic-verification"),
            verifier_id: self.0.clone(),
            task_id: input.request.task().task_id.clone(),
            proposal_id: input.proposal.proposal_id().clone(),
            repository_snapshot_id: input.request.repository_snapshot_id().clone(),
            state_revision: input.state_revision,
            source: VerifierSource::DeterministicPostcondition,
            disposition: VerifierDisposition::Failed,
            postconditions: input
                .postconditions
                .iter()
                .map(|id| PostconditionResult {
                    postcondition_id: id.clone(),
                    passed: false,
                    evidence: input.evidence.to_vec(),
                })
                .collect(),
        })
    }
}
fn compose(
    run_id: RuntimeRunId,
    session_id: SessionId,
    endpoint: LocalChatEndpoint,
    executions: Arc<AtomicUsize>,
) -> CodingEndpointHttpRun<Boundary, Verifier, Clock> {
    let transport = Arc::new(
        LocalChatTransport::new(
            endpoint.clone(),
            None,
            TransportLimits {
                total: Duration::from_secs(2),
                idle: Duration::from_secs(1),
            },
        )
        .unwrap(),
    );
    let registry = read_only_runtime_registry().unwrap();
    // Reuse only first-party common contract fields; no native profile reaches
    // the endpoint request, model port or coordinator.
    let mut value: Value = serde_json::from_slice(include_bytes!(
        "../../../fixtures/runtime-producer/v5/run-request.json"
    ))
    .unwrap();
    value.as_object_mut().unwrap().remove("model_profile");
    value["run_id"] = json!(run_id);
    value["session_id"] = json!(session_id);
    value["task"]["session_id"] = json!(session_id);
    value["task"]["constraints"] = json!(["Synthetic HTTP ownership fixture"]);
    value["mode"] = json!("ephemeral_read_only");
    value["event_cursor"] = Value::Null;
    value["model_selection"] = json!(transport.model_selection("fixture-model"));
    value["max_output_tokens"] = json!(128);
    value["context_budget"] = json!({"max_messages":16,"max_content_bytes":32768,"max_serialized_messages_bytes":65536,"max_request_body_bytes":131072});
    let catalog = ToolCatalogId::from_raw("synthetic-http-read-catalog");
    let visible = runtime_tool_references(&registry).unwrap();
    value["tool_catalog_id"] = json!(catalog);
    value["tool_catalog_sha256"] = json!(runtime_tool_catalog_sha256(&catalog, &visible).unwrap());
    value["visible_tools"] = json!(visible);
    value["limits"]["max_output_bytes"] = json!(4 * 1024 * 1024);
    let request =
        seal_runtime_endpoint_run_request(serde_json::from_value(value).unwrap()).unwrap();
    CodingEndpointHttpRun {
        request,
        connection: CodingEndpointConnection::Shared {
            transport,
            approval: Arc::new(LocalChatApproval::for_endpoint(endpoint)),
        },
        registry,
        sources: vec![],
        source_budget: 2048,
        include_usage: true,
        tool_boundary: Boundary(executions),
        verifier: Verifier(VerifierId::from_raw("synthetic-verifier")),
        clock: Clock,
    }
}
struct Api {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<(), ApiError>>>,
}
impl Api {
    fn new(endpoint: LocalChatEndpoint, executions: Arc<AtomicUsize>) -> Self {
        Self::start(endpoint, executions, false)
    }
    fn start(endpoint: LocalChatEndpoint, executions: Arc<AtomicUsize>, delegate: bool) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let worker = thread::spawn(move || {
            let tokens = ApiTokens::new(CLIENT.into(), CONTROL.into()).unwrap();
            let factory = |id, session| Ok(compose(id, session, endpoint, executions));
            if delegate {
                serve_coding_endpoint_with_observation_access(
                    listener,
                    tokens,
                    worker_stop,
                    CodingEndpointObservationAccess::DelegateObservations,
                    factory,
                )
            } else {
                serve_coding_endpoint(listener, tokens, worker_stop, factory)
            }
        });
        Self {
            address,
            stop,
            worker: Some(worker),
        }
    }
    fn call(&self, method: &str, path: &str, token: &str, input: Option<Value>) -> (u16, Value) {
        let mut s = TcpStream::connect(self.address).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let body = input.map(|v| v.to_string()).unwrap_or_default();
        s.write_all(format!("{method} {path} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", self.address, body.len()).as_bytes()).unwrap();
        let mut bytes = Vec::new();
        loop {
            let mut b = [0; 4096];
            match s.read(&mut b) {
                Ok(0) => break,
                Ok(n) => bytes.extend_from_slice(&b[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => break,
                Err(e) => panic!("{e}"),
            }
        }
        let p = bytes.windows(4).position(|p| p == b"\r\n\r\n").unwrap();
        let code = std::str::from_utf8(&bytes[..p])
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        (code, serde_json::from_slice(&bytes[p + 4..]).unwrap())
    }
    fn request(&self) -> Value {
        let (code, v) = self.call("GET", "/v1/code/endpoint/request", CONTROL, None);
        assert_eq!(code, 200);
        v["request"].clone()
    }
    fn advance(&self, r: &Value, response: Value, consent: bool) -> (u16, Value) {
        self.call("POST", "/v1/code/endpoint/advance", CONTROL, Some(json!({"run_id":r["run_id"],"request_sha256":r["request_sha256"],"response":response,"approve_model_context":consent})))
    }
    fn bind(&self, action: &str, r: &Value) -> (u16, Value) {
        let mut v = json!({"run_id":r["run_id"],"request_sha256":r["request_sha256"]});
        if action == "events" {
            v["after"] = json!(0);
        }
        self.call(
            "POST",
            &format!("/v1/code/endpoint/{action}"),
            CONTROL,
            Some(v),
        )
    }
}
impl Drop for Api {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(w) = self.worker.take() {
            w.join().unwrap().unwrap();
        }
    }
}
fn upstream() -> (TcpListener, LocalChatEndpoint) {
    let s = TcpListener::bind("127.0.0.1:0").unwrap();
    s.set_nonblocking(true).unwrap();
    let e =
        LocalChatEndpoint::parse(&format!("http://{}/selected", s.local_addr().unwrap())).unwrap();
    (s, e)
}
fn accept(listener: &TcpListener) -> TcpStream {
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        match listener.accept() {
            Ok((s, _)) => return s,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(5))
            }
            Err(e) => panic!("{e}"),
        }
    }
}
fn read_upstream(s: &mut TcpStream) -> Value {
    s.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    let mut h = Vec::new();
    while !h.ends_with(b"\r\n\r\n") {
        let mut b = [0];
        s.read_exact(&mut b).unwrap();
        h.push(b[0]);
    }
    let h = String::from_utf8(h).unwrap();
    assert!(h.starts_with("POST /selected/chat/completions "));
    let n: usize = h
        .lines()
        .find_map(|l| l.strip_prefix("Content-Length: "))
        .unwrap()
        .parse()
        .unwrap();
    let mut b = vec![0; n];
    s.read_exact(&mut b).unwrap();
    serde_json::from_slice(&b).unwrap()
}
fn respond(s: &mut TcpStream, text: &str) {
    let body = format!(
        "data: {}\n\ndata: {}\n\ndata: [DONE]\n\n",
        json!({"model":"fixture-model","choices":[{"index":0,"delta":{"content":text},"finish_reason":null}]}),
        json!({"model":"fixture-model","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]})
    );
    s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).unwrap();
}
fn tool() -> String {
    json!({"tool_call":{"tool_id":"agentmage.workspace.read-file","tool_version":"1.0.0","arguments":{"schema_version":1,"paths":[["src","lib.rs"]],"query":null,"byte_offset":0,"byte_count":256,"encoding":"utf8","limits":{"files":128,"input_bytes":4194304,"depth":16,"matches":256,"output_bytes":1048576},"call_depth":0}}}).to_string()
}

#[test]
fn endpoint_http_admission_authentication_consent_and_binding_never_dispatch() {
    let (upstream, endpoint) = upstream();
    let calls = Arc::new(AtomicUsize::new(0));
    let api = Api::new(endpoint.clone(), calls.clone());
    let r = api.request();
    assert_eq!(r["mode"], "ephemeral_read_only");
    assert!(r.get("model_profile").is_none());
    assert!(r["run_id"].as_str().unwrap().starts_with("endpoint-run-"));
    for token in [CLIENT, "invalid-token"] {
        assert_eq!(
            api.call("GET", "/v1/code/endpoint/request", token, None).0,
            if token == CLIENT { 403 } else { 401 }
        );
    }
    for path in [
        "/v1/settings",
        "/v1/code/workspace",
        "/v1/code/runtime/prepare",
        "/v1/chat",
    ] {
        assert_eq!(api.call("GET", path, CONTROL, None).0, 404);
    }
    assert_eq!(api.advance(&r, Value::Null, false).0, 403);
    for path in ["request", "events", "advance", "cancel", "release"] {
        assert_eq!(
            api.call(
                "POST",
                &format!("/v1/code/endpoint/{path}"),
                CLIENT,
                Some(json!({}))
            )
            .0,
            403
        );
    }
    for action in ["advance", "events", "cancel", "release"] {
        for malformed in [
            json!([]),
            json!({}),
            json!({"run_id":r["run_id"],"request_sha256":r["request_sha256"],"extra":"untrusted"}),
        ] {
            assert_eq!(
                api.call(
                    "POST",
                    &format!("/v1/code/endpoint/{action}"),
                    CONTROL,
                    Some(malformed)
                )
                .0,
                400
            );
        }
    }
    assert_eq!(
        api.call(
            "POST",
            "/v1/code/endpoint/events",
            CONTROL,
            Some(json!({"run_id":r["run_id"],"request_sha256":r["request_sha256"],"after":1}))
        )
        .0,
        409
    );
    let mut bad = r.clone();
    bad["request_sha256"] = json!("0".repeat(64));
    for action in ["events", "release", "cancel"] {
        assert_eq!(api.bind(action, &bad).0, 409);
    }
    assert_eq!(api.advance(&bad, Value::Null, true).0, 409);
    let mut bad_id = r.clone();
    bad_id["run_id"] = json!("foreign-run");
    assert_eq!(api.advance(&bad_id, Value::Null, true).0, 409);
    assert_eq!(
        api.bind("release", &r).1["error"],
        "code.endpoint.still-running"
    );
    assert_eq!(api.bind("events", &r).1["total"], 0);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        upstream.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    let api2 = Api::new(endpoint, calls);
    let r2 = api2.request();
    assert_ne!(r["run_id"], r2["run_id"]);
    assert_ne!(r["session_id"], r2["session_id"]);
    assert_eq!(api2.advance(&r, Value::Null, true).0, 409);
}

#[test]
fn endpoint_http_keeps_original_approval_events_terminal_and_release() {
    let (upstream, endpoint) = upstream();
    let calls = Arc::new(AtomicUsize::new(0));
    let worker = thread::spawn(move || {
        let mut s = accept(&upstream);
        let body = read_upstream(&mut s);
        assert_eq!(body["model"], "fixture-model");
        respond(&mut s, &tool());
        drop(s);
        let mut s = accept(&upstream);
        let body = read_upstream(&mut s);
        assert!(body.to_string().contains("original retained boundary"));
        respond(&mut s, "unverified synthetic completion");
        drop(s);
        upstream
    });
    let api = Api::new(endpoint, calls.clone());
    let r = api.request();
    let (code, wait) = api.advance(&r, Value::Null, true);
    assert_eq!(code, 200);
    assert_eq!(wait["state"], "awaiting_approval");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(api.advance(&r, Value::Null, true), (200, wait.clone()));
    let c = &wait["challenge"];
    let approval = json!({"schema_version":CONTRACT_SCHEMA_VERSION,"run_id":c["run_id"],"approval_id":c["approval_id"],"disposition":"allow","challenge_sha256":c["challenge_sha256"],"grant_id":c["proposed_grant_id"],"selection":null});
    let mut wrong = approval.clone();
    wrong["challenge_sha256"] = json!("0".repeat(64));
    assert_eq!(api.advance(&r, wrong, true).0, 409);
    assert_eq!(api.advance(&r, approval.clone(), false).0, 403);
    assert_eq!(api.advance(&r, Value::Null, true), (200, wait));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let (code, terminal) = api.advance(&r, approval, true);
    assert_eq!(code, 200);
    assert_eq!(terminal["state"], "terminal");
    assert_eq!(terminal["outcome"]["outcome"]["state"], "FAILED");
    assert_eq!(terminal["outcome"]["outcome"]["model_call_count"], 2);
    assert_eq!(terminal["outcome"]["outcome"]["tool_call_count"], 1);
    assert_eq!(api.advance(&r, Value::Null, true), (200, terminal));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let mut after = 0;
    let mut events = Vec::new();
    loop {
        let (code, page) = api.call(
            "POST",
            "/v1/code/endpoint/events",
            CONTROL,
            Some(json!({"run_id":r["run_id"],"request_sha256":r["request_sha256"],"after":after})),
        );
        assert_eq!(code, 200);
        events.extend(page["events"].as_array().unwrap().iter().cloned());
        after = page["next"].as_u64().unwrap();
        if after == page["total"].as_u64().unwrap() {
            break;
        }
    }
    assert!(!events.is_empty());
    let mut sequence = agentmage_kernel_engine::runtime_event::RuntimeEventSequence::new();
    for e in events {
        let e: RuntimeEvent = serde_json::from_value(e).unwrap();
        sequence.push(&e).unwrap();
    }
    assert_eq!(api.bind("release", &r).0, 200);
    for action in ["events", "release"] {
        assert_eq!(api.bind(action, &r).1["error"], "code.endpoint.not-held");
    }
    assert_eq!(
        api.advance(&r, Value::Null, true).1["error"],
        "code.endpoint.not-held"
    );
    assert_eq!(
        worker.join().unwrap().accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn endpoint_http_cancel_before_dispatch_is_latched_and_needs_no_disclosure() {
    let (upstream, endpoint) = upstream();
    let calls = Arc::new(AtomicUsize::new(0));
    let api = Api::new(endpoint, calls.clone());
    let r = api.request();
    assert_eq!(
        api.bind("cancel", &r),
        (200, json!({"cancellation":"requested"}))
    );
    let (code, terminal) = api.advance(&r, Value::Null, false);
    assert_eq!(code, 200);
    assert_eq!(terminal["state"], "terminal");
    assert_eq!(terminal["outcome"]["outcome"]["state"], "CANCELLED");
    assert_eq!(api.advance(&r, Value::Null, true), (200, terminal));
    assert_eq!(api.bind("release", &r).0, 200);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        upstream.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn endpoint_http_cancel_and_shutdown_reach_busy_dispatch_and_join_workers() {
    for shutdown in [false, true] {
        let (upstream, endpoint) = upstream();
        let calls = Arc::new(AtomicUsize::new(0));
        let api = Arc::new(Api::new(endpoint, calls.clone()));
        let r = api.request();
        let (entered, ready) = mpsc::sync_channel(1);
        let upstream_worker = thread::spawn(move || {
            let mut s = accept(&upstream);
            read_upstream(&mut s);
            entered.send(()).unwrap();
            let mut b = [0];
            assert_eq!(s.read(&mut b).unwrap(), 0);
            upstream
        });
        let runner = api.clone();
        let request = r.clone();
        let running = thread::spawn(move || runner.advance(&request, Value::Null, true));
        ready.recv_timeout(Duration::from_secs(4)).unwrap();
        if shutdown {
            api.stop.store(true, Ordering::Release);
        } else {
            assert_eq!(api.bind("events", &r).0, 409);
            assert_eq!(api.bind("release", &r).0, 409);
            assert_eq!(api.bind("cancel", &r).0, 200);
        }
        let (code, terminal) = running.join().unwrap();
        assert_eq!(code, 200);
        assert_eq!(terminal["outcome"]["outcome"]["state"], "CANCELLED");
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            upstream_worker.join().unwrap().accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        drop(api);
    }
}

#[test]
fn endpoint_http_rejects_foreign_factory_identity_before_serving_or_dispatch() {
    let (upstream, endpoint) = upstream();
    let calls = Arc::new(AtomicUsize::new(0));
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let result = serve_coding_endpoint(
        listener,
        ApiTokens::new(CLIENT.into(), CONTROL.into()).unwrap(),
        Arc::new(AtomicBool::new(false)),
        |_, session| {
            Ok(compose(
                RuntimeRunId::from_raw("caller-chosen-id"),
                session,
                endpoint,
                calls.clone(),
            ))
        },
    );
    assert!(matches!(result, Err(ApiError::Configuration)));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        upstream.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn endpoint_http_original_admission_serves_same_owner_and_original_controls() {
    let executions = Arc::new(AtomicUsize::new(0));
    let model = TcpListener::bind("127.0.0.1:0").unwrap();
    model.set_nonblocking(true).unwrap();
    let endpoint =
        LocalChatEndpoint::parse(&format!("http://{}/v1", model.local_addr().unwrap())).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = stop.clone();
    let count = executions.clone();
    let admissions = Arc::new(AtomicUsize::new(0));
    let observed = admissions.clone();
    let worker = thread::spawn(move || {
        serve_admitted_coding_endpoint(
            listener,
            ApiTokens::new(CLIENT.into(), CONTROL.into()).unwrap(),
            worker_stop,
            CodingEndpointObservationAccess::ControlOnly,
            |id, session, runs| {
                observed.fetch_add(1, Ordering::SeqCst);
                let input = compose(id, session, endpoint, count);
                let request = input.request.clone();
                runs.admit(
                    agentmage_host::coding_endpoint_owner::CodingEndpointAdmission {
                        request: input.request,
                        connection: input.connection,
                        registry: input.registry,
                        sources: &input.sources,
                        source_budget: input.source_budget,
                        include_usage: input.include_usage,
                        tool_boundary: input.tool_boundary,
                        verifier: input.verifier,
                        clock: input.clock,
                    },
                )
                .map_err(|_| ApiError::Configuration)?;
                Ok(request)
            },
        )
    });
    let api = Api {
        address,
        stop,
        worker: Some(worker),
    };
    let request = api.request();
    assert_eq!(admissions.load(Ordering::SeqCst), 1);
    assert_eq!(
        api.call("GET", "/v1/code/endpoint/request", CLIENT, None).0,
        403
    );
    assert_eq!(api.advance(&request, Value::Null, false).0, 403);
    assert_eq!(api.bind("cancel", &request).0, 200);
    assert_eq!(api.advance(&request, Value::Null, false).0, 200);
    assert_eq!(api.bind("release", &request).0, 200);
    assert_eq!(admissions.load(Ordering::SeqCst), 1);
    assert_eq!(executions.load(Ordering::SeqCst), 0);
    assert!(matches!(model.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
}

#[test]
fn endpoint_http_original_admission_refuses_unheld_or_substituted_request_before_serving() {
    for substituted in [false, true] {
        let model = TcpListener::bind("127.0.0.1:0").unwrap();
        model.set_nonblocking(true).unwrap();
        let endpoint =
            LocalChatEndpoint::parse(&format!("http://{}/v1", model.local_addr().unwrap()))
                .unwrap();
        let executions = Arc::new(AtomicUsize::new(0));
        let result = serve_admitted_coding_endpoint::<Boundary, Verifier, Clock>(
            TcpListener::bind("127.0.0.1:0").unwrap(),
            ApiTokens::new(CLIENT.into(), CONTROL.into()).unwrap(),
            Arc::new(AtomicBool::new(false)),
            CodingEndpointObservationAccess::ControlOnly,
            |id, session, _runs| {
                let input = compose(
                    if substituted {
                        RuntimeRunId::from_raw("foreign-run")
                    } else {
                        id
                    },
                    session,
                    endpoint,
                    executions.clone(),
                );
                // A sealed request cannot reconstruct a held coordinator.
                Ok(input.request)
            },
        );
        assert!(matches!(result, Err(ApiError::Configuration)));
        assert_eq!(executions.load(Ordering::SeqCst), 0);
        assert!(matches!(model.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
    }
}
