// SPDX-License-Identifier: MIT OR Apache-2.0
//! Synthetic static run admission, not native tools or a coding score.

use std::{cell::Cell, num::NonZeroU64, time::Duration};

use agentmage_kernel_contracts::{
    BudgetLimit, BudgetResource, EndpointModelBody, EndpointModelFinish, EndpointModelLimits,
    EndpointModelRequest, EndpointModelResult, EndpointModelSelection, ModelOperationControl,
    ModelOperationStop, RuntimeEndpointContextBudget, from_json, to_canonical_json,
};
use serde_json::json;

use crate::{
    runtime_hardening::RuntimeHardeningError,
    runtime_loop::{
        RuntimePortFailure,
        endpoint::{
            RuntimeEndpointContextPacket, RuntimeEndpointMessage, RuntimeEndpointMessageRole,
            RuntimeEndpointModelPhase, RuntimeEndpointModelPort,
        },
    },
};

use super::*;

#[path = "../../runtime_loop/endpoint/turn_tests.rs"]
mod turns;

#[path = "outcome_tests.rs"]
mod outcome_tests;

fn request() -> RuntimeEndpointRunRequest {
    let native = super::super::tests::request();
    RuntimeEndpointRunRequest {
        schema_version: native.schema_version,
        run_id: native.run_id,
        session_id: native.session_id,
        mode: native.mode,
        task: native.task,
        work_packet: native.work_packet,
        workspace_id: native.workspace_id,
        workspace_snapshot_sha256: native.workspace_snapshot_sha256,
        repository_snapshot_id: native.repository_snapshot_id,
        repository_snapshot_sha256: native.repository_snapshot_sha256,
        model_selection: EndpointModelSelection {
            endpoint: "http://127.0.0.1:1234/v1".into(),
            model: "synthetic".into(),
        },
        context_budget: RuntimeEndpointContextBudget {
            max_messages: 2,
            max_content_bytes: 128,
            max_serialized_messages_bytes: 256,
            max_request_body_bytes: 512,
        },
        max_output_tokens: 10,
        tool_catalog_id: native.tool_catalog_id,
        tool_catalog_sha256: native.tool_catalog_sha256,
        visible_tools: native.visible_tools,
        policy_id: native.policy_id,
        policy_sha256: native.policy_sha256,
        limits: native.limits,
        event_cursor: native.event_cursor,
        request_sha256: native.request_sha256,
    }
}

#[test]
fn endpoint_run_round_trip_is_versioned_closed_and_distinct_from_native_profiles() {
    let endpoint = seal_runtime_endpoint_run_request(request()).unwrap();
    assert_eq!(verify_runtime_endpoint_run_request(&endpoint), Ok(()));
    let bytes = to_canonical_json(&endpoint).unwrap();
    assert_eq!(
        from_json::<RuntimeEndpointRunRequest>(&bytes).unwrap(),
        endpoint
    );
    assert!(from_json::<RuntimeRunRequest>(&bytes).is_err());
    let native = seal_runtime_run_request(super::super::tests::request()).unwrap();
    let native_bytes = to_canonical_json(&native).unwrap();
    assert_eq!(
        from_json::<RuntimeRunRequest>(&native_bytes).unwrap(),
        native
    );
    assert!(from_json::<RuntimeEndpointRunRequest>(&native_bytes).is_err());
    let value = serde_json::to_value(&endpoint).unwrap();
    for field in ["model_profile", "verified_input_tokens", "token_counter"] {
        assert!(value.get(field).is_none());
    }
    for field in [
        "max_context_tokens",
        "token_counter",
        "token_counter_sha256",
    ] {
        assert!(value["context_budget"].get(field).is_none());
    }
    for (pointer, extra) in [
        ("", "credential"),
        ("/model_selection", "manifest_sha256"),
        ("/context_budget", "max_context_tokens"),
    ] {
        let mut bad = value.clone();
        bad.pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(extra.into(), json!(1));
        assert!(
            from_json::<RuntimeEndpointRunRequest>(&serde_json::to_vec(&bad).unwrap()).is_err()
        );
    }
    let mut missing = value.clone();
    missing.as_object_mut().unwrap().remove("event_cursor");
    assert!(
        from_json::<RuntimeEndpointRunRequest>(&serde_json::to_vec(&missing).unwrap()).is_err()
    );
    let duplicate = String::from_utf8(bytes).unwrap().replacen(
        "\"schema_version\":",
        "\"schema_version\":1,\"schema_version\":",
        1,
    );
    assert!(from_json::<RuntimeEndpointRunRequest>(duplicate.as_bytes()).is_err());
    let mut wrong_version = endpoint;
    wrong_version.schema_version += 1;
    assert_eq!(
        verify_runtime_endpoint_run_request(&wrong_version),
        Err(RuntimeCoordinatorError::VersionMismatch)
    );
}

#[test]
fn endpoint_and_native_runs_share_task_scope_catalog_cursor_and_limit_refusals() {
    let native =
        serde_json::to_value(seal_runtime_run_request(super::super::tests::request()).unwrap())
            .unwrap();
    let endpoint =
        serde_json::to_value(seal_runtime_endpoint_run_request(request()).unwrap()).unwrap();
    let cases = [
        ("/run_id", json!(""), RuntimeCoordinatorError::InvalidValue),
        (
            "/workspace_id",
            json!("bad id"),
            RuntimeCoordinatorError::InvalidValue,
        ),
        (
            "/workspace_snapshot_sha256",
            json!("0"),
            RuntimeCoordinatorError::InvalidValue,
        ),
        (
            "/repository_snapshot_sha256",
            json!("A".repeat(64)),
            RuntimeCoordinatorError::InvalidValue,
        ),
        (
            "/policy_sha256",
            json!("no"),
            RuntimeCoordinatorError::InvalidValue,
        ),
        (
            "/task/session_id",
            json!("another-session"),
            RuntimeCoordinatorError::BindingMismatch,
        ),
        (
            "/task/objective",
            json!(""),
            RuntimeCoordinatorError::InvalidValue,
        ),
        (
            "/task/status",
            json!("completed"),
            RuntimeCoordinatorError::InvalidValue,
        ),
        (
            "/work_packet/task_id",
            json!("another-task"),
            RuntimeCoordinatorError::BindingMismatch,
        ),
        (
            "/work_packet/objective",
            json!(""),
            RuntimeCoordinatorError::WorkPacketDenied,
        ),
        (
            "/tool_catalog_sha256",
            json!("d".repeat(64)),
            RuntimeCoordinatorError::BindingMismatch,
        ),
        (
            "/visible_tools",
            json!([{"tool_id":"one","tool_version":"1","definition_sha256":"a".repeat(64)},{"tool_id":"one","tool_version":"1","definition_sha256":"a".repeat(64)}]),
            RuntimeCoordinatorError::InvalidValue,
        ),
        (
            "/limits/max_turns",
            json!(0),
            RuntimeCoordinatorError::InvalidLimits,
        ),
        (
            "/limits/max_model_calls",
            json!(17),
            RuntimeCoordinatorError::InvalidLimits,
        ),
        (
            "/limits/max_repeated_tool_calls",
            json!(0),
            RuntimeCoordinatorError::InvalidLimits,
        ),
        (
            "/limits/max_elapsed_ms",
            json!(0),
            RuntimeCoordinatorError::InvalidLimits,
        ),
        (
            "/mode",
            json!("controlled_write"),
            RuntimeCoordinatorError::InvalidLimits,
        ),
        (
            "/event_cursor",
            json!({"run_id":"another-run","event_id":"event-1","sequence":0,"event_sha256":"a".repeat(64)}),
            RuntimeCoordinatorError::BindingMismatch,
        ),
    ];
    for (pointer, changed, expected) in cases {
        let mut n = native.clone();
        let mut e = endpoint.clone();
        *n.pointer_mut(pointer).unwrap() = changed.clone();
        *e.pointer_mut(pointer).unwrap() = changed;
        let n: RuntimeRunRequest = serde_json::from_value(n).unwrap();
        let e: RuntimeEndpointRunRequest = serde_json::from_value(e).unwrap();
        assert_eq!(
            verify_runtime_run_request(&n),
            Err(expected),
            "native {pointer}"
        );
        assert_eq!(
            verify_runtime_endpoint_run_request(&e),
            Err(expected),
            "endpoint {pointer}"
        );
    }
}

#[test]
fn endpoint_run_digest_binds_selection_context_task_policy_and_resource_ceilings() {
    let endpoint = seal_runtime_endpoint_run_request(request()).unwrap();
    let original = serde_json::to_value(&endpoint).unwrap();
    // Valid changes retain the old digest and must fail specifically on it.
    for (pointer, value) in [
        (
            "/model_selection/endpoint",
            json!("http://127.0.0.1:1235/v1"),
        ),
        ("/model_selection/model", json!("other")),
        ("/max_output_tokens", json!(9)),
        ("/context_budget/max_messages", json!(1)),
        ("/context_budget/max_content_bytes", json!(127)),
        ("/context_budget/max_serialized_messages_bytes", json!(255)),
        ("/context_budget/max_request_body_bytes", json!(511)),
        ("/repository_snapshot_sha256", json!("d".repeat(64))),
        ("/policy_sha256", json!("e".repeat(64))),
        ("/limits/max_output_bytes", json!(4095)),
        ("/task/objective", json!("Changed task")),
    ] {
        let mut changed = original.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        let changed: RuntimeEndpointRunRequest = serde_json::from_value(changed).unwrap();
        assert_eq!(
            verify_runtime_endpoint_run_request(&changed),
            Err(RuntimeCoordinatorError::DigestMismatch),
            "{pointer}"
        );
        let resealed = seal_runtime_endpoint_run_request(changed).unwrap();
        assert_ne!(resealed.request_sha256, endpoint.request_sha256);
        assert_eq!(verify_runtime_endpoint_run_request(&resealed), Ok(()));
    }
    for (pointer, value) in [
        ("/max_output_tokens", json!(0)),
        ("/max_output_tokens", json!(32769)),
        ("/context_budget/max_messages", json!(0)),
        ("/context_budget/max_messages", json!(257)),
        ("/context_budget/max_content_bytes", json!(0)),
        ("/context_budget/max_content_bytes", json!(257)),
        ("/context_budget/max_serialized_messages_bytes", json!(513)),
        (
            "/context_budget/max_request_body_bytes",
            json!(256 * 1024 + 1),
        ),
    ] {
        let mut bad = original.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        assert_eq!(
            seal_runtime_endpoint_run_request(serde_json::from_value(bad).unwrap()),
            Err(RuntimeCoordinatorError::InvalidLimits),
            "{pointer}"
        );
    }
    for (pointer, value) in [
        ("/model_selection/endpoint", "ftp://127.0.0.1/v1"),
        ("/model_selection/endpoint", "http://secret@127.0.0.1/v1"),
        (
            "/model_selection/endpoint",
            "http://127.0.0.1/v1?key=secret",
        ),
        ("/model_selection/endpoint", "http://127.0.0.1/v1#fragment"),
        ("/model_selection/endpoint", "http://127.0.0.1/\n"),
        ("/model_selection/model", ""),
        ("/model_selection/model", " model "),
        ("/model_selection/model", "model\nname"),
    ] {
        let mut bad = original.clone();
        *bad.pointer_mut(pointer).unwrap() = json!(value);
        assert_eq!(
            seal_runtime_endpoint_run_request(serde_json::from_value(bad).unwrap()),
            Err(RuntimeCoordinatorError::InvalidValue),
            "{pointer}"
        );
    }
}

fn write_budgets(request: &mut RuntimeEndpointRunRequest) {
    let limits = &request.limits;
    request.mode = RuntimeSessionMode::ControlledWrite;
    request.work_packet.budgets = [
        (BudgetResource::PlanSteps, u64::from(limits.max_turns)),
        (
            BudgetResource::ModelCalls,
            u64::from(limits.max_model_calls),
        ),
        (BudgetResource::ToolCalls, u64::from(limits.max_tool_calls)),
        (
            BudgetResource::ToolCallDepth,
            u64::from(limits.max_tool_call_depth),
        ),
        (
            BudgetResource::InputBytes,
            request.context_budget.max_request_body_bytes * u64::from(limits.max_context_refreshes),
        ),
        (BudgetResource::OutputBytes, limits.max_output_bytes),
        (BudgetResource::ElapsedMilliseconds, limits.max_elapsed_ms),
        (BudgetResource::MemoryBytes, 1),
        (BudgetResource::DiskBytes, limits.max_output_bytes),
        (
            BudgetResource::ProcessCount,
            u64::from(limits.max_tool_calls.min(1)),
        ),
    ]
    .into_iter()
    .map(|(resource, limit)| BudgetLimit { resource, limit })
    .collect();
}

#[test]
fn endpoint_controlled_write_requires_all_original_resource_budgets_covering_full_body_bytes() {
    let mut endpoint = request();
    write_budgets(&mut endpoint);
    let endpoint = seal_runtime_endpoint_run_request(endpoint).unwrap();
    for index in 0..endpoint.work_packet.budgets.len() {
        let resource = endpoint.work_packet.budgets[index].resource;
        let mut missing = endpoint.clone();
        missing.work_packet.budgets.remove(index);
        assert_eq!(
            RuntimeHardeningLimits::from_endpoint_request(&missing),
            Err(RuntimeHardeningError::MissingBudget(resource))
        );
        assert!(seal_runtime_endpoint_run_request(missing).is_err());
        let mut narrow = endpoint.clone();
        narrow.work_packet.budgets[index].limit -= 1;
        assert_eq!(
            RuntimeHardeningLimits::from_endpoint_request(&narrow),
            Err(RuntimeHardeningError::BudgetBinding(resource))
        );
        assert!(seal_runtime_endpoint_run_request(narrow).is_err());
    }
    // Content alone fits, but cannot cover the full prepared body plus JSON.
    let mut narrow = endpoint.clone();
    narrow
        .work_packet
        .budgets
        .iter_mut()
        .find(|b| b.resource == BudgetResource::InputBytes)
        .unwrap()
        .limit =
        narrow.context_budget.max_content_bytes * u64::from(narrow.limits.max_context_refreshes);
    assert_eq!(
        RuntimeHardeningLimits::from_endpoint_request(&narrow),
        Err(RuntimeHardeningError::BudgetBinding(
            BudgetResource::InputBytes
        ))
    );
    let mut native = super::super::tests::request();
    native.mode = endpoint.mode;
    native.work_packet = endpoint.work_packet.clone();
    native.context_budget.max_input_bytes = endpoint.context_budget.max_request_body_bytes;
    assert_eq!(
        RuntimeHardeningLimits::from_request(&native),
        RuntimeHardeningLimits::from_endpoint_request(&endpoint)
    );
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

fn prepared() -> EndpointModelRequest {
    EndpointModelRequest::from_prepared_body(
        request().model_selection,
        context().messages().clone(),
        EndpointModelBody {
            request_body_bytes: 200,
            request_body_sha256: "b".repeat(64),
        },
        EndpointModelLimits {
            requested_max_output_tokens: 10,
            provider_usage_requested: false,
            total_timeout: Duration::from_millis(100),
            idle_timeout: Duration::from_millis(50),
            response_wire_limit_bytes: 1000,
            response_body_limit_bytes: 500,
            response_text_limit_bytes: 100,
        },
    )
}

#[test]
fn endpoint_run_phase_refuses_selection_context_and_dispatch_ceiling_drift() {
    let run = seal_runtime_endpoint_run_request(request()).unwrap();
    let original = serde_json::to_value(&run).unwrap();
    for (pointer, changed, expected) in [
        (
            "/model_selection/endpoint",
            json!("http://127.0.0.1:1235/v1"),
            RuntimePortFailure::Invalid,
        ),
        (
            "/model_selection/model",
            json!("other"),
            RuntimePortFailure::Invalid,
        ),
        (
            "/context_budget/max_messages",
            json!(1),
            RuntimePortFailure::ResourceExhausted,
        ),
        (
            "/context_budget/max_content_bytes",
            json!(15),
            RuntimePortFailure::ResourceExhausted,
        ),
        (
            "/context_budget/max_serialized_messages_bytes",
            json!(128),
            RuntimePortFailure::ResourceExhausted,
        ),
        (
            "/context_budget/max_request_body_bytes",
            json!(199),
            RuntimePortFailure::ResourceExhausted,
        ),
        (
            "/max_output_tokens",
            json!(9),
            RuntimePortFailure::ResourceExhausted,
        ),
        (
            "/limits/max_output_bytes",
            json!(99),
            RuntimePortFailure::ResourceExhausted,
        ),
        (
            "/limits/max_elapsed_ms",
            json!(99),
            RuntimePortFailure::ResourceExhausted,
        ),
    ] {
        let mut changed_run = original.clone();
        *changed_run.pointer_mut(pointer).unwrap() = changed;
        // Keep the ordering of context ceilings valid for the narrower bound.
        if pointer == "/context_budget/max_serialized_messages_bytes" {
            changed_run["context_budget"]["max_content_bytes"] = json!(16);
            changed_run["context_budget"]["max_serialized_messages_bytes"] =
                json!(context().messages().serialized_messages_bytes - 1);
        }
        if pointer == "/context_budget/max_request_body_bytes" {
            changed_run["context_budget"]["max_serialized_messages_bytes"] = json!(199);
        }
        let changed_run =
            seal_runtime_endpoint_run_request(serde_json::from_value(changed_run).unwrap())
                .unwrap();
        assert!(
            matches!(RuntimeEndpointModelPhase::for_run(&changed_run, &prepared(), context()), Err(e) if e == expected),
            "{pointer}"
        );
    }
    let mut drifted = run;
    drifted.policy_sha256 = "e".repeat(64);
    assert!(matches!(
        RuntimeEndpointModelPhase::for_run(&drifted, &prepared(), context()),
        Err(RuntimePortFailure::Invalid)
    ));
    let mut wrong_context = prepared();
    wrong_context.context.serialized_messages_sha256 = "d".repeat(64);
    assert!(matches!(
        RuntimeEndpointModelPhase::for_run(
            &seal_runtime_endpoint_run_request(request()).unwrap(),
            &wrong_context,
            context()
        ),
        Err(RuntimePortFailure::Invalid)
    ));
}

struct Control;
impl ModelOperationControl for Control {
    fn remaining_ms(&self) -> Result<NonZeroU64, ModelOperationStop> {
        Ok(NonZeroU64::new(100).unwrap())
    }
}
struct SyntheticPort<'a> {
    request: EndpointModelRequest,
    calls: &'a Cell<u32>,
}
impl RuntimeEndpointModelPort for SyntheticPort<'_> {
    type Error = ();
    fn request(&self) -> &EndpointModelRequest {
        &self.request
    }
    fn dispatch(
        self,
        _: &dyn ModelOperationControl,
        text: &mut dyn FnMut(&str),
    ) -> Result<EndpointModelResult, ()> {
        self.calls.set(self.calls.get() + 1);
        text("reply");
        Ok(EndpointModelResult::from_completed_response(
            self.request,
            EndpointModelFinish::Stop,
            sha256(b"reply"),
            5,
            Duration::from_millis(1),
            None,
        ))
    }
}

#[test]
fn endpoint_run_bound_phase_dispatches_once_and_retains_exact_immutable_run_identity() {
    let mut input = request();
    let context = context();
    let prepared = prepared();
    // Exact bounds all admit; each narrower case is tested above.
    input.context_budget.max_content_bytes = context.content_bytes() as u64;
    input.context_budget.max_serialized_messages_bytes =
        context.messages().serialized_messages_bytes as u64;
    input.context_budget.max_request_body_bytes = prepared.body().request_body_bytes as u64;
    input.limits.max_output_bytes = prepared.limits().response_text_limit_bytes as u64;
    input.limits.max_elapsed_ms = 100;
    let run = seal_runtime_endpoint_run_request(input).unwrap();
    let expected = json!({"run_id":run.run_id,"session_id":run.session_id,"task_id":run.task.task_id,"request_sha256":run.request_sha256});
    let mut phase = RuntimeEndpointModelPhase::for_run(&run, &prepared, context).unwrap();
    drop(run);
    assert_eq!(
        serde_json::to_value(phase.run_binding().unwrap()).unwrap(),
        expected
    );
    let calls = Cell::new(0);
    let mut delivered = String::new();
    let result = phase
        .execute(
            &Control,
            SyntheticPort {
                request: prepared.clone(),
                calls: &calls,
            },
            |text| delivered.push_str(text),
        )
        .unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(delivered, "reply");
    assert_eq!(result.request, prepared);
    assert_eq!(phase.usage().dispatch_attempts, 1);
    assert_eq!(phase.usage().completed_response_text_bytes, Some(5));
    assert_eq!(
        serde_json::to_value(phase.run_binding().unwrap()).unwrap(),
        expected
    );
    assert!(
        phase
            .execute(
                &Control,
                SyntheticPort {
                    request: prepared.clone(),
                    calls: &calls
                },
                |_| panic!("second dispatch")
            )
            .is_err()
    );
    assert_eq!(calls.get(), 1);
    assert!(
        RuntimeEndpointModelPhase::new(&prepared, self::context())
            .unwrap()
            .run_binding()
            .is_none()
    );
}
