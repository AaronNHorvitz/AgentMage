//! Fixed private runtime files retained by descriptor until owned cleanup.

use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::{FileExt, FileTypeExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use agentmage_kernel_contracts::ModelRuntimeFailure;
use rustix::fs::{AtFlags, Mode, OFlags, open, openat, statat, unlinkat};
use rustix::rand::{GetRandomFlags, getrandom};
use zeroize::{Zeroize, Zeroizing};

use super::{API_KEY_FILE_NAME, FileSnapshot, failure};
const SOCKET_NAME: &str = "llama-server.sock";

struct OwnedFile {
    descriptor: File,
    snapshot: FileSnapshot,
}

pub(super) struct RuntimeFiles {
    root: PathBuf,
    directory: File,
    directory_identity: (u64, u64, u32, u32, u32),
    key: Option<OwnedFile>,
    socket: Option<OwnedFile>,
    key_value: Zeroizing<String>,
}

fn directory_identity(metadata: &fs::Metadata) -> (u64, u64, u32, u32, u32) {
    (
        metadata.dev(),
        metadata.ino(),
        metadata.mode(),
        metadata.uid(),
        metadata.gid(),
    )
}

fn invalid() -> ModelRuntimeFailure {
    failure("model.llama-driver.private-file-identity", true)
}

impl RuntimeFiles {
    pub(super) fn create(socket: &Path) -> Result<Self, ModelRuntimeFailure> {
        let root = socket.parent().ok_or_else(invalid)?;
        if !socket.is_absolute()
            || socket.file_name().and_then(|s| s.to_str()) != Some(SOCKET_NAME)
            || root.canonicalize().ok().as_deref() != Some(root)
        {
            return Err(invalid());
        }
        let directory = File::from(
            open(
                root,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|_| invalid())?,
        );
        let metadata = directory.metadata().map_err(|_| invalid())?;
        if !metadata.is_dir()
            || metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.mode() & 0o7777 != 0o700
        {
            return Err(invalid());
        }
        let mut files = Self {
            root: root.to_path_buf(),
            directory,
            directory_identity: directory_identity(&metadata),
            key: None,
            socket: None,
            key_value: Zeroizing::new(String::new()),
        };
        files.verify_root()?;
        files.require_absent(API_KEY_FILE_NAME)?;
        files.require_absent(SOCKET_NAME)?;
        let mut random = [0_u8; 32];
        if getrandom(&mut random, GetRandomFlags::empty()) != Ok(random.len()) {
            random.zeroize();
            return Err(failure("model.llama-driver.api-key-random-failed", true));
        }
        files.key_value = Zeroizing::new(random.iter().map(|byte| format!("{byte:02x}")).collect());
        random.zeroize();
        let mut descriptor = File::from(
            openat(
                &files.directory,
                API_KEY_FILE_NAME,
                OFlags::RDWR
                    | OFlags::CREATE
                    | OFlags::EXCL
                    | OFlags::NOFOLLOW
                    | OFlags::CLOEXEC
                    | OFlags::NONBLOCK,
                Mode::from_raw_mode(0o600),
            )
            .map_err(|_| failure("model.llama-driver.api-key-create-failed", false))?,
        );
        descriptor
            .write_all(files.key_value.as_bytes())
            .and_then(|()| descriptor.write_all(b"\n"))
            .and_then(|()| descriptor.sync_all())
            .and_then(|()| files.directory.sync_all())
            .map_err(|_| failure("model.llama-driver.api-key-write-failed", true))?;
        let metadata = descriptor.metadata().map_err(|_| invalid())?;
        if !metadata.is_file()
            || metadata.uid() != files.directory_identity.3
            || metadata.mode() & 0o7777 != 0o600
            || metadata.nlink() != 1
            || metadata.len() != 65
        {
            return Err(invalid());
        }
        files.key = Some(OwnedFile {
            descriptor,
            snapshot: FileSnapshot::from_metadata(&metadata),
        });
        files.verify_key()?;
        Ok(files)
    }

    pub(super) fn key(&self) -> &str {
        self.key_value.as_str()
    }

    pub(super) fn verify_connection(&self) -> Result<(), ModelRuntimeFailure> {
        self.verify_key()?;
        if let Some(socket) = &self.socket
            && !self.verify_file(SOCKET_NAME, socket)?
        {
            return Err(invalid());
        }
        Ok(())
    }

    fn verify_root(&self) -> Result<(), ModelRuntimeFailure> {
        let held = self.directory.metadata().map_err(|_| invalid())?;
        let named = fs::symlink_metadata(&self.root).map_err(|_| invalid())?;
        if directory_identity(&held) != self.directory_identity
            || directory_identity(&named) != self.directory_identity
            || self.root.canonicalize().ok().as_deref() != Some(self.root.as_path())
        {
            return Err(invalid());
        }
        Ok(())
    }

    fn require_absent(&self, name: &str) -> Result<(), ModelRuntimeFailure> {
        match statat(&self.directory, name, AtFlags::SYMLINK_NOFOLLOW) {
            Err(rustix::io::Errno::NOENT) => Ok(()),
            _ => Err(invalid()),
        }
    }

    fn verify_file(&self, name: &str, file: &OwnedFile) -> Result<bool, ModelRuntimeFailure> {
        self.verify_root()?;
        let held = file.descriptor.metadata().map_err(|_| invalid())?;
        if held.nlink() == 0 {
            // Only this held, now-unlinked inode can justify an already absent
            // leaf. This does not establish process exit or authorize a new file.
            let mut removed = file.snapshot;
            removed.links = 0;
            removed.changed_seconds = held.ctime();
            removed.changed_nanoseconds = held.ctime_nsec();
            if FileSnapshot::from_metadata(&held) != removed {
                return Err(invalid());
            }
            self.require_absent(name)?;
            return Ok(false);
        }
        let named = File::from(
            openat(
                &self.directory,
                name,
                OFlags::PATH | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|_| invalid())?,
        )
        .metadata()
        .map_err(|_| invalid())?;
        if FileSnapshot::from_metadata(&held) != file.snapshot
            || FileSnapshot::from_metadata(&named) != file.snapshot
        {
            return Err(invalid());
        }
        Ok(true)
    }

    pub(super) fn verify_key(&self) -> Result<(), ModelRuntimeFailure> {
        self.check_key(false)
    }

    fn check_key(&self, allow_removed: bool) -> Result<(), ModelRuntimeFailure> {
        let file = self.key.as_ref().ok_or_else(invalid)?;
        if !self.verify_file(API_KEY_FILE_NAME, file)? && !allow_removed {
            return Err(invalid());
        }
        let mut bytes = Zeroizing::new([0_u8; 65]);
        file.descriptor
            .read_exact_at(bytes.as_mut(), 0)
            .map_err(|_| invalid())?;
        if bytes[..64] != *self.key_value.as_bytes() || bytes[64] != b'\n' {
            return Err(invalid());
        }
        if !self.verify_file(API_KEY_FILE_NAME, file)? && !allow_removed {
            return Err(invalid());
        }
        Ok(())
    }

    pub(super) fn capture_socket(&mut self) -> Result<(), ModelRuntimeFailure> {
        if self.socket.is_some() {
            return Err(invalid());
        }
        self.verify_key()?;
        let descriptor = File::from(
            openat(
                &self.directory,
                SOCKET_NAME,
                OFlags::PATH | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|_| invalid())?,
        );
        let before = descriptor.metadata().map_err(|_| invalid())?;
        if !before.file_type().is_socket()
            || before.uid() != self.directory_identity.3
            || before.nlink() != 1
            || before.mode() & 0o7000 != 0
        {
            return Err(invalid());
        }
        let original = OwnedFile {
            descriptor,
            snapshot: FileSnapshot::from_metadata(&before),
        };
        if !self.verify_file(SOCKET_NAME, &original)? {
            return Err(invalid());
        }
        // The held inode and private parent are rechecked around the existing
        // mode publication. This is not atomic against a hostile same-UID writer.
        fs::set_permissions(
            self.root.join(SOCKET_NAME),
            fs::Permissions::from_mode(0o600),
        )
        .map_err(|_| failure("model.llama-driver.socket-mode-failed", true))?;
        let after = original.descriptor.metadata().map_err(|_| invalid())?;
        if after.dev() != before.dev()
            || after.ino() != before.ino()
            || !after.file_type().is_socket()
            || after.uid() != before.uid()
            || after.gid() != before.gid()
            || after.nlink() != 1
            || after.mode() & 0o7777 != 0o600
        {
            return Err(invalid());
        }
        let owned = OwnedFile {
            descriptor: original.descriptor,
            snapshot: FileSnapshot::from_metadata(&after),
        };
        if !self.verify_file(SOCKET_NAME, &owned)? {
            return Err(invalid());
        }
        self.verify_key()?;
        self.socket = Some(owned);
        Ok(())
    }

    fn remove_owned(&self, name: &str, file: &OwnedFile) -> Result<(), ModelRuntimeFailure> {
        if self.verify_file(name, file)? {
            unlinkat(&self.directory, name, AtFlags::empty()).map_err(|_| invalid())?;
        }
        if self.verify_file(name, file)? {
            return Err(invalid());
        }
        Ok(())
    }

    /// No unobserved path is ever adopted here. All identities are checked before
    /// the first removal; each removal rechecks against its retained descriptor.
    pub(super) fn cleanup(&mut self) -> Result<(), ModelRuntimeFailure> {
        self.check_key(true)?;
        self.verify_file(SOCKET_NAME, self.socket.as_ref().ok_or_else(invalid)?)?;
        self.remove_owned(API_KEY_FILE_NAME, self.key.as_ref().ok_or_else(invalid)?)?;
        self.key = None;
        self.remove_owned(SOCKET_NAME, self.socket.as_ref().ok_or_else(invalid)?)?;
        self.socket = None;
        self.verify_root()?;
        self.require_absent(API_KEY_FILE_NAME)?;
        self.require_absent(SOCKET_NAME)?;
        self.directory.sync_all().map_err(|_| invalid())?;
        self.key_value.zeroize();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::os::unix::fs::symlink;
    use std::os::unix::net::UnixListener;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "agentmage-owned-model-files-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            Self(path.canonicalize().unwrap())
        }
        fn files(&self) -> RuntimeFiles {
            let socket = self.0.join(SOCKET_NAME);
            let mut files = RuntimeFiles::create(&socket).unwrap();
            let listener = UnixListener::bind(&socket).unwrap();
            files.capture_socket().unwrap();
            drop(listener);
            files
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    fn snapshot(root: &Path) -> BTreeMap<String, (FileSnapshot, Vec<u8>)> {
        fs::read_dir(root)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                let metadata = fs::symlink_metadata(entry.path()).unwrap();
                let contents = if metadata.is_file() {
                    fs::read(entry.path()).unwrap()
                } else if metadata.file_type().is_symlink() {
                    fs::read_link(entry.path())
                        .unwrap()
                        .as_os_str()
                        .as_encoded_bytes()
                        .to_vec()
                } else {
                    Vec::new()
                };
                (
                    entry.file_name().into_string().unwrap(),
                    (FileSnapshot::from_metadata(&metadata), contents),
                )
            })
            .collect()
    }

    #[test]
    fn cleanup_removes_only_original_objects_and_accepts_observed_unlink() {
        for already_unlinked in [false, true] {
            let directory = Directory::new();
            let mut files = directory.files();
            if already_unlinked {
                fs::remove_file(directory.0.join(SOCKET_NAME)).unwrap();
                fs::remove_file(directory.0.join(API_KEY_FILE_NAME)).unwrap();
                assert!(files.verify_connection().is_err());
            }
            files.cleanup().unwrap();
            assert!(snapshot(&directory.0).is_empty());
            assert!(files.key().is_empty());
            assert!(files.cleanup().is_err());
        }
    }

    #[test]
    fn changed_objects_refuse_before_removing_any_file() {
        for change in 0..9 {
            let directory = Directory::new();
            let mut files = directory.files();
            let key = directory.0.join(API_KEY_FILE_NAME);
            let socket = directory.0.join(SOCKET_NAME);
            match change {
                0 => fs::write(&key, format!("{}\n", "x".repeat(64))).unwrap(),
                1 => {
                    fs::remove_file(&key).unwrap();
                    fs::write(&key, b"preserve replacement").unwrap();
                }
                2 => {
                    fs::remove_file(&key).unwrap();
                    symlink("sentinel", &key).unwrap();
                }
                3 => fs::hard_link(&key, directory.0.join("alias")).unwrap(),
                4 => {
                    fs::remove_file(&socket).unwrap();
                    fs::write(&socket, b"preserve replacement").unwrap();
                }
                5 => {
                    fs::remove_file(&socket).unwrap();
                    symlink("sentinel", &socket).unwrap();
                }
                6 => fs::hard_link(&socket, directory.0.join("alias")).unwrap(),
                7 => fs::set_permissions(&key, fs::Permissions::from_mode(0o644)).unwrap(),
                8 => fs::set_permissions(&directory.0, fs::Permissions::from_mode(0o755)).unwrap(),
                _ => unreachable!(),
            }
            fs::write(directory.0.join("sentinel"), b"pre-existing work").unwrap();
            let before = snapshot(&directory.0);
            assert!(files.verify_connection().is_err(), "change {change}");
            assert!(files.cleanup().is_err(), "change {change}");
            assert_eq!(snapshot(&directory.0), before, "change {change}");
        }
    }

    #[test]
    fn replacement_parent_is_preserved_alongside_held_original() {
        let directory = Directory::new();
        let mut files = directory.files();
        let moved = directory.0.with_extension("original");
        fs::rename(&directory.0, &moved).unwrap();
        let retained = Directory(moved);
        fs::create_dir(&directory.0).unwrap();
        fs::set_permissions(&directory.0, fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(directory.0.join(API_KEY_FILE_NAME), b"replacement key").unwrap();
        fs::write(directory.0.join(SOCKET_NAME), b"replacement socket").unwrap();
        let original = snapshot(&retained.0);
        let replacement = snapshot(&directory.0);
        assert!(files.cleanup().is_err());
        assert_eq!(snapshot(&retained.0), original);
        assert_eq!(snapshot(&directory.0), replacement);
    }

    #[test]
    fn creation_and_unobserved_socket_refusals_preserve_existing_work() {
        let directory = Directory::new();
        fs::write(directory.0.join(API_KEY_FILE_NAME), b"pre-existing key").unwrap();
        let before = snapshot(&directory.0);
        assert!(RuntimeFiles::create(&directory.0.join(SOCKET_NAME)).is_err());
        assert_eq!(snapshot(&directory.0), before);
        fs::remove_file(directory.0.join(API_KEY_FILE_NAME)).unwrap();
        let mut files = RuntimeFiles::create(&directory.0.join(SOCKET_NAME)).unwrap();
        fs::write(directory.0.join(SOCKET_NAME), b"not a socket").unwrap();
        let before = snapshot(&directory.0);
        assert!(files.capture_socket().is_err());
        assert!(files.cleanup().is_err());
        assert_eq!(snapshot(&directory.0), before);
    }
}
