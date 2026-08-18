//! Exact repository-map projections for native read-only coding tools.

use std::collections::BTreeSet;
use std::fmt::Write;

use agentmage_capability_read_only::{
    ReadOnlyRequest, ReadOnlyToolKind, validate_read_only_request,
};
use agentmage_capability_repository_map::{
    RepositoryEntryDisposition, RepositoryMap, verify_repository_map,
};
use agentmage_kernel_contracts::{WorkspaceObjectKind, WorkspacePath};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::coding_session::CodingSessionProfile;

/// One exact workspace object selected from a verified repository map.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CodingProjectionObject {
    /// Canonical workspace-relative identity.
    pub path: WorkspacePath,
    /// Exact expected object kind for held-descriptor resolution.
    pub object_kind: WorkspaceObjectKind,
}

/// One stable bounded object projection for a validated native read call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodingReadProjection {
    /// Stable path-ordered exact object set.
    pub objects: Vec<CodingProjectionObject>,
    /// SHA-256 of the selected tool kind, request, and complete object set.
    pub projection_sha256: String,
}

/// Stable content-free refusal from coding repository projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodingProjectionError {
    /// The repository map is malformed or differs from the immutable session profile.
    MapBindingDenied,
    /// The read request is malformed or incompatible with its exact tool.
    RequestDenied,
    /// A requested object is absent, excluded, or has the wrong object kind.
    ObjectDenied,
    /// The selected projection exceeds the exact request ceiling.
    LimitExceeded,
}

impl CodingProjectionError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::MapBindingDenied => "runtime.coding-projection.map-binding-denied",
            Self::RequestDenied => "runtime.coding-projection.request-denied",
            Self::ObjectDenied => "runtime.coding-projection.object-denied",
            Self::LimitExceeded => "runtime.coding-projection.limit-exceeded",
        }
    }
}

/// Verified repository inventory bound to one immutable coding session.
pub struct CodingRepositoryProjection {
    map: RepositoryMap,
    objects: Vec<CodingProjectionObject>,
}

impl CodingRepositoryProjection {
    /// Binds one complete repository map to the exact session snapshot and owned worktree.
    pub fn for_profile(
        profile: &CodingSessionProfile,
        map: RepositoryMap,
    ) -> Result<Self, CodingProjectionError> {
        Self::build(
            map,
            profile.write_scope().workspace_id(),
            profile.repository_snapshot_sha256(),
            &profile.worktree().worktree_path_sha256,
            profile.immutable_base_commit(),
        )
    }

    fn build(
        map: RepositoryMap,
        workspace_id: &agentmage_kernel_contracts::WorkspaceId,
        map_sha256: &str,
        worktree_sha256: &str,
        commit_id: &str,
    ) -> Result<Self, CodingProjectionError> {
        if !verify_repository_map(&map)
            || &map.workspace_id != workspace_id
            || map.map_sha256 != map_sha256
            || map.worktree_sha256 != worktree_sha256
            || map.commit_id != commit_id
        {
            return Err(CodingProjectionError::MapBindingDenied);
        }

        let mut objects = BTreeSet::new();
        for file in &map.files {
            if excluded(file.disposition) {
                continue;
            }
            for depth in 1..file.path.components().len() {
                let path = WorkspacePath::new(
                    map.workspace_id.clone(),
                    file.path.components()[..depth]
                        .iter()
                        .map(|component| component.as_str()),
                )
                .map_err(|_| CodingProjectionError::MapBindingDenied)?;
                objects.insert(CodingProjectionObject {
                    path,
                    object_kind: WorkspaceObjectKind::Directory,
                });
            }
            objects.insert(CodingProjectionObject {
                path: file.path.clone(),
                object_kind: WorkspaceObjectKind::RegularFile,
            });
        }
        Ok(Self {
            map,
            objects: objects.into_iter().collect(),
        })
    }

    /// Selects the complete exact object projection for one validated native read request.
    pub fn select(
        &self,
        kind: ReadOnlyToolKind,
        request: &ReadOnlyRequest,
    ) -> Result<CodingReadProjection, CodingProjectionError> {
        let request_bytes =
            serde_json::to_vec(request).map_err(|_| CodingProjectionError::RequestDenied)?;
        validate_read_only_request(kind, &request_bytes)
            .map_err(|_| CodingProjectionError::RequestDenied)?;
        let roots = request
            .paths
            .iter()
            .map(|components| {
                WorkspacePath::new(self.map.workspace_id.clone(), components.iter().cloned())
                    .map_err(|_| CodingProjectionError::RequestDenied)
            })
            .collect::<Result<Vec<_>, _>>()?;

        if roots
            .iter()
            .any(|root| !self.objects.iter().any(|candidate| &candidate.path == root))
        {
            return Err(CodingProjectionError::ObjectDenied);
        }

        let mut selected = self
            .objects
            .iter()
            .filter(|object| selects(kind, request, &roots, object))
            .cloned()
            .collect::<Vec<_>>();
        selected.sort();
        selected.dedup();
        if selected.is_empty() {
            return Err(CodingProjectionError::ObjectDenied);
        }
        if selected.len() > request.limits.files as usize {
            return Err(CodingProjectionError::LimitExceeded);
        }
        let projection_sha256 = sha256_json(&ProjectionMaterial {
            tool: kind.id(),
            request,
            objects: &selected,
        })?;
        Ok(CodingReadProjection {
            objects: selected,
            projection_sha256,
        })
    }
}

fn selects(
    kind: ReadOnlyToolKind,
    request: &ReadOnlyRequest,
    roots: &[WorkspacePath],
    object: &CodingProjectionObject,
) -> bool {
    roots.iter().any(|root| {
        let exact = object.path == *root;
        let descendant_depth = object
            .path
            .components()
            .strip_prefix(root.components())
            .map(|suffix| suffix.len());
        match kind {
            ReadOnlyToolKind::ReadText
            | ReadOnlyToolKind::ReadMultiple
            | ReadOnlyToolKind::Metadata
            | ReadOnlyToolKind::HashFile
            | ReadOnlyToolKind::BinaryMetadata => exact,
            ReadOnlyToolKind::ListDirectory => exact || descendant_depth == Some(1),
            ReadOnlyToolKind::DirectoryTree
            | ReadOnlyToolKind::SearchFilenames
            | ReadOnlyToolKind::SearchText
            | ReadOnlyToolKind::HashTree => {
                descendant_depth.is_some_and(|depth| depth <= request.limits.depth as usize)
            }
        }
    }) && kind_accepts_object(kind, object.object_kind)
}

fn kind_accepts_object(kind: ReadOnlyToolKind, object_kind: WorkspaceObjectKind) -> bool {
    match kind {
        ReadOnlyToolKind::ReadText
        | ReadOnlyToolKind::ReadMultiple
        | ReadOnlyToolKind::SearchText
        | ReadOnlyToolKind::HashFile
        | ReadOnlyToolKind::BinaryMetadata => object_kind == WorkspaceObjectKind::RegularFile,
        ReadOnlyToolKind::ListDirectory
        | ReadOnlyToolKind::DirectoryTree
        | ReadOnlyToolKind::SearchFilenames
        | ReadOnlyToolKind::Metadata
        | ReadOnlyToolKind::HashTree => true,
    }
}

const fn excluded(disposition: RepositoryEntryDisposition) -> bool {
    matches!(
        disposition,
        RepositoryEntryDisposition::GitIgnored
            | RepositoryEntryDisposition::PolicyExcluded
            | RepositoryEntryDisposition::GeneratedExcluded
            | RepositoryEntryDisposition::VendoredExcluded
    )
}

#[derive(Serialize)]
struct ProjectionMaterial<'projection> {
    tool: &'static str,
    request: &'projection ReadOnlyRequest,
    objects: &'projection [CodingProjectionObject],
}

fn sha256_json(value: &impl Serialize) -> Result<String, CodingProjectionError> {
    let bytes = serde_json::to_vec(value).map_err(|_| CodingProjectionError::MapBindingDenied)?;
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use agentmage_capability_read_only::{ReadOnlyEncoding, ReadOnlyLimits};
    use agentmage_capability_repository_map::{
        GitTrackedState, RepositoryFileInput, RepositoryMapInput, build_repository_map,
    };
    use agentmage_kernel_contracts::WorkspaceId;

    use super::*;

    fn map() -> RepositoryMap {
        let bytes = |value: &[u8]| RepositoryFileInput {
            path: Vec::new(),
            size_bytes: value.len() as u64,
            content_sha256: sha256_bytes(value),
            content: Some(value.to_vec()),
            object_kind: agentmage_capability_repository_map::RepositoryObjectKind::RegularFile,
            git_state: GitTrackedState::TrackedClean,
            policy_excluded: false,
            generated: false,
            vendored: false,
        };
        let mut lib = bytes(b"pub fn run() {}\n");
        lib.path = vec!["src".to_owned(), "lib.rs".to_owned()];
        let mut nested = bytes(b"pub fn nested() {}\n");
        nested.path = vec!["src".to_owned(), "nested".to_owned(), "mod.rs".to_owned()];
        let mut test = bytes(b"#[test]\nfn works() {}\n");
        test.path = vec!["tests".to_owned(), "runtime.rs".to_owned()];
        let mut excluded = bytes(b"secret\n");
        excluded.path = vec!["private".to_owned(), "secret.txt".to_owned()];
        excluded.content = None;
        excluded.policy_excluded = true;
        build_repository_map(RepositoryMapInput {
            workspace_id: WorkspaceId::from_raw("workspace-coding"),
            repository_sha256: "a".repeat(64),
            worktree_sha256: "b".repeat(64),
            branch: Some("agentmage/task".to_owned()),
            commit_id: "c".repeat(40),
            policy_sha256: "d".repeat(64),
            freshness_sha256: "e".repeat(64),
            files: vec![lib, nested, test, excluded],
        })
        .expect("repository map")
    }

    fn projection() -> CodingRepositoryProjection {
        let map = map();
        let map_sha256 = map.map_sha256.clone();
        CodingRepositoryProjection::build(
            map,
            &WorkspaceId::from_raw("workspace-coding"),
            &map_sha256,
            &"b".repeat(64),
            &"c".repeat(40),
        )
        .expect("projection")
    }

    fn request(path: &[&str], depth: u16, files: u32) -> ReadOnlyRequest {
        ReadOnlyRequest {
            schema_version: 1,
            paths: vec![path.iter().map(|value| (*value).to_owned()).collect()],
            query: None,
            byte_offset: None,
            byte_count: None,
            encoding: ReadOnlyEncoding::Binary,
            limits: ReadOnlyLimits {
                files,
                depth,
                ..ReadOnlyLimits::default()
            },
            call_depth: 0,
        }
    }

    #[test]
    fn story_48_2_projection_binds_map_worktree_commit_and_workspace() {
        let map = map();
        let map_sha256 = map.map_sha256.clone();
        assert!(
            CodingRepositoryProjection::build(
                map.clone(),
                &WorkspaceId::from_raw("workspace-coding"),
                &map_sha256,
                &"b".repeat(64),
                &"c".repeat(40),
            )
            .is_ok()
        );
        assert!(matches!(
            CodingRepositoryProjection::build(
                map,
                &WorkspaceId::from_raw("workspace-coding"),
                &"f".repeat(64),
                &"b".repeat(64),
                &"c".repeat(40),
            ),
            Err(CodingProjectionError::MapBindingDenied)
        ));
    }

    #[test]
    fn story_48_2_projection_selects_exact_files_and_bounded_descendants() {
        let projection = projection();
        let mut read = request(&["src", "lib.rs"], 0, 8);
        read.encoding = ReadOnlyEncoding::Utf8;
        let selected = projection
            .select(ReadOnlyToolKind::ReadText, &read)
            .expect("file projection");
        assert_eq!(selected.objects.len(), 1);
        assert_eq!(
            selected.objects[0].object_kind,
            WorkspaceObjectKind::RegularFile
        );

        let tree = projection
            .select(ReadOnlyToolKind::DirectoryTree, &request(&["src"], 2, 8))
            .expect("tree projection");
        let paths = tree
            .objects
            .iter()
            .map(|object| {
                object
                    .path
                    .components()
                    .iter()
                    .map(|component| component.as_str())
                    .collect::<Vec<_>>()
                    .join("/")
            })
            .collect::<Vec<_>>();
        assert_eq!(
            paths,
            ["src", "src/lib.rs", "src/nested", "src/nested/mod.rs"]
        );
    }

    #[test]
    fn story_48_2_projection_excludes_policy_paths_and_enforces_request_limits() {
        let projection = projection();
        assert!(matches!(
            projection.select(
                ReadOnlyToolKind::DirectoryTree,
                &request(&["private"], 2, 8)
            ),
            Err(CodingProjectionError::ObjectDenied)
        ));
        assert!(matches!(
            projection.select(ReadOnlyToolKind::DirectoryTree, &request(&["src"], 2, 2)),
            Err(CodingProjectionError::LimitExceeded)
        ));
    }

    fn sha256_bytes(bytes: &[u8]) -> String {
        let mut output = String::with_capacity(64);
        for byte in Sha256::digest(bytes) {
            write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
        }
        output
    }
}
