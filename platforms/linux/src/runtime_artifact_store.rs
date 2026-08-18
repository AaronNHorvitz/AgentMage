//! Descriptor-relative private storage for runtime artifact payloads.

use std::fmt;
use std::fs::File;
use std::io::Read;
#[cfg(test)]
use std::sync::Arc;

use agentmage_kernel_contracts::RuntimeArtifactId;
use agentmage_kernel_engine::runtime_artifact::{
    MAX_RUNTIME_ARTIFACT_BYTES, RuntimeArtifactPayloadError, RuntimeArtifactPayloadInventoryEntry,
    RuntimeArtifactPayloadInventoryIntegrity, RuntimeArtifactPayloadObservation,
    RuntimeArtifactPayloadPlacement, RuntimeArtifactPayloadStore,
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
use crate::runtime_artifact_crypto::{
    ArtifactPayloadEncryptionKey, MAX_ENCRYPTED_PAYLOAD_BYTES, encrypt_payload, observe_payload,
    read_complete_payload, read_payload_range,
};

const STORE_DIRECTORY: &str = ".agentmage-runtime-payloads-v1";
const STAGING_DIRECTORY: &str = "staging";
const OBJECT_DIRECTORY: &str = "objects";
const QUARANTINE_DIRECTORY: &str = "quarantine";
const STAGING_PREFIX: &str = "stage-";
const MAX_ACTIVE_OBJECTS: usize = 65_536;
const MAX_STAGING_OBJECTS: usize = 4_096;
const MAX_QUARANTINE_ATTEMPTS: usize = 1_024;

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArtifactRacePoint {
    PlaceBeforeRename,
    ReadCompleteAfterRevalidate,
    IsolateBeforeRename,
}

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
    encryption_key: ArtifactPayloadEncryptionKey,
    #[cfg(test)]
    race_hook: Option<Arc<dyn Fn(ArtifactRacePoint) + Send + Sync>>,
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
    pub(crate) fn open(
        root: &LinuxStrictLocalRoot,
        encryption_key: ArtifactPayloadEncryptionKey,
    ) -> Result<Self, RuntimeArtifactPayloadError> {
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
            encryption_key,
            #[cfg(test)]
            race_hook: None,
        })
    }

    #[cfg(test)]
    fn set_race_hook(&mut self, hook: Arc<dyn Fn(ArtifactRacePoint) + Send + Sync>) {
        self.race_hook = Some(hook);
    }

    #[cfg(test)]
    fn reach_race_point(&self, point: ArtifactRacePoint) {
        if let Some(hook) = &self.race_hook {
            hook(point);
        }
    }

    fn revalidate(&self) -> Result<(), RuntimeArtifactPayloadError> {
        verify_directory(&self.root, &self.root_snapshot, None)?;
        verify_named_directory(
            &self.root,
            STORE_DIRECTORY,
            &self.store_snapshot,
            Some(self.root_snapshot.device),
        )?;
        verify_directory(
            &self.store,
            &self.store_snapshot,
            Some(self.root_snapshot.device),
        )?;
        verify_named_directory(
            &self.store,
            STAGING_DIRECTORY,
            &self.staging_snapshot,
            Some(self.root_snapshot.device),
        )?;
        verify_directory(
            &self.staging,
            &self.staging_snapshot,
            Some(self.root_snapshot.device),
        )?;
        verify_named_directory(
            &self.store,
            OBJECT_DIRECTORY,
            &self.objects_snapshot,
            Some(self.root_snapshot.device),
        )?;
        verify_directory(
            &self.objects,
            &self.objects_snapshot,
            Some(self.root_snapshot.device),
        )?;
        verify_named_directory(
            &self.store,
            QUARANTINE_DIRECTORY,
            &self.quarantine_snapshot,
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
            observe_named_payload(&self.staging, &staged.name, &self.encryption_key, false)?;
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
            &self.encryption_key,
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
            #[cfg(test)]
            self.reach_race_point(ArtifactRacePoint::IsolateBeforeRename);
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
        let observation =
            match encrypt_payload(source, &mut file, &self.encryption_key, maximum_bytes) {
                Ok(observation) => observation,
                Err(error) => {
                    drop(file);
                    return cleanup_failed_stage(&self.staging, &name, error);
                }
            };
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
        let verified = observe_named_payload(&self.staging, &name, &self.encryption_key, false);
        let (_, snapshot, verified) = match verified {
            Ok(verified) => verified,
            Err(error) => return cleanup_failed_stage(&self.staging, &name, error),
        };
        if verified != observation {
            return cleanup_failed_stage(
                &self.staging,
                &name,
                RuntimeArtifactPayloadError::Conflict,
            );
        }
        self.revalidate()?;
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
        #[cfg(test)]
        self.reach_race_point(ArtifactRacePoint::PlaceBeforeRename);
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
        #[cfg(test)]
        self.reach_race_point(ArtifactRacePoint::ReadCompleteAfterRevalidate);
        let (bytes, _, observed) = observe_named_payload(
            &self.objects,
            &expected.payload_sha256,
            &self.encryption_key,
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
        self.revalidate()?;
        let descriptor = open_regular(&self.objects, &expected.payload_sha256)?;
        let before = file_snapshot(&self.objects, &descriptor)?;
        let read_descriptor = descriptor
            .try_clone()
            .map_err(|_| RuntimeArtifactPayloadError::Durability)?;
        let mut file = File::from(read_descriptor);
        let (bytes, observed) = read_payload_range(
            &mut file,
            &self.encryption_key,
            offset,
            maximum_bytes.min(expected.byte_size - offset),
        )?;
        let after = file_snapshot(&self.objects, &descriptor)?;
        self.revalidate()?;
        if before != after {
            return Err(RuntimeArtifactPayloadError::Conflict);
        }
        if observed != *expected {
            return Err(RuntimeArtifactPayloadError::Corrupt);
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
            match observe_named_payload(&self.objects, &name, &self.encryption_key, false) {
                Ok((_, _, observed)) if observed.payload_sha256 == name => {
                    inventory.push(RuntimeArtifactPayloadInventoryEntry {
                        payload_sha256: name,
                        byte_size: observed.byte_size,
                        integrity: RuntimeArtifactPayloadInventoryIntegrity::Verified,
                    });
                }
                Ok(_) | Err(RuntimeArtifactPayloadError::Corrupt) => {
                    inventory.push(RuntimeArtifactPayloadInventoryEntry {
                        payload_sha256: name,
                        byte_size: 0,
                        integrity: RuntimeArtifactPayloadInventoryIntegrity::Corrupt,
                    });
                }
                Err(error) => return Err(error),
            }
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

fn verify_named_directory(
    parent: &OwnedFd,
    name: &str,
    expected: &DirectorySnapshot,
    expected_device: Option<u64>,
) -> Result<(), RuntimeArtifactPayloadError> {
    let descriptor = openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| RuntimeArtifactPayloadError::UnsafeRoot)?;
    verify_directory(&descriptor, expected, expected_device)
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
        || size > MAX_ENCRYPTED_PAYLOAD_BYTES
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
    encryption_key: &ArtifactPayloadEncryptionKey,
    retain_bytes: bool,
) -> Result<(Vec<u8>, FileSnapshot, RuntimeArtifactPayloadObservation), RuntimeArtifactPayloadError>
{
    let descriptor = open_regular(directory, name)?;
    let before = file_snapshot(directory, &descriptor)?;
    let read_descriptor = descriptor
        .try_clone()
        .map_err(|_| RuntimeArtifactPayloadError::Durability)?;
    let mut file = File::from(read_descriptor);
    let (bytes, observation) = if retain_bytes {
        read_complete_payload(&mut file, encryption_key)?
    } else {
        (Vec::new(), observe_payload(&mut file, encryption_key)?)
    };
    let after = file_snapshot(directory, &descriptor)?;
    if before != after {
        return Err(RuntimeArtifactPayloadError::Conflict);
    }
    Ok((bytes, before, observation))
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
    use std::env;
    use std::fs::{self, File};
    use std::io::{Cursor, Read};
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Barrier};
    use std::time::Instant;

    use agentmage_kernel_contracts::{
        CONTRACT_SCHEMA_VERSION, CheckpointFileIdentity, ContextSensitivity, CorrelationId,
        EvidenceId, ModelProfileId, PlanId, PlanStepId, PolicyId, RepositorySnapshotId,
        RuntimeArtifactId, RuntimeArtifactIntegrityState, RuntimeArtifactKind,
        RuntimeArtifactLifecycleState, RuntimeArtifactManifest, RuntimeArtifactRef, RuntimeEvent,
        RuntimeEventId, RuntimeEventKind, RuntimeEventPersistenceClass, RuntimeEventRetention,
        RuntimeEventRetentionKind, RuntimeResumeBinding, RuntimeRunId, SessionCheckpoint,
        SessionCheckpointId, SessionId, TaskId, WorkspaceId,
    };
    use agentmage_kernel_engine::context_management::finalize_checkpoint;
    use agentmage_kernel_engine::operational_store::{
        DurableAuthorityRuntime, OperationalStoreKeyError, OperationalStoreKeyProvider,
    };
    use agentmage_kernel_engine::runtime_artifact::{
        MAX_RUNTIME_ARTIFACT_BYTES, MAX_RUNTIME_ARTIFACTS_PER_CHECKPOINT, RuntimeArtifactError,
        RuntimeArtifactPageRequest, RuntimeArtifactPayloadError,
        RuntimeArtifactPayloadInventoryEntry, RuntimeArtifactPayloadInventoryIntegrity,
        RuntimeArtifactPayloadObservation, RuntimeArtifactPayloadStore, runtime_artifact_ref,
        runtime_payload_reference, seal_runtime_artifact_manifest, seal_runtime_resume_binding,
    };
    use agentmage_kernel_engine::runtime_event::seal_runtime_event;
    use sha2::{Digest, Sha256};

    use super::{
        ArtifactRacePoint, OBJECT_DIRECTORY, QUARANTINE_DIRECTORY, STAGING_DIRECTORY,
        STORE_DIRECTORY,
    };
    use crate::runtime_artifact_crypto::derive_artifact_payload_key;
    use crate::{LinuxRuntimeArtifactPayloadStore, LinuxStrictLocalRootInspector};

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(1);
    const ARTIFACT_CRASH_CHILD_EXIT: i32 = 89;
    const ARTIFACT_CRASH_PAYLOAD: &[u8] = b"native encrypted artifact crash payload";

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum ArtifactCrashBoundary {
        Staging,
        Placement,
        MetadataCommit,
        EventCommit,
        CheckpointCommit,
        ReferenceRelease,
        Collection,
    }

    impl ArtifactCrashBoundary {
        const ALL: [Self; 7] = [
            Self::Staging,
            Self::Placement,
            Self::MetadataCommit,
            Self::EventCommit,
            Self::CheckpointCommit,
            Self::ReferenceRelease,
            Self::Collection,
        ];

        const fn code(self) -> &'static str {
            match self {
                Self::Staging => "staging",
                Self::Placement => "placement",
                Self::MetadataCommit => "metadata-commit",
                Self::EventCommit => "event-commit",
                Self::CheckpointCommit => "checkpoint-commit",
                Self::ReferenceRelease => "reference-release",
                Self::Collection => "collection",
            }
        }

        fn from_code(code: &str) -> Self {
            Self::ALL
                .into_iter()
                .find(|boundary| boundary.code() == code)
                .expect("declared artifact crash boundary")
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum ArtifactCrashPosition {
        Before,
        After,
    }

    impl ArtifactCrashPosition {
        const ALL: [Self; 2] = [Self::Before, Self::After];

        const fn code(self) -> &'static str {
            match self {
                Self::Before => "before",
                Self::After => "after",
            }
        }

        fn from_code(code: &str) -> Self {
            Self::ALL
                .into_iter()
                .find(|position| position.code() == code)
                .expect("declared artifact crash position")
        }
    }

    struct ArtifactCrashKey;

    impl OperationalStoreKeyProvider for ArtifactCrashKey {
        fn with_key<T>(
            &mut self,
            operation: impl FnOnce(&[u8]) -> T,
        ) -> Result<T, OperationalStoreKeyError> {
            Ok(operation(&[0x4a; 32]))
        }
    }

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
            self.store_with_key(0x5a)
        }

        fn store_with_key(&self, key_byte: u8) -> LinuxRuntimeArtifactPayloadStore {
            let root = LinuxStrictLocalRootInspector::inspect(&self.0).expect("root inspects");
            LinuxRuntimeArtifactPayloadStore::open(
                &root,
                derive_artifact_payload_key(&[key_byte; 32]).expect("artifact key derives"),
            )
            .expect("store opens")
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

        fn namespace(&self) -> PathBuf {
            self.0.join(STORE_DIRECTORY)
        }

        fn quarantine(&self) -> PathBuf {
            self.0.join(STORE_DIRECTORY).join(QUARANTINE_DIRECTORY)
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

    fn repeated_digest(value: char) -> String {
        value.to_string().repeat(64)
    }

    fn open_artifact_crash_components(
        path: &Path,
        recovery_epoch_ms: u64,
    ) -> (DurableAuthorityRuntime, LinuxRuntimeArtifactPayloadStore) {
        let root = LinuxStrictLocalRootInspector::inspect(path).expect("crash root inspects");
        let artifact_store = LinuxRuntimeArtifactPayloadStore::open(
            &root,
            derive_artifact_payload_key(&[0x4a; 32]).expect("crash artifact key derives"),
        )
        .expect("crash artifact store opens");
        let runtime = DurableAuthorityRuntime::open(
            &root.authority_database_path(),
            root.observation(),
            &mut ArtifactCrashKey,
            recovery_epoch_ms,
        )
        .expect("crash authority opens");
        (runtime, artifact_store)
    }

    fn artifact_crash_manifest() -> RuntimeArtifactManifest {
        seal_runtime_artifact_manifest(RuntimeArtifactManifest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            artifact_id: RuntimeArtifactId::from_raw("artifact-crash-native-22-2"),
            kind: RuntimeArtifactKind::TestLog,
            payload_sha256: super::hex_digest(Sha256::digest(ARTIFACT_CRASH_PAYLOAD)),
            byte_size: ARTIFACT_CRASH_PAYLOAD.len() as u64,
            media_type: "text/plain".to_owned(),
            sensitivity: ContextSensitivity::Private,
            retention: RuntimeEventRetention {
                kind: RuntimeEventRetentionKind::Session,
                expires_at_epoch_ms: None,
            },
            session_id: SessionId::from_raw("session-crash-native-22-2"),
            task_id: TaskId::from_raw("task-crash-native-22-2"),
            producer_run_id: RuntimeRunId::from_raw("run-crash-native-22-2"),
            producer_turn_id: None,
            producer_operation_id: None,
            receipt_id: None,
            policy_id: PolicyId::from_raw("policy-crash-native-22-2"),
            policy_sha256: repeated_digest('b'),
            created_at_epoch_ms: 2,
            integrity: RuntimeArtifactIntegrityState::Verified,
            preview: None,
            manifest_sha256: repeated_digest('0'),
        })
        .expect("crash manifest seals")
    }

    fn artifact_crash_run_start() -> RuntimeEvent {
        seal_runtime_event(RuntimeEvent {
            schema_version: CONTRACT_SCHEMA_VERSION,
            event_id: RuntimeEventId::from_raw("event-crash-native-start-22-2"),
            run_id: RuntimeRunId::from_raw("run-crash-native-22-2"),
            session_id: SessionId::from_raw("session-crash-native-22-2"),
            task_id: TaskId::from_raw("task-crash-native-22-2"),
            turn_id: None,
            operation_id: None,
            correlation_id: CorrelationId::from_raw("correlation-crash-native-22-2"),
            causation_event_id: None,
            sequence: 0,
            occurred_at_epoch_ms: 1,
            sensitivity: ContextSensitivity::Private,
            retention: RuntimeEventRetention {
                kind: RuntimeEventRetentionKind::Session,
                expires_at_epoch_ms: None,
            },
            persistence: RuntimeEventPersistenceClass::Correctness,
            policy_id: PolicyId::from_raw("policy-crash-native-22-2"),
            payload_reference: None,
            kind: RuntimeEventKind::RunStarted {
                request_sha256: repeated_digest('a'),
            },
            previous_event_sha256: repeated_digest('0'),
            event_sha256: repeated_digest('0'),
        })
        .expect("crash run start seals")
    }

    fn artifact_crash_created_event(
        runtime: &DurableAuthorityRuntime,
        manifest: &RuntimeArtifactManifest,
    ) -> RuntimeEvent {
        let cursor = runtime
            .runtime_event_cursor(&manifest.producer_run_id)
            .expect("crash cursor loads")
            .expect("crash cursor exists");
        seal_runtime_event(RuntimeEvent {
            schema_version: CONTRACT_SCHEMA_VERSION,
            event_id: RuntimeEventId::from_raw("event-crash-native-artifact-22-2"),
            run_id: manifest.producer_run_id.clone(),
            session_id: manifest.session_id.clone(),
            task_id: manifest.task_id.clone(),
            turn_id: None,
            operation_id: None,
            correlation_id: CorrelationId::from_raw("correlation-crash-native-22-2"),
            causation_event_id: Some(cursor.event_id),
            sequence: cursor.sequence + 1,
            occurred_at_epoch_ms: 3,
            sensitivity: manifest.sensitivity,
            retention: manifest.retention.clone(),
            persistence: RuntimeEventPersistenceClass::Correctness,
            policy_id: manifest.policy_id.clone(),
            payload_reference: Some(
                runtime_payload_reference(manifest).expect("crash event payload reference"),
            ),
            kind: RuntimeEventKind::ArtifactCreated {
                artifact_id: manifest.artifact_id.clone(),
                manifest_sha256: manifest.manifest_sha256.clone(),
            },
            previous_event_sha256: cursor.event_sha256,
            event_sha256: repeated_digest('0'),
        })
        .expect("crash artifact event seals")
    }

    fn artifact_crash_checkpoint() -> SessionCheckpoint {
        finalize_checkpoint(SessionCheckpoint {
            schema_version: CONTRACT_SCHEMA_VERSION,
            checkpoint_id: SessionCheckpointId::from_raw("checkpoint-crash-native-22-2"),
            session_id: SessionId::from_raw("session-crash-native-22-2"),
            task_id: TaskId::from_raw("task-crash-native-22-2"),
            objective_sha256: repeated_digest('1'),
            plan_id: PlanId::from_raw("plan-crash-native-22-2"),
            plan_revision: 1,
            plan_step_id: PlanStepId::from_raw("step-crash-native-22-2"),
            next_action_sha256: repeated_digest('2'),
            workspace_id: WorkspaceId::from_raw("workspace-crash-native-22-2"),
            workspace_state_sha256: repeated_digest('3'),
            repository_snapshot_id: RepositorySnapshotId::from_raw("snapshot-crash-native-22-2"),
            repository_branch: "main".to_owned(),
            repository_map_sha256: repeated_digest('4'),
            files: vec![CheckpointFileIdentity {
                object_id: "object-crash-native-22-2".to_owned(),
                content_sha256: repeated_digest('5'),
                observed_revision: "revision-crash-native-22-2".to_owned(),
            }],
            instruction_sha256: repeated_digest('6'),
            permission_profile_id: "permission-crash-native-22-2".to_owned(),
            permission_profile_sha256: repeated_digest('7'),
            policy_id: PolicyId::from_raw("policy-crash-native-22-2"),
            policy_sha256: repeated_digest('b'),
            model_profile_id: ModelProfileId::from_raw("model-crash-native-22-2"),
            model_manifest_sha256: repeated_digest('8'),
            model_runtime_sha256: repeated_digest('9'),
            evidence_ids: vec![EvidenceId::from_raw("evidence-crash-native-22-2")],
            citation_set_sha256: repeated_digest('c'),
            blockers: Vec::new(),
            context_packet_sha256: repeated_digest('d'),
            action_id: None,
            action_state: None,
            consumed_grant_id: None,
            receipt_id: None,
            receipt_sha256: None,
            ephemeral: false,
            checkpoint_sha256: repeated_digest('0'),
        })
        .expect("crash checkpoint finalizes")
    }

    fn artifact_crash_binding(
        runtime: &DurableAuthorityRuntime,
        manifest: &RuntimeArtifactManifest,
    ) -> RuntimeResumeBinding {
        let checkpoint = artifact_crash_checkpoint();
        seal_runtime_resume_binding(RuntimeResumeBinding {
            schema_version: CONTRACT_SCHEMA_VERSION,
            checkpoint_id: checkpoint.checkpoint_id.clone(),
            checkpoint_sha256: checkpoint.checkpoint_sha256,
            session_id: checkpoint.session_id,
            task_id: checkpoint.task_id,
            run_id: manifest.producer_run_id.clone(),
            event_cursor: runtime
                .runtime_event_cursor(&manifest.producer_run_id)
                .expect("binding cursor loads")
                .expect("binding cursor exists"),
            artifacts: vec![runtime_artifact_ref(manifest).expect("crash artifact reference")],
            binding_sha256: repeated_digest('0'),
        })
        .expect("crash binding seals")
    }

    fn artifact_pressure_manifest(id: &str, bytes: &[u8]) -> RuntimeArtifactManifest {
        let mut manifest = artifact_crash_manifest();
        manifest.artifact_id = RuntimeArtifactId::from_raw(id);
        manifest.payload_sha256 = super::hex_digest(Sha256::digest(bytes));
        manifest.byte_size = bytes.len() as u64;
        manifest.manifest_sha256 = repeated_digest('0');
        seal_runtime_artifact_manifest(manifest).expect("pressure manifest seals")
    }

    fn artifact_pressure_checkpoint(id: &str) -> SessionCheckpoint {
        let mut checkpoint = artifact_crash_checkpoint();
        checkpoint.checkpoint_id = SessionCheckpointId::from_raw(id);
        checkpoint.checkpoint_sha256 = repeated_digest('0');
        finalize_checkpoint(checkpoint).expect("pressure checkpoint finalizes")
    }

    fn artifact_pressure_binding(
        runtime: &DurableAuthorityRuntime,
        checkpoint: &SessionCheckpoint,
        artifacts: Vec<RuntimeArtifactRef>,
    ) -> RuntimeResumeBinding {
        seal_runtime_resume_binding(RuntimeResumeBinding {
            schema_version: CONTRACT_SCHEMA_VERSION,
            checkpoint_id: checkpoint.checkpoint_id.clone(),
            checkpoint_sha256: checkpoint.checkpoint_sha256.clone(),
            session_id: checkpoint.session_id.clone(),
            task_id: checkpoint.task_id.clone(),
            run_id: RuntimeRunId::from_raw("run-crash-native-22-2"),
            event_cursor: runtime
                .runtime_event_cursor(&RuntimeRunId::from_raw("run-crash-native-22-2"))
                .expect("pressure cursor loads")
                .expect("pressure cursor exists"),
            artifacts,
            binding_sha256: repeated_digest('0'),
        })
        .expect("pressure binding seals")
    }

    fn resident_memory_kib() -> u64 {
        fs::read_to_string("/proc/self/status")
            .expect("Linux resident-memory status reads")
            .lines()
            .find_map(|line| {
                line.strip_prefix("VmRSS:")?
                    .split_whitespace()
                    .next()?
                    .parse::<u64>()
                    .ok()
            })
            .expect("VmRSS is present")
    }

    fn recursive_directory_bytes(path: &Path) -> u64 {
        fs::read_dir(path)
            .expect("pressure directory lists")
            .map(|entry| {
                let entry = entry.expect("pressure entry reads");
                let metadata = entry.metadata().expect("pressure metadata reads");
                if metadata.is_dir() {
                    recursive_directory_bytes(&entry.path())
                } else {
                    metadata.len()
                }
            })
            .sum()
    }

    fn elapsed_ms(started: Instant) -> u64 {
        u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    fn prepare_artifact_crash_fixture(path: &Path) {
        let (mut runtime, payloads) = open_artifact_crash_components(path, 1);
        runtime
            .record_runtime_event(artifact_crash_run_start())
            .expect("crash run start persists");
        drop(payloads);
        drop(runtime);
    }

    fn artifact_crash_stop() -> ! {
        std::process::exit(ARTIFACT_CRASH_CHILD_EXIT)
    }

    fn stop_at(
        boundary: ArtifactCrashBoundary,
        position: ArtifactCrashPosition,
        expected_boundary: ArtifactCrashBoundary,
        expected_position: ArtifactCrashPosition,
    ) {
        if boundary == expected_boundary && position == expected_position {
            artifact_crash_stop();
        }
    }

    fn run_artifact_crash_child(
        path: &Path,
        boundary: ArtifactCrashBoundary,
        position: ArtifactCrashPosition,
    ) -> ! {
        let (mut runtime, mut payloads) = open_artifact_crash_components(path, 2);
        let manifest = artifact_crash_manifest();
        let reference = runtime_artifact_ref(&manifest).expect("crash reference projects");

        stop_at(
            boundary,
            position,
            ArtifactCrashBoundary::Staging,
            ArtifactCrashPosition::Before,
        );
        let (staged, observation) = payloads
            .stage(
                &manifest.artifact_id,
                &mut Cursor::new(ARTIFACT_CRASH_PAYLOAD),
                ARTIFACT_CRASH_PAYLOAD.len() as u64,
            )
            .expect("crash payload stages");
        stop_at(
            boundary,
            position,
            ArtifactCrashBoundary::Staging,
            ArtifactCrashPosition::After,
        );
        stop_at(
            boundary,
            position,
            ArtifactCrashBoundary::Placement,
            ArtifactCrashPosition::Before,
        );
        payloads
            .place(staged, &observation)
            .expect("crash payload places");
        stop_at(
            boundary,
            position,
            ArtifactCrashBoundary::Placement,
            ArtifactCrashPosition::After,
        );
        stop_at(
            boundary,
            position,
            ArtifactCrashBoundary::MetadataCommit,
            ArtifactCrashPosition::Before,
        );
        runtime
            .publish_runtime_artifact(
                &mut payloads,
                manifest.clone(),
                &mut Cursor::new(ARTIFACT_CRASH_PAYLOAD),
            )
            .expect("crash metadata publishes through canonical path");
        stop_at(
            boundary,
            position,
            ArtifactCrashBoundary::MetadataCommit,
            ArtifactCrashPosition::After,
        );
        stop_at(
            boundary,
            position,
            ArtifactCrashBoundary::EventCommit,
            ArtifactCrashPosition::Before,
        );
        runtime
            .record_runtime_event(artifact_crash_created_event(&runtime, &manifest))
            .expect("crash artifact event commits");
        stop_at(
            boundary,
            position,
            ArtifactCrashBoundary::EventCommit,
            ArtifactCrashPosition::After,
        );

        if boundary == ArtifactCrashBoundary::CheckpointCommit {
            stop_at(
                boundary,
                position,
                ArtifactCrashBoundary::CheckpointCommit,
                ArtifactCrashPosition::Before,
            );
            let checkpoint = artifact_crash_checkpoint();
            let binding = artifact_crash_binding(&runtime, &manifest);
            runtime
                .checkpoint_runtime_session(&checkpoint, &binding)
                .expect("crash checkpoint commits");
            artifact_crash_stop();
        }

        stop_at(
            boundary,
            position,
            ArtifactCrashBoundary::ReferenceRelease,
            ArtifactCrashPosition::Before,
        );
        runtime
            .release_runtime_artifact(
                &manifest.session_id,
                &manifest.task_id,
                &manifest.policy_sha256,
                &reference,
                4,
            )
            .expect("crash reference releases");
        stop_at(
            boundary,
            position,
            ArtifactCrashBoundary::ReferenceRelease,
            ArtifactCrashPosition::After,
        );
        stop_at(
            boundary,
            position,
            ArtifactCrashBoundary::Collection,
            ArtifactCrashPosition::Before,
        );
        runtime
            .reconcile_runtime_artifacts(&mut payloads, 5)
            .expect("crash collection reconciles");
        artifact_crash_stop();
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
                    integrity: RuntimeArtifactPayloadInventoryIntegrity::Verified,
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
        let retained_ciphertext = fs::read(root.objects().join(&observation.payload_sha256))
            .expect("retained ciphertext reads");
        assert!(
            store
                .place(staged, &observation)
                .expect("duplicate places")
                .deduplicated
        );
        assert_eq!(
            fs::read(root.objects().join(&observation.payload_sha256))
                .expect("deduplicated ciphertext reads"),
            retained_ciphertext
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
    fn staging_and_objects_are_encrypted_randomized_and_key_bound() {
        let root = TestRoot::new("encrypted-at-rest");
        let mut store = root.store();
        let canary = b"AGENTMAGE-PLAINTEXT-CANARY-MUST-NOT-REACH-DISK";
        let (staged, observation) = store
            .stage(
                &artifact_id("artifact-encrypted"),
                &mut Cursor::new(canary),
                canary.len() as u64,
            )
            .expect("payload stages encrypted");
        let staged_bytes = fs::read(
            root.staging()
                .read_dir()
                .expect("staging lists")
                .next()
                .expect("staging entry exists")
                .expect("staging entry reads")
                .path(),
        )
        .expect("staged ciphertext reads");
        assert!(
            !staged_bytes
                .windows(canary.len())
                .any(|window| window == canary)
        );

        store.place(staged, &observation).expect("payload places");
        let object_bytes = fs::read(root.objects().join(&observation.payload_sha256))
            .expect("object ciphertext reads");
        assert!(
            !object_bytes
                .windows(canary.len())
                .any(|window| window == canary)
        );
        assert_eq!(
            store
                .read_complete(&observation, canary.len() as u64)
                .expect("correct key decrypts"),
            canary
        );
        drop(store);

        let correct = root.store();
        assert_eq!(
            correct
                .read_complete(&observation, canary.len() as u64)
                .expect("same derived key reopens"),
            canary
        );
        drop(correct);
        let wrong = root.store_with_key(0x5b);
        assert_eq!(
            wrong.verify(&observation),
            Err(RuntimeArtifactPayloadError::Corrupt)
        );
    }

    #[test]
    fn hard_links_and_open_handle_collection_do_not_disclose_plaintext() {
        let root = TestRoot::new("open-handle");
        let mut store = root.store();
        let canary = b"open-handle-secret-canary";
        let observation = stage_and_place(&mut store, "artifact-open", canary);
        let object = root.objects().join(&observation.payload_sha256);
        let link = root.objects().join("b".repeat(64));
        fs::hard_link(&object, &link).expect("hard link creates");
        assert_eq!(
            store.verify(&observation),
            Err(RuntimeArtifactPayloadError::UnsafeRoot)
        );
        fs::remove_file(&link).expect("hard link removes");

        let mut held = File::open(&object).expect("ciphertext handle opens");
        store.delete(&observation).expect("payload deletes");
        assert!(!object.exists());
        let mut retained_ciphertext = Vec::new();
        held.read_to_end(&mut retained_ciphertext)
            .expect("unlinked handle remains readable");
        assert!(
            !retained_ciphertext
                .windows(canary.len())
                .any(|window| window == canary)
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
        fs::set_permissions(
            root.objects().join(&observation.payload_sha256),
            fs::Permissions::from_mode(0o700),
        )
        .expect("object becomes executable");
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
            store
                .inventory()
                .expect("corrupt inventory remains visible"),
            [
                agentmage_kernel_engine::runtime_artifact::RuntimeArtifactPayloadInventoryEntry {
                    payload_sha256: corrupt.payload_sha256.clone(),
                    byte_size: 0,
                    integrity: RuntimeArtifactPayloadInventoryIntegrity::Corrupt,
                }
            ]
        );
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

    #[test]
    fn fixed_namespace_substitution_fails_before_the_next_store_effect() {
        for namespace in ["store", "staging", "objects", "quarantine"] {
            let root = TestRoot::new(namespace);
            let mut store = root.store();
            let mut staged = None;
            let mut retained = None;
            if namespace == "objects" {
                let (candidate, observation) = store
                    .stage(
                        &artifact_id("artifact-objects-race"),
                        &mut Cursor::new(b"objects-race"),
                        12,
                    )
                    .expect("race payload stages");
                staged = Some((candidate, observation));
            } else if namespace == "quarantine" {
                retained = Some(stage_and_place(
                    &mut store,
                    "artifact-quarantine-race",
                    b"quarantine-race",
                ));
            }

            let target = match namespace {
                "store" => root.namespace(),
                "staging" => root.staging(),
                "objects" => root.objects(),
                "quarantine" => root.quarantine(),
                _ => unreachable!("closed namespace fixture"),
            };
            let displaced = target.with_extension("displaced");
            fs::rename(&target, &displaced).expect("held namespace displaces");
            fs::create_dir(&target).expect("substitute namespace creates");
            fs::set_permissions(&target, fs::Permissions::from_mode(0o700))
                .expect("substitute namespace is private");

            let result = match namespace {
                "store" => store.inventory().map(|_| ()),
                "staging" => store
                    .stage(
                        &artifact_id("artifact-staging-race"),
                        &mut Cursor::new(b"staging-race"),
                        12,
                    )
                    .map(|_| ()),
                "objects" => {
                    let (candidate, observation) = staged.take().expect("staged race fixture");
                    store.place(candidate, &observation).map(|_| ())
                }
                "quarantine" => store
                    .delete(retained.as_ref().expect("retained race fixture"))
                    .map(|_| ()),
                _ => unreachable!("closed namespace fixture"),
            };
            assert_eq!(result, Err(RuntimeArtifactPayloadError::UnsafeRoot));
            assert!(
                target
                    .read_dir()
                    .expect("substitute namespace lists")
                    .next()
                    .is_none(),
                "{namespace} substitution received a store effect"
            );
        }
    }

    #[test]
    fn concurrent_file_races_deduplicate_or_fail_closed_without_deletion_or_disclosure() {
        let publication_root = TestRoot::new("concurrent-publication");
        let payload = b"concurrent-identical-private-payload";
        let mut first = publication_root.store();
        let mut second = publication_root.store();
        let (first_staged, observation) = first
            .stage(
                &artifact_id("artifact-concurrent-first"),
                &mut Cursor::new(payload),
                payload.len() as u64,
            )
            .expect("first concurrent payload stages");
        let (second_staged, second_observation) = second
            .stage(
                &artifact_id("artifact-concurrent-second"),
                &mut Cursor::new(payload),
                payload.len() as u64,
            )
            .expect("second concurrent payload stages");
        assert_eq!(observation, second_observation);

        let placement_barrier = Arc::new(Barrier::new(2));
        for store in [&mut first, &mut second] {
            let barrier = Arc::clone(&placement_barrier);
            store.set_race_hook(Arc::new(move |point| {
                if point == ArtifactRacePoint::PlaceBeforeRename {
                    barrier.wait();
                }
            }));
        }
        let first_thread = std::thread::spawn(move || first.place(first_staged, &observation));
        let second_expected = second_observation.clone();
        let second_thread =
            std::thread::spawn(move || second.place(second_staged, &second_expected));
        let first_placement = first_thread
            .join()
            .expect("first placement thread joins")
            .expect("first concurrent placement resolves");
        let second_placement = second_thread
            .join()
            .expect("second placement thread joins")
            .expect("second concurrent placement resolves");
        assert_ne!(first_placement.deduplicated, second_placement.deduplicated);
        let verifier = publication_root.store();
        assert_eq!(
            verifier
                .read_complete(&first_placement.observation, payload.len() as u64)
                .expect("concurrent object reads exactly"),
            payload
        );
        assert_eq!(
            publication_root
                .objects()
                .read_dir()
                .expect("concurrent objects list")
                .count(),
            1
        );
        assert_eq!(
            publication_root
                .staging()
                .read_dir()
                .expect("concurrent staging lists")
                .count(),
            0
        );

        let read_root = TestRoot::new("concurrent-read-namespace");
        let mut reader = read_root.store();
        let read_payload = b"authorized-read-must-not-escape";
        let read_observation =
            stage_and_place(&mut reader, "artifact-concurrent-read", read_payload);
        let read_target = read_root.objects();
        let read_displaced = read_target.with_extension("concurrent-displaced");
        let read_reached = Arc::new(Barrier::new(2));
        let read_changed = Arc::new(Barrier::new(2));
        let hook_reached = Arc::clone(&read_reached);
        let hook_changed = Arc::clone(&read_changed);
        reader.set_race_hook(Arc::new(move |point| {
            if point == ArtifactRacePoint::ReadCompleteAfterRevalidate {
                hook_reached.wait();
                hook_changed.wait();
            }
        }));
        let attacker = std::thread::spawn(move || {
            read_reached.wait();
            fs::rename(&read_target, &read_displaced)
                .expect("read namespace displaces concurrently");
            fs::create_dir(&read_target).expect("read substitute namespace creates");
            fs::set_permissions(&read_target, fs::Permissions::from_mode(0o700))
                .expect("read substitute namespace is private");
            read_changed.wait();
            read_displaced
        });
        assert_eq!(
            reader.read_complete(&read_observation, read_payload.len() as u64),
            Err(RuntimeArtifactPayloadError::UnsafeRoot)
        );
        let read_displaced = attacker.join().expect("read attacker joins");
        assert!(
            read_displaced
                .join(&read_observation.payload_sha256)
                .is_file(),
            "the authorized encrypted object remains in the held displaced namespace"
        );
        assert_eq!(
            read_root
                .objects()
                .read_dir()
                .expect("read substitute lists")
                .count(),
            0,
            "the substitute namespace receives no payload effect"
        );

        let delete_root = TestRoot::new("concurrent-delete-object");
        let mut deleter = delete_root.store();
        let delete_observation = stage_and_place(
            &mut deleter,
            "artifact-concurrent-delete",
            b"authorized-delete-target",
        );
        let delete_target = delete_root
            .objects()
            .join(&delete_observation.payload_sha256);
        let delete_displaced = delete_root.path().join("authorized-object-displaced");
        let substitute = b"untrusted-substitute-must-not-be-deleted".to_vec();
        let delete_reached = Arc::new(Barrier::new(2));
        let delete_changed = Arc::new(Barrier::new(2));
        let hook_reached = Arc::clone(&delete_reached);
        let hook_changed = Arc::clone(&delete_changed);
        deleter.set_race_hook(Arc::new(move |point| {
            if point == ArtifactRacePoint::IsolateBeforeRename {
                hook_reached.wait();
                hook_changed.wait();
            }
        }));
        let attacker_substitute = substitute.clone();
        let attacker = std::thread::spawn(move || {
            delete_reached.wait();
            fs::rename(&delete_target, &delete_displaced)
                .expect("authorized object displaces concurrently");
            fs::write(&delete_target, &attacker_substitute).expect("substitute object creates");
            fs::set_permissions(&delete_target, fs::Permissions::from_mode(0o600))
                .expect("substitute object is private");
            delete_changed.wait();
            delete_displaced
        });
        assert_eq!(
            deleter.delete(&delete_observation),
            Err(RuntimeArtifactPayloadError::Conflict)
        );
        let delete_displaced = attacker.join().expect("delete attacker joins");
        assert!(
            delete_displaced.is_file(),
            "authorized ciphertext is not deleted"
        );
        let quarantined = delete_root
            .quarantine()
            .read_dir()
            .expect("quarantine lists")
            .map(|entry| entry.expect("quarantine entry reads").path())
            .collect::<Vec<_>>();
        assert_eq!(quarantined.len(), 1);
        assert_eq!(
            fs::read(&quarantined[0]).expect("substitute remains recoverable"),
            substitute,
            "the conflicting substitute is contained rather than deleted"
        );
    }

    #[test]
    fn repository_evidence_namespace_is_never_private_artifact_authority() {
        let root = TestRoot::new("repository-evidence-confusion");
        let mut store = root.store();
        let private = b"private-runtime-payload";
        let observation = stage_and_place(&mut store, "artifact-private", private);
        let repository_evidence = root.path().join("artifacts/sprints/sprint-22");
        fs::create_dir_all(&repository_evidence).expect("repository evidence directory creates");
        fs::set_permissions(
            root.path().join("artifacts"),
            fs::Permissions::from_mode(0o700),
        )
        .expect("repository artifacts directory is private");
        fs::set_permissions(
            root.path().join("artifacts/sprints"),
            fs::Permissions::from_mode(0o700),
        )
        .expect("repository sprints directory is private");
        fs::set_permissions(&repository_evidence, fs::Permissions::from_mode(0o700))
            .expect("repository evidence directory is private");
        let confusing = repository_evidence.join(&observation.payload_sha256);
        fs::write(&confusing, b"executable repository evidence")
            .expect("repository evidence writes");
        fs::set_permissions(&confusing, fs::Permissions::from_mode(0o700))
            .expect("repository evidence becomes executable");

        assert_eq!(
            store.inventory().expect("private inventory remains exact"),
            [RuntimeArtifactPayloadInventoryEntry {
                payload_sha256: observation.payload_sha256.clone(),
                byte_size: private.len() as u64,
                integrity: RuntimeArtifactPayloadInventoryIntegrity::Verified,
            }]
        );
        assert_eq!(
            store
                .read_complete(&observation, private.len() as u64)
                .expect("private payload reads"),
            private
        );
        assert!(confusing.exists());
    }

    #[test]
    #[ignore = "explicit Story 22.2 native maximum-payload and checkpoint-ceiling campaign"]
    fn story_22_2_native_artifact_pressure_reaches_declared_ceilings() {
        const PAGE_BYTES: u32 = 4 * 1024;
        const MIXED_OBJECT_COUNT: usize = 64;
        const MAX_CAMPAIGN_MS: u64 = 300_000;
        const MAX_RESIDENT_DELTA_KIB: u64 = 512 * 1024;
        const MAX_RETAINED_DISK_BYTES: u64 = 128 * 1024 * 1024;

        let root = TestRoot::new("pressure-ceilings");
        prepare_artifact_crash_fixture(root.path());
        let resident_start_kib = resident_memory_kib();
        let total_started = Instant::now();
        let (mut runtime, mut payloads) = open_artifact_crash_components(root.path(), 2);

        let maximum_payload = vec![0xa5; MAX_RUNTIME_ARTIFACT_BYTES as usize];
        let maximum_manifest = artifact_pressure_manifest(
            "artifact-pressure-22-2-maximum",
            maximum_payload.as_slice(),
        );
        let maximum_started = Instant::now();
        let maximum_publication = runtime
            .publish_runtime_artifact(
                &mut payloads,
                maximum_manifest.clone(),
                &mut Cursor::new(maximum_payload.as_slice()),
            )
            .expect("maximum payload publishes");
        let first_page = runtime
            .read_runtime_artifact_page(
                &payloads,
                &RuntimeArtifactPageRequest {
                    session_id: maximum_manifest.session_id.clone(),
                    task_id: maximum_manifest.task_id.clone(),
                    policy_sha256: maximum_manifest.policy_sha256.clone(),
                    reference: maximum_publication.reference.clone(),
                    now_epoch_ms: 3,
                    offset: 0,
                    maximum_bytes: PAGE_BYTES,
                },
            )
            .expect("maximum first page reads");
        let final_offset = MAX_RUNTIME_ARTIFACT_BYTES - u64::from(PAGE_BYTES);
        let final_page = runtime
            .read_runtime_artifact_page(
                &payloads,
                &RuntimeArtifactPageRequest {
                    session_id: maximum_manifest.session_id.clone(),
                    task_id: maximum_manifest.task_id.clone(),
                    policy_sha256: maximum_manifest.policy_sha256.clone(),
                    reference: maximum_publication.reference.clone(),
                    now_epoch_ms: 3,
                    offset: final_offset,
                    maximum_bytes: PAGE_BYTES,
                },
            )
            .expect("maximum final page reads");
        assert_eq!(first_page.bytes, vec![0xa5; PAGE_BYTES as usize]);
        assert_eq!(first_page.next_offset, Some(u64::from(PAGE_BYTES)));
        assert!(!first_page.complete);
        assert_eq!(final_page.bytes, vec![0xa5; PAGE_BYTES as usize]);
        assert_eq!(final_page.next_offset, None);
        assert!(final_page.complete);
        let maximum_elapsed_ms = elapsed_ms(maximum_started);
        let maximum_disk_bytes = recursive_directory_bytes(root.path());
        let resident_after_maximum_kib = resident_memory_kib();
        runtime
            .release_runtime_artifact(
                &maximum_manifest.session_id,
                &maximum_manifest.task_id,
                &maximum_manifest.policy_sha256,
                &maximum_publication.reference,
                4,
            )
            .expect("maximum reference releases");
        assert_eq!(
            runtime
                .reconcile_runtime_artifacts(&mut payloads, 4)
                .expect("maximum payload collects")
                .deleted_orphans,
            1
        );
        drop(maximum_payload);

        let mixed_started = Instant::now();
        let mut mixed_references = Vec::with_capacity(MIXED_OBJECT_COUNT);
        let mut mixed_total_payload_bytes = 0_u64;
        for index in 0..MIXED_OBJECT_COUNT {
            let byte_size = (index + 1) * 1024;
            let payload = vec![(index + 1) as u8; byte_size];
            mixed_total_payload_bytes += byte_size as u64;
            let manifest = artifact_pressure_manifest(
                &format!("artifact-pressure-22-2-mixed-{index:03}"),
                &payload,
            );
            let publication = runtime
                .publish_runtime_artifact(
                    &mut payloads,
                    manifest,
                    &mut Cursor::new(payload.as_slice()),
                )
                .expect("mixed unique payload publishes");
            assert!(!publication.payload_deduplicated);
            mixed_references.push(publication.reference);
        }
        assert_eq!(
            payloads.inventory().expect("mixed inventory reads").len(),
            MIXED_OBJECT_COUNT
        );
        let mixed_publish_elapsed_ms = elapsed_ms(mixed_started);
        let mixed_disk_bytes = recursive_directory_bytes(root.path());
        let resident_after_mixed_kib = resident_memory_kib();

        let mixed_collection_started = Instant::now();
        for reference in &mixed_references {
            runtime
                .release_runtime_artifact(
                    &maximum_manifest.session_id,
                    &maximum_manifest.task_id,
                    &maximum_manifest.policy_sha256,
                    reference,
                    5,
                )
                .expect("mixed unique reference releases");
        }
        let mixed_collection = runtime
            .reconcile_runtime_artifacts(&mut payloads, 5)
            .expect("mixed unique payloads collect");
        assert_eq!(mixed_collection.deleted_orphans, MIXED_OBJECT_COUNT as u64);
        assert_eq!(mixed_collection.quarantined_payloads, 0);
        assert!(
            payloads
                .inventory()
                .expect("post-mixed inventory reads")
                .is_empty()
        );
        let mixed_collection_elapsed_ms = elapsed_ms(mixed_collection_started);

        let shared_payload = b"deduplicated-pressure-payload";
        let reference_started = Instant::now();
        let mut references = Vec::with_capacity(MAX_RUNTIME_ARTIFACTS_PER_CHECKPOINT);
        let mut deduplicated = 0_usize;
        for index in 0..MAX_RUNTIME_ARTIFACTS_PER_CHECKPOINT {
            let manifest = artifact_pressure_manifest(
                &format!("artifact-pressure-22-2-{index:04}"),
                shared_payload,
            );
            let publication = runtime
                .publish_runtime_artifact(&mut payloads, manifest, &mut Cursor::new(shared_payload))
                .expect("pressure reference publishes");
            deduplicated += usize::from(publication.payload_deduplicated);
            references.push(publication.reference);
        }
        assert_eq!(deduplicated, MAX_RUNTIME_ARTIFACTS_PER_CHECKPOINT - 1);
        assert_eq!(
            payloads.inventory().expect("deduplicated inventory").len(),
            1
        );
        let checkpoint = artifact_pressure_checkpoint("checkpoint-pressure-22-2-full");
        let binding = artifact_pressure_binding(&runtime, &checkpoint, references.clone());
        let mut overflow = binding.clone();
        overflow.artifacts.push(
            runtime_artifact_ref(&artifact_pressure_manifest(
                "artifact-pressure-22-2-1024",
                shared_payload,
            ))
            .expect("overflow reference projects"),
        );
        overflow.binding_sha256 = repeated_digest('0');
        assert_eq!(
            seal_runtime_resume_binding(overflow),
            Err(RuntimeArtifactError::InvalidResumeBinding)
        );
        runtime
            .checkpoint_runtime_session(&checkpoint, &binding)
            .expect("ceiling checkpoint commits");
        let checkpoint_rows = runtime
            .runtime_artifact_row_counts()
            .expect("checkpoint artifact rows count");
        assert_eq!(checkpoint_rows.payloads, 66);
        assert_eq!(checkpoint_rows.artifacts, 1_089);
        assert_eq!(checkpoint_rows.lifecycle_events, 1_219);
        assert_eq!(checkpoint_rows.resume_bindings, 1);
        assert_eq!(checkpoint_rows.resume_artifacts, 1_024);
        let reference_elapsed_ms = elapsed_ms(reference_started);
        let reference_disk_bytes = recursive_directory_bytes(root.path());
        let resident_after_references_kib = resident_memory_kib();
        drop(payloads);
        drop(runtime);

        let reopen_started = Instant::now();
        let (mut runtime, mut payloads) = open_artifact_crash_components(root.path(), 5);
        assert_eq!(
            runtime
                .reconcile_runtime_artifacts(&mut payloads, 5)
                .expect("ceiling checkpoint reopens")
                .verified_payloads,
            1
        );
        assert_eq!(
            runtime
                .current_runtime_resume_binding()
                .expect("ceiling binding loads"),
            Some(binding)
        );
        assert!(
            runtime
                .release_runtime_artifact(
                    &maximum_manifest.session_id,
                    &maximum_manifest.task_id,
                    &maximum_manifest.policy_sha256,
                    &references[0],
                    6,
                )
                .is_err(),
            "a current checkpoint root must prevent release"
        );
        let empty_checkpoint = artifact_pressure_checkpoint("checkpoint-pressure-22-2-empty");
        let empty_binding = artifact_pressure_binding(&runtime, &empty_checkpoint, Vec::new());
        runtime
            .checkpoint_runtime_session(&empty_checkpoint, &empty_binding)
            .expect("empty successor checkpoint commits");
        let reopen_elapsed_ms = elapsed_ms(reopen_started);

        let collection_started = Instant::now();
        for reference in &references {
            runtime
                .release_runtime_artifact(
                    &maximum_manifest.session_id,
                    &maximum_manifest.task_id,
                    &maximum_manifest.policy_sha256,
                    reference,
                    7,
                )
                .expect("pressure reference releases");
        }
        let collection = runtime
            .reconcile_runtime_artifacts(&mut payloads, 8)
            .expect("pressure payload collects");
        assert_eq!(collection.deleted_orphans, 1);
        assert_eq!(collection.quarantined_payloads, 0);
        assert!(payloads.inventory().expect("final inventory").is_empty());
        for reference in [&references[0], references.last().expect("last reference")] {
            let state = runtime
                .runtime_artifact_state(reference)
                .expect("collected reference state");
            assert_eq!(state.lifecycle, RuntimeArtifactLifecycleState::Deleted);
            assert_eq!(state.integrity, RuntimeArtifactIntegrityState::Deleted);
        }
        let collection_elapsed_ms = elapsed_ms(collection_started);
        drop(payloads);
        drop(runtime);

        let (mut runtime, mut payloads) = open_artifact_crash_components(root.path(), 9);
        assert_eq!(
            runtime
                .reconcile_runtime_artifacts(&mut payloads, 9)
                .expect("final reopen is idempotent"),
            Default::default()
        );
        assert_eq!(
            runtime
                .current_runtime_resume_binding()
                .expect("empty binding remains current"),
            Some(empty_binding)
        );
        let final_rows = runtime
            .runtime_artifact_row_counts()
            .expect("final artifact rows count");
        assert_eq!(final_rows.payloads, 66);
        assert_eq!(final_rows.artifacts, 1_089);
        assert_eq!(final_rows.lifecycle_events, 3_267);
        assert_eq!(final_rows.resume_bindings, 2);
        assert_eq!(final_rows.resume_artifacts, 1_024);
        let final_disk_bytes = recursive_directory_bytes(root.path());
        let resident_peak_kib = resident_after_maximum_kib
            .max(resident_after_mixed_kib)
            .max(resident_after_references_kib);
        let resident_delta_kib = resident_peak_kib.saturating_sub(resident_start_kib);
        let total_elapsed_ms = elapsed_ms(total_started);

        assert!(maximum_elapsed_ms <= MAX_CAMPAIGN_MS);
        assert!(mixed_publish_elapsed_ms <= MAX_CAMPAIGN_MS);
        assert!(mixed_collection_elapsed_ms <= MAX_CAMPAIGN_MS);
        assert!(reference_elapsed_ms <= MAX_CAMPAIGN_MS);
        assert!(reopen_elapsed_ms <= MAX_CAMPAIGN_MS);
        assert!(collection_elapsed_ms <= MAX_CAMPAIGN_MS);
        assert!(total_elapsed_ms <= MAX_CAMPAIGN_MS);
        assert!(resident_delta_kib <= MAX_RESIDENT_DELTA_KIB);
        assert!(maximum_disk_bytes <= MAX_RETAINED_DISK_BYTES);
        assert!(mixed_disk_bytes <= MAX_RETAINED_DISK_BYTES);
        assert!(reference_disk_bytes <= MAX_RETAINED_DISK_BYTES);
        assert!(final_disk_bytes <= MAX_RETAINED_DISK_BYTES);

        println!(
            "AGENTMAGE_ARTIFACT_PRESSURE={}",
            serde_json::json!({
                "maximum_payload_bytes": MAX_RUNTIME_ARTIFACT_BYTES,
                "page_bytes": PAGE_BYTES,
                "mixed_object_count": MIXED_OBJECT_COUNT,
                "mixed_total_payload_bytes": mixed_total_payload_bytes,
                "mixed_publish_elapsed_ms": mixed_publish_elapsed_ms,
                "mixed_collection_elapsed_ms": mixed_collection_elapsed_ms,
                "mixed_disk_bytes": mixed_disk_bytes,
                "mixed_final_active_object_count": 0,
                "checkpoint_reference_count": references.len(),
                "checkpoint_payload_rows": checkpoint_rows.payloads,
                "checkpoint_artifact_rows": checkpoint_rows.artifacts,
                "checkpoint_lifecycle_event_rows": checkpoint_rows.lifecycle_events,
                "checkpoint_resume_binding_rows": checkpoint_rows.resume_bindings,
                "checkpoint_resume_artifact_rows": checkpoint_rows.resume_artifacts,
                "overflow_reference_count_rejected": references.len() + 1,
                "deduplicated_reference_count": deduplicated,
                "active_object_count_at_checkpoint": 1,
                "final_active_object_count": 0,
                "final_payload_rows": final_rows.payloads,
                "final_artifact_rows": final_rows.artifacts,
                "final_lifecycle_event_rows": final_rows.lifecycle_events,
                "final_resume_binding_rows": final_rows.resume_bindings,
                "final_resume_artifact_rows": final_rows.resume_artifacts,
                "maximum_elapsed_ms": maximum_elapsed_ms,
                "reference_elapsed_ms": reference_elapsed_ms,
                "reopen_elapsed_ms": reopen_elapsed_ms,
                "collection_elapsed_ms": collection_elapsed_ms,
                "total_elapsed_ms": total_elapsed_ms,
                "resident_start_kib": resident_start_kib,
                "resident_peak_kib": resident_peak_kib,
                "resident_delta_kib": resident_delta_kib,
                "maximum_disk_bytes": maximum_disk_bytes,
                "reference_disk_bytes": reference_disk_bytes,
                "final_disk_bytes": final_disk_bytes,
                "latency_ceiling_ms": MAX_CAMPAIGN_MS,
                "resident_delta_ceiling_kib": MAX_RESIDENT_DELTA_KIB,
                "retained_disk_ceiling_bytes": MAX_RETAINED_DISK_BYTES,
                "checkpoint_release_blocked": true,
                "final_reopen_verified": true,
                "external_network_used": false,
                "manual_fuzzing_executed": false,
            })
        );
    }

    #[test]
    #[ignore = "subprocess stop target; invoked only by the Story 22.2 artifact crash matrix"]
    fn story_22_2_native_artifact_crash_boundary_child() {
        if env::var_os("AGENTMAGE_ARTIFACT_CRASH_CHILD").is_none() {
            return;
        }
        let path = PathBuf::from(
            env::var_os("AGENTMAGE_ARTIFACT_CRASH_ROOT").expect("artifact crash root"),
        );
        let boundary = ArtifactCrashBoundary::from_code(
            &env::var("AGENTMAGE_ARTIFACT_CRASH_BOUNDARY").expect("artifact crash boundary"),
        );
        let position = ArtifactCrashPosition::from_code(
            &env::var("AGENTMAGE_ARTIFACT_CRASH_POSITION").expect("artifact crash position"),
        );
        run_artifact_crash_child(&path, boundary, position);
    }

    #[test]
    fn story_22_2_native_crash_matrix_reconciles_every_artifact_boundary() {
        let manifest = artifact_crash_manifest();
        let reference = runtime_artifact_ref(&manifest).expect("matrix reference projects");
        let mut cases = Vec::new();

        for boundary in ArtifactCrashBoundary::ALL {
            for position in ArtifactCrashPosition::ALL {
                let root = TestRoot::new("crash-matrix");
                prepare_artifact_crash_fixture(root.path());
                let output = Command::new(env::current_exe().expect("current test executable"))
                    .args([
                        "--exact",
                        "runtime_artifact_store::tests::story_22_2_native_artifact_crash_boundary_child",
                        "--ignored",
                        "--nocapture",
                    ])
                    .env("AGENTMAGE_ARTIFACT_CRASH_CHILD", "1")
                    .env("AGENTMAGE_ARTIFACT_CRASH_ROOT", root.path())
                    .env("AGENTMAGE_ARTIFACT_CRASH_BOUNDARY", boundary.code())
                    .env("AGENTMAGE_ARTIFACT_CRASH_POSITION", position.code())
                    .output()
                    .expect("artifact crash child launches");
                assert_eq!(
                    output.status.code(),
                    Some(ARTIFACT_CRASH_CHILD_EXIT),
                    "{boundary:?} {position:?}: {}",
                    String::from_utf8_lossy(&output.stderr)
                );

                let staged_before_recovery = fs::read_dir(root.staging())
                    .expect("staging lists before recovery")
                    .count();
                let (mut runtime, mut payloads) = open_artifact_crash_components(root.path(), 6);
                let inventory_before_recovery =
                    payloads.inventory().expect("pre-recovery inventory").len();
                let report = runtime
                    .reconcile_runtime_artifacts(&mut payloads, 7)
                    .expect("artifact recovery reconciles");
                let inventory_after_recovery =
                    payloads.inventory().expect("post-recovery inventory").len();
                let state = runtime.runtime_artifact_state(&reference).ok();
                let events = runtime
                    .runtime_events(&manifest.producer_run_id)
                    .expect("recovered events load");
                let artifact_event_count = events
                    .iter()
                    .filter(|event| matches!(event.kind, RuntimeEventKind::ArtifactCreated { .. }))
                    .count();
                let false_terminal_count = events
                    .iter()
                    .filter(|event| matches!(event.kind, RuntimeEventKind::RunTerminal { .. }))
                    .count();
                let resume_binding = runtime
                    .current_runtime_resume_binding()
                    .expect("resume binding projects");

                let metadata_expected = !matches!(
                    (boundary, position),
                    (ArtifactCrashBoundary::Staging, _)
                        | (ArtifactCrashBoundary::Placement, _)
                        | (
                            ArtifactCrashBoundary::MetadataCommit,
                            ArtifactCrashPosition::Before
                        )
                );
                let deleted_expected = matches!(
                    (boundary, position),
                    (
                        ArtifactCrashBoundary::ReferenceRelease,
                        ArtifactCrashPosition::After
                    ) | (ArtifactCrashBoundary::Collection, _)
                );
                let active_expected = metadata_expected && !deleted_expected;
                let artifact_event_expected = matches!(
                    (boundary, position),
                    (
                        ArtifactCrashBoundary::EventCommit,
                        ArtifactCrashPosition::After
                    ) | (ArtifactCrashBoundary::CheckpointCommit, _)
                        | (ArtifactCrashBoundary::ReferenceRelease, _)
                        | (ArtifactCrashBoundary::Collection, _)
                );
                let checkpoint_expected = matches!(
                    (boundary, position),
                    (
                        ArtifactCrashBoundary::CheckpointCommit,
                        ArtifactCrashPosition::After
                    )
                );
                let staged_expected = matches!(
                    (boundary, position),
                    (ArtifactCrashBoundary::Staging, ArtifactCrashPosition::After)
                        | (
                            ArtifactCrashBoundary::Placement,
                            ArtifactCrashPosition::Before
                        )
                );

                assert_eq!(staged_before_recovery, usize::from(staged_expected));
                assert_eq!(report.cleaned_staging, u64::from(staged_expected));
                assert_eq!(artifact_event_count, usize::from(artifact_event_expected));
                assert_eq!(false_terminal_count, 0);
                assert_eq!(resume_binding.is_some(), checkpoint_expected);
                if let Some(binding) = &resume_binding {
                    assert_eq!(
                        binding.artifacts.as_slice(),
                        std::slice::from_ref(&reference)
                    );
                    assert_eq!(binding.event_cursor.run_id, manifest.producer_run_id);
                }
                match state {
                    Some(state) if active_expected => {
                        assert_eq!(state.lifecycle, RuntimeArtifactLifecycleState::Active);
                        assert_eq!(state.integrity, RuntimeArtifactIntegrityState::Verified);
                        assert_eq!(inventory_after_recovery, 1);
                    }
                    Some(state) if deleted_expected => {
                        assert_eq!(state.lifecycle, RuntimeArtifactLifecycleState::Deleted);
                        assert_eq!(state.integrity, RuntimeArtifactIntegrityState::Deleted);
                        assert_eq!(inventory_after_recovery, 0);
                    }
                    None if !metadata_expected => {
                        assert_eq!(inventory_after_recovery, 0);
                    }
                    unexpected => panic!(
                        "unexpected recovered state for {boundary:?} {position:?}: {unexpected:?}"
                    ),
                }

                cases.push(serde_json::json!({
                    "boundary": boundary.code(),
                    "position": position.code(),
                    "staged_before_recovery": staged_before_recovery,
                    "inventory_before_recovery": inventory_before_recovery,
                    "inventory_after_recovery": inventory_after_recovery,
                    "recovery_cleaned_staging": report.cleaned_staging,
                    "recovery_verified_payloads": report.verified_payloads,
                    "recovery_quarantined_payloads": report.quarantined_payloads,
                    "recovery_deleted_orphans": report.deleted_orphans,
                    "artifact_event_count": artifact_event_count,
                    "checkpoint_bound": resume_binding.is_some(),
                    "metadata_expected": metadata_expected,
                    "active_expected": active_expected,
                    "deleted_expected": deleted_expected,
                    "false_terminal_count": false_terminal_count,
                }));
            }
        }

        assert_eq!(cases.len(), 14);
        println!(
            "AGENTMAGE_ARTIFACT_CRASH_MATRIX={}",
            serde_json::json!({
                "boundary_count": ArtifactCrashBoundary::ALL.len(),
                "position_count": ArtifactCrashPosition::ALL.len(),
                "case_count": cases.len(),
                "cases": cases,
                "external_network_used": false,
                "false_terminal_count": 0,
                "manual_fuzzing_executed": false,
            })
        );
    }
}
