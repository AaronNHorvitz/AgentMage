// SPDX-License-Identifier: MIT OR Apache-2.0
//! Actual MCP binary and retained HTTP owner; synthetic tool/grant ports only.

use super::*;
use std::{
    fs,
    os::unix::fs::symlink,
    process::{Command, Output, Stdio},
    sync::atomic::AtomicU64,
};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Delegate(std::path::PathBuf);
impl Delegate {
    fn watch(&self, address: SocketAddr, r: &Value, seconds: &str, consent: &str) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_agentmage-code"));
        command
            .arg(address.to_string())
            .arg(&self.0)
            .args([
                "endpoint-watch",
                seconds,
                r["run_id"].as_str().unwrap(),
                r["request_sha256"].as_str().unwrap(),
                consent,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "agentmage-endpoint-mcp-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        ApiTokens::initialize(&root).unwrap();
        fs::write(root.join("client.token"), CLIENT).unwrap();
        fs::remove_file(root.join("control.token")).unwrap();
        // A control-token load would refuse this deliberate invalid neighbor.
        symlink("not-a-control-credential", root.join("control.token")).unwrap();
        Self(root)
    }
    fn run(&self, address: SocketAddr, r: &Value, calls: &[Value], consent: &str) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_agentmage-mcp"))
            .args([
                address.to_string(),
                self.0.to_str().unwrap().into(),
                r["run_id"].as_str().unwrap().into(),
                r["request_sha256"].as_str().unwrap().into(),
                consent.into(),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut messages = vec![
            json!({"jsonrpc":"2.0","id":"init","method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"synthetic-observer","version":"1"}}}),
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        ];
        messages.extend_from_slice(calls);
        // A refused startup can close stdin before this small write.
        let _ = child.stdin.take().unwrap().write_all(
            messages
                .iter()
                .map(|m| format!("{m}\n"))
                .collect::<String>()
                .as_bytes(),
        );
        child.wait_with_output().unwrap()
    }
}
impl Drop for Delegate {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn call(id: u64, name: &str, args: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":args}})
}
fn replies(output: Output) -> Vec<Value> {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn content(reply: &Value) -> Value {
    assert_eq!(reply["result"]["isError"], false, "{reply}");
    serde_json::from_str(reply["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}
fn fixture(delegate: bool) -> (Api, TcpListener, Arc<AtomicUsize>) {
    let model = TcpListener::bind("127.0.0.1:0").unwrap();
    model.set_nonblocking(true).unwrap();
    let endpoint =
        LocalChatEndpoint::parse(&format!("http://{}/v1", model.local_addr().unwrap())).unwrap();
    let executions = Arc::new(AtomicUsize::new(0));
    (
        Api::start(endpoint, executions.clone(), delegate),
        model,
        executions,
    )
}
fn no_effects(model: &TcpListener, executions: &AtomicUsize) {
    assert_eq!(executions.load(Ordering::SeqCst), 0);
    assert!(matches!(model.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
}

#[test]
fn endpoint_observation_delegation_is_opt_in_and_never_delegates_controls() {
    for delegated in [false, true] {
        let (api, model, executions) = fixture(delegated);
        let r = api.request();
        let binding = json!({"run_id":r["run_id"],"request_sha256":r["request_sha256"]});
        assert_eq!(
            api.call("GET", "/v1/code/endpoint/request", CLIENT, None).0,
            if delegated { 200 } else { 403 }
        );
        let mut events = binding.clone();
        events["after"] = json!(0);
        assert_eq!(
            api.call("POST", "/v1/code/endpoint/events", CLIENT, Some(events))
                .0,
            if delegated { 200 } else { 403 }
        );
        assert_eq!(
            api.call("GET", "/v1/code/endpoint/request", "", None).0,
            401
        );
        for (method, route, body) in [
            (
                "POST",
                "advance",
                json!({"run_id":r["run_id"],"request_sha256":r["request_sha256"],"approve_model_context":true,"response":null}),
            ),
            ("POST", "cancel", binding.clone()),
            ("POST", "release", binding.clone()),
            ("POST", "request", json!({})),
            ("GET", "events", json!({})),
            ("POST", "approve", json!({"allow":true})),
        ] {
            assert_eq!(
                api.call(
                    method,
                    &format!("/v1/code/endpoint/{route}"),
                    CLIENT,
                    (method != "GET").then_some(body)
                )
                .0,
                403
            );
        }
        assert_eq!(
            api.call("POST", "/v1/settings", CLIENT, Some(json!({}))).0,
            403
        );
        assert_eq!(api.bind("events", &r).1["total"], 0);
        // A delegated cancel was refused: no consent still refuses advancement.
        assert_eq!(api.advance(&r, Value::Null, false).0, 403);
        no_effects(&model, &executions);
    }
}

#[test]
fn endpoint_mcp_binary_reads_only_exact_pinned_run_without_control_credential() {
    let (api, model, executions) = fixture(true);
    let r = api.request();
    let delegate = Delegate::new();
    let calls = vec![
        json!({"jsonrpc":"2.0","id":"list","method":"tools/list"}),
        call(1, "endpoint_request", json!({})),
        call(2, "endpoint_events", json!({"after":0})),
        call(3, "endpoint_advance", json!({"approve_model_context":true})),
        call(4, "endpoint_cancel", json!({})),
        call(5, "endpoint_release", json!({})),
        call(6, "workspace", json!({})),
        call(7, "endpoint_request", json!({"run_id":"other"})),
        call(8, "endpoint_events", json!({"after":-1})),
        call(9, "endpoint_events", json!({"after":4294967296_u64})),
        call(10, "endpoint_events", json!({"after":0,"approve":true})),
    ];
    let observed = replies(delegate.run(api.address, &r, &calls, "--share-selected-endpoint"));
    let catalog = observed[1]["result"]["tools"].as_array().unwrap();
    assert_eq!(catalog.len(), 2);
    assert_eq!(catalog[0]["name"], "endpoint_request");
    assert_eq!(catalog[1]["name"], "endpoint_events");
    assert!(
        catalog
            .iter()
            .all(|t| t["annotations"]["readOnlyHint"] == true
                && t["annotations"]["openWorldHint"] == false)
    );
    assert_eq!(content(&observed[2])["request"], r);
    assert_eq!(
        content(&observed[2])["verification"],
        "service-observation-only"
    );
    assert_eq!(content(&observed[3])["page"]["total"], 0);
    for reply in &observed[4..] {
        assert!(
            reply["error"].is_object() || reply["result"]["isError"] == true,
            "{reply}"
        );
    }
    for consent in ["--share-selected-workspace", "--no-consent"] {
        let output = delegate.run(api.address, &r, &calls, consent);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
    let mut foreign = r.clone();
    foreign["run_id"] = json!("foreign-run");
    for foreign in [foreign, {
        let mut v = r.clone();
        v["request_sha256"] = json!("0".repeat(64));
        v
    }] {
        let output = replies(delegate.run(
            api.address,
            &foreign,
            &[
                call(1, "endpoint_request", json!({})),
                call(2, "endpoint_events", json!({"after":0})),
            ],
            "--share-selected-endpoint",
        ));
        for reply in &output[1..] {
            assert_eq!(reply["result"]["isError"], true);
            assert!(
                !reply
                    .to_string()
                    .contains(r["task"]["objective"].as_str().unwrap())
            );
        }
    }
    let (other, other_model, other_executions) = fixture(true);
    let refused = replies(delegate.run(
        other.address,
        &r,
        &[call(1, "endpoint_request", json!({}))],
        "--share-selected-endpoint",
    ));
    assert_eq!(refused[1]["result"]["isError"], true);
    no_effects(&other_model, &other_executions);
    assert_eq!(api.bind("events", &r).1["total"], 0);
    no_effects(&model, &executions);
}

#[test]
fn endpoint_mcp_binary_exports_original_events_and_refuses_after_release_or_without_delegation() {
    let (api, model, executions) = fixture(true);
    let r = api.request();
    let delegate = Delegate::new();
    assert_eq!(api.bind("cancel", &r).0, 200);
    assert_eq!(api.advance(&r, Value::Null, false).0, 200);
    let first = api.bind("events", &r).1;
    let total = first["total"].as_u64().unwrap();
    assert!(total > 0);
    let calls = (0..total)
        .map(|after| call(after, "endpoint_events", json!({"after":after})))
        .collect::<Vec<_>>();
    let observed = replies(delegate.run(api.address, &r, &calls, "--share-selected-endpoint"));
    assert_eq!(observed.len(), total as usize + 1);
    for (after, reply) in observed[1..].iter().enumerate() {
        let got = content(reply);
        assert_eq!(got["verification"], "individual-event-hashes-only");
        assert_eq!(got["index_is_resume_cursor"], false);
        let expected = api.call("POST", "/v1/code/endpoint/events", CONTROL, Some(json!({"run_id":r["run_id"],"request_sha256":r["request_sha256"],"after":after}))).1;
        assert_eq!(got["page"], expected);
    }
    assert_eq!(api.bind("release", &r).0, 200);
    let observed = replies(delegate.run(
        api.address,
        &r,
        &[call(1, "endpoint_events", json!({"after":0}))],
        "--share-selected-endpoint",
    ));
    assert_eq!(observed[1]["result"]["isError"], true);
    no_effects(&model, &executions);
    let (api, model, executions) = fixture(false);
    let r = api.request();
    let observed = replies(delegate.run(
        api.address,
        &r,
        &[
            call(1, "endpoint_request", json!({})),
            call(2, "endpoint_events", json!({"after":0})),
        ],
        "--share-selected-endpoint",
    ));
    assert!(
        observed[1..]
            .iter()
            .all(|reply| reply["result"]["isError"] == true)
    );
    no_effects(&model, &executions);
}

#[test]
fn endpoint_mcp_observes_approved_tool_events_without_acquiring_control() {
    let (upstream, endpoint) = upstream();
    let executions = Arc::new(AtomicUsize::new(0));
    let worker = thread::spawn(move || {
        let mut stream = accept(&upstream);
        read_upstream(&mut stream);
        respond(&mut stream, &tool());
        drop(stream);
        let mut stream = accept(&upstream);
        let request = read_upstream(&mut stream);
        assert!(request.to_string().contains("original retained boundary"));
        respond(&mut stream, "unverified synthetic completion");
        upstream
    });
    let api = Api::start(endpoint, executions.clone(), true);
    let request = api.request();
    let delegate = Delegate::new();
    let (code, waiting) = api.advance(&request, Value::Null, true);
    assert_eq!(code, 200);
    assert_eq!(waiting["state"], "awaiting_approval");
    let challenge = &waiting["challenge"];
    let approval = json!({"schema_version":CONTRACT_SCHEMA_VERSION,"run_id":challenge["run_id"],
        "approval_id":challenge["approval_id"],"disposition":"allow",
        "challenge_sha256":challenge["challenge_sha256"],"grant_id":challenge["proposed_grant_id"],"selection":null});
    assert_eq!(
        api.call(
            "POST",
            "/v1/code/endpoint/advance",
            CLIENT,
            Some(
                json!({"run_id":request["run_id"],"request_sha256":request["request_sha256"],
            "response":approval,"approve_model_context":true})
            )
        )
        .0,
        403
    );
    let observed = replies(delegate.run(
        api.address,
        &request,
        &[
            call(1, "endpoint_request", json!({})),
            call(2, "endpoint_events", json!({"after":0})),
            call(3, "endpoint_advance", json!({"response":approval})),
        ],
        "--share-selected-endpoint",
    ));
    assert_eq!(content(&observed[1])["request"], request);
    assert!(content(&observed[2])["page"]["total"].as_u64().unwrap() > 0);
    assert!(observed[3]["error"].is_object());
    assert_eq!(executions.load(Ordering::SeqCst), 0);
    assert_eq!(api.advance(&request, Value::Null, true), (200, waiting));
    let (code, terminal) = api.advance(&request, approval, true);
    assert_eq!(code, 200);
    assert_eq!(terminal["state"], "terminal");
    // A rejecting synthetic verifier is never turned into task success.
    assert_eq!(terminal["outcome"]["outcome"]["state"], "FAILED");
    assert_eq!(executions.load(Ordering::SeqCst), 1);
    let total = api.bind("events", &request).1["total"].as_u64().unwrap();
    let calls = (0..total)
        .map(|after| call(after, "endpoint_events", json!({"after":after})))
        .collect::<Vec<_>>();
    let observed =
        replies(delegate.run(api.address, &request, &calls, "--share-selected-endpoint"));
    assert_eq!(observed.len(), total as usize + 1);
    let mut sequence = agentmage_kernel_engine::runtime_event::RuntimeEventSequence::new();
    for (after, reply) in observed[1..].iter().enumerate() {
        let got = content(reply);
        let original = api.call("POST", "/v1/code/endpoint/events", CONTROL,
            Some(json!({"run_id":request["run_id"],"request_sha256":request["request_sha256"],"after":after})));
        assert_eq!(original.0, 200);
        assert_eq!(got["page"], original.1);
        for event in got["page"]["events"].as_array().unwrap() {
            sequence
                .push(&serde_json::from_value::<RuntimeEvent>(event.clone()).unwrap())
                .unwrap();
        }
    }
    assert!(sequence.is_terminal());
    assert_eq!(executions.load(Ordering::SeqCst), 1);
    let upstream = worker.join().unwrap();
    assert!(matches!(upstream.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
    assert_eq!(api.bind("release", &request).0, 200);
    let observed = replies(delegate.run(
        api.address,
        &request,
        &[call(1, "endpoint_events", json!({"after":0}))],
        "--share-selected-endpoint",
    ));
    assert_eq!(observed[1]["result"]["isError"], true);
}

// Test-side coordination only: an explicit api.busy response precedes dispatch.
// Never retry a transport error, another refusal or an ambiguous effect reply.
fn when_observer_idle(mut operation: impl FnMut() -> (u16, Value)) -> Value {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let (status, value) = operation();
        if status == 409 && value["error"] == "api.busy" && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
            continue;
        }
        assert_eq!(status, 200, "{value}");
        return value;
    }
}

// Keep child cleanup on every assertion path; never leave a watcher or pipe
// reader holding the test reservation after an early failure.
struct Watcher {
    child: Option<std::process::Child>,
    reader: Option<JoinHandle<Vec<u8>>>,
}
impl Watcher {
    fn start(mut command: Command) -> Self {
        Self {
            child: Some(command.spawn().unwrap()),
            reader: None,
        }
    }
    fn ready(&mut self) {
        use std::io::{BufRead, BufReader};
        let stderr = self.child.as_mut().unwrap().stderr.take().unwrap();
        let (send, receive) = mpsc::channel();
        self.reader = Some(thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let _ = send.send(line.clone());
            let mut bytes = line.into_bytes();
            reader.read_to_end(&mut bytes).unwrap();
            bytes
        }));
        let result = receive.recv_timeout(Duration::from_secs(5));
        if result.is_err() {
            self.child.as_mut().unwrap().kill().unwrap();
        }
        assert!(
            result
                .unwrap()
                .starts_with("Watching shared service observations.")
        );
    }
    fn finish(mut self) -> Output {
        let mut output = self.child.take().unwrap().wait_with_output().unwrap();
        if let Some(reader) = self.reader.take() {
            output.stderr = reader.join().unwrap();
        }
        output
    }
}
impl Drop for Watcher {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            child.wait().unwrap();
        }
        if let Some(reader) = self.reader.take() {
            reader.join().unwrap();
        }
    }
}
fn watch_values(output: &Output) -> Vec<Value> {
    assert!(output.stdout.is_ascii());
    assert!(!output.stdout.contains(&0x1b));
    serde_json::Deserializer::from_slice(&output.stdout)
        .into_iter::<Value>()
        .map(Result::unwrap)
        .collect()
}

#[test]
fn endpoint_watch_streams_a_growing_original_chain_without_control_credentials_or_effects() {
    use agentmage_kernel_engine::runtime_event::RuntimeEventSequence;
    let (api, model, executions) = fixture(true);
    let r = api.request();
    let delegate = Delegate::new();
    let mut watcher =
        Watcher::start(delegate.watch(api.address, &r, "10", "--share-selected-endpoint"));
    watcher.ready();
    // It starts with an empty prefix and can only observe another caller's
    // cancellation and original coordinator advancement. No disclosure occurs.
    assert_eq!(api.bind("cancel", &r).0, 200);
    when_observer_idle(|| api.advance(&r, Value::Null, false));
    let output = watcher.finish();
    assert!(output.status.success());
    let values = watch_values(&output);
    assert!(!values.is_empty());
    let mut chain = RuntimeEventSequence::new();
    for (index, value) in values.iter().enumerate() {
        let expected = api.call("POST", "/v1/code/endpoint/events", CONTROL,
            Some(json!({"run_id":r["run_id"],"request_sha256":r["request_sha256"],"after":index}))).1;
        assert_eq!(value["event"], expected["events"][0]);
        assert_eq!(value["verification"], "self-consistent-service-observation");
        assert_eq!(value["index_is_resume_cursor"], false);
        assert_eq!(value["terminal_observed"], index == values.len() - 1);
        chain
            .push(&serde_json::from_value(value["event"].clone()).unwrap())
            .unwrap();
    }
    assert!(chain.is_terminal());
    assert_eq!(
        values.last().unwrap()["event"]["kind"]["state"],
        "CANCELLED"
    );
    assert!(
        fs::symlink_metadata(delegate.0.join("control.token"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    // A second watcher can read the same terminal chain; neither released it.
    let second = delegate
        .watch(api.address, &r, "10", "--share-selected-endpoint")
        .output()
        .unwrap();
    assert!(second.status.success());
    assert_eq!(second.stdout, output.stdout);
    no_effects(&model, &executions);
}

#[test]
fn endpoint_watch_refuses_wrong_scope_missing_delegation_and_invalid_launches() {
    let delegate = Delegate::new();
    for delegated in [false, true] {
        let (api, model, executions) = fixture(delegated);
        let r = api.request();
        let mut foreign = r.clone();
        foreign["run_id"] = json!("foreign-run");
        let mut stale = r.clone();
        stale["request_sha256"] = json!("0".repeat(64));
        let cases = if delegated {
            vec![foreign, stale]
        } else {
            vec![r.clone()]
        };
        for pin in cases {
            let output = delegate
                .watch(api.address, &pin, "10", "--share-selected-endpoint")
                .output()
                .unwrap();
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
        }
        for (seconds, consent) in [
            ("0", "--share-selected-endpoint"),
            ("3601", "--share-selected-endpoint"),
            ("invalid", "--share-selected-endpoint"),
            ("10", "--no-consent"),
        ] {
            let output = delegate
                .watch(api.address, &r, seconds, consent)
                .output()
                .unwrap();
            assert!(!output.status.success());
            assert!(output.stdout.is_empty());
        }
        assert_eq!(api.bind("events", &r).1["total"], 0);
        assert_eq!(api.bind("cancel", &r).0, 200);
        assert_eq!(api.advance(&r, Value::Null, false).0, 200);
        assert_eq!(api.bind("release", &r).0, 200);
        let output = delegate
            .watch(api.address, &r, "10", "--share-selected-endpoint")
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        no_effects(&model, &executions);
    }
}

#[test]
fn endpoint_watch_timeout_and_signals_stop_only_the_observer() {
    let (api, model, executions) = fixture(true);
    let r = api.request();
    let delegate = Delegate::new();
    for signal in [None, Some("INT"), Some("TERM"), Some("HUP")] {
        let started = Instant::now();
        let mut watcher = Watcher::start(delegate.watch(
            api.address,
            &r,
            if signal.is_some() { "10" } else { "1" },
            "--share-selected-endpoint",
        ));
        watcher.ready();
        if let Some(signal) = signal {
            let id = watcher.child.as_ref().unwrap().id().to_string();
            assert!(
                Command::new("kill")
                    .args(["-s", signal, &id])
                    .status()
                    .unwrap()
                    .success()
            );
        }
        let output = watcher.finish();
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(output.stdout.is_empty());
        assert!(started.elapsed() < Duration::from_secs(8));
        assert_eq!(when_observer_idle(|| api.bind("events", &r))["total"], 0);
    }
    // Cancellation itself still belongs to the original control path.
    assert_eq!(api.bind("cancel", &r).0, 200);
    assert_eq!(api.advance(&r, Value::Null, false).0, 200);
    no_effects(&model, &executions);
}

// A malicious authenticated peer can seal individual events. The watcher must
// still refuse broken links, altered start bindings, decreasing totals, excess
// budgets and trailing events beyond a terminal observation.
#[test]
fn endpoint_watch_refuses_self_sealed_bad_chains_and_page_bounds() {
    use agentmage_kernel_engine::runtime_event::seal_runtime_event;
    let (api, model, executions) = fixture(true);
    let r = api.request();
    assert_eq!(api.bind("cancel", &r).0, 200);
    assert_eq!(api.advance(&r, Value::Null, false).0, 200);
    let total = api.bind("events", &r).1["total"].as_u64().unwrap();
    let pages: Vec<Value> = (0..total)
        .map(|after| {
            api.call("POST", "/v1/code/endpoint/events", CONTROL,
        Some(json!({"run_id":r["run_id"],"request_sha256":r["request_sha256"],"after":after}))).1
        })
        .collect();
    assert!(pages.len() > 2);
    let delegate = Delegate::new();
    for mode in [
        "link",
        "start",
        "shrink",
        "limit",
        "run-limit",
        "terminal-tail",
        "foreign",
        "sequence",
    ] {
        let mut bad = pages.clone();
        let mut expected_prefix = 0;
        match mode {
            "start" => {
                let mut event: RuntimeEvent =
                    serde_json::from_value(bad[0]["events"][0].clone()).unwrap();
                if let RuntimeEventKind::RunStarted { request_sha256 } = &mut event.kind {
                    *request_sha256 = "f".repeat(64);
                } else {
                    panic!("missing original start");
                }
                bad[0]["events"][0] = json!(seal_runtime_event(event).unwrap());
            }
            "link" | "foreign" | "sequence" => {
                let mut event: RuntimeEvent =
                    serde_json::from_value(bad[1]["events"][0].clone()).unwrap();
                match mode {
                    "link" => event.previous_event_sha256 = "f".repeat(64),
                    "foreign" => event.run_id = RuntimeRunId::from_raw("foreign-run"),
                    _ => event.sequence += 1,
                }
                bad[1]["events"][0] = json!(seal_runtime_event(event).unwrap());
                expected_prefix = 1;
            }
            "shrink" => {
                bad[1]["total"] = json!(total - 1);
                expected_prefix = 1;
            }
            "limit" => bad[0]["total"] = json!(4097),
            "run-limit" => {
                let maximum = r["limits"]["max_events"].as_u64().unwrap();
                assert!(maximum < 4096);
                bad[0]["total"] = json!(maximum + 1);
            }
            "terminal-tail" => {
                for page in &mut bad {
                    page["total"] = json!(total + 1);
                }
                expected_prefix = pages.len() - 1;
            }
            _ => unreachable!(),
        }
        bad.truncate(expected_prefix + 1);
        let replies = bad
            .into_iter()
            .enumerate()
            .map(|(after, page)| (200, page, after))
            .collect();
        let output = watch_peer(&delegate, &r, replies);
        assert!(!output.status.success(), "{mode}");
        assert_eq!(watch_values(&output).len(), expected_prefix, "{mode}");
    }
    no_effects(&model, &executions);
}

fn watch_peer(delegate: &Delegate, r: &Value, replies: Vec<(u16, Value, usize)>) -> Output {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let selection =
        json!({"request":r,"retention":"process-only","configuration":"fixed-for-service"});
    let worker = thread::spawn(move || {
        for (index, (status, reply, after)) in std::iter::once((200, selection, 0))
            .chain(replies)
            .enumerate()
        {
            let mut socket = accept(&listener);
            socket
                .set_read_timeout(Some(Duration::from_secs(4)))
                .unwrap();
            let mut header = Vec::new();
            while !header.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                socket.read_exact(&mut byte).unwrap();
                header.push(byte[0]);
                assert!(header.len() < 4096);
            }
            let header = String::from_utf8(header).unwrap();
            assert!(header.contains(&format!("Authorization: Bearer {CLIENT}\r\n")));
            let route = if index == 0 {
                "GET /v1/code/endpoint/request "
            } else {
                "POST /v1/code/endpoint/events "
            };
            assert!(header.starts_with(route));
            let length: usize = header
                .lines()
                .find_map(|l| l.strip_prefix("Content-Length: "))
                .unwrap()
                .parse()
                .unwrap();
            let mut body = vec![0; length];
            socket.read_exact(&mut body).unwrap();
            if index > 0 {
                let body: Value = serde_json::from_slice(&body).unwrap();
                assert_eq!(body["after"], after);
            }
            let body = reply.to_string();
            socket.write_all(format!("HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).unwrap();
        }
    });
    let output = delegate
        .watch(address, r, "10", "--share-selected-endpoint")
        .output()
        .unwrap();
    worker.join().unwrap();
    output
}

#[test]
fn endpoint_watch_waits_only_for_explicit_busy_observations_with_the_same_index() {
    let (api, model, executions) = fixture(true);
    let r = api.request();
    assert_eq!(api.bind("cancel", &r).0, 200);
    assert_eq!(api.advance(&r, Value::Null, false).0, 200);
    let total = api.bind("events", &r).1["total"].as_u64().unwrap();
    let delegate = Delegate::new();
    let mut replies = Vec::new();
    for after in 0..total {
        replies.push((
            409,
            json!({"error":"api.busy","message":"synthetic busy observation"}),
            after as usize,
        ));
        let page = api.call("POST", "/v1/code/endpoint/events", CONTROL,
            Some(json!({"run_id":r["run_id"],"request_sha256":r["request_sha256"],"after":after}))).1;
        replies.push((200, page, after as usize));
    }
    let output = watch_peer(&delegate, &r, replies);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(watch_values(&output).len(), total as usize);
    for (status, reply) in [
        (403, json!({"error":"api.authority"})),
        (409, json!({"error":"code.endpoint.binding"})),
        (503, json!({"error":"api.stopping"})),
        (200, json!({"observation_busy":true})),
        (200, json!({"error":"api.busy"})),
    ] {
        let output = watch_peer(&delegate, &r, vec![(status, reply, 0)]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
    }
    no_effects(&model, &executions);
}
