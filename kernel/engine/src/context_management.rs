//! Deterministic context composition, checked summaries, and resume drift gates.

use std::collections::{BTreeMap, BTreeSet};

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, CheckedContextSummary, CheckedSummaryState, CheckpointFileIdentity,
    ComposedContextPacket, ContextAdmission, ContextItemAccounting, ContextItemCandidate,
    ContextItemKind, ContextOmissionReason, ContextPacketId, ResumeDriftDecision,
    ResumeDriftDimension, SessionCheckpoint, to_canonical_json,
};
use sha2::{Digest, Sha256};

const MAX_CONTEXT_ITEMS: usize = 4_096;
const MAX_EXCERPT_BYTES: usize = 65_536;
const MAX_ID_BYTES: usize = 256;
const MAX_REFERENCE_ITEMS: usize = 4_096;
const MAX_SUMMARY_BYTES: usize = 65_536;
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Stable failure from deterministic context composition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextCompositionError {
    /// The budget, counter identity, packet identity, or candidate shape is invalid.
    InvalidInput,
    /// Two candidates reuse an identity for different material.
    ConflictingIdentity,
    /// An essential eligible item cannot fit in the declared packet budget.
    EssentialItemExceedsBudget,
    /// Canonical packet serialization failed.
    SerializationFailed,
}

/// Inclusive context limits for one exact token counter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextCompositionBudget {
    /// Maximum disclosed UTF-8 bytes.
    pub max_bytes: u64,
    /// Maximum disclosed tokens.
    pub max_tokens: u32,
    /// Maximum admitted context items or model messages.
    pub max_items: u32,
    /// Exact pinned token-counter identity.
    pub token_counter_id: String,
}

/// Composes one deterministic, bounded packet and content-free debug manifest.
pub fn compose_context(
    context_packet_id: ContextPacketId,
    budget: &ContextCompositionBudget,
    candidates: Vec<ContextItemCandidate>,
) -> Result<ComposedContextPacket, ContextCompositionError> {
    validate_composition_input(&context_packet_id, budget, &candidates)?;
    let mut ordered = candidates;
    ordered.sort_by_key(|candidate| {
        (
            !candidate.authoritative_evidence,
            !candidate.essential,
            priority(candidate.kind),
            candidate.item_id.clone(),
        )
    });

    let mut canonical_items = BTreeMap::new();
    let mut disposition = BTreeMap::new();
    for candidate in &ordered {
        match candidate.admission {
            ContextAdmission::Denied => {
                disposition.insert(candidate.item_id.clone(), ContextOmissionReason::Denied);
                continue;
            }
            ContextAdmission::Stale => {
                disposition.insert(candidate.item_id.clone(), ContextOmissionReason::Stale);
                continue;
            }
            ContextAdmission::Eligible => {}
        }
        let key = (
            candidate.source_id.as_str(),
            candidate.source_revision.as_str(),
            candidate.content_sha256.as_str(),
        );
        if let std::collections::btree_map::Entry::Vacant(entry) = canonical_items.entry(key) {
            entry.insert(candidate.item_id.as_str());
        } else {
            disposition.insert(candidate.item_id.clone(), ContextOmissionReason::Duplicate);
        }
    }

    let mut items = Vec::new();
    let mut used_bytes = 0_u64;
    let mut used_tokens = 0_u32;
    for candidate in &ordered {
        if disposition.contains_key(&candidate.item_id) {
            continue;
        }
        let bytes = u64::try_from(candidate.bounded_excerpt.len())
            .map_err(|_| ContextCompositionError::InvalidInput)?;
        let next_bytes = used_bytes.checked_add(bytes);
        let next_tokens = used_tokens.checked_add(candidate.token_count);
        let fits = next_bytes.is_some_and(|value| value <= budget.max_bytes)
            && next_tokens.is_some_and(|value| value <= budget.max_tokens)
            && items.len() < budget.max_items as usize;
        if !fits {
            if candidate.essential {
                return Err(ContextCompositionError::EssentialItemExceedsBudget);
            }
            disposition.insert(candidate.item_id.clone(), ContextOmissionReason::Budget);
            continue;
        }
        used_bytes = next_bytes.expect("checked byte total");
        used_tokens = next_tokens.expect("checked token total");
        items.push(candidate.clone());
    }

    let by_id: BTreeMap<_, _> = ordered
        .iter()
        .map(|candidate| (candidate.item_id.as_str(), candidate))
        .collect();
    let accounting = by_id
        .values()
        .map(|candidate| {
            let omission = disposition.get(&candidate.item_id).copied();
            ContextItemAccounting {
                item_id: candidate.item_id.clone(),
                kind: candidate.kind,
                sensitivity: candidate.sensitivity,
                source_id: candidate.source_id.clone(),
                source_revision: candidate.source_revision.clone(),
                content_sha256: candidate.content_sha256.clone(),
                byte_count: candidate.bounded_excerpt.len() as u64,
                token_count: candidate.token_count,
                included: omission.is_none(),
                omission,
            }
        })
        .collect();
    let mut packet = ComposedContextPacket {
        schema_version: CONTRACT_SCHEMA_VERSION,
        context_packet_id,
        max_bytes: budget.max_bytes,
        max_tokens: budget.max_tokens,
        token_counter_id: budget.token_counter_id.clone(),
        items,
        accounting,
        used_bytes,
        used_tokens,
        packet_sha256: ZERO_SHA256.to_owned(),
    };
    packet.packet_sha256 = packet_digest(&packet)?;
    verify_composed_context(&packet)?;
    Ok(packet)
}

/// Verifies a composed packet's canonical digest, bounds, ordering, and complete accounting.
pub fn verify_composed_context(
    packet: &ComposedContextPacket,
) -> Result<(), ContextCompositionError> {
    if packet.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_id(packet.context_packet_id.as_str())
        || packet.max_bytes == 0
        || packet.max_tokens == 0
        || !bounded_text(&packet.token_counter_id)
        || packet.items.len() > MAX_CONTEXT_ITEMS
        || packet.accounting.len() > MAX_CONTEXT_ITEMS
        || packet.items.len() > packet.accounting.len()
        || packet.used_bytes > packet.max_bytes
        || packet.used_tokens > packet.max_tokens
        || !valid_sha256(&packet.packet_sha256)
    {
        return Err(ContextCompositionError::InvalidInput);
    }

    validate_composition_input(
        &packet.context_packet_id,
        &ContextCompositionBudget {
            max_bytes: packet.max_bytes,
            max_tokens: packet.max_tokens,
            max_items: MAX_CONTEXT_ITEMS as u32,
            token_counter_id: packet.token_counter_id.clone(),
        },
        &packet.items,
    )?;

    if packet
        .items
        .windows(2)
        .any(|pair| context_order_key(&pair[0]) > context_order_key(&pair[1]))
        || packet
            .accounting
            .windows(2)
            .any(|pair| pair[0].item_id >= pair[1].item_id)
    {
        return Err(ContextCompositionError::InvalidInput);
    }

    let items = packet
        .items
        .iter()
        .map(|item| (item.item_id.as_str(), item))
        .collect::<BTreeMap<_, _>>();
    let mut included = 0_usize;
    let mut used_bytes = 0_u64;
    let mut used_tokens = 0_u32;
    for accounting in &packet.accounting {
        if !valid_id(&accounting.item_id)
            || !bounded_text(&accounting.source_id)
            || !bounded_text(&accounting.source_revision)
            || !valid_sha256(&accounting.content_sha256)
            || accounting.byte_count > MAX_EXCERPT_BYTES as u64
            || (accounting.byte_count == 0) != (accounting.token_count == 0)
            || accounting.included == accounting.omission.is_some()
        {
            return Err(ContextCompositionError::InvalidInput);
        }
        if accounting.included {
            let item = items
                .get(accounting.item_id.as_str())
                .ok_or(ContextCompositionError::InvalidInput)?;
            if item.kind != accounting.kind
                || item.sensitivity != accounting.sensitivity
                || item.source_id != accounting.source_id
                || item.source_revision != accounting.source_revision
                || item.content_sha256 != accounting.content_sha256
                || item.bounded_excerpt.len() as u64 != accounting.byte_count
                || item.token_count != accounting.token_count
                || item.admission != ContextAdmission::Eligible
            {
                return Err(ContextCompositionError::InvalidInput);
            }
            included += 1;
            used_bytes = used_bytes
                .checked_add(accounting.byte_count)
                .ok_or(ContextCompositionError::InvalidInput)?;
            used_tokens = used_tokens
                .checked_add(accounting.token_count)
                .ok_or(ContextCompositionError::InvalidInput)?;
        } else if items.contains_key(accounting.item_id.as_str()) {
            return Err(ContextCompositionError::InvalidInput);
        }
    }
    if included != packet.items.len()
        || used_bytes != packet.used_bytes
        || used_tokens != packet.used_tokens
        || packet.packet_sha256 != packet_digest(packet)?
    {
        return Err(ContextCompositionError::InvalidInput);
    }
    Ok(())
}

fn context_order_key(candidate: &ContextItemCandidate) -> (bool, bool, u8, &str) {
    (
        !candidate.authoritative_evidence,
        !candidate.essential,
        priority(candidate.kind),
        candidate.item_id.as_str(),
    )
}

fn validate_composition_input(
    packet_id: &ContextPacketId,
    budget: &ContextCompositionBudget,
    candidates: &[ContextItemCandidate],
) -> Result<(), ContextCompositionError> {
    if !valid_id(packet_id.as_str())
        || budget.max_bytes == 0
        || budget.max_tokens == 0
        || budget.max_items == 0
        || budget.max_items as usize > MAX_CONTEXT_ITEMS
        || !bounded_text(&budget.token_counter_id)
        || candidates.len() > MAX_CONTEXT_ITEMS
    {
        return Err(ContextCompositionError::InvalidInput);
    }
    let mut identities = BTreeMap::new();
    for candidate in candidates {
        if !valid_id(&candidate.item_id)
            || !bounded_text(&candidate.source_id)
            || !bounded_text(&candidate.source_revision)
            || !valid_sha256(&candidate.content_sha256)
            || candidate.bounded_excerpt.len() > MAX_EXCERPT_BYTES
            || (candidate.bounded_excerpt.is_empty() != (candidate.token_count == 0))
            || (candidate.authoritative_evidence && candidate.kind != ContextItemKind::Evidence)
        {
            return Err(ContextCompositionError::InvalidInput);
        }
        let identity_material = (
            candidate.kind,
            candidate.source_id.as_str(),
            candidate.source_revision.as_str(),
            candidate.content_sha256.as_str(),
            candidate.bounded_excerpt.as_str(),
        );
        if identities
            .insert(candidate.item_id.as_str(), identity_material)
            .is_some()
        {
            return Err(ContextCompositionError::ConflictingIdentity);
        }
    }
    Ok(())
}

fn packet_digest(packet: &ComposedContextPacket) -> Result<String, ContextCompositionError> {
    let mut candidate = packet.clone();
    candidate.packet_sha256 = ZERO_SHA256.to_owned();
    let bytes =
        to_canonical_json(&candidate).map_err(|_| ContextCompositionError::SerializationFailed)?;
    Ok(hex(&Sha256::digest(bytes)))
}

const fn priority(kind: ContextItemKind) -> u8 {
    match kind {
        ContextItemKind::Instruction => 0,
        ContextItemKind::NewestRequest => 1,
        ContextItemKind::Correction => 2,
        ContextItemKind::Approval => 3,
        ContextItemKind::ActiveObjective => 4,
        ContextItemKind::PlanStep => 5,
        ContextItemKind::Blocker => 6,
        ContextItemKind::Evidence => 7,
        ContextItemKind::ExpectedOutput => 8,
        ContextItemKind::Memory => 9,
        ContextItemKind::Supporting => 10,
    }
}

/// Stable reason a checked summary failed validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckedSummaryError {
    /// The summary has malformed, duplicated, oversized, or unsupported fields.
    InvalidSummary,
}

/// Required treatment of one checked summary for the next requested use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SummaryUseDecision {
    /// The checked summary may provide non-authoritative continuity context.
    UseSummary,
    /// Original sources must be reopened before the summary may support the next result.
    ReopenOriginalSources,
}

/// Validates a checked summary and returns its fail-closed source-use decision.
pub fn evaluate_checked_summary(
    summary: &CheckedContextSummary,
) -> Result<SummaryUseDecision, CheckedSummaryError> {
    if summary.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_id(summary.summary_id.as_str())
        || summary.summary.len() > MAX_SUMMARY_BYTES
        || summary.summary.is_empty()
        || !valid_sha256(&summary.source_set_sha256)
        || !valid_unique_texts(&summary.paths)
        || !valid_unique_texts(&summary.errors)
        || !valid_unique_texts(&summary.identifiers)
        || !valid_unique_texts(&summary.commands)
        || !valid_unique_texts(&summary.decisions)
        || !valid_unique_texts(&summary.unresolved_questions)
        || !valid_unique_ids(summary.evidence_ids.iter().map(|value| value.as_str()))
        || !valid_unique_texts(&summary.citation_ids)
        || !valid_unique_ids(summary.receipt_ids.iter().map(|value| value.as_str()))
    {
        return Err(CheckedSummaryError::InvalidSummary);
    }
    Ok(match summary.state {
        CheckedSummaryState::Current => SummaryUseDecision::UseSummary,
        CheckedSummaryState::Stale
        | CheckedSummaryState::Disputed
        | CheckedSummaryState::Insufficient => SummaryUseDecision::ReopenOriginalSources,
    })
}

/// Stable checkpoint validation failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckpointError {
    /// Required checkpoint identities, bounds, or field relationships are invalid.
    InvalidCheckpoint,
    /// The retained checkpoint digest does not match canonical content.
    DigestMismatch,
    /// An ephemeral checkpoint was offered to a durable persistence boundary.
    EphemeralPersistenceProhibited,
    /// Canonical checkpoint serialization failed.
    SerializationFailed,
}

/// Finalizes one metadata-only checkpoint candidate with its canonical digest.
pub fn finalize_checkpoint(
    mut checkpoint: SessionCheckpoint,
) -> Result<SessionCheckpoint, CheckpointError> {
    checkpoint
        .files
        .sort_by(|left, right| left.object_id.cmp(&right.object_id));
    checkpoint
        .evidence_ids
        .sort_by(|left, right| left.as_str().cmp(right.as_str()));
    checkpoint.blockers.sort();
    checkpoint.checkpoint_sha256 = ZERO_SHA256.to_owned();
    validate_checkpoint_shape(&checkpoint)?;
    checkpoint.checkpoint_sha256 = checkpoint_digest(&checkpoint)?;
    Ok(checkpoint)
}

/// Verifies checkpoint shape and canonical digest before use.
pub fn verify_checkpoint(checkpoint: &SessionCheckpoint) -> Result<(), CheckpointError> {
    validate_checkpoint_shape(checkpoint)?;
    let digest = checkpoint_digest(checkpoint)?;
    if checkpoint.checkpoint_sha256 != digest {
        return Err(CheckpointError::DigestMismatch);
    }
    Ok(())
}

fn validate_checkpoint_shape(checkpoint: &SessionCheckpoint) -> Result<(), CheckpointError> {
    if checkpoint.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_id(checkpoint.checkpoint_id.as_str())
        || !valid_id(checkpoint.session_id.as_str())
        || !valid_id(checkpoint.task_id.as_str())
        || !valid_sha256(&checkpoint.objective_sha256)
        || !valid_id(checkpoint.plan_id.as_str())
        || checkpoint.plan_revision == 0
        || !valid_id(checkpoint.plan_step_id.as_str())
        || !valid_sha256(&checkpoint.next_action_sha256)
        || !valid_id(checkpoint.workspace_id.as_str())
        || !valid_sha256(&checkpoint.workspace_state_sha256)
        || !valid_id(checkpoint.repository_snapshot_id.as_str())
        || !bounded_text(&checkpoint.repository_branch)
        || !valid_sha256(&checkpoint.repository_map_sha256)
        || !valid_sha256(&checkpoint.instruction_sha256)
        || !valid_id(&checkpoint.permission_profile_id)
        || !valid_sha256(&checkpoint.permission_profile_sha256)
        || !valid_id(checkpoint.policy_id.as_str())
        || !valid_sha256(&checkpoint.policy_sha256)
        || !valid_id(checkpoint.model_profile_id.as_str())
        || !valid_sha256(&checkpoint.model_manifest_sha256)
        || !valid_sha256(&checkpoint.model_runtime_sha256)
        || !valid_unique_ids(checkpoint.evidence_ids.iter().map(|value| value.as_str()))
        || !valid_sha256(&checkpoint.citation_set_sha256)
        || !valid_unique_texts(&checkpoint.blockers)
        || !valid_sha256(&checkpoint.context_packet_sha256)
        || checkpoint.files.len() > MAX_REFERENCE_ITEMS
        || !valid_checkpoint_files(&checkpoint.files)
        || checkpoint.action_id.is_some() != checkpoint.action_state.is_some()
        || checkpoint.receipt_id.is_some() != checkpoint.receipt_sha256.is_some()
        || checkpoint
            .receipt_sha256
            .as_deref()
            .is_some_and(|value| !valid_sha256(value))
        || checkpoint
            .action_id
            .as_ref()
            .is_some_and(|value| !valid_id(value.as_str()))
        || checkpoint
            .consumed_grant_id
            .as_ref()
            .is_some_and(|value| !valid_id(value.as_str()))
        || checkpoint
            .receipt_id
            .as_ref()
            .is_some_and(|value| !valid_id(value.as_str()))
        || !valid_sha256(&checkpoint.checkpoint_sha256)
    {
        return Err(CheckpointError::InvalidCheckpoint);
    }
    Ok(())
}

fn valid_checkpoint_files(files: &[CheckpointFileIdentity]) -> bool {
    let mut identities = BTreeSet::new();
    files.iter().all(|file| {
        bounded_text(&file.object_id)
            && valid_sha256(&file.content_sha256)
            && bounded_text(&file.observed_revision)
            && identities.insert(file.object_id.as_str())
    })
}

fn file_map(files: &[CheckpointFileIdentity]) -> BTreeMap<&str, (&str, &str)> {
    files
        .iter()
        .map(|file| {
            (
                file.object_id.as_str(),
                (
                    file.content_sha256.as_str(),
                    file.observed_revision.as_str(),
                ),
            )
        })
        .collect()
}

fn checkpoint_digest(checkpoint: &SessionCheckpoint) -> Result<String, CheckpointError> {
    let mut candidate = checkpoint.clone();
    candidate.checkpoint_sha256 = ZERO_SHA256.to_owned();
    let bytes = to_canonical_json(&candidate).map_err(|_| CheckpointError::SerializationFailed)?;
    Ok(hex(&Sha256::digest(bytes)))
}

/// Current metadata observations supplied before one resume decision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResumeObservation {
    /// Active workspace identity.
    pub workspace_id: String,
    /// Digest of the current workspace capture.
    pub workspace_state_sha256: String,
    /// Current required-file identities.
    pub files: Vec<CheckpointFileIdentity>,
    /// Digest of current effective instructions.
    pub instruction_sha256: String,
    /// Current branch or detached-head marker.
    pub repository_branch: String,
    /// Current repository-map digest.
    pub repository_map_sha256: String,
    /// Current citation-set digest.
    pub citation_set_sha256: String,
    /// Current model profile identity.
    pub model_profile_id: String,
    /// Current model manifest digest.
    pub model_manifest_sha256: String,
    /// Current model runtime digest.
    pub model_runtime_sha256: String,
    /// Current permission-profile identity.
    pub permission_profile_id: String,
    /// Current permission-profile digest.
    pub permission_profile_sha256: String,
    /// Current policy identity.
    pub policy_id: String,
    /// Current policy digest.
    pub policy_sha256: String,
}

/// Fail-closed result of comparing a checkpoint to current observations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResumeDirective {
    /// Every required identity still matches; another action may proceed after authority recovery.
    Continue,
    /// Material drift requires one explicit user decision before any action.
    DecisionRequired {
        /// Exact sorted dimensions that changed.
        drift: Vec<ResumeDriftDimension>,
    },
}

/// Outcome of an explicit choice after material drift.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DriftDecisionOutcome {
    /// Changed state must be accepted through a new checkpoint before continuing.
    RecheckpointRequired,
    /// Existing plan state must be discarded and reconstructed from current observations.
    RestartRequired,
    /// Interrupted work is cancelled.
    Cancelled,
}

/// Detects every material Sprint 22 drift dimension before another action.
pub fn revalidate_resume(
    checkpoint: &SessionCheckpoint,
    current: &ResumeObservation,
) -> Result<ResumeDirective, CheckpointError> {
    verify_checkpoint(checkpoint)?;
    let mut drift = BTreeSet::new();
    if checkpoint.workspace_id.as_str() != current.workspace_id
        || checkpoint.workspace_state_sha256 != current.workspace_state_sha256
    {
        drift.insert(ResumeDriftDimension::Workspace);
    }
    if file_map(&checkpoint.files) != file_map(&current.files) {
        drift.insert(ResumeDriftDimension::File);
    }
    if checkpoint.instruction_sha256 != current.instruction_sha256 {
        drift.insert(ResumeDriftDimension::Instruction);
    }
    if checkpoint.repository_branch != current.repository_branch {
        drift.insert(ResumeDriftDimension::Branch);
    }
    if checkpoint.repository_map_sha256 != current.repository_map_sha256 {
        drift.insert(ResumeDriftDimension::RepositoryMap);
    }
    if checkpoint.citation_set_sha256 != current.citation_set_sha256 {
        drift.insert(ResumeDriftDimension::Citation);
    }
    if checkpoint.model_profile_id.as_str() != current.model_profile_id
        || checkpoint.model_manifest_sha256 != current.model_manifest_sha256
        || checkpoint.model_runtime_sha256 != current.model_runtime_sha256
    {
        drift.insert(ResumeDriftDimension::Model);
    }
    if checkpoint.permission_profile_id != current.permission_profile_id
        || checkpoint.permission_profile_sha256 != current.permission_profile_sha256
    {
        drift.insert(ResumeDriftDimension::Permission);
    }
    if checkpoint.policy_id.as_str() != current.policy_id
        || checkpoint.policy_sha256 != current.policy_sha256
    {
        drift.insert(ResumeDriftDimension::Policy);
    }
    Ok(if drift.is_empty() {
        ResumeDirective::Continue
    } else {
        ResumeDirective::DecisionRequired {
            drift: drift.into_iter().collect(),
        }
    })
}

/// Applies one explicit drift decision without turning it into action authority.
#[must_use]
pub const fn apply_drift_decision(decision: ResumeDriftDecision) -> DriftDecisionOutcome {
    match decision {
        ResumeDriftDecision::Continue => DriftDecisionOutcome::RecheckpointRequired,
        ResumeDriftDecision::Restart => DriftDecisionOutcome::RestartRequired,
        ResumeDriftDecision::Cancel => DriftDecisionOutcome::Cancelled,
    }
}

fn valid_unique_texts(values: &[String]) -> bool {
    values.len() <= MAX_REFERENCE_ITEMS
        && values.iter().all(|value| bounded_text(value))
        && values.iter().collect::<BTreeSet<_>>().len() == values.len()
}

fn valid_unique_ids<'a>(values: impl Iterator<Item = &'a str>) -> bool {
    let values = values.collect::<Vec<_>>();
    values.len() <= MAX_REFERENCE_ITEMS
        && values.iter().all(|value| valid_id(value))
        && values.iter().copied().collect::<BTreeSet<_>>().len() == values.len()
}

fn valid_id(value: &str) -> bool {
    bounded_text(value)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.'))
}

fn bounded_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_ID_BYTES && !value.chars().any(char::is_control)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        ActionId, ActionState, CONTRACT_SCHEMA_VERSION, CheckedContextSummary, CheckedSummaryState,
        CheckpointFileIdentity, ComposedContextPacket, ContextAdmission, ContextItemCandidate,
        ContextItemKind, ContextOmissionReason, ContextPacketId, ContextSensitivity,
        ContextSummaryId, EvidenceId, GrantId, ModelProfileId, PlanId, PlanStepId, PolicyId,
        ReceiptId, RepositorySnapshotId, ResumeDriftDecision, ResumeDriftDimension,
        SessionCheckpoint, SessionCheckpointId, SessionId, TaskId, WorkspaceId,
    };

    use super::{
        CheckedSummaryError, CheckpointError, ContextCompositionBudget, ContextCompositionError,
        DriftDecisionOutcome, ResumeDirective, ResumeObservation, SummaryUseDecision,
        apply_drift_decision, compose_context, evaluate_checked_summary, finalize_checkpoint,
        revalidate_resume, verify_checkpoint, verify_composed_context,
    };

    type ContextMutation = Box<dyn Fn(&mut ComposedContextPacket)>;

    fn hash(byte: char) -> String {
        byte.to_string().repeat(64)
    }

    fn item(id: &str, kind: ContextItemKind, excerpt: &str, tokens: u32) -> ContextItemCandidate {
        ContextItemCandidate {
            item_id: id.to_owned(),
            kind,
            sensitivity: ContextSensitivity::Internal,
            admission: ContextAdmission::Eligible,
            authoritative_evidence: kind == ContextItemKind::Evidence,
            essential: matches!(
                kind,
                ContextItemKind::Instruction
                    | ContextItemKind::NewestRequest
                    | ContextItemKind::Correction
                    | ContextItemKind::PlanStep
            ),
            source_id: format!("source-{id}"),
            source_revision: "revision-1".to_owned(),
            content_sha256: hash('a'),
            bounded_excerpt: excerpt.to_owned(),
            token_count: tokens,
        }
    }

    fn budget(bytes: u64, tokens: u32) -> ContextCompositionBudget {
        ContextCompositionBudget {
            max_bytes: bytes,
            max_tokens: tokens,
            max_items: 32,
            token_counter_id: "counter-v1".to_owned(),
        }
    }

    #[test]
    fn s020_ut01_context_is_deterministic_bounded_and_fully_accounted() {
        let mut supporting = item("supporting", ContextItemKind::Supporting, "later", 2);
        supporting.essential = false;
        let candidates = vec![
            supporting,
            item("request", ContextItemKind::NewestRequest, "do this", 2),
            item(
                "instruction",
                ContextItemKind::Instruction,
                "follow policy",
                2,
            ),
        ];
        let first = compose_context(
            ContextPacketId::from_raw("context-1"),
            &budget(20, 4),
            candidates.clone(),
        )
        .expect("valid packet");
        let second = compose_context(
            ContextPacketId::from_raw("context-1"),
            &budget(20, 4),
            candidates.into_iter().rev().collect(),
        )
        .expect("order independent packet");
        assert_eq!(first, second);
        assert_eq!(first.used_tokens, 4);
        assert!(first.used_bytes <= first.max_bytes);
        assert_eq!(first.items[0].kind, ContextItemKind::Instruction);
        assert_eq!(first.items[1].kind, ContextItemKind::NewestRequest);
        assert_eq!(first.accounting.len(), 3);
        assert!(first.accounting.iter().any(|entry| {
            entry.item_id == "supporting"
                && !entry.included
                && entry.omission == Some(agentmage_kernel_contracts::ContextOmissionReason::Budget)
        }));
        verify_composed_context(&first).expect("canonical context verifies");
    }

    #[test]
    fn canonical_context_verifier_rejects_content_accounting_order_and_digest_drift() {
        let packet = compose_context(
            ContextPacketId::from_raw("context-verified"),
            &budget(64, 16),
            vec![
                item("request", ContextItemKind::NewestRequest, "do this", 2),
                item("evidence", ContextItemKind::Evidence, "observed", 2),
            ],
        )
        .expect("valid packet");
        let mutations: Vec<ContextMutation> = vec![
            Box::new(|value| value.items[0].bounded_excerpt.push_str(" changed")),
            Box::new(|value| value.accounting[0].byte_count += 1),
            Box::new(|value| value.items.swap(0, 1)),
            Box::new(|value| value.packet_sha256 = hash('f')),
        ];
        for mutate in mutations {
            let mut changed = packet.clone();
            mutate(&mut changed);
            assert_eq!(
                verify_composed_context(&changed),
                Err(ContextCompositionError::InvalidInput)
            );
        }
    }

    #[test]
    fn s020_ut01_deduplicates_summary_before_authoritative_evidence() {
        let mut summary = item("summary", ContextItemKind::Memory, "same", 1);
        summary.source_id = "source-shared".to_owned();
        summary.essential = true;
        let mut evidence = item("evidence", ContextItemKind::Evidence, "same", 1);
        evidence.source_id = "source-shared".to_owned();
        let packet = compose_context(
            ContextPacketId::from_raw("context-2"),
            &budget(16, 4),
            vec![summary, evidence],
        )
        .expect("authoritative candidate wins");
        assert_eq!(packet.items.len(), 1);
        assert_eq!(packet.items[0].item_id, "evidence");
    }

    #[test]
    fn s020_ut01_denied_stale_and_secret_canary_never_enter_packet_content() {
        let mut denied = item(
            "denied",
            ContextItemKind::Supporting,
            "SPRINT22_SECRET_CANARY",
            2,
        );
        denied.admission = ContextAdmission::Denied;
        denied.sensitivity = ContextSensitivity::Restricted;
        denied.essential = false;
        let mut stale = item("stale", ContextItemKind::Evidence, "stale material", 2);
        stale.admission = ContextAdmission::Stale;
        stale.essential = false;
        let packet = compose_context(
            ContextPacketId::from_raw("context-3"),
            &budget(100, 20),
            vec![denied, stale],
        )
        .expect("omissions are visible");
        let encoded = serde_json::to_string(&packet).expect("packet encodes");
        assert!(!encoded.contains("SPRINT22_SECRET_CANARY"));
        assert!(encoded.contains("restricted"));
        assert!(encoded.contains("denied"));
        assert!(encoded.contains("stale"));
    }

    #[test]
    fn s020_ut01_empty_boundary_and_over_limit_cases_fail_or_account_exactly() {
        let empty = compose_context(
            ContextPacketId::from_raw("context-empty"),
            &budget(1, 1),
            Vec::new(),
        )
        .expect("empty is valid");
        assert!(empty.items.is_empty());
        assert!(empty.accounting.is_empty());

        let essential = item("request", ContextItemKind::NewestRequest, "too large", 5);
        assert_eq!(
            compose_context(
                ContextPacketId::from_raw("context-over"),
                &budget(2, 2),
                vec![essential]
            ),
            Err(ContextCompositionError::EssentialItemExceedsBudget)
        );

        let duplicate = item("same-id", ContextItemKind::Supporting, "x", 1);
        assert_eq!(
            compose_context(
                ContextPacketId::from_raw("context-duplicate"),
                &budget(10, 10),
                vec![duplicate.clone(), duplicate]
            ),
            Err(ContextCompositionError::ConflictingIdentity)
        );
    }

    #[test]
    fn s020_ut01_item_ceiling_omits_optional_content_and_rejects_essential_overflow() {
        let mut bounded = budget(1_024, 128);
        bounded.max_items = 1;
        let mut optional = item("supporting", ContextItemKind::Supporting, "optional", 1);
        optional.essential = false;
        let packet = compose_context(
            ContextPacketId::from_raw("context-item-limit"),
            &bounded,
            vec![
                item("request", ContextItemKind::NewestRequest, "required", 1),
                optional,
            ],
        )
        .expect("optional item is omitted");
        assert_eq!(packet.items.len(), 1);
        assert!(packet.accounting.iter().any(|entry| {
            entry.item_id == "supporting"
                && entry.omission == Some(ContextOmissionReason::Budget)
        }));

        let mut second_required = item(
            "second-required",
            ContextItemKind::ExpectedOutput,
            "required too",
            1,
        );
        second_required.essential = true;
        assert_eq!(
            compose_context(
                ContextPacketId::from_raw("context-essential-item-limit"),
                &bounded,
                vec![
                    item("request", ContextItemKind::NewestRequest, "required", 1),
                    second_required,
                ],
            ),
            Err(ContextCompositionError::EssentialItemExceedsBudget)
        );
    }

    fn summary(state: CheckedSummaryState) -> CheckedContextSummary {
        CheckedContextSummary {
            schema_version: CONTRACT_SCHEMA_VERSION,
            summary_id: ContextSummaryId::from_raw("summary-1"),
            state,
            summary: "Continue from the verified checkpoint.".to_owned(),
            paths: vec!["kernel/engine/src/context_management.rs".to_owned()],
            errors: vec!["context.none".to_owned()],
            identifiers: vec!["task-1".to_owned()],
            commands: vec!["cargo test -p agentmage-kernel-engine".to_owned()],
            decisions: vec!["Use exact checkpoint identity".to_owned()],
            unresolved_questions: vec!["Native evidence pending".to_owned()],
            evidence_ids: vec![EvidenceId::from_raw("evidence-1")],
            citation_ids: vec!["citation-1".to_owned()],
            receipt_ids: vec![ReceiptId::from_raw("receipt-1")],
            source_set_sha256: hash('b'),
        }
    }

    #[test]
    fn s020_i03_i04_checked_summaries_preserve_references_and_reopen_sources() {
        assert_eq!(
            evaluate_checked_summary(&summary(CheckedSummaryState::Current)),
            Ok(SummaryUseDecision::UseSummary)
        );
        for state in [
            CheckedSummaryState::Stale,
            CheckedSummaryState::Disputed,
            CheckedSummaryState::Insufficient,
        ] {
            assert_eq!(
                evaluate_checked_summary(&summary(state)),
                Ok(SummaryUseDecision::ReopenOriginalSources)
            );
        }
        let mut invalid = summary(CheckedSummaryState::Current);
        invalid.paths.push(invalid.paths[0].clone());
        assert_eq!(
            evaluate_checked_summary(&invalid),
            Err(CheckedSummaryError::InvalidSummary)
        );
    }

    fn checkpoint() -> SessionCheckpoint {
        finalize_checkpoint(SessionCheckpoint {
            schema_version: CONTRACT_SCHEMA_VERSION,
            checkpoint_id: SessionCheckpointId::from_raw("checkpoint-1"),
            session_id: SessionId::from_raw("session-1"),
            task_id: TaskId::from_raw("task-1"),
            objective_sha256: hash('1'),
            plan_id: PlanId::from_raw("plan-1"),
            plan_revision: 1,
            plan_step_id: PlanStepId::from_raw("step-1"),
            next_action_sha256: hash('2'),
            workspace_id: WorkspaceId::from_raw("workspace-1"),
            workspace_state_sha256: hash('3'),
            repository_snapshot_id: RepositorySnapshotId::from_raw("repository-1"),
            repository_branch: "main".to_owned(),
            repository_map_sha256: hash('4'),
            files: vec![CheckpointFileIdentity {
                object_id: "file-1".to_owned(),
                content_sha256: hash('5'),
                observed_revision: "revision-1".to_owned(),
            }],
            instruction_sha256: hash('6'),
            permission_profile_id: "permission-1".to_owned(),
            permission_profile_sha256: hash('7'),
            policy_id: PolicyId::from_raw("policy-1"),
            policy_sha256: hash('8'),
            model_profile_id: ModelProfileId::from_raw("model-1"),
            model_manifest_sha256: hash('9'),
            model_runtime_sha256: hash('a'),
            evidence_ids: vec![EvidenceId::from_raw("evidence-1")],
            citation_set_sha256: hash('b'),
            blockers: vec!["native-evidence-pending".to_owned()],
            context_packet_sha256: hash('c'),
            action_id: Some(ActionId::from_raw("action-1")),
            action_state: Some(ActionState::Succeeded),
            consumed_grant_id: Some(GrantId::from_raw("grant-1")),
            receipt_id: Some(ReceiptId::from_raw("receipt-1")),
            receipt_sha256: Some(hash('d')),
            ephemeral: false,
            checkpoint_sha256: hash('0'),
        })
        .expect("valid checkpoint")
    }

    fn observation(checkpoint: &SessionCheckpoint) -> ResumeObservation {
        ResumeObservation {
            workspace_id: checkpoint.workspace_id.as_str().to_owned(),
            workspace_state_sha256: checkpoint.workspace_state_sha256.clone(),
            files: checkpoint.files.clone(),
            instruction_sha256: checkpoint.instruction_sha256.clone(),
            repository_branch: checkpoint.repository_branch.clone(),
            repository_map_sha256: checkpoint.repository_map_sha256.clone(),
            citation_set_sha256: checkpoint.citation_set_sha256.clone(),
            model_profile_id: checkpoint.model_profile_id.as_str().to_owned(),
            model_manifest_sha256: checkpoint.model_manifest_sha256.clone(),
            model_runtime_sha256: checkpoint.model_runtime_sha256.clone(),
            permission_profile_id: checkpoint.permission_profile_id.clone(),
            permission_profile_sha256: checkpoint.permission_profile_sha256.clone(),
            policy_id: checkpoint.policy_id.as_str().to_owned(),
            policy_sha256: checkpoint.policy_sha256.clone(),
        }
    }

    #[test]
    fn s020_ut02_checkpoint_hash_and_schema_fail_closed() {
        let valid = checkpoint();
        assert_eq!(verify_checkpoint(&valid), Ok(()));
        let mut corrupt = valid.clone();
        corrupt.repository_branch = "changed".to_owned();
        assert_eq!(
            verify_checkpoint(&corrupt),
            Err(CheckpointError::DigestMismatch)
        );
        let mut future = valid.clone();
        future.schema_version += 1;
        assert_eq!(
            verify_checkpoint(&future),
            Err(CheckpointError::InvalidCheckpoint)
        );
        let mut mismatched = valid;
        mismatched.receipt_sha256 = None;
        assert_eq!(
            finalize_checkpoint(mismatched),
            Err(CheckpointError::InvalidCheckpoint)
        );
    }

    #[test]
    fn s020_ut02_every_material_drift_dimension_is_reported_before_action() {
        let checkpoint = checkpoint();
        assert_eq!(
            revalidate_resume(&checkpoint, &observation(&checkpoint)),
            Ok(ResumeDirective::Continue)
        );
        let cases = [
            (ResumeDriftDimension::Workspace, 0_u8),
            (ResumeDriftDimension::File, 1),
            (ResumeDriftDimension::Instruction, 2),
            (ResumeDriftDimension::Branch, 3),
            (ResumeDriftDimension::RepositoryMap, 4),
            (ResumeDriftDimension::Citation, 5),
            (ResumeDriftDimension::Model, 6),
            (ResumeDriftDimension::Permission, 7),
            (ResumeDriftDimension::Policy, 8),
        ];
        for (expected, case) in cases {
            let mut current = observation(&checkpoint);
            match case {
                0 => current.workspace_state_sha256 = hash('e'),
                1 => current.files[0].content_sha256 = hash('e'),
                2 => current.instruction_sha256 = hash('e'),
                3 => current.repository_branch = "feature".to_owned(),
                4 => current.repository_map_sha256 = hash('e'),
                5 => current.citation_set_sha256 = hash('e'),
                6 => current.model_runtime_sha256 = hash('e'),
                7 => current.permission_profile_sha256 = hash('e'),
                8 => current.policy_sha256 = hash('e'),
                _ => unreachable!(),
            }
            assert_eq!(
                revalidate_resume(&checkpoint, &current),
                Ok(ResumeDirective::DecisionRequired {
                    drift: vec![expected]
                })
            );
        }
    }

    #[test]
    fn s020_i08_all_explicit_drift_choices_are_non_authoritative() {
        assert_eq!(
            apply_drift_decision(ResumeDriftDecision::Continue),
            DriftDecisionOutcome::RecheckpointRequired
        );
        assert_eq!(
            apply_drift_decision(ResumeDriftDecision::Restart),
            DriftDecisionOutcome::RestartRequired
        );
        assert_eq!(
            apply_drift_decision(ResumeDriftDecision::Cancel),
            DriftDecisionOutcome::Cancelled
        );
    }

    #[test]
    fn at_resume_001_revalidates_one_hundred_times_without_mutating_checkpoint() {
        let checkpoint = checkpoint();
        let digest = checkpoint.checkpoint_sha256.clone();
        for _ in 0..100 {
            assert_eq!(
                revalidate_resume(&checkpoint, &observation(&checkpoint)),
                Ok(ResumeDirective::Continue)
            );
        }
        assert_eq!(checkpoint.checkpoint_sha256, digest);
    }
}
