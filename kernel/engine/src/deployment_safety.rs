//! Exact offline deployment planning, effect verification, and rollback admission.
#![allow(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RendererKind {
    Helm,
    Kustomize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderProvenance {
    pub renderer: RendererKind,
    pub renderer_version: String,
    pub source_revision: String,
    pub dependency_digest: String,
    pub input_sha256: String,
    pub output_sha256: String,
    pub output_bytes: u64,
    pub offline: bool,
    pub deterministic: bool,
    pub source_preserved: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceAction {
    Create,
    Update,
    Delete,
    NoChange,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceChange {
    pub resource_id: String,
    pub kind: String,
    pub namespace: String,
    pub action: ResourceAction,
    pub pre_state_sha256: Option<String>,
    pub post_state_sha256: Option<String>,
    pub owner_sha256: String,
    pub policy_result_sha256: String,
    pub immutable_field_change: bool,
    pub secret_value_exposed: bool,
    pub administrative_change: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeploymentGrant {
    NonProductionDeploy,
    ProductionDeploy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeploymentPlan {
    pub plan_sha256: String,
    pub render: RenderProvenance,
    pub cluster_identity_sha256: String,
    pub context_identity_sha256: String,
    pub namespace: String,
    pub policy_sha256: String,
    pub actor_sha256: String,
    pub artifact_digest: String,
    pub health_criteria_sha256: String,
    pub timeout_seconds: u32,
    pub rollback_target_digest: String,
    pub production: bool,
    pub grant: DeploymentGrant,
    pub changes: BTreeMap<String, ResourceChange>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeploymentApproval {
    pub plan_sha256: String,
    pub cluster_identity_sha256: String,
    pub namespace: String,
    pub policy_sha256: String,
    pub artifact_digest: String,
    pub resource_preconditions_sha256: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplyState {
    Healthy,
    HealthFailed,
    Partial,
    UnknownEffect,
    CancelledBeforeApply,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApplyReceipt {
    pub plan_sha256: String,
    pub cluster_identity_sha256: String,
    pub namespace: String,
    pub artifact_digest: String,
    pub resource_pre_sha256: BTreeMap<String, Option<String>>,
    pub resource_post_sha256: BTreeMap<String, Option<String>>,
    pub rollout_sha256: String,
    pub health_window_sha256: String,
    pub event_set_sha256: String,
    pub metrics_reference_sha256: String,
    pub drift_resource_ids: BTreeSet<String>,
    pub state: ApplyState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryDisposition {
    Complete,
    ReconcileOnly,
    FreshRollbackApprovalRequired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeploymentError {
    InvalidRender,
    InvalidPlan,
    StaleApproval,
    ReceiptMismatch,
}

pub fn validate_plan(plan: &DeploymentPlan) -> Result<(), DeploymentError> {
    let render = &plan.render;
    if !valid_id(&render.renderer_version)
        || !valid_id(&render.source_revision)
        || !valid_digest(&render.dependency_digest)
        || !valid_sha(&render.input_sha256)
        || !valid_sha(&render.output_sha256)
        || render.output_bytes == 0
        || render.output_bytes > 4 * 1024 * 1024
        || !render.offline
        || !render.deterministic
        || !render.source_preserved
    {
        return Err(DeploymentError::InvalidRender);
    }
    if !valid_sha(&plan.plan_sha256)
        || !valid_sha(&plan.cluster_identity_sha256)
        || !valid_sha(&plan.context_identity_sha256)
        || !valid_id(&plan.namespace)
        || !valid_sha(&plan.policy_sha256)
        || !valid_sha(&plan.actor_sha256)
        || !valid_digest(&plan.artifact_digest)
        || !valid_sha(&plan.health_criteria_sha256)
        || plan.timeout_seconds == 0
        || plan.timeout_seconds > 3600
        || !valid_digest(&plan.rollback_target_digest)
        || plan.changes.is_empty()
        || plan.changes.len() > 512
        || plan.production != matches!(plan.grant, DeploymentGrant::ProductionDeploy)
    {
        return Err(DeploymentError::InvalidPlan);
    }
    for (id, change) in &plan.changes {
        let states_valid = match change.action {
            ResourceAction::Create => {
                change.pre_state_sha256.is_none() && valid_optional_sha(&change.post_state_sha256)
            }
            ResourceAction::Update => {
                valid_optional_sha(&change.pre_state_sha256)
                    && valid_optional_sha(&change.post_state_sha256)
                    && change.pre_state_sha256 != change.post_state_sha256
            }
            ResourceAction::Delete => {
                valid_optional_sha(&change.pre_state_sha256) && change.post_state_sha256.is_none()
            }
            ResourceAction::NoChange => {
                valid_optional_sha(&change.pre_state_sha256)
                    && change.pre_state_sha256 == change.post_state_sha256
            }
        };
        if id != &change.resource_id
            || !valid_id(id)
            || !valid_id(&change.kind)
            || change.namespace != plan.namespace
            || !valid_sha(&change.owner_sha256)
            || !valid_sha(&change.policy_result_sha256)
            || !states_valid
            || change.immutable_field_change
            || change.secret_value_exposed
            || change.administrative_change
        {
            return Err(DeploymentError::InvalidPlan);
        }
    }
    Ok(())
}

pub fn approval_is_current(plan: &DeploymentPlan, approval: &DeploymentApproval) -> bool {
    approval.plan_sha256 == plan.plan_sha256
        && approval.cluster_identity_sha256 == plan.cluster_identity_sha256
        && approval.namespace == plan.namespace
        && approval.policy_sha256 == plan.policy_sha256
        && approval.artifact_digest == plan.artifact_digest
        && valid_sha(&approval.resource_preconditions_sha256)
}

pub fn verify_receipt(
    plan: &DeploymentPlan,
    receipt: &ApplyReceipt,
) -> Result<(), DeploymentError> {
    validate_plan(plan)?;
    if receipt.plan_sha256 != plan.plan_sha256
        || receipt.cluster_identity_sha256 != plan.cluster_identity_sha256
        || receipt.namespace != plan.namespace
        || receipt.artifact_digest != plan.artifact_digest
        || !valid_sha(&receipt.rollout_sha256)
        || !valid_sha(&receipt.health_window_sha256)
        || !valid_sha(&receipt.event_set_sha256)
        || !valid_sha(&receipt.metrics_reference_sha256)
    {
        return Err(DeploymentError::ReceiptMismatch);
    }
    for (id, change) in &plan.changes {
        if receipt.resource_pre_sha256.get(id) != Some(&change.pre_state_sha256)
            || receipt.resource_post_sha256.get(id) != Some(&change.post_state_sha256)
        {
            return Err(DeploymentError::ReceiptMismatch);
        }
    }
    if receipt.resource_pre_sha256.len() != plan.changes.len()
        || receipt.resource_post_sha256.len() != plan.changes.len()
        || receipt
            .drift_resource_ids
            .iter()
            .any(|id| !plan.changes.contains_key(id))
    {
        return Err(DeploymentError::ReceiptMismatch);
    }
    Ok(())
}

pub fn recovery_disposition(receipt: &ApplyReceipt) -> RecoveryDisposition {
    if receipt.state == ApplyState::Healthy && receipt.drift_resource_ids.is_empty() {
        RecoveryDisposition::Complete
    } else if matches!(
        receipt.state,
        ApplyState::UnknownEffect | ApplyState::CancelledBeforeApply
    ) {
        RecoveryDisposition::ReconcileOnly
    } else {
        RecoveryDisposition::FreshRollbackApprovalRequired
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
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

fn valid_digest(value: &str) -> bool {
    value.starts_with("sha256:") && valid_sha(&value[7..])
}

fn valid_optional_sha(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(valid_sha)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan() -> DeploymentPlan {
        let h = "a".repeat(64);
        let mut changes = BTreeMap::new();
        changes.insert(
            "apps/v1/deployment/demo".into(),
            ResourceChange {
                resource_id: "apps/v1/deployment/demo".into(),
                kind: "Deployment".into(),
                namespace: "staging".into(),
                action: ResourceAction::Update,
                pre_state_sha256: Some("b".repeat(64)),
                post_state_sha256: Some("c".repeat(64)),
                owner_sha256: h.clone(),
                policy_result_sha256: h.clone(),
                immutable_field_change: false,
                secret_value_exposed: false,
                administrative_change: false,
            },
        );
        DeploymentPlan {
            plan_sha256: h.clone(),
            render: RenderProvenance {
                renderer: RendererKind::Helm,
                renderer_version: "3.16.0".into(),
                source_revision: "rev-1".into(),
                dependency_digest: format!("sha256:{h}"),
                input_sha256: h.clone(),
                output_sha256: h.clone(),
                output_bytes: 1024,
                offline: true,
                deterministic: true,
                source_preserved: true,
            },
            cluster_identity_sha256: h.clone(),
            context_identity_sha256: h.clone(),
            namespace: "staging".into(),
            policy_sha256: h.clone(),
            actor_sha256: h.clone(),
            artifact_digest: format!("sha256:{h}"),
            health_criteria_sha256: h.clone(),
            timeout_seconds: 300,
            rollback_target_digest: format!("sha256:{}", "d".repeat(64)),
            production: false,
            grant: DeploymentGrant::NonProductionDeploy,
            changes,
        }
    }

    #[test]
    fn offline_bounded_plan_is_valid() {
        assert_eq!(validate_plan(&plan()), Ok(()));
    }

    #[test]
    fn production_and_admin_authority_are_separate() {
        let mut value = plan();
        value.production = true;
        assert_eq!(validate_plan(&value), Err(DeploymentError::InvalidPlan));
        value.production = false;
        value
            .changes
            .values_mut()
            .next()
            .unwrap()
            .administrative_change = true;
        assert_eq!(validate_plan(&value), Err(DeploymentError::InvalidPlan));
    }

    #[test]
    fn changed_precondition_stales_approval() {
        let value = plan();
        let mut approval = DeploymentApproval {
            plan_sha256: value.plan_sha256.clone(),
            cluster_identity_sha256: value.cluster_identity_sha256.clone(),
            namespace: value.namespace.clone(),
            policy_sha256: value.policy_sha256.clone(),
            artifact_digest: value.artifact_digest.clone(),
            resource_preconditions_sha256: "e".repeat(64),
        };
        assert!(approval_is_current(&value, &approval));
        approval.namespace = "production".into();
        assert!(!approval_is_current(&value, &approval));
    }

    #[test]
    fn partial_and_unknown_effects_never_retry() {
        let value = plan();
        let pre = value
            .changes
            .iter()
            .map(|(id, c)| (id.clone(), c.pre_state_sha256.clone()))
            .collect();
        let post = value
            .changes
            .iter()
            .map(|(id, c)| (id.clone(), c.post_state_sha256.clone()))
            .collect();
        let mut receipt = ApplyReceipt {
            plan_sha256: value.plan_sha256.clone(),
            cluster_identity_sha256: value.cluster_identity_sha256.clone(),
            namespace: value.namespace.clone(),
            artifact_digest: value.artifact_digest.clone(),
            resource_pre_sha256: pre,
            resource_post_sha256: post,
            rollout_sha256: "f".repeat(64),
            health_window_sha256: "f".repeat(64),
            event_set_sha256: "f".repeat(64),
            metrics_reference_sha256: "f".repeat(64),
            drift_resource_ids: BTreeSet::new(),
            state: ApplyState::Partial,
        };
        assert_eq!(verify_receipt(&value, &receipt), Ok(()));
        assert_eq!(
            recovery_disposition(&receipt),
            RecoveryDisposition::FreshRollbackApprovalRequired
        );
        receipt.state = ApplyState::UnknownEffect;
        assert_eq!(
            recovery_disposition(&receipt),
            RecoveryDisposition::ReconcileOnly
        );
    }
}
