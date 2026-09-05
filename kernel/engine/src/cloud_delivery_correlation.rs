//! Cited cross-system cloud context with explicit association and zero inherited authority.
#![allow(missing_docs)]

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssociationClass {
    ProviderNative,
    DeterministicMapping,
    UserConfirmed,
    ModelSuggestion,
    Statistical,
    Uncertain,
    Rejected,
    CausalClaim,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CorrelatedViewKind {
    DeploymentToHealth,
    IncidentToObservation,
    WorkToDeployment,
    ConfigurationToMetric,
    CostToService,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CorrelationSource {
    pub source_kind: String,
    pub provider_id: String,
    pub native_id: String,
    pub evidence_sha256: String,
    pub observed_at: String,
    pub effective_at: String,
    pub window_start: String,
    pub window_end: String,
    pub freshness_seconds: u32,
    pub limitations: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CostContext {
    pub billing_scope_id: String,
    pub service_id: String,
    pub resource_id: Option<String>,
    pub window_start: String,
    pub window_end: String,
    pub currency: String,
    pub granularity: String,
    pub source_id: String,
    pub freshness_seconds: u32,
    pub allocation_assumptions: BTreeSet<String>,
    pub amount_minor_units: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloudDeliveryCorrelation {
    pub correlation_id: String,
    pub view_kind: CorrelatedViewKind,
    pub association_class: AssociationClass,
    pub sources: Vec<CorrelationSource>,
    pub cost: Option<CostContext>,
    pub confidence_basis_points: Option<u16>,
    pub limitations: BTreeSet<String>,
    pub causal_claim: bool,
    pub deployment_authority_count: u32,
    pub infrastructure_authority_count: u32,
    pub incident_authority_count: u32,
    pub communication_authority_count: u32,
    pub finance_authority_count: u32,
    pub cloud_authority_count: u32,
    pub workflow_authority_count: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CorrelationError {
    Invalid,
    Uncited,
    CausalOverclaim,
    Authority,
    UnsupportedAction,
}

fn id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 2048
}

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub fn validate_cost(cost: &CostContext) -> Result<(), CorrelationError> {
    if !id(&cost.billing_scope_id)
        || !id(&cost.service_id)
        || !id(&cost.window_start)
        || !id(&cost.window_end)
        || !id(&cost.granularity)
        || !id(&cost.source_id)
        || cost.currency.len() != 3
        || !cost.currency.bytes().all(|byte| byte.is_ascii_uppercase())
        || cost.allocation_assumptions.is_empty()
    {
        return Err(CorrelationError::Invalid);
    }
    Ok(())
}

pub fn validate_correlation(value: &CloudDeliveryCorrelation) -> Result<(), CorrelationError> {
    if !id(&value.correlation_id) || value.sources.len() < 2 {
        return Err(CorrelationError::Uncited);
    }
    let mut identities = BTreeSet::new();
    for source in &value.sources {
        if !id(&source.source_kind)
            || !id(&source.provider_id)
            || !id(&source.native_id)
            || !digest(&source.evidence_sha256)
            || !id(&source.observed_at)
            || !id(&source.effective_at)
            || !id(&source.window_start)
            || !id(&source.window_end)
            || !identities.insert((source.provider_id.as_str(), source.native_id.as_str()))
        {
            return Err(CorrelationError::Uncited);
        }
    }
    if let Some(cost) = &value.cost {
        validate_cost(cost)?;
    }
    if value.causal_claim || value.association_class == AssociationClass::CausalClaim {
        return Err(CorrelationError::CausalOverclaim);
    }
    if matches!(
        value.association_class,
        AssociationClass::ModelSuggestion
            | AssociationClass::Statistical
            | AssociationClass::Uncertain
            | AssociationClass::Rejected
    ) && value.limitations.is_empty()
    {
        return Err(CorrelationError::Invalid);
    }
    if let Some(confidence) = value.confidence_basis_points
        && confidence > 10_000
    {
        return Err(CorrelationError::Invalid);
    }
    if [
        value.deployment_authority_count,
        value.infrastructure_authority_count,
        value.incident_authority_count,
        value.communication_authority_count,
        value.finance_authority_count,
        value.cloud_authority_count,
        value.workflow_authority_count,
    ]
    .into_iter()
    .any(|count| count != 0)
    {
        return Err(CorrelationError::Authority);
    }
    Ok(())
}

pub fn reject_correlation_action(_action: &str) -> Result<(), CorrelationError> {
    Err(CorrelationError::UnsupportedAction)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(kind: &str, native_id: &str) -> CorrelationSource {
        CorrelationSource {
            source_kind: kind.into(),
            provider_id: "provider".into(),
            native_id: native_id.into(),
            evidence_sha256: "a".repeat(64),
            observed_at: "2026-09-05T00:00:00Z".into(),
            effective_at: "2026-09-05T00:00:00Z".into(),
            window_start: "2026-09-04T00:00:00Z".into(),
            window_end: "2026-09-05T00:00:00Z".into(),
            freshness_seconds: 1,
            limitations: BTreeSet::new(),
        }
    }

    fn correlation() -> CloudDeliveryCorrelation {
        CloudDeliveryCorrelation {
            correlation_id: "correlation-1".into(),
            view_kind: CorrelatedViewKind::CostToService,
            association_class: AssociationClass::Statistical,
            sources: vec![source("cost", "billing/1"), source("service", "service/1")],
            cost: Some(CostContext {
                billing_scope_id: "billing/1".into(),
                service_id: "service/1".into(),
                resource_id: None,
                window_start: "2026-09-04T00:00:00Z".into(),
                window_end: "2026-09-05T00:00:00Z".into(),
                currency: "USD".into(),
                granularity: "daily".into(),
                source_id: "cost-source".into(),
                freshness_seconds: 60,
                allocation_assumptions: BTreeSet::from(["shared-cost".into()]),
                amount_minor_units: 100,
            }),
            confidence_basis_points: Some(5000),
            limitations: BTreeSet::from(["association-not-causation".into()]),
            causal_claim: false,
            deployment_authority_count: 0,
            infrastructure_authority_count: 0,
            incident_authority_count: 0,
            communication_authority_count: 0,
            finance_authority_count: 0,
            cloud_authority_count: 0,
            workflow_authority_count: 0,
        }
    }

    #[test]
    fn cited_statistical_view_is_not_causal() {
        assert_eq!(validate_correlation(&correlation()), Ok(()));
    }

    #[test]
    fn causal_injection_is_rejected() {
        let mut value = correlation();
        value.causal_claim = true;
        assert_eq!(
            validate_correlation(&value),
            Err(CorrelationError::CausalOverclaim)
        );
    }

    #[test]
    fn identity_collision_is_rejected() {
        let mut value = correlation();
        value.sources[1].native_id = value.sources[0].native_id.clone();
        assert_eq!(validate_correlation(&value), Err(CorrelationError::Uncited));
    }

    #[test]
    fn correlation_has_no_action_authority() {
        assert_eq!(
            reject_correlation_action("rollback"),
            Err(CorrelationError::UnsupportedAction)
        );
        let mut value = correlation();
        value.cloud_authority_count = 1;
        assert_eq!(
            validate_correlation(&value),
            Err(CorrelationError::Authority)
        );
    }
}
