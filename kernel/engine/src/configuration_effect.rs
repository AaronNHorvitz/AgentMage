//! Kernel-mediated configuration filesystem effects.

use std::fmt;
use std::path::PathBuf;

use agentmage_kernel_contracts::{GrantOperation, OperationOutcome, StateChange};

use crate::authority_transaction::{EffectAuthorization, EffectDriver, EffectLaunch, EffectResult};
use crate::configuration::{
    ApplyReceipt, ConfigurationError, ConfigurationManager, LoadedConfiguration,
    MigrationApplyReceipt, MigrationRollbackReceipt,
};

/// One exact configuration mutation proposed for mediated execution.
pub enum ConfigurationEffectRequest {
    /// Durably migrate one legacy version-zero configuration.
    MigrateV0 {
        /// Exact target selected by trusted composition code.
        target: PathBuf,
    },
    /// Restore one exact retained migration preimage.
    RollbackMigration {
        /// Exact current configuration target.
        target: PathBuf,
        /// Exact retained backup.
        backup: PathBuf,
        /// Expected current migrated identity.
        expected_migrated_sha256: String,
    },
    /// Atomically replace one configuration after retaining its preimage.
    Apply {
        /// Exact current configuration target.
        target: PathBuf,
        /// Complete bounded candidate configuration.
        candidate: Vec<u8>,
    },
    /// Restore one exact configuration backup.
    Rollback {
        /// Exact current configuration target.
        target: PathBuf,
        /// Exact retained backup.
        backup: PathBuf,
        /// Expected current configuration identity.
        expected_current_sha256: String,
    },
}

impl fmt::Debug for ConfigurationEffectRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let operation = match self {
            Self::MigrateV0 { .. } => "migrate-v0",
            Self::RollbackMigration { .. } => "rollback-migration",
            Self::Apply { .. } => "apply",
            Self::Rollback { .. } => "rollback",
        };
        formatter
            .debug_struct("ConfigurationEffectRequest")
            .field("operation", &operation)
            .finish_non_exhaustive()
    }
}

/// Typed output retained only after one mediated configuration mutation.
#[derive(Clone, Debug)]
pub enum ConfigurationEffectOutput {
    /// Durable migration evidence.
    Migration(MigrationApplyReceipt),
    /// Durable migration-rollback evidence.
    MigrationRollback(MigrationRollbackReceipt),
    /// Atomic replacement evidence.
    Applied(ApplyReceipt),
    /// Validated restored configuration.
    RolledBack(Box<LoadedConfiguration>),
}

impl ConfigurationEffectOutput {
    fn identity(&self) -> &str {
        match self {
            Self::Migration(receipt) => receipt.migrated_sha256(),
            Self::MigrationRollback(receipt) => receipt.restored_sha256(),
            Self::Applied(receipt) => receipt.applied_sha256(),
            Self::RolledBack(configuration) => configuration.sha256(),
        }
    }
}

/// Configuration driver callable only with a kernel-issued effect authorization.
///
/// Direct mutation methods on `ConfigurationManager` are crate-private:
///
/// ```compile_fail
/// use agentmage_kernel_engine::configuration::ConfigurationManager;
/// use std::path::Path;
/// fn bypass(manager: &ConfigurationManager, path: &Path, candidate: &[u8]) {
///     let _ = manager.apply_with_backup(path, candidate);
/// }
/// ```
#[derive(Debug)]
pub struct ConfigurationEffectDriver {
    manager: ConfigurationManager,
    request: Option<ConfigurationEffectRequest>,
    output: Option<ConfigurationEffectOutput>,
    error: Option<ConfigurationError>,
}

impl ConfigurationEffectDriver {
    /// Creates an inert driver. Construction performs no filesystem mutation.
    #[must_use]
    pub const fn new(manager: ConfigurationManager, request: ConfigurationEffectRequest) -> Self {
        Self {
            manager,
            request: Some(request),
            output: None,
            error: None,
        }
    }

    /// Takes the successful output after the authority transaction terminates.
    pub fn take_output(&mut self) -> Option<ConfigurationEffectOutput> {
        self.output.take()
    }

    /// Takes the bounded configuration error after a failed mediated attempt.
    pub fn take_error(&mut self) -> Option<ConfigurationError> {
        self.error.take()
    }
}

impl EffectDriver for ConfigurationEffectDriver {
    fn execute(&mut self, authorization: EffectAuthorization<'_>) -> EffectLaunch {
        if authorization.operation().operation() != GrantOperation::Administration {
            return EffectLaunch::failed();
        }
        let Some(request) = self.request.take() else {
            return EffectLaunch::failed();
        };
        let result = match request {
            ConfigurationEffectRequest::MigrateV0 { target } => self
                .manager
                .migrate_path_v0(&target)
                .map(ConfigurationEffectOutput::Migration),
            ConfigurationEffectRequest::RollbackMigration {
                target,
                backup,
                expected_migrated_sha256,
            } => self
                .manager
                .rollback_migration(&target, &backup, &expected_migrated_sha256)
                .map(ConfigurationEffectOutput::MigrationRollback),
            ConfigurationEffectRequest::Apply { target, candidate } => self
                .manager
                .apply_with_backup(&target, &candidate)
                .map(ConfigurationEffectOutput::Applied),
            ConfigurationEffectRequest::Rollback {
                target,
                backup,
                expected_current_sha256,
            } => self
                .manager
                .rollback(&target, &backup, &expected_current_sha256)
                .map(Box::new)
                .map(ConfigurationEffectOutput::RolledBack),
        };
        match result {
            Ok(output) => {
                let effect_result = EffectResult::from_redacted_material(
                    OperationOutcome::Succeeded,
                    output.identity().as_bytes(),
                    StateChange::Changed,
                );
                self.output = Some(output);
                EffectLaunch::completed(effect_result)
            }
            Err(error) => {
                let effect_result = EffectResult::from_redacted_material(
                    OperationOutcome::Failed,
                    error.code().as_bytes(),
                    StateChange::Uncertain,
                );
                self.error = Some(error);
                EffectLaunch::completed(effect_result)
            }
        }
    }
}
