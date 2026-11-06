// SPDX-License-Identifier: Apache-2.0
//! Metadata-only path discovery. No Git, file content, instructions or model calls.

use std::collections::BTreeSet;
use std::path::Path;
use std::time::{Duration, Instant};

use agentmage_kernel_contracts::{
    AdapterInstanceId, HeldWorkspaceObject, PathAdapterError, PathAdapterErrorKind,
    PathResolutionIntent, PlatformPathAdapter, WorkspaceAuthorizationId, WorkspaceId,
    WorkspaceObjectKind, WorkspacePath,
};

use crate::{LinuxCodingReadScope, LinuxPathAdapter, authorize_workspace_root};

/// Content-free discovery refusal; an incomplete traversal returns no map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxCodingMapError {
    /// Invalid absolute root or empty, oversized or non-searchable query.
    Input,
    /// Unsafe root, changed directory or failed metadata observation.
    Boundary,
    /// Fixed traversal budget exceeded; no prefix is usable as a complete map.
    Limit,
    /// Caller cancelled before publication.
    Cancelled,
}

impl LinuxCodingMapError {
    /// Stable public refusal without paths or OS diagnostics.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Input => "code.map.input",
            Self::Boundary => "code.map.boundary",
            Self::Limit => "code.map.limit",
            Self::Cancelled => "code.map.cancelled",
        }
    }
}

/// One safe regular-file path and its lexical path relevance, without source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxCodingMapEntry {
    /// Canonical relative ASCII path compatible with exact source selection.
    pub path: String,
    /// Number of distinct task terms found in the path, with basename weighting.
    pub score: usize,
}

/// A bounded directory view, not a Git inventory or a content search result.
pub struct LinuxCodingMap {
    /// Canonical root path/device/inode binding; not source or effect authority.
    pub root_sha256: String,
    /// All eligible paths, sorted by descending score then ascending path.
    pub entries: Vec<LinuxCodingMapEntry>,
    /// Number of directories actually enumerated, including the root.
    pub directories: usize,
    /// All names observed, including excluded and unsafe names.
    pub names: usize,
    /// Hidden/generated directories and hidden files excluded without opening.
    pub excluded: usize,
    /// Unsafe, special, linked or non-selectable entries omitted without content.
    pub skipped: usize,
}

fn observation(error: PathAdapterError) -> LinuxCodingMapError {
    if error.kind() == PathAdapterErrorKind::ResourceLimitExceeded {
        LinuxCodingMapError::Limit
    } else {
        LinuxCodingMapError::Boundary
    }
}

/// Enumerates an explicitly approved root through the existing native adapter.
/// Never call this at an untrusted model/tool-choice boundary. This one-shot
/// metadata consent does not select sources, disclose to a model or allow edits.
/// Fixed ceilings: a cooperative 2 s deadline, 256 directories, depth 16,
/// 8192 names, 4096 files;
/// each directory is bounded to 4096 names and 256 KiB of UTF-8 names.
/// A blocked filesystem syscall can delay cancellation and the deadline.
pub fn discover_linux_coding_map(
    root: &Path,
    query: &str,
    mut cancelled: impl FnMut() -> bool,
) -> Result<LinuxCodingMap, LinuxCodingMapError> {
    if !root.is_absolute()
        || root.as_os_str().len() > 4096
        || root.components().count() > 256
        || query.len() > 1024
        || query.chars().any(char::is_control)
    {
        return Err(LinuxCodingMapError::Input);
    }
    let terms = query
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(str::to_ascii_lowercase)
        .collect::<BTreeSet<_>>();
    if terms.is_empty() || terms.len() > 64 {
        return Err(LinuxCodingMapError::Input);
    }
    let started = Instant::now();
    let mut check = || {
        if cancelled() {
            Err(LinuxCodingMapError::Cancelled)
        } else if started.elapsed() >= Duration::from_secs(2) {
            Err(LinuxCodingMapError::Limit)
        } else {
            Ok(())
        }
    };
    check()?;
    let id = WorkspaceId::from_raw("local-coding-map");
    let adapter_id = AdapterInstanceId::from_raw("local-coding-map");
    let workspace = authorize_workspace_root(
        root,
        id.clone(),
        WorkspaceAuthorizationId::from_raw("local-coding-map"),
        adapter_id.clone(),
    )
    .map_err(observation)?;
    let adapter = LinuxPathAdapter::new(adapter_id, 1);
    let mut output = LinuxCodingMap {
        root_sha256: workspace.root_path_sha256().to_owned(),
        entries: Vec::new(),
        directories: 0,
        names: 0,
        excluded: 0,
        skipped: 0,
    };
    let mut pending = vec![Vec::<String>::new()];
    let mut held_directories = Vec::new();
    while let Some(components) = pending.pop() {
        check()?;
        output.directories += 1;
        if output.directories > 256 || components.len() > 16 {
            return Err(LinuxCodingMapError::Limit);
        }
        let names = if components.is_empty() {
            workspace.observe_root_names(4096, 256 * 1024)
        } else {
            let path = WorkspacePath::new(id.clone(), components.clone())
                .map_err(|_| LinuxCodingMapError::Boundary)?;
            let held = adapter
                .resolve(&workspace, &path, PathResolutionIntent::ReadDirectory)
                .map_err(observation)?;
            let names = held.observe_directory_names(4096, 256 * 1024);
            held_directories.push(held);
            names
        }
        .map_err(observation)?;
        output.names += names.len();
        if output.names > 8192 {
            return Err(LinuxCodingMapError::Limit);
        }
        for name in names {
            check()?;
            // Never enter hidden objects, Git metadata, dependency trees or
            // generated output. This is intentionally not Git-ignore parsing.
            if name.starts_with('.')
                || matches!(
                    name.to_ascii_lowercase().as_str(),
                    "target" | "build" | "dist" | "node_modules" | "venv" | "__pycache__"
                )
            {
                output.excluded += 1;
                continue;
            }
            let mut child = components.clone();
            child.push(name);
            let text = child.join("/");
            let path = WorkspacePath::new(id.clone(), child.clone());
            if text.len() > 256
                || !text
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-/".contains(&b))
                || path.is_err()
            {
                output.skipped += 1;
                continue;
            }
            let held = match adapter.resolve(
                &workspace,
                &path.map_err(|_| LinuxCodingMapError::Boundary)?,
                PathResolutionIntent::Metadata,
            ) {
                Ok(held) => held,
                Err(error)
                    if matches!(
                        error.kind(),
                        PathAdapterErrorKind::SymbolicLink
                            | PathAdapterErrorKind::HardLink
                            | PathAdapterErrorKind::ObjectKindMismatch
                    ) =>
                {
                    output.skipped += 1;
                    continue;
                }
                Err(error) => return Err(observation(error)),
            };
            match held.object_kind() {
                WorkspaceObjectKind::Directory => pending.push(child),
                WorkspaceObjectKind::RegularFile => {
                    let lower = text.to_ascii_lowercase();
                    let basename = lower.rsplit('/').next().unwrap_or(&lower);
                    let score = terms
                        .iter()
                        .map(|term| {
                            usize::from(lower.contains(term.as_str()))
                                + 2 * usize::from(basename.contains(term.as_str()))
                        })
                        .sum();
                    output
                        .entries
                        .push(LinuxCodingMapEntry { path: text, score });
                    if output.entries.len() > 4096 {
                        return Err(LinuxCodingMapError::Limit);
                    }
                }
            }
        }
    }
    // Re-resolve every enumerated directory so a path swap or detached subtree
    // cannot silently become the basis for selecting different source paths.
    for held in held_directories {
        check()?;
        held.revalidate().map_err(observation)?;
        let current = adapter
            .resolve(
                &workspace,
                held.workspace_path(),
                PathResolutionIntent::ReadDirectory,
            )
            .map_err(observation)?;
        if current.object_identity() != held.object_identity() {
            return Err(LinuxCodingMapError::Boundary);
        }
    }
    LinuxCodingReadScope::verify_root(root, &output.root_sha256)
        .map_err(|_| LinuxCodingMapError::Boundary)?;
    check()?;
    output
        .entries
        .sort_by(|a, b| b.score.cmp(&a.score).then(a.path.cmp(&b.path)));
    Ok(output)
}
