//! Exact infrastructure plan admission, apply verification, and recovery.
#![allow(missing_docs)]
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IaCTool {
    Terraform,
    OpenTofu,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IaCEffect {
    Create,
    Update,
    Replace,
    Delete,
    Import,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceEffect {
    pub address: String,
    pub effect: IaCEffect,
    pub before_sha256: Option<String>,
    pub after_sha256: Option<String>,
    pub destructive: bool,
    pub data_loss_sensitive: bool,
    pub cost_sensitive: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InfrastructurePlan {
    pub plan_sha256: String,
    pub tool: IaCTool,
    pub tool_version: String,
    pub source_sha256: String,
    pub lock_sha256: String,
    pub provider_set_sha256: String,
    pub module_set_sha256: String,
    pub backend_identity_sha256: String,
    pub workspace: String,
    pub state_lineage_sha256: String,
    pub state_serial: u64,
    pub variable_set_sha256: String,
    pub policy_sha256: String,
    pub state_lock_sha256: String,
    pub production: bool,
    pub production_grant: bool,
    pub destructive_grant: bool,
    pub secret_change: bool,
    pub administrative_change: bool,
    pub isolated_worker: bool,
    pub explicit_network_grant: bool,
    pub effects: BTreeMap<String, ResourceEffect>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApplyPreconditions {
    pub plan_sha256: String,
    pub source_sha256: String,
    pub lock_sha256: String,
    pub backend_identity_sha256: String,
    pub workspace: String,
    pub state_lineage_sha256: String,
    pub state_serial: u64,
    pub variable_set_sha256: String,
    pub policy_sha256: String,
    pub state_lock_sha256: String,
    pub ad_hoc_apply: bool,
    pub implicit_auto_approve: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplyOutcome {
    Applied,
    Partial,
    Unknown,
    Interrupted,
    ProviderFailed,
    LockLost,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InfrastructureReceipt {
    pub plan_sha256: String,
    pub prior_state_serial: u64,
    pub current_state_serial: u64,
    pub resource_results_sha256: String,
    pub outputs_redacted_sha256: String,
    pub drift_sha256: String,
    pub provider_requests_sha256: String,
    pub secret_value_count: u32,
    pub duplicate_apply_count: u32,
    pub state_surgery_count: u32,
    pub outcome: ApplyOutcome,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recovery {
    Complete,
    ReconcileCurrentInfrastructure,
    FreshPlanAndApprovalRequired,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IaCError {
    InvalidPlan,
    StalePlan,
    AuthorityMismatch,
    ReceiptMismatch,
}
pub fn validate_plan(p: &InfrastructurePlan) -> Result<(), IaCError> {
    if !valid_sha(&p.plan_sha256)
        || !valid_id(&p.tool_version)
        || [
            &p.source_sha256,
            &p.lock_sha256,
            &p.provider_set_sha256,
            &p.module_set_sha256,
            &p.backend_identity_sha256,
            &p.state_lineage_sha256,
            &p.variable_set_sha256,
            &p.policy_sha256,
            &p.state_lock_sha256,
        ]
        .into_iter()
        .any(|v| !valid_sha(v))
        || !valid_id(&p.workspace)
        || p.state_serial == 0
        || !p.isolated_worker
        || p.effects.is_empty()
        || p.effects.len() > 512
        || p.secret_change
        || p.administrative_change
    {
        return Err(IaCError::InvalidPlan);
    }
    if p.production != p.production_grant {
        return Err(IaCError::AuthorityMismatch);
    }
    for (id, e) in &p.effects {
        let states = match e.effect {
            IaCEffect::Create => e.before_sha256.is_none() && valid_some(&e.after_sha256),
            IaCEffect::Update | IaCEffect::Replace => {
                valid_some(&e.before_sha256)
                    && valid_some(&e.after_sha256)
                    && e.before_sha256 != e.after_sha256
            }
            IaCEffect::Delete => valid_some(&e.before_sha256) && e.after_sha256.is_none(),
            IaCEffect::Import => e.before_sha256.is_none() && valid_some(&e.after_sha256),
        };
        if id != &e.address
            || !valid_id(id)
            || !states
            || ((e.destructive || e.data_loss_sensitive) && !p.destructive_grant)
        {
            return Err(IaCError::AuthorityMismatch);
        }
    }
    Ok(())
}
pub fn admit_apply(p: &InfrastructurePlan, c: &ApplyPreconditions) -> Result<(), IaCError> {
    validate_plan(p)?;
    if c.ad_hoc_apply
        || c.implicit_auto_approve
        || c.plan_sha256 != p.plan_sha256
        || c.source_sha256 != p.source_sha256
        || c.lock_sha256 != p.lock_sha256
        || c.backend_identity_sha256 != p.backend_identity_sha256
        || c.workspace != p.workspace
        || c.state_lineage_sha256 != p.state_lineage_sha256
        || c.state_serial != p.state_serial
        || c.variable_set_sha256 != p.variable_set_sha256
        || c.policy_sha256 != p.policy_sha256
        || c.state_lock_sha256 != p.state_lock_sha256
    {
        return Err(IaCError::StalePlan);
    }
    Ok(())
}
pub fn verify_receipt(p: &InfrastructurePlan, r: &InfrastructureReceipt) -> Result<(), IaCError> {
    validate_plan(p)?;
    if r.plan_sha256 != p.plan_sha256
        || r.prior_state_serial != p.state_serial
        || r.current_state_serial < p.state_serial
        || !valid_sha(&r.resource_results_sha256)
        || !valid_sha(&r.outputs_redacted_sha256)
        || !valid_sha(&r.drift_sha256)
        || !valid_sha(&r.provider_requests_sha256)
        || r.secret_value_count != 0
        || r.duplicate_apply_count != 0
        || r.state_surgery_count != 0
    {
        return Err(IaCError::ReceiptMismatch);
    }
    Ok(())
}
pub fn recovery(r: &InfrastructureReceipt) -> Recovery {
    match r.outcome {
        ApplyOutcome::Applied if r.duplicate_apply_count == 0 => Recovery::Complete,
        ApplyOutcome::Unknown | ApplyOutcome::Interrupted | ApplyOutcome::LockLost => {
            Recovery::ReconcileCurrentInfrastructure
        }
        _ => Recovery::FreshPlanAndApprovalRequired,
    }
}
fn valid_id(v: &str) -> bool {
    !v.is_empty()
        && v.len() <= 512
        && v.bytes().all(|b| {
            b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':' | b'/' | b'[' | b']')
        })
}
fn valid_sha(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn valid_some(v: &Option<String>) -> bool {
    v.as_deref().is_some_and(valid_sha)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn plan() -> InfrastructurePlan {
        let h = "a".repeat(64);
        InfrastructurePlan {
            plan_sha256: h.clone(),
            tool: IaCTool::OpenTofu,
            tool_version: "1.8.0".into(),
            source_sha256: h.clone(),
            lock_sha256: h.clone(),
            provider_set_sha256: h.clone(),
            module_set_sha256: h.clone(),
            backend_identity_sha256: h.clone(),
            workspace: "staging".into(),
            state_lineage_sha256: h.clone(),
            state_serial: 7,
            variable_set_sha256: h.clone(),
            policy_sha256: h.clone(),
            state_lock_sha256: h.clone(),
            production: false,
            production_grant: false,
            destructive_grant: false,
            secret_change: false,
            administrative_change: false,
            isolated_worker: true,
            explicit_network_grant: false,
            effects: BTreeMap::from([(
                "module.app.resource.demo".into(),
                ResourceEffect {
                    address: "module.app.resource.demo".into(),
                    effect: IaCEffect::Update,
                    before_sha256: Some("b".repeat(64)),
                    after_sha256: Some("c".repeat(64)),
                    destructive: false,
                    data_loss_sensitive: false,
                    cost_sensitive: false,
                },
            )]),
        }
    }
    fn pre(p: &InfrastructurePlan) -> ApplyPreconditions {
        ApplyPreconditions {
            plan_sha256: p.plan_sha256.clone(),
            source_sha256: p.source_sha256.clone(),
            lock_sha256: p.lock_sha256.clone(),
            backend_identity_sha256: p.backend_identity_sha256.clone(),
            workspace: p.workspace.clone(),
            state_lineage_sha256: p.state_lineage_sha256.clone(),
            state_serial: p.state_serial,
            variable_set_sha256: p.variable_set_sha256.clone(),
            policy_sha256: p.policy_sha256.clone(),
            state_lock_sha256: p.state_lock_sha256.clone(),
            ad_hoc_apply: false,
            implicit_auto_approve: false,
        }
    }
    #[test]
    fn exact_saved_plan_is_admitted() {
        let p = plan();
        assert_eq!(validate_plan(&p), Ok(()));
        assert_eq!(admit_apply(&p, &pre(&p)), Ok(()));
    }
    #[test]
    fn changed_state_or_source_stales_plan() {
        let p = plan();
        let mut c = pre(&p);
        c.state_serial += 1;
        assert_eq!(admit_apply(&p, &c), Err(IaCError::StalePlan));
        c = pre(&p);
        c.source_sha256 = "d".repeat(64);
        assert_eq!(admit_apply(&p, &c), Err(IaCError::StalePlan));
    }
    #[test]
    fn destructive_and_admin_authority_are_separate() {
        let mut p = plan();
        let e = p.effects.values_mut().next().unwrap();
        e.effect = IaCEffect::Replace;
        e.destructive = true;
        assert_eq!(validate_plan(&p), Err(IaCError::AuthorityMismatch));
        p.administrative_change = true;
        assert_eq!(validate_plan(&p), Err(IaCError::InvalidPlan));
    }
    #[test]
    fn uncertain_apply_never_retries() {
        let p = plan();
        let h = "e".repeat(64);
        let r = InfrastructureReceipt {
            plan_sha256: p.plan_sha256.clone(),
            prior_state_serial: 7,
            current_state_serial: 7,
            resource_results_sha256: h.clone(),
            outputs_redacted_sha256: h.clone(),
            drift_sha256: h.clone(),
            provider_requests_sha256: h,
            secret_value_count: 0,
            duplicate_apply_count: 0,
            state_surgery_count: 0,
            outcome: ApplyOutcome::Unknown,
        };
        assert_eq!(verify_receipt(&p, &r), Ok(()));
        assert_eq!(recovery(&r), Recovery::ReconcileCurrentInfrastructure);
    }
}
