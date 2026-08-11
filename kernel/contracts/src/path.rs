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
    /// One component carries an absolute-root prefix.
    RootedComponent,
    /// One component carries a drive or alternate-stream prefix.
    ColonInComponent,
    /// One component contains a path separator.
    SeparatorInComponent,
    /// One component contains encoded path-control syntax.
    EncodedPathSyntax,
    /// One component contains a NUL scalar.
    NulInComponent,
    /// One component contains a control scalar.
    ControlInComponent,
    /// One component exceeds the bounded UTF-8 size.
    OversizedComponent,
    /// One component has a platform-ambiguous trailing dot or space.
    AmbiguousSuffix,
    /// One component contains invisible path-direction formatting.
    InvisibleFormat,
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
            Self::RootedComponent => "path.component.rooted",
            Self::ColonInComponent => "path.component.colon",
            Self::SeparatorInComponent => "path.component.separator",
            Self::EncodedPathSyntax => "path.component.encoded_syntax",
            Self::NulInComponent => "path.component.nul",
            Self::ControlInComponent => "path.component.control",
            Self::OversizedComponent => "path.component.size_exceeded",
            Self::AmbiguousSuffix => "path.component.ambiguous_suffix",
            Self::InvisibleFormat => "path.component.invisible_format",
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
    } else if candidate.starts_with(['/', '\\']) {
        Some(WorkspacePathErrorKind::RootedComponent)
    } else if candidate.contains(':') {
        Some(WorkspacePathErrorKind::ColonInComponent)
    } else if candidate.chars().any(is_path_separator) {
        Some(WorkspacePathErrorKind::SeparatorInComponent)
    } else if contains_encoded_path_syntax(candidate) {
        Some(WorkspacePathErrorKind::EncodedPathSyntax)
    } else if candidate.contains('\0') {
        Some(WorkspacePathErrorKind::NulInComponent)
    } else if candidate.chars().any(char::is_control) {
        Some(WorkspacePathErrorKind::ControlInComponent)
    } else if candidate.len() > MAX_WORKSPACE_PATH_COMPONENT_BYTES {
        Some(WorkspacePathErrorKind::OversizedComponent)
    } else if candidate.ends_with(['.', ' ']) {
        Some(WorkspacePathErrorKind::AmbiguousSuffix)
    } else if candidate.chars().any(is_invisible_format) {
        Some(WorkspacePathErrorKind::InvisibleFormat)
    } else if candidate.nfc().ne(candidate.chars()) || candidate.nfkc().ne(candidate.chars()) {
        Some(WorkspacePathErrorKind::NonCanonicalUnicode)
    } else {
        None
    };
    match kind {
        Some(kind) => Err(path_error(kind, Some(index))),
        None => Ok(WorkspacePathComponent(candidate.to_owned())),
    }
}

fn is_path_separator(character: char) -> bool {
    matches!(
        character,
        '/' | '\\' | '\u{2044}' | '\u{2215}' | '\u{29f8}' | '\u{ff0f}' | '\u{ff3c}'
    )
}

fn is_invisible_format(character: char) -> bool {
    matches!(
        character,
        '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{2069}' | '\u{feff}'
    )
}

fn contains_encoded_path_syntax(candidate: &str) -> bool {
    candidate.as_bytes().windows(3).any(|window| {
        window[0] == b'%'
            && matches!(
                (
                    window[1].to_ascii_lowercase(),
                    window[2].to_ascii_lowercase()
                ),
                (b'2', b'e') | (b'2', b'f') | (b'5', b'c')
            )
    })
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
            ("/rooted", WorkspacePathErrorKind::RootedComponent),
            ("\\rooted", WorkspacePathErrorKind::RootedComponent),
            ("C:", WorkspacePathErrorKind::ColonInComponent),
            ("nested/file", WorkspacePathErrorKind::SeparatorInComponent),
            ("nested\\file", WorkspacePathErrorKind::SeparatorInComponent),
            (
                "nested\u{ff0f}file",
                WorkspacePathErrorKind::SeparatorInComponent,
            ),
            ("%2e%2e", WorkspacePathErrorKind::EncodedPathSyntax),
            ("bad\0value", WorkspacePathErrorKind::NulInComponent),
            ("trailing.", WorkspacePathErrorKind::AmbiguousSuffix),
            ("trailing ", WorkspacePathErrorKind::AmbiguousSuffix),
            (
                "hidden\u{202e}name",
                WorkspacePathErrorKind::InvisibleFormat,
            ),
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
        assert!(
            serde_json::from_slice::<WorkspacePath>(
                b"{\"workspace_id\":\"workspace-0001\",\"components\":[\"\xff\"]}"
            )
            .is_err()
        );
    }

    #[test]
    fn path_and_component_bounds_fail_before_allocation_or_authority() {
        let workspace = WorkspaceId::from_raw("workspace-0001");
        let empty = WorkspacePath::new(workspace.clone(), Vec::<String>::new())
            .expect_err("empty path must fail");
        assert_eq!(empty.kind(), WorkspacePathErrorKind::EmptyPath);

        let oversized = "x".repeat(super::MAX_WORKSPACE_PATH_COMPONENT_BYTES + 1);
        let oversized_error = WorkspacePath::new(workspace.clone(), [oversized])
            .expect_err("oversized component must fail");
        assert_eq!(
            oversized_error.kind(),
            WorkspacePathErrorKind::OversizedComponent
        );

        let too_many = vec!["x"; super::MAX_WORKSPACE_PATH_COMPONENTS + 1];
        let count_error =
            WorkspacePath::new(workspace, too_many).expect_err("too many components must fail");
        assert_eq!(
            count_error.kind(),
            WorkspacePathErrorKind::TooManyComponents
        );

        let workspace_error = WorkspacePath::new(WorkspaceId::from_raw("/ambient"), ["x"])
            .expect_err("ambient workspace identity must fail");
        assert_eq!(
            workspace_error.kind(),
            WorkspacePathErrorKind::InvalidWorkspaceIdentity
        );
    }
}
