//! Non-authoritative display links derived from validated held objects.

use std::fmt;

use crate::WorkspaceObjectIdentity;

/// Maximum encoded URI length admitted for one display-only file link.
pub const MAX_DISPLAY_FILE_URI_BYTES: usize = 8 * 1024;

/// Stable, content-free display-link rejection class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayLinkErrorKind {
    /// The candidate is not an absolute local `file:///` URI.
    InvalidFileUri,
    /// The URI contains malformed percent encoding or non-ASCII wire bytes.
    InvalidEncoding,
    /// The URI contains query, fragment, NUL, or control content.
    UnsafeContent,
    /// The URI exceeds the bounded display size.
    OversizedUri,
    /// A supplied line number is zero.
    InvalidLine,
}

impl DisplayLinkErrorKind {
    /// Returns the stable redacted code for this failure class.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidFileUri => "display_link.file_uri.invalid",
            Self::InvalidEncoding => "display_link.encoding.invalid",
            Self::UnsafeContent => "display_link.content.unsafe",
            Self::OversizedUri => "display_link.uri.size_exceeded",
            Self::InvalidLine => "display_link.line.invalid",
        }
    }
}

/// Content-free error returned while creating a display-only file link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayLinkError {
    kind: DisplayLinkErrorKind,
}

impl DisplayLinkError {
    /// Returns the stable rejection class.
    #[must_use]
    pub const fn kind(&self) -> DisplayLinkErrorKind {
        self.kind
    }
}

impl fmt::Display for DisplayLinkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.kind.code())
    }
}

impl std::error::Error for DisplayLinkError {}

/// Absolute local file link carrying display value but no path authority.
///
/// This type intentionally has no deserializer and no conversion to `WorkspacePath`
/// or `GrantTarget`. Callers must explicitly request the URI for rendering.
#[derive(Clone, PartialEq, Eq)]
pub struct DisplayFileLink {
    file_uri: String,
    line: Option<u32>,
    object_identity: WorkspaceObjectIdentity,
}

impl DisplayFileLink {
    /// Creates a display-only link from an adapter-generated absolute file URI.
    pub fn new(
        file_uri: impl Into<String>,
        line: Option<u32>,
        object_identity: WorkspaceObjectIdentity,
    ) -> Result<Self, DisplayLinkError> {
        let file_uri = file_uri.into();
        validate_file_uri(&file_uri)?;
        if line == Some(0) {
            return Err(display_error(DisplayLinkErrorKind::InvalidLine));
        }
        Ok(Self {
            file_uri,
            line,
            object_identity,
        })
    }

    /// Returns the absolute local URI solely for explicit display rendering.
    #[must_use]
    pub fn file_uri(&self) -> &str {
        &self.file_uri
    }

    /// Returns the optional one-based source line.
    #[must_use]
    pub const fn line(&self) -> Option<u32> {
        self.line
    }

    /// Returns the identity evidence for the held object that produced this link.
    #[must_use]
    pub const fn object_identity(&self) -> &WorkspaceObjectIdentity {
        &self.object_identity
    }

    /// Renders the clickable target, adding a VS Code-compatible line fragment when present.
    #[must_use]
    pub fn render_target(&self) -> String {
        match self.line {
            Some(line) => format!("{}#L{line}", self.file_uri),
            None => self.file_uri.clone(),
        }
    }
}

impl fmt::Debug for DisplayFileLink {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DisplayFileLink")
            .field("uri", &"[redacted-display-uri]")
            .field("line", &self.line)
            .field("platform", &self.object_identity.platform())
            .finish()
    }
}

fn validate_file_uri(candidate: &str) -> Result<(), DisplayLinkError> {
    let kind = if candidate.len() > MAX_DISPLAY_FILE_URI_BYTES {
        Some(DisplayLinkErrorKind::OversizedUri)
    } else if !candidate.starts_with("file:///") || candidate.len() == "file:///".len() {
        Some(DisplayLinkErrorKind::InvalidFileUri)
    } else if !candidate.is_ascii() || !valid_percent_encoding(candidate.as_bytes()) {
        Some(DisplayLinkErrorKind::InvalidEncoding)
    } else if candidate
        .bytes()
        .any(|byte| byte == 0 || byte.is_ascii_control() || matches!(byte, b'?' | b'#'))
    {
        Some(DisplayLinkErrorKind::UnsafeContent)
    } else {
        None
    };
    match kind {
        Some(kind) => Err(display_error(kind)),
        None => Ok(()),
    }
}

fn valid_percent_encoding(candidate: &[u8]) -> bool {
    let mut index = 0;
    while index < candidate.len() {
        if candidate[index] == b'%' {
            if index + 2 >= candidate.len()
                || !candidate[index + 1].is_ascii_hexdigit()
                || !candidate[index + 2].is_ascii_hexdigit()
            {
                return false;
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    true
}

const fn display_error(kind: DisplayLinkErrorKind) -> DisplayLinkError {
    DisplayLinkError { kind }
}

#[cfg(test)]
mod tests {
    use super::{DisplayFileLink, DisplayLinkErrorKind};
    use crate::{PathPlatform, WorkspaceObjectIdentity};

    fn identity() -> WorkspaceObjectIdentity {
        WorkspaceObjectIdentity::new(PathPlatform::DeterministicFake, [1; 32], [2; 32])
    }

    #[test]
    fn display_link_retains_only_explicit_rendering_evidence() {
        let link =
            DisplayFileLink::new("file:///workspace/notes/My%20Note.md", Some(42), identity())
                .expect("valid display link");
        assert_eq!(link.file_uri(), "file:///workspace/notes/My%20Note.md");
        assert_eq!(link.line(), Some(42));
        assert_eq!(
            link.render_target(),
            "file:///workspace/notes/My%20Note.md#L42"
        );
        assert_eq!(link.object_identity().object_identity_sha256(), &[2; 32]);
        assert!(!format!("{link:?}").contains("workspace"));
    }

    #[test]
    fn malformed_or_authority_like_display_candidates_fail_closed() {
        for (candidate, line, expected) in [
            ("/ambient/path", None, DisplayLinkErrorKind::InvalidFileUri),
            (
                "https://example.test/x",
                None,
                DisplayLinkErrorKind::InvalidFileUri,
            ),
            ("file:///", None, DisplayLinkErrorKind::InvalidFileUri),
            ("file:///x%2", None, DisplayLinkErrorKind::InvalidEncoding),
            ("file:///x#L1", None, DisplayLinkErrorKind::UnsafeContent),
            ("file:///x?query", None, DisplayLinkErrorKind::UnsafeContent),
            ("file:///x", Some(0), DisplayLinkErrorKind::InvalidLine),
        ] {
            let error = DisplayFileLink::new(candidate, line, identity())
                .expect_err("invalid display link rejects");
            assert_eq!(error.kind(), expected);
            assert!(!error.to_string().contains(candidate));
        }
    }
}
