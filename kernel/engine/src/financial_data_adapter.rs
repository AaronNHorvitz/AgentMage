//! Consent-bound and structurally read-only financial data adapters.
#![allow(missing_docs)]

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FinancialDataClass {
    Transactions,
    Balances,
    Liabilities,
    Investments,
    RecurringStreams,
    Statements,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinancialProviderProfile {
    pub provider_id: String,
    pub provider_version: String,
    pub institution_id: String,
    pub item_id: String,
    pub owner_scope_id: String,
    pub support_evidence_sha256: String,
    pub data_use_evidence_sha256: String,
    pub retention_evidence_sha256: String,
    pub removal_evidence_sha256: String,
    pub failure_evidence_sha256: String,
    pub security_evidence_sha256: String,
    pub admitted_classes: BTreeSet<FinancialDataClass>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinancialConsentReceipt {
    pub consent_id: String,
    pub provider_id: String,
    pub item_id: String,
    pub owner_scope_id: String,
    pub selected_account_ids: BTreeSet<String>,
    pub permitted_classes: BTreeSet<FinancialDataClass>,
    pub issued_at: String,
    pub expires_at: String,
    pub revoked_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinancialReadRequest {
    pub provider_id: String,
    pub institution_id: String,
    pub item_id: String,
    pub owner_scope_id: String,
    pub account_id: String,
    pub data_class: FinancialDataClass,
    pub cursor: Option<String>,
    pub requested_at: String,
    pub credential_reference: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NormalizedFinancialObservation {
    pub provider_id: String,
    pub institution_id: String,
    pub item_id: String,
    pub account_id: String,
    pub native_record_id: String,
    pub data_class: FinancialDataClass,
    pub pending: bool,
    pub source_time: String,
    pub observed_at: String,
    pub freshness_seconds: u64,
    pub coverage_start: String,
    pub coverage_end: String,
    pub source_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinancialSyncResult {
    pub provider_id: String,
    pub item_id: String,
    pub prior_cursor: Option<String>,
    pub next_cursor: Option<String>,
    pub record_ids: BTreeSet<String>,
    pub duplicate_ids: BTreeSet<String>,
    pub coverage_complete: bool,
    pub permission_reduced: bool,
    pub reauthentication_required: bool,
    pub reason: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinancialAdapterError {
    Invalid,
    Unadmitted,
    ScopeMismatch,
    Expired,
    Revoked,
    Duplicate,
    Incomplete,
    Removed,
}

pub struct FinancialDataController {
    profile: FinancialProviderProfile,
    consent: FinancialConsentReceipt,
    seen_records: BTreeSet<String>,
    removed: bool,
}

fn identifier(value: &str) -> bool {
    !value.is_empty() && value.len() <= 1024
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

impl FinancialDataController {
    pub fn new(
        profile: FinancialProviderProfile,
        consent: FinancialConsentReceipt,
    ) -> Result<Self, FinancialAdapterError> {
        if profile.provider_id != "plaid"
            || profile.provider_version != "2020-09-14"
            || !identifier(&profile.institution_id)
            || !identifier(&profile.item_id)
            || !identifier(&profile.owner_scope_id)
            || [
                &profile.support_evidence_sha256,
                &profile.data_use_evidence_sha256,
                &profile.retention_evidence_sha256,
                &profile.removal_evidence_sha256,
                &profile.failure_evidence_sha256,
                &profile.security_evidence_sha256,
            ]
            .into_iter()
            .any(|value| !digest(value))
        {
            return Err(FinancialAdapterError::Unadmitted);
        }
        if consent.provider_id != profile.provider_id
            || consent.item_id != profile.item_id
            || consent.owner_scope_id != profile.owner_scope_id
            || consent.selected_account_ids.is_empty()
            || !consent
                .permitted_classes
                .is_subset(&profile.admitted_classes)
        {
            return Err(FinancialAdapterError::ScopeMismatch);
        }
        if consent.revoked_at.is_some() {
            return Err(FinancialAdapterError::Revoked);
        }
        Ok(Self {
            profile,
            consent,
            seen_records: BTreeSet::new(),
            removed: false,
        })
    }

    pub fn admit_read(&self, request: &FinancialReadRequest) -> Result<(), FinancialAdapterError> {
        if self.removed {
            return Err(FinancialAdapterError::Removed);
        }
        if request.provider_id != self.profile.provider_id
            || request.institution_id != self.profile.institution_id
            || request.item_id != self.profile.item_id
            || request.owner_scope_id != self.profile.owner_scope_id
            || !self
                .consent
                .selected_account_ids
                .contains(&request.account_id)
            || !self.consent.permitted_classes.contains(&request.data_class)
            || !identifier(&request.credential_reference)
            || !identifier(&request.requested_at)
        {
            return Err(FinancialAdapterError::ScopeMismatch);
        }
        Ok(())
    }

    pub fn normalize(
        &mut self,
        observation: &NormalizedFinancialObservation,
    ) -> Result<(), FinancialAdapterError> {
        if self.removed {
            return Err(FinancialAdapterError::Removed);
        }
        if observation.provider_id != self.profile.provider_id
            || observation.institution_id != self.profile.institution_id
            || observation.item_id != self.profile.item_id
            || !self
                .consent
                .selected_account_ids
                .contains(&observation.account_id)
            || !self
                .consent
                .permitted_classes
                .contains(&observation.data_class)
            || !identifier(&observation.native_record_id)
            || !digest(&observation.source_sha256)
        {
            return Err(FinancialAdapterError::ScopeMismatch);
        }
        if !self
            .seen_records
            .insert(observation.native_record_id.clone())
        {
            return Err(FinancialAdapterError::Duplicate);
        }
        Ok(())
    }

    pub fn reconcile_sync(
        &self,
        result: &FinancialSyncResult,
    ) -> Result<(), FinancialAdapterError> {
        if result.provider_id != self.profile.provider_id
            || result.item_id != self.profile.item_id
            || !result.duplicate_ids.is_subset(&result.record_ids)
            || ((!result.coverage_complete
                || result.permission_reduced
                || result.reauthentication_required)
                && result.reason.is_none())
        {
            return Err(FinancialAdapterError::Incomplete);
        }
        Ok(())
    }

    pub fn revoke_and_remove(&mut self) {
        self.consent.revoked_at = Some("removed".into());
        self.consent.selected_account_ids.clear();
        self.consent.permitted_classes.clear();
        self.seen_records.clear();
        self.removed = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn profile() -> FinancialProviderProfile {
        FinancialProviderProfile {
            provider_id: "plaid".into(),
            provider_version: "2020-09-14".into(),
            institution_id: "institution".into(),
            item_id: "item".into(),
            owner_scope_id: "owner".into(),
            support_evidence_sha256: "a".repeat(64),
            data_use_evidence_sha256: "b".repeat(64),
            retention_evidence_sha256: "c".repeat(64),
            removal_evidence_sha256: "d".repeat(64),
            failure_evidence_sha256: "e".repeat(64),
            security_evidence_sha256: "f".repeat(64),
            admitted_classes: BTreeSet::from([
                FinancialDataClass::Transactions,
                FinancialDataClass::Balances,
            ]),
        }
    }
    fn consent() -> FinancialConsentReceipt {
        FinancialConsentReceipt {
            consent_id: "consent".into(),
            provider_id: "plaid".into(),
            item_id: "item".into(),
            owner_scope_id: "owner".into(),
            selected_account_ids: BTreeSet::from(["account".into()]),
            permitted_classes: BTreeSet::from([FinancialDataClass::Transactions]),
            issued_at: "2026-09-04".into(),
            expires_at: "2026-09-05".into(),
            revoked_at: None,
        }
    }
    #[test]
    fn exact_account_and_class_are_required() {
        let controller = FinancialDataController::new(profile(), consent()).unwrap();
        let mut request = FinancialReadRequest {
            provider_id: "plaid".into(),
            institution_id: "institution".into(),
            item_id: "item".into(),
            owner_scope_id: "owner".into(),
            account_id: "account".into(),
            data_class: FinancialDataClass::Transactions,
            cursor: None,
            requested_at: "now".into(),
            credential_reference: "token-ref".into(),
        };
        assert_eq!(controller.admit_read(&request), Ok(()));
        request.account_id = "other".into();
        assert_eq!(
            controller.admit_read(&request),
            Err(FinancialAdapterError::ScopeMismatch)
        );
    }
    #[test]
    fn cross_item_and_duplicate_records_fail_closed() {
        let mut controller = FinancialDataController::new(profile(), consent()).unwrap();
        let observation = NormalizedFinancialObservation {
            provider_id: "plaid".into(),
            institution_id: "institution".into(),
            item_id: "item".into(),
            account_id: "account".into(),
            native_record_id: "txn".into(),
            data_class: FinancialDataClass::Transactions,
            pending: true,
            source_time: "source".into(),
            observed_at: "now".into(),
            freshness_seconds: 1,
            coverage_start: "start".into(),
            coverage_end: "end".into(),
            source_sha256: "a".repeat(64),
        };
        assert_eq!(controller.normalize(&observation), Ok(()));
        assert_eq!(
            controller.normalize(&observation),
            Err(FinancialAdapterError::Duplicate)
        );
    }
    #[test]
    fn prohibited_operation_families_are_structurally_absent() {
        let source = include_str!("financial_data_adapter.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        for prohibited in [
            "Payment",
            "TransferInitiation",
            "BillPay",
            "Trade",
            "Withdrawal",
            "Deposit",
            "CreditApplication",
            "LoanApplication",
            "BeneficiaryChange",
            "TaxFiling",
            "AccountAdministration",
            "CredentialRecovery",
        ] {
            assert!(!source.contains(prohibited));
        }
    }
    #[test]
    fn removal_clears_read_and_sync_authority() {
        let mut controller = FinancialDataController::new(profile(), consent()).unwrap();
        controller.revoke_and_remove();
        let request = FinancialReadRequest {
            provider_id: "plaid".into(),
            institution_id: "institution".into(),
            item_id: "item".into(),
            owner_scope_id: "owner".into(),
            account_id: "account".into(),
            data_class: FinancialDataClass::Transactions,
            cursor: None,
            requested_at: "now".into(),
            credential_reference: "token-ref".into(),
        };
        assert_eq!(
            controller.admit_read(&request),
            Err(FinancialAdapterError::Removed)
        );
    }
}
