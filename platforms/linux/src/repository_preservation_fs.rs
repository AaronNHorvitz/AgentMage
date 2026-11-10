// SPDX-License-Identifier: Apache-2.0
//! Descriptor-relative local preservation reads; never follow repository links.

use super::*;
use rustix::fs::{AtFlags, Dir, Stat, openat, readlinkat, statat};
use rustix::io::Errno;

fn failed() -> LinuxRepositoryError {
    error(LinuxRepositoryErrorKind::ObservationFailed)
}

const DIRECTORY: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);

// Separate bounds for retained relative names and content read by ONE traversal.
// Charging names before exclusions also bounds pending-directory path storage.
const MAX_TRAVERSAL_PATH_BYTES: usize = 64 * 1024 * 1024;

#[derive(Default)]
struct TraversalBudget {
    entries: usize,
    path_bytes: usize,
    content_bytes: u64,
}

impl TraversalBudget {
    fn entry(&mut self, path: &Path) -> Result<(), LinuxRepositoryError> {
        self.entries = self.entries.checked_add(1).ok_or_else(failed)?;
        self.path_bytes = self
            .path_bytes
            .checked_add(path.as_os_str().as_bytes().len())
            .ok_or_else(failed)?;
        if self.entries > MAX_METADATA_ENTRIES || self.path_bytes > MAX_TRAVERSAL_PATH_BYTES {
            return Err(failed());
        }
        Ok(())
    }

    fn content(&mut self, size: i64) -> Result<(), LinuxRepositoryError> {
        self.content_bytes = self
            .content_bytes
            .checked_add(u64::try_from(size).map_err(|_| failed())?)
            .filter(|n| *n <= MAX_HASHED_FILE_BYTES)
            .ok_or_else(failed)?;
        Ok(())
    }
}

pub(super) struct Files<'a> {
    checkout: OwnedFd,
    git: OwnedFd,
    scope: &'a LinuxRepositoryScope,
    cancellation: Option<&'a CancellationToken>,
}

impl<'a> Files<'a> {
    pub(super) fn new(
        scope: &'a LinuxRepositoryScope,
        cancellation: Option<&'a CancellationToken>,
    ) -> Result<Self, LinuxRepositoryError> {
        preservation::check(cancellation)?;
        let checkout = absolute_directory(&scope.checkout_root)?;
        let relative = scope
            .git_directory
            .strip_prefix(&scope.checkout_root)
            .map_err(|_| failed())?;
        let git = relative_directory(&checkout, relative)?;
        let files = Self {
            checkout,
            git,
            scope,
            cancellation,
        };
        files.revalidate()?;
        Ok(files)
    }

    fn check(&self) -> Result<(), LinuxRepositoryError> {
        preservation::check(self.cancellation)
    }

    pub(super) fn revalidate(&self) -> Result<(), LinuxRepositoryError> {
        self.check()?;
        let checkout = absolute_directory(&self.scope.checkout_root)?;
        let held = fstat(&self.checkout).map_err(|_| failed())?;
        let current = fstat(&checkout).map_err(|_| failed())?;
        if !same_object(&held, &current)
            || held.st_uid != getuid().as_raw()
            || held.st_mode & 0o022 != 0
            || identity(&self.scope.checkout_root, &held) != self.scope.repository_path_sha256
        {
            return Err(failed());
        }
        let relative = self
            .scope
            .git_directory
            .strip_prefix(&self.scope.checkout_root)
            .map_err(|_| failed())?;
        let git = relative_directory(&checkout, relative)?;
        let held_git = fstat(&self.git).map_err(|_| failed())?;
        let current_git = fstat(&git).map_err(|_| failed())?;
        if !same_object(&held_git, &current_git)
            || held_git.st_uid != getuid().as_raw()
            || held_git.st_mode & 0o022 != 0
        {
            return Err(failed());
        }
        self.check()
    }

    pub(super) fn common_identity(&self) -> Result<String, LinuxRepositoryError> {
        self.check()?;
        Ok(identity(
            &self.scope.git_directory,
            &fstat(&self.git).map_err(|_| failed())?,
        ))
    }

    pub(super) fn read_optional(
        &self,
        git: bool,
        name: &str,
    ) -> Result<Option<Vec<u8>>, LinuxRepositoryError> {
        self.check()?;
        let root = if git { &self.git } else { &self.checkout };
        let Some(entry) = entry(root, name)? else {
            return Ok(None);
        };
        let stat = fstat(&entry).map_err(|_| failed())?;
        let bytes = read_regular(root, name, &stat, || self.check(), || Ok(()))?;
        self.check()?;
        Ok(Some(bytes))
    }

    pub(super) fn hash_optional(
        &self,
        git: bool,
        name: &str,
    ) -> Result<String, LinuxRepositoryError> {
        Ok(hash(
            self.read_optional(git, name)?
                .as_deref()
                .unwrap_or(b"absent"),
        ))
    }

    pub(super) fn worktree(&self) -> Result<Vec<u8>, LinuxRepositoryError> {
        self.tree(&self.checkout, Path::new(""), Tree::Worktree, false)
    }

    pub(super) fn metadata(&self, name: &str) -> Result<Vec<u8>, LinuxRepositoryError> {
        self.tree(&self.git, Path::new(name), Tree::Metadata, false)
    }

    pub(super) fn reflogs(
        &self,
        name: &str,
        exclude_agentmage: bool,
    ) -> Result<Vec<u8>, LinuxRepositoryError> {
        self.tree(&self.git, Path::new(name), Tree::Reflogs, exclude_agentmage)
    }

    fn tree(
        &self,
        parent: &OwnedFd,
        path: &Path,
        kind: Tree,
        exclude: bool,
    ) -> Result<Vec<u8>, LinuxRepositoryError> {
        self.tree_with_budget(parent, path, kind, exclude, TraversalBudget::default())
    }

    // The private budget argument lets tests cross the real walk's boundaries
    // without constructing multi-gigabyte nested path fixtures. Production
    // always starts a fresh budget with the fixed limits above.
    fn tree_with_budget(
        &self,
        parent: &OwnedFd,
        path: &Path,
        kind: Tree,
        exclude: bool,
        mut budget: TraversalBudget,
    ) -> Result<Vec<u8>, LinuxRepositoryError> {
        self.check()?;
        let root = match relative_directory_optional(parent, path)? {
            Some(root) => root,
            None => return Ok(Vec::new()),
        };
        let root_stat = fstat(&root).map_err(|_| failed())?;
        let device = root_stat.st_dev;
        // Retain names and original observations, not an fd per pending child.
        // Reopen beneath the held root and verify the original object on visit.
        let mut pending = vec![(PathBuf::new(), root_stat)];
        let mut records = Vec::new();
        while let Some((relative, expected)) = pending.pop() {
            self.check()?;
            let directory = relative_directory(&root, &relative)?;
            if !same_snapshot(&expected, &fstat(&directory).map_err(|_| failed())?) {
                return Err(failed());
            }
            for item in Dir::read_from(&directory).map_err(|_| failed())? {
                self.check()?;
                let item = item.map_err(|_| failed())?;
                let name = item.file_name();
                if matches!(name.to_bytes(), b"." | b"..") {
                    continue;
                }
                let child = relative.join(std::ffi::OsStr::from_bytes(name.to_bytes()));
                budget.entry(&child)?;
                if (matches!(kind, Tree::Worktree) && child == Path::new(".git"))
                    || (exclude && child.to_string_lossy().contains("agentmage"))
                {
                    continue;
                }
                let held = entry(&directory, name)?.ok_or_else(failed)?;
                let stat = fstat(&held).map_err(|_| failed())?;
                if stat.st_dev != device {
                    return Err(failed());
                }
                let file_type = FileType::from_raw_mode(stat.st_mode);
                let mut record = Vec::new();
                record.extend_from_slice(child.as_os_str().as_bytes());
                if !matches!(kind, Tree::Reflogs) {
                    record.extend_from_slice(&stat.st_mode.to_be_bytes());
                    record.extend_from_slice(&(stat.st_size as u64).to_be_bytes());
                    record.extend_from_slice(&stat.st_mtime.to_be_bytes());
                }
                if file_type == FileType::Directory {
                    pending.push((child, stat));
                    if matches!(kind, Tree::Worktree) {
                        record.push(b'd');
                    }
                } else if !matches!(kind, Tree::Metadata) {
                    if file_type == FileType::RegularFile {
                        // Reflogs need the same aggregate read bound as source.
                        // Reject before opening/allocating this file's content.
                        budget.content(stat.st_size)?;
                        if matches!(kind, Tree::Worktree) {
                            record.push(b'f');
                        }
                        let bytes =
                            read_regular(&directory, name, &stat, || self.check(), || Ok(()))?;
                        record.extend_from_slice(hash(&bytes).as_bytes());
                    } else if matches!(kind, Tree::Worktree) && file_type == FileType::Symlink {
                        record.push(b'l');
                        let target = readlinkat(&held, "", Vec::new()).map_err(|_| failed())?;
                        record.extend_from_slice(target.to_bytes());
                    } else if matches!(kind, Tree::Worktree) {
                        record.push(b's');
                    } else {
                        return Err(failed());
                    }
                }
                self.check()?;
                verify_entry(&directory, name, &stat)?;
                if !matches!(kind, Tree::Reflogs) || file_type != FileType::Directory {
                    records.push(record);
                }
            }
        }
        self.check()?;
        records.sort();
        let result = records.concat();
        self.check()?;
        Ok(result)
    }

    pub(super) fn operations(&self) -> Result<Vec<u8>, LinuxRepositoryError> {
        let mut records = Vec::new();
        for name in [
            "MERGE_HEAD",
            "CHERRY_PICK_HEAD",
            "REVERT_HEAD",
            "BISECT_START",
            "rebase-apply",
            "rebase-merge",
            "sequencer",
            "index.lock",
            "config.lock",
            "packed-refs.lock",
            "shallow.lock",
            "gc.pid",
            "gc.log",
            "maintenance.lock",
        ] {
            self.check()?;
            if let Some(fd) = entry(&self.git, name)? {
                let stat = fstat(&fd).map_err(|_| failed())?;
                records.extend_from_slice(name.as_bytes());
                records.extend_from_slice(&stat.st_mode.to_be_bytes());
                records.extend_from_slice(&(stat.st_size as u64).to_be_bytes());
                verify_entry(&self.git, name, &stat)?;
            }
        }
        self.check()?;
        Ok(records)
    }

    pub(super) fn hooks(&self) -> Result<bool, LinuxRepositoryError> {
        self.check()?;
        let Some(directory) = relative_directory_optional(&self.git, Path::new("hooks"))? else {
            return Ok(false);
        };
        let mut count = 0;
        for item in Dir::read_from(&directory).map_err(|_| failed())? {
            self.check()?;
            let item = item.map_err(|_| failed())?;
            let name = item.file_name();
            if matches!(name.to_bytes(), b"." | b"..") {
                continue;
            }
            count += 1;
            if count > MAX_METADATA_ENTRIES {
                return Err(failed());
            }
            let fd = entry(&directory, name)?.ok_or_else(failed)?;
            let stat = fstat(&fd).map_err(|_| failed())?;
            verify_entry(&directory, name, &stat)?;
            let kind = FileType::from_raw_mode(stat.st_mode);
            if kind == FileType::Symlink
                || (kind == FileType::RegularFile
                    && stat.st_mode & 0o111 != 0
                    && !name.to_bytes().ends_with(b".sample"))
            {
                self.check()?;
                return Ok(true);
            }
        }
        self.check()?;
        Ok(false)
    }

    pub(super) fn exists(&self, name: &str) -> Result<bool, LinuxRepositoryError> {
        self.check()?;
        // Fixed internal paths only. Resolve every ancestor without links.
        let path = Path::new(name);
        let Some(parent) =
            relative_directory_optional(&self.git, path.parent().ok_or_else(failed)?)?
        else {
            return Ok(false);
        };
        let result = entry(&parent, path.file_name().ok_or_else(failed)?)?.is_some();
        self.check()?;
        Ok(result)
    }
}

#[derive(Clone, Copy)]
enum Tree {
    Worktree,
    Metadata,
    Reflogs,
}

fn absolute_directory(path: &Path) -> Result<OwnedFd, LinuxRepositoryError> {
    if !path.is_absolute() {
        return Err(failed());
    }
    let mut directory = open("/", DIRECTORY, Mode::empty()).map_err(|_| failed())?;
    for component in path.components().skip(1) {
        let Component::Normal(name) = component else {
            return Err(failed());
        };
        directory = openat(&directory, name, DIRECTORY, Mode::empty()).map_err(|_| failed())?;
    }
    Ok(directory)
}

fn relative_directory(parent: &OwnedFd, path: &Path) -> Result<OwnedFd, LinuxRepositoryError> {
    relative_directory_optional(parent, path)?.ok_or_else(failed)
}

fn relative_directory_optional(
    parent: &OwnedFd,
    path: &Path,
) -> Result<Option<OwnedFd>, LinuxRepositoryError> {
    let device = fstat(parent).map_err(|_| failed())?.st_dev;
    let mut directory = openat(parent, ".", DIRECTORY, Mode::empty()).map_err(|_| failed())?;
    for component in path.components() {
        let Component::Normal(name) = component else {
            return Err(failed());
        };
        match openat(&directory, name, DIRECTORY, Mode::empty()) {
            Ok(fd) => {
                if fstat(&fd).map_err(|_| failed())?.st_dev != device {
                    return Err(failed());
                }
                directory = fd;
            }
            Err(Errno::NOENT) => return Ok(None),
            Err(_) => return Err(failed()),
        }
    }
    Ok(Some(directory))
}

fn entry(
    parent: &OwnedFd,
    name: impl rustix::path::Arg,
) -> Result<Option<OwnedFd>, LinuxRepositoryError> {
    match openat(
        parent,
        name,
        OFlags::PATH | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(fd) => Ok(Some(fd)),
        Err(Errno::NOENT) => Ok(None),
        Err(_) => Err(failed()),
    }
}

fn same_object(a: &Stat, b: &Stat) -> bool {
    a.st_dev == b.st_dev && a.st_ino == b.st_ino && a.st_mode == b.st_mode
}

fn same_snapshot(a: &Stat, b: &Stat) -> bool {
    same_object(a, b)
        && a.st_uid == b.st_uid
        && a.st_gid == b.st_gid
        && a.st_nlink == b.st_nlink
        && a.st_size == b.st_size
        && a.st_mtime == b.st_mtime
        && a.st_mtime_nsec == b.st_mtime_nsec
        && a.st_ctime == b.st_ctime
        && a.st_ctime_nsec == b.st_ctime_nsec
}

fn verify_entry(
    parent: &OwnedFd,
    name: impl rustix::path::Arg,
    expected: &Stat,
) -> Result<(), LinuxRepositoryError> {
    let current = statat(parent, name, AtFlags::SYMLINK_NOFOLLOW).map_err(|_| failed())?;
    if same_snapshot(expected, &current) {
        Ok(())
    } else {
        Err(failed())
    }
}

fn identity(path: &Path, stat: &Stat) -> String {
    let mut bytes = path.as_os_str().as_bytes().to_vec();
    bytes.extend_from_slice(&stat.st_dev.to_be_bytes());
    bytes.extend_from_slice(&stat.st_ino.to_be_bytes());
    hash(&bytes)
}

// The hook is private: tests replace the name after metadata capture. Production
// always uses a no-op; no caller may substitute a reader or observed bytes.
fn read_regular(
    parent: &OwnedFd,
    name: impl rustix::path::Arg + Copy,
    expected: &Stat,
    mut check: impl FnMut() -> Result<(), LinuxRepositoryError>,
    before_open: impl FnOnce() -> Result<(), LinuxRepositoryError>,
) -> Result<Vec<u8>, LinuxRepositoryError> {
    check()?;
    if FileType::from_raw_mode(expected.st_mode) != FileType::RegularFile
        || expected.st_size < 0
        || expected.st_size as u64 > MAX_HASHED_FILE_BYTES
    {
        return Err(failed());
    }
    before_open()?;
    let fd = openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| failed())?;
    if !same_snapshot(expected, &fstat(&fd).map_err(|_| failed())?) {
        return Err(failed());
    }
    let mut bytes = Vec::new();
    let mut buffer = vec![0_u8; HASH_BUFFER_BYTES];
    while bytes.len() < expected.st_size as usize {
        check()?;
        let maximum = buffer.len().min(expected.st_size as usize - bytes.len());
        let count = pread(&fd, &mut buffer[..maximum], bytes.len() as u64).map_err(|_| failed())?;
        if count == 0 {
            return Err(failed());
        }
        bytes.extend_from_slice(&buffer[..count]);
        check()?;
    }
    if !same_snapshot(expected, &fstat(&fd).map_err(|_| failed())?) {
        return Err(failed());
    }
    verify_entry(parent, name, expected)?;
    check()?;
    Ok(bytes)
}

#[cfg(test)]
#[path = "repository_preservation_fs_tests.rs"]
mod tests;
