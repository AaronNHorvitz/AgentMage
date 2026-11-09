// SPDX-License-Identifier: Apache-2.0
//! Synthetic creation/model, real native edits and separately approved transfer.
use super::*;

#[path = "coding_endpoint_original_cli.rs"]
mod terminal;

#[test]
fn endpoint_original_review_requires_terminal_release_fresh_read_and_exact_apply() {
    for checked in [false, true] {
        let fixture = Fixture::new();
        fs::create_dir(fixture.0.join("original/.git")).unwrap();
        fs::write(fixture.0.join("original/.git/index"), b"index sentinel").unwrap();
        fs::write(fixture.0.join("original/human"), b"unrelated human work").unwrap();
        let mut owner = fixture.owner();
        if checked {
            // Native fixed-test preparation is unavailable in this lane.
            // Retain a real prior edit through the SAME owner, then deny the
            // combined run's first edit. This claims no test execution.
            fixture.prepare(&mut owner, "first\n");
            let edit = owner.inspect_edit().unwrap();
            let result = owner
                .decide_edit(
                    edit["pending"]["preview_sha256"].as_str().unwrap(),
                    true,
                    1000,
                )
                .unwrap();
            assert_eq!(result["outcome"], "committed");
            fixture.assert_bytes(b"first\n");
        }
        let roots = owner.inspect_apply().unwrap();
        let (upstream, endpoint) = upstream();
        let peer = thread::spawn(move || {
            let changes = if checked {
                vec![("first\n", "denied\n")]
            } else {
                vec![("before\n", "first\n"), ("first\n", "denied\n")]
            };
            for (old, new) in changes {
                let mut socket = accept(&upstream);
                read_upstream(&mut socket);
                respond(
                    &mut socket,
                    &json!({"edits":[{"path":"one.py","old":old,"new":new}]}).to_string(),
                );
            }
            upstream
        });
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let worker = thread::spawn(move || {
            if checked {
                agentmage_local_api::serve_coding_endpoint_edits_and_checked_tests_with_repository_review(
                    listener, ApiTokens::new(CLIENT.into(), CONTROL.into()).unwrap(), worker_stop,
                    CodingEndpointObservationAccess::DelegateObservations,
                    |id, session| Ok(checked_input(owner, endpoint, id, session)),
                )
            } else {
                agentmage_local_api::serve_coding_endpoint_strict_edits_with_repository_review(
                    listener,
                    ApiTokens::new(CLIENT.into(), CONTROL.into()).unwrap(),
                    worker_stop,
                    CodingEndpointObservationAccess::DelegateObservations,
                    |id, session| {
                        Ok(strict_input(
                            owner,
                            endpoint,
                            id,
                            session,
                            Arc::new(AtomicUsize::new(0)),
                        ))
                    },
                )
            }
        });
        let api = Api {
            address,
            stop,
            worker: Some(worker),
        };
        let apply = "/v1/code/repository/apply";
        let prepare = "/v1/code/repository/apply/prepare";
        let decide = "/v1/code/repository/apply/decide";
        let request = api.request();
        let body = json!({"paths":["one.py"],"task":"Review original transfer",
            "expected_repository_root_sha256":roots["repository_root_sha256"],
            "expected_worktree_root_sha256":roots["worktree_root_sha256"],"approve_read":true});
        assert_eq!(api.call("GET", apply, CLIENT, None).0, 403);
        assert_eq!(api.call("GET", apply, CONTROL, None).0, 428);
        assert_eq!(
            api.call("POST", prepare, CONTROL, Some(body.clone())).0,
            428
        );
        assert_eq!(api.bind("release", &request).0, 409);
        let (code, first) = api.advance(&request, Value::Null, true);
        assert_eq!(code, 200);
        assert_eq!(first["state"], "awaiting_approval");
        let second = if checked {
            first
        } else {
            let (code, second) = api.advance(&request, approval(&first["challenge"], true), true);
            assert_eq!(code, 200);
            assert_eq!(second["state"], "awaiting_approval");
            second
        };
        fixture.assert_bytes(b"first\n");
        assert_eq!(api.call("GET", apply, CONTROL, None).0, 428);
        let terminal = api.advance(&request, approval(&second["challenge"], false), true);
        assert_eq!(terminal.0, 200, "checked={checked} terminal={terminal:?}");
        assert_eq!(terminal.1["outcome"]["outcome"]["state"], "DECLINED");
        assert_eq!(api.call("GET", apply, CONTROL, None).0, 428);
        let mut foreign = request.clone();
        foreign["request_sha256"] = json!("0".repeat(64));
        assert_eq!(api.bind("release", &foreign).0, 409);
        assert_eq!(api.call("GET", apply, CONTROL, None).0, 428);
        let events = api.bind("events", &request);
        assert_eq!(events.0, 200);
        assert_eq!(
            api.bind("release", &request),
            (200, json!({"released":true}))
        );
        assert_eq!(api.bind("release", &request).0, 409);
        assert_eq!(api.advance(&request, Value::Null, true).0, 409);
        let (code, status) = api.call("GET", apply, CONTROL, None);
        assert_eq!(code, 200);
        assert_eq!(status["available_paths"], json!(["one.py"]));
        assert_eq!(
            status["repository_root_sha256"],
            roots["repository_root_sha256"]
        );
        assert_eq!(
            status["worktree_root_sha256"],
            roots["worktree_root_sha256"]
        );
        assert!(status["pending"].is_null());
        for route in [
            "/v1/code/repository/prepare",
            "/v1/code/repository/command/prepare",
            "/v1/code/repository/edit/prepare",
            "/v1/code/runtime/attach",
            "/v1/chat",
        ] {
            assert_eq!(api.call("POST", route, CONTROL, Some(json!({}))).0, 404);
        }
        assert_eq!(api.call("POST", prepare, CLIENT, Some(body.clone())).0, 403);
        let mut refused = body.clone();
        refused["approve_read"] = json!(false);
        assert_eq!(api.call("POST", prepare, CONTROL, Some(refused)).0, 403);
        let mut stale = body.clone();
        stale["expected_worktree_root_sha256"] = json!("0".repeat(64));
        assert_eq!(api.call("POST", prepare, CONTROL, Some(stale)).0, 409);
        let (code, preview) = api.call("POST", prepare, CONTROL, Some(body.clone()));
        assert_eq!(code, 200, "{preview}");
        assert_eq!(preview["operation"], "apply-to-original");
        fixture.assert_bytes(b"first\n");
        let choice = |hash: Value, approve: bool| json!({"preview_sha256":hash,"approve":approve});
        assert_eq!(
            api.call(
                "POST",
                decide,
                CLIENT,
                Some(choice(preview["preview_sha256"].clone(), true))
            )
            .0,
            403
        );
        assert_eq!(
            api.call(
                "POST",
                decide,
                CONTROL,
                Some(choice(json!("0".repeat(64)), true))
            )
            .0,
            409
        );
        fixture.assert_bytes(b"first\n");
        let (code, denial) = api.call(
            "POST",
            decide,
            CONTROL,
            Some(choice(preview["preview_sha256"].clone(), false)),
        );
        assert_eq!(code, 200);
        assert_eq!(denial["outcome"], "denied");
        assert_eq!(
            api.call(
                "POST",
                decide,
                CONTROL,
                Some(choice(preview["preview_sha256"].clone(), true))
            )
            .0,
            409
        );
        fixture.assert_bytes(b"first\n");
        let (code, preview) = api.call("POST", prepare, CONTROL, Some(body));
        assert_eq!(code, 200);
        let (code, result) = api.call(
            "POST",
            decide,
            CONTROL,
            Some(choice(preview["preview_sha256"].clone(), true)),
        );
        assert_eq!(code, 200, "{result}");
        assert_eq!(result["outcome"], "committed");
        assert_eq!(
            fs::read(fixture.0.join("original/one.py")).unwrap(),
            b"first\n"
        );
        assert_eq!(
            fs::read(fixture.0.join("owned/worktree/one.py")).unwrap(),
            b"first\n"
        );
        assert_eq!(
            fs::read(fixture.0.join("original/human")).unwrap(),
            b"unrelated human work"
        );
        assert_eq!(
            fs::read(fixture.0.join("original/.git/index")).unwrap(),
            b"index sentinel"
        );
        assert_eq!(
            api.call("GET", apply, CONTROL, None).1["available_paths"],
            json!([])
        );
        assert_eq!(
            api.call(
                "POST",
                decide,
                CONTROL,
                Some(choice(preview["preview_sha256"].clone(), true))
            )
            .0,
            409
        );
        drop(api);
        assert!(
            matches!(peer.join().unwrap().accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock)
        );
    }
}

#[test]
fn endpoint_original_review_cancelled_without_edits_cannot_invent_transfer_lineage() {
    let fixture = Fixture::new();
    let owner = fixture.owner();
    let roots = owner.inspect_apply().unwrap();
    let (upstream, endpoint) = upstream();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = stop.clone();
    let worker = thread::spawn(move || {
        agentmage_local_api::serve_coding_endpoint_edits_and_checked_tests_with_repository_review(
            listener,
            ApiTokens::new(CLIENT.into(), CONTROL.into()).unwrap(),
            worker_stop,
            CodingEndpointObservationAccess::ControlOnly,
            |id, session| Ok(checked_input(owner, endpoint, id, session)),
        )
    });
    let api = Api {
        address,
        stop,
        worker: Some(worker),
    };
    let request = api.request();
    assert_eq!(api.bind("cancel", &request).0, 200);
    assert_eq!(
        api.advance(&request, Value::Null, false).1["outcome"]["outcome"]["state"],
        "CANCELLED"
    );
    assert_eq!(api.bind("release", &request).0, 200);
    let apply = api.call("GET", "/v1/code/repository/apply", CONTROL, None);
    assert_eq!(apply.0, 200);
    assert_eq!(apply.1["available_paths"], json!([]));
    let refused = api.call(
        "POST",
        "/v1/code/repository/apply/prepare",
        CONTROL,
        Some(json!({
        "paths":["one.py"],"task":"No invented lineage","approve_read":true,
        "expected_repository_root_sha256":roots["repository_root_sha256"],
        "expected_worktree_root_sha256":roots["worktree_root_sha256"]})),
    );
    assert_eq!(refused.0, 409);
    fixture.assert_bytes(b"before\n");
    drop(api);
    assert!(matches!(upstream.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
}
