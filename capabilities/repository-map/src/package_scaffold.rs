//! Deterministic authority-free package scaffold plans from approved conventions.

use std::collections::BTreeSet;
use std::fmt::Write;

use agentmage_kernel_contracts::WorkspacePath;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::StructuredArtifactClass;

const PACKAGE_SCAFFOLD_SCHEMA_VERSION: u16 = 1;
const APACHE_2_LICENSE_SHA256: &str =
    "02f41e321c6eabad29b0f412b9aaa710dfcff9df7fa563a8db0a408e35e5ba6f";
const MAX_LICENSE_BYTES: usize = 32 * 1024;
const MAX_EXISTING_PATHS: usize = 50_000;

/// Closed first-increment package languages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageLanguage {
    /// Rust library package using Cargo.
    Rust,
    /// Python `src`-layout package using `pyproject.toml`.
    Python,
    /// TypeScript package using ESM and the TypeScript compiler.
    TypeScript,
    /// JavaScript package using ESM and the built-in Node test runner.
    JavaScript,
    /// Go library module using the Go toolchain.
    Go,
}

impl PackageLanguage {
    /// Complete stable language set.
    pub const ALL: [Self; 5] = [
        Self::Rust,
        Self::Python,
        Self::TypeScript,
        Self::JavaScript,
        Self::Go,
    ];

    const fn id(self) -> &'static str {
        match self {
            Self::Rust => "rust-cargo-library-v1",
            Self::Python => "python-pyproject-src-v1",
            Self::TypeScript => "typescript-esm-v1",
            Self::JavaScript => "javascript-esm-v1",
            Self::Go => "go-library-v1",
        }
    }
}

/// Purpose of one separately grantable local development command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScaffoldCommandPurpose {
    /// Check formatting without modifying files.
    FormatCheck,
    /// Run static analysis.
    Lint,
    /// Run package tests.
    Test,
    /// Build or package without publishing.
    Build,
}

/// One inert local command recipe emitted by an approved convention.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScaffoldCommand {
    /// Stable command identity.
    pub command_id: String,
    /// Closed command purpose.
    pub purpose: ScaffoldCommandPurpose,
    /// Exact executable basename; resolution remains a later trusted-host decision.
    pub program: String,
    /// Exact argument vector.
    pub arguments: Vec<String>,
    /// Exact package-root working directory.
    pub working_directory: WorkspacePath,
    /// Network remains prohibited.
    pub network_allowed: bool,
    /// Later execution requires a separate command grant.
    pub separate_grant_required: bool,
    /// This recipe carries no execution authority.
    pub execution_authority: bool,
}

/// One exact new file in an authority-free scaffold plan.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScaffoldFile {
    /// Stable file identity.
    pub file_id: String,
    /// Exact new workspace path.
    pub path: WorkspacePath,
    /// Explicit artifact class.
    pub artifact_class: StructuredArtifactClass,
    /// Complete proposed bytes.
    #[serde(with = "byte_serialization")]
    content: Vec<u8>,
    /// SHA-256 of the complete proposed bytes.
    pub content_sha256: String,
    /// Exact POSIX-compatible mode requested from a later filesystem plan.
    pub mode: u32,
    /// This record carries no mutation authority.
    pub mutation_authority: bool,
}

impl std::fmt::Debug for ScaffoldFile {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ScaffoldFile")
            .field("file_id", &self.file_id)
            .field("path", &self.path)
            .field("artifact_class", &self.artifact_class)
            .field("content_sha256", &self.content_sha256)
            .field("mode", &self.mode)
            .field("mutation_authority", &self.mutation_authority)
            .finish_non_exhaustive()
    }
}

impl ScaffoldFile {
    /// Returns the complete proposed bytes for a later explicit create preview.
    #[must_use]
    pub fn content(&self) -> &[u8] {
        &self.content
    }
}

/// Exact request for one approved package convention.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageScaffoldRequest {
    /// Stable scaffold identity.
    pub scaffold_id: String,
    /// Exact approved intent identity.
    pub intent_sha256: String,
    /// Exact approved change-plan identity.
    pub change_plan_sha256: String,
    /// Exact new package root.
    pub package_root: WorkspacePath,
    /// Package/distribution name.
    pub package_name: String,
    /// Source/module identifier.
    pub module_name: String,
    /// Closed language convention.
    pub language: PackageLanguage,
    /// Exact approved convention identity from [`package_convention_sha256`].
    pub convention_sha256: String,
    /// Exact digest of the observed parent entry set proving the review snapshot.
    pub observed_parent_entries_sha256: String,
    /// The selected package root was observed absent.
    pub package_root_absent: bool,
    /// Complete Apache-2.0 license artifact.
    pub license_bytes: Vec<u8>,
    /// Exact paths already present in the reviewed workspace snapshot.
    pub existing_paths: Vec<WorkspacePath>,
}

/// Complete immutable authority-free package scaffold plan.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageScaffoldPlan {
    /// Schema version.
    pub schema_version: u16,
    /// Stable scaffold identity.
    pub scaffold_id: String,
    /// Exact approved intent identity.
    pub intent_sha256: String,
    /// Exact approved change-plan identity.
    pub change_plan_sha256: String,
    /// Exact new package root.
    pub package_root: WorkspacePath,
    /// Package/distribution name.
    pub package_name: String,
    /// Source/module identifier.
    pub module_name: String,
    /// Closed language convention.
    pub language: PackageLanguage,
    /// Exact approved convention identity.
    pub convention_sha256: String,
    /// Exact parent-entry snapshot identity.
    pub observed_parent_entries_sha256: String,
    /// The selected package root was observed absent at planning time.
    pub package_root_absent: bool,
    /// Exact Apache-2.0 artifact identity.
    pub license_sha256: String,
    /// Stable path-sorted complete file set.
    pub files: Vec<ScaffoldFile>,
    /// Stable purpose-sorted inert local commands.
    pub local_commands: Vec<ScaffoldCommand>,
    /// Plan requires a later separately reviewed filesystem grant.
    pub separate_grant_required: bool,
    /// This plan carries no mutation authority.
    pub mutation_authority: bool,
    /// SHA-256 over every preceding field.
    pub plan_sha256: String,
}

impl std::fmt::Debug for PackageScaffoldPlan {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PackageScaffoldPlan")
            .field("scaffold_id", &self.scaffold_id)
            .field("package_root", &self.package_root)
            .field("language", &self.language)
            .field("file_count", &self.files.len())
            .field("local_commands", &self.local_commands)
            .field("plan_sha256", &self.plan_sha256)
            .finish_non_exhaustive()
    }
}

/// Stable content-free scaffold failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackageScaffoldError {
    /// An identity, path, ordering, bound, or name is invalid.
    InvalidInput,
    /// The requested convention is absent or stale.
    ConventionMismatch,
    /// The exact Apache-2.0 artifact is absent or changed.
    LicenseMismatch,
    /// The package root was not observed absent.
    RootNotAbsent,
    /// A generated target collides with an existing reviewed path.
    PathCollision,
}

impl PackageScaffoldError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "package-scaffold.input-invalid",
            Self::ConventionMismatch => "package-scaffold.convention-mismatch",
            Self::LicenseMismatch => "package-scaffold.license-mismatch",
            Self::RootNotAbsent => "package-scaffold.root-not-absent",
            Self::PathCollision => "package-scaffold.path-collision",
        }
    }
}

/// Returns the exact approved convention digest for one language.
#[must_use]
pub fn package_convention_sha256(language: PackageLanguage) -> String {
    sha256_json(&(
        "agentmage-package-convention-v1",
        language,
        language.id(),
        APACHE_2_LICENSE_SHA256,
        [
            ScaffoldCommandPurpose::FormatCheck,
            ScaffoldCommandPurpose::Lint,
            ScaffoldCommandPurpose::Test,
            ScaffoldCommandPurpose::Build,
        ],
    ))
}

/// Builds one exact package plan without observing or changing the filesystem.
pub fn build_package_scaffold(
    request: PackageScaffoldRequest,
) -> Result<PackageScaffoldPlan, PackageScaffoldError> {
    validate_request(&request)?;
    if request.convention_sha256 != package_convention_sha256(request.language) {
        return Err(PackageScaffoldError::ConventionMismatch);
    }
    if sha256_hex(&request.license_bytes) != APACHE_2_LICENSE_SHA256 {
        return Err(PackageScaffoldError::LicenseMismatch);
    }
    if !request.package_root_absent {
        return Err(PackageScaffoldError::RootNotAbsent);
    }

    let mut file_specs = convention_files(
        request.language,
        &request.package_name,
        &request.module_name,
        &request.license_bytes,
    );
    file_specs.sort_by(|left, right| left.0.cmp(&right.0));
    if file_specs.windows(2).any(|pair| pair[0].0 >= pair[1].0) {
        return Err(PackageScaffoldError::PathCollision);
    }
    let existing = request.existing_paths.iter().collect::<BTreeSet<_>>();
    let mut files = Vec::with_capacity(file_specs.len());
    for (index, (relative, artifact_class, content)) in file_specs.into_iter().enumerate() {
        let path = child_path(&request.package_root, &relative)?;
        if existing.contains(&path) {
            return Err(PackageScaffoldError::PathCollision);
        }
        files.push(ScaffoldFile {
            file_id: format!("scaffold-file-{index:02}"),
            path,
            artifact_class,
            content_sha256: sha256_hex(&content),
            content,
            mode: 0o644,
            mutation_authority: false,
        });
    }
    let local_commands = convention_commands(request.language, &request.package_root);
    let mut plan = PackageScaffoldPlan {
        schema_version: PACKAGE_SCAFFOLD_SCHEMA_VERSION,
        scaffold_id: request.scaffold_id,
        intent_sha256: request.intent_sha256,
        change_plan_sha256: request.change_plan_sha256,
        package_root: request.package_root,
        package_name: request.package_name,
        module_name: request.module_name,
        language: request.language,
        convention_sha256: request.convention_sha256,
        observed_parent_entries_sha256: request.observed_parent_entries_sha256,
        package_root_absent: true,
        license_sha256: APACHE_2_LICENSE_SHA256.to_owned(),
        files,
        local_commands,
        separate_grant_required: true,
        mutation_authority: false,
        plan_sha256: String::new(),
    };
    plan.plan_sha256 = plan_digest(&plan);
    Ok(plan)
}

/// Verifies one package plan by complete deterministic recomputation.
#[must_use]
pub fn verify_package_scaffold(plan: &PackageScaffoldPlan) -> bool {
    let Some(license) = plan.files.iter().find(|file| {
        file.path
            .components()
            .last()
            .is_some_and(|component| component.as_str() == "LICENSE")
    }) else {
        return false;
    };
    let mut expected_specs = convention_files(
        plan.language,
        &plan.package_name,
        &plan.module_name,
        license.content(),
    );
    expected_specs.sort_by(|left, right| left.0.cmp(&right.0));
    let exact_files = plan.files.len() == expected_specs.len()
        && plan.files.iter().zip(expected_specs).enumerate().all(
            |(index, (file, (relative, artifact_class, content)))| {
                child_path(&plan.package_root, &relative).is_ok_and(|path| path == file.path)
                    && file.file_id == format!("scaffold-file-{index:02}")
                    && file.artifact_class == artifact_class
                    && file.content == content
                    && file.content_sha256 == sha256_hex(&content)
                    && file.mode == 0o644
                    && !file.mutation_authority
            },
        );
    plan.schema_version == PACKAGE_SCAFFOLD_SCHEMA_VERSION
        && valid_identifier(&plan.scaffold_id)
        && is_sha256(&plan.intent_sha256)
        && is_sha256(&plan.change_plan_sha256)
        && is_package_name(&plan.package_name)
        && is_module_name(&plan.module_name)
        && plan.convention_sha256 == package_convention_sha256(plan.language)
        && is_sha256(&plan.observed_parent_entries_sha256)
        && plan.package_root_absent
        && plan.license_sha256 == APACHE_2_LICENSE_SHA256
        && license.content_sha256 == APACHE_2_LICENSE_SHA256
        && plan.separate_grant_required
        && !plan.mutation_authority
        && exact_files
        && plan.local_commands == convention_commands(plan.language, &plan.package_root)
        && plan.plan_sha256 == plan_digest(plan)
}

fn validate_request(request: &PackageScaffoldRequest) -> Result<(), PackageScaffoldError> {
    if !valid_identifier(&request.scaffold_id)
        || !is_sha256(&request.intent_sha256)
        || !is_sha256(&request.change_plan_sha256)
        || !is_package_name(&request.package_name)
        || !is_module_name(&request.module_name)
        || !is_sha256(&request.convention_sha256)
        || !is_sha256(&request.observed_parent_entries_sha256)
        || request.license_bytes.is_empty()
        || request.license_bytes.len() > MAX_LICENSE_BYTES
        || request.existing_paths.len() > MAX_EXISTING_PATHS
        || request
            .existing_paths
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        || request
            .existing_paths
            .iter()
            .any(|path| path.workspace_id() != request.package_root.workspace_id())
    {
        return Err(PackageScaffoldError::InvalidInput);
    }
    Ok(())
}

fn convention_files(
    language: PackageLanguage,
    package_name: &str,
    module_name: &str,
    license: &[u8],
) -> Vec<(Vec<String>, StructuredArtifactClass, Vec<u8>)> {
    let readme = format!(
        "# {package_name}\n\nA package initialized from the AgentMage approved {} convention.\n",
        language.id()
    )
    .into_bytes();
    let mut common = vec![
        (
            vec!["LICENSE".to_owned()],
            StructuredArtifactClass::Documentation,
            license.to_vec(),
        ),
        (
            vec!["README.md".to_owned()],
            StructuredArtifactClass::Documentation,
            readme,
        ),
    ];
    let mut language_files = match language {
        PackageLanguage::Rust => vec![
            (
                vec!["Cargo.toml".to_owned()],
                StructuredArtifactClass::Configuration,
                format!(
                    "[package]\nname = \"{package_name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\nlicense = \"Apache-2.0\"\n\n[lib]\nname = \"{module_name}\"\npath = \"src/lib.rs\"\n"
                )
                .into_bytes(),
            ),
            (
                vec!["src".to_owned(), "lib.rs".to_owned()],
                StructuredArtifactClass::Code,
                b"#![forbid(unsafe_code)]\n\n/// Returns the package identity.\n#[must_use]\npub const fn package_id() -> &'static str { env!(\"CARGO_PKG_NAME\") }\n"
                    .to_vec(),
            ),
            (
                vec!["tests".to_owned(), "smoke.rs".to_owned()],
                StructuredArtifactClass::Test,
                format!(
                    "#[test]\nfn reports_package_identity() {{\n    assert_eq!({module_name}::package_id(), \"{package_name}\");\n}}\n"
                )
                .into_bytes(),
            ),
        ],
        PackageLanguage::Python => vec![
            (
                vec!["pyproject.toml".to_owned()],
                StructuredArtifactClass::Configuration,
                format!(
                    "[build-system]\nrequires = [\"hatchling\"]\nbuild-backend = \"hatchling.build\"\n\n[project]\nname = \"{package_name}\"\nversion = \"0.1.0\"\nrequires-python = \">=3.12\"\nlicense = {{ text = \"Apache-2.0\" }}\n\n[tool.pytest.ini_options]\ntestpaths = [\"tests\"]\n\n[tool.ruff]\nline-length = 100\n"
                )
                .into_bytes(),
            ),
            (
                vec!["src".to_owned(), module_name.to_owned(), "__init__.py".to_owned()],
                StructuredArtifactClass::Code,
                format!("\"\"\"{package_name} package.\"\"\"\n\n__version__ = \"0.1.0\"\n")
                    .into_bytes(),
            ),
            (
                vec!["tests".to_owned(), "test_smoke.py".to_owned()],
                StructuredArtifactClass::Test,
                format!(
                    "from {module_name} import __version__\n\n\ndef test_version() -> None:\n    assert __version__ == \"0.1.0\"\n"
                )
                .into_bytes(),
            ),
        ],
        PackageLanguage::TypeScript => vec![
            (
                vec!["package.json".to_owned()],
                StructuredArtifactClass::Configuration,
                format!(
                    "{{\n  \"name\": \"{package_name}\",\n  \"version\": \"0.1.0\",\n  \"type\": \"module\",\n  \"license\": \"Apache-2.0\",\n  \"scripts\": {{\n    \"build\": \"tsc --noEmit\",\n    \"format:check\": \"prettier --check .\",\n    \"lint\": \"eslint .\",\n    \"test\": \"vitest run\"\n  }}\n}}\n"
                )
                .into_bytes(),
            ),
            (
                vec!["tsconfig.json".to_owned()],
                StructuredArtifactClass::Configuration,
                b"{\n  \"compilerOptions\": {\n    \"module\": \"NodeNext\",\n    \"moduleResolution\": \"NodeNext\",\n    \"strict\": true\n  },\n  \"include\": [\"src\", \"tests\"]\n}\n"
                    .to_vec(),
            ),
            (
                vec!["src".to_owned(), "index.ts".to_owned()],
                StructuredArtifactClass::Code,
                format!("export const packageName = \"{package_name}\" as const;\n").into_bytes(),
            ),
            (
                vec!["tests".to_owned(), "index.test.ts".to_owned()],
                StructuredArtifactClass::Test,
                format!(
                    "import {{ describe, expect, it }} from \"vitest\";\nimport {{ packageName }} from \"../src/index.js\";\n\ndescribe(\"package identity\", () => {{\n  it(\"is stable\", () => expect(packageName).toBe(\"{package_name}\"));\n}});\n"
                )
                .into_bytes(),
            ),
        ],
        PackageLanguage::JavaScript => vec![
            (
                vec!["package.json".to_owned()],
                StructuredArtifactClass::Configuration,
                format!(
                    "{{\n  \"name\": \"{package_name}\",\n  \"version\": \"0.1.0\",\n  \"type\": \"module\",\n  \"license\": \"Apache-2.0\",\n  \"scripts\": {{\n    \"format:check\": \"prettier --check .\",\n    \"lint\": \"eslint .\",\n    \"test\": \"node --test\",\n    \"build\": \"npm pack --dry-run --ignore-scripts\"\n  }}\n}}\n"
                )
                .into_bytes(),
            ),
            (
                vec!["src".to_owned(), "index.js".to_owned()],
                StructuredArtifactClass::Code,
                format!("export const packageName = \"{package_name}\";\n").into_bytes(),
            ),
            (
                vec!["test".to_owned(), "index.test.js".to_owned()],
                StructuredArtifactClass::Test,
                format!(
                    "import assert from \"node:assert/strict\";\nimport test from \"node:test\";\nimport {{ packageName }} from \"../src/index.js\";\n\ntest(\"package identity is stable\", () => {{\n  assert.equal(packageName, \"{package_name}\");\n}});\n"
                )
                .into_bytes(),
            ),
        ],
        PackageLanguage::Go => vec![
            (
                vec!["go.mod".to_owned()],
                StructuredArtifactClass::Configuration,
                format!("module example.invalid/{package_name}\n\ngo 1.24\n").into_bytes(),
            ),
            (
                vec![format!("{module_name}.go")],
                StructuredArtifactClass::Code,
                format!(
                    "// Package {module_name} provides the {package_name} library.\npackage {module_name}\n\n// Name is the stable package identity.\nconst Name = \"{package_name}\"\n"
                )
                .into_bytes(),
            ),
            (
                vec![format!("{module_name}_test.go")],
                StructuredArtifactClass::Test,
                format!(
                    "package {module_name}\n\nimport \"testing\"\n\nfunc TestName(t *testing.T) {{\n\tif Name != \"{package_name}\" {{\n\t\tt.Fatalf(\"unexpected package name: %q\", Name)\n\t}}\n}}\n"
                )
                .into_bytes(),
            ),
        ],
    };
    common.append(&mut language_files);
    common
}

fn convention_commands(language: PackageLanguage, root: &WorkspacePath) -> Vec<ScaffoldCommand> {
    let commands: [(ScaffoldCommandPurpose, &str, &[&str]); 4] = match language {
        PackageLanguage::Rust => [
            (
                ScaffoldCommandPurpose::FormatCheck,
                "cargo",
                &["fmt", "--all", "--", "--check"],
            ),
            (
                ScaffoldCommandPurpose::Lint,
                "cargo",
                &["clippy", "--all-targets", "--", "-D", "warnings"],
            ),
            (
                ScaffoldCommandPurpose::Test,
                "cargo",
                &["test", "--all-targets"],
            ),
            (ScaffoldCommandPurpose::Build, "cargo", &["build"]),
        ],
        PackageLanguage::Python => [
            (
                ScaffoldCommandPurpose::FormatCheck,
                "python3",
                &["-m", "ruff", "format", "--check", "."],
            ),
            (
                ScaffoldCommandPurpose::Lint,
                "python3",
                &["-m", "ruff", "check", "."],
            ),
            (ScaffoldCommandPurpose::Test, "python3", &["-m", "pytest"]),
            (
                ScaffoldCommandPurpose::Build,
                "python3",
                &["-m", "build", "--no-isolation"],
            ),
        ],
        PackageLanguage::TypeScript | PackageLanguage::JavaScript => [
            (
                ScaffoldCommandPurpose::FormatCheck,
                "npm",
                &["run", "format:check"],
            ),
            (ScaffoldCommandPurpose::Lint, "npm", &["run", "lint"]),
            (ScaffoldCommandPurpose::Test, "npm", &["test"]),
            (ScaffoldCommandPurpose::Build, "npm", &["run", "build"]),
        ],
        PackageLanguage::Go => [
            (ScaffoldCommandPurpose::FormatCheck, "gofmt", &["-d", "."]),
            (ScaffoldCommandPurpose::Lint, "go", &["vet", "./..."]),
            (ScaffoldCommandPurpose::Test, "go", &["test", "./..."]),
            (ScaffoldCommandPurpose::Build, "go", &["build", "./..."]),
        ],
    };
    commands
        .into_iter()
        .map(|(purpose, program, arguments)| ScaffoldCommand {
            command_id: format!("scaffold-command-{:?}", purpose).to_ascii_lowercase(),
            purpose,
            program: program.to_owned(),
            arguments: arguments.iter().map(|value| (*value).to_owned()).collect(),
            working_directory: root.clone(),
            network_allowed: false,
            separate_grant_required: true,
            execution_authority: false,
        })
        .collect()
}

fn child_path(
    root: &WorkspacePath,
    relative: &[String],
) -> Result<WorkspacePath, PackageScaffoldError> {
    let mut components = root
        .components()
        .iter()
        .map(|component| component.as_str().to_owned())
        .collect::<Vec<_>>();
    components.extend(relative.iter().cloned());
    WorkspacePath::new(root.workspace_id().clone(), components)
        .map_err(|_| PackageScaffoldError::InvalidInput)
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn is_package_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.as_bytes()[0].is_ascii_lowercase()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !value.ends_with('-')
        && !value.contains("--")
}

fn is_module_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && (value.as_bytes()[0].is_ascii_lowercase() || value.as_bytes()[0] == b'_')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        && !matches!(value, "self" | "super" | "crate" | "mod" | "package")
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn plan_digest(plan: &PackageScaffoldPlan) -> String {
    let mut canonical = plan.clone();
    canonical.plan_sha256.clear();
    sha256_json(&canonical)
}

fn sha256_json(value: &impl Serialize) -> String {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .unwrap_or_else(|_| sha256_hex(b"package-scaffold-serialization-failed"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

mod byte_serialization {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(bytes)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Vec::<u8>::deserialize(deserializer)
    }
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::WorkspaceId;

    use super::*;

    fn root() -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-scaffold"),
            ["packages", "sample"],
        )
        .expect("root")
    }

    fn request(language: PackageLanguage) -> PackageScaffoldRequest {
        PackageScaffoldRequest {
            scaffold_id: "scaffold-sample-1".to_owned(),
            intent_sha256: "1".repeat(64),
            change_plan_sha256: "2".repeat(64),
            package_root: root(),
            package_name: "sample-package".to_owned(),
            module_name: "sample_package".to_owned(),
            language,
            convention_sha256: package_convention_sha256(language),
            observed_parent_entries_sha256: "3".repeat(64),
            package_root_absent: true,
            license_bytes: include_bytes!("../../../LICENSE").to_vec(),
            existing_paths: Vec::new(),
        }
    }

    #[test]
    fn every_approved_language_emits_license_tests_docs_config_source_and_commands() {
        for language in PackageLanguage::ALL {
            let plan = build_package_scaffold(request(language)).expect("scaffold");
            assert!(verify_package_scaffold(&plan));
            assert!(plan.files.iter().any(|file| {
                file.path
                    .components()
                    .last()
                    .is_some_and(|part| part.as_str() == "LICENSE")
            }));
            for class in [
                StructuredArtifactClass::Code,
                StructuredArtifactClass::Configuration,
                StructuredArtifactClass::Test,
                StructuredArtifactClass::Documentation,
            ] {
                assert!(plan.files.iter().any(|file| file.artifact_class == class));
            }
            assert_eq!(plan.local_commands.len(), 4);
            assert_eq!(
                plan.local_commands[0].purpose,
                ScaffoldCommandPurpose::FormatCheck
            );
            assert!(
                plan.files
                    .windows(2)
                    .all(|pair| pair[0].path < pair[1].path)
            );
        }
    }

    #[test]
    fn collisions_stale_conventions_license_changes_and_present_roots_fail_closed() {
        let baseline = request(PackageLanguage::Rust);
        let expected = build_package_scaffold(baseline.clone()).expect("baseline");

        let mut collision = baseline.clone();
        collision.existing_paths = vec![expected.files[0].path.clone()];
        assert_eq!(
            build_package_scaffold(collision),
            Err(PackageScaffoldError::PathCollision)
        );

        let mut convention = baseline.clone();
        convention.convention_sha256 = "4".repeat(64);
        assert_eq!(
            build_package_scaffold(convention),
            Err(PackageScaffoldError::ConventionMismatch)
        );

        let mut license = baseline.clone();
        license.license_bytes.push(b'\n');
        assert_eq!(
            build_package_scaffold(license),
            Err(PackageScaffoldError::LicenseMismatch)
        );

        let mut present = baseline;
        present.package_root_absent = false;
        assert_eq!(
            build_package_scaffold(present),
            Err(PackageScaffoldError::RootNotAbsent)
        );
    }

    #[test]
    fn plan_and_file_mutations_never_verify() {
        let baseline = build_package_scaffold(request(PackageLanguage::Python)).expect("baseline");
        for sequence in 0_u8..7 {
            let mut changed = baseline.clone();
            match sequence {
                0 => changed.intent_sha256 = "a".repeat(64),
                1 => changed.convention_sha256 = "b".repeat(64),
                2 => changed.files[0].content.push(b'x'),
                3 => changed.files[0].content_sha256 = "c".repeat(64),
                4 => changed.files[0].mutation_authority = true,
                5 => changed.local_commands[0].network_allowed = true,
                _ => changed.plan_sha256 = "d".repeat(64),
            }
            assert!(!verify_package_scaffold(&changed));
        }
    }
}
