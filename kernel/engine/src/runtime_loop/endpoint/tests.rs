// SPDX-License-Identifier: MIT OR Apache-2.0
//! Synthetic ports exercise phase admission; no inference or native identity.

use std::cell::Cell;
use std::num::NonZeroU64;
use std::time::Duration;

use agentmage_kernel_contracts::{
    EndpointModelBody, EndpointModelFinish, EndpointModelLimits, EndpointModelSelection,
    ProviderTokenUsage,
};

use super::*;

struct Control(Cell<Result<NonZeroU64, ModelOperationStop>>);
impl ModelOperationControl for Control {
    fn remaining_ms(&self) -> Result<NonZeroU64, ModelOperationStop> {
        self.0.get()
    }
}
fn live() -> Control {
    Control(Cell::new(Ok(NonZeroU64::new(100).unwrap())))
}
fn context() -> RuntimeEndpointContextPacket {
    RuntimeEndpointContextPacket::from_messages(&[
        RuntimeEndpointMessage {
            role: RuntimeEndpointMessageRole::System,
            content: "instructions",
        },
        RuntimeEndpointMessage {
            role: RuntimeEndpointMessageRole::User,
            content: "task",
        },
    ])
    .unwrap()
}
fn request() -> EndpointModelRequest {
    EndpointModelRequest::from_prepared_body(
        EndpointModelSelection {
            endpoint: "http://127.0.0.1:1234/v1".into(),
            model: "synthetic".into(),
        },
        context().messages().clone(),
        EndpointModelBody {
            request_body_bytes: 200,
            request_body_sha256: "b".repeat(64),
        },
        EndpointModelLimits {
            requested_max_output_tokens: 10,
            provider_usage_requested: true,
            total_timeout: Duration::from_millis(100),
            idle_timeout: Duration::from_millis(50),
            response_wire_limit_bytes: 1000,
            response_body_limit_bytes: 500,
            response_text_limit_bytes: 100,
        },
    )
}
fn completion(request: EndpointModelRequest) -> EndpointModelResult {
    EndpointModelResult::from_completed_response(
        request,
        EndpointModelFinish::Stop,
        text_hash(&"x".repeat(50)),
        50,
        Duration::from_micros(1500),
        None,
    )
}

fn text_hash(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

// Adapt the earlier descriptor-only fixtures to a synthetic streaming port.
// New stream-specific tests below supply independent fragments and descriptors.
struct ClosurePort<F> {
    request: EndpointModelRequest,
    dispatch: F,
}
impl<E, F> RuntimeEndpointModelPort for ClosurePort<F>
where
    F: FnOnce(&dyn ModelOperationControl) -> Result<EndpointModelResult, E>,
{
    type Error = E;
    fn request(&self) -> &EndpointModelRequest {
        &self.request
    }
    fn dispatch(
        self,
        control: &dyn ModelOperationControl,
        text: &mut dyn FnMut(&str),
    ) -> Result<EndpointModelResult, E> {
        let result = (self.dispatch)(control)?;
        text(&"x".repeat(result.response_text_bytes));
        Ok(result)
    }
}
fn run_phase<E>(
    phase: &mut RuntimeEndpointModelPhase,
    control: &dyn ModelOperationControl,
    dispatch: impl FnOnce(&dyn ModelOperationControl) -> Result<EndpointModelResult, E>,
) -> Result<EndpointModelResult, RuntimeEndpointPhaseFailure<E>> {
    let request = phase.request.clone();
    phase.execute(control, ClosurePort { request, dispatch }, |_| {})
}

#[test]
fn endpoint_phase_accounts_one_frozen_dispatch_without_native_tokens_or_memory() {
    for usage in [
        None,
        Some(ProviderTokenUsage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        }),
        Some(ProviderTokenUsage {
            prompt_tokens: 20,
            completion_tokens: 5,
            total_tokens: 25,
        }),
    ] {
        for finish in [
            EndpointModelFinish::Stop,
            EndpointModelFinish::OutputTokenLimit,
        ] {
            let request = request();
            let mut phase = RuntimeEndpointModelPhase::new(&request, context()).unwrap();
            assert_eq!(phase.usage().dispatch_attempts, 0);
            assert_eq!(phase.usage().completed_response_text_bytes, None);
            let result = EndpointModelResult::from_completed_response(
                request.clone(),
                finish,
                text_hash(&"x".repeat(100)),
                100,
                Duration::from_millis(100),
                usage,
            );
            let expected = result.clone();
            let result = run_phase(&mut phase, &live(), |control| {
                assert_eq!(control.remaining_ms().unwrap().get(), 100);
                Ok::<_, ()>(result)
            })
            .unwrap();
            assert_eq!(result, expected);
            assert_eq!(
                phase.usage(),
                RuntimeEndpointPhaseUsage {
                    dispatch_attempts: 1,
                    prepared_body_bytes: 200,
                    completed_response_text_bytes: Some(100),
                    completed_dispatch_elapsed_ms: Some(100),
                }
            );
            let json = serde_json::to_value(&result).unwrap();
            assert!(json["resources"]["resident_memory_bytes"].is_null());
            assert!(json["resources"]["accelerator_memory_bytes"].is_null());
            assert!(json["token_accounting"]["verified_input_tokens"].is_null());
            assert!(json["token_accounting"]["verified_output_tokens"].is_null());
            assert_eq!(result.token_accounting.provider_reported(), usage);
            let calls = Cell::new(0);
            assert_eq!(
                run_phase(&mut phase, &live(), |_| {
                    calls.set(calls.get() + 1);
                    Ok::<_, ()>(completion(request))
                }),
                Err(RuntimeEndpointPhaseFailure::ResourceExhausted)
            );
            assert_eq!(calls.get(), 0);
        }
    }
}

#[test]
fn endpoint_phase_stops_before_dispatch_and_never_reopens_a_spent_phase() {
    for stop in [
        ModelOperationStop::Cancelled,
        ModelOperationStop::TimedOut,
        ModelOperationStop::Unavailable,
        ModelOperationStop::Invalid,
    ] {
        let mut phase = RuntimeEndpointModelPhase::new(&request(), context()).unwrap();
        let calls = Cell::new(0);
        let control = Control(Cell::new(Err(stop)));
        assert_eq!(
            run_phase(&mut phase, &control, |_| {
                calls.set(calls.get() + 1);
                Ok::<_, ()>(completion(request()))
            }),
            Err(RuntimeEndpointPhaseFailure::Stopped(stop))
        );
        assert_eq!(calls.get(), 0);
        assert_eq!(
            phase.usage(),
            RuntimeEndpointPhaseUsage {
                dispatch_attempts: 0,
                prepared_body_bytes: 0,
                completed_response_text_bytes: None,
                completed_dispatch_elapsed_ms: None,
            }
        );
        control.0.set(live().0.get());
        assert_eq!(
            run_phase(&mut phase, &control, |_| {
                calls.set(calls.get() + 1);
                Ok::<_, ()>(completion(request()))
            }),
            Err(RuntimeEndpointPhaseFailure::ResourceExhausted)
        );
        assert_eq!(calls.get(), 0);
    }
}

#[test]
fn endpoint_phase_retains_failed_attempts_and_refuses_late_completions() {
    for stop in [
        None,
        Some(ModelOperationStop::TimedOut),
        Some(ModelOperationStop::Cancelled),
    ] {
        let mut phase = RuntimeEndpointModelPhase::new(&request(), context()).unwrap();
        let control = live();
        let result = run_phase(&mut phase, &control, |_| {
            if let Some(stop) = stop {
                control.0.set(Err(stop));
                Ok(completion(request()))
            } else {
                Err("synthetic-dependency-refusal")
            }
        });
        assert_eq!(
            result,
            Err(match stop {
                Some(stop) => RuntimeEndpointPhaseFailure::Stopped(stop),
                None => RuntimeEndpointPhaseFailure::Dependency("synthetic-dependency-refusal"),
            })
        );
        assert_eq!(
            phase.usage(),
            RuntimeEndpointPhaseUsage {
                dispatch_attempts: 1,
                prepared_body_bytes: 200,
                completed_response_text_bytes: None,
                completed_dispatch_elapsed_ms: None,
            }
        );
        assert_eq!(
            run_phase(&mut phase, &live(), |_| Ok::<_, ()>(completion(request()))),
            Err(RuntimeEndpointPhaseFailure::ResourceExhausted)
        );
    }
}

#[test]
fn endpoint_phase_rejects_mismatched_result_bindings_and_bounds_atomically() {
    for case in 0..9 {
        let mut phase = RuntimeEndpointModelPhase::new(&request(), context()).unwrap();
        let mut result = completion(request());
        match case {
            0 => result.request.context.serialized_messages_sha256 = "d".repeat(64),
            1 => result.request.context.serialized_messages_bytes += 1,
            2 => result.request.context.message_count += 1,
            3 => result.response_text_sha256 = "invalid".into(),
            4 => result.resources.dispatch_elapsed = Duration::from_micros(100_001),
            5 => result.response_text_bytes = 101,
            6 => {
                result.request = EndpointModelRequest::from_prepared_body(
                    EndpointModelSelection {
                        endpoint: "http://127.0.0.1:5678/v1".into(),
                        model: "other".into(),
                    },
                    result.request.context.clone(),
                    result.request.body().clone(),
                    result.request.limits().clone(),
                )
            }
            7 => {
                let mut body = result.request.body().clone();
                body.request_body_sha256 = "d".repeat(64);
                result.request = EndpointModelRequest::from_prepared_body(
                    EndpointModelSelection {
                        endpoint: "http://127.0.0.1:1234/v1".into(),
                        model: "synthetic".into(),
                    },
                    result.request.context.clone(),
                    body,
                    result.request.limits().clone(),
                );
            }
            8 => {
                let mut limits = result.request.limits().clone();
                limits.requested_max_output_tokens += 1;
                result.request = EndpointModelRequest::from_prepared_body(
                    EndpointModelSelection {
                        endpoint: "http://127.0.0.1:1234/v1".into(),
                        model: "synthetic".into(),
                    },
                    result.request.context.clone(),
                    result.request.body().clone(),
                    limits,
                );
            }
            _ => unreachable!(),
        }
        assert_eq!(
            run_phase(&mut phase, &live(), |_| Ok::<_, ()>(result)),
            Err(if case == 5 {
                RuntimeEndpointPhaseFailure::ResourceExhausted
            } else {
                RuntimeEndpointPhaseFailure::InvalidResult
            })
        );
        assert_eq!(phase.usage().dispatch_attempts, 1);
        assert_eq!(phase.usage().completed_response_text_bytes, None);
        assert_eq!(phase.usage().completed_dispatch_elapsed_ms, None);
        assert_eq!(phase.resources.durable_usage().output_bytes, 0);
        assert_eq!(phase.resources.durable_usage().elapsed_ms, 0);
    }
}

#[test]
fn endpoint_phase_rounds_observed_time_and_preserves_zero_length_completions() {
    let mut phase = RuntimeEndpointModelPhase::new(&request(), context()).unwrap();
    let mut result = completion(request());
    result.response_text_bytes = 0;
    result.response_text_sha256 = text_hash("");
    run_phase(&mut phase, &live(), |_| Ok::<_, ()>(result)).unwrap();
    assert_eq!(phase.usage().completed_response_text_bytes, Some(0));
    assert_eq!(phase.usage().completed_dispatch_elapsed_ms, Some(2));
    let mut resources = RuntimeResourceLedger::for_endpoint_model(&request()).unwrap();
    assert!(resources.admit_event(1).is_err());
    assert!(resources.admit_artifact(1).is_err());
    assert!(resources.record_retry().is_err());
}

#[test]
fn endpoint_phase_refuses_malformed_context_and_resource_envelopes_before_dispatch() {
    for case in 0..12 {
        let request = request();
        let mut context = request.context.clone();
        let mut body = request.body().clone();
        let mut limits = request.limits().clone();
        match case {
            0 => context.message_count = 0,
            1 => context.serialized_messages_bytes = 0,
            2 => context.serialized_messages_bytes = body.request_body_bytes + 1,
            3 => context.serialized_messages_sha256 = "invalid".into(),
            4 => body.request_body_sha256 = "invalid".into(),
            5 => body.request_body_bytes = 0,
            6 => limits.requested_max_output_tokens = 0,
            7 => limits.total_timeout = Duration::ZERO,
            8 => limits.idle_timeout = Duration::ZERO,
            9 => limits.idle_timeout = limits.total_timeout + Duration::from_nanos(1),
            10 => limits.response_text_limit_bytes = 0,
            11 => limits.response_body_limit_bytes = limits.response_wire_limit_bytes + 1,
            _ => unreachable!(),
        }
        let malformed = EndpointModelRequest::from_prepared_body(
            EndpointModelSelection {
                endpoint: "http://127.0.0.1:1234/v1".into(),
                model: "synthetic".into(),
            },
            context,
            body,
            limits,
        );
        assert!(matches!(
            RuntimeEndpointModelPhase::new(&malformed, self::context()),
            Err(RuntimePortFailure::Invalid)
        ));
    }
}

struct StreamPort<F> {
    request: EndpointModelRequest,
    dispatch: F,
}
impl<E, F> RuntimeEndpointModelPort for StreamPort<F>
where
    F: FnOnce(&dyn ModelOperationControl, &mut dyn FnMut(&str)) -> Result<EndpointModelResult, E>,
{
    type Error = E;
    fn request(&self) -> &EndpointModelRequest {
        &self.request
    }
    fn dispatch(
        self,
        control: &dyn ModelOperationControl,
        text: &mut dyn FnMut(&str),
    ) -> Result<EndpointModelResult, E> {
        (self.dispatch)(control, text)
    }
}

fn streamed_result(text: &str) -> EndpointModelResult {
    EndpointModelResult::from_completed_response(
        request(),
        EndpointModelFinish::Stop,
        text_hash(text),
        text.len(),
        Duration::from_micros(1500),
        None,
    )
}

#[test]
fn endpoint_port_binds_unicode_streams_independently_of_fragmentation_and_finish() {
    for fragments in [vec!["", "é", "水", "\n", ""], vec!["é水\n"], vec!["", ""]] {
        let expected = fragments.concat();
        for finish in [
            EndpointModelFinish::Stop,
            EndpointModelFinish::OutputTokenLimit,
        ] {
            let mut phase = RuntimeEndpointModelPhase::new(&request(), context()).unwrap();
            let mut result = streamed_result(&expected);
            result.finish = finish;
            let expected_result = result.clone();
            let mut delivered = Vec::new();
            let result = phase
                .execute(
                    &live(),
                    StreamPort {
                        request: request(),
                        dispatch: |_: &dyn ModelOperationControl, text: &mut dyn FnMut(&str)| {
                            for fragment in &fragments {
                                text(fragment);
                            }
                            Ok::<_, ()>(result)
                        },
                    },
                    |text| delivered.push(text.to_owned()),
                )
                .unwrap();
            assert_eq!(result, expected_result);
            assert_eq!(delivered, fragments);
            assert_eq!(
                phase.usage().completed_response_text_bytes,
                Some(expected.len() as u64)
            );
        }
    }
}

#[test]
fn endpoint_port_refuses_stream_descriptor_disagreement_without_completed_accounting() {
    for case in 0..5 {
        let mut phase = RuntimeEndpointModelPhase::new(&request(), context()).unwrap();
        let mut result = streamed_result("éwater");
        match case {
            0 => result.response_text_sha256 = text_hash("éwaTer"),
            1 => result.response_text_bytes -= 1,
            2 => result.response_text_bytes += 1,
            3 | 4 => {}
            _ => unreachable!(),
        }
        let mut delivered = String::new();
        let output = phase.execute(
            &live(),
            StreamPort {
                request: request(),
                dispatch: |_: &dyn ModelOperationControl, text: &mut dyn FnMut(&str)| {
                    if case == 3 {
                        text("wateré");
                    } else if case != 4 {
                        text("é");
                        text("water");
                    }
                    Ok::<_, ()>(result)
                },
            },
            |text| delivered.push_str(text),
        );
        assert_eq!(output, Err(RuntimeEndpointPhaseFailure::InvalidResult));
        assert_eq!(
            delivered,
            match case {
                3 => "wateré",
                4 => "",
                _ => "éwater",
            }
        );
        assert_eq!(phase.usage().dispatch_attempts, 1);
        assert_eq!(phase.usage().completed_response_text_bytes, None);
        assert_eq!(phase.resources.durable_usage().output_bytes, 0);
        assert_eq!(phase.resources.durable_usage().elapsed_ms, 0);
    }
}

#[test]
fn endpoint_port_refuses_a_different_prepared_request_before_dispatch_and_spends_phase() {
    for case in 0..4 {
        let mut phase = RuntimeEndpointModelPhase::new(&request(), context()).unwrap();
        let mut other = request();
        let mut body = other.body().clone();
        let mut limits = other.limits().clone();
        let mut selection = EndpointModelSelection {
            endpoint: "http://127.0.0.1:1234/v1".into(),
            model: "synthetic".into(),
        };
        match case {
            0 => other.context.serialized_messages_sha256 = "d".repeat(64),
            1 => body.request_body_sha256 = "d".repeat(64),
            2 => limits.requested_max_output_tokens += 1,
            3 => selection.model = "different".into(),
            _ => unreachable!(),
        }
        other = EndpointModelRequest::from_prepared_body(selection, other.context, body, limits);
        let calls = Cell::new(0);
        let output = phase.execute(
            &live(),
            StreamPort {
                request: other,
                dispatch: |_: &dyn ModelOperationControl, _: &mut dyn FnMut(&str)| {
                    calls.set(calls.get() + 1);
                    Ok::<_, ()>(streamed_result(""))
                },
            },
            |_| panic!("a mismatched port must not deliver text"),
        );
        assert_eq!(output, Err(RuntimeEndpointPhaseFailure::InvalidResult));
        assert_eq!(calls.get(), 0);
        assert_eq!(phase.usage().dispatch_attempts, 0);
        assert_eq!(phase.usage().prepared_body_bytes, 0);
        assert_eq!(phase.usage().completed_response_text_bytes, None);
        assert_eq!(
            run_phase(&mut phase, &live(), |_| panic!("spent phase")),
            Err::<EndpointModelResult, RuntimeEndpointPhaseFailure<()>>(
                RuntimeEndpointPhaseFailure::ResourceExhausted
            )
        );
    }
}

#[test]
fn endpoint_port_caps_text_before_delivery_and_latches_overflow_across_later_fragments() {
    for exact in [true, false] {
        let mut phase = RuntimeEndpointModelPhase::new(&request(), context()).unwrap();
        let prefix = "x".repeat(if exact { 98 } else { 99 });
        let full = format!("{prefix}é");
        let mut delivered = String::new();
        let output = phase.execute(
            &live(),
            StreamPort {
                request: request(),
                dispatch: |_: &dyn ModelOperationControl, text: &mut dyn FnMut(&str)| {
                    text(&prefix);
                    text("é");
                    if !exact {
                        text("");
                        text("ignored after overflow");
                    }
                    Ok::<_, ()>(streamed_result(&full))
                },
            },
            |text| delivered.push_str(text),
        );
        assert_eq!(phase.usage().dispatch_attempts, 1);
        if exact {
            assert!(output.is_ok());
            assert_eq!(delivered, full);
            assert_eq!(phase.usage().completed_response_text_bytes, Some(100));
        } else {
            assert_eq!(output, Err(RuntimeEndpointPhaseFailure::ResourceExhausted));
            assert_eq!(delivered, prefix);
            assert_eq!(phase.usage().completed_response_text_bytes, None);
            assert_eq!(phase.resources.durable_usage().output_bytes, 0);
        }
    }
}

#[test]
fn endpoint_port_latches_control_stops_before_and_after_callbacks_even_if_port_recovers() {
    for stop in [
        ModelOperationStop::Cancelled,
        ModelOperationStop::TimedOut,
        ModelOperationStop::Unavailable,
        ModelOperationStop::Invalid,
    ] {
        for before in [true, false] {
            let control = live();
            let mut phase = RuntimeEndpointModelPhase::new(&request(), context()).unwrap();
            let mut delivered = Vec::new();
            let output = phase.execute(
                &control,
                StreamPort {
                    request: request(),
                    dispatch: |_: &dyn ModelOperationControl, text: &mut dyn FnMut(&str)| {
                        if before {
                            control.0.set(Err(stop));
                        }
                        text("first");
                        // Broken cooperative port ignores a stop, tries to renew it
                        // and returns a descriptor that would otherwise match.
                        control.0.set(live().0.get());
                        text("second");
                        Ok::<_, ()>(streamed_result("firstsecond"))
                    },
                },
                |text| {
                    delivered.push(text.to_owned());
                    control.0.set(Err(stop));
                },
            );
            assert_eq!(output, Err(RuntimeEndpointPhaseFailure::Stopped(stop)));
            assert_eq!(delivered, if before { vec![] } else { vec!["first"] });
            assert_eq!(phase.usage().dispatch_attempts, 1);
            assert_eq!(phase.usage().completed_response_text_bytes, None);
            assert_eq!(phase.usage().completed_dispatch_elapsed_ms, None);
        }
    }
}

#[test]
fn endpoint_port_dependency_failure_keeps_only_provisional_text_and_cannot_retry() {
    let mut phase = RuntimeEndpointModelPhase::new(&request(), context()).unwrap();
    let mut delivered = String::new();
    let output = phase.execute(
        &live(),
        StreamPort {
            request: request(),
            dispatch: |_: &dyn ModelOperationControl, text: &mut dyn FnMut(&str)| {
                text("provisional");
                Err::<EndpointModelResult, _>("synthetic incomplete EOF")
            },
        },
        |text| delivered.push_str(text),
    );
    assert_eq!(
        output,
        Err(RuntimeEndpointPhaseFailure::Dependency(
            "synthetic incomplete EOF"
        ))
    );
    assert_eq!(delivered, "provisional");
    assert_eq!(phase.usage().dispatch_attempts, 1);
    assert_eq!(phase.usage().completed_response_text_bytes, None);
    assert_eq!(phase.usage().completed_dispatch_elapsed_ms, None);
    assert_eq!(
        run_phase(&mut phase, &live(), |_| panic!("spent phase")),
        Err::<EndpointModelResult, RuntimeEndpointPhaseFailure<()>>(
            RuntimeEndpointPhaseFailure::ResourceExhausted
        )
    );
}

#[test]
fn endpoint_context_binds_original_roles_order_escaping_and_utf8_without_retaining_text() {
    let private = "private source: λ\n\"quoted\"\\\t".to_owned();
    let messages = [
        RuntimeEndpointMessage {
            role: RuntimeEndpointMessageRole::System,
            content: "instructions",
        },
        RuntimeEndpointMessage {
            role: RuntimeEndpointMessageRole::Assistant,
            content: "untrusted history",
        },
        RuntimeEndpointMessage {
            role: RuntimeEndpointMessageRole::User,
            content: &private,
        },
    ];
    let packet = RuntimeEndpointContextPacket::from_messages(&messages).unwrap();
    let wire = "[{\"role\":\"system\",\"content\":\"instructions\"},{\"role\":\"assistant\",\"content\":\"untrusted history\"},{\"role\":\"user\",\"content\":\"private source: λ\\n\\\"quoted\\\"\\\\\\t\"}]";
    assert_eq!(packet.messages().message_count, 3);
    assert_eq!(packet.messages().serialized_messages_bytes, wire.len());
    assert_eq!(
        packet.messages().serialized_messages_sha256,
        text_hash(wire)
    );
    assert_eq!(
        packet.content_bytes(),
        "instructions".len() + "untrusted history".len() + private.len()
    );
    drop(private);
    let value = serde_json::to_value(&packet).unwrap();
    assert_eq!(value["version"], 1);
    assert_eq!(
        value["messages"],
        serde_json::to_value(packet.messages()).unwrap()
    );
    assert_eq!(value["content_bytes"], packet.content_bytes());
    assert!(!value.to_string().contains("private source"));
    assert!(!format!("{packet:?}").contains("private source"));
    for absent in [
        "profile_id",
        "manifest_sha256",
        "input_tokens",
        "verified_input_tokens",
        "authority",
    ] {
        assert!(value.get(absent).is_none(), "{absent}");
    }
}

#[test]
fn endpoint_context_refuses_empty_messages_invalid_end_roles_and_excess_message_count() {
    assert_eq!(
        RuntimeEndpointContextPacket::from_messages(&[]),
        Err(RuntimePortFailure::Invalid)
    );
    for role in [
        RuntimeEndpointMessageRole::System,
        RuntimeEndpointMessageRole::Assistant,
    ] {
        assert_eq!(
            RuntimeEndpointContextPacket::from_messages(&[RuntimeEndpointMessage {
                role,
                content: "text"
            }]),
            Err(RuntimePortFailure::Invalid)
        );
    }
    for empty in [0, 1] {
        let mut messages = [
            RuntimeEndpointMessage {
                role: RuntimeEndpointMessageRole::System,
                content: "text",
            },
            RuntimeEndpointMessage {
                role: RuntimeEndpointMessageRole::User,
                content: "text",
            },
        ];
        messages[empty].content = "";
        assert_eq!(
            RuntimeEndpointContextPacket::from_messages(&messages),
            Err(RuntimePortFailure::Invalid)
        );
    }
    let mut messages = (0..256)
        .map(|_| RuntimeEndpointMessage {
            role: RuntimeEndpointMessageRole::User,
            content: "x",
        })
        .collect::<Vec<_>>();
    assert_eq!(
        RuntimeEndpointContextPacket::from_messages(&messages)
            .unwrap()
            .messages()
            .message_count,
        256
    );
    messages.push(RuntimeEndpointMessage {
        role: RuntimeEndpointMessageRole::User,
        content: "x",
    });
    assert_eq!(
        RuntimeEndpointContextPacket::from_messages(&messages),
        Err(RuntimePortFailure::Invalid)
    );
}

#[test]
fn endpoint_context_enforces_exact_serialized_byte_boundary_including_escaping() {
    let cap = 256 * 1024;
    let overhead = "[{\"role\":\"user\",\"content\":\"\"}]".len();
    let exact = "x".repeat(cap - overhead);
    fn make(text: &str) -> Result<RuntimeEndpointContextPacket, RuntimePortFailure> {
        RuntimeEndpointContextPacket::from_messages(&[RuntimeEndpointMessage {
            role: RuntimeEndpointMessageRole::User,
            content: text,
        }])
    }
    let packet = make(&exact).unwrap();
    assert_eq!(packet.messages().serialized_messages_bytes, cap);
    assert_eq!(packet.content_bytes(), cap - overhead);
    assert_eq!(
        make(&(exact + "x")),
        Err(RuntimePortFailure::ResourceExhausted)
    );
    // Content fits, but JSON escaping alone crosses the serialized bound.
    assert_eq!(
        make(&"\n".repeat(cap / 2)),
        Err(RuntimePortFailure::ResourceExhausted)
    );
    // Multibyte content counts UTF-8 bytes rather than characters.
    assert_eq!(
        make(&"λ".repeat(cap / 2)),
        Err(RuntimePortFailure::ResourceExhausted)
    );
    assert_eq!(
        make(&"x".repeat(cap + 1)),
        Err(RuntimePortFailure::ResourceExhausted)
    );
}

#[test]
fn endpoint_context_mismatch_refuses_phase_before_any_port_or_resource_admission() {
    let original = context();
    for case in 0..6 {
        let mut request = request();
        match case {
            0 => request.context.message_count += 1,
            1 => request.context.serialized_messages_bytes += 1,
            2 => request.context.serialized_messages_sha256 = "d".repeat(64),
            3..=5 => {
                let mut messages = [
                    RuntimeEndpointMessage {
                        role: RuntimeEndpointMessageRole::System,
                        content: "instructions",
                    },
                    RuntimeEndpointMessage {
                        role: RuntimeEndpointMessageRole::User,
                        content: "task",
                    },
                ];
                match case {
                    3 => messages[0].role = RuntimeEndpointMessageRole::Assistant,
                    4 => messages[0].content = "instructionS",
                    5 => {
                        messages[0].content = "task";
                        messages[1].content = "instructions";
                    }
                    _ => unreachable!(),
                }
                request.context = RuntimeEndpointContextPacket::from_messages(&messages)
                    .unwrap()
                    .messages()
                    .clone();
            }
            _ => unreachable!(),
        }
        assert!(
            matches!(
                RuntimeEndpointModelPhase::new(&request, original.clone()),
                Err(RuntimePortFailure::Invalid)
            ),
            "{case}"
        );
    }
    let phase = RuntimeEndpointModelPhase::new(&request(), original.clone()).unwrap();
    assert_eq!(phase.context(), &original);
    assert_eq!(phase.usage().dispatch_attempts, 0);
    assert_eq!(phase.usage().prepared_body_bytes, 0);
}
