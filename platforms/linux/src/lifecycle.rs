//! Explicit Linux first-install operational-key lifecycle.

use std::fmt;
use std::path::Path;

use agentmage_kernel_engine::platform_startup::VerifiedPlatformAdapter;
use rustix::fd::OwnedFd;
use rustix::fs::{
    AtFlags, FileType, FlockOperation, Mode, OFlags, flock, fstat, fsync, openat, statat,
};
use rustix::io::Errno;
use rustix::process::getuid;

use crate::secret_service::{operational_store_key_exists, provision_operational_store_key};
use crate::{
    LinuxPlatformAdapter, LinuxSecretReceipt, LinuxSecretService, LinuxStrictLocalRoot,
    LinuxStrictLocalRootInspector,
};

const LIFECYCLE_LOCK_NAME: &str = ".agentmage-key-provision.lock";
const AUTHORITY_DATABASE_NAME: &str = "authority.db";

/// Stable explicit operational-key lifecycle failure class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxOperationalKeyLifecycleErrorKind {
    /// The descriptor-held state root failed admission or revalidation.
    UnsafeRoot,
    /// The fixed lifecycle lock or authority database had unsafe metadata.
    UnsafeStateObject,
    /// Another lifecycle operation currently owns the fixed lock.
    LifecycleBusy,
    /// Encrypted authority state exists but its fixed key does not.
    ExistingStateWithoutKey,
    /// The fixed operational key already exists.
    AlreadyProvisioned,
    /// Secret Service provisioning or exact lookup verification failed.
    SecretServiceFailure,
    /// Interruption-safe key rotation is deliberately unavailable.
    RotationUnavailable,
}

/// Content-free explicit operational-key lifecycle failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxOperationalKeyLifecycleError {
    kind: LinuxOperationalKeyLifecycleErrorKind,
}

impl LinuxOperationalKeyLifecycleError {
    /// Returns the stable failure class.
    #[must_use]
    pub const fn kind(self) -> LinuxOperationalKeyLifecycleErrorKind {
        self.kind
    }
}

impl fmt::Display for LinuxOperationalKeyLifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            LinuxOperationalKeyLifecycleErrorKind::UnsafeRoot => {
                "linux.operational-key.root.unsafe"
            }
            LinuxOperationalKeyLifecycleErrorKind::UnsafeStateObject => {
                "linux.operational-key.state-object.unsafe"
            }
            LinuxOperationalKeyLifecycleErrorKind::LifecycleBusy => {
                "linux.operational-key.lifecycle.busy"
            }
            LinuxOperationalKeyLifecycleErrorKind::ExistingStateWithoutKey => {
                "linux.operational-key.state-without-key"
            }
            LinuxOperationalKeyLifecycleErrorKind::AlreadyProvisioned => {
                "linux.operational-key.already-provisioned"
            }
            LinuxOperationalKeyLifecycleErrorKind::SecretServiceFailure => {
                "linux.operational-key.secret-service"
            }
            LinuxOperationalKeyLifecycleErrorKind::RotationUnavailable => {
                "linux.operational-key.rotation-unavailable"
            }
        })
    }
}

impl std::error::Error for LinuxOperationalKeyLifecycleError {}

/// Content-free evidence that explicit first-install key provisioning completed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxOperationalKeyProvisionReceipt {
    secret_service_receipt: LinuxSecretReceipt,
}

impl LinuxOperationalKeyProvisionReceipt {
    /// Returns the content-free Secret Service store receipt.
    #[must_use]
    pub const fn secret_service_receipt(&self) -> &LinuxSecretReceipt {
        &self.secret_service_receipt
    }
}

/// Explicitly provisions the fixed operational-store key for an empty state root.
pub fn provision_linux_operational_key(
    _verified: &VerifiedPlatformAdapter<LinuxPlatformAdapter>,
    state_root: &Path,
    service: &LinuxSecretService,
    profile_id: impl Into<String>,
) -> Result<LinuxOperationalKeyProvisionReceipt, LinuxOperationalKeyLifecycleError> {
    let root = LinuxStrictLocalRootInspector::inspect(state_root)
        .map_err(|_| lifecycle_error(LinuxOperationalKeyLifecycleErrorKind::UnsafeRoot))?;
    root.revalidate()
        .map_err(|_| lifecycle_error(LinuxOperationalKeyLifecycleErrorKind::UnsafeRoot))?;
    let _lock = acquire_lifecycle_lock(&root)?;
    let profile_id = profile_id.into();
    let key_exists = operational_store_key_exists(service, profile_id.clone()).map_err(|_| {
        lifecycle_error(LinuxOperationalKeyLifecycleErrorKind::SecretServiceFailure)
    })?;
    let database_exists = authority_database_exists(&root)?;
    match (key_exists, database_exists) {
        (true, _) => {
            return Err(lifecycle_error(
                LinuxOperationalKeyLifecycleErrorKind::AlreadyProvisioned,
            ));
        }
        (false, true) => {
            return Err(lifecycle_error(
                LinuxOperationalKeyLifecycleErrorKind::ExistingStateWithoutKey,
            ));
        }
        (false, false) => {}
    }
    let secret_service_receipt =
        provision_operational_store_key(service, profile_id).map_err(|failure| {
            lifecycle_error(
                if failure.kind() == crate::LinuxSecretServiceErrorKind::AlreadyProvisioned {
                    LinuxOperationalKeyLifecycleErrorKind::AlreadyProvisioned
                } else {
                    LinuxOperationalKeyLifecycleErrorKind::SecretServiceFailure
                },
            )
        })?;
    root.revalidate()
        .map_err(|_| lifecycle_error(LinuxOperationalKeyLifecycleErrorKind::UnsafeRoot))?;
    Ok(LinuxOperationalKeyProvisionReceipt {
        secret_service_receipt,
    })
}

/// Refuses key rotation until a tested interruption-safe dual-key protocol exists.
pub fn rotate_linux_operational_key(
    _verified: &VerifiedPlatformAdapter<LinuxPlatformAdapter>,
) -> Result<(), LinuxOperationalKeyLifecycleError> {
    Err(lifecycle_error(
        LinuxOperationalKeyLifecycleErrorKind::RotationUnavailable,
    ))
}

fn acquire_lifecycle_lock(
    root: &LinuxStrictLocalRoot,
) -> Result<LinuxLifecycleLock, LinuxOperationalKeyLifecycleError> {
    let directory = root
        .duplicate_io_descriptor()
        .map_err(|_| lifecycle_error(LinuxOperationalKeyLifecycleErrorKind::UnsafeRoot))?;
    let lock = openat(
        &directory,
        LIFECYCLE_LOCK_NAME,
        OFlags::RDWR | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    )
    .map_err(|_| lifecycle_error(LinuxOperationalKeyLifecycleErrorKind::UnsafeStateObject))?;
    verify_fixed_regular_file(&directory, LIFECYCLE_LOCK_NAME, &lock)?;
    flock(&lock, FlockOperation::NonBlockingLockExclusive).map_err(|failure| {
        lifecycle_error(if failure == Errno::WOULDBLOCK {
            LinuxOperationalKeyLifecycleErrorKind::LifecycleBusy
        } else {
            LinuxOperationalKeyLifecycleErrorKind::UnsafeStateObject
        })
    })?;
    verify_fixed_regular_file(&directory, LIFECYCLE_LOCK_NAME, &lock)?;
    fsync(&lock)
        .and_then(|()| fsync(&directory))
        .map_err(|_| lifecycle_error(LinuxOperationalKeyLifecycleErrorKind::UnsafeStateObject))?;
    Ok(LinuxLifecycleLock(lock))
}

#[derive(Debug)]
struct LinuxLifecycleLock(OwnedFd);

impl Drop for LinuxLifecycleLock {
    fn drop(&mut self) {
        let _ = flock(&self.0, FlockOperation::Unlock);
    }
}

fn authority_database_exists(
    root: &LinuxStrictLocalRoot,
) -> Result<bool, LinuxOperationalKeyLifecycleError> {
    let directory = root
        .duplicate_io_descriptor()
        .map_err(|_| lifecycle_error(LinuxOperationalKeyLifecycleErrorKind::UnsafeRoot))?;
    let database = match openat(
        &directory,
        AUTHORITY_DATABASE_NAME,
        OFlags::PATH | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(database) => database,
        Err(Errno::NOENT) => return Ok(false),
        Err(_) => {
            return Err(lifecycle_error(
                LinuxOperationalKeyLifecycleErrorKind::UnsafeStateObject,
            ));
        }
    };
    verify_fixed_regular_file(&directory, AUTHORITY_DATABASE_NAME, &database)?;
    Ok(true)
}

fn verify_fixed_regular_file(
    directory: &OwnedFd,
    name: &str,
    descriptor: &OwnedFd,
) -> Result<(), LinuxOperationalKeyLifecycleError> {
    let held = fstat(descriptor)
        .map_err(|_| lifecycle_error(LinuxOperationalKeyLifecycleErrorKind::UnsafeStateObject))?;
    let named = statat(directory, name, AtFlags::SYMLINK_NOFOLLOW)
        .map_err(|_| lifecycle_error(LinuxOperationalKeyLifecycleErrorKind::UnsafeStateObject))?;
    let parent = fstat(directory)
        .map_err(|_| lifecycle_error(LinuxOperationalKeyLifecycleErrorKind::UnsafeStateObject))?;
    if FileType::from_raw_mode(held.st_mode) != FileType::RegularFile
        || held.st_dev != named.st_dev
        || held.st_ino != named.st_ino
        || held.st_dev != parent.st_dev
        || held.st_uid != getuid().as_raw()
        || held.st_nlink != 1
        || held.st_mode & 0o077 != 0
    {
        return Err(lifecycle_error(
            LinuxOperationalKeyLifecycleErrorKind::UnsafeStateObject,
        ));
    }
    Ok(())
}

const fn lifecycle_error(
    kind: LinuxOperationalKeyLifecycleErrorKind,
) -> LinuxOperationalKeyLifecycleError {
    LinuxOperationalKeyLifecycleError { kind }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{
        LinuxOperationalKeyLifecycleErrorKind, acquire_lifecycle_lock, authority_database_exists,
    };
    use crate::LinuxStrictLocalRootInspector;

    static TEMP_ID: AtomicU64 = AtomicU64::new(1);

    #[test]
    fn lifecycle_lock_is_private_single_link_and_exclusive() {
        let directory = private_directory();
        let root = LinuxStrictLocalRootInspector::inspect(&directory).expect("private root");
        let first = acquire_lifecycle_lock(&root).expect("first lock");
        assert_eq!(
            acquire_lifecycle_lock(&root)
                .expect_err("second lock")
                .kind(),
            LinuxOperationalKeyLifecycleErrorKind::LifecycleBusy
        );
        let metadata = std::fs::symlink_metadata(directory.join(super::LIFECYCLE_LOCK_NAME))
            .expect("lock metadata");
        assert_eq!(metadata.mode() & 0o777, 0o600);
        assert_eq!(metadata.nlink(), 1);
        drop(first);
        acquire_lifecycle_lock(&root).expect("released lock");
        std::fs::remove_dir_all(directory).expect("fixture removes");
    }

    #[test]
    fn authority_database_presence_rejects_links_and_public_objects() {
        let directory = private_directory();
        let root = LinuxStrictLocalRootInspector::inspect(&directory).expect("private root");
        assert!(!authority_database_exists(&root).expect("absent database"));
        let database = directory.join(super::AUTHORITY_DATABASE_NAME);
        std::fs::write(&database, b"synthetic").expect("database fixture");
        std::fs::set_permissions(&database, std::fs::Permissions::from_mode(0o600))
            .expect("private database");
        assert!(authority_database_exists(&root).expect("safe database"));
        std::fs::set_permissions(&database, std::fs::Permissions::from_mode(0o644))
            .expect("public database");
        assert_eq!(
            authority_database_exists(&root)
                .expect_err("public database rejected")
                .kind(),
            LinuxOperationalKeyLifecycleErrorKind::UnsafeStateObject
        );
        std::fs::remove_dir_all(directory).expect("fixture removes");
    }

    fn private_directory() -> std::path::PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "agentmage-key-lifecycle-{}-{}",
            std::process::id(),
            TEMP_ID.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir(&directory).expect("fixture creates");
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))
            .expect("fixture private");
        directory
    }
}
