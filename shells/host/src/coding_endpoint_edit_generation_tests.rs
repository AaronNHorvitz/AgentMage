// SPDX-License-Identifier: Apache-2.0
//! Synthetic model socket/creation; original phase, parser and native writer.
use super::*;
use crate::coding_endpoint::CodingEndpointConnection;
use crate::coding_endpoint_edit_generation::CodingEndpointGenerationError as GenerationError;
use agentmage_kernel_contracts::{ModelOperationControl, ModelOperationStop};
use agentmage_kernel_engine::runtime_loop::{
    RuntimePortFailure, endpoint::RuntimeEndpointPhaseFailure,
};
use agentmage_platform_linux_inference::{
    local_chat::LocalChatEndpoint,
    local_chat_transport::{
        LocalChatApproval, LocalChatTransport, TransportError, TransportLimits,
    },
};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    num::NonZeroU64,
    sync::atomic::AtomicBool,
    thread,
    time::{Duration, Instant},
};

#[path = "coding_endpoint_edit_coordinator_tests.rs"]
mod coordinator_tests;

struct Control(AtomicBool);
impl Control {
    fn new() -> Self {
        Self(AtomicBool::new(false))
    }
}
impl ModelOperationControl for Control {
    fn remaining_ms(&self) -> Result<NonZeroU64, ModelOperationStop> {
        if self.0.load(Ordering::SeqCst) {
            Err(ModelOperationStop::Cancelled)
        } else {
            Ok(NonZeroU64::new(2000).unwrap())
        }
    }
}
fn socket() -> (TcpListener, LocalChatEndpoint) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint =
        LocalChatEndpoint::parse(&format!("http://{}/v1", listener.local_addr().unwrap())).unwrap();
    (listener, endpoint)
}
fn accept(listener: &TcpListener) -> TcpStream {
    let started = Instant::now();
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                return stream;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(started.elapsed() < Duration::from_secs(5));
                thread::sleep(Duration::from_millis(2));
            }
            Err(e) => panic!("{e}"),
        }
    }
}
fn body(stream: &mut TcpStream) -> Value {
    let mut header = vec![];
    while !header.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        stream.read_exact(&mut byte).unwrap();
        header.push(byte[0]);
        assert!(header.len() < 8192);
    }
    let header = String::from_utf8(header).unwrap();
    assert!(header.starts_with("POST /v1/chat/completions HTTP/1.1\r\n"));
    let len = header
        .lines()
        .find_map(|l| l.strip_prefix("Content-Length: "))
        .unwrap()
        .parse::<usize>()
        .unwrap();
    assert!(len < 16384);
    let mut bytes = vec![0; len];
    stream.read_exact(&mut bytes).unwrap();
    serde_json::from_slice(&bytes).unwrap()
}
fn respond(stream: &mut TcpStream, answer: &str, finish: &str, complete: bool) {
    let mut wire = String::new();
    for fragment in answer.as_bytes().chunks(8192) {
        wire.push_str(&format!("data: {}\n\n",json!({"model":"synthetic-model","choices":[{"index":0,"delta":{"content":std::str::from_utf8(fragment).unwrap()},"finish_reason":null}]})));
    }
    wire.push_str(&format!(
        "data: {}\n\n",
        json!({"model":"synthetic-model","choices":[{"index":0,"delta":{},"finish_reason":finish}]})
    ));
    if complete {
        wire.push_str("data: {\"model\":\"synthetic-model\",\"choices\":[],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":12,\"total_tokens\":22}}\n\ndata: [DONE]\n\n");
    }
    // Cancellation can close a socket before the synthetic peer finishes.
    let result=stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{wire}",wire.len()).as_bytes());
    if let Err(e) = result {
        assert!(matches!(
            e.kind(),
            std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset
        ));
    }
}
fn configured_transport(endpoint: LocalChatEndpoint) -> Arc<LocalChatTransport> {
    Arc::new(
        LocalChatTransport::new(
            endpoint,
            None,
            TransportLimits {
                total: Duration::from_secs(2),
                idle: Duration::from_secs(1),
            },
        )
        .unwrap(),
    )
}
fn generation_fixture() -> Fixture {
    let mut f = Fixture::new();
    f.request.workspace_id = WorkspaceId::from_raw("local-coding-worktree-create");
    f.request = seal_runtime_endpoint_run_request(f.request.clone()).unwrap();
    f
}
fn generation_startup(
    f: &Fixture,
    endpoint: &LocalChatEndpoint,
    ledger: &InstructionEvidenceLedger,
) -> Startup {
    let mut request = controlled(f, Kind::Edit);
    request.work_packet.mutable_files = vec!["one.py".into(), "two.py".into()];
    request.work_packet.protected_files = vec!["private".into()];
    request.model_selection =
        configured_transport(endpoint.clone()).model_selection("synthetic-model");
    request.limits.max_output_bytes = 2 * 1024 * 1024;
    for budget in &mut request.work_packet.budgets {
        if matches!(
            budget.resource,
            BudgetResource::OutputBytes | BudgetResource::DiskBytes
        ) {
            budget.limit = request.limits.max_output_bytes;
        }
    }
    Startup::prepare(
        seal_runtime_endpoint_run_request(request).unwrap(),
        Kind::Edit,
        ledger,
    )
    .unwrap()
}
fn context(
    startup: &Startup,
) -> agentmage_capability_repository_map::coding_proposal::CodingProposalContext {
    proposal_context(
        &startup.request().task.objective,
        &[("one.py", "one\n"), ("two.py", "two\n")],
    )
}
fn connection(
    transport: Arc<LocalChatTransport>,
    endpoint: LocalChatEndpoint,
) -> CodingEndpointConnection<'static> {
    CodingEndpointConnection::Shared {
        transport,
        approval: Arc::new(LocalChatApproval::for_endpoint(endpoint)),
    }
}
fn good() -> String {
    json!({"edits":[edit("one.py","one","first")]}).to_string()
}

#[test]
fn endpoint_strict_generation_streams_original_context_then_requires_native_approval() {
    let f = generation_fixture();
    let mut owner = owner(&f);
    let (listener, endpoint) = socket();
    let startup = generation_startup(&f, &endpoint, &f.empty_ledger());
    let context = context(&startup);
    let expected = context.user_message().to_owned();
    let answer = good();
    let expected_answer = answer.clone();
    let peer = thread::spawn(move || {
        let mut stream = accept(&listener);
        let body = body(&mut stream);
        assert_eq!(
            body["messages"],
            json!([
                {"role":"system","content":agentmage_capability_repository_map::coding_proposal::CODING_PROPOSAL_INSTRUCTION},
                {"role":"user","content":expected}
            ])
        );
        assert_eq!(body["model"], "synthetic-model");
        assert_eq!(body["max_tokens"], 128);
        assert_eq!(body["stream_options"]["include_usage"], true);
        respond(&mut stream, &answer, "stop", true);
    });
    let generation = startup
        .prepare_edit_generation(
            connection(configured_transport(endpoint.clone()), endpoint),
            context,
            true,
        )
        .unwrap();
    let binding = generation.binding().clone();
    let control = Control::new();
    let mut streamed = String::new();
    let generated = generation.execute(&control, |s| streamed.push_str(s));
    peer.join().unwrap();
    assert_eq!(streamed, expected_answer);
    assert_eq!(generated.result().as_ref().unwrap().request, binding);
    assert!(
        generated
            .result()
            .as_ref()
            .unwrap()
            .token_accounting
            .provider_reported()
            .is_some()
    );
    assert_eq!(generated.usage().dispatch_attempts, 1);
    assert_eq!(
        generated.usage().prepared_body_bytes,
        binding.body().request_body_bytes as u64
    );
    assert_eq!(
        generated.usage().completed_response_text_bytes,
        Some(streamed.len() as u64)
    );
    assert!(owner.inspect_edit().unwrap()["pending"].is_null());
    let mut changed_request = startup.request().clone();
    changed_request
        .task
        .constraints
        .push("Separate sealed startup".into());
    let changed = Startup::prepare(
        seal_runtime_endpoint_run_request(changed_request).unwrap(),
        Kind::Edit,
        &f.empty_ledger(),
    )
    .unwrap();
    assert_eq!(
        generated.prepare_preview(&changed, &mut owner, &control, 1000),
        Err(GenerationError::Scope)
    );
    assert!(owner.inspect_edit().unwrap()["pending"].is_null());
    let preview = generated
        .prepare_preview(&startup, &mut owner, &control, 1000)
        .unwrap();
    assert_eq!(
        fs::read(f.root.join("owned/worktree/one.py")).unwrap(),
        b"one\n"
    );
    assert_eq!(
        owner
            .decide_edit(preview["preview_sha256"].as_str().unwrap(), true, 1001)
            .unwrap()["outcome"],
        "committed"
    );
    assert_eq!(
        fs::read(f.root.join("owned/worktree/one.py")).unwrap(),
        b"first\n"
    );
    assert_eq!(
        fs::read(f.root.join("owned/worktree/two.py")).unwrap(),
        b"two\n"
    );
    assert!(
        owner
            .decide_edit(preview["preview_sha256"].as_str().unwrap(), true, 1002)
            .is_err()
    );
    assert_eq!(
        owner.inspect_apply().unwrap()["available_paths"],
        json!(["one.py"])
    );
}

#[test]
fn endpoint_strict_generation_refuses_scope_destination_and_phase_limits_without_socket() {
    let f = generation_fixture();
    let (listener, endpoint) = socket();
    let transport = configured_transport(endpoint.clone());
    let startup = generation_startup(&f, &endpoint, &f.empty_ledger());
    let foreign = proposal_context("foreign objective", &[("one.py", "one\n")]);
    assert_eq!(
        startup
            .prepare_edit_generation(
                connection(transport.clone(), endpoint.clone()),
                foreign,
                true
            )
            .err(),
        Some(GenerationError::Scope)
    );
    let ledger = f.ledger(vec![constraint(
        GuidanceConstraintKind::ExcludeScope,
        "private",
    )]);
    let narrowed = generation_startup(&f, &endpoint, &ledger);
    let excluded = proposal_context(
        &narrowed.request().task.objective,
        &[("private/secret.py", "secret\n")],
    );
    assert_eq!(
        narrowed
            .prepare_edit_generation(
                connection(transport.clone(), endpoint.clone()),
                excluded,
                true
            )
            .err(),
        Some(GenerationError::Scope)
    );
    let other = LocalChatEndpoint::parse("http://127.0.0.1:1/v1").unwrap();
    assert_eq!(
        startup
            .prepare_edit_generation(
                connection(transport.clone(), other.clone()),
                context(&startup),
                true
            )
            .err(),
        Some(GenerationError::Preparation(RuntimePortFailure::Invalid))
    );
    assert_eq!(
        startup
            .prepare_edit_generation(
                connection(configured_transport(other), endpoint.clone()),
                context(&startup),
                true
            )
            .err(),
        Some(GenerationError::Preparation(RuntimePortFailure::Invalid))
    );
    let mut request = startup.request().clone();
    request.context_budget.max_content_bytes = 1;
    // Keep structurally valid byte ceilings, while admitting no complete prompt.
    let small = Startup::prepare(
        seal_runtime_endpoint_run_request(request).unwrap(),
        Kind::Edit,
        &f.empty_ledger(),
    )
    .unwrap();
    assert_eq!(
        small
            .prepare_edit_generation(
                connection(transport.clone(), endpoint.clone()),
                context(&small),
                true
            )
            .err(),
        Some(GenerationError::Preparation(
            RuntimePortFailure::ResourceExhausted
        ))
    );
    let mut small_request = startup.request().clone();
    small_request.limits.max_output_bytes = 4096;
    let small = Startup::prepare(
        seal_runtime_endpoint_run_request(small_request).unwrap(),
        Kind::Edit,
        &f.empty_ledger(),
    )
    .unwrap();
    let prepared = small
        .prepare_edit_generation(
            connection(transport.clone(), endpoint.clone()),
            context(&small),
            true,
        )
        .unwrap();
    assert_eq!(prepared.binding().limits().response_text_limit_bytes, 4096);
    drop(prepared);
    let command = Startup::prepare(
        controlled(&f, Kind::Command),
        Kind::Command,
        &f.empty_ledger(),
    )
    .unwrap();
    assert_eq!(
        command
            .prepare_edit_generation(connection(transport, endpoint), context(&command), true)
            .err(),
        Some(GenerationError::Scope)
    );
    assert!(matches!(listener.accept(),Err(e) if e.kind()==std::io::ErrorKind::WouldBlock));
}

#[test]
fn endpoint_strict_generation_keeps_usage_and_prior_preview_on_invalid_incomplete_or_stale_answer()
{
    for case in 0..5 {
        let f = generation_fixture();
        let mut owner = owner(&f);
        let (listener, endpoint) = socket();
        let startup = generation_startup(&f, &endpoint, &f.empty_ledger());
        let prior = startup
            .prepare_edit_proposal(&mut owner, &context(&startup), &good(), 1000)
            .unwrap();
        let answer = if case == 0 {
            "{\"edits\":[],\"edits\":[]}".into()
        } else if case == 4 {
            "x".repeat(65537)
        } else {
            good()
        };
        let peer = thread::spawn(move || {
            let mut stream = accept(&listener);
            body(&mut stream);
            respond(
                &mut stream,
                &answer,
                if case == 1 { "length" } else { "stop" },
                case != 2,
            );
        });
        // Exercise borrowed configuration as well as retained shared transport.
        let transport = configured_transport(endpoint.clone());
        let approval = LocalChatApproval::for_endpoint(endpoint);
        let control = Control::new();
        let generated = startup
            .prepare_edit_generation(
                CodingEndpointConnection::Borrowed {
                    transport: &transport,
                    approval: &approval,
                },
                context(&startup),
                true,
            )
            .unwrap()
            .execute(&control, |_| {});
        peer.join().unwrap();
        if case == 3 {
            fs::write(
                f.root.join("owned/worktree/two.py"),
                "changed unedited source\n",
            )
            .unwrap();
        }
        let result = generated.prepare_preview(&startup, &mut owner, &control, 1001);
        match case {
            0 => assert_eq!(
                result,
                Err(GenerationError::Proposal(Refusal::Proposal(
                    CodingProposalError::Format
                )))
            ),
            3 => assert_eq!(
                result,
                Err(GenerationError::Proposal(Refusal::Proposal(
                    CodingProposalError::Stale
                )))
            ),
            _ => assert_eq!(result, Err(GenerationError::Incomplete)),
        }
        assert_eq!(generated.usage().dispatch_attempts, 1);
        assert_eq!(generated.result().is_ok(), matches!(case, 0 | 1 | 3));
        assert_eq!(
            generated.usage().completed_response_text_bytes.is_some(),
            matches!(case, 0 | 1 | 3)
        );
        assert_eq!(owner.inspect_edit().unwrap()["pending"], prior);
        assert_eq!(
            fs::read(f.root.join("owned/worktree/one.py")).unwrap(),
            b"one\n"
        );
    }
}

#[test]
fn endpoint_strict_generation_original_control_stops_dispatch_callbacks_and_preview() {
    let f = generation_fixture();
    let mut owner = owner(&f);
    let (listener, endpoint) = socket();
    let startup = generation_startup(&f, &endpoint, &f.empty_ledger());
    let transport = configured_transport(endpoint.clone());
    let control = Control::new();
    control.0.store(true, Ordering::SeqCst);
    let stopped = startup
        .prepare_edit_generation(
            connection(transport.clone(), endpoint.clone()),
            context(&startup),
            true,
        )
        .unwrap()
        .execute(&control, |_| panic!("stopped"));
    assert_eq!(stopped.usage().dispatch_attempts, 0);
    assert!(matches!(
        stopped.result(),
        Err(RuntimeEndpointPhaseFailure::Stopped(
            ModelOperationStop::Cancelled
        ))
    ));
    assert!(matches!(listener.accept(),Err(e) if e.kind()==std::io::ErrorKind::WouldBlock));
    for cancel_in_callback in [true, false] {
        let (listener, endpoint) = socket();
        let startup = generation_startup(&f, &endpoint, &f.empty_ledger());
        let peer = thread::spawn(move || {
            let mut stream = accept(&listener);
            body(&mut stream);
            respond(&mut stream, &good(), "stop", true);
        });
        let control = Control::new();
        let transport = configured_transport(endpoint.clone());
        let generated = startup
            .prepare_edit_generation(connection(transport, endpoint), context(&startup), true)
            .unwrap()
            .execute(&control, |_| {
                if cancel_in_callback {
                    control.0.store(true, Ordering::SeqCst);
                }
            });
        peer.join().unwrap();
        control.0.store(true, Ordering::SeqCst);
        if cancel_in_callback {
            assert!(matches!(
                generated.result(),
                Err(RuntimeEndpointPhaseFailure::Dependency(
                    TransportError::OperationStopped(ModelOperationStop::Cancelled)
                ))
            ));
        } else {
            assert!(generated.result().is_ok());
        }
        assert_eq!(
            generated.prepare_preview(&startup, &mut owner, &control, 1000),
            Err(if cancel_in_callback {
                GenerationError::Incomplete
            } else {
                GenerationError::Preparation(RuntimePortFailure::Cancelled)
            })
        );
        assert!(owner.inspect_edit().unwrap()["pending"].is_null());
    }
}

#[test]
fn endpoint_strict_generation_uses_run_text_ceiling_for_preview_and_refuses_overflow() {
    for ceiling in [4096, 64 * 1024, 2 * 1024 * 1024] {
        for shared in [false, true] {
            for overflow in [false, true] {
                let f = generation_fixture();
                let mut repository = owner(&f);
                let (listener, endpoint) = socket();
                let startup = generation_startup(&f, &endpoint, &f.empty_ledger());
                let mut request = startup.request().clone();
                request.limits.max_output_bytes = ceiling;
                let startup = Startup::prepare(
                    seal_runtime_endpoint_run_request(request).unwrap(),
                    Kind::Edit,
                    &f.empty_ledger(),
                )
                .unwrap();
                let prior = startup
                    .prepare_edit_proposal(&mut repository, &context(&startup), &good(), 1000)
                    .unwrap();
                let cap = ceiling.min(64 * 1024) as usize;
                let mut answer = good();
                answer.extend(std::iter::repeat_n(
                    ' ',
                    cap + usize::from(overflow) - answer.len(),
                ));
                let expected_answer = answer.clone();
                let peer = thread::spawn(move || {
                    let mut stream = accept(&listener);
                    body(&mut stream);
                    respond(&mut stream, &answer, "stop", true);
                });
                let transport = configured_transport(endpoint.clone());
                let approval = LocalChatApproval::for_endpoint(endpoint.clone());
                let connection = if shared {
                    connection(transport.clone(), endpoint)
                } else {
                    CodingEndpointConnection::Borrowed {
                        transport: &transport,
                        approval: &approval,
                    }
                };
                let generation = startup
                    .prepare_edit_generation(connection, context(&startup), true)
                    .unwrap();
                assert_eq!(generation.binding().limits().response_text_limit_bytes, cap);
                let mut text = String::new();
                let control = Control::new();
                let generated = generation.execute(&control, |part| text.push_str(part));
                peer.join().unwrap();
                assert_eq!(generated.usage().dispatch_attempts, 1);
                if overflow {
                    assert!(matches!(
                        generated.result(),
                        Err(RuntimeEndpointPhaseFailure::Dependency(
                            TransportError::Limit
                        ))
                    ));
                    assert!(text.len() <= cap);
                    assert_eq!(generated.usage().completed_response_text_bytes, None);
                    assert_eq!(generated.usage().completed_dispatch_elapsed_ms, None);
                    assert_eq!(
                        generated.prepare_preview(&startup, &mut repository, &control, 1001),
                        Err(GenerationError::Incomplete)
                    );
                    assert_eq!(repository.inspect_edit().unwrap()["pending"], prior);
                } else {
                    assert!(generated.result().is_ok());
                    assert_eq!(text, expected_answer);
                    assert_eq!(
                        generated.usage().completed_response_text_bytes,
                        Some(cap as u64)
                    );
                    let preview = generated
                        .prepare_preview(&startup, &mut repository, &control, 1001)
                        .unwrap();
                    assert_eq!(
                        preview["write_preview"]["files"].as_array().unwrap().len(),
                        1
                    );
                }
                assert_eq!(
                    fs::read(f.root.join("owned/worktree/one.py")).unwrap(),
                    b"one\n"
                );
            }
        }
    }
}

#[test]
fn endpoint_strict_generation_foreign_workspace_preserves_usage_and_original_preview() {
    let f = generation_fixture();
    let mut repository = owner(&f);
    let (listener, endpoint) = socket();
    let startup = generation_startup(&f, &endpoint, &f.empty_ledger());
    let prior = startup
        .prepare_edit_proposal(&mut repository, &context(&startup), &good(), 1000)
        .unwrap();
    let mut request = startup.request().clone();
    request.workspace_id = WorkspaceId::from_raw("foreign-workspace");
    let foreign = Startup::prepare(
        seal_runtime_endpoint_run_request(request).unwrap(),
        Kind::Edit,
        &f.empty_ledger(),
    )
    .unwrap();
    let peer = thread::spawn(move || {
        let mut stream = accept(&listener);
        body(&mut stream);
        respond(&mut stream, &good(), "stop", true);
    });
    let control = Control::new();
    // Generation itself uses a prior-consented snapshot; it has no native
    // owner and makes no claim that disclosure proves repository ownership.
    let generated = foreign
        .prepare_edit_generation(
            connection(configured_transport(endpoint.clone()), endpoint),
            context(&foreign),
            true,
        )
        .unwrap()
        .execute(&control, |_| {});
    peer.join().unwrap();
    assert!(generated.result().is_ok());
    let usage = generated.usage().clone();
    for unreadable in [false, true] {
        if unreadable {
            fs::remove_file(f.root.join("owned/worktree/one.py")).unwrap();
        }
        assert_eq!(
            generated.prepare_preview(&foreign, &mut repository, &control, 1001),
            Err(GenerationError::Proposal(Refusal::Scope))
        );
        assert_eq!(generated.usage(), &usage);
        assert_eq!(generated.usage().dispatch_attempts, 1);
        assert!(generated.usage().completed_response_text_bytes.is_some());
        assert_eq!(repository.inspect_edit().unwrap()["pending"], prior);
        assert!(repository.inspect_edit().unwrap()["latest"].is_null());
    }
}
