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

use agentmage_platform_linux_inference::local_chat_api::{ApiError, ApiTokens, serve};
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
