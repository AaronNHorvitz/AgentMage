//! Exact release manifests, separately granted changes, and safe compensation.
#![allow(missing_docs)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseManifest {
    pub release_id: String,
    pub semantic_version: String,
    pub source_sha256: String,
    pub ci_sha256: String,
    pub artifact_digest: String,
    pub sbom_sha256: String,
    pub provenance_sha256: String,
    pub signature_sha256: String,
    pub environment_sha256: String,
    pub policy_sha256: String,
    pub migrations_sha256: String,
    pub flags_sha256: String,
    pub health_sha256: String,
    pub rollback_sha256: String,
    pub prior_release_sha256: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeKind {
    EnvironmentPromotion,
    ProductionFlag,
    ProgressiveStep,
    Migration,
    DestructiveMigration,
    Rollback,
    Compensation,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangePlan {
    pub plan_sha256: String,
    pub release: ReleaseManifest,
    pub kind: ChangeKind,
    pub source_environment_sha256: String,
    pub target_environment_sha256: String,
    pub approval_class: String,
    pub deployment_plan_sha256: String,
    pub health_window_sha256: String,
    pub postcondition_sha256: String,
    pub flag_identity_sha256: Option<String>,
    pub flag_prerequisites_sha256: Option<String>,
    pub audience_basis_points: Option<u16>,
    pub migration_tool: Option<String>,
    pub migration_checksum_sha256: Option<String>,
    pub migration_order: Option<u32>,
    pub schema_state_sha256: Option<String>,
    pub backup_sha256: Option<String>,
    pub production_approval: bool,
    pub migration_approval: bool,
    pub destructive_approval: bool,
    pub autonomous_advance: bool,
    pub later_state_sha256: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeOutcome {
    Healthy,
    FailedHealth,
    PartialMigration,
    Unknown,
    Cancelled,
    LaterStateChanged,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeReceipt {
    pub plan_sha256: String,
    pub release_id: String,
    pub pre_state_sha256: String,
    pub post_state_sha256: String,
    pub health_sha256: String,
    pub flag_state_sha256: String,
    pub schema_state_sha256: String,
    pub secret_disclosure_count: u32,
    pub automatic_advance_count: u32,
    pub automatic_retry_count: u32,
    pub outcome: ChangeOutcome,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recovery {
    Complete,
    ReconcileOnly,
    FreshCompensationPlanRequired,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReleaseError {
    InvalidManifest,
    InvalidPlan,
    AuthorityMismatch,
    ReceiptMismatch,
}
pub fn validate_plan(p: &ChangePlan) -> Result<(), ReleaseError> {
    let r = &p.release;
    if !valid_id(&r.release_id)
        || !valid_version(&r.semantic_version)
        || !valid_digest(&r.artifact_digest)
        || [
            &r.source_sha256,
            &r.ci_sha256,
            &r.sbom_sha256,
            &r.provenance_sha256,
            &r.signature_sha256,
            &r.environment_sha256,
            &r.policy_sha256,
            &r.migrations_sha256,
            &r.flags_sha256,
            &r.health_sha256,
            &r.rollback_sha256,
            &r.prior_release_sha256,
        ]
        .into_iter()
        .any(|v| !valid_sha(v))
    {
        return Err(ReleaseError::InvalidManifest);
    }
    if !valid_sha(&p.plan_sha256)
        || !valid_sha(&p.source_environment_sha256)
        || !valid_sha(&p.target_environment_sha256)
        || !valid_id(&p.approval_class)
        || [
            &p.deployment_plan_sha256,
            &p.health_window_sha256,
            &p.postcondition_sha256,
            &p.later_state_sha256,
        ]
        .into_iter()
        .any(|v| !valid_sha(v))
        || p.autonomous_advance
    {
        return Err(ReleaseError::InvalidPlan);
    }
    let auth = match p.kind {
        ChangeKind::EnvironmentPromotion => !p.production_approval,
        ChangeKind::ProductionFlag => {
            p.production_approval
                && valid_opt(&p.flag_identity_sha256)
                && valid_opt(&p.flag_prerequisites_sha256)
        }
        ChangeKind::ProgressiveStep => p.audience_basis_points.is_some_and(|v| v <= 10_000),
        ChangeKind::Migration => p.migration_approval && migration_valid(p),
        ChangeKind::DestructiveMigration => {
            p.migration_approval && p.destructive_approval && migration_valid(p)
        }
        ChangeKind::Rollback | ChangeKind::Compensation => p.approval_class == "fresh-compensation",
    };
    if !auth {
        return Err(ReleaseError::AuthorityMismatch);
    }
    Ok(())
}
pub fn verify_receipt(p: &ChangePlan, r: &ChangeReceipt) -> Result<(), ReleaseError> {
    validate_plan(p)?;
    if r.plan_sha256 != p.plan_sha256
        || r.release_id != p.release.release_id
        || [
            &r.pre_state_sha256,
            &r.post_state_sha256,
            &r.health_sha256,
            &r.flag_state_sha256,
            &r.schema_state_sha256,
        ]
        .into_iter()
        .any(|v| !valid_sha(v))
        || r.secret_disclosure_count != 0
        || r.automatic_advance_count != 0
        || r.automatic_retry_count != 0
    {
        return Err(ReleaseError::ReceiptMismatch);
    }
    Ok(())
}
pub fn recovery(r: &ChangeReceipt) -> Recovery {
    match r.outcome {
        ChangeOutcome::Healthy => Recovery::Complete,
        ChangeOutcome::Unknown | ChangeOutcome::Cancelled => Recovery::ReconcileOnly,
        _ => Recovery::FreshCompensationPlanRequired,
    }
}
fn migration_valid(p: &ChangePlan) -> bool {
    p.migration_tool.as_deref().is_some_and(valid_id)
        && valid_opt(&p.migration_checksum_sha256)
        && p.migration_order.is_some_and(|v| v > 0)
        && valid_opt(&p.schema_state_sha256)
        && valid_opt(&p.backup_sha256)
}
fn valid_opt(v: &Option<String>) -> bool {
    v.as_deref().is_some_and(valid_sha)
}
fn valid_id(v: &str) -> bool {
    !v.is_empty()
        && v.len() <= 512
        && v.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':' | b'/'))
}
fn valid_version(v: &str) -> bool {
    v.split('.').count() == 3 && v.bytes().all(|b| b.is_ascii_digit() || b == b'.')
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
    fn plan() -> ChangePlan {
        let h = "a".repeat(64);
        let r = ReleaseManifest {
            release_id: "release-1".into(),
            semantic_version: "1.0.0".into(),
            source_sha256: h.clone(),
            ci_sha256: h.clone(),
            artifact_digest: format!("sha256:{h}"),
            sbom_sha256: h.clone(),
            provenance_sha256: h.clone(),
            signature_sha256: h.clone(),
            environment_sha256: h.clone(),
            policy_sha256: h.clone(),
            migrations_sha256: h.clone(),
            flags_sha256: h.clone(),
            health_sha256: h.clone(),
            rollback_sha256: h.clone(),
            prior_release_sha256: h.clone(),
        };
        ChangePlan {
            plan_sha256: h.clone(),
            release: r,
            kind: ChangeKind::EnvironmentPromotion,
            source_environment_sha256: h.clone(),
            target_environment_sha256: h.clone(),
            approval_class: "non-production".into(),
            deployment_plan_sha256: h.clone(),
            health_window_sha256: h.clone(),
            postcondition_sha256: h.clone(),
            flag_identity_sha256: None,
            flag_prerequisites_sha256: None,
            audience_basis_points: None,
            migration_tool: None,
            migration_checksum_sha256: None,
            migration_order: None,
            schema_state_sha256: None,
            backup_sha256: None,
            production_approval: false,
            migration_approval: false,
            destructive_approval: false,
            autonomous_advance: false,
            later_state_sha256: h,
        }
    }
    #[test]
    fn exact_release_promotion_is_valid() {
        assert_eq!(validate_plan(&plan()), Ok(()));
    }
    #[test]
    fn production_flag_is_separate() {
        let mut p = plan();
        p.kind = ChangeKind::ProductionFlag;
        assert_eq!(validate_plan(&p), Err(ReleaseError::AuthorityMismatch));
        p.production_approval = true;
        p.flag_identity_sha256 = Some("b".repeat(64));
        p.flag_prerequisites_sha256 = Some("c".repeat(64));
        assert_eq!(validate_plan(&p), Ok(()));
    }
    #[test]
    fn progressive_delivery_never_advances_itself() {
        let mut p = plan();
        p.kind = ChangeKind::ProgressiveStep;
        p.audience_basis_points = Some(1000);
        assert_eq!(validate_plan(&p), Ok(()));
        p.autonomous_advance = true;
        assert_eq!(validate_plan(&p), Err(ReleaseError::InvalidPlan));
    }
    #[test]
    fn failed_health_requires_fresh_compensation() {
        let p = plan();
        let h = "d".repeat(64);
        let r = ChangeReceipt {
            plan_sha256: p.plan_sha256.clone(),
            release_id: p.release.release_id.clone(),
            pre_state_sha256: h.clone(),
            post_state_sha256: h.clone(),
            health_sha256: h.clone(),
            flag_state_sha256: h.clone(),
            schema_state_sha256: h,
            secret_disclosure_count: 0,
            automatic_advance_count: 0,
            automatic_retry_count: 0,
            outcome: ChangeOutcome::FailedHealth,
        };
        assert_eq!(verify_receipt(&p, &r), Ok(()));
        assert_eq!(recovery(&r), Recovery::FreshCompensationPlanRequired);
    }
}
