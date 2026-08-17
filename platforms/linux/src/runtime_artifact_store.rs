//! Descriptor-relative private storage for runtime artifact payloads.

use std::fmt;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};

use agentmage_kernel_contracts::RuntimeArtifactId;
use agentmage_kernel_engine::runtime_artifact::{
    MAX_RUNTIME_ARTIFACT_BYTES, RuntimeArtifactPayloadError, RuntimeArtifactPayloadInventoryEntry,
    RuntimeArtifactPayloadObservation, RuntimeArtifactPayloadPlacement,
    RuntimeArtifactPayloadStore,
};
use rustix::fd::OwnedFd;
use rustix::fs::{
    AtFlags, Dir, FileType, Mode, OFlags, RenameFlags, fchmod, fstat, fsync, mkdirat, openat,
    renameat_with, unlinkat,
};
use rustix::io::Errno;
use rustix::process::getuid;
use sha2::{Digest, Sha256};

use crate::LinuxStrictLocalRoot;

const STORE_DIRECTORY: &str = ".agentmage-runtime-payloads-v1";
const STAGING_DIRECTORY: &str = "staging";
const OBJECT_DIRECTORY: &str = "objects";
const QUARANTINE_DIRECTORY: &str = "quarantine";
const STAGING_PREFIX: &str = "stage-";
const HASH_BUFFER_BYTES: usize = 64 * 1024;
const MAX_ACTIVE_OBJECTS: usize = 65_536;
const MAX_STAGING_OBJECTS: usize = 4_096;
const MAX_QUARANTINE_ATTEMPTS: usize = 1_024;

/// One opaque, one-use staged Linux payload handle.
pub struct LinuxRuntimeArtifactStaged {
    name: String,
    observation: RuntimeArtifactPayloadObservation,
    snapshot: FileSnapshot,
}

impl fmt::Debug for LinuxRuntimeArtifactStaged {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxRuntimeArtifactStaged")
            .field("byte_size", &self.observation.byte_size)
            .finish_non_exhaustive()
    }
}

/// Linux private payload store rooted entirely in held directory descriptors.
pub struct LinuxRuntimeArtifactPayloadStore {
    root: OwnedFd,
    root_snapshot: DirectorySnapshot,
    store: OwnedFd,
    store_snapshot: DirectorySnapshot,
    staging: OwnedFd,
    staging_snapshot: DirectorySnapshot,
    objects: OwnedFd,
    objects_snapshot: DirectorySnapshot,
    quarantine: OwnedFd,
    quarantine_snapshot: DirectorySnapshot,
    quarantine_sequence: u64,
}

impl fmt::Debug for LinuxRuntimeArtifactPayloadStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxRuntimeArtifactPayloadStore")
            .field("namespace", &STORE_DIRECTORY)
            .finish_non_exhaustive()
    }
}

impl LinuxRuntimeArtifactPayloadStore {
    pub(crate) fn open(root: &LinuxStrictLocalRoot) -> Result<Self, RuntimeArtifactPayloadError> {
        root.revalidate()
            .map_err(|_| RuntimeArtifactPayloadError::UnsafeRoot)?;
        let root_descriptor = root
            .duplicate_io_descriptor()
            .map_err(|_| RuntimeArtifactPayloadError::UnsafeRoot)?;
        let root_snapshot = directory_snapshot(&root_descriptor, None)?;
        let (store, store_snapshot) =
            open_or_create_directory(&root_descriptor, STORE_DIRECTORY, root_snapshot.device)?;
        let (staging, staging_snapshot) =
            open_or_create_directory(&store, STAGING_DIRECTORY, root_snapshot.device)?;
        let (objects, objects_snapshot) =
            open_or_create_directory(&store, OBJECT_DIRECTORY, root_snapshot.device)?;
        let (quarantine, quarantine_snapshot) =
            open_or_create_directory(&store, QUARANTINE_DIRECTORY, root_snapshot.device)?;
        root.revalidate()
            .map_err(|_| RuntimeArtifactPayloadError::UnsafeRoot)?;
        Ok(Self {
            root: root_descriptor,
            root_snapshot,
            store,
            store_snapshot,
            staging,
            staging_snapshot,
            objects,
            objects_snapshot,
            quarantine,
            quarantine_snapshot,
            quarantine_sequence: 0,
        })
    }

    fn revalidate(&self) -> Result<(), RuntimeArtifactPayloadError> {
        verify_directory(&self.root, &self.root_snapshot, None)?;
        verify_directory(
            &self.store,
            &self.store_snapshot,
            Some(self.root_snapshot.device),
        )?;
        verify_directory(
            &self.staging,
            &self.staging_snapshot,
            Some(self.root_snapshot.device),
        )?;
        verify_directory(
            &self.objects,
            &self.objects_snapshot,
            Some(self.root_snapshot.device),
        )?;
        verify_directory(
            &self.quarantine,
            &self.quarantine_snapshot,
            Some(self.root_snapshot.device),
        )
    }

    fn verify_staged(
        &self,
        staged: &LinuxRuntimeArtifactStaged,
        expected: &RuntimeArtifactPayloadObservation,
    ) -> Result<(), RuntimeArtifactPayloadError> {
        if staged.observation != *expected || !valid_staging_name(&staged.name) {
            return Err(RuntimeArtifactPayloadError::Invalid);
        }
        let (_, snapshot, observed) =
            observe_named_payload(&self.staging, &staged.name, Some(expected.byte_size), false)?;
        if !snapshot.same_object(&staged.snapshot) || observed != *expected {
            return Err(RuntimeArtifactPayloadError::Conflict);
        }
        Ok(())
    }

    fn verify_active(
        &self,
        expected: &RuntimeArtifactPayloadObservation,
    ) -> Result<FileSnapshot, RuntimeArtifactPayloadError> {
        validate_observation(expected)?;
        self.revalidate()?;
        let (_, snapshot, observed) = observe_named_payload(
            &self.objects,
            &expected.payload_sha256,
            Some(expected.byte_size),
            false,
        )?;
        self.revalidate()?;
        if observed != *expected {
            return Err(RuntimeArtifactPayloadError::Corrupt);
        }
        Ok(snapshot)
    }

    fn isolate_active(
        &mut self,
        payload_sha256: &str,
        expected_snapshot: &FileSnapshot,
        purpose: &str,
    ) -> Result<String, RuntimeArtifactPayloadError> {
        for _ in 0..MAX_QUARANTINE_ATTEMPTS {
            self.quarantine_sequence = self
                .quarantine_sequence
                .checked_add(1)
                .ok_or(RuntimeArtifactPayloadError::ResourceLimit)?;
            let destination = format!(
                "{purpose}-{payload_sha256}-{:016x}",
                self.quarantine_sequence
            );
            match renameat_with(
                &self.objects,
                payload_sha256,
                &self.quarantine,
                destination.as_str(),
                RenameFlags::NOREPLACE,
            ) {
                Ok(()) => {
                    fsync(&self.objects).map_err(|_| RuntimeArtifactPayloadError::Durability)?;
                    fsync(&self.quarantine).map_err(|_| RuntimeArtifactPayloadError::Durability)?;
                    let descriptor = open_regular(&self.quarantine, &destination)?;
                    let retained = file_snapshot(&self.quarantine, &descriptor)?;
                    if !retained.same_object(expected_snapshot) {
                        return Err(RuntimeArtifactPayloadError::Conflict);
                    }
                    return Ok(destination);
                }
                Err(Errno::EXIST) => continue,
                Err(Errno::NOENT) => return Err(RuntimeArtifactPayloadError::Missing),
                Err(_) => return Err(RuntimeArtifactPayloadError::Durability),
            }
        }
        Err(RuntimeArtifactPayloadError::ResourceLimit)
    }
}

impl RuntimeArtifactPayloadStore for LinuxRuntimeArtifactPayloadStore {
    type Staged = LinuxRuntimeArtifactStaged;

    fn stage(
        &mut self,
        artifact_id: &RuntimeArtifactId,
        source: &mut dyn Read,
        maximum_bytes: u64,
    ) -> Result<(Self::Staged, RuntimeArtifactPayloadObservation), RuntimeArtifactPayloadError>
    {
        if maximum_bytes == 0 || maximum_bytes > MAX_RUNTIME_ARTIFACT_BYTES {
            return Err(RuntimeArtifactPayloadError::ResourceLimit);
        }
        self.revalidate()?;
        let name = staging_name(artifact_id);
        let descriptor = openat(
            &self.staging,
            name.as_str(),
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::from_raw_mode(0o600),
        )
        .map_err(|error| {
            if error == Errno::EXIST {
                RuntimeArtifactPayloadError::Conflict
            } else {
                RuntimeArtifactPayloadError::Durability
            }
        })?;
        if fchmod(&descriptor, Mode::from_raw_mode(0o600)).is_err() {
            return cleanup_failed_stage(
                &self.staging,
                &name,
                RuntimeArtifactPayloadError::Durability,
            );
        }
        let mut file = File::from(descriptor);
        let mut digest = Sha256::new();
        let mut byte_size = 0_u64;
        let mut buffer = [0_u8; HASH_BUFFER_BYTES];
        loop {
            let count = match source.read(&mut buffer) {
                Ok(count) => count,
                Err(_) => {
                    drop(file);
                    return cleanup_failed_stage(
                        &self.staging,
                        &name,
                        RuntimeArtifactPayloadError::Durability,
                    );
                }
            };
            if count == 0 {
                break;
            }
            byte_size = match byte_size.checked_add(count as u64) {
                Some(total) if total <= maximum_bytes => total,
                _ => {
                    drop(file);
                    return cleanup_failed_stage(
                        &self.staging,
                        &name,
                        RuntimeArtifactPayloadError::ResourceLimit,
                    );
                }
            };
            if file.write_all(&buffer[..count]).is_err() {
                drop(file);
                return cleanup_failed_stage(
                    &self.staging,
                    &name,
                    RuntimeArtifactPayloadError::Durability,
                );
            }
            digest.update(&buffer[..count]);
        }
        if byte_size == 0 {
            drop(file);
            return cleanup_failed_stage(
                &self.staging,
                &name,
                RuntimeArtifactPayloadError::Invalid,
            );
        }
        if file.sync_all().is_err() {
            drop(file);
            return cleanup_failed_stage(
                &self.staging,
                &name,
                RuntimeArtifactPayloadError::Durability,
            );
        }
        drop(file);
        fsync(&self.staging).map_err(|_| RuntimeArtifactPayloadError::Durability)?;
        let descriptor = open_regular(&self.staging, &name)?;
        let snapshot = file_snapshot(&self.staging, &descriptor)?;
        if snapshot.size != byte_size {
            return cleanup_failed_stage(
                &self.staging,
                &name,
                RuntimeArtifactPayloadError::Conflict,
            );
        }
        self.revalidate()?;
        let observation = RuntimeArtifactPayloadObservation {
            payload_sha256: hex_digest(digest.finalize()),
            byte_size,
        };
        Ok((
            LinuxRuntimeArtifactStaged {
                name,
                observation: observation.clone(),
                snapshot,
            },
            observation,
        ))
    }

    fn discard_staged(&mut self, staged: Self::Staged) -> Result<(), RuntimeArtifactPayloadError> {
        self.revalidate()?;
        if !valid_staging_name(&staged.name) {
            return Err(RuntimeArtifactPayloadError::Invalid);
        }
        let descriptor = open_regular(&self.staging, &staged.name)?;
        let current = file_snapshot(&self.staging, &descriptor)?;
        if !current.same_object(&staged.snapshot) {
            return Err(RuntimeArtifactPayloadError::Conflict);
        }
        unlink_and_sync(&self.staging, &staged.name)?;
        self.revalidate()
    }

    fn place(
        &mut self,
        staged: Self::Staged,
        expected: &RuntimeArtifactPayloadObservation,
    ) -> Result<RuntimeArtifactPayloadPlacement, RuntimeArtifactPayloadError> {
        validate_observation(expected)?;
        self.revalidate()?;
        self.verify_staged(&staged, expected)?;
        match renameat_with(
            &self.staging,
            staged.name.as_str(),
            &self.objects,
            expected.payload_sha256.as_str(),
            RenameFlags::NOREPLACE,
        ) {
            Ok(()) => {
                fsync(&self.staging).map_err(|_| RuntimeArtifactPayloadError::Durability)?;
                fsync(&self.objects).map_err(|_| RuntimeArtifactPayloadError::Durability)?;
                self.verify_active(expected)?;
                Ok(RuntimeArtifactPayloadPlacement {
                    observation: expected.clone(),
                    deduplicated: false,
                })
            }
            Err(Errno::EXIST) => {
                self.verify_active(expected)?;
                self.discard_staged(staged)?;
                Ok(RuntimeArtifactPayloadPlacement {
                    observation: expected.clone(),
                    deduplicated: true,
                })
            }
            Err(_) => Err(RuntimeArtifactPayloadError::Durability),
        }
    }

    fn verify(
        &self,
        expected: &RuntimeArtifactPayloadObservation,
    ) -> Result<(), RuntimeArtifactPayloadError> {
        self.verify_active(expected).map(|_| ())
    }

    fn read_complete(
        &self,
        expected: &RuntimeArtifactPayloadObservation,
        maximum_bytes: u64,
    ) -> Result<Vec<u8>, RuntimeArtifactPayloadError> {
        validate_observation(expected)?;
        if maximum_bytes == 0
            || maximum_bytes > MAX_RUNTIME_ARTIFACT_BYTES
            || expected.byte_size > maximum_bytes
        {
            return Err(RuntimeArtifactPayloadError::ResourceLimit);
        }
        self.revalidate()?;
        let (bytes, _, observed) = observe_named_payload(
            &self.objects,
            &expected.payload_sha256,
            Some(expected.byte_size),
            true,
        )?;
        self.revalidate()?;
        if observed != *expected {
            return Err(RuntimeArtifactPayloadError::Corrupt);
        }
        Ok(bytes)
    }

    fn read_range(
        &self,
        expected: &RuntimeArtifactPayloadObservation,
        offset: u64,
        maximum_bytes: u64,
    ) -> Result<Vec<u8>, RuntimeArtifactPayloadError> {
        validate_observation(expected)?;
        if maximum_bytes == 0
            || maximum_bytes > MAX_RUNTIME_ARTIFACT_BYTES
            || offset >= expected.byte_size
        {
            return Err(RuntimeArtifactPayloadError::ResourceLimit);
        }
        let verified = self.verify_active(expected)?;
        self.revalidate()?;
        let descriptor = open_regular(&self.objects, &expected.payload_sha256)?;
        let before = file_snapshot(&self.objects, &descriptor)?;
        if before != verified {
            return Err(RuntimeArtifactPayloadError::Conflict);
        }
        let read_descriptor = descriptor
            .try_clone()
            .map_err(|_| RuntimeArtifactPayloadError::Durability)?;
        let mut file = File::from(read_descriptor);
        file.seek(SeekFrom::Start(offset))
            .map_err(|_| RuntimeArtifactPayloadError::Durability)?;
        let byte_count = maximum_bytes.min(expected.byte_size - offset);
        let mut bytes = vec![
            0;
            usize::try_from(byte_count)
                .map_err(|_| RuntimeArtifactPayloadError::ResourceLimit)?
        ];
        file.read_exact(&mut bytes)
            .map_err(|_| RuntimeArtifactPayloadError::Corrupt)?;
        let after = file_snapshot(&self.objects, &descriptor)?;
        self.revalidate()?;
        if before != after {
            return Err(RuntimeArtifactPayloadError::Conflict);
        }
        Ok(bytes)
    }

    fn quarantine(
        &mut self,
        expected: &RuntimeArtifactPayloadObservation,
    ) -> Result<(), RuntimeArtifactPayloadError> {
        validate_observation(expected)?;
        self.revalidate()?;
        let descriptor = open_regular(&self.objects, &expected.payload_sha256)?;
        let snapshot = file_snapshot(&self.objects, &descriptor)?;
        self.isolate_active(&expected.payload_sha256, &snapshot, "quarantine")?;
        self.revalidate()
    }

    fn delete(
        &mut self,
        expected: &RuntimeArtifactPayloadObservation,
    ) -> Result<(), RuntimeArtifactPayloadError> {
        let snapshot = self.verify_active(expected)?;
        let isolated = self.isolate_active(&expected.payload_sha256, &snapshot, "delete")?;
        unlink_and_sync(&self.quarantine, &isolated)?;
        self.revalidate()
    }

    fn inventory(
        &self,
    ) -> Result<Vec<RuntimeArtifactPayloadInventoryEntry>, RuntimeArtifactPayloadError> {
        self.revalidate()?;
        let names = inventory_names(&self.objects, MAX_ACTIVE_OBJECTS, valid_sha256)?;
        let mut inventory = Vec::with_capacity(names.len());
        for name in names {
            let descriptor = open_regular(&self.objects, &name)?;
            let snapshot = file_snapshot(&self.objects, &descriptor)?;
            inventory.push(RuntimeArtifactPayloadInventoryEntry {
                payload_sha256: name,
                byte_size: snapshot.size,
            });
        }
        self.revalidate()?;
        Ok(inventory)
    }

    fn cleanup_staging(&mut self) -> Result<u64, RuntimeArtifactPayloadError> {
        self.revalidate()?;
        let names = inventory_names(&self.staging, MAX_STAGING_OBJECTS, valid_staging_name)?;
        let count =
            u64::try_from(names.len()).map_err(|_| RuntimeArtifactPayloadError::ResourceLimit)?;
        for name in names {
            let descriptor = open_regular(&self.staging, &name)?;
            let _ = file_snapshot(&self.staging, &descriptor)?;
            unlink_and_sync(&self.staging, &name)?;
        }
        self.revalidate()?;
        Ok(count)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DirectorySnapshot {
    device: u64,
    inode: u64,
    owner: u32,
    mode: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FileSnapshot {
    device: u64,
    inode: u64,
    link_count: u64,
    owner: u32,
    mode: u32,
    size: u64,
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

fn open_or_create_directory(
    parent: &OwnedFd,
    name: &str,
    expected_device: u64,
) -> Result<(OwnedFd, DirectorySnapshot), RuntimeArtifactPayloadError> {
    let created = match mkdirat(parent, name, Mode::from_raw_mode(0o700)) {
        Ok(()) => true,
        Err(Errno::EXIST) => false,
        Err(_) => return Err(RuntimeArtifactPayloadError::Durability),
    };
    let descriptor = openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| RuntimeArtifactPayloadError::UnsafeRoot)?;
    if created {
        fchmod(&descriptor, Mode::from_raw_mode(0o700))
            .map_err(|_| RuntimeArtifactPayloadError::Durability)?;
        fsync(&descriptor).map_err(|_| RuntimeArtifactPayloadError::Durability)?;
        fsync(parent).map_err(|_| RuntimeArtifactPayloadError::Durability)?;
    }
    let snapshot = directory_snapshot(&descriptor, Some(expected_device))?;
    Ok((descriptor, snapshot))
}

fn directory_snapshot(
    descriptor: &OwnedFd,
    expected_device: Option<u64>,
) -> Result<DirectorySnapshot, RuntimeArtifactPayloadError> {
    let stat = fstat(descriptor).map_err(|_| RuntimeArtifactPayloadError::UnsafeRoot)?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::Directory
        || expected_device.is_some_and(|device| stat.st_dev != device)
        || stat.st_uid != getuid().as_raw()
        || stat.st_mode & 0o777 != 0o700
    {
        return Err(RuntimeArtifactPayloadError::UnsafeRoot);
    }
    Ok(DirectorySnapshot {
        device: stat.st_dev,
        inode: stat.st_ino,
        owner: stat.st_uid,
        mode: stat.st_mode & 0o777,
    })
}

fn verify_directory(
    descriptor: &OwnedFd,
    expected: &DirectorySnapshot,
    expected_device: Option<u64>,
) -> Result<(), RuntimeArtifactPayloadError> {
    if directory_snapshot(descriptor, expected_device)? == *expected {
        Ok(())
    } else {
        Err(RuntimeArtifactPayloadError::UnsafeRoot)
    }
}

fn open_regular(directory: &OwnedFd, name: &str) -> Result<OwnedFd, RuntimeArtifactPayloadError> {
    openat(
        directory,
        name,
        OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| {
        if error == Errno::NOENT {
            RuntimeArtifactPayloadError::Missing
        } else {
            RuntimeArtifactPayloadError::UnsafeRoot
        }
    })
}

fn file_snapshot(
    directory: &OwnedFd,
    descriptor: &OwnedFd,
) -> Result<FileSnapshot, RuntimeArtifactPayloadError> {
    let directory_stat = fstat(directory).map_err(|_| RuntimeArtifactPayloadError::UnsafeRoot)?;
    let stat = fstat(descriptor).map_err(|_| RuntimeArtifactPayloadError::UnsafeRoot)?;
    let size = u64::try_from(stat.st_size).map_err(|_| RuntimeArtifactPayloadError::Corrupt)?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile
        || stat.st_dev != directory_stat.st_dev
        || stat.st_uid != getuid().as_raw()
        || stat.st_mode & 0o777 != 0o600
        || stat.st_nlink != 1
        || size > MAX_RUNTIME_ARTIFACT_BYTES
    {
        return Err(RuntimeArtifactPayloadError::UnsafeRoot);
    }
    Ok(FileSnapshot {
        device: stat.st_dev,
        inode: stat.st_ino,
        link_count: stat.st_nlink,
        owner: stat.st_uid,
        mode: stat.st_mode & 0o777,
        size,
        modified_seconds: stat.st_mtime,
        modified_nanoseconds: stat.st_mtime_nsec as u64,
        changed_seconds: stat.st_ctime,
        changed_nanoseconds: stat.st_ctime_nsec as u64,
    })
}

fn observe_named_payload(
    directory: &OwnedFd,
    name: &str,
    expected_size: Option<u64>,
    retain_bytes: bool,
) -> Result<(Vec<u8>, FileSnapshot, RuntimeArtifactPayloadObservation), RuntimeArtifactPayloadError>
{
    let descriptor = open_regular(directory, name)?;
    let before = file_snapshot(directory, &descriptor)?;
    if expected_size.is_some_and(|size| before.size != size) {
        return Err(RuntimeArtifactPayloadError::Corrupt);
    }
    let read_descriptor = descriptor
        .try_clone()
        .map_err(|_| RuntimeArtifactPayloadError::Durability)?;
    let mut file = File::from(read_descriptor);
    let capacity = if retain_bytes {
        usize::try_from(before.size).map_err(|_| RuntimeArtifactPayloadError::ResourceLimit)?
    } else {
        0
    };
    let mut bytes = Vec::with_capacity(capacity);
    let mut digest = Sha256::new();
    let mut byte_size = 0_u64;
    let mut buffer = [0_u8; HASH_BUFFER_BYTES];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|_| RuntimeArtifactPayloadError::Durability)?;
        if count == 0 {
            break;
        }
        byte_size = byte_size
            .checked_add(count as u64)
            .filter(|size| *size <= MAX_RUNTIME_ARTIFACT_BYTES)
            .ok_or(RuntimeArtifactPayloadError::Corrupt)?;
        digest.update(&buffer[..count]);
        if retain_bytes {
            bytes.extend_from_slice(&buffer[..count]);
        }
    }
    let after = file_snapshot(directory, &descriptor)?;
    if before != after || byte_size != before.size {
        return Err(RuntimeArtifactPayloadError::Conflict);
    }
    Ok((
        bytes,
        before,
        RuntimeArtifactPayloadObservation {
            payload_sha256: hex_digest(digest.finalize()),
            byte_size,
        },
    ))
}

fn inventory_names(
    directory: &OwnedFd,
    maximum: usize,
    validator: fn(&str) -> bool,
) -> Result<Vec<String>, RuntimeArtifactPayloadError> {
    let entries = Dir::read_from(directory).map_err(|_| RuntimeArtifactPayloadError::UnsafeRoot)?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|_| RuntimeArtifactPayloadError::UnsafeRoot)?;
        let raw = entry.file_name().to_bytes();
        if matches!(raw, b"." | b"..") {
            continue;
        }
        if names.len() >= maximum {
            return Err(RuntimeArtifactPayloadError::ResourceLimit);
        }
        let name = std::str::from_utf8(raw).map_err(|_| RuntimeArtifactPayloadError::UnsafeRoot)?;
        if !validator(name) {
            return Err(RuntimeArtifactPayloadError::UnsafeRoot);
        }
        names.push(name.to_owned());
    }
    names.sort_unstable();
    if names.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(RuntimeArtifactPayloadError::Conflict);
    }
    Ok(names)
}

fn unlink_and_sync(directory: &OwnedFd, name: &str) -> Result<(), RuntimeArtifactPayloadError> {
    unlinkat(directory, name, AtFlags::empty())
        .map_err(|_| RuntimeArtifactPayloadError::Durability)?;
    fsync(directory).map_err(|_| RuntimeArtifactPayloadError::Durability)
}

fn cleanup_failed_stage<T>(
    directory: &OwnedFd,
    name: &str,
    error: RuntimeArtifactPayloadError,
) -> Result<T, RuntimeArtifactPayloadError> {
    match unlinkat(directory, name, AtFlags::empty()) {
        Ok(()) | Err(Errno::NOENT) => {}
        Err(_) => return Err(RuntimeArtifactPayloadError::Durability),
    }
    fsync(directory).map_err(|_| RuntimeArtifactPayloadError::Durability)?;
    Err(error)
}

fn validate_observation(
    expected: &RuntimeArtifactPayloadObservation,
) -> Result<(), RuntimeArtifactPayloadError> {
    if valid_sha256(&expected.payload_sha256)
        && expected.byte_size > 0
        && expected.byte_size <= MAX_RUNTIME_ARTIFACT_BYTES
    {
        Ok(())
    } else {
        Err(RuntimeArtifactPayloadError::Invalid)
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn staging_name(artifact_id: &RuntimeArtifactId) -> String {
    format!(
        "{STAGING_PREFIX}{}",
        hex_digest(Sha256::digest(artifact_id.as_str().as_bytes()))
    )
}

fn valid_staging_name(value: &str) -> bool {
    value.strip_prefix(STAGING_PREFIX).is_some_and(valid_sha256)
}

fn hex_digest(digest: impl AsRef<[u8]>) -> String {
    digest
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Cursor;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::RuntimeArtifactId;
    use agentmage_kernel_engine::runtime_artifact::{
        RuntimeArtifactPayloadError, RuntimeArtifactPayloadObservation, RuntimeArtifactPayloadStore,
    };

    use super::{OBJECT_DIRECTORY, STAGING_DIRECTORY, STORE_DIRECTORY};
    use crate::{LinuxRuntimeArtifactPayloadStore, LinuxStrictLocalRootInspector};

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(1);

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "agentmage-linux-runtime-artifact-{label}-{}-{}",
                std::process::id(),
                NEXT_ROOT.fetch_add(1, Ordering::SeqCst)
            ));
            fs::create_dir(&path).expect("root creates");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("root is private");
            Self(path)
        }

        fn store(&self) -> LinuxRuntimeArtifactPayloadStore {
            let root = LinuxStrictLocalRootInspector::inspect(&self.0).expect("root inspects");
            LinuxRuntimeArtifactPayloadStore::open(&root).expect("store opens")
        }

        fn path(&self) -> &Path {
            &self.0
        }

        fn objects(&self) -> PathBuf {
            self.0.join(STORE_DIRECTORY).join(OBJECT_DIRECTORY)
        }

        fn staging(&self) -> PathBuf {
            self.0.join(STORE_DIRECTORY).join(STAGING_DIRECTORY)
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("root removes");
        }
    }

    fn artifact_id(value: &str) -> RuntimeArtifactId {
        RuntimeArtifactId::from_raw(value)
    }

    fn stage_and_place(
        store: &mut LinuxRuntimeArtifactPayloadStore,
        id: &str,
        bytes: &[u8],
    ) -> RuntimeArtifactPayloadObservation {
        let (staged, observation) = store
            .stage(
                &artifact_id(id),
                &mut Cursor::new(bytes),
                bytes.len() as u64,
            )
            .expect("payload stages");
        store.place(staged, &observation).expect("payload places");
        observation
    }

    #[test]
    fn stage_place_read_inventory_and_dedup_are_content_addressed() {
        let root = TestRoot::new("round-trip");
        let mut store = root.store();
        let bytes = b"bounded artifact payload";
        let observation = stage_and_place(&mut store, "artifact-1", bytes);
        assert_eq!(
            store
                .read_complete(&observation, bytes.len() as u64)
                .expect("payload reads"),
            bytes
        );
        assert_eq!(
            store
                .read_range(&observation, 8, 8)
                .expect("bounded range reads"),
            bytes[8..16]
        );
        assert_eq!(
            store.read_range(&observation, bytes.len() as u64, 1),
            Err(RuntimeArtifactPayloadError::ResourceLimit)
        );
        assert_eq!(
            store.read_range(&observation, 0, 0),
            Err(RuntimeArtifactPayloadError::ResourceLimit)
        );
        assert_eq!(
            store.inventory().expect("inventory reads"),
            [
                agentmage_kernel_engine::runtime_artifact::RuntimeArtifactPayloadInventoryEntry {
                    payload_sha256: observation.payload_sha256.clone(),
                    byte_size: bytes.len() as u64,
                }
            ]
        );

        let (staged, duplicate) = store
            .stage(
                &artifact_id("artifact-2"),
                &mut Cursor::new(bytes),
                bytes.len() as u64,
            )
            .expect("duplicate stages");
        assert_eq!(duplicate, observation);
        assert!(
            store
                .place(staged, &observation)
                .expect("duplicate places")
                .deduplicated
        );
        assert!(
            root.staging()
                .read_dir()
                .expect("staging lists")
                .next()
                .is_none()
        );
    }

    #[test]
    fn staging_is_bounded_and_interrupted_objects_are_cleaned() {
        let root = TestRoot::new("staging");
        let mut store = root.store();
        assert!(matches!(
            store.stage(&artifact_id("artifact-large"), &mut Cursor::new(b"four"), 3,),
            Err(RuntimeArtifactPayloadError::ResourceLimit)
        ));
        assert!(
            root.staging()
                .read_dir()
                .expect("staging lists")
                .next()
                .is_none()
        );

        let _ = store
            .stage(
                &artifact_id("artifact-interrupted"),
                &mut Cursor::new(b"retained"),
                8,
            )
            .expect("interrupted stage exists");
        assert_eq!(store.cleanup_staging().expect("staging cleans"), 1);
        assert_eq!(store.cleanup_staging().expect("cleanup is idempotent"), 0);
    }

    #[test]
    fn invalid_names_symlinks_modes_and_root_drift_fail_closed() {
        let root = TestRoot::new("adversarial");
        let mut store = root.store();
        let invalid = RuntimeArtifactPayloadObservation {
            payload_sha256: "../authority.db".to_owned(),
            byte_size: 1,
        };
        assert_eq!(
            store.verify(&invalid),
            Err(RuntimeArtifactPayloadError::Invalid)
        );

        let symlink_digest = "a".repeat(64);
        symlink("../../authority.db", root.objects().join(&symlink_digest))
            .expect("symlink creates");
        assert_eq!(
            store.verify(&RuntimeArtifactPayloadObservation {
                payload_sha256: symlink_digest,
                byte_size: 1,
            }),
            Err(RuntimeArtifactPayloadError::UnsafeRoot)
        );

        fs::remove_file(root.objects().join("a".repeat(64))).expect("symlink removes");
        let observation = stage_and_place(&mut store, "artifact-mode", b"mode");
        fs::set_permissions(
            root.objects().join(&observation.payload_sha256),
            fs::Permissions::from_mode(0o644),
        )
        .expect("mode changes");
        assert_eq!(
            store.verify(&observation),
            Err(RuntimeArtifactPayloadError::UnsafeRoot)
        );

        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o755))
            .expect("root mode changes");
        assert_eq!(
            store.inventory(),
            Err(RuntimeArtifactPayloadError::UnsafeRoot)
        );
    }

    #[test]
    fn corruption_quarantine_and_verified_delete_leave_no_active_object() {
        let root = TestRoot::new("lifecycle");
        let mut store = root.store();
        let corrupt = stage_and_place(&mut store, "artifact-corrupt", b"before");
        fs::write(root.objects().join(&corrupt.payload_sha256), b"after!")
            .expect("payload corrupts");
        assert_eq!(
            store.verify(&corrupt),
            Err(RuntimeArtifactPayloadError::Corrupt)
        );
        store.quarantine(&corrupt).expect("payload quarantines");
        assert!(store.inventory().expect("inventory reads").is_empty());

        let deleted = stage_and_place(&mut store, "artifact-delete", b"delete");
        store.delete(&deleted).expect("payload deletes");
        assert_eq!(
            store.verify(&deleted),
            Err(RuntimeArtifactPayloadError::Missing)
        );
    }
}
