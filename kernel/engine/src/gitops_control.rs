//! Exact GitOps observation, bounded synchronization, and controller reconciliation.
#![allow(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitOpsProvider {
    ArgoCd,
    Flux,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GitOpsEffect {
    Sync,
    Reconcile,
    Prune,
    Force,
    Replace,
    Hook,
    Suspend,
    Resume,
    Rollback,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitOpsGrant {
    OrdinarySync,
    Suspend,
    Resume,
    Rollback,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitOpsSnapshot {
    pub provider: GitOpsProvider,
    pub controller_identity_sha256: String,
    pub application_identity_sha256: String,
    pub repository_identity_sha256: String,
    pub source_revision: String,
    pub rendered_resources_sha256: String,
    pub artifact_digest: String,
    pub cluster_identity_sha256: String,
    pub namespace: String,
    pub live_state_sha256: String,
    pub desired_state_sha256: String,
    pub prior_sync_sha256: String,
    pub auto_sync_enabled: bool,
    pub health_sha256: String,
    pub drift_sha256: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitOpsPlan {
    pub plan_sha256: String,
    pub snapshot: GitOpsSnapshot,
    pub effect: GitOpsEffect,
    pub grant: GitOpsGrant,
    pub resource_diffs: BTreeMap<String, String>,
    pub policy_sha256: String,
    pub health_criteria_sha256: String,
    pub timeout_seconds: u32,
    pub idempotency_key_sha256: String,
    pub desired_state_write: bool,
    pub secret_change: bool,
    pub administrative_change: bool,
    pub embedded_effects: BTreeSet<GitOpsEffect>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControllerOutcome {
    Healthy,
    Partial,
    Unknown,
    Cancelled,
    ControllerChanged,
    DesiredStateChanged,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitOpsReceipt {
    pub plan_sha256: String,
    pub controller_identity_sha256: String,
    pub source_revision: String,
    pub destination_sha256: String,
    pub resource_effects_sha256: String,
    pub health_sha256: String,
    pub observed_live_state_sha256: String,
    pub outcome: ControllerOutcome,
    pub duplicate_effect_count: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reconciliation {
    Complete,
    ObserveUntilKnown,
    FreshPlanAndApprovalRequired,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitOpsError {
    InvalidSnapshot,
    InvalidPlan,
    StrongerEffect,
    ReceiptMismatch,
}

pub fn validate_plan(plan: &GitOpsPlan) -> Result<(), GitOpsError> {
    let s = &plan.snapshot;
    if [
        &s.controller_identity_sha256,
        &s.application_identity_sha256,
        &s.repository_identity_sha256,
        &s.rendered_resources_sha256,
        &s.cluster_identity_sha256,
        &s.live_state_sha256,
        &s.desired_state_sha256,
        &s.prior_sync_sha256,
        &s.health_sha256,
        &s.drift_sha256,
    ]
    .into_iter()
    .any(|v| !valid_sha(v))
        || !valid_id(&s.source_revision)
        || !valid_id(&s.namespace)
        || !valid_digest(&s.artifact_digest)
    {
        return Err(GitOpsError::InvalidSnapshot);
    }
    if !valid_sha(&plan.plan_sha256)
        || !valid_sha(&plan.policy_sha256)
        || !valid_sha(&plan.health_criteria_sha256)
        || !valid_sha(&plan.idempotency_key_sha256)
        || plan.timeout_seconds == 0
        || plan.timeout_seconds > 3600
        || plan.resource_diffs.is_empty()
        || plan.resource_diffs.len() > 512
        || plan.desired_state_write
        || plan.secret_change
        || plan.administrative_change
    {
        return Err(GitOpsError::InvalidPlan);
    }
    let grant_matches = matches!(
        (plan.effect, plan.grant),
        (
            GitOpsEffect::Sync | GitOpsEffect::Reconcile,
            GitOpsGrant::OrdinarySync
        ) | (GitOpsEffect::Suspend, GitOpsGrant::Suspend)
            | (GitOpsEffect::Resume, GitOpsGrant::Resume)
            | (GitOpsEffect::Rollback, GitOpsGrant::Rollback)
    );
    if !grant_matches
        || plan.embedded_effects.iter().any(|e| {
            matches!(
                e,
                GitOpsEffect::Prune
                    | GitOpsEffect::Force
                    | GitOpsEffect::Replace
                    | GitOpsEffect::Hook
            )
        })
    {
        return Err(GitOpsError::StrongerEffect);
    }
    if plan
        .resource_diffs
        .iter()
        .any(|(id, digest)| !valid_id(id) || !valid_sha(digest))
    {
        return Err(GitOpsError::InvalidPlan);
    }
    Ok(())
}
pub fn state_is_current(plan: &GitOpsPlan, current: &GitOpsSnapshot) -> bool {
    plan.snapshot == *current
}
pub fn verify_receipt(plan: &GitOpsPlan, receipt: &GitOpsReceipt) -> Result<(), GitOpsError> {
    validate_plan(plan)?;
    if receipt.plan_sha256 != plan.plan_sha256
        || receipt.controller_identity_sha256 != plan.snapshot.controller_identity_sha256
        || receipt.source_revision != plan.snapshot.source_revision
        || !valid_sha(&receipt.destination_sha256)
        || !valid_sha(&receipt.resource_effects_sha256)
        || !valid_sha(&receipt.health_sha256)
        || !valid_sha(&receipt.observed_live_state_sha256)
        || receipt.duplicate_effect_count != 0
    {
        return Err(GitOpsError::ReceiptMismatch);
    }
    Ok(())
}
pub fn reconcile(receipt: &GitOpsReceipt) -> Reconciliation {
    match receipt.outcome {
        ControllerOutcome::Healthy if receipt.duplicate_effect_count == 0 => {
            Reconciliation::Complete
        }
        ControllerOutcome::Unknown | ControllerOutcome::Cancelled => {
            Reconciliation::ObserveUntilKnown
        }
        _ => Reconciliation::FreshPlanAndApprovalRequired,
    }
}
fn valid_id(v: &str) -> bool {
    !v.is_empty()
        && v.len() <= 512
        && v.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':' | b'/'))
}
fn valid_sha(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn valid_digest(v: &str) -> bool {
    v.starts_with("sha256:") && valid_sha(&v[7..])
}

#[cfg(test)]
mod tests {
    use super::*;
    fn plan() -> GitOpsPlan {
        let h = "a".repeat(64);
        let snapshot = GitOpsSnapshot {
            provider: GitOpsProvider::ArgoCd,
            controller_identity_sha256: h.clone(),
            application_identity_sha256: h.clone(),
            repository_identity_sha256: h.clone(),
            source_revision: "rev-1".into(),
            rendered_resources_sha256: h.clone(),
            artifact_digest: format!("sha256:{h}"),
            cluster_identity_sha256: h.clone(),
            namespace: "staging".into(),
            live_state_sha256: h.clone(),
            desired_state_sha256: "b".repeat(64),
            prior_sync_sha256: h.clone(),
            auto_sync_enabled: false,
            health_sha256: h.clone(),
            drift_sha256: h.clone(),
        };
        GitOpsPlan {
            plan_sha256: h.clone(),
            snapshot,
            effect: GitOpsEffect::Sync,
            grant: GitOpsGrant::OrdinarySync,
            resource_diffs: BTreeMap::from([("apps/v1/deployment/demo".into(), h.clone())]),
            policy_sha256: h.clone(),
            health_criteria_sha256: h.clone(),
            timeout_seconds: 300,
            idempotency_key_sha256: h,
            desired_state_write: false,
            secret_change: false,
            administrative_change: false,
            embedded_effects: BTreeSet::new(),
        }
    }
    #[test]
    fn ordinary_sync_is_bounded() {
        assert_eq!(validate_plan(&plan()), Ok(()));
    }
    #[test]
    fn stronger_and_repository_effects_are_refused() {
        let mut p = plan();
        p.embedded_effects.insert(GitOpsEffect::Prune);
        assert_eq!(validate_plan(&p), Err(GitOpsError::StrongerEffect));
        p.embedded_effects.clear();
        p.desired_state_write = true;
        assert_eq!(validate_plan(&p), Err(GitOpsError::InvalidPlan));
    }
    #[test]
    fn controller_or_desired_state_change_stales_plan() {
        let p = plan();
        let mut current = p.snapshot.clone();
        assert!(state_is_current(&p, &current));
        current.desired_state_sha256 = "c".repeat(64);
        assert!(!state_is_current(&p, &current));
    }
    #[test]
    fn uncertain_or_partial_outcomes_never_retry() {
        let p = plan();
        let h = "d".repeat(64);
        let mut r = GitOpsReceipt {
            plan_sha256: p.plan_sha256.clone(),
            controller_identity_sha256: p.snapshot.controller_identity_sha256.clone(),
            source_revision: p.snapshot.source_revision.clone(),
            destination_sha256: h.clone(),
            resource_effects_sha256: h.clone(),
            health_sha256: h.clone(),
            observed_live_state_sha256: h,
            outcome: ControllerOutcome::Unknown,
            duplicate_effect_count: 0,
        };
        assert_eq!(verify_receipt(&p, &r), Ok(()));
        assert_eq!(reconcile(&r), Reconciliation::ObserveUntilKnown);
        r.outcome = ControllerOutcome::Partial;
        assert_eq!(reconcile(&r), Reconciliation::FreshPlanAndApprovalRequired);
    }
}
