// SPDX-License-Identifier: MIT OR Apache-2.0
//! Explicit fixed-test criteria, not semantic certification of arbitrary tasks.

use std::collections::{BTreeMap, BTreeSet};

use agentmage_kernel_contracts::*;
use agentmage_kernel_engine::{
    command_runner::{CommandSpec, CommandWorkingDirectory},
    runtime_coordinator::verify_runtime_endpoint_run_request,
    runtime_loop::{
        RuntimeCompletionProposal, RuntimeCompletionVerificationInput, RuntimePortFailure,
        runtime_tool_references,
    },
    validation_result::{
        ValidationAttemptEvidence, ValidationRerunPlan, verify_bounded_validation_history,
    },
    validation_template::{ValidationKind, ValidationParserKind, ValidationTemplate},
};
use agentmage_platform_linux::{
    LinuxCodingValidationEvidence, linux_coding_endpoint_coding_registry,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{
    coding_endpoint_edit_previews::{CodingEndpointAcceptancePort, CodingEndpointValidation},
    coding_verifier::{CodingTerminalClaim, parse_candidate},
};

/// A person-selected, exact command/kind/parser/minimum-count criterion.
/// Construction grants no inspection, model disclosure, execution or approval.
/// Passing proves only this criterion; project tests remain untrusted code and
/// neither their coverage nor the meaning of an arbitrary objective is certified.
pub struct CodingEndpointFixedTestCheck {
    command: CommandSpec,
    kind: ValidationKind,
    parser: ValidationParserKind,
    minimum_test_count: u32,
}

impl CodingEndpointFixedTestCheck {
    /// Freeze the same trusted definition supplied to the combined native owner.
    /// Only the built-in per-test text parsers are supported here. A project's
    /// self-reported JSON envelope cannot select this accepting checker.
    pub fn new(definition: &CodingEndpointValidation) -> Result<Self, RuntimePortFailure> {
        use agentmage_platform_linux::LinuxCodingTestFormat;
        definition
            .command
            .verify()
            .map_err(|_| RuntimePortFailure::Invalid)?;
        let parser = match definition.output_format {
            LinuxCodingTestFormat::RustLibtestTextV1 => ValidationParserKind::RustLibtestTextV1,
            LinuxCodingTestFormat::PythonUnittestTextV1 => {
                ValidationParserKind::PythonUnittestTextV1
            }
            LinuxCodingTestFormat::AgentMageJsonV1 => return Err(RuntimePortFailure::Invalid),
        };
        if !definition.kind.is_test()
            || definition.minimum_test_count == 0
            || definition.command.working_directory != CommandWorkingDirectory::OwnedWorktree
        {
            return Err(RuntimePortFailure::Invalid);
        }
        Ok(Self {
            command: definition.command.clone(),
            kind: definition.kind,
            parser,
            minimum_test_count: definition.minimum_test_count,
        })
    }

    /// Stable acceptance criterion; display the selected definition alongside it
    /// before run sealing so the person can read more than its digest.
    #[must_use]
    pub fn criterion(&self) -> String {
        let definition = json!({"command": self.command, "kind": self.kind,
            "parser": self.parser, "minimum_test_count": self.minimum_test_count});
        format!(
            "Observe a complete passing offline test run for fixed definition SHA-256 {}. This checks the selected test run only.",
            hash(&serde_json::to_vec(&definition).expect("fixed definition"))
        )
    }
}

/// Concrete library checker for ONE explicit fixed-test criterion. Use through
/// `CodingEndpointEditPreviews::completion_verifier` so the original owner checks
/// linked validation and sealed current-source samples before and after dispatch.
/// No default launcher selection, broader task grading or durable proof is added.
pub struct CodingEndpointFixedTestVerifier {
    request: Box<RuntimeEndpointRunRequest>,
    check: CodingEndpointFixedTestCheck,
    verifier_id: VerifierId,
    postcondition: PostconditionId,
}

impl CodingEndpointFixedTestVerifier {
    /// Bind the final sealed combined request. Refuse unsupported or additional
    /// criteria rather than marking arbitrary postconditions as passed.
    pub fn for_request(
        request: &RuntimeEndpointRunRequest,
        check: CodingEndpointFixedTestCheck,
    ) -> Result<Self, RuntimePortFailure> {
        verify_runtime_endpoint_run_request(request).map_err(|_| RuntimePortFailure::Invalid)?;
        let criterion = check.criterion();
        let registry =
            linux_coding_endpoint_coding_registry().map_err(|_| RuntimePortFailure::Invalid)?;
        if request.mode != RuntimeSessionMode::ControlledWrite
            || request.event_cursor.is_some()
            || request.task.acceptance_criteria != [criterion.clone()]
            || request.work_packet.acceptance_checks != [criterion.clone()]
            || request.work_packet.required_evidence != [EvidenceKind::ToolOutput]
            || request.work_packet.required_capability_class != AuthorityClass::Execute
            || !request.work_packet.authoritative_evidence.is_empty()
            || request.visible_tools
                != runtime_tool_references(&registry).map_err(|_| RuntimePortFailure::Invalid)?
        {
            return Err(RuntimePortFailure::Invalid);
        }
        Ok(Self {
            request: Box::new(request.clone()),
            check,
            verifier_id: VerifierId::from_raw(format!(
                "endpoint-fixed-test:{}",
                &request.request_sha256[..24]
            )),
            postcondition: PostconditionId::from_raw(format!(
                "postcondition:0:{}",
                &hash(criterion.as_bytes())[..16]
            )),
        })
    }

    fn verify_records(
        &self,
        input: RuntimeCompletionVerificationInput<'_>,
        template: &ValidationTemplate,
        attempts: &[ValidationAttemptEvidence],
        reruns: &[ValidationRerunPlan],
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        if input.request.endpoint_request() != Some(self.request.as_ref())
            || !input.proposal.matches_scope(input.request)
            || input.postconditions != std::slice::from_ref(&self.postcondition)
        {
            return Err(RuntimePortFailure::Invalid);
        }
        let passed =
            parse_candidate(&input.proposal, &self.request.task.objective).is_some_and(|claim| {
                claim.terminal_claim == CodingTerminalClaim::Changed
                    && claim.checks_not_run.is_empty()
            }) && template.command == self.check.command
                && template.kind == self.check.kind
                && template.parser == self.check.parser
                && template.minimum_test_count == self.check.minimum_test_count
                && verify_bounded_validation_history(template, attempts, reruns)
                && self.correlates(&input, attempts);
        let evidence = if passed {
            input.evidence.to_vec()
        } else {
            vec![]
        };
        Ok(VerifierCandidate {
            schema_version: CONTRACT_SCHEMA_VERSION,
            verifier_record_id: VerifierRecordId::from_raw(format!(
                "endpoint-fixed-test-verification:{}",
                &hash(input.proposal.proposal_sha256().as_bytes())[..24]
            )),
            verifier_id: self.verifier_id.clone(),
            task_id: self.request.task.task_id.clone(),
            proposal_id: input.proposal.proposal_id().clone(),
            repository_snapshot_id: self.request.repository_snapshot_id.clone(),
            state_revision: input.state_revision,
            source: VerifierSource::DeterministicPostcondition,
            disposition: if passed {
                VerifierDisposition::Success
            } else {
                VerifierDisposition::Failed
            },
            postconditions: vec![PostconditionResult {
                postcondition_id: self.postcondition.clone(),
                passed,
                evidence,
            }],
        })
    }

    fn correlates(
        &self,
        input: &RuntimeCompletionVerificationInput<'_>,
        attempts: &[ValidationAttemptEvidence],
    ) -> bool {
        // The combined owner always emits edit/test pairs, including admitted
        // failures. No missing attempt, extra result or post-test edit is allowed.
        if input.tool_results.len() != attempts.len() * 2
            || input.tool_results.len() > self.request.limits.max_tool_calls as usize
            || input.receipt_ids.len() != input.tool_results.len()
            || input.evidence.len() != input.tool_results.len()
            || !unique(input.receipt_ids.iter().map(ReceiptId::as_str))
            || !unique(input.evidence.iter().map(|e| e.evidence_id.as_str()))
            || !unique(input.tool_results.iter().map(|r| r.tool_call_id.as_str()))
        {
            return false;
        }
        // The coordinator sorts aggregate evidence by ID, independently of
        // chronological tool results/receipts. Require an exact identity set
        // with complete reference equality; never correlate it by position.
        let evidence_by_id: BTreeMap<_, _> = input
            .evidence
            .iter()
            .map(|e| (e.evidence_id.as_str(), e))
            .collect();
        let mut result_evidence_ids = BTreeSet::new();
        let Ok(registry) = linux_coding_endpoint_coding_registry() else {
            return false;
        };
        let definitions = registry.list_tools();
        let Ok(command_registry) =
            agentmage_platform_linux::linux_coding_endpoint_command_registry()
        else {
            return false;
        };
        let command_id = &command_registry.list_tools()[0].tool_id;
        for (index, result) in input.tool_results.iter().enumerate() {
            let command = index % 2 == 1;
            let Some(payload) = &result.output else {
                return false;
            };
            let Some(definition) = definitions
                .iter()
                .find(|d| d.output_schema == payload.schema)
            else {
                return false;
            };
            // Resolve the actual registered family, not a model field or suffix.
            if (&definition.tool_id == command_id) != command
                || result.schema_version != CONTRACT_SCHEMA_VERSION
                || result.tool_call_id.as_str().is_empty()
                || result.error.is_some()
                || !result.validation_issues.is_empty()
                || result.evidence.len() != 1
                || payload.media_type != "application/json"
                || payload.bytes.len() as u64 > self.request.limits.max_output_bytes
                || payload.sha256 != hash(&payload.bytes)
            {
                return false;
            }
            let evidence = &result.evidence[0];
            if !result_evidence_ids.insert(evidence.evidence_id.as_str())
                || evidence_by_id.get(evidence.evidence_id.as_str()).copied() != Some(evidence)
                || evidence.schema_version != CONTRACT_SCHEMA_VERSION
                || evidence.kind != EvidenceKind::ToolOutput
                || evidence.fragment.is_some()
                || evidence.source_id
                    != format!(
                        "native:{}@{}",
                        definition.tool_id.as_str(),
                        definition.tool_version
                    )
                || evidence.object_id != result.tool_call_id.as_str()
                || evidence.content_sha256 != payload.sha256
                || evidence.observed_revision.as_deref()
                    != Some(self.request.repository_snapshot_id.as_str())
            {
                return false;
            }
            let Ok(value) = serde_json::from_slice::<Value>(&payload.bytes) else {
                return false;
            };
            if command {
                let attempt = &attempts[index / 2];
                let Ok(receipt) = serde_json::from_value::<Receipt>(value["receipt"].clone())
                else {
                    return false;
                };
                if receipt.schema_version != CONTRACT_SCHEMA_VERSION
                    || value["command_receipt"] != json!(attempt.command)
                    || value["validation"]["receipt"] != json!(attempt.validation)
                    || receipt.receipt_id != input.receipt_ids[index]
                    || receipt.authority_transaction_id.as_str()
                        != attempt.command.authority_transaction_id
                    || receipt.operation_attempt_id.as_str() != attempt.command.operation_attempt_id
                    || receipt.outcome != attempt.command.outcome
                    || result.outcome != receipt.outcome
                    || result.state_change != StateChange::NotChanged
                {
                    return false;
                }
            } else {
                let Some(last) = value["write_result"]["receipts"]
                    .as_array()
                    .and_then(|v| v.last())
                else {
                    return false;
                };
                let Some(transaction) = last["transaction_id"].as_str() else {
                    return false;
                };
                let Some(sequence) = last["sequence"].as_u64() else {
                    return false;
                };
                if result.outcome != OperationOutcome::Succeeded
                    || result.state_change != StateChange::Changed
                    || value["outcome"] != "committed"
                    || input.receipt_ids[index].as_str() != format!("{transaction}:{sequence}")
                {
                    return false;
                }
            }
        }
        true
    }
}

impl CodingEndpointAcceptancePort for CodingEndpointFixedTestVerifier {
    fn verifier_id(&self) -> &VerifierId {
        &self.verifier_id
    }
    fn verify_coding_completion(
        &mut self,
        input: RuntimeCompletionVerificationInput<'_>,
        validation: LinuxCodingValidationEvidence<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        self.verify_records(
            input,
            validation.template(),
            validation.attempts(),
            validation.reruns(),
        )
    }
}

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn unique<'a>(mut values: impl Iterator<Item = &'a str>) -> bool {
    let mut seen = BTreeSet::new();
    values.all(|value| !value.is_empty() && seen.insert(value))
}

#[cfg(test)]
#[path = "coding_endpoint_fixed_test_verifier_tests.rs"]
mod tests;
