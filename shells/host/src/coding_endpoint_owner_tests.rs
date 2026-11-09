// SPDX-License-Identifier: Apache-2.0
//! Owner lifecycle through real sockets and synthetic source/tool/grant fixtures.

use super::*;
use crate::coding_endpoint_owner::{
    CodingEndpointAdmission, CodingEndpointOwnerError as Error, CodingEndpointRuns,
};
use agentmage_kernel_contracts::{
    BoundaryKind, CancellationId, CancellationReason, CancellationSignal, CorrelationId,
    ModelCancellationProbe, RuntimeApprovalChallenge, RuntimeEventKind,
};

type Runs<'a> = CodingEndpointRuns<'a, ReadBoundary, ContextVerifier, TestClock>;

fn admission<'a, 'source>(
    request: RuntimeEndpointRunRequest,
    transport: &'a LocalChatTransport,
    approval: &'a LocalChatApproval,
    executions: &Arc<AtomicUsize>,
) -> CodingEndpointAdmission<'a, 'source, ReadBoundary, ContextVerifier, TestClock> {
    CodingEndpointAdmission {
        request,
        connection: crate::coding_endpoint::CodingEndpointConnection::Borrowed {
            transport,
            approval,
        },
        registry: registry(),
        sources: &[],
        source_budget: 2048,
        include_usage: true,
        tool_boundary: ReadBoundary(executions.clone()),
        verifier: ContextVerifier(VerifierId::from_raw("context-verifier")),
        clock: TestClock(1000),
    }
}

fn selected_request(transport: &LocalChatTransport, id: &str) -> RuntimeEndpointRunRequest {
    let mut request = request();
    request.run_id = RuntimeRunId::from_raw(id);
    request.model_selection = transport.model_selection("fixture-model");
    seal_runtime_endpoint_run_request(request).unwrap()
}

fn allow(challenge: &RuntimeApprovalChallenge) -> RuntimeApprovalResponse {
    RuntimeApprovalResponse {
        schema_version: CONTRACT_SCHEMA_VERSION,
        run_id: challenge.run_id.clone(),
        approval_id: challenge.approval_id.clone(),
        disposition: RuntimeApprovalDisposition::Allow,
        challenge_sha256: challenge.challenge_sha256.clone(),
        grant_id: Some(challenge.proposed_grant_id.clone()),
        selection: None,
    }
}

#[test]
fn endpoint_owner_retains_context_approval_terminal_and_spent_identity_across_advances() {
    let (listener, endpoint) = listener();
    let transport = transport(endpoint.clone());
    let approval = LocalChatApproval::for_endpoint(endpoint);
    let request = selected_request(&transport, "owned-endpoint-run");
    let objective = sha256(request.task.objective.as_bytes());
    let worker = thread::spawn(move || {
        for turn in 1..=2 {
            let mut socket = accept(&listener);
            let body = read_context_request(&mut socket);
            let user: serde_json::Value =
                serde_json::from_str(body["messages"][1]["content"].as_str().unwrap()).unwrap();
            assert_eq!(user["turn"], turn);
            assert_eq!(
                user["untrusted_tool_results"].as_array().unwrap().len(),
                turn - 1
            );
            if turn == 1 {
                assert_eq!(
                    user["untrusted_source_excerpts"].as_array().unwrap().len(),
                    1
                );
                respond(&mut socket, &read_proposal(), "stop", true);
            } else {
                let claim = serde_json::to_string(&CodingCompletionCandidate {
                    schema_version: 1,
                    objective_sha256: objective.clone(),
                    terminal_claim: CodingTerminalClaim::NoOp,
                    summary: "fixture function returns 42".into(),
                    checks_not_run: vec!["project tests".into()],
                    residual_risks: vec![],
                })
                .unwrap();
                respond(&mut socket, &claim, "stop", true);
            }
        }
        listener
    });
    let executions = Arc::new(AtomicUsize::new(0));
    let mut runs = Runs::default();
    {
        // The owner retains selected source bytes, never this caller's buffer.
        let sources = sources();
        let mut input = admission(request.clone(), &transport, &approval, &executions);
        input.sources = &sources;
        runs.admit(input).unwrap();
    }
    assert!(runs.events(&request).unwrap().is_empty());
    assert_eq!(runs.release(&request), Err(Error::StillRunning));
    assert_eq!(
        runs.admit(admission(
            request.clone(),
            &transport,
            &approval,
            &executions
        )),
        Err(Error::AlreadyAdmitted)
    );
    let first = runs.advance(&request, None, None).unwrap();
    let RuntimeEndpointCoordinatorStep::AwaitingApproval { challenge } = &first else {
        panic!("approval expected: {first:?}")
    };
    let events = runs.events(&request).unwrap().to_vec();
    assert_eq!(runs.advance(&request, None, None).unwrap(), first);
    assert_eq!(runs.events(&request).unwrap(), events);
    assert_eq!(runs.release(&request), Err(Error::StillRunning));
    assert_eq!(executions.load(Ordering::SeqCst), 0);
    let mut invalid = allow(challenge);
    invalid.challenge_sha256 = "0".repeat(64);
    assert!(matches!(
        runs.advance(&request, Some(&invalid), None),
        Err(Error::Coordinator(_))
    ));
    assert_eq!(runs.advance(&request, None, None).unwrap(), first);
    assert_eq!(runs.events(&request).unwrap(), events);
    let terminal = runs
        .advance(&request, Some(&allow(challenge)), None)
        .unwrap();
    let RuntimeEndpointCoordinatorStep::Complete { outcome } = &terminal else {
        panic!("terminal expected")
    };
    assert_eq!(outcome.outcome.state, AgentStateKind::NoOp);
    assert_eq!(outcome.outcome.model_call_count, 2);
    assert_eq!(outcome.outcome.tool_call_count, 1);
    assert!(outcome.answer_evidence.is_some());
    assert_eq!(executions.load(Ordering::SeqCst), 1);
    let events = runs.events(&request).unwrap().to_vec();
    assert!(matches!(
        events.last().unwrap().kind,
        RuntimeEventKind::RunTerminal { .. }
    ));
    assert_eq!(runs.advance(&request, None, None).unwrap(), terminal);
    assert_eq!(runs.events(&request).unwrap(), events);
    runs.release(&request).unwrap();
    assert_eq!(runs.advance(&request, None, None), Err(Error::NotHeld));
    assert_eq!(runs.events(&request), Err(Error::NotHeld));
    assert_eq!(runs.release(&request), Err(Error::NotHeld));
    assert_eq!(
        runs.admit(admission(request, &transport, &approval, &executions)),
        Err(Error::AlreadyAdmitted)
    );
    assert_eq!(
        worker.join().unwrap().accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn endpoint_owner_rejects_foreign_resealed_and_unsealed_requests_without_mutation() {
    let (listener, endpoint) = listener();
    let transport = transport(endpoint.clone());
    let approval = LocalChatApproval::for_endpoint(endpoint);
    let executions = Arc::new(AtomicUsize::new(0));
    let request = selected_request(&transport, "pinned-owner-run");
    let mut runs = Runs::default();
    runs.admit(admission(
        request.clone(),
        &transport,
        &approval,
        &executions,
    ))
    .unwrap();
    let mut changed = request.clone();
    changed.task.objective.push_str(" changed");
    assert_eq!(runs.advance(&changed, None, None), Err(Error::Invalid));
    assert_eq!(runs.events(&changed), Err(Error::Invalid));
    assert_eq!(runs.release(&changed), Err(Error::Invalid));
    let changed = seal_runtime_endpoint_run_request(changed).unwrap();
    assert_eq!(runs.advance(&changed, None, None), Err(Error::NotHeld));
    assert_eq!(runs.events(&changed), Err(Error::NotHeld));
    assert_eq!(runs.release(&changed), Err(Error::NotHeld));
    assert_eq!(
        runs.admit(admission(changed, &transport, &approval, &executions)),
        Err(Error::AlreadyAdmitted)
    );
    let foreign = selected_request(&transport, "foreign-owner-run");
    assert_eq!(runs.advance(&foreign, None, None), Err(Error::NotHeld));
    assert!(runs.events(&request).unwrap().is_empty());
    assert_eq!(executions.load(Ordering::SeqCst), 0);
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn endpoint_owner_latches_boundary_errors_and_preserves_events_without_redispatch() {
    struct Boundary(ReadBoundary, bool);
    impl RuntimeToolBoundary for Boundary {
        fn evaluate(
            &mut self,
            _: &RuntimeRunRequest,
            _: &RuntimeOperationId,
            _: &ToolDefinition,
            _: &ToolCall,
            _: u64,
        ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
            panic!("native family")
        }
        fn resolve(
            &mut self,
            _: &RuntimeRunRequest,
            _: &RuntimeApprovalChallenge,
            _: &RuntimeApprovalResponse,
            _: &ToolDefinition,
            _: &ToolCall,
            _: u64,
        ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
            panic!("native family")
        }
        fn execute(
            &mut self,
            _: &RuntimeRunRequest,
            _: &RuntimePermissionEvaluation,
            _: &ToolDefinition,
            _: &ToolCall,
            _: Option<&dyn ModelCancellationProbe>,
        ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
            panic!("native family")
        }
        fn evaluate_scoped(
            &mut self,
            scope: RuntimeRunScope<'_>,
            operation: &RuntimeOperationId,
            definition: &ToolDefinition,
            call: &ToolCall,
            now: u64,
        ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
            self.0
                .evaluate_scoped(scope, operation, definition, call, now)
        }
        fn resolve_scoped(
            &mut self,
            scope: RuntimeRunScope<'_>,
            challenge: &RuntimeApprovalChallenge,
            response: &RuntimeApprovalResponse,
            definition: &ToolDefinition,
            call: &ToolCall,
            now: u64,
        ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
            self.0
                .resolve_scoped(scope, challenge, response, definition, call, now)
        }
        fn execute_scoped(
            &mut self,
            scope: RuntimeRunScope<'_>,
            evaluation: &RuntimePermissionEvaluation,
            definition: &ToolDefinition,
            call: &ToolCall,
            cancellation: Option<&dyn ModelCancellationProbe>,
        ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
            let result =
                self.0
                    .execute_scoped(scope, evaluation, definition, call, cancellation)?;
            if self.1 {
                Err(RuntimePortFailure::Unavailable)
            } else {
                Ok(result)
            }
        }
    }

    for verifier_failure in [false, true] {
        let (listener, endpoint) = listener();
        let transport = transport(endpoint.clone());
        let approval = LocalChatApproval::for_endpoint(endpoint);
        let worker = thread::spawn(move || {
            let mut socket = accept(&listener);
            read_context_request(&mut socket);
            respond(&mut socket, &read_proposal(), "stop", true);
            drop(socket);
            if verifier_failure {
                let mut socket = accept(&listener);
                read_context_request(&mut socket);
                respond(&mut socket, "fixture verifier is unavailable", "stop", true);
            }
            listener
        });
        struct UnavailableVerifier(VerifierId, Arc<AtomicUsize>);
        impl RuntimeVerifierPort for UnavailableVerifier {
            fn verifier_id(&self) -> &VerifierId {
                &self.0
            }
            fn verify(
                &mut self,
                _: RuntimeVerificationInput<'_>,
            ) -> Result<VerifierCandidate, RuntimePortFailure> {
                panic!("native family")
            }
            fn verify_completion(
                &mut self,
                _: RuntimeCompletionVerificationInput<'_>,
            ) -> Result<VerifierCandidate, RuntimePortFailure> {
                self.1.fetch_add(1, Ordering::SeqCst);
                Err(RuntimePortFailure::Unavailable)
            }
        }
        let executions = Arc::new(AtomicUsize::new(0));
        let verifier_calls = Arc::new(AtomicUsize::new(0));
        let request = selected_request(&transport, "faulted-owner-run");
        let mut runs = CodingEndpointRuns::default();
        runs.admit(CodingEndpointAdmission {
            request: request.clone(),
            connection: crate::coding_endpoint::CodingEndpointConnection::Borrowed {
                transport: &transport,
                approval: &approval,
            },
            registry: registry(),
            sources: &[],
            source_budget: 2048,
            include_usage: true,
            tool_boundary: Boundary(ReadBoundary(executions.clone()), !verifier_failure),
            verifier: UnavailableVerifier(
                VerifierId::from_raw("unavailable-verifier"),
                verifier_calls.clone(),
            ),
            clock: TestClock(1000),
        })
        .unwrap();
        let RuntimeEndpointCoordinatorStep::AwaitingApproval { challenge } =
            runs.advance(&request, None, None).unwrap()
        else {
            panic!("approval expected")
        };
        if verifier_failure {
            let RuntimeEndpointCoordinatorStep::Complete { outcome } = runs
                .advance(&request, Some(&allow(&challenge)), None)
                .unwrap()
            else {
                panic!("terminal verifier refusal")
            };
            assert_eq!(outcome.outcome.state, AgentStateKind::Failed);
            assert_eq!(
                outcome.outcome.unresolved_codes,
                ["runtime.port.unavailable"]
            );
            assert_eq!(outcome.outcome.model_call_count, 2);
            assert_eq!(outcome.outcome.tool_call_count, 1);
            assert!(outcome.outcome.output.is_none());
            assert!(outcome.answer_evidence.is_none());
            let events = runs.events(&request).unwrap().to_vec();
            assert_eq!(
                events
                    .iter()
                    .filter(|e| matches!(e.kind, RuntimeEventKind::RunTerminal { .. }))
                    .count(),
                1
            );
            let mut sequence = agentmage_kernel_engine::runtime_event::RuntimeEventSequence::new();
            for event in &events {
                sequence.push(event).unwrap();
            }
            assert_eq!(
                runs.advance(&request, None, None).unwrap(),
                RuntimeEndpointCoordinatorStep::Complete { outcome }
            );
            assert_eq!(runs.events(&request).unwrap(), events);
            assert_eq!(executions.load(Ordering::SeqCst), 1);
            assert_eq!(verifier_calls.load(Ordering::SeqCst), 1);
            runs.release(&request).unwrap();
            assert_eq!(runs.advance(&request, None, None), Err(Error::NotHeld));
            assert_eq!(
                worker.join().unwrap().accept().unwrap_err().kind(),
                std::io::ErrorKind::WouldBlock
            );
            continue;
        }
        assert_eq!(verifier_calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            runs.advance(&request, Some(&allow(&challenge)), None),
            Err(Error::Coordinator(
                agentmage_kernel_engine::runtime_loop::RuntimeLoopError::Dependency(
                    RuntimePortFailure::Unavailable
                )
            ))
        );
        let events = runs.events(&request).unwrap().to_vec();
        assert!(
            !events
                .iter()
                .any(|e| matches!(e.kind, RuntimeEventKind::RunTerminal { .. }))
        );
        assert_eq!(executions.load(Ordering::SeqCst), 1);
        assert_eq!(runs.advance(&request, None, None), Err(Error::Faulted));
        assert!(matches!(
            runs.release_with_tool_boundary(&request),
            Err(Error::Faulted)
        ));
        assert_eq!(
            runs.advance(&request, Some(&allow(&challenge)), None),
            Err(Error::Faulted)
        );
        assert_eq!(runs.events(&request).unwrap(), events);
        let inert = |request| CodingEndpointAdmission {
            request,
            connection: crate::coding_endpoint::CodingEndpointConnection::Borrowed {
                transport: &transport,
                approval: &approval,
            },
            registry: registry(),
            sources: &[],
            source_budget: 2048,
            include_usage: true,
            tool_boundary: Boundary(ReadBoundary(executions.clone()), !verifier_failure),
            verifier: UnavailableVerifier(
                VerifierId::from_raw("unavailable-verifier"),
                verifier_calls.clone(),
            ),
            clock: TestClock(1000),
        };
        for i in 0..3 {
            runs.admit(inert(selected_request(
                &transport,
                &format!("beside-faulted-run-{i}"),
            )))
            .unwrap();
        }
        let fifth = selected_request(&transport, "after-faulted-release");
        assert_eq!(runs.admit(inert(fifth.clone())), Err(Error::Capacity));
        assert_eq!(runs.events(&request).unwrap(), events);
        runs.release(&request).unwrap();
        runs.admit(inert(fifth.clone())).unwrap();
        assert!(runs.events(&fifth).unwrap().is_empty());
        assert_eq!(
            runs.admit(inert(request.clone())),
            Err(Error::AlreadyAdmitted)
        );
        assert_eq!(runs.advance(&request, None, None), Err(Error::NotHeld));
        assert_eq!(executions.load(Ordering::SeqCst), 1);
        assert_eq!(
            worker.join().unwrap().accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
}

#[test]
fn endpoint_owner_caught_clock_and_verifier_panics_keep_events_but_never_advance_again() {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::atomic::AtomicBool;

    struct PanicClock {
        inner: TestClock,
        armed: Arc<AtomicBool>,
        calls: Arc<AtomicUsize>,
    }
    impl RuntimeClock for PanicClock {
        fn now_epoch_ms(&mut self) -> Result<u64, RuntimePortFailure> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            assert!(
                !self.armed.load(Ordering::SeqCst),
                "owner fixture clock panic"
            );
            self.inner.now_epoch_ms()
        }
    }
    struct PanicVerifier(VerifierId, Arc<AtomicUsize>);
    impl RuntimeVerifierPort for PanicVerifier {
        fn verifier_id(&self) -> &VerifierId {
            &self.0
        }
        fn verify(
            &mut self,
            _: RuntimeVerificationInput<'_>,
        ) -> Result<VerifierCandidate, RuntimePortFailure> {
            panic!("native family")
        }
        fn verify_completion(
            &mut self,
            _: RuntimeCompletionVerificationInput<'_>,
        ) -> Result<VerifierCandidate, RuntimePortFailure> {
            self.1.fetch_add(1, Ordering::SeqCst);
            panic!("owner fixture verifier panic")
        }
    }
    for panic_in_clock in [true, false] {
        let (listener, endpoint) = listener();
        let transport = transport(endpoint.clone());
        let approval = LocalChatApproval::for_endpoint(endpoint);
        let worker = thread::spawn(move || {
            let mut socket = accept(&listener);
            read_context_request(&mut socket);
            respond(&mut socket, &read_proposal(), "stop", true);
            drop(socket);
            if !panic_in_clock {
                let mut socket = accept(&listener);
                read_context_request(&mut socket);
                respond(&mut socket, "fixture completion", "stop", true);
            }
            listener
        });
        let executions = Arc::new(AtomicUsize::new(0));
        let clock_armed = Arc::new(AtomicBool::new(false));
        let clock_calls = Arc::new(AtomicUsize::new(0));
        let verifier_calls = Arc::new(AtomicUsize::new(0));
        let request = selected_request(&transport, "panicking-owner-run");
        let mut runs = CodingEndpointRuns::default();
        runs.admit(CodingEndpointAdmission {
            request: request.clone(),
            connection: crate::coding_endpoint::CodingEndpointConnection::Borrowed {
                transport: &transport,
                approval: &approval,
            },
            registry: registry(),
            sources: &[],
            source_budget: 2048,
            include_usage: true,
            tool_boundary: ReadBoundary(executions.clone()),
            verifier: PanicVerifier(
                VerifierId::from_raw("panic-verifier"),
                verifier_calls.clone(),
            ),
            clock: PanicClock {
                inner: TestClock(1000),
                armed: clock_armed.clone(),
                calls: clock_calls.clone(),
            },
        })
        .unwrap();
        let RuntimeEndpointCoordinatorStep::AwaitingApproval { challenge } =
            runs.advance(&request, None, None).unwrap()
        else {
            panic!("approval expected")
        };
        assert_eq!(executions.load(Ordering::SeqCst), 0);
        clock_armed.store(panic_in_clock, Ordering::SeqCst);
        let response = allow(&challenge);
        let caught = catch_unwind(AssertUnwindSafe(|| {
            runs.advance(&request, Some(&response), None)
        }))
        .unwrap_err();
        let message = caught
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| caught.downcast_ref::<&str>().copied());
        assert_eq!(
            message,
            Some(if panic_in_clock {
                "owner fixture clock panic"
            } else {
                "owner fixture verifier panic"
            })
        );
        // Disarm the dependency: refusal must come from the retained owner.
        clock_armed.store(false, Ordering::SeqCst);
        let events = runs.events(&request).unwrap().to_vec();
        let clock_count = clock_calls.load(Ordering::SeqCst);
        assert!(
            !events
                .iter()
                .any(|event| matches!(event.kind, RuntimeEventKind::RunTerminal { .. }))
        );
        for response in [None, Some(&response)] {
            assert_eq!(runs.advance(&request, response, None), Err(Error::Faulted));
            assert_eq!(runs.events(&request).unwrap(), events);
        }
        assert!(matches!(
            runs.release_with_tool_boundary(&request),
            Err(Error::Faulted)
        ));
        assert_eq!(runs.events(&request).unwrap(), events);
        assert_eq!(clock_calls.load(Ordering::SeqCst), clock_count);
        let expected_executions = usize::from(!panic_in_clock);
        assert_eq!(executions.load(Ordering::SeqCst), expected_executions);
        assert_eq!(verifier_calls.load(Ordering::SeqCst), expected_executions);
        runs.release(&request).unwrap();
        assert_eq!(runs.advance(&request, None, None), Err(Error::NotHeld));
        assert_eq!(
            worker.join().unwrap().accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
}

#[test]
fn endpoint_owner_terminal_return_reuses_only_the_tool_and_keeps_both_run_identities_spent() {
    let (listener, endpoint) = listener();
    let transport = transport(endpoint.clone());
    let approval = LocalChatApproval::for_endpoint(endpoint);
    let request = selected_request(&transport, "terminal-return-first");
    let objective = sha256(request.task.objective.as_bytes());
    let worker = thread::spawn(move || {
        for turn in 0..4 {
            let mut socket = accept(&listener);
            let body = read_context_request(&mut socket);
            let user: serde_json::Value =
                serde_json::from_str(body["messages"][1]["content"].as_str().unwrap()).unwrap();
            assert_eq!(user["turn"], turn % 2 + 1);
            assert_eq!(
                user["untrusted_tool_results"].as_array().unwrap().len(),
                turn % 2
            );
            if turn % 2 == 0 {
                respond(&mut socket, &read_proposal(), "stop", true);
            } else {
                let claim = serde_json::to_string(&CodingCompletionCandidate {
                    schema_version: 1,
                    objective_sha256: objective.clone(),
                    terminal_claim: CodingTerminalClaim::NoOp,
                    summary: "fixture function returns 42".into(),
                    checks_not_run: vec!["project tests".into()],
                    residual_risks: vec![],
                })
                .unwrap();
                respond(&mut socket, &claim, "stop", true);
            }
        }
        listener
    });
    let executions = Arc::new(AtomicUsize::new(0));
    let mut runs = Runs::default();
    let mut next_boundary = None;
    let mut previous_response = None;
    for ordinal in 0..2 {
        let current = selected_request(
            &transport,
            if ordinal == 0 {
                "terminal-return-first"
            } else {
                "terminal-return-second"
            },
        );
        let mut input = admission(current.clone(), &transport, &approval, &executions);
        if let Some(boundary) = next_boundary.take() {
            input.tool_boundary = boundary;
        }
        runs.admit(input).unwrap();
        assert!(matches!(
            runs.release_with_tool_boundary(&current),
            Err(Error::StillRunning)
        ));
        let first = runs.advance(&current, None, None).unwrap();
        let RuntimeEndpointCoordinatorStep::AwaitingApproval { challenge } = &first else {
            panic!("fresh approval expected")
        };
        let events = runs.events(&current).unwrap().to_vec();
        assert!(matches!(
            runs.release_with_tool_boundary(&current),
            Err(Error::StillRunning)
        ));
        let mut changed = current.clone();
        changed.task.objective.push_str(" changed");
        assert!(matches!(
            runs.release_with_tool_boundary(&changed),
            Err(Error::Invalid)
        ));
        changed = seal_runtime_endpoint_run_request(changed).unwrap();
        assert!(matches!(
            runs.release_with_tool_boundary(&changed),
            Err(Error::NotHeld)
        ));
        if let Some(old) = &previous_response {
            assert!(runs.advance(&current, Some(old), None).is_err());
        }
        assert_eq!(runs.advance(&current, None, None).unwrap(), first);
        assert_eq!(runs.events(&current).unwrap(), events);
        let response = allow(challenge);
        assert!(matches!(
            runs.advance(&current, Some(&response), None).unwrap(),
            RuntimeEndpointCoordinatorStep::Complete { .. }
        ));
        assert_eq!(executions.load(Ordering::SeqCst), ordinal + 1);
        let boundary = runs.release_with_tool_boundary(&current).unwrap();
        assert!(Arc::ptr_eq(&boundary.0, &executions));
        assert_eq!(executions.load(Ordering::SeqCst), ordinal + 1);
        assert_eq!(runs.events(&current), Err(Error::NotHeld));
        assert_eq!(runs.advance(&current, None, None), Err(Error::NotHeld));
        assert!(matches!(
            runs.release_with_tool_boundary(&current),
            Err(Error::NotHeld)
        ));
        assert_eq!(
            runs.admit(admission(current, &transport, &approval, &executions)),
            Err(Error::AlreadyAdmitted)
        );
        next_boundary = Some(boundary);
        previous_response = Some(response);
    }
    assert_eq!(
        worker.join().unwrap().accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn endpoint_owner_capacity_and_lifetime_tombstones_never_evict_or_renew_a_run() {
    let (listener, endpoint) = listener();
    let transport = transport(endpoint.clone());
    let approval = LocalChatApproval::for_endpoint(endpoint);
    let executions = Arc::new(AtomicUsize::new(0));
    let mut runs = Runs::default();
    let mut requests = vec![];
    // Low output headroom exhausts before socket entry. No cancellation fixture
    // is needed and a release never creates a fresh invocation of the same run.
    for i in 0..4 {
        let mut request = selected_request(&transport, &format!("capacity-run-{i}"));
        request.limits.max_output_bytes = 1;
        let request = seal_runtime_endpoint_run_request(request).unwrap();
        runs.admit(admission(
            request.clone(),
            &transport,
            &approval,
            &executions,
        ))
        .unwrap();
        requests.push(request);
    }
    let fifth = selected_request(&transport, "capacity-run-4");
    assert_eq!(
        runs.admit(admission(fifth.clone(), &transport, &approval, &executions)),
        Err(Error::Capacity)
    );
    assert_eq!(runs.advance(&fifth, None, None), Err(Error::NotHeld));
    for request in &requests {
        let RuntimeEndpointCoordinatorStep::Complete { outcome } =
            runs.advance(request, None, None).unwrap()
        else {
            panic!("exhaustion expected")
        };
        assert_eq!(outcome.outcome.state, AgentStateKind::Exhausted);
        assert_eq!(outcome.outcome.model_call_count, 0);
    }
    // Completed entries still consume a held slot until explicitly released.
    assert_eq!(
        runs.admit(admission(fifth, &transport, &approval, &executions)),
        Err(Error::Capacity)
    );
    for request in &requests {
        runs.release(request).unwrap();
    }
    for i in 4..256 {
        let mut request = selected_request(&transport, &format!("capacity-run-{i}"));
        request.limits.max_output_bytes = 1;
        let request = seal_runtime_endpoint_run_request(request).unwrap();
        runs.admit(admission(
            request.clone(),
            &transport,
            &approval,
            &executions,
        ))
        .unwrap();
        assert!(matches!(
            runs.advance(&request, None, None).unwrap(),
            RuntimeEndpointCoordinatorStep::Complete { .. }
        ));
        runs.release(&request).unwrap();
    }
    let beyond = selected_request(&transport, "capacity-run-256");
    assert_eq!(
        runs.admit(admission(beyond, &transport, &approval, &executions)),
        Err(Error::Capacity)
    );
    assert_eq!(
        runs.admit(admission(
            requests[0].clone(),
            &transport,
            &approval,
            &executions
        )),
        Err(Error::AlreadyAdmitted)
    );
    assert_eq!(executions.load(Ordering::SeqCst), 0);
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn endpoint_owner_failed_admission_is_inert_and_original_cancellation_is_terminal() {
    let (listener, endpoint) = listener();
    let transport = transport(endpoint.clone());
    let approval = LocalChatApproval::for_endpoint(endpoint);
    let executions = Arc::new(AtomicUsize::new(0));
    let request = selected_request(&transport, "cancelled-owner-run");
    let mut runs = Runs::default();
    let mut invalid = admission(request.clone(), &transport, &approval, &executions);
    invalid.source_budget = 0;
    assert_eq!(
        runs.admit(invalid),
        Err(Error::Context(RuntimePortFailure::Invalid))
    );
    assert_eq!(runs.events(&request), Err(Error::NotHeld));
    runs.admit(admission(
        request.clone(),
        &transport,
        &approval,
        &executions,
    ))
    .unwrap();
    let cancellation = CancellationSignal {
        schema_version: CONTRACT_SCHEMA_VERSION,
        cancellation_id: CancellationId::from_raw("owner-cancellation"),
        correlation_id: CorrelationId::from_raw(format!(
            "correlation:{}",
            &sha256(format!("{}:0", request.run_id.as_str()).as_bytes())[..32]
        )),
        task_id: request.task.task_id.clone(),
        reason: CancellationReason::UserRequested,
        requested_by: BoundaryKind::Shell,
    };
    let terminal = runs.advance(&request, None, Some(&cancellation)).unwrap();
    let RuntimeEndpointCoordinatorStep::Complete { outcome } = &terminal else {
        panic!("cancellation expected")
    };
    assert_eq!(outcome.outcome.state, AgentStateKind::Cancelled);
    assert_eq!(outcome.outcome.model_call_count, 0);
    assert_eq!(runs.advance(&request, None, None).unwrap(), terminal);
    let events = runs.events(&request).unwrap();
    assert!(!events.iter().any(|e| matches!(
        e.kind,
        RuntimeEventKind::PermissionRequested { .. } | RuntimeEventKind::PermissionDecided { .. }
    )));
    assert_eq!(executions.load(Ordering::SeqCst), 0);
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
fn shared_endpoint_owner_keeps_original_config_across_settings_drop_and_releases_it() {
    use crate::coding_endpoint::CodingEndpointConnection;
    let (listener, endpoint) = listener();
    let transport = Arc::new(transport(endpoint.clone()));
    let approval = Arc::new(LocalChatApproval::for_endpoint(endpoint));
    let transport_weak = Arc::downgrade(&transport);
    let approval_weak = Arc::downgrade(&approval);
    let request = selected_request(&transport, "shared-owner-run");
    let objective = sha256(request.task.objective.as_bytes());
    let worker = thread::spawn(move || {
        for turn in 1..=2 {
            let mut socket = accept(&listener);
            let body = read_context_request(&mut socket);
            let user: serde_json::Value =
                serde_json::from_str(body["messages"][1]["content"].as_str().unwrap()).unwrap();
            assert_eq!(user["turn"], turn);
            if turn == 1 {
                respond(&mut socket, &read_proposal(), "stop", true);
            } else {
                assert_eq!(user["untrusted_tool_results"].as_array().unwrap().len(), 1);
                let claim = serde_json::to_string(&CodingCompletionCandidate {
                    schema_version: 1,
                    objective_sha256: objective.clone(),
                    terminal_claim: CodingTerminalClaim::NoOp,
                    summary: "fixture function returns 42".into(),
                    checks_not_run: vec!["project tests".into()],
                    residual_risks: vec![],
                })
                .unwrap();
                respond(&mut socket, &claim, "stop", true);
            }
        }
        listener
    });
    let executions = Arc::new(AtomicUsize::new(0));
    // This service-owned registry is genuinely independent of caller lifetimes.
    let mut runs: Runs<'static> = Runs::default();
    runs.admit(CodingEndpointAdmission {
        request: request.clone(),
        connection: CodingEndpointConnection::Shared {
            transport: Arc::clone(&transport),
            approval: Arc::clone(&approval),
        },
        registry: registry(),
        sources: &sources(),
        source_budget: 2048,
        include_usage: true,
        tool_boundary: ReadBoundary(executions.clone()),
        verifier: ContextVerifier(VerifierId::from_raw("context-verifier")),
        clock: TestClock(1000),
    })
    .unwrap();
    drop(transport);
    drop(approval);
    assert_eq!(transport_weak.strong_count(), 1);
    assert_eq!(approval_weak.strong_count(), 1);
    let first = runs.advance(&request, None, None).unwrap();
    let RuntimeEndpointCoordinatorStep::AwaitingApproval { challenge } = &first else {
        panic!("approval expected")
    };
    assert_eq!(executions.load(Ordering::SeqCst), 0);
    let events = runs.events(&request).unwrap().to_vec();
    // A newly configured destination cannot reseal this retained run's identity.
    let (replacement_listener, replacement_endpoint) = super::listener();
    let replacement = super::transport(replacement_endpoint);
    let foreign = selected_request(&replacement, request.run_id.as_str());
    assert_eq!(runs.advance(&foreign, None, None), Err(Error::NotHeld));
    assert_eq!(runs.events(&request).unwrap(), events);
    assert_eq!(runs.advance(&request, None, None).unwrap(), first);
    let terminal = runs
        .advance(&request, Some(&allow(challenge)), None)
        .unwrap();
    let RuntimeEndpointCoordinatorStep::Complete { outcome } = &terminal else {
        panic!("terminal expected")
    };
    assert_eq!(outcome.outcome.state, AgentStateKind::NoOp);
    assert_eq!(outcome.outcome.model_call_count, 2);
    assert_eq!(outcome.outcome.tool_call_count, 1);
    assert!(outcome.answer_evidence.is_some());
    assert_eq!(executions.load(Ordering::SeqCst), 1);
    assert_eq!(runs.advance(&request, None, None).unwrap(), terminal);
    assert_eq!(transport_weak.strong_count(), 1);
    runs.release(&request).unwrap();
    assert!(transport_weak.upgrade().is_none());
    assert!(approval_weak.upgrade().is_none());
    assert_eq!(runs.advance(&request, None, None), Err(Error::NotHeld));
    assert_eq!(
        replacement_listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert_eq!(
        worker.join().unwrap().accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}
