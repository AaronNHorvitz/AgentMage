// SPDX-License-Identifier: Apache-2.0
//! Synthetic explicit-turn integration, not model scoring or tool execution.

use std::collections::VecDeque;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};

use agentmage_kernel_contracts::{
    BoundaryKind, CONTRACT_SCHEMA_VERSION, CancellationId, CancellationReason, CancellationSignal,
    CorrelationId, ModelCancellationProbe, ModelRuntimeFailure, RuntimeEventCursor, RuntimeEventId,
    RuntimeSessionMode, TaskId,
};

use super::*;
use crate::runtime_loop::{
    RuntimeClock,
    endpoint::{RuntimeEndpointPhaseFailure, RuntimeEndpointTurnFailure, RuntimeEndpointTurns},
};

#[derive(Clone)]
struct Clock(Arc<AtomicU64>);
impl RuntimeClock for Clock {
    fn now_epoch_ms(&mut self) -> Result<u64, RuntimePortFailure> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}
fn clock() -> Clock {
    Clock(Arc::new(AtomicU64::new(1000)))
}
fn run(mut request: RuntimeEndpointRunRequest, clock: Clock) -> RuntimeEndpointTurns<Clock> {
    // Each test changes its own synthetic request, never a frozen task.
    request = seal_runtime_endpoint_run_request(request).unwrap();
    RuntimeEndpointTurns::new(request, clock).unwrap()
}

struct Port<F> {
    request: EndpointModelRequest,
    calls: Arc<AtomicUsize>,
    dispatch: F,
}
impl<F> RuntimeEndpointModelPort for Port<F>
where
    F: FnOnce(
        &dyn ModelOperationControl,
        &mut dyn FnMut(&str),
    ) -> Result<EndpointModelResult, &'static str>,
{
    type Error = &'static str;
    fn request(&self) -> &EndpointModelRequest {
        &self.request
    }
    fn dispatch(
        self,
        control: &dyn ModelOperationControl,
        text: &mut dyn FnMut(&str),
    ) -> Result<EndpointModelResult, Self::Error> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        (self.dispatch)(control, text)
    }
}
fn port<F>(request: &EndpointModelRequest, calls: &Arc<AtomicUsize>, dispatch: F) -> Port<F>
where
    F: FnOnce(
        &dyn ModelOperationControl,
        &mut dyn FnMut(&str),
    ) -> Result<EndpointModelResult, &'static str>,
{
    Port {
        request: request.clone(),
        calls: calls.clone(),
        dispatch,
    }
}
fn result(request: &EndpointModelRequest, text: &str) -> EndpointModelResult {
    EndpointModelResult::from_completed_response(
        request.clone(),
        EndpointModelFinish::Stop,
        sha256(text.as_bytes()),
        text.len(),
        Duration::from_micros(1500),
        None,
    )
}
fn complete(
    request: &EndpointModelRequest,
    calls: &Arc<AtomicUsize>,
    response: &'static str,
) -> impl RuntimeEndpointModelPort<Error = &'static str> {
    let result = result(request, response);
    port(request, calls, move |control, text| {
        assert!(control.remaining_ms().is_ok());
        text(response);
        Ok(result)
    })
}
fn budget(request: &mut RuntimeEndpointRunRequest, resource: BudgetResource, limit: u64) {
    request
        .work_packet
        .budgets
        .retain(|b| b.resource != resource);
    request
        .work_packet
        .budgets
        .push(BudgetLimit { resource, limit });
}

#[test]
fn endpoint_turns_bind_original_context_and_unique_turns_under_one_clock_and_ledger() {
    let original = seal_runtime_endpoint_run_request(request()).unwrap();
    let clock = clock();
    let mut turns = RuntimeEndpointTurns::new(original.clone(), clock.clone()).unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let first = prepared();
    let accepted = turns
        .execute_turn(
            &first,
            context(),
            complete(&first, &calls, "one"),
            None,
            |_| {},
        )
        .unwrap();
    assert_eq!(
        accepted.turn_id().as_str(),
        "turn:d150adcee09244cc603ee8652efb6937"
    );
    assert_eq!(accepted.result().request, first);
    let binding = serde_json::to_value(accepted.run()).unwrap();
    assert_eq!(binding["request_sha256"], original.request_sha256);
    assert_eq!(binding["run_id"], original.run_id.as_str());
    assert_eq!(binding["session_id"], original.session_id.as_str());
    assert_eq!(binding["task_id"], original.task.task_id.as_str());
    assert_eq!(
        turns.correlation_id().as_str(),
        "correlation:d751581d1e2e20ca4996df21731bf7d7"
    );
    clock.0.store(1500, Ordering::SeqCst);
    assert_eq!(turns.remaining_ms(None), Ok(9500));
    let new_context = RuntimeEndpointContextPacket::from_messages(&[RuntimeEndpointMessage {
        role: RuntimeEndpointMessageRole::User,
        content: "next observation",
    }])
    .unwrap();
    let second = EndpointModelRequest::from_prepared_body(
        original.model_selection.clone(),
        new_context.messages().clone(),
        first.body().clone(),
        first.limits().clone(),
    );
    let accepted = turns
        .execute_turn(
            &second,
            new_context,
            complete(&second, &calls, "two"),
            None,
            |_| {},
        )
        .unwrap();
    assert_eq!(
        accepted.turn_id().as_str(),
        "turn:a77d6acfc85470f8831c160605b5c5d7"
    );
    assert_eq!(accepted.result().request, second);
    assert_ne!(accepted.result().request.context, first.context);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(turns.usage().turns, 2);
    assert_eq!(turns.usage().dispatch_attempts, 2);
    assert_eq!(turns.usage().prepared_body_bytes, 400);
    assert_eq!(turns.usage().accepted_response_text_bytes, 6);
    assert_eq!(turns.usage().accepted_dispatch_elapsed_ms, 4);
    assert!(!turns.is_closed());
    let usage = serde_json::to_string(&turns.usage()).unwrap();
    for unknown in ["tokens", "memory", "native", "network"] {
        assert!(!usage.contains(unknown));
    }
}

#[test]
fn endpoint_turns_enforce_each_run_count_and_narrower_work_packet_budget() {
    for case in 0..6 {
        let mut request = request();
        match case {
            0 => {
                request.limits.max_turns = 1;
                request.limits.max_no_progress_turns = 1;
                request.limits.max_model_calls = 4;
            }
            1 => request.limits.max_model_calls = 1,
            2 => request.limits.max_context_refreshes = 1,
            3 => budget(&mut request, BudgetResource::ModelCalls, 1),
            4 => budget(&mut request, BudgetResource::PlanSteps, 1),
            5 => budget(&mut request, BudgetResource::InputBytes, 399),
            _ => unreachable!(),
        }
        let mut turns = run(request, clock());
        let prepared = prepared();
        let calls = Arc::new(AtomicUsize::new(0));
        turns
            .execute_turn(
                &prepared,
                context(),
                complete(&prepared, &calls, "ok"),
                None,
                |_| {},
            )
            .unwrap();
        let before = turns.usage();
        assert!(
            matches!(
                turns.execute_turn(
                    &prepared,
                    context(),
                    complete(&prepared, &calls, "ok"),
                    None,
                    |_| panic!("over budget")
                ),
                Err(RuntimeEndpointTurnFailure::Admission(
                    RuntimePortFailure::ResourceExhausted
                ))
            ),
            "{case}"
        );
        assert_eq!(turns.usage(), before, "{case}");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(turns.is_closed());
        assert!(
            turns
                .execute_turn(
                    &prepared,
                    context(),
                    complete(&prepared, &calls, "ok"),
                    None,
                    |_| {}
                )
                .is_err()
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn endpoint_turns_reserve_complete_output_and_time_envelopes_under_cumulative_ceilings() {
    for case in 0..3 {
        let mut request = request();
        match case {
            0 => request.limits.max_output_bytes = 100,
            1 => budget(&mut request, BudgetResource::OutputBytes, 100),
            2 => budget(&mut request, BudgetResource::ElapsedMilliseconds, 3),
            _ => unreachable!(),
        }
        let mut turns = run(request, clock());
        let prepared = prepared();
        let prepared = if case == 2 {
            let mut limits = prepared.limits().clone();
            limits.total_timeout = Duration::from_millis(2);
            limits.idle_timeout = Duration::from_millis(1);
            EndpointModelRequest::from_prepared_body(
                super::request().model_selection,
                prepared.context.clone(),
                prepared.body().clone(),
                limits,
            )
        } else {
            prepared
        };
        let calls = Arc::new(AtomicUsize::new(0));
        let text = "x".repeat(60);
        let result1 = result(&prepared, &text);
        for attempt in 0..2 {
            let response = result1.clone();
            let mut provisional = String::new();
            let completed = turns.execute_turn(
                &prepared,
                context(),
                port(&prepared, &calls, |_, output| {
                    output(&text);
                    Ok(response)
                }),
                None,
                |part| provisional.push_str(part),
            );
            if attempt == 0 {
                completed.unwrap();
                assert_eq!(provisional, text);
            } else {
                assert!(provisional.is_empty());
                assert!(matches!(
                    completed,
                    Err(RuntimeEndpointTurnFailure::Admission(
                        RuntimePortFailure::ResourceExhausted
                    ))
                ));
            }
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(turns.usage().dispatch_attempts, 1);
        assert_eq!(turns.usage().prepared_body_bytes, 200);
        assert_eq!(turns.usage().accepted_response_text_bytes, 60);
        assert_eq!(turns.usage().accepted_dispatch_elapsed_ms, 2);
        assert!(turns.is_closed());
    }
}

#[test]
fn endpoint_turns_keep_failed_attempts_spent_without_retry_or_accepted_output() {
    for case in 0..3 {
        let mut turns = run(request(), clock());
        let prepared = prepared();
        let calls = Arc::new(AtomicUsize::new(0));
        let mut response = result(&prepared, "ok");
        if case == 1 {
            response.response_text_sha256 = "a".repeat(64);
        }
        let failed = turns.execute_turn(
            &prepared,
            context(),
            port(&prepared, &calls, |_, text| {
                text("ok");
                if case == 0 {
                    Err("dependency")
                } else {
                    if case == 2 {
                        text(&"x".repeat(101));
                    }
                    Ok(response)
                }
            }),
            None,
            |_| {},
        );
        assert!(matches!(
            failed,
            Err(RuntimeEndpointTurnFailure::Dispatch(_))
        ));
        assert!(turns.is_closed());
        assert_eq!(turns.usage().dispatch_attempts, 1);
        assert_eq!(turns.usage().prepared_body_bytes, 200);
        assert_eq!(turns.usage().accepted_response_text_bytes, 0);
        assert_eq!(turns.usage().accepted_dispatch_elapsed_ms, 0);
        assert!(
            turns
                .execute_turn(
                    &prepared,
                    context(),
                    complete(&prepared, &calls, "ok"),
                    None,
                    |_| {}
                )
                .is_err()
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn endpoint_turns_accept_exact_cumulative_envelopes_without_spending_reserved_maxima() {
    let mut request = request();
    request.limits.max_output_bytes = 100;
    for (resource, limit) in [
        (BudgetResource::PlanSteps, 2),
        (BudgetResource::ModelCalls, 2),
        (BudgetResource::InputBytes, 400),
        (BudgetResource::OutputBytes, 100),
        (BudgetResource::ElapsedMilliseconds, 4),
    ] {
        budget(&mut request, resource, limit);
    }
    let selection = request.model_selection.clone();
    let mut turns = run(request, clock());
    let original = prepared();
    let mut limits = original.limits().clone();
    limits.response_text_limit_bytes = 50;
    limits.total_timeout = Duration::from_millis(2);
    limits.idle_timeout = Duration::from_millis(1);
    let prepared = EndpointModelRequest::from_prepared_body(
        selection,
        original.context.clone(),
        original.body().clone(),
        limits,
    );
    let calls = Arc::new(AtomicUsize::new(0));
    for _ in 0..2 {
        let text = "x".repeat(50);
        let response = result(&prepared, &text);
        turns
            .execute_turn(
                &prepared,
                context(),
                port(&prepared, &calls, |_, output| {
                    output(&text);
                    Ok(response)
                }),
                None,
                |_| {},
            )
            .unwrap();
    }
    assert_eq!(turns.usage().dispatch_attempts, 2);
    assert_eq!(turns.usage().prepared_body_bytes, 400);
    assert_eq!(turns.usage().accepted_response_text_bytes, 100);
    assert_eq!(turns.usage().accepted_dispatch_elapsed_ms, 4);
    let before = turns.usage();
    assert!(
        turns
            .execute_turn(
                &prepared,
                context(),
                complete(&prepared, &calls, "ok"),
                None,
                |_| {}
            )
            .is_err()
    );
    assert_eq!(turns.usage(), before);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[test]
fn endpoint_turns_refuse_zero_unavailable_and_regressing_clocks_across_observations() {
    let request = seal_runtime_endpoint_run_request(request()).unwrap();
    let clock = clock();
    clock.0.store(0, Ordering::SeqCst);
    assert!(matches!(
        RuntimeEndpointTurns::new(request.clone(), clock.clone()),
        Err(RuntimePortFailure::Invalid)
    ));
    assert!(matches!(
        RuntimeEndpointTurns::new(request.clone(), SequenceClock([].into())),
        Err(RuntimePortFailure::Unavailable)
    ));
    clock.0.store(1000, Ordering::SeqCst);
    let mut turns = RuntimeEndpointTurns::new(request.clone(), clock.clone()).unwrap();
    clock.0.store(1500, Ordering::SeqCst);
    assert_eq!(turns.remaining_ms(None), Ok(9500));
    clock.0.store(1499, Ordering::SeqCst);
    assert_eq!(turns.remaining_ms(None), Err(RuntimePortFailure::Invalid));
    assert!(turns.is_closed());
    let mut turns = RuntimeEndpointTurns::new(request, SequenceClock([1000].into())).unwrap();
    assert_eq!(
        turns.remaining_ms(None),
        Err(RuntimePortFailure::Unavailable)
    );
    assert!(turns.is_closed());
}

#[test]
fn endpoint_turns_share_runtime_clock_refusals_and_never_reset_the_run_deadline() {
    for now in [0, 999, 11_000] {
        let clock = clock();
        let mut turns = run(request(), clock.clone());
        clock.0.store(now, Ordering::SeqCst);
        let prepared = prepared();
        let calls = Arc::new(AtomicUsize::new(0));
        let expected = if now == 11_000 {
            RuntimePortFailure::TimedOut
        } else {
            RuntimePortFailure::Invalid
        };
        assert!(
            matches!(turns.execute_turn(&prepared, context(), complete(&prepared, &calls, "ok"), None, |_| {}),
            Err(RuntimeEndpointTurnFailure::Admission(e)) if e == expected)
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(turns.usage().dispatch_attempts, 0);
        assert_eq!(turns.usage().turns, 0);
        assert!(turns.is_closed());
    }
    let clock = clock();
    let mut turns = run(request(), clock.clone());
    clock.0.store(10_999, Ordering::SeqCst);
    assert_eq!(turns.remaining_ms(None), Ok(1));
    clock.0.store(11_000, Ordering::SeqCst);
    assert_eq!(turns.remaining_ms(None), Err(RuntimePortFailure::TimedOut));
    clock.0.store(1000, Ordering::SeqCst);
    assert_eq!(turns.remaining_ms(None), Err(RuntimePortFailure::Invalid));
}

#[test]
fn endpoint_turns_latch_callback_deadline_even_when_a_port_ignores_control() {
    let clock = clock();
    let mut turns = run(request(), clock.clone());
    let prepared = prepared();
    let calls = Arc::new(AtomicUsize::new(0));
    let mut delivered = String::new();
    let response = result(&prepared, "ab");
    let failed = turns.execute_turn(
        &prepared,
        context(),
        port(&prepared, &calls, |control, text| {
            text("a");
            assert_eq!(control.remaining_ms(), Err(ModelOperationStop::TimedOut));
            text("b"); // Deliberately broken dependency continues after the stop.
            Ok(response)
        }),
        None,
        |text| {
            delivered.push_str(text);
            clock.0.store(11_000, Ordering::SeqCst);
        },
    );
    assert_eq!(delivered, "a");
    assert!(matches!(
        failed,
        Err(RuntimeEndpointTurnFailure::Dispatch(
            RuntimeEndpointPhaseFailure::Stopped(ModelOperationStop::TimedOut)
        ))
    ));
    assert_eq!(turns.usage().dispatch_attempts, 1);
    assert_eq!(turns.usage().accepted_response_text_bytes, 0);
    assert!(turns.is_closed());
}

struct Probe(Mutex<Option<CancellationSignal>>);
impl ModelCancellationProbe for Probe {
    fn observe(&self) -> Result<Option<CancellationSignal>, ModelRuntimeFailure> {
        Ok(self.0.lock().unwrap().clone())
    }
}
fn signal(turns: &RuntimeEndpointTurns<Clock>) -> CancellationSignal {
    CancellationSignal {
        schema_version: CONTRACT_SCHEMA_VERSION,
        cancellation_id: CancellationId::from_raw("cancel-synthetic"),
        correlation_id: turns.correlation_id().clone(),
        task_id: request().task.task_id,
        reason: CancellationReason::UserRequested,
        requested_by: BoundaryKind::Shell,
    }
}

#[test]
fn endpoint_turns_latch_only_exact_runtime_cancellation_before_and_during_dispatch() {
    for case in 0..5 {
        let mut turns = run(request(), clock());
        let mut cancel = signal(&turns);
        match case {
            1 => cancel.schema_version += 1,
            2 => cancel.task_id = TaskId::from_raw("other-task"),
            3 => cancel.correlation_id = CorrelationId::from_raw("other-correlation"),
            _ => {}
        }
        let prepared = prepared();
        let calls = Arc::new(AtomicUsize::new(0));
        let probe = Probe(Mutex::new(if case == 4 {
            None
        } else {
            Some(cancel.clone())
        }));
        let response = result(&prepared, "ab");
        let mut delivered = String::new();
        let failed = turns.execute_turn(
            &prepared,
            context(),
            port(&prepared, &calls, |_, text| {
                text("a");
                *probe.0.lock().unwrap() = Some(cancel.clone());
                text("b");
                Ok(response)
            }),
            Some(&probe),
            |part| delivered.push_str(part),
        );
        assert!(failed.is_err());
        assert_eq!(
            turns.cancellation(),
            if case == 0 || case == 4 {
                Some(&cancel)
            } else {
                None
            }
        );
        assert_eq!(calls.load(Ordering::SeqCst), usize::from(case == 4));
        assert_eq!(delivered, if case == 4 { "a" } else { "" });
        assert_eq!(turns.usage().dispatch_attempts, u64::from(case == 4));
        assert_eq!(turns.usage().accepted_response_text_bytes, 0);
        assert!(turns.is_closed());
    }
}

struct SequenceClock(VecDeque<u64>);

#[test]
fn endpoint_turns_keep_reservations_spent_when_a_dependency_unwinds() {
    let mut turns = run(request(), clock());
    let prepared = prepared();
    let calls = Arc::new(AtomicUsize::new(0));
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = turns.execute_turn(
            &prepared,
            context(),
            port(&prepared, &calls, |_, text| {
                text("partial");
                panic!("synthetic dependency panic");
            }),
            None,
            |_| {},
        );
    }));
    assert!(unwound.is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(turns.is_closed());
    assert_eq!(turns.usage().turns, 1);
    assert_eq!(turns.usage().dispatch_attempts, 1);
    assert_eq!(turns.usage().prepared_body_bytes, 200);
    assert_eq!(turns.usage().accepted_response_text_bytes, 0);
    assert_eq!(turns.usage().accepted_dispatch_elapsed_ms, 0);
    assert!(
        turns
            .execute_turn(
                &prepared,
                context(),
                complete(&prepared, &calls, "ok"),
                None,
                |_| {}
            )
            .is_err()
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

impl RuntimeClock for SequenceClock {
    fn now_epoch_ms(&mut self) -> Result<u64, RuntimePortFailure> {
        self.0.pop_front().ok_or(RuntimePortFailure::Unavailable)
    }
}

#[test]
fn endpoint_turns_do_not_count_dispatch_when_the_phase_control_stops_before_attempt() {
    let request = seal_runtime_endpoint_run_request(request()).unwrap();
    let mut turns =
        RuntimeEndpointTurns::new(request, SequenceClock([1000, 1000, 11_000].into())).unwrap();
    let prepared = prepared();
    let calls = Arc::new(AtomicUsize::new(0));
    assert!(matches!(
        turns.execute_turn(
            &prepared,
            context(),
            complete(&prepared, &calls, "ok"),
            None,
            |_| {}
        ),
        Err(RuntimeEndpointTurnFailure::Dispatch(
            RuntimeEndpointPhaseFailure::Stopped(ModelOperationStop::TimedOut)
        ))
    ));
    assert_eq!(turns.usage().turns, 1);
    assert_eq!(turns.usage().dispatch_attempts, 0);
    assert_eq!(turns.usage().prepared_body_bytes, 0);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(turns.is_closed());
}

#[test]
fn endpoint_turns_refuse_unowned_modes_resume_context_and_prepared_binding_drift() {
    let original = seal_runtime_endpoint_run_request(request()).unwrap();
    let mut invalid = original.clone();
    invalid.model_selection.model = "drift".into();
    assert!(RuntimeEndpointTurns::new(invalid, clock()).is_err());
    let mut durable = original.clone();
    durable.mode = RuntimeSessionMode::DurableReadOnly;
    let durable = seal_runtime_endpoint_run_request(durable).unwrap();
    assert!(RuntimeEndpointTurns::new(durable.clone(), clock()).is_err());
    let mut resume = durable;
    resume.event_cursor = Some(RuntimeEventCursor {
        run_id: resume.run_id.clone(),
        event_id: RuntimeEventId::from_raw("event-synthetic"),
        sequence: 1,
        event_sha256: "a".repeat(64),
    });
    assert!(
        RuntimeEndpointTurns::new(seal_runtime_endpoint_run_request(resume).unwrap(), clock())
            .is_err()
    );
    for case in 0..3 {
        let mut turns = RuntimeEndpointTurns::new(original.clone(), clock()).unwrap();
        let prepared = prepared();
        let mut value = serde_json::to_value(&prepared).unwrap();
        // Reconstruct a distinct frozen descriptor through its public constructor.
        let mut selection = original.model_selection.clone();
        selection.model = "other".into();
        let drift = EndpointModelRequest::from_prepared_body(
            selection,
            prepared.context.clone(),
            prepared.body().clone(),
            prepared.limits().clone(),
        );
        let calls = Arc::new(AtomicUsize::new(0));
        let supplied = if case == 0 { &drift } else { &prepared };
        let ctx = if case == 2 {
            RuntimeEndpointContextPacket::from_messages(&[RuntimeEndpointMessage {
                role: RuntimeEndpointMessageRole::User,
                content: "other",
            }])
            .unwrap()
        } else {
            context()
        };
        let port_request = if case == 1 { &drift } else { supplied };
        assert!(
            turns
                .execute_turn(
                    supplied,
                    ctx,
                    complete(port_request, &calls, "ok"),
                    None,
                    |_| {}
                )
                .is_err()
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(turns.usage().turns, 0);
        assert_eq!(turns.usage().dispatch_attempts, 0);
        assert!(turns.is_closed());
        // Context metadata stays content-free even on an invalid preparation.
        value.as_object_mut().unwrap().remove("context");
        assert!(
            !serde_json::to_string(&value)
                .unwrap()
                .contains("instructions")
        );
    }
}
