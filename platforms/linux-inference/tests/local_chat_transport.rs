// SPDX-License-Identifier: Apache-2.0
//! Actual loopback sockets, synthetic servers only; no model qualification.

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use agentmage_platform_linux_inference::local_chat::{
    ChatError, ChatFinish, ChatMessage, ChatRole, LocalChatEndpoint,
};
use agentmage_platform_linux_inference::local_chat_transport::{
    BearerKey, ChatCancellation, LocalChatApproval, LocalChatTransport, LocalProbeApproval,
    TransportError, TransportLimits, probe_endpoints,
};

const LIST: &str = "{\"data\":[{\"id\":\"test-model\"},{\"id\":\"second-model\"}]}";

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
