//! Pure normalization for the existing research authority and artifact owners.
//! Supplied records and prepared references establish consistency, not canonical
//! provenance, native admission, permission, publication or workflow completion.

use std::collections::BTreeSet;
use std::io::{self, Write};

use agentmage_kernel_contracts::*;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::research_fetch::PublicGetWorkerPacket;
use crate::research_result_binding::{PublicGetNativeIdentity, PublicGetResultBinding};
use crate::research_retrieval::{PublicGetArtifactBundle, PublicGetArtifactOutput};
use crate::runtime_event::verify_runtime_event;
use crate::runtime_loop::{
    RuntimePortFailure, RuntimeToolArtifactCandidate, RuntimeToolExecution,
    RuntimeToolTerminalBuilder,
};

/// Complete descriptions supplied by trusted composition after successful cleanup.
/// None of these borrowed values constitutes a dispatch permit or canonical proof.
pub struct PublicGetCompletionRequest<'a> {
    /// Independently registered tool definition, not model-provided metadata.
    pub definition: &'a ToolDefinition,
    /// Original approved call, including its exact argument bytes.
    pub call: &'a ToolCall,
    /// Original prepared worker packet; no renewed deadline is admitted.
    pub packet: &'a PublicGetWorkerPacket,
    /// Original spent reservation, not a later accounting head.
    pub reservation_sha256: &'a str,
    /// Pending terminal snapshot from the existing authority owner.
    pub transaction: &'a AuthorityTransactionRecord,
    /// Exact receipt returned by that same owner.
    pub receipt: &'a Receipt,
    /// Complete native result material and response frame.
    pub binding: &'a PublicGetResultBinding,
    /// Independently admitted identities, never inferred from the result itself.
    pub expected_native: &'a PublicGetNativeIdentity,
    /// Exact start observed after durability, before the attempted effect.
    pub started: &'a RuntimeEvent,
}

/// Content-free refusal. The caller must propagate failure without retrying the
/// effect, accepting partial artifacts or attempting another terminal result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResearchCompletionError {
    /// Input descriptions, bytes, owner, schema or native interval do not agree.
    Binding,
    /// Serialization exceeds the existing complete-object read ceilings.
    Limit,
    /// The borrowed owner refused preparation or terminal sealing.
    Builder(RuntimePortFailure),
    /// Returned references do not describe the exact ordered candidates.
    Artifact,
    /// The returned terminal does not bind this exact start and complete result.
    Terminal,
}

/// Unpublished description, deliberately distinct from CanonicalPublicGetSource.
/// The existing authority owner must commit the terminal before its coordinator
/// publishes any artifact. Partial publication never makes a readable source.
pub struct PreparedPublicGetCompletion {
    execution: RuntimeToolExecution,
    terminal: RuntimeEvent,
    bundle: RuntimeArtifactRef,
}

impl std::fmt::Debug for PreparedPublicGetCompletion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedPublicGetCompletion")
            .field("bundle_sha256", &self.bundle.payload_sha256)
            .finish_non_exhaustive()
    }
}

impl PreparedPublicGetCompletion {
    /// Moves the exact sealed description to the existing commit owner. Never
    /// rewrite it or seal it again; this method grants no persistence authority.
    #[must_use]
    pub fn into_parts(self) -> (RuntimeToolExecution, RuntimeEvent, RuntimeArtifactRef) {
        (self.execution, self.terminal, self.bundle)
    }
}

/// Prepares four full source objects, their ToolResult, then the acyclic bundle,
/// using one borrowed artifact/terminal owner. No identifiers or times are minted
/// here. All ordinary runtime modes continue to refuse this network operation.
pub fn prepare_public_get_completion(
    request: &PublicGetCompletionRequest<'_>,
    builder: &mut dyn RuntimeToolTerminalBuilder,
) -> Result<PreparedPublicGetCompletion, ResearchCompletionError> {
    use ResearchCompletionError as E;
    validate(request)?;
    let mut candidates = vec![
        candidate(
            to_canonical_json(request.call).map_err(|_| E::Binding)?,
            false,
        ),
        candidate(request.packet.bytes().to_vec(), false),
        candidate(request.binding.redacted_material().to_vec(), false),
        candidate(request.binding.response().frame().to_vec(), true),
    ];
    let mut identifiers = BTreeSet::new();
    let references = append(builder, request.receipt, &candidates, &mut identifiers)?;
    let output = PublicGetArtifactOutput {
        schema_version: 1,
        packet: references[1].clone(),
        material: references[2].clone(),
        frame: references[3].clone(),
    };
    check_size(&output, crate::research_retrieval::MAX_BUNDLE)?;
    let bytes = serde_json::to_vec(&output).map_err(|_| E::Binding)?;
    let interval = request.binding.parent_interval();
    let result = ToolResult {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_call_id: request.call.tool_call_id.clone(),
        correlation_id: request.call.correlation_id.clone(),
        outcome: OperationOutcome::Succeeded,
        output: Some(ContractPayload {
            schema: request.definition.output_schema.clone(),
            media_type: "application/json".into(),
            sha256: digest(&bytes),
            bytes,
        }),
        validation_issues: Vec::new(),
        evidence: Vec::new(),
        error: None,
        elapsed_ms: interval.cleanup_verified_epoch_ms - interval.started_epoch_ms,
        state_change: StateChange::Changed,
    };
    check_size(&result, crate::research_retrieval::MAX_RESULT)?;
    let result_bytes = to_canonical_json(&result).map_err(|_| E::Binding)?;
    let result_sha256 = digest(&result_bytes);
    candidates.push(candidate(result_bytes, false));
    let result_refs = append(builder, request.receipt, &candidates[4..], &mut identifiers)?;
    let bundle = PublicGetArtifactBundle {
        schema_version: 1,
        transaction_id: request.receipt.authority_transaction_id.clone(),
        reservation_sha256: request.reservation_sha256.to_owned(),
        call: references[0].clone(),
        output,
        result: result_refs[0].clone(),
    };
    check_size(&bundle, crate::research_retrieval::MAX_BUNDLE)?;
    candidates.push(candidate(
        serde_json::to_vec(&bundle).map_err(|_| E::Binding)?,
        false,
    ));
    let bundle_refs = append(builder, request.receipt, &candidates[5..], &mut identifiers)?;
    let execution = RuntimeToolExecution {
        receipt_id: request.receipt.receipt_id.clone(),
        receipt_sha256: request.receipt.receipt_sha256.clone(),
        result,
        result_output_kind: Some(RuntimeArtifactKind::Report),
        artifact_candidates: candidates,
    };
    let terminal = builder
        .build_terminal_event(&execution)
        .map_err(E::Builder)?;
    validate_terminal(request, &terminal, &result_sha256)?;
    Ok(PreparedPublicGetCompletion {
        execution,
        terminal,
        bundle: bundle_refs[0].clone(),
    })
}

fn candidate(bytes: Vec<u8>, binary: bool) -> RuntimeToolArtifactCandidate {
    RuntimeToolArtifactCandidate {
        kind: RuntimeArtifactKind::Report,
        media_type: if binary {
            "application/octet-stream"
        } else {
            "application/json"
        }
        .into(),
        bytes,
    }
}

fn append(
    builder: &mut dyn RuntimeToolTerminalBuilder,
    receipt: &Receipt,
    candidates: &[RuntimeToolArtifactCandidate],
    identifiers: &mut BTreeSet<RuntimeArtifactId>,
) -> Result<Vec<RuntimeArtifactRef>, ResearchCompletionError> {
    use ResearchCompletionError as E;
    let refs = builder
        .prepare_artifacts(&receipt.receipt_id, &receipt.receipt_sha256, candidates)
        .map_err(E::Builder)?;
    if refs.len() != candidates.len() {
        return Err(E::Artifact);
    }
    for (reference, candidate) in refs.iter().zip(candidates) {
        if reference.schema_version != CONTRACT_SCHEMA_VERSION
            || !valid_id(reference.artifact_id.as_str())
            || !valid_digest(&reference.manifest_sha256)
            || reference.payload_sha256 != digest(&candidate.bytes)
            || reference.byte_size != candidate.bytes.len() as u64
            || reference.media_type != candidate.media_type
            || !identifiers.insert(reference.artifact_id.clone())
        {
            return Err(E::Artifact);
        }
    }
    Ok(refs)
}

fn validate(r: &PublicGetCompletionRequest<'_>) -> Result<(), ResearchCompletionError> {
    use ResearchCompletionError as E;
    // Bound serialization before any clone or canonical encoding of caller data.
    check_size(r.call, crate::research_retrieval::MAX_CALL)?;
    check_size(r.receipt, crate::research_retrieval::MAX_RESULT)?;
    check_size(r.transaction, crate::research_retrieval::MAX_RESULT)?;
    check_size(r.started, crate::research_retrieval::MAX_RESULT)?;
    crate::research_effect_binding::validate_call(r.definition, r.call, r.packet)
        .map_err(|_| E::Binding)?;
    verify_runtime_event(r.started).map_err(|_| E::Binding)?;
    let receipt = r.receipt;
    let tx = r.transaction;
    let operation = OperationBinding::new(GrantOperation::NetworkAccess);
    let schema = &r.definition.output_schema;
    if schema.schema_version != 1
        || !valid_id(schema.schema_id.as_str())
        || !valid_digest(&schema.schema_sha256)
        || receipt.schema_version != CONTRACT_SCHEMA_VERSION
        || tx.schema_version != CONTRACT_SCHEMA_VERSION
        || receipt.sequence == 0
        || tx.revision == 0
        || receipt.operation != operation
        || tx.operation != operation
        || receipt.outcome != OperationOutcome::Succeeded
        || receipt.error.is_some()
        || tx.state != AuthorityTransactionState::Terminal
        || tx.outcome != Some(OperationOutcome::Succeeded)
        || tx.uncertain_effect
        || tx.result_sha256.as_deref() != Some(digest(r.binding.redacted_material()).as_str())
        || tx
            .consumed_grant_sha256
            .as_deref()
            .is_none_or(|s| !valid_digest(s))
        || tx.receipt_id.as_ref() != Some(&receipt.receipt_id)
        || tx.receipt_sha256.as_ref() != Some(&receipt.receipt_sha256)
        || tx.authority_transaction_id != receipt.authority_transaction_id
        || tx.operation_attempt_id != receipt.operation_attempt_id
        || tx.approval_id != receipt.approval_id
        || tx.grant_id != receipt.grant_id
        || tx.session_id != receipt.session_id
        || tx.task_id != receipt.task_id
        || tx.action_id != receipt.action_id
        || tx.correlation_id != receipt.correlation_id
        || receipt.tool_call_id.as_ref() != Some(&tx.tool_call_id)
        || tx.tool_call_id != r.call.tool_call_id
        || tx.action_id != r.call.action_id
        || tx.correlation_id != r.call.correlation_id
        || tx.occurred_at != receipt.occurred_at
        || receipt.session_id != r.started.session_id
        || receipt.task_id != r.started.task_id
        || receipt.task_id.as_str() != r.packet.task_id()
        || r.started.correlation_id != r.call.correlation_id
        || r.started.turn_id.is_none()
        || r.started.operation_id.is_none()
        // The start carries the issued grant digest, while the transaction carries
        // its consumed revision. Only the canonical owner can compare that history.
        || !matches!(&r.started.kind, RuntimeEventKind::ToolStarted { tool_call_id, authority_sha256 }
            if tool_call_id == &r.call.tool_call_id && valid_digest(authority_sha256))
        || r.binding.native_identity() != r.expected_native
        || r.binding.parent_interval().started_epoch_ms < r.started.occurred_at_epoch_ms
    {
        return Err(E::Binding);
    }
    if [
        receipt.receipt_id.as_str(),
        receipt.authority_transaction_id.as_str(),
        receipt.operation_attempt_id.as_str(),
        receipt.approval_id.as_str(),
        receipt.grant_id.as_str(),
        receipt.action_id.as_str(),
    ]
    .iter()
    .any(|s| !valid_id(s))
        || !valid_digest(&receipt.receipt_sha256)
        || receipt.operation_sha256
            != digest(&serde_json::to_vec(&operation).map_err(|_| E::Binding)?)
    {
        return Err(E::Binding);
    }
    // Match the existing owner's canonical receipt representation, not a new signature.
    let mut unhashed = receipt.clone();
    unhashed.receipt_sha256 = "0".repeat(64);
    if digest(&to_canonical_json(&unhashed).map_err(|_| E::Binding)?) != receipt.receipt_sha256 {
        return Err(E::Binding);
    }
    PublicGetWorkerPacket::decode_worker_packet(r.packet.bytes(), r.started.occurred_at_epoch_ms)
        .map_err(|_| E::Binding)?;
    PublicGetResultBinding::decode_consistent(
        r.packet,
        r.reservation_sha256,
        r.binding.redacted_material(),
        r.binding.response().frame(),
    )
    .map_err(|_| E::Binding)?;
    Ok(())
}

fn validate_terminal(
    r: &PublicGetCompletionRequest<'_>,
    terminal: &RuntimeEvent,
    result_sha256: &str,
) -> Result<(), ResearchCompletionError> {
    use ResearchCompletionError as E;
    check_size(terminal, crate::research_retrieval::MAX_RESULT)?;
    verify_runtime_event(terminal).map_err(|_| E::Terminal)?;
    let start = r.started;
    if start.sequence.checked_add(1) != Some(terminal.sequence)
        || terminal.event_id == start.event_id
        || terminal.previous_event_sha256 != start.event_sha256
        || terminal.causation_event_id.as_ref() != Some(&start.event_id)
        || terminal.session_id != start.session_id
        || terminal.task_id != start.task_id
        || terminal.run_id != start.run_id
        || terminal.turn_id != start.turn_id
        || terminal.operation_id != start.operation_id
        || terminal.policy_id != start.policy_id
        || terminal.correlation_id != start.correlation_id
        || terminal.sensitivity != start.sensitivity
        || terminal.retention != start.retention
        || terminal.payload_reference.is_some()
        || terminal.occurred_at_epoch_ms < r.binding.parent_interval().cleanup_verified_epoch_ms
        || !matches!(&terminal.kind, RuntimeEventKind::ToolCompleted { tool_call_id, receipt_id, result_sha256: actual }
            if tool_call_id == &r.call.tool_call_id && receipt_id == &r.receipt.receipt_id && actual == result_sha256)
    {
        return Err(E::Terminal);
    }
    Ok(())
}

// Count encoded bytes without allocating a complete caller-controlled value first.
fn check_size(value: &impl Serialize, limit: u64) -> Result<(), ResearchCompletionError> {
    struct Counter(u64);
    impl Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0 = self
                .0
                .checked_sub(bytes.len() as u64)
                .ok_or_else(|| io::Error::other("research completion size limit"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(&mut Counter(limit), value).map_err(|_| ResearchCompletionError::Limit)
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':'))
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        && value.bytes().any(|b| b != b'0')
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
