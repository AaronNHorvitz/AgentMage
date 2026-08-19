//! Descriptor-relative inventory of write-adjacent workspace and strict-local roots.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::File;
use std::io::Read;

use rustix::fd::OwnedFd;
use rustix::fs::{Dir, FileType, Mode, OFlags, fstat, openat};
use sha2::{Digest, Sha256};

use crate::{LinuxAuthorizedWorkspace, LinuxStrictLocalRoot};

const HASH_BUFFER_BYTES: usize = 64 * 1024;

/// One closed class of root scanned for write residue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LinuxWriteRootClass {
    /// The exact authorized user workspace.
    Workspace,
    /// The private configuration root.
    Configuration,
    /// The private authority and runtime-state root.
    State,
}

impl LinuxWriteRootClass {
    fn code(self) -> &'static [u8] {
        match self {
            Self::Workspace => b"workspace",
            Self::Configuration => b"configuration",
            Self::State => b"state",
        }
    }
}

/// One strict-local root included in a complete write-root scan.
pub struct LinuxWriteStrictRoot<'root> {
    class: LinuxWriteRootClass,
    root: &'root LinuxStrictLocalRoot,
}

impl<'root> LinuxWriteStrictRoot<'root> {
    /// Declares a configuration or state root for one scan.
    #[must_use]
    pub const fn new(class: LinuxWriteRootClass, root: &'root LinuxStrictLocalRoot) -> Self {
        Self { class, root }
    }
}

/// Retention role assigned to one expected reserved write object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxWriteObjectRole {
    /// A temporary object owned by a live transaction.
    ActiveTemporary,
    /// Exact prestate retained for a promised rollback.
    RetainedRollback,
    /// Durable content-addressed runtime data.
    DurableObject,
    /// Material isolated for explicit review.
    Quarantined,
}

/// Content-free declaration for one expected reserved write object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxDeclaredWriteObject {
    /// SHA-256 identity of its root class and relative component bytes.
    pub identity_sha256: String,
    /// SHA-256 of the exact expected bytes.
    pub content_sha256: String,
    /// Declared retention role.
    pub role: LinuxWriteObjectRole,
    /// Exclusive expiry for temporary or rollback material; zero means no expiry.
    pub expires_at_epoch_ms: u64,
}

/// Fixed limits for one complete root scan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxWriteRootScanLimits {
    /// Maximum entries across every root.
    pub maximum_entries: usize,
    /// Maximum recursive directory depth.
    pub maximum_depth: usize,
    /// Maximum bytes hashed from one reserved file.
    pub maximum_reserved_file_bytes: u64,
}

impl Default for LinuxWriteRootScanLimits {
    fn default() -> Self {
        Self {
            maximum_entries: 131_072,
            maximum_depth: 64,
            maximum_reserved_file_bytes: 64 * 1024 * 1024,
        }
    }
}

/// Content-free result of a complete declared-root residue scan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxWriteRootScanReport {
    /// Number of distinct roots scanned and revalidated.
    pub roots_scanned: usize,
    /// Number of filesystem entries observed.
    pub entries_scanned: usize,
    /// Number of reserved AgentMage objects observed.
    pub reserved_objects_observed: usize,
    /// Undeclared temporary objects with no known live owner.
    pub orphan_staging_objects: usize,
    /// Declared temporary or rollback objects retained past expiry.
    pub expired_objects: usize,
    /// Undeclared durable, rollback, quarantine, or unknown reserved objects.
    pub undeclared_copies: usize,
    /// Missing, linked, special, unreadable, mismatched, or cross-device protected material.
    pub inaccessible_or_mismatched_objects: usize,
    /// Digest of the sorted content-free observation inventory.
    pub inventory_sha256: String,
}

impl LinuxWriteRootScanReport {
    /// Returns true only when no residue or protected-material violation was found.
    #[must_use]
    pub const fn is_clean(&self) -> bool {
        self.orphan_staging_objects == 0
            && self.expired_objects == 0
            && self.undeclared_copies == 0
            && self.inaccessible_or_mismatched_objects == 0
    }
}

/// Stable failure from a bounded root inventory operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxWriteRootScanErrorKind {
    /// A root class, declaration, or bound was malformed or duplicated.
    InvalidManifest,
    /// A held root changed identity or became unsafe.
    RootChanged,
    /// The scan could not enumerate or inspect the complete tree.
    ScanFailed,
    /// A fixed scan limit was exceeded.
    LimitExceeded,
}

/// Content-free root-scan failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxWriteRootScanError {
    kind: LinuxWriteRootScanErrorKind,
}

impl LinuxWriteRootScanError {
    /// Returns the stable failure class.
    #[must_use]
    pub const fn kind(self) -> LinuxWriteRootScanErrorKind {
        self.kind
    }
}

impl fmt::Display for LinuxWriteRootScanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            LinuxWriteRootScanErrorKind::InvalidManifest => "linux.write_roots.invalid_manifest",
            LinuxWriteRootScanErrorKind::RootChanged => "linux.write_roots.root_changed",
            LinuxWriteRootScanErrorKind::ScanFailed => "linux.write_roots.scan_failed",
            LinuxWriteRootScanErrorKind::LimitExceeded => "linux.write_roots.limit_exceeded",
        })
    }
}

impl std::error::Error for LinuxWriteRootScanError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReservedKind {
    Temporary,
    Rollback,
    Durable,
    Quarantine,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ReservedObservation {
    identity_sha256: String,
    content_sha256: Option<String>,
    kind: ReservedKind,
    inaccessible: bool,
}

/// Scans the exact workspace plus one configuration and one state root without following links.
///
/// The report contains no raw path. Every noncanonical reserved AgentMage object must have an exact
/// content-bound declaration; absent active temporaries are treated as successfully cleaned.
pub fn scan_linux_write_roots(
    workspace: &LinuxAuthorizedWorkspace,
    strict_roots: &[LinuxWriteStrictRoot<'_>],
    declarations: &[LinuxDeclaredWriteObject],
    now_epoch_ms: u64,
    limits: LinuxWriteRootScanLimits,
) -> Result<LinuxWriteRootScanReport, LinuxWriteRootScanError> {
    validate_inputs(strict_roots, declarations, limits)?;
    workspace
        .revalidate()
        .map_err(|_| error(LinuxWriteRootScanErrorKind::RootChanged))?;
    for root in strict_roots {
        root.root
            .revalidate()
            .map_err(|_| error(LinuxWriteRootScanErrorKind::RootChanged))?;
    }

    let mut observations = Vec::new();
    let mut entries_scanned = 0_usize;
    let workspace_descriptor = workspace
        .reopen_root_directory()
        .map_err(|_| error(LinuxWriteRootScanErrorKind::RootChanged))?;
    scan_root(
        LinuxWriteRootClass::Workspace,
        workspace_descriptor,
        &mut observations,
        &mut entries_scanned,
        limits,
    )?;
    for root in strict_roots {
        let descriptor = root
            .root
            .duplicate_io_descriptor()
            .map_err(|_| error(LinuxWriteRootScanErrorKind::RootChanged))?;
        scan_root(
            root.class,
            descriptor,
            &mut observations,
            &mut entries_scanned,
            limits,
        )?;
    }

    workspace
        .revalidate()
        .map_err(|_| error(LinuxWriteRootScanErrorKind::RootChanged))?;
    for root in strict_roots {
        root.root
            .revalidate()
            .map_err(|_| error(LinuxWriteRootScanErrorKind::RootChanged))?;
    }
    observations.sort_by(|left, right| left.identity_sha256.cmp(&right.identity_sha256));
    reconcile(
        strict_roots.len() + 1,
        entries_scanned,
        observations,
        declarations,
        now_epoch_ms,
    )
}

fn validate_inputs(
    roots: &[LinuxWriteStrictRoot<'_>],
    declarations: &[LinuxDeclaredWriteObject],
    limits: LinuxWriteRootScanLimits,
) -> Result<(), LinuxWriteRootScanError> {
    if roots.len() != 2
        || roots
            .iter()
            .any(|root| root.class == LinuxWriteRootClass::Workspace)
        || roots
            .iter()
            .map(|root| root.class)
            .collect::<BTreeSet<_>>()
            .len()
            != roots.len()
        || !roots
            .iter()
            .any(|root| root.class == LinuxWriteRootClass::Configuration)
        || !roots
            .iter()
            .any(|root| root.class == LinuxWriteRootClass::State)
        || roots
            .iter()
            .map(|root| root.root.observation().root_identity_sha256)
            .collect::<BTreeSet<_>>()
            .len()
            != roots.len()
        || limits.maximum_entries == 0
        || limits.maximum_depth == 0
        || limits.maximum_reserved_file_bytes == 0
        || declarations.len() > limits.maximum_entries
    {
        return Err(error(LinuxWriteRootScanErrorKind::InvalidManifest));
    }
    let mut identities = BTreeSet::new();
    for declaration in declarations {
        if !valid_sha256(&declaration.identity_sha256)
            || !valid_sha256(&declaration.content_sha256)
            || !identities.insert(declaration.identity_sha256.as_str())
            || (matches!(
                declaration.role,
                LinuxWriteObjectRole::ActiveTemporary | LinuxWriteObjectRole::RetainedRollback
            ) && declaration.expires_at_epoch_ms == 0)
            || (matches!(
                declaration.role,
                LinuxWriteObjectRole::DurableObject | LinuxWriteObjectRole::Quarantined
            ) && declaration.expires_at_epoch_ms != 0)
        {
            return Err(error(LinuxWriteRootScanErrorKind::InvalidManifest));
        }
    }
    Ok(())
}

fn scan_root(
    class: LinuxWriteRootClass,
    descriptor: OwnedFd,
    observations: &mut Vec<ReservedObservation>,
    entries_scanned: &mut usize,
    limits: LinuxWriteRootScanLimits,
) -> Result<(), LinuxWriteRootScanError> {
    let device = fstat(&descriptor)
        .map_err(|_| error(LinuxWriteRootScanErrorKind::ScanFailed))?
        .st_dev;
    scan_directory(
        class,
        descriptor,
        device,
        &mut Vec::new(),
        0,
        observations,
        entries_scanned,
        limits,
    )
}

#[allow(clippy::too_many_arguments)]
fn scan_directory(
    class: LinuxWriteRootClass,
    descriptor: OwnedFd,
    root_device: u64,
    components: &mut Vec<Vec<u8>>,
    depth: usize,
    observations: &mut Vec<ReservedObservation>,
    entries_scanned: &mut usize,
    limits: LinuxWriteRootScanLimits,
) -> Result<(), LinuxWriteRootScanError> {
    if depth > limits.maximum_depth {
        return Err(error(LinuxWriteRootScanErrorKind::LimitExceeded));
    }
    let directory =
        Dir::read_from(&descriptor).map_err(|_| error(LinuxWriteRootScanErrorKind::ScanFailed))?;
    for entry in directory {
        let entry = entry.map_err(|_| error(LinuxWriteRootScanErrorKind::ScanFailed))?;
        let name = entry.file_name().to_bytes();
        if matches!(name, b"." | b"..") {
            continue;
        }
        *entries_scanned = entries_scanned
            .checked_add(1)
            .filter(|count| *count <= limits.maximum_entries)
            .ok_or_else(|| error(LinuxWriteRootScanErrorKind::LimitExceeded))?;
        components.push(name.to_vec());
        let kind = classify_reserved(class, components);
        let opened = openat(
            &descriptor,
            entry.file_name(),
            OFlags::PATH | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        );
        let Ok(opened) = opened else {
            if let Some(kind) = kind {
                observations.push(inaccessible_observation(class, components, kind));
            }
            components.pop();
            continue;
        };
        let stat = fstat(&opened).map_err(|_| error(LinuxWriteRootScanErrorKind::ScanFailed))?;
        let file_type = FileType::from_raw_mode(stat.st_mode);
        if stat.st_dev != root_device {
            if let Some(kind) = kind {
                observations.push(inaccessible_observation(class, components, kind));
            }
        } else if file_type == FileType::Directory {
            let child = openat(
                &descriptor,
                entry.file_name(),
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            );
            if let Ok(child) = child {
                scan_directory(
                    class,
                    child,
                    root_device,
                    components,
                    depth + 1,
                    observations,
                    entries_scanned,
                    limits,
                )?;
            } else if let Some(kind) = kind {
                observations.push(inaccessible_observation(class, components, kind));
            }
        } else if let Some(kind) = kind {
            if file_type != FileType::RegularFile {
                observations.push(inaccessible_observation(class, components, kind));
            } else {
                let content_sha256 = match openat(
                    &descriptor,
                    entry.file_name(),
                    OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    Mode::empty(),
                ) {
                    Ok(file) => Some(hash_file(file, limits.maximum_reserved_file_bytes)?),
                    Err(_) => None,
                };
                let inaccessible = content_sha256.is_none();
                observations.push(ReservedObservation {
                    identity_sha256: path_identity(class, components),
                    content_sha256,
                    kind,
                    inaccessible,
                });
            }
        }
        components.pop();
    }
    Ok(())
}

fn reconcile(
    roots_scanned: usize,
    entries_scanned: usize,
    observations: Vec<ReservedObservation>,
    declarations: &[LinuxDeclaredWriteObject],
    now_epoch_ms: u64,
) -> Result<LinuxWriteRootScanReport, LinuxWriteRootScanError> {
    let declared: BTreeMap<_, _> = declarations
        .iter()
        .map(|item| (item.identity_sha256.as_str(), item))
        .collect();
    let observed_ids: BTreeSet<_> = observations
        .iter()
        .map(|item| item.identity_sha256.as_str())
        .collect();
    let mut orphan_staging_objects = 0;
    let mut expired_objects = 0;
    let mut undeclared_copies = 0;
    let mut inaccessible_or_mismatched_objects = 0;
    for item in &observations {
        let Some(expected) = declared.get(item.identity_sha256.as_str()) else {
            match item.kind {
                ReservedKind::Temporary => orphan_staging_objects += 1,
                _ => undeclared_copies += 1,
            }
            if item.inaccessible || item.content_sha256.is_none() {
                inaccessible_or_mismatched_objects += 1;
            }
            continue;
        };
        if item.inaccessible
            || item.content_sha256.as_deref() != Some(expected.content_sha256.as_str())
            || !role_matches(expected.role, item.kind)
        {
            inaccessible_or_mismatched_objects += 1;
        }
        if expected.expires_at_epoch_ms != 0 && now_epoch_ms >= expected.expires_at_epoch_ms {
            expired_objects += 1;
        }
    }
    for expected in declarations {
        if !observed_ids.contains(expected.identity_sha256.as_str())
            && expected.role != LinuxWriteObjectRole::ActiveTemporary
        {
            inaccessible_or_mismatched_objects += 1;
        }
    }
    let mut digest = Sha256::new();
    digest.update(b"agentmage.linux.write-root-inventory.v1\0");
    for item in &observations {
        digest.update(item.identity_sha256.as_bytes());
        digest.update(
            item.content_sha256
                .as_deref()
                .unwrap_or("unavailable")
                .as_bytes(),
        );
        digest.update([item.kind as u8, u8::from(item.inaccessible)]);
    }
    Ok(LinuxWriteRootScanReport {
        roots_scanned,
        entries_scanned,
        reserved_objects_observed: observations.len(),
        orphan_staging_objects,
        expired_objects,
        undeclared_copies,
        inaccessible_or_mismatched_objects,
        inventory_sha256: hex_digest(&digest.finalize()),
    })
}

fn classify_reserved(class: LinuxWriteRootClass, components: &[Vec<u8>]) -> Option<ReservedKind> {
    let name = components.last()?.as_slice();
    if canonical_name(class, components) {
        return None;
    }
    if class == LinuxWriteRootClass::Workspace && agentmage_write_temporary(name) {
        return Some(ReservedKind::Temporary);
    }
    if class != LinuxWriteRootClass::Workspace {
        if name.starts_with(b".agentmage-new-") && name.ends_with(b".tmp")
            || name.starts_with(b".agentmage-derived-export-") && name.ends_with(b".tmp")
            || name == b".agentmage-key-provision.lock"
        {
            return Some(ReservedKind::Temporary);
        }
        if name.starts_with(b".agentmage-backup-") && name.ends_with(b".json") {
            return Some(ReservedKind::Rollback);
        }
        if components.len() >= 3 && components[0] == b".agentmage-runtime-payloads-v1" {
            return match components[1].as_slice() {
                b"staging" => Some(ReservedKind::Temporary),
                b"objects" => Some(ReservedKind::Durable),
                b"quarantine" => Some(ReservedKind::Quarantine),
                _ => Some(ReservedKind::Unknown),
            };
        }
    }
    if name.starts_with(b".agentmage") {
        Some(ReservedKind::Unknown)
    } else {
        None
    }
}

fn canonical_name(class: LinuxWriteRootClass, components: &[Vec<u8>]) -> bool {
    if class == LinuxWriteRootClass::Workspace || components.len() != 1 {
        return false;
    }
    matches!(
        components[0].as_slice(),
        b"authority.db" | b"authority.db-wal" | b"authority.db-shm" | b"agentmage.json"
    )
}

fn agentmage_write_temporary(name: &[u8]) -> bool {
    name.starts_with(b".agentmage-write-") && name.ends_with(b".tmp")
}

fn role_matches(role: LinuxWriteObjectRole, kind: ReservedKind) -> bool {
    matches!(
        (role, kind),
        (
            LinuxWriteObjectRole::ActiveTemporary,
            ReservedKind::Temporary
        ) | (
            LinuxWriteObjectRole::RetainedRollback,
            ReservedKind::Rollback
        ) | (
            LinuxWriteObjectRole::RetainedRollback,
            ReservedKind::Temporary
        ) | (LinuxWriteObjectRole::DurableObject, ReservedKind::Durable)
            | (LinuxWriteObjectRole::Quarantined, ReservedKind::Quarantine)
    )
}

fn inaccessible_observation(
    class: LinuxWriteRootClass,
    components: &[Vec<u8>],
    kind: ReservedKind,
) -> ReservedObservation {
    ReservedObservation {
        identity_sha256: path_identity(class, components),
        content_sha256: None,
        kind,
        inaccessible: true,
    }
}

fn path_identity(class: LinuxWriteRootClass, components: &[Vec<u8>]) -> String {
    let mut digest = Sha256::new();
    digest.update(b"agentmage.linux.write-root-path.v1\0");
    digest.update(class.code());
    for component in components {
        digest.update((component.len() as u64).to_be_bytes());
        digest.update(component);
    }
    hex_digest(&digest.finalize())
}

fn hash_file(descriptor: OwnedFd, maximum: u64) -> Result<String, LinuxWriteRootScanError> {
    let mut file = File::from(descriptor);
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; HASH_BUFFER_BYTES];
    let mut total = 0_u64;
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|_| error(LinuxWriteRootScanErrorKind::ScanFailed))?;
        if count == 0 {
            break;
        }
        total = total
            .checked_add(count as u64)
            .filter(|value| *value <= maximum)
            .ok_or_else(|| error(LinuxWriteRootScanErrorKind::LimitExceeded))?;
        digest.update(&buffer[..count]);
    }
    Ok(hex_digest(&digest.finalize()))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        write!(&mut output, "{byte:02x}").expect("String writes cannot fail");
    }
    output
}

const fn error(kind: LinuxWriteRootScanErrorKind) -> LinuxWriteRootScanError {
    LinuxWriteRootScanError { kind }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::{AdapterInstanceId, WorkspaceAuthorizationId, WorkspaceId};
    use sha2::{Digest, Sha256};

    use super::{
        LinuxDeclaredWriteObject, LinuxWriteObjectRole, LinuxWriteRootClass,
        LinuxWriteRootScanErrorKind, LinuxWriteRootScanLimits, LinuxWriteStrictRoot, path_identity,
        scan_linux_write_roots,
    };
    use crate::{LinuxStrictLocalRootInspector, authorize_workspace_root};

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(1);

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new(label: &str) -> Self {
            let sequence = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "agentmage-write-root-scan-{}-{label}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("test root creates");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
                .expect("test root private");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("test root removes");
        }
    }

    fn digest(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    fn declaration(
        class: LinuxWriteRootClass,
        components: &[&[u8]],
        bytes: &[u8],
        role: LinuxWriteObjectRole,
        expires_at_epoch_ms: u64,
    ) -> LinuxDeclaredWriteObject {
        LinuxDeclaredWriteObject {
            identity_sha256: path_identity(
                class,
                &components
                    .iter()
                    .map(|value| value.to_vec())
                    .collect::<Vec<_>>(),
            ),
            content_sha256: digest(bytes),
            role,
            expires_at_epoch_ms,
        }
    }

    #[test]
    fn s_032_it01_scans_every_declared_root_and_retention_transition() {
        let workspace_root = TestRoot::new("workspace");
        let configuration_root = TestRoot::new("configuration");
        let state_root = TestRoot::new("state");
        fs::write(workspace_root.path().join("user.rs"), b"user-owned\n").expect("user file");
        fs::write(configuration_root.path().join("agentmage.json"), b"{}\n")
            .expect("configuration");
        fs::write(state_root.path().join("authority.db"), b"encrypted-fixture")
            .expect("authority fixture");
        fs::create_dir(state_root.path().join(".agentmage-runtime-payloads-v1"))
            .expect("artifact namespace");
        for child in ["staging", "objects", "quarantine"] {
            fs::create_dir(
                state_root
                    .path()
                    .join(".agentmage-runtime-payloads-v1")
                    .join(child),
            )
            .expect("artifact child");
        }

        let workspace = authorize_workspace_root(
            workspace_root.path(),
            WorkspaceId::from_raw("workspace-write-root-scan"),
            WorkspaceAuthorizationId::from_raw("authorization-write-root-scan"),
            AdapterInstanceId::from_raw("adapter-write-root-scan"),
        )
        .expect("workspace authorizes");
        let configuration = LinuxStrictLocalRootInspector::inspect(configuration_root.path())
            .expect("configuration root inspects");
        let state =
            LinuxStrictLocalRootInspector::inspect(state_root.path()).expect("state root inspects");
        let roots = [
            LinuxWriteStrictRoot::new(LinuxWriteRootClass::Configuration, &configuration),
            LinuxWriteStrictRoot::new(LinuxWriteRootClass::State, &state),
        ];

        let clean = scan_linux_write_roots(
            &workspace,
            &roots,
            &[],
            1_000,
            LinuxWriteRootScanLimits::default(),
        )
        .expect("clean roots scan");
        assert!(clean.is_clean());
        assert_eq!(clean.roots_scanned, 3);

        let stage_name = b".agentmage-write-00112233445566778899aabbccddeeff.tmp";
        let stage_bytes = b"reviewed postimage\n";
        fs::write(
            workspace_root
                .path()
                .join(std::str::from_utf8(stage_name).expect("ASCII")),
            stage_bytes,
        )
        .expect("staging fixture");
        let stage = declaration(
            LinuxWriteRootClass::Workspace,
            &[stage_name],
            stage_bytes,
            LinuxWriteObjectRole::ActiveTemporary,
            2_000,
        );
        let active = scan_linux_write_roots(
            &workspace,
            &roots,
            std::slice::from_ref(&stage),
            1_999,
            LinuxWriteRootScanLimits::default(),
        )
        .expect("active staging scans");
        assert!(active.is_clean());
        let expired = scan_linux_write_roots(
            &workspace,
            &roots,
            std::slice::from_ref(&stage),
            2_000,
            LinuxWriteRootScanLimits::default(),
        )
        .expect("expired staging scans");
        assert_eq!(expired.expired_objects, 1);
        assert!(!expired.is_clean());
        fs::remove_file(
            workspace_root
                .path()
                .join(std::str::from_utf8(stage_name).expect("ASCII")),
        )
        .expect("stage cleans");
        let cleaned = scan_linux_write_roots(
            &workspace,
            &roots,
            &[stage],
            2_001,
            LinuxWriteRootScanLimits::default(),
        )
        .expect("cleaned stage scans");
        assert!(cleaned.is_clean());

        let backup_name = b".agentmage-backup-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.json";
        let backup_bytes = b"reviewed rollback\n";
        fs::write(
            configuration_root
                .path()
                .join(std::str::from_utf8(backup_name).expect("ASCII")),
            backup_bytes,
        )
        .expect("rollback fixture");
        let backup = declaration(
            LinuxWriteRootClass::Configuration,
            &[backup_name],
            backup_bytes,
            LinuxWriteObjectRole::RetainedRollback,
            4_000,
        );
        assert!(
            scan_linux_write_roots(
                &workspace,
                &roots,
                std::slice::from_ref(&backup),
                3_000,
                LinuxWriteRootScanLimits::default(),
            )
            .expect("retained rollback scans")
            .is_clean()
        );
        fs::remove_file(
            configuration_root
                .path()
                .join(std::str::from_utf8(backup_name).expect("ASCII")),
        )
        .expect("rollback removed");
        let missing = scan_linux_write_roots(
            &workspace,
            &roots,
            &[backup],
            3_001,
            LinuxWriteRootScanLimits::default(),
        )
        .expect("missing rollback scans");
        assert_eq!(missing.inaccessible_or_mismatched_objects, 1);
    }

    #[test]
    fn s_032_it01_orphans_copies_links_mismatches_and_limits_fail_closed() {
        let workspace_root = TestRoot::new("adversarial-workspace");
        let configuration_root = TestRoot::new("adversarial-configuration");
        let state_root = TestRoot::new("adversarial-state");
        fs::create_dir(state_root.path().join(".agentmage-runtime-payloads-v1"))
            .expect("artifact namespace");
        for child in ["staging", "objects", "quarantine"] {
            fs::create_dir(
                state_root
                    .path()
                    .join(".agentmage-runtime-payloads-v1")
                    .join(child),
            )
            .expect("artifact child");
        }
        let workspace = authorize_workspace_root(
            workspace_root.path(),
            WorkspaceId::from_raw("workspace-write-root-adversarial"),
            WorkspaceAuthorizationId::from_raw("authorization-write-root-adversarial"),
            AdapterInstanceId::from_raw("adapter-write-root-adversarial"),
        )
        .expect("workspace authorizes");
        let configuration = LinuxStrictLocalRootInspector::inspect(configuration_root.path())
            .expect("configuration root inspects");
        let state =
            LinuxStrictLocalRootInspector::inspect(state_root.path()).expect("state root inspects");
        let roots = [
            LinuxWriteStrictRoot::new(LinuxWriteRootClass::Configuration, &configuration),
            LinuxWriteStrictRoot::new(LinuxWriteRootClass::State, &state),
        ];

        fs::write(
            workspace_root
                .path()
                .join(".agentmage-write-ffeeddccbbaa99887766554433221100.tmp"),
            b"orphan",
        )
        .expect("orphan staging");
        fs::write(
            state_root
                .path()
                .join(".agentmage-runtime-payloads-v1/objects/object-undeclared"),
            b"undeclared durable copy",
        )
        .expect("undeclared object");
        symlink(
            workspace_root.path().join("user-owned"),
            workspace_root.path().join(".agentmage-unknown-link"),
        )
        .expect("reserved link");
        let adversarial = scan_linux_write_roots(
            &workspace,
            &roots,
            &[],
            1_000,
            LinuxWriteRootScanLimits::default(),
        )
        .expect("adversarial roots scan");
        assert_eq!(adversarial.orphan_staging_objects, 1);
        assert_eq!(adversarial.undeclared_copies, 2);
        assert_eq!(adversarial.inaccessible_or_mismatched_objects, 1);
        assert!(!adversarial.is_clean());
        assert_eq!(adversarial.inventory_sha256.len(), 64);

        let oversized = scan_linux_write_roots(
            &workspace,
            &roots,
            &[],
            1_000,
            LinuxWriteRootScanLimits {
                maximum_reserved_file_bytes: 1,
                ..LinuxWriteRootScanLimits::default()
            },
        )
        .expect_err("reserved byte bound fails closed");
        assert_eq!(oversized.kind(), LinuxWriteRootScanErrorKind::LimitExceeded);

        let limited = scan_linux_write_roots(
            &workspace,
            &roots,
            &[],
            1_000,
            LinuxWriteRootScanLimits {
                maximum_entries: 1,
                ..LinuxWriteRootScanLimits::default()
            },
        )
        .expect_err("entry bound fails closed");
        assert_eq!(limited.kind(), LinuxWriteRootScanErrorKind::LimitExceeded);

        let duplicate_root_classes = [
            LinuxWriteStrictRoot::new(LinuxWriteRootClass::Configuration, &state),
            LinuxWriteStrictRoot::new(LinuxWriteRootClass::State, &state),
        ];
        let duplicate = scan_linux_write_roots(
            &workspace,
            &duplicate_root_classes,
            &[],
            1_000,
            LinuxWriteRootScanLimits::default(),
        )
        .expect_err("duplicate physical roots fail closed");
        assert_eq!(
            duplicate.kind(),
            LinuxWriteRootScanErrorKind::InvalidManifest
        );
    }
}
