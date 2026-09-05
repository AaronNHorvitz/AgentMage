//! Provider-neutral, strictly read-only cloud observation contracts.
#![allow(missing_docs)]
use std::collections::BTreeSet;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CloudReadKind {
    Inventory,
    Configuration,
    Tags,
    Health,
    Metrics,
    Logs,
    AuditReference,
    SecurityObservation,
    DeploymentIdentity,
    CostSummary,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloudResultState {
    Complete,
    Partial,
    Stale,
    Truncated,
    PermissionLimited,
    Cached,
    Throttled,
    QuotaLimited,
    TimedOut,
    Revoked,
    Recovered,
    Removed,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloudObserverScope {
    pub provider_id: String,
    pub organization_id: String,
    pub tenant_id: Option<String>,
    pub account_id: String,
    pub subscription_or_project_id: String,
    pub regions: BTreeSet<String>,
    pub services: BTreeSet<String>,
    pub resource_prefixes: BTreeSet<String>,
    pub allowed_reads: BTreeSet<CloudReadKind>,
    pub credential_reference: String,
    pub support_profile_id: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloudQueryBudget {
    pub query_id: String,
    pub time_start: String,
    pub time_end: String,
    pub fields: BTreeSet<String>,
    pub max_rows: u32,
    pub max_bytes: u64,
    pub max_requests: u32,
    pub rate_per_minute: u32,
    pub cancellation_millis: u64,
    pub cache_max_age_seconds: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloudObservation {
    pub result_id: String,
    pub query_id: String,
    pub provider_id: String,
    pub account_id: String,
    pub region: String,
    pub service: String,
    pub resource_id: String,
    pub kind: CloudReadKind,
    pub content_sha256: String,
    pub state: CloudResultState,
    pub freshness_seconds: u32,
    pub limitation_codes: BTreeSet<String>,
    pub untrusted_content: bool,
    pub complete_claim: bool,
    pub authority: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloudRemovalReceipt {
    pub provider_id: String,
    pub state: CloudResultState,
    pub credential_count: u32,
    pub cache_count: u32,
    pub cursor_count: u32,
    pub worker_count: u32,
    pub socket_count: u32,
    pub schedule_count: u32,
    pub network_count: u32,
    pub cloud_authority_count: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloudObserverError {
    Invalid,
    OutOfScope,
    Limit,
    Untrusted,
    UnsupportedEffect,
}
fn id(v: &str) -> bool {
    !v.is_empty() && v.len() <= 1024
}
fn digest(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
pub fn validate_scope(v: &CloudObserverScope) -> Result<(), CloudObserverError> {
    if !id(&v.provider_id)
        || !id(&v.organization_id)
        || !id(&v.account_id)
        || !id(&v.subscription_or_project_id)
        || v.regions.is_empty()
        || v.services.is_empty()
        || v.resource_prefixes.is_empty()
        || v.allowed_reads.is_empty()
        || !id(&v.credential_reference)
        || !id(&v.support_profile_id)
    {
        return Err(CloudObserverError::Invalid);
    }
    Ok(())
}
pub fn validate_budget(v: &CloudQueryBudget) -> Result<(), CloudObserverError> {
    if !id(&v.query_id)
        || !id(&v.time_start)
        || !id(&v.time_end)
        || v.fields.is_empty()
        || v.max_rows == 0
        || v.max_bytes == 0
        || v.max_requests == 0
        || v.rate_per_minute == 0
        || v.cancellation_millis == 0
    {
        return Err(CloudObserverError::Limit);
    }
    Ok(())
}
pub fn validate_observation(
    v: &CloudObservation,
    s: &CloudObserverScope,
) -> Result<(), CloudObserverError> {
    if !id(&v.result_id)
        || !id(&v.query_id)
        || v.provider_id != s.provider_id
        || v.account_id != s.account_id
        || !s.regions.contains(&v.region)
        || !s.services.contains(&v.service)
        || !s
            .resource_prefixes
            .iter()
            .any(|p| v.resource_id.starts_with(p))
        || !s.allowed_reads.contains(&v.kind)
        || !digest(&v.content_sha256)
        || !v.untrusted_content
        || v.authority
    {
        return Err(CloudObserverError::OutOfScope);
    }
    if v.state != CloudResultState::Complete && (v.limitation_codes.is_empty() || v.complete_claim)
    {
        return Err(CloudObserverError::Invalid);
    }
    Ok(())
}
pub fn treat_content_as_evidence(_v: &str) -> Result<(), CloudObserverError> {
    Err(CloudObserverError::Untrusted)
}
pub fn reject_cloud_effect(_v: &str) -> Result<(), CloudObserverError> {
    Err(CloudObserverError::UnsupportedEffect)
}
pub fn validate_removal(v: &CloudRemovalReceipt) -> Result<(), CloudObserverError> {
    if !id(&v.provider_id)
        || ![CloudResultState::Revoked, CloudResultState::Removed].contains(&v.state)
        || [
            v.credential_count,
            v.cache_count,
            v.cursor_count,
            v.worker_count,
            v.socket_count,
            v.schedule_count,
            v.network_count,
            v.cloud_authority_count,
        ]
        .into_iter()
        .any(|n| n != 0)
    {
        return Err(CloudObserverError::Invalid);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn scope() -> CloudObserverScope {
        CloudObserverScope {
            provider_id: "fake".into(),
            organization_id: "org".into(),
            tenant_id: None,
            account_id: "account".into(),
            subscription_or_project_id: "project".into(),
            regions: BTreeSet::from(["us".into()]),
            services: BTreeSet::from(["compute".into()]),
            resource_prefixes: BTreeSet::from(["resource/".into()]),
            allowed_reads: BTreeSet::from([CloudReadKind::Inventory]),
            credential_reference: "credential-ref".into(),
            support_profile_id: "profile".into(),
        }
    }
    #[test]
    fn exact_scope_and_limits() {
        assert_eq!(validate_scope(&scope()), Ok(()));
        let b = CloudQueryBudget {
            query_id: "q".into(),
            time_start: "a".into(),
            time_end: "b".into(),
            fields: BTreeSet::from(["id".into()]),
            max_rows: 10,
            max_bytes: 100,
            max_requests: 2,
            rate_per_minute: 2,
            cancellation_millis: 1000,
            cache_max_age_seconds: 60,
        };
        assert_eq!(validate_budget(&b), Ok(()));
    }
    #[test]
    fn partial_never_looks_complete() {
        let s = scope();
        let mut v = CloudObservation {
            result_id: "r".into(),
            query_id: "q".into(),
            provider_id: "fake".into(),
            account_id: "account".into(),
            region: "us".into(),
            service: "compute".into(),
            resource_id: "resource/1".into(),
            kind: CloudReadKind::Inventory,
            content_sha256: "a".repeat(64),
            state: CloudResultState::Partial,
            freshness_seconds: 1,
            limitation_codes: BTreeSet::from(["partial".into()]),
            untrusted_content: true,
            complete_claim: false,
            authority: false,
        };
        assert_eq!(validate_observation(&v, &s), Ok(()));
        v.complete_claim = true;
        assert_eq!(
            validate_observation(&v, &s),
            Err(CloudObserverError::Invalid)
        );
    }
    #[test]
    fn content_and_effects_are_inert() {
        assert_eq!(
            treat_content_as_evidence("inject"),
            Err(CloudObserverError::Untrusted)
        );
        assert_eq!(
            reject_cloud_effect("delete"),
            Err(CloudObserverError::UnsupportedEffect)
        );
    }
    #[test]
    fn removal_has_zero_authority() {
        let v = CloudRemovalReceipt {
            provider_id: "fake".into(),
            state: CloudResultState::Removed,
            credential_count: 0,
            cache_count: 0,
            cursor_count: 0,
            worker_count: 0,
            socket_count: 0,
            schedule_count: 0,
            network_count: 0,
            cloud_authority_count: 0,
        };
        assert_eq!(validate_removal(&v), Ok(()));
    }
}
