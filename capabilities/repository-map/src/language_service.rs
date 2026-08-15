//! Confined, read-only language-service descriptors and untrusted observations.

use std::collections::BTreeSet;
use std::fmt::Write;

use agentmage_kernel_contracts::WorkspacePath;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{RepositoryLanguage, SourceRange, grammar_descriptor};

const LANGUAGE_SERVICE_SCHEMA_VERSION: u16 = 1;
const MAX_ITEMS: usize = 10_000;
const MAX_PATHS: usize = 50_000;
const MAX_TIMEOUT_MS: u64 = 120_000;
const MAX_MEMORY_BYTES: u64 = 8 * 1024 * 1024 * 1024;
const MAX_CPU_MS: u64 = 120_000;

/// Closed read-only language-service capability set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguageServiceCapability {
    /// Resolve an exact definition.
    Definition,
    /// Resolve exact references.
    References,
    /// Return a rename proposal without applying it.
    RenamePreview,
    /// Return diagnostics.
    Diagnostics,
    /// Return a code-action proposal without command execution.
    CodeActions,
}

/// Exact confined service descriptor supplied by an approved host boundary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanguageServiceDescriptor {
    /// Stable registered service identity.
    pub service_id: String,
    /// Exact implementation version.
    pub implementation_version: String,
    /// SHA-256 of the exact installed executable artifact.
    pub executable_sha256: String,
    /// Exact parser or service language.
    pub language: RepositoryLanguage,
    /// SHA-256 of the exact compiled grammar descriptor for the language.
    pub grammar_descriptor_sha256: String,
    /// Stable sorted closed capability set.
    pub capabilities: Vec<LanguageServiceCapability>,
    /// Exact authorized workspace-root identity without a host path.
    pub workspace_root_sha256: String,
    /// Maximum response-item count.
    pub maximum_response_items: u32,
    /// Fixed request timeout.
    pub timeout_ms: u64,
    /// Fixed memory ceiling.
    pub memory_limit_bytes: u64,
    /// Fixed CPU-time ceiling.
    pub cpu_time_limit_ms: u64,
    /// Only these non-secret environment names may be provided by a later launcher.
    pub environment_name_allowlist: Vec<String>,
    /// Service operation must remain read-only.
    pub read_only: bool,
    /// Network is prohibited.
    pub network_allowed: bool,
    /// Workspace writes are prohibited.
    pub workspace_write_allowed: bool,
    /// Arbitrary command execution is prohibited.
    pub command_execution_allowed: bool,
    /// Package installation is prohibited.
    pub package_installation_allowed: bool,
    /// Plugin loading is prohibited.
    pub plugin_loading_allowed: bool,
    /// Ambient executable discovery is prohibited.
    pub executable_discovery_allowed: bool,
    /// Ambient environment inheritance is prohibited.
    pub environment_inheritance_allowed: bool,
    /// SHA-256 over every preceding field.
    pub descriptor_sha256: String,
}

/// One exact workspace file visible to a confined service request.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanguageServiceVisibleFile {
    /// Exact permitted workspace path.
    pub path: WorkspacePath,
    /// SHA-256 of the complete visible source bytes.
    pub source_sha256: String,
    /// Exact visible source length in bytes.
    pub source_byte_len: u64,
}

/// Exact request class for one service observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguageServiceRequestKind {
    /// Definition lookup.
    Definition,
    /// Reference lookup.
    References,
    /// Rename preview.
    RenamePreview,
    /// Diagnostics request.
    Diagnostics,
    /// Code-action proposal request.
    CodeActions,
}

impl LanguageServiceRequestKind {
    const fn capability(self) -> LanguageServiceCapability {
        match self {
            Self::Definition => LanguageServiceCapability::Definition,
            Self::References => LanguageServiceCapability::References,
            Self::RenamePreview => LanguageServiceCapability::RenamePreview,
            Self::Diagnostics => LanguageServiceCapability::Diagnostics,
            Self::CodeActions => LanguageServiceCapability::CodeActions,
        }
    }
}

/// Exact immutable request identity and granted path set.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanguageServiceRequest {
    /// Stable request identity.
    pub request_id: String,
    /// Exact deep repository index identity.
    pub index_sha256: String,
    /// Exact descriptor identity.
    pub descriptor_sha256: String,
    /// Exact request class.
    pub kind: LanguageServiceRequestKind,
    /// Exact source path.
    pub source_path: WorkspacePath,
    /// Exact complete source digest.
    pub source_sha256: String,
    /// Exact complete source length in bytes.
    pub source_byte_len: u64,
    /// Exact source position.
    pub source_byte: u64,
    /// Optional hash of the requested symbol or replacement text, never its value.
    pub subject_sha256: Option<String>,
    /// Stable path-sorted exact file snapshots visible to the service.
    pub allowed_files: Vec<LanguageServiceVisibleFile>,
    /// Later execution requires its own grant.
    pub separate_grant_required: bool,
    /// The request carries no write authority.
    pub write_authority: bool,
}

/// Closed terminal service observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguageServiceObservationStatus {
    /// Complete within the declared request and item bound.
    Complete,
    /// Valid but explicitly truncated or incomplete.
    Partial,
    /// Service or capability was unavailable.
    Unavailable,
    /// Policy or confinement rejected execution.
    Rejected,
    /// Request was cancelled.
    Cancelled,
    /// Request exceeded its fixed timeout.
    TimedOut,
}

/// Closed result-item class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LanguageServiceItemKind {
    /// Definition location.
    Definition,
    /// Reference location.
    Reference,
    /// Proposed rename edit.
    RenameEdit,
    /// Diagnostic observation.
    Diagnostic,
    /// Proposed code edit without command execution.
    CodeActionEdit,
}

/// One bounded hash-only untrusted service result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanguageServiceItem {
    /// Stable item identity.
    pub item_id: String,
    /// Closed item class.
    pub kind: LanguageServiceItemKind,
    /// Exact permitted target path.
    pub path: WorkspacePath,
    /// Exact source range.
    pub range: SourceRange,
    /// SHA-256 of the exact target source bytes.
    pub source_sha256: String,
    /// Hash of bounded diagnostic or action metadata, never raw text.
    pub metadata_sha256: String,
    /// Exact proposed replacement digest for edit previews only.
    pub replacement_sha256: Option<String>,
    /// Service output remains untrusted data.
    pub untrusted_output: bool,
    /// An observation carries no write authority.
    pub write_authority: bool,
}

/// Complete hash-bound language-service observation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LanguageServiceObservation {
    /// Schema version.
    pub schema_version: u16,
    /// Exact descriptor.
    pub descriptor: LanguageServiceDescriptor,
    /// Exact request.
    pub request: LanguageServiceRequest,
    /// Explicit terminal status.
    pub status: LanguageServiceObservationStatus,
    /// Stable sorted bounded result items.
    pub items: Vec<LanguageServiceItem>,
    /// SHA-256 of the complete bounded raw response retained outside this record.
    pub raw_response_sha256: Option<String>,
    /// Stable content-free terminal code for noncomplete outcomes.
    pub terminal_code: Option<String>,
    /// Whether the response was truncated.
    pub truncated: bool,
    /// No command was executed from a response.
    pub response_command_executed: bool,
    /// No response changed workspace bytes.
    pub workspace_mutated: bool,
    /// SHA-256 over every preceding field.
    pub observation_sha256: String,
}

/// Content-free language-service validation failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LanguageServiceError {
    /// Descriptor syntax, authority, or limits are invalid.
    DescriptorInvalid,
    /// Request syntax, scope, or authority is invalid.
    RequestInvalid,
    /// Response status or result items are invalid.
    ObservationInvalid,
}

impl LanguageServiceError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::DescriptorInvalid => "language-service.descriptor-invalid",
            Self::RequestInvalid => "language-service.request-invalid",
            Self::ObservationInvalid => "language-service.observation-invalid",
        }
    }
}

/// Seals one exact confined service descriptor.
pub fn seal_language_service_descriptor(
    mut descriptor: LanguageServiceDescriptor,
) -> Result<LanguageServiceDescriptor, LanguageServiceError> {
    descriptor.descriptor_sha256.clear();
    if !valid_identifier(&descriptor.service_id)
        || !valid_version(&descriptor.implementation_version)
        || !is_sha256(&descriptor.executable_sha256)
        || descriptor.grammar_descriptor_sha256
            != grammar_descriptor(descriptor.language).descriptor_sha256
        || descriptor.capabilities.is_empty()
        || descriptor.capabilities.len() > LanguageServiceCapability::ALL.len()
        || descriptor
            .capabilities
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        || !is_sha256(&descriptor.workspace_root_sha256)
        || descriptor.maximum_response_items == 0
        || usize::try_from(descriptor.maximum_response_items)
            .map_or(true, |value| value > MAX_ITEMS)
        || descriptor.timeout_ms == 0
        || descriptor.timeout_ms > MAX_TIMEOUT_MS
        || descriptor.memory_limit_bytes == 0
        || descriptor.memory_limit_bytes > MAX_MEMORY_BYTES
        || descriptor.cpu_time_limit_ms == 0
        || descriptor.cpu_time_limit_ms > MAX_CPU_MS
        || descriptor.environment_name_allowlist
            != [
                "LANG".to_owned(),
                "LC_ALL".to_owned(),
                "NO_COLOR".to_owned(),
            ]
        || !descriptor.read_only
        || descriptor.network_allowed
        || descriptor.workspace_write_allowed
        || descriptor.command_execution_allowed
        || descriptor.package_installation_allowed
        || descriptor.plugin_loading_allowed
        || descriptor.executable_discovery_allowed
        || descriptor.environment_inheritance_allowed
    {
        return Err(LanguageServiceError::DescriptorInvalid);
    }
    descriptor.descriptor_sha256 = descriptor_digest(&descriptor);
    Ok(descriptor)
}

impl LanguageServiceCapability {
    const ALL: [Self; 5] = [
        Self::Definition,
        Self::References,
        Self::RenamePreview,
        Self::Diagnostics,
        Self::CodeActions,
    ];
}

/// Seals one untrusted observation without granting authority.
pub fn seal_language_service_observation(
    descriptor: &LanguageServiceDescriptor,
    request: LanguageServiceRequest,
    status: LanguageServiceObservationStatus,
    items: Vec<LanguageServiceItem>,
    raw_response_sha256: Option<String>,
    terminal_code: Option<String>,
    truncated: bool,
) -> Result<LanguageServiceObservation, LanguageServiceError> {
    if !seal_language_service_descriptor(descriptor.clone())
        .is_ok_and(|expected| expected == *descriptor)
    {
        return Err(LanguageServiceError::DescriptorInvalid);
    }
    validate_request(descriptor, &request)?;
    validate_observation(
        descriptor,
        &request,
        status,
        &items,
        raw_response_sha256.as_deref(),
        terminal_code.as_deref(),
        truncated,
    )?;
    let mut observation = LanguageServiceObservation {
        schema_version: LANGUAGE_SERVICE_SCHEMA_VERSION,
        descriptor: descriptor.clone(),
        request,
        status,
        items,
        raw_response_sha256,
        terminal_code,
        truncated,
        response_command_executed: false,
        workspace_mutated: false,
        observation_sha256: String::new(),
    };
    observation.observation_sha256 = observation_digest(&observation);
    Ok(observation)
}

/// Verifies one observation by exact deterministic resealing.
#[must_use]
pub fn verify_language_service_observation(observation: &LanguageServiceObservation) -> bool {
    observation.schema_version == LANGUAGE_SERVICE_SCHEMA_VERSION
        && !observation.response_command_executed
        && !observation.workspace_mutated
        && observation.observation_sha256 == observation_digest(observation)
        && seal_language_service_observation(
            &observation.descriptor,
            observation.request.clone(),
            observation.status,
            observation.items.clone(),
            observation.raw_response_sha256.clone(),
            observation.terminal_code.clone(),
            observation.truncated,
        )
        .is_ok_and(|expected| expected == *observation)
}

fn validate_request(
    descriptor: &LanguageServiceDescriptor,
    request: &LanguageServiceRequest,
) -> Result<(), LanguageServiceError> {
    if !valid_identifier(&request.request_id)
        || !is_sha256(&request.index_sha256)
        || request.descriptor_sha256 != descriptor.descriptor_sha256
        || !descriptor.capabilities.contains(&request.kind.capability())
        || !is_sha256(&request.source_sha256)
        || request.source_byte > request.source_byte_len
        || request
            .subject_sha256
            .as_deref()
            .is_some_and(|value| !is_sha256(value))
        || request.allowed_files.is_empty()
        || request.allowed_files.len() > MAX_PATHS
        || request
            .allowed_files
            .windows(2)
            .any(|pair| pair[0].path >= pair[1].path)
        || request.allowed_files.iter().any(|file| {
            file.path.workspace_id() != request.source_path.workspace_id()
                || !is_sha256(&file.source_sha256)
        })
        || !request.allowed_files.iter().any(|file| {
            file.path == request.source_path
                && file.source_sha256 == request.source_sha256
                && file.source_byte_len == request.source_byte_len
        })
        || !request.separate_grant_required
        || request.write_authority
    {
        return Err(LanguageServiceError::RequestInvalid);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_observation(
    descriptor: &LanguageServiceDescriptor,
    request: &LanguageServiceRequest,
    status: LanguageServiceObservationStatus,
    items: &[LanguageServiceItem],
    raw_response_sha256: Option<&str>,
    terminal_code: Option<&str>,
    truncated: bool,
) -> Result<(), LanguageServiceError> {
    let complete = status == LanguageServiceObservationStatus::Complete;
    let partial = status == LanguageServiceObservationStatus::Partial;
    if items.len() > usize::try_from(descriptor.maximum_response_items).unwrap_or(usize::MAX)
        || items
            .windows(2)
            .any(|pair| pair[0].item_id >= pair[1].item_id)
        || raw_response_sha256.is_some_and(|value| !is_sha256(value))
        || match status {
            LanguageServiceObservationStatus::Complete => {
                raw_response_sha256.is_none() || terminal_code.is_some() || truncated
            }
            LanguageServiceObservationStatus::Partial => {
                raw_response_sha256.is_none() || terminal_code.is_none() || !truncated
            }
            LanguageServiceObservationStatus::Unavailable
            | LanguageServiceObservationStatus::Rejected
            | LanguageServiceObservationStatus::Cancelled
            | LanguageServiceObservationStatus::TimedOut => {
                !items.is_empty()
                    || raw_response_sha256.is_some()
                    || terminal_code.is_none_or(|code| !valid_identifier(code))
                    || truncated
            }
        }
    {
        return Err(LanguageServiceError::ObservationInvalid);
    }
    let expected_kind = match request.kind {
        LanguageServiceRequestKind::Definition => LanguageServiceItemKind::Definition,
        LanguageServiceRequestKind::References => LanguageServiceItemKind::Reference,
        LanguageServiceRequestKind::RenamePreview => LanguageServiceItemKind::RenameEdit,
        LanguageServiceRequestKind::Diagnostics => LanguageServiceItemKind::Diagnostic,
        LanguageServiceRequestKind::CodeActions => LanguageServiceItemKind::CodeActionEdit,
    };
    let allowed = request
        .allowed_files
        .iter()
        .map(|file| (&file.path, (&file.source_sha256, file.source_byte_len)))
        .collect::<BTreeSet<_>>();
    for item in items {
        let edit = matches!(
            item.kind,
            LanguageServiceItemKind::RenameEdit | LanguageServiceItemKind::CodeActionEdit
        );
        let visible_source = allowed
            .iter()
            .find_map(|(path, source)| (*path == &item.path).then_some(*source));
        if !valid_identifier(&item.item_id)
            || item.kind != expected_kind
            || visible_source.is_none()
            || item.path.workspace_id() != request.source_path.workspace_id()
            || item.range.start_byte > item.range.end_byte
            || item.range.start_line == 0
            || item.range.end_line < item.range.start_line
            || !is_sha256(&item.source_sha256)
            || visible_source.is_some_and(|(sha256, byte_len)| {
                item.source_sha256 != *sha256 || item.range.end_byte > byte_len
            })
            || !is_sha256(&item.metadata_sha256)
            || edit != item.replacement_sha256.is_some()
            || item
                .replacement_sha256
                .as_deref()
                .is_some_and(|value| !is_sha256(value))
            || !item.untrusted_output
            || item.write_authority
        {
            return Err(LanguageServiceError::ObservationInvalid);
        }
    }
    if (complete || partial)
        && items.is_empty()
        && request.kind != LanguageServiceRequestKind::Diagnostics
    {
        return Err(LanguageServiceError::ObservationInvalid);
    }
    Ok(())
}

fn descriptor_digest(descriptor: &LanguageServiceDescriptor) -> String {
    let mut canonical = descriptor.clone();
    canonical.descriptor_sha256.clear();
    sha256_json(&canonical)
}

fn observation_digest(observation: &LanguageServiceObservation) -> String {
    sha256_json(&(
        observation.schema_version,
        &observation.descriptor,
        &observation.request,
        observation.status,
        &observation.items,
        &observation.raw_response_sha256,
        &observation.terminal_code,
        observation.truncated,
        observation.response_command_executed,
        observation.workspace_mutated,
    ))
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn valid_version(value: &str) -> bool {
    valid_identifier(value) && value.bytes().any(|byte| byte.is_ascii_digit())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn sha256_json(value: &impl Serialize) -> String {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .unwrap_or_else(|_| sha256_hex(b"language-service-serialization-failed"))
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

    use super::*;

    fn path(parts: &[&str]) -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-language-service"),
            parts.iter().copied(),
        )
        .expect("path")
    }

    fn descriptor() -> LanguageServiceDescriptor {
        seal_language_service_descriptor(LanguageServiceDescriptor {
            service_id: "pyright-confined".to_owned(),
            implementation_version: "1.1.404".to_owned(),
            executable_sha256: "1".repeat(64),
            language: RepositoryLanguage::Python,
            grammar_descriptor_sha256: grammar_descriptor(RepositoryLanguage::Python)
                .descriptor_sha256,
            capabilities: LanguageServiceCapability::ALL.to_vec(),
            workspace_root_sha256: "2".repeat(64),
            maximum_response_items: 100,
            timeout_ms: 30_000,
            memory_limit_bytes: 512 * 1024 * 1024,
            cpu_time_limit_ms: 30_000,
            environment_name_allowlist: vec![
                "LANG".to_owned(),
                "LC_ALL".to_owned(),
                "NO_COLOR".to_owned(),
            ],
            read_only: true,
            network_allowed: false,
            workspace_write_allowed: false,
            command_execution_allowed: false,
            package_installation_allowed: false,
            plugin_loading_allowed: false,
            executable_discovery_allowed: false,
            environment_inheritance_allowed: false,
            descriptor_sha256: String::new(),
        })
        .expect("descriptor")
    }

    fn request(
        descriptor: &LanguageServiceDescriptor,
        kind: LanguageServiceRequestKind,
    ) -> LanguageServiceRequest {
        LanguageServiceRequest {
            request_id: "request-language-1".to_owned(),
            index_sha256: "3".repeat(64),
            descriptor_sha256: descriptor.descriptor_sha256.clone(),
            kind,
            source_path: path(&["src", "module.py"]),
            source_sha256: "6".repeat(64),
            source_byte_len: 128,
            source_byte: 4,
            subject_sha256: Some("5".repeat(64)),
            allowed_files: vec![
                LanguageServiceVisibleFile {
                    path: path(&["src", "module.py"]),
                    source_sha256: "6".repeat(64),
                    source_byte_len: 128,
                },
                LanguageServiceVisibleFile {
                    path: path(&["tests", "test_module.py"]),
                    source_sha256: "6".repeat(64),
                    source_byte_len: 256,
                },
            ],
            separate_grant_required: true,
            write_authority: false,
        }
    }

    fn item(kind: LanguageServiceItemKind, target: WorkspacePath) -> LanguageServiceItem {
        LanguageServiceItem {
            item_id: "item-language-1".to_owned(),
            kind,
            path: target,
            range: SourceRange {
                start_byte: 0,
                end_byte: 5,
                start_line: 1,
                end_line: 1,
                start_column: 0,
                end_column: 5,
            },
            source_sha256: "6".repeat(64),
            metadata_sha256: "7".repeat(64),
            replacement_sha256: matches!(
                kind,
                LanguageServiceItemKind::RenameEdit | LanguageServiceItemKind::CodeActionEdit
            )
            .then(|| "8".repeat(64)),
            untrusted_output: true,
            write_authority: false,
        }
    }

    #[test]
    fn every_read_capability_seals_bounded_untrusted_observations() {
        let descriptor = descriptor();
        let cases = [
            (
                LanguageServiceRequestKind::Definition,
                LanguageServiceItemKind::Definition,
            ),
            (
                LanguageServiceRequestKind::References,
                LanguageServiceItemKind::Reference,
            ),
            (
                LanguageServiceRequestKind::RenamePreview,
                LanguageServiceItemKind::RenameEdit,
            ),
            (
                LanguageServiceRequestKind::Diagnostics,
                LanguageServiceItemKind::Diagnostic,
            ),
            (
                LanguageServiceRequestKind::CodeActions,
                LanguageServiceItemKind::CodeActionEdit,
            ),
        ];
        for (request_kind, item_kind) in cases {
            let request = request(&descriptor, request_kind);
            let observation = seal_language_service_observation(
                &descriptor,
                request.clone(),
                LanguageServiceObservationStatus::Complete,
                vec![item(item_kind, request.source_path)],
                Some("9".repeat(64)),
                None,
                false,
            )
            .expect("observation");
            assert!(verify_language_service_observation(&observation));
            assert!(!observation.response_command_executed);
            assert!(!observation.workspace_mutated);
            assert!(
                observation
                    .items
                    .iter()
                    .all(|entry| { entry.untrusted_output && !entry.write_authority })
            );
        }
    }

    #[test]
    fn network_plugins_commands_writes_install_discovery_and_environment_fail_closed() {
        let mutations: [fn(&mut LanguageServiceDescriptor); 7] = [
            |value| value.network_allowed = true,
            |value| value.plugin_loading_allowed = true,
            |value| value.command_execution_allowed = true,
            |value| value.workspace_write_allowed = true,
            |value| value.package_installation_allowed = true,
            |value| value.executable_discovery_allowed = true,
            |value| value.environment_inheritance_allowed = true,
        ];
        for mutate in mutations {
            let mut candidate = descriptor();
            mutate(&mut candidate);
            assert_eq!(
                seal_language_service_descriptor(candidate),
                Err(LanguageServiceError::DescriptorInvalid)
            );
        }
        let mut secret_environment = descriptor();
        secret_environment
            .environment_name_allowlist
            .push("SECRET_TOKEN".to_owned());
        assert_eq!(
            seal_language_service_descriptor(secret_environment),
            Err(LanguageServiceError::DescriptorInvalid)
        );
    }

    #[test]
    fn workspace_expansion_hostile_items_and_false_terminal_states_fail_closed() {
        let descriptor = descriptor();
        let request = request(&descriptor, LanguageServiceRequestKind::RenamePreview);
        let outside = WorkspacePath::new(WorkspaceId::from_raw("other-workspace"), ["outside.py"])
            .expect("outside");
        assert_eq!(
            seal_language_service_observation(
                &descriptor,
                request.clone(),
                LanguageServiceObservationStatus::Complete,
                vec![item(LanguageServiceItemKind::RenameEdit, outside)],
                Some("9".repeat(64)),
                None,
                false,
            ),
            Err(LanguageServiceError::ObservationInvalid)
        );

        let mut hostile = item(
            LanguageServiceItemKind::RenameEdit,
            request.source_path.clone(),
        );
        hostile.write_authority = true;
        assert_eq!(
            seal_language_service_observation(
                &descriptor,
                request.clone(),
                LanguageServiceObservationStatus::Complete,
                vec![hostile],
                Some("9".repeat(64)),
                None,
                false,
            ),
            Err(LanguageServiceError::ObservationInvalid)
        );

        assert_eq!(
            seal_language_service_observation(
                &descriptor,
                request,
                LanguageServiceObservationStatus::TimedOut,
                Vec::new(),
                Some("9".repeat(64)),
                Some("service-timeout".to_owned()),
                false,
            ),
            Err(LanguageServiceError::ObservationInvalid)
        );
    }

    #[test]
    fn descriptor_request_response_and_digest_mutation_never_verify() {
        let descriptor = descriptor();
        let request = request(&descriptor, LanguageServiceRequestKind::References);
        let baseline = seal_language_service_observation(
            &descriptor,
            request.clone(),
            LanguageServiceObservationStatus::Complete,
            vec![item(
                LanguageServiceItemKind::Reference,
                request.source_path,
            )],
            Some("9".repeat(64)),
            None,
            false,
        )
        .expect("observation");
        for sequence in 0_u8..8 {
            let mut changed = baseline.clone();
            match sequence {
                0 => changed.descriptor.implementation_version.push('x'),
                1 => changed.descriptor.executable_sha256 = "a".repeat(64),
                2 => changed.request.index_sha256 = "b".repeat(64),
                3 => changed.request.allowed_files.clear(),
                4 => changed.items[0].metadata_sha256 = "c".repeat(64),
                5 => changed.response_command_executed = true,
                6 => changed.workspace_mutated = true,
                _ => changed.observation_sha256 = "d".repeat(64),
            }
            assert!(!verify_language_service_observation(&changed));
        }
    }
}
