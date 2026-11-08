// SPDX-License-Identifier: Apache-2.0
//! Event and replay integration with synthetic ports, never coding scores.

use crate::runtime_event::{RuntimeEventBatchLimits, RuntimeEventError, RuntimeEventSequence};
use agentmage_kernel_contracts::{RuntimeEvent, RuntimeEventKind, RuntimeEventRetentionKind};

use super::*;

fn limits() -> RuntimeEventBatchLimits {
    RuntimeEventBatchLimits {
        max_events: 256,
        max_bytes: 1024 * 1024,
    }
}

fn digest(value: &impl serde::Serialize) -> String {
    sha256(&serde_json::to_vec(value).unwrap())
}

fn verify_prefix(events: &[RuntimeEvent]) {
    let mut sequence = RuntimeEventSequence::new();
    for event in events {
        sequence.push(event).unwrap();
    }
    assert_eq!(sequence.event_count(), events.len() as u64);
    assert!(!sequence.is_terminal());
}

#[test]
fn endpoint_events_bind_exact_request_result_and_turn_without_plaintext_or_success() {
    let original = seal_runtime_endpoint_run_request(request()).unwrap();
    let mut turns = RuntimeEndpointTurns::new(original.clone(), clock()).unwrap();
    let prepared = prepared();
    let calls = Arc::new(AtomicUsize::new(0));
    let subscriber = turns.subscribe_events(8).unwrap();
    for index in 0..2 {
        let accepted = turns
            .execute_turn(
                &prepared,
                context(),
                complete(&prepared, &calls, "private model text"),
                None,
                |_| {},
            )
            .unwrap();
        let events = turns.events();
        let offset = 1 + index * 4;
        assert!(matches!(events[offset].kind, RuntimeEventKind::TurnStarted));
        assert!(
            matches!(&events[offset + 1].kind, RuntimeEventKind::ModelRequested {request_sha256, ..} if request_sha256 == &digest(&prepared))
        );
        assert!(
            matches!(&events[offset + 2].kind, RuntimeEventKind::ModelCompleted {result_sha256, ..} if result_sha256 == &digest(accepted.result()))
        );
        assert!(
            matches!(&events[offset + 3].kind, RuntimeEventKind::TurnCompleted {outcome_sha256} if outcome_sha256 == &digest(&accepted))
        );
        for event in &events[offset..offset + 4] {
            assert_eq!(event.turn_id.as_ref(), Some(accepted.turn_id()));
        }
        match (&events[offset + 1].kind, &events[offset + 2].kind) {
            (
                RuntimeEventKind::ModelRequested {
                    model_run_id: requested,
                    ..
                },
                RuntimeEventKind::ModelCompleted {
                    model_run_id: completed,
                    ..
                },
            ) => assert_eq!(requested, completed),
            _ => panic!("wrong model event family"),
        }
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(turns.events().len(), 9);
    assert_eq!(turns.usage().recorded_events, 9);
    let bytes: usize = turns
        .events()
        .iter()
        .map(|event| {
            agentmage_kernel_contracts::to_canonical_json(event)
                .unwrap()
                .len()
        })
        .sum();
    assert_eq!(turns.usage().canonical_event_bytes, bytes as u64);
    verify_prefix(turns.events());
    assert!(
        matches!(&turns.events()[0].kind, RuntimeEventKind::RunStarted {request_sha256} if request_sha256 == &original.request_sha256)
    );
    for event in turns.events() {
        assert_eq!(event.run_id, original.run_id);
        assert_eq!(event.session_id, original.session_id);
        assert_eq!(event.task_id, original.task.task_id);
        assert_eq!(event.policy_id, original.policy_id);
        assert_eq!(&event.correlation_id, turns.correlation_id());
        assert_eq!(event.retention.kind, RuntimeEventRetentionKind::Ephemeral);
        assert!(event.payload_reference.is_none());
        assert!(event.operation_id.is_none());
    }
    let serialized = serde_json::to_string(turns.events()).unwrap();
    for secret in [
        "private model text",
        &original.model_selection.endpoint,
        "synthetic",
        "verified_input_tokens",
        "token_accounting",
    ] {
        assert!(!serialized.contains(secret), "unexpected event content");
    }
    let live = subscriber.try_next_batch(limits()).unwrap();
    assert_eq!(live.events, turns.events()[1..]);
    let replay = turns.replay_events(None, limits()).unwrap();
    assert_eq!(replay.events, turns.events());
    assert!(!replay.terminal);
}

#[test]
fn endpoint_events_refuse_exhausted_recording_capacity_before_any_dispatch() {
    for capacity in [1, 2, 3, 4, 5, 8, 9] {
        let mut request = request();
        request.limits.max_events = capacity;
        let mut turns = run(request, clock());
        let calls = Arc::new(AtomicUsize::new(0));
        let prepared = prepared();
        let count = (capacity - 1) / 4;
        for _ in 0..count {
            turns
                .execute_turn(
                    &prepared,
                    context(),
                    complete(&prepared, &calls, "ok"),
                    None,
                    |_| {},
                )
                .unwrap();
        }
        let before = turns.usage();
        assert!(matches!(
            turns.execute_turn(
                &prepared,
                context(),
                complete(&prepared, &calls, "ok"),
                None,
                |_| {}
            ),
            Err(RuntimeEndpointTurnFailure::Admission(
                RuntimePortFailure::ResourceExhausted
            ))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), count as usize);
        assert_eq!(turns.usage(), before);
        assert_eq!(turns.events().len(), 1 + count as usize * 4);
        assert!(turns.is_closed());
        verify_prefix(turns.events());
    }
}

#[test]
fn endpoint_events_record_failures_without_dependency_text_or_false_completion() {
    for malformed in [false, true] {
        let mut turns = run(request(), clock());
        let calls = Arc::new(AtomicUsize::new(0));
        let prepared = prepared();
        let mut invalid = result(&prepared, "partial private text");
        invalid.response_text_sha256 = "a".repeat(64);
        let failed = turns.execute_turn(
            &prepared,
            context(),
            port(&prepared, &calls, move |_, text| {
                text("partial private text");
                if malformed {
                    Ok(invalid)
                } else {
                    Err("private dependency diagnostic")
                }
            }),
            None,
            |_| {},
        );
        assert!(failed.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(turns.events().len(), 5);
        assert!(
            matches!(&turns.events()[3].kind, RuntimeEventKind::ModelFailed {failure_code, ..} if failure_code == "runtime.model.incomplete.unknown")
        );
        assert!(matches!(
            turns.events()[4].kind,
            RuntimeEventKind::TurnCompleted { .. }
        ));
        assert_eq!(turns.usage().dispatch_attempts, 1);
        assert_eq!(turns.usage().accepted_response_text_bytes, 0);
        assert!(turns.is_closed());
        verify_prefix(turns.events());
        let serialized = serde_json::to_string(turns.events()).unwrap();
        assert!(!serialized.contains("partial private text"));
        assert!(!serialized.contains("private dependency diagnostic"));
        assert!(!serialized.contains("model_completed"));
        assert_eq!(
            turns.replay_events(None, limits()).unwrap().events,
            turns.events()
        );
    }
}

#[test]
fn endpoint_events_replay_lagged_subscriptions_without_redispatch_and_refuse_wrong_cursors() {
    let mut turns = run(request(), clock());
    let calls = Arc::new(AtomicUsize::new(0));
    let prepared = prepared();
    let subscriber = turns.subscribe_events(1).unwrap();
    turns
        .execute_turn(
            &prepared,
            context(),
            complete(&prepared, &calls, "ok"),
            None,
            |_| {},
        )
        .unwrap();
    let live = subscriber.try_next_batch(limits()).unwrap();
    assert_eq!(live.events.len(), 1);
    assert!(live.disconnected);
    let cursor = live.next_cursor.unwrap();
    let page = turns.replay_events(Some(&cursor), limits()).unwrap();
    assert_eq!(page.events, turns.events()[2..]);
    assert!(!page.terminal);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let mut wrong = cursor.clone();
    wrong.event_sha256 = "a".repeat(64);
    assert_eq!(
        turns.replay_events(Some(&wrong), limits()),
        Err(RuntimeEventError::ReplayCursorMismatch)
    );
    wrong = cursor;
    wrong.run_id = agentmage_kernel_contracts::RuntimeRunId::from_raw("another-run");
    assert_eq!(
        turns.replay_events(Some(&wrong), limits()),
        Err(RuntimeEventError::ReplayCursorMismatch)
    );
    assert_eq!(
        turns.replay_events(
            None,
            RuntimeEventBatchLimits {
                max_events: 0,
                ..limits()
            }
        ),
        Err(RuntimeEventError::BatchLimit)
    );
    assert!(turns.subscribe_events(0).is_err());
    assert!(matches!(
        turns.subscribe_events(65),
        Err(RuntimeEventError::SubscriberLimit)
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn endpoint_events_keep_an_incomplete_prefix_on_caught_dependency_panic() {
    let mut turns = run(request(), clock());
    let calls = Arc::new(AtomicUsize::new(0));
    let prepared = prepared();
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = turns.execute_turn(
            &prepared,
            context(),
            port(&prepared, &calls, |_, _| panic!("synthetic port panic")),
            None,
            |_| {},
        );
    }));
    assert!(caught.is_err());
    assert!(turns.is_closed());
    assert_eq!(turns.events().len(), 3);
    assert_eq!(turns.usage().recorded_events, 3);
    assert!(matches!(
        turns.events()[2].kind,
        RuntimeEventKind::ModelRequested { .. }
    ));
    verify_prefix(turns.events());
    let replay = turns.replay_events(None, limits()).unwrap();
    assert_eq!(replay.events, turns.events());
    assert!(!replay.terminal);
    assert_eq!(turns.usage().dispatch_attempts, 1);
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

#[test]
fn endpoint_events_keep_pre_attempt_stop_distinct_from_a_dispatch_attempt() {
    let request = seal_runtime_endpoint_run_request(request()).unwrap();
    let mut turns =
        RuntimeEndpointTurns::new(request, SequenceClock([1000, 1000, 11_000].into())).unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let prepared = prepared();
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
    assert_eq!(turns.usage().turns, 1);
    assert_eq!(turns.usage().dispatch_attempts, 0);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(turns.events().len(), 5);
    assert_eq!(turns.usage().recorded_events, 5);
    assert!(matches!(
        turns.events()[3].kind,
        RuntimeEventKind::ModelFailed { .. }
    ));
    verify_prefix(turns.events());
    assert!(!turns.replay_events(None, limits()).unwrap().terminal);
}
