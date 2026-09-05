//! Read-only Linux mail interoperability without private-profile access.
use std::collections::BTreeSet;

/// Supported interoperability route.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailInteropRoute {
    /// Reuse a separately authorized provider adapter.
    Provider,
    /// Reuse standard IMAP/SMTP/JMAP authority.
    StandardProtocol,
    /// Import an explicitly selected mbox archive.
    Mbox,
    /// Import an explicitly selected Maildir tree.
    Maildir,
}

/// Originating Linux mail client, used only for provenance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxMailClient {
    /// Mozilla Thunderbird.
    Thunderbird,
    /// GNOME Evolution.
    Evolution,
    /// KDE KMail.
    Kmail,
    /// No client-specific source.
    None,
}

/// Immutable source snapshot established before parsing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailArchiveSource {
    /// User-selected source identity.
    pub source_id: String,
    /// Exact canonical path.
    pub canonical_path: String,
    /// Route.
    pub route: MailInteropRoute,
    /// Client provenance only.
    pub client: LinuxMailClient,
    /// Client version, if applicable.
    pub client_version: Option<String>,
    /// Source byte hash.
    pub content_sha256: String,
    /// Snapshot filesystem identity.
    pub filesystem_id: u64,
    /// Snapshot byte length.
    pub byte_length: u64,
    /// Explicit user selection.
    pub user_selected: bool,
    /// Read-only handle established.
    pub read_only: bool,
    /// Source is a private profile database or state path.
    pub private_profile: bool,
    /// Source contains or exposes credential material.
    pub credential_bearing: bool,
    /// Source or an ancestor is linked.
    pub linked: bool,
    /// Source is locked or concurrently mutable.
    pub locked_or_mutable: bool,
}

/// Normalized message with source provenance preserved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedMessage {
    /// Stable message identity.
    pub message_id: String,
    /// Header digest.
    pub headers_sha256: String,
    /// MIME tree digest.
    pub mime_sha256: String,
    /// Attachment digests.
    pub attachment_sha256: Vec<String>,
    /// Folder context.
    pub folder: String,
    /// Source identity.
    pub source_id: String,
    /// Byte range in the immutable snapshot.
    pub byte_range: (u64, u64),
    /// Visible parse failure, if any.
    pub parse_failure: Option<String>,
}

/// Stable fail-closed refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailInteropError {
    /// Source identity or digest is malformed.
    InvalidSource,
    /// Input was not explicitly selected and opened read-only.
    NotAuthorized,
    /// Private client state or credentials were requested.
    PrivateProfile,
    /// Link, lock, mutation, replacement, or race was observed.
    UnsafeSource,
    /// Source exceeds the bounded parser limit.
    TooLarge,
    /// Message provenance is invalid.
    InvalidMessage,
    /// Snapshot changed after admission.
    Replaced,
    /// Import authority was removed.
    Removed,
}

/// One immutable archive import boundary.
pub struct MailImportController {
    source: MailArchiveSource,
    derived_ids: BTreeSet<String>,
    removed: bool,
}

fn id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 4096
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

impl MailImportController {
    /// Admits only an explicit immutable archive, never private client state.
    pub fn admit(source: MailArchiveSource, max_bytes: u64) -> Result<Self, MailInteropError> {
        if !id(&source.source_id) || !id(&source.canonical_path) || !digest(&source.content_sha256)
        {
            return Err(MailInteropError::InvalidSource);
        }
        if !source.user_selected || !source.read_only {
            return Err(MailInteropError::NotAuthorized);
        }
        if source.private_profile || source.credential_bearing {
            return Err(MailInteropError::PrivateProfile);
        }
        if source.linked || source.locked_or_mutable {
            return Err(MailInteropError::UnsafeSource);
        }
        if source.byte_length > max_bytes {
            return Err(MailInteropError::TooLarge);
        }
        Ok(Self {
            source,
            derived_ids: BTreeSet::new(),
            removed: false,
        })
    }

    /// Adds a normalized message only against the unchanged snapshot.
    pub fn record(
        &mut self,
        message: &ImportedMessage,
        current_filesystem_id: u64,
        current_sha256: &str,
    ) -> Result<(), MailInteropError> {
        if self.removed {
            return Err(MailInteropError::Removed);
        }
        if current_filesystem_id != self.source.filesystem_id
            || current_sha256 != self.source.content_sha256
        {
            return Err(MailInteropError::Replaced);
        }
        if message.source_id != self.source.source_id
            || !id(&message.message_id)
            || !id(&message.folder)
            || !digest(&message.headers_sha256)
            || !digest(&message.mime_sha256)
            || message.attachment_sha256.iter().any(|v| !digest(v))
            || message.byte_range.0 >= message.byte_range.1
            || message.byte_range.1 > self.source.byte_length
        {
            return Err(MailInteropError::InvalidMessage);
        }
        self.derived_ids.insert(message.message_id.clone());
        Ok(())
    }

    /// Removes only derived indexes and workers; original source authority is untouched.
    pub fn remove(&mut self) {
        self.derived_ids.clear();
        self.removed = true;
    }

    /// Original snapshot identity for non-mutation verification.
    pub fn source_fingerprint(&self) -> (u64, &str) {
        (self.source.filesystem_id, &self.source.content_sha256)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> MailArchiveSource {
        MailArchiveSource {
            source_id: "archive".into(),
            canonical_path: "/selected/mail.mbox".into(),
            route: MailInteropRoute::Mbox,
            client: LinuxMailClient::Thunderbird,
            client_version: Some("stable".into()),
            content_sha256: "a".repeat(64),
            filesystem_id: 7,
            byte_length: 100,
            user_selected: true,
            read_only: true,
            private_profile: false,
            credential_bearing: false,
            linked: false,
            locked_or_mutable: false,
        }
    }
    fn message() -> ImportedMessage {
        ImportedMessage {
            message_id: "message".into(),
            headers_sha256: "b".repeat(64),
            mime_sha256: "c".repeat(64),
            attachment_sha256: vec!["d".repeat(64)],
            folder: "Inbox".into(),
            source_id: "archive".into(),
            byte_range: (0, 50),
            parse_failure: None,
        }
    }
    #[test]
    fn private_linked_and_credential_sources_deny() {
        for mutate in 0..3 {
            let mut s = source();
            if mutate == 0 {
                s.private_profile = true;
            }
            if mutate == 1 {
                s.linked = true;
            }
            if mutate == 2 {
                s.credential_bearing = true;
            }
            assert!(MailImportController::admit(s, 100).is_err());
        }
    }
    #[test]
    fn replacement_denies_and_provenance_is_exact() {
        let mut c = MailImportController::admit(source(), 100).unwrap();
        assert_eq!(
            c.record(&message(), 8, &"a".repeat(64)),
            Err(MailInteropError::Replaced)
        );
        assert_eq!(c.record(&message(), 7, &"a".repeat(64)), Ok(()));
    }
    #[test]
    fn removal_changes_no_source_fingerprint() {
        let mut c = MailImportController::admit(source(), 100).unwrap();
        let before = (
            c.source_fingerprint().0,
            c.source_fingerprint().1.to_owned(),
        );
        c.remove();
        assert_eq!(
            (
                c.source_fingerprint().0,
                c.source_fingerprint().1.to_owned()
            ),
            before
        );
        assert_eq!(
            c.record(&message(), 7, &"a".repeat(64)),
            Err(MailInteropError::Removed)
        );
    }
}
