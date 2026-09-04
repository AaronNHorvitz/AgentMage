//! Provider-neutral continuous-integration observation and effect admission.
#![allow(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CiProvider {
    GithubActions,
    AzurePipelines,
    GitLabCi,
    Jenkins,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CiEffect {
    Dispatch,
    Rerun,
    Cancel,
    EnvironmentApproval,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CiResultState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Partial,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CiRunSnapshot {
    pub provider: CiProvider,
    pub canonical_host: String,
    pub tenant: String,
    pub project: String,
    pub repository_id: String,
    pub workflow_id: String,
    pub run_id: String,
    pub attempt_id: String,
    pub queue_id: String,
    pub immutable_revision: String,
    pub actor_id: String,
    pub environment_id: String,
    pub worker_identity_sha256: String,
    pub inputs_sha256: String,
    pub permissions_sha256: String,
    pub definition_sha256: String,
    pub result: CiResultState,
    pub log_sha256: String,
    pub artifact_inventory_sha256: String,
    pub receipt_sha256: String,
    pub variable_names: BTreeSet<String>,
    pub secret_reference_names: BTreeSet<String>,
    pub secret_value_count: u32,
    pub definition_untrusted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CiEffectPlan {
    pub plan_id: String,
    pub provider: CiProvider,
    pub effect: CiEffect,
    pub canonical_host: String,
    pub tenant: String,
    pub project: String,
    pub repository_id: String,
    pub workflow_id: String,
    pub source_revision: String,
    pub run_id: Option<String>,
    pub attempt_id: Option<String>,
    pub queue_id: Option<String>,
    pub inputs: BTreeMap<String, String>,
    pub environment_id: String,
    pub permissions_sha256: String,
    pub budget_sha256: String,
    pub expected_artifacts_sha256: String,
    pub cancellation_contract_sha256: String,
    pub idempotency_sha256: String,
    pub grant_sha256: String,
    pub deployment_effect_count: u32,
    pub secret_effect_count: u32,
    pub administration_effect_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CiOutputAdmission {
    pub log_sha256: String,
    pub artifact_inventory_sha256: String,
    pub byte_count: u64,
    pub secret_match_count: u32,
    pub unsafe_archive_entry_count: u32,
    pub retained: bool,
    pub cancelled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmittedCiEffect {
    pub plan_id: String,
    pub effect: CiEffect,
    pub idempotency_sha256: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CiControlError {
    InvalidRecord,
    StaleApproval,
    HiddenAuthority,
    UnsafeOutput,
    DuplicateExecution,
}

pub fn validate_ci_run(snapshot: &CiRunSnapshot) -> Result<(), CiControlError> {
    let ids = [
        &snapshot.canonical_host,
        &snapshot.tenant,
        &snapshot.project,
        &snapshot.repository_id,
        &snapshot.workflow_id,
        &snapshot.run_id,
        &snapshot.attempt_id,
        &snapshot.queue_id,
        &snapshot.immutable_revision,
        &snapshot.actor_id,
        &snapshot.environment_id,
    ];
    let hashes = [
        &snapshot.worker_identity_sha256,
        &snapshot.inputs_sha256,
        &snapshot.permissions_sha256,
        &snapshot.definition_sha256,
        &snapshot.log_sha256,
        &snapshot.artifact_inventory_sha256,
        &snapshot.receipt_sha256,
    ];
    if ids.into_iter().any(|v| !valid_id(v))
        || hashes.into_iter().any(|v| !valid_sha(v))
        || snapshot.variable_names.iter().any(|v| !valid_id(v))
        || snapshot.secret_reference_names.iter().any(|v| !valid_id(v))
        || snapshot.secret_value_count != 0
        || !snapshot.definition_untrusted
    {
        return Err(CiControlError::InvalidRecord);
    }
    Ok(())
}

pub fn admit_ci_effect(
    current: &CiRunSnapshot,
    plan: &CiEffectPlan,
    prior_idempotency_keys: &BTreeSet<String>,
) -> Result<AdmittedCiEffect, CiControlError> {
    validate_ci_run(current)?;
    if plan.provider != current.provider
        || plan.canonical_host != current.canonical_host
        || plan.tenant != current.tenant
        || plan.project != current.project
        || plan.repository_id != current.repository_id
        || plan.workflow_id != current.workflow_id
        || plan.source_revision != current.immutable_revision
        || plan.environment_id != current.environment_id
        || plan.permissions_sha256 != current.permissions_sha256
    {
        return Err(CiControlError::StaleApproval);
    }
    if !valid_id(&plan.plan_id)
        || plan
            .inputs
            .iter()
            .any(|(k, v)| !valid_id(k) || !valid_sha(v))
        || [
            &plan.budget_sha256,
            &plan.expected_artifacts_sha256,
            &plan.cancellation_contract_sha256,
            &plan.idempotency_sha256,
            &plan.grant_sha256,
        ]
        .into_iter()
        .any(|v| !valid_sha(v))
    {
        return Err(CiControlError::InvalidRecord);
    }
    if plan.deployment_effect_count != 0
        || plan.secret_effect_count != 0
        || plan.administration_effect_count != 0
    {
        return Err(CiControlError::HiddenAuthority);
    }
    let existing = matches!(
        plan.effect,
        CiEffect::Rerun | CiEffect::Cancel | CiEffect::EnvironmentApproval
    );
    if existing
        != (plan.run_id.as_deref() == Some(current.run_id.as_str())
            && plan.attempt_id.as_deref() == Some(current.attempt_id.as_str())
            && plan.queue_id.as_deref() == Some(current.queue_id.as_str()))
    {
        return Err(CiControlError::StaleApproval);
    }
    if prior_idempotency_keys.contains(&plan.idempotency_sha256) {
        return Err(CiControlError::DuplicateExecution);
    }
    Ok(AdmittedCiEffect {
        plan_id: plan.plan_id.clone(),
        effect: plan.effect,
        idempotency_sha256: plan.idempotency_sha256.clone(),
    })
}

pub fn admit_ci_output(
    output: &CiOutputAdmission,
    maximum_bytes: u64,
) -> Result<(), CiControlError> {
    if !valid_sha(&output.log_sha256)
        || !valid_sha(&output.artifact_inventory_sha256)
        || output.byte_count > maximum_bytes
        || output.secret_match_count != 0
        || output.unsafe_archive_entry_count != 0
        || !output.retained
        || output.cancelled
    {
        return Err(CiControlError::UnsafeOutput);
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    fn run() -> CiRunSnapshot {
        let h = "a".repeat(64);
        CiRunSnapshot {
            provider: CiProvider::GitLabCi,
            canonical_host: "gitlab.example.test".into(),
            tenant: "tenant".into(),
            project: "project".into(),
            repository_id: "repo-1".into(),
            workflow_id: "workflow-1".into(),
            run_id: "run-1".into(),
            attempt_id: "attempt-1".into(),
            queue_id: "queue-1".into(),
            immutable_revision: "commit-1".into(),
            actor_id: "actor-1".into(),
            environment_id: "test".into(),
            worker_identity_sha256: h.clone(),
            inputs_sha256: h.clone(),
            permissions_sha256: h.clone(),
            definition_sha256: h.clone(),
            result: CiResultState::Running,
            log_sha256: h.clone(),
            artifact_inventory_sha256: h.clone(),
            receipt_sha256: h,
            variable_names: BTreeSet::from(["MODE".into()]),
            secret_reference_names: BTreeSet::from(["TOKEN_REF".into()]),
            secret_value_count: 0,
            definition_untrusted: true,
        }
    }
    fn plan(run: &CiRunSnapshot, effect: CiEffect) -> CiEffectPlan {
        let h = "b".repeat(64);
        let existing = effect != CiEffect::Dispatch;
        CiEffectPlan {
            plan_id: "plan-1".into(),
            provider: run.provider,
            effect,
            canonical_host: run.canonical_host.clone(),
            tenant: run.tenant.clone(),
            project: run.project.clone(),
            repository_id: run.repository_id.clone(),
            workflow_id: run.workflow_id.clone(),
            source_revision: run.immutable_revision.clone(),
            run_id: existing.then(|| run.run_id.clone()),
            attempt_id: existing.then(|| run.attempt_id.clone()),
            queue_id: existing.then(|| run.queue_id.clone()),
            inputs: BTreeMap::from([("mode".into(), h.clone())]),
            environment_id: run.environment_id.clone(),
            permissions_sha256: run.permissions_sha256.clone(),
            budget_sha256: h.clone(),
            expected_artifacts_sha256: h.clone(),
            cancellation_contract_sha256: h.clone(),
            idempotency_sha256: h.clone(),
            grant_sha256: h,
            deployment_effect_count: 0,
            secret_effect_count: 0,
            administration_effect_count: 0,
        }
    }
    #[test]
    fn exact_dispatch_is_admitted_once() {
        let r = run();
        let p = plan(&r, CiEffect::Dispatch);
        assert!(admit_ci_effect(&r, &p, &BTreeSet::new()).is_ok());
        assert_eq!(
            admit_ci_effect(&r, &p, &BTreeSet::from([p.idempotency_sha256.clone()])),
            Err(CiControlError::DuplicateExecution)
        );
    }
    #[test]
    fn stale_or_stronger_effect_is_denied() {
        let r = run();
        let mut p = plan(&r, CiEffect::Rerun);
        p.source_revision = "other".into();
        assert_eq!(
            admit_ci_effect(&r, &p, &BTreeSet::new()),
            Err(CiControlError::StaleApproval)
        );
        p.source_revision = r.immutable_revision.clone();
        p.deployment_effect_count = 1;
        assert_eq!(
            admit_ci_effect(&r, &p, &BTreeSet::new()),
            Err(CiControlError::HiddenAuthority)
        );
    }
    #[test]
    fn output_requires_bounds_scan_and_safe_archive() {
        let h = "c".repeat(64);
        let mut o = CiOutputAdmission {
            log_sha256: h.clone(),
            artifact_inventory_sha256: h,
            byte_count: 100,
            secret_match_count: 0,
            unsafe_archive_entry_count: 0,
            retained: true,
            cancelled: false,
        };
        assert!(admit_ci_output(&o, 100).is_ok());
        o.secret_match_count = 1;
        assert_eq!(admit_ci_output(&o, 100), Err(CiControlError::UnsafeOutput));
    }
}
