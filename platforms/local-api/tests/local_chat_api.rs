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
use sha2::{Digest, Sha256};

const CLIENT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const CONTROL: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[test]
fn thinking_control_is_trusted_explicit_and_cannot_be_injected_by_chat_or_proposals() {
    let api = Api::new();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
    let settings = || {
        let (status, body) = api.call("GET", "/v1/settings", CONTROL, None);
        assert_eq!(status, 200);
        serde_json::from_slice::<Value>(&body).unwrap()
    };
    assert_eq!(settings()["thinking_mode"], Value::Null);
    api.configure(&endpoint);
    assert_eq!(settings()["thinking_mode"], "provider_default");
    let choice =
        json!({"endpoint":endpoint,"approve_conversations":true,"thinking_mode":"disabled"});
    assert_eq!(
        api.call("PUT", "/v1/settings", CLIENT, Some(choice.clone()))
            .0,
        403
    );
    assert_eq!(settings()["thinking_mode"], "provider_default");
    assert_eq!(
        api.call("PUT", "/v1/settings", CONTROL, Some(choice.clone()))
            .0,
        200
    );
    let accepted = settings();
    assert_eq!(accepted["thinking_mode"], "disabled");
    for invalid in [
        json!("enabled"),
        json!(true),
        json!(0),
        Value::Null,
        json!({"enable_thinking":false}),
    ] {
        let mut bad = choice.clone();
        bad["thinking_mode"] = invalid;
        assert_eq!(api.call("PUT", "/v1/settings", CONTROL, Some(bad)).0, 400);
        assert_eq!(settings(), accepted);
    }
    let duplicate = format!(
        "{{\"endpoint\":{endpoint:?},\"approve_conversations\":true,\"thinking_mode\":\"disabled\",\"thinking_mode\":\"provider_default\"}}"
    );
    assert_eq!(
        split(api.raw(&api.bytes("PUT", "/v1/settings", CONTROL, duplicate.as_bytes(), ""))).0,
        400
    );
    assert_eq!(settings(), accepted);
    for route in ["/v1/chat", "/v1/code/proposals"] {
        let mut input = if route == "/v1/chat" {
            chat()
        } else {
            coding_request()
        };
        input["thinking_mode"] = json!("disabled");
        assert_eq!(api.call("POST", route, CLIENT, Some(input)).0, 400);
    }
    assert!(
        listener.accept().is_err(),
        "refused settings and bodies cannot connect"
    );
    api.configure(&endpoint);
    assert_eq!(settings()["thinking_mode"], "provider_default");
    assert_eq!(api.call("DELETE", "/v1/settings", CONTROL, None).0, 200);
    assert_eq!(settings()["thinking_mode"], Value::Null);
}

#[test]
fn thinking_request_in_either_framing_never_promotes_reasoning_to_an_answer() {
    for mode in ["stream", "complete"] {
        for answer_present in [false, true] {
            let api = Api::new();
            let (endpoint, task) = upstream(move |mut socket| {
                let raw = incoming(&mut socket);
                let start = raw.windows(4).position(|v| v == b"\r\n\r\n").unwrap() + 4;
                let input: Value = serde_json::from_slice(&raw[start..]).unwrap();
                assert_eq!(
                    input["chat_template_kwargs"],
                    json!({"enable_thinking":false})
                );
                assert_eq!(input["stream"], mode == "stream");
                assert!(input.get("tools").is_none());
                let answer = if answer_present {
                    coding_answer()
                } else {
                    String::new()
                };
                let finish = if answer_present { "stop" } else { "length" };
                if mode == "complete" {
                    reply(
                        &mut socket,
                        "application/json",
                        &json!({"model":"fixture","choices":[{
                        "index":0,"message":{"role":"assistant","content":answer,
                        "reasoning_content":"untrusted reasoning"},"finish_reason":finish}]})
                        .to_string(),
                    );
                } else {
                    let body = format!(
                        "data: {}\n\ndata: {}\n\ndata: [DONE]\n\n",
                        json!({"model":"fixture","choices":[{"index":0,"delta":{"content":answer,"reasoning_content":"untrusted reasoning"},"finish_reason":null}]}),
                        json!({"model":"fixture","choices":[{"index":0,"delta":{},"finish_reason":finish}]})
                    );
                    reply(&mut socket, "text/event-stream", &body);
                }
            });
            assert_eq!(api.call("PUT", "/v1/settings", CONTROL, Some(json!({
                "endpoint":endpoint,"approve_conversations":true,"response_mode":mode,"thinking_mode":"disabled"
            }))).0, 200);
            let (status, body) =
                api.call("POST", "/v1/code/proposals", CLIENT, Some(coding_request()));
            assert_eq!(status, 200);
            let rows = events(&body);
            let last = rows.last().unwrap();
            if answer_present {
                assert_eq!(last["type"], "proposal");
                assert_eq!(last["applied"], false);
                assert_eq!(last["tested"], false);
            } else {
                assert_eq!(last["type"], "error");
                assert_eq!(last["code"], "chat.response.incomplete");
                assert!(!rows.iter().any(|row| row["type"] == "proposal"));
            }
            task.join().unwrap();
        }
    }
}

#[test]
fn unsupported_thinking_control_is_an_error_without_fallback() {
    let api = Api::new();
    let (endpoint, task) = upstream(move |mut socket| {
        let raw = incoming(&mut socket);
        let start = raw.windows(4).position(|v| v == b"\r\n\r\n").unwrap() + 4;
        let input: Value = serde_json::from_slice(&raw[start..]).unwrap();
        assert_eq!(
            input["chat_template_kwargs"],
            json!({"enable_thinking":false})
        );
        socket
            .write_all(
                b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            )
            .unwrap();
    });
    assert_eq!(
        api.call(
            "PUT",
            "/v1/settings",
            CONTROL,
            Some(json!({
                "endpoint":endpoint,"approve_conversations":true,"thinking_mode":"disabled"
            }))
        )
        .0,
        200
    );
    let (status, body) = api.call("POST", "/v1/code/proposals", CLIENT, Some(coding_request()));
    assert_eq!(status, 200);
    let rows = events(&body);
    assert_eq!(rows.last().unwrap()["code"], "chat.transport.status");
    assert!(!rows.iter().any(|row| row["type"] == "proposal"));
    task.join().unwrap();
}

#[test]
fn json_object_format_requires_trusted_settings_and_resets_to_text() {
    let api = Api::new();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
    let settings = || {
        let (status, body) = api.call("GET", "/v1/settings", CONTROL, None);
        assert_eq!(status, 200);
        serde_json::from_slice::<Value>(&body).unwrap()
    };
    assert_eq!(settings()["response_format"], Value::Null);
    api.configure(&endpoint);
    assert_eq!(settings()["response_format"], "text");
    let choice =
        json!({"endpoint":endpoint,"approve_conversations":true,"response_format":"json_object"});
    assert_eq!(
        api.call("PUT", "/v1/settings", CLIENT, Some(choice.clone()))
            .0,
        403
    );
    assert_eq!(settings()["response_format"], "text");
    assert_eq!(
        api.call("PUT", "/v1/settings", CONTROL, Some(choice.clone()))
            .0,
        200
    );
    let accepted = settings();
    assert_eq!(accepted["response_format"], "json_object");
    for invalid in [
        json!("auto"),
        json!(true),
        json!(0),
        Value::Null,
        json!({"type":"json_object"}),
    ] {
        let mut bad = choice.clone();
        bad["response_format"] = invalid;
        assert_eq!(api.call("PUT", "/v1/settings", CONTROL, Some(bad)).0, 400);
        assert_eq!(settings(), accepted);
    }
    let duplicate = format!(
        "{{\"endpoint\":{endpoint:?},\"approve_conversations\":true,\"response_format\":\"json_object\",\"response_format\":\"text\"}}"
    );
    assert_eq!(
        split(api.raw(&api.bytes("PUT", "/v1/settings", CONTROL, duplicate.as_bytes(), ""))).0,
        400
    );
    assert_eq!(settings(), accepted);
    for route in ["/v1/chat", "/v1/code/proposals"] {
        let mut input = if route == "/v1/chat" {
            chat()
        } else {
            coding_request()
        };
        input["response_format"] = json!("json_object");
        assert_eq!(api.call("POST", route, CLIENT, Some(input)).0, 400);
    }
    assert!(
        listener.accept().is_err(),
        "settings and refused bodies cannot connect"
    );
    api.configure(&endpoint);
    assert_eq!(settings()["response_format"], "text");
    assert_eq!(api.call("DELETE", "/v1/settings", CONTROL, None).0, 200);
    assert_eq!(settings()["response_format"], Value::Null);
}

#[test]
fn json_object_request_never_substitutes_for_strict_proposal_validation() {
    for mode in ["stream", "complete"] {
        for (answer, refusal) in [
            (coding_answer(), None),
            (
                format!("```json\n{}\n```", coding_answer()),
                Some("code.proposal.format"),
            ),
            ("{\"edits\":[]}".into(), Some("code.proposal.format")),
            (
                "{\"edits\":[{\"path\":\"../outside\",\"old\":\"x\",\"new\":\"y\"}]}".into(),
                Some("code.proposal.edit"),
            ),
        ] {
            let api = Api::new();
            let (endpoint, task) = upstream(move |mut socket| {
                let raw = incoming(&mut socket);
                let start = raw.windows(4).position(|v| v == b"\r\n\r\n").unwrap() + 4;
                let input: Value = serde_json::from_slice(&raw[start..]).unwrap();
                assert_eq!(input["response_format"], json!({"type":"json_object"}));
                assert_eq!(input["stream"], mode == "stream");
                assert!(input.get("tools").is_none());
                if mode == "complete" {
                    reply(&mut socket, "application/json", &json!({"model":"fixture","choices":[{
                        "index":0,"message":{"role":"assistant","content":answer},"finish_reason":"stop"}]}).to_string());
                } else {
                    let body = format!(
                        "data: {}\n\ndata: {}\n\ndata: [DONE]\n\n",
                        json!({"model":"fixture","choices":[{"index":0,"delta":{"content":answer},"finish_reason":null}]}),
                        json!({"model":"fixture","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]})
                    );
                    reply(&mut socket, "text/event-stream", &body);
                }
            });
            assert_eq!(api.call("PUT", "/v1/settings", CONTROL, Some(json!({
                "endpoint":endpoint,"approve_conversations":true,"response_mode":mode,"response_format":"json_object"
            }))).0, 200);
            let (status, body) =
                api.call("POST", "/v1/code/proposals", CLIENT, Some(coding_request()));
            assert_eq!(status, 200);
            let rows = events(&body);
            let last = rows.last().unwrap();
            if let Some(code) = refusal {
                assert_eq!(last["type"], "error");
                assert_eq!(last["code"], code);
                assert!(!rows.iter().any(|v| v["type"] == "proposal"));
            } else {
                assert_eq!(last["type"], "proposal");
                assert_eq!(last["applied"], false);
                assert_eq!(last["tested"], false);
            }
            task.join().unwrap();
        }
    }
}

#[test]
fn unsupported_json_object_request_reports_status_without_retry_or_fallback() {
    let api = Api::new();
    let (endpoint, task) = upstream(move |mut socket| {
        let raw = incoming(&mut socket);
        let start = raw.windows(4).position(|v| v == b"\r\n\r\n").unwrap() + 4;
        let input: Value = serde_json::from_slice(&raw[start..]).unwrap();
        assert_eq!(input["response_format"], json!({"type":"json_object"}));
        socket
            .write_all(
                b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            )
            .unwrap();
    });
    assert_eq!(
        api.call(
            "PUT",
            "/v1/settings",
            CONTROL,
            Some(json!({
                "endpoint":endpoint,"approve_conversations":true,"response_format":"json_object"
            }))
        )
        .0,
        200
    );
    let (status, body) = api.call("POST", "/v1/code/proposals", CLIENT, Some(coding_request()));
    assert_eq!(status, 200);
    let rows = events(&body);
    let last = rows.last().unwrap();
    assert_eq!(last["type"], "error");
    assert_eq!(last["code"], "chat.transport.status");
    assert!(!rows.iter().any(|v| v["type"] == "proposal"));
    task.join().unwrap();
}

#[test]
fn complete_response_mode_is_a_trusted_immutable_setting_with_stream_default() {
    let api = Api::new();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
    let settings = || {
        let (status, body) = api.call("GET", "/v1/settings", CONTROL, None);
        assert_eq!(status, 200);
        serde_json::from_slice::<Value>(&body).unwrap()
    };
    assert_eq!(settings()["response_mode"], Value::Null);
    api.configure(&endpoint);
    assert_eq!(settings()["response_mode"], "stream");
    let choice =
        json!({"endpoint":endpoint,"approve_conversations":true,"response_mode":"complete"});
    assert_eq!(
        api.call("PUT", "/v1/settings", CLIENT, Some(choice.clone()))
            .0,
        403
    );
    assert_eq!(settings()["response_mode"], "stream");
    assert_eq!(
        api.call("PUT", "/v1/settings", CONTROL, Some(choice.clone()))
            .0,
        200
    );
    let accepted = settings();
    assert_eq!(accepted["response_mode"], "complete");
    for invalid in [json!("auto"), json!(true), json!(0), Value::Null, json!({})] {
        let mut bad = choice.clone();
        bad["response_mode"] = invalid;
        assert_eq!(api.call("PUT", "/v1/settings", CONTROL, Some(bad)).0, 400);
        assert_eq!(settings(), accepted);
    }
    let duplicate = format!(
        "{{\"endpoint\":{endpoint:?},\"approve_conversations\":true,\"response_mode\":\"complete\",\"response_mode\":\"stream\"}}"
    );
    assert_eq!(
        split(api.raw(&api.bytes("PUT", "/v1/settings", CONTROL, duplicate.as_bytes(), ""))).0,
        400
    );
    assert_eq!(settings(), accepted);
    for route in ["/v1/chat", "/v1/code/proposals"] {
        let mut input = if route == "/v1/chat" {
            chat()
        } else {
            coding_request()
        };
        input["response_mode"] = json!("stream");
        assert_eq!(api.call("POST", route, CLIENT, Some(input)).0, 400);
    }
    assert!(
        listener.accept().is_err(),
        "settings or invalid bodies must not connect"
    );
    api.configure(&endpoint);
    assert_eq!(settings()["response_mode"], "stream");
    assert_eq!(api.call("DELETE", "/v1/settings", CONTROL, None).0, 200);
    assert_eq!(settings()["response_mode"], Value::Null);
}

#[test]
fn complete_responses_reach_chat_and_strict_proposals_over_the_same_versioned_api() {
    for coding in [false, true] {
        let api = Api::new();
        let (tx, rx) = mpsc::channel();
        let (endpoint, task) = upstream(move |mut socket| {
            let raw = incoming(&mut socket);
            let start = raw.windows(4).position(|v| v == b"\r\n\r\n").unwrap() + 4;
            let input: Value = serde_json::from_slice(&raw[start..]).unwrap();
            assert_eq!(input["stream"], false);
            assert!(input.get("stream_options").is_none());
            assert!(input.get("response_mode").is_none());
            tx.send((
                raw.len() - start,
                Sha256::digest(&raw[start..])
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>(),
            ))
            .unwrap();
            reply(&mut socket, "application/json", &json!({"model":"fixture","choices":[{
                "index":0,"message":{"role":"assistant","content":if coding {coding_answer()} else {"synthetic answer".into()}},
                "finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":3,"total_tokens":13}}).to_string());
        });
        assert_eq!(
            api.call(
                "PUT",
                "/v1/settings",
                CONTROL,
                Some(json!({"endpoint":endpoint,
            "approve_conversations":true,"response_mode":"complete"}))
            )
            .0,
            200
        );
        let mut input = if coding { coding_request() } else { chat() };
        input["include_usage"] = json!(true);
        let (status, body) = api.call(
            "POST",
            if coding {
                "/v1/code/proposals"
            } else {
                "/v1/chat"
            },
            CLIENT,
            Some(input),
        );
        assert_eq!(status, 200);
        let rows = events(&body);
        let last = rows.last().unwrap();
        assert_eq!(last["type"], if coding { "proposal" } else { "complete" });
        assert_eq!(last["usage"]["total_tokens"], 13);
        let (bytes, digest) = rx.recv().unwrap();
        if coding {
            assert_eq!(last["applied"], false);
            assert_eq!(last["tested"], false);
            assert_eq!(rows[0]["model_binding"]["request_body_bytes"], bytes);
            assert_eq!(rows[0]["model_binding"]["request_body_sha256"], digest);
            assert_eq!(rows[0]["model_binding"], last["model_binding"]);
            assert_eq!(
                rows[0]["model_binding"]["response_body_limit_bytes"],
                4 * 1024 * 1024
            );
        } else {
            assert_eq!(rows[0]["text"], "synthetic answer");
        }
        task.join().unwrap();
    }
}
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

fn assert_endpoint_runtime_observation(rows: &[Value]) {
    let first = &rows[0];
    let terminal = rows.last().unwrap();
    let expected = json!({
        "kind":"selected_endpoint",
        "identity":{
            "endpoint":first["model_binding"]["endpoint"],
            "model":first["model_binding"]["model"]
        }
    });
    assert_eq!(first["runtime_model_binding"], expected);
    assert_eq!(terminal["runtime_model_binding"], expected);
    assert!(first.get("token_accounting").is_none());
    assert!(first.get("runtime_model_result").is_none());
    let context = &first["runtime_endpoint_context"];
    assert_eq!(context["version"], 1);
    assert_eq!(context["messages"], first["model_binding"]["context"]);
    assert!(context["content_bytes"].as_u64().unwrap() > 0);
    assert!(
        context["content_bytes"].as_u64().unwrap()
            < context["messages"]["serialized_messages_bytes"]
                .as_u64()
                .unwrap()
    );
    assert!(context.get("input_tokens").is_none());
    assert!(context.get("authority").is_none());
    let phase = &terminal["endpoint_phase_usage"];
    assert_eq!(phase["dispatch_attempts"], 1);
    assert_eq!(
        phase["prepared_body_bytes"],
        first["model_binding"]["request_body_bytes"]
    );
    assert!(phase.get("input_tokens").is_none());
    assert!(phase.get("peak_memory_bytes").is_none());
    if terminal["response_complete"] == true {
        let result = &terminal["runtime_model_result"];
        assert_eq!(result["version"], 1);
        assert_eq!(
            result["finish"],
            if terminal["code"] == "code.proposal.incomplete" {
                "output_token_limit"
            } else {
                "stop"
            }
        );
        assert_eq!(result["request"], first["model_binding"]);
        assert_eq!(result["request"], terminal["model_binding"]);
        let text: String = rows
            .iter()
            .filter(|r| r["type"] == "text")
            .map(|r| r["text"].as_str().unwrap())
            .collect();
        assert_eq!(result["response_text_bytes"], text.len());
        assert_eq!(
            result["response_text_sha256"],
            Sha256::digest(text.as_bytes())
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        assert_eq!(result["resources"]["resident_memory_bytes"], Value::Null);
        assert_eq!(result["resources"]["accelerator_memory_bytes"], Value::Null);
        let elapsed: Duration =
            serde_json::from_value(result["resources"]["dispatch_elapsed"].clone()).unwrap();
        assert!(!elapsed.is_zero());
        assert_eq!(phase["completed_response_text_bytes"], text.len());
        assert_eq!(
            phase["completed_dispatch_elapsed_ms"],
            u64::try_from(elapsed.as_nanos().div_ceil(1_000_000)).unwrap()
        );
        assert_eq!(
            result["token_accounting"],
            terminal["token_accounting"]["observation"]
        );
        let accounting = &terminal["token_accounting"];
        assert_eq!(accounting["kind"], "selected_endpoint");
        assert_eq!(
            accounting["observation"]["verified_input_tokens"],
            Value::Null
        );
        assert_eq!(
            accounting["observation"]["verified_output_tokens"],
            Value::Null
        );
        assert_eq!(
            accounting["observation"]["provider_reported"],
            terminal["usage"]
        );
        assert!(
            accounting["observation"]
                .get("rendered_prompt_tokens")
                .is_none()
        );
    } else {
        assert_eq!(phase["completed_response_text_bytes"], Value::Null);
        assert_eq!(phase["completed_dispatch_elapsed_ms"], Value::Null);
        assert!(terminal.get("runtime_model_result").is_none());
        assert!(terminal.get("token_accounting").is_none());
        assert!(terminal.get("usage").is_none());
    }
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
    let (tx, rx) = mpsc::channel();
    let (endpoint, task) = upstream(move |mut socket| {
        let raw = incoming(&mut socket);
        let start = raw.windows(4).position(|v| v == b"\r\n\r\n").unwrap() + 4;
        let request: Value = serde_json::from_slice(&raw[start..]).unwrap();
        let digest: String = Sha256::digest(&raw[start..])
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let array_start = raw[start..]
            .windows(b"\"messages\":".len())
            .position(|w| w == b"\"messages\":")
            .unwrap()
            + start
            + b"\"messages\":".len();
        let array_end = raw
            .windows(b",\"stream\":true".len())
            .rposition(|w| w == b",\"stream\":true")
            .unwrap();
        let array = &raw[array_start..array_end];
        assert!(array.starts_with(b"[") && array.ends_with(b"]"));
        let context = json!({"message_count":request["messages"].as_array().unwrap().len(),
            "serialized_messages_bytes":array.len(), "serialized_messages_sha256":
            Sha256::digest(array).iter().map(|b| format!("{b:02x}")).collect::<String>()});
        tx.send((raw[start..].len(), digest, request.clone(), context))
            .unwrap();
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
    let (body_bytes, body_sha256, sent, observed_context) =
        rx.recv_timeout(Duration::from_secs(3)).unwrap();
    // Independently reconstruct the binding from the upstream's observed body.
    // The transport's socket tests also hash those actual bytes independently.
    use agentmage_platform_linux_inference::local_chat::{
        ChatMessage, ChatRole, LocalChatEndpoint,
    };
    use agentmage_platform_linux_inference::local_chat_transport::{
        LocalChatApproval, LocalChatTransport, TransportLimits,
    };
    let peer = LocalChatEndpoint::parse(&endpoint).unwrap();
    let transport =
        LocalChatTransport::new(peer.clone(), None, TransportLimits::default()).unwrap();
    let messages = [
        ChatMessage {
            role: ChatRole::System,
            content: sent["messages"][0]["content"].as_str().unwrap(),
        },
        ChatMessage {
            role: ChatRole::User,
            content: sent["messages"][1]["content"].as_str().unwrap(),
        },
    ];
    let prepared = transport
        .prepare_chat(
            &LocalChatApproval::for_endpoint(peer),
            "fixture",
            &messages,
            512,
            false,
        )
        .unwrap();
    let expected = serde_json::to_value(prepared.binding()).unwrap();
    assert_eq!(events[0]["model_binding"], expected);
    assert_eq!(proposal["model_binding"], expected);
    assert_eq!(expected["request_body_bytes"], body_bytes);
    assert_eq!(expected["request_body_sha256"], body_sha256);
    assert_eq!(expected["context"], observed_context);
    assert_eq!(expected["verified_input_tokens"], Value::Null);
    assert!(!String::from_utf8_lossy(&body).contains("synthetic-key"));
    assert_eq!(proposal["response_complete"], true);
    assert_endpoint_runtime_observation(&events);
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
    for (suffix, response_complete) in [
        (length_end, true),
        (String::new(), false),
        (format!("{}late", end()), false),
    ] {
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
        assert_eq!(
            events.last().unwrap()["response_complete"],
            response_complete
        );
        assert_eq!(
            events.last().unwrap()["model_binding"],
            events[0]["model_binding"]
        );
        assert_eq!(
            events[0]["model_binding"]["verified_input_tokens"],
            Value::Null
        );
        assert_endpoint_runtime_observation(&events);
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
    assert_eq!(events.last().unwrap()["response_complete"], false);
    assert_endpoint_runtime_observation(&events);
    assert_eq!(
        events.last().unwrap()["model_binding"],
        events[0]["model_binding"]
    );
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
fn proposal_binding_refuses_stale_or_partial_identity_before_source_reads_and_transport() {
    let api = Api::new();
    let files = WorkspaceFiles::new();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
    api.configure(&endpoint);
    let revision = files.select(&api, true);
    let selected = document(&api.call("GET", "/v1/code/workspace", CLIENT, None).1);
    let settings = document(&api.call("GET", "/v1/settings", CONTROL, None).1);
    let scope =
        agentmage_platform_linux::LinuxCodingReadScope::select(&files.0, &["calc.py".into()])
            .unwrap();
    let mut input = workspace_proposal(revision);
    input["workspace_selection_id"] = selected["workspace_selection_id"].clone();
    input["settings_revision"] = settings["settings_revision"].clone();
    input["settings_endpoint"] = settings["endpoint"].clone();
    input["expected_root_sha256"] = json!(scope.root_sha256());
    // A source read here would fail at the file boundary. The exact stale-pin
    // refusals below must happen earlier, without touching a replacement file.
    fs::rename(files.0.join("calc.py"), files.0.join("parked.py")).unwrap();
    symlink(files.0.join("unselected.py"), files.0.join("calc.py")).unwrap();
    for (field, value, status, code) in [
        (
            "settings_revision",
            json!(0),
            409,
            "code.workspace.settings-stale",
        ),
        (
            "settings_endpoint",
            json!("http://127.0.0.1:1/v1"),
            409,
            "code.workspace.settings-stale",
        ),
        (
            "settings_revision",
            Value::Null,
            400,
            "code.workspace.settings-binding",
        ),
        (
            "settings_endpoint",
            Value::Null,
            400,
            "code.workspace.settings-binding",
        ),
        (
            "expected_root_sha256",
            json!("9".repeat(64)),
            409,
            "code.workspace.root-stale",
        ),
        (
            "workspace_selection_id",
            json!("9".repeat(64)),
            409,
            "code.workspace.selection-stale",
        ),
    ] {
        let mut stale = input.clone();
        stale[field] = value;
        let result = api.call("POST", "/v1/code/workspace/proposals", CLIENT, Some(stale));
        assert_eq!(
            result.0,
            status,
            "{field}: {}",
            String::from_utf8_lossy(&result.1)
        );
        assert_eq!(document(&result.1)["error"], code, "{field}");
        assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
    }
    fs::remove_file(files.0.join("calc.py")).unwrap();
    fs::rename(files.0.join("parked.py"), files.0.join("calc.py")).unwrap();
    // Reapproving the same address and selection cannot renew the old request's
    // settings pin, even when the caller updates its workspace identities.
    api.configure(&endpoint);
    input["workspace_revision"] = json!(files.select(&api, true));
    input["workspace_selection_id"] =
        document(&api.call("GET", "/v1/code/workspace", CLIENT, None).1)["workspace_selection_id"]
            .clone();
    let result = api.call("POST", "/v1/code/workspace/proposals", CLIENT, Some(input));
    assert_eq!(result.0, 409);
    assert_eq!(
        document(&result.1)["error"],
        "code.workspace.settings-stale"
    );
    assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
}

#[test]
fn exactly_bound_workspace_proposal_uses_the_original_endpoint_and_read_path() {
    let api = Api::new();
    let files = WorkspaceFiles::new();
    let (endpoint, worker) = upstream(|mut socket| {
        let raw = incoming(&mut socket);
        let start = raw.windows(4).position(|v| v == b"\r\n\r\n").unwrap() + 4;
        let body = document(&raw[start..]);
        assert!(
            body["messages"][1]["content"]
                .as_str()
                .unwrap()
                .contains("calc.py")
        );
        assert!(!body.to_string().contains("expected_root_sha256"));
        assert!(!body.to_string().contains("workspace_selection_id"));
        reply(
            &mut socket,
            "text/event-stream",
            &format!("{}{}", chunk(&coding_answer()), end()),
        );
    });
    api.configure(&endpoint);
    let mut input = workspace_proposal(files.select(&api, true));
    let selected = document(&api.call("GET", "/v1/code/workspace", CLIENT, None).1);
    let settings = document(&api.call("GET", "/v1/settings", CONTROL, None).1);
    let scope =
        agentmage_platform_linux::LinuxCodingReadScope::select(&files.0, &["calc.py".into()])
            .unwrap();
    input["workspace_selection_id"] = selected["workspace_selection_id"].clone();
    input["settings_revision"] = settings["settings_revision"].clone();
    input["settings_endpoint"] = settings["endpoint"].clone();
    input["expected_root_sha256"] = json!(scope.root_sha256());
    let (status, bytes) = api.call("POST", "/v1/code/workspace/proposals", CLIENT, Some(input));
    assert_eq!(status, 200);
    let rows = events(&bytes);
    assert_endpoint_runtime_observation(&rows);
    assert_eq!(rows.last().unwrap()["type"], "proposal");
    worker.join().unwrap();
    assert!(
        fs::read_to_string(files.0.join("calc.py"))
            .unwrap()
            .contains("return a - b")
    );
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
    assert_endpoint_runtime_observation(&rows);
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

#[test]
fn opted_in_usage_is_provider_reported_and_never_implies_verified_tokens() {
    for mode in ["reported", "missing", "bad", "duplicate", "late", "length"] {
        let api = Api::new();
        let (endpoint, task) = upstream(move |mut socket| {
            let raw = incoming(&mut socket);
            let start = raw.windows(4).position(|v| v == b"\r\n\r\n").unwrap() + 4;
            let request: Value = serde_json::from_slice(&raw[start..]).unwrap();
            assert_eq!(request["stream_options"], json!({"include_usage":true}));
            assert_eq!(request["max_tokens"], 32);
            let usage = format!(
                "data: {}\n\n",
                json!({"model":"fixture","choices":[],"usage":{
                "prompt_tokens":12,"completion_tokens":4,"total_tokens":if mode == "bad" {17} else {16}}})
            );
            let finish = format!(
                "data: {}\n\n",
                json!({"model":"fixture","choices":[{
                "index":0,"delta":{},"finish_reason":if mode == "length" {"length"} else {"stop"}}]})
            );
            let mut body = format!("{}{}", chunk("untrusted answer"), finish);
            if mode != "missing" {
                body.push_str(&usage);
            }
            if mode == "duplicate" {
                body.push_str(&usage);
            }
            body.push_str("data: [DONE]\n\n");
            if mode == "late" {
                body.push_str("late");
            }
            reply(&mut socket, "text/event-stream", &body);
        });
        api.configure(&endpoint);
        let mut input = chat();
        input["include_usage"] = json!(true);
        let (status, body) = api.call("POST", "/v1/chat", CLIENT, Some(input));
        assert_eq!(status, 200);
        let rows = events(&body);
        if matches!(mode, "bad" | "duplicate" | "late") {
            assert!(!rows.iter().any(|row| row["type"] == "complete"));
            assert!(rows.iter().all(|row| row.get("usage").is_none()));
            assert_eq!(rows.last().unwrap()["partial"], true);
        } else {
            let last = rows.last().unwrap();
            assert_eq!(last["type"], "complete");
            assert_eq!(
                last["finish"],
                if mode == "length" { "length" } else { "stop" }
            );
            assert_eq!(last["usage_source"], "provider-reported");
            assert_eq!(
                last["usage"],
                if mode == "missing" {
                    Value::Null
                } else {
                    json!({"prompt_tokens":12,"completion_tokens":4,"total_tokens":16})
                }
            );
        }
        task.join().unwrap();
    }
}

#[test]
fn proposal_usage_uses_the_same_validated_stream_without_authorizing_edits() {
    for mode in ["report", "missing", "bad", "invalid-edit"] {
        let api = Api::new();
        let (endpoint, task) = upstream(move |mut s| {
            let raw = incoming(&mut s);
            let start = raw.windows(4).position(|v| v == b"\r\n\r\n").unwrap() + 4;
            let request: Value = serde_json::from_slice(&raw[start..]).unwrap();
            assert_eq!(request["stream_options"], json!({"include_usage":true}));
            assert!(request.get("tools").is_none());
            let answer = if mode == "invalid-edit" {
                "not a strict edit".into()
            } else {
                coding_answer()
            };
            let mut body = chunk(&answer);
            let tail = end();
            let (finish, done) = tail.split_once("data: [DONE]").unwrap();
            body.push_str(finish);
            if mode != "missing" {
                body.push_str(&format!("data: {}\n\n", json!({"model":"fixture","choices":[],
                    "usage":{"prompt_tokens":100,"completion_tokens":40,"total_tokens": if mode == "bad" {141} else {140}}})));
            }
            body.push_str("data: [DONE]");
            body.push_str(done);
            reply(&mut s, "text/event-stream", &body);
        });
        api.configure(&endpoint);
        let mut input = coding_request();
        input["include_usage"] = json!(true);
        let (status, body) = api.call("POST", "/v1/code/proposals", CLIENT, Some(input));
        assert_eq!(status, 200);
        let rows = events(&body);
        let last = rows.last().unwrap();
        assert_eq!(last["model_binding"], rows[0]["model_binding"]);
        assert_eq!(last["model_binding"]["provider_usage_requested"], true);
        assert_eq!(last["model_binding"]["verified_input_tokens"], Value::Null);
        assert_eq!(last["response_complete"], mode != "bad");
        assert_endpoint_runtime_observation(&rows);
        if matches!(mode, "bad" | "invalid-edit") {
            assert_eq!(last["type"], "error");
            if mode == "bad" {
                assert!(rows.iter().all(|r| r.get("usage").is_none()));
            } else {
                assert_eq!(last["usage"]["total_tokens"], 140);
            }
            assert!(!rows.iter().any(|r| r["type"] == "proposal"));
        } else {
            assert_eq!(last["type"], "proposal");
            assert_eq!(last["applied"], false);
            assert_eq!(last["tested"], false);
            assert_eq!(last["usage_source"], "provider-reported");
            assert_eq!(
                last["usage"],
                if mode == "missing" {
                    Value::Null
                } else {
                    json!({"prompt_tokens":100,"completion_tokens":40,"total_tokens":140})
                }
            );
        }
        task.join().unwrap();
    }
}

#[test]
fn trusted_timeout_settings_preserve_defaults_and_refuse_invalid_changes_without_io() {
    let api = Api::new();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
    api.configure(&endpoint);
    let settings = || {
        let (status, body) = api.call("GET", "/v1/settings", CONTROL, None);
        assert_eq!(status, 200);
        serde_json::from_slice::<Value>(&body).unwrap()
    };
    assert_eq!(settings()["total_timeout_ms"], 120_000);
    assert_eq!(settings()["idle_timeout_ms"], 30_000);
    let input = json!({"endpoint":endpoint,"approve_conversations":true,
        "total_timeout_ms":300_000,"idle_timeout_ms":300_000});
    assert_eq!(
        api.call("PUT", "/v1/settings", CLIENT, Some(input.clone()))
            .0,
        403
    );
    assert_eq!(
        api.call("PUT", "/v1/settings", CONTROL, Some(input.clone()))
            .0,
        200
    );
    let accepted = settings();
    assert_eq!(accepted["total_timeout_ms"], 300_000);
    assert_eq!(accepted["idle_timeout_ms"], 300_000);
    assert!(accepted.get("key").is_none());
    for (field, invalid) in [
        ("total_timeout_ms", json!(0)),
        ("total_timeout_ms", json!(300_001)),
        ("total_timeout_ms", json!(u64::MAX)),
        ("total_timeout_ms", json!(-1)),
        ("total_timeout_ms", json!(1.5)),
        ("total_timeout_ms", json!("300000")),
        ("total_timeout_ms", Value::Null),
        ("idle_timeout_ms", json!(0)),
        ("idle_timeout_ms", json!(300_001)),
        ("idle_timeout_ms", json!(u64::MAX)),
        ("idle_timeout_ms", json!(true)),
        ("idle_timeout_ms", Value::Null),
    ] {
        let mut bad = input.clone();
        bad[field] = invalid;
        assert_eq!(api.call("PUT", "/v1/settings", CONTROL, Some(bad)).0, 400);
        assert_eq!(settings(), accepted);
    }
    let duplicate = format!(
        "{{\"endpoint\":{endpoint:?},\"approve_conversations\":true,\"total_timeout_ms\":300000,\"total_timeout_ms\":1}}"
    );
    assert_eq!(
        split(api.raw(&api.bytes("PUT", "/v1/settings", CONTROL, duplicate.as_bytes(), ""))).0,
        400
    );
    assert_eq!(settings(), accepted);
    assert!(listener.accept().is_err(), "settings must not connect");
    api.configure(&endpoint);
    assert_eq!(settings()["total_timeout_ms"], 120_000);
    assert_eq!(settings()["idle_timeout_ms"], 30_000);
    assert_eq!(api.call("DELETE", "/v1/settings", CONTROL, None).0, 200);
    assert_eq!(settings()["total_timeout_ms"], Value::Null);
    assert_eq!(settings()["idle_timeout_ms"], Value::Null);
}

#[test]
fn configured_timeouts_bind_the_exact_coding_exchange_and_cannot_be_model_arguments() {
    for custom in [false, true] {
        let api = Api::new();
        let (endpoint, task) = upstream(|mut s| {
            let raw = incoming(&mut s);
            let start = raw.windows(4).position(|v| v == b"\r\n\r\n").unwrap() + 4;
            let request: Value = serde_json::from_slice(&raw[start..]).unwrap();
            assert!(request.get("total_timeout_ms").is_none());
            assert!(request.get("idle_timeout_ms").is_none());
            reply(
                &mut s,
                "text/event-stream",
                &(chunk(&coding_answer()) + &end()),
            );
        });
        let mut choice = json!({"endpoint":endpoint,"approve_conversations":true});
        if custom {
            choice["total_timeout_ms"] = json!(300_000);
            choice["idle_timeout_ms"] = json!(60_000);
        }
        assert_eq!(
            api.call("PUT", "/v1/settings", CONTROL, Some(choice)).0,
            200
        );
        for field in ["total_timeout_ms", "idle_timeout_ms"] {
            let mut input = coding_request();
            input[field] = json!(300_000);
            assert_eq!(
                api.call("POST", "/v1/code/proposals", CLIENT, Some(input))
                    .0,
                400
            );
        }
        let (status, body) = api.call("POST", "/v1/code/proposals", CLIENT, Some(coding_request()));
        assert_eq!(status, 200);
        let rows = events(&body);
        let last = rows.last().unwrap();
        assert_eq!(last["type"], "proposal");
        assert_eq!(last["tested"], false);
        assert_eq!(last["applied"], false);
        assert_eq!(
            rows[0]["model_binding"]["total_timeout"],
            json!({"secs":if custom {300} else {120},"nanos":0})
        );
        assert_eq!(
            rows[0]["model_binding"]["idle_timeout"],
            json!({"secs":if custom {60} else {30},"nanos":0})
        );
        assert_endpoint_runtime_observation(&rows);
        task.join().unwrap();
    }
}

#[test]
fn configured_idle_and_total_timeouts_stop_chat_without_promoting_partial_text() {
    for idle in [false, true] {
        let api = Api::new();
        let (endpoint, task) = upstream(move |mut s| {
            incoming(&mut s);
            s.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n",
            )
            .unwrap();
            s.write_all(chunk("provisional").as_bytes()).unwrap();
            if idle {
                thread::sleep(Duration::from_secs(1));
            } else {
                // Progress must not renew the total deadline.
                for _ in 0..100 {
                    if s.write_all(b": progress\n\n").is_err() {
                        return;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
            }
            let _ = s.write_all(end().as_bytes());
        });
        assert_eq!(
            api.call(
                "PUT",
                "/v1/settings",
                CONTROL,
                Some(json!({
                    "endpoint":endpoint,"approve_conversations":true,
                    "total_timeout_ms":if idle {3000} else {250},
                    "idle_timeout_ms":if idle {100} else {250}
                }))
            )
            .0,
            200
        );
        let (status, body) = api.call(
            "POST",
            "/v1/chat",
            CLIENT,
            Some(json!({
                "model":"fixture","messages":[{"role":"user","content":"Hello"}],"max_tokens":100
            })),
        );
        assert_eq!(status, 200);
        let rows = events(&body);
        assert!(
            rows.iter()
                .any(|r| r["type"] == "text" && r["text"] == "provisional")
        );
        assert_eq!(rows.last().unwrap()["type"], "error");
        assert_eq!(rows.last().unwrap()["code"], "chat.transport.timeout");
        assert_eq!(rows.last().unwrap()["partial"], true);
        assert!(!rows.iter().any(|r| r["type"] == "complete"));
        task.join().unwrap();
    }
}

#[test]
fn strict_multifile_proposals_stream_final_batch_bounds_through_the_configured_endpoint() {
    fn padded(first: &str) -> String {
        let mut s = format!("{first}\n");
        let left = 64 * 1024 - s.len();
        s.push_str(&"p\n".repeat(left / 2));
        if left % 2 == 1 {
            s.push('p');
        }
        assert_eq!(s.len(), 64 * 1024);
        s
    }
    for (grow, shrink) in [("a.txt", "z.txt"), ("z.txt", "a.txt")] {
        for overflow in [false, true] {
            let api = Api::new();
            let original_grow = padded("alpha");
            let original_shrink = padded("trimβ");
            let replacement = if overflow { "alpha é!!" } else { "alpha é!" };
            let answer = json!({"edits":[{"path":grow,"old":"alpha","new":replacement}, {"path":shrink,"old":"trimβ","new":"β"}]}).to_string();
            let answer_copy = answer.clone();
            let (endpoint, worker) = upstream(move |mut socket| {
                let raw = incoming(&mut socket);
                let start = raw.windows(4).position(|v| v == b"\r\n\r\n").unwrap() + 4;
                let sent: Value = serde_json::from_slice(&raw[start..]).unwrap();
                let context: Value =
                    serde_json::from_str(sent["messages"][1]["content"].as_str().unwrap()).unwrap();
                assert_eq!(context["files"].as_array().unwrap().len(), 2);
                assert_eq!(
                    context["files"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|f| f["bytes"].as_u64().unwrap())
                        .sum::<u64>(),
                    128 * 1024
                );
                for (path, needle) in [(grow, "alpha"), (shrink, "trimβ")] {
                    assert!(
                        context["excerpts"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|e| e["path"] == path
                                && e["text"].as_str().unwrap().contains(needle))
                    );
                }
                let midpoint = answer_copy.find("trimβ").unwrap();
                reply(
                    &mut socket,
                    "text/event-stream",
                    &format!(
                        "{}{}{}",
                        chunk(&answer_copy[..midpoint]),
                        chunk(&answer_copy[midpoint..]),
                        end()
                    ),
                );
            });
            api.configure(&endpoint);
            let mut request = coding_request();
            request["task"] = json!("Update alpha and trim together");
            request["files"] = json!([{"path":grow,"content":original_grow},{"path":shrink,"content":original_shrink}]);
            assert!(serde_json::to_vec(&request).unwrap().len() < 256 * 1024);
            let (status, body) = api.call("POST", "/v1/code/proposals", CLIENT, Some(request));
            assert_eq!(status, 200);
            let observed = events(&body);
            worker.join().unwrap();
            assert_eq!(observed.first().unwrap()["type"], "context");
            assert_eq!(observed[0]["authority"], "proposal-only");
            assert!(observed[0]["message_bytes"].as_u64().unwrap() <= 4096);
            assert_endpoint_runtime_observation(&observed);
            let terminal = observed.last().unwrap();
            assert_eq!(terminal["response_complete"], true);
            let streamed: String = observed
                .iter()
                .filter(|e| e["type"] == "text")
                .map(|e| e["text"].as_str().unwrap())
                .collect();
            assert_eq!(streamed, answer);
            if overflow {
                assert_eq!(terminal["type"], "error");
                assert_eq!(terminal["code"], "code.proposal.edit");
                assert!(observed.iter().all(|e| e.get("changes").is_none()));
            } else {
                assert_eq!(terminal["type"], "proposal");
                assert_eq!(terminal["applied"], false);
                assert_eq!(terminal["tested"], false);
                assert_eq!(terminal["authority"], "proposal-only");
                let changes = terminal["changes"].as_array().unwrap();
                assert_eq!(changes.len(), 2);
                assert!(
                    changes[0]["path"].as_str().unwrap() < changes[1]["path"].as_str().unwrap()
                );
                assert_eq!(
                    changes
                        .iter()
                        .map(|c| c["content"].as_str().unwrap().len())
                        .sum::<usize>(),
                    128 * 1024
                );
                for change in changes {
                    let path = change["path"].as_str().unwrap();
                    let (original, old, new) = if path == grow {
                        (&original_grow, "alpha", replacement)
                    } else {
                        assert_eq!(path, shrink);
                        (&original_shrink, "trimβ", "β")
                    };
                    let expected = original.replacen(old, new, 1);
                    assert_eq!(change["content"], expected);
                    assert_eq!(
                        change["before_sha256"],
                        Sha256::digest(original.as_bytes())
                            .iter()
                            .map(|byte| format!("{byte:02x}"))
                            .collect::<String>()
                    );
                    assert_eq!(
                        change["after_sha256"],
                        Sha256::digest(expected.as_bytes())
                            .iter()
                            .map(|byte| format!("{byte:02x}"))
                            .collect::<String>()
                    );
                }
            }
        }
    }
}
