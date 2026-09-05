//! Finance-pack privacy, data-flow, absence, and removal contracts.
#![allow(missing_docs)]
use std::collections::BTreeSet;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinancialClassification {
    AccountIdentifier,
    Transaction,
    Tax,
    CredentialMetadata,
    DerivedAnalysis,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FinancialSurface {
    OperationalStore,
    Memory,
    ModelContext,
    Log,
    Diagnostic,
    Communication,
    Document,
    Task,
    Delivery,
    Cloud,
    Export,
    Backup,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProhibitedFinancialFamily {
    Transfer,
    Payment,
    BillPay,
    Trade,
    Order,
    Withdrawal,
    Deposit,
    Credit,
    Loan,
    TaxFiling,
    Beneficiary,
    AccountAdministration,
    CredentialRecovery,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinancialDataPolicy {
    pub policy_id: String,
    pub classification: FinancialClassification,
    pub allowed_stores: BTreeSet<FinancialSurface>,
    pub encrypted_at_rest: bool,
    pub memory_lifetime_seconds: u32,
    pub model_context_eligible: bool,
    pub log_redacted: bool,
    pub diagnostic_disclosure: bool,
    pub export_requires_approval: bool,
    pub backup_disclosed: bool,
    pub retention_days: u32,
    pub deletion_required: bool,
    pub allowed_cross_pack_surfaces: BTreeSet<FinancialSurface>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinancialFlowReceipt {
    pub flow_id: String,
    pub policy_id: String,
    pub source_surface: FinancialSurface,
    pub destination_surface: FinancialSurface,
    pub value_sha256: String,
    pub classification: FinancialClassification,
    pub approval_id: Option<String>,
    pub redacted: bool,
    pub disclosed: bool,
    pub external_effect: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinancialCanaryObservation {
    pub canary_sha256: String,
    pub probed_surfaces: BTreeSet<FinancialSurface>,
    pub unauthorized_observation_count: u32,
    pub raw_value_retained: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinanceRemovalReceipt {
    pub state: String,
    pub process_count: u32,
    pub socket_count: u32,
    pub credential_count: u32,
    pub cache_count: u32,
    pub cursor_count: u32,
    pub schedule_count: u32,
    pub index_count: u32,
    pub data_count: u32,
    pub network_count: u32,
    pub authority_count: u32,
    pub declared_retention_count: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinancePrivacyError {
    Invalid,
    PolicyDenied,
    Disclosure,
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
pub fn validate_policy(v: &FinancialDataPolicy) -> Result<(), FinancePrivacyError> {
    if !id(&v.policy_id)
        || v.allowed_stores.is_empty()
        || !v.encrypted_at_rest
        || v.memory_lifetime_seconds == 0
        || !v.log_redacted
        || !v.export_requires_approval
        || v.retention_days == 0
        || !v.deletion_required
    {
        return Err(FinancePrivacyError::Invalid);
    }
    Ok(())
}
pub fn admit_flow(
    v: &FinancialFlowReceipt,
    p: &FinancialDataPolicy,
) -> Result<(), FinancePrivacyError> {
    if !id(&v.flow_id)
        || v.policy_id != p.policy_id
        || !digest(&v.value_sha256)
        || v.classification != p.classification
        || v.external_effect
    {
        return Err(FinancePrivacyError::Invalid);
    }
    if !p.allowed_stores.contains(&v.destination_surface)
        && !p
            .allowed_cross_pack_surfaces
            .contains(&v.destination_surface)
    {
        return Err(FinancePrivacyError::PolicyDenied);
    }
    if matches!(v.destination_surface, FinancialSurface::ModelContext) && !p.model_context_eligible
    {
        return Err(FinancePrivacyError::PolicyDenied);
    }
    if matches!(
        v.destination_surface,
        FinancialSurface::Export | FinancialSurface::Communication | FinancialSurface::Delivery
    ) && (v.approval_id.as_deref().is_none_or(|x| !id(x)) || !v.redacted || !v.disclosed)
    {
        return Err(FinancePrivacyError::Disclosure);
    }
    Ok(())
}
pub fn validate_canary(v: &FinancialCanaryObservation) -> Result<(), FinancePrivacyError> {
    if !digest(&v.canary_sha256)
        || v.probed_surfaces.is_empty()
        || v.unauthorized_observation_count != 0
        || v.raw_value_retained
    {
        return Err(FinancePrivacyError::Disclosure);
    }
    Ok(())
}
pub fn validate_removal(v: &FinanceRemovalReceipt) -> Result<(), FinancePrivacyError> {
    if !id(&v.state)
        || [
            v.process_count,
            v.socket_count,
            v.credential_count,
            v.cache_count,
            v.cursor_count,
            v.schedule_count,
            v.index_count,
            v.data_count,
            v.network_count,
            v.authority_count,
        ]
        .into_iter()
        .any(|n| n != 0)
    {
        return Err(FinancePrivacyError::Invalid);
    }
    Ok(())
}
pub fn reject_prohibited_family(_v: ProhibitedFinancialFamily) -> Result<(), FinancePrivacyError> {
    Err(FinancePrivacyError::UnsupportedEffect)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn policy() -> FinancialDataPolicy {
        FinancialDataPolicy {
            policy_id: "policy".into(),
            classification: FinancialClassification::Transaction,
            allowed_stores: BTreeSet::from([
                FinancialSurface::OperationalStore,
                FinancialSurface::Memory,
            ]),
            encrypted_at_rest: true,
            memory_lifetime_seconds: 60,
            model_context_eligible: false,
            log_redacted: true,
            diagnostic_disclosure: false,
            export_requires_approval: true,
            backup_disclosed: true,
            retention_days: 30,
            deletion_required: true,
            allowed_cross_pack_surfaces: BTreeSet::from([FinancialSurface::Export]),
        }
    }
    #[test]
    fn policy_and_flows_fail_closed() {
        let p = policy();
        assert_eq!(validate_policy(&p), Ok(()));
        let f = FinancialFlowReceipt {
            flow_id: "flow".into(),
            policy_id: "policy".into(),
            source_surface: FinancialSurface::OperationalStore,
            destination_surface: FinancialSurface::Export,
            value_sha256: "a".repeat(64),
            classification: FinancialClassification::Transaction,
            approval_id: Some("approval".into()),
            redacted: true,
            disclosed: true,
            external_effect: false,
        };
        assert_eq!(admit_flow(&f, &p), Ok(()));
        let mut m = f;
        m.destination_surface = FinancialSurface::ModelContext;
        assert_eq!(admit_flow(&m, &p), Err(FinancePrivacyError::PolicyDenied));
    }
    #[test]
    fn canaries_retain_only_digest() {
        let v = FinancialCanaryObservation {
            canary_sha256: "a".repeat(64),
            probed_surfaces: BTreeSet::from([FinancialSurface::Log]),
            unauthorized_observation_count: 0,
            raw_value_retained: false,
        };
        assert_eq!(validate_canary(&v), Ok(()));
    }
    #[test]
    fn removal_requires_zero_inventory() {
        let v = FinanceRemovalReceipt {
            state: "crashed".into(),
            process_count: 0,
            socket_count: 0,
            credential_count: 0,
            cache_count: 0,
            cursor_count: 0,
            schedule_count: 0,
            index_count: 0,
            data_count: 0,
            network_count: 0,
            authority_count: 0,
            declared_retention_count: 0,
        };
        assert_eq!(validate_removal(&v), Ok(()));
    }
    #[test]
    fn every_money_family_is_rejected() {
        for v in [
            ProhibitedFinancialFamily::Transfer,
            ProhibitedFinancialFamily::Payment,
            ProhibitedFinancialFamily::BillPay,
            ProhibitedFinancialFamily::Trade,
            ProhibitedFinancialFamily::Order,
            ProhibitedFinancialFamily::Withdrawal,
            ProhibitedFinancialFamily::Deposit,
            ProhibitedFinancialFamily::Credit,
            ProhibitedFinancialFamily::Loan,
            ProhibitedFinancialFamily::TaxFiling,
            ProhibitedFinancialFamily::Beneficiary,
            ProhibitedFinancialFamily::AccountAdministration,
            ProhibitedFinancialFamily::CredentialRecovery,
        ] {
            assert_eq!(
                reject_prohibited_family(v),
                Err(FinancePrivacyError::UnsupportedEffect)
            );
        }
    }
}
