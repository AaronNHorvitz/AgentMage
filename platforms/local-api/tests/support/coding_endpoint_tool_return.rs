// SPDX-License-Identifier: Apache-2.0
//! Real service ownership, synthetic tools/endpoint; no native or model score.
use super::*;
use agentmage_host::coding_endpoint_owner::CodingEndpointAdmission;
use agentmage_local_api::serve_admitted_coding_endpoint_returning_tool;

struct ReturningBoundary {
    inner: Boundary,
    marker: Box<u8>,
    uncertain: bool,
    fault: bool,
}
impl RuntimeToolBoundary for ReturningBoundary {
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
        s: RuntimeRunScope<'_>,
        o: &RuntimeOperationId,
        d: &ToolDefinition,
        c: &ToolCall,
        now: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.inner.evaluate_scoped(s, o, d, c, now)
    }
    fn resolve_scoped(
        &mut self,
        s: RuntimeRunScope<'_>,
        h: &RuntimeApprovalChallenge,
        r: &RuntimeApprovalResponse,
        d: &ToolDefinition,
        c: &ToolCall,
        now: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.inner.resolve_scoped(s, h, r, d, c, now)
    }
    fn execute_scoped(
        &mut self,
        s: RuntimeRunScope<'_>,
        e: &RuntimePermissionEvaluation,
        d: &ToolDefinition,
        c: &ToolCall,
        p: Option<&dyn ModelCancellationProbe>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        if self.fault {
            return Err(RuntimePortFailure::Unavailable);
        }
        let mut result = self.inner.execute_scoped(s, e, d, c, p)?;
        if self.uncertain {
            result.result.outcome = OperationOutcome::Uncertain;
            result.result.state_change = StateChange::Uncertain;
            result.result.output = None;
            result.result.evidence.clear();
            result.result_output_kind = None;
        }
        Ok(result)
    }
}

fn returning_api(
    endpoint: LocalChatEndpoint,
    boundary: ReturningBoundary,
) -> (Api, mpsc::Receiver<Option<ReturningBoundary>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = stop.clone();
    let (tx, rx) = mpsc::sync_channel(1);
    let worker = thread::spawn(move || {
        let returned = serve_admitted_coding_endpoint_returning_tool(
            listener,
            ApiTokens::new(CLIENT.into(), CONTROL.into()).unwrap(),
            worker_stop,
            CodingEndpointObservationAccess::DelegateObservations,
            |id, session, runs| {
                let input = compose(id, session, endpoint, boundary.inner.0.clone());
                let request = input.request.clone();
                runs.admit(CodingEndpointAdmission {
                    request: input.request,
                    connection: input.connection,
                    registry: input.registry,
                    sources: &input.sources,
                    source_budget: input.source_budget,
                    include_usage: input.include_usage,
                    tool_boundary: boundary,
                    verifier: input.verifier,
                    clock: input.clock,
                })
                .map_err(|_| ApiError::Configuration)?;
                Ok(request)
            },
        )?;
        // A failing assertion may already have dropped the receiver.
        let _ = tx.send(returned);
        Ok(())
    });
    (
        Api {
            address,
            stop,
            worker: Some(worker),
        },
        rx,
    )
}

fn allow(challenge: &Value) -> Value {
    json!({"schema_version":CONTRACT_SCHEMA_VERSION,"run_id":challenge["run_id"],
        "approval_id":challenge["approval_id"],"disposition":RuntimeApprovalDisposition::Allow,
        "challenge_sha256":challenge["challenge_sha256"],
        "grant_id":challenge["proposed_grant_id"],"selection":null})
}

#[test]
fn endpoint_http_tool_return_keeps_original_owner_and_requires_fresh_run_approvals() {
    let (upstream, endpoint) = upstream();
    let calls = Arc::new(AtomicUsize::new(0));
    let mut boundary = ReturningBoundary {
        inner: Boundary(calls.clone()),
        marker: Box::new(73),
        uncertain: false,
        fault: false,
    };
    let marker_address = &*boundary.marker as *const u8 as usize;
    let model = thread::spawn(move || {
        for _ in 0..2 {
            let mut socket = accept(&upstream);
            read_upstream(&mut socket);
            respond(&mut socket, &tool());
            drop(socket);
            let mut socket = accept(&upstream);
            let body = read_upstream(&mut socket);
            assert!(body.to_string().contains("original retained boundary"));
            respond(&mut socket, "synthetic failed completion");
        }
        upstream
    });
    let mut old: Option<(Value, Value)> = None;
    for ordinal in 1..=2 {
        let (api, returned) = returning_api(endpoint.clone(), boundary);
        let r = api.request();
        assert_eq!(
            api.bind("release", &r).1["error"],
            "code.endpoint.still-running"
        );
        let (code, pending) = api.advance(&r, Value::Null, true);
        assert_eq!(code, 200);
        assert_eq!(pending["state"], "awaiting_approval");
        let events = api.bind("events", &r);
        assert_eq!(
            api.bind("release", &r).1["error"],
            "code.endpoint.still-running"
        );
        if let Some((previous_request, previous_approval)) = &old {
            assert_ne!(r["run_id"], previous_request["run_id"]);
            assert_eq!(api.advance(previous_request, Value::Null, true).0, 409);
            assert_eq!(api.advance(&r, previous_approval.clone(), true).0, 409);
            assert_eq!(api.bind("events", &r), events);
        }
        let approval = allow(&pending["challenge"]);
        let (code, terminal) = api.advance(&r, approval.clone(), true);
        assert_eq!(code, 200);
        assert_eq!(terminal["outcome"]["outcome"]["state"], "FAILED");
        assert_eq!(calls.load(Ordering::SeqCst), ordinal);
        assert!(matches!(
            returned.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
        // Observation delegation never grants release, even at terminal.
        assert_eq!(
            api.call(
                "POST",
                "/v1/code/endpoint/release",
                CLIENT,
                Some(json!({"run_id":r["run_id"],"request_sha256":r["request_sha256"]}))
            )
            .0,
            403
        );
        let mut wrong = r.clone();
        wrong["request_sha256"] = json!("0".repeat(64));
        assert_eq!(api.bind("release", &wrong).0, 409);
        assert_eq!(api.bind("release", &r), (200, json!({"released":true})));
        for operation in ["release", "events"] {
            assert_eq!(api.bind(operation, &r).1["error"], "code.endpoint.not-held");
        }
        assert_eq!(
            api.advance(&r, approval.clone(), true).1["error"],
            "code.endpoint.not-held"
        );
        assert!(matches!(
            returned.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
        drop(api); // joins every HTTP worker before returning the tool
        boundary = returned
            .recv_timeout(Duration::from_secs(1))
            .unwrap()
            .unwrap();
        assert_eq!(&*boundary.marker as *const u8 as usize, marker_address);
        assert_eq!(*boundary.marker, 73);
        assert!(Arc::ptr_eq(&boundary.inner.0, &calls));
        assert_eq!(calls.load(Ordering::SeqCst), ordinal);
        old = Some((r, approval));
    }
    assert!(
        matches!(model.join().unwrap().accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock)
    );
}

#[test]
fn endpoint_http_tool_return_refuses_uncertain_and_faulted_runs_without_losing_events() {
    for fault in [false, true] {
        let (upstream, endpoint) = upstream();
        let calls = Arc::new(AtomicUsize::new(0));
        let model = thread::spawn(move || {
            let mut socket = accept(&upstream);
            read_upstream(&mut socket);
            respond(&mut socket, &tool());
            upstream
        });
        let (api, returned) = returning_api(
            endpoint,
            ReturningBoundary {
                inner: Boundary(calls.clone()),
                marker: Box::new(73),
                uncertain: !fault,
                fault,
            },
        );
        let r = api.request();
        let (code, pending) = api.advance(&r, Value::Null, true);
        assert_eq!(code, 200);
        let terminal = api.advance(&r, allow(&pending["challenge"]), true);
        if fault {
            assert_eq!(terminal.0, 409);
        } else {
            assert_eq!(terminal.0, 200);
            assert_eq!(terminal.1["outcome"]["outcome"]["state"], "UNCERTAIN");
        }
        let events = api.bind("events", &r);
        assert_eq!(events.0, 200);
        for _ in 0..2 {
            assert_eq!(api.bind("release", &r).0, 409);
            assert_eq!(api.bind("events", &r), events);
            if fault {
                assert_eq!(
                    api.advance(&r, Value::Null, true).1["error"],
                    "code.endpoint.faulted"
                );
            } else {
                assert_eq!(api.advance(&r, Value::Null, true), terminal);
            }
        }
        assert_eq!(calls.load(Ordering::SeqCst), usize::from(!fault));
        drop(api);
        assert!(
            returned
                .recv_timeout(Duration::from_secs(1))
                .unwrap()
                .is_none()
        );
        assert!(
            matches!(model.join().unwrap().accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock)
        );
    }
}

#[test]
fn endpoint_http_tool_return_needs_explicit_release_even_after_cancellation() {
    for (cancel, release) in [(false, false), (true, false), (true, true)] {
        let (upstream, endpoint) = upstream();
        let calls = Arc::new(AtomicUsize::new(0));
        let (api, returned) = returning_api(
            endpoint,
            ReturningBoundary {
                inner: Boundary(calls.clone()),
                marker: Box::new(73),
                uncertain: false,
                fault: false,
            },
        );
        let r = api.request();
        if cancel {
            assert_eq!(api.bind("cancel", &r).0, 200);
            assert_eq!(
                api.advance(&r, Value::Null, false).1["outcome"]["outcome"]["state"],
                "CANCELLED"
            );
        }
        if release {
            assert_eq!(api.bind("release", &r).0, 200);
        }
        drop(api);
        assert_eq!(
            returned
                .recv_timeout(Duration::from_secs(1))
                .unwrap()
                .is_some(),
            release
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(matches!(upstream.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
    }
}
