//! Trusted validation-template provenance, exact registries, and bounded selection.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use agentmage_kernel_contracts::WorkspacePath;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::command_runner::{CommandSpec, CommandWorkingDirectory};

const VALIDATION_TEMPLATE_SCHEMA_VERSION: u16 = 1;
const MAX_TEMPLATES: usize = 256;
const MAX_ARTIFACTS: usize = 64;
const MAX_MAPPINGS: usize = 10_000;
const MAX_CHANGED_PATHS: usize = 10_000;

/// Closed validation result families.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationKind {
    /// Isolated unit tests.
    Unit,
    /// Cross-component integration tests.
    Integration,
    /// User-visible end-to-end tests.
    EndToEnd,
    /// Static lint checks.
    Lint,
    /// Nonmutating format checks.
    Format,
    /// Static type checks.
    Types,
    /// Compilation or product build.
    Build,
    /// Package construction or package verification.
    Packaging,
    /// Security-specific checks.
    Security,
}

impl ValidationKind {
    /// Complete stable kind set.
    pub const ALL: [Self; 9] = [
        Self::Unit,
        Self::Integration,
        Self::EndToEnd,
        Self::Lint,
        Self::Format,
        Self::Types,
        Self::Build,
        Self::Packaging,
        Self::Security,
    ];

    /// Whether this kind must observe at least one executed test.
    #[must_use]
    pub const fn is_test(self) -> bool {
        matches!(self, Self::Unit | Self::Integration | Self::EndToEnd)
    }
}

/// Only admitted origins for a validation command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationTemplateSource {
    /// Exact current project configuration observed through an approved read.
    TrustedProjectConfiguration,
    /// Exact command explicitly supplied and approved by the user.
    ExplicitUserInput,
}

/// Closed parser selected before command execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationParserKind {
    /// Strict AgentMage JSON result envelope version 1.
    AgentMageJsonV1,
    /// Process/postcondition-only validation for non-test checks.
    ProcessExitV1,
}

/// One expected output artifact verified independently from process narration.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationArtifactExpectation {
    /// Stable artifact identity.
    pub artifact_id: String,
    /// Exact workspace-relative or scratch-relative artifact path.
    pub path: WorkspacePath,
    /// Whether absence prevents a pass.
    pub required: bool,
    /// Exact upper byte bound.
    pub maximum_bytes: u64,
}

/// Complete input for one immutable validation template.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationTemplateInput {
    /// Stable validation identity.
    pub validation_id: String,
    /// Exact validation kind.
    pub kind: ValidationKind,
    /// Exact immutable command spec.
    pub command: CommandSpec,
    /// Admitted source class.
    pub source: ValidationTemplateSource,
    /// Exact source configuration or user-input identity.
    pub source_sha256: String,
    /// Exact source path for project configuration; absent for explicit user input.
    pub source_path: Option<WorkspacePath>,
    /// Separate user approval identity for explicit user input only.
    pub user_input_approval_sha256: Option<String>,
    /// Exact disposable repository snapshot and effective scope identity.
    pub execution_scope_sha256: String,
    /// Closed output parser.
    pub parser: ValidationParserKind,
    /// Exact parser implementation version.
    pub parser_version: String,
    /// Exact parser artifact or source digest.
    pub parser_sha256: String,
    /// Minimum executed tests required for a passing test-kind result.
    pub minimum_test_count: u32,
    /// Whether this exact template is a focused subset.
    pub focused: bool,
    /// Whether execution stops after the first observed failure.
    pub fail_fast: bool,
    /// Whether a later separately approved rerun may select this exact template.
    pub failed_test_rerun_allowed: bool,
    /// Whether setup is safe to repeat after an uncertain attempt.
    pub setup_idempotent: bool,
    /// Stable exact artifact expectations.
    pub expected_artifacts: Vec<ValidationArtifactExpectation>,
}

/// Immutable registered validation template with no execution authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationTemplate {
    /// Schema version.
    pub schema_version: u16,
    /// Stable validation identity.
    pub validation_id: String,
    /// Exact validation kind.
    pub kind: ValidationKind,
    /// Exact immutable command spec.
    pub command: CommandSpec,
    /// Admitted source class.
    pub source: ValidationTemplateSource,
    /// Exact source identity.
    pub source_sha256: String,
    /// Exact source configuration path when applicable.
    pub source_path: Option<WorkspacePath>,
    /// Exact explicit-user-input approval when applicable.
    pub user_input_approval_sha256: Option<String>,
    /// Exact disposable repository snapshot and effective scope identity.
    pub execution_scope_sha256: String,
    /// Closed parser.
    pub parser: ValidationParserKind,
    /// Exact parser version.
    pub parser_version: String,
    /// Exact parser identity.
    pub parser_sha256: String,
    /// Required minimum executed test count.
    pub minimum_test_count: u32,
    /// Whether this is a focused template.
    pub focused: bool,
    /// Whether execution stops after the first observed failure.
    pub fail_fast: bool,
    /// Whether failed tests may be rerun by separately selecting this exact template.
    pub failed_test_rerun_allowed: bool,
    /// Whether setup is safe to repeat.
    pub setup_idempotent: bool,
    /// Exact artifact postconditions.
    pub expected_artifacts: Vec<ValidationArtifactExpectation>,
    /// Every execution requires separate approval.
    pub separate_approval_required: bool,
    /// This template carries no execution authority.
    pub execution_authority: bool,
    /// SHA-256 over every preceding field.
    pub template_sha256: String,
}

/// Frozen stable validation-template registry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationTemplateRegistry {
    /// Stable template-id-sorted set.
    pub templates: Vec<ValidationTemplate>,
    /// SHA-256 over the complete template set.
    pub registry_sha256: String,
}

impl ValidationTemplateRegistry {
    /// Validates and freezes a nonempty exact template set.
    pub fn build(mut templates: Vec<ValidationTemplate>) -> Result<Self, ValidationTemplateError> {
        if templates.is_empty() || templates.len() > MAX_TEMPLATES {
            return Err(ValidationTemplateError::InvalidInput);
        }
        templates.sort_by(|left, right| left.validation_id.cmp(&right.validation_id));
        if templates
            .windows(2)
            .any(|pair| pair[0].validation_id >= pair[1].validation_id)
            || templates
                .iter()
                .any(|template| !verify_validation_template(template))
        {
            return Err(ValidationTemplateError::DuplicateOrInvalid);
        }
        Ok(Self {
            registry_sha256: sha256_json(&templates),
            templates,
        })
    }

    /// Verifies complete deterministic registry identity.
    #[must_use]
    pub fn verify(&self) -> bool {
        !self.templates.is_empty()
            && self.templates.len() <= MAX_TEMPLATES
            && self
                .templates
                .windows(2)
                .all(|pair| pair[0].validation_id < pair[1].validation_id)
            && self.templates.iter().all(verify_validation_template)
            && self.registry_sha256 == sha256_json(&self.templates)
    }

    /// Resolves only one exact template identity and digest.
    #[must_use]
    pub fn resolve(
        &self,
        validation_id: &str,
        template_sha256: &str,
    ) -> Option<&ValidationTemplate> {
        self.templates
            .binary_search_by(|template| template.validation_id.as_str().cmp(validation_id))
            .ok()
            .and_then(|index| self.templates.get(index))
            .filter(|template| template.template_sha256 == template_sha256)
    }
}

/// One source-to-validation mapping derived from trusted repository evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationCoverageMapping {
    /// Exact changed source path.
    pub changed_path: WorkspacePath,
    /// Stable exact validation identities covering this source.
    pub validation_ids: Vec<String>,
    /// Exact mapping evidence identity.
    pub evidence_sha256: String,
}

/// Exact request for authority-free focused validation selection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationSelectionRequest {
    /// Stable selection identity.
    pub selection_id: String,
    /// Exact registry identity.
    pub registry_sha256: String,
    /// Exact changed repository snapshot identity.
    pub repository_snapshot_sha256: String,
    /// Stable exact changed paths.
    pub changed_paths: Vec<WorkspacePath>,
    /// Stable trusted coverage mappings.
    pub mappings: Vec<ValidationCoverageMapping>,
    /// Stable exact requested validation kinds.
    pub requested_kinds: Vec<ValidationKind>,
    /// Select only pre-registered focused templates.
    pub focused_only: bool,
    /// Stop selection execution after the first failed template.
    pub fail_fast: bool,
}

/// One selected exact validation template.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedValidationTemplate {
    /// Exact validation identity.
    pub validation_id: String,
    /// Exact template digest.
    pub template_sha256: String,
    /// Closed validation kind.
    pub kind: ValidationKind,
    /// Whether this exact registered template is focused.
    pub focused: bool,
}

/// One requested kind not selected and therefore explicitly unverified.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnverifiedValidationKind {
    /// Requested kind.
    pub kind: ValidationKind,
    /// Stable content-free reason.
    pub reason_code: String,
}

/// Deterministic validation selection carrying no command authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidationSelection {
    /// Schema version.
    pub schema_version: u16,
    /// Stable selection identity.
    pub selection_id: String,
    /// Exact registry identity.
    pub registry_sha256: String,
    /// Exact repository snapshot identity.
    pub repository_snapshot_sha256: String,
    /// Exact changed-path set identity.
    pub changed_paths_sha256: String,
    /// Stable selected templates.
    pub selected: Vec<SelectedValidationTemplate>,
    /// Every unselected requested kind remains explicit.
    pub unverified: Vec<UnverifiedValidationKind>,
    /// Whether later execution should stop after first failure.
    pub fail_fast: bool,
    /// Each selected template requires its own approval.
    pub separate_approval_per_template: bool,
    /// Selection carries no execution authority.
    pub execution_authority: bool,
    /// SHA-256 over every preceding field.
    pub selection_sha256: String,
}

/// Stable content-free validation-template or selection failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationTemplateError {
    /// Syntax, bounds, identity, ordering, or provenance is invalid.
    InvalidInput,
    /// Registry contains a duplicate or invalid template.
    DuplicateOrInvalid,
    /// Requested registry or mapping is stale or inconsistent.
    StaleEvidence,
}

impl ValidationTemplateError {
    /// Stable content-free code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "validation-template.input-invalid",
            Self::DuplicateOrInvalid => "validation-template.duplicate-or-invalid",
            Self::StaleEvidence => "validation-template.evidence-stale",
        }
    }
}

/// Seals one exact validation template from trusted configuration or explicit user input.
pub fn seal_validation_template(
    input: ValidationTemplateInput,
) -> Result<ValidationTemplate, ValidationTemplateError> {
    input
        .command
        .verify()
        .map_err(|_| ValidationTemplateError::InvalidInput)?;
    if !valid_identifier(&input.validation_id)
        || !is_sha256(&input.source_sha256)
        || !is_sha256(&input.execution_scope_sha256)
        || !valid_version(&input.parser_version)
        || !is_sha256(&input.parser_sha256)
        || !matches!(
            input.command.working_directory,
            CommandWorkingDirectory::EmptyScratch | CommandWorkingDirectory::OwnedWorktree
        )
        || input.command.network
        || !input.command.grant_required
        || input.command.interactive
        || input.command.inherit_environment
        || input.expected_artifacts.len() > MAX_ARTIFACTS
        || input
            .expected_artifacts
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        || !artifacts_share_workspace(&input.expected_artifacts, input.source_path.as_ref())
        || input
            .expected_artifacts
            .iter()
            .any(|artifact| !valid_identifier(&artifact.artifact_id) || artifact.maximum_bytes == 0)
        || (input.kind.is_test() && input.minimum_test_count == 0)
        || (input.kind.is_test() && input.parser != ValidationParserKind::AgentMageJsonV1)
        || (!input.kind.is_test() && input.minimum_test_count != 0)
        || (input.failed_test_rerun_allowed && !input.setup_idempotent)
        || (input.focused && !input.kind.is_test())
        || match input.source {
            ValidationTemplateSource::TrustedProjectConfiguration => {
                input.source_path.is_none() || input.user_input_approval_sha256.is_some()
            }
            ValidationTemplateSource::ExplicitUserInput => {
                input.source_path.is_some()
                    || input
                        .user_input_approval_sha256
                        .as_deref()
                        .is_none_or(|value| !is_sha256(value))
            }
        }
    {
        return Err(ValidationTemplateError::InvalidInput);
    }
    let mut template = ValidationTemplate {
        schema_version: VALIDATION_TEMPLATE_SCHEMA_VERSION,
        validation_id: input.validation_id,
        kind: input.kind,
        command: input.command,
        source: input.source,
        source_sha256: input.source_sha256,
        source_path: input.source_path,
        user_input_approval_sha256: input.user_input_approval_sha256,
        execution_scope_sha256: input.execution_scope_sha256,
        parser: input.parser,
        parser_version: input.parser_version,
        parser_sha256: input.parser_sha256,
        minimum_test_count: input.minimum_test_count,
        focused: input.focused,
        fail_fast: input.fail_fast,
        failed_test_rerun_allowed: input.failed_test_rerun_allowed,
        setup_idempotent: input.setup_idempotent,
        expected_artifacts: input.expected_artifacts,
        separate_approval_required: true,
        execution_authority: false,
        template_sha256: String::new(),
    };
    template.template_sha256 = template_digest(&template);
    Ok(template)
}

/// Verifies one complete template and exact digest.
#[must_use]
pub fn verify_validation_template(template: &ValidationTemplate) -> bool {
    let input = ValidationTemplateInput {
        validation_id: template.validation_id.clone(),
        kind: template.kind,
        command: template.command.clone(),
        source: template.source,
        source_sha256: template.source_sha256.clone(),
        source_path: template.source_path.clone(),
        user_input_approval_sha256: template.user_input_approval_sha256.clone(),
        execution_scope_sha256: template.execution_scope_sha256.clone(),
        parser: template.parser,
        parser_version: template.parser_version.clone(),
        parser_sha256: template.parser_sha256.clone(),
        minimum_test_count: template.minimum_test_count,
        focused: template.focused,
        fail_fast: template.fail_fast,
        failed_test_rerun_allowed: template.failed_test_rerun_allowed,
        setup_idempotent: template.setup_idempotent,
        expected_artifacts: template.expected_artifacts.clone(),
    };
    template.schema_version == VALIDATION_TEMPLATE_SCHEMA_VERSION
        && template.separate_approval_required
        && !template.execution_authority
        && template.template_sha256 == template_digest(template)
        && seal_validation_template(input).is_ok_and(|expected| expected == *template)
}

/// Selects exact pre-registered validation templates from trusted source mappings.
pub fn select_validations(
    registry: &ValidationTemplateRegistry,
    request: ValidationSelectionRequest,
) -> Result<ValidationSelection, ValidationTemplateError> {
    if !registry.verify()
        || request.registry_sha256 != registry.registry_sha256
        || !valid_identifier(&request.selection_id)
        || !is_sha256(&request.repository_snapshot_sha256)
        || request.changed_paths.is_empty()
        || request.changed_paths.len() > MAX_CHANGED_PATHS
        || request
            .changed_paths
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        || request.mappings.is_empty()
        || request.mappings.len() > MAX_MAPPINGS
        || request
            .mappings
            .windows(2)
            .any(|pair| pair[0].changed_path >= pair[1].changed_path)
        || request.requested_kinds.is_empty()
        || request
            .requested_kinds
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err(ValidationTemplateError::InvalidInput);
    }
    let changed = request.changed_paths.iter().collect::<BTreeSet<_>>();
    let mut candidates = BTreeSet::new();
    for mapping in &request.mappings {
        if !changed.contains(&mapping.changed_path)
            || !is_sha256(&mapping.evidence_sha256)
            || mapping.validation_ids.is_empty()
            || mapping
                .validation_ids
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
        {
            return Err(ValidationTemplateError::StaleEvidence);
        }
        candidates.extend(mapping.validation_ids.iter().cloned());
    }
    let requested = request
        .requested_kinds
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let by_id = registry
        .templates
        .iter()
        .map(|template| (template.validation_id.as_str(), template))
        .collect::<BTreeMap<_, _>>();
    let mut selected = Vec::new();
    for validation_id in candidates {
        let Some(template) = by_id.get(validation_id.as_str()) else {
            return Err(ValidationTemplateError::StaleEvidence);
        };
        if requested.contains(&template.kind) && (!request.focused_only || template.focused) {
            selected.push(SelectedValidationTemplate {
                validation_id: template.validation_id.clone(),
                template_sha256: template.template_sha256.clone(),
                kind: template.kind,
                focused: template.focused,
            });
        }
    }
    selected.sort_by(|left, right| left.validation_id.cmp(&right.validation_id));
    let selected_kinds = selected
        .iter()
        .map(|entry| entry.kind)
        .collect::<BTreeSet<_>>();
    let unverified = request
        .requested_kinds
        .iter()
        .filter(|kind| !selected_kinds.contains(kind))
        .map(|kind| UnverifiedValidationKind {
            kind: *kind,
            reason_code: if request.focused_only {
                "validation.selection.focused-template-unavailable".to_owned()
            } else {
                "validation.selection.mapping-unavailable".to_owned()
            },
        })
        .collect::<Vec<_>>();
    let mut selection = ValidationSelection {
        schema_version: VALIDATION_TEMPLATE_SCHEMA_VERSION,
        selection_id: request.selection_id,
        registry_sha256: request.registry_sha256,
        repository_snapshot_sha256: request.repository_snapshot_sha256,
        changed_paths_sha256: sha256_json(&request.changed_paths),
        selected,
        unverified,
        fail_fast: request.fail_fast,
        separate_approval_per_template: true,
        execution_authority: false,
        selection_sha256: String::new(),
    };
    selection.selection_sha256 = selection_digest(&selection);
    Ok(selection)
}

/// Verifies a selection against the current registry without widening it.
#[must_use]
pub fn verify_validation_selection(
    registry: &ValidationTemplateRegistry,
    selection: &ValidationSelection,
) -> bool {
    let selected_kinds = selection
        .selected
        .iter()
        .map(|entry| entry.kind)
        .collect::<BTreeSet<_>>();
    selection.schema_version == VALIDATION_TEMPLATE_SCHEMA_VERSION
        && valid_identifier(&selection.selection_id)
        && registry.verify()
        && selection.registry_sha256 == registry.registry_sha256
        && is_sha256(&selection.repository_snapshot_sha256)
        && is_sha256(&selection.changed_paths_sha256)
        && selection
            .selected
            .windows(2)
            .all(|pair| pair[0].validation_id < pair[1].validation_id)
        && selection.selected.iter().all(|entry| {
            registry
                .resolve(&entry.validation_id, &entry.template_sha256)
                .is_some_and(|template| {
                    template.kind == entry.kind && template.focused == entry.focused
                })
        })
        && selection
            .unverified
            .windows(2)
            .all(|pair| pair[0].kind < pair[1].kind)
        && selection.unverified.iter().all(|entry| {
            valid_identifier(&entry.reason_code) && !selected_kinds.contains(&entry.kind)
        })
        && selection.separate_approval_per_template
        && !selection.execution_authority
        && selection.selection_sha256 == selection_digest(selection)
}

fn artifacts_share_workspace(
    artifacts: &[ValidationArtifactExpectation],
    source_path: Option<&WorkspacePath>,
) -> bool {
    let expected_workspace = source_path.map(WorkspacePath::workspace_id).or_else(|| {
        artifacts
            .first()
            .map(|artifact| artifact.path.workspace_id())
    });
    artifacts.iter().all(|artifact| {
        expected_workspace.is_none_or(|workspace| artifact.path.workspace_id() == workspace)
    })
}

fn template_digest(template: &ValidationTemplate) -> String {
    let mut canonical = template.clone();
    canonical.template_sha256.clear();
    sha256_json(&canonical)
}

fn selection_digest(selection: &ValidationSelection) -> String {
    let mut canonical = selection.clone();
    canonical.selection_sha256.clear();
    sha256_json(&canonical)
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn valid_version(value: &str) -> bool {
    let parts = value.split('.').collect::<Vec<_>>();
    value.len() <= 64
        && parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
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
        .unwrap_or_else(|_| sha256_hex(b"validation-template-serialization-failed"))
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
    use std::collections::BTreeMap;

    use agentmage_kernel_contracts::WorkspaceId;

    use crate::command_runner::{CommandBounds, CommandRisk};

    use super::*;

    fn path(parts: &[&str]) -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-validation"),
            parts.iter().copied(),
        )
        .expect("path")
    }

    fn other_path(parts: &[&str]) -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-other"),
            parts.iter().copied(),
        )
        .expect("path")
    }

    fn command(id: &str) -> CommandSpec {
        CommandSpec::seal(
            format!("command-{id}"),
            "1.0.0",
            "/usr/bin/cargo",
            "1".repeat(64),
            vec!["test".to_owned(), format!("fixture-{id}")],
            CommandWorkingDirectory::EmptyScratch,
            BTreeMap::from([
                ("LANG".to_owned(), "C".to_owned()),
                ("NO_COLOR".to_owned(), "1".to_owned()),
                ("TZ".to_owned(), "UTC".to_owned()),
            ]),
            CommandRisk::Moderate,
            CommandBounds::new(30_000, 65_536, 65_536, 256 * 1024 * 1024, 32, 200).expect("bounds"),
        )
        .expect("command")
    }

    fn input(id: &str, kind: ValidationKind) -> ValidationTemplateInput {
        ValidationTemplateInput {
            validation_id: format!("validation-{id}"),
            kind,
            command: command(id),
            source: ValidationTemplateSource::TrustedProjectConfiguration,
            source_sha256: "2".repeat(64),
            source_path: Some(path(&["Cargo.toml"])),
            user_input_approval_sha256: None,
            execution_scope_sha256: "3".repeat(64),
            parser: if kind.is_test() {
                ValidationParserKind::AgentMageJsonV1
            } else {
                ValidationParserKind::ProcessExitV1
            },
            parser_version: "1.0.0".to_owned(),
            parser_sha256: "4".repeat(64),
            minimum_test_count: u32::from(kind.is_test()),
            focused: kind.is_test(),
            fail_fast: true,
            failed_test_rerun_allowed: kind.is_test(),
            setup_idempotent: true,
            expected_artifacts: Vec::new(),
        }
    }

    #[test]
    fn all_validation_kinds_register_with_exact_command_scope_parser_and_provenance() {
        let templates = ValidationKind::ALL
            .into_iter()
            .enumerate()
            .map(|(index, kind)| {
                let mut input = input(&format!("{index:02}"), kind);
                input.focused = kind.is_test();
                seal_validation_template(input).expect("template")
            })
            .collect::<Vec<_>>();
        let registry = ValidationTemplateRegistry::build(templates).expect("registry");
        assert!(registry.verify());
        assert_eq!(registry.templates.len(), ValidationKind::ALL.len());
        assert!(registry.templates.iter().all(|template| {
            template.separate_approval_required
                && !template.execution_authority
                && !template.command.network
                && template.command.grant_required
        }));
    }

    #[test]
    fn substitutions_unknown_flags_untrusted_sources_and_unsafe_reruns_fail_closed() {
        let baseline =
            seal_validation_template(input("unit", ValidationKind::Unit)).expect("template");
        let mut argument = baseline.clone();
        argument.command.arguments.push("--unknown".to_owned());
        assert!(!verify_validation_template(&argument));

        let mut executable = baseline.clone();
        executable.command.executable_sha256 = "9".repeat(64);
        assert!(!verify_validation_template(&executable));

        let mut user = input("user", ValidationKind::Lint);
        user.source = ValidationTemplateSource::ExplicitUserInput;
        user.source_path = None;
        user.user_input_approval_sha256 = None;
        assert_eq!(
            seal_validation_template(user),
            Err(ValidationTemplateError::InvalidInput)
        );

        let mut retry = input("retry", ValidationKind::Unit);
        retry.setup_idempotent = false;
        assert_eq!(
            seal_validation_template(retry),
            Err(ValidationTemplateError::InvalidInput)
        );

        let mut parser = input("parser", ValidationKind::Unit);
        parser.parser = ValidationParserKind::ProcessExitV1;
        assert_eq!(
            seal_validation_template(parser),
            Err(ValidationTemplateError::InvalidInput)
        );

        let mut artifact = input("artifact", ValidationKind::Build);
        artifact.expected_artifacts = vec![ValidationArtifactExpectation {
            artifact_id: "build-output".to_owned(),
            path: other_path(&["target", "binary"]),
            required: true,
            maximum_bytes: 1024,
        }];
        assert_eq!(
            seal_validation_template(artifact),
            Err(ValidationTemplateError::InvalidInput)
        );
    }

    #[test]
    fn focused_selection_uses_only_exact_mapped_templates_and_marks_missing_kinds_unverified() {
        let unit = seal_validation_template(input("unit", ValidationKind::Unit)).expect("unit");
        let mut lint_input = input("lint", ValidationKind::Lint);
        lint_input.focused = false;
        let lint = seal_validation_template(lint_input).expect("lint");
        let registry = ValidationTemplateRegistry::build(vec![unit, lint]).expect("registry");
        let changed = path(&["src", "lib.rs"]);
        let selection = select_validations(
            &registry,
            ValidationSelectionRequest {
                selection_id: "selection-focused".to_owned(),
                registry_sha256: registry.registry_sha256.clone(),
                repository_snapshot_sha256: "5".repeat(64),
                changed_paths: vec![changed.clone()],
                mappings: vec![ValidationCoverageMapping {
                    changed_path: changed,
                    validation_ids: vec![
                        "validation-lint".to_owned(),
                        "validation-unit".to_owned(),
                    ],
                    evidence_sha256: "6".repeat(64),
                }],
                requested_kinds: vec![ValidationKind::Unit, ValidationKind::Lint],
                focused_only: true,
                fail_fast: true,
            },
        )
        .expect("selection");
        assert!(verify_validation_selection(&registry, &selection));
        assert_eq!(selection.selected.len(), 1);
        assert_eq!(selection.selected[0].kind, ValidationKind::Unit);
        assert_eq!(selection.unverified.len(), 1);
        assert_eq!(selection.unverified[0].kind, ValidationKind::Lint);
        assert!(selection.separate_approval_per_template);
        assert!(!selection.execution_authority);

        let mut contradictory = selection.clone();
        contradictory.unverified[0].kind = ValidationKind::Unit;
        contradictory.selection_sha256 = selection_digest(&contradictory);
        assert!(!verify_validation_selection(&registry, &contradictory));

        let mut invalid_identity = selection;
        invalid_identity.selection_id = "selection with spaces".to_owned();
        invalid_identity.selection_sha256 = selection_digest(&invalid_identity);
        assert!(!verify_validation_selection(&registry, &invalid_identity));
    }
}
