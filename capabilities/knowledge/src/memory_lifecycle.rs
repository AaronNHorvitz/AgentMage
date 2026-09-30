//! Explicit memory lifecycle transitions and human-readable Markdown previews.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use agentmage_kernel_contracts::WorkspaceId;
use sha2::{Digest, Sha256};

use crate::{MemoryError, MemoryId, MemoryItem, MemoryItemStatus, MemoryType};

const MAX_ITEMS: usize = 100_000;
const MAX_SOURCE_ID_BYTES: usize = 512;

/// Closed explicit memory transition class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryTransitionKind {
    /// Add an already decided candidate result.
    Insert,
    /// Edit through a new replacement identity while retaining the original.
    Edit,
    /// Replace a current fact while retaining the original.
    Supersede,
    /// Correct a current fact while retaining the original.
    Correct,
    /// Lower confidence through an explicit evidence-bound policy decision.
    Decay,
    /// Place an explicit user hold.
    Hold,
    /// Apply an exact expiry policy.
    Expire,
    /// Tombstone one item after an explicit deletion decision.
    Delete,
    /// Withdraw one item, or every item citing a revoked source, after an
    /// explicit revocation decision.
    Revoke,
}

/// One revoked source within one workspace (Decision 0124). An item cites it
/// when any of its evidence names this source, and this object when one is
/// given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemorySourceRevocation {
    /// The only workspace whose items the revocation reaches.
    pub workspace_id: WorkspaceId,
    /// Revoked evidence source identity.
    pub source_id: String,
    /// Optional revoked object within the source; absent revokes the source.
    pub object_id: Option<String>,
}

/// Content-free receipt for one source revocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemorySourceRevocationReceipt {
    /// Monotonic catalog revision of the single transition.
    pub revision: u64,
    /// Digest of the revoked workspace, source and object identities.
    pub source_sha256: String,
    /// Digests of every revoked item identity, in identity order.
    pub revoked_memory_id_sha256s: Vec<String>,
    /// Digest of the explicit revocation decision evidence.
    pub decision_sha256: String,
    /// Digest of the complete ordered catalog after the transition.
    pub catalog_sha256: String,
    /// Fixed false automatic-decision marker.
    pub automatic_decision: bool,
    /// Fixed false file-write marker.
    pub files_written: bool,
}

/// Content-free receipt for one catalog transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryLifecycleReceipt {
    /// Monotonic catalog revision.
    pub revision: u64,
    /// Closed transition class.
    pub transition: MemoryTransitionKind,
    /// Digest of the affected stable identity.
    pub memory_id_sha256: String,
    /// Optional digest of a replacement identity.
    pub replacement_id_sha256: Option<String>,
    /// Digest of the explicit decision or expiry policy evidence.
    pub decision_sha256: String,
    /// Digest of the complete ordered catalog after transition.
    pub catalog_sha256: String,
    /// Fixed false automatic-decision marker.
    pub automatic_decision: bool,
    /// Fixed false file-write marker.
    pub files_written: bool,
}

/// Content-free current catalog summary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryCatalogSummary {
    /// Monotonic catalog revision.
    pub revision: u64,
    /// Total current and historical item count.
    pub item_count: u64,
    /// Current approved or held item count.
    pub current_count: u64,
    /// Digest of the complete ordered catalog.
    pub catalog_sha256: String,
}

/// One deterministic Markdown file preview.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryMarkdownFile {
    /// Portable relative path with no machine-specific prefix.
    pub relative_path: String,
    /// Complete canonical preview text.
    pub markdown: String,
    /// Digest of the complete preview bytes.
    pub content_sha256: String,
    /// Fixed false apply marker.
    pub write_enabled: bool,
}

/// Complete portable `MEMORY.md` and per-topic preview bundle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryMarkdownBundle {
    /// Compact linked memory index.
    pub index: MemoryMarkdownFile,
    /// Per-topic files in stable identity order.
    pub topics: Vec<MemoryMarkdownFile>,
    /// Digest of all ordered preview path and content identities.
    pub bundle_sha256: String,
    /// Fixed false apply marker.
    pub write_enabled: bool,
}

/// In-memory current and historical memory catalog with no filesystem handle.
pub struct MemoryCatalog {
    revision: u64,
    items: BTreeMap<MemoryId, MemoryItem>,
    catalog_sha256: String,
}

impl MemoryCatalog {
    /// Creates an empty memory catalog.
    #[must_use]
    pub fn new() -> Self {
        Self {
            revision: 0,
            items: BTreeMap::new(),
            catalog_sha256: sha256(&[]),
        }
    }

    /// Inserts one explicitly approved or rejected candidate result.
    pub fn insert(&mut self, item: MemoryItem) -> Result<MemoryLifecycleReceipt, MemoryError> {
        validate_item(&item)?;
        if self.items.len() >= MAX_ITEMS {
            return Err(MemoryError::ResourceLimit);
        }
        if self.items.contains_key(&item.memory_id) {
            return Err(MemoryError::DuplicateIdentity);
        }
        let memory_id = item.memory_id.clone();
        let decision_sha256 = item.decision_sha256.clone();
        self.publish(
            MemoryTransitionKind::Insert,
            memory_id,
            None,
            decision_sha256,
            |next| {
                next.insert(item.memory_id.clone(), item);
                Ok(())
            },
        )
    }

    /// Supersedes one approved current item with another approved exact item.
    pub fn supersede(
        &mut self,
        current_id: &MemoryId,
        replacement: MemoryItem,
        decision_sha256: String,
        decided_at: String,
    ) -> Result<MemoryLifecycleReceipt, MemoryError> {
        self.replace(
            MemoryTransitionKind::Supersede,
            current_id,
            replacement,
            decision_sha256,
            decided_at,
        )
    }

    /// Edits one approved item through an independently approved replacement identity.
    pub fn edit(
        &mut self,
        current_id: &MemoryId,
        replacement: MemoryItem,
        decision_sha256: String,
        decided_at: String,
    ) -> Result<MemoryLifecycleReceipt, MemoryError> {
        self.replace(
            MemoryTransitionKind::Edit,
            current_id,
            replacement,
            decision_sha256,
            decided_at,
        )
    }

    /// Corrects one approved current item with another approved exact item.
    pub fn correct(
        &mut self,
        current_id: &MemoryId,
        replacement: MemoryItem,
        decision_sha256: String,
        decided_at: String,
    ) -> Result<MemoryLifecycleReceipt, MemoryError> {
        self.replace(
            MemoryTransitionKind::Correct,
            current_id,
            replacement,
            decision_sha256,
            decided_at,
        )
    }

    /// Lowers confidence without changing content, source evidence, or identity.
    pub fn decay_confidence(
        &mut self,
        memory_id: &MemoryId,
        new_confidence_bps: u32,
        decision_sha256: String,
        decided_at: String,
    ) -> Result<MemoryLifecycleReceipt, MemoryError> {
        validate_decision(&decision_sha256, &decided_at)?;
        self.publish(
            MemoryTransitionKind::Decay,
            memory_id.clone(),
            None,
            decision_sha256.clone(),
            |next| {
                let item = next.get_mut(memory_id).ok_or(MemoryError::NotFound)?;
                if !matches!(
                    item.status,
                    MemoryItemStatus::Approved | MemoryItemStatus::Hold
                ) || new_confidence_bps >= item.confidence_bps
                {
                    return Err(MemoryError::InvalidTransition);
                }
                item.confidence_bps = new_confidence_bps;
                item.last_verified_at = decided_at.clone();
                item.decided_at = decided_at;
                item.decision_sha256 = decision_sha256;
                Ok(())
            },
        )
    }

    /// Places an explicit user hold on a current approved item.
    pub fn hold(
        &mut self,
        memory_id: &MemoryId,
        decision_sha256: String,
        decided_at: String,
    ) -> Result<MemoryLifecycleReceipt, MemoryError> {
        validate_decision(&decision_sha256, &decided_at)?;
        self.publish(
            MemoryTransitionKind::Hold,
            memory_id.clone(),
            None,
            decision_sha256.clone(),
            |next| {
                let item = next.get_mut(memory_id).ok_or(MemoryError::NotFound)?;
                if item.status != MemoryItemStatus::Approved {
                    return Err(MemoryError::InvalidTransition);
                }
                item.status = MemoryItemStatus::Hold;
                item.decided_at = decided_at;
                item.decision_sha256 = decision_sha256;
                Ok(())
            },
        )
    }

    /// Applies exact expiry to eligible approved items; held items remain current.
    pub fn expire(
        &mut self,
        as_of: String,
        policy_sha256: String,
    ) -> Result<Vec<MemoryLifecycleReceipt>, MemoryError> {
        validate_decision(&policy_sha256, &as_of)?;
        let identities = self
            .items
            .values()
            .filter(|item| {
                item.status == MemoryItemStatus::Approved
                    && item
                        .expires_at
                        .as_ref()
                        .is_some_and(|expires| expires <= &as_of)
            })
            .map(|item| item.memory_id.clone())
            .collect::<Vec<_>>();
        let mut receipts = Vec::new();
        for identity in identities {
            receipts.push(self.publish(
                MemoryTransitionKind::Expire,
                identity.clone(),
                None,
                policy_sha256.clone(),
                |next| {
                    let item = next.get_mut(&identity).ok_or(MemoryError::NotFound)?;
                    item.status = MemoryItemStatus::Expired;
                    item.decided_at = as_of.clone();
                    item.decision_sha256 = policy_sha256.clone();
                    Ok(())
                },
            )?);
        }
        Ok(receipts)
    }

    /// Tombstones one current or historical item after an explicit decision.
    pub fn delete(
        &mut self,
        memory_id: &MemoryId,
        decision_sha256: String,
        decided_at: String,
    ) -> Result<MemoryLifecycleReceipt, MemoryError> {
        validate_decision(&decision_sha256, &decided_at)?;
        self.publish(
            MemoryTransitionKind::Delete,
            memory_id.clone(),
            None,
            decision_sha256.clone(),
            |next| {
                let item = next.get_mut(memory_id).ok_or(MemoryError::NotFound)?;
                if item.status == MemoryItemStatus::Deleted {
                    return Err(MemoryError::InvalidTransition);
                }
                item.status = MemoryItemStatus::Deleted;
                item.content = None;
                item.tags.clear();
                item.links.clear();
                item.decided_at = decided_at;
                item.decision_sha256 = decision_sha256;
                Ok(())
            },
        )
    }

    /// Revokes one item after an explicit decision (Decision 0124). The item
    /// keeps its content for inspection and a later deletion, and it is never
    /// loaded again, current or historical.
    pub fn revoke(
        &mut self,
        memory_id: &MemoryId,
        decision_sha256: String,
        decided_at: String,
    ) -> Result<MemoryLifecycleReceipt, MemoryError> {
        validate_decision(&decision_sha256, &decided_at)?;
        self.publish(
            MemoryTransitionKind::Revoke,
            memory_id.clone(),
            None,
            decision_sha256.clone(),
            |next| {
                let item = next.get_mut(memory_id).ok_or(MemoryError::NotFound)?;
                if !revocable(item.status) {
                    return Err(MemoryError::InvalidTransition);
                }
                item.status = MemoryItemStatus::Revoked;
                item.decided_at = decided_at;
                item.decision_sha256 = decision_sha256;
                Ok(())
            },
        )
    }

    /// Revokes, in one transition, every revocable item of the named
    /// workspace that cites the revoked source (Decision 0124). Items of
    /// other workspaces are never touched. A revocation no item reaches
    /// changes nothing and is refused as not found.
    pub fn revoke_source(
        &mut self,
        source: &MemorySourceRevocation,
        decision_sha256: String,
        decided_at: String,
    ) -> Result<MemorySourceRevocationReceipt, MemoryError> {
        validate_decision(&decision_sha256, &decided_at)?;
        let source_sha256 = source_digest(source)?;
        let revoked = self
            .items
            .values()
            .filter(|item| {
                revocable(item.status)
                    && item.scope.workspace_id == source.workspace_id
                    && item.evidence.iter().any(|evidence| {
                        evidence.source_id == source.source_id
                            && source
                                .object_id
                                .as_ref()
                                .is_none_or(|object| &evidence.object_id == object)
                    })
            })
            .map(|item| item.memory_id.clone())
            .collect::<Vec<_>>();
        if revoked.is_empty() {
            return Err(MemoryError::NotFound);
        }
        let mut next = self.items.clone();
        for memory_id in &revoked {
            let item = next.get_mut(memory_id).ok_or(MemoryError::NotFound)?;
            item.status = MemoryItemStatus::Revoked;
            item.decided_at.clone_from(&decided_at);
            item.decision_sha256.clone_from(&decision_sha256);
        }
        validate_catalog(&next)?;
        let catalog_sha256 = catalog_digest(&next)?;
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(MemoryError::ResourceLimit)?;
        self.items = next;
        self.revision = revision;
        self.catalog_sha256 = catalog_sha256.clone();
        Ok(MemorySourceRevocationReceipt {
            revision,
            source_sha256,
            revoked_memory_id_sha256s: revoked
                .iter()
                .map(|memory_id| sha256(memory_id.as_str().as_bytes()))
                .collect(),
            decision_sha256,
            catalog_sha256,
            automatic_decision: false,
            files_written: false,
        })
    }

    /// Returns a content-free summary.
    #[must_use]
    pub fn inspect(&self) -> MemoryCatalogSummary {
        MemoryCatalogSummary {
            revision: self.revision,
            item_count: self.items.len() as u64,
            current_count: self
                .items
                .values()
                .filter(|item| {
                    matches!(
                        item.status,
                        MemoryItemStatus::Approved | MemoryItemStatus::Hold
                    )
                })
                .count() as u64,
            catalog_sha256: self.catalog_sha256.clone(),
        }
    }

    /// Returns one exact item for inspection.
    #[must_use]
    pub fn get(&self, memory_id: &MemoryId) -> Option<&MemoryItem> {
        self.items.get(memory_id)
    }

    /// Returns a stable cloned snapshot for bounded retrieval and export preparation.
    #[must_use]
    pub fn items(&self) -> Vec<MemoryItem> {
        self.items.values().cloned().collect()
    }

    pub(crate) fn from_portable_parts(
        revision: u64,
        items: Vec<MemoryItem>,
        expected_catalog_sha256: &str,
    ) -> Result<Self, MemoryError> {
        let item_count = u64::try_from(items.len()).map_err(|_| MemoryError::ResourceLimit)?;
        if !valid_sha256(expected_catalog_sha256)
            || (items.is_empty() && revision != 0)
            || (!items.is_empty() && revision == 0)
            || revision < item_count
            || items.len() > MAX_ITEMS
        {
            return Err(MemoryError::InvalidInput);
        }
        let mut by_identity = BTreeMap::new();
        for item in items {
            if by_identity.insert(item.memory_id.clone(), item).is_some() {
                return Err(MemoryError::DuplicateIdentity);
            }
        }
        validate_catalog(&by_identity)?;
        let catalog_sha256 = catalog_digest(&by_identity)?;
        if catalog_sha256 != expected_catalog_sha256 {
            return Err(MemoryError::DecisionDrift);
        }
        Ok(Self {
            revision,
            items: by_identity,
            catalog_sha256,
        })
    }

    /// Renders portable deterministic Markdown previews without applying them.
    pub fn preview_markdown(&self) -> Result<MemoryMarkdownBundle, MemoryError> {
        let mut topics = Vec::new();
        for item in self.items.values() {
            topics.push(render_topic(item)?);
        }
        let mut index_markdown = String::from("# Memory\n\n");
        index_markdown.push_str("## Current\n\n");
        for item in self.items.values().filter(|item| {
            matches!(
                item.status,
                MemoryItemStatus::Approved | MemoryItemStatus::Hold
            )
        }) {
            writeln!(
                &mut index_markdown,
                "- [[{}|{}]] #{}",
                topic_stem(&item.memory_id),
                item.memory_id.as_str(),
                memory_type_id(item.memory_type)
            )
            .map_err(|_| MemoryError::ResourceLimit)?;
        }
        index_markdown.push_str("\n## Historical\n\n");
        for item in self.items.values().filter(|item| {
            !matches!(
                item.status,
                MemoryItemStatus::Approved | MemoryItemStatus::Hold
            )
        }) {
            writeln!(
                &mut index_markdown,
                "- [[{}|{}]] ({})",
                topic_stem(&item.memory_id),
                item.memory_id.as_str(),
                status_id(item.status)
            )
            .map_err(|_| MemoryError::ResourceLimit)?;
        }
        let index = MemoryMarkdownFile {
            relative_path: "MEMORY.md".to_owned(),
            content_sha256: sha256(index_markdown.as_bytes()),
            markdown: index_markdown,
            write_enabled: false,
        };
        let bundle_sha256 = digest_bundle(&index, &topics);
        Ok(MemoryMarkdownBundle {
            index,
            topics,
            bundle_sha256,
            write_enabled: false,
        })
    }

    fn replace(
        &mut self,
        transition: MemoryTransitionKind,
        current_id: &MemoryId,
        replacement: MemoryItem,
        decision_sha256: String,
        decided_at: String,
    ) -> Result<MemoryLifecycleReceipt, MemoryError> {
        validate_item(&replacement)?;
        validate_decision(&decision_sha256, &decided_at)?;
        if replacement.status != MemoryItemStatus::Approved
            || replacement.memory_id == *current_id
            || self.items.contains_key(&replacement.memory_id)
        {
            return Err(MemoryError::InvalidTransition);
        }
        let current = self.items.get(current_id).ok_or(MemoryError::NotFound)?;
        if current.status != MemoryItemStatus::Approved
            || current.scope != replacement.scope
            || current.fact_key != replacement.fact_key
        {
            return Err(MemoryError::InvalidTransition);
        }
        let replacement_id = replacement.memory_id.clone();
        self.publish(
            transition,
            current_id.clone(),
            Some(replacement_id.clone()),
            decision_sha256.clone(),
            |next| {
                let prior = next.get_mut(current_id).ok_or(MemoryError::NotFound)?;
                prior.status = MemoryItemStatus::Superseded;
                prior.superseded_by = Some(replacement_id.clone());
                prior.decided_at = decided_at;
                prior.decision_sha256 = decision_sha256;
                next.insert(replacement_id, replacement);
                Ok(())
            },
        )
    }

    fn publish<F>(
        &mut self,
        transition: MemoryTransitionKind,
        memory_id: MemoryId,
        replacement_id: Option<MemoryId>,
        decision_sha256: String,
        apply: F,
    ) -> Result<MemoryLifecycleReceipt, MemoryError>
    where
        F: FnOnce(&mut BTreeMap<MemoryId, MemoryItem>) -> Result<(), MemoryError>,
    {
        let mut next = self.items.clone();
        apply(&mut next)?;
        validate_catalog(&next)?;
        let catalog_sha256 = catalog_digest(&next)?;
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(MemoryError::ResourceLimit)?;
        self.items = next;
        self.revision = revision;
        self.catalog_sha256 = catalog_sha256.clone();
        Ok(MemoryLifecycleReceipt {
            revision,
            transition,
            memory_id_sha256: sha256(memory_id.as_str().as_bytes()),
            replacement_id_sha256: replacement_id.map(|value| sha256(value.as_str().as_bytes())),
            decision_sha256,
            catalog_sha256,
            automatic_decision: false,
            files_written: false,
        })
    }
}

impl Default for MemoryCatalog {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_catalog(items: &BTreeMap<MemoryId, MemoryItem>) -> Result<(), MemoryError> {
    if items.len() > MAX_ITEMS {
        return Err(MemoryError::ResourceLimit);
    }
    for item in items.values() {
        validate_item(item)?;
        if let Some(target) = &item.superseded_by
            && (!items.contains_key(target) || target == &item.memory_id)
        {
            return Err(MemoryError::InvalidTransition);
        }
        if item.links.iter().any(|target| !items.contains_key(target)) {
            return Err(MemoryError::InvalidTransition);
        }
    }
    Ok(())
}

fn validate_item(item: &MemoryItem) -> Result<(), MemoryError> {
    if !valid_sha256(&item.candidate_sha256)
        || !valid_sha256(&item.decision_sha256)
        || item.confidence_bps > 10_000
        || item.last_verified_at.is_empty()
        || item.created_at.is_empty()
        || item.decided_at.is_empty()
        || item.content.as_ref().is_some_and(|content| {
            content.is_empty()
                || content.chars().any(char::is_control)
                || crate::domain::secret_candidate(content)
        })
        || matches!(
            item.status,
            MemoryItemStatus::Approved | MemoryItemStatus::Hold | MemoryItemStatus::Revoked
        ) && item.content.is_none()
        || matches!(
            item.status,
            MemoryItemStatus::Rejected | MemoryItemStatus::Deleted
        ) && item.content.is_some()
        || item.status == MemoryItemStatus::Candidate
        || item.status == MemoryItemStatus::Superseded && item.superseded_by.is_none()
        || matches!(
            item.status,
            MemoryItemStatus::Approved
                | MemoryItemStatus::Rejected
                | MemoryItemStatus::Expired
                | MemoryItemStatus::Hold
        ) && item.superseded_by.is_some()
    {
        return Err(MemoryError::InvalidInput);
    }
    Ok(())
}

fn validate_decision(decision_sha256: &str, decided_at: &str) -> Result<(), MemoryError> {
    if !valid_sha256(decision_sha256) || decided_at.is_empty() {
        return Err(MemoryError::InvalidInput);
    }
    Ok(())
}

fn render_topic(item: &MemoryItem) -> Result<MemoryMarkdownFile, MemoryError> {
    let mut markdown = String::from("---\n");
    writeln!(&mut markdown, "memory_id: {}", item.memory_id.as_str())
        .map_err(|_| MemoryError::ResourceLimit)?;
    writeln!(
        &mut markdown,
        "memory_type: {}",
        memory_type_id(item.memory_type)
    )
    .map_err(|_| MemoryError::ResourceLimit)?;
    writeln!(&mut markdown, "status: {}", status_id(item.status))
        .map_err(|_| MemoryError::ResourceLimit)?;
    writeln!(&mut markdown, "confidence_bps: {}", item.confidence_bps)
        .map_err(|_| MemoryError::ResourceLimit)?;
    writeln!(&mut markdown, "created_at: {}", item.created_at)
        .map_err(|_| MemoryError::ResourceLimit)?;
    writeln!(&mut markdown, "last_verified_at: {}", item.last_verified_at)
        .map_err(|_| MemoryError::ResourceLimit)?;
    writeln!(&mut markdown, "candidate_sha256: {}", item.candidate_sha256)
        .map_err(|_| MemoryError::ResourceLimit)?;
    writeln!(&mut markdown, "decision_sha256: {}", item.decision_sha256)
        .map_err(|_| MemoryError::ResourceLimit)?;
    markdown.push_str("tags:");
    if item.tags.is_empty() {
        markdown.push_str(" []\n");
    } else {
        markdown.push('\n');
        for tag in &item.tags {
            writeln!(&mut markdown, "  - {tag}").map_err(|_| MemoryError::ResourceLimit)?;
        }
    }
    markdown.push_str("links:");
    if item.links.is_empty() {
        markdown.push_str(" []\n");
    } else {
        markdown.push('\n');
        for link in &item.links {
            writeln!(&mut markdown, "  - {}", link.as_str())
                .map_err(|_| MemoryError::ResourceLimit)?;
        }
    }
    markdown.push_str("---\n\n");
    writeln!(&mut markdown, "# {}", item.memory_id.as_str())
        .map_err(|_| MemoryError::ResourceLimit)?;
    if let Some(content) = &item.content {
        markdown.push('\n');
        markdown.push_str(content);
        markdown.push('\n');
    }
    if let Some(target) = &item.superseded_by {
        writeln!(&mut markdown, "\nSuperseded by [[{}]].", topic_stem(target))
            .map_err(|_| MemoryError::ResourceLimit)?;
    }
    let relative_path = format!("Memory/{}.md", topic_stem(&item.memory_id));
    Ok(MemoryMarkdownFile {
        relative_path,
        content_sha256: sha256(markdown.as_bytes()),
        markdown,
        write_enabled: false,
    })
}

fn catalog_digest(items: &BTreeMap<MemoryId, MemoryItem>) -> Result<String, MemoryError> {
    let values = items
        .values()
        .map(|item| {
            serde_json::to_vec(&CatalogDigestItem {
                memory_id: item.memory_id.as_str(),
                memory_type: memory_type_id(item.memory_type),
                status: status_id(item.status),
                workspace_id: item.scope.workspace_id.as_str(),
                project_id: item.scope.project_id.as_deref(),
                conversation_id: item.scope.conversation_id.as_deref(),
                content: item.content.as_deref(),
                fact_key: item.fact_key.as_deref(),
                tags: &item.tags,
                links: item.links.iter().map(MemoryId::as_str).collect(),
                evidence: &item.evidence,
                sensitivity: item.sensitivity,
                confidence_bps: item.confidence_bps,
                created_at: &item.created_at,
                decided_at: &item.decided_at,
                last_verified_at: &item.last_verified_at,
                expires_at: item.expires_at.as_deref(),
                candidate_sha256: &item.candidate_sha256,
                decision_sha256: &item.decision_sha256,
                superseded_by: item.superseded_by.as_ref().map(MemoryId::as_str),
            })
            .map_err(|_| MemoryError::ResourceLimit)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(sha256(&values.concat()))
}

#[derive(serde::Serialize)]
struct CatalogDigestItem<'item> {
    memory_id: &'item str,
    memory_type: &'static str,
    status: &'static str,
    workspace_id: &'item str,
    project_id: Option<&'item str>,
    conversation_id: Option<&'item str>,
    content: Option<&'item str>,
    fact_key: Option<&'item str>,
    tags: &'item [String],
    links: Vec<&'item str>,
    evidence: &'item [agentmage_kernel_contracts::EvidenceReference],
    sensitivity: agentmage_kernel_contracts::DataSensitivity,
    confidence_bps: u32,
    created_at: &'item str,
    decided_at: &'item str,
    last_verified_at: &'item str,
    expires_at: Option<&'item str>,
    candidate_sha256: &'item str,
    decision_sha256: &'item str,
    superseded_by: Option<&'item str>,
}

fn digest_bundle(index: &MemoryMarkdownFile, topics: &[MemoryMarkdownFile]) -> String {
    let mut material = Vec::new();
    material.extend_from_slice(index.relative_path.as_bytes());
    material.extend_from_slice(index.content_sha256.as_bytes());
    for topic in topics {
        material.extend_from_slice(topic.relative_path.as_bytes());
        material.extend_from_slice(topic.content_sha256.as_bytes());
    }
    sha256(&material)
}

fn topic_stem(memory_id: &MemoryId) -> String {
    memory_id.as_str().to_owned()
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

const fn status_id(value: MemoryItemStatus) -> &'static str {
    match value {
        MemoryItemStatus::Candidate => "candidate",
        MemoryItemStatus::Approved => "approved",
        MemoryItemStatus::Rejected => "rejected",
        MemoryItemStatus::Superseded => "superseded",
        MemoryItemStatus::Expired => "expired",
        MemoryItemStatus::Hold => "hold",
        MemoryItemStatus::Deleted => "deleted",
        MemoryItemStatus::Revoked => "revoked",
    }
}

/// Whether an item may be revoked: every item that still holds content and is
/// not already revoked.
const fn revocable(status: MemoryItemStatus) -> bool {
    matches!(
        status,
        MemoryItemStatus::Approved
            | MemoryItemStatus::Hold
            | MemoryItemStatus::Expired
            | MemoryItemStatus::Superseded
    )
}

fn source_digest(source: &MemorySourceRevocation) -> Result<String, MemoryError> {
    let bounded = |value: &str| {
        !value.is_empty()
            && value.len() <= MAX_SOURCE_ID_BYTES
            && !value.chars().any(char::is_control)
    };
    if !bounded(source.workspace_id.as_str())
        || !bounded(&source.source_id)
        || source
            .object_id
            .as_deref()
            .is_some_and(|object| !bounded(object))
    {
        return Err(MemoryError::InvalidInput);
    }
    let material = serde_json::to_vec(&(
        source.workspace_id.as_str(),
        source.source_id.as_str(),
        source.object_id.as_deref(),
    ))
    .map_err(|_| MemoryError::ResourceLimit)?;
    Ok(sha256(&material))
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

    use super::{MemoryCatalog, MemorySourceRevocation, MemoryTransitionKind};
    use crate::{
        MemoryCandidate, MemoryError, MemoryId, MemoryItem, MemoryItemStatus, MemoryScope,
        MemoryType, UserMemoryDecision, evaluate_memory_candidate, resolve_memory_candidate,
    };

    fn item(id: &str, content: &str, expires_at: Option<&str>) -> MemoryItem {
        let candidate = MemoryCandidate {
            memory_id: MemoryId::parse(id).expect("valid identity"),
            memory_type: MemoryType::Semantic,
            scope: MemoryScope {
                workspace_id: WorkspaceId::from_raw("workspace-memory-lifecycle"),
                project_id: Some("project-one".to_owned()),
                conversation_id: None,
            },
            content: content.to_owned(),
            fact_key: Some("release.owner".to_owned()),
            tags: vec!["release".to_owned()],
            links: Vec::new(),
            evidence: vec![EvidenceReference {
                schema_version: 1,
                evidence_id: EvidenceId::from_raw(format!("evidence-{id}")),
                kind: EvidenceKind::Document,
                source_id: "fixture-source".to_owned(),
                object_id: id.to_owned(),
                fragment: None,
                content_sha256: "a".repeat(64),
                observed_revision: Some("revision-1".to_owned()),
            }],
            sensitivity: DataSensitivity::Durable,
            confidence_bps: 9_000,
            created_at: "2026-08-14T12:00:00Z".to_owned(),
            expires_at: expires_at.map(str::to_owned),
            inferred_sensitive: false,
            model_requested_promotion: false,
        };
        let policy = evaluate_memory_candidate(&candidate, &[]).expect("policy succeeds");
        resolve_memory_candidate(
            &candidate,
            &policy,
            UserMemoryDecision::Approve,
            "b".repeat(64),
            "2026-08-14T12:01:00Z".to_owned(),
        )
        .expect("approval succeeds")
    }

    #[test]
    fn insert_supersede_and_correction_preserve_history_atomically() {
        let original = item("memory-original", "The owner is Alice.", None);
        let original_id = original.memory_id.clone();
        let mut catalog = MemoryCatalog::new();
        let insertion = catalog.insert(original.clone()).expect("insert succeeds");
        assert_eq!(insertion.transition, MemoryTransitionKind::Insert);
        assert!(!insertion.automatic_decision);
        assert!(!insertion.files_written);

        let before_duplicate = catalog.inspect();
        assert_eq!(
            catalog.insert(original),
            Err(MemoryError::DuplicateIdentity)
        );
        assert_eq!(catalog.inspect(), before_duplicate);

        let replacement = item("memory-replacement", "The owner is Bob.", None);
        let replacement_id = replacement.memory_id.clone();
        let supersession = catalog
            .supersede(
                &original_id,
                replacement,
                "c".repeat(64),
                "2026-08-14T12:02:00Z".to_owned(),
            )
            .expect("supersession succeeds");
        assert_eq!(supersession.transition, MemoryTransitionKind::Supersede);
        assert_eq!(
            catalog.get(&original_id).expect("original retained").status,
            MemoryItemStatus::Superseded
        );
        assert_eq!(
            catalog
                .get(&original_id)
                .expect("original retained")
                .superseded_by
                .as_ref(),
            Some(&replacement_id)
        );
        assert_eq!(
            catalog
                .get(&replacement_id)
                .expect("replacement present")
                .status,
            MemoryItemStatus::Approved
        );

        let wrong_fact = {
            let mut value = item("memory-wrong-fact", "Different fact.", None);
            value.fact_key = Some("different.fact".to_owned());
            value
        };
        let before_failure = catalog.inspect();
        assert_eq!(
            catalog.correct(
                &replacement_id,
                wrong_fact,
                "d".repeat(64),
                "2026-08-14T12:03:00Z".to_owned(),
            ),
            Err(MemoryError::InvalidTransition)
        );
        assert_eq!(catalog.inspect(), before_failure);
    }

    #[test]
    fn hold_expiry_and_delete_are_explicit_and_content_free() {
        let expiring = item(
            "memory-expiring",
            "Expiring fact.",
            Some("2026-08-15T12:00:00Z"),
        );
        let held = item("memory-held", "Held fact.", Some("2026-08-15T12:00:00Z"));
        let expiring_id = expiring.memory_id.clone();
        let held_id = held.memory_id.clone();
        let mut catalog = MemoryCatalog::new();
        catalog.insert(expiring).expect("insert succeeds");
        catalog.insert(held).expect("insert succeeds");
        let hold = catalog
            .hold(&held_id, "c".repeat(64), "2026-08-14T13:00:00Z".to_owned())
            .expect("hold succeeds");
        assert_eq!(hold.transition, MemoryTransitionKind::Hold);

        let expirations = catalog
            .expire("2026-08-16T12:00:00Z".to_owned(), "d".repeat(64))
            .expect("expiry succeeds");
        assert_eq!(expirations.len(), 1);
        assert_eq!(
            catalog.get(&expiring_id).expect("item present").status,
            MemoryItemStatus::Expired
        );
        assert_eq!(
            catalog.get(&held_id).expect("item present").status,
            MemoryItemStatus::Hold
        );

        let deletion = catalog
            .delete(&held_id, "e".repeat(64), "2026-08-16T13:00:00Z".to_owned())
            .expect("delete succeeds");
        assert_eq!(deletion.transition, MemoryTransitionKind::Delete);
        let deleted = catalog.get(&held_id).expect("tombstone present");
        assert_eq!(deleted.status, MemoryItemStatus::Deleted);
        assert!(deleted.content.is_none());
        assert!(deleted.tags.is_empty());
    }

    #[test]
    fn markdown_bundle_is_portable_linked_deterministic_and_never_applies() {
        let mut catalog = MemoryCatalog::new();
        catalog
            .insert(item("memory-first", "Portable user text.", None))
            .expect("insert succeeds");
        catalog
            .insert(item("memory-second", "Second user text.", None))
            .expect("insert succeeds");

        let first = catalog.preview_markdown().expect("preview succeeds");
        let second = catalog.preview_markdown().expect("preview succeeds");
        assert_eq!(first, second);
        assert_eq!(first.index.relative_path, "MEMORY.md");
        assert!(
            first
                .index
                .markdown
                .contains("[[memory-first|memory-first]]")
        );
        assert_eq!(first.topics.len(), 2);
        assert!(
            first
                .topics
                .iter()
                .all(|file| file.relative_path.starts_with("Memory/memory-")
                    && !file.relative_path.starts_with('/')
                    && !file.write_enabled)
        );
        assert!(
            first
                .topics
                .iter()
                .any(|file| file.markdown.contains("Portable user text."))
        );
        assert!(!first.write_enabled);
        assert_eq!(first.bundle_sha256.len(), 64);
    }

    #[test]
    fn edit_uses_a_new_identity_and_confidence_decay_can_only_decrease() {
        let original = item("memory-edit-original", "Original text.", None);
        let original_id = original.memory_id.clone();
        let mut catalog = MemoryCatalog::new();
        catalog.insert(original).expect("insert succeeds");
        let decay = catalog
            .decay_confidence(
                &original_id,
                8_000,
                "c".repeat(64),
                "2026-08-14T12:02:00Z".to_owned(),
            )
            .expect("decay succeeds");
        assert_eq!(decay.transition, MemoryTransitionKind::Decay);
        assert_eq!(
            catalog
                .get(&original_id)
                .expect("original present")
                .confidence_bps,
            8_000
        );
        let before_increase = catalog.inspect();
        assert_eq!(
            catalog.decay_confidence(
                &original_id,
                9_000,
                "d".repeat(64),
                "2026-08-14T12:03:00Z".to_owned(),
            ),
            Err(MemoryError::InvalidTransition)
        );
        assert_eq!(catalog.inspect(), before_increase);

        let replacement = item("memory-edit-replacement", "Edited text.", None);
        let replacement_id = replacement.memory_id.clone();
        let edit = catalog
            .edit(
                &original_id,
                replacement,
                "e".repeat(64),
                "2026-08-14T12:04:00Z".to_owned(),
            )
            .expect("edit succeeds");
        assert_eq!(edit.transition, MemoryTransitionKind::Edit);
        assert_eq!(
            catalog.get(&original_id).expect("original retained").status,
            MemoryItemStatus::Superseded
        );
        assert_eq!(
            catalog
                .get(&replacement_id)
                .expect("replacement present")
                .content
                .as_deref(),
            Some("Edited text.")
        );
    }

    fn in_workspace(mut item: MemoryItem, workspace: &str) -> MemoryItem {
        item.scope.workspace_id = WorkspaceId::from_raw(workspace);
        item
    }

    fn citing(mut item: MemoryItem, source: &str, object: &str) -> MemoryItem {
        item.evidence[0].source_id = source.to_owned();
        item.evidence[0].object_id = object.to_owned();
        item
    }

    #[test]
    fn a_revoked_item_keeps_its_content_leaves_the_current_set_and_can_then_be_deleted() {
        let revoked = item("memory-revoked", "Revoked fact.", None);
        let kept = item("memory-kept", "Kept fact.", None);
        let revoked_id = revoked.memory_id.clone();
        let mut catalog = MemoryCatalog::new();
        catalog.insert(revoked).expect("insert succeeds");
        catalog.insert(kept).expect("insert succeeds");
        assert_eq!(catalog.inspect().current_count, 2);
        let receipt = catalog
            .revoke(
                &revoked_id,
                "c".repeat(64),
                "2026-08-14T13:00:00Z".to_owned(),
            )
            .expect("revocation succeeds");
        assert_eq!(receipt.transition, MemoryTransitionKind::Revoke);
        assert!(!receipt.automatic_decision);
        assert!(!receipt.files_written);
        let item = catalog.get(&revoked_id).expect("revoked item retained");
        assert_eq!(item.status, MemoryItemStatus::Revoked);
        assert_eq!(item.content.as_deref(), Some("Revoked fact."));
        assert_eq!(item.decision_sha256, "c".repeat(64));
        assert_eq!(catalog.inspect().current_count, 1);
        let preview = catalog.preview_markdown().expect("preview succeeds");
        assert!(
            preview
                .index
                .markdown
                .contains("- [[memory-revoked|memory-revoked]] (revoked)")
        );

        // A revoked item is revoked once, and only an item that still holds
        // content can be revoked.
        let before = catalog.inspect();
        assert_eq!(
            catalog.revoke(
                &revoked_id,
                "d".repeat(64),
                "2026-08-14T13:01:00Z".to_owned()
            ),
            Err(MemoryError::InvalidTransition)
        );
        assert_eq!(catalog.inspect(), before);
        catalog
            .delete(
                &revoked_id,
                "e".repeat(64),
                "2026-08-14T13:02:00Z".to_owned(),
            )
            .expect("a revoked item can be deleted");
        let deleted = catalog.get(&revoked_id).expect("tombstone retained");
        assert_eq!(deleted.status, MemoryItemStatus::Deleted);
        assert!(deleted.content.is_none());
        assert_eq!(
            catalog.revoke(
                &revoked_id,
                "f".repeat(64),
                "2026-08-14T13:03:00Z".to_owned()
            ),
            Err(MemoryError::InvalidTransition)
        );
        let missing = MemoryId::parse("memory-missing").expect("valid identity");
        assert_eq!(
            catalog.revoke(&missing, "f".repeat(64), "2026-08-14T13:03:00Z".to_owned()),
            Err(MemoryError::NotFound)
        );
        assert_eq!(
            catalog.revoke(
                &missing,
                "not-a-digest".to_owned(),
                "2026-08-14T13:03:00Z".to_owned()
            ),
            Err(MemoryError::InvalidInput)
        );
    }

    #[test]
    fn a_source_revocation_reaches_every_citing_item_of_its_workspace_in_one_transition() {
        let first = citing(
            item("memory-first", "First fact.", None),
            "source-a",
            "page-1",
        );
        let second = citing(
            item("memory-second", "Second fact.", None),
            "source-a",
            "page-2",
        );
        let other_source = citing(
            item("memory-other", "Other fact.", None),
            "source-b",
            "page-1",
        );
        let other_workspace = in_workspace(
            citing(
                item("memory-foreign", "Foreign fact.", None),
                "source-a",
                "page-1",
            ),
            "workspace-other",
        );
        let mut catalog = MemoryCatalog::new();
        for value in [first, second, other_source, other_workspace] {
            catalog.insert(value).expect("insert succeeds");
        }
        let id = |value: &str| MemoryId::parse(value).expect("valid identity");
        // An expired item still cites the source and is withdrawn with it.
        let mut expiring = citing(
            item(
                "memory-expired",
                "Expired fact.",
                Some("2026-08-15T00:00:00Z"),
            ),
            "source-a",
            "page-3",
        );
        expiring.fact_key = Some("release.expired".to_owned());
        catalog.insert(expiring).expect("insert succeeds");
        catalog
            .expire("2026-08-16T00:00:00Z".to_owned(), "c".repeat(64))
            .expect("expiry succeeds");

        // An object revocation reaches only items citing that object.
        let before = catalog.inspect();
        let receipt = catalog
            .revoke_source(
                &MemorySourceRevocation {
                    workspace_id: WorkspaceId::from_raw("workspace-memory-lifecycle"),
                    source_id: "source-a".to_owned(),
                    object_id: Some("page-2".to_owned()),
                },
                "d".repeat(64),
                "2026-08-16T01:00:00Z".to_owned(),
            )
            .expect("object revocation succeeds");
        assert_eq!(receipt.revision, before.revision + 1);
        assert_eq!(receipt.revoked_memory_id_sha256s.len(), 1);
        assert_eq!(
            catalog.get(&id("memory-second")).unwrap().status,
            MemoryItemStatus::Revoked
        );
        assert_eq!(
            catalog.get(&id("memory-first")).unwrap().status,
            MemoryItemStatus::Approved
        );

        // A source revocation reaches every remaining citing item of the
        // workspace at once and never another workspace or source.
        let before = catalog.inspect();
        let receipt = catalog
            .revoke_source(
                &MemorySourceRevocation {
                    workspace_id: WorkspaceId::from_raw("workspace-memory-lifecycle"),
                    source_id: "source-a".to_owned(),
                    object_id: None,
                },
                "e".repeat(64),
                "2026-08-16T02:00:00Z".to_owned(),
            )
            .expect("source revocation succeeds");
        assert_eq!(receipt.revision, before.revision + 1);
        assert_eq!(receipt.catalog_sha256, catalog.inspect().catalog_sha256);
        assert_eq!(receipt.decision_sha256, "e".repeat(64));
        assert!(!receipt.automatic_decision);
        assert!(!receipt.files_written);
        let mut expected = ["memory-expired", "memory-first"]
            .iter()
            .map(|value| super::sha256(value.as_bytes()))
            .collect::<Vec<_>>();
        expected.sort();
        let mut revoked = receipt.revoked_memory_id_sha256s.clone();
        revoked.sort();
        assert_eq!(revoked, expected);
        for (memory_id, status) in [
            ("memory-first", MemoryItemStatus::Revoked),
            ("memory-second", MemoryItemStatus::Revoked),
            ("memory-expired", MemoryItemStatus::Revoked),
            ("memory-other", MemoryItemStatus::Approved),
            ("memory-foreign", MemoryItemStatus::Approved),
        ] {
            assert_eq!(
                catalog.get(&id(memory_id)).unwrap().status,
                status,
                "{memory_id}"
            );
        }
        assert_eq!(catalog.inspect().current_count, 2);

        // Nothing left to reach, a foreign workspace with no citing item and
        // malformed identities change nothing.
        let before = catalog.inspect();
        for (source, error) in [
            (
                MemorySourceRevocation {
                    workspace_id: WorkspaceId::from_raw("workspace-memory-lifecycle"),
                    source_id: "source-a".to_owned(),
                    object_id: None,
                },
                MemoryError::NotFound,
            ),
            (
                MemorySourceRevocation {
                    workspace_id: WorkspaceId::from_raw("workspace-absent"),
                    source_id: "source-b".to_owned(),
                    object_id: None,
                },
                MemoryError::NotFound,
            ),
            (
                MemorySourceRevocation {
                    workspace_id: WorkspaceId::from_raw("workspace-memory-lifecycle"),
                    source_id: String::new(),
                    object_id: None,
                },
                MemoryError::InvalidInput,
            ),
            (
                MemorySourceRevocation {
                    workspace_id: WorkspaceId::from_raw("workspace-memory-lifecycle"),
                    source_id: "source-b".to_owned(),
                    object_id: Some("page\n1".to_owned()),
                },
                MemoryError::InvalidInput,
            ),
        ] {
            assert_eq!(
                catalog.revoke_source(&source, "f".repeat(64), "2026-08-16T03:00:00Z".to_owned()),
                Err(error)
            );
            assert_eq!(catalog.inspect(), before);
        }
        assert_eq!(
            catalog.revoke_source(
                &MemorySourceRevocation {
                    workspace_id: WorkspaceId::from_raw("workspace-memory-lifecycle"),
                    source_id: "source-b".to_owned(),
                    object_id: None,
                },
                "short".to_owned(),
                "2026-08-16T03:00:00Z".to_owned(),
            ),
            Err(MemoryError::InvalidInput)
        );
        assert_eq!(catalog.inspect(), before);
    }

    #[test]
    fn a_revoked_item_without_content_is_refused() {
        let mut revoked = item("memory-empty-revoked", "Fact.", None);
        revoked.status = MemoryItemStatus::Revoked;
        revoked.content = None;
        assert_eq!(
            MemoryCatalog::new().insert(revoked),
            Err(MemoryError::InvalidInput)
        );
    }
}
