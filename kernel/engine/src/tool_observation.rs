//! Closed terminal tool-observation assembly and output-artifact binding.

use std::collections::BTreeMap;
use std::fmt::Write;

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, CanonicalArtifactReference, CanonicalRetryDisposition,
    CanonicalStateChange, CanonicalToolObservation, CanonicalToolOutcome, ToolCall,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::engineering_records::ValidateCanonicalRecord;
use crate::tool_composition::{
    ToolCleanupState, ToolTerminalAttemptRecord, ToolVerificationState, ToolWorkerDisposition,
};

/// Hard maximum complete bytes in each separately captured output family.
pub const MAX_TOOL_OBSERVATION_STREAM_BYTES: usize = 16 * 1024 * 1024;
/// Hard maximum redacted display excerpt.
pub const MAX_TOOL_OBSERVATION_EXCERPT_BYTES: usize = 8_192;

/// Exact separately stored output family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolObservationArtifactKind {
    /// Complete standard output bytes.
    StandardOutput,
    /// Complete standard error bytes.
    StandardError,
    /// Complete binary output bytes.
    BinaryOutput,
    /// Complete structured-result bytes.
    StructuredResult,
}

impl ToolObservationArtifactKind {
    /// Stable artifact-role identity.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::StandardOutput => "tool-output.stdout",
            Self::StandardError => "tool-output.stderr",
            Self::BinaryOutput => "tool-output.binary",
            Self::StructuredResult => "tool-output.structured",
        }
    }
}

/// Candidate bytes submitted atomically to the existing content-addressed artifact authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolObservationArtifactCandidate {
    /// Exact output family.
    pub kind: ToolObservationArtifactKind,
    /// Complete exact bytes.
    pub bytes: Vec<u8>,
    /// Exact byte length.
    pub byte_length: u64,
    /// Digest of complete bytes.
    pub sha256: String,
}

/// Atomic content-addressed artifact publication port.
pub trait ToolObservationArtifactPort {
    /// Publishes all four output families atomically or publishes none.
    fn publish_batch(
        &mut self,
        candidates: &[ToolObservationArtifactCandidate],
    ) -> Result<
        Vec<(ToolObservationArtifactKind, CanonicalArtifactReference)>,
        ToolObservationArtifactError,
    >;
}

/// Read-only verification of complete output artifacts during restart recovery.
pub trait ToolObservationArtifactReader {
    /// Confirms that an exact content-addressed artifact remains retrievable.
    fn contains_exact(&self, reference: &CanonicalArtifactReference) -> bool;
}

/// Closed output-publication failure from the artifact authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolObservationArtifactError;

/// Detailed terminal classification retained beyond the older canonical schema taxonomy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolObservationDisposition {
    /// Verified successful effect.
    Succeeded,
    /// Verified no-op.
    VerifiedNoOp,
    /// Authority denied dispatch.
    Denied,
    /// Cancellation won the terminal race.
    Cancelled,
    /// Timeout won the terminal race.
    TimedOut,
    /// Resource ceiling was exhausted.
    ResourceExhausted,
    /// Worker or supervisor crashed.
    Crashed,
    /// Output could not satisfy its closed schema.
    MalformedOutput,
    /// Some but not all expected effects occurred.
    PartialEffect,
    /// Current effect truth cannot be reconciled.
    Uncertain,
}

/// Exact measured process and resource facts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolObservationResources {
    /// Owned process identity, absent for non-process tools.
    pub process_id: Option<String>,
    /// Elapsed duration.
    pub elapsed_ms: u64,
    /// Peak resident memory.
    pub peak_memory_bytes: u64,
    /// Measured CPU time.
    pub cpu_time_ms: u64,
    /// Maximum descendant count.
    pub descendant_count: u32,
    /// Total captured output bytes across all four families.
    pub output_bytes: u64,
}

/// Exact admitted resource ceilings governing the attempt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolObservationLimits {
    /// Maximum elapsed duration.
    pub max_elapsed_ms: u64,
    /// Maximum resident memory.
    pub max_memory_bytes: u64,
    /// Maximum CPU time.
    pub max_cpu_time_ms: u64,
    /// Maximum descendant count.
    pub max_descendant_count: u32,
    /// Maximum aggregate captured output.
    pub max_output_bytes: u64,
}

/// Exact policy and authority lineage unavailable in the legacy canonical projection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolObservationLineage {
    /// Attempt identity.
    pub attempt_id: String,
    /// Composition policy digest.
    pub policy_sha256: String,
    /// Consumed grant digest.
    pub consumed_grant_sha256: String,
    /// Approval identity.
    pub approval_id: String,
    /// Composition terminal receipt identity.
    pub terminal_receipt_id: String,
    /// Optional worker receipt.
    pub worker_receipt_id: Option<String>,
    /// Deterministic verification state.
    pub verification: ToolVerificationState,
    /// Cleanup state.
    pub cleanup: ToolCleanupState,
}

/// Full assembly input from trusted runtime measurements.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolObservationInput {
    /// Stable canonical observation identity.
    pub observation_id: String,
    /// Owning task.
    pub task_id: String,
    /// Owning workflow step.
    pub step_id: String,
    /// Exact validated call.
    pub call: ToolCall,
    /// Exact terminal attempt record.
    pub terminal: ToolTerminalAttemptRecord,
    /// Approval identity.
    pub approval_id: String,
    /// Exact composition policy digest.
    pub policy_sha256: String,
    /// Tool input/output schema digest.
    pub tool_schema_sha256: String,
    /// Trusted RFC 3339 start time.
    pub started_at: String,
    /// Trusted RFC 3339 completion time.
    pub completed_at: String,
    /// Detailed terminal state.
    pub disposition: ToolObservationDisposition,
    /// Process exit code.
    pub exit_code: Option<i32>,
    /// Process signal description.
    pub signal: Option<String>,
    /// Complete standard output.
    pub stdout: Vec<u8>,
    /// Complete standard error.
    pub stderr: Vec<u8>,
    /// Complete binary output.
    pub binary_output: Vec<u8>,
    /// Complete structured result.
    pub structured_result: Vec<u8>,
    /// Worker-declared standard-output truncation.
    pub stdout_truncated: bool,
    /// Worker-declared standard-error truncation.
    pub stderr_truncated: bool,
    /// Caller-selected excerpt ceiling.
    pub excerpt_bytes: usize,
    /// Exact measured resources.
    pub resources: ToolObservationResources,
    /// Exact admitted resource ceilings.
    pub limits: ToolObservationLimits,
    /// Exact state change.
    pub state_change: CanonicalStateChange,
    /// Generated artifact identities besides captured output.
    pub generated_artifact_ids: Vec<String>,
}

/// Complete four-family output artifact references.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolObservationOutputArtifacts {
    /// Complete standard output.
    pub stdout: CanonicalArtifactReference,
    /// Complete standard error.
    pub stderr: CanonicalArtifactReference,
    /// Complete binary output.
    pub binary_output: CanonicalArtifactReference,
    /// Complete structured result.
    pub structured_result: CanonicalArtifactReference,
}

/// Closed complete terminal observation plus the legacy canonical projection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClosedToolObservation {
    /// Detailed terminal classification.
    pub disposition: ToolObservationDisposition,
    /// Complete canonical projection consumed by existing workflow verifiers.
    pub canonical: CanonicalToolObservation,
    /// Complete separately stored outputs.
    pub outputs: ToolObservationOutputArtifacts,
    /// Exact lineage.
    pub lineage: ToolObservationLineage,
    /// Exact resources.
    pub resources: ToolObservationResources,
    /// Exact admitted resource ceilings.
    pub limits: ToolObservationLimits,
    /// Digest binding every preceding field.
    pub observation_sha256: String,
}

impl ClosedToolObservation {
    /// Verifies all artifact bindings, lineage, terminal semantics, and observation digest.
    #[must_use]
    pub fn verify(&self) -> bool {
        self.observation_sha256 == observation_digest(self)
            && self.canonical.terminal
            && self.canonical.attempt_id == self.lineage.attempt_id
            && self.canonical.receipt_sha256
                == sha256_hex(self.lineage.terminal_receipt_id.as_bytes())
            && self.canonical.resource_usage_sha256 == sha256_json(&(&self.resources, &self.limits))
            && self.canonical.stdout.as_ref() == Some(&self.outputs.stdout)
            && self.canonical.stderr.as_ref() == Some(&self.outputs.stderr)
            && self
                .canonical
                .generated_artifact_ids
                .contains(&self.outputs.binary_output.artifact_id)
            && self
                .canonical
                .generated_artifact_ids
                .contains(&self.outputs.structured_result.artifact_id)
            && !false_success(self)
    }
}

/// Stable assembly refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolObservationError {
    /// Identity, timing, resource, output, or terminal binding is invalid.
    InvalidInput,
    /// Atomic artifact publication failed or returned forged references.
    ArtifactPublicationFailed,
    /// An observation already exists for this tool call.
    DuplicateTerminal,
}

/// Exactly-once terminal observation ledger.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ToolObservationLedger {
    observations: BTreeMap<String, ClosedToolObservation>,
}

impl ToolObservationLedger {
    /// Restores only complete, digest-valid, uniquely correlated terminal observations.
    pub fn restore(
        observations: Vec<ClosedToolObservation>,
        artifacts: &impl ToolObservationArtifactReader,
    ) -> Result<Self, ToolObservationError> {
        let mut restored = Self::default();
        for observation in observations {
            if !observation.verify()
                || observation.canonical.validate_canonical().is_err()
                || [
                    &observation.outputs.stdout,
                    &observation.outputs.stderr,
                    &observation.outputs.binary_output,
                    &observation.outputs.structured_result,
                ]
                .into_iter()
                .any(|reference| !artifacts.contains_exact(reference))
            {
                return Err(ToolObservationError::InvalidInput);
            }
            let call_id = observation.canonical.tool_call_id.clone();
            if restored.observations.insert(call_id, observation).is_some() {
                return Err(ToolObservationError::DuplicateTerminal);
            }
        }
        Ok(restored)
    }

    /// Number of exact terminal observations.
    #[must_use]
    pub fn len(&self) -> usize {
        self.observations.len()
    }

    /// Whether the ledger is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.observations.is_empty()
    }

    /// Returns the terminal observation for one tool call.
    #[must_use]
    pub fn get(&self, tool_call_id: &str) -> Option<&ClosedToolObservation> {
        self.observations.get(tool_call_id)
    }
}

/// Atomically publishes complete outputs and assembles exactly one terminal observation.
pub fn assemble_tool_observation<'ledger>(
    input: ToolObservationInput,
    artifacts: &mut impl ToolObservationArtifactPort,
    ledger: &'ledger mut ToolObservationLedger,
) -> Result<&'ledger ClosedToolObservation, ToolObservationError> {
    validate_input(&input)?;
    let call_id = input.call.tool_call_id.as_str().to_owned();
    if ledger.observations.contains_key(&call_id) {
        return Err(ToolObservationError::DuplicateTerminal);
    }
    let candidates = candidates(&input);
    let published = artifacts
        .publish_batch(&candidates)
        .map_err(|_| ToolObservationError::ArtifactPublicationFailed)?;
    let outputs = validate_publication(&candidates, published)?;
    let stdout_excerpt = redacted_excerpt(&input.stdout, input.excerpt_bytes);
    let stderr_excerpt = redacted_excerpt(&input.stderr, input.excerpt_bytes);
    let lineage = ToolObservationLineage {
        attempt_id: input.terminal.attempt_id.clone(),
        policy_sha256: input.policy_sha256.clone(),
        consumed_grant_sha256: input.terminal.consumed_grant_sha256.clone(),
        approval_id: input.approval_id.clone(),
        terminal_receipt_id: input.terminal.receipt_id.clone(),
        worker_receipt_id: input.terminal.worker_receipt_id.clone(),
        verification: input.terminal.verification,
        cleanup: input.terminal.cleanup,
    };
    let mut generated = input.generated_artifact_ids;
    generated.extend([
        outputs.binary_output.artifact_id.clone(),
        outputs.structured_result.artifact_id.clone(),
    ]);
    generated.sort();
    generated.dedup();
    let canonical = CanonicalToolObservation {
        schema_version: CONTRACT_SCHEMA_VERSION,
        observation_id: input.observation_id,
        tool_call_id: call_id.clone(),
        attempt_id: input.terminal.attempt_id,
        task_id: input.task_id,
        step_id: input.step_id,
        tool_id: input.call.tool_id.as_str().to_owned(),
        tool_version: input.call.tool_version,
        tool_schema_sha256: input.tool_schema_sha256,
        arguments_sha256: input.call.arguments.sha256,
        authority_id: input.approval_id,
        started_at: input.started_at,
        completed_at: input.completed_at,
        outcome: canonical_outcome(input.disposition),
        exit_code: input.exit_code,
        signal: input.signal,
        stdout: Some(outputs.stdout.clone()),
        stderr: Some(outputs.stderr.clone()),
        stdout_excerpt: stdout_excerpt.text,
        stderr_excerpt: stderr_excerpt.text,
        stdout_truncated: input.stdout_truncated || stdout_excerpt.truncated,
        stderr_truncated: input.stderr_truncated || stderr_excerpt.truncated,
        generated_artifact_ids: generated,
        state_change: input.state_change,
        descendants_cleaned: input.terminal.cleanup == ToolCleanupState::Complete,
        resource_usage_sha256: sha256_json(&(&input.resources, &input.limits)),
        retry_disposition: retry_disposition(input.disposition),
        receipt_sha256: sha256_hex(input.terminal.receipt_id.as_bytes()),
        terminal: true,
    };
    let mut observation = ClosedToolObservation {
        disposition: input.disposition,
        canonical,
        outputs,
        lineage,
        resources: input.resources,
        limits: input.limits,
        observation_sha256: "0".repeat(64),
    };
    observation.observation_sha256 = observation_digest(&observation);
    if !observation.verify() || observation.canonical.validate_canonical().is_err() {
        return Err(ToolObservationError::InvalidInput);
    }
    ledger.observations.insert(call_id.clone(), observation);
    Ok(ledger
        .observations
        .get(&call_id)
        .expect("inserted observation"))
}

fn validate_input(input: &ToolObservationInput) -> Result<(), ToolObservationError> {
    if !input.terminal.verify()
        || input.terminal.attempt_id.is_empty()
        || input.call.tool_call_id.as_str().is_empty()
        || input.task_id.is_empty()
        || input.step_id.is_empty()
        || input.approval_id.is_empty()
        || !valid_sha256(&input.policy_sha256)
        || !valid_sha256(&input.tool_schema_sha256)
        || input.started_at.is_empty()
        || input.completed_at.is_empty()
        || input.excerpt_bytes > MAX_TOOL_OBSERVATION_EXCERPT_BYTES
        || input.resources.output_bytes
            != [
                input.stdout.len(),
                input.stderr.len(),
                input.binary_output.len(),
                input.structured_result.len(),
            ]
            .into_iter()
            .map(|length| length as u64)
            .sum::<u64>()
        || [
            input.stdout.len(),
            input.stderr.len(),
            input.binary_output.len(),
            input.structured_result.len(),
        ]
        .into_iter()
        .any(|length| length > MAX_TOOL_OBSERVATION_STREAM_BYTES)
        || input.generated_artifact_ids.len() > 254
        || input.generated_artifact_ids.iter().any(|id| id.is_empty())
        || input
            .generated_artifact_ids
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != input.generated_artifact_ids.len()
        || input.limits.max_elapsed_ms == 0
        || input.limits.max_memory_bytes == 0
        || input.limits.max_cpu_time_ms == 0
        || input.limits.max_output_bytes == 0
    {
        return Err(ToolObservationError::InvalidInput);
    }
    let resource_ceiling_exceeded = input.resources.elapsed_ms > input.limits.max_elapsed_ms
        || input.resources.peak_memory_bytes > input.limits.max_memory_bytes
        || input.resources.cpu_time_ms > input.limits.max_cpu_time_ms
        || input.resources.descendant_count > input.limits.max_descendant_count
        || input.resources.output_bytes > input.limits.max_output_bytes;
    if resource_ceiling_exceeded
        && input.disposition != ToolObservationDisposition::ResourceExhausted
    {
        return Err(ToolObservationError::InvalidInput);
    }
    let expected = disposition_for_terminal(&input.terminal);
    if input.disposition != expected
        && !matches!(
            input.disposition,
            ToolObservationDisposition::TimedOut
                | ToolObservationDisposition::ResourceExhausted
                | ToolObservationDisposition::Crashed
                | ToolObservationDisposition::MalformedOutput
        )
    {
        return Err(ToolObservationError::InvalidInput);
    }
    if matches!(
        input.disposition,
        ToolObservationDisposition::Succeeded | ToolObservationDisposition::VerifiedNoOp
    ) && !input.terminal.completion_verified
    {
        return Err(ToolObservationError::InvalidInput);
    }
    Ok(())
}

fn candidates(input: &ToolObservationInput) -> Vec<ToolObservationArtifactCandidate> {
    [
        (ToolObservationArtifactKind::StandardOutput, &input.stdout),
        (ToolObservationArtifactKind::StandardError, &input.stderr),
        (
            ToolObservationArtifactKind::BinaryOutput,
            &input.binary_output,
        ),
        (
            ToolObservationArtifactKind::StructuredResult,
            &input.structured_result,
        ),
    ]
    .into_iter()
    .map(|(kind, bytes)| ToolObservationArtifactCandidate {
        kind,
        bytes: bytes.clone(),
        byte_length: bytes.len() as u64,
        sha256: sha256_hex(bytes),
    })
    .collect()
}

fn validate_publication(
    candidates: &[ToolObservationArtifactCandidate],
    published: Vec<(ToolObservationArtifactKind, CanonicalArtifactReference)>,
) -> Result<ToolObservationOutputArtifacts, ToolObservationError> {
    if published.len() != 4 {
        return Err(ToolObservationError::ArtifactPublicationFailed);
    }
    let map = published.into_iter().collect::<BTreeMap<_, _>>();
    if map.len() != 4
        || candidates.iter().any(|candidate| {
            map.get(&candidate.kind).is_none_or(|reference| {
                reference.artifact_id.is_empty()
                    || reference.sha256 != candidate.sha256
                    || reference.byte_length != candidate.byte_length
            })
        })
    {
        return Err(ToolObservationError::ArtifactPublicationFailed);
    }
    Ok(ToolObservationOutputArtifacts {
        stdout: map[&ToolObservationArtifactKind::StandardOutput].clone(),
        stderr: map[&ToolObservationArtifactKind::StandardError].clone(),
        binary_output: map[&ToolObservationArtifactKind::BinaryOutput].clone(),
        structured_result: map[&ToolObservationArtifactKind::StructuredResult].clone(),
    })
}

struct Excerpt {
    text: String,
    truncated: bool,
}

fn redacted_excerpt(bytes: &[u8], limit: usize) -> Excerpt {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Excerpt {
            text: "[non-utf8-output-withheld]".to_owned(),
            truncated: !bytes.is_empty(),
        };
    };
    let lowered = text.to_ascii_lowercase();
    if [
        "authorization: bearer ",
        "private key",
        "aws_secret_access_key",
        "api_key=",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
    {
        return Excerpt {
            text: "[sensitive-output-withheld]".to_owned(),
            truncated: !text.is_empty(),
        };
    }
    let end = text
        .char_indices()
        .map(|(index, _)| index)
        .take_while(|index| *index <= limit)
        .last()
        .unwrap_or(0);
    let end = if text.len() <= limit { text.len() } else { end };
    Excerpt {
        text: text[..end].to_owned(),
        truncated: end < text.len(),
    }
}

fn disposition_for_terminal(terminal: &ToolTerminalAttemptRecord) -> ToolObservationDisposition {
    match terminal.disposition {
        ToolWorkerDisposition::Succeeded => ToolObservationDisposition::Succeeded,
        ToolWorkerDisposition::NoOp => ToolObservationDisposition::VerifiedNoOp,
        ToolWorkerDisposition::Partial => ToolObservationDisposition::PartialEffect,
        ToolWorkerDisposition::Denied | ToolWorkerDisposition::Blocked => {
            ToolObservationDisposition::Denied
        }
        ToolWorkerDisposition::Cancelled => ToolObservationDisposition::Cancelled,
        ToolWorkerDisposition::Failed => ToolObservationDisposition::Crashed,
        ToolWorkerDisposition::Uncertain => ToolObservationDisposition::Uncertain,
    }
}

fn canonical_outcome(disposition: ToolObservationDisposition) -> CanonicalToolOutcome {
    match disposition {
        ToolObservationDisposition::Succeeded | ToolObservationDisposition::VerifiedNoOp => {
            CanonicalToolOutcome::Succeeded
        }
        ToolObservationDisposition::Denied => CanonicalToolOutcome::Denied,
        ToolObservationDisposition::Cancelled => CanonicalToolOutcome::Cancelled,
        ToolObservationDisposition::TimedOut => CanonicalToolOutcome::TimedOut,
        ToolObservationDisposition::ResourceExhausted => CanonicalToolOutcome::ResourceExhausted,
        ToolObservationDisposition::Uncertain | ToolObservationDisposition::PartialEffect => {
            CanonicalToolOutcome::Uncertain
        }
        ToolObservationDisposition::Crashed | ToolObservationDisposition::MalformedOutput => {
            CanonicalToolOutcome::Failed
        }
    }
}

fn retry_disposition(disposition: ToolObservationDisposition) -> CanonicalRetryDisposition {
    match disposition {
        ToolObservationDisposition::Succeeded | ToolObservationDisposition::VerifiedNoOp => {
            CanonicalRetryDisposition::NotEligible
        }
        ToolObservationDisposition::Denied => CanonicalRetryDisposition::UserDecisionRequired,
        ToolObservationDisposition::Uncertain | ToolObservationDisposition::PartialEffect => {
            CanonicalRetryDisposition::ReconcileFirst
        }
        _ => CanonicalRetryDisposition::EligibleFreshAttempt,
    }
}

fn false_success(value: &ClosedToolObservation) -> bool {
    matches!(
        value.disposition,
        ToolObservationDisposition::Succeeded | ToolObservationDisposition::VerifiedNoOp
    ) && (value.lineage.worker_receipt_id.is_none()
        || value.lineage.verification != ToolVerificationState::Passed
        || value.lineage.cleanup != ToolCleanupState::Complete)
}

fn observation_digest(value: &ClosedToolObservation) -> String {
    let mut copy = value.clone();
    copy.observation_sha256 = "0".repeat(64);
    sha256_json(&copy)
}

fn sha256_json(value: &impl Serialize) -> String {
    sha256_hex(&serde_json::to_vec(value).expect("closed observation serialization"))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("String write");
    }
    output
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        ActionId, ContractPayload, CorrelationId, SchemaId, SchemaReference, ToolCallId, ToolId,
    };

    use super::*;

    #[derive(Clone, Default)]
    struct MemoryArtifacts {
        batches: u32,
        fail: bool,
        forge: bool,
        stored: BTreeMap<String, Vec<u8>>,
    }

    impl ToolObservationArtifactPort for MemoryArtifacts {
        fn publish_batch(
            &mut self,
            candidates: &[ToolObservationArtifactCandidate],
        ) -> Result<
            Vec<(ToolObservationArtifactKind, CanonicalArtifactReference)>,
            ToolObservationArtifactError,
        > {
            if self.fail {
                return Err(ToolObservationArtifactError);
            }
            let mut staged = Vec::new();
            let mut records = Vec::new();
            for candidate in candidates {
                let id = format!("artifact:{}", candidate.kind.id());
                staged.push((id.clone(), candidate.bytes.clone()));
                records.push((
                    candidate.kind,
                    CanonicalArtifactReference {
                        artifact_id: id,
                        sha256: if self.forge {
                            "f".repeat(64)
                        } else {
                            candidate.sha256.clone()
                        },
                        byte_length: candidate.byte_length,
                    },
                ));
            }
            self.batches += 1;
            self.stored.extend(staged);
            Ok(records)
        }
    }

    impl ToolObservationArtifactReader for MemoryArtifacts {
        fn contains_exact(&self, reference: &CanonicalArtifactReference) -> bool {
            self.stored.iter().any(|(id, bytes)| {
                id == &reference.artifact_id
                    && bytes.len() as u64 == reference.byte_length
                    && sha256_hex(bytes) == reference.sha256
            })
        }
    }

    fn call(id: &str) -> ToolCall {
        let bytes = br#"{"fixture":true}"#.to_vec();
        ToolCall {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw(format!("tool-call:{id}")),
            correlation_id: CorrelationId::from_raw(format!("correlation:{id}")),
            action_id: ActionId::from_raw(format!("action:{id}")),
            tool_id: ToolId::from_raw("fixture.tool"),
            tool_version: "1.0.0".to_owned(),
            arguments: ContractPayload {
                schema: SchemaReference {
                    schema_id: SchemaId::from_raw("fixture.input"),
                    schema_version: 1,
                    schema_sha256: "a".repeat(64),
                },
                media_type: "application/json".to_owned(),
                sha256: sha256_hex(&bytes),
                bytes,
            },
        }
    }

    fn terminal(id: &str, disposition: ToolWorkerDisposition) -> ToolTerminalAttemptRecord {
        let successful = matches!(
            disposition,
            ToolWorkerDisposition::Succeeded | ToolWorkerDisposition::NoOp
        );
        ToolTerminalAttemptRecord::seal_observed(
            format!("attempt:{id}"),
            "b".repeat(64),
            "c".repeat(64),
            disposition,
            successful.then(|| format!("worker-receipt:{id}")),
            "d".repeat(64),
            Vec::new(),
            Vec::new(),
            if successful {
                ToolVerificationState::Passed
            } else {
                ToolVerificationState::Uncertain
            },
            if matches!(disposition, ToolWorkerDisposition::Uncertain) {
                ToolCleanupState::Unknown
            } else {
                ToolCleanupState::Complete
            },
        )
        .expect("terminal")
    }

    fn worker_for(disposition: ToolObservationDisposition) -> ToolWorkerDisposition {
        match disposition {
            ToolObservationDisposition::Succeeded => ToolWorkerDisposition::Succeeded,
            ToolObservationDisposition::VerifiedNoOp => ToolWorkerDisposition::NoOp,
            ToolObservationDisposition::Denied => ToolWorkerDisposition::Denied,
            ToolObservationDisposition::Cancelled => ToolWorkerDisposition::Cancelled,
            ToolObservationDisposition::PartialEffect => ToolWorkerDisposition::Partial,
            ToolObservationDisposition::Uncertain => ToolWorkerDisposition::Uncertain,
            ToolObservationDisposition::TimedOut
            | ToolObservationDisposition::ResourceExhausted
            | ToolObservationDisposition::Crashed
            | ToolObservationDisposition::MalformedOutput => ToolWorkerDisposition::Failed,
        }
    }

    fn input(id: &str, disposition: ToolObservationDisposition) -> ToolObservationInput {
        let stdout = b"ordinary stdout".to_vec();
        let stderr = b"ordinary stderr".to_vec();
        let binary_output = vec![0, 1, 2, 255];
        let structured_result = br#"{"ok":true}"#.to_vec();
        ToolObservationInput {
            observation_id: format!("observation:{id}"),
            task_id: "task:1".to_owned(),
            step_id: "step:1".to_owned(),
            call: call(id),
            terminal: terminal(id, worker_for(disposition)),
            approval_id: "approval:1".to_owned(),
            policy_sha256: "e".repeat(64),
            tool_schema_sha256: "a".repeat(64),
            started_at: "2026-08-31T12:00:00Z".to_owned(),
            completed_at: "2026-08-31T12:00:01Z".to_owned(),
            disposition,
            exit_code: Some(0),
            signal: None,
            resources: ToolObservationResources {
                process_id: Some(format!("process:{id}")),
                elapsed_ms: 1_000,
                peak_memory_bytes: 1024,
                cpu_time_ms: 10,
                descendant_count: 0,
                output_bytes: (stdout.len()
                    + stderr.len()
                    + binary_output.len()
                    + structured_result.len()) as u64,
            },
            limits: ToolObservationLimits {
                max_elapsed_ms: 10_000,
                max_memory_bytes: 1024 * 1024,
                max_cpu_time_ms: 1_000,
                max_descendant_count: 8,
                max_output_bytes: 1024 * 1024,
            },
            stdout,
            stderr,
            binary_output,
            structured_result,
            stdout_truncated: false,
            stderr_truncated: false,
            excerpt_bytes: 1024,
            state_change: CanonicalStateChange::NotChanged,
            generated_artifact_ids: vec!["artifact:generated".to_owned()],
        }
    }

    #[test]
    fn story_16_4_all_ten_terminal_states_emit_one_closed_observation() {
        for (index, disposition) in [
            ToolObservationDisposition::Succeeded,
            ToolObservationDisposition::VerifiedNoOp,
            ToolObservationDisposition::Denied,
            ToolObservationDisposition::Cancelled,
            ToolObservationDisposition::TimedOut,
            ToolObservationDisposition::ResourceExhausted,
            ToolObservationDisposition::Crashed,
            ToolObservationDisposition::MalformedOutput,
            ToolObservationDisposition::PartialEffect,
            ToolObservationDisposition::Uncertain,
        ]
        .into_iter()
        .enumerate()
        {
            let mut artifacts = MemoryArtifacts::default();
            let mut ledger = ToolObservationLedger::default();
            let observation = assemble_tool_observation(
                input(&format!("state-{index}"), disposition),
                &mut artifacts,
                &mut ledger,
            )
            .expect("observation");
            assert_eq!(observation.disposition, disposition);
            assert!(observation.verify());
            assert_eq!(artifacts.batches, 1);
            assert_eq!(artifacts.stored.len(), 4);
            if !matches!(
                disposition,
                ToolObservationDisposition::Succeeded | ToolObservationDisposition::VerifiedNoOp
            ) {
                assert_ne!(
                    observation.canonical.outcome,
                    CanonicalToolOutcome::Succeeded
                );
            }
            assert_eq!(ledger.len(), 1);
        }
    }

    #[test]
    fn story_16_4_complete_outputs_are_separate_content_addressed_and_excerpts_are_bounded_redacted()
     {
        let mut value = input("outputs", ToolObservationDisposition::Succeeded);
        value.stdout = "x".repeat(10_000).into_bytes();
        value.stderr = b"Authorization: Bearer secret-canary".to_vec();
        value.resources.output_bytes = (value.stdout.len()
            + value.stderr.len()
            + value.binary_output.len()
            + value.structured_result.len()) as u64;
        value.excerpt_bytes = 512;
        let expected_stdout = sha256_hex(&value.stdout);
        let expected_binary = sha256_hex(&value.binary_output);
        let mut artifacts = MemoryArtifacts::default();
        let mut ledger = ToolObservationLedger::default();
        let observation =
            assemble_tool_observation(value, &mut artifacts, &mut ledger).expect("observation");
        assert_eq!(observation.outputs.stdout.sha256, expected_stdout);
        assert_eq!(observation.outputs.binary_output.sha256, expected_binary);
        assert_eq!(observation.canonical.stdout_excerpt.len(), 512);
        assert!(observation.canonical.stdout_truncated);
        assert_eq!(
            observation.canonical.stderr_excerpt,
            "[sensitive-output-withheld]"
        );
        assert!(observation.canonical.stderr_truncated);
        assert!(
            !serde_json::to_string(observation)
                .expect("JSON")
                .contains("secret-canary")
        );
    }

    #[test]
    fn story_16_4_duplicate_mismatch_forgery_and_missing_artifacts_fail_closed() {
        let mut artifacts = MemoryArtifacts::default();
        let mut ledger = ToolObservationLedger::default();
        assemble_tool_observation(
            input("duplicate", ToolObservationDisposition::Succeeded),
            &mut artifacts,
            &mut ledger,
        )
        .expect("first");
        assert_eq!(
            assemble_tool_observation(
                input("duplicate", ToolObservationDisposition::Succeeded),
                &mut artifacts,
                &mut ledger,
            ),
            Err(ToolObservationError::DuplicateTerminal)
        );
        assert_eq!(artifacts.batches, 1);

        for mode in 0..4 {
            let mut artifacts = MemoryArtifacts {
                fail: mode == 0,
                forge: mode == 1,
                ..MemoryArtifacts::default()
            };
            let mut ledger = ToolObservationLedger::default();
            let mut value = input(
                &format!("invalid-{mode}"),
                ToolObservationDisposition::Succeeded,
            );
            if mode == 2 {
                value.resources.output_bytes += 1;
            } else if mode == 3 {
                value.generated_artifact_ids = vec![
                    "artifact:duplicate".to_owned(),
                    "artifact:duplicate".to_owned(),
                ];
            }
            assert!(assemble_tool_observation(value, &mut artifacts, &mut ledger).is_err());
            assert!(ledger.is_empty());
        }
    }

    #[test]
    fn story_16_4_output_and_excerpt_boundaries_fail_before_publication() {
        for mutation in 0..3 {
            let mut value = input(
                &format!("limit-{mutation}"),
                ToolObservationDisposition::Succeeded,
            );
            match mutation {
                0 => value.excerpt_bytes = MAX_TOOL_OBSERVATION_EXCERPT_BYTES + 1,
                1 => {
                    value.binary_output = vec![0; MAX_TOOL_OBSERVATION_STREAM_BYTES + 1];
                    value.resources.output_bytes = (value.stdout.len()
                        + value.stderr.len()
                        + value.binary_output.len()
                        + value.structured_result.len())
                        as u64;
                }
                _ => {
                    value.generated_artifact_ids =
                        (0..255).map(|i| format!("artifact:{i}")).collect()
                }
            }
            let mut artifacts = MemoryArtifacts::default();
            let mut ledger = ToolObservationLedger::default();
            assert_eq!(
                assemble_tool_observation(value, &mut artifacts, &mut ledger),
                Err(ToolObservationError::InvalidInput)
            );
            assert_eq!(artifacts.batches, 0);
        }
    }

    #[test]
    fn story_16_4_every_measured_resource_ceiling_fails_closed_before_publication() {
        for mutation in 0..5 {
            let mut value = input(
                &format!("resource-limit-{mutation}"),
                ToolObservationDisposition::Succeeded,
            );
            match mutation {
                0 => value.limits.max_elapsed_ms = value.resources.elapsed_ms - 1,
                1 => value.limits.max_memory_bytes = value.resources.peak_memory_bytes - 1,
                2 => value.limits.max_cpu_time_ms = value.resources.cpu_time_ms - 1,
                3 => value.limits.max_descendant_count = 0,
                _ => value.limits.max_output_bytes = value.resources.output_bytes - 1,
            }
            if mutation == 3 {
                value.resources.descendant_count = 1;
            }
            let mut artifacts = MemoryArtifacts::default();
            let mut ledger = ToolObservationLedger::default();
            assert_eq!(
                assemble_tool_observation(value, &mut artifacts, &mut ledger),
                Err(ToolObservationError::InvalidInput)
            );
            assert_eq!(artifacts.batches, 0);
        }
    }

    #[test]
    fn story_16_4_crash_race_pipe_and_uncertainty_never_infer_success() {
        for (index, disposition) in [
            ToolObservationDisposition::Cancelled,
            ToolObservationDisposition::TimedOut,
            ToolObservationDisposition::Crashed,
            ToolObservationDisposition::MalformedOutput,
            ToolObservationDisposition::PartialEffect,
            ToolObservationDisposition::Uncertain,
        ]
        .into_iter()
        .enumerate()
        {
            let mut value = input(&format!("race-{index}"), disposition);
            value.exit_code = None;
            value.signal = Some("supervisor-boundary".to_owned());
            let mut artifacts = MemoryArtifacts::default();
            let mut ledger = ToolObservationLedger::default();
            let observation = assemble_tool_observation(value, &mut artifacts, &mut ledger)
                .expect("terminal failure observation");
            assert_ne!(
                observation.canonical.outcome,
                CanonicalToolOutcome::Succeeded
            );
            assert!(observation.verify());
        }
    }

    #[test]
    fn story_16_4_tampered_observation_cannot_claim_more_than_measured() {
        let mut artifacts = MemoryArtifacts::default();
        let mut ledger = ToolObservationLedger::default();
        let original = assemble_tool_observation(
            input("tamper", ToolObservationDisposition::Succeeded),
            &mut artifacts,
            &mut ledger,
        )
        .expect("observation")
        .clone();
        let mut mutations = Vec::new();
        let mut value = original.clone();
        value.outputs.stdout.byte_length += 1;
        mutations.push(value);
        let mut value = original.clone();
        value.resources.peak_memory_bytes += 1;
        mutations.push(value);
        let mut value = original.clone();
        value.lineage.verification = ToolVerificationState::Failed;
        mutations.push(value);
        let mut value = original;
        value.canonical.descendants_cleaned = false;
        mutations.push(value);
        assert!(mutations.into_iter().all(|value| !value.verify()));
    }

    #[test]
    fn story_16_4_restart_restore_rejects_duplicate_corruption_and_replay() {
        let mut artifacts = MemoryArtifacts::default();
        let mut ledger = ToolObservationLedger::default();
        let first = assemble_tool_observation(
            input("restore-a", ToolObservationDisposition::Succeeded),
            &mut artifacts,
            &mut ledger,
        )
        .expect("first observation")
        .clone();
        let second = assemble_tool_observation(
            input("restore-b", ToolObservationDisposition::Crashed),
            &mut artifacts,
            &mut ledger,
        )
        .expect("second observation")
        .clone();

        let restored =
            ToolObservationLedger::restore(vec![second.clone(), first.clone()], &artifacts)
                .expect("reordered complete observations restore by identity");
        assert_eq!(restored.len(), 2);
        assert_eq!(
            ToolObservationLedger::restore(vec![first.clone(), first], &artifacts),
            Err(ToolObservationError::DuplicateTerminal)
        );
        let mut corrupted = second;
        corrupted.outputs.stderr.sha256 = "f".repeat(64);
        assert_eq!(
            ToolObservationLedger::restore(vec![corrupted], &artifacts),
            Err(ToolObservationError::InvalidInput)
        );
        let mut missing_artifacts = artifacts.clone();
        missing_artifacts.stored.clear();
        assert_eq!(
            ToolObservationLedger::restore(
                vec![ledger.get("tool-call:restore-a").expect("a").clone()],
                &missing_artifacts
            ),
            Err(ToolObservationError::InvalidInput)
        );
        assert!(
            ToolObservationLedger::restore(Vec::new(), &artifacts)
                .expect("an empty durable projection remains empty")
                .is_empty()
        );
    }

    #[cfg(unix)]
    #[test]
    fn story_16_4_live_local_process_output_closes_through_the_same_assembler() {
        let child = std::process::Command::new("/usr/bin/printf")
            .arg("live-local-tool-output")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("local bounded fixture process starts");
        let process_id = child.id();
        let output = child
            .wait_with_output()
            .expect("local bounded fixture process terminates");
        assert!(output.status.success());

        let mut value = input("live-local", ToolObservationDisposition::Succeeded);
        value.stdout = output.stdout;
        value.stderr = output.stderr;
        value.exit_code = output.status.code();
        value.resources.process_id = Some(format!("pid:{process_id}"));
        value.resources.output_bytes = (value.stdout.len()
            + value.stderr.len()
            + value.binary_output.len()
            + value.structured_result.len()) as u64;
        let expected = sha256_hex(&value.stdout);
        let mut artifacts = MemoryArtifacts::default();
        let mut ledger = ToolObservationLedger::default();
        let observation = assemble_tool_observation(value, &mut artifacts, &mut ledger)
            .expect("live local output closes");
        assert_eq!(observation.outputs.stdout.sha256, expected);
        assert_eq!(
            observation.resources.process_id,
            Some(format!("pid:{process_id}"))
        );
        assert!(observation.verify());
    }
}
