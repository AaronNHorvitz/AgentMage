// SPDX-License-Identifier: Apache-2.0
//! Synthetic normalized records/endpoint claims; no native execution or grade.
use super::*;
use crate::coding_verifier::{CodingCompletionCandidate, coding_completion_schema};
use agentmage_kernel_engine::runtime_loop::{
    RuntimeEndpointPreparationInput, RuntimeEndpointTurnPort, RuntimePermissionEvaluation,
    RuntimePreparedEndpointTurn, RuntimeToolBoundary, RuntimeToolExecution, RuntimeVerifierPort,
};
use agentmage_kernel_engine::tooling::ToolRegistry;
use agentmage_kernel_engine::{
    command_runner::{
        CommandBounds, CommandReceipt, CommandRegistry, CommandRequest, CommandRisk, CommandSpec,
        CommandTermination, SealedCommandWorkspaceInput, prepare_command,
    },
    runtime_coordinator::seal_runtime_endpoint_run_request,
    runtime_loop::endpoint::{
        RuntimeEndpointContextPacket, RuntimeEndpointMessage, RuntimeEndpointMessageRole,
        RuntimeEndpointModelPort,
    },
    runtime_loop::{RuntimeClock, RuntimeVerificationInput},
    validation_result::{
        ValidationObservation, ValidationOutputClassification, normalize_validation_result,
        plan_failed_test_rerun,
    },
    validation_template::{
        ValidationTemplateInput, ValidationTemplateSource, seal_validation_template,
    },
};
use std::time::Duration;

struct TurnPort {
    selection: EndpointModelSelection,
    prepared: Option<(RuntimeEndpointContextPacket, Port)>,
}
impl RuntimeEndpointTurnPort for TurnPort {
    fn model_binding(&self) -> RuntimeModelBinding<'_> {
        RuntimeModelBinding::SelectedEndpoint {
            endpoint: &self.selection.endpoint,
            model: &self.selection.model,
        }
    }
    fn prepare<'a>(
        &mut self,
        _: RuntimeEndpointPreparationInput<'_>,
        control: &dyn ModelOperationControl,
    ) -> Result<RuntimePreparedEndpointTurn<'a>, RuntimePortFailure>
    where
        Self: 'a,
    {
        control
            .remaining_ms()
            .map_err(|_| RuntimePortFailure::Invalid)?;
        let (context, port) = self.prepared.take().ok_or(RuntimePortFailure::Invalid)?;
        Ok(
            RuntimePreparedEndpointTurn::new(context, port, coding_completion_schema())
                .completion_only(),
        )
    }
}
struct ProbeVerifier<'a> {
    fixture: &'a Fixture,
    checker: &'a CodingEndpointFixedTestVerifier,
    postconditions: &'a [PostconditionId],
    observed: &'a std::cell::RefCell<Option<Result<VerifierCandidate, RuntimePortFailure>>>,
}
impl RuntimeVerifierPort for ProbeVerifier<'_> {
    fn verifier_id(&self) -> &VerifierId {
        &self.checker.verifier_id
    }
    fn verify(
        &mut self,
        _: RuntimeVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        panic!("endpoint claim only")
    }
    fn verify_completion(
        &mut self,
        input: RuntimeCompletionVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        assert!(self.observed.borrow().is_none());
        // Reuse the actual admitted claim and scope, but supply the pure
        // component's synthetic records. Never return this marker candidate to
        // the coordinator as evidence of real edits or native task success.
        let result = self.checker.verify_records(
            RuntimeVerificationInput {
                request: input.request,
                proposal: input.proposal,
                state_revision: 8,
                postconditions: self.postconditions,
                evidence: &self.fixture.evidence,
                tool_results: &self.fixture.results,
                receipt_ids: &self.fixture.receipts,
            },
            &self.fixture.template,
            &self.fixture.attempts,
            &self.fixture.reruns,
        );
        self.observed.replace(Some(result));
        Err(RuntimePortFailure::Invalid)
    }
}
struct NoTools;
impl RuntimeToolBoundary for NoTools {
    fn evaluate(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimeOperationId,
        _: &ToolDefinition,
        _: &ToolCall,
        _: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure> {
        panic!("no tools")
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
        panic!("no tools")
    }
    fn execute(
        &mut self,
        _: &RuntimeRunRequest,
        _: &RuntimePermissionEvaluation,
        _: &ToolDefinition,
        _: &ToolCall,
        _: Option<&dyn ModelCancellationProbe>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure> {
        panic!("no tools")
    }
}

fn definition() -> CodingEndpointValidation {
    CodingEndpointValidation {
        command: CommandSpec::seal(
            "fixed-tests",
            "1.0.0",
            "/usr/bin/cargo",
            "1".repeat(64),
            vec!["test".into()],
            CommandWorkingDirectory::OwnedWorktree,
            Default::default(),
            CommandRisk::Moderate,
            CommandBounds::new(1000, 8192, 8192, 16777216, 1, 100).unwrap(),
        )
        .unwrap(),
        kind: ValidationKind::Unit,
        minimum_test_count: 1,
        output_format: agentmage_platform_linux::LinuxCodingTestFormat::RustLibtestTextV1,
    }
}
struct Fixture {
    request: RuntimeEndpointRunRequest,
    template: ValidationTemplate,
    attempts: Vec<ValidationAttemptEvidence>,
    reruns: Vec<ValidationRerunPlan>,
    results: Vec<ToolResult>,
    receipts: Vec<ReceiptId>,
    evidence: Vec<EvidenceReference>,
}
impl Fixture {
    fn new(count: usize) -> Self {
        let registry = linux_coding_endpoint_coding_registry().unwrap();
        let mut request = request(&registry);
        request.mode = RuntimeSessionMode::ControlledWrite;
        request.work_packet.required_capability_class = AuthorityClass::Execute;
        request.limits.max_output_bytes = 32768;
        request.limits.max_tool_calls = 6;
        request
            .work_packet
            .budgets
            .iter_mut()
            .find(|b| b.resource == BudgetResource::ToolCalls)
            .unwrap()
            .limit = 6;
        let criterion = CodingEndpointFixedTestCheck::new(&definition())
            .unwrap()
            .criterion();
        request.task.acceptance_criteria = vec![criterion.clone()];
        request.work_packet.acceptance_checks = vec![criterion];
        request.work_packet.mutable_files = vec!["src/lib.rs".into()];
        request.work_packet.budgets.extend(
            [
                (BudgetResource::ToolCallDepth, 1),
                (BudgetResource::InputBytes, 81920),
                (BudgetResource::OutputBytes, 32768),
                (BudgetResource::ElapsedMilliseconds, 60000),
                (BudgetResource::MemoryBytes, 16777216),
                (BudgetResource::DiskBytes, 32768),
                (BudgetResource::ProcessCount, 1),
            ]
            .map(|(resource, limit)| BudgetLimit { resource, limit }),
        );
        request = seal_runtime_endpoint_run_request(request).unwrap();
        let d = definition();
        let template = seal_validation_template(ValidationTemplateInput {
            validation_id: "local-project-tests".into(),
            kind: d.kind,
            command: d.command,
            source: ValidationTemplateSource::ExplicitUserInput,
            source_sha256: "2".repeat(64),
            source_path: None,
            user_input_approval_sha256: Some("2".repeat(64)),
            execution_scope_sha256: "3".repeat(64),
            parser: ValidationParserKind::RustLibtestTextV1,
            parser_version: "1.0.0".into(),
            parser_sha256: "4".repeat(64),
            minimum_test_count: 1,
            focused: true,
            fail_fast: false,
            failed_test_rerun_allowed: true,
            setup_idempotent: true,
            expected_artifacts: vec![],
        })
        .unwrap();
        let mut f = Self {
            request,
            template,
            attempts: vec![],
            reruns: vec![],
            results: vec![],
            receipts: vec![],
            evidence: vec![],
        };
        for n in 0..count {
            let passed = n == count - 1;
            let prepared = prepare_command(
                &CommandRegistry::build(vec![f.template.command.clone()]).unwrap(),
                CommandRequest::new(format!("command-{n}"), &f.template.command),
            )
            .unwrap();
            let stdout = if passed {
                b"running 1 test\ntest sample ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n".to_vec()
            } else {
                b"running 1 test\ntest sample ... FAILED\n\nfailures:\n\n---- sample stdout ----\nassertion failed\n\nfailures:\n    sample\n\ntest result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n".to_vec()
            };
            let mut command = CommandReceipt {
                schema_version: 1,
                sealed_workspace_input: Some(SealedCommandWorkspaceInput {
                    schema_version: 1,
                    worktree_root_sha256: "a".repeat(64),
                    observation_sha256: "b".repeat(64),
                    projection_sha256: hash(format!("synthetic-input-{n}").as_bytes()),
                    files: 1,
                    directories: 1,
                    file_bytes: 1,
                }),
                authority_transaction_id: format!("transaction-{n}"),
                operation_attempt_id: format!("operation-{n}"),
                command_attempt_id: prepared.preview().command_attempt_id.clone(),
                template_id: prepared.command().template_id.clone(),
                template_version: prepared.command().template_version.clone(),
                spec_sha256: prepared.command().spec_sha256.clone(),
                request_sha256: prepared.request_sha256().into(),
                preview_sha256: prepared.preview().preview_sha256.clone(),
                termination: CommandTermination::Exited,
                outcome: if passed {
                    OperationOutcome::Succeeded
                } else {
                    OperationOutcome::Failed
                },
                exit_code: Some(i32::from(!passed)),
                signal: None,
                stdout_sha256: hash(&stdout),
                stdout_total_bytes: stdout.len() as u64,
                stdout_retained_bytes: stdout.len() as u64,
                stdout_truncated: false,
                stderr_sha256: hash(b""),
                stderr_total_bytes: 0,
                stderr_retained_bytes: 0,
                stderr_truncated: false,
                elapsed_ms: 10,
                resource_usage: None,
                descendants_terminated: true,
                platform_code: "synthetic.exited".into(),
                receipt_sha256: "0".repeat(64),
            };
            command.receipt_sha256 = hash(&serde_json::to_vec(&command).unwrap());
            let validation = normalize_validation_result(
                &f.template,
                &prepared,
                &command,
                ValidationObservation {
                    validation_attempt_id: format!("validation-{n}"),
                    approval_sha256: hash(format!("approval-{n}").as_bytes()),
                    stdout,
                    stderr: vec![],
                    stdout_classification: ValidationOutputClassification::Internal,
                    stderr_classification: ValidationOutputClassification::Internal,
                    secret_match_count: 0,
                    artifacts: vec![],
                    affected_files: vec![],
                    changed_paths: vec![],
                    baseline_failure_sha256s: vec![],
                    baseline_known_clean: false,
                    unverified_kinds: vec![],
                },
            )
            .unwrap();
            assert_eq!(validation.is_full_pass(), passed, "{:?}", validation.status);
            if let Some(prior) = f.attempts.last() {
                f.reruns.push(
                    plan_failed_test_rerun(
                        &f.template,
                        &prior.validation,
                        validation.validation_attempt_id.clone(),
                        validation.approval_sha256.clone(),
                    )
                    .unwrap(),
                );
            }
            let edit_registry =
                agentmage_platform_linux::linux_coding_endpoint_edit_registry().unwrap();
            let command_registry =
                agentmage_platform_linux::linux_coding_endpoint_command_registry().unwrap();
            f.push(edit_registry.list_tools()[0], json!({"outcome":"committed","write_result":{"receipts":[{"transaction_id":format!("edit-{n}"),"sequence":1}]}}),
                OperationOutcome::Succeeded, StateChange::Changed, ReceiptId::from_raw(format!("edit-{n}:1")));
            // The native authority receipt is synthetic; no original ownership or
            // native process execution is manufactured by this pure checker fixture.
            let mut receipt: Receipt = serde_json::from_str(include_str!(
                "../../../fixtures/contracts/v2/valid/receipt.json"
            ))
            .unwrap();
            receipt.receipt_id = ReceiptId::from_raw(format!("receipt-{n}"));
            receipt.authority_transaction_id =
                AuthorityTransactionId::from_raw(&command.authority_transaction_id);
            receipt.operation_attempt_id =
                OperationAttemptId::from_raw(&command.operation_attempt_id);
            receipt.outcome = command.outcome;
            receipt.error = None;
            f.push(command_registry.list_tools()[0], json!({"receipt":receipt,"command_receipt":command,"validation":{"receipt":validation}}),
                command.outcome, StateChange::NotChanged, receipt.receipt_id.clone());
            f.attempts.push(ValidationAttemptEvidence {
                prepared,
                command,
                validation,
            });
        }
        assert!(verify_bounded_validation_history(
            &f.template,
            &f.attempts,
            &f.reruns
        ));
        // Match the native producers' IDs and the original coordinator's
        // evidence_id sort, independently of edit/test result order.
        f.evidence
            .sort_by(|a, b| a.evidence_id.as_str().cmp(b.evidence_id.as_str()));
        f
    }
    fn push(
        &mut self,
        definition: &ToolDefinition,
        value: Value,
        outcome: OperationOutcome,
        state_change: StateChange,
        receipt: ReceiptId,
    ) {
        let id = format!("call-{}", self.results.len());
        let bytes = serde_json::to_vec(&value).unwrap();
        let evidence = EvidenceReference {
            schema_version: CONTRACT_SCHEMA_VERSION,
            evidence_id: EvidenceId::from_raw(format!(
                "native-{}:{}",
                if state_change == StateChange::Changed {
                    "edit"
                } else {
                    "command"
                },
                &hash(&bytes)[..32]
            )),
            kind: EvidenceKind::ToolOutput,
            source_id: format!(
                "native:{}@{}",
                definition.tool_id.as_str(),
                definition.tool_version
            ),
            object_id: id.clone(),
            fragment: None,
            content_sha256: hash(&bytes),
            observed_revision: Some(self.request.repository_snapshot_id.as_str().into()),
        };
        self.results.push(ToolResult {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw(id),
            correlation_id: CorrelationId::from_raw("synthetic-correlation"),
            outcome,
            output: Some(ContractPayload {
                schema: definition.output_schema.clone(),
                media_type: "application/json".into(),
                sha256: hash(&bytes),
                bytes,
            }),
            validation_issues: vec![],
            evidence: vec![evidence.clone()],
            error: None,
            elapsed_ms: 1,
            state_change,
        });
        self.receipts.push(receipt);
        self.evidence.push(evidence);
    }
    fn verify(
        &self,
        claim_kind: CodingTerminalClaim,
        checks: Vec<String>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        let verifier = CodingEndpointFixedTestVerifier::for_request(
            &self.request,
            CodingEndpointFixedTestCheck::new(&definition()).unwrap(),
        )?;
        self.verify_with(
            &verifier,
            std::slice::from_ref(&verifier.postcondition),
            claim_kind,
            checks,
        )
    }
    fn verify_with(
        &self,
        verifier: &CodingEndpointFixedTestVerifier,
        postconditions: &[PostconditionId],
        claim_kind: CodingTerminalClaim,
        checks: Vec<String>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        let text = serde_json::to_string(&CodingCompletionCandidate {
            schema_version: 1,
            objective_sha256: hash(self.request.task.objective.as_bytes()),
            terminal_claim: claim_kind,
            summary: "Synthetic claim; not native task success".into(),
            checks_not_run: checks,
            residual_risks: vec![],
        })
        .unwrap();
        let context = RuntimeEndpointContextPacket::from_messages(&[RuntimeEndpointMessage {
            role: RuntimeEndpointMessageRole::User,
            content: "synthetic completion",
        }])
        .unwrap();
        let model = EndpointModelRequest::from_prepared_body(
            self.request.model_selection.clone(),
            context.messages().clone(),
            EndpointModelBody {
                request_body_bytes: 200,
                request_body_sha256: "b".repeat(64),
            },
            EndpointModelLimits {
                requested_max_output_tokens: 16,
                provider_usage_requested: false,
                total_timeout: Duration::from_millis(100),
                idle_timeout: Duration::from_millis(50),
                response_wire_limit_bytes: 8192,
                response_body_limit_bytes: 8192,
                response_text_limit_bytes: 4096,
            },
        );
        // Obtain a real accepted CONTROLLED endpoint claim through the original
        // coordinator. The records below remain synthetic, not native effects.
        let observed = std::cell::RefCell::new(None);
        {
            let port = TurnPort {
                selection: self.request.model_selection.clone(),
                prepared: Some((
                    context,
                    Port {
                        request: model,
                        text,
                    },
                )),
            };
            let check = ProbeVerifier {
                fixture: self,
                checker: verifier,
                postconditions,
                observed: &observed,
            };
            let mut coordinator =
                agentmage_kernel_engine::runtime_loop::ReusableRuntimeCoordinator::new_endpoint(
                    self.request.clone(),
                    port,
                    linux_coding_endpoint_coding_registry().unwrap(),
                    NoTools,
                    check,
                    Clock,
                )
                .unwrap();
            let agentmage_kernel_engine::runtime_loop::RuntimeEndpointCoordinatorStep::Complete {
                outcome,
            } = coordinator.run_endpoint_until_boundary(None, None).unwrap()
            else {
                panic!("synthetic completion only")
            };
            assert_eq!(outcome.outcome.state, AgentStateKind::Failed);
            assert_eq!(outcome.outcome.model_call_count, 1);
            assert_eq!(outcome.outcome.tool_call_count, 0);
            assert!(outcome.outcome.output.is_none());
        }
        observed
            .into_inner()
            .expect("checker invoked on accepted controlled claim")
    }
}

#[test]
fn fixed_test_checker_refuses_valid_foreign_definition_request_and_postcondition() {
    for mutation in 0..4 {
        let mut f = Fixture::new(1);
        let mut d = definition();
        match mutation {
            0 => d.kind = ValidationKind::Integration,
            1 => d.minimum_test_count = 2,
            2 => {
                d.output_format =
                    agentmage_platform_linux::LinuxCodingTestFormat::PythonUnittestTextV1
            }
            3 => {
                d.command = CommandSpec::seal(
                    "alternate-tests",
                    "1.0.0",
                    "/usr/bin/cargo",
                    "1".repeat(64),
                    vec!["test".into()],
                    CommandWorkingDirectory::OwnedWorktree,
                    Default::default(),
                    CommandRisk::Moderate,
                    CommandBounds::new(1000, 8192, 8192, 16777216, 1, 100).unwrap(),
                )
                .unwrap()
            }
            _ => unreachable!(),
        }
        let check = CodingEndpointFixedTestCheck::new(&d).unwrap();
        f.request.task.acceptance_criteria = vec![check.criterion()];
        f.request.work_packet.acceptance_checks = vec![check.criterion()];
        f.request = seal_runtime_endpoint_run_request(f.request).unwrap();
        let verifier = CodingEndpointFixedTestVerifier::for_request(&f.request, check).unwrap();
        // The original history remains valid and agrees with the coordinator.
        // Only the separately selected trusted definition differs.
        assert!(verify_bounded_validation_history(
            &f.template,
            &f.attempts,
            &f.reruns
        ));
        let candidate = f
            .verify_with(
                &verifier,
                std::slice::from_ref(&verifier.postcondition),
                CodingTerminalClaim::Changed,
                vec![],
            )
            .unwrap();
        assert_eq!(
            candidate.disposition,
            VerifierDisposition::Failed,
            "definition {mutation}"
        );
    }
    let mut f = Fixture::new(1);
    let verifier = CodingEndpointFixedTestVerifier::for_request(
        &f.request,
        CodingEndpointFixedTestCheck::new(&definition()).unwrap(),
    )
    .unwrap();
    for postconditions in [
        vec![],
        vec![PostconditionId::from_raw("foreign-postcondition")],
    ] {
        assert_eq!(
            f.verify_with(
                &verifier,
                &postconditions,
                CodingTerminalClaim::Changed,
                vec![]
            ),
            Err(RuntimePortFailure::Invalid)
        );
    }
    f.request
        .task
        .objective
        .push_str(" validly resealed foreign request");
    f.request = seal_runtime_endpoint_run_request(f.request).unwrap();
    assert_eq!(
        f.verify_with(
            &verifier,
            std::slice::from_ref(&verifier.postcondition),
            CodingTerminalClaim::Changed,
            vec![]
        ),
        Err(RuntimePortFailure::Invalid)
    );
}

#[test]
fn fixed_test_checker_accepts_only_exact_correlated_one_to_three_attempt_records() {
    for count in 1..=3 {
        let f = Fixture::new(count);
        let candidate = f.verify(CodingTerminalClaim::Changed, vec![]).unwrap();
        assert_eq!(candidate.disposition, VerifierDisposition::Success);
        assert!(candidate.postconditions[0].passed);
        assert_eq!(candidate.postconditions[0].evidence, f.evidence);
        let registry = agentmage_kernel_engine::agent_verifier::VerifierRegistry::new(
            agentmage_kernel_engine::agent_verifier::VerifierContext {
                verifier_id: candidate.verifier_id.clone(),
                task_id: candidate.task_id.clone(),
                proposal_id: candidate.proposal_id.clone(),
                repository_snapshot_id: candidate.repository_snapshot_id.clone(),
                state_revision: 8,
                postconditions: vec![candidate.postconditions[0].postcondition_id.clone()],
            },
        )
        .unwrap();
        registry.verify(&candidate).unwrap();
    }
}

#[test]
fn fixed_test_checker_correlates_native_ids_in_coordinator_sorted_and_permuted_evidence() {
    for count in 1..=3 {
        let mut f = Fixture::new(count);
        assert!(
            f.evidence[0]
                .evidence_id
                .as_str()
                .starts_with("native-command:")
        );
        assert!(
            f.results[0].evidence[0]
                .evidence_id
                .as_str()
                .starts_with("native-edit:")
        );
        assert!(
            f.evidence
                .windows(2)
                .all(|pair| pair[0].evidence_id.as_str() < pair[1].evidence_id.as_str())
        );
        let original_results = f.results.clone();
        let original_receipts = f.receipts.clone();
        // The aggregate is a set; tool results and receipt IDs keep their
        // original chronological order, including earlier failed validations.
        for shift in 0..f.evidence.len() {
            f.evidence.rotate_left(shift);
            let candidate = f.verify(CodingTerminalClaim::Changed, vec![]).unwrap();
            assert_eq!(
                candidate.disposition,
                VerifierDisposition::Success,
                "{count}:{shift}"
            );
            assert_eq!(candidate.postconditions[0].evidence, f.evidence);
        }
        f.evidence.reverse();
        assert_eq!(
            f.verify(CodingTerminalClaim::Changed, vec![])
                .unwrap()
                .disposition,
            VerifierDisposition::Success
        );
        assert_eq!(f.results, original_results);
        assert_eq!(f.receipts, original_receipts);
    }
}

#[test]
fn fixed_test_checker_requires_exact_evidence_set_but_ordered_results_and_receipts() {
    for mutation in 0..10 {
        let mut f = Fixture::new(2);
        match mutation {
            0 => {
                f.evidence.pop();
            }
            1 => f.evidence.push(f.evidence[0].clone()),
            2 => f.evidence[1] = f.evidence[0].clone(),
            3 => f.evidence[0].evidence_id = EvidenceId::from_raw("foreign-evidence"),
            // A matching ID must match the complete original reference, not
            // just its payload digest or source family.
            4 => f.evidence[0].object_id = "foreign-call".into(),
            5 => f.evidence[0].source_id = "foreign-source".into(),
            6 => f.evidence[0].observed_revision = Some("foreign-revision".into()),
            7 => f.results[1].evidence[0] = f.results[0].evidence[0].clone(),
            8 => f.results.swap(1, 3),
            9 => f.receipts.swap(1, 3),
            _ => unreachable!(),
        }
        let candidate = f.verify(CodingTerminalClaim::Changed, vec![]).unwrap();
        assert_eq!(
            candidate.disposition,
            VerifierDisposition::Failed,
            "mutation {mutation}"
        );
        assert!(!candidate.postconditions[0].passed);
        assert!(candidate.postconditions[0].evidence.is_empty());
    }
}

#[test]
fn fixed_test_checker_refuses_unrun_or_no_op_claims_and_changed_original_history() {
    let f = Fixture::new(2);
    for (kind, checks) in [
        (CodingTerminalClaim::NoOp, vec![]),
        (CodingTerminalClaim::Changed, vec!["not run".into()]),
    ] {
        assert_eq!(
            f.verify(kind, checks).unwrap().disposition,
            VerifierDisposition::Failed
        );
    }
    for mutation in 0..8 {
        let mut f = Fixture::new(2);
        match mutation {
            0 => f.template.minimum_test_count += 1,
            1 => f.template.parser = ValidationParserKind::AgentMageJsonV1,
            2 => f.attempts[0].validation.failed += 1,
            3 => f.attempts[1].command.spec_sha256 = "e".repeat(64),
            4 => {
                f.attempts.remove(0);
            }
            5 => f.reruns.clear(),
            6 => f.reruns[0].approval_sha256 = "e".repeat(64),
            7 => {
                f.attempts[1].validation.status =
                    agentmage_kernel_engine::validation_result::ValidationStatus::Flaky
            }
            _ => unreachable!(),
        }
        let candidate = f.verify(CodingTerminalClaim::Changed, vec![]).unwrap();
        assert_eq!(
            candidate.disposition,
            VerifierDisposition::Failed,
            "mutation {mutation}"
        );
        assert!(candidate.postconditions[0].evidence.is_empty());
    }
}

#[test]
fn fixed_test_checker_refuses_coordinator_record_substitution_or_missing_failed_attempt() {
    for mutation in 0..16 {
        let mut f = Fixture::new(2);
        match mutation {
            0 => {
                f.results.remove(1);
                f.receipts.remove(1);
                f.evidence.remove(1);
            }
            1 => f.receipts[1] = f.receipts[0].clone(),
            2 => f.receipts[3] = ReceiptId::from_raw("foreign-receipt"),
            3 => f.evidence[3].content_sha256 = "e".repeat(64),
            4 => f.results[3].evidence[0].observed_revision = Some("stale".into()),
            5 => f.results[3].output.as_mut().unwrap().schema.schema_version += 1,
            6 => f.results[3].output.as_mut().unwrap().sha256 = "e".repeat(64),
            7 => f.results[2].state_change = StateChange::NotChanged,
            8 => f.results[3].outcome = OperationOutcome::Failed,
            9 => {
                f.results.swap(2, 3);
                f.receipts.swap(2, 3);
                f.evidence.swap(2, 3);
            }
            10 => f.results[3].tool_call_id = f.results[1].tool_call_id.clone(),
            11 => f.evidence[3].evidence_id = f.evidence[1].evidence_id.clone(),
            12..=15 => {
                let output = f.results[3].output.as_mut().unwrap();
                let mut value: Value = serde_json::from_slice(&output.bytes).unwrap();
                match mutation {
                    12 => {
                        value["command_receipt"]["sealed_workspace_input"]["projection_sha256"] =
                            json!("e".repeat(64))
                    }
                    13 => value["validation"]["receipt"]["passed"] = json!(999),
                    14 => value["receipt"]["operation_attempt_id"] = json!("foreign-operation"),
                    15 => {
                        value["receipt"]["authority_transaction_id"] = json!("foreign-transaction")
                    }
                    _ => unreachable!(),
                }
                output.bytes = serde_json::to_vec(&value).unwrap();
                output.sha256 = hash(&output.bytes);
                f.results[3].evidence[0].content_sha256 = output.sha256.clone();
                let changed = &f.results[3].evidence[0];
                *f.evidence
                    .iter_mut()
                    .find(|e| e.evidence_id == changed.evidence_id)
                    .unwrap() = changed.clone();
            }
            _ => unreachable!(),
        }
        assert_eq!(
            f.verify(CodingTerminalClaim::Changed, vec![])
                .unwrap()
                .disposition,
            VerifierDisposition::Failed,
            "mutation {mutation}"
        );
    }
}

#[test]
fn fixed_test_checker_constructor_refuses_envelopes_non_tests_and_extra_criteria() {
    for mutation in 0..5 {
        let mut d = definition();
        match mutation {
            0 => d.output_format = agentmage_platform_linux::LinuxCodingTestFormat::AgentMageJsonV1,
            1 => d.kind = ValidationKind::Build,
            2 => d.minimum_test_count = 0,
            3 => d.command.working_directory = CommandWorkingDirectory::EmptyScratch,
            4 => d.command.spec_sha256 = "e".repeat(64),
            _ => unreachable!(),
        }
        assert!(CodingEndpointFixedTestCheck::new(&d).is_err());
    }
    for mutation in 0..6 {
        let mut f = Fixture::new(1);
        match mutation {
            0 => f
                .request
                .task
                .acceptance_criteria
                .push("arbitrary additional meaning".into()),
            1 => f.request.work_packet.acceptance_checks[0].push_str(" altered"),
            2 => f
                .request
                .work_packet
                .required_evidence
                .push(EvidenceKind::Validation),
            3 => f.request.mode = RuntimeSessionMode::EphemeralReadOnly,
            4 => f.request.visible_tools.pop().map(|_| ()).unwrap(),
            5 => f.request.work_packet.required_capability_class = AuthorityClass::Observe,
            _ => unreachable!(),
        }
        // Catalog mutation is refused by the original seal first; other cases
        // are validly resealed, so the constructor's contract is exercised.
        if mutation != 4 {
            f.request = seal_runtime_endpoint_run_request(f.request).unwrap();
        }
        assert!(
            CodingEndpointFixedTestVerifier::for_request(
                &f.request,
                CodingEndpointFixedTestCheck::new(&definition()).unwrap()
            )
            .is_err()
        );
    }
}

fn request(registry: &ToolRegistry) -> RuntimeEndpointRunRequest {
    let task_id = TaskId::from_raw("endpoint-policy-task");
    let session_id = SessionId::from_raw("endpoint-policy-session");
    let objective = "Read the selected source with exact approval";
    let check = "Ground observations in current source";
    let catalog_id = ToolCatalogId::from_raw("endpoint-policy-catalog");
    let visible_tools = runtime_tool_references(registry).unwrap();
    seal_runtime_endpoint_run_request(RuntimeEndpointRunRequest {
        schema_version: CONTRACT_SCHEMA_VERSION,
        run_id: RuntimeRunId::from_raw("endpoint-policy-run"),
        session_id: session_id.clone(),
        mode: RuntimeSessionMode::EphemeralReadOnly,
        task: Task {
            schema_version: CONTRACT_SCHEMA_VERSION,
            task_id: task_id.clone(),
            session_id,
            objective: objective.into(),
            acceptance_criteria: vec![check.into()],
            constraints: vec!["Read only; no network".into()],
            status: TaskStatus::Ready,
        },
        work_packet: WorkPacket {
            schema_version: CONTRACT_SCHEMA_VERSION,
            work_packet_id: WorkPacketId::from_raw("endpoint-policy-packet"),
            task_id,
            revision: 1,
            objective: objective.into(),
            reason: "Exercise the kernel authority boundary".into(),
            owner: "synthetic-local-actor".into(),
            authoritative_evidence: vec![],
            mutable_files: vec![],
            protected_files: vec!["private".into()],
            expected_output: "Grounded source observations".into(),
            acceptance_checks: vec![check.into()],
            required_evidence: vec![EvidenceKind::ToolOutput],
            required_capability_class: AuthorityClass::Observe,
            budgets: vec![
                BudgetLimit {
                    resource: BudgetResource::PlanSteps,
                    limit: 5,
                },
                BudgetLimit {
                    resource: BudgetResource::ModelCalls,
                    limit: 5,
                },
                BudgetLimit {
                    resource: BudgetResource::ToolCalls,
                    limit: 4,
                },
            ],
            stop_conditions: [
                StopConditionKind::AcceptanceSatisfied,
                StopConditionKind::UserDecisionRequired,
                StopConditionKind::PolicyDenied,
                StopConditionKind::Error,
                StopConditionKind::Cancelled,
                StopConditionKind::BudgetExhausted,
                StopConditionKind::UncertainResult,
            ]
            .into_iter()
            .map(|kind| StopCondition {
                kind,
                description: format!("Stop for {kind:?}"),
            })
            .collect(),
            rollback: RollbackPlan {
                reversible: true,
                description: "No workspace changes".into(),
            },
            sensitivity: DataSensitivity::Ephemeral,
            last_verification_date: "2026-11-08".into(),
            next_action: None,
            next_review: None,
            status_reason: None,
            disposition: None,
            completion_evidence: vec![],
            superseding_work: None,
            validation_issues: vec![],
            plan_id: Some(PlanId::from_raw("endpoint-policy-plan")),
            state: WorkPacketState::Active,
        },
        workspace_id: WorkspaceId::from_raw("fixed-test-workspace"),
        workspace_snapshot_sha256: "a".repeat(64),
        repository_snapshot_id: RepositorySnapshotId::from_raw("endpoint-policy-snapshot"),
        repository_snapshot_sha256: "b".repeat(64),
        model_selection: EndpointModelSelection {
            endpoint: "http://127.0.0.1:1234/v1".into(),
            model: "synthetic-model".into(),
        },
        context_budget: RuntimeEndpointContextBudget {
            max_messages: 8,
            max_content_bytes: 4096,
            max_serialized_messages_bytes: 8192,
            max_request_body_bytes: 16384,
        },
        max_output_tokens: 128,
        tool_catalog_sha256:
            agentmage_kernel_engine::runtime_coordinator::runtime_tool_catalog_sha256(
                &catalog_id,
                &visible_tools,
            )
            .unwrap(),
        tool_catalog_id: catalog_id,
        visible_tools,
        policy_id: PolicyId::from_raw("unbound-startup-policy"),
        policy_sha256: "c".repeat(64),
        limits: RuntimeRunLimits {
            max_turns: 5,
            max_model_calls: 5,
            max_tool_calls: 4,
            max_repeated_tool_calls: 2,
            max_tool_call_depth: 1,
            max_no_progress_turns: 2,
            max_context_refreshes: 5,
            max_events: 128,
            max_elapsed_ms: 60000,
            max_output_bytes: 4096,
        },
        event_cursor: None,
        request_sha256: "0".repeat(64),
    })
    .unwrap()
}

struct Clock;
impl RuntimeClock for Clock {
    fn now_epoch_ms(&mut self) -> Result<u64, RuntimePortFailure> {
        Ok(1000)
    }
}
struct Port {
    request: EndpointModelRequest,
    text: String,
}
impl RuntimeEndpointModelPort for Port {
    type Error = RuntimePortFailure;
    fn request(&self) -> &EndpointModelRequest {
        &self.request
    }
    fn dispatch(
        self,
        control: &dyn ModelOperationControl,
        deliver: &mut dyn FnMut(&str),
    ) -> Result<EndpointModelResult, Self::Error> {
        control
            .remaining_ms()
            .map_err(|_| RuntimePortFailure::Invalid)?;
        deliver(&self.text);
        Ok(EndpointModelResult::from_completed_response(
            self.request,
            EndpointModelFinish::Stop,
            hash(self.text.as_bytes()),
            self.text.len(),
            Duration::from_millis(1),
            None,
        ))
    }
}
