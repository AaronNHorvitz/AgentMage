//! Metadata-only attachment classification and exact workspace path resolution.

use agentmage_kernel_contracts::{
    AuthorizedWorkspaceHandle, FilePreimage, HeldWorkspaceObject, PathAdapterErrorKind,
    PathResolutionIntent, PlatformPathAdapter, WorkspaceAuthorizationId, WorkspaceObjectIdentity,
    WorkspaceObjectKind, WorkspacePath,
};

use crate::session_environment::{AttachedFileProvenance, SessionEnvironmentCapture};

const MAX_ATTACHMENT_BYTES: u64 = 1_099_511_627_776;

/// Closed attachment format classification supplied by the trusted shell boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttachmentFormat {
    /// Unstructured UTF-8 text candidate.
    PlainText,
    /// Markdown text candidate.
    Markdown,
    /// JSON text candidate.
    Json,
    /// Portable Document Format candidate.
    Pdf,
    /// Word-processing document candidate.
    Word,
    /// Spreadsheet candidate.
    Spreadsheet,
    /// Presentation candidate.
    Presentation,
    /// Raster or vector image candidate.
    Image,
    /// Archive candidate.
    Archive,
    /// Notebook candidate.
    Notebook,
    /// Plain-text log candidate.
    Log,
    /// Unrecognized format.
    Unknown,
}

impl AttachmentFormat {
    /// Complete stable format order.
    pub const ALL: [Self; 12] = [
        Self::PlainText,
        Self::Markdown,
        Self::Json,
        Self::Pdf,
        Self::Word,
        Self::Spreadsheet,
        Self::Presentation,
        Self::Image,
        Self::Archive,
        Self::Notebook,
        Self::Log,
        Self::Unknown,
    ];
}

/// Explicit parser availability for one attachment format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttachmentParserDisposition {
    /// A future bounded raw-text read may validate UTF-8 without structural parsing.
    RawTextEligible,
    /// A structured parser is assigned to a later release pack and is unavailable now.
    DeferredToReleasePack,
    /// No parser is declared for this format.
    Unsupported,
}

/// Bounded metadata proposed for one captured attached file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttachmentMetadata {
    /// Session-local attachment identity.
    pub attachment_id: String,
    /// Canonical workspace path selected for independent resolution.
    pub workspace_path: WorkspacePath,
    /// Closed format classification.
    pub format: AttachmentFormat,
    /// Expected exact byte length.
    pub byte_len: u64,
    /// Expected lowercase content digest.
    pub content_sha256: String,
}

/// Stable reason attachment metadata or path resolution failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttachmentResolutionErrorKind {
    /// Attachment metadata is malformed or over its resource bound.
    InvalidMetadata,
    /// The attachment does not occur in the exact session capture.
    MissingProvenance,
    /// Metadata does not match the captured attachment provenance.
    ProvenanceMismatch,
    /// The attachment path names a foreign or inactive workspace.
    WorkspaceMismatch,
    /// The workspace handle and selected adapter do not match.
    AdapterMismatch,
    /// The platform adapter refused exact resolution.
    AdapterRefusal,
    /// The held object differs from the requested path, identity, intent, or preimage.
    ResolvedObjectMismatch,
}

impl AttachmentResolutionErrorKind {
    /// Returns the stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidMetadata => "attachment.metadata.invalid",
            Self::MissingProvenance => "attachment.provenance.missing",
            Self::ProvenanceMismatch => "attachment.provenance.mismatch",
            Self::WorkspaceMismatch => "attachment.workspace.mismatch",
            Self::AdapterMismatch => "attachment.adapter.mismatch",
            Self::AdapterRefusal => "attachment.adapter.refused",
            Self::ResolvedObjectMismatch => "attachment.resolved_object.mismatch",
        }
    }
}

/// Content-free failure from attachment resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttachmentResolutionError {
    kind: AttachmentResolutionErrorKind,
    path_error_kind: Option<PathAdapterErrorKind>,
}

impl AttachmentResolutionError {
    const fn new(
        kind: AttachmentResolutionErrorKind,
        path_error_kind: Option<PathAdapterErrorKind>,
    ) -> Self {
        Self {
            kind,
            path_error_kind,
        }
    }

    /// Returns the stable attachment failure class.
    #[must_use]
    pub const fn kind(&self) -> AttachmentResolutionErrorKind {
        self.kind
    }

    /// Returns the nested stable adapter class without native error text.
    #[must_use]
    pub const fn path_error_kind(&self) -> Option<PathAdapterErrorKind> {
        self.path_error_kind
    }
}

/// Content-free, independently verified attachment path and preimage record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedAttachment {
    attachment_id: String,
    session_capture_sha256: String,
    workspace_path: WorkspacePath,
    authorization_id: WorkspaceAuthorizationId,
    object_identity: WorkspaceObjectIdentity,
    preimage: FilePreimage,
    format: AttachmentFormat,
    parser_disposition: AttachmentParserDisposition,
}

impl ResolvedAttachment {
    /// Returns the exact captured attachment identity.
    #[must_use]
    pub fn attachment_id(&self) -> &str {
        &self.attachment_id
    }

    /// Returns the session snapshot against which provenance was verified.
    #[must_use]
    pub fn session_capture_sha256(&self) -> &str {
        &self.session_capture_sha256
    }

    /// Returns the canonical path independently resolved by the adapter.
    #[must_use]
    pub const fn workspace_path(&self) -> &WorkspacePath {
        &self.workspace_path
    }

    /// Returns the exact workspace authorization used for resolution.
    #[must_use]
    pub const fn authorization_id(&self) -> &WorkspaceAuthorizationId {
        &self.authorization_id
    }

    /// Returns content-free platform object identity evidence.
    #[must_use]
    pub const fn object_identity(&self) -> &WorkspaceObjectIdentity {
        &self.object_identity
    }

    /// Returns exact size and digest evidence, never file bytes.
    #[must_use]
    pub const fn preimage(&self) -> &FilePreimage {
        &self.preimage
    }

    /// Returns the trusted-shell format classification.
    #[must_use]
    pub const fn format(&self) -> AttachmentFormat {
        self.format
    }

    /// Returns the explicit no-parser, deferred, or raw-text-only disposition.
    #[must_use]
    pub const fn parser_disposition(&self) -> AttachmentParserDisposition {
        self.parser_disposition
    }
}

/// Returns parser availability without invoking a parser or reading attachment content.
#[must_use]
pub const fn parser_disposition(format: AttachmentFormat) -> AttachmentParserDisposition {
    match format {
        AttachmentFormat::PlainText
        | AttachmentFormat::Markdown
        | AttachmentFormat::Json
        | AttachmentFormat::Log => AttachmentParserDisposition::RawTextEligible,
        AttachmentFormat::Pdf
        | AttachmentFormat::Word
        | AttachmentFormat::Spreadsheet
        | AttachmentFormat::Presentation
        | AttachmentFormat::Image
        | AttachmentFormat::Archive
        | AttachmentFormat::Notebook => AttachmentParserDisposition::DeferredToReleasePack,
        AttachmentFormat::Unknown => AttachmentParserDisposition::Unsupported,
    }
}

/// Independently resolves and hashes one captured attachment without retaining content.
pub fn resolve_attachment<A: PlatformPathAdapter>(
    metadata: AttachmentMetadata,
    session: &SessionEnvironmentCapture,
    adapter: &A,
    workspace: &A::WorkspaceHandle,
) -> Result<ResolvedAttachment, AttachmentResolutionError> {
    validate_metadata(&metadata)?;
    let provenance = provenance(session, &metadata.attachment_id)?;
    if metadata.byte_len != provenance.byte_len
        || metadata.content_sha256 != provenance.content_sha256
    {
        return Err(AttachmentResolutionError::new(
            AttachmentResolutionErrorKind::ProvenanceMismatch,
            None,
        ));
    }
    if metadata.workspace_path.workspace_id() != &session.input().active_workspace_id
        || workspace.workspace_id() != &session.input().active_workspace_id
    {
        return Err(AttachmentResolutionError::new(
            AttachmentResolutionErrorKind::WorkspaceMismatch,
            None,
        ));
    }
    if workspace.adapter_instance_id() != adapter.adapter_instance_id()
        || workspace.platform() != adapter.platform()
    {
        return Err(AttachmentResolutionError::new(
            AttachmentResolutionErrorKind::AdapterMismatch,
            None,
        ));
    }
    let held = adapter
        .resolve(
            workspace,
            &metadata.workspace_path,
            PathResolutionIntent::ContentHash,
        )
        .map_err(|error| {
            AttachmentResolutionError::new(
                AttachmentResolutionErrorKind::AdapterRefusal,
                Some(error.kind()),
            )
        })?;
    let Some(preimage) = held.preimage() else {
        return Err(AttachmentResolutionError::new(
            AttachmentResolutionErrorKind::ResolvedObjectMismatch,
            None,
        ));
    };
    if held.workspace_path() != &metadata.workspace_path
        || held.authorization_id() != workspace.authorization_id()
        || held.adapter_instance_id() != adapter.adapter_instance_id()
        || held.intent() != PathResolutionIntent::ContentHash
        || held.object_kind() != WorkspaceObjectKind::RegularFile
        || held.object_identity().platform() != adapter.platform()
        || preimage.byte_len() != metadata.byte_len
        || hex(preimage.content_sha256()) != metadata.content_sha256
    {
        return Err(AttachmentResolutionError::new(
            AttachmentResolutionErrorKind::ResolvedObjectMismatch,
            None,
        ));
    }
    Ok(ResolvedAttachment {
        attachment_id: metadata.attachment_id,
        session_capture_sha256: session.sha256().to_owned(),
        workspace_path: metadata.workspace_path,
        authorization_id: held.authorization_id().clone(),
        object_identity: held.object_identity().clone(),
        preimage: preimage.clone(),
        format: metadata.format,
        parser_disposition: parser_disposition(metadata.format),
    })
}

fn provenance<'a>(
    session: &'a SessionEnvironmentCapture,
    attachment_id: &str,
) -> Result<&'a AttachedFileProvenance, AttachmentResolutionError> {
    session
        .input()
        .attachments
        .iter()
        .find(|candidate| candidate.attachment_id == attachment_id)
        .ok_or_else(|| {
            AttachmentResolutionError::new(AttachmentResolutionErrorKind::MissingProvenance, None)
        })
}

fn validate_metadata(metadata: &AttachmentMetadata) -> Result<(), AttachmentResolutionError> {
    if !identifier(&metadata.attachment_id)
        || metadata.byte_len > MAX_ATTACHMENT_BYTES
        || !sha256_text(&metadata.content_sha256)
    {
        return Err(AttachmentResolutionError::new(
            AttachmentResolutionErrorKind::InvalidMetadata,
            None,
        ));
    }
    Ok(())
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn sha256_text(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use agentmage_kernel_contracts::{
        AdapterInstanceId, PathAdapterError, PathPlatform, PlatformArchitecture, PlatformFamily,
        PlatformRuntimeIdentity, SessionId, WorkspaceId, WorkspaceScopePath,
    };

    use super::*;
    use crate::authority::{
        DescriptiveArtifactKind, NonAuthoritativeArtifact, reject_as_authority,
    };
    use crate::configuration::ConfigurationManager;
    use crate::session_environment::{
        RepositoryHead, RepositoryObservation, SessionEnvironmentInput, capture_session_environment,
    };

    #[derive(Debug)]
    struct FakeHandle {
        workspace_id: WorkspaceId,
        authorization_id: WorkspaceAuthorizationId,
        adapter_instance_id: AdapterInstanceId,
        platform: PathPlatform,
    }

    impl AuthorizedWorkspaceHandle for FakeHandle {
        fn workspace_id(&self) -> &WorkspaceId {
            &self.workspace_id
        }

        fn authorization_id(&self) -> &WorkspaceAuthorizationId {
            &self.authorization_id
        }

        fn adapter_instance_id(&self) -> &AdapterInstanceId {
            &self.adapter_instance_id
        }

        fn platform(&self) -> PathPlatform {
            self.platform
        }
    }

    #[derive(Debug)]
    struct FakeHeldObject {
        path: WorkspacePath,
        authorization_id: WorkspaceAuthorizationId,
        adapter_instance_id: AdapterInstanceId,
        intent: PathResolutionIntent,
        object_kind: WorkspaceObjectKind,
        object_identity: WorkspaceObjectIdentity,
        preimage: Option<FilePreimage>,
    }

    impl HeldWorkspaceObject for FakeHeldObject {
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
            self.intent
        }

        fn object_kind(&self) -> WorkspaceObjectKind {
            self.object_kind
        }

        fn object_identity(&self) -> &WorkspaceObjectIdentity {
            &self.object_identity
        }

        fn preimage(&self) -> Option<&FilePreimage> {
            self.preimage.as_ref()
        }
    }

    #[derive(Debug)]
    struct FakeAdapter {
        adapter_instance_id: AdapterInstanceId,
        resolve_count: AtomicUsize,
        content_sha256: [u8; 32],
        byte_len: u64,
        omit_preimage: bool,
        refuse: bool,
    }

    impl PlatformPathAdapter for FakeAdapter {
        type WorkspaceHandle = FakeHandle;
        type HeldObject = FakeHeldObject;

        fn adapter_instance_id(&self) -> &AdapterInstanceId {
            &self.adapter_instance_id
        }

        fn platform(&self) -> PathPlatform {
            PathPlatform::DeterministicFake
        }

        fn resolve(
            &self,
            workspace: &Self::WorkspaceHandle,
            path: &WorkspacePath,
            intent: PathResolutionIntent,
        ) -> Result<Self::HeldObject, PathAdapterError> {
            self.resolve_count.fetch_add(1, Ordering::SeqCst);
            if self.refuse {
                return Err(PathAdapterError::new(PathAdapterErrorKind::NotFound, None));
            }
            Ok(FakeHeldObject {
                path: path.clone(),
                authorization_id: workspace.authorization_id.clone(),
                adapter_instance_id: self.adapter_instance_id.clone(),
                intent,
                object_kind: WorkspaceObjectKind::RegularFile,
                object_identity: WorkspaceObjectIdentity::new(
                    PathPlatform::DeterministicFake,
                    [8; 32],
                    [9; 32],
                ),
                preimage: (!self.omit_preimage)
                    .then(|| FilePreimage::new(self.byte_len, self.content_sha256)),
            })
        }
    }

    fn scope(components: &[&str]) -> WorkspaceScopePath {
        WorkspaceScopePath::new(
            WorkspaceId::from_raw("workspace-0001"),
            components.iter().copied(),
        )
        .expect("scope is canonical")
    }

    fn path() -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-0001"),
            ["attachments", "brief.md"],
        )
        .expect("path is canonical")
    }

    fn session() -> SessionEnvironmentCapture {
        let configuration = ConfigurationManager::default()
            .safe_defaults()
            .expect("safe defaults load");
        capture_session_environment(
            SessionEnvironmentInput {
                session_id: SessionId::from_raw("session-0001"),
                captured_at_utc: "2026-08-13T20:00:00Z".to_owned(),
                local_date: "2026-08-13".to_owned(),
                timezone_id: "America/Chicago".to_owned(),
                current_directory: scope(&["attachments"]),
                workspace_roots: vec![scope(&[])],
                active_workspace_id: WorkspaceId::from_raw("workspace-0001"),
                repository: Some(RepositoryObservation {
                    workspace_id: WorkspaceId::from_raw("workspace-0001"),
                    root: scope(&[]),
                    repository_sha256: "b".repeat(64),
                    head_commit: "c".repeat(40),
                    head: RepositoryHead::Branch("agent/attachments".to_owned()),
                }),
                attachments: vec![AttachedFileProvenance {
                    attachment_id: "attachment-0001".to_owned(),
                    source_id: "vscode-chat-attachment".to_owned(),
                    object_id: "object-0001".to_owned(),
                    byte_len: 42,
                    content_sha256: "a".repeat(64),
                    observed_revision: "capture-0001".to_owned(),
                }],
            },
            &configuration,
            &PlatformRuntimeIdentity::new(
                PlatformFamily::DeterministicFake,
                PlatformArchitecture::X86_64,
                [1; 32],
                [2; 32],
                [3; 32],
                [4; 32],
            ),
        )
        .expect("session captures")
    }

    fn metadata(format: AttachmentFormat) -> AttachmentMetadata {
        AttachmentMetadata {
            attachment_id: "attachment-0001".to_owned(),
            workspace_path: path(),
            format,
            byte_len: 42,
            content_sha256: "a".repeat(64),
        }
    }

    fn handle() -> FakeHandle {
        FakeHandle {
            workspace_id: WorkspaceId::from_raw("workspace-0001"),
            authorization_id: WorkspaceAuthorizationId::from_raw("authorization-0001"),
            adapter_instance_id: AdapterInstanceId::from_raw("adapter-0001"),
            platform: PathPlatform::DeterministicFake,
        }
    }

    fn adapter() -> FakeAdapter {
        FakeAdapter {
            adapter_instance_id: AdapterInstanceId::from_raw("adapter-0001"),
            resolve_count: AtomicUsize::new(0),
            content_sha256: [0xaa; 32],
            byte_len: 42,
            omit_preimage: false,
            refuse: false,
        }
    }

    #[test]
    fn exact_captured_attachment_resolves_and_retains_no_content() {
        let adapter = adapter();
        let resolved = resolve_attachment(
            metadata(AttachmentFormat::Markdown),
            &session(),
            &adapter,
            &handle(),
        )
        .expect("exact attachment resolves");
        assert_eq!(adapter.resolve_count.load(Ordering::SeqCst), 1);
        assert_eq!(resolved.attachment_id(), "attachment-0001");
        assert_eq!(resolved.preimage().byte_len(), 42);
        assert_eq!(resolved.preimage().content_sha256(), &[0xaa; 32]);
        assert_eq!(
            resolved.parser_disposition(),
            AttachmentParserDisposition::RawTextEligible
        );
    }

    #[test]
    fn every_format_has_one_explicit_nonexecuting_parser_disposition() {
        let dispositions: Vec<_> = AttachmentFormat::ALL
            .into_iter()
            .map(parser_disposition)
            .collect();
        assert_eq!(
            dispositions
                .iter()
                .filter(|value| **value == AttachmentParserDisposition::RawTextEligible)
                .count(),
            4
        );
        assert_eq!(
            dispositions
                .iter()
                .filter(|value| **value == AttachmentParserDisposition::DeferredToReleasePack)
                .count(),
            7
        );
        assert_eq!(
            dispositions
                .iter()
                .filter(|value| **value == AttachmentParserDisposition::Unsupported)
                .count(),
            1
        );
    }

    #[test]
    fn malformed_missing_and_changed_provenance_fail_before_resolution() {
        for case in ["malformed", "missing", "changed"] {
            let adapter = adapter();
            let mut candidate = metadata(AttachmentFormat::PlainText);
            match case {
                "malformed" => candidate.content_sha256 = "bad".to_owned(),
                "missing" => candidate.attachment_id = "attachment-absent".to_owned(),
                "changed" => candidate.byte_len = 41,
                _ => unreachable!(),
            }
            assert!(resolve_attachment(candidate, &session(), &adapter, &handle()).is_err());
            assert_eq!(adapter.resolve_count.load(Ordering::SeqCst), 0);
        }
    }

    #[test]
    fn foreign_workspace_handle_and_adapter_fail_before_resolution() {
        let mut foreign_handle = handle();
        foreign_handle.workspace_id = WorkspaceId::from_raw("workspace-0002");
        let local_adapter = adapter();
        assert_eq!(
            resolve_attachment(
                metadata(AttachmentFormat::PlainText),
                &session(),
                &local_adapter,
                &foreign_handle,
            )
            .expect_err("foreign workspace fails")
            .kind(),
            AttachmentResolutionErrorKind::WorkspaceMismatch
        );
        assert_eq!(local_adapter.resolve_count.load(Ordering::SeqCst), 0);

        let mut foreign_adapter = adapter();
        foreign_adapter.adapter_instance_id = AdapterInstanceId::from_raw("adapter-0002");
        assert_eq!(
            resolve_attachment(
                metadata(AttachmentFormat::PlainText),
                &session(),
                &foreign_adapter,
                &handle(),
            )
            .expect_err("foreign adapter fails")
            .kind(),
            AttachmentResolutionErrorKind::AdapterMismatch
        );
        assert_eq!(foreign_adapter.resolve_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn adapter_refusal_and_missing_or_changed_preimage_fail_closed() {
        let mut refusing = adapter();
        refusing.refuse = true;
        let error = resolve_attachment(
            metadata(AttachmentFormat::Pdf),
            &session(),
            &refusing,
            &handle(),
        )
        .expect_err("adapter refusal propagates");
        assert_eq!(error.kind(), AttachmentResolutionErrorKind::AdapterRefusal);
        assert_eq!(
            error.path_error_kind(),
            Some(PathAdapterErrorKind::NotFound)
        );

        let mut missing = adapter();
        missing.omit_preimage = true;
        assert_eq!(
            resolve_attachment(
                metadata(AttachmentFormat::Pdf),
                &session(),
                &missing,
                &handle(),
            )
            .expect_err("missing preimage fails")
            .kind(),
            AttachmentResolutionErrorKind::ResolvedObjectMismatch
        );

        let mut changed = adapter();
        changed.byte_len = 41;
        assert_eq!(
            resolve_attachment(
                metadata(AttachmentFormat::Pdf),
                &session(),
                &changed,
                &handle(),
            )
            .expect_err("changed preimage fails")
            .kind(),
            AttachmentResolutionErrorKind::ResolvedObjectMismatch
        );
    }

    #[test]
    fn metadata_and_resolution_records_cannot_become_authority() {
        assert_eq!(
            <AttachmentMetadata as NonAuthoritativeArtifact>::KIND,
            DescriptiveArtifactKind::SessionRecord
        );
        assert_eq!(
            <ResolvedAttachment as NonAuthoritativeArtifact>::KIND,
            DescriptiveArtifactKind::SessionRecord
        );
        let denial = reject_as_authority(&metadata(AttachmentFormat::Markdown));
        assert_eq!(denial.artifact_kind, DescriptiveArtifactKind::SessionRecord);
    }
}
