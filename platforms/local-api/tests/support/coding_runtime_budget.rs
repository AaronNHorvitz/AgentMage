// SPDX-License-Identifier: Apache-2.0
//! Actual terminal against a stalled synthetic native-runtime HTTP peer.
//! No model, native tool, cancellation effect or coding score is fabricated.

use super::*;

fn terminal(request: &RuntimeRunRequest) -> Value {
    let mut stream = Stream::new(request);
    stream.event(
        RuntimeEventKind::RunStarted {
            request_sha256: request.request_sha256.clone(),
        },
        None,
        None,
    );
    let outcome = stream.terminal(AgentStateKind::NoOp);
    stream.step(stream.events.clone(), None, outcome)
}

#[test]
fn pending_selection_preparation_and_start_reads_stop_on_deadline_and_every_signal() {
    for route in ["selection", "prepare", "start"] {
        for mode in [
            "approve",
            "workflow-socket:SIGINT",
            "workflow-socket:SIGTERM",
            "workflow-socket:SIGHUP",
        ] {
            for stage in ["header", "body"] {
                let f = Fixture::new();
                let r = driven_request();
                let mut replies = vec![driven_selection(&r), json!(r), terminal(&r)];
                let index = match route {
                    "selection" => 0,
                    "prepare" => 1,
                    _ => 2,
                };
                replies.truncate(index + 1);
                replies[index] = json!({"test_peer_stall":stage});
                if route == "start" {
                    replies.push(json!({"released":true}));
                }
                let peer = f.peer(replies);
                let began = Instant::now();
                let out = f.tty(peer.address, &["runtime-run", "1", &r.task.objective], mode);
                assert_ne!(out["status"], 0, "{route}/{stage}/{mode}: {out}");
                assert!(began.elapsed() < Duration::from_secs(4), "{out}");
                assert_eq!(out["answered"], index, "{out}");
                assert!(
                    !out["transcript"]
                        .as_str()
                        .unwrap()
                        .contains("verified-outcome"),
                    "{out}"
                );
                let calls = peer.finish();
                assert_eq!(
                    calls.len(),
                    index + 1 + usize::from(route == "start"),
                    "{calls:?}"
                );
                assert_eq!(calls[index]["test_peer_disconnected"], true, "{calls:?}");
                if route == "start" {
                    assert_eq!(
                        calls.last().unwrap()["route"],
                        "POST /v1/code/runtime/release HTTP/1.1"
                    );
                    assert_eq!(
                        calls.last().unwrap()["body"],
                        json!({"run_id":r.run_id,"request_sha256":r.request_sha256})
                    );
                }
                assert!(
                    calls.iter().all(|c| c["body"]["response"].is_null()),
                    "{calls:?}"
                );
            }
        }
    }
}

#[test]
fn pending_approved_advance_stops_without_replaying_a_grant_or_generic_cancellation() {
    for mode in [
        "approve",
        "workflow-socket:SIGINT",
        "workflow-socket:SIGTERM",
        "workflow-socket:SIGHUP",
    ] {
        for stage in ["header", "body"] {
            let f = Fixture::new();
            let r = driven_request();
            let (first, _) = driven_steps(&r, true);
            let peer = f.peer(vec![
                driven_selection(&r),
                json!(r),
                first,
                json!({"test_peer_stall":stage}),
                json!({"released":true}),
            ]);
            let out = f.tty(peer.address, &["runtime-run", "1", &r.task.objective], mode);
            assert_ne!(out["status"], 0, "{out}");
            assert!(out["seconds"].as_f64().unwrap() < 4.0, "{out}");
            assert_eq!(out["answered"], 3, "{out}");
            assert!(
                !out["transcript"]
                    .as_str()
                    .unwrap()
                    .contains("verified-outcome"),
                "{out}"
            );
            let calls = peer.finish();
            assert_eq!(calls.len(), 5, "{calls:?}");
            assert_eq!(calls[3]["route"], "POST /v1/code/runtime/advance HTTP/1.1");
            assert_eq!(calls[3]["body"]["response"]["disposition"], "allow");
            assert_eq!(calls[3]["test_peer_disconnected"], true);
            assert_eq!(calls[4]["route"], "POST /v1/code/runtime/release HTTP/1.1");
            assert_eq!(
                calls[4]["body"],
                json!({"run_id":r.run_id,"request_sha256":r.request_sha256})
            );
        }
    }
}

#[test]
fn release_after_a_lost_start_reply_has_a_short_bounded_grace_period() {
    for stage in ["header", "body"] {
        let f = Fixture::new();
        let r = driven_request();
        let peer = f.peer(vec![
            driven_selection(&r),
            json!(r),
            json!({"test_peer_close":true}),
            json!({"test_peer_stall":stage}),
        ]);
        let out = f.tty(
            peer.address,
            &["runtime-run", "30", &r.task.objective],
            "approve",
        );
        assert_ne!(out["status"], 0, "{out}");
        let elapsed = out["seconds"].as_f64().unwrap();
        assert!((1.8..4.0).contains(&elapsed), "{out}");
        let calls = peer.finish();
        assert_eq!(calls.len(), 4, "{calls:?}");
        assert_eq!(calls[3]["route"], "POST /v1/code/runtime/release HTTP/1.1");
        assert_eq!(calls[3]["test_peer_disconnected"], true);
        assert!(
            !out["transcript"]
                .as_str()
                .unwrap()
                .contains("verified-outcome"),
            "{out}"
        );
    }
}

#[test]
fn one_runtime_deadline_spans_inspection_and_start_approval() {
    let f = Fixture::new();
    let r = driven_request();
    let peer = f.peer(vec![
        driven_selection(&r),
        json!(r),
        json!({"released":true}),
    ]);
    let out = f.tty(
        peer.address,
        &["runtime-run", "1", &r.task.objective],
        "runtime-clock",
    );
    assert_ne!(out["status"], 0, "{out}");
    assert_eq!(out["answered"], 2, "{out}");
    assert!(
        (0.9..1.6).contains(&out["seconds"].as_f64().unwrap()),
        "{out}"
    );
    let calls = peer.finish();
    assert_eq!(calls.len(), 3, "{calls:?}");
    assert_eq!(calls[1]["route"], "POST /v1/code/runtime/prepare HTTP/1.1");
    assert_eq!(calls[2]["route"], "POST /v1/code/runtime/release HTTP/1.1");
}

#[test]
fn stalled_terminal_evidence_cannot_be_reported_as_verified_success() {
    for route in ["artifact", "declarations", "job"] {
        for mode in ["approve", "workflow-socket:SIGTERM"] {
            let f = Fixture::new();
            let r = driven_request();
            let mut step = terminal(&r);
            let mut replies = vec![driven_selection(&r), json!(r)];
            if route == "artifact" {
                let artifact = RuntimeArtifactRef {
                    schema_version: CONTRACT_SCHEMA_VERSION,
                    artifact_id: RuntimeArtifactId::from_raw("budget-artifact"),
                    manifest_sha256: "a".repeat(64),
                    payload_sha256:
                        "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a".into(),
                    byte_size: 2,
                    media_type: "application/json".into(),
                };
                let mut stream = Stream::new(&r);
                stream.event(
                    RuntimeEventKind::RunStarted {
                        request_sha256: r.request_sha256.clone(),
                    },
                    None,
                    None,
                );
                stream.event(
                    RuntimeEventKind::ArtifactCreated {
                        artifact_id: artifact.artifact_id.clone(),
                        manifest_sha256: artifact.manifest_sha256.clone(),
                    },
                    None,
                    None,
                );
                let outcome = stream.terminal(AgentStateKind::NoOp);
                step = stream.step(stream.events.clone(), None, outcome);
                step["artifacts"] = json!([artifact]);
            }
            replies.push(step);
            if route == "job" {
                replies.push(Value::Null);
            }
            let stalled = replies.len();
            replies.push(json!({"test_peer_stall":"body"}));
            // Declarations are optional in the original driver; it still asks
            // for the bound terminal job before release. No run is advanced.
            if route == "declarations" {
                replies.push(Value::Null);
            }
            replies.push(json!({"released":true}));
            let peer = f.peer(replies);
            let out = f.tty(peer.address, &["runtime-run", "1", &r.task.objective], mode);
            assert_ne!(out["status"], 0, "{route}/{mode}: {out}");
            assert!(out["seconds"].as_f64().unwrap() < 4.0, "{out}");
            assert!(
                !out["transcript"]
                    .as_str()
                    .unwrap()
                    .contains("verified-outcome"),
                "{out}"
            );
            let calls = peer.finish();
            assert_eq!(
                calls.len(),
                stalled + 2 + usize::from(route == "declarations"),
                "{calls:?}"
            );
            assert_eq!(
                calls[stalled]["route"],
                format!("POST /v1/code/runtime/{route} HTTP/1.1")
            );
            assert_eq!(calls[stalled]["test_peer_disconnected"], true);
            assert_eq!(
                calls.last().unwrap()["route"],
                "POST /v1/code/runtime/release HTTP/1.1"
            );
        }
    }
}

#[test]
fn uncertain_release_is_never_repeated_by_best_effort_cleanup() {
    for reply in [json!({"test_peer_close":true}), json!({"released":false})] {
        let f = Fixture::new();
        let r = driven_request();
        let peer = f.peer(vec![
            driven_selection(&r),
            json!(r),
            terminal(&r),
            Value::Null,
            Value::Null,
            reply,
        ]);
        let out = f.tty(
            peer.address,
            &["runtime-run", "30", &r.task.objective],
            "approve",
        );
        assert_ne!(out["status"], 0, "{out}");
        assert!(
            !out["transcript"]
                .as_str()
                .unwrap()
                .contains("verified-outcome"),
            "{out}"
        );
        let calls = peer.finish();
        assert_eq!(calls.len(), 6, "{calls:?}");
        assert_eq!(calls[5]["route"], "POST /v1/code/runtime/release HTTP/1.1");
        assert_eq!(
            calls[5]["body"],
            json!({"run_id":r.run_id,"request_sha256":r.request_sha256})
        );
    }
}

fn cancelling(request: &RuntimeRunRequest) -> Vec<Value> {
    let (first, _) = driven_steps(request, true);
    let mut stream = Stream {
        request: request.clone(),
        events: serde_json::from_value(first["events"].clone()).unwrap(),
    };
    let split = stream.events.len();
    for kind in [
        RuntimeEventKind::CancellationRequested {
            cancellation_id: CancellationId::from_raw("budget-cancel"),
        },
        RuntimeEventKind::CancellationObserved {
            cancellation_id: CancellationId::from_raw("budget-cancel"),
        },
    ] {
        stream.event(kind, None, None);
    }
    let outcome = stream.terminal(AgentStateKind::Cancelled);
    let terminal = stream.step(stream.events[split..].to_vec(), None, outcome);
    let status = json!({"schema_version":1,"run_id":request.run_id,"request_sha256":request.request_sha256,
        "job":{"job_id":request.run_id,"phase":"running","revision":7,"cancellation_requested":false,"head_sha256":"a".repeat(64)}});
    let mut cancelling = status.clone();
    cancelling["job"]["phase"] = json!("cancelling");
    cancelling["job"]["revision"] = json!(8);
    cancelling["job"]["cancellation_requested"] = json!(true);
    let mut cancelled = cancelling.clone();
    cancelled["job"]["phase"] = json!("cancelled");
    cancelled["job"]["revision"] = json!(9);
    vec![
        driven_selection(request),
        json!(request),
        first,
        status,
        json!({"decision":{"decision":"applied","revision":8,"phase":"cancelling"},"status":cancelling}),
        terminal,
        Value::Null,
        cancelled,
        json!({"released":true}),
    ]
}

#[test]
fn every_signal_and_deadline_preserve_original_revision_bound_cancellation_and_terminal_verification()
 {
    for signal in ["SIGINT", "SIGTERM", "SIGHUP", "wait"] {
        let f = Fixture::new();
        let r = driven_request();
        let peer = f.peer(cancelling(&r));
        let out = f.tty(
            peer.address,
            &[
                "runtime-run",
                if signal == "wait" { "1" } else { "30" },
                &r.task.objective,
            ],
            &format!("workflow-stop:2:{signal}"),
        );
        assert_ne!(out["status"], 0, "{out}");
        let transcript = out["transcript"].as_str().unwrap();
        assert!(
            transcript.contains("verified-outcome") && transcript.contains("CANCELLED"),
            "{out}"
        );
        let calls = peer.finish();
        assert_eq!(calls.len(), 9, "{calls:?}");
        assert_eq!(calls[4]["route"], "POST /v1/code/runtime/control HTTP/1.1");
        assert_eq!(calls[4]["body"]["request"]["action"], "cancel");
        assert_eq!(calls[4]["body"]["request"]["observed_revision"], 7);
        assert_eq!(calls[5]["route"], "POST /v1/code/runtime/advance HTTP/1.1");
        assert!(
            calls.iter().all(|c| c["body"]["response"].is_null()),
            "{calls:?}"
        );
    }
}

#[test]
fn cancellation_grace_is_shared_across_job_control_and_next_boundary_reads() {
    let f = Fixture::new();
    let r = driven_request();
    let mut replies = cancelling(&r);
    for (index, delay) in [(3, 900), (4, 700)] {
        replies[index] = json!({"test_peer_delay_ms":delay,"body":replies[index]});
    }
    replies[5] = json!({"test_peer_stall":"body"});
    replies.truncate(6);
    let peer = f.peer(replies);
    let out = f.tty(
        peer.address,
        &["runtime-run", "30", &r.task.objective],
        "workflow-stop:2:SIGINT",
    );
    assert_ne!(out["status"], 0, "{out}");
    assert!(
        (1.8..2.7).contains(&out["seconds"].as_f64().unwrap()),
        "{out}"
    );
    assert!(
        !out["transcript"]
            .as_str()
            .unwrap()
            .contains("verified-outcome"),
        "{out}"
    );
    let calls = peer.finish();
    assert_eq!(calls.len(), 6, "{calls:?}");
    assert_eq!(calls[5]["test_peer_disconnected"], true);
    assert!(
        calls.iter().all(|c| c["body"]["response"].is_null()),
        "{calls:?}"
    );
}
