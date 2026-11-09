// SPDX-License-Identifier: Apache-2.0
//! Synthetic scoped effects through the real coordinator. No native executor,
//! repository ownership, local model or coding score is claimed here.

use super::*;

fn registry(operations: &[GrantOperation]) -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    for operation in operations {
        let original = registry_for_operation(*operation);
        let mut definition = original.list_tools()[0].clone();
        let name = match operation {
            GrantOperation::WorkspaceWrite => "fixture.write",
            GrantOperation::CommandExecute => "fixture.command",
            _ => panic!("synthetic write/command catalog only"),
        };
        definition.tool_id = ToolId::from_raw(name);
        definition.display_name = name.into();
        definition.description = "Synthetic scoped effect; no native execution".into();
        registry
            .register_tool(Box::new(FixtureTool {
                definition,
                correctable: true,
            }))
            .unwrap();
    }
    registry
}

fn request(operations: &[GrantOperation]) -> RuntimeEndpointRunRequest {
    let mut native =
        super::super::request(profile("endpoint-write-fixture"), &registry(operations));
    native.mode = RuntimeSessionMode::ControlledWrite;
    native.task.objective = "Propose an edit and check the resulting fixture".into();
    native.task.constraints = vec!["Every effect requires an exact decision".into()];
    native.work_packet.mutable_files = vec!["fixture.txt".into()];
    native.work_packet.protected_files.clear();
    // This test uses no native model. Only shared request framing is projected.
    scope_tests::endpoint(&native)
}

fn build(
    operations: &[GrantOperation],
    responses: impl IntoIterator<Item = Response>,
) -> Coordinator {
    let mut coordinator =
        build_registry(request(operations), responses, true, registry(operations)).unwrap();
    coordinator.verifier.expected_results = operations.len();
    coordinator.verifier.disposition = VerifierDisposition::Success;
    coordinator
}

fn approval(coordinator: &mut Coordinator) -> agentmage_kernel_contracts::RuntimeApprovalChallenge {
    let EndpointStep::AwaitingApproval { challenge } =
        coordinator.run_endpoint_until_boundary(None, None).unwrap()
    else {
        panic!("exact effect approval expected")
    };
    challenge
}

fn finish(
    coordinator: &mut Coordinator,
    challenge: &agentmage_kernel_contracts::RuntimeApprovalChallenge,
) -> crate::runtime_coordinator::RuntimeEndpointOutcome {
    let response = decide(challenge, RuntimeApprovalDisposition::Allow);
    let EndpointStep::Complete { outcome } = coordinator
        .run_endpoint_until_boundary(Some(&response), None)
        .unwrap()
    else {
        panic!("terminal expected")
    };
    outcome
}

fn assert_terminal(
    coordinator: &mut Coordinator,
    outcome: &crate::runtime_coordinator::RuntimeEndpointOutcome,
) {
    let calls = coordinator.model.0.calls.load(Ordering::SeqCst);
    let effects = coordinator.tool_boundary.executions.load(Ordering::SeqCst);
    let events = coordinator.events().to_vec();
    let mut sequence = RuntimeEventSequence::new();
    for event in &events {
        sequence.push(event).unwrap();
    }
    assert!(
        matches!(&events.last().unwrap().kind, RuntimeEventKind::RunTerminal { outcome_sha256, .. }
        if outcome_sha256 == &outcome.outcome.outcome_sha256)
    );
    assert_eq!(&terminal(coordinator), outcome);
    assert_eq!(coordinator.events(), events);
    assert_eq!(coordinator.model.0.calls.load(Ordering::SeqCst), calls);
    assert_eq!(
        coordinator.tool_boundary.executions.load(Ordering::SeqCst),
        effects
    );
    assert!(coordinator.outcome().is_none());
}

#[test]
fn endpoint_controlled_write_edit_then_command_uses_separate_approvals_and_verified_terminal() {
    let mut c = build(
        &[
            GrantOperation::WorkspaceWrite,
            GrantOperation::CommandExecute,
        ],
        [
            Response::Effect(GrantOperation::WorkspaceWrite),
            Response::Effect(GrantOperation::CommandExecute),
            Response::Answer,
        ],
    );
    let edit = approval(&mut c);
    assert_eq!(edit.operation, GrantOperation::WorkspaceWrite);
    assert_eq!(c.tool_boundary.executions.load(Ordering::SeqCst), 0);
    let response = decide(&edit, RuntimeApprovalDisposition::Allow);
    let EndpointStep::AwaitingApproval { challenge: command } = c
        .run_endpoint_until_boundary(Some(&response), None)
        .unwrap()
    else {
        panic!("a write decision must not approve a command")
    };
    assert_eq!(command.operation, GrantOperation::CommandExecute);
    assert_ne!(edit.challenge_sha256, command.challenge_sha256);
    assert_eq!(c.tool_boundary.executions.load(Ordering::SeqCst), 1);
    assert!(
        c.run_endpoint_until_boundary(Some(&response), None)
            .is_err()
    );
    assert_eq!(c.tool_boundary.executions.load(Ordering::SeqCst), 1);
    let outcome = finish(&mut c, &command);
    assert_eq!(
        outcome.outcome.state,
        AgentStateKind::Success,
        "{outcome:?}"
    );
    assert_eq!(
        (
            outcome.outcome.model_call_count,
            outcome.outcome.tool_call_count
        ),
        (3, 2)
    );
    assert_eq!(outcome.outcome.receipt_ids.len(), 2);
    assert!(outcome.answer_evidence.is_some());
    assert!(outcome.outcome.answer_evidence.is_none());
    let request = c.request.endpoint().unwrap();
    let claim = c
        .endpoint_claim
        .as_ref()
        .unwrap()
        .claim(c.request.scope().unwrap())
        .unwrap();
    crate::runtime_coordinator::verify_runtime_endpoint_outcome(&outcome, request, Some(&claim))
        .unwrap();
    assert!(
        crate::runtime_coordinator::verify_runtime_endpoint_outcome(&outcome, request, None)
            .is_err()
    );
    let mut tampered = outcome.clone();
    tampered.outcome.receipt_ids.clear();
    assert!(
        crate::runtime_coordinator::verify_runtime_endpoint_outcome(
            &tampered,
            request,
            Some(&claim)
        )
        .is_err()
    );
    assert_terminal(&mut c, &outcome);
}

#[test]
fn endpoint_controlled_write_denial_and_changed_decisions_never_dispatch() {
    for operation in [
        GrantOperation::WorkspaceWrite,
        GrantOperation::CommandExecute,
    ] {
        for changed in 0..5 {
            let mut c = build(
                &[operation],
                [Response::Effect(operation), Response::Answer],
            );
            let challenge = approval(&mut c);
            assert_eq!(challenge.operation, operation);
            let mut response = decide(&challenge, RuntimeApprovalDisposition::Allow);
            match changed {
                0 => response.challenge_sha256 = "0".repeat(64),
                1 => response.run_id = RuntimeRunId::from_raw("foreign-run"),
                2 => response.approval_id = ApprovalId::from_raw("foreign-approval"),
                3 => response.grant_id = Some(GrantId::from_raw("foreign-grant")),
                _ => response.grant_id = None,
            }
            assert!(
                c.run_endpoint_until_boundary(Some(&response), None)
                    .is_err()
            );
            assert_eq!(c.tool_boundary.executions.load(Ordering::SeqCst), 0);
            assert_eq!(c.model.0.calls.load(Ordering::SeqCst), 1);
            let deny = decide(&challenge, RuntimeApprovalDisposition::Deny);
            let EndpointStep::Complete { outcome } =
                c.run_endpoint_until_boundary(Some(&deny), None).unwrap()
            else {
                panic!("denial terminates")
            };
            assert!(!outcome.outcome.state.is_success());
            assert!(outcome.answer_evidence.is_none());
            assert_terminal(&mut c, &outcome);
        }
    }
}

#[test]
fn endpoint_controlled_write_cancellation_and_expiry_win_before_pending_allow() {
    for operation in [
        GrantOperation::WorkspaceWrite,
        GrantOperation::CommandExecute,
    ] {
        for expire in [false, true] {
            let mut c = build(
                &[operation],
                [Response::Effect(operation), Response::Answer],
            );
            let challenge = approval(&mut c);
            let response = decide(&challenge, RuntimeApprovalDisposition::Allow);
            let cancellation = CancellationSignal {
                schema_version: CONTRACT_SCHEMA_VERSION,
                cancellation_id: CancellationId::from_raw("write-stop"),
                correlation_id: CorrelationId::from_raw(derived_id(
                    "correlation",
                    c.request.run_id.as_str(),
                    0,
                )),
                task_id: c.request.task.task_id.clone(),
                reason: CancellationReason::UserRequested,
                requested_by: BoundaryKind::Shell,
            };
            if expire {
                c.clock.0.store(1_000_000, Ordering::SeqCst);
            }
            let step =
                c.run_endpoint_until_boundary(Some(&response), (!expire).then_some(&cancellation));
            let outcome = if expire {
                // The original approval validator refuses an expired response
                // before stateful advancement. A response-free advance then
                // observes the expired original run clock; no grant is renewed.
                assert_eq!(
                    step,
                    Err(RuntimeLoopError::Contract(
                        crate::runtime_coordinator::RuntimeCoordinatorError::ApprovalDenied
                    ))
                );
                assert!(!c.requires_reconciliation());
                assert_eq!(c.tool_boundary.executions.load(Ordering::SeqCst), 0);
                terminal(&mut c)
            } else {
                let EndpointStep::Complete { outcome } = step.unwrap() else {
                    panic!("cancelled")
                };
                outcome
            };
            assert_eq!(
                outcome.outcome.state,
                if expire {
                    AgentStateKind::Exhausted
                } else {
                    AgentStateKind::Cancelled
                }
            );
            assert_eq!(c.tool_boundary.executions.load(Ordering::SeqCst), 0);
            assert_eq!(c.model.0.calls.load(Ordering::SeqCst), 1);
            assert!(outcome.answer_evidence.is_none());
            assert_terminal(&mut c, &outcome);
        }
    }
}

#[test]
fn endpoint_controlled_write_effect_observations_cannot_invent_success_or_retry() {
    for (operation, outcome, change) in [
        (
            GrantOperation::WorkspaceWrite,
            OperationOutcome::Succeeded,
            StateChange::NotChanged,
        ),
        (
            GrantOperation::WorkspaceWrite,
            OperationOutcome::Uncertain,
            StateChange::Uncertain,
        ),
        (
            GrantOperation::WorkspaceWrite,
            OperationOutcome::Failed,
            StateChange::NotChanged,
        ),
        (
            GrantOperation::CommandExecute,
            OperationOutcome::Uncertain,
            StateChange::Uncertain,
        ),
        (
            GrantOperation::CommandExecute,
            OperationOutcome::Succeeded,
            StateChange::Changed,
        ),
        (
            GrantOperation::CommandExecute,
            OperationOutcome::Failed,
            StateChange::NotChanged,
        ),
    ] {
        let mut c = build(
            &[operation],
            [Response::Effect(operation), Response::Answer],
        );
        c.tool_boundary.result_override = Some((outcome, change));
        let challenge = approval(&mut c);
        let response = decide(&challenge, RuntimeApprovalDisposition::Allow);
        let step = c.run_endpoint_until_boundary(Some(&response), None);
        if outcome == OperationOutcome::Succeeded {
            // A lying state-change result is not a terminal observation. The
            // original owner latches reconciliation after the attempted effect.
            assert_eq!(step, Err(RuntimeLoopError::InvalidBoundaryResult));
            assert!(c.requires_reconciliation());
            assert!(c.endpoint_outcome().is_none());
            let events = c.events().to_vec();
            assert_eq!(
                c.run_endpoint_until_boundary(None, None),
                Err(RuntimeLoopError::InvalidBoundaryResult)
            );
            assert_eq!(c.events(), events);
        } else {
            let EndpointStep::Complete { outcome: result } = step.unwrap() else {
                panic!("failed or uncertain effect must stop")
            };
            assert_eq!(
                result.outcome.state,
                if outcome == OperationOutcome::Uncertain {
                    AgentStateKind::Uncertain
                } else {
                    AgentStateKind::Failed
                }
            );
            assert!(result.answer_evidence.is_none());
            assert_terminal(&mut c, &result);
        }
        assert_eq!(c.tool_boundary.executions.load(Ordering::SeqCst), 1);
        assert_eq!(c.model.0.calls.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn endpoint_controlled_write_completion_still_requires_original_independent_checks() {
    for mutation in 1..=7 {
        let mut c = build(
            &[GrantOperation::WorkspaceWrite],
            [
                Response::Effect(GrantOperation::WorkspaceWrite),
                Response::Answer,
            ],
        );
        c.verifier.mutation = mutation;
        let challenge = approval(&mut c);
        let outcome = finish(&mut c, &challenge);
        assert!(
            !outcome.outcome.state.is_success(),
            "verifier mutation {mutation}"
        );
        assert!(outcome.answer_evidence.is_none());
        assert_eq!(c.tool_boundary.executions.load(Ordering::SeqCst), 1);
        assert_terminal(&mut c, &outcome);
    }
    let mut c = build(&[GrantOperation::WorkspaceWrite], [Response::Answer]);
    let outcome = terminal(&mut c);
    assert!(!outcome.outcome.state.is_success());
    assert_eq!(c.tool_boundary.executions.load(Ordering::SeqCst), 0);
    assert_terminal(&mut c, &outcome);
}

#[test]
fn endpoint_controlled_write_does_not_authorize_read_only_or_unregistered_effects() {
    for operation in [
        GrantOperation::WorkspaceWrite,
        GrantOperation::CommandExecute,
    ] {
        let mut selected = request(&[operation]);
        selected.mode = RuntimeSessionMode::EphemeralReadOnly;
        let selected =
            crate::runtime_coordinator::seal_runtime_endpoint_run_request(selected).unwrap();
        assert!(matches!(
            build_registry(
                selected,
                [Response::Effect(operation)],
                true,
                registry(&[operation])
            ),
            Err(RuntimeLoopError::ToolCatalogBinding)
        ));
        let mut c = super::coordinator([Response::Effect(operation), Response::Answer], false);
        let outcome = terminal(&mut c);
        assert!(!outcome.outcome.state.is_success());
        assert_eq!(c.tool_boundary.executions.load(Ordering::SeqCst), 0);
        assert_terminal(&mut c, &outcome);
    }
}

#[test]
fn endpoint_controlled_write_cannot_resume_or_omit_a_required_resource_ceiling() {
    let selected = request(&[GrantOperation::WorkspaceWrite]);
    for resource in selected.work_packet.budgets.iter().map(|b| b.resource) {
        let mut missing = selected.clone();
        missing
            .work_packet
            .budgets
            .retain(|b| b.resource != resource);
        assert!(
            crate::runtime_coordinator::seal_runtime_endpoint_run_request(missing).is_err(),
            "missing {resource:?}"
        );
    }
    let mut continued = selected;
    continued.event_cursor = Some(agentmage_kernel_contracts::RuntimeEventCursor {
        run_id: continued.run_id.clone(),
        event_id: agentmage_kernel_contracts::RuntimeEventId::from_raw("old-event"),
        sequence: 0,
        event_sha256: "a".repeat(64),
    });
    let continued =
        crate::runtime_coordinator::seal_runtime_endpoint_run_request(continued).unwrap();
    assert!(matches!(
        build_registry(
            continued,
            [Response::Effect(GrantOperation::WorkspaceWrite)],
            true,
            registry(&[GrantOperation::WorkspaceWrite])
        ),
        Err(RuntimeLoopError::UnsupportedMode)
    ));
}

#[test]
fn endpoint_controlled_write_refuses_every_other_operation_class_before_model_dispatch() {
    for operation in GrantOperation::ALL {
        if matches!(
            operation,
            GrantOperation::WorkspaceRead
                | GrantOperation::WorkspaceWrite
                | GrantOperation::CommandExecute
        ) {
            continue;
        }
        let catalog = registry_for_operation(operation);
        let mut selected = request(&[GrantOperation::WorkspaceWrite]);
        selected.visible_tools = runtime_tool_references(&catalog).unwrap();
        selected.tool_catalog_sha256 =
            runtime_tool_catalog_sha256(&selected.tool_catalog_id, &selected.visible_tools)
                .unwrap();
        let selected =
            crate::runtime_coordinator::seal_runtime_endpoint_run_request(selected).unwrap();
        assert!(
            matches!(
                build_registry(selected, [Response::Read], true, catalog),
                Err(RuntimeLoopError::ToolCatalogBinding)
            ),
            "{operation:?}"
        );
    }
}

#[test]
fn endpoint_failed_test_observation_keeps_failure_and_demands_fresh_edit_approval() {
    let mut c = build(
        &[
            GrantOperation::CommandExecute,
            GrantOperation::WorkspaceWrite,
        ],
        [
            Response::Effect(GrantOperation::CommandExecute),
            Response::Effect(GrantOperation::WorkspaceWrite),
            Response::Answer,
        ],
    );
    c.tool_boundary.failure_observation = Ok(true);
    c.tool_boundary.feedback_output = true;
    c.tool_boundary.result_override = Some((OperationOutcome::Failed, StateChange::NotChanged));
    let test = approval(&mut c);
    let response = decide(&test, RuntimeApprovalDisposition::Allow);
    let EndpointStep::AwaitingApproval { challenge: edit } = c
        .run_endpoint_until_boundary(Some(&response), None)
        .unwrap()
    else {
        panic!("fresh edit approval")
    };
    assert_eq!(edit.operation, GrantOperation::WorkspaceWrite);
    assert_ne!(test.challenge_sha256, edit.challenge_sha256);
    assert_ne!(test.tool_call_id, edit.tool_call_id);
    assert_eq!(c.tool_boundary.executions.load(Ordering::SeqCst), 1);
    assert_eq!(c.tool_boundary.failure_observation_calls, 1);
    let observed = c.model.0.observed_results.lock().unwrap().clone();
    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0].outcome, OperationOutcome::Failed);
    assert_eq!(observed[0].state_change, StateChange::NotChanged);
    assert_eq!(
        observed[0].output.as_ref().unwrap().bytes,
        br#"{"diagnostic":"synthetic failed assertion"}"#
    );
    assert_eq!(c.tool_results, observed);
    assert!(c.events().iter().any(|e| matches!(&e.kind,
        RuntimeEventKind::ToolFailed { tool_call_id, receipt_id: Some(_), failure_code }
        if tool_call_id == &test.tool_call_id && failure_code == "runtime.tool.failed")));
    assert!(!c.events().iter().any(|e| matches!(&e.kind,
        RuntimeEventKind::ToolCompleted { tool_call_id, .. } if tool_call_id == &test.tool_call_id)));
    let events = c.events().to_vec();
    let resources = c.resource_snapshot().clone();
    assert!(
        c.run_endpoint_until_boundary(Some(&response), None)
            .is_err()
    );
    assert_eq!(c.events(), events);
    assert_eq!(c.resource_snapshot(), &resources);
    c.tool_boundary.result_override = None;
    let outcome = finish(&mut c, &edit);
    // This fixture verifier requires all observed checks to pass. Failed tests
    // remain failed even after a later edit; model prose cannot erase them.
    assert_eq!(outcome.outcome.state, AgentStateKind::Failed);
    assert_eq!(
        (
            outcome.outcome.model_call_count,
            outcome.outcome.tool_call_count
        ),
        (3, 2)
    );
    assert_eq!(outcome.outcome.receipt_ids.len(), 2);
    assert!(outcome.answer_evidence.is_none());
    assert_eq!(
        c.resource_snapshot().cumulative[&BudgetResource::ToolCalls],
        2
    );
    assert_eq!(
        c.resource_snapshot().cumulative[&BudgetResource::ModelCalls],
        3
    );
    assert_terminal(&mut c, &outcome);
}

#[test]
fn endpoint_failure_feedback_is_explicit_and_never_admits_other_outcomes_or_effects() {
    for (operation, outcome, output, permit, expected_hook) in [
        (
            GrantOperation::CommandExecute,
            OperationOutcome::Failed,
            true,
            false,
            1,
        ),
        (
            GrantOperation::CommandExecute,
            OperationOutcome::Failed,
            false,
            true,
            0,
        ),
        (
            GrantOperation::WorkspaceWrite,
            OperationOutcome::Failed,
            true,
            true,
            0,
        ),
        (
            GrantOperation::CommandExecute,
            OperationOutcome::Denied,
            true,
            true,
            0,
        ),
        (
            GrantOperation::CommandExecute,
            OperationOutcome::Cancelled,
            true,
            true,
            0,
        ),
        (
            GrantOperation::CommandExecute,
            OperationOutcome::TimedOut,
            true,
            true,
            0,
        ),
        (
            GrantOperation::CommandExecute,
            OperationOutcome::Uncertain,
            true,
            true,
            0,
        ),
    ] {
        let mut c = build(
            &[operation],
            [Response::Effect(operation), Response::Answer],
        );
        c.tool_boundary.feedback_output = output;
        c.tool_boundary.failure_observation = Ok(permit);
        c.tool_boundary.result_override = Some((
            outcome,
            if outcome == OperationOutcome::Uncertain {
                StateChange::Uncertain
            } else {
                StateChange::NotChanged
            },
        ));
        let ch = approval(&mut c);
        let result = finish(&mut c, &ch);
        assert!(!result.outcome.state.is_success());
        assert_eq!(result.outcome.model_call_count, 1);
        assert_eq!(c.tool_boundary.executions.load(Ordering::SeqCst), 1);
        assert_eq!(c.tool_boundary.failure_observation_calls, expected_hook);
        assert!(c.model.0.observed_results.lock().unwrap().is_empty());
        assert_terminal(&mut c, &result);
    }
}

#[test]
fn endpoint_failed_test_observation_cannot_renew_model_budget_or_mask_classifier_error() {
    for error in [false, true] {
        let operations = [GrantOperation::CommandExecute];
        let mut r = request(&operations);
        r.limits.max_model_calls = 1;
        let r = crate::runtime_coordinator::seal_runtime_endpoint_run_request(r).unwrap();
        let mut c = build_registry(
            r,
            [Response::Effect(operations[0]), Response::Answer],
            true,
            registry(&operations),
        )
        .unwrap();
        c.tool_boundary.feedback_output = true;
        c.tool_boundary.failure_observation = if error {
            Err(RuntimePortFailure::Unavailable)
        } else {
            Ok(true)
        };
        c.tool_boundary.result_override = Some((OperationOutcome::Failed, StateChange::NotChanged));
        let ch = approval(&mut c);
        let response = decide(&ch, RuntimeApprovalDisposition::Allow);
        let result = c.run_endpoint_until_boundary(Some(&response), None);
        if error {
            assert_eq!(
                result,
                Err(RuntimeLoopError::Dependency(
                    RuntimePortFailure::Unavailable
                ))
            );
            assert!(c.requires_reconciliation());
            assert!(c.run_endpoint_until_boundary(None, None).is_err());
        } else {
            let EndpointStep::Complete { outcome } = result.unwrap() else {
                panic!("exhausted")
            };
            assert_eq!(outcome.outcome.state, AgentStateKind::Exhausted);
            assert_eq!(outcome.outcome.receipt_ids.len(), 1);
            assert_eq!(c.tool_results[0].outcome, OperationOutcome::Failed);
            assert_terminal(&mut c, &outcome);
        }
        assert_eq!(c.model.0.calls.load(Ordering::SeqCst), 1);
        assert_eq!(c.tool_boundary.executions.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn endpoint_failed_test_feedback_preserves_cancellation_and_output_exhaustion() {
    struct AfterExecution {
        calls: Arc<AtomicUsize>,
        signal: CancellationSignal,
    }
    impl agentmage_kernel_contracts::ModelCancellationProbe for AfterExecution {
        fn observe(
            &self,
        ) -> Result<Option<CancellationSignal>, agentmage_kernel_contracts::ModelRuntimeFailure>
        {
            Ok((self.calls.load(Ordering::SeqCst) > 0).then(|| self.signal.clone()))
        }
    }
    for cancel in [false, true] {
        let operation = GrantOperation::CommandExecute;
        let mut c = build(
            &[operation],
            [Response::Effect(operation), Response::Answer],
        );
        c.tool_boundary.feedback_output = true;
        c.tool_boundary.failure_observation = Ok(true);
        c.tool_boundary.result_override = Some((OperationOutcome::Failed, StateChange::NotChanged));
        let ch = approval(&mut c);
        let response = decide(&ch, RuntimeApprovalDisposition::Allow);
        let probe = AfterExecution {
            calls: c.tool_boundary.executions.clone(),
            signal: CancellationSignal {
                schema_version: CONTRACT_SCHEMA_VERSION,
                cancellation_id: CancellationId::from_raw("feedback-stop"),
                correlation_id: c.correlation_id().clone(),
                task_id: c.request.task.task_id.clone(),
                reason: CancellationReason::UserRequested,
                requested_by: BoundaryKind::Shell,
            },
        };
        if !cancel {
            let remaining = c.request.limits.max_output_bytes
                - c.resource_snapshot().cumulative[&BudgetResource::OutputBytes];
            c.resources
                .consume(BudgetResource::OutputBytes, remaining - 1)
                .unwrap();
        }
        let EndpointStep::Complete { outcome } = c
            .run_endpoint_until_boundary(
                Some(&response),
                cancel.then_some(&probe as &dyn agentmage_kernel_contracts::ModelCancellationProbe),
            )
            .unwrap()
        else {
            panic!("stop before a fresh dispatch")
        };
        assert_eq!(
            outcome.outcome.state,
            if cancel {
                AgentStateKind::Cancelled
            } else {
                AgentStateKind::Exhausted
            }
        );
        assert_eq!(outcome.outcome.model_call_count, 1);
        assert_eq!(outcome.outcome.tool_call_count, 1);
        assert!(outcome.answer_evidence.is_none());
        assert_eq!(c.tool_boundary.executions.load(Ordering::SeqCst), 1);
        assert!(c.model.0.observed_results.lock().unwrap().is_empty());
        assert_terminal(&mut c, &outcome);
    }
}
