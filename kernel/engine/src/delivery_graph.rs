//! Provider-neutral delivery graph and non-inheriting adapter conformance.

use std::collections::{BTreeMap, BTreeSet};

/// Closed provider-neutral delivery node family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeliveryNodeKind {
    /// Service.
    Service,
    /// Work item.
    WorkItem,
    /// Repository.
    Repository,
    /// Change.
    Change,
    /// Commit.
    Commit,
    /// Review.
    Review,
    /// Build.
    Build,
    /// Check.
    Check,
    /// Artifact.
    Artifact,
    /// Provenance.
    Provenance,
    /// Environment.
    Environment,
    /// Deployment.
    Deployment,
    /// Telemetry.
    Telemetry,
    /// Incident.
    Incident,
    /// Finding.
    Finding,
    /// Release.
    Release,
    /// Rollback.
    Rollback,
}

/// Closed relationship evidence state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelationshipState {
    /// Directly observed.
    Observed,
    /// Deterministically derived.
    Derived,
    /// Non-authoritative inference.
    Inferred,
    /// Conflicting sources.
    Conflicting,
    /// Stale source.
    Stale,
    /// Deleted source.
    Deleted,
    /// Inaccessible source.
    Inaccessible,
    /// Unknown relationship.
    Unknown,
}

/// One exact provider-neutral graph node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeliveryNode {
    /// Node kind.
    pub kind: DeliveryNodeKind,
    /// Provider identity.
    pub provider_id: String,
    /// Exact host.
    pub host: String,
    /// Tenant or organization.
    pub tenant_or_organization: String,
    /// Project identity.
    pub project: String,
    /// Immutable identity.
    pub immutable_id: String,
    /// Display identity.
    pub display_id: String,
    /// Exact version.
    pub version: String,
    /// Freshness state.
    pub fresh: bool,
    /// Sensitivity label.
    pub sensitivity: String,
    /// Tombstone state.
    pub tombstoned: bool,
    /// Source receipt digest.
    pub source_receipt_sha256: String,
}

/// One exact evidence-backed edge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeliveryEdge {
    /// Source immutable identity.
    pub source_immutable_id: String,
    /// Target immutable identity.
    pub target_immutable_id: String,
    /// Relationship identity.
    pub relationship: String,
    /// Evidence state.
    pub state: RelationshipState,
    /// Evidence digest.
    pub evidence_sha256: String,
}

/// Versioned bounded graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeliveryGraph {
    /// Schema version.
    pub schema_version: u16,
    /// Node ceiling.
    pub max_nodes: usize,
    /// Edge ceiling.
    pub max_edges: usize,
    /// Immutable node index.
    pub nodes: BTreeMap<String, DeliveryNode>,
    /// Deterministic edge index.
    pub edges: BTreeMap<(String, String, String), DeliveryEdge>,
}

/// Closed adapter operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdapterOperation {
    /// Describe.
    Describe,
    /// Diagnose.
    Diagnose,
    /// Discover.
    Discover,
    /// Plan.
    Plan,
    /// Preview.
    Preview,
    /// Execute.
    Execute,
    /// Reconcile.
    Reconcile,
    /// Roll back or compensate.
    RollbackOrCompensate,
    /// Remove.
    Remove,
}

/// Non-inheriting conformance level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConformanceLevel {
    /// Manifested.
    L0Manifested,
    /// Observable.
    L1Observable,
    /// Writable.
    L2Writable,
    /// Executable.
    L3Executable,
    /// Deployable.
    L4Deployable,
    /// Administrative.
    L5Administrative,
}

/// Namespaced extension binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderExtension {
    /// Namespaced identity.
    pub namespaced_id: String,
    /// Schema digest.
    pub schema_sha256: String,
    /// Policy digest.
    pub policy_sha256: String,
    /// Conformance digest.
    pub conformance_sha256: String,
}

/// Exact signed adapter support tuple.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdapterManifest {
    /// Adapter identity.
    pub adapter_id: String,
    /// Provider identity.
    pub provider_id: String,
    /// Exact host.
    pub host: String,
    /// Exact version.
    pub version: String,
    /// Object kinds.
    pub object_kinds: BTreeSet<DeliveryNodeKind>,
    /// Exact operations without inheritance.
    pub operations: BTreeSet<AdapterOperation>,
    /// Exact scopes.
    pub scopes: BTreeSet<String>,
    /// Event kinds.
    pub event_kinds: BTreeSet<String>,
    /// Limit digest.
    pub limit_sha256: String,
    /// Degradation behavior.
    pub degradation: String,
    /// Support state.
    pub support_state: String,
    /// Conformance level.
    pub conformance_level: ConformanceLevel,
    /// Conformance digest.
    pub conformance_sha256: String,
    /// Signature digest.
    pub signature_sha256: String,
    /// Namespaced extensions.
    pub extensions: Vec<ProviderExtension>,
    /// Explicit enablement.
    pub enabled: bool,
}

/// Fake-adapter behavior mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FakeAdapterMode {
    /// Normal fixture.
    Normal,
    /// Fault fixture.
    Fault,
    /// Future-version fixture.
    FutureVersion,
    /// Eventual-consistency fixture.
    EventualConsistency,
    /// Hostile fixture.
    Hostile,
    /// Removed fixture.
    Removed,
}

/// Stable delivery-foundation refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeliveryFoundationError {
    /// Invalid record.
    InvalidRecord,
    /// Bounded capacity exhausted.
    CapacityExceeded,
    /// Node missing.
    MissingNode,
    /// Edge cannot authorize.
    NonAuthoritativeEdge,
    /// Extension rejected.
    ExtensionRejected,
    /// Adapter unavailable.
    AdapterUnavailable,
}

impl DeliveryGraph {
    /// Create an empty schema-version-one graph.
    #[must_use]
    pub fn new(max_nodes: usize, max_edges: usize) -> Self {
        Self {
            schema_version: 1,
            max_nodes,
            max_edges,
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
        }
    }

    /// Insert or refresh one exact node.
    pub fn upsert_node(&mut self, node: DeliveryNode) -> Result<(), DeliveryFoundationError> {
        validate_node(&node)?;
        if !self.nodes.contains_key(&node.immutable_id) && self.nodes.len() >= self.max_nodes {
            return Err(DeliveryFoundationError::CapacityExceeded);
        }
        self.nodes.insert(node.immutable_id.clone(), node);
        Ok(())
    }

    /// Add one evidence-backed edge.
    pub fn add_edge(&mut self, edge: DeliveryEdge) -> Result<(), DeliveryFoundationError> {
        if self.edges.len() >= self.max_edges {
            return Err(DeliveryFoundationError::CapacityExceeded);
        }
        if !self.nodes.contains_key(&edge.source_immutable_id)
            || !self.nodes.contains_key(&edge.target_immutable_id)
            || !valid_id(&edge.relationship)
            || !valid_sha256(&edge.evidence_sha256)
        {
            return Err(DeliveryFoundationError::MissingNode);
        }
        self.edges.insert(
            (
                edge.source_immutable_id.clone(),
                edge.target_immutable_id.clone(),
                edge.relationship.clone(),
            ),
            edge,
        );
        Ok(())
    }

    /// Return an edge only when it and both endpoint nodes remain authoritative.
    pub fn authoritative_edge(
        &self,
        key: &(String, String, String),
    ) -> Result<&DeliveryEdge, DeliveryFoundationError> {
        let edge = self
            .edges
            .get(key)
            .ok_or(DeliveryFoundationError::MissingNode)?;
        let source = self
            .nodes
            .get(&edge.source_immutable_id)
            .ok_or(DeliveryFoundationError::MissingNode)?;
        let target = self
            .nodes
            .get(&edge.target_immutable_id)
            .ok_or(DeliveryFoundationError::MissingNode)?;
        if edge.state != RelationshipState::Observed
            || !source.fresh
            || !target.fresh
            || source.tombstoned
            || target.tombstoned
        {
            Err(DeliveryFoundationError::NonAuthoritativeEdge)
        } else {
            Ok(edge)
        }
    }

    /// Mark one node and all incident relationships stale.
    pub fn mark_stale(&mut self, id: &str) -> Result<(), DeliveryFoundationError> {
        self.nodes
            .get_mut(id)
            .ok_or(DeliveryFoundationError::MissingNode)?
            .fresh = false;
        for edge in self
            .edges
            .values_mut()
            .filter(|e| e.source_immutable_id == id || e.target_immutable_id == id)
        {
            edge.state = RelationshipState::Stale;
        }
        Ok(())
    }

    /// Tombstone one node and remove all incident relationships.
    pub fn remove_node(&mut self, id: &str) -> Result<(), DeliveryFoundationError> {
        let node = self
            .nodes
            .get_mut(id)
            .ok_or(DeliveryFoundationError::MissingNode)?;
        node.fresh = false;
        node.tombstoned = true;
        self.edges
            .retain(|_, e| e.source_immutable_id != id && e.target_immutable_id != id);
        Ok(())
    }

    #[must_use]
    /// Capture a deterministic in-memory graph snapshot.
    pub fn snapshot(&self) -> Self {
        self.clone()
    }
    /// Normalize a supported historical graph into schema version one.
    pub fn migrate(mut value: Self) -> Result<Self, DeliveryFoundationError> {
        if value.schema_version > 1 {
            Err(DeliveryFoundationError::InvalidRecord)
        } else {
            value.schema_version = 1;
            Ok(value)
        }
    }
}

/// Validate and register an explicitly enabled adapter manifest.
pub fn register_adapter(
    manifest: &AdapterManifest,
    mode: FakeAdapterMode,
) -> Result<BTreeSet<AdapterOperation>, DeliveryFoundationError> {
    if !manifest.enabled
        || matches!(
            mode,
            FakeAdapterMode::Fault
                | FakeAdapterMode::FutureVersion
                | FakeAdapterMode::Hostile
                | FakeAdapterMode::Removed
        )
    {
        return Err(DeliveryFoundationError::AdapterUnavailable);
    }
    if !valid_id(&manifest.adapter_id)
        || !valid_id(&manifest.provider_id)
        || !valid_host(&manifest.host)
        || !valid_id(&manifest.version)
        || manifest.object_kinds.is_empty()
        || manifest.scopes.is_empty()
        || manifest.event_kinds.is_empty()
        || !valid_sha256(&manifest.limit_sha256)
        || !valid_sha256(&manifest.conformance_sha256)
        || !valid_sha256(&manifest.signature_sha256)
        || !valid_id(&manifest.degradation)
        || !valid_id(&manifest.support_state)
    {
        return Err(DeliveryFoundationError::InvalidRecord);
    }
    if manifest.operations.is_empty()
        || manifest.extensions.iter().any(|e| {
            !e.namespaced_id
                .starts_with(&format!("{}.", manifest.provider_id))
                || !valid_sha256(&e.schema_sha256)
                || !valid_sha256(&e.policy_sha256)
                || !valid_sha256(&e.conformance_sha256)
        })
    {
        return Err(DeliveryFoundationError::ExtensionRejected);
    }
    Ok(manifest.operations.clone())
}

fn validate_node(n: &DeliveryNode) -> Result<(), DeliveryFoundationError> {
    if !valid_id(&n.provider_id)
        || !valid_host(&n.host)
        || !valid_id(&n.tenant_or_organization)
        || !valid_id(&n.project)
        || !valid_id(&n.immutable_id)
        || !valid_id(&n.display_id)
        || !valid_id(&n.version)
        || !valid_id(&n.sensitivity)
        || !valid_sha256(&n.source_receipt_sha256)
    {
        Err(DeliveryFoundationError::InvalidRecord)
    } else {
        Ok(())
    }
}
fn valid_id(v: &str) -> bool {
    !v.is_empty()
        && v.len() <= 256
        && v.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':' | b'/'))
}
fn valid_host(v: &str) -> bool {
    valid_id(v) && !v.contains("..") && !v.contains('/')
}
fn valid_sha256(v: &str) -> bool {
    v.len() == 64 && v.bytes().all(|b| b.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn node(id: &str) -> DeliveryNode {
        DeliveryNode {
            kind: DeliveryNodeKind::Repository,
            provider_id: "fake".into(),
            host: "fake.local".into(),
            tenant_or_organization: "tenant".into(),
            project: "project".into(),
            immutable_id: id.into(),
            display_id: "same-display".into(),
            version: "v1".into(),
            fresh: true,
            sensitivity: "internal".into(),
            tombstoned: false,
            source_receipt_sha256: "a".repeat(64),
        }
    }
    fn manifest() -> AdapterManifest {
        AdapterManifest {
            adapter_id: "fake.adapter".into(),
            provider_id: "fake".into(),
            host: "fake.local".into(),
            version: "v1".into(),
            object_kinds: [DeliveryNodeKind::Repository].into_iter().collect(),
            operations: [
                AdapterOperation::Describe,
                AdapterOperation::Discover,
                AdapterOperation::Remove,
            ]
            .into_iter()
            .collect(),
            scopes: ["repository.read".into()].into_iter().collect(),
            event_kinds: ["changed".into()].into_iter().collect(),
            limit_sha256: "b".repeat(64),
            degradation: "block".into(),
            support_state: "fixture".into(),
            conformance_level: ConformanceLevel::L1Observable,
            conformance_sha256: "c".repeat(64),
            signature_sha256: "d".repeat(64),
            extensions: vec![ProviderExtension {
                namespaced_id: "fake.extension".into(),
                schema_sha256: "e".repeat(64),
                policy_sha256: "f".repeat(64),
                conformance_sha256: "1".repeat(64),
            }],
            enabled: true,
        }
    }
    #[test]
    fn graph_round_trip_stale_and_removal() {
        let mut g = DeliveryGraph::new(2, 1);
        g.upsert_node(node("one")).unwrap();
        g.upsert_node(node("two")).unwrap();
        let key = ("one".into(), "two".into(), "links".into());
        g.add_edge(DeliveryEdge {
            source_immutable_id: "one".into(),
            target_immutable_id: "two".into(),
            relationship: "links".into(),
            state: RelationshipState::Observed,
            evidence_sha256: "2".repeat(64),
        })
        .unwrap();
        assert!(g.authoritative_edge(&key).is_ok());
        assert_eq!(g.snapshot(), g);
        g.mark_stale("one").unwrap();
        assert_eq!(
            g.authoritative_edge(&key),
            Err(DeliveryFoundationError::NonAuthoritativeEdge)
        );
        g.remove_node("one").unwrap();
        assert!(g.edges.is_empty());
    }
    #[test]
    fn inferred_edge_never_authorizes() {
        let mut g = DeliveryGraph::new(2, 1);
        g.upsert_node(node("one")).unwrap();
        g.upsert_node(node("two")).unwrap();
        let key = ("one".into(), "two".into(), "maybe".into());
        g.add_edge(DeliveryEdge {
            source_immutable_id: "one".into(),
            target_immutable_id: "two".into(),
            relationship: "maybe".into(),
            state: RelationshipState::Inferred,
            evidence_sha256: "3".repeat(64),
        })
        .unwrap();
        assert_eq!(
            g.authoritative_edge(&key),
            Err(DeliveryFoundationError::NonAuthoritativeEdge)
        );
    }
    #[test]
    fn registration_is_exact_and_fault_modes_block() {
        let m = manifest();
        assert_eq!(
            register_adapter(&m, FakeAdapterMode::Normal).unwrap(),
            m.operations
        );
        for mode in [
            FakeAdapterMode::Fault,
            FakeAdapterMode::FutureVersion,
            FakeAdapterMode::Hostile,
            FakeAdapterMode::Removed,
        ] {
            assert_eq!(
                register_adapter(&m, mode),
                Err(DeliveryFoundationError::AdapterUnavailable)
            );
        }
    }
    #[test]
    fn extension_and_capacity_fail_closed() {
        let mut m = manifest();
        m.extensions[0].namespaced_id = "other.extension".into();
        assert_eq!(
            register_adapter(&m, FakeAdapterMode::Normal),
            Err(DeliveryFoundationError::ExtensionRejected)
        );
        let mut g = DeliveryGraph::new(1, 1);
        g.upsert_node(node("one")).unwrap();
        assert_eq!(
            g.upsert_node(node("two")),
            Err(DeliveryFoundationError::CapacityExceeded)
        );
    }
}
