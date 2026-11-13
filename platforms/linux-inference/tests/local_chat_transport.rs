// SPDX-License-Identifier: Apache-2.0
//! Actual loopback sockets, synthetic servers only; no model qualification.

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use agentmage_platform_linux_inference::local_chat::{
    ChatError, ChatFinish, ChatMessage, ChatResponseFormat, ChatResponseMode, ChatRole,
    LocalChatEndpoint,
};
use agentmage_platform_linux_inference::local_chat_transport::{
    BearerKey, ChatCancellation, LocalChatApproval, LocalChatTransport, LocalProbeApproval,
    TransportError, TransportLimits, probe_endpoints,
};

const LIST: &str = "{\"data\":[{\"id\":\"test-model\"},{\"id\":\"second-model\"}]}";

fn complete_body() -> serde_json::Value {
    serde_json::json!({"model":"test-model","choices":[{"index":0,
        "message":{"role":"assistant","content":"hello 🦀"},"finish_reason":"stop"}],
        "usage":{"prompt_tokens":10,"completion_tokens":3,"total_tokens":13}})
}

#[test]
fn complete_mode_freezes_actual_body_and_reports_only_validated_text_and_optional_usage() {
    use sha2::{Digest, Sha256};
    for (usage, format) in [false, true].into_iter().flat_map(|usage| {
        [ChatResponseFormat::Text, ChatResponseFormat::JsonObject]
            .into_iter()
            .map(move |format| (usage, format))
    }) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint =
            LocalChatEndpoint::parse(&format!("http://{}/v1", listener.local_addr().unwrap()))
                .unwrap();
        let (client, approval) = connection(endpoint, limits());
        assert_eq!(client.response_mode(), ChatResponseMode::Stream);
        assert_eq!(client.response_format(), ChatResponseFormat::Text);
        let client = client
            .with_response_mode(ChatResponseMode::Complete)
            .with_response_format(format);
        assert_eq!(client.response_format(), format);
        let (tx, rx) = mpsc::channel();
        let task = thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let request = read_request(&mut socket);
            let start = request.windows(4).position(|v| v == b"\r\n\r\n").unwrap() + 4;
            assert!(
                std::str::from_utf8(&request[..start])
                    .unwrap()
                    .contains("Accept: application/json\r\n")
            );
            let body: serde_json::Value = serde_json::from_slice(&request[start..]).unwrap();
            assert_eq!(body["stream"], false);
            match format {
                ChatResponseFormat::Text => assert!(body.get("response_format").is_none()),
                ChatResponseFormat::JsonObject => assert_eq!(
                    body["response_format"],
                    serde_json::json!({"type":"json_object"})
                ),
            }
            assert!(body.get("stream_options").is_none());
            assert!(body.get("tools").is_none());
            tx.send((
                request.len() - start,
                Sha256::digest(&request[start..])
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>(),
            ))
            .unwrap();
            let body = serde_json::to_vec(&complete_body()).unwrap();
            let reply = response("application/json", &body);
            // Fragmented UTF-8 and JSON must never leak partial parsed text.
            for fragment in reply.chunks(11) {
                socket.write_all(fragment).unwrap();
            }
        });
        let prepared = client
            .prepare_chat(
                &approval,
                "test-model",
                &[ChatMessage {
                    role: ChatRole::User,
                    content: "synthetic task",
                }],
                16,
                usage,
            )
            .unwrap();
        let binding = prepared.binding().clone();
        let mut text = String::new();
        let result = prepared
            .execute_recorded(&ChatCancellation::default(), |s| text.push_str(s))
            .unwrap();
        assert_eq!(text, "hello 🦀");
        assert_eq!(result.response_text_bytes, text.len());
        assert_eq!(result.token_accounting.provider_reported().is_some(), usage);
        let (body_bytes, body_sha256) = rx.recv().unwrap();
        assert_eq!(binding.body().request_body_bytes, body_bytes);
        assert_eq!(binding.body().request_body_sha256, body_sha256);
        task.join().unwrap();
    }
}

#[test]
fn complete_mode_refuses_bad_envelopes_tools_counts_and_framing_without_emitting_text() {
    let base = complete_body();
    let mut cases = vec![b"{".to_vec(), b"data: [DONE]\n\n".to_vec()];
    for (pointer, value) in [
        ("/model", serde_json::json!("another-model")),
        ("/choices", serde_json::json!([])),
        (
            "/choices",
            serde_json::json!([base["choices"][0], base["choices"][0]]),
        ),
        ("/choices/0/index", serde_json::json!(1)),
        ("/choices/0/message/role", serde_json::json!("user")),
        ("/choices/0/message/content", serde_json::Value::Null),
        ("/choices/0/finish_reason", serde_json::json!("tool_calls")),
        ("/usage/total_tokens", serde_json::json!(12)),
        ("/usage/completion_tokens", serde_json::json!(17)),
    ] {
        let mut bad = base.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        cases.push(serde_json::to_vec(&bad).unwrap());
    }
    for field in ["tool_calls", "function_call", "refusal"] {
        let mut bad = base.clone();
        bad["choices"][0]["message"][field] = serde_json::json!([]);
        cases.push(serde_json::to_vec(&bad).unwrap());
    }
    cases.push(format!("{{\"model\":\"test-model\",{}", &base.to_string()[1..]).into_bytes());
    cases.push(format!("{} {{}}", base).into_bytes());
    for body in cases {
        let (endpoint, task) = server(move |mut socket| {
            socket
                .write_all(&response("application/json", &body))
                .unwrap();
        });
        let (client, approval) = connection(endpoint, limits());
        let client = client.with_response_mode(ChatResponseMode::Complete);
        let mut emitted = false;
        assert!(
            client
                .chat_with_usage(
                    &approval,
                    "test-model",
                    &[ChatMessage {
                        role: ChatRole::User,
                        content: "synthetic task",
                    }],
                    16,
                    &ChatCancellation::default(),
                    |_| emitted = true
                )
                .is_err()
        );
        assert!(!emitted);
        task.join().unwrap();
    }
}

#[test]
fn complete_mode_empty_answers_match_stream_refusal_without_losing_nonempty_token_limits() {
    use agentmage_kernel_contracts::EndpointModelFinish;

    for mode in [ChatResponseMode::Stream, ChatResponseMode::Complete] {
        for finish in ["stop", "length"] {
            for content in ["", "answer"] {
                for usage in [false, true] {
                    let (endpoint, task) = server(move |mut socket| {
                        let (media, body) = match mode {
                            ChatResponseMode::Stream => (
                                "text/event-stream",
                                [text_frame(content), ending(finish)].concat(),
                            ),
                            ChatResponseMode::Complete => {
                                let mut body = complete_body();
                                body["choices"][0]["message"]["content"] = content.into();
                                body["choices"][0]["message"]["reasoning_content"] =
                                    "untrusted reasoning is not an answer".into();
                                body["choices"][0]["finish_reason"] = finish.into();
                                ("application/json", serde_json::to_vec(&body).unwrap())
                            }
                        };
                        socket.write_all(&response(media, &body)).unwrap();
                    });
                    let (client, approval) = connection(endpoint, limits());
                    let client = client.with_response_mode(mode);
                    let prepared = client
                        .prepare_chat(
                            &approval,
                            "test-model",
                            &[ChatMessage {
                                role: ChatRole::User,
                                content: "synthetic task",
                            }],
                            16,
                            usage,
                        )
                        .unwrap();
                    let mut emitted = String::new();
                    let result = prepared.execute_recorded(&ChatCancellation::default(), |text| {
                        emitted.push_str(text);
                    });
                    if content.is_empty() {
                        assert_eq!(result, Err(TransportError::Chat(ChatError::Incomplete)));
                        assert!(emitted.is_empty());
                    } else {
                        let completed = result.unwrap();
                        assert_eq!(emitted, content);
                        assert_eq!(completed.response_text_bytes, content.len());
                        assert_eq!(
                            completed.finish,
                            if finish == "stop" {
                                EndpointModelFinish::Stop
                            } else {
                                EndpointModelFinish::OutputTokenLimit
                            }
                        );
                    }
                    task.join().unwrap();
                }
            }
        }
    }
}

#[test]
fn complete_mode_preserves_body_and_narrowed_text_ceilings_and_callback_cancellation() {
    for mode in ["body", "text", "narrow", "cancel", "truncated", "media"] {
        let (endpoint, task) = server(move |mut socket| {
            let mut value = complete_body();
            if mode == "text" {
                value["choices"][0]["message"]["content"] =
                    serde_json::json!("x".repeat(1024 * 1024 + 1));
            }
            let body = serde_json::to_vec(&value).unwrap();
            let mut reply = response(
                if mode == "media" {
                    "text/event-stream"
                } else {
                    "application/json"
                },
                &body,
            );
            if mode == "body" {
                reply = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 4194305\r\n\r\n".to_vec();
            } else if mode == "truncated" {
                reply.pop();
            }
            let _ = socket.write_all(&reply);
        });
        let (client, approval) = connection(endpoint, limits());
        let client = client.with_response_mode(ChatResponseMode::Complete);
        let mut prepared = client
            .prepare_chat(
                &approval,
                "test-model",
                &[ChatMessage {
                    role: ChatRole::User,
                    content: "synthetic task",
                }],
                16,
                false,
            )
            .unwrap();
        if mode == "narrow" {
            prepared = prepared.with_response_text_limit(3).unwrap();
        }
        let cancel = ChatCancellation::default();
        let mut emitted = false;
        let result = prepared.execute_recorded(&cancel, |_| {
            emitted = true;
            if mode == "cancel" {
                cancel.cancel();
            }
        });
        assert!(result.is_err(), "{mode}");
        assert_eq!(emitted, mode == "cancel", "{mode}");
        task.join().unwrap();
    }
}

#[test]
fn complete_mode_wait_is_bounded_by_idle_total_and_owner_stop_without_partial_text() {
    use agentmage_kernel_contracts::ModelOperationStop;
    for cause in ["idle", "total", "owner", "cancel"] {
        let cancel = ChatCancellation::default();
        let server_cancel = cancel.clone();
        let control = run_control(500);
        let stop = control.stop.clone();
        let (endpoint, task) = server(move |mut socket| {
            socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{").unwrap();
            if cause == "owner" {
                *stop.lock().unwrap() = Some(ModelOperationStop::TimedOut);
            }
            if cause == "cancel" {
                server_cancel.cancel();
            }
            thread::sleep(Duration::from_millis(100));
        });
        let bounds = TransportLimits {
            total: Duration::from_millis(if cause == "total" { 30 } else { 500 }),
            idle: Duration::from_millis(if cause == "idle" {
                20
            } else if cause == "total" {
                30
            } else {
                500
            }),
        };
        let (client, approval) = connection(endpoint, bounds);
        let client = client.with_response_mode(ChatResponseMode::Complete);
        let prepared = std::sync::Arc::new(client)
            .prepare_chat_shared(
                &approval,
                "test-model",
                &[ChatMessage {
                    role: ChatRole::User,
                    content: "synthetic task",
                }],
                16,
                false,
            )
            .unwrap();
        let mut emitted = false;
        let result = prepared.execute_recorded_controlled(&cancel, &control, |_| emitted = true);
        let expected = match cause {
            "owner" => TransportError::OperationStopped(ModelOperationStop::TimedOut),
            "cancel" => TransportError::Cancelled,
            _ => TransportError::Timeout,
        };
        assert_eq!(result.unwrap_err(), expected, "{cause}");
        assert!(!emitted, "{cause}");
        task.join().unwrap();
    }
}

fn limits() -> TransportLimits {
    TransportLimits {
        total: Duration::from_secs(3),
        idle: Duration::from_secs(1),
    }
}

fn read_request(stream: &mut TcpStream) -> Vec<u8> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut data = Vec::new();
    let mut buffer = [0; 1024];
    loop {
        let n = stream.read(&mut buffer).unwrap();
        assert_ne!(n, 0, "client closed before sending request");
        data.extend_from_slice(&buffer[..n]);
        if let Some(end) = data.windows(4).position(|v| v == b"\r\n\r\n") {
            let header = std::str::from_utf8(&data[..end]).unwrap();
            let length: usize = header
                .lines()
                .find_map(|line| line.strip_prefix("Content-Length: "))
                .unwrap()
                .parse()
                .unwrap();
            if data.len() == end + 4 + length {
                return data;
            }
        }
        assert!(data.len() < 300 * 1024);
    }
}

fn server(handler: impl FnOnce(TcpStream) + Send + 'static) -> (LocalChatEndpoint, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint =
        LocalChatEndpoint::parse(&format!("http://{}/api/v1", listener.local_addr().unwrap()))
            .unwrap();
    let task = thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let start = Instant::now();
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        start.elapsed() < Duration::from_secs(4),
                        "no client arrived"
                    );
                    thread::sleep(Duration::from_millis(2));
                }
                Err(e) => panic!("{e}"),
            }
        };
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let request = read_request(&mut stream);
        assert!(!String::from_utf8_lossy(&request).contains("Authorization:"));
        assert!(!String::from_utf8_lossy(&request).contains("Cookie:"));
        handler(stream);
    });
    (endpoint, task)
}

fn connection(
    endpoint: LocalChatEndpoint,
    bounds: TransportLimits,
) -> (LocalChatTransport, LocalChatApproval) {
    let approval = LocalChatApproval::for_endpoint(endpoint.clone());
    (
        LocalChatTransport::new(endpoint, None, bounds).unwrap(),
        approval,
    )
}

fn response(media: &str, body: &[u8]) -> Vec<u8> {
    [
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: {media}\r\nContent-Length: {}\r\n\r\n",
            body.len()
        )
        .into_bytes(),
        body.to_vec(),
    ]
    .concat()
}

fn text_frame(text: &str) -> Vec<u8> {
    format!("data: {}\n\n", serde_json::json!({"model":"test-model","choices":[{"index":0,"delta":{"content":text},"finish_reason":null}]})).into_bytes()
}

fn ending(reason: &str) -> Vec<u8> {
    format!("data: {}\n\ndata: [DONE]\n\n", serde_json::json!({"model":"test-model","choices":[{"index":0,"delta":{},"finish_reason":reason}]})).into_bytes()
}

fn chat(
    connection: &LocalChatTransport,
    approval: &LocalChatApproval,
    cancel: &ChatCancellation,
    consume: impl FnMut(&str),
) -> Result<ChatFinish, TransportError> {
    connection.chat(
        approval,
        "test-model",
        &[ChatMessage {
            role: ChatRole::User,
            content: "synthetic task",
        }],
        16,
        cancel,
        consume,
    )
}

#[test]
fn credential_is_owned_and_only_sent_to_approved_address_and_path() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint =
        LocalChatEndpoint::parse(&format!("http://{}/api/v1", listener.local_addr().unwrap()))
            .unwrap();
    let address = endpoint.address();
    let task = thread::spawn(move || {
        for method in ["GET /api/v1/models", "POST /api/v1/chat/completions"] {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            let request = std::str::from_utf8(&request).unwrap();
            assert!(request.starts_with(method));
            assert!(request.contains(&format!("Host: {address}\r\n")));
            assert!(request.contains("Authorization: Bearer synthetic-key\r\n"));
            assert!(request.contains("Connection: close\r\n"));
            if method.starts_with("GET") {
                stream
                    .write_all(&response(
                        "application/json; charset=utf-8",
                        LIST.as_bytes(),
                    ))
                    .unwrap();
            } else {
                let (_, body) = request.split_once("\r\n\r\n").unwrap();
                let body: serde_json::Value = serde_json::from_str(body).unwrap();
                assert_eq!(body["messages"][0]["content"], "synthetic task");
                assert_eq!(body["stream"], true);
                assert!(body.get("tools").is_none());
                stream
                    .write_all(&response(
                        "text/event-stream",
                        &[text_frame("hello"), ending("length")].concat(),
                    ))
                    .unwrap();
            }
        }
    });
    let approval = LocalChatApproval::for_endpoint(endpoint.clone());
    let client = LocalChatTransport::new(
        endpoint,
        Some(BearerKey::new("synthetic-key".to_owned()).unwrap()),
        limits(),
    )
    .unwrap();
    let cancel = ChatCancellation::default();
    assert_eq!(
        client.models(&approval, &cancel).unwrap(),
        ["test-model", "second-model"]
    );
    let mut output = String::new();
    assert_eq!(
        chat(&client, &approval, &cancel, |s| output.push_str(s)),
        Ok(ChatFinish::Length)
    );
    assert_eq!(output, "hello");
    task.join().unwrap();
}

#[test]
fn approval_and_pre_cancel_refuse_before_any_connection_or_credential_use() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint =
        LocalChatEndpoint::parse(&format!("http://{}/v1", listener.local_addr().unwrap())).unwrap();
    let (client, approval) = connection(endpoint, limits());
    let wrong_path = LocalChatApproval::for_endpoint(
        LocalChatEndpoint::parse(&format!("http://{}/other", listener.local_addr().unwrap()))
            .unwrap(),
    );
    let wrong_address =
        LocalChatApproval::for_endpoint(LocalChatEndpoint::parse("http://127.0.0.2:1/v1").unwrap());
    for wrong in [&wrong_path, &wrong_address] {
        assert_eq!(
            client.models(wrong, &ChatCancellation::default()),
            Err(TransportError::Approval)
        );
        assert_eq!(
            chat(&client, wrong, &ChatCancellation::default(), |_| panic!()),
            Err(TransportError::Approval)
        );
    }
    let cancel = ChatCancellation::default();
    cancel.cancel();
    assert_eq!(
        client.models(&approval, &cancel),
        Err(TransportError::Cancelled)
    );
    assert_eq!(
        chat(&client, &approval, &cancel, |_| panic!()),
        Err(TransportError::Cancelled)
    );
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn invalid_credentials_and_limits_fail_without_io() {
    for key in [
        "",
        "secret\r\nInjected: yes",
        "secret key",
        "secret\0",
        "🔑",
    ] {
        assert!(matches!(
            BearerKey::new(key.to_owned()),
            Err(TransportError::Configuration)
        ));
    }
    assert!(BearerKey::new("k".repeat(4097)).is_err());
    let endpoint = LocalChatEndpoint::parse("http://127.0.0.1:1").unwrap();
    for bounds in [
        TransportLimits {
            total: Duration::ZERO,
            idle: Duration::ZERO,
        },
        TransportLimits {
            total: Duration::from_secs(301),
            idle: Duration::from_secs(1),
        },
        TransportLimits {
            total: Duration::from_secs(1),
            idle: Duration::from_secs(2),
        },
    ] {
        assert!(matches!(
            LocalChatTransport::new(endpoint.clone(), None, bounds),
            Err(TransportError::Configuration)
        ));
    }
}

#[test]
fn streaming_text_is_delivered_before_server_finishes() {
    let (send, receive) = mpsc::channel();
    let (endpoint, task) = server(move |mut stream| {
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n").unwrap();
        let first = text_frame("café 🐈");
        // Every byte is its own HTTP chunk, including split UTF-8 and SSE fields.
        for b in first {
            stream
                .write_all(&[b'1', b'\r', b'\n', b, b'\r', b'\n'])
                .unwrap();
        }
        receive.recv_timeout(Duration::from_secs(2)).unwrap();
        let tail = ending("stop");
        stream
            .write_all(format!("{:x}\r\n", tail.len()).as_bytes())
            .unwrap();
        stream.write_all(&tail).unwrap();
        stream.write_all(b"\r\n0\r\n\r\n").unwrap();
    });
    let (client, approval) = connection(endpoint, limits());
    assert_eq!(
        chat(&client, &approval, &ChatCancellation::default(), |text| {
            assert_eq!(text, "café 🐈");
            send.send(()).unwrap();
        }),
        Ok(ChatFinish::Stop)
    );
    task.join().unwrap();
}

#[test]
fn response_statuses_are_content_free_and_redirect_is_not_followed() {
    let target = TcpListener::bind("127.0.0.1:0").unwrap();
    target.set_nonblocking(true).unwrap();
    for (status, error) in [
        ("401 Unauthorized", TransportError::Authentication),
        ("403 Forbidden", TransportError::Authentication),
        ("302 Found", TransportError::Redirect),
        ("500 Broken", TransportError::Status),
        ("100 Continue", TransportError::Status),
    ] {
        let wire = format!(
            "HTTP/1.1 {status}\r\nLocation: http://{}/private\r\n\r\nprivate-upstream-secret",
            target.local_addr().unwrap()
        );
        let (endpoint, task) = server(move |mut stream| {
            let _ = stream.write_all(wire.as_bytes());
        });
        let (client, approval) = connection(endpoint, limits());
        let observed = client
            .models(&approval, &ChatCancellation::default())
            .unwrap_err();
        assert_eq!(observed, error);
        assert!(!format!("{observed:?} {observed} {}", observed.message()).contains("private"));
        task.join().unwrap();
    }
    assert_eq!(
        target.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn invalid_http_headers_and_framing_are_refused() {
    let cases = [
        "HTTP/1.1 200 OK\nContent-Type: application/json\n\n{}",
        "HTTP/1.1 200 OK\r\nContent-Length : 2\r\nContent-Type: application/json\r\n\r\n{}",
        "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nContent-Length: 2\r\nContent-Type: application/json\r\n\r\n{}",
        "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nTransfer-Encoding: chunked\r\nContent-Type: application/json\r\n\r\n{}",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Type: application/json\r\n\r\n{}",
        "HTTP/1.1 200 OK\r\nContent-Encoding: gzip\r\nContent-Type: application/json\r\n\r\n{}",
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n{}",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=latin-1\r\n\r\n{}",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json; boundary=x\r\n\r\n{}",
        "HTTP/1.1 200 OK\r\n X-Fold: x\r\nContent-Type: application/json\r\n\r\n{}",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: gzip, chunked\r\n\r\n{}",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: +2\r\n\r\n{}",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 30\r\n\r\n{}",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}EXTRA",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n2\r\n{}\r\n0\r\nX: y\r\n\r\n",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n2;ext=yes\r\n{}\r\n0\r\n\r\n",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n2\r\n{}!\n0\r\n\r\n",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\nzz\r\n",
    ];
    for wire in cases {
        let (endpoint, task) = server(move |mut stream| {
            let _ = stream.write_all(wire.as_bytes());
        });
        let (client, approval) = connection(endpoint, limits());
        assert_eq!(
            client.models(&approval, &ChatCancellation::default()),
            Err(TransportError::Http),
            "case: {wire}"
        );
        task.join().unwrap();
    }
}

#[test]
fn model_list_requires_bounded_unique_valid_ids() {
    for body in [
        "{}",
        "{\"data\":null}",
        "{\"data\":[{}]}",
        "{\"data\":[{\"id\":\"\"}]}",
        "{\"data\":[{\"id\":\"a\\n\"}]}",
        "{\"data\":[{\"id\":\"a\"},{\"id\":\"a\"}]}",
    ] {
        let (endpoint, task) = server(move |mut stream| {
            stream
                .write_all(&response("application/json", body.as_bytes()))
                .unwrap();
        });
        let (client, approval) = connection(endpoint, limits());
        assert_eq!(
            client.models(&approval, &ChatCancellation::default()),
            Err(TransportError::Http)
        );
        task.join().unwrap();
    }
    for (body, error) in [
        (
            serde_json::json!({"data":vec![serde_json::json!({"id":"a"});257]}).to_string(),
            TransportError::Limit,
        ),
        (
            serde_json::json!({"data":[{"id":"a".repeat(257)}]}).to_string(),
            TransportError::Http,
        ),
    ] {
        let (endpoint, task) = server(move |mut stream| {
            stream
                .write_all(&response("application/json", body.as_bytes()))
                .unwrap();
        });
        let (client, approval) = connection(endpoint, limits());
        assert_eq!(
            client.models(&approval, &ChatCancellation::default()),
            Err(error)
        );
        task.join().unwrap();
    }
}

#[test]
fn connection_probe_retains_failures_and_never_sends_credentials() {
    let (endpoint, task) = server(|mut stream| {
        stream
            .write_all(&response("application/json", LIST.as_bytes()))
            .unwrap();
    });
    let unused = TcpListener::bind("127.0.0.1:0").unwrap();
    let missing =
        LocalChatEndpoint::parse(&format!("http://{}", unused.local_addr().unwrap())).unwrap();
    drop(unused);
    let probes = probe_endpoints(
        &LocalProbeApproval::for_endpoints(vec![missing.clone(), endpoint.clone()]).unwrap(),
        limits(),
        &ChatCancellation::default(),
    )
    .unwrap();
    assert_eq!(probes.len(), 2);
    assert_eq!(probes[0].endpoint, missing);
    assert_eq!(probes[0].models, Err(TransportError::Connection));
    assert_eq!(probes[1].endpoint, endpoint);
    assert_eq!(
        probes[1].models.as_ref().unwrap(),
        &["test-model", "second-model"]
    );
    assert!(matches!(
        LocalProbeApproval::for_endpoints(vec![]),
        Err(TransportError::Configuration)
    ));
    assert!(matches!(
        LocalProbeApproval::for_endpoints(vec![probes[1].endpoint.clone(); 9]),
        Err(TransportError::Configuration)
    ));
    task.join().unwrap();
}

#[test]
fn late_bytes_or_truncated_body_revoke_provisional_chat_completion() {
    let valid = [text_frame("partial"), ending("stop")].concat();
    let malformed = [valid.clone(), b"\n".to_vec()].concat();
    for (wire, expected) in [
        (response("text/event-stream", &malformed), TransportError::Chat(ChatError::Protocol)),
        ([response("text/event-stream", &valid), b"trailing-wire-bytes".to_vec()].concat(), TransportError::Http),
        (response("text/event-stream", &text_frame("partial")), TransportError::Chat(ChatError::Incomplete)),
        ([format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n", valid.len()+1).into_bytes(), valid].concat(), TransportError::Http),
    ] {
        let (endpoint, task) = server(move |mut stream| { stream.write_all(&wire).unwrap(); });
        let (client, approval) = connection(endpoint, limits());
        let mut text = String::new();
        assert_eq!(chat(&client, &approval, &ChatCancellation::default(), |s| text.push_str(s)), Err(expected));
        // A decoder may reject an entire bad input batch before emitting its text.
        assert!(text.is_empty() || text == "partial");
        task.join().unwrap();
    }
}

#[test]
fn all_response_sizes_are_bounded_before_allocating_declared_body() {
    let cases = [
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 262145\r\n\r\n".to_owned(),
        format!("HTTP/1.1 200 OK\r\nX-Large: {}\r\nContent-Type: application/json\r\n\r\n", "x".repeat(16384)),
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\n\r\n40001\r\n".to_owned(),
        format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{}", "x".repeat(262145)),
    ];
    for wire in cases {
        let (endpoint, task) = server(move |mut stream| {
            let _ = stream.write_all(wire.as_bytes());
        });
        let (client, approval) = connection(endpoint, limits());
        assert_eq!(
            client.models(&approval, &ChatCancellation::default()),
            Err(TransportError::Limit)
        );
        task.join().unwrap();
    }
}

#[test]
fn inactivity_total_time_and_cancellation_stop_stalled_io_and_close_sockets() {
    for cancel_requested in [false, true] {
        let (observed_tx, observed_rx) = mpsc::channel();
        let (endpoint, task) = server(move |mut stream| {
            let mut b = [0; 1];
            // On cancellation/timeout the transport drops its own socket.
            observed_tx.send(stream.read(&mut b).unwrap()).unwrap();
        });
        let bounds = TransportLimits {
            total: Duration::from_secs(2),
            idle: Duration::from_millis(150),
        };
        let (client, approval) = connection(endpoint, bounds);
        let cancel = ChatCancellation::default();
        let other = cancel.clone();
        let trigger = thread::spawn(move || {
            if cancel_requested {
                thread::sleep(Duration::from_millis(40));
                other.cancel();
            }
        });
        let start = Instant::now();
        assert_eq!(
            client.models(&approval, &cancel),
            Err(if cancel_requested {
                TransportError::Cancelled
            } else {
                TransportError::Timeout
            })
        );
        assert!(start.elapsed() < Duration::from_secs(1));
        assert_eq!(observed_rx.recv_timeout(Duration::from_secs(1)).unwrap(), 0);
        task.join().unwrap();
        trigger.join().unwrap();
    }
    let (endpoint, task) = server(|mut stream| {
        for _ in 0..30 {
            if stream.write_all(b"H").is_err() {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
    });
    let (client, approval) = connection(
        endpoint,
        TransportLimits {
            total: Duration::from_millis(200),
            idle: Duration::from_millis(100),
        },
    );
    let start = Instant::now();
    assert_eq!(
        client.models(&approval, &ChatCancellation::default()),
        Err(TransportError::Timeout)
    );
    assert!(start.elapsed() < Duration::from_millis(600));
    task.join().unwrap();
}

#[test]
fn cancellation_from_stream_callback_never_reports_completion() {
    let (endpoint, task) = server(|mut stream| {
        let _ = stream.write_all(&response(
            "text/event-stream",
            &[text_frame("first"), ending("stop")].concat(),
        ));
    });
    let (client, approval) = connection(endpoint, limits());
    let cancel = ChatCancellation::default();
    assert_eq!(
        chat(&client, &approval, &cancel, |_| cancel.cancel()),
        Err(TransportError::Cancelled)
    );
    task.join().unwrap();
}

#[test]
fn logical_finish_does_not_succeed_while_http_connection_remains_open() {
    let (endpoint, task) = server(|mut stream| {
        stream
            .write_all(&response(
                "text/event-stream",
                &[text_frame("first"), ending("stop")].concat(),
            ))
            .unwrap();
        let mut b = [0; 1];
        assert_eq!(stream.read(&mut b).unwrap(), 0);
        let _ = stream.shutdown(Shutdown::Both);
    });
    let (client, approval) = connection(
        endpoint,
        TransportLimits {
            total: Duration::from_secs(1),
            idle: Duration::from_millis(100),
        },
    );
    assert_eq!(
        chat(&client, &approval, &ChatCancellation::default(), |_| {}),
        Err(TransportError::Timeout)
    );
    task.join().unwrap();
}

#[test]
fn ambient_proxy_configuration_is_ignored() {
    if std::env::var_os("AGENTMAGE_PROXY_TEST_CHILD").is_none() {
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "ambient_proxy_configuration_is_ignored"])
            .env("AGENTMAGE_PROXY_TEST_CHILD", "1")
            .env("HTTP_PROXY", "http://127.0.0.1:1")
            .env("http_proxy", "http://127.0.0.1:1")
            .env("HTTPS_PROXY", "http://127.0.0.1:1")
            .env("ALL_PROXY", "http://127.0.0.1:1")
            .env_remove("NO_PROXY")
            .env_remove("no_proxy")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stdout)
        );
        return;
    }
    let (endpoint, task) = server(|mut stream| {
        stream
            .write_all(&response("application/json", LIST.as_bytes()))
            .unwrap();
    });
    let (client, approval) = connection(endpoint, limits());
    assert_eq!(
        client
            .models(&approval, &ChatCancellation::default())
            .unwrap(),
        ["test-model", "second-model"]
    );
    task.join().unwrap();
}

#[test]
fn ipv6_loopback_and_close_delimited_model_listing_work() {
    let listener = TcpListener::bind("[::1]:0").unwrap();
    let endpoint =
        LocalChatEndpoint::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let task = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let request = read_request(&mut stream);
        assert!(String::from_utf8_lossy(&request).contains("Host: [::1]:"));
        stream
            .write_all(b"HTTP/1.0 200 OK\r\nContent-Type: application/json\r\n\r\n{\"data\":[]}")
            .unwrap();
    });
    let (client, approval) = connection(endpoint, limits());
    assert!(
        client
            .models(&approval, &ChatCancellation::default())
            .unwrap()
            .is_empty()
    );
    task.join().unwrap();
}

#[test]
fn chunk_framing_overhead_counts_toward_wire_budget() {
    let (endpoint, task) = server(|mut stream| {
        let mut wire = b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n".to_vec();
        // SSE comments, less than 1 MiB decoded but over 5 MiB on the wire.
        // Empty SSE events reset the codec's per-event bound without text.
        for _ in 0..300_000 {
            wire.extend_from_slice(b"1\r\n:\r\n1\r\n\n\r\n1\r\n\n\r\n");
        }
        let _ = stream.write_all(&wire);
    });
    let (client, approval) = connection(
        endpoint,
        TransportLimits {
            total: Duration::from_secs(15),
            idle: Duration::from_secs(3),
        },
    );
    assert_eq!(
        chat(
            &client,
            &approval,
            &ChatCancellation::default(),
            |_| panic!("comments cannot be text")
        ),
        Err(TransportError::Limit)
    );
    task.join().unwrap();
}

#[test]
fn probe_candidates_share_one_total_deadline() {
    let (endpoint, task) = server(|mut stream| {
        let mut b = [0; 1];
        assert_eq!(stream.read(&mut b).unwrap(), 0);
    });
    let allowed = LocalProbeApproval::for_endpoints(vec![endpoint.clone(), endpoint]).unwrap();
    let start = Instant::now();
    let result = probe_endpoints(
        &allowed,
        TransportLimits {
            total: Duration::from_millis(150),
            idle: Duration::from_millis(150),
        },
        &ChatCancellation::default(),
    );
    assert!(matches!(result, Err(TransportError::Timeout)));
    assert!(start.elapsed() < Duration::from_secs(1));
    task.join().unwrap();
}

#[test]
fn usage_transport_keeps_exact_approval_and_reports_only_valid_eof() {
    for mode in ["report", "missing", "truncated", "status", "cancelled"] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let endpoint =
            LocalChatEndpoint::parse(&format!("http://{}/v1", listener.local_addr().unwrap()))
                .unwrap();
        let wrong = LocalChatApproval::for_endpoint(
            LocalChatEndpoint::parse(&format!("http://{}/other", listener.local_addr().unwrap()))
                .unwrap(),
        );
        let (transport, approval) = connection(endpoint, limits());
        let cancel = ChatCancellation::default();
        let messages = [ChatMessage {
            role: ChatRole::User,
            content: "synthetic task",
        }];
        assert_eq!(
            transport.chat_with_usage(&wrong, "test-model", &messages, 16, &cancel, |_| {}),
            Err(TransportError::Approval)
        );
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        let task = thread::spawn(move || {
            let start = Instant::now();
            let mut s = loop {
                match listener.accept() {
                    Ok((s, _)) => break s,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(start.elapsed() < Duration::from_secs(4));
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(e) => panic!("{e}"),
                }
            };
            let raw = read_request(&mut s);
            let offset = raw.windows(4).position(|v| v == b"\r\n\r\n").unwrap() + 4;
            let input: serde_json::Value = serde_json::from_slice(&raw[offset..]).unwrap();
            assert_eq!(
                input["stream_options"],
                serde_json::json!({"include_usage":true})
            );
            if mode == "status" {
                s.write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n")
                    .unwrap();
            } else {
                let finish = ending("stop");
                let cutoff = finish
                    .windows(b"data: [DONE]".len())
                    .position(|v| v == b"data: [DONE]")
                    .unwrap();
                let mut body = text_frame("answer");
                body.extend_from_slice(&finish[..cutoff]);
                if mode != "missing" {
                    body.extend_from_slice(
                        format!(
                            "data: {}\n\n",
                            serde_json::json!({"model":"test-model","choices":[],
                        "usage":{"prompt_tokens":12,"completion_tokens":4,"total_tokens":16}})
                        )
                        .as_bytes(),
                    );
                }
                if mode != "truncated" {
                    body.extend_from_slice(b"data: [DONE]\n\n");
                }
                // Ignore a broken pipe only in the explicitly cancelled case.
                let write = s.write_all(&response("text/event-stream", &body));
                if mode != "cancelled" {
                    write.unwrap();
                }
            }
            drop(s);
            assert_eq!(
                listener.accept().unwrap_err().kind(),
                std::io::ErrorKind::WouldBlock
            );
        });
        let mut text = String::new();
        let result = transport.chat_with_usage(
            &approval,
            "test-model",
            &messages,
            16,
            &cancel,
            |fragment| {
                text.push_str(fragment);
                if mode == "cancelled" {
                    cancel.cancel();
                }
            },
        );
        match mode {
            "report" | "missing" => {
                let completion = result.unwrap();
                assert_eq!(completion.finish, ChatFinish::Stop);
                assert_eq!(
                    completion.usage.map(|u| u.total_tokens),
                    if mode == "report" { Some(16) } else { None }
                );
                assert_eq!(text, "answer");
            }
            "status" => assert_eq!(result, Err(TransportError::Status)),
            "cancelled" => assert_eq!(result, Err(TransportError::Cancelled)),
            "truncated" => assert_eq!(result, Err(TransportError::Chat(ChatError::Incomplete))),
            _ => unreachable!(),
        }
        task.join().unwrap();
    }
}

#[test]
fn prepared_endpoint_request_sends_the_exact_frozen_body_and_redacts_private_inputs() {
    use serde_json::{Value, json};
    use sha2::{Digest, Sha256};
    for include_usage in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let endpoint = LocalChatEndpoint::parse(&format!(
            "http://localhost:{}/api/v1/",
            listener.local_addr().unwrap().port()
        ))
        .unwrap();
        let approval = LocalChatApproval::for_endpoint(endpoint.clone());
        let transport = LocalChatTransport::new(
            endpoint.clone(),
            Some(BearerKey::new("synthetic-key".into()).unwrap()),
            limits(),
        )
        .unwrap();
        let mut prompt = "private synthetic prompt: 界\0\n\"\\".repeat(512);
        let original = prompt.clone();
        let request = transport
            .prepare_chat(
                &approval,
                "test-model",
                &[ChatMessage {
                    role: ChatRole::User,
                    content: &prompt,
                }],
                16,
                include_usage,
            )
            .unwrap();
        let binding = serde_json::to_value(request.binding()).unwrap();
        let runtime_binding =
            serde_json::to_value(request.binding().runtime_model_binding()).unwrap();
        assert_eq!(
            runtime_binding,
            json!({
                "kind":"selected_endpoint",
                "identity":{"endpoint":endpoint.base_url(),"model":"test-model"}
            })
        );
        assert!(!runtime_binding.to_string().contains("synthetic-key"));
        assert!(
            !runtime_binding
                .to_string()
                .contains("private synthetic prompt")
        );
        prompt.clear(); // Execute cannot re-encode caller-owned, subsequently changed text.
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        let printed = binding.to_string();
        assert!(!printed.contains("synthetic-key"));
        assert!(!printed.contains("private synthetic prompt"));
        assert_eq!(
            binding["identity_kind"],
            "selected_endpoint_unverified_model"
        );
        assert_eq!(binding["verified_input_tokens"], Value::Null);
        assert_eq!(binding["endpoint"], endpoint.base_url());
        assert_eq!(binding["model"], "test-model");
        assert_eq!(binding["provider_usage_requested"], include_usage);
        assert_eq!(binding["requested_max_output_tokens"], 16);
        assert_eq!(binding["total_timeout"], json!({"secs":3,"nanos":0}));
        assert_eq!(binding["idle_timeout"], json!({"secs":1,"nanos":0}));
        assert_eq!(binding["response_wire_limit_bytes"], 5 * 1024 * 1024);
        assert_eq!(binding["response_body_limit_bytes"], 4 * 1024 * 1024);
        assert_eq!(binding["response_text_limit_bytes"], 1024 * 1024);
        let (tx, rx) = mpsc::channel();
        let task = thread::spawn(move || {
            let start = Instant::now();
            let mut socket = loop {
                match listener.accept() {
                    Ok((s, _)) => break s,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(start.elapsed() < Duration::from_secs(4));
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(e) => panic!("{e}"),
                }
            };
            socket
                .set_write_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let bytes = read_request(&mut socket);
            let offset = bytes.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
            let headers = std::str::from_utf8(&bytes[..offset]).unwrap();
            assert!(headers.starts_with("POST /api/v1/chat/completions HTTP/1.1"));
            assert!(headers.contains("Authorization: Bearer synthetic-key\r\n"));
            tx.send(bytes[offset..].to_vec()).unwrap();
            socket
                .write_all(&response(
                    "text/event-stream",
                    &[text_frame("answer"), ending("stop")].concat(),
                ))
                .unwrap();
            socket.shutdown(Shutdown::Write).unwrap();
            assert_eq!(
                listener.accept().unwrap_err().kind(),
                std::io::ErrorKind::WouldBlock
            );
        });
        let mut text = String::new();
        let execution_started = Instant::now();
        let completion = request
            .execute_recorded(&ChatCancellation::default(), |s| text.push_str(s))
            .unwrap();
        assert_eq!(
            completion.finish,
            agentmage_kernel_contracts::EndpointModelFinish::Stop
        );
        assert_eq!(completion.token_accounting.provider_reported(), None);
        assert!(completion.resources.dispatch_elapsed <= execution_started.elapsed());
        let observation = serde_json::to_value(&completion).unwrap();
        assert_eq!(observation["request"], binding);
        assert_eq!(observation["response_text_bytes"], 6);
        assert_eq!(
            observation["response_text_sha256"],
            Sha256::digest(b"answer")
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        assert_eq!(
            observation["resources"]["resident_memory_bytes"],
            Value::Null
        );
        assert_eq!(
            observation["resources"]["accelerator_memory_bytes"],
            Value::Null
        );
        assert_eq!(
            observation["token_accounting"]["verified_input_tokens"],
            Value::Null
        );
        assert_eq!(
            observation["token_accounting"]["verified_output_tokens"],
            Value::Null
        );
        assert!(!observation.to_string().contains("synthetic-key"));
        assert!(!observation.to_string().contains("private synthetic prompt"));
        assert_eq!(text, "answer");
        let body = rx.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_eq!(
            binding["request_body_sha256"],
            Sha256::digest(&body)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );
        assert_eq!(binding["request_body_bytes"], body.len());
        let message_array = format!(
            "[{{\"role\":\"user\",\"content\":{}}}]",
            serde_json::to_string(&original).unwrap()
        );
        assert_eq!(
            body.windows(message_array.len())
                .filter(|w| *w == message_array.as_bytes())
                .count(),
            1
        );
        assert_eq!(binding["context"]["message_count"], 1);
        assert_eq!(
            binding["context"]["serialized_messages_bytes"],
            message_array.len()
        );
        assert_eq!(
            binding["context"]["serialized_messages_sha256"],
            Sha256::digest(message_array.as_bytes())
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        let wire: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(wire["messages"][0]["content"], original);
        assert_eq!(wire["max_tokens"], 16);
        assert_eq!(wire.get("stream_options").is_some(), include_usage);
        task.join().unwrap();
    }
}

#[test]
fn recorded_endpoint_exchanges_keep_completion_usage_time_and_failures_distinct() {
    use agentmage_kernel_contracts::EndpointModelFinish;
    use serde_json::{Value, json};
    use sha2::{Digest, Sha256};

    for mode in [
        "report",
        "zero",
        "missing",
        "length",
        "bad-usage",
        "truncated",
        "late",
        "cancelled",
        "status",
    ] {
        let (endpoint, worker) = server(move |mut socket| {
            if mode == "status" {
                socket
                    .write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n")
                    .unwrap();
                return;
            }
            let mut body = [text_frame("界\n"), text_frame("\"\\answer")].concat();
            let finish = ending(if mode == "length" { "length" } else { "stop" });
            let cutoff = finish
                .windows(b"data: [DONE]".len())
                .position(|w| w == b"data: [DONE]")
                .unwrap();
            body.extend_from_slice(&finish[..cutoff]);
            if mode != "missing" {
                let (prompt, output, total) = match mode {
                    "zero" => (0, 0, 0),
                    "bad-usage" => (12, 4, 17),
                    _ => (12, 4, 16),
                };
                body.extend_from_slice(format!("data: {}\n\n", json!({"model":"test-model","choices":[],
                    "usage":{"prompt_tokens":prompt,"completion_tokens":output,"total_tokens":total}})).as_bytes());
            }
            if mode != "truncated" {
                body.extend_from_slice(b"data: [DONE]\n\n");
            }
            if mode == "late" {
                body.extend_from_slice(b"late");
            }
            let sent = socket.write_all(&response("text/event-stream", &body));
            if mode != "cancelled" {
                sent.unwrap();
            }
        });
        let (transport, approval) = connection(endpoint, limits());
        let request = transport
            .prepare_chat(
                &approval,
                "test-model",
                &[ChatMessage {
                    role: ChatRole::User,
                    content: "synthetic task",
                }],
                16,
                true,
            )
            .unwrap();
        let expected = request.binding().clone();
        let cancel = ChatCancellation::default();
        let mut text = String::new();
        let mut callback_time = Duration::ZERO;
        let started = Instant::now();
        let result = request.execute_recorded(&cancel, |fragment| {
            let entered = Instant::now();
            text.push_str(fragment);
            // Time includes the synchronous consumer, without claiming GPU time.
            thread::sleep(Duration::from_millis(2));
            if mode == "cancelled" {
                cancel.cancel();
            }
            callback_time += entered.elapsed();
        });
        match mode {
            "report" | "zero" | "missing" | "length" => {
                let result = result.unwrap();
                assert_eq!(result.request, expected);
                assert_eq!(
                    result.finish,
                    if mode == "length" {
                        EndpointModelFinish::OutputTokenLimit
                    } else {
                        EndpointModelFinish::Stop
                    }
                );
                assert_eq!(text, "界\n\"\\answer");
                assert_eq!(result.response_text_bytes, text.len());
                assert_eq!(
                    result.response_text_sha256,
                    Sha256::digest(text.as_bytes())
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>()
                );
                assert!(result.resources.dispatch_elapsed >= callback_time);
                assert!(result.resources.dispatch_elapsed <= started.elapsed());
                let value = serde_json::to_value(result).unwrap();
                assert_eq!(
                    value["token_accounting"]["provider_reported"],
                    match mode {
                        "missing" => Value::Null,
                        "zero" => json!({"prompt_tokens":0,"completion_tokens":0,"total_tokens":0}),
                        _ => json!({"prompt_tokens":12,"completion_tokens":4,"total_tokens":16}),
                    }
                );
                for field in ["verified_input_tokens", "verified_output_tokens"] {
                    assert_eq!(value["token_accounting"][field], Value::Null);
                }
                for field in ["resident_memory_bytes", "accelerator_memory_bytes"] {
                    assert_eq!(value["resources"][field], Value::Null);
                }
            }
            "status" => assert_eq!(result, Err(TransportError::Status)),
            "cancelled" => assert_eq!(result, Err(TransportError::Cancelled)),
            "truncated" => assert_eq!(result, Err(TransportError::Chat(ChatError::Incomplete))),
            "bad-usage" | "late" => {
                assert_eq!(result, Err(TransportError::Chat(ChatError::Protocol)))
            }
            _ => unreachable!(),
        }
        worker.join().unwrap();
    }
}

#[test]
fn prepared_endpoint_request_refuses_invalid_approval_and_cancellation_without_connecting() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint =
        LocalChatEndpoint::parse(&format!("http://{}/v1", listener.local_addr().unwrap())).unwrap();
    let (transport, approval) = connection(endpoint, limits());
    let wrong = LocalChatApproval::for_endpoint(
        LocalChatEndpoint::parse(&format!("http://{}/other", listener.local_addr().unwrap()))
            .unwrap(),
    );
    let messages = [ChatMessage {
        role: ChatRole::User,
        content: "synthetic",
    }];
    assert!(matches!(
        transport.prepare_chat(&wrong, "test-model", &messages, 16, false),
        Err(TransportError::Approval)
    ));
    for (model, tokens) in [("", 16), ("test-model", 0), ("test-model", 65537)] {
        assert!(matches!(
            transport.prepare_chat(&approval, model, &messages, tokens, true),
            Err(TransportError::Chat(ChatError::Request))
        ));
    }
    let oversized = "x".repeat(256 * 1024);
    assert!(matches!(
        transport.prepare_chat(
            &approval,
            "test-model",
            &[ChatMessage {
                role: ChatRole::User,
                content: &oversized,
            }],
            16,
            false
        ),
        Err(TransportError::Chat(ChatError::Request))
    ));
    let prepared = transport
        .prepare_chat(&approval, "test-model", &messages, 16, true)
        .unwrap();
    let cancel = ChatCancellation::default();
    cancel.cancel();
    assert_eq!(
        prepared.execute(&cancel, |_| panic!("no text allowed")),
        Err(TransportError::Cancelled)
    );
    drop(
        transport
            .prepare_chat(&approval, "test-model", &messages, 16, false)
            .unwrap(),
    );
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn endpoint_binding_separates_request_identity_from_credentials_and_unverified_server_identity() {
    use serde_json::{Value, json};
    let endpoint = LocalChatEndpoint::parse("http://127.0.0.1:1/v1").unwrap();
    let (transport, approval) = connection(endpoint.clone(), limits());
    let messages = [ChatMessage {
        role: ChatRole::User,
        content: "synthetic",
    }];
    let describe = |transport: &LocalChatTransport,
                    approval: &LocalChatApproval,
                    model,
                    messages: &[ChatMessage<'_>],
                    tokens,
                    usage| {
        serde_json::to_value(
            transport
                .prepare_chat(approval, model, messages, tokens, usage)
                .unwrap()
                .binding(),
        )
        .unwrap()
    };
    let original = describe(&transport, &approval, "test-model", &messages, 16, false);
    let with_key = LocalChatTransport::new(
        endpoint.clone(),
        Some(BearerKey::new("another-key".into()).unwrap()),
        limits(),
    )
    .unwrap();
    assert_eq!(
        describe(&with_key, &approval, "test-model", &messages, 16, false),
        original
    );
    for changed in [
        describe(&transport, &approval, "other-model", &messages, 16, false),
        describe(&transport, &approval, "test-model", &messages, 17, false),
        describe(&transport, &approval, "test-model", &messages, 16, true),
        describe(
            &transport,
            &approval,
            "test-model",
            &[ChatMessage {
                role: ChatRole::User,
                content: "changed",
            }],
            16,
            false,
        ),
    ] {
        assert_ne!(
            changed["request_body_sha256"],
            original["request_body_sha256"]
        );
        assert_eq!(changed["verified_input_tokens"], Value::Null);
    }
    let (other_path, other_approval) = connection(
        LocalChatEndpoint::parse("http://127.0.0.1:1/other").unwrap(),
        limits(),
    );
    let changed = describe(
        &other_path,
        &other_approval,
        "test-model",
        &messages,
        16,
        false,
    );
    assert_ne!(changed["endpoint"], original["endpoint"]);
    assert_eq!(
        changed["request_body_sha256"],
        original["request_body_sha256"]
    );
    // The fingerprint is explicitly of the body, not an approval or server identity.
    let (short, _) = connection(
        endpoint,
        TransportLimits {
            total: Duration::from_nanos(50),
            idle: Duration::from_nanos(25),
        },
    );
    let changed = describe(&short, &approval, "test-model", &messages, 16, false);
    assert_eq!(changed["total_timeout"], json!({"secs":0,"nanos":50}));
    assert_eq!(changed["idle_timeout"], json!({"secs":0,"nanos":25}));
    assert_eq!(
        changed["request_body_sha256"],
        original["request_body_sha256"]
    );
    assert_ne!(changed, original);
}

struct RunControl {
    stop: std::sync::Arc<std::sync::Mutex<Option<agentmage_kernel_contracts::ModelOperationStop>>>,
    milliseconds: u64,
}

impl agentmage_kernel_contracts::ModelOperationControl for RunControl {
    fn remaining_ms(
        &self,
    ) -> Result<std::num::NonZeroU64, agentmage_kernel_contracts::ModelOperationStop> {
        match *self.stop.lock().unwrap() {
            Some(stop) => Err(stop),
            // Deliberately renews its report. Transport must independently cap
            // dispatch at the first observed budget, even for a broken owner.
            None => Ok(std::num::NonZeroU64::new(self.milliseconds).unwrap()),
        }
    }
}

fn run_control(milliseconds: u64) -> RunControl {
    RunControl {
        stop: Default::default(),
        milliseconds,
    }
}

#[test]
fn controlled_endpoint_refuses_every_owner_stop_before_connecting() {
    use agentmage_kernel_contracts::{ModelOperationControl, ModelOperationStop};
    use std::cell::Cell;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint =
        LocalChatEndpoint::parse(&format!("http://{}/v1", listener.local_addr().unwrap())).unwrap();
    let (transport, approval) = connection(endpoint, limits());
    for stop in [
        ModelOperationStop::Cancelled,
        ModelOperationStop::TimedOut,
        ModelOperationStop::Unavailable,
        ModelOperationStop::Invalid,
    ] {
        // Both initial refusal and refusal on the next pre-connect observation.
        for positive_observations in [0, 1] {
            struct StopAfter {
                seen: Cell<u32>,
                positives: u32,
                stop: ModelOperationStop,
            }
            impl ModelOperationControl for StopAfter {
                fn remaining_ms(&self) -> Result<std::num::NonZeroU64, ModelOperationStop> {
                    let seen = self.seen.get();
                    self.seen.set(seen + 1);
                    if seen < self.positives {
                        Ok(std::num::NonZeroU64::new(1000).unwrap())
                    } else {
                        Err(self.stop)
                    }
                }
            }
            let control = StopAfter {
                seen: Cell::new(0),
                positives: positive_observations,
                stop,
            };
            let request = transport
                .prepare_chat(
                    &approval,
                    "test-model",
                    &[ChatMessage {
                        role: ChatRole::User,
                        content: "synthetic",
                    }],
                    16,
                    false,
                )
                .unwrap();
            let mut emitted = false;
            let cancel = ChatCancellation::default();
            assert_eq!(
                request.execute_recorded_controlled(&cancel, &control, |_| emitted = true),
                Err(TransportError::OperationStopped(stop))
            );
            assert!(!emitted);
            assert!(
                !cancel.is_cancelled(),
                "stop observations never acknowledge or set cancellation"
            );
            assert!(
                matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock)
            );
            assert_eq!(TransportError::OperationStopped(stop).code(), stop.code());
            assert!(!TransportError::OperationStopped(stop).message().is_empty());
        }
    }
}

#[test]
fn controlled_endpoint_stops_stalled_headers_bodies_and_eof_without_completion() {
    use agentmage_kernel_contracts::ModelOperationStop;
    for stage in ["headers", "body", "eof"] {
        for stop in [
            ModelOperationStop::Cancelled,
            ModelOperationStop::TimedOut,
            ModelOperationStop::Unavailable,
            ModelOperationStop::Invalid,
        ] {
            let control = run_control(3000);
            let state = control.stop.clone();
            let (endpoint, worker) = server(move |mut socket| {
                let body = [text_frame("provisional"), ending("stop")].concat();
                let bytes = match stage {
                    "headers" => b"HTTP/1.1 200 OK\r\nContent-Type:".to_vec(),
                    "body" => response("text/event-stream", &body)
                        [..response("text/event-stream", &body).len() - 10]
                        .to_vec(),
                    "eof" => response("text/event-stream", &body),
                    _ => unreachable!(),
                };
                socket.write_all(&bytes).unwrap();
                *state.lock().unwrap() = Some(stop);
                socket
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                // Completion must not depend on a cooperative peer closing.
                let mut byte = [0];
                assert_eq!(socket.read(&mut byte).unwrap(), 0);
            });
            let (transport, approval) = connection(endpoint, limits());
            let request = transport
                .prepare_chat(
                    &approval,
                    "test-model",
                    &[ChatMessage {
                        role: ChatRole::User,
                        content: "synthetic",
                    }],
                    16,
                    true,
                )
                .unwrap();
            let cancel = ChatCancellation::default();
            assert_eq!(
                request.execute_recorded_controlled(&cancel, &control, |_| {}),
                Err(TransportError::OperationStopped(stop)),
                "{stage}"
            );
            assert!(!cancel.is_cancelled());
            worker.join().unwrap();
        }
    }
}

#[test]
fn controlled_endpoint_does_not_renew_deadlines_or_complete_after_a_slow_callback() {
    use agentmage_kernel_contracts::ModelOperationStop;
    for slow_callback in [false, true] {
        let (endpoint, worker) = server(move |mut socket| {
            if slow_callback {
                let body = [text_frame("provisional"), ending("stop")].concat();
                socket
                    .write_all(&response("text/event-stream", &body))
                    .unwrap();
            } else {
                socket
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                assert_eq!(socket.read(&mut [0]).unwrap(), 0);
            }
        });
        let (transport, approval) = connection(endpoint, limits());
        let request = transport
            .prepare_chat(
                &approval,
                "test-model",
                &[ChatMessage {
                    role: ChatRole::User,
                    content: "synthetic",
                }],
                16,
                false,
            )
            .unwrap();
        let control = run_control(100);
        let started = Instant::now();
        let mut fragments = 0;
        let result =
            request.execute_recorded_controlled(&ChatCancellation::default(), &control, |_| {
                fragments += 1;
                thread::sleep(Duration::from_millis(150));
            });
        assert_eq!(
            result,
            Err(TransportError::OperationStopped(
                ModelOperationStop::TimedOut
            ))
        );
        assert!(started.elapsed() >= Duration::from_millis(100));
        assert!(started.elapsed() < Duration::from_secs(2));
        assert_eq!(fragments, usize::from(slow_callback));
        worker.join().unwrap();
    }
    // The original transport limit is still enforced even if the owner allows longer.
    let (endpoint, worker) = server(|mut socket| {
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        assert_eq!(socket.read(&mut [0]).unwrap(), 0);
    });
    let (transport, approval) = connection(
        endpoint,
        TransportLimits {
            total: Duration::from_millis(100),
            idle: Duration::from_millis(100),
        },
    );
    let request = transport
        .prepare_chat(
            &approval,
            "test-model",
            &[ChatMessage {
                role: ChatRole::User,
                content: "synthetic",
            }],
            16,
            false,
        )
        .unwrap();
    assert_eq!(
        request.execute_recorded_controlled(
            &ChatCancellation::default(),
            &run_control(3000),
            |_| {}
        ),
        Err(TransportError::Timeout)
    );
    worker.join().unwrap();
}

#[test]
fn controlled_endpoint_retains_a_later_narrowed_budget_even_if_owner_renews_it() {
    use agentmage_kernel_contracts::{ModelOperationControl, ModelOperationStop};
    use std::cell::Cell;
    struct NarrowOnce(Cell<u32>);
    impl ModelOperationControl for NarrowOnce {
        fn remaining_ms(&self) -> Result<std::num::NonZeroU64, ModelOperationStop> {
            let seen = self.0.get();
            self.0.set(seen + 1);
            Ok(std::num::NonZeroU64::new(if seen == 1 { 100 } else { 3000 }).unwrap())
        }
    }
    let (endpoint, worker) = server(|mut socket| {
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        assert_eq!(socket.read(&mut [0]).unwrap(), 0);
    });
    let (transport, approval) = connection(endpoint, limits());
    let request = transport
        .prepare_chat(
            &approval,
            "test-model",
            &[ChatMessage {
                role: ChatRole::User,
                content: "synthetic",
            }],
            16,
            false,
        )
        .unwrap();
    let started = Instant::now();
    let control = NarrowOnce(Cell::new(0));
    assert_eq!(
        request.execute_recorded_controlled(&ChatCancellation::default(), &control, |_| {}),
        Err(TransportError::OperationStopped(
            ModelOperationStop::TimedOut
        ))
    );
    assert!(control.0.get() > 2);
    assert!(started.elapsed() >= Duration::from_millis(100));
    assert!(started.elapsed() < Duration::from_secs(2));
    worker.join().unwrap();
}

#[test]
fn controlled_endpoint_success_keeps_frozen_request_usage_and_unknown_resources() {
    use sha2::{Digest, Sha256};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint =
        LocalChatEndpoint::parse(&format!("http://{}/v1", listener.local_addr().unwrap())).unwrap();
    let (sent, received) = mpsc::channel();
    let worker = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        let raw = read_request(&mut socket);
        let end = raw.windows(4).position(|v| v == b"\r\n\r\n").unwrap() + 4;
        sent.send(raw[end..].to_vec()).unwrap();
        let body = [text_frame("héllo"), ending("stop")].concat();
        socket
            .write_all(&response("text/event-stream", &body))
            .unwrap();
    });
    let (transport, approval) = connection(endpoint, limits());
    let request = transport
        .prepare_chat(
            &approval,
            "test-model",
            &[ChatMessage {
                role: ChatRole::User,
                content: "escaped\n\"synthetic\"",
            }],
            16,
            true,
        )
        .unwrap();
    let expected = request.binding().clone();
    let mut text = String::new();
    let cancel = ChatCancellation::default();
    let result = request
        .execute_recorded_controlled(&cancel, &run_control(3000), |fragment| {
            text.push_str(fragment)
        })
        .unwrap();
    assert_eq!(result.request, expected);
    assert_eq!(result.response_text_bytes, text.len());
    assert_eq!(text, "héllo");
    assert_eq!(
        result.response_text_sha256,
        Sha256::digest(text.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    let body = received.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(expected.body().request_body_bytes, body.len());
    assert_eq!(
        expected.body().request_body_sha256,
        Sha256::digest(body)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    let value = serde_json::to_value(result).unwrap();
    assert!(value["token_accounting"]["provider_reported"].is_null());
    assert!(value["token_accounting"]["verified_input_tokens"].is_null());
    assert!(value["token_accounting"]["verified_output_tokens"].is_null());
    assert!(value["resources"]["resident_memory_bytes"].is_null());
    assert!(value["resources"]["accelerator_memory_bytes"].is_null());
    assert!(!cancel.is_cancelled());
    worker.join().unwrap();
}

#[test]
fn shared_prepared_request_retains_exact_transport_body_key_and_releases_after_dispatch() {
    use sha2::{Digest, Sha256};
    use std::sync::Arc;
    for include_usage in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let endpoint =
            LocalChatEndpoint::parse(&format!("http://{}/api/v1", listener.local_addr().unwrap()))
                .unwrap();
        let approval = LocalChatApproval::for_endpoint(endpoint.clone());
        let transport = Arc::new(
            LocalChatTransport::new(
                endpoint,
                Some(BearerKey::new("synthetic-retained-key".into()).unwrap()),
                limits(),
            )
            .unwrap(),
        );
        let weak = Arc::downgrade(&transport);
        let mut prompt = "synthetic private source: é \n\"".to_owned();
        let messages = [ChatMessage {
            role: ChatRole::User,
            content: &prompt,
        }];
        let ordinary = transport
            .prepare_chat(&approval, "test-model", &messages, 16, include_usage)
            .unwrap();
        let binding = ordinary.binding().clone();
        drop(ordinary);
        let prepared = Arc::clone(&transport)
            .prepare_chat_shared(&approval, "test-model", &messages, 16, include_usage)
            .unwrap();
        assert_eq!(prepared.binding(), &binding);
        let expected: serde_json::Value = serde_json::from_slice(&if include_usage {
            agentmage_platform_linux_inference::local_chat::encode_chat_with_usage(
                "test-model",
                &messages,
                16,
            )
            .unwrap()
        } else {
            agentmage_platform_linux_inference::local_chat::encode_chat("test-model", &messages, 16)
                .unwrap()
        })
        .unwrap();
        prompt.clear();
        drop(approval);
        drop(transport);
        assert_eq!(weak.strong_count(), 1);
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        let worker = thread::spawn(move || {
            let start = Instant::now();
            let mut socket = loop {
                match listener.accept() {
                    Ok((socket, _)) => break socket,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(start.elapsed() < Duration::from_secs(4));
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(e) => panic!("{e}"),
                }
            };
            let request = read_request(&mut socket);
            let offset = request.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
            let headers = std::str::from_utf8(&request[..offset]).unwrap();
            assert!(headers.starts_with("POST /api/v1/chat/completions HTTP/1.1\r\n"));
            assert!(headers.contains("Authorization: Bearer synthetic-retained-key\r\n"));
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&request[offset..]).unwrap(),
                expected
            );
            let digest = Sha256::digest(&request[offset..])
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            socket
                .write_all(&response(
                    "text/event-stream",
                    &[text_frame("answer"), ending("stop")].concat(),
                ))
                .unwrap();
            socket.shutdown(Shutdown::Write).unwrap();
            assert_eq!(
                listener.accept().unwrap_err().kind(),
                std::io::ErrorKind::WouldBlock
            );
            digest
        });
        // The frozen request is movable to another worker with no settings borrow.
        let completion =
            thread::spawn(move || prepared.execute_recorded(&ChatCancellation::default(), |_| {}))
                .join()
                .unwrap()
                .unwrap();
        assert_eq!(&completion.request, &binding);
        let observation = serde_json::to_value(&completion).unwrap();
        assert_eq!(
            observation["request"]["request_body_sha256"],
            worker.join().unwrap()
        );
        assert!(!observation.to_string().contains("synthetic-retained-key"));
        assert!(!observation.to_string().contains("synthetic private source"));
        assert!(weak.upgrade().is_none());
    }
}

#[test]
fn shared_preparation_preserves_refusal_control_and_drop_without_connection() {
    use agentmage_kernel_contracts::ModelOperationStop;
    use std::sync::Arc;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint =
        LocalChatEndpoint::parse(&format!("http://{}/v1", listener.local_addr().unwrap())).unwrap();
    let (transport, approval) = connection(endpoint, limits());
    let transport = Arc::new(transport);
    let weak = Arc::downgrade(&transport);
    let messages = [ChatMessage {
        role: ChatRole::User,
        content: "synthetic",
    }];
    let wrong = LocalChatApproval::for_endpoint(
        LocalChatEndpoint::parse(&format!("http://{}/other", listener.local_addr().unwrap()))
            .unwrap(),
    );
    assert!(matches!(
        Arc::clone(&transport).prepare_chat_shared(&wrong, "test-model", &messages, 16, false),
        Err(TransportError::Approval)
    ));
    for (model, tokens) in [("", 16), ("test-model", 0), ("test-model", 65537)] {
        assert!(matches!(
            Arc::clone(&transport).prepare_chat_shared(&approval, model, &messages, tokens, true),
            Err(TransportError::Chat(ChatError::Request))
        ));
    }
    assert_eq!(weak.strong_count(), 1);
    let prepare = || {
        Arc::clone(&transport)
            .prepare_chat_shared(&approval, "test-model", &messages, 16, true)
            .unwrap()
    };
    drop(prepare());
    for stop in [
        ModelOperationStop::Cancelled,
        ModelOperationStop::TimedOut,
        ModelOperationStop::Unavailable,
        ModelOperationStop::Invalid,
    ] {
        let control = run_control(1000);
        *control.stop.lock().unwrap() = Some(stop);
        assert_eq!(
            prepare().execute_recorded_controlled(
                &ChatCancellation::default(),
                &control,
                |_| panic!("no text")
            ),
            Err(TransportError::OperationStopped(stop))
        );
        assert_eq!(weak.strong_count(), 1);
    }
    let prepared = prepare();
    drop(transport);
    assert_eq!(weak.strong_count(), 1);
    let cancel = ChatCancellation::default();
    cancel.cancel();
    assert_eq!(
        prepared.execute(&cancel, |_| panic!("no text")),
        Err(TransportError::Cancelled)
    );
    assert!(weak.upgrade().is_none());
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn prepared_text_ceiling_only_narrows_without_connecting_or_changing_body() {
    use std::sync::Arc;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint =
        LocalChatEndpoint::parse(&format!("http://{}/v1", listener.local_addr().unwrap())).unwrap();
    let (transport, approval) = connection(endpoint, limits());
    let transport = Arc::new(transport);
    let messages = [ChatMessage {
        role: ChatRole::User,
        content: "synthetic",
    }];
    for shared in [false, true] {
        let prepare = || {
            if shared {
                Arc::clone(&transport).prepare_chat_shared(
                    &approval,
                    "test-model",
                    &messages,
                    16,
                    true,
                )
            } else {
                transport.prepare_chat(&approval, "test-model", &messages, 16, true)
            }
            .unwrap()
        };
        let original = prepare().binding().clone();
        for cap in [0, original.limits().response_text_limit_bytes + 1] {
            assert!(matches!(
                prepare().with_response_text_limit(cap),
                Err(TransportError::Configuration)
            ));
        }
        let narrowed = prepare().with_response_text_limit(5).unwrap();
        let mut expected = serde_json::to_value(&original).unwrap();
        expected["response_text_limit_bytes"] = 5.into();
        assert_eq!(serde_json::to_value(narrowed.binding()).unwrap(), expected);
        assert_eq!(
            serde_json::to_value(narrowed.with_response_text_limit(5).unwrap().binding()).unwrap(),
            expected
        );
        assert!(matches!(
            prepare()
                .with_response_text_limit(5)
                .unwrap()
                .with_response_text_limit(6),
            Err(TransportError::Configuration)
        ));
        assert_eq!(
            prepare()
                .with_response_text_limit(1)
                .unwrap()
                .binding()
                .limits()
                .response_text_limit_bytes,
            1
        );
        assert_eq!(
            prepare()
                .with_response_text_limit(original.limits().response_text_limit_bytes)
                .unwrap()
                .binding(),
            &original
        );
    }
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn narrowed_text_ceiling_counts_utf8_across_fragments_before_callbacks() {
    use sha2::Digest;
    use std::sync::Arc;
    for shared in [false, true] {
        for include_usage in [false, true] {
            for overflow in [false, true] {
                let (endpoint, peer) = server(move |mut stream| {
                    let wire = response(
                        "text/event-stream",
                        &[
                            text_frame("é"),
                            text_frame("a"),
                            text_frame(if overflow { "€" } else { "é" }),
                            ending("stop"),
                        ]
                        .concat(),
                    );
                    if let Err(e) = stream.write_all(&wire) {
                        assert!(matches!(
                            e.kind(),
                            std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset
                        ));
                    }
                });
                let (transport, approval) = connection(endpoint, limits());
                let transport = Arc::new(transport);
                let messages = [ChatMessage {
                    role: ChatRole::User,
                    content: "synthetic",
                }];
                let prepared = if shared {
                    Arc::clone(&transport).prepare_chat_shared(
                        &approval,
                        "test-model",
                        &messages,
                        16,
                        include_usage,
                    )
                } else {
                    transport.prepare_chat(&approval, "test-model", &messages, 16, include_usage)
                }
                .unwrap()
                .with_response_text_limit(5)
                .unwrap();
                let binding = prepared.binding().clone();
                let mut text = String::new();
                let result = prepared.execute_recorded_controlled(
                    &ChatCancellation::default(),
                    &run_control(2000),
                    |part| text.push_str(part),
                );
                peer.join().unwrap();
                if overflow {
                    assert_eq!(result, Err(TransportError::Limit));
                    assert_eq!(text, "éa");
                } else {
                    let result = result.unwrap();
                    assert_eq!(text, "éaé");
                    assert_eq!(result.response_text_bytes, 5);
                    assert_eq!(result.request, binding);
                    assert_eq!(
                        result.response_text_sha256,
                        sha2::Sha256::digest(text.as_bytes())
                            .iter()
                            .map(|b| format!("{b:02x}"))
                            .collect::<String>()
                    );
                }
            }
        }
    }
}
