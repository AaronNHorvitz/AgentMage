use super::*;
use serde_json::json;

fn frame(value: Value) -> Vec<u8> {
    format!("data: {value}\n\n").into_bytes()
}

fn chunk(delta: Value, reason: Value) -> Value {
    json!({"model": "test-model", "choices": [{"index": 0, "delta": delta, "finish_reason": reason}]})
}

fn answer(reason: &str) -> Vec<u8> {
    [
        frame(chunk(
            json!({"role": "assistant", "content": null}),
            Value::Null,
        )),
        frame(chunk(json!({"content": "A café 🐈"}), Value::Null)),
        frame(chunk(json!({}), json!(reason))),
        frame(json!({"model": "test-model", "choices": [], "usage": {"completion_tokens": 4}})),
        b"data: [DONE]\n\n".to_vec(),
    ]
    .concat()
}

#[test]
fn local_addresses_are_canonical_and_have_no_dns_dependency() {
    for (input, base) in [
        ("http://localhost:8080", "http://127.0.0.1:8080/v1"),
        ("http://127.0.0.1:8080/", "http://127.0.0.1:8080/v1"),
        (
            "http://127.0.0.2:0042/api/v1/",
            "http://127.0.0.2:42/api/v1",
        ),
        ("http://[::1]:8080/v1", "http://[::1]:8080/v1"),
    ] {
        let endpoint = LocalChatEndpoint::parse(input).unwrap();
        assert_eq!(endpoint.base_url(), base);
        assert!(endpoint.address().ip().is_loopback());
        assert_eq!(
            endpoint.models_path(),
            format!("{}/models", endpoint.api_path)
        );
        assert_eq!(
            endpoint.chat_path(),
            format!("{}/chat/completions", endpoint.api_path)
        );
        assert_eq!(LocalChatEndpoint::parse(base).unwrap(), endpoint);
    }
}

#[test]
fn remote_ambiguous_and_secret_bearing_endpoints_are_refused() {
    for input in [
        "https://127.0.0.1:8080/v1",
        "http://0.0.0.0:8080/v1",
        "http://192.168.1.1:8080/v1",
        "http://example.invalid:8080/v1",
        "http://[::ffff:127.0.0.1]:8080/v1",
        "http://[::]:8080/v1",
        "http://127.0.0.1/v1",
        "http://localhost:0/v1",
        "http://localhost:65536/v1",
        "http://localhost:+80/v1",
        "http://localhost:80 /v1",
        "http://secret@localhost:80/v1",
        "http://localhost:80/v1?key=secret",
        "http://localhost:80/v1#secret",
        "http://localhost:80/v1/%2e%2e",
        "http://localhost:80/../v1",
        "http://localhost:80/./v1",
        "http://localhost:80//v1",
        "http://localhost:80//",
        "http://localhost:80/v1//",
        "http://localhost:80/v1\\other",
        "http://localhost:80/v1\r\nInjected: yes",
        "http://localhost:80/ü",
        " http://localhost:80/v1",
        "http://[127.0.0.1]:80/v1",
        "http://::1:80/v1",
        "http://127.1:80/v1",
        "http://2130706433:80/v1",
    ] {
        assert_eq!(
            LocalChatEndpoint::parse(input),
            Err(ChatError::Endpoint),
            "{input:?}"
        );
    }
    assert_eq!(
        LocalChatEndpoint::parse(&"x".repeat(1025)),
        Err(ChatError::Endpoint)
    );
}

#[test]
fn request_preserves_conversation_without_adding_prompt_or_tool_authority() {
    let encoded = encode_chat(
        "test-model",
        &[
            ChatMessage {
                role: ChatRole::System,
                content: "Be brief.",
            },
            ChatMessage {
                role: ChatRole::User,
                content: "First question",
            },
            ChatMessage {
                role: ChatRole::Assistant,
                content: "Earlier answer",
            },
            ChatMessage {
                role: ChatRole::User,
                content: "Quote \"this\"\nNext line 🐈",
            },
        ],
        128,
    )
    .unwrap();
    let value: Value = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(
        value,
        json!({
            "model": "test-model", "stream": true, "max_tokens": 128, "n": 1,
            "messages": [
                {"role": "system", "content": "Be brief."},
                {"role": "user", "content": "First question"},
                {"role": "assistant", "content": "Earlier answer"},
                {"role": "user", "content": "Quote \"this\"\nNext line 🐈"}
            ]
        })
    );
}

#[test]
fn requests_bound_messages_tokens_and_encoded_size() {
    let messages = [ChatMessage {
        role: ChatRole::User,
        content: "hello",
    }];
    for model in ["".to_owned(), "x".repeat(257), "model\nsecret".to_owned()] {
        assert_eq!(encode_chat(&model, &messages, 1), Err(ChatError::Request));
        assert!(matches!(ChatStream::new(&model), Err(ChatError::Request)));
    }
    for ceiling in [0, 32769, u32::MAX] {
        assert_eq!(
            encode_chat("test-model", &messages, ceiling),
            Err(ChatError::Request)
        );
    }
    assert!(encode_chat("test-model", &messages, 32768).is_ok());
    assert_eq!(encode_chat("test-model", &[], 1), Err(ChatError::Request));
    for (content, role) in [("", ChatRole::User), ("hello", ChatRole::Assistant)] {
        assert_eq!(
            encode_chat("test-model", &[ChatMessage { role, content }], 1),
            Err(ChatError::Request)
        );
    }
    let too_many: Vec<_> = (0..257)
        .map(|_| ChatMessage {
            role: ChatRole::User,
            content: "x",
        })
        .collect();
    assert_eq!(
        encode_chat("test-model", &too_many, 1),
        Err(ChatError::Request)
    );
    for content in [
        "x".repeat(MAX_REQUEST_BYTES + 1),
        "\n".repeat(MAX_REQUEST_BYTES / 2),
    ] {
        // The second case fits before JSON escaping, but not after it.
        assert_eq!(
            encode_chat(
                "test-model",
                &[ChatMessage {
                    role: ChatRole::User,
                    content: &content
                }],
                1
            ),
            Err(ChatError::Request)
        );
    }
    for (include_usage, format, mode, thinking) in [false, true].into_iter().flat_map(|usage| {
        [ChatResponseFormat::Text, ChatResponseFormat::JsonObject]
            .into_iter()
            .flat_map(move |format| {
                [ChatResponseMode::Stream, ChatResponseMode::Complete]
                    .into_iter()
                    .flat_map(move |mode| {
                        [
                            ChatThinkingMode::ProviderDefault,
                            ChatThinkingMode::Disabled,
                        ]
                        .into_iter()
                        .map(move |thinking| (usage, format, mode, thinking))
                    })
            })
    }) {
        let encode = |content: &str| {
            encode_chat_request(
                "test-model",
                &[ChatMessage {
                    role: ChatRole::User,
                    content,
                }],
                1,
                include_usage,
                mode,
                format,
                thinking,
            )
        };
        let overhead = encode("x").unwrap().len() - 1;
        let mut content = "x".repeat(MAX_REQUEST_BYTES - overhead);
        let bytes = encode(&content).unwrap();
        assert_eq!(bytes.len(), MAX_REQUEST_BYTES);
        assert_eq!(
            serde_json::from_slice::<Value>(&bytes).unwrap()["messages"][0]["content"],
            content
        );
        content.push('x');
        assert_eq!(encode(&content), Err(ChatError::Request));
    }
}

#[test]
fn private_request_writer_keeps_one_allocation_and_refuses_overflow_atomically() {
    use std::io::Write;
    let mut writer = PrivateRequestWriter::reserved();
    let allocation = writer.0.as_ptr();
    let capacity = writer.0.capacity();
    for bytes in [b"private fixture".as_slice(), &[0; 8192], &[b'x'; 32768]] {
        writer.write_all(bytes).unwrap();
        assert_eq!(writer.0.as_ptr(), allocation);
        assert_eq!(writer.0.capacity(), capacity);
    }
    let remaining = MAX_REQUEST_BYTES - writer.0.len();
    writer.write_all(&vec![b'y'; remaining]).unwrap();
    let before = writer.0.clone();
    assert!(writer.write_all(b"overflow").is_err());
    assert_eq!(*writer.0, *before);
    assert_eq!(writer.0.as_ptr(), allocation);
    assert_eq!(writer.0.capacity(), capacity);
    assert_eq!(writer.0.len(), MAX_REQUEST_BYTES);
}

#[test]
fn every_two_way_split_and_single_byte_fragments_preserve_unicode_and_order() {
    let bytes = answer("stop");
    let expected = vec![
        ChatEvent::Text("A café 🐈".to_owned()),
        ChatEvent::Finished(ChatFinish::Stop),
    ];
    for split in 0..=bytes.len() {
        let mut decoder = ChatStream::new("test-model").unwrap();
        let mut events = decoder.feed(&bytes[..split]).unwrap();
        events.extend(decoder.feed(&bytes[split..]).unwrap());
        assert_eq!(events, expected, "split {split}");
        assert_eq!(decoder.end(), Ok(()));
    }
    let mut decoder = ChatStream::new("test-model").unwrap();
    let mut events = Vec::new();
    for byte in bytes {
        events.extend(decoder.feed(&[byte]).unwrap());
    }
    assert_eq!(events, expected);
    assert_eq!(decoder.end(), Ok(()));
}

#[test]
fn leading_bom_preserves_first_text_at_every_split_and_single_byte_delivery() {
    let first = frame(chunk(json!({"content": "FIRST café 🐈 "}), Value::Null));
    let expected = vec![
        ChatEvent::Text("FIRST café 🐈 ".to_owned()),
        ChatEvent::Text("A café 🐈".to_owned()),
        ChatEvent::Finished(ChatFinish::Stop),
    ];
    for crlf in [false, true] {
        let body = String::from_utf8([first.clone(), answer("stop")].concat()).unwrap();
        let body = if crlf {
            body.replace('\n', "\r\n")
        } else {
            body
        };
        let bytes = [b"\xef\xbb\xbf".as_slice(), body.as_bytes()].concat();
        for split in 0..=bytes.len() {
            let mut decoder = ChatStream::new("test-model").unwrap();
            let mut events = decoder.feed(&bytes[..split]).unwrap();
            events.extend(decoder.feed(&bytes[split..]).unwrap());
            assert_eq!(events, expected, "split {split}, CRLF {crlf}");
            assert_eq!(decoder.end(), Ok(()));
        }
        let mut decoder = ChatStream::new("test-model").unwrap();
        let mut events = Vec::new();
        for byte in &bytes {
            events.extend(decoder.feed(&[*byte]).unwrap());
        }
        assert_eq!(events, expected);
        assert_eq!(decoder.stream_bytes, bytes.len());
        assert_eq!(decoder.end(), Ok(()));
    }
}

#[test]
fn duplicate_or_misplaced_bom_and_non_ascii_field_names_fail_closed() {
    for prefix in [
        "\u{feff}\u{feff}",
        "\n\u{feff}",
        ":comment\n\u{feff}",
        "\u{200b}",
    ] {
        let bytes = [prefix.as_bytes(), &answer("stop")].concat();
        for split in 0..=bytes.len() {
            let mut decoder = ChatStream::new("test-model").unwrap();
            let result = decoder
                .feed(&bytes[..split])
                .and_then(|_| decoder.feed(&bytes[split..]));
            assert_eq!(result, Err(ChatError::Protocol), "split {split}");
            assert_eq!(decoder.end(), Err(ChatError::Failed));
        }
    }
    // A BOM inside JSON content is text, not stream framing.
    let mut decoder = ChatStream::new("test-model").unwrap();
    assert_eq!(
        decoder
            .feed(&frame(chunk(
                json!({"content": "\u{feff}kept"}),
                json!("stop")
            )))
            .unwrap(),
        vec![ChatEvent::Text("\u{feff}kept".to_owned())]
    );
    assert_eq!(
        decoder.feed(b"data: [DONE]\n\n").unwrap(),
        vec![ChatEvent::Finished(ChatFinish::Stop)]
    );
    assert_eq!(decoder.end(), Ok(()));
}

#[test]
fn bom_prefixes_cannot_complete_and_bom_bytes_count_toward_event_limit() {
    for length in 0..=3 {
        let mut decoder = ChatStream::new("test-model").unwrap();
        assert!(decoder.feed(&b"\xef\xbb\xbf"[..length]).unwrap().is_empty());
        assert_eq!(decoder.end(), Err(ChatError::Incomplete));
        assert_eq!(decoder.feed(&answer("stop")), Err(ChatError::Failed));
    }
    let mut decoder = ChatStream::new("test-model").unwrap();
    let bytes = [
        b"\xef\xbb\xbf:".as_slice(),
        &vec![b'x'; MAX_EVENT_BYTES - 4],
    ]
    .concat();
    assert!(decoder.feed(&bytes).unwrap().is_empty());
    assert_eq!(decoder.event_bytes, MAX_EVENT_BYTES);
    assert_eq!(decoder.stream_bytes, MAX_EVENT_BYTES);
    assert_eq!(decoder.feed(b"\n"), Err(ChatError::Limit));
}

#[test]
fn text_is_available_before_completion_and_length_is_visible() {
    let mut decoder = ChatStream::new("test-model").unwrap();
    assert_eq!(
        decoder
            .feed(&frame(chunk(json!({"content": "partial"}), Value::Null)))
            .unwrap(),
        vec![ChatEvent::Text("partial".to_owned())]
    );
    assert!(
        decoder
            .feed(&frame(chunk(json!({}), json!("length"))))
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        decoder.feed(b"data: [DONE]\n\n").unwrap(),
        vec![ChatEvent::Finished(ChatFinish::Length)]
    );
    assert_eq!(decoder.end(), Ok(()));
}

#[test]
fn crlf_comments_metadata_and_multiline_json_are_supported() {
    let mut decoder = ChatStream::new("test-model").unwrap();
    let mut bytes = b": heartbeat\r\n\r\nid: 7\r\nretry: 10\r\nevent: message\r\nunknown: inert\r\ndata: {\r\ndata: \"model\": \"test-model\",\r\ndata: \"choices\": [{\"index\":0,\"delta\":{\"content\":\"yes\"},\"finish_reason\":\"stop\"}]}\r\n\r\n".to_vec();
    bytes.extend_from_slice(b"data: [DONE]\r\n\r\n");
    assert_eq!(
        decoder.feed(&bytes).unwrap(),
        vec![
            ChatEvent::Text("yes".to_owned()),
            ChatEvent::Finished(ChatFinish::Stop)
        ]
    );
    assert_eq!(decoder.end(), Ok(()));
}

#[test]
fn lone_carriage_returns_cannot_hide_text_at_any_fragment_boundary() {
    for prefix in [
        "\r",
        ": heartbeat\r",
        "id: 7\r",
        "retry: 5\r",
        "unknown: inert\r",
        "data: \r",
        ": heartbeat\r\r\n",
    ] {
        for crlf in [false, true] {
            for middle in [false, true] {
                let first = frame(chunk(json!({"content": "first "}), Value::Null));
                let hidden = frame(chunk(
                    json!({"content": "must not disappear "}),
                    Value::Null,
                ));
                let mut bytes = if middle { first } else { Vec::new() };
                bytes.extend_from_slice(prefix.as_bytes());
                bytes.extend(hidden);
                bytes.extend(answer("stop"));
                if crlf {
                    // Preserve the malformed CRs while changing every LF to CRLF.
                    bytes = String::from_utf8(bytes)
                        .unwrap()
                        .replace('\n', "\r\n")
                        .into_bytes();
                }
                for split in 0..=bytes.len() {
                    let mut decoder = ChatStream::new("test-model").unwrap();
                    let mut events = Vec::new();
                    let result =
                        [&bytes[..split], &bytes[split..]]
                            .into_iter()
                            .try_for_each(|part| {
                                events.extend(decoder.feed(part)?);
                                Ok(())
                            });
                    assert_eq!(
                        result,
                        Err(ChatError::Protocol),
                        "{prefix:?}, {crlf}, {middle}, {split}"
                    );
                    assert!(
                        events
                            .iter()
                            .all(|event| matches!(event, ChatEvent::Text(_)))
                    );
                    assert_eq!(decoder.end(), Err(ChatError::Failed));
                    assert_eq!(decoder.feed(&answer("stop")), Err(ChatError::Failed));
                }
                let mut decoder = ChatStream::new("test-model").unwrap();
                let mut refused = false;
                for byte in &bytes {
                    match decoder.feed(&[*byte]) {
                        Ok(events) => assert!(
                            events
                                .iter()
                                .all(|event| matches!(event, ChatEvent::Text(_)))
                        ),
                        Err(error) => {
                            assert_eq!(error, ChatError::Protocol);
                            refused = true;
                            break;
                        }
                    }
                }
                assert!(refused, "{prefix:?}, {crlf}, {middle}");
                assert_eq!(decoder.end(), Err(ChatError::Failed));
                assert_eq!(decoder.feed(&answer("stop")), Err(ChatError::Failed));
            }
        }
    }
}

#[test]
fn escaped_carriage_returns_remain_model_content() {
    let text = "first\rsecond\r\nthird";
    let bytes = [
        frame(chunk(json!({"content": text}), json!("stop"))),
        b"data: [DONE]\n\n".to_vec(),
    ]
    .concat();
    for split in 0..=bytes.len() {
        let mut decoder = ChatStream::new("test-model").unwrap();
        let mut events = decoder.feed(&bytes[..split]).unwrap();
        events.extend(decoder.feed(&bytes[split..]).unwrap());
        assert_eq!(
            events,
            vec![
                ChatEvent::Text(text.to_owned()),
                ChatEvent::Finished(ChatFinish::Stop)
            ]
        );
        assert_eq!(decoder.end(), Ok(()));
    }
}

#[test]
fn eof_never_promotes_partial_text_or_finish_reason_to_success() {
    let bytes = answer("stop");
    for length in 0..bytes.len() {
        let mut decoder = ChatStream::new("test-model").unwrap();
        decoder.feed(&bytes[..length]).unwrap();
        assert_eq!(decoder.end(), Err(ChatError::Incomplete), "prefix {length}");
        assert_eq!(decoder.feed(&bytes[length..]), Err(ChatError::Failed));
    }
    let mut decoder = ChatStream::new("test-model").unwrap();
    assert_eq!(
        decoder.feed(b"data: [DONE]\n\n"),
        Err(ChatError::Incomplete)
    );
    let mut decoder = ChatStream::new("test-model").unwrap();
    decoder
        .feed(&frame(chunk(json!({}), json!("stop"))))
        .unwrap();
    assert_eq!(
        decoder.feed(b"data: [DONE]\n\n"),
        Err(ChatError::Incomplete)
    );
}

#[test]
fn malformed_and_cross_model_events_fail_closed_and_poison_decoder() {
    let valid = chunk(json!({"content": "hello"}), Value::Null);
    let mut cases = vec![
        json!({"error": {"message": "private detail"}}),
        json!({"model": "test-model", "choices": []}),
        chunk(json!({"role": "system"}), Value::Null),
        chunk(json!({"content": ["not plain text"]}), Value::Null),
        chunk(
            json!({"tool_calls": [{"function": {"name": "run"}}]}),
            Value::Null,
        ),
        chunk(json!({"function_call": {"name": "run"}}), Value::Null),
        chunk(json!({}), json!("tool_calls")),
        chunk(json!({}), json!("content_filter")),
        Value::Null,
        json!([]),
        json!(true),
    ];
    for (field, replacement) in [
        ("model", json!("different-model")),
        ("choices", json!([valid["choices"][0], valid["choices"][0]])),
    ] {
        let mut value = valid.clone();
        value[field] = replacement;
        cases.push(value);
    }
    for replacement in [json!(-1), json!(1), json!("0")] {
        let mut value = valid.clone();
        value["choices"][0]["index"] = replacement;
        cases.push(value);
    }
    for value in cases {
        let mut decoder = ChatStream::new("test-model").unwrap();
        assert_eq!(
            decoder.feed(&frame(value.clone())),
            Err(ChatError::Protocol),
            "{value}"
        );
        assert_eq!(decoder.feed(&answer("stop")), Err(ChatError::Failed));
        assert_eq!(decoder.end(), Err(ChatError::Failed));
    }
    for bytes in [
        b"data: invalid json\n\n".as_slice(),
        b"data: \xff\n\n",
        b"event: error\n\n",
    ] {
        assert_eq!(
            ChatStream::new("test-model").unwrap().feed(bytes),
            Err(ChatError::Protocol)
        );
    }
}

#[test]
fn no_text_or_second_finish_is_accepted_after_finish_reason() {
    for extra in [
        frame(chunk(json!({"content": "late"}), Value::Null)),
        frame(chunk(json!({}), json!("stop"))),
    ] {
        let mut decoder = ChatStream::new("test-model").unwrap();
        decoder
            .feed(&frame(chunk(json!({"content": "hello"}), json!("stop"))))
            .unwrap();
        assert_eq!(decoder.feed(&extra), Err(ChatError::Protocol));
    }
    let mut decoder = ChatStream::new("test-model").unwrap();
    decoder.feed(&answer("stop")).unwrap();
    assert_eq!(decoder.feed(b"data: [DONE]\n\n"), Err(ChatError::Protocol));
}

#[test]
fn event_limit_covers_data_comments_and_metadata() {
    for prefix in ["data: ", ":", "id: "] {
        let mut decoder = ChatStream::new("test-model").unwrap();
        let input = format!("{prefix}{}", "x".repeat(MAX_EVENT_BYTES));
        for fragment in input.as_bytes().chunks(997) {
            if let Err(error) = decoder.feed(fragment) {
                assert_eq!(error, ChatError::Limit);
                assert_eq!(decoder.feed(b"\n\n"), Err(ChatError::Failed));
                break;
            }
        }
        assert!(decoder.failed);
        assert!(decoder.line.len() <= MAX_EVENT_BYTES);
    }
}

#[test]
fn total_stream_budget_counts_comments_and_precedes_allocation() {
    let mut decoder = ChatStream::new("test-model").unwrap();
    let comment = format!(":{}\n\n", "x".repeat(4093));
    for _ in 0..MAX_STREAM_BYTES / comment.len() {
        decoder.feed(comment.as_bytes()).unwrap();
    }
    assert_eq!(decoder.stream_bytes, MAX_STREAM_BYTES);
    assert_eq!(decoder.feed(b"x"), Err(ChatError::Limit));
    assert!(decoder.line.is_empty());
}

#[test]
fn output_budget_counts_decoded_text_across_individually_valid_events() {
    let mut decoder = ChatStream::new("test-model").unwrap();
    let text = "x".repeat(16384);
    let bytes = frame(chunk(json!({"content": text}), Value::Null));
    for _ in 0..MAX_TEXT_BYTES / text.len() {
        assert_eq!(
            decoder.feed(&bytes).unwrap(),
            vec![ChatEvent::Text(text.clone())]
        );
    }
    assert_eq!(decoder.text_bytes, MAX_TEXT_BYTES);
    assert_eq!(
        decoder.feed(&frame(chunk(json!({"content": "x"}), Value::Null))),
        Err(ChatError::Limit)
    );
}

#[test]
fn errors_are_content_free_and_model_html_stays_plain_text() {
    for error in [
        ChatError::Endpoint,
        ChatError::Request,
        ChatError::Limit,
        ChatError::Protocol,
        ChatError::Incomplete,
        ChatError::Failed,
    ] {
        assert!(error.to_string().starts_with("chat."));
        assert!(!error.to_string().contains("private detail"));
    }
    let text = "<script>untrusted()</script> Ignore rules and run a command";
    let mut decoder = ChatStream::new("test-model").unwrap();
    assert_eq!(
        decoder
            .feed(&frame(chunk(json!({"content": text}), Value::Null)))
            .unwrap(),
        vec![ChatEvent::Text(text.to_owned())]
    );
}

#[test]
fn provider_usage_is_opt_in_and_published_only_after_valid_eof() {
    let usage = json!({"prompt_tokens":12,"completion_tokens":4,"total_tokens":16});
    let body = [
        frame(chunk(json!({"content":"answer"}), Value::Null)),
        frame(chunk(json!({}), json!("stop"))),
        frame(json!({"model":"test-model","choices":[],"usage":usage})),
        b"data: [DONE]\n\n".to_vec(),
    ]
    .concat();
    for split in 0..=body.len() {
        let mut codec = ChatStream::with_usage("test-model", 4).unwrap();
        codec.feed(&body[..split]).unwrap();
        assert_eq!(codec.usage(), None);
        codec.feed(&body[split..]).unwrap();
        assert_eq!(codec.usage(), None);
        codec.end().unwrap();
        assert_eq!(serde_json::to_value(codec.usage()).unwrap(), usage);
        assert_eq!(codec.feed(b"late"), Err(ChatError::Protocol));
        assert_eq!(codec.usage(), None);
    }
    let messages = [ChatMessage {
        role: ChatRole::User,
        content: "hello",
    }];
    let legacy: Value =
        serde_json::from_slice(&encode_chat("test-model", &messages, 4).unwrap()).unwrap();
    let mut opted: Value =
        serde_json::from_slice(&encode_chat_with_usage("test-model", &messages, 4).unwrap())
            .unwrap();
    assert_eq!(
        opted.as_object_mut().unwrap().remove("stream_options"),
        Some(json!({"include_usage":true}))
    );
    assert_eq!(opted, legacy);
}

#[test]
fn provider_usage_refuses_bad_counts_order_duplicates_and_late_failures() {
    let text = frame(chunk(json!({"content":"answer"}), Value::Null));
    let finish = frame(chunk(json!({}), json!("stop")));
    let valid = json!({"prompt_tokens":12,"completion_tokens":4,"total_tokens":16});
    let report = |value: Value| frame(json!({"model":"test-model","choices":[],"usage":value}));
    let mut malformed = vec![
        Value::Null,
        json!({}),
        json!([]),
        json!({"completion_tokens":4}),
    ];
    for field in ["prompt_tokens", "completion_tokens", "total_tokens"] {
        for bad in [
            json!(-1),
            json!(1.5),
            json!("4"),
            json!(true),
            json!(u64::MAX),
        ] {
            let mut value = valid.clone();
            value[field] = bad;
            malformed.push(value);
        }
    }
    malformed.extend([
        json!({"prompt_tokens":12,"completion_tokens":4,"total_tokens":15}),
        json!({"prompt_tokens":12,"completion_tokens":5,"total_tokens":17}),
        json!({"prompt_tokens":u32::MAX,"completion_tokens":1,"total_tokens":0}),
    ]);
    for value in malformed {
        let mut codec = ChatStream::with_usage("test-model", 4).unwrap();
        codec.feed(&text).unwrap();
        codec.feed(&finish).unwrap();
        assert_eq!(codec.feed(&report(value)), Err(ChatError::Protocol));
        assert_eq!(codec.usage(), None);
        assert_eq!(codec.end(), Err(ChatError::Failed));
    }
    for body in [
        [text.clone(), report(valid.clone()), finish.clone()].concat(),
        [
            text.clone(),
            finish.clone(),
            report(valid.clone()),
            report(valid.clone()),
        ]
        .concat(),
    ] {
        let mut codec = ChatStream::with_usage("test-model", 4).unwrap();
        assert_eq!(codec.feed(&body), Err(ChatError::Protocol));
        assert_eq!(codec.usage(), None);
    }
    let mut codec = ChatStream::with_usage("test-model", 4).unwrap();
    codec.feed(&[text, finish, report(valid)].concat()).unwrap();
    assert_eq!(codec.end(), Err(ChatError::Incomplete));
    assert_eq!(codec.usage(), None);
}

#[test]
fn provider_usage_can_be_absent_or_part_of_the_finish_chunk() {
    for usage in [
        None,
        Some(json!({"prompt_tokens":0,"completion_tokens":0,"total_tokens":0})),
    ] {
        let mut finish = chunk(json!({"content":"answer"}), json!("length"));
        if let Some(value) = usage.clone() {
            finish["usage"] = value;
        }
        let mut codec = ChatStream::with_usage("test-model", 1).unwrap();
        codec
            .feed(&[frame(finish), b"data: [DONE]\n\n".to_vec()].concat())
            .unwrap();
        codec.end().unwrap();
        assert_eq!(
            serde_json::to_value(codec.usage()).unwrap(),
            usage.unwrap_or(Value::Null)
        );
    }
    for ceiling in [0, 32769, u32::MAX] {
        assert!(matches!(
            ChatStream::with_usage("test-model", ceiling),
            Err(ChatError::Request)
        ));
    }
}
