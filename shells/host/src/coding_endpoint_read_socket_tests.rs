// SPDX-License-Identifier: MIT OR Apache-2.0
//! Synthetic socket + snapshot boundary, with the production read verifier/owner.

use super::*;
use crate::coding_endpoint::CodingEndpointConnection;
use crate::coding_endpoint_owner::{CodingEndpointAdmission, CodingEndpointRuns};
use crate::coding_endpoint_read_verifier::{CodingEndpointReadCheck, CodingEndpointReadVerifier};
use agentmage_capability_read_only::ReadOnlyResult;
use agentmage_kernel_contracts::{ModelCancellationProbe, RuntimeApprovalChallenge, WorkspacePath};

// The original snapshot boundary has a fixture-specific evidence shape. This
// wrapper only uses the production native-file evidence shape for the new test;
// it remains a SYNTHETIC permission/effect/receipt source, never native authority.
struct SnapshotBoundary(ReadBoundary);
impl RuntimeToolBoundary for SnapshotBoundary {
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
        scope: RuntimeRunScope<'_>,
        id: &RuntimeOperationId,
        definition: &ToolDefinition,
        call: &ToolCall,
        now: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.0.evaluate_scoped(scope, id, definition, call, now)
    }
    fn resolve_scoped(
        &mut self,
        scope: RuntimeRunScope<'_>,
        challenge: &RuntimeApprovalChallenge,
        response: &RuntimeApprovalResponse,
        definition: &ToolDefinition,
        call: &ToolCall,
        now: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        self.0
            .resolve_scoped(scope, challenge, response, definition, call, now)
    }
    fn execute_scoped(
        &mut self,
        scope: RuntimeRunScope<'_>,
        evaluation: &RuntimePermissionEvaluation,
        definition: &ToolDefinition,
        call: &ToolCall,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        let mut execution =
            self.0
                .execute_scoped(scope, evaluation, definition, call, cancellation)?;
        let read: ReadOnlyResult =
            serde_json::from_slice(&execution.result.output.as_ref().unwrap().bytes).unwrap();
        let evidence = &mut execution.result.evidence[0];
        evidence.source_id = format!("native:{}@{}", call.tool_id.as_str(), call.tool_version);
        evidence.object_id = call.tool_call_id.as_str().into();
        evidence.content_sha256 = read.result_sha256;
        Ok(execution)
    }
}

#[test]
fn endpoint_read_verifier_runs_in_retained_owner_with_approval_and_canonical_outcome() {
    for matches in [true, false] {
        let (listener, endpoint) = listener();
        let transport = transport(endpoint.clone());
        let approval = LocalChatApproval::for_endpoint(endpoint);
        let mut request = request();
        request.model_selection = transport.model_selection("fixture-model");
        let text = if matches {
            "pub fn fixture() -> u64 { 42 }\n"
        } else {
            "a different selected snapshot\n"
        };
        let check = CodingEndpointReadCheck::new(
            WorkspacePath::new(request.workspace_id.clone(), ["src", "lib.rs"]).unwrap(),
            text.len() as u64,
            sha256(text.as_bytes()),
        )
        .unwrap();
        request.task.acceptance_criteria = vec![check.criterion()];
        request.work_packet.acceptance_checks = vec![check.criterion()];
        request.work_packet.required_evidence = vec![EvidenceKind::ToolOutput];
        request.work_packet.authoritative_evidence.clear();
        request.work_packet.mutable_files.clear();
        request.work_packet.required_capability_class = AuthorityClass::Observe;
        let request = seal_runtime_endpoint_run_request(request).unwrap();
        let verifier = CodingEndpointReadVerifier::for_request(&request, vec![check]).unwrap();
        let objective = sha256(request.task.objective.as_bytes());
        let worker = thread::spawn(move || {
            for turn in 1..=2 {
                let mut socket = accept(&listener);
                let body = read_context_request(&mut socket);
                let user: serde_json::Value =
                    serde_json::from_str(body["messages"][1]["content"].as_str().unwrap()).unwrap();
                assert_eq!(user["turn"], turn);
                if turn == 1 {
                    respond(&mut socket, &read_proposal(), "stop", true);
                } else {
                    assert_eq!(
                        user["untrusted_tool_results"][0]["output"]["content"]["items"][0]["content"],
                        "pub fn fixture() -> u64 { 42 }\n"
                    );
                    let claim = CodingCompletionCandidate {
                        schema_version: 1,
                        objective_sha256: objective.clone(),
                        terminal_claim: CodingTerminalClaim::NoOp,
                        summary: "Synthetic read observation, not a coding qualification.".into(),
                        checks_not_run: vec![],
                        residual_risks: vec![],
                    };
                    respond(
                        &mut socket,
                        &serde_json::to_string(&claim).unwrap(),
                        "stop",
                        true,
                    );
                }
            }
        });
        let executions = Arc::new(AtomicUsize::new(0));
        let mut runs = CodingEndpointRuns::default();
        runs.admit(CodingEndpointAdmission {
            request: request.clone(),
            connection: CodingEndpointConnection::Borrowed {
                transport: &transport,
                approval: &approval,
            },
            registry: registry(),
            sources: &[],
            source_budget: 2048,
            include_usage: true,
            tool_boundary: SnapshotBoundary(ReadBoundary(executions.clone())),
            verifier,
            clock: TestClock(1000),
        })
        .unwrap();
        assert_eq!(executions.load(Ordering::SeqCst), 0);
        let RuntimeEndpointCoordinatorStep::AwaitingApproval { challenge } =
            runs.advance(&request, None, None).unwrap()
        else {
            panic!("exact approval expected")
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
        let RuntimeEndpointCoordinatorStep::Complete { outcome } =
            runs.advance(&request, Some(&response), None).unwrap()
        else {
            panic!("terminal expected")
        };
        worker.join().unwrap();
        assert_eq!(executions.load(Ordering::SeqCst), 1);
        assert_eq!(
            outcome.outcome.state,
            if matches {
                AgentStateKind::NoOp
            } else {
                AgentStateKind::Failed
            },
            "{outcome:?}"
        );
        assert_eq!(outcome.answer_evidence.is_some(), matches);
        assert_eq!(
            (
                outcome.outcome.model_call_count,
                outcome.outcome.tool_call_count
            ),
            (2, 1)
        );
        assert_eq!(
            runs.advance(&request, None, None).unwrap(),
            RuntimeEndpointCoordinatorStep::Complete { outcome }
        );
        assert_eq!(executions.load(Ordering::SeqCst), 1);
        assert!(!runs.events(&request).unwrap().is_empty());
        runs.release(&request).unwrap();
    }
}
