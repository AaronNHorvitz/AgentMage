//! Bounded working-memory previews, compaction candidates, and selective durable loading.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use agentmage_kernel_contracts::{DataSensitivity, EvidenceReference, WorkspaceId};
use sha2::{Digest, Sha256};

use crate::{
    MemoryCandidate, MemoryCandidateClass, MemoryError, MemoryId, MemoryItem, MemoryItemStatus,
    MemoryType, evaluate_memory_candidate,
};

const MAX_WORKING_ENTRIES: usize = 10_000;
const MAX_WORKING_BYTES: u64 = 4 * 1024 * 1024;
const MAX_LOAD_RESULTS: u32 = 1_000;
const MAX_LOAD_BYTES: u64 = 4 * 1024 * 1024;
const MAX_QUERY_TERMS: usize = 64;
const MAX_QUERY_ITEM_BYTES: usize = 256;

/// One proposed durable candidate derived from one working entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkingCompactionProposal {
    /// Existing working-memory identity.
    pub source_memory_id: MemoryId,
    /// New independent durable candidate identity.
    pub candidate_memory_id: MemoryId,
    /// Proposed durable memory type; never `Working`.
    pub memory_type: MemoryType,
    /// Bounded proposed compacted content.
    pub content: String,
    /// Optional normalized fact key.
    pub fact_key: Option<String>,
    /// Stable proposed tags.
    pub tags: Vec<String>,
    /// Stable proposed links.
    pub links: Vec<MemoryId>,
    /// Proposed confidence no greater than the working entry confidence.
    pub confidence_bps: u32,
    /// Optional exact expiry.
    pub expires_at: Option<String>,
}

/// Complete no-write compaction preview.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkingCompactionPreview {
    /// New source-backed candidates requiring ordinary policy and user review.
    pub candidates: Vec<MemoryCandidate>,
    /// Digest of the exact working snapshot and proposals.
    pub preview_sha256: String,
    /// Fixed false durable-promotion marker.
    pub durable_memory_created: bool,
    /// Fixed false working-clear marker.
    pub working_memory_cleared: bool,
    /// Fixed false file-write marker.
    pub files_written: bool,
}

/// Complete `WORKING.md` preview.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkingMemoryPreview {
    /// Fixed portable filename.
    pub relative_path: String,
    /// Complete bounded working set.
    pub markdown: String,
    /// Digest of complete preview bytes.
    pub content_sha256: String,
    /// Fixed false apply marker.
    pub write_enabled: bool,
}

/// In-memory bounded working set with no durable or filesystem authority.
pub struct WorkingMemory {
    max_entries: usize,
    max_bytes: u64,
    entries: BTreeMap<MemoryId, MemoryCandidate>,
    used_bytes: u64,
}

/// Visible reason one durable item entered a selective context load.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryLoadReason {
    /// Exact workspace namespace matched.
    Workspace,
    /// Exact project namespace matched.
    Project,
    /// Exact conversation namespace matched.
    Conversation,
    /// At least one requested tag matched.
    Tag,
    /// At least one requested memory link matched.
    Link,
    /// At least one requested evidence identity matched.
    Source,
    /// Every requested literal relevance term matched.
    Relevance,
    /// Requested memory type matched.
    MemoryType,
}

/// Complete bounded selective-load query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryLoadQuery {
    /// Exact required workspace namespace.
    pub workspace_id: WorkspaceId,
    /// Optional exact project namespace.
    pub project_id: Option<String>,
    /// Optional exact conversation namespace.
    pub conversation_id: Option<String>,
    /// Optional allowed memory types.
    pub memory_types: BTreeSet<MemoryType>,
    /// Optional any-match tags.
    pub tags: BTreeSet<String>,
    /// Optional any-match linked memory identities.
    pub links: BTreeSet<MemoryId>,
    /// Optional any-match evidence identities.
    pub evidence_ids: BTreeSet<String>,
    /// Literal case-insensitive relevance terms; all required.
    pub relevance_terms: Vec<String>,
    /// Whether noncurrent items may enter.
    pub include_historical: bool,
    /// Maximum returned items.
    pub max_results: u32,
    /// Maximum complete UTF-8 content bytes.
    pub max_context_bytes: u64,
}

/// One source-backed selected memory item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryLoadHit {
    /// Stable memory identity.
    pub memory_id: MemoryId,
    /// Memory type.
    pub memory_type: MemoryType,
    /// Exact user-visible content.
    pub content: String,
    /// Immutable source evidence.
    pub evidence: Vec<EvidenceReference>,
    /// Visible selection reasons.
    pub reasons: Vec<MemoryLoadReason>,
    /// Deterministic integer score.
    pub score: i32,
    /// Exact last verification timestamp.
    pub last_verified_at: String,
    /// Data sensitivity label.
    pub sensitivity: DataSensitivity,
    /// Current or historical state.
    pub status: MemoryItemStatus,
    /// Optional superseding identity.
    pub superseded_by: Option<MemoryId>,
}

/// Complete bounded selective-load result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryLoadResult {
    /// Stable ranked selected items.
    pub hits: Vec<MemoryLoadHit>,
    /// Count omitted by result or byte budget.
    pub omitted_count: u64,
    /// Fixed false cross-workspace marker.
    pub cross_workspace_content: bool,
}

impl WorkingMemory {
    /// Creates an empty working set under explicit lower-or-equal fixed bounds.
    pub fn new(max_entries: usize, max_bytes: u64) -> Result<Self, MemoryError> {
        if max_entries == 0
            || max_entries > MAX_WORKING_ENTRIES
            || max_bytes == 0
            || max_bytes > MAX_WORKING_BYTES
        {
            return Err(MemoryError::InvalidInput);
        }
        Ok(Self {
            max_entries,
            max_bytes,
            entries: BTreeMap::new(),
            used_bytes: 0,
        })
    }

    /// Adds one policy-classified temporary entry without durable promotion.
    pub fn insert(&mut self, candidate: MemoryCandidate) -> Result<(), MemoryError> {
        let policy = evaluate_memory_candidate(&candidate, &[])?;
        if policy.classification != MemoryCandidateClass::TemporaryContext
            || candidate.memory_type != MemoryType::Working
        {
            return Err(MemoryError::IneligibleCandidate);
        }
        if self.entries.contains_key(&candidate.memory_id) {
            return Err(MemoryError::DuplicateIdentity);
        }
        if self.entries.len() >= self.max_entries {
            return Err(MemoryError::ResourceLimit);
        }
        let bytes =
            u64::try_from(candidate.content.len()).map_err(|_| MemoryError::ResourceLimit)?;
        let used = self
            .used_bytes
            .checked_add(bytes)
            .ok_or(MemoryError::ResourceLimit)?;
        if used > self.max_bytes {
            return Err(MemoryError::ResourceLimit);
        }
        self.used_bytes = used;
        self.entries.insert(candidate.memory_id.clone(), candidate);
        Ok(())
    }

    /// Renders the complete bounded working set without applying a file.
    pub fn preview_markdown(&self) -> Result<WorkingMemoryPreview, MemoryError> {
        let mut markdown = String::from("# Working Memory\n\n");
        for entry in self.entries.values() {
            writeln!(
                &mut markdown,
                "## {}\n\n{}\n\nSource evidence: {} item(s)\n",
                entry.memory_id.as_str(),
                entry.content,
                entry.evidence.len()
            )
            .map_err(|_| MemoryError::ResourceLimit)?;
        }
        Ok(WorkingMemoryPreview {
            relative_path: "WORKING.md".to_owned(),
            content_sha256: sha256(markdown.as_bytes()),
            markdown,
            write_enabled: false,
        })
    }

    /// Produces new review candidates without clearing working or creating durable memory.
    pub fn compact_preview(
        &self,
        proposals: &[WorkingCompactionProposal],
    ) -> Result<WorkingCompactionPreview, MemoryError> {
        if proposals.len() > self.entries.len() {
            return Err(MemoryError::ResourceLimit);
        }
        let mut source_ids = BTreeSet::new();
        let mut candidate_ids = BTreeSet::new();
        let mut candidates = Vec::new();
        for proposal in proposals {
            if proposal.memory_type == MemoryType::Working
                || !source_ids.insert(proposal.source_memory_id.clone())
                || !candidate_ids.insert(proposal.candidate_memory_id.clone())
            {
                return Err(MemoryError::InvalidInput);
            }
            let source = self
                .entries
                .get(&proposal.source_memory_id)
                .ok_or(MemoryError::NotFound)?;
            if proposal.confidence_bps > source.confidence_bps {
                return Err(MemoryError::InvalidInput);
            }
            let candidate = MemoryCandidate {
                memory_id: proposal.candidate_memory_id.clone(),
                memory_type: proposal.memory_type,
                scope: source.scope.clone(),
                content: proposal.content.clone(),
                fact_key: proposal.fact_key.clone(),
                tags: proposal.tags.clone(),
                links: proposal.links.clone(),
                evidence: source.evidence.clone(),
                sensitivity: source.sensitivity,
                confidence_bps: proposal.confidence_bps,
                created_at: source.created_at.clone(),
                expires_at: proposal.expires_at.clone(),
                inferred_sensitive: source.inferred_sensitive,
                model_requested_promotion: true,
            };
            let policy = evaluate_memory_candidate(&candidate, &[])?;
            if !policy.eligible_for_user_decision {
                return Err(MemoryError::IneligibleCandidate);
            }
            candidates.push(candidate);
        }
        let preview_sha256 = compaction_digest(&candidates)?;
        Ok(WorkingCompactionPreview {
            candidates,
            preview_sha256,
            durable_memory_created: false,
            working_memory_cleared: false,
            files_written: false,
        })
    }

    /// Clears temporary working state explicitly; this creates no durable item.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.used_bytes = 0;
    }

    /// Returns current entry and byte counts.
    #[must_use]
    pub fn counts(&self) -> (usize, u64) {
        (self.entries.len(), self.used_bytes)
    }
}

/// Selects exact scoped durable memory under result and byte budgets.
pub fn select_memory(
    items: &[MemoryItem],
    query: &MemoryLoadQuery,
) -> Result<MemoryLoadResult, MemoryError> {
    validate_query(query)?;
    let terms = query
        .relevance_terms
        .iter()
        .map(|value| value.to_lowercase())
        .collect::<Vec<_>>();
    let mut candidates = Vec::new();
    for item in items {
        let current = matches!(
            item.status,
            MemoryItemStatus::Approved | MemoryItemStatus::Hold
        );
        if item.scope.workspace_id != query.workspace_id
            || (!query.include_historical && !current)
            || query
                .project_id
                .as_ref()
                .is_some_and(|value| item.scope.project_id.as_ref() != Some(value))
            || query
                .conversation_id
                .as_ref()
                .is_some_and(|value| item.scope.conversation_id.as_ref() != Some(value))
            || !query.memory_types.is_empty() && !query.memory_types.contains(&item.memory_type)
        {
            continue;
        }
        let Some(content) = item.content.as_ref() else {
            continue;
        };
        let normalized = content.to_lowercase();
        if !terms.iter().all(|term| normalized.contains(term))
            || !query.tags.is_empty() && !item.tags.iter().any(|tag| query.tags.contains(tag))
            || !query.links.is_empty() && !item.links.iter().any(|link| query.links.contains(link))
            || !query.evidence_ids.is_empty()
                && !item
                    .evidence
                    .iter()
                    .any(|value| query.evidence_ids.contains(value.evidence_id.as_str()))
        {
            continue;
        }
        let mut reasons = vec![MemoryLoadReason::Workspace];
        let mut score = 10_i32;
        if query.project_id.is_some() {
            reasons.push(MemoryLoadReason::Project);
            score += 20;
        }
        if query.conversation_id.is_some() {
            reasons.push(MemoryLoadReason::Conversation);
            score += 25;
        }
        if !query.memory_types.is_empty() {
            reasons.push(MemoryLoadReason::MemoryType);
            score += 10;
        }
        if !query.tags.is_empty() {
            reasons.push(MemoryLoadReason::Tag);
            score += 15;
        }
        if !query.links.is_empty() {
            reasons.push(MemoryLoadReason::Link);
            score += 15;
        }
        if !query.evidence_ids.is_empty() {
            reasons.push(MemoryLoadReason::Source);
            score += 20;
        }
        if !terms.is_empty() {
            reasons.push(MemoryLoadReason::Relevance);
            score += 20;
        }
        score +=
            i32::try_from(item.confidence_bps / 1_000).map_err(|_| MemoryError::ResourceLimit)?;
        candidates.push(MemoryLoadHit {
            memory_id: item.memory_id.clone(),
            memory_type: item.memory_type,
            content: content.clone(),
            evidence: item.evidence.clone(),
            reasons,
            score,
            last_verified_at: item.last_verified_at.clone(),
            sensitivity: item.sensitivity,
            status: item.status,
            superseded_by: item.superseded_by.clone(),
        });
    }
    candidates.sort_by(|left, right| {
        Reverse(left.score)
            .cmp(&Reverse(right.score))
            .then_with(|| Reverse(&left.last_verified_at).cmp(&Reverse(&right.last_verified_at)))
            .then_with(|| left.memory_id.cmp(&right.memory_id))
    });
    let candidate_count = candidates.len();
    candidates.truncate(query.max_results as usize);
    let mut used = 0_u64;
    candidates.retain(|hit| {
        let Ok(bytes) = u64::try_from(hit.content.len()) else {
            return false;
        };
        let Some(next) = used.checked_add(bytes) else {
            return false;
        };
        if next > query.max_context_bytes {
            return false;
        }
        used = next;
        true
    });
    let omitted_count = u64::try_from(candidate_count.saturating_sub(candidates.len()))
        .map_err(|_| MemoryError::ResourceLimit)?;
    Ok(MemoryLoadResult {
        hits: candidates,
        omitted_count,
        cross_workspace_content: false,
    })
}

fn validate_query(query: &MemoryLoadQuery) -> Result<(), MemoryError> {
    if query.max_results == 0
        || query.max_results > MAX_LOAD_RESULTS
        || query.max_context_bytes == 0
        || query.max_context_bytes > MAX_LOAD_BYTES
        || query.relevance_terms.len() > MAX_QUERY_TERMS
        || query.relevance_terms.iter().any(|value| {
            value.is_empty()
                || value.len() > MAX_QUERY_ITEM_BYTES
                || value.chars().any(char::is_control)
                || crate::domain::secret_candidate(value)
        })
    {
        return Err(MemoryError::InvalidInput);
    }
    Ok(())
}

fn compaction_digest(candidates: &[MemoryCandidate]) -> Result<String, MemoryError> {
    let material = candidates
        .iter()
        .map(|candidate| {
            serde_json::to_vec(&(
                candidate.memory_id.as_str(),
                candidate.content.as_str(),
                candidate.confidence_bps,
                candidate.created_at.as_str(),
            ))
            .map_err(|_| MemoryError::ResourceLimit)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(sha256(&material.concat()))
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
    use std::collections::BTreeSet;

    use agentmage_kernel_contracts::{
        DataSensitivity, EvidenceId, EvidenceKind, EvidenceReference, WorkspaceId,
    };

    use super::{
        MemoryLoadQuery, MemoryLoadReason, WorkingCompactionProposal, WorkingMemory, select_memory,
    };
    use crate::{
        MemoryCandidate, MemoryError, MemoryId, MemoryItem, MemoryItemStatus, MemoryScope,
        MemoryType, UserMemoryDecision, evaluate_memory_candidate, resolve_memory_candidate,
    };

    fn evidence(id: &str) -> EvidenceReference {
        EvidenceReference {
            schema_version: 1,
            evidence_id: EvidenceId::from_raw(id),
            kind: EvidenceKind::Document,
            source_id: "fixture-source".to_owned(),
            object_id: id.to_owned(),
            fragment: None,
            content_sha256: "a".repeat(64),
            observed_revision: Some("revision-1".to_owned()),
        }
    }

    fn working(id: &str, content: &str) -> MemoryCandidate {
        MemoryCandidate {
            memory_id: MemoryId::parse(id).expect("valid identity"),
            memory_type: MemoryType::Working,
            scope: MemoryScope {
                workspace_id: WorkspaceId::from_raw("workspace-working"),
                project_id: Some("project-one".to_owned()),
                conversation_id: Some("conversation-one".to_owned()),
            },
            content: content.to_owned(),
            fact_key: None,
            tags: Vec::new(),
            links: Vec::new(),
            evidence: vec![evidence(&format!("evidence-{id}"))],
            sensitivity: DataSensitivity::Ephemeral,
            confidence_bps: 8_000,
            created_at: "2026-08-14T12:00:00Z".to_owned(),
            expires_at: None,
            inferred_sensitive: false,
            model_requested_promotion: false,
        }
    }

    fn durable(
        id: &str,
        content: &str,
        workspace: &str,
        project: &str,
        status: MemoryItemStatus,
    ) -> MemoryItem {
        let candidate = MemoryCandidate {
            memory_id: MemoryId::parse(id).expect("valid identity"),
            memory_type: MemoryType::Semantic,
            scope: MemoryScope {
                workspace_id: WorkspaceId::from_raw(workspace),
                project_id: Some(project.to_owned()),
                conversation_id: Some("conversation-one".to_owned()),
            },
            content: content.to_owned(),
            fact_key: Some(format!("fact.{id}")),
            tags: vec!["release".to_owned()],
            links: Vec::new(),
            evidence: vec![evidence(&format!("evidence-{id}"))],
            sensitivity: DataSensitivity::Durable,
            confidence_bps: 9_000,
            created_at: "2026-08-14T12:00:00Z".to_owned(),
            expires_at: None,
            inferred_sensitive: false,
            model_requested_promotion: false,
        };
        let policy = evaluate_memory_candidate(&candidate, &[]).expect("policy succeeds");
        let mut item = resolve_memory_candidate(
            &candidate,
            &policy,
            UserMemoryDecision::Approve,
            "b".repeat(64),
            "2026-08-14T12:01:00Z".to_owned(),
        )
        .expect("approval succeeds");
        item.status = status;
        item
    }

    #[test]
    fn working_memory_is_completely_previewed_and_strictly_bounded() {
        let mut memory = WorkingMemory::new(2, 64).expect("working memory creates");
        memory
            .insert(working("memory-working-1", "first context"))
            .expect("insert succeeds");
        memory
            .insert(working("memory-working-2", "second context"))
            .expect("insert succeeds");
        let preview = memory.preview_markdown().expect("preview succeeds");
        assert_eq!(preview.relative_path, "WORKING.md");
        assert!(preview.markdown.contains("first context"));
        assert!(preview.markdown.contains("second context"));
        assert!(!preview.write_enabled);
        assert_eq!(preview.content_sha256.len(), 64);

        assert_eq!(
            memory.insert(working("memory-working-3", "third context")),
            Err(MemoryError::ResourceLimit)
        );
        assert_eq!(memory.counts().0, 2);
        memory.clear();
        assert_eq!(memory.counts(), (0, 0));
    }

    #[test]
    fn compaction_only_emits_review_candidates_and_never_promotes_or_clears() {
        let source = working("memory-working-1", "Alice owns the release.");
        let source_evidence = source.evidence.clone();
        let mut memory = WorkingMemory::new(10, 1_000).expect("working memory creates");
        memory.insert(source).expect("insert succeeds");
        let preview = memory
            .compact_preview(&[WorkingCompactionProposal {
                source_memory_id: MemoryId::parse("memory-working-1").expect("valid identity"),
                candidate_memory_id: MemoryId::parse("memory-durable-1").expect("valid identity"),
                memory_type: MemoryType::Semantic,
                content: "Alice owns the release.".to_owned(),
                fact_key: Some("release.owner".to_owned()),
                tags: vec!["release".to_owned()],
                links: Vec::new(),
                confidence_bps: 8_000,
                expires_at: None,
            }])
            .expect("compaction preview succeeds");
        assert_eq!(preview.candidates.len(), 1);
        assert_eq!(preview.candidates[0].evidence, source_evidence);
        assert!(preview.candidates[0].model_requested_promotion);
        assert!(!preview.durable_memory_created);
        assert!(!preview.working_memory_cleared);
        assert!(!preview.files_written);
        assert_eq!(memory.counts().0, 1);

        let policy = evaluate_memory_candidate(&preview.candidates[0], &[])
            .expect("ordinary policy succeeds");
        assert!(policy.user_decision_required);
        assert!(!policy.automatic_promotion);
    }

    #[test]
    fn compaction_cannot_raise_confidence_create_working_or_capture_secrets() {
        let mut memory = WorkingMemory::new(10, 1_000).expect("working memory creates");
        memory
            .insert(working("memory-working-1", "safe context"))
            .expect("insert succeeds");
        let proposal = |memory_type, content: &str, confidence| WorkingCompactionProposal {
            source_memory_id: MemoryId::parse("memory-working-1").expect("valid identity"),
            candidate_memory_id: MemoryId::parse("memory-candidate-1").expect("valid identity"),
            memory_type,
            content: content.to_owned(),
            fact_key: Some("safe.fact".to_owned()),
            tags: Vec::new(),
            links: Vec::new(),
            confidence_bps: confidence,
            expires_at: None,
        };
        assert_eq!(
            memory.compact_preview(&[proposal(MemoryType::Working, "safe", 8_000)]),
            Err(MemoryError::InvalidInput)
        );
        assert_eq!(
            memory.compact_preview(&[proposal(MemoryType::Semantic, "safe", 9_000)]),
            Err(MemoryError::InvalidInput)
        );
        assert_eq!(
            memory.compact_preview(&[proposal(MemoryType::Semantic, "password=hidden", 8_000,)]),
            Err(MemoryError::IneligibleCandidate)
        );
        assert_eq!(memory.counts().0, 1);
    }

    #[test]
    fn selective_loading_explains_scope_tag_source_relevance_and_type() {
        let approved = durable(
            "memory-approved",
            "Alice owns the release milestone.",
            "workspace-working",
            "project-one",
            MemoryItemStatus::Approved,
        );
        let other_project = durable(
            "memory-other-project",
            "Alice owns another release.",
            "workspace-working",
            "project-two",
            MemoryItemStatus::Approved,
        );
        let other_workspace = durable(
            "memory-other-workspace",
            "Alice owns a private release.",
            "workspace-other",
            "project-one",
            MemoryItemStatus::Approved,
        );
        let historical = durable(
            "memory-historical",
            "Alice previously owned the release.",
            "workspace-working",
            "project-one",
            MemoryItemStatus::Superseded,
        );
        let query = MemoryLoadQuery {
            workspace_id: WorkspaceId::from_raw("workspace-working"),
            project_id: Some("project-one".to_owned()),
            conversation_id: Some("conversation-one".to_owned()),
            memory_types: [MemoryType::Semantic].into_iter().collect(),
            tags: ["release".to_owned()].into_iter().collect(),
            links: BTreeSet::new(),
            evidence_ids: ["evidence-memory-approved".to_owned()]
                .into_iter()
                .collect(),
            relevance_terms: vec!["release milestone".to_owned()],
            include_historical: false,
            max_results: 10,
            max_context_bytes: 1_000,
        };
        let result = select_memory(
            &[approved, other_project, other_workspace, historical],
            &query,
        )
        .expect("selection succeeds");
        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.hits[0].memory_id.as_str(), "memory-approved");
        assert_eq!(
            result.hits[0].reasons,
            [
                MemoryLoadReason::Workspace,
                MemoryLoadReason::Project,
                MemoryLoadReason::Conversation,
                MemoryLoadReason::MemoryType,
                MemoryLoadReason::Tag,
                MemoryLoadReason::Source,
                MemoryLoadReason::Relevance,
            ]
        );
        assert!(!result.cross_workspace_content);
    }

    #[test]
    fn historical_and_context_budgets_are_explicit_and_stable() {
        let current = durable(
            "memory-current",
            "release current",
            "workspace-working",
            "project-one",
            MemoryItemStatus::Approved,
        );
        let historical = durable(
            "memory-historical",
            "release historical",
            "workspace-working",
            "project-one",
            MemoryItemStatus::Superseded,
        );
        let mut query = MemoryLoadQuery {
            workspace_id: WorkspaceId::from_raw("workspace-working"),
            project_id: Some("project-one".to_owned()),
            conversation_id: None,
            memory_types: BTreeSet::new(),
            tags: BTreeSet::new(),
            links: BTreeSet::new(),
            evidence_ids: BTreeSet::new(),
            relevance_terms: vec!["release".to_owned()],
            include_historical: true,
            max_results: 10,
            max_context_bytes: 16,
        };
        let result =
            select_memory(&[historical.clone(), current], &query).expect("selection succeeds");
        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.omitted_count, 1);

        query.include_historical = false;
        query.max_context_bytes = 1_000;
        let result = select_memory(&[historical], &query).expect("selection succeeds");
        assert!(result.hits.is_empty());
    }
}
