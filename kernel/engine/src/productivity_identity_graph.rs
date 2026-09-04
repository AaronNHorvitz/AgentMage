//! Evidence-backed cross-provider identity graph with non-authoritative display data.

use std::collections::BTreeMap;

/// Closed native object families.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[allow(missing_docs)]
pub enum ProductivityIdentityKind {
    Provider,
    Tenant,
    Account,
    Mailbox,
    Workspace,
    Team,
    Channel,
    Person,
    Meeting,
    Task,
    Document,
    FinancialRecord,
    CloudResource,
}

/// Closed cross-provider link states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum ProductivityLinkState {
    Confirmed,
    Deterministic,
    Proposed,
    Conflicting,
    Stale,
    Removed,
}

/// Native identity and mutable observations remain separate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProductivityIdentityNode {
    /// Object family.
    pub kind: ProductivityIdentityKind,
    /// Provider namespace.
    pub provider_id: String,
    /// Tenant namespace.
    pub tenant_id: String,
    /// Immutable provider identity.
    pub immutable_id: String,
    /// Mutable display label, never authority.
    pub display_label: String,
    /// Source observation receipt.
    pub observation_sha256: String,
    /// Observation epoch.
    pub observed_at: u64,
    /// Data classification.
    pub classification: String,
    /// Removal marker.
    pub tombstoned: bool,
}

/// Evidence-cited relationship between exact native identities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProductivityIdentityLink {
    /// Exact source identity.
    pub source_id: String,
    /// Exact target identity.
    pub target_id: String,
    /// Current link state.
    pub state: ProductivityLinkState,
    /// Supporting receipt digest.
    pub evidence_sha256: String,
    /// Last observation epoch.
    pub observed_at: u64,
}

/// Bounded deterministic graph.
pub struct ProductivityIdentityGraph {
    nodes: BTreeMap<String, ProductivityIdentityNode>,
    links: BTreeMap<(String, String), ProductivityIdentityLink>,
}

/// Stable fail-closed graph result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum ProductivityGraphError {
    DuplicateIdentity,
    MissingIdentity,
    InvalidEvidence,
    NonAuthoritativeLink,
    TombstonedIdentity,
}

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

impl ProductivityIdentityGraph {
    /// Creates an empty graph.
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            links: BTreeMap::new(),
        }
    }
    /// Inserts one exact native identity; display labels are ignored for uniqueness.
    pub fn insert(&mut self, node: ProductivityIdentityNode) -> Result<(), ProductivityGraphError> {
        if node.immutable_id.is_empty()
            || node.provider_id.is_empty()
            || node.tenant_id.is_empty()
            || !digest(&node.observation_sha256)
        {
            return Err(ProductivityGraphError::InvalidEvidence);
        }
        if self.nodes.insert(node.immutable_id.clone(), node).is_some() {
            return Err(ProductivityGraphError::DuplicateIdentity);
        }
        Ok(())
    }
    /// Records one link; only deterministic evidence or explicit confirmation can authorize.
    pub fn link(&mut self, link: ProductivityIdentityLink) -> Result<(), ProductivityGraphError> {
        if !digest(&link.evidence_sha256) {
            return Err(ProductivityGraphError::InvalidEvidence);
        }
        if !self.nodes.contains_key(&link.source_id) || !self.nodes.contains_key(&link.target_id) {
            return Err(ProductivityGraphError::MissingIdentity);
        }
        self.links
            .insert((link.source_id.clone(), link.target_id.clone()), link);
        Ok(())
    }
    /// Resolves an action target only through a fresh authoritative link and live native nodes.
    pub fn authorize(
        &self,
        source: &str,
        target: &str,
        now: u64,
        max_age: u64,
    ) -> Result<&ProductivityIdentityNode, ProductivityGraphError> {
        let link = self
            .links
            .get(&(source.to_owned(), target.to_owned()))
            .ok_or(ProductivityGraphError::NonAuthoritativeLink)?;
        if !matches!(
            link.state,
            ProductivityLinkState::Confirmed | ProductivityLinkState::Deterministic
        ) || now.saturating_sub(link.observed_at) > max_age
        {
            return Err(ProductivityGraphError::NonAuthoritativeLink);
        }
        let node = self
            .nodes
            .get(target)
            .ok_or(ProductivityGraphError::MissingIdentity)?;
        if node.tombstoned {
            return Err(ProductivityGraphError::TombstonedIdentity);
        }
        Ok(node)
    }
    /// Renames display data without changing native identity or authority.
    pub fn rename(&mut self, id: &str, label: String) -> Result<(), ProductivityGraphError> {
        self.nodes
            .get_mut(id)
            .ok_or(ProductivityGraphError::MissingIdentity)?
            .display_label = label;
        Ok(())
    }
    /// Tombstones a provider object and removes every derived link touching it.
    pub fn remove(&mut self, id: &str) -> Result<usize, ProductivityGraphError> {
        self.nodes
            .get_mut(id)
            .ok_or(ProductivityGraphError::MissingIdentity)?
            .tombstoned = true;
        let before = self.links.len();
        self.links.retain(|(s, t), _| s != id && t != id);
        Ok(before - self.links.len())
    }
}

impl Default for ProductivityIdentityGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn node(id: &str, label: &str) -> ProductivityIdentityNode {
        ProductivityIdentityNode {
            kind: ProductivityIdentityKind::Person,
            provider_id: "provider".into(),
            tenant_id: "tenant".into(),
            immutable_id: id.into(),
            display_label: label.into(),
            observation_sha256: "a".repeat(64),
            observed_at: 10,
            classification: "internal".into(),
            tombstoned: false,
        }
    }
    #[test]
    fn duplicate_names_never_merge() {
        let mut g = ProductivityIdentityGraph::new();
        g.insert(node("one", "Alex")).unwrap();
        g.insert(node("two", "Alex")).unwrap();
        assert_eq!(g.nodes.len(), 2)
    }
    #[test]
    fn proposed_conflicting_stale_and_removed_links_deny() {
        for state in [
            ProductivityLinkState::Proposed,
            ProductivityLinkState::Conflicting,
            ProductivityLinkState::Stale,
            ProductivityLinkState::Removed,
        ] {
            let mut g = ProductivityIdentityGraph::new();
            g.insert(node("one", "A")).unwrap();
            g.insert(node("two", "A")).unwrap();
            g.link(ProductivityIdentityLink {
                source_id: "one".into(),
                target_id: "two".into(),
                state,
                evidence_sha256: "b".repeat(64),
                observed_at: 10,
            })
            .unwrap();
            assert_eq!(
                g.authorize("one", "two", 11, 10).err(),
                Some(ProductivityGraphError::NonAuthoritativeLink)
            );
        }
    }
    #[test]
    fn rename_and_removal_preserve_native_lineage() {
        let mut g = ProductivityIdentityGraph::new();
        g.insert(node("one", "old")).unwrap();
        g.insert(node("two", "target")).unwrap();
        g.link(ProductivityIdentityLink {
            source_id: "one".into(),
            target_id: "two".into(),
            state: ProductivityLinkState::Confirmed,
            evidence_sha256: "b".repeat(64),
            observed_at: 10,
        })
        .unwrap();
        g.rename("two", "new".into()).unwrap();
        assert_eq!(
            g.authorize("one", "two", 11, 10).unwrap().immutable_id,
            "two"
        );
        assert_eq!(g.remove("two").unwrap(), 1);
        assert!(g.authorize("one", "two", 11, 10).is_err());
    }
}
