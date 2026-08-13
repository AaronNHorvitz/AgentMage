//! Descriptor-relative Linux configuration storage and mediated mutation.

use std::fmt;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use agentmage_kernel_contracts::{GrantOperation, OperationOutcome, PlatformAdapter, StateChange};
use agentmage_kernel_engine::authority_transaction::{
    EffectAuthorization, EffectDriver, EffectLaunch, EffectResult,
};
use agentmage_kernel_engine::configuration::{
    ApplyReceipt, ConfigurationError, ConfigurationManager, LoadedConfiguration,
    MigrationApplyReceipt, MigrationRollbackReceipt,
};
use agentmage_kernel_engine::platform_startup::VerifiedPlatformAdapter;
use rustix::fd::OwnedFd;
use rustix::fs::{
    AtFlags, FileType, Mode, OFlags, RenameFlags, fstat, fsync, openat, renameat_with, unlinkat,
};
use rustix::io::Errno;
use rustix::process::getuid;
use sha2::{Digest, Sha256};

use crate::{LinuxPlatformAdapter, LinuxStrictLocalRoot, LinuxStrictLocalRootInspector};

const TARGET_NAME: &str = "agentmage.json";
const MAX_CONFIGURATION_BYTES: u64 = 1024 * 1024;

/// Stable Linux configuration-storage failure class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxConfigurationErrorKind {
    /// The private descriptor-held root failed admission or revalidation.
    UnsafeRoot,
    /// The fixed configuration target could not be opened.
    TargetUnavailable,
    /// A target, backup, or candidate had unsafe native metadata.
    UnsafeObject,
    /// A bounded configuration read or write exceeded its limit.
    ResourceLimitExceeded,
    /// The expected target or backup identity changed.
    Conflict,
    /// A write, exchange, or synchronization operation failed.
    DurabilityFailure,
    /// Kernel configuration validation rejected the bytes.
    InvalidConfiguration,
}

/// Content-free Linux configuration error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxConfigurationError {
    kind: LinuxConfigurationErrorKind,
    configuration_code: Option<&'static str>,
}

impl LinuxConfigurationError {
    /// Returns the stable native failure class.
    #[must_use]
    pub const fn kind(&self) -> LinuxConfigurationErrorKind {
        self.kind
    }

    /// Returns the bounded kernel-validation code when applicable.
    #[must_use]
    pub const fn configuration_code(&self) -> Option<&'static str> {
        self.configuration_code
    }

    fn native(kind: LinuxConfigurationErrorKind) -> Self {
        Self {
            kind,
            configuration_code: None,
        }
    }

    fn configuration(error: &ConfigurationError) -> Self {
        Self {
            kind: LinuxConfigurationErrorKind::InvalidConfiguration,
            configuration_code: Some(error.code()),
        }
    }
}

impl fmt::Display for LinuxConfigurationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            LinuxConfigurationErrorKind::UnsafeRoot => "linux.configuration.unsafe_root",
            LinuxConfigurationErrorKind::TargetUnavailable => {
                "linux.configuration.target_unavailable"
            }
            LinuxConfigurationErrorKind::UnsafeObject => "linux.configuration.unsafe_object",
            LinuxConfigurationErrorKind::ResourceLimitExceeded => {
                "linux.configuration.resource_limit"
            }
            LinuxConfigurationErrorKind::Conflict => "linux.configuration.conflict",
            LinuxConfigurationErrorKind::DurabilityFailure => {
                "linux.configuration.durability_failure"
            }
            LinuxConfigurationErrorKind::InvalidConfiguration => {
                "linux.configuration.invalid_configuration"
            }
        })
    }
}

impl std::error::Error for LinuxConfigurationError {}

/// Linux-owned fixed-target configuration store with a continuously held root.
pub struct LinuxConfigurationStore {
    root: LinuxStrictLocalRoot,
    manager: ConfigurationManager,
}

impl fmt::Debug for LinuxConfigurationStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxConfigurationStore")
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}

impl LinuxConfigurationStore {
    /// Loads and validates the fixed current configuration without returning a path.
    pub fn load(&self) -> Result<LoadedConfiguration, LinuxConfigurationError> {
        let current = self.read_named(TARGET_NAME)?;
        self.manager
            .load_bytes(&current.bytes)
            .map_err(|error| LinuxConfigurationError::configuration(&error))
    }

    fn apply(
        &self,
        candidate: &[u8],
        publication_id: &str,
    ) -> Result<ApplyReceipt, LinuxConfigurationError> {
        let current_file = self.read_named(TARGET_NAME)?;
        let current = self
            .manager
            .load_bytes(&current_file.bytes)
            .map_err(|error| LinuxConfigurationError::configuration(&error))?;
        let next = self
            .manager
            .load_bytes(candidate)
            .map_err(|error| LinuxConfigurationError::configuration(&error))?;
        if current.sha256() == next.sha256() {
            return Err(LinuxConfigurationError::native(
                LinuxConfigurationErrorKind::Conflict,
            ));
        }
        self.retain_backup(current.sha256(), current.canonical_bytes())?;
        self.publish(
            &current_file,
            next.sha256(),
            next.canonical_bytes(),
            publication_id,
        )?;
        ApplyReceipt::from_verified_storage(
            current.sha256().to_owned(),
            next.sha256().to_owned(),
            current.sha256().to_owned(),
        )
        .map_err(|error| LinuxConfigurationError::configuration(&error))
    }

    fn rollback(
        &self,
        backup_sha256: &str,
        expected_current_sha256: &str,
        publication_id: &str,
    ) -> Result<LoadedConfiguration, LinuxConfigurationError> {
        validate_sha256(backup_sha256)?;
        validate_sha256(expected_current_sha256)?;
        let current_file = self.read_named(TARGET_NAME)?;
        let current = self
            .manager
            .load_bytes(&current_file.bytes)
            .map_err(|error| LinuxConfigurationError::configuration(&error))?;
        let backup_file = self.read_named(&backup_name(backup_sha256))?;
        if sha256(&backup_file.bytes) != backup_sha256 {
            return Err(LinuxConfigurationError::native(
                LinuxConfigurationErrorKind::Conflict,
            ));
        }
        let backup = self
            .manager
            .load_bytes(&backup_file.bytes)
            .map_err(|error| LinuxConfigurationError::configuration(&error))?;
        if current.sha256() == backup.sha256() {
            return Ok(backup);
        }
        if current.sha256() != expected_current_sha256 {
            return Err(LinuxConfigurationError::native(
                LinuxConfigurationErrorKind::Conflict,
            ));
        }
        self.publish(
            &current_file,
            backup.sha256(),
            backup.canonical_bytes(),
            publication_id,
        )?;
        self.load()
    }

    fn migrate_v0(
        &self,
        publication_id: &str,
    ) -> Result<MigrationApplyReceipt, LinuxConfigurationError> {
        let current_file = self.read_named(TARGET_NAME)?;
        let migration = self
            .manager
            .migrate_v0(&current_file.bytes)
            .map_err(|error| LinuxConfigurationError::configuration(&error))?;
        let previous_sha256 = sha256(&current_file.bytes);
        let migrated = migration.configuration();
        self.retain_backup(&previous_sha256, &current_file.bytes)?;
        self.publish(
            &current_file,
            migrated.sha256(),
            migrated.canonical_bytes(),
            publication_id,
        )?;
        MigrationApplyReceipt::from_verified_storage(
            previous_sha256.clone(),
            migrated.sha256().to_owned(),
            previous_sha256,
            migration.changes().to_vec(),
        )
        .map_err(|error| LinuxConfigurationError::configuration(&error))
    }

    fn rollback_migration(
        &self,
        backup_sha256: &str,
        expected_migrated_sha256: &str,
        publication_id: &str,
    ) -> Result<MigrationRollbackReceipt, LinuxConfigurationError> {
        validate_sha256(backup_sha256)?;
        validate_sha256(expected_migrated_sha256)?;
        let backup_file = self.read_named(&backup_name(backup_sha256))?;
        if sha256(&backup_file.bytes) != backup_sha256 {
            return Err(LinuxConfigurationError::native(
                LinuxConfigurationErrorKind::Conflict,
            ));
        }
        self.manager
            .migrate_v0(&backup_file.bytes)
            .map_err(|error| LinuxConfigurationError::configuration(&error))?;
        let current_file = self.read_named(TARGET_NAME)?;
        let current_sha256 = sha256(&current_file.bytes);
        if current_sha256 == backup_sha256 {
            return MigrationRollbackReceipt::from_verified_storage(backup_sha256.to_owned(), true)
                .map_err(|error| LinuxConfigurationError::configuration(&error));
        }
        if current_sha256 != expected_migrated_sha256 {
            return Err(LinuxConfigurationError::native(
                LinuxConfigurationErrorKind::Conflict,
            ));
        }
        self.manager
            .load_bytes(&current_file.bytes)
            .map_err(|error| LinuxConfigurationError::configuration(&error))?;
        self.publish(
            &current_file,
            backup_sha256,
            &backup_file.bytes,
            publication_id,
        )?;
        MigrationRollbackReceipt::from_verified_storage(backup_sha256.to_owned(), false)
            .map_err(|error| LinuxConfigurationError::configuration(&error))
    }

    fn retain_backup(
        &self,
        expected_sha256: &str,
        bytes: &[u8],
    ) -> Result<(), LinuxConfigurationError> {
        if sha256(bytes) != expected_sha256 {
            return Err(LinuxConfigurationError::native(
                LinuxConfigurationErrorKind::Conflict,
            ));
        }
        self.write_new_or_verify(&backup_name(expected_sha256), bytes)
    }

    fn publish(
        &self,
        expected: &HeldConfigurationFile,
        next_sha256: &str,
        next: &[u8],
        publication_id: &str,
    ) -> Result<(), LinuxConfigurationError> {
        if sha256(next) != next_sha256 {
            return Err(LinuxConfigurationError::native(
                LinuxConfigurationErrorKind::Conflict,
            ));
        }
        validate_sha256(publication_id)?;
        let temporary = candidate_name(publication_id);
        match self.read_named(&temporary) {
            Ok(retained)
                if retained.snapshot.same_object(&expected.snapshot)
                    && retained.bytes == expected.bytes =>
            {
                let published = self.read_named(TARGET_NAME)?;
                if published.bytes != next || sha256(&published.bytes) != next_sha256 {
                    return Err(LinuxConfigurationError::native(
                        LinuxConfigurationErrorKind::Conflict,
                    ));
                }
                let directory = self.directory()?;
                unlinkat(&directory, temporary.as_str(), AtFlags::empty()).map_err(|_| {
                    LinuxConfigurationError::native(LinuxConfigurationErrorKind::DurabilityFailure)
                })?;
                fsync(&directory).map_err(|_| {
                    LinuxConfigurationError::native(LinuxConfigurationErrorKind::DurabilityFailure)
                })?;
                return Ok(());
            }
            Ok(retained) if retained.bytes == next => {}
            Ok(_) => {
                return Err(LinuxConfigurationError::native(
                    LinuxConfigurationErrorKind::Conflict,
                ));
            }
            Err(error) if error.kind() == LinuxConfigurationErrorKind::TargetUnavailable => {
                self.write_new_or_verify(&temporary, next)?;
            }
            Err(error) => return Err(error),
        }
        let current = self.read_named(TARGET_NAME)?;
        if current.snapshot != expected.snapshot || current.bytes != expected.bytes {
            return Err(LinuxConfigurationError::native(
                LinuxConfigurationErrorKind::Conflict,
            ));
        }
        let directory = self.directory()?;
        renameat_with(
            &directory,
            temporary.as_str(),
            &directory,
            TARGET_NAME,
            RenameFlags::EXCHANGE,
        )
        .map_err(|_| {
            LinuxConfigurationError::native(LinuxConfigurationErrorKind::DurabilityFailure)
        })?;

        let displaced = self.read_named(&temporary)?;
        if !displaced.snapshot.same_object(&expected.snapshot) || displaced.bytes != expected.bytes
        {
            let _ = renameat_with(
                &directory,
                temporary.as_str(),
                &directory,
                TARGET_NAME,
                RenameFlags::EXCHANGE,
            );
            let _ = fsync(&directory);
            return Err(LinuxConfigurationError::native(
                LinuxConfigurationErrorKind::Conflict,
            ));
        }
        fsync(&directory).map_err(|_| {
            LinuxConfigurationError::native(LinuxConfigurationErrorKind::DurabilityFailure)
        })?;
        let published = self.read_named(TARGET_NAME)?;
        if published.bytes != next || sha256(&published.bytes) != next_sha256 {
            return Err(LinuxConfigurationError::native(
                LinuxConfigurationErrorKind::DurabilityFailure,
            ));
        }
        unlinkat(&directory, temporary.as_str(), AtFlags::empty()).map_err(|_| {
            LinuxConfigurationError::native(LinuxConfigurationErrorKind::DurabilityFailure)
        })?;
        fsync(&directory).map_err(|_| {
            LinuxConfigurationError::native(LinuxConfigurationErrorKind::DurabilityFailure)
        })?;
        Ok(())
    }

    fn write_new_or_verify(&self, name: &str, bytes: &[u8]) -> Result<(), LinuxConfigurationError> {
        if bytes.len() as u64 > MAX_CONFIGURATION_BYTES {
            return Err(LinuxConfigurationError::native(
                LinuxConfigurationErrorKind::ResourceLimitExceeded,
            ));
        }
        let directory = self.directory()?;
        match openat(
            &directory,
            name,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        ) {
            Ok(descriptor) => {
                let mut file = File::from(descriptor);
                file.write_all(bytes).map_err(|_| {
                    LinuxConfigurationError::native(LinuxConfigurationErrorKind::DurabilityFailure)
                })?;
                file.sync_all().map_err(|_| {
                    LinuxConfigurationError::native(LinuxConfigurationErrorKind::DurabilityFailure)
                })?;
                drop(file);
                fsync(&directory).map_err(|_| {
                    LinuxConfigurationError::native(LinuxConfigurationErrorKind::DurabilityFailure)
                })?;
                let retained = self.read_named(name)?;
                if retained.bytes != bytes {
                    return Err(LinuxConfigurationError::native(
                        LinuxConfigurationErrorKind::Conflict,
                    ));
                }
                Ok(())
            }
            Err(Errno::EXIST) => {
                let retained = self.read_named(name)?;
                if retained.bytes == bytes {
                    Ok(())
                } else {
                    Err(LinuxConfigurationError::native(
                        LinuxConfigurationErrorKind::Conflict,
                    ))
                }
            }
            Err(_) => Err(LinuxConfigurationError::native(
                LinuxConfigurationErrorKind::DurabilityFailure,
            )),
        }
    }

    fn read_named(&self, name: &str) -> Result<HeldConfigurationFile, LinuxConfigurationError> {
        self.root.revalidate().map_err(|_| {
            LinuxConfigurationError::native(LinuxConfigurationErrorKind::UnsafeRoot)
        })?;
        let directory = self.directory()?;
        let descriptor = openat(
            &directory,
            name,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|error| {
            LinuxConfigurationError::native(if error == Errno::NOENT {
                LinuxConfigurationErrorKind::TargetUnavailable
            } else {
                LinuxConfigurationErrorKind::UnsafeObject
            })
        })?;
        let before = file_snapshot(&directory, &descriptor)?;
        let mut file = File::from(descriptor);
        let mut bytes = Vec::new();
        Read::by_ref(&mut file)
            .take(MAX_CONFIGURATION_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| {
                LinuxConfigurationError::native(LinuxConfigurationErrorKind::TargetUnavailable)
            })?;
        if bytes.len() as u64 > MAX_CONFIGURATION_BYTES {
            return Err(LinuxConfigurationError::native(
                LinuxConfigurationErrorKind::ResourceLimitExceeded,
            ));
        }
        let after = file_snapshot(&directory, file_descriptor(&file))?;
        if before != after {
            return Err(LinuxConfigurationError::native(
                LinuxConfigurationErrorKind::Conflict,
            ));
        }
        self.root.revalidate().map_err(|_| {
            LinuxConfigurationError::native(LinuxConfigurationErrorKind::UnsafeRoot)
        })?;
        Ok(HeldConfigurationFile {
            bytes,
            snapshot: before,
        })
    }

    fn directory(&self) -> Result<OwnedFd, LinuxConfigurationError> {
        self.root
            .duplicate_io_descriptor()
            .map_err(|_| LinuxConfigurationError::native(LinuxConfigurationErrorKind::UnsafeRoot))
    }
}

/// Opens a fixed Linux configuration store only after aggregate platform activation.
pub fn open_linux_configuration_store(
    verified: &VerifiedPlatformAdapter<LinuxPlatformAdapter>,
    root: &Path,
) -> Result<LinuxConfigurationStore, LinuxConfigurationError> {
    if verified
        .adapter()
        .runtime_identity()
        .map_err(|_| LinuxConfigurationError::native(LinuxConfigurationErrorKind::UnsafeRoot))?
        .family()
        != verified.manifest_identity().target().family()
    {
        return Err(LinuxConfigurationError::native(
            LinuxConfigurationErrorKind::UnsafeRoot,
        ));
    }
    let root = LinuxStrictLocalRootInspector::inspect(root)
        .map_err(|_| LinuxConfigurationError::native(LinuxConfigurationErrorKind::UnsafeRoot))?;
    let store = LinuxConfigurationStore {
        root,
        manager: ConfigurationManager::default(),
    };
    let _ = store.load()?;
    Ok(store)
}

/// One exact fixed-target Linux configuration mutation.
pub enum LinuxConfigurationEffectRequest {
    /// Migrate the fixed target from legacy version zero.
    MigrateV0,
    /// Restore a retained legacy preimage.
    RollbackMigration {
        /// Exact retained backup identity.
        backup_sha256: String,
        /// Exact expected migrated target identity.
        expected_migrated_sha256: String,
    },
    /// Apply one complete bounded configuration candidate.
    Apply {
        /// Untrusted candidate bytes validated by the kernel before publication.
        candidate: Vec<u8>,
    },
    /// Restore one retained validated configuration.
    Rollback {
        /// Exact retained backup identity.
        backup_sha256: String,
        /// Exact expected current target identity.
        expected_current_sha256: String,
    },
}

impl fmt::Debug for LinuxConfigurationEffectRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let operation = match self {
            Self::MigrateV0 => "migrate-v0",
            Self::RollbackMigration { .. } => "rollback-migration",
            Self::Apply { .. } => "apply",
            Self::Rollback { .. } => "rollback",
        };
        formatter
            .debug_struct("LinuxConfigurationEffectRequest")
            .field("operation", &operation)
            .finish_non_exhaustive()
    }
}

/// Typed output retained only after one mediated Linux configuration mutation.
#[derive(Clone, Debug)]
pub enum LinuxConfigurationEffectOutput {
    /// Durable migration evidence.
    Migration(MigrationApplyReceipt),
    /// Durable migration rollback evidence.
    MigrationRollback(MigrationRollbackReceipt),
    /// Durable apply evidence.
    Applied(ApplyReceipt),
    /// Validated restored configuration.
    RolledBack(Box<LoadedConfiguration>),
}

impl LinuxConfigurationEffectOutput {
    fn identity(&self) -> &str {
        match self {
            Self::Migration(receipt) => receipt.migrated_sha256(),
            Self::MigrationRollback(receipt) => receipt.restored_sha256(),
            Self::Applied(receipt) => receipt.applied_sha256(),
            Self::RolledBack(configuration) => configuration.sha256(),
        }
    }
}

/// Linux configuration driver callable only with kernel-issued effect authorization.
pub struct LinuxConfigurationEffectDriver<'a> {
    store: &'a LinuxConfigurationStore,
    request: Option<LinuxConfigurationEffectRequest>,
    output: Option<LinuxConfigurationEffectOutput>,
    error: Option<LinuxConfigurationError>,
}

impl fmt::Debug for LinuxConfigurationEffectDriver<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxConfigurationEffectDriver")
            .field("has_request", &self.request.is_some())
            .field("has_output", &self.output.is_some())
            .field("has_error", &self.error.is_some())
            .finish_non_exhaustive()
    }
}

impl<'a> LinuxConfigurationEffectDriver<'a> {
    /// Creates an inert driver. Construction performs no filesystem mutation.
    #[must_use]
    pub const fn new(
        store: &'a LinuxConfigurationStore,
        request: LinuxConfigurationEffectRequest,
    ) -> Self {
        Self {
            store,
            request: Some(request),
            output: None,
            error: None,
        }
    }

    /// Takes the successful output after authority-transaction completion.
    pub fn take_output(&mut self) -> Option<LinuxConfigurationEffectOutput> {
        self.output.take()
    }

    /// Takes the bounded native error after a failed attempt.
    pub fn take_error(&mut self) -> Option<LinuxConfigurationError> {
        self.error.take()
    }
}

impl EffectDriver for LinuxConfigurationEffectDriver<'_> {
    fn execute(&mut self, authorization: EffectAuthorization<'_>) -> EffectLaunch {
        if authorization.operation().operation() != GrantOperation::Administration {
            return EffectLaunch::failed();
        }
        let Some(request) = self.request.take() else {
            return EffectLaunch::failed();
        };
        let publication_id = sha256(authorization.transaction_id().as_str().as_bytes());
        let result = match request {
            LinuxConfigurationEffectRequest::MigrateV0 => self
                .store
                .migrate_v0(&publication_id)
                .map(LinuxConfigurationEffectOutput::Migration),
            LinuxConfigurationEffectRequest::RollbackMigration {
                backup_sha256,
                expected_migrated_sha256,
            } => self
                .store
                .rollback_migration(&backup_sha256, &expected_migrated_sha256, &publication_id)
                .map(LinuxConfigurationEffectOutput::MigrationRollback),
            LinuxConfigurationEffectRequest::Apply { candidate } => self
                .store
                .apply(&candidate, &publication_id)
                .map(LinuxConfigurationEffectOutput::Applied),
            LinuxConfigurationEffectRequest::Rollback {
                backup_sha256,
                expected_current_sha256,
            } => self
                .store
                .rollback(&backup_sha256, &expected_current_sha256, &publication_id)
                .map(Box::new)
                .map(LinuxConfigurationEffectOutput::RolledBack),
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
                    error.to_string().as_bytes(),
                    StateChange::Uncertain,
                );
                self.error = Some(error);
                EffectLaunch::completed(effect_result)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FileSnapshot {
    device: u64,
    inode: u64,
    link_count: u64,
    owner: u32,
    mode: u32,
    size: i64,
    modified_seconds: i64,
    modified_nanoseconds: u64,
    changed_seconds: i64,
    changed_nanoseconds: u64,
}

impl FileSnapshot {
    fn same_object(&self, other: &Self) -> bool {
        self.device == other.device
            && self.inode == other.inode
            && self.link_count == other.link_count
            && self.owner == other.owner
            && self.mode == other.mode
            && self.size == other.size
    }
}

struct HeldConfigurationFile {
    bytes: Vec<u8>,
    snapshot: FileSnapshot,
}

fn file_snapshot(
    directory: &OwnedFd,
    descriptor: impl std::os::fd::AsFd,
) -> Result<FileSnapshot, LinuxConfigurationError> {
    let directory_stat = fstat(directory)
        .map_err(|_| LinuxConfigurationError::native(LinuxConfigurationErrorKind::UnsafeRoot))?;
    let stat = fstat(descriptor)
        .map_err(|_| LinuxConfigurationError::native(LinuxConfigurationErrorKind::UnsafeObject))?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile
        || stat.st_dev != directory_stat.st_dev
        || stat.st_uid != getuid().as_raw()
        || stat.st_mode & 0o777 != 0o600
        || stat.st_nlink != 1
    {
        return Err(LinuxConfigurationError::native(
            LinuxConfigurationErrorKind::UnsafeObject,
        ));
    }
    Ok(FileSnapshot {
        device: stat.st_dev,
        inode: stat.st_ino,
        link_count: stat.st_nlink,
        owner: stat.st_uid,
        mode: stat.st_mode & 0o777,
        size: stat.st_size,
        modified_seconds: stat.st_mtime,
        modified_nanoseconds: stat.st_mtime_nsec as u64,
        changed_seconds: stat.st_ctime,
        changed_nanoseconds: stat.st_ctime_nsec as u64,
    })
}

fn file_descriptor(file: &File) -> impl std::os::fd::AsFd + '_ {
    file
}

fn backup_name(sha256: &str) -> String {
    format!(".agentmage-backup-{sha256}.json")
}

fn candidate_name(publication_id: &str) -> String {
    format!(".agentmage-new-{publication_id}.tmp")
}

fn validate_sha256(value: &str) -> Result<(), LinuxConfigurationError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(LinuxConfigurationError::native(
            LinuxConfigurationErrorKind::Conflict,
        ))
    }
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_engine::configuration::ConfigurationManager;

    use super::{
        LinuxConfigurationErrorKind, LinuxConfigurationStore, TARGET_NAME, backup_name, sha256,
    };
    use crate::LinuxStrictLocalRootInspector;

    const STRICT: &[u8] =
        include_bytes!("../../../configuration/profiles/strict-local-read-only.json");
    const SYNTHETIC: &[u8] = include_bytes!("../../../configuration/profiles/synthetic-test.json");
    const LEGACY: &[u8] = include_bytes!("../../../fixtures/configuration/migration/v0.valid.json");

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(1);

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new(label: &str, content: &[u8]) -> Self {
            let path = std::env::temp_dir().join(format!(
                "agentmage-linux-configuration-{label}-{}-{}",
                std::process::id(),
                NEXT_ROOT.fetch_add(1, Ordering::SeqCst)
            ));
            fs::create_dir(&path).expect("root creates");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("root is private");
            fs::write(path.join(TARGET_NAME), content).expect("target writes");
            fs::set_permissions(path.join(TARGET_NAME), fs::Permissions::from_mode(0o600))
                .expect("target is private");
            Self(path)
        }

        fn store(&self) -> LinuxConfigurationStore {
            LinuxConfigurationStore {
                root: LinuxStrictLocalRootInspector::inspect(&self.0).expect("root inspects"),
                manager: ConfigurationManager::default(),
            }
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("root removes");
        }
    }

    #[test]
    fn strict_local_state_root_configuration_load_rejects_sync_before_read() {
        let root = TestRoot::new("synchronized", STRICT);
        fs::create_dir(root.path().join(".stfolder")).expect("sync marker creates");
        let before = fs::read(root.path().join(TARGET_NAME)).expect("target preimage reads");
        assert_eq!(
            root.store()
                .load()
                .expect_err("synchronized configuration root rejects")
                .kind(),
            LinuxConfigurationErrorKind::UnsafeRoot
        );
        assert_eq!(
            fs::read(root.path().join(TARGET_NAME)).expect("target remains readable"),
            before
        );
    }

    #[test]
    fn apply_retains_immutable_backup_and_rollback_restores_it() {
        let root = TestRoot::new("apply", STRICT);
        let store = root.store();
        let before = store.load().expect("baseline loads");
        let receipt = store
            .apply(SYNTHETIC, &"1".repeat(64))
            .expect("candidate applies");
        assert_eq!(receipt.previous_sha256(), before.sha256());
        assert_eq!(receipt.backup_sha256(), before.sha256());
        assert_eq!(
            fs::symlink_metadata(root.path().join(backup_name(before.sha256())))
                .expect("backup metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        let applied = store.load().expect("applied loads");
        assert_eq!(applied.sha256(), receipt.applied_sha256());
        let restored = store
            .rollback(
                receipt.backup_sha256(),
                receipt.applied_sha256(),
                &"2".repeat(64),
            )
            .expect("rollback succeeds");
        assert_eq!(restored.sha256(), before.sha256());
    }

    #[test]
    fn migration_and_repeatable_rollback_preserve_the_exact_legacy_preimage() {
        let root = TestRoot::new("migration", LEGACY);
        let store = root.store();
        let receipt = store
            .migrate_v0(&"3".repeat(64))
            .expect("migration succeeds");
        assert_eq!(receipt.previous_sha256(), sha256(LEGACY));
        assert_eq!(receipt.backup_sha256(), sha256(LEGACY));
        let first = store
            .rollback_migration(
                receipt.backup_sha256(),
                receipt.migrated_sha256(),
                &"4".repeat(64),
            )
            .expect("rollback succeeds");
        assert!(!first.already_restored());
        assert_eq!(
            fs::read(root.path().join(TARGET_NAME)).expect("target"),
            LEGACY
        );
        let repeated = store
            .rollback_migration(
                receipt.backup_sha256(),
                receipt.migrated_sha256(),
                &"5".repeat(64),
            )
            .expect("rollback retry succeeds");
        assert!(repeated.already_restored());
    }

    #[test]
    fn symlink_hard_link_and_public_mode_targets_fail_closed() {
        let symlink_root = TestRoot::new("symlink", STRICT);
        let target = symlink_root.path().join(TARGET_NAME);
        let replacement = symlink_root.path().join("replacement.json");
        fs::rename(&target, &replacement).expect("target moves");
        symlink(&replacement, &target).expect("symlink creates");
        assert_eq!(
            symlink_root
                .store()
                .load()
                .expect_err("symlink rejects")
                .kind(),
            LinuxConfigurationErrorKind::UnsafeObject
        );

        let hard_link_root = TestRoot::new("hard-link", STRICT);
        fs::hard_link(
            hard_link_root.path().join(TARGET_NAME),
            hard_link_root.path().join("alias.json"),
        )
        .expect("hard link creates");
        assert_eq!(
            hard_link_root
                .store()
                .load()
                .expect_err("hard link rejects")
                .kind(),
            LinuxConfigurationErrorKind::UnsafeObject
        );

        let mode_root = TestRoot::new("mode", STRICT);
        fs::set_permissions(
            mode_root.path().join(TARGET_NAME),
            fs::Permissions::from_mode(0o640),
        )
        .expect("mode weakens");
        assert_eq!(
            mode_root.store().load().expect_err("mode rejects").kind(),
            LinuxConfigurationErrorKind::UnsafeObject
        );
    }

    #[test]
    fn changed_preimage_is_preserved_and_never_overwritten() {
        let root = TestRoot::new("preimage", STRICT);
        let store = root.store();
        let expected = store.read_named(TARGET_NAME).expect("preimage held");
        fs::write(root.path().join(TARGET_NAME), SYNTHETIC).expect("concurrent replacement");
        fs::set_permissions(
            root.path().join(TARGET_NAME),
            fs::Permissions::from_mode(0o600),
        )
        .expect("replacement private");
        let next = ConfigurationManager::default()
            .safe_defaults()
            .expect("candidate loads");
        assert_eq!(
            store
                .publish(
                    &expected,
                    next.sha256(),
                    next.canonical_bytes(),
                    &"6".repeat(64),
                )
                .expect_err("changed preimage rejects")
                .kind(),
            LinuxConfigurationErrorKind::Conflict
        );
        assert_eq!(
            fs::read(root.path().join(TARGET_NAME)).expect("replacement remains"),
            SYNTHETIC
        );
    }

    #[test]
    fn interrupted_exchange_is_verified_and_completed_without_republication() {
        let root = TestRoot::new("interrupted-exchange", STRICT);
        let store = root.store();
        let expected = store.read_named(TARGET_NAME).expect("preimage held");
        let next = ConfigurationManager::default()
            .load_bytes(SYNTHETIC)
            .expect("candidate loads");
        let publication_id = "7".repeat(64);
        let temporary = root.path().join(super::candidate_name(&publication_id));
        fs::rename(root.path().join(TARGET_NAME), &temporary).expect("old target displaced");
        fs::write(root.path().join(TARGET_NAME), next.canonical_bytes())
            .expect("new target published");
        fs::set_permissions(
            root.path().join(TARGET_NAME),
            fs::Permissions::from_mode(0o600),
        )
        .expect("published target private");

        store
            .publish(
                &expected,
                next.sha256(),
                next.canonical_bytes(),
                &publication_id,
            )
            .expect("interrupted exchange completes");
        assert!(!temporary.exists());
        assert_eq!(
            fs::read(root.path().join(TARGET_NAME)).expect("published target remains"),
            next.canonical_bytes()
        );
    }
}
