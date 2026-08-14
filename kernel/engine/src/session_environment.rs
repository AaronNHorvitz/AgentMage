//! Bounded, configuration-bound session environment and provenance capture.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{
    PlatformArchitecture, PlatformFamily, PlatformRuntimeIdentity, SessionId, WorkspaceId,
    WorkspaceScopePath,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::configuration::LoadedConfiguration;

const MAX_TIMEZONE_BYTES: usize = 64;
const MAX_REPOSITORY_BRANCH_BYTES: usize = 255;
const MAX_PROVENANCE_TEXT_BYTES: usize = 256;
const MAX_ATTACHMENTS: usize = 32;
const MAX_WORKSPACE_ROOTS: usize = 16;
const MAX_ATTACHMENT_BYTES: u64 = 1_099_511_627_776;

/// Closed reason a session environment candidate failed validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionEnvironmentErrorKind {
    /// A stable identifier is empty, oversized, or syntactically invalid.
    InvalidIdentifier,
    /// The UTC capture timestamp is not canonical second-resolution UTC.
    InvalidTimestamp,
    /// The local date is not a canonical calendar date.
    InvalidLocalDate,
    /// The timezone identity is empty, oversized, or malformed.
    InvalidTimezone,
    /// Workspace roots are empty, duplicated, non-root, or over the bound.
    InvalidWorkspaceRoots,
    /// The active workspace does not match the current directory.
    ActiveWorkspaceMismatch,
    /// Repository identity, root, head, or branch evidence is malformed.
    InvalidRepository,
    /// The repository is outside the active workspace declaration.
    RepositoryWorkspaceMismatch,
    /// Attached-file provenance is malformed, duplicated, or over the bound.
    InvalidAttachmentProvenance,
}

impl SessionEnvironmentErrorKind {
    /// Returns the stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidIdentifier => "session.environment.invalid_identifier",
            Self::InvalidTimestamp => "session.environment.invalid_timestamp",
            Self::InvalidLocalDate => "session.environment.invalid_local_date",
            Self::InvalidTimezone => "session.environment.invalid_timezone",
            Self::InvalidWorkspaceRoots => "session.environment.invalid_workspace_roots",
            Self::ActiveWorkspaceMismatch => "session.environment.active_workspace_mismatch",
            Self::InvalidRepository => "session.environment.invalid_repository",
            Self::RepositoryWorkspaceMismatch => {
                "session.environment.repository_workspace_mismatch"
            }
            Self::InvalidAttachmentProvenance => {
                "session.environment.invalid_attachment_provenance"
            }
        }
    }
}

/// Content-free session environment validation failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionEnvironmentError {
    kind: SessionEnvironmentErrorKind,
    item_index: Option<usize>,
}

impl SessionEnvironmentError {
    const fn new(kind: SessionEnvironmentErrorKind, item_index: Option<usize>) -> Self {
        Self { kind, item_index }
    }

    /// Returns the stable failure class.
    #[must_use]
    pub const fn kind(&self) -> SessionEnvironmentErrorKind {
        self.kind
    }

    /// Returns only the failing collection index, never candidate content.
    #[must_use]
    pub const fn item_index(&self) -> Option<usize> {
        self.item_index
    }
}

/// Exact repository-head state observed for the active repository.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum RepositoryHead {
    /// One validated local branch name.
    Branch(String),
    /// A detached head with no implied branch.
    Detached,
}

/// Content-free repository identity captured within one workspace.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryObservation {
    /// Workspace containing the repository.
    pub workspace_id: WorkspaceId,
    /// Canonical workspace-relative repository root.
    pub root: WorkspaceScopePath,
    /// Digest of stable repository identity material, not a remote URL.
    pub repository_sha256: String,
    /// Exact observed Git object identity.
    pub head_commit: String,
    /// Named branch or explicit detached-head state.
    pub head: RepositoryHead,
}

/// Metadata-only provenance for one file attached to the session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AttachedFileProvenance {
    /// Stable session-local attachment identity.
    pub attachment_id: String,
    /// Stable adapter or user-attachment source identity.
    pub source_id: String,
    /// Stable object identity within the source, never an ambient path capability.
    pub object_id: String,
    /// Exact observed byte length.
    pub byte_len: u64,
    /// Lowercase SHA-256 digest of the observed bytes.
    pub content_sha256: String,
    /// Exact source revision or capture generation.
    pub observed_revision: String,
}

/// Untrusted observations proposed for one session environment capture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionEnvironmentInput {
    /// Stable local session identity.
    pub session_id: SessionId,
    /// Canonical UTC capture instant at second resolution.
    pub captured_at_utc: String,
    /// Canonical local calendar date.
    pub local_date: String,
    /// IANA-style local timezone identity.
    pub timezone_id: String,
    /// Current directory expressed only as a canonical workspace scope.
    pub current_directory: WorkspaceScopePath,
    /// Exact authorized workspace roots.
    pub workspace_roots: Vec<WorkspaceScopePath>,
    /// Workspace selected for this session.
    pub active_workspace_id: WorkspaceId,
    /// Optional active repository observation.
    pub repository: Option<RepositoryObservation>,
    /// Metadata-only attached-file provenance records.
    pub attachments: Vec<AttachedFileProvenance>,
}

/// Exact platform identity bound into one session capture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionPlatformBinding {
    family: PlatformFamily,
    architecture: PlatformArchitecture,
    os_build_sha256: String,
    toolchain_sha256: String,
    vscode_build_sha256: String,
    package_sha256: String,
}

impl SessionPlatformBinding {
    /// Returns the selected platform family.
    #[must_use]
    pub const fn family(&self) -> PlatformFamily {
        self.family
    }

    /// Returns the selected processor architecture.
    #[must_use]
    pub const fn architecture(&self) -> PlatformArchitecture {
        self.architecture
    }

    /// Returns the normalized operating-system identity digest.
    #[must_use]
    pub fn os_build_sha256(&self) -> &str {
        &self.os_build_sha256
    }

    /// Returns the pinned toolchain identity digest.
    #[must_use]
    pub fn toolchain_sha256(&self) -> &str {
        &self.toolchain_sha256
    }

    /// Returns the supported Visual Studio Code identity digest.
    #[must_use]
    pub fn vscode_build_sha256(&self) -> &str {
        &self.vscode_build_sha256
    }

    /// Returns the installed AgentMage package identity digest.
    #[must_use]
    pub fn package_sha256(&self) -> &str {
        &self.package_sha256
    }
}

/// Configuration-bound permission profile captured for one session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionPermissionProfile {
    profile_id: String,
    configuration_sha256: String,
}

impl SessionPermissionProfile {
    /// Returns the effective configuration profile identity.
    #[must_use]
    pub fn profile_id(&self) -> &str {
        &self.profile_id
    }

    /// Returns the exact configuration identity owning the permission policy.
    #[must_use]
    pub fn configuration_sha256(&self) -> &str {
        &self.configuration_sha256
    }
}

/// Configuration-bound model profile captured for one session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionModelProfile {
    profile_id: String,
    configuration_sha256: String,
    enabled: bool,
}

impl SessionModelProfile {
    /// Returns the selected model profile identity.
    #[must_use]
    pub fn profile_id(&self) -> &str {
        &self.profile_id
    }

    /// Returns the exact configuration identity selecting the model profile.
    #[must_use]
    pub fn configuration_sha256(&self) -> &str {
        &self.configuration_sha256
    }

    /// Reports whether model execution is enabled by this configuration.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }
}

/// Validated, immutable, content-addressed session environment snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionEnvironmentCapture {
    input: SessionEnvironmentInput,
    platform: SessionPlatformBinding,
    permission_profile: SessionPermissionProfile,
    model_profile: SessionModelProfile,
    capture_sha256: String,
}

impl SessionEnvironmentCapture {
    /// Returns the exact validated observations.
    #[must_use]
    pub const fn input(&self) -> &SessionEnvironmentInput {
        &self.input
    }

    /// Returns the platform identity bound into the capture.
    #[must_use]
    pub const fn platform(&self) -> &SessionPlatformBinding {
        &self.platform
    }

    /// Returns the exact effective permission-profile binding.
    #[must_use]
    pub const fn permission_profile(&self) -> &SessionPermissionProfile {
        &self.permission_profile
    }

    /// Returns the exact selected model-profile binding.
    #[must_use]
    pub const fn model_profile(&self) -> &SessionModelProfile {
        &self.model_profile
    }

    /// Returns the deterministic digest of all captured facts.
    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.capture_sha256
    }
}

#[derive(Serialize)]
struct CaptureMaterial<'a> {
    schema_version: u16,
    record_type: &'static str,
    session_id: &'a str,
    captured_at_utc: &'a str,
    local_date: &'a str,
    timezone_id: &'a str,
    current_directory: &'a WorkspaceScopePath,
    workspace_roots: &'a [WorkspaceScopePath],
    active_workspace_id: &'a str,
    repository: &'a Option<RepositoryObservation>,
    attachments: &'a [AttachedFileProvenance],
    platform_family: &'static str,
    platform_architecture: &'static str,
    os_build_sha256: String,
    toolchain_sha256: String,
    vscode_build_sha256: String,
    package_sha256: String,
    permission_profile_id: &'a str,
    model_profile_id: &'a str,
    model_enabled: bool,
    configuration_sha256: &'a str,
}

/// Validates and captures one exact session environment without ambient discovery.
pub fn capture_session_environment(
    input: SessionEnvironmentInput,
    configuration: &LoadedConfiguration,
    runtime: &PlatformRuntimeIdentity,
) -> Result<SessionEnvironmentCapture, SessionEnvironmentError> {
    validate_input(&input)?;
    let platform = platform_binding(runtime);
    let material = CaptureMaterial {
        schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
        record_type: "agentmage-session-environment-v1",
        session_id: input.session_id.as_str(),
        captured_at_utc: &input.captured_at_utc,
        local_date: &input.local_date,
        timezone_id: &input.timezone_id,
        current_directory: &input.current_directory,
        workspace_roots: &input.workspace_roots,
        active_workspace_id: input.active_workspace_id.as_str(),
        repository: &input.repository,
        attachments: &input.attachments,
        platform_family: platform_family(runtime.family()),
        platform_architecture: platform_architecture(runtime.architecture()),
        os_build_sha256: hex(runtime.os_build_sha256()),
        toolchain_sha256: hex(runtime.toolchain_sha256()),
        vscode_build_sha256: hex(runtime.vscode_build_sha256()),
        package_sha256: hex(runtime.package_sha256()),
        permission_profile_id: configuration.permission_profile_id(),
        model_profile_id: configuration.model_profile_id(),
        model_enabled: configuration.model_enabled(),
        configuration_sha256: configuration.sha256(),
    };
    let bytes = serde_json::to_vec(&material).expect("fixed session material serializes");
    Ok(SessionEnvironmentCapture {
        platform,
        permission_profile: SessionPermissionProfile {
            profile_id: configuration.permission_profile_id().to_owned(),
            configuration_sha256: configuration.sha256().to_owned(),
        },
        model_profile: SessionModelProfile {
            profile_id: configuration.model_profile_id().to_owned(),
            configuration_sha256: configuration.sha256().to_owned(),
            enabled: configuration.model_enabled(),
        },
        input,
        capture_sha256: hex(&Sha256::digest(bytes)),
    })
}

fn validate_input(input: &SessionEnvironmentInput) -> Result<(), SessionEnvironmentError> {
    if !identifier(input.session_id.as_str()) {
        return Err(SessionEnvironmentError::new(
            SessionEnvironmentErrorKind::InvalidIdentifier,
            None,
        ));
    }
    if !utc_timestamp(&input.captured_at_utc) {
        return Err(SessionEnvironmentError::new(
            SessionEnvironmentErrorKind::InvalidTimestamp,
            None,
        ));
    }
    if !calendar_date(&input.local_date) {
        return Err(SessionEnvironmentError::new(
            SessionEnvironmentErrorKind::InvalidLocalDate,
            None,
        ));
    }
    if !timezone(&input.timezone_id) {
        return Err(SessionEnvironmentError::new(
            SessionEnvironmentErrorKind::InvalidTimezone,
            None,
        ));
    }
    validate_workspaces(input)?;
    if let Some(repository) = &input.repository {
        validate_repository(input, repository)?;
    }
    validate_attachments(&input.attachments)
}

fn validate_workspaces(input: &SessionEnvironmentInput) -> Result<(), SessionEnvironmentError> {
    if input.workspace_roots.is_empty() || input.workspace_roots.len() > MAX_WORKSPACE_ROOTS {
        return Err(SessionEnvironmentError::new(
            SessionEnvironmentErrorKind::InvalidWorkspaceRoots,
            None,
        ));
    }
    let mut identities = BTreeSet::new();
    for (index, root) in input.workspace_roots.iter().enumerate() {
        if !root.components().is_empty()
            || !identities.insert(root.workspace_id().as_str())
            || !identifier(root.workspace_id().as_str())
        {
            return Err(SessionEnvironmentError::new(
                SessionEnvironmentErrorKind::InvalidWorkspaceRoots,
                Some(index),
            ));
        }
    }
    if input.current_directory.workspace_id() != &input.active_workspace_id
        || !input
            .workspace_roots
            .iter()
            .any(|root| root.workspace_id() == &input.active_workspace_id)
    {
        return Err(SessionEnvironmentError::new(
            SessionEnvironmentErrorKind::ActiveWorkspaceMismatch,
            None,
        ));
    }
    Ok(())
}

fn validate_repository(
    input: &SessionEnvironmentInput,
    repository: &RepositoryObservation,
) -> Result<(), SessionEnvironmentError> {
    if repository.workspace_id != input.active_workspace_id
        || repository.root.workspace_id() != &repository.workspace_id
        || !input
            .workspace_roots
            .iter()
            .any(|root| root.contains_scope(&repository.root))
    {
        return Err(SessionEnvironmentError::new(
            SessionEnvironmentErrorKind::RepositoryWorkspaceMismatch,
            None,
        ));
    }
    if !sha256_text(&repository.repository_sha256)
        || !git_object(&repository.head_commit)
        || matches!(&repository.head, RepositoryHead::Branch(name) if !git_branch(name))
    {
        return Err(SessionEnvironmentError::new(
            SessionEnvironmentErrorKind::InvalidRepository,
            None,
        ));
    }
    Ok(())
}

fn validate_attachments(
    attachments: &[AttachedFileProvenance],
) -> Result<(), SessionEnvironmentError> {
    if attachments.len() > MAX_ATTACHMENTS {
        return Err(SessionEnvironmentError::new(
            SessionEnvironmentErrorKind::InvalidAttachmentProvenance,
            None,
        ));
    }
    let mut identities = BTreeSet::new();
    for (index, attachment) in attachments.iter().enumerate() {
        if !identifier(&attachment.attachment_id)
            || !bounded_text(&attachment.source_id)
            || !bounded_text(&attachment.object_id)
            || attachment.byte_len > MAX_ATTACHMENT_BYTES
            || !sha256_text(&attachment.content_sha256)
            || !bounded_text(&attachment.observed_revision)
            || !identities.insert(attachment.attachment_id.as_str())
        {
            return Err(SessionEnvironmentError::new(
                SessionEnvironmentErrorKind::InvalidAttachmentProvenance,
                Some(index),
            ));
        }
    }
    Ok(())
}

fn platform_binding(runtime: &PlatformRuntimeIdentity) -> SessionPlatformBinding {
    SessionPlatformBinding {
        family: runtime.family(),
        architecture: runtime.architecture(),
        os_build_sha256: hex(runtime.os_build_sha256()),
        toolchain_sha256: hex(runtime.toolchain_sha256()),
        vscode_build_sha256: hex(runtime.vscode_build_sha256()),
        package_sha256: hex(runtime.package_sha256()),
    }
}

const fn platform_family(value: PlatformFamily) -> &'static str {
    match value {
        PlatformFamily::DeterministicFake => "deterministic_fake",
        PlatformFamily::Fedora => "fedora",
        PlatformFamily::Ubuntu => "ubuntu",
        PlatformFamily::MacOsAppleSilicon => "macos_apple_silicon",
    }
}

const fn platform_architecture(value: PlatformArchitecture) -> &'static str {
    match value {
        PlatformArchitecture::X86_64 => "x86_64",
        PlatformArchitecture::Aarch64 => "aarch64",
    }
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn bounded_text(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PROVENANCE_TEXT_BYTES
        && !value.chars().any(char::is_control)
}

fn sha256_text(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn git_object(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn git_branch(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_REPOSITORY_BRANCH_BYTES
        && !value.starts_with(['-', '.', '/'])
        && !value.ends_with(['.', '/'])
        && !value.ends_with(".lock")
        && !value.contains("..")
        && !value.contains("@{")
        && !value.contains("//")
        && value
            .split('/')
            .all(|part| !part.is_empty() && !part.starts_with('.') && !part.ends_with('.'))
        && !value
            .chars()
            .any(|character| character.is_control() || " ~^:?*[\\".contains(character))
}

fn calendar_date(value: &str) -> bool {
    if value.len() != 10 || value.as_bytes()[4] != b'-' || value.as_bytes()[7] != b'-' {
        return false;
    }
    let Ok(year) = value[0..4].parse::<u16>() else {
        return false;
    };
    let Ok(month) = value[5..7].parse::<u8>() else {
        return false;
    };
    let Ok(day) = value[8..10].parse::<u8>() else {
        return false;
    };
    if year == 0 || !(1..=12).contains(&month) {
        return false;
    }
    let maximum = match month {
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(400) || (year.is_multiple_of(4) && !year.is_multiple_of(100)) => {
            29
        }
        2 => 28,
        _ => 31,
    };
    (1..=maximum).contains(&day)
}

fn utc_timestamp(value: &str) -> bool {
    value.len() == 20
        && value.as_bytes()[10] == b'T'
        && value.as_bytes()[13] == b':'
        && value.as_bytes()[16] == b':'
        && value.as_bytes()[19] == b'Z'
        && calendar_date(&value[0..10])
        && value[11..13].parse::<u8>().is_ok_and(|hour| hour < 24)
        && value[14..16].parse::<u8>().is_ok_and(|minute| minute < 60)
        && value[17..19].parse::<u8>().is_ok_and(|second| second < 60)
}

fn timezone(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_TIMEZONE_BYTES
        && !value.starts_with('/')
        && !value.ends_with('/')
        && !value.contains("//")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'+' | b'/'))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authority::{
        DescriptiveArtifactKind, NonAuthoritativeArtifact, reject_as_authority,
    };
    use crate::configuration::ConfigurationManager;

    fn scope(workspace: &str, components: &[&str]) -> WorkspaceScopePath {
        WorkspaceScopePath::new(WorkspaceId::from_raw(workspace), components.iter().copied())
            .expect("fixture scope is canonical")
    }

    fn runtime() -> PlatformRuntimeIdentity {
        PlatformRuntimeIdentity::new(
            PlatformFamily::Fedora,
            PlatformArchitecture::X86_64,
            [1; 32],
            [2; 32],
            [3; 32],
            [4; 32],
        )
    }

    fn attachment(identity: &str) -> AttachedFileProvenance {
        AttachedFileProvenance {
            attachment_id: identity.to_owned(),
            source_id: "vscode-chat-attachment".to_owned(),
            object_id: format!("object-{identity}"),
            byte_len: 42,
            content_sha256: "a".repeat(64),
            observed_revision: "capture-0001".to_owned(),
        }
    }

    fn input() -> SessionEnvironmentInput {
        SessionEnvironmentInput {
            session_id: SessionId::from_raw("session-0001"),
            captured_at_utc: "2026-08-13T20:00:00Z".to_owned(),
            local_date: "2026-08-13".to_owned(),
            timezone_id: "America/Chicago".to_owned(),
            current_directory: scope("workspace-0001", &["src"]),
            workspace_roots: vec![scope("workspace-0001", &[])],
            active_workspace_id: WorkspaceId::from_raw("workspace-0001"),
            repository: Some(RepositoryObservation {
                workspace_id: WorkspaceId::from_raw("workspace-0001"),
                root: scope("workspace-0001", &[]),
                repository_sha256: "b".repeat(64),
                head_commit: "c".repeat(40),
                head: RepositoryHead::Branch("agent/session-capture".to_owned()),
            }),
            attachments: vec![attachment("attachment-0001")],
        }
    }

    fn configuration() -> LoadedConfiguration {
        ConfigurationManager::default()
            .safe_defaults()
            .expect("safe defaults load")
    }

    #[test]
    fn exact_environment_captures_every_required_binding_deterministically() {
        let first = capture_session_environment(input(), &configuration(), &runtime())
            .expect("valid environment captures");
        let second = capture_session_environment(input(), &configuration(), &runtime())
            .expect("same environment captures");
        assert_eq!(first, second);
        assert_eq!(first.input().timezone_id, "America/Chicago");
        assert_eq!(first.input().workspace_roots.len(), 1);
        assert!(first.input().repository.is_some());
        assert_eq!(first.input().attachments.len(), 1);
        assert_eq!(first.platform().family(), PlatformFamily::Fedora);
        assert_eq!(
            first.platform().architecture(),
            PlatformArchitecture::X86_64
        );
        assert_eq!(first.sha256().len(), 64);
    }

    #[test]
    fn malformed_time_timezone_repository_and_branch_fail_closed() {
        let cases = [
            ("timestamp", "session.environment.invalid_timestamp"),
            ("date", "session.environment.invalid_local_date"),
            ("timezone", "session.environment.invalid_timezone"),
            ("repository", "session.environment.invalid_repository"),
            ("branch", "session.environment.invalid_repository"),
        ];
        for (case, code) in cases {
            let mut candidate = input();
            match case {
                "timestamp" => candidate.captured_at_utc = "2026-08-13".to_owned(),
                "date" => candidate.local_date = "2026-02-30".to_owned(),
                "timezone" => candidate.timezone_id = "America//Chicago".to_owned(),
                "repository" => {
                    candidate
                        .repository
                        .as_mut()
                        .expect("repository")
                        .repository_sha256 = "not-a-hash".to_owned();
                }
                "branch" => {
                    candidate.repository.as_mut().expect("repository").head =
                        RepositoryHead::Branch("../unsafe".to_owned());
                }
                _ => unreachable!(),
            }
            let error = capture_session_environment(candidate, &configuration(), &runtime())
                .expect_err("malformed environment fails");
            assert_eq!(error.kind().code(), code);
        }
    }

    #[test]
    fn workspace_and_repository_drift_fail_before_capture() {
        let mut wrong_active = input();
        wrong_active.active_workspace_id = WorkspaceId::from_raw("workspace-0002");
        assert_eq!(
            capture_session_environment(wrong_active, &configuration(), &runtime())
                .expect_err("wrong active workspace fails")
                .kind(),
            SessionEnvironmentErrorKind::ActiveWorkspaceMismatch
        );

        let mut wrong_repository = input();
        wrong_repository
            .repository
            .as_mut()
            .expect("repository")
            .workspace_id = WorkspaceId::from_raw("workspace-0002");
        assert_eq!(
            capture_session_environment(wrong_repository, &configuration(), &runtime())
                .expect_err("foreign repository fails")
                .kind(),
            SessionEnvironmentErrorKind::RepositoryWorkspaceMismatch
        );
    }

    #[test]
    fn duplicate_and_oversized_workspace_or_attachment_sets_fail_closed() {
        let mut duplicate_roots = input();
        duplicate_roots
            .workspace_roots
            .push(scope("workspace-0001", &[]));
        let error = capture_session_environment(duplicate_roots, &configuration(), &runtime())
            .expect_err("duplicate roots fail");
        assert_eq!(error.item_index(), Some(1));

        let mut duplicate_attachments = input();
        duplicate_attachments
            .attachments
            .push(attachment("attachment-0001"));
        let error =
            capture_session_environment(duplicate_attachments, &configuration(), &runtime())
                .expect_err("duplicate attachments fail");
        assert_eq!(error.item_index(), Some(1));

        let mut oversized = input();
        oversized.attachments = (0..=MAX_ATTACHMENTS)
            .map(|index| attachment(&format!("attachment-{index:04}")))
            .collect();
        assert_eq!(
            capture_session_environment(oversized, &configuration(), &runtime())
                .expect_err("oversized attachment set fails")
                .kind(),
            SessionEnvironmentErrorKind::InvalidAttachmentProvenance
        );
    }

    #[test]
    fn permission_and_model_profiles_come_only_from_loaded_configuration() {
        let configuration = configuration();
        let capture = capture_session_environment(input(), &configuration, &runtime())
            .expect("valid environment captures");
        assert_eq!(
            capture.permission_profile().profile_id(),
            configuration.profile_id()
        );
        assert_eq!(capture.model_profile().profile_id(), "fake-model-v1");
        assert!(!capture.model_profile().enabled());
        assert_eq!(
            capture.permission_profile().configuration_sha256(),
            configuration.sha256()
        );
        assert_eq!(
            capture.model_profile().configuration_sha256(),
            configuration.sha256()
        );
    }

    #[test]
    fn session_capture_is_descriptive_and_cannot_grant_authority() {
        assert_eq!(
            <SessionEnvironmentCapture as NonAuthoritativeArtifact>::KIND,
            DescriptiveArtifactKind::SessionRecord
        );
        let capture = capture_session_environment(input(), &configuration(), &runtime())
            .expect("valid environment captures");
        let denial = reject_as_authority(&capture);
        assert_eq!(denial.artifact_kind, DescriptiveArtifactKind::SessionRecord);
    }
}
