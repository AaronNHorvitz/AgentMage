// SPDX-License-Identifier: Apache-2.0
//! Context boundaries and socket composition, using synthetic source/model/authority fixtures.

use super::*;
use crate::{
    coding_endpoint_context::CodingEndpointContext,
    coding_tools::model_visible_coding_tools,
    coding_verifier::{CodingCompletionCandidate, CodingTerminalClaim, coding_completion_schema},
};
use agentmage_capability_repository_map::coding_proposal::CodingSource;
use agentmage_kernel_engine::{runtime_loop::runtime_tool_references, tooling::ToolRegistry};

fn read_context_request(socket: &mut TcpStream) -> serde_json::Value {
    socket
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    socket
        .set_write_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut header = vec![];
    while !header.ends_with(b"\r\n\r\n") {
        let mut byte = [0];
        socket.read_exact(&mut byte).unwrap();
        header.push(byte[0]);
        assert!(header.len() < 8192);
    }
    let header = String::from_utf8(header).unwrap();
    assert!(header.starts_with("POST /selected/api/chat/completions HTTP/1.1\r\n"));
    assert!(header.contains("Authorization: Bearer synthetic-transport-key\r\n"));
    let length = header
        .lines()
        .find_map(|s| s.strip_prefix("Content-Length: "))
        .unwrap()
        .parse::<usize>()
        .unwrap();
    assert!(length <= 128 * 1024);
    let mut body = vec![0; length];
    socket.read_exact(&mut body).unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["model"], "fixture-model");
    assert_eq!(body["stream_options"]["include_usage"], true);
    body
}

fn registry() -> ToolRegistry {
    let mut registry = read_only_runtime_registry().unwrap();
    registry.retain_tools(|d| d.tool_id.as_str() == ReadOnlyToolKind::ReadText.id());
    registry
}

fn request() -> RuntimeEndpointRunRequest {
    let mut request = endpoint_request(EndpointModelSelection {
        endpoint: "http://127.0.0.1:12345/selected/api".into(),
        model: "fixture-model".into(),
    });
    request.visible_tools = runtime_tool_references(&registry()).unwrap();
    request.tool_catalog_sha256 =
        runtime_tool_catalog_sha256(&request.tool_catalog_id, &request.visible_tools).unwrap();
    seal_runtime_endpoint_run_request(request).unwrap()
}

fn sources() -> Vec<CodingSource> {
    vec![CodingSource {
        path: "src/lib.rs".into(),
        content: format!("pub fn fixture() -> u32 {{ 42 }}\n// {PRIVATE_TEXT}\n"),
    }]
}

fn input(request: &RuntimeEndpointRunRequest, turn: u32) -> RuntimeEndpointPreparationInput<'_> {
    RuntimeEndpointPreparationInput {
        request,
        turn,
        completed_tool_calls: &[],
        tool_results: &[],
        evidence: &[],
        rejected_tool_calls: &[],
    }
}

fn available() -> Control {
    Control(Ok(NonZeroU64::new(1000).unwrap()))
}

#[test]
fn endpoint_context_preserves_original_schema_source_ranges_and_untrusted_roles() {
    let request = request();
    let sources = sources();
    let mut context = CodingEndpointContext::new(&request, &registry(), &sources, 2048).unwrap();
    let messages = context
        .build_messages(input(&request, 1), &available())
        .unwrap();
    assert!(matches!(messages[0].role, ChatRole::System));
    assert!(matches!(messages[1].role, ChatRole::User));
    let system: serde_json::Value = serde_json::from_str(messages[0].content).unwrap();
    let user: serde_json::Value = serde_json::from_str(messages[1].content).unwrap();
    assert!(!messages[0].content.contains(PRIVATE_TEXT));
    assert!(!messages[0].content.contains("native_profile"));
    let original = model_visible_coding_tools(&registry()).unwrap();
    assert_eq!(system["tools"].as_array().unwrap().len(), original.len());
    for (definition, original) in system["tools"].as_array().unwrap().iter().zip(original) {
        assert_eq!(
            *definition,
            serde_json::to_value(&original.definition).unwrap()
        );
        assert_eq!(
            system["input_schemas"][original.definition.input_schema.schema_sha256],
            original.input_schema
        );
    }
    assert_eq!(
        system["completion_schema"],
        serde_json::to_value(coding_completion_schema()).unwrap()
    );
    assert_eq!(user["task"], serde_json::to_value(&request.task).unwrap());
    assert_eq!(
        user["objective_sha256"],
        sha256(request.task.objective.as_bytes())
    );
    assert_eq!(
        user["source_coverage"][0]["sha256"],
        sha256(sources[0].content.as_bytes())
    );
    for excerpt in user["untrusted_source_excerpts"].as_array().unwrap() {
        let start = excerpt["start"].as_u64().unwrap() as usize;
        let end = excerpt["end"].as_u64().unwrap() as usize;
        assert_eq!(excerpt["text"], sources[0].content[start..end]);
    }
    assert!(
        user["untrusted_source_excerpts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["text"].as_str().unwrap().contains("fixture()"))
    );
    assert_eq!(user["untrusted_tool_results"], serde_json::json!([]));
    assert!(!serde_json::to_string(&user).unwrap().contains("127.0.0.1"));
}

#[test]
fn endpoint_context_refuses_foreign_sealed_run_repeated_skipped_and_mutated_turns() {
    let request = request();
    let mut foreign = request.clone();
    foreign.run_id = RuntimeRunId::from_raw("another-endpoint-run");
    let foreign = seal_runtime_endpoint_run_request(foreign).unwrap();
    let mut unsealed = request.clone();
    unsealed.task.objective.push_str(" forged task");
    for (run, turn) in [(&foreign, 1), (&unsealed, 1), (&request, 0), (&request, 2)] {
        let mut context =
            CodingEndpointContext::new(&request, &registry(), &sources(), 2048).unwrap();
        assert!(matches!(
            context.build_messages(input(run, turn), &available()),
            Err(RuntimePortFailure::Invalid)
        ));
        // A refused preparation cannot be repaired into a fresh attempt.
        assert!(matches!(
            context.build_messages(input(&request, 1), &available()),
            Err(RuntimePortFailure::Invalid)
        ));
    }
    let mut context = CodingEndpointContext::new(&request, &registry(), &[], 2048).unwrap();
    context
        .build_messages(input(&request, 1), &available())
        .unwrap();
    assert!(matches!(
        context.build_messages(input(&request, 1), &available()),
        Err(RuntimePortFailure::Invalid)
    ));
}

#[test]
fn endpoint_context_rejects_catalog_source_and_message_count_mismatches_before_preparation() {
    let request = request();
    assert!(matches!(
        CodingEndpointContext::new(
            &request,
            &read_only_runtime_registry().unwrap(),
            &sources(),
            2048
        ),
        Err(RuntimePortFailure::Invalid)
    ));
    for path in [
        "../outside.rs",
        "/absolute.rs",
        ".git/config",
        "src/lib.rs/../x",
    ] {
        let mut source = sources();
        source[0].path = path.into();
        assert!(matches!(
            CodingEndpointContext::new(&request, &registry(), &source, 2048),
            Err(RuntimePortFailure::Invalid)
        ));
    }
    let mut narrower = request.clone();
    narrower.context_budget.max_messages = 1;
    let narrower = seal_runtime_endpoint_run_request(narrower).unwrap();
    assert!(matches!(
        CodingEndpointContext::new(&narrower, &registry(), &[], 2048),
        Err(RuntimePortFailure::Invalid)
    ));
    for budget in [0, 2047, 32769, usize::MAX] {
        assert!(matches!(
            CodingEndpointContext::new(&request, &registry(), &[], budget),
            Err(RuntimePortFailure::Invalid)
        ));
    }
}

#[test]
fn endpoint_context_includes_all_current_results_and_refusals_or_refuses_whole_context() {
    let request = request();
    let NativeReadModelStep::Tool { tool_id, arguments } = read_model_step(
        &registry(),
        ReadOnlyToolKind::ReadText,
        ReadOnlyRequest {
            schema_version: 1,
            paths: vec![vec!["src".into(), "lib.rs".into()]],
            query: None,
            byte_offset: Some(0),
            byte_count: Some(256),
            encoding: ReadOnlyEncoding::Utf8,
            limits: ReadOnlyLimits::default(),
            call_depth: 0,
        },
    ) else {
        unreachable!()
    };
    let call = ToolCall {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_call_id: agentmage_kernel_contracts::ToolCallId::from_raw("context-call"),
        correlation_id: agentmage_kernel_contracts::CorrelationId::from_raw("context-correlation"),
        action_id: agentmage_kernel_contracts::ActionId::from_raw("context-action"),
        tool_id,
        tool_version: "1.0.0".into(),
        arguments,
    };
    let calls = vec![call.clone()];
    let results = vec![ToolResult {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_call_id: call.tool_call_id.clone(),
        correlation_id: call.correlation_id.clone(),
        outcome: OperationOutcome::Failed,
        output: None,
        validation_issues: vec![],
        evidence: vec![],
        error: None,
        elapsed_ms: 1,
        state_change: StateChange::NotChanged,
    }];
    let refusals = vec![agentmage_kernel_contracts::RuntimeToolRejection {
        call,
        reason: agentmage_kernel_contracts::RuntimeToolRejectionReason::ArgumentsInvalid,
    }];
    let prepared = || RuntimeEndpointPreparationInput {
        request: &request,
        turn: 1,
        completed_tool_calls: &calls,
        tool_results: &results,
        evidence: &[],
        rejected_tool_calls: &refusals,
    };
    let mut context = CodingEndpointContext::new(&request, &registry(), &[], 2048).unwrap();
    let messages = context.build_messages(prepared(), &available()).unwrap();
    let user: serde_json::Value = serde_json::from_str(messages[1].content).unwrap();
    assert_eq!(
        user["actual_completed_calls"],
        serde_json::to_value(&calls).unwrap()
    );
    assert_eq!(
        user["untrusted_tool_results"],
        serde_json::to_value(&results).unwrap()
    );
    assert_eq!(
        user["pre_effect_refusals"],
        serde_json::to_value(&refusals).unwrap()
    );
    let mut context = CodingEndpointContext::new(&request, &registry(), &[], 2048).unwrap();
    let mut mismatched = prepared();
    mismatched.tool_results = &[];
    assert!(matches!(
        context.build_messages(mismatched, &available()),
        Err(RuntimePortFailure::Invalid)
    ));
    let mut oversized = results.clone();
    oversized[0].output = Some(payload("fixture.oversize", &vec![b'X'; 100_000]));
    let mut context = CodingEndpointContext::new(&request, &registry(), &[], 2048).unwrap();
    let mut too_large = prepared();
    too_large.tool_results = &oversized;
    assert!(matches!(
        context.build_messages(too_large, &available()),
        Err(RuntimePortFailure::ResourceExhausted)
    ));
    assert!(matches!(
        context.build_messages(input(&request, 1), &available()),
        Err(RuntimePortFailure::Invalid)
    ));
}

#[test]
fn endpoint_context_enforces_exact_utf8_content_and_encoded_message_ceilings_and_stops() {
    let original = request();
    let mut context = CodingEndpointContext::new(&original, &registry(), &sources(), 2048).unwrap();
    let messages = context
        .build_messages(input(&original, 1), &available())
        .unwrap();
    let content: usize = messages.iter().map(|m| m.content.len()).sum();
    let encoded = serde_json::to_vec(&messages.iter().map(|m| serde_json::json!({"role":match m.role {ChatRole::System=>"system",ChatRole::User=>"user",ChatRole::Assistant=>"assistant"},"content":m.content})).collect::<Vec<_>>()).unwrap().len();
    for (content_limit, json_limit, pass) in [
        (content, encoded, true),
        (content - 1, encoded, false),
        (content, encoded - 1, false),
    ] {
        let mut exact = original.clone();
        exact.context_budget.max_content_bytes = content_limit as u64;
        exact.context_budget.max_serialized_messages_bytes = json_limit as u64;
        let exact = seal_runtime_endpoint_run_request(exact).unwrap();
        let mut context =
            CodingEndpointContext::new(&exact, &registry(), &sources(), 2048).unwrap();
        let result = context.build_messages(input(&exact, 1), &available());
        assert_eq!(result.is_ok(), pass);
        if !pass {
            assert!(matches!(result, Err(RuntimePortFailure::ResourceExhausted)));
        }
    }
    for stop in [
        ModelOperationStop::Cancelled,
        ModelOperationStop::TimedOut,
        ModelOperationStop::Unavailable,
        ModelOperationStop::Invalid,
    ] {
        let mut context = CodingEndpointContext::new(&original, &registry(), &[], 2048).unwrap();
        assert!(
            context
                .build_messages(input(&original, 1), &Control(Err(stop)))
                .is_err()
        );
        assert!(matches!(
            context.build_messages(input(&original, 1), &available()),
            Err(RuntimePortFailure::Invalid)
        ));
    }
}

// Synthetic deterministic verifier for this fixture only, not live coding admission.
struct ContextVerifier(VerifierId);
impl RuntimeVerifierPort for ContextVerifier {
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
        input: RuntimeCompletionVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        let claim: CodingCompletionCandidate =
            serde_json::from_slice(&input.proposal.payload().unwrap().bytes).unwrap();
        let grounded = input.proposal.matches_scope(input.request)
            && claim.objective_sha256 == sha256(input.request.task().objective.as_bytes())
            && claim.terminal_claim == CodingTerminalClaim::NoOp
            && input.tool_results.len() == 1
            && input.receipt_ids.len() == 1
            && claim.summary == "fixture function returns 42";
        Ok(VerifierCandidate {
            schema_version: CONTRACT_SCHEMA_VERSION,
            verifier_record_id: VerifierRecordId::from_raw("context-verifier-record"),
            verifier_id: self.0.clone(),
            task_id: input.request.task().task_id.clone(),
            proposal_id: input.proposal.proposal_id().clone(),
            repository_snapshot_id: input.request.repository_snapshot_id().clone(),
            state_revision: input.state_revision,
            source: VerifierSource::DeterministicPostcondition,
            disposition: VerifierDisposition::NoOp,
            postconditions: input
                .postconditions
                .iter()
                .map(|id| PostconditionResult {
                    postcondition_id: id.clone(),
                    passed: grounded,
                    evidence: input.evidence.to_vec(),
                })
                .collect(),
        })
    }
}

#[test]
fn endpoint_context_drives_two_socket_turns_through_existing_approval_and_verification() {
    let (listener, endpoint) = listener();
    let mut request = request();
    request.model_selection = transport(endpoint.clone()).model_selection("fixture-model");
    let request = seal_runtime_endpoint_run_request(request).unwrap();
    let objective = sha256(request.task.objective.as_bytes());
    let worker = thread::spawn(move || {
        for turn in 1..=2 {
            let mut socket = accept(&listener);
            let request = read_context_request(&mut socket);
            let system: serde_json::Value =
                serde_json::from_str(request["messages"][0]["content"].as_str().unwrap()).unwrap();
            let user: serde_json::Value =
                serde_json::from_str(request["messages"][1]["content"].as_str().unwrap()).unwrap();
            assert_eq!(user["turn"], turn);
            assert_eq!(
                user["untrusted_tool_results"].as_array().unwrap().len(),
                turn as usize - 1
            );
            assert_eq!(system["tools"].as_array().unwrap().len(), 1);
            assert_eq!(user["objective_sha256"], objective);
            if turn == 1 {
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
    });
    let transport = transport(endpoint.clone());
    let approval = LocalChatApproval::for_endpoint(endpoint);
    let port = ConfiguredCodingEndpoint::for_coding_run(
        &transport,
        &approval,
        &request,
        &registry(),
        &sources(),
        2048,
        true,
    )
    .unwrap();
    let executions = Arc::new(AtomicUsize::new(0));
    let mut coordinator = ReusableRuntimeCoordinator::new_endpoint(
        request,
        port,
        registry(),
        ReadBoundary(executions.clone()),
        ContextVerifier(VerifierId::from_raw("context-verifier")),
        TestClock(1000),
    )
    .unwrap();
    let RuntimeEndpointCoordinatorStep::AwaitingApproval { challenge } =
        coordinator.run_endpoint_until_boundary(None, None).unwrap()
    else {
        panic!("approval expected")
    };
    assert_eq!(executions.load(Ordering::SeqCst), 0);
    let response = RuntimeApprovalResponse {
        schema_version: CONTRACT_SCHEMA_VERSION,
        run_id: challenge.run_id.clone(),
        approval_id: challenge.approval_id.clone(),
        disposition: RuntimeApprovalDisposition::Allow,
        challenge_sha256: challenge.challenge_sha256.clone(),
        grant_id: Some(challenge.proposed_grant_id.clone()),
        selection: None,
    };
    let RuntimeEndpointCoordinatorStep::Complete { outcome } = coordinator
        .run_endpoint_until_boundary(Some(&response), None)
        .unwrap()
    else {
        panic!("terminal expected")
    };
    worker.join().unwrap();
    assert_eq!(executions.load(Ordering::SeqCst), 1);
    assert_eq!(outcome.outcome.state, AgentStateKind::NoOp, "{outcome:?}");
    assert_eq!(
        (
            outcome.outcome.model_call_count,
            outcome.outcome.tool_call_count
        ),
        (2, 1)
    );
    assert!(outcome.answer_evidence.is_some());
    assert_eq!(
        coordinator.run_endpoint_until_boundary(None, None).unwrap(),
        RuntimeEndpointCoordinatorStep::Complete { outcome }
    );
}

#[test]
fn endpoint_context_overflow_terminates_before_socket_or_permission_entry() {
    let (listener, endpoint) = listener();
    let transport = transport(endpoint.clone());
    let approval = LocalChatApproval::for_endpoint(endpoint);
    let mut request = request();
    request.model_selection = transport.model_selection("fixture-model");
    let request = seal_runtime_endpoint_run_request(request).unwrap();
    let mut context = CodingEndpointContext::new(&request, &registry(), &[], 2048).unwrap();
    let messages = context
        .build_messages(input(&request, 1), &available())
        .unwrap();
    let minimal: usize = messages.iter().map(|m| m.content.len()).sum();
    let mut narrow = request.clone();
    narrow.context_budget.max_content_bytes = minimal as u64;
    let narrow = seal_runtime_endpoint_run_request(narrow).unwrap();
    let source = sources();
    let port = ConfiguredCodingEndpoint::for_coding_run(
        &transport,
        &approval,
        &narrow,
        &registry(),
        &source,
        2048,
        true,
    )
    .unwrap();
    let executions = Arc::new(AtomicUsize::new(0));
    let mut coordinator = ReusableRuntimeCoordinator::new_endpoint(
        narrow,
        port,
        registry(),
        ReadBoundary(executions.clone()),
        ContextVerifier(VerifierId::from_raw("context-overflow-verifier")),
        TestClock(1000),
    )
    .unwrap();
    let RuntimeEndpointCoordinatorStep::Complete { outcome } =
        coordinator.run_endpoint_until_boundary(None, None).unwrap()
    else {
        panic!("whole terminal expected")
    };
    assert_eq!(outcome.outcome.state, AgentStateKind::Exhausted);
    assert_eq!(executions.load(Ordering::SeqCst), 0);
    assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
    assert!(!coordinator.events().iter().any(|e| matches!(
        e.kind,
        agentmage_kernel_contracts::RuntimeEventKind::ModelCompleted { .. }
    )));
    let wrong = super::endpoint_request(transport.model_selection("other-model"));
    // A different selected model is allowed only when it is also the sealed
    // run's selection; changing only the address cannot be silently repaired.
    let mut wrong_address = wrong;
    wrong_address.model_selection.endpoint = "http://127.0.0.1:1/elsewhere".into();
    let wrong_address = seal_runtime_endpoint_run_request(wrong_address).unwrap();
    assert!(matches!(
        ConfiguredCodingEndpoint::for_coding_run(
            &transport,
            &approval,
            &wrong_address,
            &registry(),
            &[],
            2048,
            true
        ),
        Err(RuntimePortFailure::Invalid)
    ));
}
