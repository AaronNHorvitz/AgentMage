//! Fresh citation resolution, complete answer ledgers, and keyed receipt integrity.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Write as _};

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, EvidenceReference, MaterialClaimEvidenceAssignment,
    MaterialClaimEvidenceState, Receipt, TaskId, UnknownBlockedReason, WorkspacePath,
    to_canonical_json,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use crate::evidence_state::ValidatedEvidenceAssignments;

const MAX_CITATIONS: usize = 256;
const MAX_CLAIMS: usize = 128;
const MAX_LIMITATIONS: usize = 32;
const MAX_CODE_BYTES: usize = 256;
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Exact bounded selector within one cited source.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CitationSelector {
    /// Half-open UTF-8 byte range in the exact observed file.
    ByteRange {
        /// Inclusive first byte.
        start: u64,
        /// Exclusive terminal byte.
        end: u64,
    },
    /// Stable structured identity emitted by an approved parser or adapter.
    Structured {
        /// Bounded adapter-owned object identity.
        identity: String,
    },
}

/// Exact file identity observed when a citation was created or resolved.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CitationFileIdentity {
    /// Canonical workspace-relative path.
    pub path: WorkspacePath,
    /// Complete file length in bytes.
    pub byte_len: u64,
    /// Lowercase SHA-256 digest of complete file bytes.
    pub content_sha256: String,
    /// Stable platform object identity digest.
    pub object_identity_sha256: String,
    /// Exact repository, document, or adapter revision.
    pub revision: String,
}

/// Citation created at one exact observation point.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceCitation {
    /// Stable citation identity.
    pub citation_id: String,
    /// Existing content-addressed evidence identity.
    pub evidence: EvidenceReference,
    /// Exact selector within the observed source.
    pub selector: CitationSelector,
    /// Monotonic observation point within the owning session.
    pub observed_at_epoch_ms: u64,
    /// Exact file identity at the observation point.
    pub observed_file: CitationFileIdentity,
}

/// Closed currentness result for one citation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CitationFreshness {
    /// Every current identity equals the observed source identity.
    Current,
    /// A current source exists but at least one identity changed.
    Stale,
    /// The observed source is no longer available at its exact path.
    Missing,
}

/// Deterministic resolution of one citation against a caller-held current identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CitationResolution {
    /// Exact citation that was checked.
    pub citation: SourceCitation,
    /// Currentness disposition.
    pub freshness: CitationFreshness,
    /// Stable content-free explanation code.
    pub reason_code: String,
    /// Current identity when the source still exists.
    pub current_file: Option<CitationFileIdentity>,
    /// Hash over the complete resolution record excluding this field.
    pub resolution_sha256: String,
}

/// One content-free citation or ledger validation failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceReconciliationError {
    /// Citation identity, selector, observation, or current source is malformed.
    InvalidCitation,
    /// Claim coverage, state provenance, conflict handling, or limitation data is invalid.
    InvalidAnswerLedger,
    /// Receipt order, identity, digest, or chain is invalid.
    InvalidReceiptChain,
    /// Integrity anchor or secret key is malformed or does not verify.
    InvalidIntegrityAnchor,
}

/// Resolves a citation without reading an ambient path or replacing observed identity.
pub fn resolve_citation(
    citation: SourceCitation,
    current_file: Option<CitationFileIdentity>,
) -> Result<CitationResolution, EvidenceReconciliationError> {
    if !valid_citation(&citation)
        || current_file
            .as_ref()
            .is_some_and(|identity| !valid_file_identity(identity))
    {
        return Err(EvidenceReconciliationError::InvalidCitation);
    }
    let (freshness, reason_code) = match &current_file {
        None => (CitationFreshness::Missing, "citation.source.missing"),
        Some(current) if current == &citation.observed_file => {
            (CitationFreshness::Current, "citation.source.current")
        }
        Some(_) => (CitationFreshness::Stale, "citation.source.stale"),
    };
    let mut resolution = CitationResolution {
        citation,
        freshness,
        reason_code: reason_code.to_owned(),
        current_file,
        resolution_sha256: String::new(),
    };
    resolution.resolution_sha256 = resolution_digest(&resolution);
    Ok(resolution)
}

/// Safe content-free projection of one evidence identity for model or UI use.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SafeCitationProjection {
    /// Stable citation identity.
    pub citation_id: String,
    /// Stable evidence identity.
    pub evidence_id: String,
    /// Complete observed content digest.
    pub content_sha256: String,
    /// Currentness disposition.
    pub freshness: CitationFreshness,
    /// Stable explanation code.
    pub reason_code: String,
}

/// One claim-level record in the complete answer ledger.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnswerClaimEntry {
    /// Validated evidence-state assignment.
    pub assignment: MaterialClaimEvidenceAssignment,
    /// Exact citation resolutions supporting this claim.
    pub citations: Vec<CitationResolution>,
    /// Stable visible limitation codes.
    pub limitation_codes: Vec<String>,
}

/// Complete deterministic claim ledger for one rendered answer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnswerClaimLedger {
    /// Schema version.
    pub schema_version: u16,
    /// Exact owning task.
    pub task_id: TaskId,
    /// Rendered claim identities in answer order.
    pub rendered_claim_ids: Vec<String>,
    /// One state-complete entry per rendered material claim.
    pub entries: Vec<AnswerClaimEntry>,
    /// Hash over every preceding ledger field.
    pub ledger_sha256: String,
}

/// Builds an exact-coverage answer ledger from an opaque validated assignment set.
pub fn build_answer_claim_ledger(
    assignments: ValidatedEvidenceAssignments,
    rendered_claim_ids: Vec<String>,
    evidence: BTreeMap<String, (Vec<CitationResolution>, Vec<String>)>,
) -> Result<AnswerClaimLedger, EvidenceReconciliationError> {
    let task_id = assignments.task_id().clone();
    let assignment_values = assignments.assignments();
    if assignment_values.is_empty()
        || assignment_values.len() > MAX_CLAIMS
        || rendered_claim_ids.len() != assignment_values.len()
        || has_duplicates(rendered_claim_ids.iter().map(String::as_str))
        || evidence.len() != assignment_values.len()
    {
        return Err(EvidenceReconciliationError::InvalidAnswerLedger);
    }
    let by_claim = assignment_values
        .iter()
        .map(|assignment| (assignment.claim.claim_id.as_str(), assignment))
        .collect::<BTreeMap<_, _>>();
    if by_claim.len() != assignment_values.len()
        || rendered_claim_ids
            .iter()
            .any(|claim_id| !by_claim.contains_key(claim_id.as_str()))
    {
        return Err(EvidenceReconciliationError::InvalidAnswerLedger);
    }
    let by_assignment = assignment_values
        .iter()
        .map(|assignment| (assignment.assignment_id.as_str(), assignment))
        .collect::<BTreeMap<_, _>>();
    let mut entries = Vec::with_capacity(rendered_claim_ids.len());
    for claim_id in &rendered_claim_ids {
        let assignment = (*by_claim
            .get(claim_id.as_str())
            .ok_or(EvidenceReconciliationError::InvalidAnswerLedger)?)
        .clone();
        let (citations, limitation_codes) = evidence
            .get(&assignment.assignment_id)
            .cloned()
            .ok_or(EvidenceReconciliationError::InvalidAnswerLedger)?;
        if !valid_limitations(&limitation_codes)
            || !valid_claim_citations(&assignment, &citations, &by_assignment)
        {
            return Err(EvidenceReconciliationError::InvalidAnswerLedger);
        }
        entries.push(AnswerClaimEntry {
            assignment,
            citations,
            limitation_codes,
        });
    }
    let mut ledger = AnswerClaimLedger {
        schema_version: CONTRACT_SCHEMA_VERSION,
        task_id,
        rendered_claim_ids,
        entries,
        ledger_sha256: String::new(),
    };
    ledger.ledger_sha256 = answer_ledger_digest(&ledger);
    Ok(ledger)
}

/// Verifies exact coverage, state provenance, citation freshness, and ledger digest.
#[must_use]
pub fn verify_answer_claim_ledger(ledger: &AnswerClaimLedger) -> bool {
    if ledger.schema_version != CONTRACT_SCHEMA_VERSION
        || ledger.entries.is_empty()
        || ledger.entries.len() != ledger.rendered_claim_ids.len()
        || ledger.ledger_sha256 != answer_ledger_digest(ledger)
        || has_duplicates(ledger.rendered_claim_ids.iter().map(String::as_str))
    {
        return false;
    }
    let by_assignment = ledger
        .entries
        .iter()
        .map(|entry| (entry.assignment.assignment_id.as_str(), &entry.assignment))
        .collect::<BTreeMap<_, _>>();
    ledger.entries.iter().enumerate().all(|(index, entry)| {
        entry.assignment.schema_version == CONTRACT_SCHEMA_VERSION
            && entry.assignment.claim.task_id == ledger.task_id
            && entry.assignment.claim.claim_id == ledger.rendered_claim_ids[index]
            && valid_limitations(&entry.limitation_codes)
            && valid_claim_citations(&entry.assignment, &entry.citations, &by_assignment)
    })
}

/// Produces a bounded projection containing identities, hashes, states, and limitations only.
#[must_use]
pub fn safe_evidence_projection(ledger: &AnswerClaimLedger) -> Vec<SafeCitationProjection> {
    if !verify_answer_claim_ledger(ledger) {
        return Vec::new();
    }
    ledger
        .entries
        .iter()
        .flat_map(|entry| &entry.citations)
        .map(|resolution| SafeCitationProjection {
            citation_id: resolution.citation.citation_id.clone(),
            evidence_id: resolution.citation.evidence.evidence_id.as_str().to_owned(),
            content_sha256: resolution.citation.evidence.content_sha256.clone(),
            freshness: resolution.freshness,
            reason_code: resolution.reason_code.clone(),
        })
        .collect()
}

/// Renders stable claim-level audit lines without source content or model response text.
#[must_use]
pub fn render_claim_audit(ledger: &AnswerClaimLedger) -> String {
    if !verify_answer_claim_ledger(ledger) {
        return String::new();
    }
    let mut output = String::new();
    for entry in &ledger.entries {
        let state = match &entry.assignment.evidence_state {
            MaterialClaimEvidenceState::Observed(_) => "observed",
            MaterialClaimEvidenceState::Derived(_) => "derived",
            MaterialClaimEvidenceState::Inferred(_) => "inferred",
            MaterialClaimEvidenceState::UnknownBlocked(_) => "unknown_blocked",
        };
        writeln!(
            output,
            "claim={} state={} citations={} limitations={}",
            entry.assignment.claim.claim_id,
            state,
            entry.citations.len(),
            entry.limitation_codes.join(",")
        )
        .expect("writing to String cannot fail");
    }
    output
}

/// Append-only receipt sequence with no embedded integrity key or anchor.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TamperEvidentReceiptLedger {
    receipts: Vec<Receipt>,
}

impl TamperEvidentReceiptLedger {
    /// Appends exactly the next valid receipt and rejects reuse, gaps, and chain drift.
    pub fn append(&mut self, receipt: Receipt) -> Result<(), EvidenceReconciliationError> {
        let expected_sequence = u64::try_from(self.receipts.len())
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or(EvidenceReconciliationError::InvalidReceiptChain)?;
        let expected_previous = self
            .receipts
            .last()
            .map_or(ZERO_SHA256, |previous| previous.receipt_sha256.as_str());
        if !valid_receipt(&receipt)
            || receipt.sequence != expected_sequence
            || receipt.previous_receipt_sha256 != expected_previous
            || receipt.receipt_sha256 != receipt_digest(&receipt)
            || self.receipts.iter().any(|existing| {
                existing.receipt_id == receipt.receipt_id
                    || existing.operation_attempt_id == receipt.operation_attempt_id
            })
        {
            return Err(EvidenceReconciliationError::InvalidReceiptChain);
        }
        self.receipts.push(receipt);
        Ok(())
    }

    /// Returns the immutable retained receipt sequence.
    #[must_use]
    pub fn receipts(&self) -> &[Receipt] {
        &self.receipts
    }

    /// Verifies every sequence, digest, previous hash, and unique operation attempt.
    #[must_use]
    pub fn verify(&self) -> bool {
        let mut previous = ZERO_SHA256;
        let mut receipts = BTreeSet::new();
        let mut attempts = BTreeSet::new();
        for (index, receipt) in self.receipts.iter().enumerate() {
            if !valid_receipt(receipt)
                || receipt.sequence != u64::try_from(index).unwrap_or(u64::MAX) + 1
                || receipt.previous_receipt_sha256 != previous
                || receipt.receipt_sha256 != receipt_digest(receipt)
                || !receipts.insert(receipt.receipt_id.as_str())
                || !attempts.insert(receipt.operation_attempt_id.as_str())
            {
                return false;
            }
            previous = &receipt.receipt_sha256;
        }
        true
    }
}

/// Secret anchor key that is never serializable, cloneable, displayable, or ledger-owned.
pub struct ReceiptIntegrityKey {
    bytes: [u8; 32],
}

impl ReceiptIntegrityKey {
    /// Admits one exact 256-bit key supplied by the platform secret boundary.
    pub fn new(bytes: [u8; 32]) -> Result<Self, EvidenceReconciliationError> {
        if bytes == [0; 32] {
            return Err(EvidenceReconciliationError::InvalidIntegrityAnchor);
        }
        Ok(Self { bytes })
    }
}

impl fmt::Debug for ReceiptIntegrityKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ReceiptIntegrityKey([REDACTED])")
    }
}

impl Drop for ReceiptIntegrityKey {
    fn drop(&mut self) {
        self.bytes.zeroize();
    }
}

/// Keyed integrity anchor stored separately from the receipt ledger.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptIntegrityAnchor {
    /// Number of receipts covered.
    pub receipt_count: u64,
    /// Covered terminal chain digest or genesis digest.
    pub head_receipt_sha256: String,
    /// HMAC-SHA-256 over the count and head digest.
    pub anchor_hmac_sha256: String,
}

/// Seals one verified ledger without placing the key or anchor inside it.
pub fn seal_receipt_ledger(
    ledger: &TamperEvidentReceiptLedger,
    key: &ReceiptIntegrityKey,
) -> Result<ReceiptIntegrityAnchor, EvidenceReconciliationError> {
    if !ledger.verify() || ledger.receipts.is_empty() {
        return Err(EvidenceReconciliationError::InvalidReceiptChain);
    }
    let count = u64::try_from(ledger.receipts.len())
        .map_err(|_| EvidenceReconciliationError::InvalidReceiptChain)?;
    let head = ledger
        .receipts
        .last()
        .map(|receipt| receipt.receipt_sha256.clone())
        .ok_or(EvidenceReconciliationError::InvalidReceiptChain)?;
    Ok(ReceiptIntegrityAnchor {
        receipt_count: count,
        head_receipt_sha256: head.clone(),
        anchor_hmac_sha256: hmac_sha256(&key.bytes, &anchor_message(count, &head)),
    })
}

/// Verifies an external keyed anchor against the complete current ledger.
#[must_use]
pub fn verify_receipt_anchor(
    ledger: &TamperEvidentReceiptLedger,
    anchor: &ReceiptIntegrityAnchor,
    key: &ReceiptIntegrityKey,
) -> bool {
    if !ledger.verify() || ledger.receipts.is_empty() {
        return false;
    }
    let Ok(count) = u64::try_from(ledger.receipts.len()) else {
        return false;
    };
    let Some(head) = ledger
        .receipts
        .last()
        .map(|receipt| &receipt.receipt_sha256)
    else {
        return false;
    };
    anchor.receipt_count == count
        && anchor.head_receipt_sha256 == *head
        && valid_sha256(&anchor.anchor_hmac_sha256)
        && constant_time_equal(
            anchor.anchor_hmac_sha256.as_bytes(),
            hmac_sha256(&key.bytes, &anchor_message(count, head)).as_bytes(),
        )
}

fn valid_claim_citations(
    assignment: &MaterialClaimEvidenceAssignment,
    citations: &[CitationResolution],
    assignments: &BTreeMap<&str, &MaterialClaimEvidenceAssignment>,
) -> bool {
    if citations.len() > MAX_CITATIONS
        || has_duplicates(
            citations
                .iter()
                .map(|resolution| resolution.citation.citation_id.as_str()),
        )
        || citations.iter().any(|resolution| {
            resolution.resolution_sha256 != resolution_digest(resolution)
                || !valid_citation(&resolution.citation)
        })
    {
        return false;
    }
    match &assignment.evidence_state {
        MaterialClaimEvidenceState::Observed(provenance) => {
            exact_evidence_ids(&provenance.sources, citations)
                && citations
                    .iter()
                    .all(|value| value.freshness == CitationFreshness::Current)
        }
        MaterialClaimEvidenceState::Derived(provenance) => {
            citations.is_empty()
                && provenance
                    .observed_input_assignment_ids
                    .iter()
                    .all(|identity| {
                        assignments.get(identity.as_str()).is_some_and(|input| {
                            matches!(
                                input.evidence_state,
                                MaterialClaimEvidenceState::Observed(_)
                            )
                        })
                    })
        }
        MaterialClaimEvidenceState::Inferred(provenance) => {
            exact_evidence_ids(&provenance.citations, citations)
                && citations
                    .iter()
                    .all(|value| value.freshness == CitationFreshness::Current)
        }
        MaterialClaimEvidenceState::UnknownBlocked(provenance) => {
            let exact = exact_evidence_ids(&provenance.evidence, citations);
            match provenance.reason {
                UnknownBlockedReason::StaleEvidence => {
                    exact
                        && !citations.is_empty()
                        && citations
                            .iter()
                            .any(|value| value.freshness != CitationFreshness::Current)
                }
                UnknownBlockedReason::Conflict => provenance.evidence.len() >= 2 && exact,
                _ => exact,
            }
        }
    }
}

fn exact_evidence_ids(evidence: &[EvidenceReference], citations: &[CitationResolution]) -> bool {
    evidence.len() == citations.len()
        && evidence.iter().zip(citations).all(|(left, right)| {
            left == &right.citation.evidence && right.resolution_sha256 == resolution_digest(right)
        })
}

fn valid_citation(citation: &SourceCitation) -> bool {
    if !valid_code(&citation.citation_id)
        || citation.observed_at_epoch_ms == 0
        || !valid_file_identity(&citation.observed_file)
        || citation.evidence.content_sha256 != citation.observed_file.content_sha256
        || citation.evidence.observed_revision.as_deref()
            != Some(citation.observed_file.revision.as_str())
    {
        return false;
    }
    match &citation.selector {
        CitationSelector::ByteRange { start, end } => {
            start < end && *end <= citation.observed_file.byte_len
        }
        CitationSelector::Structured { identity } => valid_code(identity),
    }
}

fn valid_file_identity(value: &CitationFileIdentity) -> bool {
    valid_sha256(&value.content_sha256)
        && valid_sha256(&value.object_identity_sha256)
        && valid_code(&value.revision)
}

fn valid_receipt(receipt: &Receipt) -> bool {
    receipt.schema_version == CONTRACT_SCHEMA_VERSION
        && receipt.sequence > 0
        && valid_code(receipt.receipt_id.as_str())
        && valid_code(receipt.correlation_id.as_str())
        && valid_code(receipt.authority_transaction_id.as_str())
        && valid_code(receipt.operation_attempt_id.as_str())
        && valid_code(receipt.approval_id.as_str())
        && valid_code(receipt.grant_id.as_str())
        && valid_code(receipt.session_id.as_str())
        && valid_code(receipt.task_id.as_str())
        && valid_code(receipt.action_id.as_str())
        && receipt
            .tool_call_id
            .as_ref()
            .is_none_or(|identity| valid_code(identity.as_str()))
        && valid_sha256(&receipt.operation_sha256)
        && valid_sha256(&receipt.previous_receipt_sha256)
        && valid_sha256(&receipt.receipt_sha256)
        && receipt.operation_sha256
            == serde_json::to_vec(&receipt.operation)
                .map(|bytes| sha256_hex(&bytes))
                .unwrap_or_default()
        && !receipt.occurred_at.trim().is_empty()
        && receipt.occurred_at.len() <= 64
}

fn valid_limitations(values: &[String]) -> bool {
    values.len() <= MAX_LIMITATIONS
        && !has_duplicates(values.iter().map(String::as_str))
        && values.iter().all(|value| valid_code(value))
}

fn valid_code(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_CODE_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn has_duplicates<'a>(values: impl Iterator<Item = &'a str>) -> bool {
    let values = values.collect::<Vec<_>>();
    values.iter().copied().collect::<BTreeSet<_>>().len() != values.len()
}

fn resolution_digest(value: &CitationResolution) -> String {
    #[derive(Serialize)]
    struct Unsigned<'a> {
        citation: &'a SourceCitation,
        freshness: CitationFreshness,
        reason_code: &'a str,
        current_file: &'a Option<CitationFileIdentity>,
    }
    serde_json::to_vec(&Unsigned {
        citation: &value.citation,
        freshness: value.freshness,
        reason_code: &value.reason_code,
        current_file: &value.current_file,
    })
    .map(|bytes| sha256_hex(&bytes))
    .unwrap_or_default()
}

fn answer_ledger_digest(value: &AnswerClaimLedger) -> String {
    #[derive(Serialize)]
    struct Unsigned<'a> {
        schema_version: u16,
        task_id: &'a TaskId,
        rendered_claim_ids: &'a [String],
        entries: &'a [AnswerClaimEntry],
    }
    serde_json::to_vec(&Unsigned {
        schema_version: value.schema_version,
        task_id: &value.task_id,
        rendered_claim_ids: &value.rendered_claim_ids,
        entries: &value.entries,
    })
    .map(|bytes| sha256_hex(&bytes))
    .unwrap_or_default()
}

fn receipt_digest(receipt: &Receipt) -> String {
    let mut candidate = receipt.clone();
    candidate.receipt_sha256 = ZERO_SHA256.to_owned();
    to_canonical_json(&candidate)
        .map(|bytes| sha256_hex(&bytes))
        .unwrap_or_default()
}

fn anchor_message(count: u64, head: &str) -> Vec<u8> {
    let mut message = count.to_be_bytes().to_vec();
    message.extend_from_slice(head.as_bytes());
    message
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> String {
    let mut block = [0_u8; 64];
    if key.len() > block.len() {
        block[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let mut inner_pad = block;
    let mut outer_pad = block;
    for byte in &mut inner_pad {
        *byte ^= 0x36;
    }
    for byte in &mut outer_pad {
        *byte ^= 0x5c;
    }
    let inner = Sha256::new()
        .chain_update(inner_pad)
        .chain_update(message)
        .finalize();
    let digest = Sha256::new()
        .chain_update(outer_pad)
        .chain_update(inner)
        .finalize();
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    block.zeroize();
    inner_pad.zeroize();
    outer_pad.zeroize();
    output
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

fn sha256_hex(value: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use agentmage_kernel_contracts::{
        ActionId, ApprovalId, AuthorityTransactionId, CONTRACT_SCHEMA_VERSION, CorrelationId,
        DeterministicMethodIdentity, EvidenceId, EvidenceKind, EvidenceReference, GrantId,
        GrantOperation, MaterialClaim, MaterialClaimKind, ModelAdapterId, ModelManifestObservation,
        ModelProfileId, ModelRunId, ModelRuntimeIdentity, ModelRuntimeKind, OperationAttemptId,
        OperationBinding, OperationOutcome, PlatformArchitecture, PlatformFamily, Receipt,
        ReceiptId, SessionId, TaskId, ToolCallId, UnknownBlockedReason, WorkspaceId, WorkspacePath,
    };

    use super::{
        CitationFileIdentity, CitationFreshness, CitationSelector, ReceiptIntegrityKey,
        SourceCitation, TamperEvidentReceiptLedger, ZERO_SHA256, answer_ledger_digest,
        build_answer_claim_ledger, receipt_digest, render_claim_audit, resolve_citation,
        safe_evidence_projection, seal_receipt_ledger, sha256_hex, verify_answer_claim_ledger,
        verify_receipt_anchor,
    };
    use crate::evidence_state::{DeterministicMethodRegistry, EvidenceStateAssigner};

    const SHA_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const SHA_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    type EntryEvidence = BTreeMap<String, (Vec<super::CitationResolution>, Vec<String>)>;
    type LedgerFixture = (
        crate::evidence_state::ValidatedEvidenceAssignments,
        Vec<String>,
        EntryEvidence,
    );

    fn path(name: &str) -> WorkspacePath {
        WorkspacePath::new(WorkspaceId::from_raw("workspace-evidence"), ["src", name]).unwrap()
    }

    fn file(name: &str, hash: &str, revision: &str, byte_len: u64) -> CitationFileIdentity {
        CitationFileIdentity {
            path: path(name),
            byte_len,
            content_sha256: hash.to_owned(),
            object_identity_sha256: SHA_A.to_owned(),
            revision: revision.to_owned(),
        }
    }

    fn evidence(id: &str, subject: &str, hash: &str) -> EvidenceReference {
        EvidenceReference {
            schema_version: CONTRACT_SCHEMA_VERSION,
            evidence_id: EvidenceId::from_raw(id),
            kind: EvidenceKind::Observation,
            source_id: format!("src/{subject}.rs"),
            object_id: subject.to_owned(),
            fragment: Some("bytes:0-8".to_owned()),
            content_sha256: hash.to_owned(),
            observed_revision: Some("revision-1".to_owned()),
        }
    }

    fn citation(
        id: &str,
        evidence: EvidenceReference,
        file: CitationFileIdentity,
    ) -> SourceCitation {
        SourceCitation {
            citation_id: id.to_owned(),
            evidence,
            selector: CitationSelector::ByteRange { start: 0, end: 8 },
            observed_at_epoch_ms: 1_000,
            observed_file: file,
        }
    }

    fn claim(id: &str, subject: &str) -> MaterialClaim {
        MaterialClaim {
            schema_version: CONTRACT_SCHEMA_VERSION,
            claim_id: id.to_owned(),
            task_id: TaskId::from_raw("task-evidence"),
            kind: MaterialClaimKind::Read,
            statement: format!("Material statement {id}"),
            subject_id: subject.to_owned(),
            expected_revision: "revision-1".to_owned(),
            prerequisite_claim_ids: Vec::new(),
        }
    }

    fn method() -> DeterministicMethodIdentity {
        DeterministicMethodIdentity {
            method_id: "method.sum".to_owned(),
            method_version: "1.0.0".to_owned(),
            implementation_sha256: SHA_A.to_owned(),
        }
    }

    fn receipt(
        sequence: u64,
        id: &str,
        attempt: &str,
        previous: &str,
        evidence: Vec<EvidenceReference>,
    ) -> Receipt {
        let operation = OperationBinding::new(GrantOperation::WorkspaceRead);
        let mut value = Receipt {
            schema_version: CONTRACT_SCHEMA_VERSION,
            receipt_id: ReceiptId::from_raw(id),
            sequence,
            correlation_id: CorrelationId::from_raw(format!("correlation-{id}")),
            authority_transaction_id: AuthorityTransactionId::from_raw(format!("transaction-{id}")),
            operation_attempt_id: OperationAttemptId::from_raw(attempt),
            approval_id: ApprovalId::from_raw(format!("approval-{id}")),
            grant_id: GrantId::from_raw(format!("grant-{id}")),
            session_id: SessionId::from_raw("session-evidence"),
            task_id: TaskId::from_raw("task-evidence"),
            action_id: ActionId::from_raw(format!("action-{id}")),
            tool_call_id: Some(ToolCallId::from_raw(format!("call-{id}"))),
            operation,
            outcome: OperationOutcome::Succeeded,
            operation_sha256: sha256_hex(&serde_json::to_vec(&operation).unwrap()),
            evidence,
            error: None,
            previous_receipt_sha256: previous.to_owned(),
            receipt_sha256: ZERO_SHA256.to_owned(),
            occurred_at: format!("1970-01-01T00:00:0{sequence}Z"),
        };
        value.receipt_sha256 = receipt_digest(&value);
        value
    }

    fn manifest() -> ModelManifestObservation {
        ModelManifestObservation {
            profile_id: ModelProfileId::from_raw("profile-exact"),
            manifest_sha256: SHA_A.to_owned(),
            artifact_sha256: SHA_A.to_owned(),
            tokenizer_sha256: SHA_A.to_owned(),
            template_sha256: SHA_A.to_owned(),
            codec_sha256: SHA_A.to_owned(),
            runtime: ModelRuntimeIdentity {
                adapter_id: ModelAdapterId::from_raw("adapter-exact"),
                kind: ModelRuntimeKind::NativeLlamaCpp,
                contract_version: 1,
                runtime_build: "runtime-1.0.0".to_owned(),
                runtime_sha256: SHA_A.to_owned(),
                platform: PlatformFamily::Fedora,
                architecture: PlatformArchitecture::X86_64,
            },
        }
    }

    fn ledger_fixture() -> LedgerFixture {
        let observed_evidence = evidence("evidence-observed", "subject-observed", SHA_A);
        let inferred_evidence = evidence("evidence-inferred", "subject-source", SHA_A);
        let stale_evidence = evidence("evidence-stale", "subject-stale", SHA_A);
        let observed_resolution = resolve_citation(
            citation(
                "citation-observed",
                observed_evidence.clone(),
                file("subject-observed.rs", SHA_A, "revision-1", 16),
            ),
            Some(file("subject-observed.rs", SHA_A, "revision-1", 16)),
        )
        .unwrap();
        let inferred_resolution = resolve_citation(
            citation(
                "citation-inferred",
                inferred_evidence.clone(),
                file("subject-source.rs", SHA_A, "revision-1", 16),
            ),
            Some(file("subject-source.rs", SHA_A, "revision-1", 16)),
        )
        .unwrap();
        let stale_resolution = resolve_citation(
            citation(
                "citation-stale",
                stale_evidence.clone(),
                file("subject-stale.rs", SHA_A, "revision-1", 16),
            ),
            Some(file("subject-stale.rs", SHA_B, "revision-2", 16)),
        )
        .unwrap();

        let methods = DeterministicMethodRegistry::new(vec![method()]).unwrap();
        let mut assigner =
            EvidenceStateAssigner::new(TaskId::from_raw("task-evidence"), methods).unwrap();
        assigner
            .assign_observed(
                "assignment-observed",
                claim("claim-observed", "subject-observed"),
                receipt(
                    1,
                    "observed",
                    "attempt-observed",
                    ZERO_SHA256,
                    vec![observed_evidence],
                ),
                vec![evidence("evidence-observed", "subject-observed", SHA_A)],
            )
            .unwrap();
        assigner
            .assign_derived(
                "assignment-derived",
                claim("claim-derived", "subject-derived"),
                method(),
                vec!["assignment-observed".to_owned()],
            )
            .unwrap();
        assigner
            .assign_inferred(
                "assignment-inferred",
                claim("claim-inferred", "subject-inferred"),
                vec![inferred_evidence],
                ModelRunId::from_raw("run-exact"),
                manifest(),
                SHA_A.to_owned(),
            )
            .unwrap();
        assigner
            .assign_unknown_blocked(
                "assignment-unknown",
                claim("claim-unknown", "subject-unknown"),
                UnknownBlockedReason::StaleEvidence,
                "citation.source.stale".to_owned(),
                vec![stale_evidence],
            )
            .unwrap();

        let rendered = vec![
            "claim-observed".to_owned(),
            "claim-derived".to_owned(),
            "claim-inferred".to_owned(),
            "claim-unknown".to_owned(),
        ];
        let evidence = BTreeMap::from([
            (
                "assignment-observed".to_owned(),
                (vec![observed_resolution], Vec::new()),
            ),
            (
                "assignment-derived".to_owned(),
                (Vec::new(), vec!["method.deterministic".to_owned()]),
            ),
            (
                "assignment-inferred".to_owned(),
                (
                    vec![inferred_resolution],
                    vec!["model.inference".to_owned()],
                ),
            ),
            (
                "assignment-unknown".to_owned(),
                (vec![stale_resolution], vec!["citation.stale".to_owned()]),
            ),
        ]);
        (assigner.finalize(), rendered, evidence)
    }

    #[test]
    fn citation_resolution_preserves_current_stale_missing_unicode_and_bounds() {
        let observed = file("café.rs", SHA_A, "revision-1", 16);
        let source = citation(
            "citation-unicode",
            evidence("evidence-unicode", "subject", SHA_A),
            observed.clone(),
        );
        let current = resolve_citation(source.clone(), Some(observed.clone())).unwrap();
        assert_eq!(current.freshness, CitationFreshness::Current);
        assert_eq!(
            resolve_citation(source.clone(), None).unwrap().freshness,
            CitationFreshness::Missing
        );
        for changed in [
            file("renamed.rs", SHA_A, "revision-1", 16),
            file("café.rs", SHA_B, "revision-1", 16),
            file("café.rs", SHA_A, "revision-2", 16),
            file("café.rs", SHA_A, "revision-1", 8),
        ] {
            assert_eq!(
                resolve_citation(source.clone(), Some(changed))
                    .unwrap()
                    .freshness,
                CitationFreshness::Stale
            );
        }
        let mut invalid = source;
        invalid.selector = CitationSelector::ByteRange { start: 0, end: 17 };
        assert!(resolve_citation(invalid, Some(observed)).is_err());
    }

    #[test]
    fn complete_four_state_ledger_projects_and_renders_without_raw_content() {
        let (assignments, rendered, evidence) = ledger_fixture();
        let ledger = build_answer_claim_ledger(assignments, rendered, evidence).unwrap();
        assert!(verify_answer_claim_ledger(&ledger));
        assert_eq!(safe_evidence_projection(&ledger).len(), 3);
        let audit = render_claim_audit(&ledger);
        assert_eq!(audit.lines().count(), 4);
        assert!(audit.contains("state=unknown_blocked"));
        assert!(!audit.contains("Material statement"));
        assert!(
            !serde_json::to_string(&safe_evidence_projection(&ledger))
                .unwrap()
                .contains("src/")
        );
        assert_eq!(
            crate::authority::reject_as_authority(&ledger).artifact_kind,
            crate::authority::DescriptiveArtifactKind::ClaimRecord
        );
    }

    #[test]
    fn omitted_reordered_stale_fabricated_and_false_completion_claims_fail_closed() {
        let (assignments, mut rendered, evidence) = ledger_fixture();
        rendered.pop();
        assert!(build_answer_claim_ledger(assignments, rendered, evidence).is_err());

        let (assignments, mut rendered, evidence) = ledger_fixture();
        rendered.swap(0, 1);
        let ledger = build_answer_claim_ledger(assignments, rendered, evidence).unwrap();
        let mut tampered = ledger.clone();
        tampered.entries.swap(0, 1);
        tampered.ledger_sha256 = answer_ledger_digest(&tampered);
        assert!(!verify_answer_claim_ledger(&tampered));

        let (assignments, rendered, mut evidence) = ledger_fixture();
        evidence.get_mut("assignment-observed").unwrap().0[0].freshness = CitationFreshness::Stale;
        assert!(build_answer_claim_ledger(assignments, rendered, evidence).is_err());

        let (assignments, rendered, mut evidence) = ledger_fixture();
        evidence.get_mut("assignment-inferred").unwrap().0[0]
            .citation
            .evidence
            .evidence_id = EvidenceId::from_raw("fabricated");
        assert!(build_answer_claim_ledger(assignments, rendered, evidence).is_err());

        let (assignments, rendered, evidence) = ledger_fixture();
        let mut conflict = build_answer_claim_ledger(assignments, rendered, evidence).unwrap();
        let unknown = conflict.entries.last_mut().unwrap();
        let agentmage_kernel_contracts::MaterialClaimEvidenceState::UnknownBlocked(provenance) =
            &mut unknown.assignment.evidence_state
        else {
            panic!("fixture unknown state");
        };
        provenance.reason = UnknownBlockedReason::Conflict;
        conflict.ledger_sha256 = answer_ledger_digest(&conflict);
        assert!(!verify_answer_claim_ledger(&conflict));
    }

    #[test]
    fn receipt_chain_and_external_keyed_anchor_detect_every_tamper_class() {
        let first = receipt(1, "one", "attempt-one", ZERO_SHA256, Vec::new());
        let second = receipt(2, "two", "attempt-two", &first.receipt_sha256, Vec::new());
        let mut ledger = TamperEvidentReceiptLedger::default();
        ledger.append(first.clone()).unwrap();
        ledger.append(second.clone()).unwrap();
        assert!(ledger.verify());
        let key = ReceiptIntegrityKey::new([7; 32]).unwrap();
        let anchor = seal_receipt_ledger(&ledger, &key).unwrap();
        assert!(verify_receipt_anchor(&ledger, &anchor, &key));
        assert!(!verify_receipt_anchor(
            &ledger,
            &anchor,
            &ReceiptIntegrityKey::new([8; 32]).unwrap()
        ));
        assert!(!format!("{key:?}").contains('7'));
        assert!(ReceiptIntegrityKey::new([0; 32]).is_err());

        let mut removed = ledger.clone();
        removed.receipts.pop();
        assert!(!verify_receipt_anchor(&removed, &anchor, &key));
        let mut reordered = ledger.clone();
        reordered.receipts.swap(0, 1);
        assert!(!reordered.verify());
        let mut tampered = ledger.clone();
        tampered.receipts[0].outcome = OperationOutcome::Failed;
        assert!(!tampered.verify());

        let mut duplicate = TamperEvidentReceiptLedger::default();
        duplicate.append(first.clone()).unwrap();
        let replay = receipt(2, "other", "attempt-one", &first.receipt_sha256, Vec::new());
        assert!(duplicate.append(replay).is_err());
    }
}
