//! One descriptor-held native inference owner across participating local hosts.

use std::fs::File;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use agentmage_kernel_contracts::ModelRuntimeFailure;
use rustix::fs::{AtFlags, FlockOperation, Mode, OFlags, flock, open, openat, statat};

const LOCK_NAME: &str = "agentmage-native-inference-v1.lock";

/// Not cloneable or serializable: ownership ends only when the descriptor closes.
pub(crate) struct NativeInferenceLease {
    _file: File,
}

impl NativeInferenceLease {
    #[cfg(test)]
    pub(crate) fn acquire_for_test(root: &Path) -> Result<Self, ModelRuntimeFailure> {
        Self::acquire_in(root)
    }

    pub(crate) fn retain_until_process_exit(self) {
        // Intentional fail-closed descriptor retention after unverified cleanup.
        // A second participating model cannot launch in this process or another.
        // The kernel releases the descriptor on process exit, never by unlinking.
        std::mem::forget(self);
    }

    pub(crate) fn acquire() -> Result<Self, ModelRuntimeFailure> {
        // Deliberately ignore caller-controlled XDG paths and model/workspace names.
        let uid = rustix::process::geteuid().as_raw();
        if uid == 0 {
            return Err(denied("model.native-lease.unsafe-root", false));
        }
        Self::acquire_in(Path::new(&format!("/run/user/{uid}")))
    }

    fn acquire_in(root: &Path) -> Result<Self, ModelRuntimeFailure> {
        let unsafe_root = || denied("model.native-lease.unsafe-root", false);
        if !root.is_absolute() || root.canonicalize().ok().as_deref() != Some(root) {
            return Err(unsafe_root());
        }
        let directory = File::from(
            open(
                root,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|_| unsafe_root())?,
        );
        let metadata = directory.metadata().map_err(|_| unsafe_root())?;
        if !metadata.is_dir()
            || metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.mode() & 0o7777 != 0o700
        {
            return Err(unsafe_root());
        }
        let unsafe_lock = || denied("model.native-lease.unsafe-lock", false);
        let file = File::from(
            openat(
                &directory,
                LOCK_NAME,
                OFlags::RDWR
                    | OFlags::CREATE
                    | OFlags::NOFOLLOW
                    | OFlags::CLOEXEC
                    | OFlags::NONBLOCK,
                Mode::from_raw_mode(0o600),
            )
            .map_err(|_| unsafe_lock())?,
        );
        let verify = || {
            let held = file.metadata().map_err(|_| unsafe_lock())?;
            let named = statat(&directory, LOCK_NAME, AtFlags::SYMLINK_NOFOLLOW)
                .map_err(|_| unsafe_lock())?;
            let current_root = std::fs::symlink_metadata(root).map_err(|_| unsafe_root())?;
            if !held.is_file()
                || held.uid() != metadata.uid()
                || held.mode() & 0o7777 != 0o600
                || held.nlink() != 1
                || held.len() != 0
                || held.dev() != named.st_dev
                || held.ino() != named.st_ino
                || current_root.dev() != metadata.dev()
                || current_root.ino() != metadata.ino()
                || current_root.mode() != metadata.mode()
                || current_root.uid() != metadata.uid()
            {
                return Err(unsafe_lock());
            }
            Ok(())
        };
        verify()?;
        flock(&file, FlockOperation::NonBlockingLockExclusive).map_err(|error| {
            if error == rustix::io::Errno::WOULDBLOCK {
                denied("model.native-lease.busy", true)
            } else {
                unsafe_lock()
            }
        })?;
        verify()?;
        // Never unlink: a replacement inode would permit two owners.
        Ok(Self { _file: file })
    }
}

fn denied(code: &str, dependency: bool) -> ModelRuntimeFailure {
    ModelRuntimeFailure {
        code: code.to_owned(),
        retryable_after_correction: false,
        dependency_recovery_required: dependency,
        contract_error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::path::PathBuf;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "agentmage-lease-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            Self(path.canonicalize().unwrap())
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn contention_refuses_and_drop_releases_without_removing_lock() {
        let root = Directory::new();
        let first = NativeInferenceLease::acquire_in(&root.0).unwrap();
        assert_eq!(
            NativeInferenceLease::acquire_in(&root.0)
                .err()
                .unwrap()
                .code,
            "model.native-lease.busy"
        );
        let before = fs::metadata(root.0.join(LOCK_NAME)).unwrap().ino();
        drop(first);
        let _second = NativeInferenceLease::acquire_in(&root.0).unwrap();
        assert_eq!(before, fs::metadata(root.0.join(LOCK_NAME)).unwrap().ino());
    }

    #[test]
    fn unsafe_roots_and_aliases_refuse() {
        let root = Directory::new();
        fs::set_permissions(&root.0, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(NativeInferenceLease::acquire_in(&root.0).is_err());
        fs::set_permissions(&root.0, fs::Permissions::from_mode(0o700)).unwrap();
        symlink(&root.0, root.0.join("alias")).unwrap();
        assert!(NativeInferenceLease::acquire_in(&root.0.join("alias")).is_err());
    }

    #[test]
    fn linked_nonempty_and_nonprivate_locks_refuse() {
        let root = Directory::new();
        let path = root.0.join(LOCK_NAME);
        drop(NativeInferenceLease::acquire_in(&root.0).unwrap());
        fs::hard_link(&path, root.0.join("link")).unwrap();
        assert!(NativeInferenceLease::acquire_in(&root.0).is_err());
        fs::remove_file(root.0.join("link")).unwrap();
        fs::write(&path, b"untrusted").unwrap();
        assert!(NativeInferenceLease::acquire_in(&root.0).is_err());
        fs::write(&path, b"").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(NativeInferenceLease::acquire_in(&root.0).is_err());
    }

    #[test]
    fn symlink_lock_refuses_without_touching_target() {
        let root = Directory::new();
        let target = root.0.join("sentinel");
        fs::write(&target, b"preserve").unwrap();
        symlink(&target, root.0.join(LOCK_NAME)).unwrap();
        assert!(NativeInferenceLease::acquire_in(&root.0).is_err());
        assert_eq!(fs::read(target).unwrap(), b"preserve");
    }

    #[test]
    fn separate_process_observes_owner_then_release() {
        let root = Directory::new();
        let owner = NativeInferenceLease::acquire_in(&root.0).unwrap();
        let child = |expect_busy: bool| {
            let result = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "native_inference_lease::tests::lease_child",
                    "--ignored",
                    "--test-threads=1",
                ])
                .env("AGENTMAGE_LEASE_TEST_ROOT", &root.0)
                .env(
                    "AGENTMAGE_LEASE_TEST_BUSY",
                    if expect_busy { "yes" } else { "no" },
                )
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stdout)
            );
        };
        child(true);
        drop(owner);
        child(false);
    }

    #[test]
    fn uncertain_cleanup_holds_until_owner_process_exits() {
        let root = Directory::new();
        let result = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "native_inference_lease::tests::retained_lease_child",
                "--ignored",
                "--test-threads=1",
            ])
            .env("AGENTMAGE_LEASE_TEST_ROOT", &root.0)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stdout)
        );
        assert!(NativeInferenceLease::acquire_in(&root.0).is_ok());
    }

    #[test]
    #[ignore = "subprocess helper, exercised by uncertain_cleanup_holds_until_owner_process_exits"]
    fn retained_lease_child() {
        let root = std::env::var_os("AGENTMAGE_LEASE_TEST_ROOT").unwrap();
        NativeInferenceLease::acquire_in(Path::new(&root))
            .unwrap()
            .retain_until_process_exit();
        assert_eq!(
            NativeInferenceLease::acquire_in(Path::new(&root))
                .err()
                .unwrap()
                .code,
            "model.native-lease.busy"
        );
    }

    #[test]
    #[ignore = "subprocess helper, exercised by separate_process_observes_owner_then_release"]
    fn lease_child() {
        let root = std::env::var_os("AGENTMAGE_LEASE_TEST_ROOT").unwrap();
        let result = NativeInferenceLease::acquire_in(Path::new(&root));
        if std::env::var("AGENTMAGE_LEASE_TEST_BUSY").unwrap() == "yes" {
            assert_eq!(result.err().unwrap().code, "model.native-lease.busy");
        } else {
            assert!(result.is_ok());
        }
    }
}
