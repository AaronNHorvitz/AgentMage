// SPDX-License-Identifier: MIT OR Apache-2.0
//! Executable consumers of the actual API. Upstream generation is synthetic;
//! native ownership refusal is real. No successful native run is claimed.
use super::*;

fn configure(f: &Fixture, endpoint: &str, disclose: bool) -> Value {
    assert_eq!(
        f.call(
            "PUT",
            "/v1/settings",
            true,
            json!({"endpoint":endpoint,"approve_conversations":true})
        )
        .0,
        200
    );
    let (status, selected) = f.call(
        "PUT",
        "/v1/code/workspace",
        true,
        json!({"root":f.root.join("repo"),"paths":["sentinel"],"approve_read":true,
        "approve_model_context":disclose,"settings_revision":1,"settings_endpoint":endpoint}),
    );
    assert_eq!(status, 200);
    let (status, observed) = f.call_bytes("GET", "/v1/code/workspace", true, b"");
    assert_eq!(status, 200);
    assert_eq!(
        observed["workspace_selection_id"],
        selected["workspace_selection_id"]
    );
    assert_eq!(
        observed["workspace_revision"],
        selected["workspace_revision"]
    );
    assert_eq!(observed["model_context_approved"], disclose);
    selected
}

fn states(f: &Fixture) -> Vec<Value> {
    [
        "/v1/code/repository",
        "/v1/code/repository/edit",
        "/v1/code/repository/command",
        "/v1/code/repository/validation",
        "/v1/code/repository/apply",
        "/v1/code/workspace/changes",
    ]
    .into_iter()
    .map(|path| {
        let (status, state) = f.call_bytes("GET", path, true, b"");
        assert_eq!(status, 200);
        state
    })
    .collect()
}

fn assert_preserved(f: &Fixture, before: &[Value]) {
    assert_eq!(states(f), before);
    assert_eq!(
        fs::read(f.root.join("repo/sentinel")).unwrap(),
        b"human work"
    );
    assert!(fs::read_dir(f.root.join("owned")).unwrap().next().is_none());
}

#[test]
fn repository_setup_actual_service_stops_at_original_native_inspection_refusal() {
    repository_setup_refusal("code-repository");
}

#[test]
fn repository_transfer_actual_service_stops_at_original_native_inspection_refusal() {
    repository_setup_refusal("code-repository-apply");
}

#[test]
fn native_driver_actual_service_refuses_absent_runtime_without_effects_or_disclosure() {
    let f = Fixture::new();
    let upstream = TcpListener::bind("127.0.0.1:0").unwrap();
    upstream.set_nonblocking(true).unwrap();
    configure(
        &f,
        &format!("http://{}/v1", upstream.local_addr().unwrap()),
        false,
    );
    let before = states(&f);
    let (status, refusal) = f.call_bytes("GET", "/v1/code/runtime", true, b"");
    assert_eq!(status, 428);
    assert_eq!(refusal["error"], "code.runtime.activation-required");
    let out = f.terminal(
        &["runtime-run", "1", "Inspect the selected worktree"],
        "approve-all",
    );
    assert_eq!(out["status"], 1, "{out}");
    let text = out["transcript"].as_str().unwrap();
    assert!(text.contains(refusal["error"].as_str().unwrap()), "{out}");
    assert!(
        !text.contains("Type exactly ") && !text.contains("verified-outcome"),
        "{out}"
    );
    assert_preserved(&f, &before);
    assert_eq!(
        upstream.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

fn repository_setup_refusal(command: &str) {
    let f = Fixture::new();
    let upstream = TcpListener::bind("127.0.0.1:0").unwrap();
    upstream.set_nonblocking(true).unwrap();
    configure(
        &f,
        &format!("http://{}/v1", upstream.local_addr().unwrap()),
        false,
    );
    let before = states(&f);
    let (status, refusal) = f.call("POST", "/v1/code/repository/prepare", true, f.prepare());
    assert_eq!(status, 409);
    let definition = json!({"command":synthetic_command_spec(),"kind":"unit",
        "minimum_test_count":1,"approve_definition":true})
    .to_string();
    let event_count = f.call_bytes("GET", "/v1/activity", true, b"").1["events"]
        .as_array()
        .unwrap()
        .len();
    let started = Instant::now();
    let out = f.terminal(
        &[
            command,
            "30",
            f.root.join("repo").to_str().unwrap(),
            f.root.join("owned").to_str().unwrap(),
            "fixture",
            &"1".repeat(40),
            "sentinel",
            "synthetic-model",
            "4096",
            "128",
            "Inspect sentinel",
            &definition,
        ],
        "approve-all",
    );
    assert_eq!(out["status"], 1, "{out}");
    let transcript = out["transcript"].as_str().unwrap();
    assert!(transcript.contains("Type exactly inspect "), "{out}");
    assert!(
        transcript.contains(refusal["error"].as_str().unwrap()),
        "{out}"
    );
    for denied in [
        "Type exactly create ",
        "Type exactly discover ",
        "Type exactly share ",
    ] {
        assert!(!transcript.contains(denied), "{out}");
    }
    assert_eq!(
        upstream.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert_preserved(&f, &before);
    let events = f.call_bytes("GET", "/v1/activity", true, b"").1["events"]
        .as_array()
        .unwrap()
        .clone();
    assert!(!events[event_count..].iter().any(|event| matches!(
        event["action"].as_str(),
        Some(
            "code.map"
                | "code.workspace.proposal"
                | "code.workspace.select"
                | "code.repository.repair"
        )
    )));
    eprintln!(
        "actual API repository-setup native refusal elapsed_us={}",
        started.elapsed().as_micros()
    );
}

#[test]
fn guided_actual_service_refuses_unowned_workspace_without_generation_or_effects() {
    for disclose in [false, true] {
        let mut f = Fixture::new();
        let upstream = TcpListener::bind("127.0.0.1:0").unwrap();
        upstream.set_nonblocking(true).unwrap();
        let endpoint = format!("http://{}/v1", upstream.local_addr().unwrap());
        for restarted in [false, true] {
            if restarted {
                f.stop();
                f.start();
            }
            configure(&f, &endpoint, disclose);
            let before = states(&f);
            let definition = json!({"command":synthetic_command_spec(),"kind":"unit",
                "minimum_test_count":1,"approve_definition":true})
            .to_string();
            for bounded in [false, true] {
                let mut args = if bounded {
                    vec!["code-worktree-bounded", "10"]
                } else {
                    vec!["code-worktree"]
                };
                let root = f.root.join("repo");
                args.extend([
                    root.to_str().unwrap(),
                    "synthetic-model",
                    "4096",
                    "128",
                    "Repair sentinel",
                    &definition,
                ]);
                let started = Instant::now();
                let out = f.terminal(&args, "approve-all");
                assert_eq!(
                    out["status"], 1,
                    "disclose={disclose}, restarted={restarted}: {out}"
                );
                assert_eq!(out["sent"], false, "{out}");
                let transcript = out["transcript"].as_str().unwrap();
                assert!(transcript.contains("Guided workflow stopped"), "{out}");
                assert!(!transcript.contains("Type exactly"), "{out}");
                assert_eq!(
                    upstream.accept().unwrap_err().kind(),
                    std::io::ErrorKind::WouldBlock
                );
                assert_preserved(&f, &before);
                eprintln!(
                    "actual API guided ownership refusal disclose={disclose} restarted={restarted} bounded={bounded} elapsed_us={}",
                    started.elapsed().as_micros()
                );
            }
            let events = f.call_bytes("GET", "/v1/activity", true, b"").1["events"]
                .as_array()
                .unwrap()
                .clone();
            assert!(
                !events
                    .iter()
                    .any(|e| e["action"] == "code.workspace.proposal"
                        || e["action"] == "code.repository.repair")
            );
        }
    }
}

#[test]
fn repair_actual_service_requires_owned_failed_receipt_before_disclosure() {
    let mut f = Fixture::new();
    let upstream = TcpListener::bind("127.0.0.1:0").unwrap();
    upstream.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}/v1", upstream.local_addr().unwrap());
    for restarted in [false, true] {
        if restarted {
            f.stop();
            f.start();
        }
        let selected = configure(&f, &endpoint, true);
        let before = states(&f);
        let root = f.root.join("repo");
        let out = f.terminal(
            &[
                "repair-worktree",
                root.to_str().unwrap(),
                "synthetic-model",
                "8192",
                "128",
                "Repair sentinel",
            ],
            "approve-all",
        );
        assert_eq!(out["status"], 1, "{out}");
        assert_eq!(out["sent"], false, "{out}");
        assert!(
            out["transcript"]
                .as_str()
                .unwrap()
                .contains("code.command.worktree-required")
        );
        let scope =
            agentmage_platform_linux::LinuxCodingReadScope::select(&root, &["sentinel".into()])
                .unwrap();
        let body = json!({"workspace_revision":selected["workspace_revision"],
            "workspace_selection_id":selected["workspace_selection_id"],
            "expected_root_sha256":scope.root_sha256(),"paths":["sentinel"],"model":"synthetic-model",
            "task":"Repair sentinel","max_message_bytes":8192,"max_tokens":128,
            "receipt_sha256":"a".repeat(64),"feedback_sha256":"b".repeat(64),
            "settings_revision":1,"settings_endpoint":endpoint,"approve_failure_disclosure":true});
        // Even a direct request with correct selection/root/endpoint pins and
        // claimed consent cannot invent an owned failed receipt. Neither a
        // repeated request nor reopening the service can create repair capacity.
        for _ in 0..2 {
            let (status, reply) = f.call(
                "POST",
                "/v1/code/repository/repair/proposals",
                true,
                body.clone(),
            );
            assert_eq!(status, 409);
            assert_eq!(reply["error"], "code.command.worktree-required");
        }
        assert_eq!(
            upstream.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        assert_preserved(&f, &before);
    }
}

struct Model {
    endpoint: String,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<Vec<Value>>>,
}
impl Model {
    fn new(finish: &'static str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let done = stop.clone();
        let worker = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut requests = Vec::new();
            while !done.load(Ordering::Relaxed) {
                assert!(Instant::now() < deadline, "synthetic upstream timed out");
                let (mut socket, _) = match listener.accept() {
                    Ok(pair) => pair,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(e) => panic!("{e}"),
                };
                socket
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                socket
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut header = Vec::new();
                while !header.ends_with(b"\r\n\r\n") {
                    let mut byte = [0];
                    socket.read_exact(&mut byte).unwrap();
                    header.push(byte[0]);
                    assert!(header.len() < 8192);
                }
                let header = std::str::from_utf8(&header).unwrap();
                assert!(header.starts_with("POST /v1/chat/completions HTTP/1.1\r\n"));
                let length: usize = header
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length: ")
                            .map(str::to_owned)
                    })
                    .unwrap()
                    .parse()
                    .unwrap();
                assert!(length <= 32768);
                let mut body = vec![0; length];
                socket.read_exact(&mut body).unwrap();
                requests.push(serde_json::from_slice(&body).unwrap());
                let answer =
                    r#"{"edits":[{"path":"sentinel","old":"human work","new":"replacement"}]}"#;
                let body = format!(
                    "data: {}\n\ndata: {}\n\ndata: [DONE]\n\n",
                    json!({"model":"synthetic-model","choices":[{"index":0,"delta":{"content":answer},"finish_reason":null}]}),
                    json!({"model":"synthetic-model","choices":[{"index":0,"delta":{},"finish_reason":finish}]})
                );
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).unwrap();
            }
            requests
        });
        Self {
            endpoint,
            stop,
            worker: Some(worker),
        }
    }
    fn finish(mut self) -> Vec<Value> {
        self.stop.store(true, Ordering::Relaxed);
        self.worker.take().unwrap().join().unwrap()
    }
}
impl Drop for Model {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[test]
fn worktree_proposal_actual_service_streams_but_cannot_adopt_an_unowned_directory() {
    for (disclose, finish) in [(false, "stop"), (true, "stop"), (true, "length")] {
        let f = Fixture::new();
        let model = Model::new(finish);
        configure(&f, &model.endpoint, disclose);
        fs::write(f.root.join("repo/unselected"), b"excluded-source-marker").unwrap();
        let before = states(&f);
        let started = Instant::now();
        let out = f.terminal(
            &[
                "propose-worktree",
                f.root.join("repo").to_str().unwrap(),
                "synthetic-model",
                "4096",
                "128",
                "Repair sentinel",
            ],
            "approve-all",
        );
        let requests = model.finish();
        assert_eq!(out["status"], 1, "{out}");
        assert_eq!(out["sent"], false, "{out}");
        assert_eq!(requests.len(), usize::from(disclose));
        let transcript = out["transcript"].as_str().unwrap();
        assert!(!transcript.contains("Type exactly"));
        if disclose {
            let request = &requests[0];
            assert_eq!(request["model"], "synthetic-model");
            assert_eq!(request["stream"], true);
            assert_eq!(request["max_tokens"], 128);
            assert_eq!(request["messages"].as_array().unwrap().len(), 2);
            let message = request["messages"][1]["content"].as_str().unwrap();
            assert!(message.contains("sentinel"));
            assert!(message.contains("human work"));
            assert!(!request.to_string().contains("excluded-source-marker"));
            assert!(transcript.contains("\"type\": \"context\""));
            assert!(
                transcript.contains(if finish == "stop" {
                    "\"type\": \"proposal\""
                } else {
                    "\"type\": \"error\""
                }),
                "{out}"
            );
        } else {
            assert!(!transcript.contains("\"type\": \"context\""));
        }
        assert_preserved(&f, &before);
        assert_eq!(
            fs::read(f.root.join("repo/unselected")).unwrap(),
            b"excluded-source-marker"
        );
        eprintln!(
            "actual API worktree proposal synthetic-upstream disclose={disclose} finish={finish} elapsed_us={}",
            started.elapsed().as_micros()
        );
    }
}
