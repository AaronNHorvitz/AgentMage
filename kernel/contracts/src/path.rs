//! Canonical workspace-relative path contracts.

use std::fmt;

use serde::{Deserialize, Deserializer};
use unicode_normalization::UnicodeNormalization;

use crate::WorkspaceId;

/// Maximum number of relative components in one workspace path.
pub const MAX_WORKSPACE_PATH_COMPONENTS: usize = 256;

/// Maximum UTF-8 byte length of one normalized path component.
pub const MAX_WORKSPACE_PATH_COMPONENT_BYTES: usize = 255;

const MAX_WORKSPACE_ID_BYTES: usize = 128;

/// Stable, content-free reason that a workspace path candidate was rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspacePathErrorKind {
    /// The workspace identity is empty, oversized, or syntactically invalid.
    InvalidWorkspaceIdentity,
    /// A workspace-relative path contains no components.
    EmptyPath,
    /// A workspace-relative path contains too many components.
    TooManyComponents,
    /// One path component is empty.
    EmptyComponent,
    /// One component is the current-directory marker.
    CurrentDirectoryComponent,
    /// One component is the parent-directory marker.
    ParentTraversalComponent,
    /// One component contains a path separator.
    SeparatorInComponent,
    /// One component contains a NUL scalar.
    NulInComponent,
    /// One component contains a control scalar.
    ControlInComponent,
    /// One component exceeds the bounded UTF-8 size.
    OversizedComponent,
    /// One component is not in canonical Unicode NFC form.
    NonCanonicalUnicode,
}

impl WorkspacePathErrorKind {
    /// Returns the stable redacted code for this failure class.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidWorkspaceIdentity => "path.workspace.invalid",
            Self::EmptyPath => "path.components.empty",
            Self::TooManyComponents => "path.components.exceeded",
            Self::EmptyComponent => "path.component.empty",
            Self::CurrentDirectoryComponent => "path.component.current_directory",
            Self::ParentTraversalComponent => "path.component.parent_traversal",
            Self::SeparatorInComponent => "path.component.separator",
            Self::NulInComponent => "path.component.nul",
            Self::ControlInComponent => "path.component.control",
            Self::OversizedComponent => "path.component.size_exceeded",
            Self::NonCanonicalUnicode => "path.component.noncanonical_unicode",
        }
    }
}

/// Content-free failure returned while constructing a canonical workspace path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspacePathError {
    kind: WorkspacePathErrorKind,
    component_index: Option<usize>,
}

impl WorkspacePathError {
    /// Returns the stable failure class.
    #[must_use]
    pub const fn kind(&self) -> WorkspacePathErrorKind {
        self.kind
    }

    /// Returns only the unsafe component index, never its content.
    #[must_use]
    pub const fn component_index(&self) -> Option<usize> {
        self.component_index
    }
}

impl fmt::Display for WorkspacePathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.component_index {
            Some(index) => write!(formatter, "{} at component {index}", self.kind.code()),
            None => formatter.write_str(self.kind.code()),
        }
    }
}

impl std::error::Error for WorkspacePathError {}

/// One validated, normalized relative component.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
#[serde(transparent)]
pub struct WorkspacePathComponent(String);

impl WorkspacePathComponent {
    /// Returns the normalized component value.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for WorkspacePathComponent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let candidate = String::deserialize(deserializer)?;
        validate_component(&candidate, 0).map_err(serde::de::Error::custom)
    }
}

/// Exact workspace identity plus canonical relative path components.
///
/// Fields are private so every constructed or deserialized value passes the same
/// invariant checks. This value contains no ambient root, absolute path, descriptor,
/// URL, or filesystem-observation capability.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
pub struct WorkspacePath {
    workspace_id: WorkspaceId,
    components: Vec<WorkspacePathComponent>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspacePathWire {
    workspace_id: WorkspaceId,
    components: Vec<String>,
}

impl WorkspacePath {
    /// Constructs a canonical workspace-relative path from already separated components.
    pub fn new<I, S>(workspace_id: WorkspaceId, components: I) -> Result<Self, WorkspacePathError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        validate_workspace_id(&workspace_id)?;
        let candidates: Vec<String> = components.into_iter().map(Into::into).collect();
        if candidates.is_empty() {
            return Err(path_error(WorkspacePathErrorKind::EmptyPath, None));
        }
        if candidates.len() > MAX_WORKSPACE_PATH_COMPONENTS {
            return Err(path_error(WorkspacePathErrorKind::TooManyComponents, None));
        }
        let components = candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| validate_component(candidate, index))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            workspace_id,
            components,
        })
    }

    /// Returns the exact approved workspace identity.
    #[must_use]
    pub const fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    /// Returns the ordered canonical relative components.
    #[must_use]
    pub fn components(&self) -> &[WorkspacePathComponent] {
        &self.components
    }
}

impl<'de> Deserialize<'de> for WorkspacePath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = WorkspacePathWire::deserialize(deserializer)?;
        Self::new(wire.workspace_id, wire.components).map_err(serde::de::Error::custom)
    }
}

fn validate_workspace_id(workspace_id: &WorkspaceId) -> Result<(), WorkspacePathError> {
    let value = workspace_id.as_str();
    if value.is_empty()
        || value.len() > MAX_WORKSPACE_ID_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(path_error(
            WorkspacePathErrorKind::InvalidWorkspaceIdentity,
            None,
        ));
    }
    Ok(())
}

fn validate_component(
    candidate: &str,
    index: usize,
) -> Result<WorkspacePathComponent, WorkspacePathError> {
    let kind = if candidate.is_empty() {
        Some(WorkspacePathErrorKind::EmptyComponent)
    } else if candidate == "." {
        Some(WorkspacePathErrorKind::CurrentDirectoryComponent)
    } else if candidate == ".." {
        Some(WorkspacePathErrorKind::ParentTraversalComponent)
    } else if candidate.contains(['/', '\\']) {
        Some(WorkspacePathErrorKind::SeparatorInComponent)
    } else if candidate.contains('\0') {
        Some(WorkspacePathErrorKind::NulInComponent)
    } else if candidate.chars().any(char::is_control) {
        Some(WorkspacePathErrorKind::ControlInComponent)
    } else if candidate.len() > MAX_WORKSPACE_PATH_COMPONENT_BYTES {
        Some(WorkspacePathErrorKind::OversizedComponent)
    } else if candidate.nfc().ne(candidate.chars()) {
        Some(WorkspacePathErrorKind::NonCanonicalUnicode)
    } else {
        None
    };
    match kind {
        Some(kind) => Err(path_error(kind, Some(index))),
        None => Ok(WorkspacePathComponent(candidate.to_owned())),
    }
}

const fn path_error(
    kind: WorkspacePathErrorKind,
    component_index: Option<usize>,
) -> WorkspacePathError {
    WorkspacePathError {
        kind,
        component_index,
    }
}

#[cfg(test)]
mod tests {
    use super::{WorkspacePath, WorkspacePathErrorKind};
    use crate::WorkspaceId;

    #[test]
    fn path_contains_only_workspace_identity_and_normalized_components() {
        let path = WorkspacePath::new(
            WorkspaceId::from_raw("workspace-0001"),
            ["src", "caf\u{e9}.rs"],
        )
        .expect("canonical path");
        assert_eq!(path.workspace_id().as_str(), "workspace-0001");
        assert_eq!(
            path.components()
                .iter()
                .map(|component| component.as_str())
                .collect::<Vec<_>>(),
            ["src", "caf\u{e9}.rs"]
        );
    }

    #[test]
    fn construction_rejects_non_relative_or_noncanonical_components() {
        let workspace = WorkspaceId::from_raw("workspace-0001");
        for (candidate, expected) in [
            ("", WorkspacePathErrorKind::EmptyComponent),
            (".", WorkspacePathErrorKind::CurrentDirectoryComponent),
            ("..", WorkspacePathErrorKind::ParentTraversalComponent),
            ("nested/file", WorkspacePathErrorKind::SeparatorInComponent),
            ("nested\\file", WorkspacePathErrorKind::SeparatorInComponent),
            ("bad\0value", WorkspacePathErrorKind::NulInComponent),
            (
                "cafe\u{301}.md",
                WorkspacePathErrorKind::NonCanonicalUnicode,
            ),
        ] {
            let error = WorkspacePath::new(workspace.clone(), [candidate])
                .expect_err("invalid component must fail");
            assert_eq!(error.kind(), expected);
            assert_eq!(error.component_index(), Some(0));
            if candidate.len() > 2 {
                assert!(!error.to_string().contains(candidate));
            }
        }
    }

    #[test]
    fn deserialization_revalidates_private_path_fields() {
        let valid = br#"{"workspace_id":"workspace-0001","components":["src","lib.rs"]}"#;
        let path: WorkspacePath = serde_json::from_slice(valid).expect("valid path");
        assert_eq!(path.components()[1].as_str(), "lib.rs");

        for invalid in [
            br#"{"workspace_id":"workspace-0001","components":[]}"#.as_slice(),
            br#"{"workspace_id":"workspace-0001","components":[".."]}"#.as_slice(),
            br#"{"workspace_id":"workspace-0001","components":["src"],"root":"/tmp"}"#.as_slice(),
        ] {
            assert!(serde_json::from_slice::<WorkspacePath>(invalid).is_err());
        }
    }
}
