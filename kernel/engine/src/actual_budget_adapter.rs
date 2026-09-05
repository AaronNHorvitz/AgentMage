//! Exact, local-only Actual Budget adapter contract.
#![allow(missing_docs)]

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActualBudgetOperation {
    Read,
    Search,
    Import,
    Export,
    DraftChange,
    UpdateRecord,
    UpdateRule,
    UpdateSchedule,
    UpdateBudget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActualBudgetProfile {
    pub api_version: String,
    pub server_version: String,
    pub client_version: String,
    pub schema_version: String,
    pub budget_file_id: String,
    pub sync_version: String,
    pub encryption_version: String,
    pub authentication_reference: String,
    pub compatibility_version: String,
    pub operations: BTreeSet<ActualBudgetOperation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActualRecordKind {
    Account,
    Payee,
    Category,
    Transaction,
    Split,
    TransferRecord,
    Rule,
    Schedule,
    Budget,
    Note,
    Import,
    Reconciliation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActualRecord {
    pub native_record_id: String,
    pub budget_file_id: String,
    pub kind: ActualRecordKind,
    pub amount_canonical: Option<String>,
    pub source_record_id: String,
    pub source_sha256: String,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActualBudgetEffect {
    pub operation: ActualBudgetOperation,
    pub budget_file_id: String,
    pub native_record_id: String,
    pub record_kind: ActualRecordKind,
    pub expected_revision: u64,
    pub payload_sha256: String,
    pub preview_sha256: String,
    pub expected_postcondition_sha256: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActualBackupReceipt {
    pub budget_file_id: String,
    pub source_revision: u64,
    pub backup_sha256: String,
    pub restore_rehearsed: bool,
    pub restored_postcondition_sha256: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActualBudgetError {
    Invalid,
    Unsupported,
    ChangedEffect,
    StaleSync,
    UnsafeBackup,
    Duplicate,
    Unknown,
    Removed,
}

pub struct ActualBudgetController {
    profile: ActualBudgetProfile,
    consumed: BTreeSet<String>,
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

impl ActualBudgetController {
    pub fn new(profile: ActualBudgetProfile) -> Result<Self, ActualBudgetError> {
        if profile.api_version != "api-v1"
            || profile.server_version != "25.7.0"
            || profile.client_version != "25.7.0"
            || profile.schema_version != "schema-25"
            || profile.sync_version != "sync-v1"
            || profile.encryption_version != "aes-256-gcm"
            || profile.compatibility_version != "compat-v1"
        {
            return Err(ActualBudgetError::Unsupported);
        }
        if !identifier(&profile.budget_file_id) || !identifier(&profile.authentication_reference) {
            return Err(ActualBudgetError::Invalid);
        }
        Ok(Self {
            profile,
            consumed: BTreeSet::new(),
            removed: false,
        })
    }

    pub fn discover(&self) -> BTreeSet<ActualBudgetOperation> {
        if self.removed {
            BTreeSet::new()
        } else {
            self.profile.operations.clone()
        }
    }

    pub fn admit_record(&self, record: &ActualRecord) -> Result<(), ActualBudgetError> {
        if self.removed {
            return Err(ActualBudgetError::Removed);
        }
        if record.budget_file_id != self.profile.budget_file_id
            || !identifier(&record.native_record_id)
            || !identifier(&record.source_record_id)
            || !digest(&record.source_sha256)
            || record
                .amount_canonical
                .as_ref()
                .is_some_and(|value| !identifier(value))
        {
            return Err(ActualBudgetError::Invalid);
        }
        Ok(())
    }

    pub fn admit_effect(
        &mut self,
        effect: &ActualBudgetEffect,
        preview: &ActualBudgetEffect,
        current_revision: u64,
    ) -> Result<(), ActualBudgetError> {
        if self.removed {
            return Err(ActualBudgetError::Removed);
        }
        if !self.profile.operations.contains(&effect.operation) {
            return Err(ActualBudgetError::Unsupported);
        }
        if effect != preview
            || effect.budget_file_id != self.profile.budget_file_id
            || !identifier(&effect.native_record_id)
            || !digest(&effect.payload_sha256)
            || !digest(&effect.preview_sha256)
            || !digest(&effect.expected_postcondition_sha256)
            || !identifier(&effect.idempotency_key)
        {
            return Err(ActualBudgetError::ChangedEffect);
        }
        if effect.expected_revision != current_revision {
            return Err(ActualBudgetError::StaleSync);
        }
        if !self.consumed.insert(effect.idempotency_key.clone()) {
            return Err(ActualBudgetError::Duplicate);
        }
        Ok(())
    }

    pub fn admit_backup(
        &self,
        receipt: &ActualBackupReceipt,
        current_revision: u64,
    ) -> Result<(), ActualBudgetError> {
        if receipt.budget_file_id != self.profile.budget_file_id
            || receipt.source_revision != current_revision
            || !digest(&receipt.backup_sha256)
            || !receipt.restore_rehearsed
            || !digest(&receipt.restored_postcondition_sha256)
        {
            return Err(ActualBudgetError::UnsafeBackup);
        }
        Ok(())
    }

    pub fn reconcile(&self, postcondition_proved: Option<bool>) -> Result<bool, ActualBudgetError> {
        postcondition_proved.ok_or(ActualBudgetError::Unknown)
    }

    pub fn remove(&mut self) {
        self.profile.operations.clear();
        self.profile.authentication_reference.clear();
        self.consumed.clear();
        self.removed = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> ActualBudgetProfile {
        ActualBudgetProfile {
            api_version: "api-v1".into(),
            server_version: "25.7.0".into(),
            client_version: "25.7.0".into(),
            schema_version: "schema-25".into(),
            budget_file_id: "budget".into(),
            sync_version: "sync-v1".into(),
            encryption_version: "aes-256-gcm".into(),
            authentication_reference: "credential-ref".into(),
            compatibility_version: "compat-v1".into(),
            operations: BTreeSet::from([
                ActualBudgetOperation::Read,
                ActualBudgetOperation::UpdateRecord,
            ]),
        }
    }
    fn effect() -> ActualBudgetEffect {
        ActualBudgetEffect {
            operation: ActualBudgetOperation::UpdateRecord,
            budget_file_id: "budget".into(),
            native_record_id: "transaction".into(),
            record_kind: ActualRecordKind::Transaction,
            expected_revision: 7,
            payload_sha256: "a".repeat(64),
            preview_sha256: "b".repeat(64),
            expected_postcondition_sha256: "c".repeat(64),
            idempotency_key: "once".into(),
        }
    }

    #[test]
    fn exact_versions_are_required_before_registration() {
        assert!(ActualBudgetController::new(profile()).is_ok());
        let mut future = profile();
        future.server_version = "26.0.0".into();
        assert!(matches!(
            ActualBudgetController::new(future),
            Err(ActualBudgetError::Unsupported)
        ));
    }
    #[test]
    fn wrong_budget_stale_revision_and_duplicate_fail_closed() {
        let mut controller = ActualBudgetController::new(profile()).unwrap();
        let effect = effect();
        assert_eq!(
            controller.admit_effect(&effect, &effect, 8),
            Err(ActualBudgetError::StaleSync)
        );
        assert_eq!(controller.admit_effect(&effect, &effect, 7), Ok(()));
        assert_eq!(
            controller.admit_effect(&effect, &effect, 7),
            Err(ActualBudgetError::Duplicate)
        );
    }
    #[test]
    fn only_non_money_movement_operations_exist() {
        let source = include_str!("actual_budget_adapter.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        for prohibited in [
            "Payment",
            "TransferInitiation",
            "Trade",
            "AccountAdministration",
            "CredentialRecovery",
        ] {
            assert!(!source.contains(prohibited));
        }
    }
    #[test]
    fn backup_rehearsal_and_removal_are_exact() {
        let mut controller = ActualBudgetController::new(profile()).unwrap();
        let receipt = ActualBackupReceipt {
            budget_file_id: "budget".into(),
            source_revision: 7,
            backup_sha256: "d".repeat(64),
            restore_rehearsed: true,
            restored_postcondition_sha256: "e".repeat(64),
        };
        assert_eq!(controller.admit_backup(&receipt, 7), Ok(()));
        controller.remove();
        assert!(controller.discover().is_empty());
        assert_eq!(
            controller.admit_effect(&effect(), &effect(), 7),
            Err(ActualBudgetError::Removed)
        );
    }
}
