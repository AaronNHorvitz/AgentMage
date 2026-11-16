// SPDX-License-Identifier: MIT OR Apache-2.0
//! Exact read observations, never semantic approval of arbitrary coding tasks.

use std::collections::BTreeSet;

use agentmage_capability_read_only::{
    MAX_READ_ONLY_OUTPUT_BYTES, ReadOnlyItem, ReadOnlyOutcome, ReadOnlyResult, ReadOnlyToolKind,
    read_only_tool_definition,
};
use agentmage_kernel_contracts::*;
use agentmage_kernel_engine::{
    runtime_coordinator::verify_runtime_endpoint_run_request,
    runtime_loop::{
        RuntimeCompletionProposal, RuntimeCompletionVerificationInput, RuntimePortFailure,
        RuntimeVerificationInput, RuntimeVerifierPort, runtime_tool_references,
    },
};
use sha2::{Digest, Sha256};

use crate::{
    coding_verifier::{CodingTerminalClaim, parse_candidate},
    runtime_tools::workspace_file_read_runtime_registry,
};

/// One trusted acceptance check over a previously consented full-file snapshot.
/// Construction reads no file and grants no read or model-disclosure authority.
/// The caller must show the generated criterion as the task's actual acceptance
/// check; a free-text objective or model-generated digest cannot select checks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodingEndpointReadCheck {
    path: WorkspacePath,
    byte_len: u64,
    content_sha256: String,
}

impl CodingEndpointReadCheck {
    /// Retains an exact canonical path, full-file length and lowercase SHA-256.
    pub fn new(
        path: WorkspacePath,
        byte_len: u64,
        content_sha256: String,
    ) -> Result<Self, RuntimePortFailure> {
        if byte_len > MAX_READ_ONLY_OUTPUT_BYTES
            || content_sha256.len() != 64
            || !content_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(RuntimePortFailure::Invalid);
        }
        Ok(Self {
            path,
            byte_len,
            content_sha256,
        })
    }

    /// Exact file target; metadata is not permission to resolve or read it.
    #[must_use]
    pub const fn path(&self) -> &WorkspacePath {
        &self.path
    }

    /// Complete supported acceptance criterion. Arbitrary prose is not verified.
    #[must_use]
    pub fn criterion(&self) -> String {
        format!(
            "Observe complete UTF-8 file {} ({} bytes; SHA-256 {}).",
            self.path
                .components()
                .iter()
                .map(WorkspacePathComponent::as_str)
                .collect::<Vec<_>>()
                .join("/"),
            self.byte_len,
            self.content_sha256
        )
    }

    fn matches(&self, item: &ReadOnlyItem) -> bool {
        matches!(item, ReadOnlyItem::Text { path, byte_start: 0, byte_end, content, file_sha256 }
            if same_path(path, &self.path) && *byte_end == self.byte_len
                && content.len() as u64 == self.byte_len
                && file_sha256 == &self.content_sha256
                && hash(content.as_bytes()) == self.content_sha256)
    }
}

/// Concrete verifier for exact read-only observations. It binds the whole sealed
/// request and refuses every unsupported criterion, required validation, changed
/// state, incomplete read or ungrounded claim. It does not certify the meaning of
/// the model's summary, repository correctness, tests, or later filesystem state.
pub struct CodingEndpointReadVerifier {
    request_sha256: String,
    verifier_id: VerifierId,
    checks: Vec<CodingEndpointReadCheck>,
    postconditions: Vec<PostconditionId>,
}

impl CodingEndpointReadVerifier {
    /// Binds only caller-selected checks already present verbatim, in order, in
    /// both the sealed task and work packet. No inference from objective prose.
    pub fn for_request(
        request: &RuntimeEndpointRunRequest,
        checks: Vec<CodingEndpointReadCheck>,
    ) -> Result<Self, RuntimePortFailure> {
        verify_runtime_endpoint_run_request(request).map_err(|_| RuntimePortFailure::Invalid)?;
        let registry =
            workspace_file_read_runtime_registry().map_err(|_| RuntimePortFailure::Invalid)?;
        let criteria = checks
            .iter()
            .map(CodingEndpointReadCheck::criterion)
            .collect::<Vec<_>>();
        if request.mode != RuntimeSessionMode::EphemeralReadOnly
            || request.event_cursor.is_some()
            || checks.is_empty()
            || checks.len() > request.limits.max_tool_calls as usize
            || checks
                .iter()
                .any(|check| check.path.workspace_id() != &request.workspace_id)
            || checks
                .iter()
                .map(|check| {
                    check
                        .path
                        .components()
                        .iter()
                        .map(WorkspacePathComponent::as_str)
                        .collect::<Vec<_>>()
                })
                .collect::<BTreeSet<_>>()
                .len()
                != checks.len()
            || request.task.acceptance_criteria != criteria
            || request.work_packet.acceptance_checks != criteria
            || request.work_packet.required_evidence != [EvidenceKind::ToolOutput]
            || request.work_packet.required_capability_class != AuthorityClass::Observe
            || !request.work_packet.mutable_files.is_empty()
            || !request.work_packet.authoritative_evidence.is_empty()
            || runtime_tool_references(&registry).map_err(|_| RuntimePortFailure::Invalid)?
                != request.visible_tools
        {
            return Err(RuntimePortFailure::Invalid);
        }
        // Original runtime_loop postcondition identities, derived from the exact
        // criterion and its order. Coordinator integration tests check this contract.
        let postconditions = criteria
            .iter()
            .enumerate()
            .map(|(index, criterion)| {
                PostconditionId::from_raw(format!(
                    "postcondition:{index}:{}",
                    &hash(criterion.as_bytes())[..16]
                ))
            })
            .collect();
        Ok(Self {
            request_sha256: request.request_sha256.clone(),
            verifier_id: VerifierId::from_raw(format!(
                "endpoint-read-verifier:{}",
                &request.request_sha256[..24]
            )),
            checks,
            postconditions,
        })
    }

    fn grounded_reads(
        &self,
        input: &RuntimeCompletionVerificationInput<'_>,
    ) -> Option<Vec<ReadOnlyResult>> {
        let request = input.request.endpoint_request()?;
        if input.tool_results.is_empty()
            || input.tool_results.len() > request.limits.max_tool_calls as usize
            || input.receipt_ids.len() != input.tool_results.len()
            || input.evidence.len() != input.tool_results.len()
            || input
                .receipt_ids
                .iter()
                .map(|r| r.as_str())
                .collect::<BTreeSet<_>>()
                .len()
                != input.receipt_ids.len()
            || input
                .tool_results
                .iter()
                .map(|r| r.tool_call_id.as_str())
                .collect::<BTreeSet<_>>()
                .len()
                != input.tool_results.len()
            || input
                .evidence
                .iter()
                .map(|r| r.evidence_id.as_str())
                .collect::<BTreeSet<_>>()
                .len()
                != input.evidence.len()
            || input.receipt_ids.iter().any(|r| r.as_str().is_empty())
        {
            return None;
        }
        let definition = read_only_tool_definition(ReadOnlyToolKind::ReadText);
        input
            .tool_results
            .iter()
            .map(|result| {
                let output = result.output.as_ref()?;
                if result.schema_version != CONTRACT_SCHEMA_VERSION
                    || result.outcome != OperationOutcome::Succeeded
                    || result.state_change != StateChange::NotChanged
                    || result.error.is_some()
                    || !result.validation_issues.is_empty()
                    || result.tool_call_id.as_str().is_empty()
                    || output.schema != definition.output_schema
                    || output.media_type != "application/json"
                    || output.bytes.len() as u64
                        > request
                            .limits
                            .max_output_bytes
                            .min(MAX_READ_ONLY_OUTPUT_BYTES + 4096)
                    || output.sha256 != hash(&output.bytes)
                {
                    return None;
                }
                let read: ReadOnlyResult = serde_json::from_slice(&output.bytes).ok()?;
                if !read.verify(ReadOnlyToolKind::ReadText)
                    || read.outcome != ReadOnlyOutcome::Succeeded
                    || read.truncated
                    || read.observed_files != 1
                    || read.items.len() != 1
                    || result.evidence.len() != 1
                {
                    return None;
                }
                let ReadOnlyItem::Text {
                    path,
                    byte_start,
                    byte_end,
                    content,
                    file_sha256,
                } = &read.items[0]
                else {
                    return None;
                };
                WorkspacePath::new(request.workspace_id.clone(), path.clone()).ok()?;
                if *byte_start != 0
                    || *byte_end != read.observed_bytes
                    || content.len() as u64 != read.observed_bytes
                    || hash(content.as_bytes()) != *file_sha256
                {
                    return None;
                }
                let evidence = &result.evidence[0];
                if evidence.schema_version != CONTRACT_SCHEMA_VERSION
                    || evidence.evidence_id.as_str().is_empty()
                    || evidence.kind != EvidenceKind::ToolOutput
                    || evidence.source_id
                        != format!(
                            "native:{}@{}",
                            definition.tool_id.as_str(),
                            definition.tool_version
                        )
                    || evidence.object_id != result.tool_call_id.as_str()
                    || evidence.fragment.is_some()
                    || evidence.content_sha256 != read.result_sha256
                    || evidence.observed_revision.as_deref()
                        != Some(request.repository_snapshot_id.as_str())
                    || !input.evidence.contains(evidence)
                {
                    return None;
                }
                Some(read)
            })
            .collect()
    }
}

impl RuntimeVerifierPort for CodingEndpointReadVerifier {
    fn verifier_id(&self) -> &VerifierId {
        &self.verifier_id
    }

    fn verify(
        &mut self,
        _: RuntimeVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        Err(RuntimePortFailure::Invalid)
    }

    fn verify_completion(
        &mut self,
        input: RuntimeCompletionVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure> {
        let request = input
            .request
            .endpoint_request()
            .ok_or(RuntimePortFailure::Invalid)?;
        verify_runtime_endpoint_run_request(request).map_err(|_| RuntimePortFailure::Invalid)?;
        if request.request_sha256 != self.request_sha256
            || !input.proposal.matches_scope(input.request)
            || input.postconditions != self.postconditions
        {
            return Err(RuntimePortFailure::Invalid);
        }
        let claim_ok =
            parse_candidate(&input.proposal, &request.task.objective).is_some_and(|claim| {
                claim.terminal_claim == CodingTerminalClaim::NoOp && claim.checks_not_run.is_empty()
            });
        let reads = self.grounded_reads(&input);
        let postconditions = self
            .checks
            .iter()
            .zip(&self.postconditions)
            .map(|(check, id)| {
                let matching = reads
                    .as_ref()
                    .map(|reads| {
                        reads
                            .iter()
                            .enumerate()
                            .filter(|(_, read)| {
                                matches!(&read.items[0], ReadOnlyItem::Text { path, .. }
                                if same_path(path, &check.path))
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                // All observations of a checked file must match: an earlier good
                // read cannot conceal a later conflicting read of that same path.
                let passed = claim_ok
                    && !matching.is_empty()
                    && matching
                        .iter()
                        .all(|(_, read)| check.matches(&read.items[0]));
                let evidence = if passed {
                    matching
                        .iter()
                        .take(32)
                        .map(|(index, _)| input.tool_results[*index].evidence[0].clone())
                        .collect()
                } else {
                    vec![]
                };
                PostconditionResult {
                    postcondition_id: id.clone(),
                    passed,
                    evidence,
                }
            })
            .collect::<Vec<_>>();
        let passed = postconditions.iter().all(|check| check.passed);
        Ok(VerifierCandidate {
            schema_version: CONTRACT_SCHEMA_VERSION,
            verifier_record_id: VerifierRecordId::from_raw(format!(
                "endpoint-read-verification:{}",
                &hash(input.proposal.proposal_sha256().as_bytes())[..24]
            )),
            verifier_id: self.verifier_id.clone(),
            task_id: request.task.task_id.clone(),
            proposal_id: input.proposal.proposal_id().clone(),
            repository_snapshot_id: request.repository_snapshot_id.clone(),
            state_revision: input.state_revision,
            source: VerifierSource::DeterministicPostcondition,
            disposition: if passed {
                VerifierDisposition::NoOp
            } else {
                VerifierDisposition::Failed
            },
            postconditions,
        })
    }
}

fn same_path(path: &[String], expected: &WorkspacePath) -> bool {
    path.iter().map(String::as_str).eq(expected
        .components()
        .iter()
        .map(WorkspacePathComponent::as_str))
}

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
