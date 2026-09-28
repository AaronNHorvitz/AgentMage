//! Descriptor-held inference ownership with conservative crash quarantine.

use std::fs::{self, File};
use std::os::unix::fs::{FileExt, MetadataExt};
use std::path::{Path, PathBuf};

use agentmage_kernel_contracts::ModelRuntimeFailure;
use rustix::fs::{AtFlags, FlockOperation, Mode, OFlags, flock, open, openat, statat};
use rustix::rand::{GetRandomFlags, getrandom};

const LOCK_NAME: &str = "agentmage-native-inference-v1.lock";
const MARKER_BYTES: usize = 64;

/// Not cloneable or serializable. An armed marker survives descriptor/owner exit.
pub(crate) struct NativeInferenceLease {
    file: File,
    directory: File,
    root: PathBuf,
    root_identity: (u64, u64, u32, u32, u32),
    file_identity: (u64, u64, u32, u32, u32),
    marker: Option<[u8; MARKER_BYTES]>,
}

fn identity(metadata: &fs::Metadata) -> (u64, u64, u32, u32, u32) {
    (
        metadata.dev(),
        metadata.ino(),
        metadata.mode(),
        metadata.uid(),
        metadata.gid(),
    )
}

impl NativeInferenceLease {
    #[cfg(test)]
    pub(crate) fn acquire_for_test(root: &Path) -> Result<Self, ModelRuntimeFailure> {
        Self::acquire_in(root)
    }

    pub(crate) fn retain_until_process_exit(self) {
        // Keep the exact inode locked during this process; an armed marker also
        // denies future admission after the kernel closes this descriptor.
        std::mem::forget(self);
    }

    pub(crate) fn acquire() -> Result<Self, ModelRuntimeFailure> {
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
        let root_metadata = directory.metadata().map_err(|_| unsafe_root())?;
        if !root_metadata.is_dir()
            || root_metadata.uid() != rustix::process::geteuid().as_raw()
            || root_metadata.mode() & 0o7777 != 0o700
        {
            return Err(unsafe_root());
        }
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
            .map_err(|_| denied("model.native-lease.unsafe-lock", false))?,
        );
        let held = file
            .metadata()
            .map_err(|_| denied("model.native-lease.unsafe-lock", false))?;
        let lease = Self {
            file,
            directory,
            root: root.to_path_buf(),
            root_identity: identity(&root_metadata),
            file_identity: identity(&held),
            marker: None,
        };
        lease.verify_identity()?;
        // Exclusivity is observed before contents, so a live cooperating owner
        // reports busy; abandoned or malformed nonempty state remains uncertain.
        flock(&lease.file, FlockOperation::NonBlockingLockExclusive).map_err(|error| {
            if error == rustix::io::Errno::WOULDBLOCK {
                denied("model.native-lease.busy", true)
            } else {
                denied("model.native-lease.unsafe-lock", false)
            }
        })?;
        lease.verify_identity()?;
        if lease
            .file
            .metadata()
            .map_err(|_| denied("model.native-lease.unsafe-lock", false))?
            .len()
            != 0
        {
            return Err(denied("model.native-lease.cleanup-uncertain", true));
        }
        Ok(lease)
    }

    fn verify_identity(&self) -> Result<(), ModelRuntimeFailure> {
        let error = || denied("model.native-lease.unsafe-lock", false);
        let root = fs::symlink_metadata(&self.root).map_err(|_| error())?;
        let held_root = self.directory.metadata().map_err(|_| error())?;
        let held = self.file.metadata().map_err(|_| error())?;
        let named =
            statat(&self.directory, LOCK_NAME, AtFlags::SYMLINK_NOFOLLOW).map_err(|_| error())?;
        if self.root.canonicalize().ok().as_deref() != Some(self.root.as_path())
            || identity(&root) != self.root_identity
            || identity(&held_root) != self.root_identity
            || !held.is_file()
            || held.uid() != root.uid()
            || held.mode() & 0o7777 != 0o600
            || held.nlink() != 1
            || identity(&held) != self.file_identity
            || held.dev() != named.st_dev
            || held.ino() != named.st_ino
        {
            return Err(error());
        }
        Ok(())
    }

    /// Arm before any possible child effect. A partial write is never cleared.
    pub(crate) fn arm(&mut self) -> Result<(), ModelRuntimeFailure> {
        self.verify_identity()?;
        let error = || denied("model.native-lease.arm-failed", true);
        if self.marker.is_some() || self.file.metadata().map_err(|_| error())?.len() != 0 {
            return Err(error());
        }
        let mut marker = [0_u8; MARKER_BYTES];
        let prefix = b"AgentMage inference reservation v1\n";
        marker[..prefix.len()].copy_from_slice(prefix);
        let random = &mut marker[prefix.len()..];
        if getrandom(&mut *random, GetRandomFlags::empty()) != Ok(random.len()) {
            return Err(error());
        }
        self.marker = Some(marker);
        self.file
            .write_all_at(&marker, 0)
            .and_then(|()| self.file.sync_all())
            .and_then(|()| self.directory.sync_all())
            .map_err(|_| error())?;
        self.verify_marker()
    }

    fn verify_marker(&self) -> Result<(), ModelRuntimeFailure> {
        self.verify_identity()?;
        let error = || denied("model.native-lease.cleanup-uncertain", true);
        let expected = self.marker.as_ref().ok_or_else(error)?;
        let mut observed = [0_u8; MARKER_BYTES];
        if self.file.metadata().map_err(|_| error())?.len() != MARKER_BYTES as u64 {
            return Err(error());
        }
        self.file
            .read_exact_at(&mut observed, 0)
            .map_err(|_| error())?;
        if observed != *expected {
            return Err(error());
        }
        self.verify_identity()
    }

    /// Only the driver calls this after its held processes exited and exact files
    /// were removed. It grants no cleanup authority to a recovered/new owner.
    pub(crate) fn finish_owned_cleanup(&mut self) -> Result<(), ModelRuntimeFailure> {
        self.verify_marker()?;
        let error = || denied("model.native-lease.clear-failed", true);
        self.file
            .set_len(0)
            .and_then(|()| self.file.sync_all())
            .and_then(|()| self.directory.sync_all())
            .map_err(|_| error())?;
        self.verify_identity()?;
        if self.file.metadata().map_err(|_| error())?.len() != 0 {
            return Err(error());
        }
        self.marker = None;
        Ok(())
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
    fn unarmed_retention_holds_until_owner_process_exits() {
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
    #[ignore = "subprocess helper, exercised by unarmed_retention_holds_until_owner_process_exits"]
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
    fn armed_cleanup_releases_only_the_exact_marker_and_inode() {
        let root = Directory::new();
        let mut owner = NativeInferenceLease::acquire_in(&root.0).unwrap();
        let inode = fs::metadata(root.0.join(LOCK_NAME)).unwrap().ino();
        owner.arm().unwrap();
        assert_eq!(
            fs::metadata(root.0.join(LOCK_NAME)).unwrap().len(),
            MARKER_BYTES as u64
        );
        assert_eq!(
            NativeInferenceLease::acquire_in(&root.0)
                .err()
                .unwrap()
                .code,
            "model.native-lease.busy"
        );
        assert!(owner.arm().is_err());
        owner.finish_owned_cleanup().unwrap();
        drop(owner);
        let _next = NativeInferenceLease::acquire_in(&root.0).unwrap();
        assert_eq!(fs::metadata(root.0.join(LOCK_NAME)).unwrap().ino(), inode);
        assert_eq!(fs::metadata(root.0.join(LOCK_NAME)).unwrap().len(), 0);
    }

    #[test]
    fn changed_marker_is_preserved_and_refuses_later_admission() {
        let root = Directory::new();
        let mut owner = NativeInferenceLease::acquire_in(&root.0).unwrap();
        owner.arm().unwrap();
        let path = root.0.join(LOCK_NAME);
        let mut changed = fs::read(&path).unwrap();
        changed[0] ^= 1;
        fs::write(&path, &changed).unwrap();
        assert_eq!(
            owner.finish_owned_cleanup().unwrap_err().code,
            "model.native-lease.cleanup-uncertain"
        );
        assert_eq!(fs::read(&path).unwrap(), changed);
        drop(owner);
        assert_eq!(
            NativeInferenceLease::acquire_in(&root.0)
                .err()
                .unwrap()
                .code,
            "model.native-lease.cleanup-uncertain"
        );
    }

    #[test]
    fn parent_replacement_preserves_both_lock_objects() {
        let root = Directory::new();
        let mut owner = NativeInferenceLease::acquire_in(&root.0).unwrap();
        owner.arm().unwrap();
        let original = fs::read(root.0.join(LOCK_NAME)).unwrap();
        let moved = root.0.with_extension("retained");
        fs::rename(&root.0, &moved).unwrap();
        fs::create_dir(&root.0).unwrap();
        fs::set_permissions(&root.0, fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(root.0.join(LOCK_NAME), b"preserve replacement").unwrap();
        assert!(owner.finish_owned_cleanup().is_err());
        assert_eq!(
            fs::read(root.0.join(LOCK_NAME)).unwrap(),
            b"preserve replacement"
        );
        assert_eq!(fs::read(moved.join(LOCK_NAME)).unwrap(), original);
        drop(owner);
        fs::remove_dir_all(moved).unwrap();
    }

    #[test]
    fn armed_owner_death_keeps_future_hosts_closed() {
        let root = Directory::new();
        drop(NativeInferenceLease::acquire_in(&root.0).unwrap());
        let inode = fs::metadata(root.0.join(LOCK_NAME)).unwrap().ino();
        let result = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "native_inference_lease::tests::armed_exit_child",
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
        assert_eq!(fs::metadata(root.0.join(LOCK_NAME)).unwrap().ino(), inode);
        assert_eq!(
            fs::metadata(root.0.join(LOCK_NAME)).unwrap().len(),
            MARKER_BYTES as u64
        );
        assert_eq!(
            NativeInferenceLease::acquire_in(&root.0)
                .err()
                .unwrap()
                .code,
            "model.native-lease.cleanup-uncertain"
        );
    }

    #[test]
    #[ignore = "subprocess helper, exercised by armed_owner_death_keeps_future_hosts_closed"]
    fn armed_exit_child() {
        let root = std::env::var_os("AGENTMAGE_LEASE_TEST_ROOT").unwrap();
        let mut owner = NativeInferenceLease::acquire_in(Path::new(&root)).unwrap();
        owner.arm().unwrap();
        // No model is launched. Skip destructors to exercise process-death marker
        // persistence without pretending this proves any descendant cleanup.
        std::process::exit(0);
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
