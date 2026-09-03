//! Authority-free protected-memory planning over the controlled filesystem transaction boundary.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use agentmage_capability_knowledge::{MemoryMarkdownBundle, MemoryMarkdownFile};
use agentmage_kernel_contracts::{GrantTarget, WorkspaceObjectKind, WorkspacePath};
use agentmage_kernel_engine::filesystem_control::{
    ExistingSourceDraft, ExistingWorkDisposition, FileClassification, FilesystemOperationDraft,
    NewDestinationDraft, StructuredPatch, StructuredPatchHunk,
};
use sha2::{Digest, Sha256};

const MAX_FILES: usize = 100_001;
const MAX_FILE_BYTES: usize = 1024 * 1024;
const MAX_EXPORT_BYTES: usize = 256 * 1024 * 1024;

/// Closed installed-memory operation intent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryFileIntent {
    /// Publish one newly approved current bundle.
    Publish,
    /// Restore one previously verified last-good bundle.
    RestoreLastGood,
    /// Preserve a complete proposal after a simultaneous-edit mismatch.
    PreserveConflict,
}

/// Exact held observation for one governed logical memory file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemoryFileObservation {
    /// The destination is absent under one continuously held parent.
    Absent {
        /// Logical bundle-relative path, independent of its conflict/backup destination.
        logical_relative_path: String,
        /// Exact canonical absent destination.
        path: WorkspacePath,
        /// Held destination parent.
        parent: GrantTarget,
        /// Complete bounded sibling-name observation.
        observed_sibling_names: Vec<String>,
    },
    /// The destination is an exactly observed existing regular file.
    Existing {
        /// Logical bundle-relative path.
        logical_relative_path: String,
        /// Held existing target.
        target: GrantTarget,
        /// Exact held bytes.
        observed_bytes: Vec<u8>,
        /// Exact observed POSIX-compatible mode.
        mode: u32,
        /// Current repository/work-packet ownership classification.
        work_disposition: ExistingWorkDisposition,
    },
}

/// Authority-free plan for one approved installed-memory operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryFilePlan {
    /// Closed requested intent.
    pub intent: MemoryFileIntent,
    /// Exact bundle identity installed or preserved by the plan.
    pub bundle_sha256: String,
    /// Ordered controlled filesystem drafts requiring ordinary kernel approval.
    pub operations: Vec<FilesystemOperationDraft>,
    /// Fixed false network marker.
    pub network_used: bool,
    /// Fixed false automatic-decision marker.
    pub automatic_decision: bool,
}

/// Content-free protected-memory planning failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryFileError {
    /// Bundle, identity, path, observation, or export input was invalid.
    InvalidInput,
    /// Expected/current identity did not match the declared intent.
    ConcurrentState,
    /// A held object or exact preimage did not match the requested file.
    ObservationMismatch,
    /// Complete replacement could not be represented as a bounded structured patch.
    PatchUnavailable,
}

/// Builds a controlled publication, restoration, or conflict-preservation plan.
///
/// This function performs no I/O. Every returned operation remains subject to the kernel's normal
/// plan rendering, explicit approval, one-use grant, atomic platform driver, receipt verification,
/// and crash recovery. The caller supplies exact held observations for the current, last-good, or
/// conflict destination projection.
pub fn compose_memory_file_plan(
    operation_prefix: &str,
    bundle: &MemoryMarkdownBundle,
    expected_current_bundle_sha256: Option<&str>,
    observed_current_bundle_sha256: Option<&str>,
    intent: MemoryFileIntent,
    observations: Vec<MemoryFileObservation>,
) -> Result<MemoryFilePlan, MemoryFileError> {
    validate_identifier(operation_prefix)?;
    let files = bundle_files(bundle)?;
    match intent {
        MemoryFileIntent::Publish | MemoryFileIntent::RestoreLastGood => {
            if expected_current_bundle_sha256 != observed_current_bundle_sha256 {
                return Err(MemoryFileError::ConcurrentState);
            }
        }
        MemoryFileIntent::PreserveConflict => {
            if expected_current_bundle_sha256 == observed_current_bundle_sha256 {
                return Err(MemoryFileError::ConcurrentState);
            }
        }
    }
    for digest in [
        expected_current_bundle_sha256,
        observed_current_bundle_sha256,
    ]
    .into_iter()
    .flatten()
    {
        if !valid_sha256(digest) {
            return Err(MemoryFileError::InvalidInput);
        }
    }
    let mut observed = observations
        .into_iter()
        .map(|observation| (logical_path(&observation).to_owned(), observation))
        .collect::<BTreeMap<_, _>>();
    if observed.len() != files.len() {
        return Err(MemoryFileError::ObservationMismatch);
    }
    let mut operations = Vec::with_capacity(files.len());
    for (ordinal, (relative_path, bytes)) in files.into_iter().enumerate() {
        let observation = observed
            .remove(&relative_path)
            .ok_or(MemoryFileError::ObservationMismatch)?;
        validate_destination(
            intent,
            &relative_path,
            observation_path(&observation).ok_or(MemoryFileError::ObservationMismatch)?,
        )?;
        let operation_id = format!("{operation_prefix}-{ordinal}");
        operations.push(operation_for(operation_id, bytes, observation)?);
    }
    if !observed.is_empty() {
        return Err(MemoryFileError::ObservationMismatch);
    }
    Ok(MemoryFilePlan {
        intent,
        bundle_sha256: bundle.bundle_sha256.clone(),
        operations,
        network_used: false,
        automatic_decision: false,
    })
}

/// Builds one kernel-controlled copy plan for a verified current projection into last-good paths.
pub fn compose_memory_backup_plan(
    operation_prefix: &str,
    bundle: &MemoryMarkdownBundle,
    sources: Vec<ExistingSourceDraft>,
    destinations: Vec<NewDestinationDraft>,
) -> Result<MemoryFilePlan, MemoryFileError> {
    validate_identifier(operation_prefix)?;
    let files = bundle_files(bundle)?;
    if files.len() != sources.len() || files.len() != destinations.len() {
        return Err(MemoryFileError::ObservationMismatch);
    }
    let mut operations = Vec::with_capacity(files.len());
    for (ordinal, (((relative, bytes), source), destination)) in
        files.into_iter().zip(sources).zip(destinations).enumerate()
    {
        if source.target.workspace_path().is_none()
            || source.target.object_kind() != Some(WorkspaceObjectKind::RegularFile)
            || source.observed_bytes != bytes
            || source.mode > 0o777
            || !matches!(
                source.work_disposition,
                ExistingWorkDisposition::Clean | ExistingWorkDisposition::OwnedByCurrentTask
            )
            || !path_ends_with(&destination.path, &relative)
            || destination.parent.object_kind() != Some(WorkspaceObjectKind::Directory)
        {
            return Err(MemoryFileError::ObservationMismatch);
        }
        operations.push(FilesystemOperationDraft::Copy {
            operation_id: format!("{operation_prefix}-{ordinal}"),
            source,
            destination,
            classification: FileClassification::Data,
        });
    }
    Ok(MemoryFilePlan {
        intent: MemoryFileIntent::RestoreLastGood,
        bundle_sha256: bundle.bundle_sha256.clone(),
        operations,
        network_used: false,
        automatic_decision: false,
    })
}

/// Builds one controlled create for exact opaque encrypted export bytes.
pub fn compose_portable_memory_export(
    operation_id: String,
    bytes: Vec<u8>,
    expected_sha256: &str,
    destination: NewDestinationDraft,
) -> Result<FilesystemOperationDraft, MemoryFileError> {
    validate_identifier(&operation_id)?;
    if bytes.is_empty()
        || bytes.len() > MAX_EXPORT_BYTES
        || sha256(&bytes) != expected_sha256
        || destination.parent.object_kind() != Some(WorkspaceObjectKind::Directory)
    {
        return Err(MemoryFileError::InvalidInput);
    }
    Ok(FilesystemOperationDraft::Create {
        operation_id,
        destination,
        content: bytes,
        mode: 0o600,
        classification: FileClassification::Data,
    })
}

fn operation_for(
    operation_id: String,
    bytes: Vec<u8>,
    observation: MemoryFileObservation,
) -> Result<FilesystemOperationDraft, MemoryFileError> {
    match observation {
        MemoryFileObservation::Absent {
            path,
            parent,
            observed_sibling_names,
            ..
        } => {
            if parent.object_kind() != Some(WorkspaceObjectKind::Directory)
                || parent.workspace_id() != path.workspace_id()
            {
                return Err(MemoryFileError::ObservationMismatch);
            }
            Ok(FilesystemOperationDraft::Create {
                operation_id,
                destination: NewDestinationDraft {
                    parent,
                    path,
                    observed_sibling_names,
                },
                content: bytes,
                mode: 0o600,
                classification: FileClassification::Data,
            })
        }
        MemoryFileObservation::Existing {
            target,
            observed_bytes,
            mode,
            work_disposition,
            ..
        } => {
            if target.object_kind() != Some(WorkspaceObjectKind::RegularFile)
                || mode > 0o777
                || !matches!(
                    work_disposition,
                    ExistingWorkDisposition::Clean | ExistingWorkDisposition::OwnedByCurrentTask
                )
            {
                return Err(MemoryFileError::ObservationMismatch);
            }
            let patch = StructuredPatch {
                schema_version: 1,
                hunks: vec![StructuredPatchHunk {
                    old_start_line: 1,
                    old_lines: exact_lines(&observed_bytes)?,
                    new_lines: exact_lines(&bytes)?,
                }],
            };
            Ok(FilesystemOperationDraft::ExactPatch {
                operation_id,
                source: ExistingSourceDraft {
                    target,
                    observed_bytes,
                    work_disposition,
                    mode,
                },
                patch_json: serde_json::to_vec(&patch)
                    .map_err(|_| MemoryFileError::PatchUnavailable)?,
                expected_postimage_sha256: sha256(&bytes),
            })
        }
    }
}

fn bundle_files(bundle: &MemoryMarkdownBundle) -> Result<Vec<(String, Vec<u8>)>, MemoryFileError> {
    if bundle.write_enabled
        || bundle.topics.len().saturating_add(1) > MAX_FILES
        || !valid_sha256(&bundle.bundle_sha256)
        || bundle_digest(&bundle.index, &bundle.topics) != bundle.bundle_sha256
    {
        return Err(MemoryFileError::InvalidInput);
    }
    let mut inputs = Vec::with_capacity(bundle.topics.len() + 1);
    inputs.push(&bundle.index);
    inputs.extend(bundle.topics.iter());
    let mut seen = BTreeSet::new();
    let mut files = Vec::with_capacity(inputs.len());
    for file in inputs {
        if file.write_enabled
            || file.markdown.is_empty()
            || file.markdown.len() > MAX_FILE_BYTES
            || sha256(file.markdown.as_bytes()) != file.content_sha256
            || !valid_relative_path(&file.relative_path)
            || !seen.insert(file.relative_path.clone())
        {
            return Err(MemoryFileError::InvalidInput);
        }
        files.push((
            file.relative_path.clone(),
            file.markdown.as_bytes().to_vec(),
        ));
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(files)
}

fn validate_destination(
    intent: MemoryFileIntent,
    logical_path: &str,
    destination: &WorkspacePath,
) -> Result<(), MemoryFileError> {
    if !path_ends_with(destination, logical_path) {
        return Err(MemoryFileError::ObservationMismatch);
    }
    let has_conflict = destination
        .components()
        .iter()
        .any(|component| component.as_str() == "Conflicts");
    if has_conflict != (intent == MemoryFileIntent::PreserveConflict) {
        return Err(MemoryFileError::ObservationMismatch);
    }
    Ok(())
}

fn observation_path(observation: &MemoryFileObservation) -> Option<&WorkspacePath> {
    match observation {
        MemoryFileObservation::Absent { path, .. } => Some(path),
        MemoryFileObservation::Existing { target, .. } => target.workspace_path(),
    }
}

fn logical_path(observation: &MemoryFileObservation) -> &str {
    match observation {
        MemoryFileObservation::Absent {
            logical_relative_path,
            ..
        }
        | MemoryFileObservation::Existing {
            logical_relative_path,
            ..
        } => logical_relative_path,
    }
}

fn path_ends_with(path: &WorkspacePath, relative: &str) -> bool {
    let expected = relative.split('/').collect::<Vec<_>>();
    path.components().len() >= expected.len()
        && path.components()[path.components().len() - expected.len()..]
            .iter()
            .map(|component| component.as_str())
            .eq(expected)
}

fn valid_relative_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 4096
        && value.split('/').all(|component| {
            !component.is_empty()
                && component != "."
                && component != ".."
                && !component.chars().any(char::is_control)
        })
}

fn exact_lines(bytes: &[u8]) -> Result<Vec<String>, MemoryFileError> {
    let text = std::str::from_utf8(bytes).map_err(|_| MemoryFileError::PatchUnavailable)?;
    let lines = text
        .split_inclusive('\n')
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return Err(MemoryFileError::PatchUnavailable);
    }
    Ok(lines)
}

fn bundle_digest(index: &MemoryMarkdownFile, topics: &[MemoryMarkdownFile]) -> String {
    let mut material = Vec::new();
    material.extend_from_slice(index.relative_path.as_bytes());
    material.extend_from_slice(index.content_sha256.as_bytes());
    for topic in topics {
        material.extend_from_slice(topic.relative_path.as_bytes());
        material.extend_from_slice(topic.content_sha256.as_bytes());
    }
    sha256(&material)
}

fn validate_identifier(value: &str) -> Result<(), MemoryFileError> {
    if value.is_empty()
        || value.len() > 128
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
    {
        return Err(MemoryFileError::InvalidInput);
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    let mut value = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut value, "{byte:02x}").expect("writing to String cannot fail");
    }
    value
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        AdapterInstanceId, FilePreimage, HeldWorkspaceObject, PathPlatform, PathResolutionIntent,
        WorkspaceAuthorizationId, WorkspaceId, WorkspaceObjectIdentity, WorkspaceObjectKind,
    };

    use super::*;

    fn path(parts: &[&str]) -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-memory-files"),
            parts.iter().copied(),
        )
        .expect("workspace path")
    }

    #[derive(Debug)]
    struct Held {
        path: WorkspacePath,
        kind: WorkspaceObjectKind,
        preimage: Option<FilePreimage>,
        authorization_id: WorkspaceAuthorizationId,
        adapter_instance_id: AdapterInstanceId,
        identity: WorkspaceObjectIdentity,
    }

    impl HeldWorkspaceObject for Held {
        fn workspace_path(&self) -> &WorkspacePath {
            &self.path
        }

        fn authorization_id(&self) -> &WorkspaceAuthorizationId {
            &self.authorization_id
        }

        fn adapter_instance_id(&self) -> &AdapterInstanceId {
            &self.adapter_instance_id
        }

        fn intent(&self) -> PathResolutionIntent {
            if self.kind == WorkspaceObjectKind::RegularFile {
                PathResolutionIntent::ReadFile
            } else {
                PathResolutionIntent::Metadata
            }
        }

        fn object_kind(&self) -> WorkspaceObjectKind {
            self.kind
        }

        fn object_identity(&self) -> &WorkspaceObjectIdentity {
            &self.identity
        }

        fn preimage(&self) -> Option<&FilePreimage> {
            self.preimage.as_ref()
        }
    }

    fn target(parts: &[&str], kind: WorkspaceObjectKind) -> GrantTarget {
        let held = Held {
            path: path(parts),
            kind,
            preimage: (kind == WorkspaceObjectKind::RegularFile)
                .then(|| FilePreimage::new(1, [8; 32])),
            authorization_id: WorkspaceAuthorizationId::from_raw("authorization-memory-files"),
            adapter_instance_id: AdapterInstanceId::from_raw("adapter-memory-files"),
            identity: WorkspaceObjectIdentity::new(
                PathPlatform::DeterministicFake,
                [7; 32],
                [6; 32],
            ),
        };
        GrantTarget::held_object(&held).expect("target")
    }

    fn file(relative: &str, markdown: &str) -> MemoryMarkdownFile {
        MemoryMarkdownFile {
            relative_path: relative.to_owned(),
            markdown: markdown.to_owned(),
            content_sha256: sha256(markdown.as_bytes()),
            write_enabled: false,
        }
    }

    fn bundle(version: &str) -> MemoryMarkdownBundle {
        let index = file("MEMORY.md", &format!("# Memory {version}\n"));
        let topic = file(
            "Memory/memory-one.md",
            &format!("---\nstatus: current\n---\n# Topic {version}\n"),
        );
        let bundle_sha256 = bundle_digest(&index, std::slice::from_ref(&topic));
        MemoryMarkdownBundle {
            index,
            topics: vec![topic],
            bundle_sha256,
            write_enabled: false,
        }
    }

    fn absent(logical: &str, actual: &[&str]) -> MemoryFileObservation {
        let mut parent_parts = actual.to_vec();
        parent_parts.pop();
        MemoryFileObservation::Absent {
            logical_relative_path: logical.to_owned(),
            path: path(actual),
            parent: target(&parent_parts, WorkspaceObjectKind::Directory),
            observed_sibling_names: Vec::new(),
        }
    }

    fn existing(logical: &str, actual: &[&str], bytes: &[u8]) -> MemoryFileObservation {
        MemoryFileObservation::Existing {
            logical_relative_path: logical.to_owned(),
            target: target(actual, WorkspaceObjectKind::RegularFile),
            observed_bytes: bytes.to_vec(),
            mode: 0o600,
            work_disposition: ExistingWorkDisposition::OwnedByCurrentTask,
        }
    }

    #[test]
    fn publish_and_last_good_restore_are_complete_controlled_transactions() {
        let first = bundle("one");
        let publish = compose_memory_file_plan(
            "memory-publish",
            &first,
            None,
            None,
            MemoryFileIntent::Publish,
            vec![
                absent("MEMORY.md", &["memory", "MEMORY.md"]),
                absent(
                    "Memory/memory-one.md",
                    &["memory", "Memory", "memory-one.md"],
                ),
            ],
        )
        .expect("publish plan");
        assert_eq!(publish.operations.len(), 2);
        assert!(!publish.network_used);
        assert!(!publish.automatic_decision);

        let second = bundle("two");
        let restore = compose_memory_file_plan(
            "memory-restore",
            &first,
            Some(&second.bundle_sha256),
            Some(&second.bundle_sha256),
            MemoryFileIntent::RestoreLastGood,
            vec![
                existing("MEMORY.md", &["memory", "MEMORY.md"], b"# Memory two\n"),
                existing(
                    "Memory/memory-one.md",
                    &["memory", "Memory", "memory-one.md"],
                    b"---\nstatus: current\n---\n# Topic two\n",
                ),
            ],
        )
        .expect("restore plan");
        assert!(
            restore
                .operations
                .iter()
                .all(|operation| matches!(operation, FilesystemOperationDraft::ExactPatch { .. }))
        );
    }

    #[test]
    fn simultaneous_edit_preserves_complete_conflict_bundle_and_current_identity() {
        let current = bundle("current");
        let proposed = bundle("proposed");
        let conflict = compose_memory_file_plan(
            "memory-conflict",
            &proposed,
            Some(&"9".repeat(64)),
            Some(&current.bundle_sha256),
            MemoryFileIntent::PreserveConflict,
            vec![
                absent(
                    "MEMORY.md",
                    &["memory", "Conflicts", "transaction-one", "MEMORY.md"],
                ),
                absent(
                    "Memory/memory-one.md",
                    &[
                        "memory",
                        "Conflicts",
                        "transaction-one",
                        "Memory",
                        "memory-one.md",
                    ],
                ),
            ],
        )
        .expect("conflict plan");
        assert_eq!(conflict.operations.len(), 2);
        assert_eq!(conflict.intent, MemoryFileIntent::PreserveConflict);
        assert_eq!(
            compose_memory_file_plan(
                "memory-wrong",
                &proposed,
                Some(&current.bundle_sha256),
                Some(&current.bundle_sha256),
                MemoryFileIntent::PreserveConflict,
                Vec::new(),
            ),
            Err(MemoryFileError::ConcurrentState)
        );
    }

    #[test]
    fn backup_and_portable_export_are_exact_and_authority_bounded() {
        let value = bundle("backup");
        let files = bundle_files(&value).expect("files");
        let sources = files
            .iter()
            .map(|(relative, bytes)| {
                let mut parts = vec!["memory"];
                parts.extend(relative.split('/'));
                ExistingSourceDraft {
                    target: target(&parts, WorkspaceObjectKind::RegularFile),
                    observed_bytes: bytes.clone(),
                    work_disposition: ExistingWorkDisposition::OwnedByCurrentTask,
                    mode: 0o600,
                }
            })
            .collect::<Vec<_>>();
        let destinations = files
            .iter()
            .map(|(relative, _)| {
                let mut parts = vec!["memory", "LastGood"];
                parts.extend(relative.split('/'));
                let destination_path = path(&parts);
                parts.pop();
                NewDestinationDraft {
                    parent: target(&parts, WorkspaceObjectKind::Directory),
                    path: destination_path,
                    observed_sibling_names: Vec::new(),
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(
            compose_memory_backup_plan("memory-backup", &value, sources, destinations)
                .expect("backup")
                .operations
                .len(),
            2
        );

        let encrypted = b"synthetic-authenticated-memory-envelope".to_vec();
        let digest = sha256(&encrypted);
        let export = compose_portable_memory_export(
            "memory-export".to_owned(),
            encrypted,
            &digest,
            NewDestinationDraft {
                parent: target(&["exports"], WorkspaceObjectKind::Directory),
                path: path(&["exports", "memory.age"]),
                observed_sibling_names: Vec::new(),
            },
        )
        .expect("export");
        assert!(matches!(
            export,
            FilesystemOperationDraft::Create { mode: 0o600, .. }
        ));
        assert_eq!(
            compose_portable_memory_export(
                "memory-export".to_owned(),
                b"changed".to_vec(),
                &digest,
                NewDestinationDraft {
                    parent: target(&["exports"], WorkspaceObjectKind::Directory),
                    path: path(&["exports", "memory.age"]),
                    observed_sibling_names: Vec::new(),
                },
            ),
            Err(MemoryFileError::InvalidInput)
        );
    }
}
