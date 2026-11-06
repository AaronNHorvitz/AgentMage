// SPDX-License-Identifier: Apache-2.0
//! Explicit source selection for the local coding API. Read consent alone
//! confers no writes; separately approved changes use the native kernel driver.

use std::collections::BTreeMap;
use std::path::Path;

use agentmage_kernel_contracts::{
    AdapterInstanceId, HeldWorkspaceObject, PathResolutionIntent, PlatformPathAdapter,
    WorkspaceAuthorizationId, WorkspaceId, WorkspacePath,
};

use crate::{LinuxAuthorizedWorkspace, LinuxPathAdapter, authorize_workspace_root};

#[path = "coding_workspace_write.rs"]
mod write;
pub use write::{LinuxCodingPendingWrite, LinuxCodingReplacement, LinuxCodingWriteError};

const MAX_FILES: usize = 16;
const MAX_BYTES: usize = 128 * 1024;

/// Content-free refusal from an explicitly selected read-only source scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxCodingReadError {
    /// Invalid/duplicate/reserved selection, or an unselected requested path.
    Selection,
    /// Native path resolution, identity checks or exact reads refused the file.
    Boundary,
    /// Nonempty UTF-8 text without NUL and the source byte budget are required.
    Content,
}

impl LinuxCodingReadError {
    /// Stable public code; OS errors and absolute paths are never included.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Selection => "code.workspace.selection",
            Self::Boundary => "code.workspace.boundary",
            Self::Content => "code.workspace.content",
        }
    }
}

/// A complete exact file read. The path is relative, never an ambient grant.
pub struct LinuxCodingSource {
    /// Selected workspace-relative display path.
    pub path: String,
    /// Complete revalidated UTF-8 file bytes.
    pub content: String,
}

/// A continuously held directory and an immutable exact path allowlist. Call
/// `select` only at a trusted explicit person-choice boundary. This type cannot
/// execute, enumerate other paths or activate a production platform. Preparing
/// a shadow edit is inert; applying it requires a separate trusted decision.
pub struct LinuxCodingReadScope {
    workspace: LinuxAuthorizedWorkspace,
    adapter: LinuxPathAdapter,
    paths: BTreeMap<String, WorkspacePath>,
}

impl LinuxCodingReadScope {
    /// Verifies a person's displayed root against an API preview's held identity.
    /// Opens directory metadata only; grants no source, write or command access.
    pub fn verify_root(root: &Path, expected_sha256: &str) -> Result<(), LinuxCodingReadError> {
        if !root.is_absolute()
            || root.as_os_str().len() > 4096
            || root.components().count() > 256
            || expected_sha256.len() != 64
            || !expected_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(LinuxCodingReadError::Selection);
        }
        let held = authorize_workspace_root(
            root,
            WorkspaceId::from_raw("local-coding-read"),
            WorkspaceAuthorizationId::from_raw("local-coding-read"),
            AdapterInstanceId::from_raw("local-coding-read"),
        )
        .map_err(|_| LinuxCodingReadError::Boundary)?;
        if held.root_path_sha256() != expected_sha256 {
            return Err(LinuxCodingReadError::Boundary);
        }
        Ok(())
    }

    /// Selects an absolute root with no symlink ancestors, validates a bounded
    /// allowlist, and checks every selected source before returning a grant.
    /// Later reads resolve afresh beneath the held root through the same native
    /// no-symlink/no-mount-crossing/hardlink-refusing adapter.
    pub fn select(root: &Path, paths: &[String]) -> Result<Self, LinuxCodingReadError> {
        if paths.is_empty()
            || paths.len() > MAX_FILES
            || root.as_os_str().len() > 4096
            || root.components().count() > 256
        {
            return Err(LinuxCodingReadError::Selection);
        }
        let workspace_id = WorkspaceId::from_raw("local-coding-read");
        let adapter_id = AdapterInstanceId::from_raw("local-coding-read");
        let mut selected = BTreeMap::new();
        let mut folded = std::collections::BTreeSet::new();
        for path in paths {
            // Stricter than display paths: canonical kernel path checks also
            // reject ambiguous suffixes. Git metadata and case aliases refuse.
            if path.len() > 256
                || !path
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-/".contains(&b))
                || path.split('/').any(|p| p.eq_ignore_ascii_case(".git"))
                || !folded.insert(path.to_ascii_lowercase())
            {
                return Err(LinuxCodingReadError::Selection);
            }
            let canonical = WorkspacePath::new(workspace_id.clone(), path.split('/'))
                .map_err(|_| LinuxCodingReadError::Selection)?;
            selected.insert(path.clone(), canonical);
        }
        let workspace = authorize_workspace_root(
            root,
            workspace_id,
            WorkspaceAuthorizationId::from_raw("local-coding-read"),
            adapter_id.clone(),
        )
        .map_err(|_| LinuxCodingReadError::Boundary)?;
        let scope = Self {
            workspace,
            adapter: LinuxPathAdapter::new(adapter_id, MAX_BYTES as u64),
            paths: selected,
        };
        scope.read(paths)?;
        Ok(scope)
    }

    /// Sorted selected relative names only. No file or directory is opened.
    #[must_use]
    pub fn paths(&self) -> Vec<String> {
        self.paths.keys().cloned().collect()
    }

    /// Reads only a nonempty unique subset of the selected paths. Budgets apply
    /// to the whole result; any failure refuses the whole batch, never a prefix.
    /// Held files are revalidated together after reading and resolved a second
    /// time to refuse path replacements before returning source snapshots.
    pub fn read(&self, paths: &[String]) -> Result<Vec<LinuxCodingSource>, LinuxCodingReadError> {
        if paths.is_empty() || paths.len() > MAX_FILES {
            return Err(LinuxCodingReadError::Selection);
        }
        let mut requested = std::collections::BTreeSet::new();
        for path in paths {
            if !self.paths.contains_key(path) || !requested.insert(path) {
                return Err(LinuxCodingReadError::Selection);
            }
        }
        let mut bytes = 0;
        let mut sources = Vec::new();
        let mut held = Vec::new();
        for path in requested {
            let object = self
                .adapter
                .resolve(
                    &self.workspace,
                    &self.paths[path],
                    PathResolutionIntent::ReadFile,
                )
                .map_err(|_| LinuxCodingReadError::Boundary)?;
            let content = object
                .read_exact_bytes()
                .map_err(|_| LinuxCodingReadError::Boundary)?;
            bytes += content.len();
            if content.is_empty() || content.contains(&0) || bytes > MAX_BYTES {
                return Err(LinuxCodingReadError::Content);
            }
            let content = String::from_utf8(content).map_err(|_| LinuxCodingReadError::Content)?;
            sources.push(LinuxCodingSource {
                path: path.clone(),
                content,
            });
            held.push((path, object));
        }
        for (path, object) in held {
            object
                .revalidate()
                .map_err(|_| LinuxCodingReadError::Boundary)?;
            let current = self
                .adapter
                .resolve(
                    &self.workspace,
                    &self.paths[path],
                    PathResolutionIntent::ReadFile,
                )
                .map_err(|_| LinuxCodingReadError::Boundary)?;
            if current.object_identity() != object.object_identity()
                || current.preimage() != object.preimage()
            {
                return Err(LinuxCodingReadError::Boundary);
            }
        }
        self.workspace
            .revalidate()
            .map_err(|_| LinuxCodingReadError::Boundary)?;
        Ok(sources)
    }
}
