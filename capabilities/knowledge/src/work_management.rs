//! Provider-neutral work-item packets, links, drafts, and exact effect admission.
#![allow(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};

/// Closed first-GA work-management providers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorkProvider {
    GithubIssues,
    JiraCloud,
    JiraDataCenter,
    AzureBoards,
}
/// Closed work-item effect classes; no variant inherits another.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorkEffect {
    Create,
    Edit,
    Comment,
    Assign,
    LabelOrTag,
    Link,
    Attach,
    Transition,
    Close,
    Reopen,
}
/// Evidence strength of a cross-system link.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkLinkState {
    Observed,
    ProviderReference,
    Inferred,
    Conflicting,
    Stale,
    Unknown,
}
/// Built-in provider-backed planning and delivery roles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderWorkflow {
    ProductDiscovery,
    IssueAuthoring,
    EpicDecomposition,
    AcceptanceCriteria,
    BacklogCuration,
    DependencyPlanning,
    SprintPlanning,
    RiskAnalysis,
    RoadmapAudit,
    ProgressReconciliation,
    IssueIntake,
    BugLifecycle,
    PullRequestStewardship,
    IndependentReview,
}

/// One exact provider object with common facts and attributable extensions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkItem {
    pub provider: WorkProvider,
    pub provider_id: String,
    pub host: String,
    pub tenant: String,
    pub project: String,
    pub immutable_id: String,
    pub display_id: String,
    pub object_type: String,
    pub state: String,
    pub revision: String,
    pub parent_id: Option<String>,
    pub iteration: Option<String>,
    pub field_schema_sha256: String,
    pub permissions_sha256: String,
    pub content_sha256: String,
    pub comments_sha256: String,
    pub attachments_sha256: String,
    pub history_sha256: String,
    pub provider_extensions: BTreeMap<String, String>,
    pub untrusted_content: bool,
}
/// One evidence-labeled delivery-graph relationship.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkLink {
    pub work_item_id: String,
    pub target_kind: String,
    pub target_immutable_id: String,
    pub evidence_sha256: String,
    pub state: WorkLinkState,
}
/// Immutable input packet for one provider-backed workflow.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderWorkPacket {
    pub packet_id: String,
    pub workflow: ProviderWorkflow,
    pub provider: WorkProvider,
    pub provider_identity_sha256: String,
    pub repository_sha256: Option<String>,
    pub base_revision: Option<String>,
    pub head_revision: Option<String>,
    pub object_ids: BTreeSet<String>,
    pub object_revision_sha256: String,
    pub field_schema_sha256: String,
    pub permissions_sha256: String,
    pub recipients_sha256: String,
    pub visibility: String,
    pub source_sha256: BTreeSet<String>,
    pub support_tuple_sha256: String,
    pub maximum_effect: Option<WorkEffect>,
    pub untrusted_input_sha256: BTreeSet<String>,
}

/// Compute the immutable digest that an effect plan must bind.
pub fn work_packet_sha256(packet: &ProviderWorkPacket) -> String {
    let mut digest = Sha256::new();
    digest.update(format!("{:?}\0{:?}\0", packet.workflow, packet.provider));
    for value in [
        Some(packet.packet_id.as_str()),
        Some(packet.provider_identity_sha256.as_str()),
        packet.repository_sha256.as_deref(),
        packet.base_revision.as_deref(),
        packet.head_revision.as_deref(),
        Some(packet.object_revision_sha256.as_str()),
        Some(packet.field_schema_sha256.as_str()),
        Some(packet.permissions_sha256.as_str()),
        Some(packet.recipients_sha256.as_str()),
        Some(packet.visibility.as_str()),
        Some(packet.support_tuple_sha256.as_str()),
    ] {
        digest.update(value.unwrap_or("<none>").as_bytes());
        digest.update([0]);
    }
    for value in packet
        .object_ids
        .iter()
        .chain(packet.source_sha256.iter())
        .chain(packet.untrusted_input_sha256.iter())
    {
        digest.update(value.as_bytes());
        digest.update([0]);
    }
    digest.update(format!("{:?}", packet.maximum_effect).as_bytes());
    digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
/// Field-level inert work-item effect plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkEffectPlan {
    pub plan_id: String,
    pub packet_sha256: String,
    pub provider: WorkProvider,
    pub host: String,
    pub tenant: String,
    pub project: String,
    pub object_id: String,
    pub effect: WorkEffect,
    pub changed_fields: BTreeMap<String, String>,
    pub recipients: BTreeSet<String>,
    pub visibility: String,
    pub attachment_sha256: BTreeSet<String>,
    pub approved_revision: String,
    pub approved_schema_sha256: String,
    pub approved_permissions_sha256: String,
    pub expected_postcondition_sha256: String,
    pub idempotency_sha256: String,
    pub grant_sha256: String,
    pub hidden_watcher_count: u32,
    pub cascading_effect_count: u32,
    pub project_move: bool,
}
/// State re-read immediately before one effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkSubmissionState {
    pub revision: String,
    pub schema_sha256: String,
    pub permissions_sha256: String,
    pub recipients: BTreeSet<String>,
    pub visibility: String,
    pub attachment_sha256: BTreeSet<String>,
}
/// Exact admitted effect with no executor or provider credential.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmittedWorkEffect {
    pub plan_id: String,
    pub effect: WorkEffect,
    pub changed_fields: BTreeMap<String, String>,
    pub expected_postcondition_sha256: String,
    pub receipt_sha256: String,
}
/// Stable work-management refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkManagementError {
    InvalidRecord,
    IdentityMismatch,
    ExtensionNamespaceMismatch,
    LinkNonAuthoritative,
    StaleApproval,
    HiddenEffect,
    UnsupportedEffect,
}

/// Validate provider identity, hashes, closed common fields, and namespaced extensions.
pub fn validate_work_item(item: &WorkItem) -> Result<(), WorkManagementError> {
    let ids = [
        &item.provider_id,
        &item.host,
        &item.tenant,
        &item.project,
        &item.immutable_id,
        &item.display_id,
        &item.object_type,
        &item.state,
        &item.revision,
    ];
    let hashes = [
        &item.field_schema_sha256,
        &item.permissions_sha256,
        &item.content_sha256,
        &item.comments_sha256,
        &item.attachments_sha256,
        &item.history_sha256,
    ];
    if ids.into_iter().any(|v| !valid_id(v))
        || hashes.into_iter().any(|v| !valid_sha(v))
        || !item.untrusted_content
    {
        return Err(WorkManagementError::InvalidRecord);
    }
    let prefix = format!("{}.", provider_wire(item.provider));
    if item
        .provider_extensions
        .iter()
        .any(|(key, value)| !key.starts_with(&prefix) || !valid_id(key) || !valid_sha(value))
    {
        return Err(WorkManagementError::ExtensionNamespaceMismatch);
    }
    Ok(())
}

/// Admit only observed or exact provider-reference links; inference never authorizes.
pub fn admit_work_link(item: &WorkItem, link: &WorkLink) -> Result<(), WorkManagementError> {
    validate_work_item(item)?;
    if link.work_item_id != item.immutable_id
        || !valid_id(&link.target_kind)
        || !valid_id(&link.target_immutable_id)
        || !valid_sha(&link.evidence_sha256)
    {
        return Err(WorkManagementError::IdentityMismatch);
    }
    if !matches!(
        link.state,
        WorkLinkState::Observed | WorkLinkState::ProviderReference
    ) {
        return Err(WorkManagementError::LinkNonAuthoritative);
    }
    Ok(())
}

/// Validate an immutable provider-backed workflow packet without granting effects.
pub fn validate_work_packet(packet: &ProviderWorkPacket) -> Result<(), WorkManagementError> {
    if !valid_id(&packet.packet_id)
        || !valid_sha(&packet.provider_identity_sha256)
        || packet.object_ids.is_empty()
        || packet.object_ids.iter().any(|v| !valid_id(v))
        || ![
            &packet.object_revision_sha256,
            &packet.field_schema_sha256,
            &packet.permissions_sha256,
            &packet.recipients_sha256,
            &packet.support_tuple_sha256,
        ]
        .into_iter()
        .all(|v| valid_sha(v))
        || packet.source_sha256.is_empty()
        || packet.source_sha256.iter().any(|v| !valid_sha(v))
        || packet.untrusted_input_sha256.iter().any(|v| !valid_sha(v))
        || !valid_id(&packet.visibility)
    {
        return Err(WorkManagementError::InvalidRecord);
    }
    Ok(())
}

/// Admit exactly one field-level effect after a complete mutable-state re-read.
pub fn admit_work_effect(
    packet: &ProviderWorkPacket,
    plan: &WorkEffectPlan,
    current: &WorkSubmissionState,
) -> Result<AdmittedWorkEffect, WorkManagementError> {
    validate_work_packet(packet)?;
    if packet.provider != plan.provider
        || !packet.object_ids.contains(&plan.object_id)
        || packet.maximum_effect != Some(plan.effect)
        || plan.packet_sha256 != work_packet_sha256(packet)
    {
        return Err(WorkManagementError::UnsupportedEffect);
    }
    if !valid_id(&plan.plan_id)
        || !valid_sha(&plan.packet_sha256)
        || !valid_id(&plan.host)
        || !valid_id(&plan.tenant)
        || !valid_id(&plan.project)
        || plan.changed_fields.is_empty()
        || plan
            .changed_fields
            .iter()
            .any(|(k, v)| !valid_id(k) || !valid_sha(v))
        || [
            &plan.approved_schema_sha256,
            &plan.approved_permissions_sha256,
            &plan.expected_postcondition_sha256,
            &plan.idempotency_sha256,
            &plan.grant_sha256,
        ]
        .into_iter()
        .any(|v| !valid_sha(v))
        || plan.attachment_sha256.iter().any(|v| !valid_sha(v))
    {
        return Err(WorkManagementError::InvalidRecord);
    }
    if plan.hidden_watcher_count != 0 || plan.cascading_effect_count != 0 || plan.project_move {
        return Err(WorkManagementError::HiddenEffect);
    }
    if current.revision != plan.approved_revision
        || current.schema_sha256 != plan.approved_schema_sha256
        || current.permissions_sha256 != plan.approved_permissions_sha256
        || current.recipients != plan.recipients
        || current.visibility != plan.visibility
        || current.attachment_sha256 != plan.attachment_sha256
    {
        return Err(WorkManagementError::StaleApproval);
    }
    Ok(AdmittedWorkEffect {
        plan_id: plan.plan_id.clone(),
        effect: plan.effect,
        changed_fields: plan.changed_fields.clone(),
        expected_postcondition_sha256: plan.expected_postcondition_sha256.clone(),
        receipt_sha256: plan.grant_sha256.clone(),
    })
}

fn provider_wire(value: WorkProvider) -> &'static str {
    match value {
        WorkProvider::GithubIssues => "github",
        WorkProvider::JiraCloud => "jira-cloud",
        WorkProvider::JiraDataCenter => "jira-data-center",
        WorkProvider::AzureBoards => "azure-boards",
    }
}
fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':' | b'/'))
}
fn valid_sha(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn item() -> WorkItem {
        let h = "a".repeat(64);
        WorkItem {
            provider: WorkProvider::JiraCloud,
            provider_id: "jira".into(),
            host: "jira.example.test".into(),
            tenant: "tenant".into(),
            project: "project".into(),
            immutable_id: "item-1".into(),
            display_id: "ONE-1".into(),
            object_type: "story".into(),
            state: "open".into(),
            revision: "r1".into(),
            parent_id: None,
            iteration: Some("sprint-1".into()),
            field_schema_sha256: h.clone(),
            permissions_sha256: h.clone(),
            content_sha256: h.clone(),
            comments_sha256: h.clone(),
            attachments_sha256: h.clone(),
            history_sha256: h.clone(),
            provider_extensions: BTreeMap::from([("jira-cloud.custom".into(), h)]),
            untrusted_content: true,
        }
    }
    fn packet() -> ProviderWorkPacket {
        let h = "a".repeat(64);
        ProviderWorkPacket {
            packet_id: "packet-1".into(),
            workflow: ProviderWorkflow::SprintPlanning,
            provider: WorkProvider::JiraCloud,
            provider_identity_sha256: h.clone(),
            repository_sha256: Some(h.clone()),
            base_revision: Some("base-1".into()),
            head_revision: Some("head-1".into()),
            object_ids: BTreeSet::from(["item-1".into()]),
            object_revision_sha256: h.clone(),
            field_schema_sha256: h.clone(),
            permissions_sha256: h.clone(),
            recipients_sha256: h.clone(),
            visibility: "private".into(),
            source_sha256: BTreeSet::from([h.clone()]),
            support_tuple_sha256: h.clone(),
            maximum_effect: Some(WorkEffect::Transition),
            untrusted_input_sha256: BTreeSet::from([h]),
        }
    }
    fn plan(packet: &ProviderWorkPacket) -> WorkEffectPlan {
        let h = "a".repeat(64);
        WorkEffectPlan {
            plan_id: "plan-1".into(),
            packet_sha256: work_packet_sha256(packet),
            provider: WorkProvider::JiraCloud,
            host: "jira.example.test".into(),
            tenant: "tenant".into(),
            project: "project".into(),
            object_id: "item-1".into(),
            effect: WorkEffect::Transition,
            changed_fields: BTreeMap::from([("state".into(), h.clone())]),
            recipients: BTreeSet::from(["user-1".into()]),
            visibility: "private".into(),
            attachment_sha256: BTreeSet::from([h.clone()]),
            approved_revision: "r1".into(),
            approved_schema_sha256: h.clone(),
            approved_permissions_sha256: h.clone(),
            expected_postcondition_sha256: h.clone(),
            idempotency_sha256: h.clone(),
            grant_sha256: h,
            hidden_watcher_count: 0,
            cascading_effect_count: 0,
            project_move: false,
        }
    }
    fn submission(plan: &WorkEffectPlan) -> WorkSubmissionState {
        WorkSubmissionState {
            revision: plan.approved_revision.clone(),
            schema_sha256: plan.approved_schema_sha256.clone(),
            permissions_sha256: plan.approved_permissions_sha256.clone(),
            recipients: plan.recipients.clone(),
            visibility: plan.visibility.clone(),
            attachment_sha256: plan.attachment_sha256.clone(),
        }
    }
    #[test]
    fn provider_extension_and_identity_are_exact() {
        assert!(validate_work_item(&item()).is_ok());
        let mut v = item();
        v.provider_extensions
            .insert("github.custom".into(), "a".repeat(64));
        assert_eq!(
            validate_work_item(&v),
            Err(WorkManagementError::ExtensionNamespaceMismatch)
        );
    }
    #[test]
    fn inference_cannot_authorize_link() {
        let v = item();
        let link = WorkLink {
            work_item_id: v.immutable_id.clone(),
            target_kind: "commit".into(),
            target_immutable_id: "commit-1".into(),
            evidence_sha256: "a".repeat(64),
            state: WorkLinkState::Inferred,
        };
        assert_eq!(
            admit_work_link(&v, &link),
            Err(WorkManagementError::LinkNonAuthoritative)
        );
    }
    #[test]
    fn exact_field_effect_is_admitted_once() {
        let packet = packet();
        let plan = plan(&packet);
        let admitted = admit_work_effect(&packet, &plan, &submission(&plan)).unwrap();
        assert_eq!(admitted.plan_id, "plan-1");
        assert_eq!(admitted.changed_fields.len(), 1);
    }
    #[test]
    fn stale_or_hidden_effect_is_refused() {
        let packet = packet();
        let mut plan = plan(&packet);
        let mut current = submission(&plan);
        current.revision = "r2".into();
        assert_eq!(
            admit_work_effect(&packet, &plan, &current),
            Err(WorkManagementError::StaleApproval)
        );
        plan.hidden_watcher_count = 1;
        assert_eq!(
            admit_work_effect(&packet, &plan, &submission(&plan)),
            Err(WorkManagementError::HiddenEffect)
        );
    }
}
