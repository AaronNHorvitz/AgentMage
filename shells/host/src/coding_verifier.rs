//! Deterministic completion verification for bounded coding runs.

use std::collections::{BTreeSet, HashSet};
use std::fmt::Write;

use agentmage_capability_read_only::{
    GIT_INSPECTION_OUTPUT_SCHEMA_ID, GitInspectionOperation, GitInspectionOutcome,
    GitInspectionResult,
};
use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, ClosedModelProposal, ContractPayload, EvidenceKind, EvidenceReference,
    ModelProposalKind, OperationOutcome, PostconditionResult, StateChange, VerifierCandidate,
    VerifierDisposition, VerifierId, VerifierRecordId, VerifierSource,
};
use agentmage_kernel_engine::{
    runtime_loop::{RuntimePortFailure, RuntimeVerificationInput, RuntimeVerifierPort},
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
}

impl CodingCompletionVerifier {
    /// Binds one verifier to the profile's worktree and registered validations.
    #[must_use]
    pub fn for_profile(profile: &CodingSessionProfile) -> Self {
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
        }
    }

    fn evaluate(&self, input: &RuntimeVerificationInput<'_>) -> Option<VerifierDisposition> {
        let candidate = parse_candidate(input.proposal, &input.request.task.objective)?;
        if input.request.workspace_snapshot_sha256 != self.workspace_snapshot_sha256
            || input.request.repository_snapshot_sha256 != self.repository_snapshot_sha256
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
        let validation_indexes = input
            .tool_results
            .iter()
            .enumerate()
            .filter_map(|(index, result)| self.full_validation(result).then_some(index))
            .collect::<Vec<_>>();
        let git_results = input
            .tool_results
            .iter()
            .enumerate()
            .filter_map(|(index, result)| self.git_result(result).map(|git| (index, git)))
            .collect::<Vec<_>>();

        let validation_required = input
            .request
            .work_packet
            .required_evidence
            .contains(&EvidenceKind::Validation);
        if validation_required && validation_indexes.is_empty() {
            return None;
        }
        if let Some(last_write) = last_write {
            if !validation_indexes.iter().any(|index| *index > last_write)
                || !git_results.iter().any(|(index, _)| *index > last_write)
                || !input
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

    fn full_validation(&self, result: &agentmage_kernel_contracts::ToolResult) -> bool {
        let Some(payload) = result.output.as_ref() else {
            return false;
        };
        if payload.schema.schema_id.as_str() != TARGETED_VALIDATION_OUTPUT_SCHEMA_ID
            || payload.sha256 != sha256(&payload.bytes)
        {
            return false;
        }
        serde_json::from_slice::<ValidationReceipt>(&payload.bytes).is_ok_and(|receipt| {
            receipt.is_full_pass()
                && !receipt.execution_authority
                && receipt.execution_scope_sha256 == self.workspace_snapshot_sha256
                && self
                    .validation_templates
                    .contains(&(receipt.validation_id, receipt.validation_template_sha256))
        })
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
        let disposition = self.evaluate(&input).unwrap_or(VerifierDisposition::Failed);
        let selected_evidence =
            select_evidence(input.evidence, &input.request.work_packet.required_evidence);
        let passed = disposition != VerifierDisposition::Failed && !selected_evidence.is_empty();
        Ok(VerifierCandidate {
            schema_version: CONTRACT_SCHEMA_VERSION,
            verifier_record_id: VerifierRecordId::from_raw(format!(
                "coding-verification:{}",
                &sha256(input.proposal.proposal_sha256.as_bytes())[..24]
            )),
            verifier_id: self.verifier_id.clone(),
            task_id: input.request.task.task_id.clone(),
            proposal_id: input.proposal.proposal_id.clone(),
            repository_snapshot_id: input.request.repository_snapshot_id.clone(),
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

fn parse_candidate(
    proposal: &ClosedModelProposal,
    objective: &str,
) -> Option<CodingCompletionCandidate> {
    if proposal.kind != ModelProposalKind::CompletionCandidate || proposal.tool_call.is_some() {
        return None;
    }
    let payload = proposal.payload.as_ref()?;
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

fn required_evidence_present(input: &RuntimeVerificationInput<'_>) -> bool {
    input
        .request
        .work_packet
        .required_evidence
        .iter()
        .all(|kind| input.evidence.iter().any(|evidence| &evidence.kind == kind))
        && input.evidence.iter().all(|evidence| {
            evidence.observed_revision.as_deref()
                == Some(input.request.repository_snapshot_id.as_str())
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
