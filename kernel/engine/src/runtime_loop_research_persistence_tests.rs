//! Research plans and reservations persisted through the real coordinator and
//! the real owners (Decision 0140, AMR-03.1.1). The coordinator publishes the
//! plan and has its budget opened; the trusted glue reserves each request
//! before the effect starts. These tests cover interruption, recovery with the
//! original clocks, terminal cancellation and expiry, failed attempts, and one
//! budget per task and one owner per store. Only the native result is
//! synthetic; nothing is sent.

use super::*;
use crate::research_budget::ResearchBudgetError;
use crate::research_journal::ResearchJournalError;

/// The person's cancellation of the run's own task.
fn cancellation_of(runtime: &OwnerCoordinator) -> CancellationSignal {
    CancellationSignal {
        schema_version: CONTRACT_SCHEMA_VERSION,
        cancellation_id: CancellationId::from_raw("research-cancellation-1"),
        correlation_id: runtime.correlation_id.clone(),
        task_id: runtime.request.task.task_id.clone(),
        reason: CancellationReason::UserRequested,
        requested_by: BoundaryKind::Shell,
    }
}

/// The budget owner's refusal of a reservation, as the glue receives it.
fn budget_refusal(error: ResearchBudgetError) -> DurableAuthorityError {
    DurableAuthorityError::ResearchJournal(ResearchJournalError::Budget(error))
}

/// The run's own request for another page of the plan's first domain,
/// prepared through the owner at `now`.
fn later_request(port: &mut OwnerPort, operation_id: &str, now: u64) -> PreparedPublicGet {
    let mut call = port.last_call.clone().unwrap();
    call.tool_call_id = ToolCallId::from_raw(operation_id);
    call.arguments.bytes = serde_json::to_vec(&draft(operation_id, "/later")).unwrap();
    call.arguments.sha256 = sha256(&call.arguments.bytes);
    port.authority
        .prepare_public_get(&port.payloads, &port.registry, &port.context, &call, now)
        .unwrap()
}

/// What the canonical budget says, compared across a reopening.
fn snapshot(port: &mut OwnerPort) -> (u16, String, ResearchBudgetProgress) {
    let state = port.authority.research_budget_state(&port.context).unwrap();
    (state.revision, state.head_sha256, state.progress)
}

fn has_event(port: &mut OwnerPort, matches: fn(&RuntimeEventKind) -> bool) -> bool {
    port.authority
        .runtime_events(&port.context.run_id)
        .unwrap()
        .iter()
        .any(|event| matches(&event.kind))
}

/// Composes another admitted run of the same task over the same owner: a new
/// run identity, the given plan and the same policy.
fn another_run(port: OwnerPort, run_id: &str, plan: &[u8]) -> OwnerCoordinator {
    let registry = public_get_registry();
    let profile = profile("runtime-loop-research-completion");
    let mut request = request(profile.clone(), &registry);
    request.mode = RuntimeSessionMode::DurableReadOnly;
    request.run_id = RuntimeRunId::from_raw(run_id);
    request.policy_sha256 = port.context.policy_sha256.clone();
    let context = ResearchBudgetContext {
        run_id: request.run_id.clone(),
        ..port.context.clone()
    };
    let admission =
        RuntimeResearchAdmission::new(plan, context, public_get_registry().list_tools()[0])
            .unwrap();
    request.task.constraints.push(admission.constraint());
    request.request_sha256 = "0".repeat(64);
    let request = seal_runtime_run_request(request).unwrap();
    let clock = port.clock.clone();
    let bundles = Rc::clone(&port.bundles);
    ReusableRuntimeCoordinator::new_with_research_admission(
        request,
        PublicGetModel {
            inner: FakeModel::new(profile, [ModelScript::Tool, ModelScript::Completion]),
            drafts: one_get().into(),
            report: None,
            bundles,
        },
        FakeContext,
        registry,
        port,
        FakeVerifier {
            verifier_id: VerifierId::from_raw("verifier-research-completion"),
            source: VerifierSource::DeterministicPostcondition,
        },
        clock,
        admission,
    )
    .unwrap()
}

#[test]
fn a_reservation_is_durable_before_the_start_and_survives_an_interruption() {
    let mut runtime = admitted_run(Fault::InterruptAfterReservation, one_get());
    assert!(runtime.run_until_boundary(None, None).is_err());
    let mut port = runtime.tool_boundary;
    // The reservation committed, and nothing started: the grant is unspent,
    // the worker never ran and the journal holds no start.
    assert!(port.attempts[0].reservation_sha256.is_some());
    assert_eq!(port.attempts[0].driver_calls, 0);
    assert_eq!(
        port.authority
            .current_grant(&port.attempts[0].grant.grant_id)
            .unwrap()
            .status,
        GrantStatus::Issued
    );
    assert!(!has_event(&mut port, |kind| matches!(
        kind,
        RuntimeEventKind::ToolStarted { .. }
    )));
    let before = snapshot(&mut port);
    assert_eq!(before.0, 1);
    assert_eq!(before.2.visits, 1);
    assert_eq!(before.2.reserved_bytes, MAXIMUM_RESPONSE_BYTES);

    // The reopened owner recovers the same bounded snapshot: revision, head,
    // counts and the original clocks.
    port = port.reopen(READ_AT);
    assert_eq!(snapshot(&mut port), before);
    // The spent request is never reserved again, so it cannot start.
    let prepared = Rc::clone(&port.attempts[0].prepared);
    assert_eq!(
        port.authority
            .reserve_research_request(
                &port.payloads,
                &port.context,
                &prepared,
                before.2.last_epoch_ms + 1
            )
            .err(),
        Some(budget_refusal(ResearchBudgetError::Binding))
    );
    // Accounting continues from the recovered snapshot under the original
    // start: a new request of the run counts as the second visit.
    let now = before.2.last_epoch_ms + 2;
    let later = later_request(&mut port, "public-get-2", now);
    port.authority
        .reserve_research_request(&port.payloads, &port.context, &later, now)
        .unwrap();
    let after = snapshot(&mut port);
    assert_eq!(after.0, 2);
    assert_eq!(after.2.visits, 2);
    assert_eq!(after.2.started_epoch_ms, before.2.started_epoch_ms);
    assert_eq!(after.2.last_epoch_ms, now);
    port.close();
}

#[test]
fn a_rolled_back_or_late_clock_ends_the_budget_and_the_end_survives_reopening() {
    for rollback in [true, false] {
        let mut runtime = admitted_run(Fault::InterruptAfterReservation, one_get());
        assert!(runtime.run_until_boundary(None, None).is_err());
        let mut port = runtime.tool_boundary;
        let before = snapshot(&mut port);
        port = port.reopen(READ_AT);
        // A clock before the last retained observation, or at the original
        // start plus the plan's elapsed limit, refuses the reservation and
        // retains the observation.
        let elapsed = ResearchLimits::ceiling(ResearchDepth::Quick).elapsed_ms;
        let prepared_at = before.2.last_epoch_ms + 1;
        let later = later_request(&mut port, "public-get-2", prepared_at);
        let now = if rollback {
            before.2.last_epoch_ms - 1
        } else {
            before.2.started_epoch_ms + elapsed
        };
        assert_eq!(
            port.authority
                .reserve_research_request(&port.payloads, &port.context, &later, now)
                .err(),
            Some(budget_refusal(ResearchBudgetError::Exhausted)),
            "{rollback}"
        );
        let ended = snapshot(&mut port);
        assert!(ended.2.deadline_exhausted, "{rollback}");
        assert_eq!(ended.0, before.0 + 1, "{rollback}");
        assert_eq!(ended.2.visits, 1, "{rollback}");
        // Reopening keeps the end; a valid clock does not reset it.
        port = port.reopen(READ_AT + 1);
        assert_eq!(snapshot(&mut port), ended, "{rollback}");
        assert_eq!(
            port.authority
                .reserve_research_request(&port.payloads, &port.context, &later, prepared_at)
                .err(),
            Some(budget_refusal(ResearchBudgetError::Exhausted)),
            "{rollback}"
        );
        port.close();
    }
}

#[test]
fn an_interrupted_effect_is_recovered_and_never_dispatched_again() {
    let mut runtime = admitted_run(Fault::InterruptAfterStart, one_get());
    assert!(runtime.run_until_boundary(None, None).is_err());
    let mut port = runtime.tool_boundary;
    assert_eq!(port.attempts[0].driver_calls, 1);
    // While the terminal is pending the owner refuses everything else.
    assert_eq!(
        port.authority.research_budget_state(&port.context).err(),
        Some(DurableAuthorityError::Poisoned)
    );
    port = port.reopen(READ_AT);
    // The start is durable and no terminal event names the call.
    assert!(has_event(&mut port, |kind| matches!(
        kind,
        RuntimeEventKind::ToolStarted { .. }
    )));
    assert!(!has_event(&mut port, |kind| matches!(
        kind,
        RuntimeEventKind::ToolCompleted { .. } | RuntimeEventKind::ToolFailed { .. }
    )));
    // Recovery closed the transaction with the worker's recorded outcome and
    // a receipt. The grant stays spent, and the source bytes the worker
    // returned were never published, so nothing under the receipt is readable.
    let transaction = port
        .authority
        .current_transaction(&port.attempts[0].transaction_id)
        .unwrap()
        .clone();
    assert_eq!(transaction.state, AuthorityTransactionState::Terminal);
    assert_eq!(transaction.outcome, Some(OperationOutcome::Succeeded));
    assert!(
        port.authority
            .receipts()
            .iter()
            .any(|receipt| Some(&receipt.receipt_id) == transaction.receipt_id.as_ref())
    );
    assert_eq!(
        port.authority
            .current_grant(&port.attempts[0].grant.grant_id)
            .unwrap()
            .status,
        GrantStatus::Consumed
    );
    assert_eq!(receipt_artifact_events(&mut port), 0);
    // The visit stays spent and the request is never reserved again.
    let state = port.authority.research_budget_state(&port.context).unwrap();
    assert_eq!(state.progress.visits, 1);
    let prepared = Rc::clone(&port.attempts[0].prepared);
    assert_eq!(
        port.authority
            .reserve_research_request(&port.payloads, &port.context, &prepared, READ_AT + 1)
            .err(),
        Some(budget_refusal(ResearchBudgetError::Binding))
    );
    assert_eq!(port.attempts[0].driver_calls, 1);
    port.close();
}

#[test]
fn a_worker_that_did_not_succeed_keeps_its_spent_visit() {
    for (outcome, state) in [
        (OperationOutcome::Failed, AgentStateKind::Failed),
        (OperationOutcome::TimedOut, AgentStateKind::Exhausted),
        (OperationOutcome::Uncertain, AgentStateKind::Uncertain),
    ] {
        let mut runtime = admitted_run(Fault::WorkerOutcome(outcome), one_get());
        let RuntimeCoordinatorStep::Complete { outcome: ended } =
            runtime.run_until_boundary(None, None).unwrap()
        else {
            panic!("the failed attempt ends the run");
        };
        assert_eq!(ended.state, state, "{outcome:?}");
        let mut port = runtime.tool_boundary;
        assert_eq!(port.attempts[0].driver_calls, 1, "{outcome:?}");
        // The terminal names the owner's receipt, which records the outcome;
        // the attempt retained nothing.
        let receipt = port.attempts[0].receipt.clone().unwrap();
        assert_eq!(receipt.outcome, outcome);
        assert_eq!(port.authority.receipts(), std::slice::from_ref(&receipt));
        assert_eq!(ended.receipt_ids, std::slice::from_ref(&receipt.receipt_id));
        assert!(
            port.authority
                .runtime_events(&port.context.run_id)
                .unwrap()
                .iter()
                .any(
                    |event| matches!(&event.kind, RuntimeEventKind::ToolFailed { receipt_id, .. }
                if receipt_id.as_ref() == Some(&receipt.receipt_id))
                )
        );
        assert_eq!(receipt_artifact_events(&mut port), 0, "{outcome:?}");
        assert!(port.attempts[0].bundle.is_none());
        // The visit and its worst-case bytes stay spent; the request is never
        // reserved again, also after reopening.
        port = port.reopen(READ_AT);
        let state = port.authority.research_budget_state(&port.context).unwrap();
        assert_eq!(state.progress.visits, 1, "{outcome:?}");
        assert_eq!(state.progress.reserved_bytes, MAXIMUM_RESPONSE_BYTES);
        assert!(!state.progress.cancelled);
        let prepared = Rc::clone(&port.attempts[0].prepared);
        assert!(
            port.authority
                .reserve_research_request(&port.payloads, &port.context, &prepared, READ_AT + 1)
                .is_err()
        );
        assert_eq!(port.budget_cancellations, 0);
        port.close();
    }
}

#[test]
fn a_cancelled_run_cancels_its_budget_durably() {
    // Before any request: nothing is spent.
    let mut runtime = admitted_run(Fault::None, one_get());
    let signal = cancellation_of(&runtime);
    let RuntimeCoordinatorStep::Complete { outcome } =
        runtime.run_until_boundary(None, Some(&signal)).unwrap()
    else {
        panic!("a cancelled run ends");
    };
    assert_eq!(outcome.state, AgentStateKind::Cancelled);
    assert_eq!(outcome.unresolved_codes, ["runtime.cancelled"]);
    let mut port = runtime.tool_boundary;
    assert_eq!(port.budget_cancellations, 1);
    assert!(port.attempts.is_empty());
    let cancelled = snapshot(&mut port);
    assert!(cancelled.2.cancelled);
    assert_eq!(cancelled.2.visits, 0);
    // The cancellation is retained across reopening and refuses any later
    // reservation before anything else is checked.
    port = port.reopen(READ_AT);
    assert_eq!(snapshot(&mut port), cancelled);
    let scope = port
        .authority
        .research_budget_state(&port.context)
        .unwrap()
        .scope;
    let later = PreparedPublicGet::prepare(
        &scope,
        draft("public-get-2", "/later"),
        cancelled.2.started_epoch_ms,
        cancelled.2.last_epoch_ms + 1,
    )
    .unwrap();
    assert_eq!(
        port.authority
            .reserve_research_request(
                &port.payloads,
                &port.context,
                &later,
                cancelled.2.last_epoch_ms + 1
            )
            .err(),
        Some(budget_refusal(ResearchBudgetError::Cancelled))
    );
    port.close();
}

#[test]
fn a_cancellation_after_a_completed_request_keeps_the_spent_visit() {
    // An ask plan of two requests: the first completes, and the person
    // cancels while the second waits for approval. The second is never
    // issued or reserved.
    let mut runtime = asking_run(
        2,
        vec![
            draft("public-get-1", "/guide"),
            draft("public-get-2", "/reference"),
        ],
    );
    let RuntimeCoordinatorStep::AwaitingApproval { challenge } =
        runtime.run_until_boundary(None, None).unwrap()
    else {
        panic!("the first request waits for the person");
    };
    let RuntimeCoordinatorStep::AwaitingApproval { challenge: second } = runtime
        .run_until_boundary(
            Some(&decide(&challenge, RuntimeApprovalDisposition::Allow)),
            None,
        )
        .unwrap()
    else {
        panic!("the second request waits for the person");
    };
    let signal = cancellation_of(&runtime);
    let RuntimeCoordinatorStep::Complete { outcome } =
        runtime.run_until_boundary(None, Some(&signal)).unwrap()
    else {
        panic!("the cancelled run ends");
    };
    assert_eq!(outcome.state, AgentStateKind::Cancelled);
    let mut port = runtime.tool_boundary;
    assert_eq!(port.budget_cancellations, 1);
    assert_eq!(
        completed_calls(&mut port),
        [ToolCallId::from_raw("public-get-1")]
    );
    assert!(
        port.authority
            .current_grant(&second.proposed_grant_id)
            .is_none()
    );
    let state = port.authority.research_budget_state(&port.context).unwrap();
    assert!(state.progress.cancelled);
    assert_eq!(state.progress.visits, 1);
    assert_eq!(state.progress.reserved_bytes, MAXIMUM_RESPONSE_BYTES);
    port.close();
}

#[test]
fn a_cancellation_during_the_effect_ends_the_run_and_its_budget() {
    // The person's cancellation arrives while the worker runs; the worker
    // stops, and the owner's receipt records the cancelled attempt.
    let mut runtime = admitted_run(Fault::WorkerOutcome(OperationOutcome::Cancelled), one_get());
    let probe = RaisedCancellation {
        raised: Arc::clone(&runtime.tool_boundary.cancellation),
        signal: cancellation_of(&runtime),
    };
    let RuntimeCoordinatorStep::Complete { outcome } =
        runtime.run_until_boundary(None, Some(&probe)).unwrap()
    else {
        panic!("the cancelled run ends");
    };
    assert_eq!(outcome.state, AgentStateKind::Cancelled);
    assert!(
        !outcome
            .unresolved_codes
            .contains(&RESEARCH_BUDGET_CANCELLATION_UNCONFIRMED.to_owned())
    );
    let mut port = runtime.tool_boundary;
    assert_eq!(port.budget_cancellations, 1);
    assert_eq!(port.attempts[0].driver_calls, 1);
    assert_eq!(
        port.attempts[0].receipt.as_ref().unwrap().outcome,
        OperationOutcome::Cancelled
    );
    let state = port.authority.research_budget_state(&port.context).unwrap();
    assert!(state.progress.cancelled);
    assert_eq!(state.progress.visits, 1);
    port.close();
}

#[test]
fn an_expired_plan_refuses_the_start_and_stays_expired() {
    let mut runtime = admitted_run(Fault::ExpireAfterIssue, one_get());
    assert!(runtime.run_until_boundary(None, None).is_err());
    let mut port = runtime.tool_boundary;
    // The start's reservation was refused before anything started: the
    // grant is unspent and the worker never ran.
    assert!(port.attempts[0].reservation_sha256.is_none());
    assert_eq!(port.attempts[0].driver_calls, 0);
    assert_eq!(
        port.authority
            .current_grant(&port.attempts[0].grant.grant_id)
            .unwrap()
            .status,
        GrantStatus::Issued
    );
    assert!(!has_event(&mut port, |kind| matches!(
        kind,
        RuntimeEventKind::ToolStarted { .. }
    )));
    // The observed expiry was retained as a restriction, with nothing spent.
    let expired = snapshot(&mut port);
    assert!(expired.2.deadline_exhausted);
    assert_eq!(expired.0, 1);
    assert_eq!(expired.2.visits, 0);
    // Reopening keeps the expiry, at a later or an earlier clock.
    port = port.reopen(READ_AT + 70_000);
    assert_eq!(snapshot(&mut port), expired);
    let prepared = Rc::clone(&port.attempts[0].prepared);
    for now in [
        expired.2.started_epoch_ms + 1,
        prepared.packet().prepared_at_epoch_ms(),
    ] {
        assert_eq!(
            port.authority
                .reserve_research_request(&port.payloads, &port.context, &prepared, now)
                .err(),
            Some(budget_refusal(ResearchBudgetError::Exhausted))
        );
    }
    assert_eq!(snapshot(&mut port).2, expired.2);
    port.close();
}

#[test]
fn a_task_budget_admits_one_run_and_its_store_one_owner() {
    let mut runtime = admitted_run(Fault::None, one_get());
    runtime.run_until_boundary(None, None).unwrap();
    let mut port = runtime.tool_boundary;
    let first = snapshot(&mut port);
    // A second owner of the same store is refused while the first is open.
    assert!(
        DurableAuthorityRuntime::open(
            &port.directory.store(),
            &observation(),
            &mut TestKey,
            READ_AT
        )
        .is_err()
    );
    // Another run of the same task, with the same plan or another one, is
    // refused when its budget would open; the task's budget is unchanged.
    for (run, plan) in [
        (
            "runtime-run-0002",
            confirmed_plan(ResearchNetworkMode::TaskAuthorized),
        ),
        ("runtime-run-0003", confirmed_plan(ResearchNetworkMode::Ask)),
    ] {
        let mut second = another_run(port, run, &plan);
        assert_eq!(
            second.run_until_boundary(None, None).err(),
            Some(RuntimeLoopError::Dependency(RuntimePortFailure::Uncertain)),
            "{run}"
        );
        assert!(
            !second
                .events()
                .iter()
                .any(|event| matches!(event.kind, RuntimeEventKind::TurnStarted)),
            "{run}"
        );
        port = second.tool_boundary;
        assert_eq!(snapshot(&mut port), first, "{run}");
        // The budget stays bound to the run that opened it.
        let mut context = port.context.clone();
        context.run_id = RuntimeRunId::from_raw(run);
        assert_eq!(
            port.authority.research_budget_state(&context).err(),
            Some(DurableAuthorityError::ResearchJournal(
                ResearchJournalError::Binding
            )),
            "{run}"
        );
    }
    port.close();
}
