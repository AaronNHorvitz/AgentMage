//! Synthetic coordinator fixtures, not native retrieval or model qualification.

use super::*;
use crate::runtime_loop::{operation_allowed_for_mode, valid_runtime_state_change};

pub(super) fn candidate(bytes: &[u8]) -> RuntimeToolArtifactCandidate {
    RuntimeToolArtifactCandidate {
        kind: RuntimeArtifactKind::Report,
        media_type: "application/json".into(),
        bytes: bytes.to_vec(),
    }
}

pub(super) fn prepare(
    builder: &mut dyn RuntimeToolTerminalBuilder,
    execution: &mut RuntimeToolExecution,
    mutation: u8,
) -> Result<(), RuntimePortFailure> {
    let first = candidate(b"original full bytes");
    if mutation == 14 {
        assert!(
            builder
                .prepare_artifacts(
                    &execution.receipt_id,
                    &execution.receipt_sha256,
                    &vec![first; 65],
                )
                .is_err()
        );
        return Ok(()); // Deliberately ignore refusal; sealing must still reject.
    }
    let references = builder.prepare_artifacts(
        &execution.receipt_id,
        &execution.receipt_sha256,
        std::slice::from_ref(&first),
    )?;
    assert_eq!(references.len(), 1);
    let second = candidate(&serde_json::to_vec(&references[0]).unwrap());
    if mutation == 9 {
        assert!(
            builder
                .prepare_artifacts(&execution.receipt_id, &execution.receipt_sha256, &[],)
                .is_err()
        );
        execution.artifact_candidates = vec![first];
        return Ok(());
    }
    if mutation == 13 {
        assert!(
            builder
                .prepare_artifacts(
                    &ReceiptId::from_raw("foreign-receipt"),
                    &execution.receipt_sha256,
                    std::slice::from_ref(&second),
                )
                .is_err()
        );
        execution.artifact_candidates = vec![first];
        return Ok(());
    }
    let second_refs = builder.prepare_artifacts(
        &execution.receipt_id,
        &execution.receipt_sha256,
        std::slice::from_ref(&second),
    )?;
    assert_eq!(second_refs.len(), 1);
    // A closed synthetic output demonstrates dependent references before result
    // hashing. It is not a public research ToolResult or a canonical source proof.
    let bytes = serde_json::to_vec(&second_refs[0]).unwrap();
    let mut output = payload("runtime.tool-result", &bytes);
    output.media_type = "application/json".into();
    if mutation == 19 {
        output = payload(
            "runtime.tool-result",
            &vec![b'r'; MAX_RUNTIME_INLINE_OUTPUT_BYTES + 1],
        );
    }
    execution.result.output = Some(output);
    execution.result_output_kind = Some(RuntimeArtifactKind::Report);
    execution.artifact_candidates = vec![first, second];
    match mutation {
        1 => execution.receipt_id = ReceiptId::from_raw("foreign-receipt"),
        2 => execution.receipt_sha256 = sha256(b"different receipt"),
        3 => execution.artifact_candidates[0].bytes[0] ^= 1,
        4 => execution.artifact_candidates[0].kind = RuntimeArtifactKind::StandardOutput,
        5 => execution.artifact_candidates[0].media_type = "text/plain".into(),
        6 => execution.artifact_candidates.swap(0, 1),
        7 => {
            execution.artifact_candidates.pop();
        }
        8 => execution.artifact_candidates.push(candidate(b"unprepared")),
        _ => {}
    }
    Ok(())
}

#[test]
fn prepared_artifacts_keep_exact_dependent_refs_after_terminal_commit() {
    let (mut runtime, executions) = callback_fixture(PermissionScript::PrepareArtifacts(0));
    let RuntimeCoordinatorStep::Complete { outcome } =
        runtime.run_until_boundary(None, None).unwrap()
    else {
        panic!("synthetic completion expected");
    };
    assert_eq!(outcome.state, AgentStateKind::Success);
    assert_eq!(executions.load(Ordering::SeqCst), 1);
    let result = &runtime.tool_results[0];
    let output = result.output.as_ref().unwrap();
    let second_ref: RuntimeArtifactRef = serde_json::from_slice(&output.bytes).unwrap();
    let artifacts = runtime.tool_boundary.artifacts.lock().unwrap();
    let owned: Vec<_> = artifacts
        .iter()
        .filter(|(m, _)| m.receipt_id.is_some())
        .collect();
    assert_eq!(owned.len(), 2);
    assert_eq!(runtime_artifact_ref(&owned[1].0).unwrap(), second_ref);
    let first_ref: RuntimeArtifactRef = serde_json::from_slice(&owned[1].1).unwrap();
    assert_eq!(runtime_artifact_ref(&owned[0].0).unwrap(), first_ref);
    assert_eq!(owned[0].1, b"original full bytes");
    let terminal = runtime
        .events()
        .iter()
        .find(|e| matches!(e.kind, RuntimeEventKind::ToolCompleted { .. }))
        .unwrap();
    assert!(
        matches!(&terminal.kind, RuntimeEventKind::ToolCompleted { result_sha256, .. }
        if result_sha256 == &sha256(&to_canonical_json(result).unwrap()))
    );
    for (manifest, _) in &owned {
        assert_eq!(manifest.created_at_epoch_ms, terminal.occurred_at_epoch_ms);
        let event = runtime.events().iter().find(|e| matches!(&e.kind,
            RuntimeEventKind::ArtifactCreated { artifact_id, .. } if artifact_id == &manifest.artifact_id)).unwrap();
        assert!(event.sequence > terminal.sequence);
        assert_eq!(event.occurred_at_epoch_ms, terminal.occurred_at_epoch_ms);
        assert_eq!(event.operation_id, terminal.operation_id);
        assert_eq!(event.turn_id, terminal.turn_id);
    }
    // Complete candidate bytes are charged once, as is the inline tool output.
    // Model completion output is charged through the unchanged output owner.
    let final_bytes = match outcome.output.as_ref().unwrap() {
        RuntimeOutput::Inline { payload } => payload.bytes.len(),
        RuntimeOutput::Artifact { reference } => reference.byte_size as usize,
    };
    let expected_output = owned.iter().map(|(_, bytes)| bytes.len()).sum::<usize>()
        + output.bytes.len()
        + final_bytes;
    assert_eq!(
        runtime.resource_snapshot().cumulative[&BudgetResource::OutputBytes],
        expected_output as u64
    );
    let retained_bytes = artifacts
        .iter()
        .map(|(_, bytes)| bytes.len() as u64)
        .sum::<u64>();
    assert_eq!(runtime.resource_snapshot().artifact_bytes, retained_bytes);
    assert_eq!(
        runtime.resource_snapshot().artifact_count as usize,
        artifacts.len()
    );
    drop(artifacts);
    assert_valid_terminal_stream(&runtime);
}

#[test]
fn prepared_artifact_substitution_and_ignored_refusals_cannot_publish_or_replay() {
    for mutation in [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 14] {
        let (mut runtime, executions) =
            callback_fixture(PermissionScript::PrepareArtifacts(mutation));
        assert!(
            runtime.run_until_boundary(None, None).is_err(),
            "mutation {mutation}"
        );
        assert_eq!(executions.load(Ordering::SeqCst), 1);
        assert!(runtime.tool_results.is_empty());
        assert!(runtime.receipt_ids.is_empty());
        assert!(
            runtime
                .tool_boundary
                .artifacts
                .lock()
                .unwrap()
                .iter()
                .all(|(m, _)| m.receipt_id.is_none())
        );
        assert!(!runtime.events().iter().any(|e| matches!(
            e.kind,
            RuntimeEventKind::ToolCompleted { .. } | RuntimeEventKind::ToolFailed { .. }
        )));
        assert_callback_failure_stays_latched(&mut runtime, &executions);
    }
}

#[test]
fn preparation_without_artifact_owner_refuses_after_start_and_stays_latched() {
    let (mut runtime, executions) = durable_fault_coordinator(
        [ModelScript::Tool, ModelScript::Completion],
        PermissionScript::PrepareArtifacts(0),
    );
    assert!(runtime.run_until_boundary(None, None).is_err());
    assert_eq!(executions.load(Ordering::SeqCst), 1);
    assert!(runtime.tool_boundary.artifacts.lock().unwrap().is_empty());
    assert_callback_failure_stays_latched(&mut runtime, &executions);
}

#[test]
fn prepared_partial_publication_and_missing_event_never_replay_the_effect() {
    for mutation in [20, 21] {
        let (mut runtime, executions) =
            callback_fixture(PermissionScript::PrepareArtifacts(mutation));
        assert!(runtime.run_until_boundary(None, None).is_err());
        assert_eq!(executions.load(Ordering::SeqCst), 1);
        assert!(runtime.tool_results.is_empty());
        assert!(runtime.receipt_ids.is_empty());
        let artifacts = runtime.tool_boundary.artifacts.lock().unwrap();
        assert_eq!(
            artifacts
                .iter()
                .filter(|(m, _)| m.receipt_id.is_some())
                .count(),
            1
        );
        let published = runtime
            .events()
            .iter()
            .filter(|e| {
                matches!(e.kind, RuntimeEventKind::ArtifactCreated { .. })
                    && e.operation_id.is_some()
            })
            .count();
        assert_eq!(published, usize::from(mutation == 20));
        assert!(
            runtime
                .events()
                .iter()
                .any(|e| matches!(e.kind, RuntimeEventKind::ToolCompleted { .. }))
        );
        drop(artifacts);
        assert_callback_failure_stays_latched(&mut runtime, &executions);
    }
}

fn builder_inputs() -> (
    RuntimeRunRequest,
    ToolDefinition,
    ToolCall,
    RuntimeEvent,
    RuntimeToolExecution,
) {
    let (mut runtime, _) = callback_fixture(PermissionScript::Allow);
    runtime.run_until_boundary(None, None).unwrap();
    let call = runtime.completed_tool_calls[0].clone();
    let definition = runtime
        .registry
        .get_tool(&call.tool_id, &call.tool_version)
        .unwrap()
        .clone();
    let started = runtime
        .events()
        .iter()
        .find(|e| matches!(e.kind, RuntimeEventKind::ToolStarted { .. }))
        .unwrap()
        .clone();
    let execution = RuntimeToolExecution {
        receipt_id: runtime.receipt_ids[0].clone(),
        receipt_sha256: SHA.into(),
        result: runtime.tool_results[0].clone(),
        result_output_kind: runtime.tool_results[0]
            .output
            .as_ref()
            .map(|_| RuntimeArtifactKind::Report),
        artifact_candidates: vec![candidate(b"prepared")],
    };
    (runtime.request, definition, call, started, execution)
}

#[test]
fn preparation_samples_one_observed_time_without_advancing_it_for_dependent_bytes() {
    use crate::runtime_loop::artifact_preparation::ToolCompletionBuilder;
    let (request, definition, call, started, execution) = builder_inputs();
    let observed = std::cell::Cell::new(true);
    let mut clock = FakeClock {
        now: started.occurred_at_epoch_ms,
    };
    let mut builder = ToolCompletionBuilder::new(
        &request,
        &definition,
        &call,
        &started,
        &observed,
        &mut clock,
        RuntimeResourceLedger::new(&request).unwrap(),
        7,
        true,
    );
    let references = builder
        .prepare_artifacts(
            &execution.receipt_id,
            &execution.receipt_sha256,
            &execution.artifact_candidates,
        )
        .unwrap();
    let terminal = builder.build_terminal_event(&execution).unwrap();
    assert_eq!(
        references[0].artifact_id.as_str(),
        derived_id("artifact", request.run_id.as_str(), 8)
    );
    assert_eq!(
        terminal.occurred_at_epoch_ms,
        started.occurred_at_epoch_ms + 1
    );
    assert!(builder.matches_return(&RuntimeToolCorrectnessCommit {
        execution: execution.clone(),
        events: vec![started.clone(), terminal.clone()]
    }));
    let (_, prepared) = builder.into_parts();
    assert_eq!(clock.now, terminal.occurred_at_epoch_ms);
    assert_eq!(
        prepared.unwrap()[0].manifest.created_at_epoch_ms,
        terminal.occurred_at_epoch_ms
    );
}

#[test]
fn preparation_checks_each_remaining_budget_before_exposing_references() {
    use crate::runtime_loop::artifact_preparation::ToolCompletionBuilder;
    for resource in [
        "output",
        "disk",
        "artifact-count",
        "artifact-bytes",
        "event-count",
    ] {
        let (request, definition, call, started, execution) = builder_inputs();
        let mut resources = RuntimeResourceLedger::new(&request).unwrap();
        let bytes = execution.artifact_candidates[0].bytes.len() as u64;
        let limit = |kind| {
            request
                .work_packet
                .budgets
                .iter()
                .find(|b| b.resource == kind)
                .unwrap()
                .limit
        };
        match resource {
            "output" => resources
                .consume(
                    BudgetResource::OutputBytes,
                    limit(BudgetResource::OutputBytes) - bytes + 1,
                )
                .unwrap(),
            "disk" => resources
                .consume(
                    BudgetResource::DiskBytes,
                    limit(BudgetResource::DiskBytes) - bytes + 1,
                )
                .unwrap(),
            "artifact-count" => {
                for _ in 0..resources.limits().artifact_count {
                    resources.admit_artifact(1).unwrap();
                }
            }
            "artifact-bytes" => resources
                .admit_artifact(resources.limits().artifact_bytes - bytes + 1)
                .unwrap(),
            "event-count" => {
                for _ in 0..resources.limits().run_events - 1 {
                    resources.admit_event(1).unwrap();
                }
            }
            _ => unreachable!(),
        }
        let observed = std::cell::Cell::new(true);
        let mut clock = FakeClock {
            now: started.occurred_at_epoch_ms,
        };
        let mut builder = ToolCompletionBuilder::new(
            &request,
            &definition,
            &call,
            &started,
            &observed,
            &mut clock,
            resources,
            0,
            true,
        );
        assert_eq!(
            builder.prepare_artifacts(
                &execution.receipt_id,
                &execution.receipt_sha256,
                &execution.artifact_candidates
            ),
            Err(RuntimePortFailure::ResourceExhausted),
            "{resource}"
        );
        assert_eq!(
            builder.build_terminal_event(&execution),
            Err(RuntimePortFailure::Invalid)
        );
        let (_, prepared) = builder.into_parts();
        assert!(prepared.is_none());
        assert_eq!(
            clock.now, started.occurred_at_epoch_ms,
            "budget refusal does not sample a completion clock"
        );
    }
}

#[test]
fn prepared_candidates_cannot_spend_the_final_output_allowance() {
    use crate::runtime_loop::artifact_preparation::ToolCompletionBuilder;
    let (request, definition, call, started, mut execution) = builder_inputs();
    execution.result.output = Some(payload("runtime.tool-result", b"final complete output"));
    execution.result_output_kind = Some(RuntimeArtifactKind::Report);
    let bytes = execution.artifact_candidates[0].bytes.len()
        + execution.result.output.as_ref().unwrap().bytes.len();
    let limit = request
        .work_packet
        .budgets
        .iter()
        .find(|b| b.resource == BudgetResource::OutputBytes)
        .unwrap()
        .limit;
    let mut resources = RuntimeResourceLedger::new(&request).unwrap();
    resources
        .consume(BudgetResource::OutputBytes, limit - bytes as u64 + 1)
        .unwrap();
    let observed = std::cell::Cell::new(true);
    let mut clock = FakeClock {
        now: started.occurred_at_epoch_ms,
    };
    let mut builder = ToolCompletionBuilder::new(
        &request,
        &definition,
        &call,
        &started,
        &observed,
        &mut clock,
        resources,
        0,
        true,
    );
    builder
        .prepare_artifacts(
            &execution.receipt_id,
            &execution.receipt_sha256,
            &execution.artifact_candidates,
        )
        .unwrap();
    assert_eq!(
        builder.build_terminal_event(&execution),
        Err(RuntimePortFailure::ResourceExhausted)
    );
    execution.result.output = None;
    execution.result_output_kind = None;
    assert_eq!(
        builder.build_terminal_event(&execution),
        Err(RuntimePortFailure::Invalid)
    );
}

#[test]
fn preparation_bounds_total_candidates_across_appends_and_poisoned_sealing() {
    use crate::runtime_loop::artifact_preparation::ToolCompletionBuilder;
    let (mut request, definition, call, started, mut execution) = builder_inputs();
    // Pure high-capacity fixture, within existing hard ceilings. No product limit
    // changes: the request must have enough artifact slots to reach the tool cap.
    request.limits.max_turns = 32;
    let resources = RuntimeResourceLedger::new(&request).unwrap();
    assert!(resources.limits().artifact_count > 64);
    let observed = std::cell::Cell::new(true);
    let mut clock = FakeClock {
        now: started.occurred_at_epoch_ms,
    };
    let mut builder = ToolCompletionBuilder::new(
        &request,
        &definition,
        &call,
        &started,
        &observed,
        &mut clock,
        resources,
        0,
        true,
    );
    execution.artifact_candidates = vec![candidate(b"x"); 64];
    for chunk in execution.artifact_candidates.chunks(16) {
        assert_eq!(
            builder
                .prepare_artifacts(&execution.receipt_id, &execution.receipt_sha256, chunk)
                .unwrap()
                .len(),
            16
        );
    }
    assert_eq!(
        builder.prepare_artifacts(
            &execution.receipt_id,
            &execution.receipt_sha256,
            &[candidate(b"extra")]
        ),
        Err(RuntimePortFailure::Invalid)
    );
    assert_eq!(
        builder.build_terminal_event(&execution),
        Err(RuntimePortFailure::Invalid)
    );
}

#[test]
fn preparation_refuses_reversed_time_and_clock_failure_without_fallback() {
    use crate::runtime_loop::artifact_preparation::ToolCompletionBuilder;
    struct Clock(Result<u64, RuntimePortFailure>);
    impl RuntimeClock for Clock {
        fn now_epoch_ms(&mut self) -> Result<u64, RuntimePortFailure> {
            self.0
        }
    }
    for observation in [Ok(0), Ok(1), Err(RuntimePortFailure::Unavailable)] {
        let (request, definition, call, started, execution) = builder_inputs();
        let observed = std::cell::Cell::new(true);
        let mut clock = Clock(observation);
        let mut builder = ToolCompletionBuilder::new(
            &request,
            &definition,
            &call,
            &started,
            &observed,
            &mut clock,
            RuntimeResourceLedger::new(&request).unwrap(),
            0,
            true,
        );
        assert!(
            builder
                .prepare_artifacts(
                    &execution.receipt_id,
                    &execution.receipt_sha256,
                    &execution.artifact_candidates
                )
                .is_err()
        );
        assert_eq!(
            builder.build_terminal_event(&execution),
            Err(RuntimePortFailure::Invalid)
        );
    }
}

#[test]
fn prepared_non_success_retains_audit_bytes_without_successful_tool_evidence() {
    let (mut runtime, executions) = callback_fixture(PermissionScript::PrepareArtifacts(0));
    runtime.tool_boundary.outcome = OperationOutcome::Failed;
    let RuntimeCoordinatorStep::Complete { outcome } =
        runtime.run_until_boundary(None, None).unwrap()
    else {
        panic!("failed effect must close");
    };
    assert_eq!(outcome.state, AgentStateKind::Failed);
    assert_eq!(executions.load(Ordering::SeqCst), 1);
    assert!(runtime.tool_results.is_empty());
    assert!(outcome.evidence.is_empty());
    let failed = runtime
        .events()
        .iter()
        .find(|e| matches!(e.kind, RuntimeEventKind::ToolFailed { .. }))
        .unwrap();
    let artifacts = runtime.tool_boundary.artifacts.lock().unwrap();
    let owned: Vec<_> = artifacts
        .iter()
        .filter(|(m, _)| m.receipt_id.is_some())
        .collect();
    assert_eq!(owned.len(), 3); // Two prepared members and the separately retained failure output.
    for (manifest, _) in owned {
        let event = runtime.events().iter().find(|e| matches!(&e.kind,
            RuntimeEventKind::ArtifactCreated { artifact_id, .. } if artifact_id == &manifest.artifact_id)).unwrap();
        assert!(event.sequence > failed.sequence);
        assert!(manifest.created_at_epoch_ms >= failed.occurred_at_epoch_ms);
    }
    drop(artifacts);
    assert_valid_terminal_stream(&runtime);
}

#[test]
fn prepared_ordinals_precede_large_output_without_double_accounting() {
    let (initial, executions) = callback_fixture(PermissionScript::PrepareArtifacts(19));
    let mut request = initial.request;
    request.limits.max_output_bytes = (MAX_RUNTIME_INLINE_OUTPUT_BYTES + 1) as u64;
    request.request_sha256 = "0".repeat(64);
    let mut runtime = ReusableRuntimeCoordinator::new_with_durable_state(
        seal_runtime_run_request(request).unwrap(),
        initial.model,
        initial.context,
        initial.registry,
        initial.tool_boundary,
        initial.verifier,
        initial.clock,
    )
    .unwrap();
    let RuntimeCoordinatorStep::Complete { outcome } =
        runtime.run_until_boundary(None, None).unwrap()
    else {
        panic!("large-output synthetic completion expected");
    };
    assert_eq!(outcome.state, AgentStateKind::Success);
    assert_eq!(executions.load(Ordering::SeqCst), 1);
    let artifacts = runtime.tool_boundary.artifacts.lock().unwrap();
    let owned: Vec<_> = artifacts
        .iter()
        .filter(|(m, _)| m.receipt_id.is_some())
        .collect();
    assert_eq!(owned.len(), 3);
    let first_ref: RuntimeArtifactRef = serde_json::from_slice(&owned[1].1).unwrap();
    assert_eq!(runtime_artifact_ref(&owned[0].0).unwrap(), first_ref);
    assert_eq!(owned[2].1, vec![b'r'; MAX_RUNTIME_INLINE_OUTPUT_BYTES + 1]);
    assert!(owned[0].0.created_at_epoch_ms < owned[2].0.created_at_epoch_ms);
    let final_bytes = match outcome.output.as_ref().unwrap() {
        RuntimeOutput::Inline { payload } => payload.bytes.len() as u64,
        RuntimeOutput::Artifact { reference } => reference.byte_size,
    };
    assert_eq!(
        runtime.resource_snapshot().cumulative[&BudgetResource::OutputBytes],
        owned
            .iter()
            .map(|(_, bytes)| bytes.len() as u64)
            .sum::<u64>()
            + final_bytes
    );
    assert_eq!(
        runtime.resource_snapshot().artifact_bytes,
        artifacts
            .iter()
            .map(|(_, bytes)| bytes.len() as u64)
            .sum::<u64>()
    );
    assert_eq!(
        runtime.resource_snapshot().artifact_count as usize,
        artifacts.len()
    );
    drop(artifacts);
    assert_valid_terminal_stream(&runtime);
}

#[test]
fn research_preparation_does_not_enable_network_in_any_ordinary_runtime_mode() {
    for mode in [
        RuntimeSessionMode::EphemeralReadOnly,
        RuntimeSessionMode::DurableReadOnly,
        RuntimeSessionMode::ControlledWrite,
    ] {
        assert!(!operation_allowed_for_mode(
            mode,
            GrantOperation::NetworkAccess
        ));
        let (initial, executions) = callback_fixture(PermissionScript::PrepareArtifacts(0));
        let registry = registry_for_operation(GrantOperation::NetworkAccess);
        let mut request = request(initial.model.exact_profile().clone(), &registry);
        request.mode = mode;
        let request = seal_runtime_run_request(request).unwrap();
        let admitted = if mode == RuntimeSessionMode::EphemeralReadOnly {
            ReusableRuntimeCoordinator::new(
                request,
                initial.model,
                initial.context,
                registry,
                initial.tool_boundary,
                initial.verifier,
                initial.clock,
            )
        } else {
            ReusableRuntimeCoordinator::new_with_durable_state(
                request,
                initial.model,
                initial.context,
                registry,
                initial.tool_boundary,
                initial.verifier,
                initial.clock,
            )
        };
        assert_eq!(admitted.err(), Some(RuntimeLoopError::ToolCatalogBinding));
        assert_eq!(executions.load(Ordering::SeqCst), 0);

        for state_change in [
            StateChange::Changed,
            StateChange::NotChanged,
            StateChange::Uncertain,
        ] {
            assert!(!valid_runtime_state_change(
                mode,
                GrantOperation::NetworkAccess,
                OperationOutcome::Succeeded,
                state_change
            ));
        }
    }
}
