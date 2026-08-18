//! Descriptor-relative Linux discovery of metadata-only instruction evidence.

use std::collections::BTreeSet;
use std::fmt;

use agentmage_kernel_contracts::{
    AuthorizedWorkspaceHandle, HeldWorkspaceObject, PathAdapterErrorKind, PathResolutionIntent,
    PlatformPathAdapter, WorkspaceObjectKind, WorkspacePath,
};
use agentmage_kernel_engine::instruction_provenance::{
    InstructionDiscoveryInput, InstructionDiscoveryPlan, InstructionDiscoveryRecord,
    InstructionLocation, InstructionSourceKind, record_instruction_discovery,
    verify_instruction_discovery_plan,
};
use sha2::{Digest, Sha256};

use crate::{LinuxAuthorizedWorkspace, LinuxHeldObject, LinuxPathAdapter};

const HIERARCHICAL_INSTRUCTION_NAME: &str = "AGENTS.md";
const HARD_MAX_DIRECTORIES: usize = 4_096;
const HARD_MAX_ENTRIES: usize = 65_536;
const HARD_MAX_RECORDS: usize = 4_096;
const HARD_MAX_DEPTH: usize = 64;
const HARD_MAX_NAMES_PER_DIRECTORY: usize = 4_096;
const HARD_MAX_NAME_BYTES_PER_DIRECTORY: usize = 1024 * 1024;

const EXCLUDED_DIRECTORIES: [&str; 11] = [
    ".git",
    ".cache",
    ".mypy_cache",
    ".pytest_cache",
    ".venv",
    "__pycache__",
    "build",
    "dist",
    "node_modules",
    "target",
    "venv",
];

/// Stable content-free Linux instruction-discovery failure class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxInstructionDiscoveryErrorKind {
    /// The kernel plan or scanner limits are malformed.
    InvalidPlan,
    /// A bounded traversal or result limit was exceeded.
    ResourceLimitExceeded,
    /// A candidate or traversed object could not be observed safely.
    ObservationFailed,
    /// The held workspace changed during discovery.
    WorkspaceChanged,
}

impl LinuxInstructionDiscoveryErrorKind {
    /// Returns the stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidPlan => "linux.instruction-discovery.plan.invalid",
            Self::ResourceLimitExceeded => "linux.instruction-discovery.limit.exceeded",
            Self::ObservationFailed => "linux.instruction-discovery.observation.failed",
            Self::WorkspaceChanged => "linux.instruction-discovery.workspace.changed",
        }
    }
}

/// Content-free error at the Linux instruction-discovery boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxInstructionDiscoveryError {
    kind: LinuxInstructionDiscoveryErrorKind,
}

impl LinuxInstructionDiscoveryError {
    /// Returns the stable failure class.
    #[must_use]
    pub const fn kind(self) -> LinuxInstructionDiscoveryErrorKind {
        self.kind
    }
}

impl fmt::Display for LinuxInstructionDiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.kind.code())
    }
}

impl std::error::Error for LinuxInstructionDiscoveryError {}

/// Closed resource limits for one metadata-only discovery pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxInstructionDiscoveryLimits {
    /// Maximum directories enumerated across all hierarchical roots.
    pub maximum_directories: usize,
    /// Maximum directory entries observed across all hierarchical roots.
    pub maximum_entries: usize,
    /// Maximum resulting instruction records.
    pub maximum_records: usize,
    /// Maximum relative traversal depth below one hierarchical root.
    pub maximum_depth: usize,
    /// Maximum names admitted from one directory observation.
    pub maximum_names_per_directory: usize,
    /// Maximum aggregate UTF-8 name bytes admitted from one directory observation.
    pub maximum_name_bytes_per_directory: usize,
}

impl Default for LinuxInstructionDiscoveryLimits {
    fn default() -> Self {
        Self {
            maximum_directories: HARD_MAX_DIRECTORIES,
            maximum_entries: HARD_MAX_ENTRIES,
            maximum_records: HARD_MAX_RECORDS,
            maximum_depth: HARD_MAX_DEPTH,
            maximum_names_per_directory: HARD_MAX_NAMES_PER_DIRECTORY,
            maximum_name_bytes_per_directory: HARD_MAX_NAME_BYTES_PER_DIRECTORY,
        }
    }
}

/// Complete metadata-only output from one bounded Linux discovery pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxInstructionDiscoveryResult {
    records: Vec<InstructionDiscoveryRecord>,
    directories_observed: usize,
    entries_observed: usize,
    excluded_directories: usize,
    unsafe_entries_skipped: usize,
}

impl LinuxInstructionDiscoveryResult {
    /// Returns sorted hash-bound discovery records without source content.
    #[must_use]
    pub fn records(&self) -> &[InstructionDiscoveryRecord] {
        &self.records
    }

    /// Returns the exact number of directories enumerated.
    #[must_use]
    pub const fn directories_observed(&self) -> usize {
        self.directories_observed
    }

    /// Returns the exact number of names observed.
    #[must_use]
    pub const fn entries_observed(&self) -> usize {
        self.entries_observed
    }

    /// Returns the number of excluded generated/cache directories encountered.
    #[must_use]
    pub const fn excluded_directories(&self) -> usize {
        self.excluded_directories
    }

    /// Returns the number of symlink, hard-link, special, or raced entries ignored.
    #[must_use]
    pub const fn unsafe_entries_skipped(&self) -> usize {
        self.unsafe_entries_skipped
    }
}

/// Discovers exact candidate metadata and nested `AGENTS.md` files without reading content.
pub fn discover_linux_instructions(
    workspace: &LinuxAuthorizedWorkspace,
    plan: &InstructionDiscoveryPlan,
    limits: LinuxInstructionDiscoveryLimits,
) -> Result<LinuxInstructionDiscoveryResult, LinuxInstructionDiscoveryError> {
    if !verify_instruction_discovery_plan(plan)
        || workspace.workspace_id() != plan.workspace_id()
        || !valid_limits(limits)
    {
        return Err(error(LinuxInstructionDiscoveryErrorKind::InvalidPlan));
    }
    workspace
        .revalidate()
        .map_err(|_| error(LinuxInstructionDiscoveryErrorKind::WorkspaceChanged))?;
    let adapter = LinuxPathAdapter::new(workspace.adapter_instance_id().clone(), 1);
    let mut records = Vec::new();
    let mut seen_paths = BTreeSet::new();
    let mut unsafe_entries_skipped = 0_usize;

    for candidate in plan.candidates() {
        match adapter.resolve(workspace, candidate.path(), PathResolutionIntent::Metadata) {
            Ok(held) if held.object_kind() == WorkspaceObjectKind::RegularFile => {
                push_record(
                    &mut records,
                    &mut seen_paths,
                    candidate.source_kind(),
                    candidate.path().clone(),
                    &held,
                    plan,
                    limits,
                )?;
            }
            Ok(_) => unsafe_entries_skipped += 1,
            Err(path_error) if path_error.kind() == PathAdapterErrorKind::NotFound => {}
            Err(path_error) if ignorable_path_error(path_error.kind()) => {
                unsafe_entries_skipped += 1;
            }
            Err(_) => {
                return Err(error(LinuxInstructionDiscoveryErrorKind::ObservationFailed));
            }
        }
    }

    let mut directories_observed = 0_usize;
    let mut entries_observed = 0_usize;
    let mut excluded_directories = 0_usize;
    for root in plan.hierarchical_roots() {
        let root_components = root
            .components()
            .iter()
            .map(|component| component.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut pending = vec![(root_components.clone(), 0_usize)];
        while let Some((components, depth)) = pending.pop() {
            directories_observed = directories_observed
                .checked_add(1)
                .filter(|count| *count <= limits.maximum_directories)
                .ok_or_else(|| error(LinuxInstructionDiscoveryErrorKind::ResourceLimitExceeded))?;
            let names = observe_names(workspace, &adapter, &components, limits)?;
            entries_observed = entries_observed
                .checked_add(names.len())
                .filter(|count| *count <= limits.maximum_entries)
                .ok_or_else(|| error(LinuxInstructionDiscoveryErrorKind::ResourceLimitExceeded))?;
            for name in names.into_iter().rev() {
                let mut child_components = components.clone();
                child_components.push(name.clone());
                let child = WorkspacePath::new(plan.workspace_id().clone(), child_components)
                    .map_err(|_| error(LinuxInstructionDiscoveryErrorKind::ObservationFailed))?;
                let held = match adapter.resolve(workspace, &child, PathResolutionIntent::Metadata)
                {
                    Ok(held) => held,
                    Err(path_error) if path_error.kind() == PathAdapterErrorKind::NotFound => {
                        unsafe_entries_skipped += 1;
                        continue;
                    }
                    Err(path_error) if ignorable_path_error(path_error.kind()) => {
                        unsafe_entries_skipped += 1;
                        continue;
                    }
                    Err(_) => {
                        return Err(error(LinuxInstructionDiscoveryErrorKind::ObservationFailed));
                    }
                };
                match held.object_kind() {
                    WorkspaceObjectKind::RegularFile if name == HIERARCHICAL_INSTRUCTION_NAME => {
                        push_record(
                            &mut records,
                            &mut seen_paths,
                            InstructionSourceKind::HierarchicalInstruction,
                            child,
                            &held,
                            plan,
                            limits,
                        )?;
                    }
                    WorkspaceObjectKind::Directory
                        if EXCLUDED_DIRECTORIES.contains(&name.as_str()) =>
                    {
                        excluded_directories += 1;
                    }
                    WorkspaceObjectKind::Directory if depth < limits.maximum_depth => {
                        pending.push((
                            child
                                .components()
                                .iter()
                                .map(|component| component.as_str().to_owned())
                                .collect(),
                            depth + 1,
                        ));
                    }
                    WorkspaceObjectKind::Directory => {
                        return Err(error(
                            LinuxInstructionDiscoveryErrorKind::ResourceLimitExceeded,
                        ));
                    }
                    WorkspaceObjectKind::RegularFile => {}
                }
            }
        }
    }
    workspace
        .revalidate()
        .map_err(|_| error(LinuxInstructionDiscoveryErrorKind::WorkspaceChanged))?;
    records.sort_by(|left, right| left.discovery_id.cmp(&right.discovery_id));
    Ok(LinuxInstructionDiscoveryResult {
        records,
        directories_observed,
        entries_observed,
        excluded_directories,
        unsafe_entries_skipped,
    })
}

fn observe_names(
    workspace: &LinuxAuthorizedWorkspace,
    adapter: &LinuxPathAdapter,
    components: &[String],
    limits: LinuxInstructionDiscoveryLimits,
) -> Result<Vec<String>, LinuxInstructionDiscoveryError> {
    if components.is_empty() {
        workspace
            .observe_root_names(
                limits.maximum_names_per_directory,
                limits.maximum_name_bytes_per_directory,
            )
            .map_err(map_observation_error)
    } else {
        let path = WorkspacePath::new(workspace.workspace_id().clone(), components.iter().cloned())
            .map_err(|_| error(LinuxInstructionDiscoveryErrorKind::ObservationFailed))?;
        let held = adapter
            .resolve(workspace, &path, PathResolutionIntent::ReadDirectory)
            .map_err(map_observation_error)?;
        held.observe_directory_names(
            limits.maximum_names_per_directory,
            limits.maximum_name_bytes_per_directory,
        )
        .map_err(map_observation_error)
    }
}

fn push_record(
    records: &mut Vec<InstructionDiscoveryRecord>,
    seen_paths: &mut BTreeSet<WorkspacePath>,
    source_kind: InstructionSourceKind,
    path: WorkspacePath,
    held: &LinuxHeldObject,
    plan: &InstructionDiscoveryPlan,
    limits: LinuxInstructionDiscoveryLimits,
) -> Result<(), LinuxInstructionDiscoveryError> {
    if !seen_paths.insert(path.clone()) {
        return Ok(());
    }
    if records.len() >= limits.maximum_records {
        return Err(error(
            LinuxInstructionDiscoveryErrorKind::ResourceLimitExceeded,
        ));
    }
    held.revalidate()
        .map_err(|_| error(LinuxInstructionDiscoveryErrorKind::ObservationFailed))?;
    let source_identity_sha256 = source_identity_sha256(source_kind, &path);
    let freshness_sha256 = freshness_sha256(&path, held);
    let discovery_id = format!("instruction-discovery-{}", &source_identity_sha256[..32]);
    let record = record_instruction_discovery(InstructionDiscoveryInput {
        discovery_id,
        source_kind,
        location: InstructionLocation {
            workspace_path: Some(path),
            source_identity_sha256,
            revision_sha256: freshness_sha256.clone(),
            line_start: None,
            line_end: None,
        },
        workspace_manifest_sha256: plan.workspace_manifest_sha256().to_owned(),
        freshness_sha256,
    })
    .map_err(|_| error(LinuxInstructionDiscoveryErrorKind::ObservationFailed))?;
    records.push(record);
    Ok(())
}

fn valid_limits(limits: LinuxInstructionDiscoveryLimits) -> bool {
    (1..=HARD_MAX_DIRECTORIES).contains(&limits.maximum_directories)
        && (1..=HARD_MAX_ENTRIES).contains(&limits.maximum_entries)
        && (1..=HARD_MAX_RECORDS).contains(&limits.maximum_records)
        && (1..=HARD_MAX_DEPTH).contains(&limits.maximum_depth)
        && (1..=HARD_MAX_NAMES_PER_DIRECTORY).contains(&limits.maximum_names_per_directory)
        && (1..=HARD_MAX_NAME_BYTES_PER_DIRECTORY)
            .contains(&limits.maximum_name_bytes_per_directory)
}

const fn ignorable_path_error(kind: PathAdapterErrorKind) -> bool {
    matches!(
        kind,
        PathAdapterErrorKind::SymbolicLink
            | PathAdapterErrorKind::HardLink
            | PathAdapterErrorKind::ObjectKindMismatch
            | PathAdapterErrorKind::IdentityChanged
    )
}

fn map_observation_error(
    error: agentmage_kernel_contracts::PathAdapterError,
) -> LinuxInstructionDiscoveryError {
    if error.kind() == PathAdapterErrorKind::ResourceLimitExceeded {
        self::error(LinuxInstructionDiscoveryErrorKind::ResourceLimitExceeded)
    } else {
        self::error(LinuxInstructionDiscoveryErrorKind::ObservationFailed)
    }
}

fn source_identity_sha256(source_kind: InstructionSourceKind, path: &WorkspacePath) -> String {
    let mut digest = Sha256::new();
    update_field(&mut digest, b"agentmage.linux.instruction-source.v1");
    update_field(&mut digest, source_kind_label(source_kind).as_bytes());
    update_field(&mut digest, path.workspace_id().as_str().as_bytes());
    for component in path.components() {
        update_field(&mut digest, component.as_str().as_bytes());
    }
    finish_hash(digest)
}

fn freshness_sha256(path: &WorkspacePath, held: &LinuxHeldObject) -> String {
    let snapshot = &held.object_snapshot;
    let mut digest = Sha256::new();
    update_field(&mut digest, b"agentmage.linux.instruction-freshness.v1");
    update_field(&mut digest, path.workspace_id().as_str().as_bytes());
    for component in path.components() {
        update_field(&mut digest, component.as_str().as_bytes());
    }
    digest.update(snapshot.device.to_be_bytes());
    digest.update(snapshot.inode.to_be_bytes());
    digest.update(snapshot.link_count.to_be_bytes());
    digest.update(snapshot.mode.to_be_bytes());
    digest.update(snapshot.size.to_be_bytes());
    digest.update(snapshot.modified_seconds.to_be_bytes());
    digest.update(snapshot.modified_nanoseconds.to_be_bytes());
    digest.update(snapshot.changed_seconds.to_be_bytes());
    digest.update(snapshot.changed_nanoseconds.to_be_bytes());
    match snapshot.mount_id {
        Some(mount_id) => {
            digest.update([1]);
            digest.update(mount_id.to_be_bytes());
        }
        None => digest.update([0]),
    }
    finish_hash(digest)
}

fn update_field(digest: &mut Sha256, bytes: &[u8]) {
    digest.update((bytes.len() as u64).to_be_bytes());
    digest.update(bytes);
}

fn finish_hash(digest: Sha256) -> String {
    let mut output = String::with_capacity(64);
    for byte in digest.finalize() {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

const fn source_kind_label(source_kind: InstructionSourceKind) -> &'static str {
    match source_kind {
        InstructionSourceKind::WorkspaceInstruction => "workspace_instruction",
        InstructionSourceKind::RepositoryInstruction => "repository_instruction",
        InstructionSourceKind::ProjectDocument => "project_document",
        InstructionSourceKind::HierarchicalInstruction => "hierarchical_instruction",
        _ => "unsupported_instruction_source",
    }
}

const fn error(kind: LinuxInstructionDiscoveryErrorKind) -> LinuxInstructionDiscoveryError {
    LinuxInstructionDiscoveryError { kind }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _, symlink};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::{
        AdapterInstanceId, WorkspaceAuthorizationId, WorkspaceId, WorkspacePath, WorkspaceScopePath,
    };
    use agentmage_kernel_engine::instruction_provenance::{
        InstructionDiscoveryRequest, InstructionSourceKind, plan_instruction_discovery,
    };

    use super::*;

    static NEXT_ID: AtomicU64 = AtomicU64::new(1);

    struct Fixture {
        root: PathBuf,
        workspace: LinuxAuthorizedWorkspace,
        workspace_id: WorkspaceId,
    }

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "agentmage-instruction-discovery-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(root.join("repository/src")).expect("source directory");
            fs::create_dir_all(root.join("repository/node_modules/package"))
                .expect("excluded directory");
            fs::create_dir_all(root.join("repository/hard-linked")).expect("hard-link directory");
            fs::write(root.join("WORKSPACE.md"), b"workspace-secret-canary")
                .expect("workspace instruction");
            fs::write(
                root.join("repository/AGENTS.md"),
                b"repository-secret-canary",
            )
            .expect("repository instruction");
            fs::write(root.join("repository/README.md"), b"project-secret-canary")
                .expect("project document");
            fs::write(
                root.join("repository/src/AGENTS.md"),
                b"nested-secret-canary",
            )
            .expect("nested instruction");
            fs::write(
                root.join("repository/node_modules/package/AGENTS.md"),
                b"excluded-secret-canary",
            )
            .expect("excluded instruction");
            fs::write(
                root.join("repository/hard-linked/source.md"),
                b"hard-link-secret-canary",
            )
            .expect("hard-link source");
            fs::hard_link(
                root.join("repository/hard-linked/source.md"),
                root.join("repository/hard-linked/AGENTS.md"),
            )
            .expect("hard-link fixture");
            symlink("src", root.join("repository/linked-source")).expect("symlink fixture");
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700))
                .expect("private fixture root");
            let workspace_id = WorkspaceId::from_raw("workspace-instruction-discovery");
            let workspace = crate::authorize_workspace_root(
                &root,
                workspace_id.clone(),
                WorkspaceAuthorizationId::from_raw("authorization-instruction-discovery"),
                AdapterInstanceId::from_raw("adapter-instruction-discovery"),
            )
            .expect("workspace authorizes");
            Self {
                root,
                workspace,
                workspace_id,
            }
        }

        fn plan(&self) -> InstructionDiscoveryPlan {
            plan_instruction_discovery(InstructionDiscoveryRequest {
                workspace_id: self.workspace_id.clone(),
                workspace_manifest_sha256: "a".repeat(64),
                workspace_instruction_paths: vec![self.path(["WORKSPACE.md"])],
                repository_instruction_paths: vec![self.path(["repository", "AGENTS.md"])],
                project_document_paths: vec![self.path(["repository", "README.md"])],
                hierarchical_roots: vec![
                    WorkspaceScopePath::new(self.workspace_id.clone(), ["repository"])
                        .expect("repository scope"),
                ],
            })
            .expect("instruction discovery plan")
        }

        fn path<const N: usize>(&self, components: [&str; N]) -> WorkspacePath {
            WorkspacePath::new(self.workspace_id.clone(), components).expect("workspace path")
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn exact_candidates_and_nested_agents_are_metadata_only_and_invariant() {
        let fixture = Fixture::new();
        let before = tree_snapshot(&fixture.root);
        let result = discover_linux_instructions(
            &fixture.workspace,
            &fixture.plan(),
            LinuxInstructionDiscoveryLimits::default(),
        )
        .expect("instruction discovery succeeds");

        assert_eq!(result.records().len(), 4);
        let kinds = result
            .records()
            .iter()
            .map(|record| record.source_kind)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            kinds,
            BTreeSet::from([
                InstructionSourceKind::WorkspaceInstruction,
                InstructionSourceKind::RepositoryInstruction,
                InstructionSourceKind::ProjectDocument,
                InstructionSourceKind::HierarchicalInstruction,
            ])
        );
        assert_eq!(result.excluded_directories(), 1);
        assert_eq!(result.unsafe_entries_skipped(), 3);
        assert!(result.directories_observed() >= 3);
        assert!(result.entries_observed() >= 7);
        assert_eq!(tree_snapshot(&fixture.root), before);

        let serialized = serde_json::to_vec(result.records()).expect("records serialize");
        for canary in [
            b"workspace-secret-canary".as_slice(),
            b"repository-secret-canary".as_slice(),
            b"project-secret-canary".as_slice(),
            b"nested-secret-canary".as_slice(),
            b"excluded-secret-canary".as_slice(),
            b"hard-link-secret-canary".as_slice(),
        ] {
            assert!(
                !serialized
                    .windows(canary.len())
                    .any(|window| window == canary)
            );
        }
    }

    #[test]
    fn metadata_change_produces_new_freshness_without_reading_content() {
        let fixture = Fixture::new();
        let first = discover_linux_instructions(
            &fixture.workspace,
            &fixture.plan(),
            LinuxInstructionDiscoveryLimits::default(),
        )
        .expect("first discovery");
        fs::write(
            fixture.root.join("repository/README.md"),
            b"changed-project-secret-canary-with-new-size",
        )
        .expect("project document changes");
        let second = discover_linux_instructions(
            &fixture.workspace,
            &fixture.plan(),
            LinuxInstructionDiscoveryLimits::default(),
        )
        .expect("second discovery");
        let project_freshness = |result: &LinuxInstructionDiscoveryResult| {
            result
                .records()
                .iter()
                .find(|record| record.source_kind == InstructionSourceKind::ProjectDocument)
                .expect("project record")
                .freshness_sha256
                .clone()
        };
        assert_ne!(project_freshness(&first), project_freshness(&second));
    }

    #[test]
    fn traversal_and_record_limits_fail_closed() {
        let fixture = Fixture::new();
        let directory_limited = discover_linux_instructions(
            &fixture.workspace,
            &fixture.plan(),
            LinuxInstructionDiscoveryLimits {
                maximum_directories: 1,
                ..LinuxInstructionDiscoveryLimits::default()
            },
        )
        .expect_err("directory limit must fail closed");
        assert_eq!(
            directory_limited.kind(),
            LinuxInstructionDiscoveryErrorKind::ResourceLimitExceeded
        );

        let record_limited = discover_linux_instructions(
            &fixture.workspace,
            &fixture.plan(),
            LinuxInstructionDiscoveryLimits {
                maximum_records: 1,
                ..LinuxInstructionDiscoveryLimits::default()
            },
        )
        .expect_err("record limit must fail closed");
        assert_eq!(
            record_limited.kind(),
            LinuxInstructionDiscoveryErrorKind::ResourceLimitExceeded
        );
    }

    fn tree_snapshot(root: &Path) -> Vec<(PathBuf, u32, u64, Vec<u8>)> {
        fn visit(root: &Path, current: &Path, output: &mut Vec<(PathBuf, u32, u64, Vec<u8>)>) {
            let mut entries = fs::read_dir(current)
                .expect("snapshot directory")
                .map(|entry| entry.expect("snapshot entry"))
                .collect::<Vec<_>>();
            entries.sort_by_key(fs::DirEntry::file_name);
            for entry in entries {
                let path = entry.path();
                let metadata = fs::symlink_metadata(&path).expect("snapshot metadata");
                let relative = path
                    .strip_prefix(root)
                    .expect("relative path")
                    .to_path_buf();
                let bytes = if metadata.is_file() {
                    fs::read(&path).expect("snapshot bytes")
                } else if metadata.file_type().is_symlink() {
                    fs::read_link(&path)
                        .expect("snapshot link")
                        .as_os_str()
                        .as_encoded_bytes()
                        .to_vec()
                } else {
                    Vec::new()
                };
                output.push((relative, metadata.mode(), metadata.ino(), bytes));
                if metadata.is_dir() {
                    visit(root, &path, output);
                }
            }
        }

        let mut output = Vec::new();
        visit(root, root, &mut output);
        output
    }
}
