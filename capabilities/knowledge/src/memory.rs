//! Source-backed memory candidate policy without automatic durable authority.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use agentmage_kernel_contracts::{DataSensitivity, EvidenceReference, WorkspaceId};
use sha2::{Digest, Sha256};

const MAX_ID_BYTES: usize = 128;
const MAX_CONTENT_BYTES: usize = 64 * 1024;
const MAX_SCOPE_ID_BYTES: usize = 128;
const MAX_FACT_KEY_BYTES: usize = 128;
const MAX_TAGS: usize = 128;
const MAX_TAG_BYTES: usize = 128;
const MAX_LINKS: usize = 256;
const MAX_EVIDENCE: usize = 256;

/// Stable memory identity independent of a Markdown path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MemoryId(String);

impl MemoryId {
    /// Parses one stable memory identity.
    pub fn parse(value: impl Into<String>) -> Result<Self, MemoryError> {
        let value = value.into();
        if value.is_empty()
            || value.len() > MAX_ID_BYTES
            || !value.starts_with("memory-")
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(MemoryError::InvalidInput);
        }
        Ok(Self(value))
    }

    /// Returns the stable wire identity.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Closed memory type taxonomy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryType {
    /// Bounded temporary context for the active task.
    Working,
    /// Source-backed record of an event or interaction.
    Episodic,
    /// Source-backed durable fact.
    Semantic,
    /// Reviewed reusable procedure.
    Procedural,
    /// Explicitly observed or stated user preference.
    Preference,
}

/// Closed policy classification for a proposed memory candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryCandidateClass {
    /// Temporary context that cannot be durably promoted as-is.
    TemporaryContext,
    /// Source-backed durable fact candidate.
    DurableFact,
    /// Source-backed preference candidate.
    Preference,
    /// Source-backed procedure candidate.
    Procedure,
    /// Source-backed event candidate.
    Episodic,
    /// Source-free, low-confidence, or otherwise unresolved assertion.
    UnresolvedClaim,
    /// Candidate conflicts with a current source-backed item.
    Contradiction,
    /// Secret, restricted, or inferred-sensitive content prohibited from Markdown memory.
    Prohibited,
}

/// Current user-visible memory lifecycle state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryItemStatus {
    /// Candidate exists but has no durable decision.
    Candidate,
    /// Explicitly approved current durable item.
    Approved,
    /// Explicitly rejected candidate.
    Rejected,
    /// Retained historical item replaced by another identity.
    Superseded,
    /// Item passed its exact expiry.
    Expired,
    /// Explicit user hold prevents automatic expiry or deletion.
    Hold,
    /// Tombstoned after an explicit deletion decision.
    Deleted,
}

/// Exact workspace, project, and conversation namespace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryScope {
    /// Exact workspace identity.
    pub workspace_id: WorkspaceId,
    /// Optional stable project identity.
    pub project_id: Option<String>,
    /// Optional stable conversation identity.
    pub conversation_id: Option<String>,
}

/// Complete source-backed memory proposal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryCandidate {
    /// Stable candidate/item identity.
    pub memory_id: MemoryId,
    /// Requested memory type.
    pub memory_type: MemoryType,
    /// Exact namespace.
    pub scope: MemoryScope,
    /// Bounded user-visible content.
    pub content: String,
    /// Optional normalized fact identity for contradiction and supersession handling.
    pub fact_key: Option<String>,
    /// Stable searchable tags.
    pub tags: Vec<String>,
    /// Stable related memory identities.
    pub links: Vec<MemoryId>,
    /// Content-addressed supporting evidence.
    pub evidence: Vec<EvidenceReference>,
    /// Data handling label.
    pub sensitivity: DataSensitivity,
    /// Explicit confidence in basis points.
    pub confidence_bps: u32,
    /// Exact creation timestamp supplied by a trusted caller.
    pub created_at: String,
    /// Optional exact expiry timestamp.
    pub expires_at: Option<String>,
    /// Whether policy identified inferred-sensitive content.
    pub inferred_sensitive: bool,
    /// Whether a model requested promotion; this grants no authority.
    pub model_requested_promotion: bool,
}

/// Content-free policy result for one exact candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryCandidateDecision {
    /// Candidate digest bound to the decision.
    pub candidate_sha256: String,
    /// Closed policy class.
    pub classification: MemoryCandidateClass,
    /// Whether the candidate may be offered for an explicit user decision.
    pub eligible_for_user_decision: bool,
    /// Whether explicit user approval is required for durable promotion.
    pub user_decision_required: bool,
    /// Stable content-free reason code.
    pub reason_code: String,
    /// Current item identities that contradict this candidate.
    pub conflicting_memory_ids: Vec<MemoryId>,
    /// Fixed false automatic-promotion marker.
    pub automatic_promotion: bool,
}

/// Explicit human decision on one exact candidate and policy result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserMemoryDecision {
    /// Approve the exact candidate as a durable item.
    Approve,
    /// Reject the exact candidate while retaining a content-free decision record.
    Reject,
}

/// Current or historical source-backed memory item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryItem {
    /// Stable identity.
    pub memory_id: MemoryId,
    /// Memory type.
    pub memory_type: MemoryType,
    /// Exact namespace.
    pub scope: MemoryScope,
    /// Exact approved content, absent from rejected tombstones.
    pub content: Option<String>,
    /// Optional normalized fact identity.
    pub fact_key: Option<String>,
    /// Stable searchable tags.
    pub tags: Vec<String>,
    /// Stable related memory identities.
    pub links: Vec<MemoryId>,
    /// Immutable content-addressed evidence.
    pub evidence: Vec<EvidenceReference>,
    /// Data handling label.
    pub sensitivity: DataSensitivity,
    /// Confidence in basis points.
    pub confidence_bps: u32,
    /// Current lifecycle state.
    pub status: MemoryItemStatus,
    /// Exact creation timestamp.
    pub created_at: String,
    /// Exact decision timestamp.
    pub decided_at: String,
    /// Exact last verification timestamp.
    pub last_verified_at: String,
    /// Optional expiry timestamp.
    pub expires_at: Option<String>,
    /// Digest of the candidate bound to the decision.
    pub candidate_sha256: String,
    /// Digest of the explicit user decision evidence.
    pub decision_sha256: String,
    /// Optional identity that supersedes this item.
    pub superseded_by: Option<MemoryId>,
}

/// Closed memory policy failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryError {
    /// Identity, scope, content, metadata, evidence, timestamp, or bounds are invalid.
    InvalidInput,
    /// Candidate contains content prohibited from Markdown memory.
    ProhibitedContent,
    /// Candidate is unresolved, contradictory, temporary, or otherwise ineligible.
    IneligibleCandidate,
    /// Policy result does not bind the supplied candidate.
    DecisionDrift,
    /// Fixed resource or arithmetic bound was exceeded.
    ResourceLimit,
    /// Stable memory identity already exists.
    DuplicateIdentity,
    /// Requested stable memory identity is absent.
    NotFound,
    /// Requested lifecycle transition is prohibited.
    InvalidTransition,
}

/// Evaluates one candidate without mutating or promoting any memory state.
pub fn evaluate_memory_candidate(
    candidate: &MemoryCandidate,
    current_items: &[MemoryItem],
) -> Result<MemoryCandidateDecision, MemoryError> {
    validate_candidate(candidate)?;
    let candidate_sha256 = candidate_digest(candidate)?;
    if crate::domain::secret_candidate(&candidate.content)
        || candidate.inferred_sensitive
        || candidate.sensitivity == DataSensitivity::Restricted
    {
        return Ok(decision(
            candidate_sha256,
            MemoryCandidateClass::Prohibited,
            false,
            "memory.candidate.prohibited",
            Vec::new(),
        ));
    }
    if candidate.memory_type == MemoryType::Working {
        return Ok(decision(
            candidate_sha256,
            MemoryCandidateClass::TemporaryContext,
            false,
            "memory.candidate.temporary",
            Vec::new(),
        ));
    }
    if candidate.evidence.is_empty() || candidate.confidence_bps < 7_000 {
        return Ok(decision(
            candidate_sha256,
            MemoryCandidateClass::UnresolvedClaim,
            false,
            "memory.candidate.unresolved",
            Vec::new(),
        ));
    }
    let candidate_content_sha256 = sha256(candidate.content.as_bytes());
    let mut conflicts = current_items
        .iter()
        .filter(|item| {
            item.status == MemoryItemStatus::Approved
                && item.scope.workspace_id == candidate.scope.workspace_id
                && item.scope.project_id == candidate.scope.project_id
                && item.fact_key.is_some()
                && item.fact_key == candidate.fact_key
                && item
                    .content
                    .as_ref()
                    .is_some_and(|content| sha256(content.as_bytes()) != candidate_content_sha256)
        })
        .map(|item| item.memory_id.clone())
        .collect::<Vec<_>>();
    conflicts.sort();
    conflicts.dedup();
    if !conflicts.is_empty() {
        return Ok(decision(
            candidate_sha256,
            MemoryCandidateClass::Contradiction,
            false,
            "memory.candidate.contradiction",
            conflicts,
        ));
    }
    let classification = match candidate.memory_type {
        MemoryType::Working => unreachable!("working candidates return before durable policy"),
        MemoryType::Episodic => MemoryCandidateClass::Episodic,
        MemoryType::Semantic => MemoryCandidateClass::DurableFact,
        MemoryType::Procedural => MemoryCandidateClass::Procedure,
        MemoryType::Preference => MemoryCandidateClass::Preference,
    };
    Ok(decision(
        candidate_sha256,
        classification,
        true,
        "memory.candidate.user-decision-required",
        Vec::new(),
    ))
}

/// Applies an explicit human decision to one exact eligible candidate.
pub fn resolve_memory_candidate(
    candidate: &MemoryCandidate,
    policy: &MemoryCandidateDecision,
    user_decision: UserMemoryDecision,
    decision_sha256: String,
    decided_at: String,
) -> Result<MemoryItem, MemoryError> {
    validate_candidate(candidate)?;
    if policy.candidate_sha256 != candidate_digest(candidate)? {
        return Err(MemoryError::DecisionDrift);
    }
    if !valid_sha256(&decision_sha256) || !valid_timestamp(&decided_at) {
        return Err(MemoryError::InvalidInput);
    }
    if !policy.eligible_for_user_decision
        || !policy.user_decision_required
        || policy.automatic_promotion
        || !matches!(
            policy.classification,
            MemoryCandidateClass::DurableFact
                | MemoryCandidateClass::Preference
                | MemoryCandidateClass::Procedure
                | MemoryCandidateClass::Episodic
        )
    {
        return Err(MemoryError::IneligibleCandidate);
    }
    let approved = user_decision == UserMemoryDecision::Approve;
    Ok(MemoryItem {
        memory_id: candidate.memory_id.clone(),
        memory_type: candidate.memory_type,
        scope: candidate.scope.clone(),
        content: approved.then(|| candidate.content.clone()),
        fact_key: candidate.fact_key.clone(),
        tags: candidate.tags.clone(),
        links: candidate.links.clone(),
        evidence: candidate.evidence.clone(),
        sensitivity: candidate.sensitivity,
        confidence_bps: candidate.confidence_bps,
        status: if approved {
            MemoryItemStatus::Approved
        } else {
            MemoryItemStatus::Rejected
        },
        created_at: candidate.created_at.clone(),
        decided_at: decided_at.clone(),
        last_verified_at: decided_at,
        expires_at: candidate.expires_at.clone(),
        candidate_sha256: policy.candidate_sha256.clone(),
        decision_sha256,
        superseded_by: None,
    })
}

fn decision(
    candidate_sha256: String,
    classification: MemoryCandidateClass,
    eligible_for_user_decision: bool,
    reason_code: &str,
    conflicting_memory_ids: Vec<MemoryId>,
) -> MemoryCandidateDecision {
    MemoryCandidateDecision {
        candidate_sha256,
        classification,
        eligible_for_user_decision,
        user_decision_required: eligible_for_user_decision,
        reason_code: reason_code.to_owned(),
        conflicting_memory_ids,
        automatic_promotion: false,
    }
}

fn validate_candidate(candidate: &MemoryCandidate) -> Result<(), MemoryError> {
    if candidate.content.is_empty()
        || candidate.content.len() > MAX_CONTENT_BYTES
        || candidate.content.chars().any(char::is_control)
        || !valid_scope(&candidate.scope)
        || candidate.confidence_bps > 10_000
        || !valid_timestamp(&candidate.created_at)
        || candidate
            .expires_at
            .as_ref()
            .is_some_and(|value| !valid_timestamp(value) || value <= &candidate.created_at)
        || candidate.tags.len() > MAX_TAGS
        || candidate.links.len() > MAX_LINKS
        || candidate.evidence.len() > MAX_EVIDENCE
        || candidate.fact_key.as_ref().is_some_and(|value| {
            value.is_empty()
                || value.len() > MAX_FACT_KEY_BYTES
                || !value.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'.' | b'-' | b'_')
                })
        })
    {
        return Err(MemoryError::InvalidInput);
    }
    let tags = candidate.tags.iter().collect::<BTreeSet<_>>();
    if tags.len() != candidate.tags.len()
        || candidate.tags.iter().any(|tag| {
            tag.is_empty()
                || tag.len() > MAX_TAG_BYTES
                || !tag.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'-' | b'_')
                })
        })
        || candidate.links.iter().collect::<BTreeSet<_>>().len() != candidate.links.len()
        || candidate
            .evidence
            .iter()
            .map(|item| item.evidence_id.as_str())
            .collect::<BTreeSet<_>>()
            .len()
            != candidate.evidence.len()
        || candidate
            .evidence
            .iter()
            .any(|item| !valid_sha256(&item.content_sha256))
    {
        return Err(MemoryError::InvalidInput);
    }
    Ok(())
}

fn valid_scope(scope: &MemoryScope) -> bool {
    valid_optional_scope_id(&scope.project_id) && valid_optional_scope_id(&scope.conversation_id)
}

fn valid_optional_scope_id(value: &Option<String>) -> bool {
    value.as_ref().is_none_or(|value| {
        !value.is_empty()
            && value.len() <= MAX_SCOPE_ID_BYTES
            && value.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'-' | b'_' | b'.')
            })
    })
}

fn candidate_digest(candidate: &MemoryCandidate) -> Result<String, MemoryError> {
    let evidence = candidate
        .evidence
        .iter()
        .map(|item| {
            (
                item.evidence_id.as_str(),
                &item.source_id,
                &item.object_id,
                &item.fragment,
                &item.content_sha256,
            )
        })
        .collect::<Vec<_>>();
    let material = serde_json::to_vec(&(
        candidate.memory_id.as_str(),
        memory_type_id(candidate.memory_type),
        candidate.scope.workspace_id.as_str(),
        &candidate.scope.project_id,
        &candidate.scope.conversation_id,
        &candidate.content,
        &candidate.fact_key,
        &candidate.tags,
        candidate
            .links
            .iter()
            .map(MemoryId::as_str)
            .collect::<Vec<_>>(),
        evidence,
        sensitivity_id(candidate.sensitivity),
        candidate.confidence_bps,
        &candidate.created_at,
        &candidate.expires_at,
        candidate.inferred_sensitive,
        candidate.model_requested_promotion,
    ))
    .map_err(|_| MemoryError::ResourceLimit)?;
    Ok(sha256(&material))
}

const fn memory_type_id(value: MemoryType) -> &'static str {
    match value {
        MemoryType::Working => "working",
        MemoryType::Episodic => "episodic",
        MemoryType::Semantic => "semantic",
        MemoryType::Procedural => "procedural",
        MemoryType::Preference => "preference",
    }
}

const fn sensitivity_id(value: DataSensitivity) -> &'static str {
    match value {
        DataSensitivity::Ephemeral => "ephemeral",
        DataSensitivity::Operational => "operational",
        DataSensitivity::Durable => "durable",
        DataSensitivity::Restricted => "restricted",
    }
}

fn valid_timestamp(value: &str) -> bool {
    value.len() >= 20
        && value.len() <= 64
        && value.ends_with('Z')
        && value.as_bytes().get(4) == Some(&b'-')
        && value.as_bytes().get(7) == Some(&b'-')
        && value.as_bytes().get(10) == Some(&b'T')
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256(value: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        DataSensitivity, EvidenceId, EvidenceKind, EvidenceReference, WorkspaceId,
    };

    use super::{
        MemoryCandidate, MemoryCandidateClass, MemoryError, MemoryId, MemoryItem, MemoryItemStatus,
        MemoryScope, MemoryType, UserMemoryDecision, evaluate_memory_candidate,
        resolve_memory_candidate,
    };

    fn evidence(id: &str) -> EvidenceReference {
        EvidenceReference {
            schema_version: 1,
            evidence_id: EvidenceId::from_raw(id),
            kind: EvidenceKind::Document,
            source_id: "fixture-source".to_owned(),
            object_id: "fixture-object".to_owned(),
            fragment: Some("line-1".to_owned()),
            content_sha256: "a".repeat(64),
            observed_revision: Some("revision-1".to_owned()),
        }
    }

    fn candidate(memory_type: MemoryType) -> MemoryCandidate {
        MemoryCandidate {
            memory_id: MemoryId::parse("memory-fixture-1").expect("valid identity"),
            memory_type,
            scope: MemoryScope {
                workspace_id: WorkspaceId::from_raw("workspace-memory"),
                project_id: Some("project-one".to_owned()),
                conversation_id: Some("conversation-one".to_owned()),
            },
            content: "The release owner is Alice.".to_owned(),
            fact_key: Some("release.owner".to_owned()),
            tags: vec!["release".to_owned()],
            links: Vec::new(),
            evidence: vec![evidence("evidence-memory-1")],
            sensitivity: DataSensitivity::Durable,
            confidence_bps: 9_000,
            created_at: "2026-08-14T12:00:00Z".to_owned(),
            expires_at: None,
            inferred_sensitive: false,
            model_requested_promotion: false,
        }
    }

    fn approved_item(content: &str) -> MemoryItem {
        let mut source = candidate(MemoryType::Semantic);
        source.content = content.to_owned();
        let policy = evaluate_memory_candidate(&source, &[]).expect("policy succeeds");
        resolve_memory_candidate(
            &source,
            &policy,
            UserMemoryDecision::Approve,
            "b".repeat(64),
            "2026-08-14T12:01:00Z".to_owned(),
        )
        .expect("approval succeeds")
    }

    #[test]
    fn candidate_taxonomy_is_closed_source_backed_and_explicit() {
        let working = evaluate_memory_candidate(&candidate(MemoryType::Working), &[])
            .expect("working policy succeeds");
        assert_eq!(
            working.classification,
            MemoryCandidateClass::TemporaryContext
        );
        assert!(!working.eligible_for_user_decision);

        for (memory_type, expected) in [
            (MemoryType::Semantic, MemoryCandidateClass::DurableFact),
            (MemoryType::Preference, MemoryCandidateClass::Preference),
            (MemoryType::Procedural, MemoryCandidateClass::Procedure),
            (MemoryType::Episodic, MemoryCandidateClass::Episodic),
        ] {
            let result = evaluate_memory_candidate(&candidate(memory_type), &[])
                .expect("durable policy succeeds");
            assert_eq!(result.classification, expected);
            assert!(result.eligible_for_user_decision);
            assert!(result.user_decision_required);
            assert!(!result.automatic_promotion);
        }

        let mut unresolved = candidate(MemoryType::Semantic);
        unresolved.evidence.clear();
        let result =
            evaluate_memory_candidate(&unresolved, &[]).expect("unresolved policy succeeds");
        assert_eq!(result.classification, MemoryCandidateClass::UnresolvedClaim);
        assert!(!result.eligible_for_user_decision);
    }

    #[test]
    fn secret_restricted_and_inferred_sensitive_candidates_are_prohibited() {
        let mut secret = candidate(MemoryType::Semantic);
        secret.content = "password=hidden".to_owned();
        let result = evaluate_memory_candidate(&secret, &[]).expect("policy succeeds");
        assert_eq!(result.classification, MemoryCandidateClass::Prohibited);

        let mut restricted = candidate(MemoryType::Semantic);
        restricted.sensitivity = DataSensitivity::Restricted;
        let result = evaluate_memory_candidate(&restricted, &[]).expect("policy succeeds");
        assert_eq!(result.classification, MemoryCandidateClass::Prohibited);

        let mut inferred = candidate(MemoryType::Preference);
        inferred.inferred_sensitive = true;
        let result = evaluate_memory_candidate(&inferred, &[]).expect("policy succeeds");
        assert_eq!(result.classification, MemoryCandidateClass::Prohibited);
    }

    #[test]
    fn contradictions_are_preserved_and_never_silently_superseded() {
        let current = approved_item("The release owner is Bob.");
        let proposed = candidate(MemoryType::Semantic);
        let result = evaluate_memory_candidate(&proposed, std::slice::from_ref(&current))
            .expect("policy succeeds");
        assert_eq!(result.classification, MemoryCandidateClass::Contradiction);
        assert_eq!(result.conflicting_memory_ids, [current.memory_id]);
        assert!(!result.eligible_for_user_decision);
        assert_eq!(
            resolve_memory_candidate(
                &proposed,
                &result,
                UserMemoryDecision::Approve,
                "b".repeat(64),
                "2026-08-14T12:02:00Z".to_owned(),
            ),
            Err(MemoryError::IneligibleCandidate)
        );
    }

    #[test]
    fn explicit_approve_and_reject_bind_candidate_and_decision_evidence() {
        let mut proposed = candidate(MemoryType::Preference);
        proposed.model_requested_promotion = true;
        let policy = evaluate_memory_candidate(&proposed, &[]).expect("policy succeeds");
        assert!(!policy.automatic_promotion);

        let approved = resolve_memory_candidate(
            &proposed,
            &policy,
            UserMemoryDecision::Approve,
            "c".repeat(64),
            "2026-08-14T12:03:00Z".to_owned(),
        )
        .expect("approval succeeds");
        assert_eq!(approved.status, MemoryItemStatus::Approved);
        assert_eq!(approved.content.as_deref(), Some(proposed.content.as_str()));
        assert_eq!(approved.evidence, proposed.evidence);
        assert_eq!(approved.last_verified_at, approved.decided_at);

        let rejected = resolve_memory_candidate(
            &proposed,
            &policy,
            UserMemoryDecision::Reject,
            "d".repeat(64),
            "2026-08-14T12:04:00Z".to_owned(),
        )
        .expect("rejection succeeds");
        assert_eq!(rejected.status, MemoryItemStatus::Rejected);
        assert!(rejected.content.is_none());

        let mut changed = proposed;
        changed.content = "Changed after review.".to_owned();
        assert_eq!(
            resolve_memory_candidate(
                &changed,
                &policy,
                UserMemoryDecision::Approve,
                "e".repeat(64),
                "2026-08-14T12:05:00Z".to_owned(),
            ),
            Err(MemoryError::DecisionDrift)
        );
    }

    #[test]
    fn malformed_scope_tags_evidence_expiry_and_confidence_fail_closed() {
        let mut invalid = candidate(MemoryType::Semantic);
        invalid.tags.push("release".to_owned());
        assert_eq!(
            evaluate_memory_candidate(&invalid, &[]),
            Err(MemoryError::InvalidInput)
        );

        invalid = candidate(MemoryType::Semantic);
        invalid.scope.project_id = Some("../other".to_owned());
        assert_eq!(
            evaluate_memory_candidate(&invalid, &[]),
            Err(MemoryError::InvalidInput)
        );

        invalid = candidate(MemoryType::Semantic);
        invalid.evidence.push(evidence("evidence-memory-1"));
        assert_eq!(
            evaluate_memory_candidate(&invalid, &[]),
            Err(MemoryError::InvalidInput)
        );

        invalid = candidate(MemoryType::Semantic);
        invalid.expires_at = Some("2026-08-13T12:00:00Z".to_owned());
        assert_eq!(
            evaluate_memory_candidate(&invalid, &[]),
            Err(MemoryError::InvalidInput)
        );

        invalid = candidate(MemoryType::Semantic);
        invalid.confidence_bps = 10_001;
        assert_eq!(
            evaluate_memory_candidate(&invalid, &[]),
            Err(MemoryError::InvalidInput)
        );
    }
}
