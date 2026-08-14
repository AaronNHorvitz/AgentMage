//! Exact source and Git resolution for parser-backed structural records.

use std::fmt::Write;

use agentmage_kernel_contracts::WorkspacePath;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    RepositoryMap, SourceRange, StructuralItemKind, grammar_descriptor, verify_repository_map,
};

/// Exact repository and worktree identity attached to one structural fact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryGitIdentity {
    /// Stable repository identity digest, never a remote URL.
    pub repository_sha256: String,
    /// Exact held worktree identity digest.
    pub worktree_sha256: String,
    /// Exact branch, or no branch for detached HEAD.
    pub branch: Option<String>,
    /// Exact Git commit object identity.
    pub commit_id: String,
}

/// One structural record resolved to exact source, parser, and Git evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuralSourceResolution {
    /// Canonical workspace-relative source path.
    pub path: WorkspacePath,
    /// Exact complete file content digest.
    pub content_sha256: String,
    /// Exact source range of the parser-backed syntax node.
    pub range: SourceRange,
    /// Exact digest of the syntax-node bytes.
    pub syntax_sha256: String,
    /// Closed structural kind.
    pub kind: StructuralItemKind,
    /// Bounded control-safe name or import declaration.
    pub name: String,
    /// Exact compiled grammar descriptor identity.
    pub grammar_sha256: String,
    /// Exact parser runtime version.
    pub parser_version: String,
    /// Exact repository, worktree, branch, and commit identity.
    pub git: RepositoryGitIdentity,
    /// SHA-256 over every preceding resolution field.
    pub resolution_sha256: String,
}

/// Resolves every retained structural item in stable file and range order.
#[must_use]
pub fn resolve_structural_records(map: &RepositoryMap) -> Vec<StructuralSourceResolution> {
    if !verify_repository_map(map) {
        return Vec::new();
    }
    let git = RepositoryGitIdentity {
        repository_sha256: map.repository_sha256.clone(),
        worktree_sha256: map.worktree_sha256.clone(),
        branch: map.branch.clone(),
        commit_id: map.commit_id.clone(),
    };
    let mut resolutions = Vec::new();
    for file in &map.files {
        let Some(structure) = &file.structure else {
            continue;
        };
        let descriptor = grammar_descriptor(structure.language);
        for item in &structure.items {
            let mut resolution = StructuralSourceResolution {
                path: file.path.clone(),
                content_sha256: file.content_sha256.clone(),
                range: item.range,
                syntax_sha256: item.source_sha256.clone(),
                kind: item.kind,
                name: item.name.clone(),
                grammar_sha256: descriptor.descriptor_sha256.clone(),
                parser_version: descriptor.parser_version.clone(),
                git: git.clone(),
                resolution_sha256: String::new(),
            };
            resolution.resolution_sha256 = resolution_digest(&resolution);
            resolutions.push(resolution);
        }
    }
    resolutions
}

/// Verifies a resolution against one current complete repository map.
#[must_use]
pub fn verify_structural_source_resolution(
    map: &RepositoryMap,
    resolution: &StructuralSourceResolution,
) -> bool {
    if !verify_repository_map(map)
        || resolution.git.repository_sha256 != map.repository_sha256
        || resolution.git.worktree_sha256 != map.worktree_sha256
        || resolution.git.branch != map.branch
        || resolution.git.commit_id != map.commit_id
        || resolution.resolution_sha256 != resolution_digest(resolution)
    {
        return false;
    }
    map.files.iter().any(|file| {
        file.path == resolution.path
            && file.content_sha256 == resolution.content_sha256
            && file.structure.as_ref().is_some_and(|structure| {
                let descriptor = grammar_descriptor(structure.language);
                descriptor.descriptor_sha256 == resolution.grammar_sha256
                    && descriptor.parser_version == resolution.parser_version
                    && structure.items.iter().any(|item| {
                        item.kind == resolution.kind
                            && item.name == resolution.name
                            && item.range == resolution.range
                            && item.source_sha256 == resolution.syntax_sha256
                    })
            })
    })
}

fn resolution_digest(resolution: &StructuralSourceResolution) -> String {
    serde_json::to_vec(&(
        &resolution.path,
        &resolution.content_sha256,
        resolution.range,
        &resolution.syntax_sha256,
        resolution.kind,
        &resolution.name,
        &resolution.grammar_sha256,
        &resolution.parser_version,
        &resolution.git,
    ))
    .map(|bytes| sha256_hex(&bytes))
    .unwrap_or_else(|_| sha256_hex(b"repository-resolution-serialization-failed"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::WorkspaceId;

    use super::{resolve_structural_records, sha256_hex, verify_structural_source_resolution};
    use crate::{GitTrackedState, RepositoryFileInput, RepositoryMapInput, build_repository_map};

    fn exact_map() -> crate::RepositoryMap {
        let content = b"use std::fmt;\npub struct Item;\npub fn run() {}\n".to_vec();
        build_repository_map(RepositoryMapInput {
            workspace_id: WorkspaceId::from_raw("workspace-resolution"),
            repository_sha256: "a".repeat(64),
            worktree_sha256: "b".repeat(64),
            branch: Some("main".to_owned()),
            commit_id: "c".repeat(40),
            policy_sha256: "d".repeat(64),
            freshness_sha256: "e".repeat(64),
            files: vec![RepositoryFileInput {
                path: vec!["src".to_owned(), "lib.rs".to_owned()],
                size_bytes: content.len() as u64,
                content_sha256: sha256_hex(&content),
                content: Some(content),
                git_state: GitTrackedState::TrackedClean,
                policy_excluded: false,
                generated: false,
                vendored: false,
            }],
        })
        .expect("map")
    }

    #[test]
    fn every_structural_fact_resolves_to_exact_current_source_parser_and_git() {
        let map = exact_map();
        let first = resolve_structural_records(&map);
        let second = resolve_structural_records(&map);
        assert_eq!(first, second);
        assert_eq!(first.len(), 4);
        assert!(
            first
                .iter()
                .all(|resolution| verify_structural_source_resolution(&map, resolution))
        );
    }

    #[test]
    fn every_source_parser_and_git_identity_mutation_is_stale() {
        let map = exact_map();
        let exact = resolve_structural_records(&map).remove(0);
        let mutations = [
            ("content", "f".repeat(64)),
            ("grammar", "f".repeat(64)),
            ("worktree", "f".repeat(64)),
            ("repository", "f".repeat(64)),
        ];
        for (field, value) in mutations {
            let mut changed = exact.clone();
            match field {
                "content" => changed.content_sha256 = value,
                "grammar" => changed.grammar_sha256 = value,
                "worktree" => changed.git.worktree_sha256 = value,
                "repository" => changed.git.repository_sha256 = value,
                _ => unreachable!(),
            }
            assert!(!verify_structural_source_resolution(&map, &changed));
        }
        let mut changed_range = exact.clone();
        changed_range.range.start_byte += 1;
        assert!(!verify_structural_source_resolution(&map, &changed_range));
        let mut changed_commit = exact;
        changed_commit.git.commit_id = "f".repeat(40);
        assert!(!verify_structural_source_resolution(&map, &changed_commit));
    }
}
