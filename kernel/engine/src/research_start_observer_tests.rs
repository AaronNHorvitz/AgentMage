// Canonical durability and no-replay tests; every effect driver is synthetic.

use super::*;
use crate::research_dispatch::tests::ObservedResearchDriver;
use crate::runtime_journal::RuntimeJournalError;
use std::cell::Cell;

#[test]
fn research_start_observer_sees_exact_durable_start_once_before_driver() {
    let mut f = fixture();
    let observed = Cell::new(false);
    let mut calls = 0;
    let mut driver = ObservedResearchDriver {
        inner: RecordingResearchDriver {
            packet: f.prepared.packet(),
            now: 103,
            calls: 0,
            reservation: None,
        },
        observed: &observed,
    };
    let (receipt, pending) = f
        .runtime
        .begin_research_effect_with_start_observer(
            &f.registry,
            &f.policy,
            f.request,
            &mut driver,
            f.started.clone(),
            &f.payloads,
            &f.context,
            &f.prepared,
            f.reservation,
            &mut |start| {
                assert_eq!(start, &f.started);
                assert!(!observed.replace(true));
                calls += 1;
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(calls, 1);
    assert_eq!(driver.inner.calls, 1);
    assert_eq!(
        f.runtime.current_grant(&f.grant_id).unwrap().status,
        GrantStatus::Consumed
    );
    assert!(matches!(
        f.runtime.runtime_events(&f.context.run_id),
        Err(DurableAuthorityError::Poisoned)
    )); // Pending terminal forbids ordinary reads.

    let terminal = next_event(
        &f.started,
        RuntimeEventKind::ToolCompleted {
            tool_call_id: ToolCallId::from_raw("dispatch-call-1"),
            receipt_id: receipt.receipt_id.clone(),
            result_sha256: digest('6'),
        },
        104,
    );
    f.runtime
        .finish_effect_with_runtime_event(pending, terminal.clone())
        .unwrap();
    drop(f.runtime);
    let mut reopened = DurableAuthorityRuntime::open(
        &f.directory.join("authority.db"),
        &observation(),
        &mut TestKey,
        105,
    )
    .unwrap();
    assert_eq!(reopened.receipts(), [receipt]);
    assert_eq!(
        reopened.runtime_events(&f.context.run_id).unwrap().last(),
        Some(&terminal)
    );
    let state = reopened.research_budget_state(&f.context).unwrap();
    assert_eq!(state.progress.visits, 1);
    assert_eq!(state.progress.reserved_bytes, 512);
    assert!(
        reopened
            .reserve_research_request(
                &f.payloads,
                &f.context,
                &f.prepared,
                ResearchOperation::Visit,
                105
            )
            .is_err()
    );
    assert_eq!(driver.inner.calls, 1);
    drop(reopened);
    fs::remove_dir_all(f.directory).unwrap();
}

#[test]
fn research_start_observer_failure_retains_consumption_and_reconciles_without_replay() {
    let mut f = fixture();
    let mut calls = 0;
    let mut driver = RecordingResearchDriver {
        packet: f.prepared.packet(),
        now: 103,
        calls: 0,
        reservation: None,
    };
    let error = f
        .runtime
        .begin_research_effect_with_start_observer(
            &f.registry,
            &f.policy,
            f.request,
            &mut driver,
            f.started.clone(),
            &f.payloads,
            &f.context,
            &f.prepared,
            f.reservation,
            &mut |start| {
                assert_eq!(start, &f.started);
                calls += 1;
                Err(RuntimeJournalError::Integrity)
            },
        )
        .err();
    assert_eq!(
        error,
        Some(DurableAuthorityError::Transaction(
            crate::authority_transaction::AuthorityTransactionError::PersistenceFailure
        ))
    );
    assert_eq!(calls, 1);
    assert_eq!(driver.calls, 0);
    assert_eq!(
        f.runtime.current_grant(&f.grant_id).unwrap().status,
        GrantStatus::Consumed
    );
    assert_eq!(
        f.runtime.cancel_research_budget(&f.context),
        Err(DurableAuthorityError::Poisoned)
    );
    drop(f.runtime);
    let mut reopened = DurableAuthorityRuntime::open(
        &f.directory.join("authority.db"),
        &observation(),
        &mut TestKey,
        105,
    )
    .unwrap();
    let events = reopened.runtime_events(&f.context.run_id).unwrap();
    assert_eq!(events.iter().filter(|e| *e == &f.started).count(), 1);
    let tx = reopened.current_transaction(&f.transaction_id).unwrap();
    assert_eq!(tx.state, AuthorityTransactionState::Terminal);
    assert_eq!(tx.outcome, Some(OperationOutcome::Uncertain));
    assert!(tx.uncertain_effect);
    assert_eq!(reopened.receipts().len(), 1);
    assert_eq!(reopened.receipts()[0].outcome, OperationOutcome::Uncertain);
    let state = reopened.research_budget_state(&f.context).unwrap();
    assert_eq!(state.progress.visits, 1);
    assert_eq!(state.progress.reserved_bytes, 512);
    assert_eq!(state.progress.last_epoch_ms, 103);
    assert!(
        reopened
            .reserve_research_request(
                &f.payloads,
                &f.context,
                &f.prepared,
                ResearchOperation::Visit,
                105
            )
            .is_err()
    );
    assert_eq!(driver.calls, 0);
    let receipts = reopened.receipts().to_vec();
    drop(reopened);
    let reopened = DurableAuthorityRuntime::open(
        &f.directory.join("authority.db"),
        &observation(),
        &mut TestKey,
        106,
    )
    .unwrap();
    assert_eq!(reopened.receipts(), receipts);
    drop(reopened);
    fs::remove_dir_all(f.directory).unwrap();
}

#[test]
fn research_start_observer_is_not_called_when_preflight_or_start_persistence_refuses() {
    for mutation in ["cancelled", "stale", "missing-plan", "start-persistence"] {
        let mut f = fixture();
        match mutation {
            "cancelled" => {
                f.runtime.cancel_research_budget(&f.context).unwrap();
            }
            "stale" => {
                f.runtime
                    .reserve_research_request(
                        &f.payloads,
                        &f.context,
                        &packet(&f.plan, "newer-call", 102, 1),
                        ResearchOperation::Visit,
                        102,
                    )
                    .unwrap();
            }
            "missing-plan" => {
                f.payloads.objects.clear();
            }
            "start-persistence" => {
                f.runtime.flush_runtime_events().unwrap();
                let shared = f.runtime.engineering_store().unwrap().shared_store();
                shared
                    .lock()
                    .unwrap()
                    .connection
                    .execute_batch(
                        "CREATE TRIGGER synthetic_start_failure BEFORE INSERT ON runtime_events
                     BEGIN SELECT RAISE(ABORT, 'synthetic.start.failure'); END;",
                    )
                    .unwrap();
            }
            _ => unreachable!(),
        }
        let mut calls = 0;
        let mut driver = RecordingResearchDriver {
            packet: f.prepared.packet(),
            now: 103,
            calls: 0,
            reservation: None,
        };
        assert!(
            f.runtime
                .begin_research_effect_with_start_observer(
                    &f.registry,
                    &f.policy,
                    f.request,
                    &mut driver,
                    f.started,
                    &f.payloads,
                    &f.context,
                    &f.prepared,
                    f.reservation,
                    &mut |_| {
                        calls += 1;
                        Ok(())
                    }
                )
                .is_err()
        );
        assert_eq!(calls, 0, "observed failed start {mutation}");
        assert_eq!(driver.calls, 0, "effect after refusal {mutation}");
        drop(f.runtime);
        fs::remove_dir_all(f.directory).unwrap();
    }
}
