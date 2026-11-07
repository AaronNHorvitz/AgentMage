// SPDX-License-Identifier: Apache-2.0
//! Explicit, descriptor-held linked Git metadata for read-only development use.

use std::fs::File;
use std::io::Read;
use std::os::fd::OwnedFd;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use rustix::fs::{FileType, Mode, OFlags, Stat, fstat, open, openat};
use rustix::process::getuid;
use sha2::{Digest, Sha256};

use crate::repository_safety::{LinuxRepositoryError, LinuxRepositoryErrorKind};

const MAX_POINTER_BYTES: u64 = 4096;
type Result<T> = std::result::Result<T, LinuxRepositoryError>;

#[derive(Debug)]
struct HeldDirectory {
    path: PathBuf,
    fd: OwnedFd,
}

/// A checked linked worktree and its two exact Git metadata roots. Construction
/// does not adopt repository ownership or grant a read, write or command. The
/// development activation must separately bind this identity into its marker.
#[derive(Clone, Debug)]
pub struct LinuxLinkedRepository {
    root: Arc<HeldDirectory>,
    git: Arc<HeldDirectory>,
    common: Arc<HeldDirectory>,
    // Retain pointer inodes too: unlink/recreate cannot recycle a consumed
    // identity while this process holds the activation.
    _pointer_files: Arc<Vec<OwnedFd>>,
    identity_sha256: String,
}

impl PartialEq for LinuxLinkedRepository {
    fn eq(&self, other: &Self) -> bool {
        self.identity_sha256 == other.identity_sha256
    }
}
impl Eq for LinuxLinkedRepository {}

impl LinuxLinkedRepository {
    /// Checks canonical current-user-owned roots, single-link bounded pointer
    /// files, the exact `worktrees/<name>` layout, common-dir link and backlink.
    /// Relative gitdir pointers, aliases, nested links and writable metadata
    /// roots are refused. Checks the workspace pointer against both explicitly
    /// selected metadata paths before opening external metadata. No Git process
    /// is started.
    pub fn verify(worktree: &Path, expected_git: &Path, expected_common: &Path) -> Result<Self> {
        let root = directory(worktree)?;
        let (pointer, pointer_stat, pointer_fd) = regular(&root, ".git")?;
        let git_path = line(&pointer)?
            .strip_prefix("gitdir: ")
            .ok_or_else(denied)?;
        if Path::new(git_path) != expected_git {
            return Err(denied());
        }
        let worktrees = expected_git.parent().ok_or_else(denied)?;
        if worktrees.file_name() != Some(std::ffi::OsStr::new("worktrees")) {
            return Err(denied());
        }
        if worktrees.parent() != Some(expected_common) {
            return Err(denied());
        }
        let common = directory(expected_common)?;
        let git = directory(expected_git)?;
        let _worktrees = directory(worktrees)?;
        if root.path.starts_with(&common.path) || common.path.starts_with(&root.path) {
            return Err(denied());
        }
        let (common_pointer, common_stat, common_fd) = regular(&git, "commondir")?;
        if common_pointer != b"../..\n" {
            return Err(denied());
        }
        let (backlink, backlink_stat, backlink_fd) = regular(&git, "gitdir")?;
        if Path::new(line(&backlink)?) != root.path.join(".git") {
            return Err(denied());
        }
        // HEAD contents may change during a run. Its kind and ownership must
        // still be safe; it is never followed as a filesystem pointer here.
        regular(&git, "HEAD")?;
        let mut digest = Sha256::new();
        digest.update(b"agentmage-linked-repository-read-v1\0");
        for dir in [&root, &git, &common] {
            let stat = fstat(&dir.fd).map_err(|_| denied())?;
            let path = dir.path.to_str().ok_or_else(denied)?.as_bytes();
            digest.update((path.len() as u64).to_be_bytes());
            digest.update(path);
            bind_stat(&mut digest, &stat);
        }
        for (bytes, stat) in [
            (&pointer, pointer_stat),
            (&common_pointer, common_stat),
            (&backlink, backlink_stat),
        ] {
            bind_stat(&mut digest, &stat);
            digest.update((bytes.len() as u64).to_be_bytes());
            digest.update(bytes);
        }
        Ok(Self {
            root: Arc::new(root),
            git: Arc::new(git),
            common: Arc::new(common),
            _pointer_files: Arc::new(vec![pointer_fd, common_fd, backlink_fd]),
            identity_sha256: digest
                .finalize()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
        })
    }

    /// Rechecks the exact pointer bytes and object identities; a recreated tree
    /// or metadata directory cannot inherit this binding.
    pub fn revalidate(&self) -> Result<()> {
        if Self::verify(&self.root.path, &self.git.path, &self.common.path)? != *self {
            return Err(denied());
        }
        Ok(())
    }

    /// Path, directory-object and pointer-file identity used by explicit setup.
    #[must_use]
    pub fn identity_sha256(&self) -> &str {
        &self.identity_sha256
    }

    /// Exact workspace to which these metadata roots belong.
    #[must_use]
    pub fn worktree_root(&self) -> &Path {
        &self.root.path
    }

    /// Exact per-worktree metadata root to disclose during activation setup.
    #[must_use]
    pub fn git_directory(&self) -> &Path {
        &self.git.path
    }

    /// Exact shared metadata root to disclose during activation setup.
    #[must_use]
    pub fn common_directory(&self) -> &Path {
        &self.common.path
    }

    pub(crate) fn git_fd(&self) -> &OwnedFd {
        &self.git.fd
    }

    pub(crate) fn common_fd(&self) -> &OwnedFd {
        &self.common.fd
    }

    pub(crate) fn matches_root(&self, fd: &OwnedFd) -> bool {
        match (fstat(fd), fstat(&self.root.fd)) {
            (Ok(a), Ok(b)) => a.st_dev == b.st_dev && a.st_ino == b.st_ino,
            _ => false,
        }
    }
}

fn denied() -> LinuxRepositoryError {
    LinuxRepositoryError::from_kind(LinuxRepositoryErrorKind::InvalidPath)
}

fn line(bytes: &[u8]) -> Result<&str> {
    let value = std::str::from_utf8(bytes).map_err(|_| denied())?;
    let value = value.strip_suffix('\n').ok_or_else(denied)?;
    if value.is_empty() || value.chars().any(char::is_control) {
        return Err(denied());
    }
    Ok(value)
}

fn directory(path: &Path) -> Result<HeldDirectory> {
    if !path.is_absolute()
        || path.as_os_str().len() > 4096
        || path
            .to_str()
            .is_none_or(|p| p.chars().any(char::is_control))
    {
        return Err(denied());
    }
    let flags = OFlags::RDONLY | OFlags::CLOEXEC | OFlags::DIRECTORY | OFlags::NOFOLLOW;
    let mut fd = open("/", flags, Mode::empty()).map_err(|_| denied())?;
    for component in path.components().skip(1) {
        let Component::Normal(name) = component else {
            return Err(denied());
        };
        fd = openat(&fd, name, flags, Mode::empty()).map_err(|_| denied())?;
    }
    if path.canonicalize().map_err(|_| denied())? != path {
        return Err(denied());
    }
    let stat = fstat(&fd).map_err(|_| denied())?;
    if stat.st_uid != getuid().as_raw() || stat.st_mode & 0o022 != 0 {
        return Err(denied());
    }
    Ok(HeldDirectory {
        path: path.to_owned(),
        fd,
    })
}

fn regular(parent: &HeldDirectory, name: &str) -> Result<(Vec<u8>, Stat, OwnedFd)> {
    let fd = openat(
        &parent.fd,
        name,
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
        Mode::empty(),
    )
    .map_err(|_| denied())?;
    let stat = fstat(&fd).map_err(|_| denied())?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile
        || stat.st_uid != getuid().as_raw()
        || stat.st_nlink != 1
        || stat.st_mode & 0o022 != 0
        || stat.st_size <= 0
        || stat.st_size as u64 > MAX_POINTER_BYTES
    {
        return Err(denied());
    }
    let mut file = File::from(fd);
    let mut bytes = Vec::new();
    file.by_ref()
        .take(MAX_POINTER_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| denied())?;
    let after = fstat(&file).map_err(|_| denied())?;
    if bytes.len() as u64 > MAX_POINTER_BYTES
        || bytes.len() as i64 != stat.st_size
        || after.st_dev != stat.st_dev
        || after.st_ino != stat.st_ino
        || after.st_mode != stat.st_mode
        || after.st_nlink != stat.st_nlink
        || after.st_uid != stat.st_uid
        || after.st_size != stat.st_size
        || after.st_mtime != stat.st_mtime
        || after.st_mtime_nsec != stat.st_mtime_nsec
        || after.st_ctime != stat.st_ctime
        || after.st_ctime_nsec != stat.st_ctime_nsec
    {
        return Err(denied());
    }
    Ok((bytes, stat, file.into()))
}

fn bind_stat(digest: &mut Sha256, stat: &Stat) {
    digest.update(stat.st_dev.to_be_bytes());
    digest.update(stat.st_ino.to_be_bytes());
    digest.update(stat.st_uid.to_be_bytes());
    digest.update(stat.st_mode.to_be_bytes());
}

#[cfg(test)]
#[path = "linked_repository_tests.rs"]
mod tests;
