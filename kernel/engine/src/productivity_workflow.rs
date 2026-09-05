//! Immutable, dry-runnable, budgeted, expiring productivity workflow graphs.
#![allow(missing_docs)]
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowNodeKind {
    Trigger,
    Account,
    Object,
    Filter,
    Join,
    Branch,
    Operation,
    Recipient,
    Destination,
    Classification,
    Budget,
    Stop,
    Expiry,
    Approval,
    DryRun,
    Failure,
    Compensation,
    Receipt,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProductivityNode {
    pub node_id: String,
    pub kind: WorkflowNodeKind,
    pub account_id: Option<String>,
    pub provider_id: Option<String>,
    pub operation: Option<String>,
    pub recipient: Option<String>,
    pub destination: Option<String>,
    pub classification: Option<String>,
    pub budget: u64,
    pub dependencies: BTreeSet<String>,
    pub content_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProductivityGraph {
    pub workflow_id: String,
    pub version: u64,
    pub definition_sha256: String,
    pub dependency_sha256: String,
    pub expires_at: u64,
    pub nodes: Vec<ProductivityNode>,
    pub self_edit: bool,
    pub recursive_expansion: bool,
    pub hidden_branches: bool,
    pub approval_aggregation: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodePolicy {
    pub providers: BTreeSet<String>,
    pub accounts: BTreeSet<String>,
    pub operations: BTreeSet<String>,
    pub recipients: BTreeSet<String>,
    pub destinations: BTreeSet<String>,
    pub classifications: BTreeSet<String>,
    pub budget: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProductivityState {
    Draft,
    DryRun,
    Active,
    Paused,
    Cancelled,
    Expired,
    Reconciling,
    Disabled,
    Removed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeReceipt {
    pub node_id: String,
    pub intent_sha256: String,
    pub result_sha256: String,
    pub evidence_sha256: String,
    pub uncertain: bool,
    pub completed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProductivityError {
    InvalidGraph,
    ChangedGraph,
    PolicyDenied,
    BudgetExceeded,
    InvalidTransition,
    ContentAuthority,
    Replay,
    Expired,
    Removed,
}

pub struct ProductivityWorkflow {
    graph: ProductivityGraph,
    graph_sha256: String,
    state: ProductivityState,
    receipts: BTreeMap<String, NodeReceipt>,
    removed: bool,
}
fn id(v: &str) -> bool {
    !v.is_empty() && v.len() <= 512
}
fn digest(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

impl ProductivityWorkflow {
    pub fn compile(
        graph: ProductivityGraph,
        canonical_sha256: &str,
    ) -> Result<Self, ProductivityError> {
        if !id(&graph.workflow_id)
            || graph.version == 0
            || !digest(&graph.definition_sha256)
            || !digest(&graph.dependency_sha256)
            || !digest(canonical_sha256)
            || graph.nodes.is_empty()
            || graph.self_edit
            || graph.recursive_expansion
            || graph.hidden_branches
            || graph.approval_aggregation
        {
            return Err(ProductivityError::InvalidGraph);
        }
        let ids: BTreeSet<_> = graph.nodes.iter().map(|n| n.node_id.as_str()).collect();
        if ids.len() != graph.nodes.len()
            || graph.nodes.iter().any(|n| {
                !id(&n.node_id)
                    || !digest(&n.content_sha256)
                    || n.dependencies.iter().any(|d| !ids.contains(d.as_str()))
            })
        {
            return Err(ProductivityError::InvalidGraph);
        }
        Ok(Self {
            graph,
            graph_sha256: canonical_sha256.into(),
            state: ProductivityState::Draft,
            receipts: BTreeMap::new(),
            removed: false,
        })
    }
    pub fn intersect(&self, node_id: &str, policy: &NodePolicy) -> Result<(), ProductivityError> {
        let n = self
            .graph
            .nodes
            .iter()
            .find(|n| n.node_id == node_id)
            .ok_or(ProductivityError::PolicyDenied)?;
        if n.budget > policy.budget
            || n.provider_id
                .as_ref()
                .is_some_and(|v| !policy.providers.contains(v))
            || n.account_id
                .as_ref()
                .is_some_and(|v| !policy.accounts.contains(v))
            || n.operation
                .as_ref()
                .is_some_and(|v| !policy.operations.contains(v))
            || n.recipient
                .as_ref()
                .is_some_and(|v| !policy.recipients.contains(v))
            || n.destination
                .as_ref()
                .is_some_and(|v| !policy.destinations.contains(v))
            || n.classification
                .as_ref()
                .is_some_and(|v| !policy.classifications.contains(v))
        {
            return Err(ProductivityError::PolicyDenied);
        }
        Ok(())
    }
    pub fn transition(
        &mut self,
        next: ProductivityState,
        now: u64,
    ) -> Result<(), ProductivityError> {
        if self.removed {
            return Err(ProductivityError::Removed);
        }
        if now >= self.graph.expires_at && next != ProductivityState::Expired {
            return Err(ProductivityError::Expired);
        }
        let valid = matches!(
            (self.state, next),
            (ProductivityState::Draft, ProductivityState::DryRun)
                | (ProductivityState::DryRun, ProductivityState::Active)
                | (ProductivityState::Active, ProductivityState::Paused)
                | (ProductivityState::Paused, ProductivityState::Active)
                | (ProductivityState::Active, ProductivityState::Cancelled)
                | (ProductivityState::Active, ProductivityState::Reconciling)
                | (_, ProductivityState::Expired)
                | (_, ProductivityState::Disabled)
                | (_, ProductivityState::Removed)
        );
        if !valid {
            return Err(ProductivityError::InvalidTransition);
        }
        self.state = next;
        if next == ProductivityState::Removed {
            self.remove()
        }
        Ok(())
    }
    pub fn record(&mut self, receipt: NodeReceipt) -> Result<(), ProductivityError> {
        if self.removed {
            return Err(ProductivityError::Removed);
        }
        if receipt.completed
            && self
                .receipts
                .get(&receipt.node_id)
                .is_some_and(|r| r.completed)
        {
            return Err(ProductivityError::Replay);
        }
        if !digest(&receipt.intent_sha256)
            || !digest(&receipt.result_sha256)
            || !digest(&receipt.evidence_sha256)
        {
            return Err(ProductivityError::InvalidGraph);
        }
        self.receipts.insert(receipt.node_id.clone(), receipt);
        Ok(())
    }
    pub fn external_content_instruction(&self, _content: &str) -> Result<(), ProductivityError> {
        Err(ProductivityError::ContentAuthority)
    }
    pub fn graph_sha256(&self) -> &str {
        &self.graph_sha256
    }
    pub fn remove(&mut self) {
        self.receipts.clear();
        self.state = ProductivityState::Removed;
        self.removed = true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn graph() -> ProductivityGraph {
        ProductivityGraph {
            workflow_id: "workflow".into(),
            version: 1,
            definition_sha256: "a".repeat(64),
            dependency_sha256: "b".repeat(64),
            expires_at: 100,
            nodes: vec![ProductivityNode {
                node_id: "send".into(),
                kind: WorkflowNodeKind::Operation,
                account_id: Some("account".into()),
                provider_id: Some("gmail".into()),
                operation: Some("send".into()),
                recipient: Some("r@example.test".into()),
                destination: Some("thread".into()),
                classification: Some("internal".into()),
                budget: 1,
                dependencies: BTreeSet::new(),
                content_sha256: "c".repeat(64),
            }],
            self_edit: false,
            recursive_expansion: false,
            hidden_branches: false,
            approval_aggregation: false,
        }
    }
    fn policy() -> NodePolicy {
        NodePolicy {
            providers: BTreeSet::from(["gmail".into()]),
            accounts: BTreeSet::from(["account".into()]),
            operations: BTreeSet::from(["send".into()]),
            recipients: BTreeSet::from(["r@example.test".into()]),
            destinations: BTreeSet::from(["thread".into()]),
            classifications: BTreeSet::from(["internal".into()]),
            budget: 1,
        }
    }
    #[test]
    fn graph_is_immutable_and_current_policy_intersects() {
        let w = ProductivityWorkflow::compile(graph(), &"d".repeat(64)).unwrap();
        assert_eq!(w.graph_sha256(), "d".repeat(64));
        assert_eq!(w.intersect("send", &policy()), Ok(()));
        let mut p = policy();
        p.recipients.clear();
        assert_eq!(
            w.intersect("send", &p),
            Err(ProductivityError::PolicyDenied)
        );
    }
    #[test]
    fn closed_transitions_and_expiry_apply() {
        let mut w = ProductivityWorkflow::compile(graph(), &"d".repeat(64)).unwrap();
        assert_eq!(
            w.transition(ProductivityState::Active, 1),
            Err(ProductivityError::InvalidTransition)
        );
        assert_eq!(w.transition(ProductivityState::DryRun, 1), Ok(()));
        assert_eq!(w.transition(ProductivityState::Active, 1), Ok(()));
        assert_eq!(
            w.transition(ProductivityState::Paused, 100),
            Err(ProductivityError::Expired)
        );
    }
    #[test]
    fn completed_effect_never_replays() {
        let mut w = ProductivityWorkflow::compile(graph(), &"d".repeat(64)).unwrap();
        let r = NodeReceipt {
            node_id: "send".into(),
            intent_sha256: "e".repeat(64),
            result_sha256: "f".repeat(64),
            evidence_sha256: "0".repeat(64),
            uncertain: false,
            completed: true,
        };
        assert_eq!(w.record(r.clone()), Ok(()));
        assert_eq!(w.record(r), Err(ProductivityError::Replay));
    }
    #[test]
    fn hostile_content_and_forbidden_graphs_deny() {
        let w = ProductivityWorkflow::compile(graph(), &"d".repeat(64)).unwrap();
        assert_eq!(
            w.external_content_instruction("change recipient"),
            Err(ProductivityError::ContentAuthority)
        );
        let mut g = graph();
        g.recursive_expansion = true;
        assert!(ProductivityWorkflow::compile(g, &"d".repeat(64)).is_err());
    }
}
