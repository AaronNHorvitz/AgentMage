use agentmage_kernel_contracts::{
    BoundaryFailure, BoundaryKind, BoundaryOutcomeKind, CONTRACT_SCHEMA_VERSION, CancellationId,
    CancellationReason, CancellationSignal, ContractError, ContractPayload, CorrelationId,
    ErrorCategory, ErrorId, OperationOutcome, RetryDisposition, SchemaId, SchemaReference,
    StateChange, TaskId, ToolCallId, ToolResult, from_json, to_canonical_json,
};
use agentmage_kernel_engine::propagation::{propagate_failure, validate_origin};
use serde_json::{Value, json};

const TRACE_MARKER: &str = "AGENTMAGE_BOUNDARY_TRACES=";

fn task_id() -> TaskId {
    TaskId::from_raw("task-integration-0001")
}

fn correlation_id() -> CorrelationId {
    CorrelationId::from_raw("correlation-integration-0001")
}

fn success_result() -> ToolResult {
    let bytes = br#"{"observed":true}"#.to_vec();
    ToolResult {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_call_id: ToolCallId::from_raw("call-integration-0001"),
        correlation_id: correlation_id(),
        outcome: OperationOutcome::Succeeded,
        output: Some(ContractPayload {
            schema: SchemaReference {
                schema_id: SchemaId::from_raw("fixture.output"),
                schema_version: 1,
                schema_sha256: "a".repeat(64),
            },
            media_type: "application/json".to_owned(),
            bytes,
            sha256: "517fe0410b426ee5d4402666fdd520487e953669b789ed308b60b962c9968cd5".to_owned(),
        }),
        validation_issues: Vec::new(),
        evidence: Vec::new(),
        error: None,
        elapsed_ms: 7,
        state_change: StateChange::NotChanged,
    }
}

fn category(outcome: BoundaryOutcomeKind) -> ErrorCategory {
    match outcome {
        BoundaryOutcomeKind::Denied => ErrorCategory::Policy,
        BoundaryOutcomeKind::Cancelled => ErrorCategory::Cancellation,
        BoundaryOutcomeKind::TimedOut => ErrorCategory::Timeout,
        BoundaryOutcomeKind::Failed => ErrorCategory::Dependency,
        BoundaryOutcomeKind::Uncertain => ErrorCategory::Uncertain,
    }
}

fn failure(origin: BoundaryKind, outcome: BoundaryOutcomeKind, code: &str) -> BoundaryFailure {
    BoundaryFailure {
        schema_version: CONTRACT_SCHEMA_VERSION,
        correlation_id: correlation_id(),
        task_id: task_id(),
        origin,
        route: vec![origin],
        outcome,
        error: ContractError {
            schema_version: CONTRACT_SCHEMA_VERSION,
            error_id: ErrorId::from_raw(format!("error-{code}")),
            code: code.to_owned(),
            category: category(outcome),
            message: "Synthetic integration failure".to_owned(),
            field_path: vec!["fixture".to_owned()],
            retry: RetryDisposition::Never,
            caused_by: Some(ErrorId::from_raw("error-upstream-0001")),
        },
        cancellation: (outcome == BoundaryOutcomeKind::Cancelled).then(|| CancellationSignal {
            schema_version: CONTRACT_SCHEMA_VERSION,
            cancellation_id: CancellationId::from_raw("cancel-integration-0001"),
            correlation_id: correlation_id(),
            task_id: task_id(),
            reason: CancellationReason::UserRequested,
            requested_by: BoundaryKind::Tool,
        }),
    }
}

fn wire_round_trip<T>(value: &T) -> T
where
    T: agentmage_kernel_contracts::VersionedContract,
{
    let first = to_canonical_json(value).expect("integration value serializes");
    let decoded = from_json::<T>(&first).expect("integration value parses");
    let second = to_canonical_json(&decoded).expect("decoded value serializes");
    assert_eq!(first, second);
    decoded
}

fn boundary_name(boundary: BoundaryKind) -> &'static str {
    match boundary {
        BoundaryKind::Shell => "shell",
        BoundaryKind::Kernel => "kernel",
        BoundaryKind::PlatformAdapter => "platform_adapter",
        BoundaryKind::Tool => "tool",
        BoundaryKind::Model => "model",
    }
}

fn outcome_name(outcome: BoundaryOutcomeKind) -> &'static str {
    match outcome {
        BoundaryOutcomeKind::Denied => "denied",
        BoundaryOutcomeKind::Cancelled => "cancelled",
        BoundaryOutcomeKind::TimedOut => "timed_out",
        BoundaryOutcomeKind::Failed => "failed",
        BoundaryOutcomeKind::Uncertain => "uncertain",
    }
}

fn propagate_through(originated: BoundaryFailure, observers: &[BoundaryKind]) -> BoundaryFailure {
    validate_origin(&originated).expect("valid integration origin");
    observers
        .iter()
        .copied()
        .fold(originated, |current, observer| {
            let decoded = wire_round_trip(&current);
            propagate_failure(decoded, observer).expect("declared integration edge")
        })
}

fn failure_trace(case_id: &str, failure: &BoundaryFailure) -> Value {
    json!({
        "case_id": case_id,
        "outcome": outcome_name(failure.outcome),
        "route": failure.route.iter().copied().map(boundary_name).collect::<Vec<_>>(),
        "task_id": failure.task_id.as_str(),
        "correlation_id": failure.correlation_id.as_str(),
        "error_id": failure.error.error_id.as_str(),
        "error_code": failure.error.code,
        "error_category": format!("{:?}", failure.error.category).to_ascii_lowercase(),
        "error_field_path": failure.error.field_path,
        "caused_by": failure.error.caused_by.as_ref().map(|value| value.as_str()),
        "cancellation_id": failure.cancellation.as_ref()
            .map(|signal| signal.cancellation_id.as_str()),
    })
}

#[test]
fn success_and_every_failure_class_preserve_boundary_context() {
    let expected_task = task_id();
    let expected_correlation = correlation_id();
    let mut traces = Vec::new();

    let mut success = success_result();
    let success_route = [
        BoundaryKind::Tool,
        BoundaryKind::PlatformAdapter,
        BoundaryKind::Kernel,
        BoundaryKind::Shell,
    ];
    for _observer in &success_route[1..] {
        success = wire_round_trip(&success);
    }
    assert_eq!(success.outcome, OperationOutcome::Succeeded);
    assert_eq!(success.correlation_id, expected_correlation);
    assert!(success.error.is_none());
    assert!(success.output.is_some());
    assert_eq!(success.state_change, StateChange::NotChanged);
    traces.push(json!({
        "case_id": "boundary.success",
        "outcome": "succeeded",
        "route": success_route.into_iter().map(boundary_name).collect::<Vec<_>>(),
        "task_id": expected_task.as_str(),
        "correlation_id": success.correlation_id.as_str(),
        "error_id": Value::Null,
        "error_code": Value::Null,
        "error_category": Value::Null,
        "error_field_path": [],
        "caused_by": Value::Null,
        "cancellation_id": Value::Null,
    }));

    for (case_id, origin, outcome, code, observers) in [
        (
            "boundary.denied",
            BoundaryKind::Tool,
            BoundaryOutcomeKind::Denied,
            "fixture.denied",
            vec![
                BoundaryKind::PlatformAdapter,
                BoundaryKind::Kernel,
                BoundaryKind::Shell,
            ],
        ),
        (
            "boundary.cancelled",
            BoundaryKind::Tool,
            BoundaryOutcomeKind::Cancelled,
            "fixture.cancelled",
            vec![
                BoundaryKind::PlatformAdapter,
                BoundaryKind::Kernel,
                BoundaryKind::Shell,
            ],
        ),
        (
            "boundary.timed_out",
            BoundaryKind::Model,
            BoundaryOutcomeKind::TimedOut,
            "fixture.timed_out",
            vec![BoundaryKind::Kernel, BoundaryKind::Shell],
        ),
        (
            "boundary.failed",
            BoundaryKind::Model,
            BoundaryOutcomeKind::Failed,
            "fixture.failed",
            vec![BoundaryKind::Kernel, BoundaryKind::Shell],
        ),
    ] {
        let originated = failure(origin, outcome, code);
        let expected_error = originated.error.clone();
        let expected_cancellation = originated.cancellation.clone();
        let propagated = propagate_through(originated, &observers);
        assert_eq!(propagated.task_id, expected_task);
        assert_eq!(propagated.correlation_id, expected_correlation);
        assert_eq!(propagated.error, expected_error);
        assert_eq!(propagated.cancellation, expected_cancellation);
        traces.push(failure_trace(case_id, &propagated));
    }

    if std::env::var("AGENTMAGE_EMIT_BOUNDARY_TRACES").as_deref() == Ok("1") {
        println!(
            "{TRACE_MARKER}{}",
            serde_json::to_string(&traces).expect("boundary trace serialization")
        );
    }
}
