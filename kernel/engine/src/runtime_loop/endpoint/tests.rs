// SPDX-License-Identifier: Apache-2.0
//! Synthetic ports exercise phase admission; no inference or native identity.

use std::cell::Cell;
use std::num::NonZeroU64;
use std::time::Duration;

use agentmage_kernel_contracts::{
    EndpointModelBody, EndpointModelContext, EndpointModelFinish, EndpointModelLimits,
    EndpointModelSelection, ProviderTokenUsage,
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
fn request() -> EndpointModelRequest {
    EndpointModelRequest::from_prepared_body(
        EndpointModelSelection {
            endpoint: "http://127.0.0.1:1234/v1".into(),
            model: "synthetic".into(),
        },
        EndpointModelContext {
            message_count: 2,
            serialized_messages_bytes: 100,
            serialized_messages_sha256: "a".repeat(64),
        },
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
        "c".repeat(64),
        50,
        Duration::from_micros(1500),
        None,
    )
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
            let mut phase = RuntimeEndpointModelPhase::new(&request).unwrap();
            assert_eq!(phase.usage().dispatch_attempts, 0);
            assert_eq!(phase.usage().completed_response_text_bytes, None);
            let result = EndpointModelResult::from_completed_response(
                request.clone(),
                finish,
                "c".repeat(64),
                100,
                Duration::from_millis(100),
                usage,
            );
            let expected = result.clone();
            let result = phase
                .execute(&live(), |control| {
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
                phase.execute(&live(), |_| {
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
        let mut phase = RuntimeEndpointModelPhase::new(&request()).unwrap();
        let calls = Cell::new(0);
        let control = Control(Cell::new(Err(stop)));
        assert_eq!(
            phase.execute(&control, |_| {
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
            phase.execute(&control, |_| {
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
        let mut phase = RuntimeEndpointModelPhase::new(&request()).unwrap();
        let control = live();
        let result = phase.execute(&control, |_| {
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
            phase.execute(&live(), |_| Ok::<_, ()>(completion(request()))),
            Err(RuntimeEndpointPhaseFailure::ResourceExhausted)
        );
    }
}

#[test]
fn endpoint_phase_rejects_mismatched_result_bindings_and_bounds_atomically() {
    for case in 0..9 {
        let mut phase = RuntimeEndpointModelPhase::new(&request()).unwrap();
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
            phase.execute(&live(), |_| Ok::<_, ()>(result)),
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
    let mut phase = RuntimeEndpointModelPhase::new(&request()).unwrap();
    let mut result = completion(request());
    result.response_text_bytes = 0;
    phase.execute(&live(), |_| Ok::<_, ()>(result)).unwrap();
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
            RuntimeEndpointModelPhase::new(&malformed),
            Err(RuntimePortFailure::Invalid)
        ));
    }
}
