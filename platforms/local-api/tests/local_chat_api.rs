// SPDX-License-Identifier: Apache-2.0
//! Synthetic API-to-upstream socket contracts, never real-model evidence.

use std::fs;
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use agentmage_local_api::{ApiError, ApiTokens, serve};
use serde_json::{Value, json};

const CLIENT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const CONTROL: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Api {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<(), ApiError>>>,
}
impl Api {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let worker = {
            let stop = stop.clone();
            thread::spawn(move || {
                serve(
                    listener,
                    ApiTokens::new(CLIENT.into(), CONTROL.into()).unwrap(),
                    stop,
                )
            })
        };
        Self {
            address,
            stop,
            worker: Some(worker),
        }
    }
    fn socket(&self) -> TcpStream {
        let s = TcpStream::connect(self.address).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(4))).unwrap();
        s.set_write_timeout(Some(Duration::from_secs(2))).unwrap();
        s
    }
    fn raw(&self, bytes: &[u8]) -> Vec<u8> {
        let mut s = self.socket();
        s.write_all(bytes).unwrap();
        let mut data = Vec::new();
        // The server can close without consuming a rejected body (TCP reset).
        loop {
            let mut b = [0; 4096];
            match s.read(&mut b) {
                Ok(0) => break,
                Ok(n) => data.extend_from_slice(&b[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => break,
                Err(e) => panic!("{e}"),
            }
        }
        data
    }
    fn bytes(&self, method: &str, path: &str, token: &str, body: &[u8], extra: &str) -> Vec<u8> {
        [format!("{method} {path} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{extra}\r\n", self.address, body.len()).into_bytes(), body.to_vec()].concat()
    }
    fn call(&self, method: &str, path: &str, token: &str, value: Option<Value>) -> (u16, Vec<u8>) {
        let body = value
            .map(|v| serde_json::to_vec(&v).unwrap())
            .unwrap_or_default();
        split(self.raw(&self.bytes(method, path, token, &body, "")))
    }
    fn configure(&self, endpoint: &str) {
        assert_eq!(
            self.call(
                "PUT",
                "/v1/settings",
                CONTROL,
                Some(
                    json!({"endpoint":endpoint,"key":"synthetic-key","approve_conversations":true})
                )
            )
            .0,
            200
        );
    }
}
impl Drop for Api {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            assert_eq!(worker.join().unwrap(), Ok(()));
        }
    }
}

fn split(bytes: Vec<u8>) -> (u16, Vec<u8>) {
    let end = bytes.windows(4).position(|v| v == b"\r\n\r\n").unwrap();
    let header = std::str::from_utf8(&bytes[..end]).unwrap();
    assert!(header.contains("Cache-Control: no-store"));
    assert!(!header.to_lowercase().contains("access-control-allow"));
    let status = header.split_whitespace().nth(1).unwrap().parse().unwrap();
    (status, bytes[end + 4..].to_vec())
}

fn upstream(handler: impl FnOnce(TcpStream) + Send + 'static) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
    let task = thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let started = Instant::now();
        loop {
            match listener.accept() {
                Ok((s, _)) => {
                    s.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                    s.set_write_timeout(Some(Duration::from_secs(2))).unwrap();
                    handler(s);
                    return;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(started.elapsed() < Duration::from_secs(5));
                    thread::sleep(Duration::from_millis(2));
                }
                Err(e) => panic!("{e}"),
            }
        }
    });
    (endpoint, task)
}

fn incoming(s: &mut TcpStream) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut one = [0];
    while !bytes.ends_with(b"\r\n\r\n") {
        s.read_exact(&mut one).unwrap();
        bytes.push(one[0]);
        assert!(bytes.len() < 8192);
    }
    let length: usize = std::str::from_utf8(&bytes)
        .unwrap()
        .lines()
        .find_map(|l| l.strip_prefix("Content-Length: "))
        .unwrap()
        .parse()
        .unwrap();
    let n = bytes.len();
    bytes.resize(n + length, 0);
    s.read_exact(&mut bytes[n..]).unwrap();
    bytes
}

fn reply(s: &mut TcpStream, media: &str, body: &str) {
    s.write_all(
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: {media}\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
        .as_bytes(),
    )
    .unwrap();
    s.shutdown(Shutdown::Write).unwrap();
}

fn chunk(text: &str) -> String {
    format!(
        "data: {}\n\n",
        json!({"model":"fixture","choices":[{"index":0,"delta":{"content":text},"finish_reason":null}]})
    )
}
fn end() -> String {
    format!(
        "data: {}\n\ndata: [DONE]\n\n",
        json!({"model":"fixture","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]})
    )
}
fn chat() -> Value {
    json!({"model":"fixture","messages":[{"role":"user","content":"Synthetic question"}],"max_tokens":32})
}
fn events(body: &[u8]) -> Vec<Value> {
    std::str::from_utf8(body)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}

fn coding_request() -> Value {
    json!({"model":"fixture", "task":"Fix add so it adds the numbers",
        "files":[{"path":"calc.py","content":"def add(a, b):\n    return a - b\n"}],
        "max_message_bytes":4096,"max_tokens":512})
}

fn coding_answer() -> String {
    json!({"edits":[{"path":"calc.py","old":"return a - b","new":"return a + b"}]}).to_string()
}

#[test]
fn coding_proposal_streams_selected_context_and_returns_only_an_unapplied_change() {
    let api = Api::new();
    let (endpoint, task) = upstream(|mut socket| {
        let raw = incoming(&mut socket);
        let start = raw.windows(4).position(|v| v == b"\r\n\r\n").unwrap() + 4;
        let request: Value = serde_json::from_slice(&raw[start..]).unwrap();
        assert_eq!(request["max_tokens"], 512);
        assert!(request.get("tools").is_none());
        let context: Value =
            serde_json::from_str(request["messages"][1]["content"].as_str().unwrap()).unwrap();
        assert_eq!(context["files"][0]["path"], "calc.py");
        assert!(
            context["excerpts"][0]["text"]
                .as_str()
                .unwrap()
                .contains("return a - b")
        );
        let answer = coding_answer();
        reply(
            &mut socket,
            "text/event-stream",
            &format!("{}{}{}", chunk(&answer[..10]), chunk(&answer[10..]), end()),
        );
    });
    api.configure(&endpoint);
    let (status, body) = api.call("POST", "/v1/code/proposals", CLIENT, Some(coding_request()));
    assert_eq!(status, 200);
    let events = events(&body);
    assert_eq!(events[0]["type"], "context");
    assert_eq!(events[0]["authority"], "proposal-only");
    assert!(events[0]["message_bytes"].as_u64().unwrap() <= 4096);
    assert_eq!(events[1]["type"], "text");
    let proposal = events.last().unwrap();
    assert_eq!(proposal["type"], "proposal");
    assert_eq!(proposal["applied"], false);
    assert_eq!(proposal["tested"], false);
    assert_eq!(proposal["changes"][0]["path"], "calc.py");
    assert_eq!(
        proposal["changes"][0]["content"],
        "def add(a, b):\n    return a + b\n"
    );
    assert_ne!(
        proposal["changes"][0]["before_sha256"],
        proposal["changes"][0]["after_sha256"]
    );
    task.join().unwrap();
    let activity: Value =
        serde_json::from_slice(&api.call("GET", "/v1/activity", CLIENT, None).1).unwrap();
    assert_eq!(
        activity["events"].as_array().unwrap().last().unwrap()["outcome"],
        "proposed"
    );
    assert!(
        !String::from_utf8_lossy(&api.call("GET", "/v1/activity", CLIENT, None).1)
            .contains("return a")
    );
}

#[test]
fn coding_requests_need_auth_endpoint_choice_and_valid_snapshot_before_inference() {
    let api = Api::new();
    assert_eq!(
        api.call(
            "POST",
            "/v1/code/proposals",
            "wrong",
            Some(coding_request())
        )
        .0,
        401
    );
    assert_eq!(
        api.call("POST", "/v1/code/proposals", CLIENT, Some(coding_request()))
            .0,
        428
    );
    // A listening unused endpoint proves invalid proposals never start inference.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    api.configure(&format!("http://{}/v1", listener.local_addr().unwrap()));
    for (field, value) in [
        ("max_message_bytes", json!(1)),
        ("max_tokens", json!(4097)),
        ("task", json!("")),
        ("files", json!([{"path":"../outside","content":"bytes"}])),
        ("command", json!("run")),
        ("endpoint", json!("http://127.0.0.1:1/v1")),
    ] {
        let mut request = coding_request();
        request[field] = value;
        assert_eq!(
            api.call("POST", "/v1/code/proposals", CLIENT, Some(request))
                .0,
            400
        );
    }
    let raw = api.bytes(
        "POST",
        "/v1/code/proposals",
        CLIENT,
        b"{\"model\":\"fixture\",\"model\":\"other\"}",
        "",
    );
    assert_eq!(split(api.raw(&raw)).0, 400);
    let mut positional_file = coding_request();
    positional_file["files"] = json!([["calc.py", "bytes"]]);
    assert_eq!(
        api.call("POST", "/v1/code/proposals", CLIENT, Some(positional_file))
            .0,
        400
    );
    let positional_request =
        json!(["fixture", "task", [{"path":"calc.py","content":"bytes"}], 4096, 512]);
    assert_eq!(
        api.call(
            "POST",
            "/v1/code/proposals",
            CLIENT,
            Some(positional_request)
        )
        .0,
        400
    );
    assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
}

#[test]
fn coding_nested_ambiguous_and_out_of_scope_model_edits_never_produce_a_proposal() {
    let deep = format!("{{\"edits\":{}0{}}}", "[".repeat(2000), "]".repeat(2000));
    for answer in [
        deep,
        "{\"edits\":[],\"edits\":[]}".into(),
        json!({"edits":[{"path":"../outside","old":"return a - b","new":"run commands"}]})
            .to_string(),
        json!({"edits":[{"path":"calc.py","old":"return a - b","new":"ok","command":"run"}]})
            .to_string(),
    ] {
        let api = Api::new();
        let (endpoint, task) = upstream(move |mut s| {
            incoming(&mut s);
            reply(
                &mut s,
                "text/event-stream",
                &format!("{}{}", chunk(&answer), end()),
            );
        });
        api.configure(&endpoint);
        let (status, body) = api.call("POST", "/v1/code/proposals", CLIENT, Some(coding_request()));
        assert_eq!(status, 200);
        let events = events(&body);
        assert!(!events.iter().any(|e| e["type"] == "proposal"));
        assert_eq!(events.last().unwrap()["type"], "error");
        task.join().unwrap();
    }
}

#[test]
fn coding_truncated_or_incomplete_generations_never_become_valid_edits() {
    let length_end = end().replace("\"stop\"", "\"length\"");
    for suffix in [length_end, String::new(), format!("{}late", end())] {
        let api = Api::new();
        let (endpoint, task) = upstream(move |mut s| {
            incoming(&mut s);
            reply(
                &mut s,
                "text/event-stream",
                &format!("{}{}", chunk(&coding_answer()), suffix),
            );
        });
        api.configure(&endpoint);
        let events = events(
            &api.call("POST", "/v1/code/proposals", CLIENT, Some(coding_request()))
                .1,
        );
        assert!(!events.iter().any(|e| e["type"] == "proposal"));
        assert_eq!(events.last().unwrap()["type"], "error");
        task.join().unwrap();
    }
}

#[test]
fn coding_cancellation_remains_available_and_inert() {
    let api = Api::new();
    let (tx, rx) = mpsc::channel();
    let (endpoint, task) = upstream(move |mut s| {
        incoming(&mut s);
        s.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n")
            .unwrap();
        tx.send(()).unwrap();
        let mut byte = [0];
        assert_eq!(s.read(&mut byte).unwrap(), 0);
    });
    api.configure(&endpoint);
    let mut socket = api.socket();
    socket
        .write_all(&api.bytes(
            "POST",
            "/v1/code/proposals",
            CLIENT,
            &serde_json::to_vec(&coding_request()).unwrap(),
            "",
        ))
        .unwrap();
    rx.recv_timeout(Duration::from_secs(3)).unwrap();
    assert_eq!(
        api.call("POST", "/v1/code/proposals", CLIENT, Some(coding_request()))
            .0,
        409
    );
    assert_eq!(api.call("DELETE", "/v1/settings", CONTROL, None).0, 409);
    assert_eq!(
        api.call(
            "PUT",
            "/v1/settings",
            CONTROL,
            Some(json!({"endpoint":endpoint,"approve_conversations":true}))
        )
        .0,
        409
    );
    assert_eq!(api.call("POST", "/v1/cancel", CLIENT, None).0, 200);
    let mut bytes = Vec::new();
    socket.read_to_end(&mut bytes).unwrap();
    let events = events(&split(bytes).1);
    assert_eq!(events.last().unwrap()["code"], "chat.transport.cancelled");
    assert!(!events.iter().any(|e| e["type"] == "proposal"));
    task.join().unwrap();
}

#[test]
fn authentication_origins_host_and_authority_fail_closed() {
    let api = Api::new();
    assert_eq!(api.call("GET", "/v1/status", "wrong", None).0, 401);
    for extra in [
        "Origin: null\r\n",
        "Origin: http://127.0.0.1\r\n",
        "Cookie: session=test\r\n",
        "Sec-Fetch-Site: same-origin\r\n",
        "Forwarded: host=localhost\r\n",
        "X-Forwarded-Host: localhost\r\n",
    ] {
        assert_eq!(
            split(api.raw(&api.bytes("GET", "/v1/status", CONTROL, b"", extra))).0,
            403
        );
    }
    let bytes = api.bytes("GET", "/v1/status", CLIENT, b"", "");
    let wrong_host = String::from_utf8(bytes)
        .unwrap()
        .replace(&api.address.to_string(), "localhost:1234");
    assert_eq!(split(api.raw(wrong_host.as_bytes())).0, 403);
    assert_eq!(api.call("GET", "/v1/settings", CLIENT, None).0, 403);
    assert_eq!(
        api.call("POST", "/v1/probe", CLIENT, Some(json!({"endpoints":[]})))
            .0,
        403
    );
    assert_eq!(api.call("DELETE", "/v1/settings", CLIENT, None).0, 403);
    let body = api.call("GET", "/v1/status", CLIENT, None);
    assert_eq!(body.0, 200);
    assert_eq!(
        serde_json::from_slice::<Value>(&body.1).unwrap()["configured"],
        false
    );
}

#[test]
fn discovery_cannot_authorize_chat_or_forward_a_credential() {
    let api = Api::new();
    let (endpoint, task) = upstream(|mut s| {
        let req = String::from_utf8(incoming(&mut s)).unwrap();
        assert!(req.starts_with("GET /v1/models"));
        assert!(!req.contains("Authorization:"));
        reply(&mut s, "application/json", r#"{"data":[{"id":"fixture"}]}"#);
    });
    assert_eq!(
        api.call(
            "POST",
            "/v1/probe",
            CONTROL,
            Some(json!({"endpoints":[endpoint]}))
        )
        .0,
        200
    );
    task.join().unwrap();
    assert_eq!(api.call("POST", "/v1/chat", CLIENT, Some(chat())).0, 428);
    assert_eq!(api.call("GET", "/v1/models", CLIENT, None).0, 428);
    for (token, approved, status) in [(CLIENT, true, 403), (CONTROL, false, 403)] {
        assert_eq!(
            api.call(
                "PUT",
                "/v1/settings",
                token,
                Some(json!({"endpoint":endpoint,"approve_conversations":approved}))
            )
            .0,
            status
        );
    }
    assert_eq!(api.call("POST", "/v1/chat", CLIENT, Some(chat())).0, 428);
}

#[test]
fn explicit_settings_route_key_to_exact_server_and_revoke_without_echo() {
    let api = Api::new();
    let (endpoint, task) = upstream(|mut s| {
        let req = String::from_utf8(incoming(&mut s)).unwrap();
        assert!(req.starts_with("GET /v1/models"));
        assert!(req.contains("Authorization: Bearer synthetic-key\r\n"));
        reply(&mut s, "application/json", r#"{"data":[{"id":"fixture"}]}"#);
    });
    api.configure(&endpoint);
    let settings = api.call("GET", "/v1/settings", CONTROL, None);
    assert!(!String::from_utf8_lossy(&settings.1).contains("synthetic-key"));
    assert_eq!(
        serde_json::from_slice::<Value>(&settings.1).unwrap()["endpoint"],
        endpoint
    );
    let models = api.call("GET", "/v1/models", CLIENT, None);
    assert_eq!(models.0, 200);
    assert_eq!(
        serde_json::from_slice::<Value>(&models.1).unwrap(),
        json!({"models":["fixture"]})
    );
    task.join().unwrap();
    assert_eq!(api.call("DELETE", "/v1/settings", CONTROL, None).0, 200);
    assert_eq!(api.call("GET", "/v1/models", CLIENT, None).0, 428);
}

#[test]
fn chat_streams_before_upstream_finishes_and_escapes_untrusted_text() {
    let api = Api::new();
    let (tx, rx) = mpsc::channel();
    let (endpoint, task) = upstream(move |mut s| {
        let req = String::from_utf8(incoming(&mut s)).unwrap();
        assert!(req.starts_with("POST /v1/chat/completions"));
        assert!(req.contains("Authorization: Bearer synthetic-key"));
        s.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n")
            .unwrap();
        s.write_all(chunk("<script>\n\u{1b}[31m\u{202e}plain").as_bytes())
            .unwrap();
        rx.recv_timeout(Duration::from_secs(3)).unwrap();
        s.write_all(end().as_bytes()).unwrap();
        s.shutdown(Shutdown::Write).unwrap();
    });
    api.configure(&endpoint);
    let mut s = api.socket();
    s.write_all(&api.bytes(
        "POST",
        "/v1/chat",
        CLIENT,
        &serde_json::to_vec(&chat()).unwrap(),
        "",
    ))
    .unwrap();
    let mut bytes = Vec::new();
    let mut b = [0];
    while !bytes.ends_with(b"\n")
        || !bytes.windows(4).any(|v| v == b"\r\n\r\n")
        || bytes.ends_with(b"\r\n\r\n")
    {
        s.read_exact(&mut b).unwrap();
        bytes.push(b[0]);
    }
    assert!(String::from_utf8_lossy(&bytes).contains("\"type\":\"text\""));
    tx.send(()).unwrap();
    s.read_to_end(&mut bytes).unwrap();
    let (status, body) = split(bytes);
    assert_eq!(status, 200);
    let e = events(&body);
    assert_eq!(
        e[0],
        json!({"type":"text","text":"<script>\n\u{1b}[31m\u{202e}plain"})
    );
    assert_eq!(e[1], json!({"type":"complete","finish":"stop"}));
    assert_eq!(e.len(), 2);
    task.join().unwrap();
}

#[test]
fn partial_and_late_invalid_streams_never_emit_completion() {
    for body in [
        chunk("partial"),
        format!("{}{}late", chunk("partial"), end()),
    ] {
        let api = Api::new();
        let (endpoint, task) = upstream(move |mut s| {
            incoming(&mut s);
            reply(&mut s, "text/event-stream", &body);
        });
        api.configure(&endpoint);
        let (status, body) = api.call("POST", "/v1/chat", CLIENT, Some(chat()));
        assert_eq!(status, 200);
        let e = events(&body);
        assert!(!e.iter().any(|v| v["type"] == "complete"));
        assert_eq!(e.last().unwrap()["type"], "error");
        assert_eq!(e.last().unwrap()["partial"], true);
        task.join().unwrap();
    }
}

#[test]
fn cancellation_is_available_during_chat_and_settings_cannot_race_it() {
    let api = Api::new();
    let (tx, rx) = mpsc::channel();
    let (endpoint, task) = upstream(move |mut s| {
        incoming(&mut s);
        s.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n")
            .unwrap();
        tx.send(()).unwrap();
        let mut b = [0];
        assert_eq!(s.read(&mut b).unwrap(), 0);
    });
    api.configure(&endpoint);
    let mut s = api.socket();
    s.write_all(&api.bytes(
        "POST",
        "/v1/chat",
        CLIENT,
        &serde_json::to_vec(&chat()).unwrap(),
        "",
    ))
    .unwrap();
    rx.recv_timeout(Duration::from_secs(3)).unwrap();
    assert_eq!(api.call("DELETE", "/v1/settings", CONTROL, None).0, 409);
    assert_eq!(
        api.call(
            "PUT",
            "/v1/settings",
            CONTROL,
            Some(json!({"endpoint":endpoint,"approve_conversations":true}))
        )
        .0,
        409
    );
    assert_eq!(api.call("GET", "/v1/models", CLIENT, None).0, 409);
    let started = Instant::now();
    assert_eq!(api.call("POST", "/v1/cancel", CLIENT, None).0, 200);
    let mut bytes = Vec::new();
    s.read_to_end(&mut bytes).unwrap();
    assert!(started.elapsed() < Duration::from_secs(2));
    let e = events(&split(bytes).1);
    assert_eq!(e.last().unwrap()["code"], "chat.transport.cancelled");
    assert_eq!(api.call("DELETE", "/v1/settings", CONTROL, None).0, 200);
    task.join().unwrap();
}

#[test]
fn malformed_ambiguous_or_oversized_requests_fail_before_upstream_io() {
    let api = Api::new();
    for (extra, status) in [
        ("Content-Length: 0\r\n", 400),
        ("Transfer-Encoding: chunked\r\n", 403),
        ("Authorization: Bearer second\r\n", 400),
        (" folded: value\r\n", 400),
        ("Expect: 100-continue\r\n", 403),
        ("Content-Encoding: gzip\r\n", 415),
    ] {
        assert_eq!(
            split(api.raw(&api.bytes("GET", "/v1/status", CLIENT, b"", extra))).0,
            status
        );
    }
    let raw = format!(
        "POST /v1/chat HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {CLIENT}\r\nContent-Length: 262145\r\n\r\n",
        api.address
    );
    assert_eq!(split(api.raw(raw.as_bytes())).0, 413);
    let huge = format!(
        "GET /v1/status HTTP/1.1\r\nHost: {}\r\nX-Padding: {}",
        api.address,
        "x".repeat(8192)
    );
    assert_eq!(split(api.raw(huge.as_bytes())).0, 413);
    let raw = api.bytes("POST", "/v1/chat", CLIENT, b"{", "");
    assert_eq!(split(api.raw(&raw)).0, 400);
    let raw = String::from_utf8(api.bytes("GET", "/v1/status", CLIENT, b"", ""))
        .unwrap()
        .replace("Content-Length: 0\r\n", "");
    assert_eq!(split(api.raw(raw.as_bytes())).0, 411);
    for endpoint in [
        "http://192.0.2.1:80/v1",
        "https://127.0.0.1:443/v1",
        "http://localhost/v1",
    ] {
        assert_eq!(
            api.call(
                "PUT",
                "/v1/settings",
                CONTROL,
                Some(json!({"endpoint":endpoint,"approve_conversations":true}))
            )
            .0,
            400
        );
    }
    assert_eq!(api.call("PUT","/v1/settings",CONTROL,Some(json!({"endpoint":"http://127.0.0.1:1/v1","approve_conversations":true,"key":"bad\r\nkey"}))).0,400);
    assert_eq!(api.call("GET", "/v2/status", CLIENT, None).0, 400);
}

#[test]
fn upstream_errors_are_content_free_after_stream_headers() {
    let api = Api::new();
    let (endpoint, task) = upstream(|mut s| {
        incoming(&mut s);
        s.write_all(b"HTTP/1.1 401 private-upstream-reason\r\nContent-Length: 21\r\n\r\nprivate-upstream-body").unwrap();
    });
    api.configure(&endpoint);
    let (status, body) = api.call("POST", "/v1/chat", CLIENT, Some(chat()));
    assert_eq!(status, 200);
    let text = std::str::from_utf8(&body).unwrap();
    for value in ["private-upstream", "synthetic-key", CLIENT, CONTROL] {
        assert!(!text.contains(value));
    }
    let events = events(&body);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["type"], "error");
    assert_eq!(events[0]["code"], "chat.transport.authentication");
    assert_eq!(events[0]["partial"], true);
    assert!(events[0]["message"].as_str().unwrap().contains("key"));
    task.join().unwrap();
}

#[test]
fn invalid_chat_and_unknown_fields_do_not_contact_the_configured_endpoint() {
    let api = Api::new();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    api.configure(&format!("http://{}/v1", listener.local_addr().unwrap()));
    for change in [
        json!({"model":"fixture","messages":[],"max_tokens":1}),
        json!({"model":"fixture","messages":[{"role":"tool","content":"x"}],"max_tokens":1}),
        json!({"model":"fixture","messages":[{"role":"user","content":"x"}],"max_tokens":0}),
        json!({"model":"fixture","messages":[{"role":"user","content":"x"}],"max_tokens":1,"endpoint":"http://127.0.0.1:1"}),
    ] {
        assert_eq!(api.call("POST", "/v1/chat", CLIENT, Some(change)).0, 400);
    }
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn slow_header_and_body_are_bounded_and_workers_recover() {
    let api = Api::new();
    let mut sockets = Vec::new();
    for _ in 0..4 {
        let mut s = api.socket();
        s.write_all(b"GET /v1/status HTTP/1.1\r\n").unwrap();
        sockets.push(s);
    }
    let start = Instant::now();
    for mut s in sockets {
        let mut b = Vec::new();
        s.read_to_end(&mut b).unwrap();
        assert_eq!(split(b).0, 408);
    }
    assert!(start.elapsed() < Duration::from_secs(4));
    let mut s = api.socket();
    let raw = format!(
        "POST /v1/chat HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {CLIENT}\r\nContent-Type: application/json\r\nContent-Length: 10\r\n\r\nx",
        api.address
    );
    s.write_all(raw.as_bytes()).unwrap();
    let mut bytes = Vec::new();
    s.read_to_end(&mut bytes).unwrap();
    assert_eq!(split(bytes).0, 408);
    assert_eq!(api.call("GET", "/v1/status", CLIENT, None).0, 200);
}

#[test]
fn shutdown_cancels_stalled_upstream_and_joins_workers() {
    let api = Api::new();
    let (tx, rx) = mpsc::channel();
    let (endpoint, task) = upstream(move |mut s| {
        incoming(&mut s);
        tx.send(()).unwrap();
        let mut b = [0];
        assert_eq!(s.read(&mut b).unwrap(), 0);
    });
    api.configure(&endpoint);
    let mut s = api.socket();
    s.write_all(&api.bytes("GET", "/v1/models", CLIENT, b"", ""))
        .unwrap();
    rx.recv_timeout(Duration::from_secs(3)).unwrap();
    let start = Instant::now();
    drop(api);
    assert!(start.elapsed() < Duration::from_secs(2));
    task.join().unwrap();
}

#[test]
fn activity_records_exact_choices_and_results_without_keys_or_conversations() {
    let api = Api::new();
    let (endpoint, task) = upstream(|mut s| {
        incoming(&mut s);
        reply(
            &mut s,
            "text/event-stream",
            &format!("{}{}", chunk("private-synthetic-answer"), end()),
        );
    });
    api.configure(&endpoint);
    assert_eq!(api.call("POST", "/v1/chat", CLIENT, Some(chat())).0, 200);
    task.join().unwrap();
    let (status, body) = api.call("GET", "/v1/activity", CLIENT, None);
    assert_eq!(status, 200);
    let text = std::str::from_utf8(&body).unwrap();
    for secret in [
        "synthetic-key",
        "Synthetic question",
        "private-synthetic-answer",
        CLIENT,
        CONTROL,
    ] {
        assert!(!text.contains(secret));
    }
    let log: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(log["retention"], "process-only");
    let events = log["events"].as_array().unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0]["action"], "settings.choice");
    assert_eq!(events[0]["endpoint"], endpoint);
    assert_eq!(events[1]["action"], "chat");
    assert_eq!(events[1]["outcome"], "started");
    assert_eq!(events[2]["outcome"], "finished");
    for (i, e) in events.iter().enumerate() {
        assert_eq!(e["sequence"], i + 1);
        assert_eq!(e["settings_revision"], 1);
    }
    for _ in 0..130 {
        assert_eq!(api.call("POST", "/v1/cancel", CLIENT, None).0, 200);
    }
    let (_, body) = api.call("GET", "/v1/activity", CLIENT, None);
    let log: Value = serde_json::from_slice(&body).unwrap();
    let events = log["events"].as_array().unwrap();
    assert_eq!(events.len(), 128);
    assert_eq!(log["last_sequence"], 133);
    assert_eq!(events[0]["sequence"], 6);
    assert_eq!(events[127]["sequence"], 133);
}

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "agentmage-api-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        if self.0.is_dir() {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
}

#[test]
fn token_setup_is_private_random_and_never_overwrites() {
    let root = Scratch::new();
    ApiTokens::initialize(&root.0).unwrap();
    assert_eq!(
        fs::metadata(&root.0).unwrap().permissions().mode() & 0o7777,
        0o700
    );
    let first = fs::read(root.0.join("client.token")).unwrap();
    let second = fs::read(root.0.join("control.token")).unwrap();
    assert_eq!(first.len(), 64);
    assert_ne!(first, second);
    assert!(ApiTokens::load(&root.0).is_ok());
    assert_eq!(ApiTokens::initialize(&root.0), Err(ApiError::Configuration));
    assert_eq!(fs::read(root.0.join("client.token")).unwrap(), first);
    assert!(ApiTokens::new(CLIENT.into(), CLIENT.into()).is_err());
    assert!(ApiTokens::new("short".into(), CONTROL.into()).is_err());
}

#[test]
fn token_loading_refuses_symlinks_hardlinks_modes_and_oversize() {
    let root = Scratch::new();
    ApiTokens::initialize(&root.0).unwrap();
    let client = root.0.join("client.token");
    fs::set_permissions(&client, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(ApiTokens::load(&root.0).is_err());
    fs::set_permissions(&client, fs::Permissions::from_mode(0o600)).unwrap();
    let other = root.0.join("other");
    fs::hard_link(&client, &other).unwrap();
    assert!(ApiTokens::load(&root.0).is_err());
    fs::remove_file(&other).unwrap();
    fs::rename(&client, &other).unwrap();
    symlink(&other, &client).unwrap();
    assert!(ApiTokens::load(&root.0).is_err());
    fs::remove_file(&client).unwrap();
    fs::rename(&other, &client).unwrap();
    fs::write(&client, "x".repeat(4096)).unwrap();
    assert!(ApiTokens::load(&root.0).is_err());
    fs::write(&client, CLIENT).unwrap();
    fs::set_permissions(&root.0, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(ApiTokens::load(&root.0).is_err());
}

#[test]
fn process_entrypoint_initializes_tokens_and_serves_authenticated_status() {
    use std::process::{Command, Stdio};
    let root = Scratch::new();
    let binary = env!("CARGO_BIN_EXE_agentmage-local-api");
    assert!(
        Command::new(binary)
            .args(["init-auth", root.0.to_str().unwrap()])
            .status()
            .unwrap()
            .success()
    );
    let probe = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = probe.local_addr().unwrap();
    drop(probe);
    let mut child = Command::new(binary)
        .args(["serve", &address.to_string(), root.0.to_str().unwrap()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // Always reap this test's child, including if an assertion panics.
    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    use std::io::BufRead;
    let mut line = String::new();
    let mut output = std::io::BufReader::new(child.stdout.take().unwrap());
    let _child = Child(child);
    output.read_line(&mut line).unwrap();
    assert_eq!(
        line,
        format!("AgentMage local API listening on {address}\n")
    );
    let token = fs::read_to_string(root.0.join("client.token")).unwrap();
    let mut s = TcpStream::connect(address).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(4))).unwrap();
    s.write_all(format!("GET /v1/status HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer {token}\r\nContent-Length: 0\r\n\r\n").as_bytes()).unwrap();
    let mut bytes = Vec::new();
    s.read_to_end(&mut bytes).unwrap();
    let (status, body) = split(bytes);
    assert_eq!(status, 200);
    assert_eq!(
        serde_json::from_slice::<Value>(&body).unwrap()["configured"],
        false
    );
}

struct WorkspaceFiles(PathBuf);
impl WorkspaceFiles {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "agentmage-api-workspace-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(root.join("calc.py"), "def add(a, b):\n    return a - b\n").unwrap();
        fs::write(root.join("unselected.py"), "private unselected content").unwrap();
        Self(root)
    }
    fn selection(&self, model: bool) -> Value {
        json!({"root":self.0,"paths":["calc.py"],"approve_read":true,"approve_model_context":model})
    }
    fn select(&self, api: &Api, model: bool) -> u64 {
        let (status, body) = api.call(
            "PUT",
            "/v1/code/workspace",
            CONTROL,
            Some(self.selection(model)),
        );
        assert_eq!(status, 200, "{}", String::from_utf8_lossy(&body));
        serde_json::from_slice::<Value>(&body).unwrap()["workspace_revision"]
            .as_u64()
            .unwrap()
    }
}
impl Drop for WorkspaceFiles {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn workspace_proposal(revision: u64) -> Value {
    json!({"workspace_revision":revision,"paths":["calc.py"],"model":"fixture",
        "task":"Fix add so it adds the numbers","max_message_bytes":4096,"max_tokens":512})
}
fn document(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap()
}

#[test]
fn workspace_requires_control_choice_exact_paths_and_explicit_model_disclosure() {
    let api = Api::new();
    let files = WorkspaceFiles::new();
    assert_eq!(
        api.call(
            "PUT",
            "/v1/code/workspace",
            CLIENT,
            Some(files.selection(false))
        )
        .0,
        403
    );
    assert_eq!(
        api.call("DELETE", "/v1/code/workspace", CLIENT, None).0,
        403
    );
    assert_eq!(api.call("GET", "/v1/code/workspace", CLIENT, None).0, 200);
    let mut refused = files.selection(false);
    refused["approve_read"] = json!(false);
    assert_eq!(
        api.call("PUT", "/v1/code/workspace", CONTROL, Some(refused))
            .0,
        403
    );
    assert_eq!(
        api.call(
            "PUT",
            "/v1/code/workspace",
            CONTROL,
            Some(files.selection(true))
        )
        .0,
        428
    );
    let revision = files.select(&api, false);
    let state = document(&api.call("GET", "/v1/code/workspace", CLIENT, None).1);
    assert_eq!(state["paths"], json!(["calc.py"]));
    assert_eq!(state["authority"], "read-only");
    assert_eq!(state["model_context_approved"], false);
    assert!(!state.to_string().contains(files.0.to_str().unwrap()));
    let request = json!({"workspace_revision":revision,"paths":["calc.py"]});
    let result = api.call("POST", "/v1/code/workspace/read", CLIENT, Some(request));
    assert_eq!(result.0, 200);
    assert_eq!(
        document(&result.1)["files"][0]["content"],
        "def add(a, b):\n    return a - b\n"
    );
    let mut invalid = files.selection(false);
    invalid["paths"] = json!(["missing.py"]);
    assert_eq!(
        api.call("PUT", "/v1/code/workspace", CONTROL, Some(invalid))
            .0,
        400
    );
    assert_eq!(
        document(&api.call("GET", "/v1/code/workspace", CLIENT, None).1)["workspace_revision"],
        revision
    );
    for paths in [
        json!(["unselected.py"]),
        json!(["../outside"]),
        json!(["calc.py", "calc.py"]),
        json!([]),
    ] {
        assert_eq!(
            api.call(
                "POST",
                "/v1/code/workspace/read",
                CLIENT,
                Some(json!({"workspace_revision":revision,"paths":paths}))
            )
            .0,
            400
        );
    }
    assert_eq!(
        api.call(
            "POST",
            "/v1/code/workspace/proposals",
            CLIENT,
            Some(workspace_proposal(revision))
        )
        .0,
        403
    );
    // Giving read consent never grants model disclosure, even after endpoint choice.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    api.configure(&format!("http://{}/v1", listener.local_addr().unwrap()));
    assert_eq!(
        api.call(
            "POST",
            "/v1/code/workspace/proposals",
            CLIENT,
            Some(workspace_proposal(revision))
        )
        .0,
        403
    );
    assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
    assert_eq!(
        api.call("DELETE", "/v1/code/workspace", CONTROL, None).0,
        200
    );
    assert_eq!(
        api.call(
            "POST",
            "/v1/code/workspace/read",
            CLIENT,
            Some(json!({"workspace_revision":revision,"paths":["calc.py"]}))
        )
        .0,
        409
    );
    assert_eq!(
        api.call(
            "POST",
            "/v1/code/workspace/read",
            CLIENT,
            Some(json!({"workspace_revision":revision+1,"paths":["calc.py"]}))
        )
        .0,
        428
    );
}

#[test]
fn workspace_file_to_model_to_checked_proposal_is_read_only_and_recorded() {
    let api = Api::new();
    let files = WorkspaceFiles::new();
    let (endpoint, task) = upstream(|mut socket| {
        let raw = incoming(&mut socket);
        let start = raw.windows(4).position(|v| v == b"\r\n\r\n").unwrap() + 4;
        let body = document(&raw[start..]);
        let context = document(body["messages"][1]["content"].as_str().unwrap().as_bytes());
        assert_eq!(context["files"].as_array().unwrap().len(), 1);
        assert_eq!(context["files"][0]["path"], "calc.py");
        assert!(!body.to_string().contains("private unselected"));
        assert!(!body.to_string().contains("agentmage-api-workspace"));
        reply(
            &mut socket,
            "text/event-stream",
            &format!("{}{}", chunk(&coding_answer()), end()),
        );
    });
    api.configure(&endpoint);
    let revision = files.select(&api, true);
    let (status, bytes) = api.call(
        "POST",
        "/v1/code/workspace/proposals",
        CLIENT,
        Some(workspace_proposal(revision)),
    );
    assert_eq!(status, 200);
    let rows = events(&bytes);
    let proposal = rows.last().unwrap();
    assert_eq!(rows[0]["workspace_revision"], revision);
    assert_eq!(proposal["type"], "proposal");
    assert_eq!(proposal["workspace_revision"], revision);
    assert_eq!(proposal["applied"], false);
    assert_eq!(proposal["tested"], false);
    assert!(
        proposal["changes"][0]["content"]
            .as_str()
            .unwrap()
            .contains("return a + b")
    );
    assert!(
        fs::read_to_string(files.0.join("calc.py"))
            .unwrap()
            .contains("return a - b")
    );
    task.join().unwrap();
    let activity = document(&api.call("GET", "/v1/activity", CLIENT, None).1);
    assert!(
        activity["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["action"] == "code.workspace.proposal"
                && v["outcome"] == "proposed"
                && v["workspace_revision"] == revision)
    );
    assert!(!activity.to_string().contains(files.0.to_str().unwrap()));
    assert!(!activity.to_string().contains("return a - b"));
}

#[test]
fn workspace_search_is_literal_bounded_and_uses_utf8_byte_offsets() {
    let api = Api::new();
    let files = WorkspaceFiles::new();
    fs::write(files.0.join("calc.py"), "é.*é.*").unwrap();
    let revision = files.select(&api, false);
    let request = |q: &str| json!({"workspace_revision":revision,"paths":["calc.py"],"query":q});
    let (status, bytes) = api.call(
        "POST",
        "/v1/code/workspace/search",
        CLIENT,
        Some(request(".*")),
    );
    assert_eq!(status, 200);
    let result = document(&bytes);
    assert_eq!(
        result["matches"],
        json!([{"path":"calc.py","start":2,"end":4,"text":".*"},{"path":"calc.py","start":6,"end":8,"text":".*"}])
    );
    assert_eq!(result["truncated"], false);
    fs::write(files.0.join("calc.py"), "x".repeat(129)).unwrap();
    let result = document(
        &api.call(
            "POST",
            "/v1/code/workspace/search",
            CLIENT,
            Some(request("x")),
        )
        .1,
    );
    assert_eq!(result["matches"].as_array().unwrap().len(), 128);
    assert_eq!(result["truncated"], true);
    for query in ["".to_string(), "a".repeat(129), "\0".into()] {
        assert_eq!(
            api.call(
                "POST",
                "/v1/code/workspace/search",
                CLIENT,
                Some(request(&query))
            )
            .0,
            400
        );
    }
    assert_eq!(
        api.call(
            "POST",
            "/v1/code/workspace/search",
            CLIENT,
            Some(request("absent"))
        )
        .0,
        200
    );
}

#[test]
fn workspace_endpoint_changes_and_path_substitution_require_fresh_authority() {
    let api = Api::new();
    let files = WorkspaceFiles::new();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
    api.configure(&endpoint);
    let revision = files.select(&api, true);
    api.configure(&endpoint); // Even the same address invalidates the earlier disclosure revision.
    assert_eq!(
        api.call(
            "POST",
            "/v1/code/workspace/proposals",
            CLIENT,
            Some(workspace_proposal(revision))
        )
        .0,
        403
    );
    assert_eq!(
        document(&api.call("GET", "/v1/code/workspace", CLIENT, None).1)["model_context_approved"],
        false
    );
    assert_eq!(
        api.call(
            "POST",
            "/v1/code/workspace/read",
            CLIENT,
            Some(json!({"workspace_revision":revision,"paths":["calc.py"]}))
        )
        .0,
        200
    );
    let fresh = files.select(&api, true);
    assert!(fresh > revision);
    fs::remove_file(files.0.join("calc.py")).unwrap();
    symlink(files.0.join("unselected.py"), files.0.join("calc.py")).unwrap();
    let result = api.call(
        "POST",
        "/v1/code/workspace/proposals",
        CLIENT,
        Some(workspace_proposal(fresh)),
    );
    assert_eq!(result.0, 400);
    assert_eq!(document(&result.1)["error"], "code.workspace.boundary");
    assert!(!String::from_utf8_lossy(&result.1).contains(files.0.to_str().unwrap()));
    assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
    assert_eq!(
        api.call(
            "POST",
            "/v1/code/workspace/proposals",
            CLIENT,
            Some(workspace_proposal(revision))
        )
        .0,
        409
    );
}

#[test]
fn workspace_json_shape_unknown_and_duplicate_fields_refuse_before_selection() {
    let api = Api::new();
    let files = WorkspaceFiles::new();
    let mut extra = files.selection(false);
    extra["command"] = json!("execute");
    for value in [
        extra,
        json!([files.0, ["calc.py"], true, false]),
        json!({"root":files.0,"paths":["calc.py"],"approve_read":true}),
    ] {
        assert_eq!(
            api.call("PUT", "/v1/code/workspace", CONTROL, Some(value))
                .0,
            400
        );
    }
    for extra in [
        "Origin: null\r\n",
        "Cookie: session=test\r\n",
        "Sec-Fetch-Site: same-origin\r\n",
    ] {
        let raw = api.bytes(
            "PUT",
            "/v1/code/workspace",
            CONTROL,
            &serde_json::to_vec(&files.selection(false)).unwrap(),
            extra,
        );
        assert_eq!(split(api.raw(&raw)).0, 403);
    }
    let raw = api.bytes("PUT", "/v1/code/workspace", CONTROL,
        b"{\"root\":\"/\",\"root\":\"/other\",\"paths\":[\"file.py\"],\"approve_read\":true,\"approve_model_context\":false}", "");
    assert_eq!(split(api.raw(&raw)).0, 400);
    assert_eq!(
        document(&api.call("GET", "/v1/code/workspace", CLIENT, None).1)["selected"],
        false
    );
    for path in ["read", "search", "proposals"] {
        assert_eq!(
            api.call(
                "POST",
                &format!("/v1/code/workspace/{path}"),
                CLIENT,
                Some(json!([0, ["calc.py"]]))
            )
            .0,
            400
        );
    }
}

#[test]
fn coding_api_answer_cap_is_terminal_and_recorded_without_a_proposal() {
    let api = Api::new();
    let (endpoint, task) = upstream(|mut socket| {
        incoming(&mut socket);
        // Each event fits the transport cap; their sum exceeds the API cap.
        let body = format!(
            "{}{}",
            (0..9).map(|_| chunk(&"x".repeat(8192))).collect::<String>(),
            end()
        );
        let _ = socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n{}", body.len(), body).as_bytes());
    });
    api.configure(&endpoint);
    let rows = events(
        &api.call("POST", "/v1/code/proposals", CLIENT, Some(coding_request()))
            .1,
    );
    assert_eq!(rows.last().unwrap()["code"], "code.proposal.limit");
    assert_eq!(
        rows.iter()
            .filter_map(|r| r.get("text").and_then(Value::as_str))
            .map(str::len)
            .sum::<usize>(),
        65536
    );
    assert!(!rows.iter().any(|r| r["type"] == "proposal"));
    task.join().unwrap();
    let activity = document(&api.call("GET", "/v1/activity", CLIENT, None).1);
    assert!(
        activity["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["outcome"] == "code.proposal.limit")
    );
}

#[test]
fn workspace_proposal_blocks_scope_or_settings_replacement_until_cancelled() {
    let api = Api::new();
    let files = WorkspaceFiles::new();
    let (tx, rx) = mpsc::channel();
    let (endpoint, task) = upstream(move |mut socket| {
        incoming(&mut socket);
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n")
            .unwrap();
        tx.send(()).unwrap();
        let mut byte = [0];
        assert_eq!(socket.read(&mut byte).unwrap(), 0);
    });
    api.configure(&endpoint);
    let revision = files.select(&api, true);
    let mut socket = api.socket();
    socket
        .write_all(&api.bytes(
            "POST",
            "/v1/code/workspace/proposals",
            CLIENT,
            &serde_json::to_vec(&workspace_proposal(revision)).unwrap(),
            "",
        ))
        .unwrap();
    rx.recv_timeout(Duration::from_secs(3)).unwrap();
    assert_eq!(api.call("GET", "/v1/status", CLIENT, None).0, 200);
    assert_eq!(
        api.call(
            "PUT",
            "/v1/code/workspace",
            CONTROL,
            Some(files.selection(true))
        )
        .0,
        409
    );
    assert_eq!(
        api.call("DELETE", "/v1/code/workspace", CONTROL, None).0,
        409
    );
    assert_eq!(api.call("DELETE", "/v1/settings", CONTROL, None).0, 409);
    assert_eq!(
        api.call(
            "PUT",
            "/v1/settings",
            CONTROL,
            Some(json!({"endpoint":endpoint,"approve_conversations":true}))
        )
        .0,
        409
    );
    assert_eq!(api.call("POST", "/v1/cancel", CLIENT, None).0, 200);
    let mut bytes = Vec::new();
    socket.read_to_end(&mut bytes).unwrap();
    let rows = events(&split(bytes).1);
    assert_eq!(rows.last().unwrap()["code"], "chat.transport.cancelled");
    assert!(!rows.iter().any(|r| r["type"] == "proposal"));
    task.join().unwrap();
    assert_eq!(
        api.call("DELETE", "/v1/code/workspace", CONTROL, None).0,
        200
    );
}

fn idle(api: &Api) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while document(&api.call("GET", "/v1/status", CLIENT, None).1)["busy"] == true {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(2));
    }
}

fn change_request(api: &Api, files: &WorkspaceFiles) -> (u64, Value) {
    let (endpoint, task) = upstream(|mut socket| {
        incoming(&mut socket);
        reply(
            &mut socket,
            "text/event-stream",
            &format!("{}{}", chunk(&coding_answer()), end()),
        );
    });
    api.configure(&endpoint);
    let revision = files.select(api, true);
    idle(api);
    let (status, bytes) = api.call(
        "POST",
        "/v1/code/workspace/proposals",
        CLIENT,
        Some(workspace_proposal(revision)),
    );
    assert_eq!(status, 200);
    let rows = events(&bytes);
    assert_eq!(rows.last().unwrap()["type"], "proposal");
    let expected = rows[0]["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| json!({"path":s["path"],"sha256":s["sha256"]}))
        .collect::<Vec<_>>();
    let request = json!({"workspace_revision":revision,"paths":["calc.py"],
        "expected_sources":expected,"task":"Fix add so it adds the numbers", "max_message_bytes":4096,
        "answer":coding_answer()});
    task.join().unwrap();
    idle(api);
    (revision, request)
}

fn prepare_change(api: &Api, request: Value) -> Value {
    let (status, bytes) = api.call("POST", "/v1/code/workspace/changes", CLIENT, Some(request));
    assert_eq!(status, 200, "{}", String::from_utf8_lossy(&bytes));
    let value = document(&bytes);
    assert_eq!(value["applied"], false);
    assert_eq!(value["tested"], false);
    assert_eq!(value["isolation_verified"], false);
    assert!(
        value["preview"]["complete_diffs"][0]
            .as_str()
            .unwrap()
            .contains("return a + b")
    );
    value
}

fn apply_request(prepared: &Value, approve: bool) -> Value {
    json!({"workspace_revision":prepared["workspace_revision"],
        "change_set_sha256":prepared["preview"]["change_set_sha256"],
        "preview_sha256":prepared["preview"]["preview_sha256"],"approve":approve})
}

#[test]
fn checked_model_edit_needs_exact_control_approval_and_retains_receipts_without_replay() {
    let api = Api::new();
    let files = WorkspaceFiles::new();
    let (_, request) = change_request(&api, &files);
    let prepared = prepare_change(&api, request);
    assert!(
        fs::read_to_string(files.0.join("calc.py"))
            .unwrap()
            .contains("return a - b")
    );
    let selection = document(
        &api.call("GET", "/v1/code/workspace/changes", CLIENT, None)
            .1,
    );
    assert_eq!(selection["pending"]["preview"], prepared["preview"]);
    let approval = apply_request(&prepared, true);
    assert_eq!(
        api.call(
            "POST",
            "/v1/code/workspace/apply",
            CLIENT,
            Some(approval.clone())
        )
        .0,
        403
    );
    let mut forged = approval.clone();
    forged["preview_sha256"] = json!("0".repeat(64));
    assert_eq!(
        api.call("POST", "/v1/code/workspace/apply", CONTROL, Some(forged))
            .0,
        409
    );
    let (status, bytes) = api.call(
        "POST",
        "/v1/code/workspace/apply",
        CONTROL,
        Some(approval.clone()),
    );
    assert_eq!(status, 200, "{}", String::from_utf8_lossy(&bytes));
    let result = document(&bytes);
    assert_eq!(result["outcome"], "committed");
    assert_eq!(result["tested"], false);
    assert_eq!(
        result["receipts"].as_array().unwrap().last().unwrap()["status"],
        "verified"
    );
    assert!(
        fs::read_to_string(files.0.join("calc.py"))
            .unwrap()
            .contains("return a + b")
    );
    assert_eq!(
        fs::read_to_string(files.0.join("unselected.py")).unwrap(),
        "private unselected content"
    );
    // The new apply releases the lease and records its result before replying.
    assert_eq!(
        api.call("POST", "/v1/code/workspace/apply", CONTROL, Some(approval))
            .0,
        428
    );
    let history = document(
        &api.call("GET", "/v1/code/workspace/changes", CLIENT, None)
            .1,
    );
    assert_eq!(history["pending"], Value::Null);
    assert_eq!(history["records"].as_array().unwrap().len(), 2);
    assert_eq!(history["records"][0]["outcome"], "dispatching");
    assert_eq!(history["records"][1]["result"], result);
    let activity = document(&api.call("GET", "/v1/activity", CLIENT, None).1);
    for value in [history, activity] {
        let text = value.to_string();
        assert!(!text.contains(files.0.to_str().unwrap()));
        assert!(!text.contains("return a"));
        assert!(!text.contains(CLIENT));
        assert!(!text.contains(CONTROL));
    }
}

#[test]
fn denied_superseded_and_revoked_previews_cannot_write() {
    let api = Api::new();
    let files = WorkspaceFiles::new();
    let (_, request) = change_request(&api, &files);
    let first = prepare_change(&api, request.clone());
    let second = prepare_change(&api, request.clone());
    assert_ne!(
        first["preview"]["preview_sha256"],
        second["preview"]["preview_sha256"]
    );
    assert_eq!(
        api.call(
            "POST",
            "/v1/code/workspace/apply",
            CONTROL,
            Some(apply_request(&first, true))
        )
        .0,
        409
    );
    let (status, bytes) = api.call(
        "POST",
        "/v1/code/workspace/apply",
        CONTROL,
        Some(apply_request(&second, false)),
    );
    assert_eq!(status, 200);
    assert_eq!(document(&bytes)["outcome"], "denied");
    assert_eq!(
        api.call(
            "POST",
            "/v1/code/workspace/apply",
            CONTROL,
            Some(apply_request(&second, true))
        )
        .0,
        428
    );
    let third = prepare_change(&api, request);
    assert_eq!(api.call("DELETE", "/v1/settings", CONTROL, None).0, 200);
    assert_eq!(
        api.call(
            "POST",
            "/v1/code/workspace/apply",
            CONTROL,
            Some(apply_request(&third, true))
        )
        .0,
        428
    );
    // No model disclosure is needed to prepare an already supplied edit locally.
    let (_, request) = change_request(&api, &files);
    let fourth = prepare_change(&api, request);
    assert_eq!(
        api.call("DELETE", "/v1/code/workspace", CONTROL, None).0,
        200
    );
    assert_eq!(
        api.call(
            "POST",
            "/v1/code/workspace/apply",
            CONTROL,
            Some(apply_request(&fourth, true))
        )
        .0,
        428
    );
    assert!(
        fs::read_to_string(files.0.join("calc.py"))
            .unwrap()
            .contains("return a - b")
    );
}

#[test]
fn stale_model_context_and_later_file_substitution_never_overwrite_person_edits() {
    let api = Api::new();
    let files = WorkspaceFiles::new();
    let (_, request) = change_request(&api, &files);
    let mut wrong = request.clone();
    wrong["expected_sources"][0]["sha256"] = json!("0".repeat(64));
    assert_eq!(
        api.call("POST", "/v1/code/workspace/changes", CLIENT, Some(wrong))
            .0,
        409
    );
    fs::write(
        files.0.join("calc.py"),
        "# person's prefix\ndef add(a,b):\n    return a - b\n",
    )
    .unwrap();
    assert_eq!(
        api.call(
            "POST",
            "/v1/code/workspace/changes",
            CLIENT,
            Some(request.clone())
        )
        .0,
        409
    );
    fs::write(
        files.0.join("calc.py"),
        "def add(a, b):\n    return a - b\n",
    )
    .unwrap();
    let prepared = prepare_change(&api, request);
    fs::remove_file(files.0.join("calc.py")).unwrap();
    symlink(files.0.join("unselected.py"), files.0.join("calc.py")).unwrap();
    let (status, bytes) = api.call(
        "POST",
        "/v1/code/workspace/apply",
        CONTROL,
        Some(apply_request(&prepared, true)),
    );
    assert_eq!(status, 409);
    assert_eq!(document(&bytes)["effect"], "unknown");
    assert_eq!(
        fs::read_to_string(files.0.join("unselected.py")).unwrap(),
        "private unselected content"
    );
    assert_eq!(
        api.call(
            "POST",
            "/v1/code/workspace/apply",
            CONTROL,
            Some(apply_request(&prepared, true))
        )
        .0,
        428
    );
}

#[test]
fn change_json_and_edit_injection_refuse_without_replacing_a_valid_preview() {
    let api = Api::new();
    let files = WorkspaceFiles::new();
    let (_, request) = change_request(&api, &files);
    let prepared = prepare_change(&api, request.clone());
    for (field, value) in [
        ("root", json!("/")),
        ("command", json!("execute")),
        ("expected_sources", json!([["calc.py", "digest"]])),
        (
            "answer",
            json!(r#"{"edits":[{"path":"../unselected.py","old":"a","new":"b"}]}"#),
        ),
        ("answer", json!(r#"{"edits":[],"approve":true}"#)),
    ] {
        let mut wrong = request.clone();
        wrong[field] = value;
        assert_eq!(
            api.call("POST", "/v1/code/workspace/changes", CLIENT, Some(wrong))
                .0,
            400
        );
    }
    let mut duplicate = request.clone();
    duplicate["expected_sources"]
        .as_array_mut()
        .unwrap()
        .push(request["expected_sources"][0].clone());
    assert_eq!(
        api.call(
            "POST",
            "/v1/code/workspace/changes",
            CLIENT,
            Some(duplicate)
        )
        .0,
        409
    );
    let mut wrong = apply_request(&prepared, true);
    wrong["approve"] = json!("true");
    assert_eq!(
        api.call("POST", "/v1/code/workspace/apply", CONTROL, Some(wrong))
            .0,
        400
    );
    assert_eq!(
        document(
            &api.call("GET", "/v1/code/workspace/changes", CLIENT, None)
                .1
        )["pending"]["preview"],
        prepared["preview"]
    );
    assert!(
        fs::read_to_string(files.0.join("calc.py"))
            .unwrap()
            .contains("return a - b")
    );
}

#[test]
fn concurrent_identical_approvals_dispatch_exactly_one_kernel_transaction() {
    let api = Api::new();
    let files = WorkspaceFiles::new();
    let (_, request) = change_request(&api, &files);
    let prepared = prepare_change(&api, request);
    let raw = api.bytes(
        "POST",
        "/v1/code/workspace/apply",
        CONTROL,
        &serde_json::to_vec(&apply_request(&prepared, true)).unwrap(),
        "",
    );
    let mut a = api.socket();
    let mut b = api.socket();
    a.write_all(&raw).unwrap();
    b.write_all(&raw).unwrap();
    let mut left = Vec::new();
    let mut right = Vec::new();
    a.read_to_end(&mut left).unwrap();
    b.read_to_end(&mut right).unwrap();
    let a = split(left);
    let b = split(right);
    assert_eq!([a.0, b.0].into_iter().filter(|s| *s == 200).count(), 1);
    assert!([a.0, b.0].into_iter().all(|s| [200, 409, 428].contains(&s)));
    let value = if a.0 == 200 {
        document(&a.1)
    } else {
        document(&b.1)
    };
    assert_eq!(value["outcome"], "committed");
    let history = document(
        &api.call("GET", "/v1/code/workspace/changes", CLIENT, None)
            .1,
    );
    assert_eq!(history["records"].as_array().unwrap().len(), 2);
}

#[test]
fn workspace_disclosure_choice_can_bind_the_displayed_settings_revision() {
    let api = Api::new();
    let files = WorkspaceFiles::new();
    api.configure("http://127.0.0.1:19999/v1");
    let settings: Value =
        serde_json::from_slice(&api.call("GET", "/v1/settings", CONTROL, None).1).unwrap();
    api.configure("http://127.0.0.1:19998/v1");
    let request = json!({"root":files.0,"paths":["calc.py"],"approve_read":true,
        "approve_model_context":true,"settings_revision":settings["settings_revision"]});
    let (status, body) = api.call("PUT", "/v1/code/workspace", CONTROL, Some(request.clone()));
    assert_eq!(status, 409);
    assert_eq!(
        serde_json::from_slice::<Value>(&body).unwrap()["error"],
        "code.workspace.settings-stale"
    );
    let current: Value =
        serde_json::from_slice(&api.call("GET", "/v1/settings", CONTROL, None).1).unwrap();
    let mut fresh = request;
    fresh["settings_revision"] = current["settings_revision"].clone();
    fresh["settings_endpoint"] = settings["endpoint"].clone();
    assert_eq!(
        api.call("PUT", "/v1/code/workspace", CONTROL, Some(fresh.clone()))
            .0,
        409
    );
    fresh["settings_endpoint"] = current["endpoint"].clone();
    // Lease release can lag the error response in this existing selection route.
    let start = Instant::now();
    while serde_json::from_slice::<Value>(&api.call("GET", "/v1/status", CLIENT, None).1).unwrap()["busy"]
        == true
    {
        assert!(start.elapsed() < Duration::from_secs(1));
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        api.call("PUT", "/v1/code/workspace", CONTROL, Some(fresh))
            .0,
        200
    );
}

#[test]
fn repository_map_needs_explicit_control_and_grants_no_source_or_model_authority() {
    let api = Api::new();
    let files = WorkspaceFiles::new();
    fs::create_dir(files.0.join(".git")).unwrap();
    fs::write(files.0.join(".git/config"), "secret-token").unwrap();
    let request = json!({"root":files.0,"query":"calc","approve_discovery":true});
    assert_eq!(
        api.call("POST", "/v1/code/map", CLIENT, Some(request.clone()))
            .0,
        403
    );
    let mut denied = request.clone();
    denied["approve_discovery"] = json!(false);
    assert_eq!(
        api.call("POST", "/v1/code/map", CONTROL, Some(denied)).0,
        403
    );
    for invalid in [
        json!([files.0, "calc", true]),
        json!({"root":files.0,"query":"calc","approve_discovery":true,"extra":true}),
        json!({"root":files.0,"query":"!!!","approve_discovery":true}),
    ] {
        assert_eq!(
            api.call("POST", "/v1/code/map", CONTROL, Some(invalid)).0,
            400
        );
    }
    let duplicate = format!(
        "{{\"root\":{},\"query\":\"calc\",\"query\":\"other\",\"approve_discovery\":true}}",
        json!(files.0)
    );
    assert_eq!(
        split(api.raw(&api.bytes("POST", "/v1/code/map", CONTROL, duplicate.as_bytes(), ""))).0,
        400
    );
    let (status, bytes) = api.call("POST", "/v1/code/map", CONTROL, Some(request));
    assert_eq!(status, 200);
    let map = document(&bytes);
    assert_eq!(map["authority"], "metadata-only");
    assert_eq!(map["ranking"], "lexical-path");
    assert_eq!(map["entries"][0], json!({"path":"calc.py","score":3}));
    assert_eq!(map["entries"].as_array().unwrap().len(), 2);
    assert_eq!(map["excluded"], 1);
    assert!(
        !String::from_utf8(bytes)
            .unwrap()
            .contains("private unselected content")
    );
    assert_eq!(
        document(&api.call("GET", "/v1/code/workspace", CLIENT, None).1)["selected"],
        false
    );
    assert_eq!(
        document(&api.call("GET", "/v1/status", CLIENT, None).1)["busy"],
        false
    );
    assert_eq!(
        api.call(
            "POST",
            "/v1/code/workspace/read",
            CLIENT,
            Some(json!({"workspace_revision":0,"paths":["calc.py"]}))
        )
        .0,
        428
    );
    let events = document(&api.call("GET", "/v1/activity", CLIENT, None).1);
    assert!(
        events["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["action"] == "code.map" && e["outcome"] == "mapped")
    );
}

#[test]
fn selection_binds_map_root_without_losing_an_existing_scope_on_refusal() {
    let api = Api::new();
    let files = WorkspaceFiles::new();
    let revision = files.select(&api, false);
    let request = json!({"root":files.0,"query":"calc","approve_discovery":true});
    let map = document(&api.call("POST", "/v1/code/map", CONTROL, Some(request)).1);
    let mut selection = files.selection(false);
    selection["expected_root_sha256"] = json!("0".repeat(64));
    let (status, error) = api.call(
        "PUT",
        "/v1/code/workspace",
        CONTROL,
        Some(selection.clone()),
    );
    assert_eq!(status, 409);
    assert_eq!(document(&error)["error"], "code.workspace.root-stale");
    let state = document(&api.call("GET", "/v1/code/workspace", CLIENT, None).1);
    assert_eq!(state["workspace_revision"], revision);
    selection["expected_root_sha256"] = map["root_sha256"].clone();
    assert_eq!(
        api.call(
            "PUT",
            "/v1/code/workspace",
            CONTROL,
            Some(selection.clone())
        )
        .0,
        200
    );
    let moved = files.0.with_extension("moved");
    fs::rename(&files.0, &moved).unwrap();
    fs::create_dir(&files.0).unwrap();
    fs::write(files.0.join("calc.py"), "replacement").unwrap();
    let (status, error) = api.call("PUT", "/v1/code/workspace", CONTROL, Some(selection));
    assert_eq!(status, 409);
    assert_eq!(document(&error)["error"], "code.workspace.root-stale");
    fs::remove_dir_all(moved).unwrap();
}
