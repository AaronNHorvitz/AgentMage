//! Deterministic completion verification for bounded coding runs.

use std::collections::{BTreeSet, HashSet};
use std::fmt::Write;

use agentmage_capability_read_only::{
    GIT_INSPECTION_OUTPUT_SCHEMA_ID, GitInspectionOperation, GitInspectionOutcome,
    GitInspectionResult,
};
use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, ContractPayload, EvidenceKind, EvidenceReference, ModelProposalKind,
    OperationOutcome, PostconditionResult, StateChange, VerifierCandidate, VerifierDisposition,
    VerifierId, VerifierRecordId, VerifierSource,
};
use agentmage_kernel_engine::{
    runtime_loop::{
        RuntimeCompletionProposal, RuntimeCompletionVerificationInput, RuntimePortFailure,
        RuntimeRunScope, RuntimeScopedVerificationInput, RuntimeVerificationInput,
        RuntimeVerifierPort,
    },
    validation_result::ValidationReceipt,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    coding_changes::CONTROLLED_CHANGE_OUTPUT_SCHEMA_ID, coding_session::CodingSessionProfile,
    coding_tools::TARGETED_VALIDATION_OUTPUT_SCHEMA_ID,
};

/// Exact schema identity for an inert model completion claim.
pub const CODING_COMPLETION_INPUT_SCHEMA_ID: &str = "agentmage.code.completion-candidate";

/// Closed JSON Schema shown to the local model for one completion claim.
pub const CODING_COMPLETION_INPUT_SCHEMA_JSON: &str = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","$id":"agentmage.code.completion-candidate","type":"object","additionalProperties":false,"required":["schema_version","objective_sha256","terminal_claim","summary","checks_not_run","residual_risks"],"properties":{"schema_version":{"const":1},"objective_sha256":{"type":"string","pattern":"^[0-9a-f]{64}$"},"terminal_claim":{"enum":["changed","no_op"]},"summary":{"type":"string","minLength":1,"maxLength":16384},"checks_not_run":{"type":"array","maxItems":128,"items":{"type":"string","minLength":1,"maxLength":512}},"residual_risks":{"type":"array","maxItems":128,"items":{"type":"string","minLength":1,"maxLength":512}}}}"#;

/// Model's inert terminal claim; only the deterministic verifier selects the runtime state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodingTerminalClaim {
    /// The model claims at least one approved byte change completed.
    Changed,
    /// The model claims current evidence proves no change is required.
    NoOp,
}

/// Bounded model-authored completion candidate reviewed against trusted runtime facts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodingCompletionCandidate {
    /// Closed candidate schema version.
    pub schema_version: u16,
    /// Digest of the exact current user objective.
    pub objective_sha256: String,
    /// Inert changed/no-op claim.
    pub terminal_claim: CodingTerminalClaim,
    /// Bounded user-facing narrative; never completion authority.
    pub summary: String,
    /// Checks the model explicitly reports as unrun.
    pub checks_not_run: Vec<String>,
    /// Residual risks retained for the final renderer.
    pub residual_risks: Vec<String>,
}

/// Profile-bound deterministic verifier shared by every coding interface.
pub struct CodingCompletionVerifier {
    verifier_id: VerifierId,
    workspace_snapshot_sha256: String,
    repository_snapshot_sha256: String,
    validation_templates: BTreeSet<(String, String)>,
    changed_validation_ids: BTreeSet<String>,
    unconditional_validation_ids: BTreeSet<String>,
}

impl CodingCompletionVerifier {
    /// Binds one verifier to the profile's worktree and registered validations.
    #[must_use]
    pub fn for_profile(profile: &CodingSessionProfile) -> Self {
        let changed_validation_ids = profile
            .change_plan()
            .planned_validation_ids()
            .iter()
            .cloned()
            .collect();
        let unconditional_validation_ids = profile
            .coding_guidance()
            .required_validation_ids()
            .iter()
            .cloned()
            .collect();
        Self {
            verifier_id: VerifierId::from_raw(format!(
                "coding-verifier:{}",
                &profile.profile_sha256()[..24]
            )),
            workspace_snapshot_sha256: profile.worktree().record_sha256.clone(),
            repository_snapshot_sha256: profile.repository_snapshot_sha256().to_owned(),
            validation_templates: profile
                .validations()
                .templates
                .iter()
                .map(|template| {
                    (
                        template.validation_id.clone(),
                        template.template_sha256.clone(),
                    )
                })
                .collect(),
            changed_validation_ids,
            unconditional_validation_ids,
        }
    }

    fn evaluate<P: RuntimeCompletionProposal>(
        &self,
        input: &RuntimeVerificationInput<'_, RuntimeRunScope<'_>, P>,
    ) -> Option<VerifierDisposition> {
        let candidate = parse_candidate(&input.proposal, &input.request.task().objective)?;
        if input.request.workspace_snapshot_sha256() != self.workspace_snapshot_sha256
            || input.request.repository_snapshot_sha256() != self.repository_snapshot_sha256
            || input.tool_results.is_empty()
            || input.receipt_ids.len() != input.tool_results.len()
            || input
                .receipt_ids
                .iter()
                .map(|receipt| receipt.as_str())
                .collect::<HashSet<_>>()
                .len()
                != input.receipt_ids.len()
            || input.tool_results.iter().any(|result| {
                result.outcome != OperationOutcome::Succeeded
                    || matches!(result.state_change, StateChange::Uncertain)
            })
            || !required_evidence_present(input)
        {
            return None;
        }

        let write_indexes = input
            .tool_results
            .iter()
            .enumerate()
            .filter(|(_, result)| {
                result.state_change == StateChange::Changed
                    && output_schema_id(result) == Some(CONTROLLED_CHANGE_OUTPUT_SCHEMA_ID)
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if input.tool_results.iter().any(|result| {
            (result.state_change == StateChange::Changed)
                != (output_schema_id(result) == Some(CONTROLLED_CHANGE_OUTPUT_SCHEMA_ID))
        }) {
            return None;
        }
        let changed = !write_indexes.is_empty();
        if candidate.terminal_claim
            != if changed {
                CodingTerminalClaim::Changed
            } else {
                CodingTerminalClaim::NoOp
            }
        {
            return None;
        }

        let last_write = write_indexes.last().copied();
        let validations = input
            .tool_results
            .iter()
            .enumerate()
            .filter_map(|(index, result)| {
                self.full_validation(result)
                    .map(|validation_id| (index, validation_id))
            })
            .collect::<Vec<_>>();
        let git_results = input
            .tool_results
            .iter()
            .enumerate()
            .filter_map(|(index, result)| self.git_result(result).map(|git| (index, git)))
            .collect::<Vec<_>>();

        let validation_required = input
            .request
            .work_packet()
            .required_evidence
            .contains(&EvidenceKind::Validation);
        let completed_validation_ids = validations
            .iter()
            .map(|(_, validation_id)| validation_id.as_str())
            .collect::<BTreeSet<_>>();
        if validation_required && validations.is_empty() {
            return None;
        }
        if self
            .unconditional_validation_ids
            .iter()
            .any(|required| !completed_validation_ids.contains(required.as_str()))
        {
            return None;
        }
        if let Some(last_write) = last_write {
            let post_write_validation_ids = validations
                .iter()
                .filter(|(index, _)| *index > last_write)
                .map(|(_, validation_id)| validation_id.as_str())
                .collect::<BTreeSet<_>>();
            if self
                .changed_validation_ids
                .iter()
                .chain(&self.unconditional_validation_ids)
                .any(|required| !post_write_validation_ids.contains(required.as_str()))
            {
                return None;
            }
            if !git_results.iter().any(|(index, _)| *index > last_write) {
                return None;
            }
            if !input
                .evidence
                .iter()
                .any(|evidence| evidence.kind == EvidenceKind::Receipt)
            {
                return None;
            }
            Some(VerifierDisposition::Success)
        } else if git_results
            .iter()
            .any(|(_, result)| clean_git_state(result))
        {
            Some(VerifierDisposition::NoOp)
        } else {
            None
        }
    }

    fn full_validation(&self, result: &agentmage_kernel_contracts::ToolResult) -> Option<String> {
        let payload = result.output.as_ref()?;
        if payload.schema.schema_id.as_str() != TARGETED_VALIDATION_OUTPUT_SCHEMA_ID
            || payload.sha256 != sha256(&payload.bytes)
        {
            return None;
        }
        let receipt = serde_json::from_slice::<ValidationReceipt>(&payload.bytes).ok()?;
        if !receipt.is_full_pass() {
            return None;
        }
        if receipt.execution_authority
            || receipt.execution_scope_sha256 != self.workspace_snapshot_sha256
            || !self.validation_templates.contains(&(
                receipt.validation_id.clone(),
                receipt.validation_template_sha256.clone(),
            ))
        {
            return None;
        }
        Some(receipt.validation_id)
    }

    fn git_result(
        &self,
        result: &agentmage_kernel_contracts::ToolResult,
    ) -> Option<GitInspectionResult> {
        let payload = result.output.as_ref()?;
        if payload.schema.schema_id.as_str() != GIT_INSPECTION_OUTPUT_SCHEMA_ID
            || payload.sha256 != sha256(&payload.bytes)
        {
            return None;
        }
        serde_json::from_slice::<GitInspectionResult>(&payload.bytes)
            .ok()
            .filter(|git| {
                git.verify()
                    && git.repository_sha256 == self.repository_snapshot_sha256
                    && git.worktree_sha256 == self.workspace_snapshot_sha256
                    && !git.truncated
                    && !matches!(
                        git.outcome,
                        GitInspectionOutcome::Failed | GitInspectionOutcome::Truncated
                    )
            })
    }
}

impl RuntimeVerifierPort for CodingCompletionVerifier {
    fn verifier_id(&self) -> &VerifierId {
        &self.verifier_id
    }

    fn verify(
        &mut self,
        input: RuntimeVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        self.verify_scoped(input.into_scoped()?)
    }

    fn verify_scoped(
        &mut self,
        input: RuntimeScopedVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        self.verify_claim(input)
    }

    fn verify_completion(
        &mut self,
        input: RuntimeCompletionVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        if !input.proposal.matches_scope(input.request) {
            return Err(RuntimePortFailure::Invalid);
        }
        self.verify_claim(input)
    }
}

impl CodingCompletionVerifier {
    fn verify_claim<P: RuntimeCompletionProposal>(
        &self,
        input: RuntimeVerificationInput<'_, RuntimeRunScope<'_>, P>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        let disposition = self.evaluate(&input).unwrap_or(VerifierDisposition::Failed);
        let selected_evidence = select_evidence(
            input.evidence,
            &input.request.work_packet().required_evidence,
        );
        let passed = disposition != VerifierDisposition::Failed && !selected_evidence.is_empty();
        Ok(VerifierCandidate {
            schema_version: CONTRACT_SCHEMA_VERSION,
            verifier_record_id: VerifierRecordId::from_raw(format!(
                "coding-verification:{}",
                &sha256(input.proposal.proposal_sha256().as_bytes())[..24]
            )),
            verifier_id: self.verifier_id.clone(),
            task_id: input.request.task().task_id.clone(),
            proposal_id: input.proposal.proposal_id().clone(),
            repository_snapshot_id: input.request.repository_snapshot_id().clone(),
            state_revision: input.state_revision,
            source: VerifierSource::DeterministicPostcondition,
            disposition,
            postconditions: input
                .postconditions
                .iter()
                .map(|postcondition_id| PostconditionResult {
                    postcondition_id: postcondition_id.clone(),
                    passed,
                    evidence: selected_evidence.clone(),
                })
                .collect(),
        })
    }
}

pub(crate) fn parse_candidate(
    proposal: &impl RuntimeCompletionProposal,
    objective: &str,
) -> Option<CodingCompletionCandidate> {
    if proposal.kind() != ModelProposalKind::CompletionCandidate || proposal.has_tool_call() {
        return None;
    }
    let payload = proposal.payload()?;
    if payload.schema.schema_id.as_str() != CODING_COMPLETION_INPUT_SCHEMA_ID
        || payload.schema.schema_version != 1
        || payload.schema.schema_sha256 != sha256(CODING_COMPLETION_INPUT_SCHEMA_JSON.as_bytes())
        || payload.media_type != "application/json"
        || payload.sha256 != sha256(&payload.bytes)
    {
        return None;
    }
    serde_json::from_slice::<CodingCompletionCandidate>(&payload.bytes)
        .ok()
        .filter(|candidate| {
            candidate.schema_version == 1
                && candidate.objective_sha256 == sha256(objective.as_bytes())
                && !candidate.summary.is_empty()
                && candidate.summary.len() <= 16_384
                && bounded_texts(&candidate.checks_not_run)
                && bounded_texts(&candidate.residual_risks)
        })
}

fn required_evidence_present<P>(
    input: &RuntimeVerificationInput<'_, RuntimeRunScope<'_>, P>,
) -> bool {
    input
        .request
        .work_packet()
        .required_evidence
        .iter()
        .all(|kind| input.evidence.iter().any(|evidence| &evidence.kind == kind))
        && input.evidence.iter().all(|evidence| {
            evidence.observed_revision.as_deref()
                == Some(input.request.repository_snapshot_id().as_str())
        })
}

fn select_evidence(
    evidence: &[EvidenceReference],
    required: &[EvidenceKind],
) -> Vec<EvidenceReference> {
    let mut ordered = evidence.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
        let left_required = required.contains(&left.kind);
        let right_required = required.contains(&right.kind);
        right_required
            .cmp(&left_required)
            .then_with(|| left.evidence_id.as_str().cmp(right.evidence_id.as_str()))
    });
    ordered.into_iter().take(32).cloned().collect()
}

fn output_schema_id(result: &agentmage_kernel_contracts::ToolResult) -> Option<&str> {
    result
        .output
        .as_ref()
        .map(|payload| payload.schema.schema_id.as_str())
}

fn clean_git_state(result: &GitInspectionResult) -> bool {
    match result.operation {
        GitInspectionOperation::DirtyTree => result.dirty == Some(false),
        GitInspectionOperation::Status => result
            .records
            .iter()
            .all(|record| record.record_kind == "branch_header"),
        GitInspectionOperation::Diff | GitInspectionOperation::StagedDiff => {
            result.records.is_empty()
        }
        _ => false,
    }
}

fn bounded_texts(values: &[String]) -> bool {
    values.len() <= 128
        && values.iter().all(|value| {
            !value.is_empty() && value.len() <= 512 && !value.chars().any(char::is_control)
        })
}

/// Returns the exact completion payload schema reference shown to model codecs.
#[must_use]
pub fn coding_completion_schema() -> agentmage_kernel_contracts::SchemaReference {
    agentmage_kernel_contracts::SchemaReference {
        schema_id: agentmage_kernel_contracts::SchemaId::from_raw(
            CODING_COMPLETION_INPUT_SCHEMA_ID,
        ),
        schema_version: 1,
        schema_sha256: sha256(CODING_COMPLETION_INPUT_SCHEMA_JSON.as_bytes()),
    }
}

/// Seals one typed completion payload for deterministic fixtures and trusted clients.
pub fn coding_completion_payload(
    candidate: &CodingCompletionCandidate,
) -> Result<ContractPayload, RuntimePortFailure> {
    let bytes = serde_json::to_vec(candidate).map_err(|_| RuntimePortFailure::Invalid)?;
    Ok(ContractPayload {
        schema: coding_completion_schema(),
        media_type: "application/json".to_owned(),
        sha256: sha256(&bytes),
        bytes,
    })
}

fn sha256(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use agentmage_capability_read_only::{GitInspectionRequest, parse_git_inspection};
    use agentmage_kernel_contracts::ClosedModelProposal;
    use agentmage_kernel_contracts::{
        CorrelationId, EvidenceId, ModelCodecId, ModelProfileId, ModelRunId, PostconditionId,
        ProposalId, ReceiptId, SchemaId, SchemaReference, ToolCallId, ToolResult,
    };

    use super::*;
    use crate::coding_run::tests::fixture_profile_and_request;

    #[test]
    fn story_48_2_changed_completion_requires_later_validation_git_and_receipts() {
        let (profile, request) = fixture_profile_and_request();
        let mut verifier = CodingCompletionVerifier::for_profile(&profile);
        let proposal = proposal(&request, CodingTerminalClaim::Changed);
        let results = vec![
            write_result(&request, "write-call"),
            validation_result(&profile, &request, "validation-call"),
            git_result(&request, "git-call", false),
        ];
        let evidence = vec![
            evidence(&request, "write-evidence", EvidenceKind::Receipt),
            evidence(&request, "validation-evidence", EvidenceKind::Validation),
            evidence(&request, "git-evidence", EvidenceKind::Observation),
        ];
        let postconditions = vec![PostconditionId::from_raw("postcondition-fixture")];
        let result_receipts = receipts(results.len());
        let candidate = verifier
            .verify(RuntimeVerificationInput {
                request: &request,
                proposal: &proposal,
                state_revision: 8,
                postconditions: &postconditions,
                evidence: &evidence,
                tool_results: &results,
                receipt_ids: &result_receipts,
            })
            .expect("deterministic candidate");
        assert_eq!(candidate.disposition, VerifierDisposition::Success);
        assert!(candidate.postconditions[0].passed);

        let missing_validation = vec![results[0].clone(), results[2].clone()];
        let missing_receipts = receipts(missing_validation.len());
        let failed = verifier
            .verify(RuntimeVerificationInput {
                request: &request,
                proposal: &proposal,
                state_revision: 8,
                postconditions: &postconditions,
                evidence: &evidence,
                tool_results: &missing_validation,
                receipt_ids: &missing_receipts,
            })
            .expect("failed candidate remains typed");
        assert_eq!(failed.disposition, VerifierDisposition::Failed);
        assert!(!failed.postconditions[0].passed);
    }

    #[test]
    fn story_48_2_no_op_requires_clean_git_and_rejects_model_state_substitution() {
        let (profile, request) = fixture_profile_and_request();
        let mut verifier = CodingCompletionVerifier::for_profile(&profile);
        let no_op = proposal(&request, CodingTerminalClaim::NoOp);
        let results = vec![
            validation_result(&profile, &request, "validation-call"),
            git_result(&request, "git-call", false),
        ];
        let evidence = vec![
            evidence(&request, "validation-evidence", EvidenceKind::Validation),
            evidence(&request, "git-evidence", EvidenceKind::Observation),
        ];
        let postconditions = vec![PostconditionId::from_raw("postcondition-fixture")];
        let result_receipts = receipts(results.len());
        let candidate = verifier
            .verify(RuntimeVerificationInput {
                request: &request,
                proposal: &no_op,
                state_revision: 8,
                postconditions: &postconditions,
                evidence: &evidence,
                tool_results: &results,
                receipt_ids: &result_receipts,
            })
            .expect("no-op candidate");
        assert_eq!(candidate.disposition, VerifierDisposition::NoOp);

        let false_changed_claim = proposal(&request, CodingTerminalClaim::Changed);
        let candidate = verifier
            .verify(RuntimeVerificationInput {
                request: &request,
                proposal: &false_changed_claim,
                state_revision: 8,
                postconditions: &postconditions,
                evidence: &evidence,
                tool_results: &results,
                receipt_ids: &result_receipts,
            })
            .expect("contradictory claim is typed failure");
        assert_eq!(candidate.disposition, VerifierDisposition::Failed);
    }

    #[test]
    fn story_48_2_verifier_rejects_stale_evidence_and_git_before_write() {
        let (profile, request) = fixture_profile_and_request();
        let mut verifier = CodingCompletionVerifier::for_profile(&profile);
        let proposal = proposal(&request, CodingTerminalClaim::Changed);
        let results = vec![
            git_result(&request, "git-call", false),
            write_result(&request, "write-call"),
            validation_result(&profile, &request, "validation-call"),
        ];
        let mut evidence = vec![
            evidence(&request, "write-evidence", EvidenceKind::Receipt),
            evidence(&request, "validation-evidence", EvidenceKind::Validation),
            evidence(&request, "git-evidence", EvidenceKind::Observation),
        ];
        evidence[0].observed_revision = Some("stale-snapshot".to_owned());
        let postconditions = vec![PostconditionId::from_raw("postcondition-fixture")];
        let result_receipts = receipts(results.len());
        let candidate = verifier
            .verify(RuntimeVerificationInput {
                request: &request,
                proposal: &proposal,
                state_revision: 8,
                postconditions: &postconditions,
                evidence: &evidence,
                tool_results: &results,
                receipt_ids: &result_receipts,
            })
            .expect("failed candidate");
        assert_eq!(candidate.disposition, VerifierDisposition::Failed);
    }

    fn endpoint_request(
        native: &agentmage_kernel_contracts::RuntimeRunRequest,
    ) -> agentmage_kernel_contracts::RuntimeEndpointRunRequest {
        use agentmage_kernel_contracts::{
            EndpointModelSelection, RuntimeEndpointContextBudget, RuntimeEndpointRunRequest,
        };
        agentmage_kernel_engine::runtime_coordinator::seal_runtime_endpoint_run_request(
            RuntimeEndpointRunRequest {
                schema_version: native.schema_version,
                run_id: native.run_id.clone(),
                session_id: native.session_id.clone(),
                mode: native.mode,
                task: native.task.clone(),
                work_packet: native.work_packet.clone(),
                workspace_id: native.workspace_id.clone(),
                workspace_snapshot_sha256: native.workspace_snapshot_sha256.clone(),
                repository_snapshot_id: native.repository_snapshot_id.clone(),
                repository_snapshot_sha256: native.repository_snapshot_sha256.clone(),
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
                max_output_tokens: 16,
                tool_catalog_id: native.tool_catalog_id.clone(),
                tool_catalog_sha256: native.tool_catalog_sha256.clone(),
                visible_tools: native.visible_tools.clone(),
                policy_id: native.policy_id.clone(),
                policy_sha256: native.policy_sha256.clone(),
                limits: native.limits.clone(),
                event_cursor: None,
                request_sha256: "0".repeat(64),
            },
        )
        .unwrap()
    }

    #[test]
    fn scoped_coding_verifier_preserves_native_and_endpoint_deterministic_results() {
        use agentmage_kernel_engine::runtime_loop::RuntimeRunScope;
        let (profile, native) = fixture_profile_and_request();
        let endpoint = endpoint_request(&native);
        let mut verifier = CodingCompletionVerifier::for_profile(&profile);
        for claim in [CodingTerminalClaim::Changed, CodingTerminalClaim::NoOp] {
            let proposal = proposal(&native, claim);
            let mut results = Vec::new();
            let mut evidence = Vec::new();
            if claim == CodingTerminalClaim::Changed {
                results.push(write_result(&native, "write-call"));
                evidence.push(self::evidence(
                    &native,
                    "write-evidence",
                    EvidenceKind::Receipt,
                ));
            }
            results.extend([
                validation_result(&profile, &native, "validation-call"),
                git_result(&native, "git-call", false),
            ]);
            evidence.extend([
                self::evidence(&native, "validation-evidence", EvidenceKind::Validation),
                self::evidence(&native, "git-evidence", EvidenceKind::Observation),
            ]);
            let postconditions = [PostconditionId::from_raw("postcondition-fixture")];
            let receipts = receipts(results.len());
            let expected = verifier
                .verify(RuntimeVerificationInput {
                    request: &native,
                    proposal: &proposal,
                    state_revision: 8,
                    postconditions: &postconditions,
                    evidence: &evidence,
                    tool_results: &results,
                    receipt_ids: &receipts,
                })
                .unwrap();
            assert_eq!(
                expected.disposition,
                if claim == CodingTerminalClaim::Changed {
                    VerifierDisposition::Success
                } else {
                    VerifierDisposition::NoOp
                }
            );
            let actual = verifier
                .verify_scoped(RuntimeVerificationInput {
                    request: RuntimeRunScope::endpoint(&endpoint).unwrap(),
                    proposal: &proposal,
                    state_revision: 8,
                    postconditions: &postconditions,
                    evidence: &evidence,
                    tool_results: &results,
                    receipt_ids: &receipts,
                })
                .unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn scoped_coding_verifier_rejects_missing_receipts_stale_evidence_and_wrong_snapshot() {
        use agentmage_kernel_engine::runtime_loop::RuntimeRunScope;
        let (profile, native) = fixture_profile_and_request();
        let mut verifier = CodingCompletionVerifier::for_profile(&profile);
        let proposal = proposal(&native, CodingTerminalClaim::Changed);
        let original_results = vec![
            write_result(&native, "write-call"),
            validation_result(&profile, &native, "validation-call"),
            git_result(&native, "git-call", false),
        ];
        let original_evidence = vec![
            evidence(&native, "validation-evidence", EvidenceKind::Validation),
            evidence(&native, "git-evidence", EvidenceKind::Observation),
        ];
        for mutation in 0..5 {
            let mut request = endpoint_request(&native);
            let mut results = original_results.clone();
            let mut evidence = original_evidence.clone();
            let mut receipts = receipts(results.len());
            match mutation {
                0 => {
                    receipts.pop();
                }
                1 => {
                    evidence[0].observed_revision = Some("stale-snapshot".into());
                }
                2 => {
                    request.workspace_snapshot_sha256 = "d".repeat(64);
                }
                3 => {
                    request.repository_snapshot_sha256 = "d".repeat(64);
                }
                4 => {
                    results.remove(1);
                    receipts.pop();
                }
                _ => unreachable!(),
            }
            request =
                agentmage_kernel_engine::runtime_coordinator::seal_runtime_endpoint_run_request(
                    request,
                )
                .unwrap();
            let postconditions = [PostconditionId::from_raw("postcondition-fixture")];
            let candidate = verifier
                .verify_scoped(RuntimeVerificationInput {
                    request: RuntimeRunScope::endpoint(&request).unwrap(),
                    proposal: &proposal,
                    state_revision: 8,
                    postconditions: &postconditions,
                    evidence: &evidence,
                    tool_results: &results,
                    receipt_ids: &receipts,
                })
                .unwrap();
            assert_eq!(
                candidate.disposition,
                VerifierDisposition::Failed,
                "mutation {mutation}"
            );
            assert!(!candidate.postconditions[0].passed);
        }
    }

    struct CompletionClock;
    impl agentmage_kernel_engine::runtime_loop::RuntimeClock for CompletionClock {
        fn now_epoch_ms(&mut self) -> Result<u64, RuntimePortFailure> {
            Ok(1000)
        }
    }

    struct CompletionPort<'a> {
        request: agentmage_kernel_contracts::EndpointModelRequest,
        text: &'a str,
        finish: agentmage_kernel_contracts::EndpointModelFinish,
    }
    impl agentmage_kernel_engine::runtime_loop::endpoint::RuntimeEndpointModelPort
        for CompletionPort<'_>
    {
        type Error = RuntimePortFailure;
        fn request(&self) -> &agentmage_kernel_contracts::EndpointModelRequest {
            &self.request
        }
        fn dispatch(
            self,
            control: &dyn agentmage_kernel_contracts::ModelOperationControl,
            text: &mut dyn FnMut(&str),
        ) -> Result<agentmage_kernel_contracts::EndpointModelResult, Self::Error> {
            control
                .remaining_ms()
                .map_err(|_| RuntimePortFailure::Invalid)?;
            text(self.text);
            Ok(
                agentmage_kernel_contracts::EndpointModelResult::from_completed_response(
                    self.request,
                    self.finish,
                    sha256(self.text.as_bytes()),
                    self.text.len(),
                    std::time::Duration::from_millis(1),
                    None,
                ),
            )
        }
    }

    fn completion_turn(
        native: &agentmage_kernel_contracts::RuntimeRunRequest,
        text: &str,
    ) -> (
        agentmage_kernel_contracts::RuntimeEndpointRunRequest,
        agentmage_kernel_engine::runtime_loop::endpoint::RuntimeEndpointTurnResult,
    ) {
        completion_turn_with_finish(
            native,
            text,
            agentmage_kernel_contracts::EndpointModelFinish::Stop,
        )
    }

    fn completion_turn_with_finish(
        native: &agentmage_kernel_contracts::RuntimeRunRequest,
        text: &str,
        finish: agentmage_kernel_contracts::EndpointModelFinish,
    ) -> (
        agentmage_kernel_contracts::RuntimeEndpointRunRequest,
        agentmage_kernel_engine::runtime_loop::endpoint::RuntimeEndpointTurnResult,
    ) {
        use agentmage_kernel_contracts::{
            EndpointModelBody, EndpointModelLimits, EndpointModelRequest, EndpointModelSelection,
            RuntimeSessionMode,
        };
        use agentmage_kernel_engine::runtime_loop::endpoint::{
            RuntimeEndpointContextPacket, RuntimeEndpointMessage, RuntimeEndpointMessageRole,
            RuntimeEndpointTurns,
        };
        use std::time::Duration;
        let mut run = endpoint_request(native);
        run.mode = RuntimeSessionMode::EphemeralReadOnly;
        run.limits.max_output_bytes = run.limits.max_output_bytes.max(4096);
        run = agentmage_kernel_engine::runtime_coordinator::seal_runtime_endpoint_run_request(run)
            .unwrap();
        let context = RuntimeEndpointContextPacket::from_messages(&[RuntimeEndpointMessage {
            role: RuntimeEndpointMessageRole::User,
            content: "synthetic completion request",
        }])
        .unwrap();
        let request = EndpointModelRequest::from_prepared_body(
            EndpointModelSelection {
                endpoint: run.model_selection.endpoint.clone(),
                model: run.model_selection.model.clone(),
            },
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
        let mut turns = RuntimeEndpointTurns::new(run.clone(), CompletionClock).unwrap();
        let turn = turns
            .execute_turn(
                &request,
                context,
                CompletionPort {
                    request: request.clone(),
                    text,
                    finish,
                },
                None,
                |_| {},
            )
            .unwrap();
        assert_eq!(turn.result().response_text_sha256, sha256(text.as_bytes()));
        (run, turn)
    }

    #[test]
    fn completion_claim_endpoint_verifies_accepted_stream_without_native_profile_or_codec() {
        use agentmage_kernel_engine::runtime_loop::{
            RuntimeCompletionClaim, RuntimeCompletionProposal,
        };
        let (profile, native) = fixture_profile_and_request();
        let mut verifier = CodingCompletionVerifier::for_profile(&profile);
        for terminal_claim in [CodingTerminalClaim::Changed, CodingTerminalClaim::NoOp] {
            let native_proposal = proposal(&native, terminal_claim);
            let payload = native_proposal.payload.as_ref().unwrap();
            let text = std::str::from_utf8(&payload.bytes).unwrap();
            let (run, turn) = completion_turn(&native, text);
            let scope = RuntimeRunScope::endpoint(&run).unwrap();
            let mut results = Vec::new();
            let mut evidence = Vec::new();
            if terminal_claim == CodingTerminalClaim::Changed {
                results.push(write_result(&native, "write-call"));
                evidence.push(self::evidence(
                    &native,
                    "write-evidence",
                    EvidenceKind::Receipt,
                ));
            }
            results.extend([
                validation_result(&profile, &native, "validation-call"),
                git_result(&native, "git-call", false),
            ]);
            evidence.extend([
                self::evidence(&native, "validation-evidence", EvidenceKind::Validation),
                self::evidence(&native, "git-evidence", EvidenceKind::Observation),
            ]);
            let postconditions = [PostconditionId::from_raw("completion-postcondition")];
            let receipts = receipts(results.len());
            let claim =
                RuntimeCompletionClaim::endpoint(scope, &turn, text, coding_completion_schema())
                    .unwrap();
            assert!(claim.native_proposal().is_none());
            assert!(claim.matches_scope(scope));
            assert_eq!(claim.payload().unwrap(), payload);
            let proposal_id = claim.proposal_id().clone();
            let candidate = verifier
                .verify_completion(RuntimeVerificationInput {
                    request: scope,
                    proposal: claim,
                    state_revision: 8,
                    postconditions: &postconditions,
                    evidence: &evidence,
                    tool_results: &results,
                    receipt_ids: &receipts,
                })
                .unwrap();
            assert_eq!(candidate.proposal_id, proposal_id);
            assert_eq!(
                candidate.disposition,
                if terminal_claim == CodingTerminalClaim::Changed {
                    VerifierDisposition::Success
                } else {
                    VerifierDisposition::NoOp
                }
            );
            assert!(candidate.postconditions[0].passed);
            assert_eq!(candidate.postconditions[0].evidence.len(), evidence.len());
            // The candidate remains inert until the coordinator's registry verifies it.
            let registry = agentmage_kernel_engine::agent_verifier::VerifierRegistry::new(
                agentmage_kernel_engine::agent_verifier::VerifierContext {
                    verifier_id: verifier.verifier_id().clone(),
                    task_id: native.task.task_id.clone(),
                    proposal_id,
                    repository_snapshot_id: native.repository_snapshot_id.clone(),
                    state_revision: 8,
                    postconditions: postconditions.to_vec(),
                },
            )
            .unwrap();
            registry.verify(&candidate).unwrap();
        }
    }

    #[test]
    fn completion_claim_endpoint_refuses_stream_run_schema_and_family_substitution() {
        use agentmage_kernel_engine::runtime_loop::{
            RuntimeCompletionClaim, RuntimeCompletionProposal,
        };
        let (profile, native) = fixture_profile_and_request();
        let payload = proposal(&native, CodingTerminalClaim::NoOp)
            .payload
            .unwrap();
        let text = std::str::from_utf8(&payload.bytes).unwrap();
        let (run, turn) = completion_turn(&native, text);
        let scope = RuntimeRunScope::endpoint(&run).unwrap();
        assert!(
            RuntimeCompletionClaim::endpoint(
                scope,
                &turn,
                &format!("{text} "),
                coding_completion_schema()
            )
            .is_err()
        );
        let changed_text = text.replacen("bounded", "altered", 1);
        assert_eq!(changed_text.len(), text.len());
        assert!(
            RuntimeCompletionClaim::endpoint(
                scope,
                &turn,
                &changed_text,
                coding_completion_schema()
            )
            .is_err()
        );
        assert!(
            RuntimeCompletionClaim::endpoint(
                RuntimeRunScope::native(&native).unwrap(),
                &turn,
                text,
                coding_completion_schema()
            )
            .is_err()
        );
        for mutation in 0..4 {
            let mut changed = run.clone();
            match mutation {
                0 => {
                    changed.run_id = agentmage_kernel_contracts::RuntimeRunId::from_raw("other-run")
                }
                1 => changed.model_selection.model = "other-model".into(),
                2 => changed.policy_sha256 = "d".repeat(64),
                3 => changed.task.objective.push_str(" changed"),
                _ => unreachable!(),
            }
            changed =
                agentmage_kernel_engine::runtime_coordinator::seal_runtime_endpoint_run_request(
                    changed,
                )
                .unwrap();
            let wrong = RuntimeRunScope::endpoint(&changed).unwrap();
            assert!(
                RuntimeCompletionClaim::endpoint(wrong, &turn, text, coding_completion_schema())
                    .is_err()
            );
            let claim =
                RuntimeCompletionClaim::endpoint(scope, &turn, text, coding_completion_schema())
                    .unwrap();
            assert!(!claim.matches_scope(wrong));
            let mut verifier = CodingCompletionVerifier::for_profile(&profile);
            assert_eq!(
                verifier.verify_completion(RuntimeVerificationInput {
                    request: wrong,
                    proposal: claim,
                    state_revision: 8,
                    postconditions: &[],
                    evidence: &[],
                    tool_results: &[],
                    receipt_ids: &[],
                }),
                Err(RuntimePortFailure::Invalid)
            );
        }
        let claim =
            RuntimeCompletionClaim::endpoint(scope, &turn, text, coding_completion_schema())
                .unwrap();
        let again =
            RuntimeCompletionClaim::endpoint(scope, &turn, text, coding_completion_schema())
                .unwrap();
        assert_eq!(claim.proposal_sha256(), again.proposal_sha256());
        assert_eq!(claim.proposal_id(), again.proposal_id());
        let mut schema = coding_completion_schema();
        schema.schema_version = 0;
        assert!(RuntimeCompletionClaim::endpoint(scope, &turn, text, schema).is_err());
        let mut schema = coding_completion_schema();
        schema.schema_sha256 = "bad".into();
        assert!(RuntimeCompletionClaim::endpoint(scope, &turn, text, schema).is_err());
        // Schema choice is trusted host input and part of the complete claim digest.
        let mut schema = coding_completion_schema();
        schema.schema_version += 1;
        let alternate = RuntimeCompletionClaim::endpoint(scope, &turn, text, schema).unwrap();
        assert_ne!(claim.proposal_sha256(), alternate.proposal_sha256());
        assert_ne!(claim.proposal_id(), alternate.proposal_id());
    }

    #[test]
    fn endpoint_completion_refuses_token_limit_even_with_valid_complete_claim_json() {
        use agentmage_kernel_contracts::EndpointModelFinish;
        use agentmage_kernel_engine::runtime_loop::RuntimeCompletionClaim;
        let (_, native) = fixture_profile_and_request();
        for terminal_claim in [CodingTerminalClaim::Changed, CodingTerminalClaim::NoOp] {
            let payload = proposal(&native, terminal_claim).payload.unwrap();
            let text = std::str::from_utf8(&payload.bytes).unwrap();
            for finish in [
                EndpointModelFinish::Stop,
                EndpointModelFinish::OutputTokenLimit,
            ] {
                let (run, turn) = completion_turn_with_finish(&native, text, finish);
                // Both finishes are valid transport observations with the same
                // exact text. Only an ordinary stop can become a completion claim.
                assert_eq!(turn.result().finish, finish);
                let claim = RuntimeCompletionClaim::endpoint(
                    RuntimeRunScope::endpoint(&run).unwrap(),
                    &turn,
                    text,
                    coding_completion_schema(),
                );
                match finish {
                    EndpointModelFinish::Stop => assert!(claim.is_ok()),
                    EndpointModelFinish::OutputTokenLimit => {
                        assert!(matches!(claim, Err(RuntimePortFailure::Invalid)));
                    }
                }
            }
        }
    }

    #[test]
    fn endpoint_completion_registry_refuses_candidates_from_other_runs_and_native_family() {
        use agentmage_kernel_engine::{
            agent_verifier::{VerifierContext, VerifierError, VerifierRegistry},
            runtime_loop::{RuntimeCompletionClaim, RuntimeCompletionProposal},
        };
        let (profile, native) = fixture_profile_and_request();
        let native_proposal = proposal(&native, CodingTerminalClaim::NoOp);
        let payload = native_proposal.payload.as_ref().unwrap();
        let text = std::str::from_utf8(&payload.bytes).unwrap();
        let (run, turn) = completion_turn(&native, text);
        let mut other = native.clone();
        other.run_id = agentmage_kernel_contracts::RuntimeRunId::from_raw("other-completion-run");
        let (other_run, other_turn) = completion_turn(&other, text);
        assert_ne!(run.request_sha256, other_run.request_sha256);
        let results = [
            validation_result(&profile, &native, "validation"),
            git_result(&native, "git", false),
        ];
        let evidence = [
            self::evidence(&native, "validation-evidence", EvidenceKind::Validation),
            self::evidence(&native, "git-evidence", EvidenceKind::Observation),
        ];
        let receipts = receipts(results.len());
        let postconditions = [PostconditionId::from_raw("completion-postcondition")];
        let mut verifier = CodingCompletionVerifier::for_profile(&profile);
        let native_candidate = verifier
            .verify(RuntimeVerificationInput {
                request: &native,
                proposal: &native_proposal,
                state_revision: 8,
                postconditions: &postconditions,
                evidence: &evidence,
                tool_results: &results,
                receipt_ids: &receipts,
            })
            .unwrap();
        let mut candidates = vec![native_candidate];
        for (request, turn) in [(&run, &turn), (&other_run, &other_turn)] {
            let scope = RuntimeRunScope::endpoint(request).unwrap();
            let claim =
                RuntimeCompletionClaim::endpoint(scope, turn, text, coding_completion_schema())
                    .unwrap();
            let proposal_id = claim.proposal_id().clone();
            let candidate = verifier
                .verify_completion(RuntimeVerificationInput {
                    request: scope,
                    proposal: claim,
                    state_revision: 8,
                    postconditions: &postconditions,
                    evidence: &evidence,
                    tool_results: &results,
                    receipt_ids: &receipts,
                })
                .unwrap();
            assert_eq!(candidate.proposal_id, proposal_id);
            candidates.push(candidate);
        }
        for (owner, candidate) in candidates.iter().enumerate() {
            assert_eq!(candidate.disposition, VerifierDisposition::NoOp);
            assert!(candidate.postconditions[0].passed);
            let registry = VerifierRegistry::new(VerifierContext {
                verifier_id: verifier.verifier_id().clone(),
                task_id: native.task.task_id.clone(),
                proposal_id: candidate.proposal_id.clone(),
                repository_snapshot_id: native.repository_snapshot_id.clone(),
                state_revision: 8,
                postconditions: postconditions.to_vec(),
            })
            .unwrap();
            // Passing controls prevent refusal due to missing test evidence
            // from concealing a candidate-substitution admission regression.
            registry.verify(candidate).unwrap();
            for (foreign, candidate) in candidates.iter().enumerate() {
                if owner != foreign {
                    assert_eq!(
                        registry.verify(candidate),
                        Err(VerifierError::ContextMismatch)
                    );
                }
            }
        }
    }

    struct LegacyCompletionVerifier(VerifierId);
    impl RuntimeVerifierPort for LegacyCompletionVerifier {
        fn verifier_id(&self) -> &VerifierId {
            &self.0
        }
        fn verify(
            &mut self,
            _: RuntimeVerificationInput<'_>,
        ) -> Result<VerifierCandidate, RuntimePortFailure> {
            panic!("endpoint must refuse before legacy verifier entry")
        }
        fn verify_scoped(
            &mut self,
            _: RuntimeScopedVerificationInput<'_>,
        ) -> Result<VerifierCandidate, RuntimePortFailure> {
            panic!("endpoint must refuse before scoped legacy verifier entry")
        }
    }

    #[test]
    fn completion_claim_accepted_endpoint_refuses_legacy_verifier_before_entry() {
        use agentmage_kernel_engine::runtime_loop::RuntimeCompletionClaim;
        let (_, native) = fixture_profile_and_request();
        let payload = proposal(&native, CodingTerminalClaim::NoOp)
            .payload
            .unwrap();
        let text = std::str::from_utf8(&payload.bytes).unwrap();
        let (run, turn) = completion_turn(&native, text);
        let scope = RuntimeRunScope::endpoint(&run).unwrap();
        let mut verifier = LegacyCompletionVerifier(VerifierId::from_raw("legacy-verifier"));
        let claim =
            RuntimeCompletionClaim::endpoint(scope, &turn, text, coding_completion_schema())
                .unwrap();
        assert!(claim.native_proposal().is_none());
        assert_eq!(
            verifier.verify_completion(RuntimeVerificationInput {
                request: scope,
                proposal: claim,
                state_revision: 1,
                postconditions: &[],
                evidence: &[],
                tool_results: &[],
                receipt_ids: &[],
            }),
            Err(RuntimePortFailure::Invalid)
        );
    }

    #[test]
    fn completion_claim_endpoint_requires_current_receipts_validation_and_snapshot() {
        use agentmage_kernel_engine::runtime_loop::RuntimeCompletionClaim;
        let (profile, native) = fixture_profile_and_request();
        let payload = proposal(&native, CodingTerminalClaim::Changed)
            .payload
            .unwrap();
        let text = std::str::from_utf8(&payload.bytes).unwrap();
        let (run, turn) = completion_turn(&native, text);
        let scope = RuntimeRunScope::endpoint(&run).unwrap();
        let original_results = vec![
            write_result(&native, "write"),
            validation_result(&profile, &native, "validation"),
            git_result(&native, "git", false),
        ];
        let original_evidence = vec![
            evidence(&native, "write-evidence", EvidenceKind::Receipt),
            evidence(&native, "validation-evidence", EvidenceKind::Validation),
            evidence(&native, "git-evidence", EvidenceKind::Observation),
        ];
        let postconditions = [PostconditionId::from_raw("completion-postcondition")];
        let original_receipts = receipts(original_results.len());
        let mut verifier = CodingCompletionVerifier::for_profile(&profile);
        let control = verifier
            .verify_completion(RuntimeVerificationInput {
                request: scope,
                proposal: RuntimeCompletionClaim::endpoint(
                    scope,
                    &turn,
                    text,
                    coding_completion_schema(),
                )
                .unwrap(),
                state_revision: 8,
                postconditions: &postconditions,
                evidence: &original_evidence,
                tool_results: &original_results,
                receipt_ids: &original_receipts,
            })
            .unwrap();
        assert_eq!(control.disposition, VerifierDisposition::Success);
        assert!(control.postconditions[0].passed);
        for mutation in 0..5 {
            let mut verifier = CodingCompletionVerifier::for_profile(&profile);
            let mut results = original_results.clone();
            let mut evidence = original_evidence.clone();
            let mut receipts = receipts(results.len());
            match mutation {
                0 => {
                    receipts.pop();
                }
                1 => receipts[1] = receipts[0].clone(),
                2 => evidence[0].observed_revision = Some("stale-snapshot".into()),
                3 => {
                    results.remove(1);
                    receipts.pop();
                }
                4 => verifier.workspace_snapshot_sha256 = "d".repeat(64),
                _ => unreachable!(),
            }
            let candidate = verifier
                .verify_completion(RuntimeVerificationInput {
                    request: scope,
                    proposal: RuntimeCompletionClaim::endpoint(
                        scope,
                        &turn,
                        text,
                        coding_completion_schema(),
                    )
                    .unwrap(),
                    state_revision: 8,
                    postconditions: &postconditions,
                    evidence: &evidence,
                    tool_results: &results,
                    receipt_ids: &receipts,
                })
                .unwrap();
            assert_eq!(
                candidate.disposition,
                VerifierDisposition::Failed,
                "mutation {mutation}"
            );
            assert!(!candidate.postconditions[0].passed);
        }
    }

    fn proposal(
        request: &agentmage_kernel_contracts::RuntimeRunRequest,
        terminal_claim: CodingTerminalClaim,
    ) -> ClosedModelProposal {
        let payload = coding_completion_payload(&CodingCompletionCandidate {
            schema_version: 1,
            objective_sha256: sha256(request.task.objective.as_bytes()),
            terminal_claim,
            summary: "The bounded fixture was evaluated against current evidence.".to_owned(),
            checks_not_run: Vec::new(),
            residual_risks: Vec::new(),
        })
        .expect("completion payload");
        ClosedModelProposal {
            schema_version: CONTRACT_SCHEMA_VERSION,
            proposal_id: ProposalId::from_raw("coding-proposal-0001"),
            model_run_id: ModelRunId::from_raw("coding-model-run-0001"),
            context_packet_id: agentmage_kernel_contracts::ContextPacketId::from_raw(
                "coding-context-0001",
            ),
            profile_id: ModelProfileId::from_raw(request.model_profile.profile_id.as_str()),
            codec_id: ModelCodecId::from_raw(request.model_profile.codec.codec_id.as_str()),
            correlation_id: CorrelationId::from_raw("coding-correlation-0001"),
            kind: ModelProposalKind::CompletionCandidate,
            payload: Some(payload),
            tool_call: None,
            proposal_sha256: "a".repeat(64),
        }
    }

    fn write_result(
        request: &agentmage_kernel_contracts::RuntimeRunRequest,
        call_id: &str,
    ) -> ToolResult {
        result(
            request,
            call_id,
            CONTROLLED_CHANGE_OUTPUT_SCHEMA_ID,
            br#"{"outcome":"succeeded"}"#.to_vec(),
            StateChange::Changed,
            EvidenceKind::Receipt,
        )
    }

    fn validation_result(
        profile: &CodingSessionProfile,
        request: &agentmage_kernel_contracts::RuntimeRunRequest,
        call_id: &str,
    ) -> ToolResult {
        let mut receipt: ValidationReceipt = serde_json::from_str(include_str!(
            "../../../schemas/runtime/examples/validation-receipt.valid.json"
        ))
        .expect("validation fixture");
        receipt.validation_id = profile.validations().templates[0].validation_id.clone();
        receipt.validation_template_sha256 =
            profile.validations().templates[0].template_sha256.clone();
        receipt.execution_scope_sha256 = request.workspace_snapshot_sha256.clone();
        result(
            request,
            call_id,
            TARGETED_VALIDATION_OUTPUT_SCHEMA_ID,
            serde_json::to_vec(&receipt).expect("validation bytes"),
            StateChange::NotChanged,
            EvidenceKind::Validation,
        )
    }

    fn git_result(
        request: &agentmage_kernel_contracts::RuntimeRunRequest,
        call_id: &str,
        dirty: bool,
    ) -> ToolResult {
        let git_request = GitInspectionRequest {
            schema_version: 1,
            operation: GitInspectionOperation::DirtyTree,
            revision: None,
            object_id: None,
            pathspecs: Vec::new(),
            max_records: 16,
            max_output_bytes: 4_096,
        };
        let output = if dirty {
            b"? changed.txt\0".as_slice()
        } else {
            b"".as_slice()
        };
        let git = parse_git_inspection(
            &git_request,
            &request.repository_snapshot_sha256,
            &request.workspace_snapshot_sha256,
            &"c".repeat(64),
            true,
            output,
        )
        .expect("git result");
        result(
            request,
            call_id,
            GIT_INSPECTION_OUTPUT_SCHEMA_ID,
            serde_json::to_vec(&git).expect("git bytes"),
            StateChange::NotChanged,
            EvidenceKind::Observation,
        )
    }

    fn result(
        request: &agentmage_kernel_contracts::RuntimeRunRequest,
        call_id: &str,
        schema_id: &str,
        bytes: Vec<u8>,
        state_change: StateChange,
        evidence_kind: EvidenceKind,
    ) -> ToolResult {
        ToolResult {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw(call_id),
            correlation_id: CorrelationId::from_raw("coding-correlation-0001"),
            outcome: OperationOutcome::Succeeded,
            output: Some(ContractPayload {
                schema: SchemaReference {
                    schema_id: SchemaId::from_raw(schema_id),
                    schema_version: 1,
                    schema_sha256: "d".repeat(64),
                },
                media_type: "application/json".to_owned(),
                sha256: sha256(&bytes),
                bytes,
            }),
            validation_issues: Vec::new(),
            evidence: vec![evidence(
                request,
                &format!("{call_id}-evidence"),
                evidence_kind,
            )],
            error: None,
            elapsed_ms: 1,
            state_change,
        }
    }

    fn evidence(
        request: &agentmage_kernel_contracts::RuntimeRunRequest,
        id: &str,
        kind: EvidenceKind,
    ) -> EvidenceReference {
        EvidenceReference {
            schema_version: CONTRACT_SCHEMA_VERSION,
            evidence_id: EvidenceId::from_raw(id),
            kind,
            source_id: "native:fixture@1.0.0".to_owned(),
            object_id: id.to_owned(),
            fragment: None,
            content_sha256: "e".repeat(64),
            observed_revision: Some(request.repository_snapshot_id.as_str().to_owned()),
        }
    }

    fn receipts(count: usize) -> Vec<ReceiptId> {
        (0..count)
            .map(|index| ReceiptId::from_raw(format!("coding-receipt-{index}")))
            .collect()
    }
}
