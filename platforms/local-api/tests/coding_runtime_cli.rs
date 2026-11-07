// SPDX-License-Identifier: Apache-2.0
//! Actual terminal consumer against a synthetic HTTP peer; not a native run.
use agentmage_kernel_contracts::*;
use agentmage_kernel_engine::runtime_coordinator::seal_runtime_approval_challenge;
use agentmage_local_api::ApiTokens;
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    net::{SocketAddr, TcpListener},
    path::PathBuf,
    process::{Command, Output, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "agentmage-runtime-terminal-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        ApiTokens::initialize(&root.join("auth")).unwrap();
        Self { root }
    }
    fn cli(&self, address: SocketAddr, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_agentmage-code"))
            .arg(address.to_string())
            .arg(self.root.join("auth"))
            .args(args)
            .stdin(Stdio::null())
            .output()
            .unwrap()
    }
    fn tty(&self, address: SocketAddr, args: &[&str], answer: &str) -> Value {
        let output = Command::new("python3")
            .args([
                "-c",
                PTY,
                answer,
                env!("CARGO_BIN_EXE_agentmage-code"),
                &address.to_string(),
                self.root.join("auth").to_str().unwrap(),
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
    fn peer(&self, replies: Vec<Value>) -> Peer {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let token = fs::read_to_string(self.root.join("auth/control.token")).unwrap();
        let worker = {
            let stop = stop.clone();
            thread::spawn(move || {
                let mut observed = Vec::new();
                let deadline = Instant::now() + Duration::from_secs(15);
                while !stop.load(Ordering::Relaxed) {
                    assert!(Instant::now() < deadline, "terminal peer timed out");
                    let (mut socket, _) = match listener.accept() {
                        Ok(v) => v,
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(5));
                            continue;
                        }
                        Err(e) => panic!("{e}"),
                    };
                    socket
                        .set_read_timeout(Some(Duration::from_secs(3)))
                        .unwrap();
                    let mut header = Vec::new();
                    while !header.ends_with(b"\r\n\r\n") {
                        let mut byte = [0];
                        socket.read_exact(&mut byte).unwrap();
                        header.push(byte[0]);
                        assert!(header.len() < 8192);
                    }
                    let header = String::from_utf8(header).unwrap();
                    assert!(header.contains(&format!("\r\nAuthorization: Bearer {token}\r\n")));
                    let length: usize = header
                        .lines()
                        .find_map(|s| s.strip_prefix("Content-Length: "))
                        .unwrap()
                        .parse()
                        .unwrap();
                    let mut body = vec![0; length];
                    socket.read_exact(&mut body).unwrap();
                    observed.push(json!({"route":header.lines().next().unwrap(),"body":if body.is_empty(){Value::Null}else{serde_json::from_slice(&body).unwrap()}}));
                    if replies.get(observed.len() - 1) == Some(&json!({"test_peer_close":true})) {
                        continue;
                    }
                    let reply = serde_json::to_vec(
                        replies
                            .get(observed.len() - 1)
                            .unwrap_or(&json!({"unexpected":true})),
                    )
                    .unwrap();
                    socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",reply.len()).as_bytes()).unwrap();
                    socket.write_all(&reply).unwrap();
                }
                observed
            })
        };
        Peer {
            address,
            stop,
            worker: Some(worker),
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
struct Peer {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<Vec<Value>>>,
}
impl Peer {
    fn finish(mut self) -> Vec<Value> {
        self.stop.store(true, Ordering::Relaxed);
        self.worker.take().unwrap().join().unwrap()
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn challenge() -> RuntimeApprovalChallenge {
    // All values are synthetic. The kernel seals this complete presentation;
    // changing any argument below without resealing must make the client refuse.
    let arguments = ContractPayload {
        schema: SchemaReference {
            schema_id: SchemaId::from_raw("fixture.read"),
            schema_version: 1,
            schema_sha256: "a".repeat(64),
        },
        media_type: "application/json".into(),
        bytes: b"{}".to_vec(),
        sha256: "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a".into(),
    };
    seal_runtime_approval_challenge(RuntimeApprovalChallenge {
        schema_version: CONTRACT_SCHEMA_VERSION,
        run_id: RuntimeRunId::from_raw("run"),
        task_id: TaskId::from_raw("task"),
        turn_id: RuntimeTurnId::from_raw("turn"),
        operation_id: RuntimeOperationId::from_raw("operation"),
        tool_call_id: ToolCallId::from_raw("call"),
        approval_id: ApprovalId::from_raw("approval"),
        proposed_grant_id: GrantId::from_raw("grant"),
        operation: GrantOperation::WorkspaceRead,
        presentation: RuntimeApprovalPresentation {
            tool_id: ToolId::from_raw("fixture.read"),
            tool_version: "1.0.0".into(),
            display_name: "Synthetic read".into(),
            risk_level: ToolRiskLevel::Low,
            target_scope: "one synthetic source".into(),
            arguments,
            declared_effects: vec![OperationBinding::new(GrantOperation::WorkspaceRead)],
            single_use: true,
            timeout_ms: 1000,
        },
        preview_sha256: "d".repeat(64),
        expires_at_epoch_ms: 4_000_000_000_000,
        challenge_sha256: "0".repeat(64),
    })
    .unwrap()
}
fn step(approval: Value) -> Value {
    json!({"run_id":"run","request_sha256":"b".repeat(64),"events":[],"artifacts":[],"approval":approval,"outcome":null})
}
#[test]
fn model_start_requires_a_terminal_and_preserves_the_prepared_request() {
    let f = Fixture::new();
    let request: RuntimeRunRequest = agentmage_host::runtime_transport::decode_exact(
        include_bytes!("../../../fixtures/runtime-producer/v5/run-request.json"),
    )
    .unwrap();
    let encoded = serde_json::to_string(&request).unwrap();
    let mut reply = step(Value::Null);
    reply["run_id"] = json!(request.run_id);
    reply["request_sha256"] = json!(request.request_sha256);
    let peer = f.peer(vec![reply.clone()]);
    assert!(
        !f.cli(peer.address, &["runtime-start", &encoded])
            .status
            .success()
    );
    assert!(peer.finish().is_empty());
    let peer = f.peer(vec![reply]);
    let result = f.tty(peer.address, &["runtime-start", &encoded], "approve");
    assert_eq!(result["status"], 0);
    let calls = peer.finish();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0]["body"]["request"], json!(request));
    assert_eq!(calls[0]["body"]["approve_model_context"], true);
}
#[test]
fn piped_prepare_and_approval_never_send_authority() {
    let f = Fixture::new();
    let peer = f.peer(vec![json!({"expected_entry_sha256":"a".repeat(64)})]);
    assert!(
        !f.cli(peer.address, &["runtime-prepare", "Fix add"])
            .status
            .success()
    );
    assert_eq!(peer.finish().len(), 1);
    let peer = f.peer(vec![step(json!(challenge()))]);
    assert!(
        !f.cli(peer.address, &["runtime-approve", "run", &"b".repeat(64)])
            .status
            .success()
    );
    let calls = peer.finish();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0]["body"]["response"], Value::Null);
}
#[test]
fn exact_typed_approval_and_denial_use_only_the_current_host_challenge() {
    let f = Fixture::new();
    let challenge = challenge();
    let peer = f.peer(vec![step(json!(challenge)), step(Value::Null)]);
    let output = f.tty(
        peer.address,
        &["runtime-approve", "run", &"b".repeat(64)],
        "approve",
    );
    assert_eq!(output["status"], 0);
    let calls = peer.finish();
    assert_eq!(calls.len(), 2);
    assert_eq!(
        calls[1]["body"]["response"]["challenge_sha256"],
        challenge.challenge_sha256
    );
    assert_eq!(calls[1]["body"]["response"]["grant_id"], "grant");
    assert_eq!(calls[1]["body"]["response"]["disposition"], "allow");
    assert!(
        output["transcript"]
            .as_str()
            .unwrap()
            .contains("single_use")
    );
    assert!(
        output["transcript"]
            .as_str()
            .unwrap()
            .contains("arguments_json")
    );
    let peer = f.peer(vec![step(json!(challenge)), step(Value::Null)]);
    assert!(
        f.cli(peer.address, &["runtime-deny", "run", &"b".repeat(64)])
            .status
            .success()
    );
    let calls = peer.finish();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[1]["body"]["response"]["disposition"], "deny");
    assert_eq!(calls[1]["body"]["response"]["grant_id"], Value::Null);
}
#[test]
fn modified_challenge_or_foreign_step_never_sends_approval() {
    let f = Fixture::new();
    let mut bad = json!(challenge());
    bad["presentation"]["target_scope"] = json!("changed");
    for reply in [step(bad), {
        let mut foreign = step(json!(challenge()));
        foreign["run_id"] = json!("foreign");
        foreign
    }] {
        let peer = f.peer(vec![reply]);
        let output = f.tty(
            peer.address,
            &["runtime-approve", "run", &"b".repeat(64)],
            "approve",
        );
        assert_ne!(output["status"], 0);
        assert_eq!(peer.finish().len(), 1);
        assert_eq!(output["answered"], 0);
    }
}
#[test]
fn cancellation_uses_the_observed_job_revision_without_direct_cancel() {
    let f = Fixture::new();
    let status = json!({"schema_version":1,"run_id":"run","request_sha256":"b".repeat(64),"job":{"job_id":"run","revision":7,"phase":"running","head_sha256":"a".repeat(64),"cancellation_requested":false}});
    let peer = f.peer(vec![
        status,
        json!({"decision":{"decision":"applied","revision":8,"phase":"cancelling"}}),
    ]);
    assert!(
        f.cli(
            peer.address,
            &["runtime-control", "run", &"b".repeat(64), "cancel"]
        )
        .status
        .success()
    );
    let calls = peer.finish();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0]["route"], "POST /v1/code/runtime/job HTTP/1.1");
    assert_eq!(calls[1]["route"], "POST /v1/code/runtime/control HTTP/1.1");
    assert_eq!(calls[1]["body"]["request"]["observed_revision"], 7);
    assert_eq!(calls[1]["body"]["request"]["action"], "cancel");
    assert!(
        calls[1]["body"]["request"]["request_id"]
            .as_str()
            .unwrap()
            .starts_with("terminal-")
    );
}
// These are newly constructed synthetic transport records, sealed by the
// canonical kernel. They are neither native tool execution nor a model score.
fn driven_request() -> RuntimeRunRequest {
    let mut request: RuntimeRunRequest = agentmage_host::runtime_transport::decode_exact(
        include_bytes!("../../../fixtures/runtime-producer/v5/run-request.json"),
    )
    .unwrap();
    request
        .task
        .constraints
        .retain(|s| !s.starts_with("Recipe"));
    agentmage_kernel_engine::runtime_coordinator::seal_runtime_run_request(request).unwrap()
}
fn driven_selection(r: &RuntimeRunRequest) -> Value {
    json!({"version":1,"activation":"coding-development-v1","scenario":"failed-test-repair",
        "profile_id":r.model_profile.profile_id,"workspace_id":r.workspace_id,
        "workspace_root":"/synthetic-disposable","expected_entry_sha256":"a".repeat(64),
        "resume":false,"requires_control":true,"preauthorization":false,"chat_endpoint_used":false})
}
struct Stream {
    request: RuntimeRunRequest,
    events: Vec<RuntimeEvent>,
}
impl Stream {
    fn new(request: &RuntimeRunRequest) -> Self {
        Self {
            request: request.clone(),
            events: Vec::new(),
        }
    }
    fn event(
        &mut self,
        kind: RuntimeEventKind,
        turn: Option<&RuntimeTurnId>,
        operation: Option<&RuntimeOperationId>,
    ) -> RuntimeEvent {
        let previous = self.events.last();
        let event = agentmage_kernel_engine::runtime_event::seal_runtime_event(RuntimeEvent {
            schema_version: CONTRACT_SCHEMA_VERSION,
            event_id: RuntimeEventId::from_raw(format!("http-event-{}", self.events.len())),
            run_id: self.request.run_id.clone(),
            session_id: self.request.session_id.clone(),
            task_id: self.request.task.task_id.clone(),
            turn_id: turn.cloned(),
            operation_id: operation.cloned(),
            correlation_id: CorrelationId::from_raw("http-correlation"),
            causation_event_id: previous.map(|e| e.event_id.clone()),
            sequence: self.events.len() as u64,
            occurred_at_epoch_ms: 1000 + self.events.len() as u64,
            sensitivity: ContextSensitivity::Internal,
            retention: RuntimeEventRetention {
                kind: RuntimeEventRetentionKind::Ephemeral,
                expires_at_epoch_ms: None,
            },
            persistence: agentmage_kernel_engine::runtime_event::runtime_event_persistence(&kind),
            policy_id: self.request.policy_id.clone(),
            payload_reference: if let RuntimeEventKind::ArtifactCreated { artifact_id, .. } = &kind
            {
                Some(RuntimePayloadReference {
                    artifact_id: artifact_id.clone(),
                    sha256: "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
                        .into(),
                    byte_size: 2,
                    media_type: "application/json".into(),
                })
            } else {
                None
            },
            kind,
            previous_event_sha256: previous
                .map_or_else(|| "0".repeat(64), |e| e.event_sha256.clone()),
            event_sha256: "0".repeat(64),
        })
        .unwrap();
        self.events.push(event.clone());
        event
    }
    fn terminal(&mut self, state: AgentStateKind) -> Value {
        let prior = self.events.last().unwrap();
        let outcome = agentmage_kernel_engine::runtime_coordinator::seal_runtime_outcome(
            RuntimeOutcome {
                schema_version: CONTRACT_SCHEMA_VERSION,
                run_id: self.request.run_id.clone(),
                session_id: self.request.session_id.clone(),
                task_id: self.request.task.task_id.clone(),
                request_sha256: self.request.request_sha256.clone(),
                state,
                turn_count: 1,
                model_call_count: 1,
                tool_call_count: 1,
                prior_event_id: prior.event_id.clone(),
                prior_event_sha256: prior.event_sha256.clone(),
                evidence: if state.is_success() {
                    vec![EvidenceReference {
                        schema_version: CONTRACT_SCHEMA_VERSION,
                        evidence_id: EvidenceId::from_raw("synthetic-evidence"),
                        kind: EvidenceKind::Observation,
                        source_id: "synthetic".into(),
                        object_id: "no-op-observation".into(),
                        fragment: None,
                        content_sha256: "a".repeat(64),
                        observed_revision: None,
                    }]
                } else {
                    Vec::new()
                },
                receipt_ids: Vec::new(),
                unresolved_codes: Vec::new(),
                output: None,
                answer_evidence: None,
                outcome_sha256: "0".repeat(64),
            },
            &self.request,
        )
        .unwrap();
        self.event(
            RuntimeEventKind::RunTerminal {
                state,
                outcome_sha256: outcome.outcome_sha256.clone(),
            },
            None,
            None,
        );
        json!(outcome)
    }
    fn step(
        &self,
        events: Vec<RuntimeEvent>,
        approval: Option<RuntimeApprovalChallenge>,
        outcome: Value,
    ) -> Value {
        json!({"run_id":self.request.run_id,"request_sha256":self.request.request_sha256,"events":events,"approval":approval,"artifacts":[],"outcome":outcome})
    }
}
fn driven_steps(r: &RuntimeRunRequest, allow: bool) -> (Value, Value) {
    let mut c = challenge();
    driven_steps_with_challenge(r, allow, &mut c)
}
fn driven_steps_with_challenge(
    r: &RuntimeRunRequest,
    allow: bool,
    c: &mut RuntimeApprovalChallenge,
) -> (Value, Value) {
    c.run_id = r.run_id.clone();
    c.task_id = r.task.task_id.clone();
    let c = seal_runtime_approval_challenge(c.clone()).unwrap();
    let mut s = Stream::new(r);
    s.event(
        RuntimeEventKind::RunStarted {
            request_sha256: r.request_sha256.clone(),
        },
        None,
        None,
    );
    s.event(RuntimeEventKind::TurnStarted, Some(&c.turn_id), None);
    s.event(
        RuntimeEventKind::ModelRequested {
            model_run_id: ModelRunId::from_raw("http-model"),
            request_sha256: "a".repeat(64),
        },
        Some(&c.turn_id),
        None,
    );
    s.event(
        RuntimeEventKind::ModelCompleted {
            model_run_id: ModelRunId::from_raw("http-model"),
            result_sha256: "b".repeat(64),
        },
        Some(&c.turn_id),
        None,
    );
    s.event(
        RuntimeEventKind::ToolRequested {
            tool_call_id: c.tool_call_id.clone(),
            arguments_sha256: c.presentation.arguments.sha256.clone(),
        },
        Some(&c.turn_id),
        Some(&c.operation_id),
    );
    s.event(
        RuntimeEventKind::PermissionRequested {
            approval_id: c.approval_id.clone(),
            operation: c.operation,
            preview_sha256: c.preview_sha256.clone(),
            expires_at_epoch_ms: c.expires_at_epoch_ms,
        },
        Some(&c.turn_id),
        Some(&c.operation_id),
    );
    let first = s.step(s.events.clone(), Some(c.clone()), Value::Null);
    let split = s.events.len();
    s.event(
        RuntimeEventKind::PermissionDecided {
            approval_id: c.approval_id.clone(),
            disposition: if allow {
                RuntimePermissionDisposition::Allow
            } else {
                RuntimePermissionDisposition::Deny
            },
            grant_id: allow.then_some(c.proposed_grant_id.clone()),
            decision_sha256: "d".repeat(64),
        },
        Some(&c.turn_id),
        Some(&c.operation_id),
    );
    if allow {
        s.event(
            RuntimeEventKind::ToolStarted {
                tool_call_id: c.tool_call_id.clone(),
                authority_sha256: "e".repeat(64),
            },
            Some(&c.turn_id),
            Some(&c.operation_id),
        );
        s.event(
            RuntimeEventKind::ToolCompleted {
                tool_call_id: c.tool_call_id.clone(),
                receipt_id: ReceiptId::from_raw("synthetic-receipt"),
                result_sha256: "f".repeat(64),
            },
            Some(&c.turn_id),
            Some(&c.operation_id),
        );
    }
    s.event(
        RuntimeEventKind::TurnCompleted {
            outcome_sha256: "1".repeat(64),
        },
        Some(&c.turn_id),
        None,
    );
    let outcome = s.terminal(if allow {
        AgentStateKind::Blocked
    } else {
        AgentStateKind::Declined
    });
    let last = s.step(s.events[split..].to_vec(), None, outcome);
    (first, last)
}

#[test]
fn connected_driver_checks_events_asks_exactly_and_reports_a_verified_non_success() {
    let f = Fixture::new();
    let r = driven_request();
    for (allow, scenario) in [
        (false, "failed-test-repair"),
        (true, "failed-test-repair"),
        (false, "python-project"),
        (true, "python-project"),
    ] {
        let (first, last) = driven_steps(&r, allow);
        let mut selection = driven_selection(&r);
        selection["scenario"] = json!(scenario);
        let peer = f.peer(vec![
            selection,
            json!(r),
            first,
            last,
            Value::Null,
            Value::Null,
            json!({"released":true}),
        ]);
        let output = f.tty(
            peer.address,
            &["runtime-run", "30", &r.task.objective],
            if allow { "approve" } else { "deny-tools" },
        );
        assert_ne!(
            output["status"], 0,
            "A verified BLOCKED/DECLINED run must not exit successfully: {output}"
        );
        assert_eq!(output["answered"], 3, "{output}");
        let text = output["transcript"].as_str().unwrap();
        assert!(text.contains("verified-outcome"), "{text}");
        assert!(
            text.contains(if allow { "BLOCKED" } else { "DECLINED" }),
            "{text}"
        );
        let calls = peer.finish();
        assert_eq!(calls.len(), 7, "{calls:?}");
        assert_eq!(calls[2]["body"]["request"], json!(r));
        assert_eq!(
            calls[3]["body"]["response"]["disposition"],
            if allow { "allow" } else { "deny" }
        );
        assert_eq!(
            calls[3]["body"]["response"]["grant_id"],
            if allow { json!("grant") } else { Value::Null }
        );
        assert_eq!(calls[3]["body"]["after_event_cursor"]["sequence"], 5);
        assert_eq!(calls[6]["route"], "POST /v1/code/runtime/release HTTP/1.1");
    }
}

#[test]
fn connected_driver_drains_large_inspection_before_accepting_a_typed_refusal() {
    let f = Fixture::new();
    let r = driven_request();
    let task = "\u{1}".repeat(16 * 1024);
    let peer = f.peer(vec![driven_selection(&r)]);
    let output = f.tty(peer.address, &["runtime-run", "30", &task], "slow-deny");
    assert_ne!(output["status"], 0, "{output}");
    assert_eq!(output["answered"], 1);
    assert!(output["prompt_bytes"][0].as_u64().unwrap() >= 64 * 1024);
    assert_eq!(peer.finish().len(), 1);
}

#[test]
fn connected_driver_drains_large_start_and_tool_reviews_before_exact_decisions() {
    let f = Fixture::new();
    let mut r = driven_request();
    r.task.constraints.extend(vec!["S".repeat(4096); 16]);
    let r = agentmage_kernel_engine::runtime_coordinator::seal_runtime_run_request(r).unwrap();
    let mut c = challenge();
    // Original synthetic JSON string, with its independently calculated digest.
    c.presentation.arguments.bytes = format!("\"{}\"", "T".repeat(8192)).into_bytes();
    c.presentation.arguments.sha256 =
        "cd1a2f6baf0c0464a3c8fccde1361e3715148e3022a0cf16846045b20c86cd30".into();
    for allow in [false, true] {
        let (first, last) = driven_steps_with_challenge(&r, allow, &mut c);
        let peer = f.peer(vec![
            driven_selection(&r),
            json!(r),
            first,
            last,
            Value::Null,
            Value::Null,
            json!({"released":true}),
        ]);
        let output = f.tty(
            peer.address,
            &["runtime-run", "30", &r.task.objective],
            if allow {
                "slow-approve"
            } else {
                "slow-deny-tools"
            },
        );
        assert_ne!(output["status"], 0);
        assert_eq!(output["answered"], 3, "{output}");
        for index in [1, 2] {
            assert!(output["prompt_bytes"][index].as_u64().unwrap() >= 64 * 1024);
        }
        assert!(
            output["transcript"]
                .as_str()
                .unwrap()
                .contains("verified-outcome")
        );
        let calls = peer.finish();
        assert_eq!(calls.len(), 7);
        assert_eq!(calls[2]["body"]["request"], json!(r));
        assert_eq!(
            calls[3]["body"]["response"]["disposition"],
            if allow { "allow" } else { "deny" }
        );
    }
}

#[test]
fn connected_driver_terminal_eof_and_overflow_send_no_decision() {
    let f = Fixture::new();
    let r = driven_request();
    for mode in ["eof-inspect", "eof-start", "eof-tool", "overflow-tool"] {
        let (first, _) = driven_steps(&r, true);
        let replies = match mode {
            "eof-inspect" => vec![driven_selection(&r)],
            "eof-start" => vec![driven_selection(&r), json!(r), json!({"released":true})],
            _ => vec![
                driven_selection(&r),
                json!(r),
                first,
                json!({"released":true}),
            ],
        };
        let expected_calls = replies.len();
        let peer = f.peer(replies);
        let output = f.tty(
            peer.address,
            &["runtime-run", "30", &r.task.objective],
            mode,
        );
        assert_ne!(output["status"], 0, "{output}");
        let calls = peer.finish();
        assert_eq!(calls.len(), expected_calls, "{calls:?}");
        assert!(
            calls
                .iter()
                .all(|c| c["route"] != "POST /v1/code/runtime/advance HTTP/1.1")
        );
        if mode.ends_with("tool") {
            assert!(
                output["transcript"]
                    .as_str()
                    .unwrap()
                    .contains("cli.runtime.approval_unavailable")
            );
        }
    }
}

#[test]
fn connected_driver_stop_interrupts_backpressured_prompt_writes() {
    let f = Fixture::new();
    let r = driven_request();
    let task = "\u{1}".repeat(16 * 1024);
    for mode in ["stalled-signal", "stalled-deadline"] {
        let peer = f.peer(vec![driven_selection(&r)]);
        let began = Instant::now();
        let output = f.tty(peer.address, &["runtime-run", "1", &task], mode);
        assert_ne!(output["status"], 0, "{output}");
        assert_eq!(output["answered"], 0);
        assert!(began.elapsed() < Duration::from_secs(5));
        assert_eq!(peer.finish().len(), 1);
    }
}

#[test]
fn connected_driver_refuses_event_tampering_and_never_retries_an_uncertain_approval() {
    let f = Fixture::new();
    let r = driven_request();
    let (mut bad, _) = driven_steps(&r, true);
    bad["events"][0]["event_sha256"] = json!("f".repeat(64));
    let peer = f.peer(vec![
        driven_selection(&r),
        json!(r),
        bad,
        json!({"released":true}),
    ]);
    let output = f.tty(
        peer.address,
        &["runtime-run", "30", &r.task.objective],
        "approve",
    );
    assert_ne!(output["status"], 0);
    assert_eq!(output["answered"], 2);
    let calls = peer.finish();
    assert_eq!(calls.len(), 4);
    assert!(
        calls
            .iter()
            .all(|c| c["route"] != "POST /v1/code/runtime/advance HTTP/1.1")
    );
    let (first, _) = driven_steps(&r, true);
    let peer = f.peer(vec![
        driven_selection(&r),
        json!(r),
        first,
        json!({"test_peer_close":true}),
        json!({"released":true}),
    ]);
    let output = f.tty(
        peer.address,
        &["runtime-run", "30", &r.task.objective],
        "approve",
    );
    assert_ne!(output["status"], 0);
    assert_eq!(output["answered"], 3);
    assert!(
        !output["transcript"]
            .as_str()
            .unwrap()
            .contains("verified-outcome")
    );
    let calls = peer.finish();
    assert_eq!(calls.len(), 5);
    assert_eq!(
        calls
            .iter()
            .filter(|c| c["route"] == "POST /v1/code/runtime/advance HTTP/1.1")
            .count(),
        1
    );
}
#[test]
fn connected_driver_refuses_pipes_and_preparation_substitution_before_model_start() {
    let f = Fixture::new();
    let r = driven_request();
    let peer = f.peer(vec![]);
    assert!(
        !f.cli(peer.address, &["runtime-run", "30", &r.task.objective])
            .status
            .success()
    );
    assert!(peer.finish().is_empty());
    let mut wrong = r.clone();
    wrong.task.objective = "Different task".into();
    let wrong =
        agentmage_kernel_engine::runtime_coordinator::seal_runtime_run_request(wrong).unwrap();
    let peer = f.peer(vec![
        driven_selection(&r),
        json!(wrong),
        json!({"released":true}),
    ]);
    let output = f.tty(
        peer.address,
        &["runtime-run", "30", &r.task.objective],
        "approve",
    );
    assert_ne!(output["status"], 0);
    assert_eq!(output["answered"], 1);
    let calls = peer.finish();
    assert_eq!(calls.len(), 3);
    assert!(
        calls
            .iter()
            .all(|c| c["route"] != "POST /v1/code/runtime/start HTTP/1.1")
    );
}

#[test]
fn connected_driver_interrupt_and_deadline_abort_a_waiting_prompt_without_preparing() {
    let f = Fixture::new();
    let r = driven_request();
    for mode in ["signal", "wait"] {
        let peer = f.peer(vec![driven_selection(&r)]);
        let began = Instant::now();
        let output = f.tty(peer.address, &["runtime-run", "1", &r.task.objective], mode);
        assert_ne!(output["status"], 0, "{output}");
        assert_eq!(output["answered"], 1, "{output}");
        assert!(began.elapsed() < Duration::from_secs(5));
        let calls = peer.finish();
        assert_eq!(calls.len(), 1, "{calls:?}");
        assert_eq!(calls[0]["route"], "GET /v1/code/runtime HTTP/1.1");
    }
}

#[test]
fn connected_driver_reads_complete_artifacts_and_refuses_corrupt_pages_before_success() {
    let f = Fixture::new();
    let r = driven_request();
    let mut stream = Stream::new(&r);
    stream.event(
        RuntimeEventKind::RunStarted {
            request_sha256: r.request_sha256.clone(),
        },
        None,
        None,
    );
    let artifact = RuntimeArtifactRef {
        schema_version: CONTRACT_SCHEMA_VERSION,
        artifact_id: RuntimeArtifactId::from_raw("http-artifact"),
        manifest_sha256: "a".repeat(64),
        payload_sha256: "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a".into(),
        byte_size: 2,
        media_type: "application/json".into(),
    };
    stream.event(
        RuntimeEventKind::ArtifactCreated {
            artifact_id: artifact.artifact_id.clone(),
            manifest_sha256: artifact.manifest_sha256.clone(),
        },
        None,
        None,
    );
    let outcome = stream.terminal(AgentStateKind::NoOp);
    let mut step = stream.step(stream.events.clone(), None, outcome);
    step["artifacts"] = json!([artifact]);
    let page = json!({"reference":artifact,"offset":0,"bytes":[123,125],"page_sha256":artifact.payload_sha256,"next_offset":null,"complete":true});
    for corrupt in [false, true] {
        let mut response = page.clone();
        if corrupt {
            response["bytes"] = json!([91, 93]);
        }
        let mut replies = vec![driven_selection(&r), json!(r), step.clone(), response];
        if !corrupt {
            replies.extend([Value::Null, Value::Null]);
        }
        replies.push(json!({"released":true}));
        let peer = f.peer(replies);
        let output = f.tty(
            peer.address,
            &["runtime-run", "30", &r.task.objective],
            "approve",
        );
        assert_eq!(output["status"] == 0, !corrupt, "{output}");
        assert_eq!(output["answered"], 2);
        let text = output["transcript"].as_str().unwrap();
        assert_eq!(text.contains("verified-outcome"), !corrupt, "{text}");
        let calls = peer.finish();
        assert_eq!(calls[3]["route"], "POST /v1/code/runtime/artifact HTTP/1.1");
        assert_eq!(calls[3]["body"]["maximum_bytes"], 4096);
        assert_eq!(calls[3]["body"]["reference"], json!(artifact));
        assert_eq!(calls.len(), if corrupt { 5 } else { 7 });
    }
}

#[test]
fn connected_driver_interrupt_at_approval_uses_canonical_job_control_and_sends_no_grant() {
    let f = Fixture::new();
    let r = driven_request();
    let (first, _) = driven_steps(&r, true);
    let mut stream = Stream {
        request: r.clone(),
        events: serde_json::from_value(first["events"].clone()).unwrap(),
    };
    let split = stream.events.len();
    stream.event(
        RuntimeEventKind::CancellationRequested {
            cancellation_id: CancellationId::from_raw("canonical-cancel"),
        },
        None,
        None,
    );
    stream.event(
        RuntimeEventKind::CancellationObserved {
            cancellation_id: CancellationId::from_raw("canonical-cancel"),
        },
        None,
        None,
    );
    let outcome = stream.terminal(AgentStateKind::Cancelled);
    let terminal = stream.step(stream.events[split..].to_vec(), None, outcome);
    let status = json!({"schema_version":1,"run_id":r.run_id,"request_sha256":r.request_sha256,"job":{"job_id":r.run_id,"phase":"running","revision":7,"cancellation_requested":false,"head_sha256":"a".repeat(64)}});
    let mut cancelling = status.clone();
    cancelling["job"]["phase"] = json!("cancelling");
    cancelling["job"]["revision"] = json!(8);
    cancelling["job"]["cancellation_requested"] = json!(true);
    let mut cancelled = cancelling.clone();
    cancelled["job"]["phase"] = json!("cancelled");
    cancelled["job"]["revision"] = json!(9);
    let answer = json!({"decision":{"decision":"applied","revision":8,"phase":"cancelling"},"status":cancelling});
    let peer = f.peer(vec![
        driven_selection(&r),
        json!(r),
        first,
        status,
        answer,
        terminal,
        Value::Null,
        cancelled,
        json!({"released":true}),
    ]);
    let output = f.tty(
        peer.address,
        &["runtime-run", "30", &r.task.objective],
        "signal-tool",
    );
    assert_ne!(output["status"], 0, "{output}");
    assert_eq!(output["answered"], 3, "{output}");
    let text = output["transcript"].as_str().unwrap();
    assert!(text.contains("verified-outcome"), "{text}");
    assert!(text.contains("CANCELLED"));
    assert!(text.contains("verified-job-control"));
    let calls = peer.finish();
    assert_eq!(calls.len(), 9, "{calls:?}");
    assert_eq!(calls[4]["route"], "POST /v1/code/runtime/control HTTP/1.1");
    assert_eq!(calls[4]["body"]["request"]["action"], "cancel");
    assert_eq!(calls[4]["body"]["request"]["observed_revision"], 7);
    assert_eq!(calls[5]["body"]["response"], Value::Null);
    assert!(
        calls.iter().all(|c| c["body"]["response"].is_null()),
        "{calls:?}"
    );
}

const PTY: &str = r#"
import errno,fcntl,json,os,pty,select,signal,subprocess,sys,termios,time
master,slave=pty.openpty()
def session():
    os.setsid();fcntl.ioctl(0,termios.TIOCSCTTY,0)
p=subprocess.Popen(sys.argv[2:],stdin=slave,stdout=slave,stderr=slave,preexec_fn=session)
os.close(slave);raw=b'';answered=0;started=time.monotonic();prompt_bytes=[];review_start=0
try:
    if sys.argv[1].startswith('stalled-'):
        # Do not drain output: the review exceeds the tty output queue.
        assert select.select([master],[],[],2)[0],'no review output'
        raw+=os.read(master,1)
        time.sleep(.2 if sys.argv[1]=='stalled-signal' else 1.2)
        if sys.argv[1]=='stalled-signal':os.kill(p.pid,signal.SIGINT)
    while True:
        assert time.monotonic()-started<10,'terminal timeout'
        if not select.select([master],[],[],.05)[0]:
            if p.poll() is not None:break
            continue
        try:chunk=os.read(master,1024 if sys.argv[1].startswith('slow-') else 65536)
        except OSError as e:
            if e.errno==errno.EIO:break
            raise
        if not chunk:break
        raw+=chunk
        if sys.argv[1].startswith('slow-'):time.sleep(.005)
        if raw.count(b'anything else declines:')>answered:
            prompt_bytes.append(len(raw)-review_start);review_start=len(raw)
            prompt=raw.split(b'Type exactly ')[-1].split(b' to approve;')[0]
            mode=sys.argv[1];tool=prompt.startswith(b'allow ')
            allow = mode in ('approve','slow-approve') or (mode in ('deny-tools','slow-deny-tools','signal-tool','eof-start','eof-tool','overflow-tool') and not tool)
            if sys.argv[1]=='signal' or (sys.argv[1]=='signal-tool' and prompt.startswith(b'allow ')):os.kill(p.pid,signal.SIGINT)
            elif (mode=='eof-inspect' or (mode=='eof-start' and prompt.startswith(b'start ')) or (mode=='eof-tool' and tool)):os.write(master,b'\x04')
            elif mode=='overflow-tool' and tool:os.write(master,b'x'*257+b'\n')
            elif sys.argv[1]!='wait':os.write(master,(prompt if allow else b'no')+b'\n')
            answered+=1
    status=p.wait(timeout=2)
    assert b'\x1b' not in raw
    print(json.dumps({'status':status,'answered':answered,'prompt_bytes':prompt_bytes,'transcript':raw.decode()}))
finally:
    if p.poll() is None:os.killpg(p.pid,signal.SIGKILL);p.wait()
    os.close(master)
"#;
