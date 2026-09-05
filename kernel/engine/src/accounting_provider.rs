//! Organization-exact, non-payment accounting-provider contracts.
#![allow(missing_docs)]
use std::collections::BTreeSet;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountingProvider {
    QuickBooksOnline,
    Xero,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AccountingCapability {
    Read,
    LocalDraft,
    ProviderDraft,
    ApprovedWrite,
    Correction,
    Void,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountingObjectKind {
    Account,
    Contact,
    Item,
    Invoice,
    Bill,
    Journal,
    TaxCode,
    Attachment,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectOutcome {
    Verified,
    NoOp,
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountingDiscovery {
    pub provider: AccountingProvider,
    pub provider_version: String,
    pub organization_id: String,
    pub role_id: String,
    pub ledger_id: String,
    pub period_id: String,
    pub period_open: bool,
    pub accounting_basis: String,
    pub tax_support: BTreeSet<String>,
    pub currency: String,
    pub precision_scale: u8,
    pub rate_limit: u32,
    pub limitations: BTreeSet<String>,
    pub capabilities: BTreeSet<AccountingCapability>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedAccountingObject {
    pub object_id: String,
    pub kind: AccountingObjectKind,
    pub organization_id: String,
    pub source_revision: String,
    pub source_sha256: String,
    pub currency: String,
    pub canonical_amount: String,
    pub tax_code_id: Option<String>,
    pub immutable_source: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountingWriteProposal {
    pub proposal_id: String,
    pub provider: AccountingProvider,
    pub organization_id: String,
    pub period_id: String,
    pub object_id: String,
    pub kind: AccountingObjectKind,
    pub account_ids: BTreeSet<String>,
    pub contact_ids: BTreeSet<String>,
    pub currency: String,
    pub precision_scale: u8,
    pub tax_code_id: Option<String>,
    pub payload_sha256: String,
    pub expected_revision: String,
    pub idempotency_key: String,
    pub approval_id: String,
    pub approval_fresh: bool,
    pub expected_postcondition_sha256: String,
    pub money_movement: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountingEffectReceipt {
    pub proposal_id: String,
    pub organization_id: String,
    pub idempotency_key: String,
    pub attempt_id: String,
    pub outcome: EffectOutcome,
    pub observed_revision: Option<String>,
    pub postcondition_sha256: Option<String>,
    pub reconciled: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountingRemovalReceipt {
    pub provider: AccountingProvider,
    pub organization_id: String,
    pub credential_count: u32,
    pub cursor_count: u32,
    pub webhook_count: u32,
    pub schedule_count: u32,
    pub worker_count: u32,
    pub socket_count: u32,
    pub cache_count: u32,
    pub write_authority_count: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountingError {
    Invalid,
    Unsupported,
    Stale,
    ClosedPeriod,
    AmbiguousEffect,
    PostconditionMismatch,
}
fn id(v: &str) -> bool {
    !v.is_empty() && v.len() <= 1024
}
fn digest(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
pub fn validate_discovery(v: &AccountingDiscovery) -> Result<(), AccountingError> {
    if !id(&v.provider_version)
        || !id(&v.organization_id)
        || !id(&v.role_id)
        || !id(&v.ledger_id)
        || !id(&v.period_id)
        || !id(&v.accounting_basis)
        || v.tax_support.is_empty()
        || !id(&v.currency)
        || v.precision_scale > 18
        || v.rate_limit == 0
        || v.limitations.is_empty()
        || v.capabilities.is_empty()
    {
        return Err(AccountingError::Invalid);
    }
    Ok(())
}
pub fn validate_read(
    v: &NormalizedAccountingObject,
    d: &AccountingDiscovery,
) -> Result<(), AccountingError> {
    if !id(&v.object_id)
        || v.organization_id != d.organization_id
        || !id(&v.source_revision)
        || !digest(&v.source_sha256)
        || v.currency != d.currency
        || !id(&v.canonical_amount)
        || !v.immutable_source
    {
        return Err(AccountingError::Invalid);
    }
    Ok(())
}
pub fn admit_write(
    v: &AccountingWriteProposal,
    d: &AccountingDiscovery,
) -> Result<(), AccountingError> {
    if !d.period_open {
        return Err(AccountingError::ClosedPeriod);
    }
    if v.provider != d.provider
        || v.organization_id != d.organization_id
        || v.period_id != d.period_id
        || !id(&v.proposal_id)
        || !id(&v.object_id)
        || v.account_ids.is_empty()
        || v.currency != d.currency
        || v.precision_scale != d.precision_scale
        || !digest(&v.payload_sha256)
        || !id(&v.expected_revision)
        || !id(&v.idempotency_key)
        || !id(&v.approval_id)
        || !v.approval_fresh
        || !digest(&v.expected_postcondition_sha256)
        || v.money_movement
    {
        return Err(AccountingError::Stale);
    }
    Ok(())
}
pub fn reconcile_effect(
    v: &AccountingEffectReceipt,
    p: &AccountingWriteProposal,
) -> Result<(), AccountingError> {
    if v.proposal_id != p.proposal_id
        || v.organization_id != p.organization_id
        || v.idempotency_key != p.idempotency_key
        || !id(&v.attempt_id)
    {
        return Err(AccountingError::Invalid);
    }
    if v.outcome == EffectOutcome::Unknown || !v.reconciled {
        return Err(AccountingError::AmbiguousEffect);
    }
    if v.outcome == EffectOutcome::Verified
        && (v.postcondition_sha256.as_deref() != Some(p.expected_postcondition_sha256.as_str())
            || v.observed_revision.is_none())
    {
        return Err(AccountingError::PostconditionMismatch);
    }
    Ok(())
}
pub fn validate_removal(v: &AccountingRemovalReceipt) -> Result<(), AccountingError> {
    if !id(&v.organization_id)
        || [
            v.credential_count,
            v.cursor_count,
            v.webhook_count,
            v.schedule_count,
            v.worker_count,
            v.socket_count,
            v.cache_count,
            v.write_authority_count,
        ]
        .into_iter()
        .any(|n| n != 0)
    {
        return Err(AccountingError::Invalid);
    }
    Ok(())
}
pub fn reject_prohibited_operation(_name: &str) -> Result<(), AccountingError> {
    Err(AccountingError::Unsupported)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn d() -> AccountingDiscovery {
        AccountingDiscovery {
            provider: AccountingProvider::Xero,
            provider_version: "v2".into(),
            organization_id: "org".into(),
            role_id: "accountant".into(),
            ledger_id: "ledger".into(),
            period_id: "2026-09".into(),
            period_open: true,
            accounting_basis: "accrual".into(),
            tax_support: BTreeSet::from(["tax".into()]),
            currency: "USD".into(),
            precision_scale: 2,
            rate_limit: 60,
            limitations: BTreeSet::from(["no-payments".into()]),
            capabilities: BTreeSet::from([
                AccountingCapability::Read,
                AccountingCapability::ApprovedWrite,
            ]),
        }
    }
    fn p() -> AccountingWriteProposal {
        AccountingWriteProposal {
            proposal_id: "proposal".into(),
            provider: AccountingProvider::Xero,
            organization_id: "org".into(),
            period_id: "2026-09".into(),
            object_id: "invoice".into(),
            kind: AccountingObjectKind::Invoice,
            account_ids: BTreeSet::from(["receivable".into()]),
            contact_ids: BTreeSet::from(["contact".into()]),
            currency: "USD".into(),
            precision_scale: 2,
            tax_code_id: Some("tax".into()),
            payload_sha256: "a".repeat(64),
            expected_revision: "r1".into(),
            idempotency_key: "key".into(),
            approval_id: "approval".into(),
            approval_fresh: true,
            expected_postcondition_sha256: "b".repeat(64),
            money_movement: false,
        }
    }
    #[test]
    fn exact_write() {
        let d = d();
        assert_eq!(validate_discovery(&d), Ok(()));
        assert_eq!(admit_write(&p(), &d), Ok(()));
        let mut w = p();
        w.organization_id = "other".into();
        assert_eq!(admit_write(&w, &d), Err(AccountingError::Stale));
    }
    #[test]
    fn closed_and_money_fail() {
        let mut closed = d();
        closed.period_open = false;
        assert_eq!(
            admit_write(&p(), &closed),
            Err(AccountingError::ClosedPeriod)
        );
        let discovery = d();
        let mut p = p();
        p.money_movement = true;
        assert_eq!(admit_write(&p, &discovery), Err(AccountingError::Stale));
    }
    #[test]
    fn unknown_never_retries() {
        let p = p();
        let r = AccountingEffectReceipt {
            proposal_id: p.proposal_id.clone(),
            organization_id: p.organization_id.clone(),
            idempotency_key: p.idempotency_key.clone(),
            attempt_id: "attempt".into(),
            outcome: EffectOutcome::Unknown,
            observed_revision: None,
            postcondition_sha256: None,
            reconciled: false,
        };
        assert_eq!(
            reconcile_effect(&r, &p),
            Err(AccountingError::AmbiguousEffect)
        );
    }
    #[test]
    fn removal_and_absence() {
        let r = AccountingRemovalReceipt {
            provider: AccountingProvider::QuickBooksOnline,
            organization_id: "org".into(),
            credential_count: 0,
            cursor_count: 0,
            webhook_count: 0,
            schedule_count: 0,
            worker_count: 0,
            socket_count: 0,
            cache_count: 0,
            write_authority_count: 0,
        };
        assert_eq!(validate_removal(&r), Ok(()));
        assert_eq!(
            reject_prohibited_operation("payment"),
            Err(AccountingError::Unsupported)
        );
    }
}
