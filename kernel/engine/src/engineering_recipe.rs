//! Versioned engineering recipes (Decision 0126).
//!
//! A recipe is a versioned, parameterized manifest for a recurring engineering
//! change: a dependency update, a migration, a security repair, documentation
//! or tests. It declares its typed parameters, the workspace paths it may
//! change, its prerequisites, the kinds of registered validation that verify
//! it and its rollback boundary. Admission is closed: unknown fields, kinds,
//! types and out-of-bound values are refused, and each kind must declare what
//! that kind of change needs. Instantiating an admitted recipe with parameter
//! values and the workspace's validation registry yields a plan with an exact
//! digest. A plan is only a proposal: it holds no write, command or network
//! authority, and applying it goes through the ordinary change approval,
//! validation and grant paths.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath, WorkspaceScopePath};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::persistence::detect_secret_classes;
use crate::validation_template::{ValidationKind, ValidationTemplateRegistry};

const SCHEMA_VERSION: u16 = 1;
const MAX_IDENTIFIER_BYTES: usize = 64;
const MAX_LABEL_BYTES: usize = 200;
const MAX_PARAMETERS: usize = 16;
const MAX_CHOICES: usize = 32;
const MAX_SCOPE_ENTRIES: usize = 64;
const MAX_CHANGED_FILES: u32 = 1_000;
const MAX_PREREQUISITES: usize = 16;
const MAX_VERSION_PARTS: usize = 4;
const MAX_VERSION_PART_DIGITS: usize = 9;
const MAX_PATH_BYTES: usize = 1_024;
const SCOPE_CHECK_WORKSPACE: &str = "recipe-scope-check";

/// Semantic version of a recipe.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeVersion {
    /// Major version.
    pub major: u32,
    /// Minor version.
    pub minor: u32,
    /// Patch version.
    pub patch: u32,
}

/// Closed recipe kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeKind {
    /// Update declared dependencies within the locked dependency set.
    DependencyUpdate,
    /// Add or change migration files.
    Migration,
    /// Repair a known security weakness.
    SecurityRepair,
    /// Change documentation.
    Documentation,
    /// Add or change tests.
    Tests,
}

/// Closed parameter type.
///
/// A variant without members has an empty member list, so decoding refuses a
/// member beside its tag (review F1 of `84e531fb`); its encoding is the tag
/// alone.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum RecipeParameterType {
    /// One of a sorted set of plain values.
    Choice {
        /// The accepted values.
        values: Vec<String>,
    },
    /// An integer within an inclusive range.
    Integer {
        /// Least accepted value.
        minimum: i64,
        /// Greatest accepted value.
        maximum: i64,
    },
    /// A workspace-relative path inside the recipe's scope.
    Path {},
    /// Dot-separated decimal numbers, such as a dependency version.
    Version {},
}

/// One declared parameter.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeParameter {
    /// Plain parameter name.
    pub name: String,
    /// What the parameter means, shown to the person.
    pub summary: String,
    /// Its type.
    pub parameter_type: RecipeParameterType,
    /// Whether instantiation needs a value.
    pub required: bool,
}

/// Closed prerequisite a host checks before applying a plan.
///
/// A variant without members has an empty member list, so decoding refuses a
/// member beside its tag (review F1 of `84e531fb`); its encoding is the tag
/// alone.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RecipePrerequisite {
    /// The worktree has no uncommitted change.
    CleanWorktree {},
    /// Dependencies are resolved from a lock file.
    LockedDependencies {},
    /// A registered tool is available.
    Tool {
        /// Plain tool identity.
        tool_id: String,
    },
}

/// What can be undone after a plan is applied.
///
/// A variant without members has an empty member list, so decoding refuses a
/// member beside its tag (review F1 of `84e531fb`); its encoding is the tag
/// alone.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "boundary", rename_all = "snake_case", deny_unknown_fields)]
pub enum RecipeRollback {
    /// Only the plan's own file writes, each by an inverse write that needs
    /// a fresh approval.
    RevertWritesOnly {},
    /// Nothing can be undone.
    Irreversible {
        /// Plain reason code.
        reason_code: String,
    },
}

/// Sealed recipe manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeManifest {
    /// Manifest schema version.
    pub schema_version: u16,
    /// Plain recipe identity.
    pub recipe_id: String,
    /// Recipe version.
    pub version: RecipeVersion,
    /// Closed kind.
    pub kind: RecipeKind,
    /// Title shown to the person.
    pub title: String,
    /// Parameters sorted by name.
    pub parameters: Vec<RecipeParameter>,
    /// Workspace-relative path prefixes the plan may change, sorted.
    pub scope: Vec<String>,
    /// Most files one plan may change.
    pub max_changed_files: u32,
    /// Prerequisites, sorted.
    pub prerequisites: Vec<RecipePrerequisite>,
    /// Validation kinds that verify the change, sorted.
    pub verification: Vec<ValidationKind>,
    /// Rollback boundary.
    pub rollback: RecipeRollback,
    /// Whether applying needs a separate network grant, which the recipe
    /// never holds.
    pub needs_network_grant: bool,
    /// SHA-256 of the canonical manifest with this field empty.
    pub manifest_sha256: String,
}

/// A parameter value supplied at instantiation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RecipeValue {
    /// A choice.
    Choice(String),
    /// An integer.
    Integer(i64),
    /// A `/`-separated workspace-relative path.
    Path(String),
    /// A version.
    Version(String),
}

/// One registered validation a plan binds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipePlanValidation {
    /// Validation identity.
    pub validation_id: String,
    /// Its kind.
    pub kind: ValidationKind,
    /// Its exact template digest.
    pub template_sha256: String,
}

/// An instantiated recipe: a proposal with an exact digest. A host hands it
/// to its client in a run's declarations (Decision 0133), so it also decodes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipePlan {
    /// Plan schema version.
    pub schema_version: u16,
    /// Recipe identity.
    pub recipe_id: String,
    /// Recipe version.
    pub recipe_version: RecipeVersion,
    /// Recipe kind.
    pub recipe_kind: RecipeKind,
    /// Recipe manifest digest.
    pub manifest_sha256: String,
    /// Target workspace.
    pub workspace_id: WorkspaceId,
    /// Parameter values by name.
    pub parameters: BTreeMap<String, RecipeValue>,
    /// Paths the change may touch.
    pub scope: Vec<WorkspaceScopePath>,
    /// Most files the change may touch.
    pub max_changed_files: u32,
    /// Prerequisites to check before applying.
    pub prerequisites: Vec<RecipePrerequisite>,
    /// Every registered validation of each declared kind.
    pub validations: Vec<RecipePlanValidation>,
    /// Digest of the registry the validations come from.
    pub validation_registry_sha256: String,
    /// Rollback boundary.
    pub rollback: RecipeRollback,
    /// Whether a separate network grant is needed first.
    pub network_grant_required: bool,
    /// Fixed true: the plan proposes and holds no authority.
    pub proposal_only: bool,
    /// SHA-256 of this plan with this field empty.
    pub plan_sha256: String,
}

/// Content-free recipe failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecipeError {
    /// The manifest's shape, fields, bounds or seal are invalid.
    ManifestInvalid,
    /// The manifest omits what its kind needs.
    KindRuleViolated,
    /// A value is unknown, missing, mistyped or out of bounds.
    ParameterInvalid,
    /// The validation registry is invalid.
    RegistryInvalid,
    /// A declared validation kind has no registered template.
    VerificationUnavailable,
    /// A change lies outside the plan's scope.
    OutOfScope,
    /// A change touches more files than the plan allows.
    TooManyChanges,
    /// The plan does not verify.
    PlanInvalid,
}

impl RecipeError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ManifestInvalid => "recipe.manifest-invalid",
            Self::KindRuleViolated => "recipe.kind-rule-violated",
            Self::ParameterInvalid => "recipe.parameter-invalid",
            Self::RegistryInvalid => "recipe.registry-invalid",
            Self::VerificationUnavailable => "recipe.verification-unavailable",
            Self::OutOfScope => "recipe.out-of-scope",
            Self::TooManyChanges => "recipe.too-many-changes",
            Self::PlanInvalid => "recipe.plan-invalid",
        }
    }
}

/// Seals a manifest after checking every field and its kind's rules.
pub fn seal_recipe_manifest(mut manifest: RecipeManifest) -> Result<RecipeManifest, RecipeError> {
    manifest.manifest_sha256.clear();
    validate_manifest(&manifest)?;
    manifest.manifest_sha256 = sha256_json(&manifest).map_err(|_| RecipeError::ManifestInvalid)?;
    Ok(manifest)
}

/// Verifies a sealed manifest: fields, kind rules and seal.
pub fn admit_recipe_manifest(manifest: &RecipeManifest) -> Result<(), RecipeError> {
    validate_manifest(manifest)?;
    let mut canonical = manifest.clone();
    canonical.manifest_sha256.clear();
    if sha256_json(&canonical).map_err(|_| RecipeError::ManifestInvalid)?
        != manifest.manifest_sha256
    {
        return Err(RecipeError::ManifestInvalid);
    }
    Ok(())
}

/// Parses and admits a manifest; unknown fields and variants and repeated
/// members are refused. The bytes are parsed into the manifest directly, which
/// refuses a repeated member (review F3 of `7c593b3b`), and the manifest must
/// serialize back to exactly the bytes' value, which refuses any member that
/// the types do not name.
pub fn parse_recipe_manifest(bytes: &[u8]) -> Result<RecipeManifest, RecipeError> {
    let invalid = RecipeError::ManifestInvalid;
    let manifest: RecipeManifest = serde_json::from_slice(bytes).map_err(|_| invalid)?;
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| invalid)?;
    if serde_json::to_value(&manifest).map_err(|_| invalid)? != value {
        return Err(invalid);
    }
    admit_recipe_manifest(&manifest)?;
    Ok(manifest)
}

fn validate_manifest(manifest: &RecipeManifest) -> Result<(), RecipeError> {
    let invalid = RecipeError::ManifestInvalid;
    if manifest.schema_version != SCHEMA_VERSION
        || !plain_identifier(&manifest.recipe_id, b".-")
        || !label(&manifest.title)
        || manifest.parameters.len() > MAX_PARAMETERS
        || manifest
            .parameters
            .windows(2)
            .any(|pair| pair[0].name >= pair[1].name)
        || manifest.scope.is_empty()
        || manifest.scope.len() > MAX_SCOPE_ENTRIES
        || manifest.scope.windows(2).any(|pair| pair[0] >= pair[1])
        || manifest.max_changed_files == 0
        || manifest.max_changed_files > MAX_CHANGED_FILES
        || manifest.prerequisites.len() > MAX_PREREQUISITES
        || manifest
            .prerequisites
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        || manifest.verification.is_empty()
        || manifest
            .verification
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err(invalid);
    }
    for parameter in &manifest.parameters {
        let type_valid = match &parameter.parameter_type {
            RecipeParameterType::Choice { values } => {
                !values.is_empty()
                    && values.len() <= MAX_CHOICES
                    && values.windows(2).all(|pair| pair[0] < pair[1])
                    && values.iter().all(|value| plain_identifier(value, b".-_"))
            }
            RecipeParameterType::Integer { minimum, maximum } => minimum <= maximum,
            RecipeParameterType::Path {} | RecipeParameterType::Version {} => true,
        };
        if !type_valid || !plain_identifier(&parameter.name, b"_") || !label(&parameter.summary) {
            return Err(invalid);
        }
    }
    for prefix in &manifest.scope {
        scope_path(&WorkspaceId::from_raw(SCOPE_CHECK_WORKSPACE), prefix).ok_or(invalid)?;
    }
    if manifest.prerequisites.iter().any(|prerequisite| {
        matches!(prerequisite, RecipePrerequisite::Tool { tool_id } if !plain_identifier(tool_id, b".-"))
    }) || matches!(&manifest.rollback, RecipeRollback::Irreversible { reason_code } if !plain_identifier(reason_code, b".-"))
    {
        return Err(invalid);
    }
    validate_kind_rules(manifest)
}

/// What each kind of change must declare.
fn validate_kind_rules(manifest: &RecipeManifest) -> Result<(), RecipeError> {
    let verifies = |kinds: &[ValidationKind]| {
        manifest
            .verification
            .iter()
            .any(|kind| kind.is_test() || kinds.contains(kind))
    };
    let clean = manifest
        .prerequisites
        .contains(&RecipePrerequisite::CleanWorktree {});
    let satisfied = match manifest.kind {
        RecipeKind::DependencyUpdate => {
            manifest
                .prerequisites
                .contains(&RecipePrerequisite::LockedDependencies {})
                && verifies(&[ValidationKind::Build])
        }
        RecipeKind::Migration => clean && verifies(&[]),
        RecipeKind::SecurityRepair => clean && verifies(&[ValidationKind::Security]),
        RecipeKind::Documentation => !manifest.needs_network_grant,
        RecipeKind::Tests => verifies(&[]),
    };
    if satisfied {
        Ok(())
    } else {
        Err(RecipeError::KindRuleViolated)
    }
}

/// Checks parameter values against an admitted manifest for one workspace,
/// without a validation registry: every value must name a declared parameter
/// of its type, within its bounds, and every required parameter must have
/// one. A client can check what it sends before a host instantiates the
/// recipe (Decision 0133).
pub fn check_recipe_parameters(
    manifest: &RecipeManifest,
    workspace_id: &WorkspaceId,
    values: &BTreeMap<String, RecipeValue>,
) -> Result<(), RecipeError> {
    admit_recipe_manifest(manifest)?;
    recipe_scope(manifest, workspace_id, values).map(|_| ())
}

/// Instantiates an admitted recipe for one workspace. Every value must name a
/// declared parameter of its type, and every declared validation kind must
/// have at least one registered template, all of which the plan binds.
pub fn instantiate_recipe(
    manifest: &RecipeManifest,
    workspace_id: &WorkspaceId,
    values: &BTreeMap<String, RecipeValue>,
    registry: &ValidationTemplateRegistry,
) -> Result<RecipePlan, RecipeError> {
    admit_recipe_manifest(manifest)?;
    if !registry.verify() {
        return Err(RecipeError::RegistryInvalid);
    }
    let scope = recipe_scope(manifest, workspace_id, values)?;
    let mut validations = Vec::new();
    for kind in &manifest.verification {
        let before = validations.len();
        validations.extend(
            registry
                .templates
                .iter()
                .filter(|template| template.kind == *kind)
                .map(|template| RecipePlanValidation {
                    validation_id: template.validation_id.clone(),
                    kind: template.kind,
                    template_sha256: template.template_sha256.clone(),
                }),
        );
        if validations.len() == before {
            return Err(RecipeError::VerificationUnavailable);
        }
    }
    let mut plan = RecipePlan {
        schema_version: SCHEMA_VERSION,
        recipe_id: manifest.recipe_id.clone(),
        recipe_version: manifest.version,
        recipe_kind: manifest.kind,
        manifest_sha256: manifest.manifest_sha256.clone(),
        workspace_id: workspace_id.clone(),
        parameters: values.clone(),
        scope,
        max_changed_files: manifest.max_changed_files,
        prerequisites: manifest.prerequisites.clone(),
        validations,
        validation_registry_sha256: registry.registry_sha256.clone(),
        rollback: manifest.rollback.clone(),
        network_grant_required: manifest.needs_network_grant,
        proposal_only: true,
        plan_sha256: String::new(),
    };
    plan.plan_sha256 = sha256_json(&plan).map_err(|_| RecipeError::PlanInvalid)?;
    Ok(plan)
}

/// The plan's scope for one workspace, once every value is checked against
/// its declared parameter.
fn recipe_scope(
    manifest: &RecipeManifest,
    workspace_id: &WorkspaceId,
    values: &BTreeMap<String, RecipeValue>,
) -> Result<Vec<WorkspaceScopePath>, RecipeError> {
    let scope = manifest
        .scope
        .iter()
        .map(|prefix| scope_path(workspace_id, prefix))
        .collect::<Option<Vec<_>>>()
        .ok_or(RecipeError::ParameterInvalid)?;
    let declared = manifest
        .parameters
        .iter()
        .map(|parameter| parameter.name.as_str())
        .collect::<BTreeSet<_>>();
    if values.keys().any(|name| !declared.contains(name.as_str())) {
        return Err(RecipeError::ParameterInvalid);
    }
    for parameter in &manifest.parameters {
        let Some(value) = values.get(&parameter.name) else {
            if parameter.required {
                return Err(RecipeError::ParameterInvalid);
            }
            continue;
        };
        let valid = match (&parameter.parameter_type, value) {
            (RecipeParameterType::Choice { values }, RecipeValue::Choice(choice)) => {
                values.contains(choice)
            }
            (RecipeParameterType::Integer { minimum, maximum }, RecipeValue::Integer(number)) => {
                (minimum..=maximum).contains(&number)
            }
            (RecipeParameterType::Path {}, RecipeValue::Path(path)) => {
                workspace_path(workspace_id, path)
                    .is_some_and(|path| scope.iter().any(|prefix| prefix.contains_path(&path)))
            }
            (RecipeParameterType::Version {}, RecipeValue::Version(version)) => {
                valid_version(version)
            }
            _ => false,
        };
        if !valid {
            return Err(RecipeError::ParameterInvalid);
        }
    }
    Ok(scope)
}

/// Verifies a plan's digest and fixed proposal marker.
#[must_use]
pub fn verify_recipe_plan(plan: &RecipePlan) -> bool {
    let mut canonical = plan.clone();
    canonical.plan_sha256.clear();
    plan.schema_version == SCHEMA_VERSION
        && plan.proposal_only
        && sha256_json(&canonical).is_ok_and(|digest| digest == plan.plan_sha256)
}

/// Checks that a proposed change stays inside the plan: each distinct path
/// within its scope, and no more paths than it allows.
pub fn check_recipe_changes(
    plan: &RecipePlan,
    changed_paths: &[WorkspacePath],
) -> Result<(), RecipeError> {
    if !verify_recipe_plan(plan) {
        return Err(RecipeError::PlanInvalid);
    }
    let distinct = changed_paths.iter().collect::<BTreeSet<_>>();
    if distinct
        .iter()
        .any(|path| !plan.scope.iter().any(|prefix| prefix.contains_path(path)))
    {
        return Err(RecipeError::OutOfScope);
    }
    if distinct.len() > usize::try_from(plan.max_changed_files).unwrap_or(usize::MAX) {
        return Err(RecipeError::TooManyChanges);
    }
    Ok(())
}

fn scope_path(workspace_id: &WorkspaceId, prefix: &str) -> Option<WorkspaceScopePath> {
    if prefix.is_empty() || prefix.len() > MAX_PATH_BYTES {
        return None;
    }
    WorkspaceScopePath::new(workspace_id.clone(), prefix.split('/')).ok()
}

fn workspace_path(workspace_id: &WorkspaceId, path: &str) -> Option<WorkspacePath> {
    if path.is_empty() || path.len() > MAX_PATH_BYTES {
        return None;
    }
    WorkspacePath::new(workspace_id.clone(), path.split('/')).ok()
}

/// A lowercase identifier starting with a letter; `extra` lists the other
/// allowed punctuation.
fn plain_identifier(value: &str, extra: &[u8]) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value.as_bytes()[0].is_ascii_lowercase()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || extra.contains(&byte))
        && detect_secret_classes("recipe-identifier", value.as_bytes()).is_empty()
}

fn label(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_LABEL_BYTES
        && !value.chars().any(char::is_control)
        && detect_secret_classes("recipe-label", value.as_bytes()).is_empty()
}

fn valid_version(value: &str) -> bool {
    let parts = value.split('.').collect::<Vec<_>>();
    !parts.is_empty()
        && parts.len() <= MAX_VERSION_PARTS
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.len() <= MAX_VERSION_PART_DIGITS
                && part.bytes().all(|byte| byte.is_ascii_digit())
        })
}

fn sha256_json(value: &impl Serialize) -> Result<String, serde_json::Error> {
    serde_json::to_vec(value).map(|bytes| {
        let mut output = String::with_capacity(64);
        for byte in Sha256::digest(bytes) {
            let _ = write!(output, "{byte:02x}");
        }
        output
    })
}

#[cfg(test)]
mod tests {
    use crate::command_runner::{CommandBounds, CommandRisk, CommandSpec, CommandWorkingDirectory};
    use crate::validation_template::{
        ValidationParserKind, ValidationTemplateInput, ValidationTemplateSource,
        seal_validation_template,
    };

    use super::*;

    fn template_input(id: &str, kind: ValidationKind) -> ValidationTemplateInput {
        ValidationTemplateInput {
            validation_id: format!("validation-{id}"),
            kind,
            command: CommandSpec::seal(
                format!("command-{id}"),
                "1.0.0",
                "/usr/bin/cargo",
                "1".repeat(64),
                vec!["test".to_owned()],
                CommandWorkingDirectory::EmptyScratch,
                BTreeMap::from([("LANG".to_owned(), "C".to_owned())]),
                CommandRisk::Moderate,
                CommandBounds::new(30_000, 65_536, 65_536, 256 * 1024 * 1024, 32, 200).unwrap(),
            )
            .unwrap(),
            source: ValidationTemplateSource::TrustedProjectConfiguration,
            source_sha256: "2".repeat(64),
            source_path: Some(
                WorkspacePath::new(WorkspaceId::from_raw("workspace-recipe"), ["Cargo.toml"])
                    .unwrap(),
            ),
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

    fn registry() -> ValidationTemplateRegistry {
        ValidationTemplateRegistry::build(vec![
            seal_validation_template(template_input("build", ValidationKind::Build)).unwrap(),
            seal_validation_template(template_input("unit-a", ValidationKind::Unit)).unwrap(),
            seal_validation_template(template_input("unit-b", ValidationKind::Unit)).unwrap(),
        ])
        .unwrap()
    }

    fn workspace() -> WorkspaceId {
        WorkspaceId::from_raw("workspace-recipe")
    }

    fn draft() -> RecipeManifest {
        RecipeManifest {
            schema_version: SCHEMA_VERSION,
            recipe_id: "update-crate-pin".to_owned(),
            version: RecipeVersion {
                major: 1,
                minor: 2,
                patch: 0,
            },
            kind: RecipeKind::DependencyUpdate,
            title: "Update one pinned crate".to_owned(),
            parameters: vec![
                RecipeParameter {
                    name: "channel".to_owned(),
                    summary: "Release channel".to_owned(),
                    parameter_type: RecipeParameterType::Choice {
                        values: vec!["beta".to_owned(), "stable".to_owned()],
                    },
                    required: false,
                },
                RecipeParameter {
                    name: "manifest".to_owned(),
                    summary: "Manifest to change".to_owned(),
                    parameter_type: RecipeParameterType::Path {},
                    required: true,
                },
                RecipeParameter {
                    name: "retries".to_owned(),
                    summary: "Resolution attempts".to_owned(),
                    parameter_type: RecipeParameterType::Integer {
                        minimum: 1,
                        maximum: 3,
                    },
                    required: false,
                },
                RecipeParameter {
                    name: "target_version".to_owned(),
                    summary: "Version to pin".to_owned(),
                    parameter_type: RecipeParameterType::Version {},
                    required: true,
                },
            ],
            scope: vec!["Cargo.lock".to_owned(), "crates/core".to_owned()],
            max_changed_files: 2,
            prerequisites: vec![
                RecipePrerequisite::CleanWorktree {},
                RecipePrerequisite::LockedDependencies {},
                RecipePrerequisite::Tool {
                    tool_id: "cargo".to_owned(),
                },
            ],
            verification: vec![ValidationKind::Unit, ValidationKind::Build],
            rollback: RecipeRollback::RevertWritesOnly {},
            needs_network_grant: true,
            manifest_sha256: String::new(),
        }
    }

    type ManifestChange = Box<dyn Fn(&mut RecipeManifest)>;
    type ValuesChange<'a> = &'a dyn Fn(&mut BTreeMap<String, RecipeValue>);

    fn manifest() -> RecipeManifest {
        seal_recipe_manifest(draft()).unwrap()
    }

    fn values() -> BTreeMap<String, RecipeValue> {
        BTreeMap::from([
            (
                "manifest".to_owned(),
                RecipeValue::Path("crates/core/Cargo.toml".to_owned()),
            ),
            (
                "target_version".to_owned(),
                RecipeValue::Version("1.4.2".to_owned()),
            ),
            ("retries".to_owned(), RecipeValue::Integer(3)),
        ])
    }

    fn path(value: &str) -> WorkspacePath {
        WorkspacePath::new(workspace(), value.split('/')).unwrap()
    }

    #[test]
    fn a_manifest_is_admitted_only_with_closed_bounded_fields_and_its_seal() {
        let manifest = manifest();
        assert_eq!(admit_recipe_manifest(&manifest), Ok(()));
        let changes: Vec<ManifestChange> = vec![
            Box::new(|value| value.schema_version = 2),
            Box::new(|value| value.recipe_id = "Update".to_owned()),
            Box::new(|value| value.title = " ".to_owned()),
            Box::new(|value| value.title = "Bearer am-recipe-canary-token".to_owned()),
            Box::new(|value| value.parameters.swap(0, 1)),
            Box::new(|value| value.parameters[1].name = "Manifest".to_owned()),
            Box::new(|value| value.parameters[0].summary = "line\nbreak".to_owned()),
            Box::new(|value| {
                value.parameters[0].parameter_type = RecipeParameterType::Choice {
                    values: vec!["stable".to_owned(), "beta".to_owned()],
                };
            }),
            Box::new(|value| {
                value.parameters[0].parameter_type =
                    RecipeParameterType::Choice { values: Vec::new() };
            }),
            Box::new(|value| {
                value.parameters[2].parameter_type = RecipeParameterType::Integer {
                    minimum: 4,
                    maximum: 3,
                };
            }),
            Box::new(|value| value.scope.clear()),
            Box::new(|value| value.scope = vec!["crates/../secrets".to_owned()]),
            Box::new(|value| value.scope = vec!["/etc".to_owned()]),
            Box::new(|value| value.scope = vec!["b".to_owned(), "a".to_owned()]),
            Box::new(|value| value.max_changed_files = 0),
            Box::new(|value| value.max_changed_files = MAX_CHANGED_FILES + 1),
            Box::new(|value| value.prerequisites.reverse()),
            Box::new(|value| {
                value.prerequisites[2] = RecipePrerequisite::Tool {
                    tool_id: "Cargo".to_owned(),
                };
            }),
            Box::new(|value| value.verification.clear()),
            Box::new(|value| value.verification.reverse()),
            Box::new(|value| {
                value.rollback = RecipeRollback::Irreversible {
                    reason_code: "Applied Outside".to_owned(),
                };
            }),
        ];
        for change in changes {
            let mut value = draft();
            change(&mut value);
            assert_eq!(
                seal_recipe_manifest(value),
                Err(RecipeError::ManifestInvalid)
            );
        }
        let mut tampered = manifest.clone();
        tampered.max_changed_files = 3;
        assert_eq!(
            admit_recipe_manifest(&tampered),
            Err(RecipeError::ManifestInvalid)
        );
        // Manifest files are parsed closed.
        let bytes = serde_json::to_vec(&manifest).unwrap();
        assert_eq!(parse_recipe_manifest(&bytes), Ok(manifest.clone()));
        let text = String::from_utf8(bytes).unwrap();
        for invalid in [
            text.replacen("\"recipe_id\"", "\"run_after\":true,\"recipe_id\"", 1),
            text.replacen("\"required\":false", "\"required\":false,\"default\":1", 1),
            text.replacen("\"dependency_update\"", "\"upgrade_everything\"", 1),
            text.replacen(
                "{\"type\":\"path\"}",
                "{\"type\":\"path\",\"root\":\"/\"}",
                1,
            ),
            text.replacen("{\"type\":\"path\"}", "{\"type\":\"shell\"}", 1),
            text.replacen(
                "{\"kind\":\"clean_worktree\"}",
                "{\"kind\":\"clean_worktree\",\"x\":1}",
                1,
            ),
            // Review F3 of `7c593b3b`: a repeated member, a repeated member
            // beside a tag without fields and a repeated tag.
            text.replacen(
                "\"max_changed_files\":2",
                "\"max_changed_files\":5,\"max_changed_files\":2",
                1,
            ),
            text.replacen(
                "{\"kind\":\"clean_worktree\"}",
                "{\"kind\":\"clean_worktree\",\"x\":1,\"x\":2}",
                1,
            ),
            text.replacen(
                "{\"kind\":\"clean_worktree\"}",
                "{\"kind\":\"clean_worktree\",\"kind\":\"clean_worktree\"}",
                1,
            ),
        ] {
            assert_ne!(invalid, text);
            assert_eq!(
                parse_recipe_manifest(invalid.as_bytes()),
                Err(RecipeError::ManifestInvalid),
                "{invalid}"
            );
        }
    }

    #[test]
    fn each_kind_declares_what_its_change_needs() {
        let rule = |change: &dyn Fn(&mut RecipeManifest)| {
            let mut value = draft();
            change(&mut value);
            seal_recipe_manifest(value).map(|_| ())
        };
        let accepted: [&dyn Fn(&mut RecipeManifest); 5] = [
            &|_| {},
            &|value| value.kind = RecipeKind::Migration,
            &|value| value.kind = RecipeKind::SecurityRepair,
            &|value| value.kind = RecipeKind::Tests,
            &|value| {
                value.kind = RecipeKind::Documentation;
                value.needs_network_grant = false;
            },
        ];
        for change in accepted {
            assert_eq!(rule(change), Ok(()));
        }
        let refused: [&dyn Fn(&mut RecipeManifest); 8] = [
            // A dependency update needs locked dependencies and a build or test.
            &|value| {
                value.prerequisites.remove(1);
            },
            &|value| value.verification = vec![ValidationKind::Lint],
            // A migration and a security repair need a clean worktree.
            &|value| {
                value.kind = RecipeKind::Migration;
                value.prerequisites.remove(0);
            },
            &|value| {
                value.kind = RecipeKind::SecurityRepair;
                value.prerequisites.remove(0);
            },
            // Each needs its kind of verification.
            &|value| {
                value.kind = RecipeKind::Migration;
                value.verification = vec![ValidationKind::Build];
            },
            &|value| {
                value.kind = RecipeKind::SecurityRepair;
                value.verification = vec![ValidationKind::Lint];
            },
            &|value| {
                value.kind = RecipeKind::Tests;
                value.verification = vec![ValidationKind::Build];
            },
            // Documentation never needs the network.
            &|value| value.kind = RecipeKind::Documentation,
        ];
        for change in refused {
            assert_eq!(rule(change), Err(RecipeError::KindRuleViolated));
        }
        let security = rule(&|value| {
            value.kind = RecipeKind::SecurityRepair;
            value.verification = vec![ValidationKind::Security];
        });
        assert_eq!(security, Ok(()));
    }

    #[test]
    fn instantiation_binds_values_scope_and_every_registered_validation_to_an_exact_plan() {
        let registry = registry();
        let plan = instantiate_recipe(&manifest(), &workspace(), &values(), &registry).unwrap();
        assert!(verify_recipe_plan(&plan));
        assert!(plan.proposal_only && plan.network_grant_required);
        assert_eq!(plan.manifest_sha256, manifest().manifest_sha256);
        assert_eq!(plan.validation_registry_sha256, registry.registry_sha256);
        assert_eq!(
            plan.validations
                .iter()
                .map(|validation| (validation.validation_id.as_str(), validation.kind))
                .collect::<Vec<_>>(),
            vec![
                ("validation-unit-a", ValidationKind::Unit),
                ("validation-unit-b", ValidationKind::Unit),
                ("validation-build", ValidationKind::Build),
            ]
        );
        // The same inputs give the same plan; other values another digest.
        assert_eq!(
            instantiate_recipe(&manifest(), &workspace(), &values(), &registry),
            Ok(plan.clone())
        );
        let mut other = values();
        other.insert("channel".to_owned(), RecipeValue::Choice("beta".to_owned()));
        assert_ne!(
            instantiate_recipe(&manifest(), &workspace(), &other, &registry)
                .unwrap()
                .plan_sha256,
            plan.plan_sha256
        );
        let changes: [&dyn Fn(&mut RecipePlan); 4] = [
            &|value| value.proposal_only = false,
            &|value| value.max_changed_files = 99,
            &|value| value.validations.pop().map(|_| ()).unwrap(),
            &|value| value.network_grant_required = false,
        ];
        for change in changes {
            let mut tampered = plan.clone();
            change(&mut tampered);
            assert!(!verify_recipe_plan(&tampered));
        }
        // A consistent digest does not make a plan with authority verify.
        let mut authoritative = plan.clone();
        authoritative.proposal_only = false;
        authoritative.plan_sha256.clear();
        authoritative.plan_sha256 = sha256_json(&authoritative).unwrap();
        assert!(!verify_recipe_plan(&authoritative));
        // A kind without a registered template leaves the recipe unverifiable.
        let mut security = draft();
        security.kind = RecipeKind::SecurityRepair;
        security.verification = vec![ValidationKind::Security];
        let security = seal_recipe_manifest(security).unwrap();
        assert_eq!(
            instantiate_recipe(&security, &workspace(), &values(), &registry),
            Err(RecipeError::VerificationUnavailable)
        );
        let mut broken = registry;
        broken.registry_sha256 = "0".repeat(64);
        assert_eq!(
            instantiate_recipe(&manifest(), &workspace(), &values(), &broken),
            Err(RecipeError::RegistryInvalid)
        );
        let mut unsealed = manifest();
        unsealed.title = "Changed".to_owned();
        assert_eq!(
            instantiate_recipe(&unsealed, &workspace(), &values(), &self::registry()),
            Err(RecipeError::ManifestInvalid)
        );
    }

    #[test]
    fn parameters_are_named_typed_and_bounded() {
        let registry = registry();
        let cases: [ValuesChange<'_>; 12] = [
            &|values| {
                values.remove("manifest");
            },
            &|values| {
                values.insert(
                    "script".to_owned(),
                    RecipeValue::Choice("stable".to_owned()),
                );
            },
            &|values| {
                values.insert("retries".to_owned(), RecipeValue::Integer(4));
            },
            &|values| {
                values.insert("retries".to_owned(), RecipeValue::Integer(0));
            },
            &|values| {
                values.insert("retries".to_owned(), RecipeValue::Version("3".to_owned()));
            },
            &|values| {
                values.insert(
                    "channel".to_owned(),
                    RecipeValue::Choice("nightly".to_owned()),
                );
            },
            &|values| {
                values.insert(
                    "manifest".to_owned(),
                    RecipeValue::Path("src/main.rs".to_owned()),
                );
            },
            &|values| {
                values.insert(
                    "manifest".to_owned(),
                    RecipeValue::Path("crates/core/../../etc/passwd".to_owned()),
                );
            },
            &|values| {
                values.insert(
                    "manifest".to_owned(),
                    RecipeValue::Path("crates/core2/Cargo.toml".to_owned()),
                );
            },
            &|values| {
                values.insert(
                    "target_version".to_owned(),
                    RecipeValue::Version("1.4.x".to_owned()),
                );
            },
            &|values| {
                values.insert(
                    "target_version".to_owned(),
                    RecipeValue::Version("1.2.3.4.5".to_owned()),
                );
            },
            &|values| {
                values.insert(
                    "target_version".to_owned(),
                    RecipeValue::Version("1..2".to_owned()),
                );
            },
        ];
        for change in cases {
            let mut changed = values();
            change(&mut changed);
            assert_eq!(
                instantiate_recipe(&manifest(), &workspace(), &changed, &registry),
                Err(RecipeError::ParameterInvalid)
            );
            // A client's check without a registry refuses the same values
            // (Decision 0133).
            assert_eq!(
                check_recipe_parameters(&manifest(), &workspace(), &changed),
                Err(RecipeError::ParameterInvalid)
            );
        }
        assert_eq!(
            check_recipe_parameters(&manifest(), &workspace(), &values()),
            Ok(())
        );
        let mut unsealed = manifest();
        unsealed.title = "Another title".to_owned();
        assert_eq!(
            check_recipe_parameters(&unsealed, &workspace(), &values()),
            Err(RecipeError::ManifestInvalid)
        );
        // An optional parameter may be absent; the scope prefix itself holds
        // no file path, but a path under a single-file scope entry is exact.
        let mut minimal = values();
        minimal.remove("retries");
        minimal.insert(
            "manifest".to_owned(),
            RecipeValue::Path("Cargo.lock".to_owned()),
        );
        assert!(instantiate_recipe(&manifest(), &workspace(), &minimal, &registry).is_ok());
    }

    #[test]
    fn a_plan_covers_only_changes_inside_its_scope_and_bound() {
        let plan = instantiate_recipe(&manifest(), &workspace(), &values(), &registry()).unwrap();
        assert_eq!(
            check_recipe_changes(
                &plan,
                &[
                    path("Cargo.lock"),
                    path("crates/core/Cargo.toml"),
                    path("Cargo.lock"),
                ]
            ),
            Ok(())
        );
        assert_eq!(
            check_recipe_changes(&plan, &[path("src/main.rs")]),
            Err(RecipeError::OutOfScope)
        );
        let foreign =
            WorkspacePath::new(WorkspaceId::from_raw("workspace-other"), ["Cargo.lock"]).unwrap();
        assert_eq!(
            check_recipe_changes(&plan, &[foreign]),
            Err(RecipeError::OutOfScope)
        );
        assert_eq!(
            check_recipe_changes(
                &plan,
                &[
                    path("Cargo.lock"),
                    path("crates/core/Cargo.toml"),
                    path("crates/core/src/lib.rs"),
                ]
            ),
            Err(RecipeError::TooManyChanges)
        );
        let mut tampered = plan;
        tampered.max_changed_files = 10;
        assert_eq!(
            check_recipe_changes(&tampered, &[path("Cargo.lock")]),
            Err(RecipeError::PlanInvalid)
        );
    }

    #[test]
    fn a_plan_decodes_exactly_as_a_host_declares_it() {
        // Decision 0133: a client reads the plan back from a run's
        // declarations; it decodes to the same plan, still verifies, and
        // unknown members are refused.
        let plan = instantiate_recipe(&manifest(), &workspace(), &values(), &registry()).unwrap();
        let encoded = serde_json::to_value(&plan).unwrap();
        let decoded: RecipePlan = serde_json::from_value(encoded.clone()).unwrap();
        assert_eq!(decoded, plan);
        assert!(verify_recipe_plan(&decoded));
        let mut extra = encoded.clone();
        extra["granted"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<RecipePlan>(extra).is_err());
        let mut extra_validation = encoded.clone();
        extra_validation["validations"][0]["command"] = serde_json::Value::Null;
        assert!(serde_json::from_value::<RecipePlan>(extra_validation).is_err());
        let mut extra_value = encoded.clone();
        extra_value["parameters"]["retries"]["unit"] = serde_json::Value::Null;
        assert!(serde_json::from_value::<RecipePlan>(extra_value).is_err());
        // Review F1 of `84e531fb`: a variant without members is encoded as its
        // tag alone, and a member beside that tag is refused by the types, in
        // a manifest and in a plan.
        let sealed = serde_json::to_value(manifest()).unwrap();
        for (pointer, tag) in [
            (
                "/parameters/1/parameter_type",
                serde_json::json!({"type": "path"}),
            ),
            (
                "/parameters/3/parameter_type",
                serde_json::json!({"type": "version"}),
            ),
            (
                "/prerequisites/0",
                serde_json::json!({"kind": "clean_worktree"}),
            ),
            (
                "/prerequisites/1",
                serde_json::json!({"kind": "locked_dependencies"}),
            ),
            (
                "/rollback",
                serde_json::json!({"boundary": "revert_writes_only"}),
            ),
        ] {
            assert_eq!(sealed.pointer(pointer), Some(&tag), "{pointer}");
            let mut extra = sealed.clone();
            extra.pointer_mut(pointer).unwrap()["review"] = serde_json::Value::Bool(true);
            assert!(
                serde_json::from_value::<RecipeManifest>(extra.clone()).is_err(),
                "{pointer}"
            );
            assert_eq!(
                parse_recipe_manifest(&serde_json::to_vec(&extra).unwrap()),
                Err(RecipeError::ManifestInvalid),
                "{pointer}"
            );
        }
        for pointer in ["/prerequisites/0", "/prerequisites/1", "/rollback"] {
            let mut extra = encoded.clone();
            extra.pointer_mut(pointer).unwrap()["review"] = serde_json::Value::Bool(true);
            assert!(
                serde_json::from_value::<RecipePlan>(extra).is_err(),
                "{pointer}"
            );
        }
        for (value, expected) in [
            (
                serde_json::json!({"type": "integer", "value": 3}),
                Some(RecipeValue::Integer(3)),
            ),
            (
                serde_json::json!({"type": "path", "value": "Cargo.lock"}),
                Some(RecipeValue::Path("Cargo.lock".to_owned())),
            ),
            (serde_json::json!({"type": "integer", "value": "3"}), None),
            (serde_json::json!({"type": "url", "value": "x"}), None),
            (serde_json::json!({"type": "choice"}), None),
        ] {
            assert_eq!(serde_json::from_value::<RecipeValue>(value).ok(), expected);
        }
    }
}
