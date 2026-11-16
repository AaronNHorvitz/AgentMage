// SPDX-License-Identifier: MIT OR Apache-2.0
//! Synthetic creation/process observations, original edit and grant transactions.
use super::*;

fn combined(owner: &LinuxCodingRepository) -> RuntimeEndpointRunRequest {
    let (mut req, _) = binding(owner);
    let registry = linux_coding_endpoint_coding_registry().unwrap();
    req.visible_tools = runtime_tool_references(&registry).unwrap();
    req.tool_catalog_sha256 =
        runtime_tool_catalog_sha256(&req.tool_catalog_id, &req.visible_tools).unwrap();
    seal_runtime_endpoint_run_request(req).unwrap()
}
fn call(
    req: &RuntimeEndpointRunRequest,
    definition: &ToolDefinition,
    preview: Value,
    seq: u64,
) -> ToolCall {
    let bytes = serde_json::to_vec(&json!({"preview":preview})).unwrap();
    ToolCall {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_call_id: ToolCallId::from_raw(derived("endpoint-call", req, seq)),
        correlation_id: CorrelationId::from_raw(derived("correlation", req, 0)),
        action_id: runtime_action_id(&req.run_id, u32::try_from(seq).unwrap()),
        tool_id: definition.tool_id.clone(),
        tool_version: definition.tool_version.clone(),
        arguments: ContractPayload {
            schema: definition.input_schema.clone(),
            media_type: "application/json".into(),
            sha256: hash(&bytes),
            bytes,
        },
    }
}
fn command(
    owner: LinuxCodingRepository,
    req: &RuntimeEndpointRunRequest,
    seq: u64,
) -> LinuxCodingEndpointCommandBoundary<Clock> {
    let c = call(
        req,
        &EndpointCommandTool::new().0,
        owner.command.as_ref().unwrap().preview.clone(),
        seq,
    );
    let b = owner
        .into_endpoint_coding_command(
            req.clone(),
            RuntimeTurnId::from_raw(derived("turn", req, seq)),
            RuntimeOperationId::from_raw(derived("operation", req, seq)),
            c,
        )
        .ok()
        .unwrap();
    b.into_boundary(
        Arc::new(Control::new()),
        Clock(Arc::new(AtomicU64::new(NOW))),
    )
    .with_failure_observations()
}
fn apply(
    mut owner: LinuxCodingRepository,
    req: &RuntimeEndpointRunRequest,
    seq: u64,
    before: &[u8],
    after: &str,
) -> LinuxCodingRepository {
    let paths = vec!["one.py".to_owned()];
    let scope = owner.select_created_sources(&paths).unwrap();
    let preview = owner
        .prepare_edit(
            scope,
            &[crate::LinuxCodingReplacement {
                path: "one.py",
                before_sha256: &hash(before),
                content: after,
            }],
            "repair current source",
            NOW,
        )
        .unwrap();
    let registry = linux_coding_endpoint_edit_registry().unwrap();
    let def = registry.list_tools()[0];
    let c = call(req, def, preview, seq);
    let turn = RuntimeTurnId::from_raw(derived("turn", req, seq));
    let op = RuntimeOperationId::from_raw(derived("operation", req, seq));
    // Legacy binding still refuses the combined catalog, returning the owner.
    let (owner, _) = owner
        .into_endpoint_edit(req.clone(), turn.clone(), op.clone(), c.clone())
        .err()
        .unwrap();
    let edit = owner
        .into_endpoint_coding_edit(req.clone(), turn, op.clone(), c.clone())
        .ok()
        .unwrap();
    let challenge = edit.challenge().clone();
    let mut b = edit.into_boundary(
        Arc::new(Control::new()),
        Clock(Arc::new(AtomicU64::new(NOW))),
    );
    let scope = RuntimeRunScope::endpoint(req).unwrap();
    assert!(matches!(
        b.evaluate_scoped(scope, &op, def, &c, NOW).unwrap(),
        RuntimePermissionEvaluation::Ask { .. }
    ));
    let mut r = RuntimeApprovalResponse {
        schema_version: CONTRACT_SCHEMA_VERSION,
        run_id: req.run_id.clone(),
        approval_id: challenge.approval_id.clone(),
        disposition: RuntimeApprovalDisposition::Allow,
        challenge_sha256: hash(b"foreign"),
        grant_id: Some(challenge.proposed_grant_id.clone()),
        selection: None,
    };
    assert!(
        b.resolve_scoped(scope, &challenge, &r, def, &c, NOW)
            .is_err()
    );
    r.challenge_sha256 = challenge.challenge_sha256.clone();
    let e = b
        .resolve_scoped(scope, &challenge, &r, def, &c, NOW)
        .unwrap();
    let result = b.execute_scoped(scope, &e, def, &c, None).unwrap();
    assert_eq!(result.result.outcome, OperationOutcome::Succeeded);
    assert!(b.execute_scoped(scope, &e, def, &c, None).is_err());
    b.into_repository()
}

#[test]
fn endpoint_coding_catalog_preserves_both_original_definitions_and_legacy_refusals() {
    let registry = linux_coding_endpoint_coding_registry().unwrap();
    let originals = [
        linux_coding_endpoint_edit_registry().unwrap(),
        linux_coding_endpoint_command_registry().unwrap(),
    ];
    assert_eq!(registry.list_tools().len(), 2);
    for single in &originals {
        assert!(registry.list_tools().contains(&single.list_tools()[0]));
    }
    let f = Fixture::new();
    let owner = owner(&f, true, None);
    let req = combined(&owner);
    let c = call(
        &req,
        &EndpointCommandTool::new().0,
        owner.command.as_ref().unwrap().preview.clone(),
        1,
    );
    let (mut owner, _) = owner
        .into_endpoint_command(
            req.clone(),
            RuntimeTurnId::from_raw("turn"),
            RuntimeOperationId::from_raw("op"),
            c.clone(),
        )
        .err()
        .unwrap();
    for variant in 0..4 {
        let mut bad = req.clone();
        match variant {
            0 => bad.visible_tools.pop().map(|_| ()).unwrap(),
            1 => bad.visible_tools.reverse(),
            2 => bad.visible_tools[0].tool_version = "foreign".into(),
            3 => bad.visible_tools.push(bad.visible_tools[0].clone()),
            _ => unreachable!(),
        }
        bad.tool_catalog_sha256 =
            runtime_tool_catalog_sha256(&bad.tool_catalog_id, &bad.visible_tools)
                .unwrap_or_else(|_| hash(b"invalid"));
        if let Ok(bad) = seal_runtime_endpoint_run_request(bad) {
            let (returned, _) = owner
                .into_endpoint_coding_command(
                    bad,
                    RuntimeTurnId::from_raw("turn"),
                    RuntimeOperationId::from_raw("op"),
                    c.clone(),
                )
                .err()
                .expect("non-exact catalog refused");
            owner = returned;
        }
    }
    assert!(owner.command.is_some());
}

#[test]
fn endpoint_coding_failed_validation_returns_only_consumed_original_feedback_and_preserves_attempts()
 {
    for admit in [false, true] {
        let f = Fixture::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let owner = owner(&f, true, None);
        let req = combined(&owner);
        let mut b = command(owner, &req, 1);
        b = b
            .into_repair_repository()
            .err()
            .expect("unexecuted owner stays held");
        ask(&mut b);
        let e = resolve(&mut b, true).unwrap();
        let result = execute(&mut b, &e, Synthetic::new(&calls, false), None).unwrap();
        if admit {
            assert_eq!(failure_feedback(&mut b, &result), Ok(true));
        }
        let transferred = b.into_repair_repository();
        if !admit {
            assert!(transferred.is_err());
            continue;
        }
        let (mut owner, feedback) = transferred.ok().unwrap();
        assert!(owner.command.is_none());
        assert_eq!(owner.inspect_validation().unwrap()["attempts"], 1);
        assert_eq!(
            owner.inspect_validation().unwrap()["repair_requested"],
            true
        );
        assert!(owner.repair_feedback().is_err());
        assert!(
            owner
                .consume_repair_feedback(
                    feedback["receipt_sha256"].as_str().unwrap(),
                    feedback["feedback_sha256"].as_str().unwrap()
                )
                .is_err()
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(result.result.outcome, OperationOutcome::Failed);
        assert_eq!(feedback["attempt"], 1);
        assert_eq!(feedback["maximum_attempts"], 3);
    }
}

#[test]
fn endpoint_coding_alternates_real_edits_failed_feedback_and_fresh_exact_rerun_in_one_request() {
    let f = Fixture::new();
    fs::write(f.0.join("owned/worktree/one.py"), b"before\n").unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let owner = owner(&f, true, None);
    let req = combined(&owner);
    let mut owner = apply(owner, &req, 1, b"before\n", "first\n");
    prepare(&mut owner, true, None);
    let mut b = command(owner, &req, 2);
    ask(&mut b);
    let e = resolve(&mut b, true).unwrap();
    let failed = execute(&mut b, &e, Synthetic::new(&calls, false), None).unwrap();
    assert_eq!(failure_feedback(&mut b, &failed), Ok(true));
    let old = response(&b, true);
    let (owner, feedback) = b.into_repair_repository().ok().unwrap();
    let mut owner = apply(owner, &req, 3, b"first\n", "fixed\n");
    assert_eq!(owner.inspect_validation().unwrap()["attempts"], 1);
    prepare(&mut owner, true, feedback["receipt_sha256"].as_str());
    let mut b = command(owner, &req, 4);
    ask(&mut b);
    let challenge = b.command.challenge.clone();
    let c = b.command.call.clone();
    assert!(
        b.resolve_using(
            RuntimeRunScope::endpoint(&req).unwrap(),
            (&challenge, &old, &EndpointCommandTool::new().0, &c),
            NOW,
            |_| panic!("stale decision reached preflight")
        )
        .is_err()
    );
    let e = resolve(&mut b, true).unwrap();
    let passed = execute(&mut b, &e, Synthetic::new(&calls, true), None).unwrap();
    assert_eq!(passed.result.outcome, OperationOutcome::Succeeded);
    let value: Value = serde_json::from_slice(&passed.result.output.unwrap().bytes).unwrap();
    assert_eq!(value["validation"]["validation_passed"], true);
    assert_eq!(value["validation"]["attempt"], 2);
    assert_eq!(
        value["rerun_plan"]["prior_receipt_sha256"],
        feedback["receipt_sha256"]
    );
    assert_eq!(b.inspect().unwrap()["validation"]["attempts"], 2);
    assert_eq!(
        fs::read(f.0.join("owned/worktree/one.py")).unwrap(),
        b"fixed\n"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert!(
        b.into_repair_repository().is_err(),
        "a pass cannot mint repair feedback"
    );
}
