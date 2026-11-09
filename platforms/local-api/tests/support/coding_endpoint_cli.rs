// SPDX-License-Identifier: Apache-2.0
//! Endpoint terminal consent tests against a recording synthetic HTTP peer.
use super::*;
use agentmage_kernel_engine::runtime_coordinator::{
    runtime_tool_catalog_sha256, seal_runtime_endpoint_outcome, seal_runtime_endpoint_run_request,
};

fn request() -> RuntimeEndpointRunRequest {
    let mut value: Value = serde_json::from_slice(include_bytes!(
        "../../../../fixtures/runtime-producer/v5/run-request.json"
    ))
    .unwrap();
    value.as_object_mut().unwrap().remove("model_profile");
    value["run_id"] = json!("run");
    value["mode"] = json!("ephemeral_read_only");
    value["event_cursor"] = Value::Null;
    value["model_selection"] =
        json!({"endpoint":"http://127.0.0.1:4321/v1","model":"synthetic-model"});
    value["max_output_tokens"] = json!(128);
    value["context_budget"] = json!({"max_messages":16,"max_content_bytes":32768,"max_serialized_messages_bytes":65536,"max_request_body_bytes":131072});
    let catalog = ToolCatalogId::from_raw("synthetic-terminal-catalog");
    let visible = vec![RuntimeToolReference {
        tool_id: ToolId::from_raw("fixture.read"),
        tool_version: "1.0.0".into(),
        definition_sha256: "a".repeat(64),
    }];
    value["tool_catalog_id"] = json!(catalog);
    value["tool_catalog_sha256"] = json!(runtime_tool_catalog_sha256(&catalog, &visible).unwrap());
    value["visible_tools"] = json!(visible);
    seal_runtime_endpoint_run_request(serde_json::from_value(value).unwrap()).unwrap()
}

fn selection(r: &RuntimeEndpointRunRequest) -> Value {
    json!({"request":r,"retention":"process-only","configuration":"fixed-for-service"})
}
fn approval(r: &RuntimeEndpointRunRequest) -> RuntimeApprovalChallenge {
    let mut c = challenge();
    c.run_id = r.run_id.clone();
    c.task_id = r.task.task_id.clone();
    seal_runtime_approval_challenge(c).unwrap()
}
fn waiting(r: &RuntimeEndpointRunRequest) -> Value {
    json!({"state":"awaiting_approval","challenge":approval(r)})
}

#[test]
fn endpoint_terminal_controlled_mode_keeps_exact_tool_and_disclosure_consent() {
    let f = Fixture::new();
    let mut r = request();
    r.mode = RuntimeSessionMode::ControlledWrite;
    let r = seal_runtime_endpoint_run_request(r).unwrap();
    let encoded = serde_json::to_string(&approval(&r)).unwrap();
    for (command, prompts) in [
        ("endpoint-step", 1),
        ("endpoint-approve", 2),
        ("endpoint-deny", 1),
    ] {
        let peer = f.peer(vec![selection(&r), waiting(&r)]);
        let mut args = vec![command, "run", &r.request_sha256];
        if command != "endpoint-step" {
            args.push(&encoded);
        }
        let output = f.tty(peer.address, &args, "approve");
        assert_eq!(output["status"], 0, "{output}");
        assert_eq!(output["answered"], prompts);
        let calls = peer.finish();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[1]["route"], "POST /v1/code/endpoint/advance HTTP/1.1");
        assert_eq!(calls[1]["body"]["approve_model_context"], true);
        assert_eq!(calls[1]["body"]["request_sha256"], r.request_sha256);
        assert_eq!(
            calls[1]["body"]["response"]["disposition"],
            match command {
                "endpoint-approve" => json!("allow"),
                "endpoint-deny" => json!("deny"),
                _ => Value::Null,
            }
        );
    }
    let peer = f.peer(vec![selection(&r)]);
    let output = f.tty(
        peer.address,
        &["endpoint-approve", "run", &r.request_sha256, &encoded],
        "allow-tool-only",
    );
    assert_ne!(output["status"], 0);
    assert_eq!(output["answered"], 2);
    assert_eq!(peer.finish().len(), 1);
}

#[test]
fn endpoint_terminal_still_refuses_durable_mode_before_disclosure_or_release() {
    let f = Fixture::new();
    let mut r = request();
    r.mode = RuntimeSessionMode::DurableReadOnly;
    let r = seal_runtime_endpoint_run_request(r).unwrap();
    for command in [
        "endpoint",
        "endpoint-step",
        "endpoint-release",
        "endpoint-record",
    ] {
        let peer = f.peer(vec![selection(&r)]);
        let mut args = vec![command];
        if command != "endpoint" {
            args.extend(["run", &r.request_sha256]);
        }
        let output = f.tty(peer.address, &args, "approve");
        assert_ne!(output["status"], 0, "{output}");
        assert_eq!(output["answered"], 0);
        assert_eq!(peer.finish().len(), 1);
    }
}

#[test]
fn endpoint_terminal_step_pins_original_request_and_requires_fresh_disclosure() {
    let f = Fixture::new();
    let r = request();
    let peer = f.peer(vec![selection(&r), waiting(&r)]);
    let output = f.tty(
        peer.address,
        &["endpoint-step", "run", &r.request_sha256],
        "approve",
    );
    assert_eq!(output["status"], 0, "{output}");
    assert_eq!(output["answered"], 1);
    let text = output["transcript"].as_str().unwrap();
    assert!(text.contains("http://127.0.0.1:4321/v1"));
    assert!(text.contains("synthetic-model"));
    assert!(text.contains("service-observation-only"));
    assert!(text.find("max_tool_calls").unwrap() < text.find("Type exactly share ").unwrap());
    let calls = peer.finish();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0]["route"], "GET /v1/code/endpoint/request HTTP/1.1");
    assert_eq!(calls[1]["route"], "POST /v1/code/endpoint/advance HTTP/1.1");
    assert_eq!(
        calls[1]["body"],
        json!({"run_id":"run","request_sha256":r.request_sha256,"approve_model_context":true,"response":null})
    );

    let peer = f.peer(vec![selection(&r)]);
    let output = f.tty(
        peer.address,
        &["endpoint-step", "run", &r.request_sha256],
        "decline",
    );
    assert_ne!(output["status"], 0);
    assert_eq!(output["answered"], 1);
    assert_eq!(peer.finish().len(), 1);
    for command in [
        "endpoint-step",
        "endpoint-release",
        "endpoint-approve",
        "endpoint-deny",
    ] {
        let peer = f.peer(vec![]);
        let c = serde_json::to_string(&approval(&r)).unwrap();
        let mut args = vec![command, "run", &r.request_sha256];
        if command.ends_with("approve") || command.ends_with("deny") {
            args.push(&c);
        }
        assert!(!f.cli(peer.address, &args).status.success());
        assert!(peer.finish().is_empty());
    }
}

#[test]
fn endpoint_terminal_approval_needs_both_exact_operation_and_disclosure_consent() {
    let f = Fixture::new();
    let r = request();
    let c = approval(&r);
    let encoded = serde_json::to_string(&c).unwrap();
    for (command, prompts, disposition, grant) in [
        (
            "endpoint-approve",
            2,
            json!("allow"),
            json!(c.proposed_grant_id),
        ),
        ("endpoint-deny", 1, json!("deny"), Value::Null),
    ] {
        let peer = f.peer(vec![selection(&r), waiting(&r)]);
        let output = f.tty(
            peer.address,
            &[command, "run", &r.request_sha256, &encoded],
            "approve",
        );
        assert_eq!(output["status"], 0, "{output}");
        assert_eq!(output["answered"], prompts);
        let calls = peer.finish();
        assert_eq!(calls.len(), 2);
        assert_eq!(
            calls[1]["body"]["response"],
            json!({"schema_version":CONTRACT_SCHEMA_VERSION,"run_id":"run","approval_id":c.approval_id,"challenge_sha256":c.challenge_sha256,"disposition":disposition,"grant_id":grant})
        );
        assert_eq!(calls[1]["body"]["approve_model_context"], true);
        if command == "endpoint-approve" {
            let text = output["transcript"].as_str().unwrap();
            assert!(
                text.find("Type exactly allow ").unwrap()
                    < text.find("Type exactly share ").unwrap()
            );
            assert!(text.contains("arguments_json"));
        }
    }
    // Approve the operation, decline the separate model disclosure. No approval
    // may be posted when only one of these permissions was given.
    let peer = f.peer(vec![selection(&r)]);
    let output = f.tty(
        peer.address,
        &["endpoint-approve", "run", &r.request_sha256, &encoded],
        "allow-tool-only",
    );
    assert_ne!(output["status"], 0);
    assert_eq!(output["answered"], 2);
    assert_eq!(peer.finish().len(), 1);
}

#[test]
fn endpoint_terminal_refuses_foreign_tampered_or_malformed_bindings_before_approval() {
    let f = Fixture::new();
    let r = request();
    let c = approval(&r);
    for field in ["run_id", "task_id", "challenge_sha256"] {
        let mut value = serde_json::to_value(&c).unwrap();
        value[field] = json!("foreign");
        let encoded = value.to_string();
        let peer = f.peer(vec![]);
        let output = f.tty(
            peer.address,
            &["endpoint-approve", "run", &r.request_sha256, &encoded],
            "approve",
        );
        assert_ne!(output["status"], 0);
        assert_eq!(output["answered"], 0);
        assert!(peer.finish().is_empty());
    }
    // Even a correctly sealed presentation must not silently collapse repeated
    // argument members in the readable permission review.
    let mut ambiguous = c.clone();
    ambiguous.presentation.arguments.bytes = br#"{"path":"first","path":"second"}"#.to_vec();
    {
        use sha2::{Digest, Sha256};
        ambiguous.presentation.arguments.sha256 =
            Sha256::digest(&ambiguous.presentation.arguments.bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
    }
    let encoded =
        serde_json::to_string(&seal_runtime_approval_challenge(ambiguous).unwrap()).unwrap();
    let peer = f.peer(vec![selection(&r)]);
    let output = f.tty(
        peer.address,
        &["endpoint-approve", "run", &r.request_sha256, &encoded],
        "approve",
    );
    assert_ne!(output["status"], 0);
    assert_eq!(output["answered"], 0);
    assert_eq!(peer.finish().len(), 1);
    let mut foreign_task = c.clone();
    foreign_task.task_id = TaskId::from_raw("foreign-task");
    let encoded =
        serde_json::to_string(&seal_runtime_approval_challenge(foreign_task).unwrap()).unwrap();
    let peer = f.peer(vec![selection(&r)]);
    let output = f.tty(
        peer.address,
        &["endpoint-approve", "run", &r.request_sha256, &encoded],
        "approve",
    );
    assert_ne!(output["status"], 0);
    assert_eq!(output["answered"], 0);
    assert_eq!(peer.finish().len(), 1);
    for field in [
        "request_sha256",
        "model_selection",
        "retention",
        "configuration",
    ] {
        let mut value = selection(&r);
        if field == "request_sha256" {
            value["request"][field] = json!("a".repeat(64));
        } else if field == "model_selection" {
            value["request"][field]["model"] = json!("substituted");
        } else {
            value[field] = json!("other");
        }
        let peer = f.peer(vec![value]);
        let output = f.tty(
            peer.address,
            &["endpoint-step", "run", &r.request_sha256],
            "approve",
        );
        assert_ne!(output["status"], 0);
        assert_eq!(output["answered"], 0);
        assert_eq!(peer.finish().len(), 1);
    }
    // A valid request from a different retained service also fails, even
    // though its seal is intact. Never repin consent to a newly fetched id.
    for (run, hash) in [
        ("foreign-run", r.request_sha256.as_str()),
        (
            "run",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
    ] {
        let peer = f.peer(vec![selection(&r)]);
        let output = f.tty(peer.address, &["endpoint-step", run, hash], "approve");
        assert_ne!(output["status"], 0);
        assert_eq!(output["answered"], 0);
        assert_eq!(peer.finish().len(), 1);
    }
    for args in [
        vec!["endpoint-step", "run", "bad"],
        vec!["endpoint-step", "bad\nrun", &r.request_sha256],
        vec!["endpoint-events", "run", &r.request_sha256, "-1"],
        vec!["endpoint-approve", "run", &r.request_sha256, "{}"],
        vec!["endpoint-release", "run", &r.request_sha256, "extra"],
    ] {
        let peer = f.peer(vec![]);
        let output = f.tty(peer.address, &args, "approve");
        assert_ne!(output["status"], 0);
        assert_eq!(output["answered"], 0);
        assert!(peer.finish().is_empty());
    }
}

#[test]
fn endpoint_terminal_cancel_needs_no_model_disclosure_and_release_is_separately_confirmed() {
    let f = Fixture::new();
    let r = request();
    let peer = f.peer(vec![json!({"cancellation":"requested"})]);
    assert!(
        f.cli(peer.address, &["endpoint-cancel", "run", &r.request_sha256])
            .status
            .success()
    );
    let calls = peer.finish();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0]["route"], "POST /v1/code/endpoint/cancel HTTP/1.1");
    assert_eq!(
        calls[0]["body"],
        json!({"run_id":"run","request_sha256":r.request_sha256})
    );
    let peer = f.peer(vec![selection(&r), json!({"released":true})]);
    let output = f.tty(
        peer.address,
        &["endpoint-release", "run", &r.request_sha256],
        "approve",
    );
    assert_eq!(output["status"], 0, "{output}");
    assert_eq!(output["answered"], 1);
    let text = output["transcript"].as_str().unwrap();
    assert!(text.contains("Export needed events first"));
    assert!(text.contains("Type exactly release "));
    let calls = peer.finish();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[1]["route"], "POST /v1/code/endpoint/release HTTP/1.1");
    assert_eq!(
        calls[1]["body"],
        json!({"run_id":"run","request_sha256":r.request_sha256})
    );
    let peer = f.peer(vec![selection(&r)]);
    let output = f.tty(
        peer.address,
        &["endpoint-release", "run", &r.request_sha256],
        "decline",
    );
    assert_ne!(output["status"], 0);
    assert_eq!(peer.finish().len(), 1);
}

#[test]
fn endpoint_terminal_reports_extraction_refusal_without_retry_or_follow_on() {
    let f = Fixture::new();
    let mut r = request();
    r.mode = RuntimeSessionMode::ControlledWrite;
    let r = seal_runtime_endpoint_run_request(r).unwrap();
    // Synthetic HTTP error, separately from the real service/coordinator test.
    let peer = f.peer(vec![
        selection(&r),
        json!({"test_peer_status":409,"body":{
            "error":"code.endpoint.original-review-unavailable",
            "released":true,"original_review_available":false,
            "message":"The run was released, but original review is unavailable. Inspect state and disk; do not repeat release or approval."
        }}),
    ]);
    let output = f.tty(
        peer.address,
        &["endpoint-release", "run", &r.request_sha256],
        "approve",
    );
    assert_ne!(output["status"], 0, "{output}");
    assert_eq!(output["answered"], 1);
    let text = output["transcript"].as_str().unwrap();
    assert!(text.contains("API refusal (409)"));
    assert!(text.contains("code.endpoint.original-review-unavailable"));
    assert!(text.contains("do not repeat release or approval"));
    let calls = peer.finish();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0]["route"], "GET /v1/code/endpoint/request HTTP/1.1");
    assert_eq!(calls[1]["route"], "POST /v1/code/endpoint/release HTTP/1.1");
    assert_eq!(
        calls[1]["body"],
        json!({"run_id":"run","request_sha256":r.request_sha256})
    );
}

#[test]
fn endpoint_terminal_refused_or_lost_advance_is_never_retried_or_auto_cancelled() {
    let f = Fixture::new();
    let r = request();
    for reply in [
        json!({"test_peer_close":true}),
        json!({"test_peer_status":409,"body":{"error":"code.endpoint.refused"}}),
        json!({"state":"terminal","outcome":null}),
    ] {
        let peer = f.peer(vec![selection(&r), reply]);
        let output = f.tty(
            peer.address,
            &["endpoint-step", "run", &r.request_sha256],
            "approve",
        );
        assert_ne!(output["status"], 0);
        assert_eq!(output["answered"], 1);
        let calls = peer.finish();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[1]["route"], "POST /v1/code/endpoint/advance HTTP/1.1");
    }
}

#[test]
fn endpoint_terminal_events_are_inspection_only_and_refuse_inconsistent_paging() {
    let f = Fixture::new();
    let r = request();
    let page = json!({"events":[],"next":0,"total":0,"retention":"process-only"});
    let peer = f.peer(vec![selection(&r), page.clone()]);
    let output = f.cli(
        peer.address,
        &["endpoint-events", "run", &r.request_sha256, "0"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let parsed: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed["index_is_resume_cursor"], false);
    assert_eq!(parsed["verification"], "individual-event-hashes-only");
    let calls = peer.finish();
    assert_eq!(calls.len(), 2);
    assert_eq!(
        calls[1]["body"],
        json!({"run_id":"run","request_sha256":r.request_sha256,"after":0})
    );
    for (field, value) in [
        ("next", json!(1)),
        ("total", json!(1)),
        ("events", json!([{}])),
        ("retention", json!("durable")),
    ] {
        let mut invalid = page.clone();
        invalid[field] = value;
        let peer = f.peer(vec![selection(&r), invalid]);
        assert!(
            !f.cli(
                peer.address,
                &["endpoint-events", "run", &r.request_sha256, "0"]
            )
            .status
            .success()
        );
        assert_eq!(peer.finish().len(), 2);
    }
}

#[test]
fn endpoint_record_uses_only_observation_routes_and_refuses_unstable_or_partial_histories() {
    use agentmage_kernel_engine::runtime_event::{RuntimeEventSequence, seal_runtime_event};
    let f = Fixture::new();
    let mut r = request();
    r.task.objective = "Synthetic record with escaped \u{202e} and \u{1f600}".into();
    let r = seal_runtime_endpoint_run_request(r).unwrap();
    // First-party synthetic event envelopes, not a runtime/model outcome.
    let start = seal_runtime_event(RuntimeEvent {
        schema_version: CONTRACT_SCHEMA_VERSION,
        event_id: RuntimeEventId::from_raw("synthetic-start"),
        run_id: r.run_id.clone(),
        session_id: r.session_id.clone(),
        task_id: r.task.task_id.clone(),
        turn_id: None,
        operation_id: None,
        correlation_id: CorrelationId::from_raw("synthetic-record-correlation"),
        causation_event_id: None,
        sequence: 0,
        occurred_at_epoch_ms: 1000,
        sensitivity: ContextSensitivity::Internal,
        retention: RuntimeEventRetention {
            kind: RuntimeEventRetentionKind::Ephemeral,
            expires_at_epoch_ms: None,
        },
        persistence: RuntimeEventPersistenceClass::Correctness,
        policy_id: r.policy_id.clone(),
        payload_reference: None,
        kind: RuntimeEventKind::RunStarted {
            request_sha256: r.request_sha256.clone(),
        },
        previous_event_sha256: "0".repeat(64),
        event_sha256: "0".repeat(64),
    })
    .unwrap();
    let mut terminal = start.clone();
    terminal.event_id = RuntimeEventId::from_raw("synthetic-terminal");
    terminal.sequence = 1;
    terminal.causation_event_id = Some(start.event_id.clone());
    terminal.previous_event_sha256 = start.event_sha256.clone();
    terminal.kind = RuntimeEventKind::RunTerminal {
        state: AgentStateKind::Failed,
        outcome_sha256: "f".repeat(64),
    };
    let terminal = seal_runtime_event(terminal).unwrap();
    let mut sequence = RuntimeEventSequence::new();
    sequence.push(&start).unwrap();
    sequence.push(&terminal).unwrap();
    assert!(sequence.is_terminal());
    let page = |event: &RuntimeEvent, next: u64, total: u64| json!({"events":[event],"next":next,"total":total,"retention":"process-only"});
    let peer = f.peer(vec![
        selection(&r),
        page(&start, 1, 2),
        page(&terminal, 2, 2),
    ]);
    let output = f.cli(peer.address, &["endpoint-record", "run", &r.request_sha256]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_ascii());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        json!({"version":1,"request":r,"events":[start,terminal]})
    );
    let calls = peer.finish();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0]["route"], "GET /v1/code/endpoint/request HTTP/1.1");
    for (index, call) in calls[1..].iter().enumerate() {
        assert_eq!(call["route"], "POST /v1/code/endpoint/events HTTP/1.1");
        assert_eq!(
            call["body"],
            json!({"run_id":"run","request_sha256":r.request_sha256,"after":index})
        );
    }
    for pages in [
        vec![json!({"events":[],"next":0,"total":0,"retention":"process-only"})],
        vec![page(&start, 1, 1)],
        vec![page(&start, 1, 4097)],
        vec![page(&start, 1, u64::from(r.limits.max_events) + 1)],
        vec![page(&start, 1, 3), page(&terminal, 2, 2)],
        vec![page(&start, 1, 2), json!({"test_peer_close":true})],
    ] {
        let expected = 1 + pages.len();
        let mut replies = vec![selection(&r)];
        replies.extend(pages);
        let peer = f.peer(replies);
        let output = f.cli(peer.address, &["endpoint-record", "run", &r.request_sha256]);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        // No effect route, retry or automatic cancellation after refusal/loss.
        let calls = peer.finish();
        assert_eq!(calls.len(), expected);
        assert!(
            calls[1..]
                .iter()
                .all(|c| c["route"] == "POST /v1/code/endpoint/events HTTP/1.1")
        );
    }
}

#[test]
fn endpoint_terminal_terminal_envelope_is_preserved_without_an_independent_success_claim() {
    let f = Fixture::new();
    let r = request();
    let outcome = seal_runtime_endpoint_outcome(
        RuntimeOutcome {
            schema_version: CONTRACT_SCHEMA_VERSION,
            run_id: r.run_id.clone(),
            session_id: r.session_id.clone(),
            task_id: r.task.task_id.clone(),
            request_sha256: r.request_sha256.clone(),
            state: AgentStateKind::Cancelled,
            turn_count: 0,
            model_call_count: 0,
            tool_call_count: 0,
            prior_event_id: RuntimeEventId::from_raw("synthetic-prior"),
            prior_event_sha256: "a".repeat(64),
            evidence: vec![],
            receipt_ids: vec![],
            unresolved_codes: vec!["runtime.cancelled".into()],
            output: None,
            answer_evidence: None,
            outcome_sha256: "0".repeat(64),
        },
        &r,
        None,
    )
    .unwrap();
    let reply = json!({"state":"terminal","outcome":outcome});
    let peer = f.peer(vec![selection(&r), reply.clone()]);
    let output = f.tty(
        peer.address,
        &["endpoint-step", "run", &r.request_sha256],
        "approve",
    );
    assert_eq!(output["status"], 0, "{output}");
    let text = output["transcript"].as_str().unwrap();
    assert!(text.contains("service-observation-only"));
    assert!(text.contains(&outcome.outcome.outcome_sha256));
    assert_eq!(peer.finish().len(), 2);
    for field in ["run_id", "session_id", "task_id", "request_sha256"] {
        let mut invalid = reply.clone();
        invalid["outcome"]["outcome"][field] = json!("foreign");
        let peer = f.peer(vec![selection(&r), invalid]);
        let output = f.tty(
            peer.address,
            &["endpoint-step", "run", &r.request_sha256],
            "approve",
        );
        assert_ne!(output["status"], 0, "{field}: {output}");
        assert_eq!(peer.finish().len(), 2);
    }
    let mut foreign = approval(&r);
    foreign.run_id = RuntimeRunId::from_raw("foreign-run");
    let peer=f.peer(vec![selection(&r),json!({"state":"awaiting_approval","challenge":seal_runtime_approval_challenge(foreign).unwrap()})]);
    let output = f.tty(
        peer.address,
        &["endpoint-step", "run", &r.request_sha256],
        "approve",
    );
    assert_ne!(output["status"], 0);
    assert_eq!(peer.finish().len(), 2);
}
