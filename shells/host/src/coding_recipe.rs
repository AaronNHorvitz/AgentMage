//! Recipe plans handed to the ordinary change approval (Decision 0133).
//!
//! A person names a recipe manifest file and parameter values with a coding
//! run. The development CLI reads and checks both before any host is
//! launched; the host instantiates the plan against the workspace's
//! validation templates when it prepares the run and binds the plan's digest
//! into the run's constraints; and the tool boundary refuses each proposed
//! write outside the plan's scope, or beyond its file bound, before any
//! approval is asked. A write inside the plan still needs the person's own
//! approval: the plan narrows what may be proposed and grants nothing. The
//! run's declarations carry the plan back, and the client keeps it only when
//! it matches what was sent and the run's constraints.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;
use std::sync::{Arc, Mutex};

use agentmage_kernel_contracts::{
    RuntimeRunRequest, WorkspaceId, WorkspacePath, WorkspaceScopePath,
};
use agentmage_kernel_engine::engineering_recipe::{
    RecipeError, RecipeKind, RecipeManifest, RecipeParameterType, RecipePlan, RecipePrerequisite,
    RecipeRollback, RecipeValue, check_recipe_changes, check_recipe_parameters, instantiate_recipe,
    parse_recipe_manifest, verify_recipe_plan,
};
use agentmage_kernel_engine::validation_template::{ValidationKind, ValidationTemplateRegistry};
use serde::{Deserialize, Serialize};

use crate::coding_operation::NativeCodingTargetPlan;

/// Largest recipe manifest file the CLI reads.
pub const MAX_RECIPE_FILE_BYTES: u64 = 64 * 1024;
/// Largest `NAME=VALUE` argument.
pub const MAX_RECIPE_PARAMETER_BYTES: usize = 1_024;
/// Most `--recipe-param` arguments; a manifest declares at most sixteen
/// parameters.
pub const MAX_RECIPE_PARAMETERS: usize = 16;
/// The run constraint that binds a plan's digest into the run request.
const PLAN_CONSTRAINT_PREFIX: &str = "Recipe plan digest: ";
/// Longest scope list the run's constraints spell out for the model.
const MAX_SCOPE_TEXT_BYTES: usize = 1_024;

/// What a client sends with a run: the recipe's sealed manifest and the
/// typed values of its parameters.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRecipeRequest {
    /// Sealed manifest, as the file holds it.
    pub manifest: RecipeManifest,
    /// Parameter values by name.
    pub parameters: BTreeMap<String, RecipeValue>,
}

/// Content-free refusal of a run's recipe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecipeRunRefusal {
    /// The manifest file is not an absolute, readable, bounded regular file.
    FileInvalid,
    /// The manifest, a value or the registry is refused by the component.
    Recipe(RecipeError),
    /// Applying needs a network grant, which the development host never
    /// holds.
    NetworkGrantUnavailable,
}

impl RecipeRunRefusal {
    /// Stable content-free code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::FileInvalid => "recipe.file-invalid",
            Self::Recipe(error) => error.code(),
            Self::NetworkGrantUnavailable => "recipe.network-grant-unavailable",
        }
    }

    /// The closed process exit class of a refusal before launch.
    #[must_use]
    pub const fn exit_code(self) -> crate::headless::ClientExitCode {
        use crate::headless::ClientExitCode;
        match self {
            Self::NetworkGrantUnavailable => ClientExitCode::AuthorityDenied,
            Self::FileInvalid | Self::Recipe(_) => ClientExitCode::InvalidInput,
        }
    }
}

/// Splits one `--recipe-param` argument at its first `=`. The name is a
/// plain parameter name and the value is not empty; the component checks
/// both against the manifest later.
#[must_use]
pub fn parse_recipe_parameter(argument: &str) -> Option<(String, String)> {
    let (name, value) = argument.split_once('=')?;
    (argument.len() <= MAX_RECIPE_PARAMETER_BYTES
        && !name.is_empty()
        && name.len() <= 64
        && name.as_bytes()[0].is_ascii_lowercase()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        && !value.is_empty()
        && !value.chars().any(char::is_control))
    .then(|| (name.to_owned(), value.to_owned()))
}

/// Types one textual value by its declared parameter type. An integer must
/// be written exactly as it prints, so `+3`, `03` and `3.0` are refused.
#[must_use]
pub fn recipe_value(parameter_type: &RecipeParameterType, text: &str) -> Option<RecipeValue> {
    match parameter_type {
        RecipeParameterType::Choice { .. } => Some(RecipeValue::Choice(text.to_owned())),
        RecipeParameterType::Integer { .. } => text
            .parse::<i64>()
            .ok()
            .filter(|number| number.to_string() == text)
            .map(RecipeValue::Integer),
        RecipeParameterType::Path => Some(RecipeValue::Path(text.to_owned())),
        RecipeParameterType::Version => Some(RecipeValue::Version(text.to_owned())),
    }
}

/// Builds the request a client sends from an admitted manifest and the
/// person's `NAME=VALUE` pairs, and checks every value for the run's
/// workspace before any host is launched. A name the manifest does not
/// declare, a repeated name, an untyped value, a missing required value and
/// a recipe that needs a network grant are refused.
pub fn recipe_request(
    manifest: RecipeManifest,
    parameters: &[(String, String)],
    workspace_id: &WorkspaceId,
) -> Result<RuntimeRecipeRequest, RecipeRunRefusal> {
    let invalid = RecipeRunRefusal::Recipe(RecipeError::ParameterInvalid);
    let mut values = BTreeMap::new();
    for (name, text) in parameters {
        let declared = manifest
            .parameters
            .iter()
            .find(|parameter| &parameter.name == name)
            .ok_or(invalid)?;
        let value = recipe_value(&declared.parameter_type, text).ok_or(invalid)?;
        if values.insert(name.clone(), value).is_some() {
            return Err(invalid);
        }
    }
    check_recipe_parameters(&manifest, workspace_id, &values).map_err(RecipeRunRefusal::Recipe)?;
    if manifest.needs_network_grant {
        return Err(RecipeRunRefusal::NetworkGrantUnavailable);
    }
    Ok(RuntimeRecipeRequest {
        manifest,
        parameters: values,
    })
}

/// Reads and admits one manifest file named by an absolute path, without
/// following a link at its last component.
pub fn read_recipe_manifest(path: &Path) -> Result<RecipeManifest, RecipeRunRefusal> {
    let bytes = read_recipe_file(path)?;
    parse_recipe_manifest(&bytes).map_err(RecipeRunRefusal::Recipe)
}

#[cfg(target_os = "linux")]
fn read_recipe_file(path: &Path) -> Result<Vec<u8>, RecipeRunRefusal> {
    use std::io::Read as _;

    use rustix::fs::{FileType, Mode, OFlags};

    let invalid = RecipeRunRefusal::FileInvalid;
    if !path.is_absolute() {
        return Err(invalid);
    }
    // A FIFO or device opens without blocking and is then refused as not a
    // regular file.
    let file = rustix::fs::open(
        path,
        OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| invalid)?;
    let metadata = rustix::fs::fstat(&file).map_err(|_| invalid)?;
    if FileType::from_raw_mode(metadata.st_mode) != FileType::RegularFile {
        return Err(invalid);
    }
    let length = u64::try_from(metadata.st_size)
        .ok()
        .filter(|length| *length > 0 && *length <= MAX_RECIPE_FILE_BYTES)
        .ok_or(invalid)?;
    let mut bytes = Vec::with_capacity(usize::try_from(length).unwrap_or(0));
    std::fs::File::from(file)
        .take(length + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid)?;
    if bytes.len() as u64 != length {
        return Err(invalid);
    }
    Ok(bytes)
}

#[cfg(not(target_os = "linux"))]
fn read_recipe_file(_path: &Path) -> Result<Vec<u8>, RecipeRunRefusal> {
    Err(RecipeRunRefusal::FileInvalid)
}

/// Instantiates the plan of a run's recipe against the workspace's
/// validation templates, as the host does when it prepares the run. Every
/// value and the manifest's seal are checked again here.
pub fn instantiate_run_recipe(
    request: &RuntimeRecipeRequest,
    workspace_id: &WorkspaceId,
    registry: &ValidationTemplateRegistry,
) -> Result<RecipePlan, RecipeRunRefusal> {
    if request.manifest.needs_network_grant {
        return Err(RecipeRunRefusal::NetworkGrantUnavailable);
    }
    instantiate_recipe(
        &request.manifest,
        workspace_id,
        &request.parameters,
        registry,
    )
    .map_err(RecipeRunRefusal::Recipe)
}

/// The run constraints that state a plan: one binds its digest into the run
/// request, and one tells the model what it may change.
#[must_use]
pub fn recipe_constraints(plan: &RecipePlan) -> Vec<String> {
    let scope = plan.scope.iter().map(scope_text).collect::<Vec<_>>();
    let joined = scope.join(", ");
    let scope = if joined.len() <= MAX_SCOPE_TEXT_BYTES {
        joined
    } else {
        format!("{} declared prefixes", scope.len())
    };
    vec![
        format!("{PLAN_CONSTRAINT_PREFIX}{}", plan.plan_sha256),
        format!(
            "Recipe {} {} ({}): change only files under {scope}, at most {} distinct files; \
             a write outside this plan is refused, and each write inside it needs its own approval",
            plan.recipe_id,
            version_text(plan),
            kind_text(plan.recipe_kind),
            plan.max_changed_files,
        ),
    ]
}

/// Every plan digest a run request's constraints bind: none for a run
/// without a recipe, one for a run held to a plan.
#[must_use]
pub fn declared_plan_sha256s(request: &RuntimeRunRequest) -> Vec<&str> {
    request
        .task
        .constraints
        .iter()
        .filter_map(|constraint| constraint.strip_prefix(PLAN_CONSTRAINT_PREFIX))
        .collect()
}

/// Whether a prepared run request binds one plan exactly when a recipe was
/// sent with it.
#[must_use]
pub fn request_binds_recipe(
    request: &RuntimeRunRequest,
    sent: Option<&RuntimeRecipeRequest>,
) -> bool {
    match declared_plan_sha256s(request).as_slice() {
        [] => sent.is_none(),
        [digest] => sent.is_some() && is_sha256(digest),
        _ => false,
    }
}

/// Whether a plan a host declared for a run is the plan of what the client
/// sent: it verifies, names the sent manifest, values and the run's
/// workspace, and its digest is the one the run's constraints bind.
#[must_use]
pub fn verify_declared_recipe_plan(
    plan: &RecipePlan,
    request: &RuntimeRunRequest,
    sent: Option<&RuntimeRecipeRequest>,
) -> bool {
    let Some(sent) = sent else {
        return false;
    };
    let manifest = &sent.manifest;
    let kinds = plan
        .validations
        .iter()
        .map(|validation| validation.kind)
        .collect::<BTreeSet<_>>();
    verify_recipe_plan(plan)
        && plan.recipe_id == manifest.recipe_id
        && plan.recipe_version == manifest.version
        && plan.recipe_kind == manifest.kind
        && plan.manifest_sha256 == manifest.manifest_sha256
        && plan.parameters == sent.parameters
        && plan.workspace_id == request.workspace_id
        && plan.scope.len() == manifest.scope.len()
        && plan
            .scope
            .iter()
            .zip(&manifest.scope)
            .all(|(scope, prefix)| {
                scope.workspace_id() == &request.workspace_id && scope_text(scope) == *prefix
            })
        && kinds == manifest.verification.iter().copied().collect()
        && plan.rollback == manifest.rollback
        && plan.prerequisites == manifest.prerequisites
        && plan.max_changed_files == manifest.max_changed_files
        && !plan.network_grant_required
        && declared_plan_sha256s(request) == [plan.plan_sha256.as_str()]
}

/// The workspace path one prepared operation would write, if it writes.
#[must_use]
pub const fn recipe_write_target(target: &NativeCodingTargetPlan) -> Option<&WorkspacePath> {
    match target {
        NativeCodingTargetPlan::ExistingFile { path, .. } => Some(path),
        NativeCodingTargetPlan::DestinationParent { destination, .. } => Some(destination),
        NativeCodingTargetPlan::ReadProjection { .. }
        | NativeCodingTargetPlan::OwnedWorktreeRoot => None,
    }
}

/// One run's plan and the distinct paths it admitted so far.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipeRunScope {
    plan: RecipePlan,
    admitted: BTreeSet<WorkspacePath>,
}

impl RecipeRunScope {
    /// Starts a verified plan with no admitted path.
    pub fn new(plan: RecipePlan) -> Result<Self, RecipeError> {
        if !verify_recipe_plan(&plan) {
            return Err(RecipeError::PlanInvalid);
        }
        Ok(Self {
            plan,
            admitted: BTreeSet::new(),
        })
    }

    /// Admits one proposed write when it and every path admitted before stay
    /// inside the plan. Each distinct path counts once, whether or not the
    /// person later approves its write, so the bound covers every path the
    /// plan let a proposal reach. A refused path is not counted.
    pub fn admit(&mut self, path: &WorkspacePath) -> Result<(), RecipeError> {
        if self.admitted.contains(path) {
            return Ok(());
        }
        let mut candidate = self.admitted.iter().cloned().collect::<Vec<_>>();
        candidate.push(path.clone());
        check_recipe_changes(&self.plan, &candidate)?;
        self.admitted.insert(path.clone());
        Ok(())
    }

    /// The plan.
    #[must_use]
    pub const fn plan(&self) -> &RecipePlan {
        &self.plan
    }

    /// How many distinct paths the plan admitted.
    #[must_use]
    pub fn admitted_count(&self) -> usize {
        self.admitted.len()
    }
}

/// A run's plan scope, shared by the host's prepared run and each tool
/// boundary composed for it, so a run continued in the same host keeps the
/// paths admitted before it was suspended.
#[derive(Clone, Debug)]
pub struct SharedRecipeScope(Arc<Mutex<RecipeRunScope>>);

impl SharedRecipeScope {
    /// Shares one run's scope.
    #[must_use]
    pub fn new(scope: RecipeRunScope) -> Self {
        Self(Arc::new(Mutex::new(scope)))
    }

    /// Admits one proposed write; a poisoned lock refuses.
    pub fn admit(&self, path: &WorkspacePath) -> Result<(), RecipeError> {
        self.0
            .lock()
            .map_err(|_| RecipeError::PlanInvalid)?
            .admit(path)
    }

    /// The plan, unless the lock is poisoned.
    #[must_use]
    pub fn plan(&self) -> Option<RecipePlan> {
        self.0.lock().ok().map(|scope| scope.plan.clone())
    }
}

fn scope_text(scope: &WorkspaceScopePath) -> String {
    scope
        .components()
        .iter()
        .map(|component| component.as_str())
        .collect::<Vec<_>>()
        .join("/")
}

const fn kind_text(kind: RecipeKind) -> &'static str {
    match kind {
        RecipeKind::DependencyUpdate => "dependency update",
        RecipeKind::Migration => "migration",
        RecipeKind::SecurityRepair => "security repair",
        RecipeKind::Documentation => "documentation",
        RecipeKind::Tests => "tests",
    }
}

fn version_text(plan: &RecipePlan) -> String {
    format!(
        "{}.{}.{}",
        plan.recipe_version.major, plan.recipe_version.minor, plan.recipe_version.patch
    )
}

fn validation_kind_text(kind: ValidationKind) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| "unknown".to_owned())
}

fn prerequisite_text(prerequisite: &RecipePrerequisite) -> String {
    match prerequisite {
        RecipePrerequisite::CleanWorktree => "clean worktree".to_owned(),
        RecipePrerequisite::LockedDependencies => "locked dependencies".to_owned(),
        RecipePrerequisite::Tool { tool_id } => format!("tool {tool_id}"),
    }
}

fn rollback_text(rollback: &RecipeRollback) -> String {
    match rollback {
        RecipeRollback::RevertWritesOnly => {
            "the plan's own writes, each by an inverse write with its own approval".to_owned()
        }
        RecipeRollback::Irreversible { reason_code } => format!("nothing ({reason_code})"),
    }
}

fn value_text(value: &RecipeValue) -> String {
    match value {
        RecipeValue::Choice(text) | RecipeValue::Path(text) | RecipeValue::Version(text) => {
            text.escape_debug().to_string()
        }
        RecipeValue::Integer(number) => number.to_string(),
    }
}

/// What the person is told on standard error before the host is launched:
/// the recipe the run will be held to. Free text is escaped.
#[must_use]
pub fn render_recipe_request(request: &RuntimeRecipeRequest, json: bool) -> String {
    let manifest = &request.manifest;
    if json {
        return format!(
            "{}\n",
            serde_json::json!({
                "type": "recipe_requested",
                "recipe_id": manifest.recipe_id,
                "version": manifest.version,
                "kind": manifest.kind,
                "manifest_sha256": manifest.manifest_sha256,
                "scope": manifest.scope,
                "max_changed_files": manifest.max_changed_files,
                "verification": manifest.verification,
                "prerequisites": manifest.prerequisites,
                "rollback": manifest.rollback,
                "parameters": request.parameters,
                "proposal_only": true,
            })
        );
    }
    let mut output = format!(
        "recipe {} {}.{}.{} ({}) \"{}\" manifest {}\n",
        manifest.recipe_id,
        manifest.version.major,
        manifest.version.minor,
        manifest.version.patch,
        kind_text(manifest.kind),
        manifest.title.escape_debug(),
        short(&manifest.manifest_sha256),
    );
    let _ = writeln!(
        output,
        "  may change: {}; at most {} files",
        manifest
            .scope
            .iter()
            .map(|prefix| prefix.escape_debug().to_string())
            .collect::<Vec<_>>()
            .join(", "),
        manifest.max_changed_files
    );
    let _ = writeln!(
        output,
        "  verified by registered {} validation",
        manifest
            .verification
            .iter()
            .map(|kind| validation_kind_text(*kind))
            .collect::<Vec<_>>()
            .join(", ")
    );
    if !manifest.prerequisites.is_empty() {
        let _ = writeln!(
            output,
            "  prerequisites (shown, not checked by the plan): {}",
            manifest
                .prerequisites
                .iter()
                .map(prerequisite_text)
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    for (name, value) in &request.parameters {
        let _ = writeln!(output, "  {name} = {}", value_text(value));
    }
    let _ = writeln!(output, "  rollback: {}", rollback_text(&manifest.rollback));
    output.push_str(
        "  proposal only: a write outside the plan is refused; each write inside it still needs your approval\n",
    );
    output
}

/// The plan a run was held to, in its declarations' text form.
#[must_use]
pub fn render_recipe_plan(plan: Option<&RecipePlan>, sent: bool) -> String {
    let Some(plan) = plan else {
        return if sent {
            "recipe plan: unavailable; the host did not declare the plan that was sent\n".to_owned()
        } else {
            String::new()
        };
    };
    let mut output = format!(
        "recipe plan {} {} ({}) digest {} manifest {}\n",
        plan.recipe_id,
        version_text(plan),
        kind_text(plan.recipe_kind),
        short(&plan.plan_sha256),
        short(&plan.manifest_sha256),
    );
    let _ = writeln!(
        output,
        "  scope: {}; at most {} files",
        plan.scope
            .iter()
            .map(scope_text)
            .collect::<Vec<_>>()
            .join(", "),
        plan.max_changed_files
    );
    for validation in &plan.validations {
        let _ = writeln!(
            output,
            "  validation {} ({}) template {}",
            validation.validation_id,
            validation_kind_text(validation.kind),
            short(&validation.template_sha256)
        );
    }
    let _ = writeln!(output, "  rollback: {}", rollback_text(&plan.rollback));
    output.push_str("  proposal only: the plan granted nothing\n");
    output
}

/// The plan of an ended run after its declarations, when a recipe was sent:
/// the plan the client kept, or that it is unavailable. Nothing when no
/// recipe was sent.
#[must_use]
pub fn render_declared_recipe_plan(plan: Option<&RecipePlan>, sent: bool, json: bool) -> String {
    if !sent {
        return String::new();
    }
    if json {
        return format!(
            "{}\n",
            serde_json::json!({
                "type": "recipe_plan",
                "available": plan.is_some(),
                "plan": plan,
            })
        );
    }
    render_recipe_plan(plan, sent)
}

/// A refusal before launch, on standard error.
#[must_use]
pub fn render_recipe_refusal(refusal: RecipeRunRefusal, json: bool) -> String {
    if json {
        format!(
            "{}\n",
            serde_json::json!({"type": "recipe_refused", "code": refusal.code()})
        )
    } else {
        format!("recipe refused: {}\n", refusal.code())
    }
}

fn short(digest: &str) -> &str {
    digest.get(..12).unwrap_or(digest)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

/// A run scope of a synthetic `tests` recipe holding writes to `prefix`, with
/// one registered unit validation, for the tool boundary's tests.
#[cfg(test)]
pub(crate) fn test_scope(
    workspace_id: &WorkspaceId,
    prefix: &str,
    max_changed_files: u32,
) -> SharedRecipeScope {
    let (_, plan) = test_recipe(workspace_id, prefix, max_changed_files);
    SharedRecipeScope::new(RecipeRunScope::new(plan).unwrap())
}

/// A synthetic `tests` recipe holding writes to `prefix` and its plan for
/// one registered unit validation, for the host's tests.
#[cfg(test)]
pub(crate) fn test_recipe(
    workspace_id: &WorkspaceId,
    prefix: &str,
    max_changed_files: u32,
) -> (RuntimeRecipeRequest, RecipePlan) {
    use agentmage_kernel_engine::command_runner::{
        CommandBounds, CommandRisk, CommandSpec, CommandWorkingDirectory,
    };
    use agentmage_kernel_engine::engineering_recipe::{RecipeVersion, seal_recipe_manifest};
    use agentmage_kernel_engine::validation_template::{
        ValidationParserKind, ValidationTemplateInput, ValidationTemplateSource,
        seal_validation_template,
    };

    let template = seal_validation_template(ValidationTemplateInput {
        validation_id: "validation-unit".to_owned(),
        kind: ValidationKind::Unit,
        command: CommandSpec::seal(
            "command-unit".to_owned(),
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
        source_path: Some(WorkspacePath::new(workspace_id.clone(), ["Cargo.toml"]).unwrap()),
        user_input_approval_sha256: None,
        execution_scope_sha256: "3".repeat(64),
        parser: ValidationParserKind::AgentMageJsonV1,
        parser_version: "1.0.0".to_owned(),
        parser_sha256: "4".repeat(64),
        minimum_test_count: 1,
        focused: true,
        fail_fast: true,
        failed_test_rerun_allowed: true,
        setup_idempotent: true,
        expected_artifacts: Vec::new(),
    })
    .unwrap();
    let registry = ValidationTemplateRegistry::build(vec![template]).unwrap();
    let manifest = seal_recipe_manifest(RecipeManifest {
        schema_version: 1,
        recipe_id: "agentmage-test-recipe".to_owned(),
        version: RecipeVersion {
            major: 1,
            minor: 0,
            patch: 0,
        },
        kind: RecipeKind::Tests,
        title: "Synthetic test recipe".to_owned(),
        parameters: Vec::new(),
        scope: vec![prefix.to_owned()],
        max_changed_files,
        prerequisites: Vec::new(),
        verification: vec![ValidationKind::Unit],
        rollback: RecipeRollback::RevertWritesOnly,
        needs_network_grant: false,
        manifest_sha256: String::new(),
    })
    .unwrap();
    let request = RuntimeRecipeRequest {
        manifest,
        parameters: BTreeMap::new(),
    };
    let plan = instantiate_run_recipe(&request, workspace_id, &registry).unwrap();
    (request, plan)
}

#[cfg(all(
    test,
    target_os = "linux",
    feature = "source-artifacts",
    feature = "workflow-supervisor"
))]
#[path = "coding_recipe_tests.rs"]
mod tests;
