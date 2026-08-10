//! Closed configuration parsing, migration, comparison, and local recovery.

use std::collections::BTreeSet;
use std::fmt;
use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::fs::File;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

use ed25519_dalek::{Signature, VerifyingKey};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Number, Value};
use sha2::{Digest, Sha256};

const CURRENT_SCHEMA_VERSION: u32 = 1;
const DEFAULT_MAXIMUM_BYTES: usize = 1_048_576;
const MAXIMUM_SCHEMA_INTEGER: u64 = 2_147_483_647;
const MAXIMUM_SCHEMA_BYTES: u64 = 1_099_511_627_776;
const STRICT_LOCAL_PROFILE: &[u8] =
    include_bytes!("../../../configuration/profiles/strict-local-read-only.json");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ErrorCode {
    TooLarge,
    MalformedJson,
    UnsupportedVersion,
    ContractViolation,
    Io,
    Conflict,
    AuthorityBroadening,
    ParentSignatureInvalid,
}

impl ErrorCode {
    const fn as_str(self) -> &'static str {
        match self {
            Self::TooLarge => "configuration-too-large",
            Self::MalformedJson => "configuration-malformed-json",
            Self::UnsupportedVersion => "configuration-unsupported-version",
            Self::ContractViolation => "configuration-contract-violation",
            Self::Io => "configuration-io-failure",
            Self::Conflict => "configuration-state-conflict",
            Self::AuthorityBroadening => "configuration-authority-broadening",
            Self::ParentSignatureInvalid => "configuration-parent-signature-invalid",
        }
    }
}

/// A bounded configuration failure that never includes raw configuration data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigurationError {
    code: ErrorCode,
    diagnostic: &'static str,
}

impl ConfigurationError {
    const fn new(code: ErrorCode, diagnostic: &'static str) -> Self {
        Self { code, diagnostic }
    }

    /// Returns the stable machine-readable error code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.code.as_str()
    }

    /// Returns a bounded diagnostic that contains no raw input value or private path.
    #[must_use]
    pub const fn diagnostic(&self) -> &'static str {
        self.diagnostic
    }
}

impl fmt::Display for ConfigurationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code(), self.diagnostic)
    }
}

impl std::error::Error for ConfigurationError {}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct AgentConfiguration {
    schema_version: u32,
    core: CoreConfiguration,
    platform: PlatformConfiguration,
    model: ModelConfiguration,
    workspace: WorkspaceConfiguration,
    tool: ToolConfiguration,
    permission: PermissionConfiguration,
    budget: BudgetConfiguration,
    logging: LoggingConfiguration,
    retention: RetentionConfiguration,
    skill: SkillConfiguration,
    shell: ShellConfiguration,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct CoreConfiguration {
    schema_version: u32,
    product_id: String,
    profile_id: String,
    configuration_mode: String,
    startup_failure_policy: String,
    strict_local: bool,
    inheritance_mode: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct PlatformConfiguration {
    schema_version: u32,
    platform_id: String,
    adapter_id: String,
    sandbox_profile_id: String,
    path_identity_strategy: String,
    secret_provider_id: String,
    standard_user_required: bool,
    administrator_required: bool,
    ambient_dependency_policy: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ModelConfiguration {
    schema_version: u32,
    enabled: bool,
    model_profile_id: String,
    manifest_path: String,
    runtime_adapter_id: String,
    transport: String,
    network_access: bool,
    automatic_routing: bool,
    decoding: DecodingConfiguration,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct DecodingConfiguration {
    deterministic: bool,
    temperature: f64,
    top_k: u32,
    seed: u32,
    maximum_context_tokens: u32,
    maximum_output_tokens: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceConfiguration {
    schema_version: u32,
    maximum_roots: u32,
    default_access: String,
    symlink_policy: String,
    roots: Vec<WorkspaceRoot>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceRoot {
    root_id: String,
    locator_kind: String,
    locator: String,
    access: String,
    follow_mount_changes: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ToolConfiguration {
    schema_version: u32,
    registry_path: String,
    unregistered_tool_policy: String,
    tools: Vec<ToolDefinition>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ToolDefinition {
    tool_id: String,
    version: String,
    integrity_sha256: String,
    enabled: bool,
    required_capabilities: Vec<String>,
    side_effect_class: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct PermissionConfiguration {
    schema_version: u32,
    default_effect: String,
    authority_inheritance: String,
    model_authority: String,
    grant_policy: GrantPolicy,
    allowed_capabilities: Vec<String>,
    network: NetworkPermission,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct GrantPolicy {
    single_use: bool,
    expiry_required: bool,
    exact_scope_required: bool,
    transferable: bool,
    replay_policy: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct NetworkPermission {
    mode: String,
    allowed_endpoints: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct BudgetConfiguration {
    schema_version: u32,
    maximum_input_bytes: u64,
    maximum_output_bytes: u64,
    maximum_file_count: u32,
    maximum_tree_depth: u32,
    maximum_context_tokens: u32,
    maximum_output_tokens: u32,
    maximum_operation_milliseconds: u64,
    maximum_memory_bytes: u64,
    maximum_processes: u32,
    maximum_concurrency: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct LoggingConfiguration {
    schema_version: u32,
    level: String,
    structured: bool,
    destination: LoggingDestination,
    maximum_event_bytes: u32,
    redaction: RedactionConfiguration,
    record_prompts: bool,
    record_tool_arguments: bool,
    record_environment_values: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct LoggingDestination {
    kind: String,
    managed_relative_path: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct RedactionConfiguration {
    secret_values: String,
    credentials: String,
    private_paths: String,
    environment_values: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct RetentionConfiguration {
    schema_version: u32,
    sessions_days: u32,
    receipts_days: u32,
    logs_days: u32,
    temporary_artifacts_minutes: u32,
    expired_data_action: String,
    backup_policy: String,
    secure_deletion_claim: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct SkillConfiguration {
    schema_version: u32,
    enablement: String,
    unsigned_skill_policy: String,
    self_modification: String,
    catalog_paths: Vec<String>,
    maximum_enabled_skills: u32,
    capability_ceiling: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ShellConfiguration {
    schema_version: u32,
    shell_id: String,
    surface_id: String,
    transport: String,
    launch_privilege: String,
    network_access: bool,
    direct_command_execution: bool,
    model_to_tool_channel: String,
    diagnostics_surface: String,
}

/// A validated configuration with deterministic canonical bytes and identity.
#[derive(Clone, Debug)]
pub struct LoadedConfiguration {
    configuration: AgentConfiguration,
    canonical_bytes: Vec<u8>,
    sha256: String,
}

#[derive(Serialize)]
struct ParentProfileSignatureMaterial<'a> {
    schema_version: u32,
    record_type: &'static str,
    profile_id: &'a str,
    configuration_sha256: &'a str,
}

/// A configuration whose detached Ed25519 parent-profile signature was verified.
#[derive(Clone, Debug)]
pub struct VerifiedParentProfile {
    configuration: LoadedConfiguration,
    verifying_key_sha256: String,
    signature_sha256: String,
}

impl VerifiedParentProfile {
    /// Returns the signed configuration identity used as the authority ceiling.
    #[must_use]
    pub fn configuration_sha256(&self) -> &str {
        self.configuration.sha256()
    }

    /// Returns the hash of the trusted Ed25519 verification key.
    #[must_use]
    pub fn verifying_key_sha256(&self) -> &str {
        &self.verifying_key_sha256
    }

    /// Returns the hash of the verified detached signature.
    #[must_use]
    pub fn signature_sha256(&self) -> &str {
        &self.signature_sha256
    }
}

impl LoadedConfiguration {
    /// Returns the configured profile identity.
    #[must_use]
    pub fn profile_id(&self) -> &str {
        &self.configuration.core.profile_id
    }

    /// Returns the canonical SHA-256 identity.
    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    /// Returns deterministic canonical bytes suitable for an atomic write.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
}

/// Classifies whether a configuration change expands or restricts effective scope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChangeImpact {
    /// The change expands an authority ceiling or enables a broader operation.
    AuthorityBroadening,
    /// The change removes or narrows an authority ceiling.
    AuthorityRestriction,
    /// The change increases a bounded resource limit.
    ResourceIncrease,
    /// The change decreases a bounded resource limit.
    ResourceRestriction,
    /// The change is operational and does not directly classify as authority or budget.
    Operational,
}

/// A redacted configuration difference containing hashes rather than raw values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigurationChange {
    path: String,
    before_sha256: String,
    after_sha256: String,
    impact: ChangeImpact,
}

impl ConfigurationChange {
    /// Returns the JSON-pointer-like path of the change.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the hash of the canonical value before the change.
    #[must_use]
    pub fn before_sha256(&self) -> &str {
        &self.before_sha256
    }

    /// Returns the hash of the canonical value after the change.
    #[must_use]
    pub fn after_sha256(&self) -> &str {
        &self.after_sha256
    }

    /// Returns the classified security or resource impact.
    #[must_use]
    pub const fn impact(&self) -> ChangeImpact {
        self.impact
    }
}

/// A deterministic, redacted comparison between two validated configurations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigurationDiff {
    before_sha256: String,
    after_sha256: String,
    changes: Vec<ConfigurationChange>,
    authority_broadening: bool,
}

impl ConfigurationDiff {
    /// Returns the identity of the complete configuration before the changes.
    #[must_use]
    pub fn before_sha256(&self) -> &str {
        &self.before_sha256
    }

    /// Returns the identity of the complete configuration after the changes.
    #[must_use]
    pub fn after_sha256(&self) -> &str {
        &self.after_sha256
    }

    /// Returns all stable, path-sorted change records.
    #[must_use]
    pub fn changes(&self) -> &[ConfigurationChange] {
        &self.changes
    }

    /// Returns true when at least one change broadens an authority ceiling.
    #[must_use]
    pub fn broadens_authority(&self) -> bool {
        self.authority_broadening
    }
}

/// Identifies an untrusted configuration channel that may only restrict authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RestrictedConfigurationSource {
    /// A candidate read from a local configuration file.
    ConfigurationFile,
    /// A candidate assembled from process environment input.
    Environment,
    /// A candidate supplied by a child configuration or subordinate task.
    ChildProfile,
    /// A candidate discovered inside a repository or workspace.
    Repository,
    /// A candidate proposed by model output.
    ModelOutput,
}

impl RestrictedConfigurationSource {
    /// Returns the stable source identifier used in bounded evidence.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ConfigurationFile => "configuration-file",
            Self::Environment => "environment",
            Self::ChildProfile => "child-profile",
            Self::Repository => "repository",
            Self::ModelOutput => "model-output",
        }
    }
}

/// A validated candidate proven not to exceed its trusted parent configuration.
#[derive(Clone, Debug)]
pub struct RestrictedConfigurationOutcome {
    source: RestrictedConfigurationSource,
    parent_sha256: String,
    configuration: LoadedConfiguration,
    diff: ConfigurationDiff,
}

/// Identifies the product result boundary receiving a configuration identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConfigurationResultKind {
    /// A bounded session result or session audit result.
    Session,
    /// A bounded release result.
    Release,
}

impl ConfigurationResultKind {
    /// Returns the stable result-kind identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Session => "session",
            Self::Release => "release",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct ResultConfigurationIdentity {
    profile_id: String,
    sha256: String,
}

#[derive(Serialize)]
struct ConfigurationBoundResultIdentityMaterial<'a> {
    schema_version: u32,
    record_type: &'static str,
    result_kind: ConfigurationResultKind,
    result_id: &'a str,
    payload_sha256: &'a str,
    configuration: &'a ResultConfigurationIdentity,
}

/// A session or release result bound to one exact validated configuration.
#[derive(Clone, Debug, Serialize)]
pub struct ConfigurationBoundResult {
    schema_version: u32,
    record_type: &'static str,
    result_kind: ConfigurationResultKind,
    result_id: String,
    payload_sha256: String,
    configuration: ResultConfigurationIdentity,
    record_sha256: String,
    #[serde(skip)]
    canonical_bytes: Vec<u8>,
}

impl ConfigurationBoundResult {
    /// Returns whether this record binds a session or release result.
    #[must_use]
    pub const fn result_kind(&self) -> ConfigurationResultKind {
        self.result_kind
    }

    /// Returns the caller-provided bounded result identity.
    #[must_use]
    pub fn result_id(&self) -> &str {
        &self.result_id
    }

    /// Returns the hash of the result payload kept outside this record.
    #[must_use]
    pub fn payload_sha256(&self) -> &str {
        &self.payload_sha256
    }

    /// Returns the exact configuration profile identity.
    #[must_use]
    pub fn configuration_profile_id(&self) -> &str {
        &self.configuration.profile_id
    }

    /// Returns the exact canonical configuration hash.
    #[must_use]
    pub fn configuration_sha256(&self) -> &str {
        &self.configuration.sha256
    }

    /// Returns the hash of the record identity material.
    #[must_use]
    pub fn record_sha256(&self) -> &str {
        &self.record_sha256
    }

    /// Returns deterministic canonical JSON containing identities but no raw configuration.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
}

impl RestrictedConfigurationOutcome {
    /// Returns the untrusted channel through which the candidate arrived.
    #[must_use]
    pub const fn source(&self) -> RestrictedConfigurationSource {
        self.source
    }

    /// Returns the trusted parent identity used for the authority comparison.
    #[must_use]
    pub fn parent_sha256(&self) -> &str {
        &self.parent_sha256
    }

    /// Returns the validated, non-broadening candidate.
    #[must_use]
    pub const fn configuration(&self) -> &LoadedConfiguration {
        &self.configuration
    }

    /// Returns the redacted difference from the parent configuration.
    #[must_use]
    pub const fn diff(&self) -> &ConfigurationDiff {
        &self.diff
    }
}

/// The deterministic result of migrating a legacy configuration to version 1.
#[derive(Clone, Debug)]
pub struct MigrationOutcome {
    source_version: u32,
    target_version: u32,
    changes: Vec<String>,
    configuration: LoadedConfiguration,
}

impl MigrationOutcome {
    /// Returns the source schema version.
    #[must_use]
    pub const fn source_version(&self) -> u32 {
        self.source_version
    }

    /// Returns the target schema version.
    #[must_use]
    pub const fn target_version(&self) -> u32 {
        self.target_version
    }

    /// Returns the fixed migration operations in application order.
    #[must_use]
    pub fn changes(&self) -> &[String] {
        &self.changes
    }

    /// Returns the validated migrated configuration.
    #[must_use]
    pub const fn configuration(&self) -> &LoadedConfiguration {
        &self.configuration
    }
}

/// Evidence returned after an atomic configuration replacement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplyReceipt {
    previous_sha256: String,
    applied_sha256: String,
    backup_path: PathBuf,
}

/// Evidence returned after a durable version 0-to-1 file migration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationApplyReceipt {
    previous_sha256: String,
    migrated_sha256: String,
    backup_path: PathBuf,
    changes: Vec<String>,
}

impl MigrationApplyReceipt {
    /// Returns the exact pre-migration file identity retained in the backup.
    #[must_use]
    pub fn previous_sha256(&self) -> &str {
        &self.previous_sha256
    }

    /// Returns the canonical migrated configuration identity.
    #[must_use]
    pub fn migrated_sha256(&self) -> &str {
        &self.migrated_sha256
    }

    /// Returns the content-addressed version 0 backup path.
    #[must_use]
    pub fn backup_path(&self) -> &Path {
        &self.backup_path
    }

    /// Returns the deterministic migration operations in application order.
    #[must_use]
    pub fn changes(&self) -> &[String] {
        &self.changes
    }
}

/// Evidence returned by an exact, repeatable migration rollback.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationRollbackReceipt {
    restored_sha256: String,
    already_restored: bool,
}

impl MigrationRollbackReceipt {
    /// Returns the exact restored version 0 file identity.
    #[must_use]
    pub fn restored_sha256(&self) -> &str {
        &self.restored_sha256
    }

    /// Reports whether the requested version 0 state was already selected.
    #[must_use]
    pub const fn already_restored(&self) -> bool {
        self.already_restored
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MigrationDurableTransition {
    Backup,
    Candidate,
    Target,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TransitionBoundary {
    Before,
    After,
}

impl ApplyReceipt {
    /// Returns the previous configuration identity.
    #[must_use]
    pub fn previous_sha256(&self) -> &str {
        &self.previous_sha256
    }

    /// Returns the applied configuration identity.
    #[must_use]
    pub fn applied_sha256(&self) -> &str {
        &self.applied_sha256
    }

    /// Returns the content-addressed backup path retained beside the target.
    #[must_use]
    pub fn backup_path(&self) -> &Path {
        &self.backup_path
    }
}

/// Fail-closed version 1 configuration loader and local recovery manager.
#[derive(Clone, Debug)]
pub struct ConfigurationManager {
    maximum_bytes: usize,
}

impl Default for ConfigurationManager {
    fn default() -> Self {
        Self {
            maximum_bytes: DEFAULT_MAXIMUM_BYTES,
        }
    }
}

impl ConfigurationManager {
    /// Creates a manager with an explicit positive input-size bound.
    pub fn with_maximum_bytes(maximum_bytes: usize) -> Result<Self, ConfigurationError> {
        if maximum_bytes == 0 {
            return Err(ConfigurationError::new(
                ErrorCode::ContractViolation,
                "maximum configuration bytes must be positive",
            ));
        }
        Ok(Self { maximum_bytes })
    }

    /// Loads and validates one complete version 1 configuration from bytes.
    pub fn load_bytes(&self, input: &[u8]) -> Result<LoadedConfiguration, ConfigurationError> {
        if input.len() > self.maximum_bytes {
            return Err(ConfigurationError::new(
                ErrorCode::TooLarge,
                "configuration exceeds the configured byte limit",
            ));
        }
        let value = parse_unique_json(input)?;
        let version = value
            .get("schema_version")
            .and_then(Value::as_u64)
            .ok_or_else(|| {
                ConfigurationError::new(
                    ErrorCode::UnsupportedVersion,
                    "configuration schema version is missing or invalid",
                )
            })?;
        if version != u64::from(CURRENT_SCHEMA_VERSION) {
            return Err(ConfigurationError::new(
                ErrorCode::UnsupportedVersion,
                "configuration schema version is unsupported",
            ));
        }
        let configuration: AgentConfiguration = serde_json::from_value(value).map_err(|_| {
            ConfigurationError::new(
                ErrorCode::ContractViolation,
                "configuration does not match the closed version 1 contract",
            )
        })?;
        validate_configuration(&configuration)?;
        loaded(configuration)
    }

    /// Loads a configuration from a file without retaining its private path in errors.
    pub fn load_path(&self, path: &Path) -> Result<LoadedConfiguration, ConfigurationError> {
        let input = fs::read(path).map_err(|_| {
            ConfigurationError::new(ErrorCode::Io, "configuration file could not be read")
        })?;
        self.load_bytes(&input)
    }

    /// Returns an explicit strict-local, read-only, no-tool baseline.
    pub fn safe_defaults(&self) -> Result<LoadedConfiguration, ConfigurationError> {
        self.load_bytes(STRICT_LOCAL_PROFILE)
    }

    /// Verifies one detached Ed25519 signature before admitting a parent authority ceiling.
    pub fn verify_parent_profile(
        &self,
        configuration: &LoadedConfiguration,
        verifying_key: &[u8; 32],
        signature: &[u8; 64],
    ) -> Result<VerifiedParentProfile, ConfigurationError> {
        let key = VerifyingKey::from_bytes(verifying_key).map_err(|_| {
            ConfigurationError::new(
                ErrorCode::ParentSignatureInvalid,
                "parent profile signature verification failed",
            )
        })?;
        let signature = Signature::from_bytes(signature);
        let material = parent_profile_signature_material(configuration)?;
        key.verify_strict(&material, &signature).map_err(|_| {
            ConfigurationError::new(
                ErrorCode::ParentSignatureInvalid,
                "parent profile signature verification failed",
            )
        })?;
        Ok(VerifiedParentProfile {
            configuration: configuration.clone(),
            verifying_key_sha256: sha256(verifying_key),
            signature_sha256: sha256(signature.to_bytes().as_slice()),
        })
    }

    /// Migrates the single supported legacy version without adding authority.
    pub fn migrate_v0(&self, input: &[u8]) -> Result<MigrationOutcome, ConfigurationError> {
        if input.len() > self.maximum_bytes {
            return Err(ConfigurationError::new(
                ErrorCode::TooLarge,
                "legacy configuration exceeds the configured byte limit",
            ));
        }
        let mut value = parse_unique_json(input)?;
        if value.get("schema_version").and_then(Value::as_u64) != Some(0) {
            return Err(ConfigurationError::new(
                ErrorCode::UnsupportedVersion,
                "migration accepts only legacy schema version 0",
            ));
        }
        let object = value.as_object_mut().ok_or_else(|| {
            ConfigurationError::new(
                ErrorCode::ContractViolation,
                "legacy configuration must be an object",
            )
        })?;
        object.insert("schema_version".into(), Value::from(1));
        let mut changes = vec!["/schema_version:0-to-1".to_owned()];
        for section in section_names() {
            let section_object = object
                .get_mut(section)
                .and_then(Value::as_object_mut)
                .ok_or_else(|| {
                    ConfigurationError::new(
                        ErrorCode::ContractViolation,
                        "legacy configuration is missing a required section",
                    )
                })?;
            if section_object
                .insert("schema_version".into(), Value::from(1))
                .is_some()
            {
                return Err(ConfigurationError::new(
                    ErrorCode::ContractViolation,
                    "legacy section already contains a reserved schema version",
                ));
            }
            changes.push(format!("/{section}/schema_version:added-1"));
        }
        let core = object
            .get_mut("core")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| {
                ConfigurationError::new(
                    ErrorCode::ContractViolation,
                    "legacy core configuration is invalid",
                )
            })?;
        let profile = core.remove("profile").ok_or_else(|| {
            ConfigurationError::new(
                ErrorCode::ContractViolation,
                "legacy profile identity is missing",
            )
        })?;
        if core.insert("profile_id".into(), profile).is_some() {
            return Err(ConfigurationError::new(
                ErrorCode::ContractViolation,
                "legacy profile identity is ambiguous",
            ));
        }
        changes.push("/core/profile:renamed-to-profile_id".to_owned());
        let migrated = serde_json::to_vec(&value).map_err(|_| {
            ConfigurationError::new(
                ErrorCode::ContractViolation,
                "migrated configuration could not be serialized",
            )
        })?;
        let configuration = self.load_bytes(&migrated)?;
        Ok(MigrationOutcome {
            source_version: 0,
            target_version: 1,
            changes,
            configuration,
        })
    }

    /// Durably migrates a version 0 configuration file while retaining its exact preimage.
    pub fn migrate_path_v0(
        &self,
        target: &Path,
    ) -> Result<MigrationApplyReceipt, ConfigurationError> {
        self.migrate_path_v0_with_observer(target, |_, _| Ok(()))
    }

    fn migrate_path_v0_with_observer<F>(
        &self,
        target: &Path,
        mut observer: F,
    ) -> Result<MigrationApplyReceipt, ConfigurationError>
    where
        F: FnMut(MigrationDurableTransition, TransitionBoundary) -> Result<(), ConfigurationError>,
    {
        let source = fs::read(target).map_err(|_| {
            ConfigurationError::new(ErrorCode::Io, "legacy configuration file could not be read")
        })?;
        let migration = self.migrate_v0(&source)?;
        let previous_sha256 = sha256(&source);
        let migrated_sha256 = migration.configuration.sha256.clone();
        let migrated_bytes = migration.configuration.canonical_bytes.clone();
        let backup_path = backup_path(target, &previous_sha256)?;
        let temporary_path = temporary_path(target, &migrated_sha256)?;

        observer(
            MigrationDurableTransition::Backup,
            TransitionBoundary::Before,
        )?;
        write_new_or_verify(&backup_path, &source)?;
        sync_parent_directory(&backup_path)?;
        observer(
            MigrationDurableTransition::Backup,
            TransitionBoundary::After,
        )?;

        observer(
            MigrationDurableTransition::Candidate,
            TransitionBoundary::Before,
        )?;
        prepare_migration_candidate(&temporary_path, &migrated_bytes)?;
        sync_parent_directory(&temporary_path)?;
        observer(
            MigrationDurableTransition::Candidate,
            TransitionBoundary::After,
        )?;

        observer(
            MigrationDurableTransition::Target,
            TransitionBoundary::Before,
        )?;
        let current = fs::read(target).map_err(|_| {
            ConfigurationError::new(ErrorCode::Io, "migration preimage could not be read")
        })?;
        if sha256(&current) != previous_sha256 {
            return Err(ConfigurationError::new(
                ErrorCode::Conflict,
                "migration preimage changed before publication",
            ));
        }
        fs::rename(&temporary_path, target).map_err(|_| {
            ConfigurationError::new(ErrorCode::Io, "migrated configuration publish failed")
        })?;
        sync_parent_directory(target)?;
        observer(
            MigrationDurableTransition::Target,
            TransitionBoundary::After,
        )?;

        let observed = self.load_path(target)?;
        if observed.sha256 != migrated_sha256 {
            return Err(ConfigurationError::new(
                ErrorCode::Conflict,
                "migrated configuration identity did not verify",
            ));
        }
        Ok(MigrationApplyReceipt {
            previous_sha256,
            migrated_sha256,
            backup_path,
            changes: migration.changes,
        })
    }

    /// Restores an exact version 0 migration backup and succeeds on an identical retry.
    pub fn rollback_migration(
        &self,
        target: &Path,
        backup: &Path,
        expected_migrated_sha256: &str,
    ) -> Result<MigrationRollbackReceipt, ConfigurationError> {
        let backup_bytes = fs::read(backup).map_err(|_| {
            ConfigurationError::new(ErrorCode::Io, "migration backup could not be read")
        })?;
        self.migrate_v0(&backup_bytes)?;
        let restored_sha256 = sha256(&backup_bytes);
        let current_bytes = fs::read(target).map_err(|_| {
            ConfigurationError::new(ErrorCode::Io, "current configuration could not be read")
        })?;
        let current_sha256 = sha256(&current_bytes);
        if current_sha256 == restored_sha256 {
            return Ok(MigrationRollbackReceipt {
                restored_sha256,
                already_restored: true,
            });
        }
        if current_sha256 != expected_migrated_sha256 {
            return Err(ConfigurationError::new(
                ErrorCode::Conflict,
                "migration rollback preimage does not match current configuration",
            ));
        }
        self.load_bytes(&current_bytes)?;
        atomic_replace(target, &backup_bytes, &restored_sha256)?;
        let observed = fs::read(target).map_err(|_| {
            ConfigurationError::new(ErrorCode::Io, "restored configuration could not be read")
        })?;
        if sha256(&observed) != restored_sha256 {
            return Err(ConfigurationError::new(
                ErrorCode::Conflict,
                "migration rollback identity did not verify",
            ));
        }
        Ok(MigrationRollbackReceipt {
            restored_sha256,
            already_restored: false,
        })
    }

    /// Compares two validated configurations without retaining raw changed values.
    pub fn diff(
        &self,
        before: &LoadedConfiguration,
        after: &LoadedConfiguration,
    ) -> Result<ConfigurationDiff, ConfigurationError> {
        let before_value = serde_json::to_value(&before.configuration).map_err(|_| {
            ConfigurationError::new(ErrorCode::ContractViolation, "configuration diff failed")
        })?;
        let after_value = serde_json::to_value(&after.configuration).map_err(|_| {
            ConfigurationError::new(ErrorCode::ContractViolation, "configuration diff failed")
        })?;
        let mut changes = Vec::new();
        collect_changes("", &before_value, &after_value, &mut changes)?;
        changes.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(ConfigurationDiff {
            before_sha256: before.sha256.clone(),
            after_sha256: after.sha256.clone(),
            changes,
            authority_broadening: !authority_is_subset(&before.configuration, &after.configuration),
        })
    }

    /// Loads an untrusted full candidate only when it cannot exceed its trusted parent.
    pub fn load_restricted_candidate(
        &self,
        parent: &VerifiedParentProfile,
        source: RestrictedConfigurationSource,
        candidate: &[u8],
    ) -> Result<RestrictedConfigurationOutcome, ConfigurationError> {
        let configuration = self.load_bytes(candidate)?;
        if !authority_is_subset(
            &parent.configuration.configuration,
            &configuration.configuration,
        ) {
            return Err(ConfigurationError::new(
                ErrorCode::AuthorityBroadening,
                "untrusted configuration candidate exceeds its parent authority",
            ));
        }
        let diff = self.diff(&parent.configuration, &configuration)?;
        Ok(RestrictedConfigurationOutcome {
            source,
            parent_sha256: parent.configuration.sha256.clone(),
            configuration,
            diff,
        })
    }

    /// Reads a local candidate file and applies the same signed-parent restriction boundary.
    pub fn load_restricted_path(
        &self,
        parent: &VerifiedParentProfile,
        path: &Path,
    ) -> Result<RestrictedConfigurationOutcome, ConfigurationError> {
        let candidate = fs::read(path).map_err(|_| {
            ConfigurationError::new(
                ErrorCode::Io,
                "restricted configuration file could not be read",
            )
        })?;
        self.load_restricted_candidate(
            parent,
            RestrictedConfigurationSource::ConfigurationFile,
            &candidate,
        )
    }

    /// Binds a session result to the exact validated configuration that produced it.
    pub fn bind_session_result(
        &self,
        configuration: &LoadedConfiguration,
        result_id: &str,
        payload_sha256: &str,
    ) -> Result<ConfigurationBoundResult, ConfigurationError> {
        bind_configuration_result(
            configuration,
            ConfigurationResultKind::Session,
            result_id,
            payload_sha256,
        )
    }

    /// Binds a release result to the exact validated configuration that produced it.
    pub fn bind_release_result(
        &self,
        configuration: &LoadedConfiguration,
        result_id: &str,
        payload_sha256: &str,
    ) -> Result<ConfigurationBoundResult, ConfigurationError> {
        bind_configuration_result(
            configuration,
            ConfigurationResultKind::Release,
            result_id,
            payload_sha256,
        )
    }

    /// Atomically applies a valid configuration after retaining a content-addressed backup.
    pub fn apply_with_backup(
        &self,
        target: &Path,
        candidate: &[u8],
    ) -> Result<ApplyReceipt, ConfigurationError> {
        let current_bytes = fs::read(target).map_err(|_| {
            ConfigurationError::new(ErrorCode::Io, "current configuration could not be read")
        })?;
        let current = self.load_bytes(&current_bytes)?;
        let next = self.load_bytes(candidate)?;
        if current.sha256 == next.sha256 {
            return Err(ConfigurationError::new(
                ErrorCode::Conflict,
                "candidate configuration is identical to current configuration",
            ));
        }
        let backup_path = backup_path(target, &current.sha256)?;
        write_new_or_verify(&backup_path, &current.canonical_bytes)?;
        sync_parent_directory(&backup_path)?;
        atomic_replace(target, &next.canonical_bytes, &next.sha256)?;
        let observed = self.load_path(target)?;
        if observed.sha256 != next.sha256 {
            return Err(ConfigurationError::new(
                ErrorCode::Conflict,
                "applied configuration identity did not verify",
            ));
        }
        Ok(ApplyReceipt {
            previous_sha256: current.sha256,
            applied_sha256: next.sha256,
            backup_path,
        })
    }

    /// Restores a retained backup only when the current identity matches the caller's preimage.
    pub fn rollback(
        &self,
        target: &Path,
        backup: &Path,
        expected_current_sha256: &str,
    ) -> Result<LoadedConfiguration, ConfigurationError> {
        let current = self.load_path(target)?;
        if current.sha256 != expected_current_sha256 {
            return Err(ConfigurationError::new(
                ErrorCode::Conflict,
                "rollback preimage does not match current configuration",
            ));
        }
        let backup_configuration = self.load_path(backup)?;
        atomic_replace(
            target,
            &backup_configuration.canonical_bytes,
            &backup_configuration.sha256,
        )?;
        let restored = self.load_path(target)?;
        if restored.sha256 != backup_configuration.sha256 {
            return Err(ConfigurationError::new(
                ErrorCode::Conflict,
                "rollback configuration identity did not verify",
            ));
        }
        Ok(restored)
    }
}

fn parent_profile_signature_material(
    configuration: &LoadedConfiguration,
) -> Result<Vec<u8>, ConfigurationError> {
    serde_json::to_vec(&ParentProfileSignatureMaterial {
        schema_version: 1,
        record_type: "agentmage-parent-profile-authority",
        profile_id: configuration.profile_id(),
        configuration_sha256: configuration.sha256(),
    })
    .map_err(|_| {
        ConfigurationError::new(
            ErrorCode::ContractViolation,
            "parent profile signature material could not be serialized",
        )
    })
}

fn loaded(configuration: AgentConfiguration) -> Result<LoadedConfiguration, ConfigurationError> {
    let canonical_bytes = serde_json::to_vec(&configuration).map_err(|_| {
        ConfigurationError::new(
            ErrorCode::ContractViolation,
            "configuration could not be canonicalized",
        )
    })?;
    let sha256 = sha256(&canonical_bytes);
    Ok(LoadedConfiguration {
        configuration,
        canonical_bytes,
        sha256,
    })
}

fn bind_configuration_result(
    configuration: &LoadedConfiguration,
    result_kind: ConfigurationResultKind,
    result_id: &str,
    payload_sha256: &str,
) -> Result<ConfigurationBoundResult, ConfigurationError> {
    if !identifier(result_id) || !sha256_text(payload_sha256) {
        return contract_error();
    }
    let configuration_identity = ResultConfigurationIdentity {
        profile_id: configuration.profile_id().to_owned(),
        sha256: configuration.sha256().to_owned(),
    };
    let identity_material = ConfigurationBoundResultIdentityMaterial {
        schema_version: 1,
        record_type: "configuration-bound-result",
        result_kind,
        result_id,
        payload_sha256,
        configuration: &configuration_identity,
    };
    let identity_bytes = serde_json::to_vec(&identity_material).map_err(|_| {
        ConfigurationError::new(
            ErrorCode::ContractViolation,
            "configuration-bound result could not be serialized",
        )
    })?;
    let mut record = ConfigurationBoundResult {
        schema_version: 1,
        record_type: "configuration-bound-result",
        result_kind,
        result_id: result_id.to_owned(),
        payload_sha256: payload_sha256.to_owned(),
        configuration: configuration_identity,
        record_sha256: sha256(&identity_bytes),
        canonical_bytes: Vec::new(),
    };
    record.canonical_bytes = serde_json::to_vec(&record).map_err(|_| {
        ConfigurationError::new(
            ErrorCode::ContractViolation,
            "configuration-bound result could not be serialized",
        )
    })?;
    Ok(record)
}

fn validate_configuration(value: &AgentConfiguration) -> Result<(), ConfigurationError> {
    if value.schema_version != 1
        || section_versions(value).iter().any(|version| *version != 1)
        || value.core.product_id != "agentmage"
        || value.core.startup_failure_policy != "fail-closed"
        || value.core.inheritance_mode != "restrict-only"
    {
        return contract_error();
    }
    let modes = [
        "development",
        "synthetic-test",
        "strict-local-read-only",
        "knowledge",
        "write",
        "coding",
        "network-enabled",
    ];
    if !modes.contains(&value.core.configuration_mode.as_str())
        || (value.core.configuration_mode != "network-enabled" && !value.core.strict_local)
        || !identifier(&value.core.profile_id)
    {
        return contract_error();
    }
    let platforms = ["macos-arm64", "fedora-x86_64", "ubuntu-x86_64"];
    if !platforms.contains(&value.platform.platform_id.as_str())
        || !identifier(&value.platform.adapter_id)
        || !identifier(&value.platform.sandbox_profile_id)
        || !["descriptor-relative", "security-scoped-bookmark"]
            .contains(&value.platform.path_identity_strategy.as_str())
        || !["secret-service", "keychain"].contains(&value.platform.secret_provider_id.as_str())
        || !value.platform.standard_user_required
        || value.platform.administrator_required
        || value.platform.ambient_dependency_policy != "deny"
    {
        return contract_error();
    }
    if !identifier(&value.model.model_profile_id)
        || !relative_path(&value.model.manifest_path)
        || !identifier(&value.model.runtime_adapter_id)
        || !["local-stdio", "local-unix-socket"].contains(&value.model.transport.as_str())
        || value.model.network_access
        || value.model.automatic_routing
        || !(0.0..=2.0).contains(&value.model.decoding.temperature)
        || !(1..=1000).contains(&value.model.decoding.top_k)
        || u64::from(value.model.decoding.seed) > MAXIMUM_SCHEMA_INTEGER
        || value.model.decoding.maximum_context_tokens == 0
        || u64::from(value.model.decoding.maximum_context_tokens) > MAXIMUM_SCHEMA_INTEGER
        || value.model.decoding.maximum_output_tokens == 0
        || u64::from(value.model.decoding.maximum_output_tokens) > MAXIMUM_SCHEMA_INTEGER
        || (value.model.decoding.deterministic
            && (value.model.decoding.temperature != 0.0 || value.model.decoding.top_k != 1))
    {
        return contract_error();
    }
    if !(1..=32).contains(&value.workspace.maximum_roots)
        || value.workspace.default_access != "deny"
        || !["deny", "same-root-only"].contains(&value.workspace.symlink_policy.as_str())
        || value.workspace.roots.is_empty()
        || value.workspace.roots.len() > 32
        || value.workspace.roots.len() > value.workspace.maximum_roots as usize
    {
        return contract_error();
    }
    let mut root_ids = BTreeSet::new();
    for (index, root) in value.workspace.roots.iter().enumerate() {
        if !identifier(&root.root_id)
            || !root_ids.insert(&root.root_id)
            || value.workspace.roots[..index].contains(root)
            || !["managed-relative", "platform-resolved-handle"]
                .contains(&root.locator_kind.as_str())
            || root.locator.is_empty()
            || root.locator.len() > 1024
            || !["read-only", "read-write"].contains(&root.access.as_str())
            || root.follow_mount_changes
        {
            return contract_error();
        }
    }
    if !relative_path(&value.tool.registry_path)
        || value.tool.unregistered_tool_policy != "deny"
        || value.tool.tools.len() > 256
    {
        return contract_error();
    }
    let allowed_capabilities: BTreeSet<&str> = value
        .permission
        .allowed_capabilities
        .iter()
        .map(String::as_str)
        .collect();
    if allowed_capabilities.len() != value.permission.allowed_capabilities.len()
        || value.permission.allowed_capabilities.len() > 256
        || value
            .permission
            .allowed_capabilities
            .iter()
            .any(|capability| !identifier(capability))
    {
        return contract_error();
    }
    let mut tool_ids = BTreeSet::new();
    for (index, tool) in value.tool.tools.iter().enumerate() {
        let capabilities: BTreeSet<&str> = tool
            .required_capabilities
            .iter()
            .map(String::as_str)
            .collect();
        if !identifier(&tool.tool_id)
            || !tool_ids.insert(&tool.tool_id)
            || value.tool.tools[..index].contains(tool)
            || !semantic_version(&tool.version)
            || !sha256_text(&tool.integrity_sha256)
            || capabilities.len() != tool.required_capabilities.len()
            || tool.required_capabilities.len() > 32
            || capabilities
                .iter()
                .any(|capability| !identifier(capability))
            || (tool.enabled
                && capabilities
                    .iter()
                    .any(|capability| !allowed_capabilities.contains(capability)))
            || !["none", "read", "write", "process", "network"]
                .contains(&tool.side_effect_class.as_str())
        {
            return contract_error();
        }
    }
    let grant = &value.permission.grant_policy;
    if value.permission.default_effect != "deny"
        || value.permission.authority_inheritance != "restrict-only"
        || value.permission.model_authority != "none"
        || !grant.single_use
        || !grant.expiry_required
        || !grant.exact_scope_required
        || grant.transferable
        || grant.replay_policy != "deny"
        || value.permission.network.mode != "deny-all"
        || !value.permission.network.allowed_endpoints.is_empty()
    {
        return contract_error();
    }
    if !(1..=MAXIMUM_SCHEMA_BYTES).contains(&value.budget.maximum_input_bytes)
        || value.budget.maximum_output_bytes == 0
        || value.budget.maximum_output_bytes > MAXIMUM_SCHEMA_BYTES
        || value.budget.maximum_file_count == 0
        || u64::from(value.budget.maximum_file_count) > MAXIMUM_SCHEMA_INTEGER
        || !(1..=1024).contains(&value.budget.maximum_tree_depth)
        || value.budget.maximum_context_tokens == 0
        || u64::from(value.budget.maximum_context_tokens) > MAXIMUM_SCHEMA_INTEGER
        || value.budget.maximum_output_tokens == 0
        || u64::from(value.budget.maximum_output_tokens) > MAXIMUM_SCHEMA_INTEGER
        || value.budget.maximum_operation_milliseconds == 0
        || value.budget.maximum_operation_milliseconds > 86_400_000
        || value.budget.maximum_memory_bytes == 0
        || value.budget.maximum_memory_bytes > MAXIMUM_SCHEMA_BYTES
        || value.budget.maximum_processes > 64
        || !(1..=64).contains(&value.budget.maximum_concurrency)
        || value.model.decoding.maximum_context_tokens > value.budget.maximum_context_tokens
        || value.model.decoding.maximum_output_tokens > value.budget.maximum_output_tokens
    {
        return contract_error();
    }
    let logging = &value.logging;
    if !["error", "warn", "info", "debug"].contains(&logging.level.as_str())
        || !logging.structured
        || logging.destination.kind != "managed-local-file"
        || !relative_path(&logging.destination.managed_relative_path)
        || !(256..=1_048_576).contains(&logging.maximum_event_bytes)
        || logging.redaction.secret_values != "redact"
        || logging.redaction.credentials != "redact"
        || logging.redaction.private_paths != "redact"
        || logging.redaction.environment_values != "drop"
        || logging.record_prompts
        || logging.record_tool_arguments
        || logging.record_environment_values
    {
        return contract_error();
    }
    let retention = &value.retention;
    if retention.sessions_days > 3650
        || !(1..=3650).contains(&retention.receipts_days)
        || retention.logs_days > 365
        || retention.temporary_artifacts_minutes > 10_080
        || retention.expired_data_action != "delete-on-next-startup-or-maintenance"
        || !["exclude-by-default", "user-managed-encrypted"]
            .contains(&retention.backup_policy.as_str())
        || retention.secure_deletion_claim != "best-effort-no-physical-erasure-claim"
    {
        return contract_error();
    }
    let skill_capabilities: BTreeSet<&str> = value
        .skill
        .capability_ceiling
        .iter()
        .map(String::as_str)
        .collect();
    let catalog_paths: BTreeSet<&str> = value
        .skill
        .catalog_paths
        .iter()
        .map(String::as_str)
        .collect();
    if value.skill.enablement != "manual-only"
        || value.skill.unsigned_skill_policy != "disabled"
        || value.skill.self_modification != "deny"
        || value.skill.maximum_enabled_skills > 256
        || value.skill.catalog_paths.len() > 32
        || catalog_paths.len() != value.skill.catalog_paths.len()
        || catalog_paths.iter().any(|path| !relative_path(path))
        || skill_capabilities.len() != value.skill.capability_ceiling.len()
        || value.skill.capability_ceiling.len() > 64
        || skill_capabilities
            .iter()
            .any(|capability| !allowed_capabilities.contains(capability))
    {
        return contract_error();
    }
    let shell = &value.shell;
    if !identifier(&shell.shell_id)
        || !["vscode-native-chat", "host-cli"].contains(&shell.surface_id.as_str())
        || !["local-stdio", "local-ipc"].contains(&shell.transport.as_str())
        || shell.launch_privilege != "standard-user"
        || shell.network_access
        || shell.direct_command_execution
        || shell.model_to_tool_channel != "prohibited"
        || !["status-command", "native-chat-status"].contains(&shell.diagnostics_surface.as_str())
    {
        return contract_error();
    }
    Ok(())
}

fn contract_error<T>() -> Result<T, ConfigurationError> {
    Err(ConfigurationError::new(
        ErrorCode::ContractViolation,
        "configuration violates a version 1 safety invariant",
    ))
}

fn authority_is_subset(parent: &AgentConfiguration, child: &AgentConfiguration) -> bool {
    parent.platform == child.platform
        && parent.core.configuration_mode == child.core.configuration_mode
        && (!parent.core.strict_local || child.core.strict_local)
        && model_is_subset(&parent.model, &child.model)
        && workspace_is_subset(&parent.workspace, &child.workspace)
        && tools_are_subset(&parent.tool, &child.tool)
        && string_values_are_subset(
            &child.permission.allowed_capabilities,
            &parent.permission.allowed_capabilities,
        )
        && budget_is_subset(&parent.budget, &child.budget)
        && logging_is_subset(&parent.logging, &child.logging)
        && retention_is_subset(&parent.retention, &child.retention)
        && skills_are_subset(&parent.skill, &child.skill)
        && parent.shell == child.shell
}

fn model_is_subset(parent: &ModelConfiguration, child: &ModelConfiguration) -> bool {
    (!child.enabled || parent.enabled)
        && parent.model_profile_id == child.model_profile_id
        && parent.manifest_path == child.manifest_path
        && parent.runtime_adapter_id == child.runtime_adapter_id
        && parent.transport == child.transport
        && parent.decoding.deterministic == child.decoding.deterministic
        && parent.decoding.temperature == child.decoding.temperature
        && parent.decoding.top_k == child.decoding.top_k
        && parent.decoding.seed == child.decoding.seed
        && child.decoding.maximum_context_tokens <= parent.decoding.maximum_context_tokens
        && child.decoding.maximum_output_tokens <= parent.decoding.maximum_output_tokens
}

fn workspace_is_subset(parent: &WorkspaceConfiguration, child: &WorkspaceConfiguration) -> bool {
    child.maximum_roots <= parent.maximum_roots
        && symlink_policy_rank(&child.symlink_policy) <= symlink_policy_rank(&parent.symlink_policy)
        && child.roots.iter().all(|child_root| {
            parent.roots.iter().any(|parent_root| {
                parent_root.root_id == child_root.root_id
                    && parent_root.locator_kind == child_root.locator_kind
                    && parent_root.locator == child_root.locator
                    && access_rank(&child_root.access) <= access_rank(&parent_root.access)
            })
        })
}

fn tools_are_subset(parent: &ToolConfiguration, child: &ToolConfiguration) -> bool {
    parent.registry_path == child.registry_path
        && child.tools.iter().all(|child_tool| {
            parent.tools.iter().any(|parent_tool| {
                parent_tool.tool_id == child_tool.tool_id
                    && parent_tool.version == child_tool.version
                    && parent_tool.integrity_sha256 == child_tool.integrity_sha256
                    && parent_tool.required_capabilities == child_tool.required_capabilities
                    && parent_tool.side_effect_class == child_tool.side_effect_class
                    && (!child_tool.enabled || parent_tool.enabled)
            })
        })
}

fn budget_is_subset(parent: &BudgetConfiguration, child: &BudgetConfiguration) -> bool {
    child.maximum_input_bytes <= parent.maximum_input_bytes
        && child.maximum_output_bytes <= parent.maximum_output_bytes
        && child.maximum_file_count <= parent.maximum_file_count
        && child.maximum_tree_depth <= parent.maximum_tree_depth
        && child.maximum_context_tokens <= parent.maximum_context_tokens
        && child.maximum_output_tokens <= parent.maximum_output_tokens
        && child.maximum_operation_milliseconds <= parent.maximum_operation_milliseconds
        && child.maximum_memory_bytes <= parent.maximum_memory_bytes
        && child.maximum_processes <= parent.maximum_processes
        && child.maximum_concurrency <= parent.maximum_concurrency
}

fn logging_is_subset(parent: &LoggingConfiguration, child: &LoggingConfiguration) -> bool {
    parent.destination == child.destination
        && logging_level_rank(&child.level) <= logging_level_rank(&parent.level)
        && child.maximum_event_bytes <= parent.maximum_event_bytes
}

fn retention_is_subset(parent: &RetentionConfiguration, child: &RetentionConfiguration) -> bool {
    child.sessions_days <= parent.sessions_days
        && child.receipts_days <= parent.receipts_days
        && child.logs_days <= parent.logs_days
        && child.temporary_artifacts_minutes <= parent.temporary_artifacts_minutes
        && child.backup_policy == parent.backup_policy
}

fn skills_are_subset(parent: &SkillConfiguration, child: &SkillConfiguration) -> bool {
    child.maximum_enabled_skills <= parent.maximum_enabled_skills
        && string_values_are_subset(&child.catalog_paths, &parent.catalog_paths)
        && string_values_are_subset(&child.capability_ceiling, &parent.capability_ceiling)
}

fn string_values_are_subset(child: &[String], parent: &[String]) -> bool {
    child.iter().all(|value| parent.contains(value))
}

fn access_rank(value: &str) -> u8 {
    match value {
        "read-only" => 1,
        "read-write" => 2,
        _ => u8::MAX,
    }
}

fn symlink_policy_rank(value: &str) -> u8 {
    match value {
        "deny" => 0,
        "same-root-only" => 1,
        _ => u8::MAX,
    }
}

fn logging_level_rank(value: &str) -> u8 {
    match value {
        "error" => 0,
        "warn" => 1,
        "info" => 2,
        "debug" => 3,
        _ => u8::MAX,
    }
}

fn section_versions(value: &AgentConfiguration) -> [u32; 11] {
    [
        value.core.schema_version,
        value.platform.schema_version,
        value.model.schema_version,
        value.workspace.schema_version,
        value.tool.schema_version,
        value.permission.schema_version,
        value.budget.schema_version,
        value.logging.schema_version,
        value.retention.schema_version,
        value.skill.schema_version,
        value.shell.schema_version,
    ]
}

fn section_names() -> [&'static str; 11] {
    [
        "core",
        "platform",
        "model",
        "workspace",
        "tool",
        "permission",
        "budget",
        "logging",
        "retention",
        "skill",
        "shell",
    ]
}

fn identifier(value: &str) -> bool {
    if value.is_empty() || value.len() > 128 {
        return false;
    }
    value.split(['.', '-']).all(|segment| {
        !segment.is_empty()
            && segment
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    }) && value.as_bytes()[0].is_ascii_lowercase()
}

fn relative_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 1024
        && !value.starts_with('/')
        && !value.split('/').any(|part| part == "..")
}

fn semantic_version(value: &str) -> bool {
    let (base, suffix) = value
        .split_once('-')
        .map_or((value, None), |(prefix, suffix)| (prefix, Some(suffix)));
    let parts: Vec<&str> = base.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (part == &"0" || !part.starts_with('0'))
        })
        && suffix.is_none_or(|suffix| {
            !suffix.is_empty()
                && suffix
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
        })
}

fn sha256_text(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256(value: &[u8]) -> String {
    Sha256::digest(value)
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            write!(output, "{byte:02x}").expect("writing to a string cannot fail");
            output
        })
}

fn collect_changes(
    path: &str,
    before: &Value,
    after: &Value,
    changes: &mut Vec<ConfigurationChange>,
) -> Result<(), ConfigurationError> {
    match (before, after) {
        (Value::Object(before_map), Value::Object(after_map)) => {
            let keys: BTreeSet<&String> = before_map.keys().chain(after_map.keys()).collect();
            for key in keys {
                let child_path = format!("{path}/{}", pointer_escape(key));
                match (before_map.get(key), after_map.get(key)) {
                    (Some(left), Some(right)) => {
                        collect_changes(&child_path, left, right, changes)?;
                    }
                    (left, right) => push_change(&child_path, left, right, changes)?,
                }
            }
        }
        _ if before != after => push_change(path, Some(before), Some(after), changes)?,
        _ => {}
    }
    Ok(())
}

fn push_change(
    path: &str,
    before: Option<&Value>,
    after: Option<&Value>,
    changes: &mut Vec<ConfigurationChange>,
) -> Result<(), ConfigurationError> {
    let before_bytes = serde_json::to_vec(&before).map_err(|_| {
        ConfigurationError::new(ErrorCode::ContractViolation, "configuration diff failed")
    })?;
    let after_bytes = serde_json::to_vec(&after).map_err(|_| {
        ConfigurationError::new(ErrorCode::ContractViolation, "configuration diff failed")
    })?;
    changes.push(ConfigurationChange {
        path: path.to_owned(),
        before_sha256: sha256(&before_bytes),
        after_sha256: sha256(&after_bytes),
        impact: classify_change(path, before, after),
    });
    Ok(())
}

fn classify_change(path: &str, before: Option<&Value>, after: Option<&Value>) -> ChangeImpact {
    if path == "/permission/allowed_capabilities" || path == "/skill/capability_ceiling" {
        let before_set = string_set(before);
        let after_set = string_set(after);
        if before_set.is_subset(&after_set) && before_set != after_set {
            return ChangeImpact::AuthorityBroadening;
        }
        if after_set.is_subset(&before_set) && before_set != after_set {
            return ChangeImpact::AuthorityRestriction;
        }
    }
    if path.starts_with("/budget/maximum_") {
        let left = before.and_then(Value::as_u64);
        let right = after.and_then(Value::as_u64);
        if let (Some(left), Some(right)) = (left, right) {
            return if right > left {
                ChangeImpact::ResourceIncrease
            } else {
                ChangeImpact::ResourceRestriction
            };
        }
    }
    if path == "/workspace/roots" || path == "/tool/tools" {
        return ChangeImpact::AuthorityBroadening;
    }
    ChangeImpact::Operational
}

fn string_set(value: Option<&Value>) -> BTreeSet<&str> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

fn pointer_escape(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn backup_path(target: &Path, hash: &str) -> Result<PathBuf, ConfigurationError> {
    let parent = target.parent().ok_or_else(|| {
        ConfigurationError::new(
            ErrorCode::Io,
            "configuration parent directory is unavailable",
        )
    })?;
    let filename = target
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            ConfigurationError::new(ErrorCode::Io, "configuration filename is invalid")
        })?;
    Ok(parent.join(format!(".{filename}.agentmage-backup-{hash}.json")))
}

fn temporary_path(target: &Path, hash: &str) -> Result<PathBuf, ConfigurationError> {
    let parent = target.parent().ok_or_else(|| {
        ConfigurationError::new(
            ErrorCode::Io,
            "configuration parent directory is unavailable",
        )
    })?;
    let filename = target
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            ConfigurationError::new(ErrorCode::Io, "configuration filename is invalid")
        })?;
    Ok(parent.join(format!(".{filename}.agentmage-new-{hash}.tmp")))
}

fn write_new_or_verify(path: &Path, content: &[u8]) -> Result<(), ConfigurationError> {
    match create_private_new(path) {
        Ok(mut file) => {
            file.write_all(content).map_err(|_| {
                ConfigurationError::new(ErrorCode::Io, "configuration backup write failed")
            })?;
            set_private_permissions(path)?;
            file.sync_all().map_err(|_| {
                ConfigurationError::new(ErrorCode::Io, "configuration backup sync failed")
            })?;
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let metadata = fs::symlink_metadata(path).map_err(|_| {
                ConfigurationError::new(ErrorCode::Io, "configuration backup inspect failed")
            })?;
            if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
                return Err(ConfigurationError::new(
                    ErrorCode::Conflict,
                    "configuration backup path is not a regular file",
                ));
            }
            let existing = fs::read(path).map_err(|_| {
                ConfigurationError::new(ErrorCode::Io, "configuration backup read failed")
            })?;
            if existing == content {
                Ok(())
            } else {
                Err(ConfigurationError::new(
                    ErrorCode::Conflict,
                    "configuration backup identity conflicts with existing file",
                ))
            }
        }
        Err(_) => Err(ConfigurationError::new(
            ErrorCode::Io,
            "configuration backup could not be created",
        )),
    }
}

fn prepare_migration_candidate(path: &Path, content: &[u8]) -> Result<(), ConfigurationError> {
    match create_private_new(path) {
        Ok(mut file) => {
            file.write_all(content).map_err(|_| {
                ConfigurationError::new(ErrorCode::Io, "migration candidate write failed")
            })?;
            set_private_permissions(path)?;
            file.sync_all().map_err(|_| {
                ConfigurationError::new(ErrorCode::Io, "migration candidate sync failed")
            })?;
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let metadata = fs::symlink_metadata(path).map_err(|_| {
                ConfigurationError::new(ErrorCode::Io, "migration candidate could not be inspected")
            })?;
            if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
                return Err(ConfigurationError::new(
                    ErrorCode::Conflict,
                    "migration candidate path is not a regular file",
                ));
            }
            let existing = fs::read(path).map_err(|_| {
                ConfigurationError::new(ErrorCode::Io, "migration candidate could not be read")
            })?;
            if existing == content {
                set_private_permissions(path)?;
                Ok(())
            } else {
                Err(ConfigurationError::new(
                    ErrorCode::Conflict,
                    "migration candidate conflicts with existing file",
                ))
            }
        }
        Err(_) => Err(ConfigurationError::new(
            ErrorCode::Io,
            "migration candidate could not be created",
        )),
    }
}

fn atomic_replace(target: &Path, content: &[u8], hash: &str) -> Result<(), ConfigurationError> {
    let temporary = temporary_path(target, hash)?;
    let mut file = create_private_new(&temporary).map_err(|_| {
        ConfigurationError::new(ErrorCode::Conflict, "configuration temporary file exists")
    })?;
    if file.write_all(content).is_err() || file.sync_all().is_err() {
        drop(file);
        let _ = fs::remove_file(&temporary);
        return Err(ConfigurationError::new(
            ErrorCode::Io,
            "configuration temporary write failed",
        ));
    }
    drop(file);
    if let Err(error) = set_private_permissions(&temporary) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    if fs::rename(&temporary, target).is_err() {
        let _ = fs::remove_file(&temporary);
        return Err(ConfigurationError::new(
            ErrorCode::Io,
            "configuration atomic replacement failed",
        ));
    }
    sync_parent_directory(target)?;
    Ok(())
}

fn create_private_new(path: &Path) -> std::io::Result<std::fs::File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    options.open(path)
}

#[cfg(unix)]
fn sync_parent_directory(path: &Path) -> Result<(), ConfigurationError> {
    let parent = path.parent().ok_or_else(|| {
        ConfigurationError::new(
            ErrorCode::Io,
            "configuration parent directory is unavailable",
        )
    })?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| ConfigurationError::new(ErrorCode::Io, "configuration directory sync failed"))
}

#[cfg(not(unix))]
fn sync_parent_directory(_path: &Path) -> Result<(), ConfigurationError> {
    Ok(())
}

#[cfg(unix)]
fn set_private_permissions(path: &Path) -> Result<(), ConfigurationError> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|_| {
        ConfigurationError::new(ErrorCode::Io, "configuration permissions could not be set")
    })
}

#[cfg(not(unix))]
fn set_private_permissions(_path: &Path) -> Result<(), ConfigurationError> {
    Ok(())
}

struct UniqueJson(Value);

impl<'de> Deserialize<'de> for UniqueJson {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(UniqueJsonVisitor)
    }
}

struct UniqueJsonVisitor;

impl<'de> Visitor<'de> for UniqueJsonVisitor {
    type Value = UniqueJson;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value without duplicate object keys")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(UniqueJson(Value::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(UniqueJson(Value::Number(Number::from(value))))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(UniqueJson(Value::Number(Number::from(value))))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .map(UniqueJson)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_string(value.to_owned())
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(UniqueJson(Value::String(value)))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(UniqueJson(Value::Null))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(UniqueJson(Value::Null))
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element::<UniqueJson>()? {
            values.push(value.0);
        }
        Ok(UniqueJson(Value::Array(values)))
    }

    fn visit_map<A>(self, mut object: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = Map::new();
        while let Some(key) = object.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(de::Error::custom("duplicate JSON object key"));
            }
            let value = object.next_value::<UniqueJson>()?;
            values.insert(key, value.0);
        }
        Ok(UniqueJson(Value::Object(values)))
    }
}

fn parse_unique_json(input: &[u8]) -> Result<Value, ConfigurationError> {
    let mut deserializer = serde_json::Deserializer::from_slice(input);
    let value = UniqueJson::deserialize(&mut deserializer).map_err(|_| {
        ConfigurationError::new(
            ErrorCode::MalformedJson,
            "configuration is malformed or contains a duplicate key",
        )
    })?;
    deserializer.end().map_err(|_| {
        ConfigurationError::new(
            ErrorCode::MalformedJson,
            "configuration contains trailing input",
        )
    })?;
    Ok(value.0)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use ed25519_dalek::{Signer, SigningKey};

    use super::*;

    const SYNTHETIC_PROFILE: &[u8] =
        include_bytes!("../../../configuration/profiles/synthetic-test.json");
    const MIGRATION_V0_PROFILE: &[u8] =
        include_bytes!("../../../fixtures/configuration/migration/v0.valid.json");
    const MIGRATION_V1_EXPECTED: &[u8] =
        include_bytes!("../../../fixtures/configuration/migration/v1.expected.json");
    const MIGRATION_V1_NOT_MIGRATABLE: &[u8] =
        include_bytes!("../../../fixtures/configuration/migration/v1.not-migratable.invalid.json");
    const MIGRATION_V0_RESERVED_VERSION: &[u8] = include_bytes!(
        "../../../fixtures/configuration/migration/v0.reserved-version.invalid.json"
    );
    const MIGRATION_V0_MISSING_SECTION: &[u8] =
        include_bytes!("../../../fixtures/configuration/migration/v0.missing-section.invalid.json");
    const PERMISSION_BEARING_VALUES: &[u8] =
        include_bytes!("../../../configuration/permission-bearing-values.json");
    static TEMPORARY_ID: AtomicU64 = AtomicU64::new(0);

    fn manager() -> ConfigurationManager {
        ConfigurationManager::default()
    }

    fn restricted_sources() -> [RestrictedConfigurationSource; 5] {
        [
            RestrictedConfigurationSource::ConfigurationFile,
            RestrictedConfigurationSource::Environment,
            RestrictedConfigurationSource::ChildProfile,
            RestrictedConfigurationSource::Repository,
            RestrictedConfigurationSource::ModelOutput,
        ]
    }

    fn signed_parent(input: &[u8]) -> VerifiedParentProfile {
        let configuration = manager().load_bytes(input).expect("signed parent loads");
        // This deterministic key is synthetic unit-test material, never a product trust root.
        let signing_key = SigningKey::from_bytes(&[0x42; 32]);
        let material =
            parent_profile_signature_material(&configuration).expect("signature material builds");
        let signature = signing_key.sign(&material).to_bytes();
        manager()
            .verify_parent_profile(
                &configuration,
                &signing_key.verifying_key().to_bytes(),
                &signature,
            )
            .expect("synthetic parent signature verifies")
    }

    fn set_value(value: &mut Value, path: &[&str], replacement: Value) {
        let mut current = value;
        for part in &path[..path.len() - 1] {
            current = match current {
                Value::Array(items) => items
                    .get_mut(part.parse::<usize>().expect("array path must be numeric"))
                    .expect("fixture array path must exist"),
                _ => current.get_mut(*part).expect("fixture path must exist"),
            };
        }
        current
            .as_object_mut()
            .expect("fixture parent must be an object")
            .insert(path[path.len() - 1].to_owned(), replacement);
    }

    fn mutate(mut value: Value, path: &[&str], replacement: Value) -> Vec<u8> {
        set_value(&mut value, path, replacement);
        serde_json::to_vec(&value).expect("fixture must serialize")
    }

    fn schema_failure_fixture(schema: &str, failure_class: &str) -> Vec<u8> {
        let mut value = fixture_value();
        match failure_class {
            "missing" => {
                if schema == "agent-configuration" {
                    value.as_object_mut().expect("bundle object").remove("core");
                } else {
                    value[schema]
                        .as_object_mut()
                        .expect("section object")
                        .remove("schema_version");
                }
            }
            "extra" => {
                let object = if schema == "agent-configuration" {
                    value.as_object_mut().expect("bundle object")
                } else {
                    value[schema].as_object_mut().expect("section object")
                };
                object.insert("unreviewed_extension".to_owned(), Value::Bool(true));
            }
            "wrong-type" => {
                if schema == "agent-configuration" {
                    value["core"] = Value::Bool(true);
                } else {
                    value[schema]["schema_version"] = Value::String("1".to_owned());
                }
            }
            "oversized" => {
                if schema == "agent-configuration" {
                    let mut bytes = serde_json::to_vec(&value).expect("fixture serializes");
                    bytes.resize(DEFAULT_MAXIMUM_BYTES + 1, b' ');
                    return bytes;
                }
                match schema {
                    "core" => value["core"]["profile_id"] = Value::String("a".repeat(129)),
                    "platform" => {
                        value["platform"]["adapter_id"] = Value::String("a".repeat(129));
                    }
                    "model" => {
                        value["model"]["manifest_path"] = Value::String("a".repeat(1025));
                    }
                    "workspace" => value["workspace"]["maximum_roots"] = Value::from(33),
                    "tool" => {
                        value["tool"]["registry_path"] = Value::String("a".repeat(1025));
                    }
                    "permission" => {
                        value["permission"]["allowed_capabilities"] = Value::Array(
                            (0..257)
                                .map(|index| Value::String(format!("fixture.capability-{index}")))
                                .collect(),
                        );
                    }
                    "budget" => {
                        value["budget"]["maximum_input_bytes"] =
                            Value::from(MAXIMUM_SCHEMA_BYTES + 1);
                    }
                    "logging" => {
                        value["logging"]["maximum_event_bytes"] = Value::from(1_048_577);
                    }
                    "retention" => value["retention"]["receipts_days"] = Value::from(3651),
                    "skill" => value["skill"]["maximum_enabled_skills"] = Value::from(257),
                    "shell" => value["shell"]["shell_id"] = Value::String("a".repeat(129)),
                    _ => panic!("unknown configuration schema"),
                }
            }
            "unsupported-version" => {
                if schema == "agent-configuration" {
                    value["schema_version"] = Value::from(2);
                } else {
                    value[schema]["schema_version"] = Value::from(2);
                }
            }
            "ambiguous" => {
                let serialized = serde_json::to_string(&value).expect("fixture serializes");
                let needle = if schema == "agent-configuration" {
                    "{".to_owned()
                } else {
                    format!("\"{schema}\":{{")
                };
                let replacement = format!("{needle}\"schema_version\":1,");
                let duplicated = serialized.replacen(&needle, &replacement, 1);
                assert_ne!(duplicated, serialized, "duplicate insertion must occur");
                return duplicated.into_bytes();
            }
            _ => panic!("unknown failure class"),
        }
        serde_json::to_vec(&value).expect("fixture must serialize")
    }

    fn expected_schema_failure(schema: &str, failure_class: &str) -> (&'static str, &'static str) {
        match (schema, failure_class) {
            (_, "ambiguous") => (
                "configuration-malformed-json",
                "configuration is malformed or contains a duplicate key",
            ),
            ("agent-configuration", "oversized") => (
                "configuration-too-large",
                "configuration exceeds the configured byte limit",
            ),
            ("agent-configuration", "unsupported-version") => (
                "configuration-unsupported-version",
                "configuration schema version is unsupported",
            ),
            (_, "unsupported-version" | "oversized") => (
                "configuration-contract-violation",
                "configuration violates a version 1 safety invariant",
            ),
            _ => (
                "configuration-contract-violation",
                "configuration does not match the closed version 1 contract",
            ),
        }
    }

    fn permission_bearing_mutation(path: &str) -> (Vec<u8>, Vec<u8>) {
        let mut parent = fixture_value();
        if path == "/tool/tools/0/enabled" {
            parent["tool"]["tools"][0]["enabled"] = Value::Bool(false);
        }
        let mut candidate = parent.clone();
        match path {
            "/core/configuration_mode" => {
                candidate["core"]["configuration_mode"] = Value::String("coding".to_owned());
            }
            "/core/startup_failure_policy" => {
                candidate["core"]["startup_failure_policy"] = Value::String("continue".to_owned());
            }
            "/core/strict_local" => {
                candidate["core"]["configuration_mode"] =
                    Value::String("network-enabled".to_owned());
                candidate["core"]["strict_local"] = Value::Bool(false);
            }
            "/core/inheritance_mode" => {
                candidate["core"]["inheritance_mode"] = Value::String("merge".to_owned());
            }
            "/platform/platform_id" => {
                candidate["platform"]["platform_id"] = Value::String("ubuntu-x86_64".to_owned());
            }
            "/platform/adapter_id" => {
                candidate["platform"]["adapter_id"] =
                    Value::String("alternate-linux-platform-v1".to_owned());
            }
            "/platform/sandbox_profile_id" => {
                candidate["platform"]["sandbox_profile_id"] =
                    Value::String("alternate-sandbox-v1".to_owned());
            }
            "/platform/path_identity_strategy" => {
                candidate["platform"]["path_identity_strategy"] =
                    Value::String("security-scoped-bookmark".to_owned());
            }
            "/platform/secret_provider_id" => {
                candidate["platform"]["secret_provider_id"] = Value::String("keychain".to_owned());
            }
            "/platform/standard_user_required" => {
                candidate["platform"]["standard_user_required"] = Value::Bool(false);
            }
            "/platform/administrator_required" => {
                candidate["platform"]["administrator_required"] = Value::Bool(true);
            }
            "/platform/ambient_dependency_policy" => {
                candidate["platform"]["ambient_dependency_policy"] =
                    Value::String("allow".to_owned());
            }
            "/model/enabled" => candidate["model"]["enabled"] = Value::Bool(true),
            "/model/model_profile_id" => {
                candidate["model"]["model_profile_id"] =
                    Value::String("alternate-model-v1".to_owned());
            }
            "/model/manifest_path" => {
                candidate["model"]["manifest_path"] =
                    Value::String("fixtures/alternate-model-manifest.json".to_owned());
            }
            "/model/runtime_adapter_id" => {
                candidate["model"]["runtime_adapter_id"] =
                    Value::String("alternate-runtime-v1".to_owned());
            }
            "/model/transport" => {
                candidate["model"]["transport"] = Value::String("local-unix-socket".to_owned());
            }
            "/model/network_access" => {
                candidate["model"]["network_access"] = Value::Bool(true);
            }
            "/model/automatic_routing" => {
                candidate["model"]["automatic_routing"] = Value::Bool(true);
            }
            "/model/decoding/deterministic" => {
                candidate["model"]["decoding"]["deterministic"] = Value::Bool(false);
            }
            "/model/decoding/temperature" => {
                candidate["model"]["decoding"]["deterministic"] = Value::Bool(false);
                candidate["model"]["decoding"]["temperature"] = Value::from(1.0);
            }
            "/model/decoding/top_k" => {
                candidate["model"]["decoding"]["deterministic"] = Value::Bool(false);
                candidate["model"]["decoding"]["top_k"] = Value::from(2);
            }
            "/model/decoding/seed" => {
                candidate["model"]["decoding"]["seed"] = Value::from(104_730);
            }
            "/model/decoding/maximum_context_tokens" => {
                candidate["model"]["decoding"]["maximum_context_tokens"] = Value::from(4097);
                candidate["budget"]["maximum_context_tokens"] = Value::from(4097);
            }
            "/model/decoding/maximum_output_tokens" => {
                candidate["model"]["decoding"]["maximum_output_tokens"] = Value::from(1025);
                candidate["budget"]["maximum_output_tokens"] = Value::from(1025);
            }
            "/workspace/maximum_roots" => {
                candidate["workspace"]["maximum_roots"] = Value::from(2);
            }
            "/workspace/default_access" => {
                candidate["workspace"]["default_access"] = Value::String("allow".to_owned());
            }
            "/workspace/symlink_policy" => {
                candidate["workspace"]["symlink_policy"] =
                    Value::String("same-root-only".to_owned());
            }
            "/workspace/roots" => {
                candidate["workspace"]["maximum_roots"] = Value::from(2);
                candidate["workspace"]["roots"]
                    .as_array_mut()
                    .expect("workspace roots array")
                    .push(serde_json::json!({
                        "root_id": "second-workspace",
                        "locator_kind": "managed-relative",
                        "locator": "fixtures/generated/second-workspace",
                        "access": "read-only",
                        "follow_mount_changes": false
                    }));
            }
            "/workspace/roots/0/root_id" => {
                candidate["workspace"]["roots"][0]["root_id"] =
                    Value::String("alternate-workspace".to_owned());
            }
            "/workspace/roots/0/locator_kind" => {
                candidate["workspace"]["roots"][0]["locator_kind"] =
                    Value::String("platform-resolved-handle".to_owned());
            }
            "/workspace/roots/0/locator" => {
                candidate["workspace"]["roots"][0]["locator"] =
                    Value::String("fixtures/generated/alternate-workspace".to_owned());
            }
            "/workspace/roots/0/access" => {
                candidate["workspace"]["roots"][0]["access"] =
                    Value::String("read-write".to_owned());
            }
            "/workspace/roots/0/follow_mount_changes" => {
                candidate["workspace"]["roots"][0]["follow_mount_changes"] = Value::Bool(true);
            }
            "/tool/registry_path" => {
                candidate["tool"]["registry_path"] =
                    Value::String("fixtures/alternate-tool-registry.json".to_owned());
            }
            "/tool/unregistered_tool_policy" => {
                candidate["tool"]["unregistered_tool_policy"] = Value::String("allow".to_owned());
            }
            "/tool/tools" => {
                candidate["tool"]["tools"]
                    .as_array_mut()
                    .expect("tools array")
                    .push(serde_json::json!({
                        "tool_id": "synthetic-second-tool",
                        "version": "1.0.0",
                        "integrity_sha256": "2".repeat(64),
                        "enabled": false,
                        "required_capabilities": [],
                        "side_effect_class": "none"
                    }));
            }
            "/tool/tools/0/tool_id" => {
                candidate["tool"]["tools"][0]["tool_id"] =
                    Value::String("alternate-read-file".to_owned());
            }
            "/tool/tools/0/version" => {
                candidate["tool"]["tools"][0]["version"] = Value::String("1.0.1".to_owned());
            }
            "/tool/tools/0/integrity_sha256" => {
                candidate["tool"]["tools"][0]["integrity_sha256"] = Value::String("2".repeat(64));
            }
            "/tool/tools/0/enabled" => {
                candidate["tool"]["tools"][0]["enabled"] = Value::Bool(true);
            }
            "/tool/tools/0/required_capabilities" => {
                candidate["permission"]["allowed_capabilities"] =
                    serde_json::json!(["workspace.read", "workspace.write"]);
                candidate["tool"]["tools"][0]["required_capabilities"] =
                    serde_json::json!(["workspace.read", "workspace.write"]);
            }
            "/tool/tools/0/side_effect_class" => {
                candidate["tool"]["tools"][0]["side_effect_class"] =
                    Value::String("write".to_owned());
            }
            "/permission/default_effect" => {
                candidate["permission"]["default_effect"] = Value::String("allow".to_owned());
            }
            "/permission/authority_inheritance" => {
                candidate["permission"]["authority_inheritance"] =
                    Value::String("merge".to_owned());
            }
            "/permission/model_authority" => {
                candidate["permission"]["model_authority"] = Value::String("tools".to_owned());
            }
            "/permission/grant_policy/single_use" => {
                candidate["permission"]["grant_policy"]["single_use"] = Value::Bool(false);
            }
            "/permission/grant_policy/expiry_required" => {
                candidate["permission"]["grant_policy"]["expiry_required"] = Value::Bool(false);
            }
            "/permission/grant_policy/exact_scope_required" => {
                candidate["permission"]["grant_policy"]["exact_scope_required"] =
                    Value::Bool(false);
            }
            "/permission/grant_policy/transferable" => {
                candidate["permission"]["grant_policy"]["transferable"] = Value::Bool(true);
            }
            "/permission/grant_policy/replay_policy" => {
                candidate["permission"]["grant_policy"]["replay_policy"] =
                    Value::String("allow".to_owned());
            }
            "/permission/allowed_capabilities" => {
                candidate["permission"]["allowed_capabilities"] =
                    serde_json::json!(["workspace.read", "workspace.write"]);
            }
            "/permission/network/mode" => {
                candidate["permission"]["network"]["mode"] = Value::String("allow-list".to_owned());
            }
            "/permission/network/allowed_endpoints" => {
                candidate["permission"]["network"]["allowed_endpoints"] =
                    serde_json::json!(["invalid-endpoint"]);
            }
            path @ ("/budget/maximum_input_bytes"
            | "/budget/maximum_output_bytes"
            | "/budget/maximum_file_count"
            | "/budget/maximum_tree_depth"
            | "/budget/maximum_context_tokens"
            | "/budget/maximum_output_tokens"
            | "/budget/maximum_operation_milliseconds"
            | "/budget/maximum_memory_bytes"
            | "/budget/maximum_processes"
            | "/budget/maximum_concurrency") => {
                let field = path.rsplit('/').next().expect("budget field");
                let current = candidate["budget"][field]
                    .as_u64()
                    .expect("numeric budget field");
                candidate["budget"][field] = Value::from(current + 1);
            }
            "/logging/level" => {
                candidate["logging"]["level"] = Value::String("debug".to_owned());
            }
            "/logging/structured" => candidate["logging"]["structured"] = Value::Bool(false),
            "/logging/destination/kind" => {
                candidate["logging"]["destination"]["kind"] = Value::String("stdout".to_owned());
            }
            "/logging/destination/managed_relative_path" => {
                candidate["logging"]["destination"]["managed_relative_path"] =
                    Value::String("state/logs/alternate.jsonl".to_owned());
            }
            "/logging/maximum_event_bytes" => {
                candidate["logging"]["maximum_event_bytes"] = Value::from(16_385);
            }
            path @ ("/logging/redaction/secret_values"
            | "/logging/redaction/credentials"
            | "/logging/redaction/private_paths"
            | "/logging/redaction/environment_values") => {
                let field = path.rsplit('/').next().expect("redaction field");
                candidate["logging"]["redaction"][field] = Value::String("retain".to_owned());
            }
            path @ ("/logging/record_prompts"
            | "/logging/record_tool_arguments"
            | "/logging/record_environment_values") => {
                let field = path.rsplit('/').next().expect("logging field");
                candidate["logging"][field] = Value::Bool(true);
            }
            path @ ("/retention/sessions_days"
            | "/retention/receipts_days"
            | "/retention/logs_days"
            | "/retention/temporary_artifacts_minutes") => {
                let field = path.rsplit('/').next().expect("retention field");
                let current = candidate["retention"][field]
                    .as_u64()
                    .expect("numeric retention field");
                candidate["retention"][field] = Value::from(current + 1);
            }
            "/retention/expired_data_action" => {
                candidate["retention"]["expired_data_action"] = Value::String("retain".to_owned());
            }
            "/retention/backup_policy" => {
                candidate["retention"]["backup_policy"] =
                    Value::String("user-managed-encrypted".to_owned());
            }
            "/retention/secure_deletion_claim" => {
                candidate["retention"]["secure_deletion_claim"] =
                    Value::String("guaranteed".to_owned());
            }
            "/skill/enablement" => {
                candidate["skill"]["enablement"] = Value::String("automatic".to_owned());
            }
            "/skill/unsigned_skill_policy" => {
                candidate["skill"]["unsigned_skill_policy"] = Value::String("allow".to_owned());
            }
            "/skill/self_modification" => {
                candidate["skill"]["self_modification"] = Value::String("allow".to_owned());
            }
            "/skill/catalog_paths" => {
                candidate["skill"]["catalog_paths"] =
                    serde_json::json!(["skills/alternate-catalog.json"]);
            }
            "/skill/maximum_enabled_skills" => {
                candidate["skill"]["maximum_enabled_skills"] = Value::from(1);
            }
            "/skill/capability_ceiling" => {
                candidate["skill"]["capability_ceiling"] = serde_json::json!(["workspace.read"]);
            }
            "/shell/shell_id" => {
                candidate["shell"]["shell_id"] = Value::String("alternate-shell-v1".to_owned());
            }
            "/shell/surface_id" => {
                candidate["shell"]["surface_id"] = Value::String("host-cli".to_owned());
            }
            "/shell/transport" => {
                candidate["shell"]["transport"] = Value::String("local-stdio".to_owned());
            }
            "/shell/launch_privilege" => {
                candidate["shell"]["launch_privilege"] = Value::String("administrator".to_owned());
            }
            "/shell/network_access" => candidate["shell"]["network_access"] = Value::Bool(true),
            "/shell/direct_command_execution" => {
                candidate["shell"]["direct_command_execution"] = Value::Bool(true);
            }
            "/shell/model_to_tool_channel" => {
                candidate["shell"]["model_to_tool_channel"] = Value::String("allowed".to_owned());
            }
            "/shell/diagnostics_surface" => {
                candidate["shell"]["diagnostics_surface"] =
                    Value::String("status-command".to_owned());
            }
            _ => panic!("unhandled permission-bearing path: {path}"),
        }
        (
            serde_json::to_vec(&parent).expect("parent fixture serializes"),
            serde_json::to_vec(&candidate).expect("candidate fixture serializes"),
        )
    }

    fn fixture_value() -> Value {
        serde_json::from_slice(SYNTHETIC_PROFILE).expect("fixture must parse")
    }

    fn temporary_directory() -> PathBuf {
        let identity = TEMPORARY_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "agentmage-configuration-test-{}-{identity}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("temporary directory must be created");
        path
    }

    #[test]
    fn loads_canonical_profile_deterministically() {
        let first = manager()
            .load_bytes(SYNTHETIC_PROFILE)
            .expect("profile loads");
        let second = manager()
            .load_bytes(SYNTHETIC_PROFILE)
            .expect("profile loads");
        assert_eq!(first.profile_id(), "synthetic-test");
        assert_eq!(first.sha256(), second.sha256());
        assert_eq!(first.canonical_bytes(), second.canonical_bytes());
    }

    #[test]
    fn rejects_unknown_duplicate_missing_unsupported_and_oversized_input() {
        let unknown = mutate(
            fixture_value(),
            &["core", "unreviewed_extension"],
            Value::Bool(true),
        );
        assert_eq!(
            manager()
                .load_bytes(&unknown)
                .expect_err("unknown must fail")
                .code(),
            "configuration-contract-violation"
        );
        let duplicate = br#"{"schema_version":1,"schema_version":1}"#;
        assert_eq!(
            manager()
                .load_bytes(duplicate)
                .expect_err("duplicate must fail")
                .code(),
            "configuration-malformed-json"
        );
        let missing = mutate(fixture_value(), &["schema_version"], Value::Null);
        assert_eq!(
            manager()
                .load_bytes(&missing)
                .expect_err("missing must fail")
                .code(),
            "configuration-unsupported-version"
        );
        let unsupported = mutate(fixture_value(), &["schema_version"], Value::from(2));
        assert_eq!(
            manager()
                .load_bytes(&unsupported)
                .expect_err("unsupported must fail")
                .code(),
            "configuration-unsupported-version"
        );
        assert_eq!(
            ConfigurationManager::with_maximum_bytes(16)
                .expect("limit is valid")
                .load_bytes(SYNTHETIC_PROFILE)
                .expect_err("oversized must fail")
                .code(),
            "configuration-too-large"
        );
    }

    #[test]
    fn every_configuration_schema_failure_class_is_stable_and_side_effect_free() {
        let schemas = [
            "agent-configuration",
            "core",
            "platform",
            "model",
            "workspace",
            "tool",
            "permission",
            "budget",
            "logging",
            "retention",
            "skill",
            "shell",
        ];
        let failure_classes = [
            "missing",
            "extra",
            "wrong-type",
            "oversized",
            "unsupported-version",
            "ambiguous",
        ];
        let directory = temporary_directory();
        let target = directory.join("agentmage.json");
        fs::write(&target, STRICT_LOCAL_PROFILE).expect("baseline writes");
        let baseline = fs::read(&target).expect("baseline reads");
        let mut rejected = 0;

        for schema in schemas {
            for failure_class in failure_classes {
                let invalid = schema_failure_fixture(schema, failure_class);
                let expected = expected_schema_failure(schema, failure_class);
                let first = manager()
                    .load_bytes(&invalid)
                    .expect_err("invalid schema case must fail");
                let second = manager()
                    .load_bytes(&invalid)
                    .expect_err("repeat invalid schema case must fail");
                assert_eq!(first, second, "diagnostic must be deterministic");
                assert_eq!((first.code(), first.diagnostic()), expected);

                let apply_error = manager()
                    .apply_with_backup(&target, &invalid)
                    .expect_err("invalid schema cannot reach configuration activation");
                assert_eq!(apply_error, first);
                assert_eq!(fs::read(&target).expect("target reads"), baseline);
                assert_eq!(
                    fs::read_dir(&directory)
                        .expect("temporary directory reads")
                        .count(),
                    1,
                    "rejection must not create a backup or temporary file"
                );
                rejected += 1;
            }
        }

        assert_eq!(rejected, 72);
        assert_eq!(
            manager()
                .safe_defaults()
                .expect("manager remains usable")
                .profile_id(),
            "strict-local-read-only"
        );
        fs::remove_dir_all(directory).expect("temporary directory removes");
    }

    #[test]
    fn rejects_permission_network_logging_and_budget_broadening() {
        let mutations = [
            mutate(
                fixture_value(),
                &["permission", "default_effect"],
                Value::String("allow".to_owned()),
            ),
            mutate(
                fixture_value(),
                &["model", "network_access"],
                Value::Bool(true),
            ),
            mutate(
                fixture_value(),
                &["logging", "record_prompts"],
                Value::Bool(true),
            ),
            mutate(
                fixture_value(),
                &["model", "decoding", "maximum_output_tokens"],
                Value::from(2048),
            ),
        ];
        for changed in mutations {
            assert_eq!(
                manager()
                    .load_bytes(&changed)
                    .expect_err("unsafe mutation must fail")
                    .code(),
                "configuration-contract-violation"
            );
        }
    }

    #[test]
    fn mirrors_published_identifier_version_collection_and_numeric_bounds() {
        let mutations = [
            mutate(
                fixture_value(),
                &["core", "profile_id"],
                Value::String("synthetic--test".to_owned()),
            ),
            mutate(
                fixture_value(),
                &["tool", "tools"],
                serde_json::json!([{
                    "tool_id": "fixture.tool",
                    "version": "01.0.0",
                    "integrity_sha256": "0".repeat(64),
                    "enabled": false,
                    "required_capabilities": [],
                    "side_effect_class": "none"
                }]),
            ),
            mutate(
                fixture_value(),
                &["model", "decoding", "seed"],
                Value::from(MAXIMUM_SCHEMA_INTEGER + 1),
            ),
            mutate(
                fixture_value(),
                &["budget", "maximum_input_bytes"],
                Value::from(MAXIMUM_SCHEMA_BYTES + 1),
            ),
            mutate(
                fixture_value(),
                &["permission", "allowed_capabilities"],
                Value::Array(
                    (0..257)
                        .map(|index| Value::String(format!("fixture.capability-{index}")))
                        .collect(),
                ),
            ),
        ];
        for changed in mutations {
            assert_eq!(
                manager()
                    .load_bytes(&changed)
                    .expect_err("out-of-schema value must fail")
                    .code(),
                "configuration-contract-violation"
            );
        }
    }

    #[test]
    fn every_untrusted_channel_accepts_only_a_valid_restriction() {
        let parent = signed_parent(SYNTHETIC_PROFILE);
        let mut child = fixture_value();
        child["core"]["profile_id"] = Value::String("restricted-child".to_owned());
        child["tool"]["tools"] = Value::Array(Vec::new());
        child["permission"]["allowed_capabilities"] = Value::Array(Vec::new());
        child["budget"]["maximum_output_bytes"] = Value::from(131_072);
        child["logging"]["level"] = Value::String("error".to_owned());
        child["retention"]["sessions_days"] = Value::from(7);
        let bytes = serde_json::to_vec(&child).expect("candidate serializes");
        for source in restricted_sources() {
            let outcome = manager()
                .load_restricted_candidate(&parent, source, &bytes)
                .expect("restriction must load");
            assert_eq!(outcome.source(), source);
            assert_eq!(outcome.source().as_str(), source.as_str());
            assert_eq!(outcome.parent_sha256(), parent.configuration_sha256());
            assert!(!outcome.diff().broadens_authority());
            assert_eq!(outcome.configuration().profile_id(), "restricted-child");
        }
    }

    #[test]
    fn every_untrusted_channel_rejects_capability_broadening() {
        let parent = signed_parent(STRICT_LOCAL_PROFILE);
        let candidate = mutate(
            serde_json::to_value(&parent.configuration.configuration).expect("parent serializes"),
            &["permission", "allowed_capabilities"],
            serde_json::json!(["workspace.read", "workspace.write"]),
        );
        for source in restricted_sources() {
            let error = manager()
                .load_restricted_candidate(&parent, source, &candidate)
                .expect_err("authority broadening must fail");
            assert_eq!(error.code(), "configuration-authority-broadening");
            assert!(!format!("{error:?}").contains("workspace.write"));
        }
    }

    #[test]
    fn parent_profile_signature_verification_rejects_tampering_and_wrong_keys() {
        let configuration = manager()
            .load_bytes(SYNTHETIC_PROFILE)
            .expect("parent loads");
        let signing_key = SigningKey::from_bytes(&[0x42; 32]);
        let material =
            parent_profile_signature_material(&configuration).expect("signature material builds");
        let signature = signing_key.sign(&material).to_bytes();
        let verified = manager()
            .verify_parent_profile(
                &configuration,
                &signing_key.verifying_key().to_bytes(),
                &signature,
            )
            .expect("signature verifies");
        assert_eq!(verified.configuration_sha256(), configuration.sha256());
        assert_eq!(verified.verifying_key_sha256().len(), 64);
        assert_eq!(verified.signature_sha256().len(), 64);

        let mut tampered_signature = signature;
        tampered_signature[0] ^= 1;
        let wrong_key = SigningKey::from_bytes(&[0x24; 32]);
        let alternate = manager()
            .safe_defaults()
            .expect("alternate configuration loads");
        for (candidate, key, signed) in [
            (
                &configuration,
                signing_key.verifying_key().to_bytes(),
                tampered_signature,
            ),
            (
                &configuration,
                wrong_key.verifying_key().to_bytes(),
                signature,
            ),
            (
                &alternate,
                signing_key.verifying_key().to_bytes(),
                signature,
            ),
        ] {
            let error = manager()
                .verify_parent_profile(candidate, &key, &signed)
                .expect_err("tampered signature boundary must fail");
            assert_eq!(error.code(), "configuration-parent-signature-invalid");
            assert_eq!(
                error.diagnostic(),
                "parent profile signature verification failed"
            );
        }
    }

    #[test]
    fn every_permission_bearing_value_is_rejected_through_every_untrusted_channel() {
        let registry: Value =
            serde_json::from_slice(PERMISSION_BEARING_VALUES).expect("registry parses");
        assert_eq!(registry["schema_version"], Value::from(1));
        assert_eq!(registry["status"], "enforced-test-closure");
        let entries = registry["entries"].as_array().expect("entries array");
        let mut seen = BTreeSet::new();
        let directory = temporary_directory();
        let candidate_path = directory.join("untrusted-candidate.json");
        let mut attempted = 0;

        for entry in entries {
            let path = entry["path"].as_str().expect("path string");
            let expected = entry["expected_rejection"]
                .as_str()
                .expect("expected rejection string");
            assert!(seen.insert(path), "permission-bearing path must be unique");
            let (parent_bytes, candidate) = permission_bearing_mutation(path);
            let parent = signed_parent(&parent_bytes);
            let parent_identity = parent.configuration_sha256().to_owned();

            let direct = manager().load_bytes(&candidate);
            if expected == "configuration-authority-broadening" {
                direct.expect("authority mutation must satisfy the closed schema");
            } else {
                assert_eq!(
                    direct
                        .expect_err("safety mutation must fail schema loading")
                        .code(),
                    expected
                );
            }

            for source in restricted_sources() {
                let error = if source == RestrictedConfigurationSource::ConfigurationFile {
                    fs::write(&candidate_path, &candidate).expect("candidate file writes");
                    manager()
                        .load_restricted_path(&parent, &candidate_path)
                        .expect_err("file broadening must fail")
                } else {
                    manager()
                        .load_restricted_candidate(&parent, source, &candidate)
                        .expect_err("channel broadening must fail")
                };
                assert_eq!(error.code(), expected, "failed path: {path}");
                assert_eq!(parent.configuration_sha256(), parent_identity);
                attempted += 1;
            }
        }

        assert_eq!(entries.len(), 97);
        assert_eq!(seen.len(), entries.len());
        assert_eq!(attempted, entries.len() * restricted_sources().len());
        fs::remove_dir_all(directory).expect("temporary directory removes");
    }

    #[test]
    fn roots_models_tools_and_platform_identity_cannot_broaden_or_change() {
        let strict = signed_parent(STRICT_LOCAL_PROFILE);
        let root_write = mutate(
            serde_json::to_value(&strict.configuration.configuration).expect("parent serializes"),
            &["workspace", "roots"],
            serde_json::json!([{
                "root_id": "user-selected-workspace",
                "locator_kind": "platform-resolved-handle",
                "locator": "unresolved-until-explicit-user-selection",
                "access": "read-write",
                "follow_mount_changes": false
            }]),
        );
        let model_enabled = mutate(
            serde_json::to_value(&strict.configuration.configuration).expect("parent serializes"),
            &["model", "enabled"],
            Value::Bool(true),
        );
        let synthetic = signed_parent(SYNTHETIC_PROFILE);
        let tool_changed = mutate(
            fixture_value(),
            &["tool", "tools", "0", "integrity_sha256"],
            Value::String("2".repeat(64)),
        );
        let platform_changed = mutate(
            fixture_value(),
            &["platform", "platform_id"],
            Value::String("ubuntu-x86_64".to_owned()),
        );
        for (parent, candidate) in [
            (&strict, root_write.as_slice()),
            (&strict, model_enabled.as_slice()),
            (&synthetic, tool_changed.as_slice()),
            (&synthetic, platform_changed.as_slice()),
        ] {
            assert_eq!(
                manager()
                    .load_restricted_candidate(
                        parent,
                        RestrictedConfigurationSource::Repository,
                        candidate,
                    )
                    .expect_err("boundary broadening must fail")
                    .code(),
                "configuration-authority-broadening"
            );
        }
    }

    #[test]
    fn resource_logging_and_retention_increases_are_rejected() {
        let parent = signed_parent(SYNTHETIC_PROFILE);
        let candidates = [
            mutate(
                fixture_value(),
                &["budget", "maximum_output_bytes"],
                Value::from(262_145),
            ),
            mutate(
                fixture_value(),
                &["logging", "level"],
                Value::String("debug".to_owned()),
            ),
            mutate(
                fixture_value(),
                &["retention", "sessions_days"],
                Value::from(31),
            ),
        ];
        for candidate in candidates {
            assert_eq!(
                manager()
                    .load_restricted_candidate(
                        &parent,
                        RestrictedConfigurationSource::ChildProfile,
                        &candidate,
                    )
                    .expect_err("increase must fail")
                    .code(),
                "configuration-authority-broadening"
            );
        }
    }

    #[test]
    fn malformed_untrusted_input_fails_before_authority_comparison() {
        let parent = signed_parent(STRICT_LOCAL_PROFILE);
        let candidate = br#"{"permission":"private-value""#;
        let error = manager()
            .load_restricted_candidate(
                &parent,
                RestrictedConfigurationSource::ModelOutput,
                candidate,
            )
            .expect_err("malformed input must fail");
        assert_eq!(error.code(), "configuration-malformed-json");
        assert!(!format!("{error:?}").contains("private-value"));
        assert_eq!(
            manager().safe_defaults().expect("parent reloads").sha256(),
            parent.configuration_sha256()
        );
    }

    #[test]
    fn aggregate_diff_detects_non_capability_authority_broadening() {
        let parent = manager().safe_defaults().expect("parent loads");
        let candidate = mutate(
            serde_json::to_value(&parent.configuration).expect("parent serializes"),
            &["model", "enabled"],
            Value::Bool(true),
        );
        let loaded = manager()
            .load_bytes(&candidate)
            .expect("candidate is valid");
        let diff = manager().diff(&parent, &loaded).expect("diff succeeds");
        assert!(diff.broadens_authority());
    }

    #[test]
    fn session_and_release_results_bind_the_exact_configuration_identity() {
        let configuration = manager().safe_defaults().expect("configuration loads");
        let payload = "a".repeat(64);
        let session = manager()
            .bind_session_result(&configuration, "session-result-1", &payload)
            .expect("session result binds");
        let release = manager()
            .bind_release_result(&configuration, "release-result-1", &payload)
            .expect("release result binds");
        assert_eq!(session.result_kind(), ConfigurationResultKind::Session);
        assert_eq!(session.result_kind().as_str(), "session");
        assert_eq!(release.result_kind(), ConfigurationResultKind::Release);
        assert_eq!(release.result_kind().as_str(), "release");
        for record in [&session, &release] {
            assert_eq!(
                record.configuration_profile_id(),
                configuration.profile_id()
            );
            assert_eq!(record.configuration_sha256(), configuration.sha256());
            assert_eq!(record.payload_sha256(), payload);
            assert_eq!(record.record_sha256().len(), 64);
        }
    }

    #[test]
    fn configuration_bound_results_are_deterministic_and_minimized() {
        let configuration = manager().safe_defaults().expect("configuration loads");
        let payload = "b".repeat(64);
        let first = manager()
            .bind_session_result(&configuration, "session-result-2", &payload)
            .expect("session result binds");
        let second = manager()
            .bind_session_result(&configuration, "session-result-2", &payload)
            .expect("session result binds");
        assert_eq!(first.record_sha256(), second.record_sha256());
        assert_eq!(first.canonical_bytes(), second.canonical_bytes());
        let text = std::str::from_utf8(first.canonical_bytes()).expect("record is UTF-8");
        assert!(text.contains(configuration.sha256()));
        assert!(!text.contains("permission"));
        assert!(!text.contains("workspace"));
    }

    #[test]
    fn configuration_or_result_changes_produce_distinct_binding_hashes() {
        let strict = manager().safe_defaults().expect("strict loads");
        let synthetic = manager()
            .load_bytes(SYNTHETIC_PROFILE)
            .expect("synthetic loads");
        let payload = "c".repeat(64);
        let strict_record = manager()
            .bind_release_result(&strict, "release-result-2", &payload)
            .expect("strict result binds");
        let synthetic_record = manager()
            .bind_release_result(&synthetic, "release-result-2", &payload)
            .expect("synthetic result binds");
        let changed_payload = manager()
            .bind_release_result(&strict, "release-result-2", &"d".repeat(64))
            .expect("changed result binds");
        assert_ne!(
            strict_record.record_sha256(),
            synthetic_record.record_sha256()
        );
        assert_ne!(
            strict_record.record_sha256(),
            changed_payload.record_sha256()
        );
    }

    #[test]
    fn configuration_bound_results_reject_invalid_identifiers_and_hashes() {
        let configuration = manager().safe_defaults().expect("configuration loads");
        for (result_id, payload) in [
            ("Invalid Result", "a".repeat(64)),
            ("session-result-3", "A".repeat(64)),
            ("session-result-3", "0".repeat(63)),
        ] {
            assert_eq!(
                manager()
                    .bind_session_result(&configuration, result_id, &payload)
                    .expect_err("invalid binding must fail")
                    .code(),
                "configuration-contract-violation"
            );
        }
    }

    #[test]
    fn safe_defaults_are_explicit_read_only_and_minimum_authority() {
        let defaults = manager().safe_defaults().expect("safe defaults load");
        assert_eq!(defaults.profile_id(), "strict-local-read-only");
        assert!(defaults.configuration.core.strict_local);
        assert!(defaults.configuration.tool.tools.is_empty());
        assert_eq!(
            defaults.configuration.permission.allowed_capabilities,
            ["workspace.read"]
        );
        assert_eq!(
            defaults.configuration.workspace.roots[0].access,
            "read-only"
        );
        assert!(!defaults.configuration.model.enabled);
        assert!(!defaults.configuration.shell.network_access);
    }

    #[test]
    fn migrates_only_version_zero_with_fixed_non_broadening_operations() {
        let mut legacy = fixture_value();
        legacy["schema_version"] = Value::from(0);
        for section in section_names() {
            legacy[section]
                .as_object_mut()
                .expect("section must be object")
                .remove("schema_version");
        }
        let core = legacy["core"].as_object_mut().expect("core must be object");
        let profile = core.remove("profile_id").expect("profile must exist");
        core.insert("profile".to_owned(), profile);
        let input = serde_json::to_vec(&legacy).expect("legacy must serialize");
        let first = manager().migrate_v0(&input).expect("migration succeeds");
        let second = manager().migrate_v0(&input).expect("migration succeeds");
        assert_eq!(first.source_version(), 0);
        assert_eq!(first.target_version(), 1);
        assert_eq!(first.changes().len(), 13);
        assert_eq!(
            first.configuration().sha256(),
            second.configuration().sha256()
        );
        assert_eq!(
            first
                .configuration()
                .configuration
                .permission
                .allowed_capabilities,
            ["workspace.read"]
        );
    }

    #[test]
    fn published_migration_fixtures_match_the_runtime_contract() {
        let migrated = manager()
            .migrate_v0(MIGRATION_V0_PROFILE)
            .expect("published version zero fixture migrates");
        let expected = manager()
            .load_bytes(MIGRATION_V1_EXPECTED)
            .expect("published version one fixture loads");
        assert_eq!(migrated.configuration().sha256(), expected.sha256());
        assert_eq!(
            migrated.configuration().canonical_bytes(),
            expected.canonical_bytes()
        );
        assert_eq!(migrated.changes().len(), 13);

        for (fixture, error_code) in [
            (
                MIGRATION_V1_NOT_MIGRATABLE,
                "configuration-unsupported-version",
            ),
            (
                MIGRATION_V0_RESERVED_VERSION,
                "configuration-contract-violation",
            ),
            (
                MIGRATION_V0_MISSING_SECTION,
                "configuration-contract-violation",
            ),
        ] {
            assert_eq!(
                manager()
                    .migrate_v0(fixture)
                    .expect_err("published invalid fixture must fail")
                    .code(),
                error_code
            );
        }
    }

    #[test]
    fn migration_rejects_wrong_source_ambiguous_version_and_missing_section() {
        assert_eq!(
            manager()
                .migrate_v0(SYNTHETIC_PROFILE)
                .expect_err("version one must fail")
                .code(),
            "configuration-unsupported-version"
        );
        let mut ambiguous = fixture_value();
        ambiguous["schema_version"] = Value::from(0);
        ambiguous["core"]["profile"] = Value::String("synthetic-test".to_owned());
        let bytes = serde_json::to_vec(&ambiguous).expect("fixture serializes");
        assert_eq!(
            manager()
                .migrate_v0(&bytes)
                .expect_err("reserved versions must fail")
                .code(),
            "configuration-contract-violation"
        );
        let mut missing = fixture_value();
        missing["schema_version"] = Value::from(0);
        missing
            .as_object_mut()
            .expect("root object")
            .remove("shell");
        let bytes = serde_json::to_vec(&missing).expect("fixture serializes");
        assert_eq!(
            manager()
                .migrate_v0(&bytes)
                .expect_err("missing section must fail")
                .code(),
            "configuration-contract-violation"
        );
    }

    #[test]
    fn diff_is_stable_redacted_and_classifies_authority_and_resource_changes() {
        let before = manager().safe_defaults().expect("defaults load");
        let mut changed = serde_json::to_value(&before.configuration).expect("serialize");
        changed["permission"]["allowed_capabilities"] =
            serde_json::json!(["workspace.read", "workspace.write"]);
        changed["budget"]["maximum_output_bytes"] = Value::from(524_288);
        let bytes = serde_json::to_vec(&changed).expect("serialize");
        let after = manager().load_bytes(&bytes).expect("changed config loads");
        let diff = manager().diff(&before, &after).expect("diff succeeds");
        assert!(diff.broadens_authority());
        assert_eq!(diff.before_sha256(), before.sha256());
        assert_eq!(diff.after_sha256(), after.sha256());
        assert_eq!(diff.changes().len(), 2);
        assert_eq!(diff.changes()[0].path(), "/budget/maximum_output_bytes");
        assert_eq!(diff.changes()[0].impact(), ChangeImpact::ResourceIncrease);
        assert_eq!(diff.changes()[0].before_sha256().len(), 64);
        assert_eq!(diff.changes()[0].after_sha256().len(), 64);
        assert_eq!(
            diff.changes()[1].impact(),
            ChangeImpact::AuthorityBroadening
        );
        let debug = format!("{diff:?}");
        assert!(!debug.contains("workspace.write"));
    }

    #[test]
    fn atomic_apply_retains_backup_and_rollback_restores_exact_identity() {
        let directory = temporary_directory();
        let target = directory.join("agentmage.json");
        fs::write(&target, STRICT_LOCAL_PROFILE).expect("fixture write succeeds");
        let before = manager().load_path(&target).expect("current loads");
        let receipt = manager()
            .apply_with_backup(&target, SYNTHETIC_PROFILE)
            .expect("apply succeeds");
        assert_eq!(receipt.previous_sha256(), before.sha256());
        assert!(receipt.backup_path().is_file());
        assert_eq!(
            manager()
                .load_path(&target)
                .expect("applied loads")
                .sha256(),
            receipt.applied_sha256()
        );
        let restored = manager()
            .rollback(&target, receipt.backup_path(), receipt.applied_sha256())
            .expect("rollback succeeds");
        assert_eq!(restored.sha256(), before.sha256());
        assert!(receipt.backup_path().is_file());
        fs::remove_dir_all(directory).expect("temporary directory removes");
    }

    #[test]
    fn migration_interruptions_select_valid_state_and_rollback_is_repeatable() {
        let transition_points = [
            (
                MigrationDurableTransition::Backup,
                TransitionBoundary::Before,
                false,
            ),
            (
                MigrationDurableTransition::Backup,
                TransitionBoundary::After,
                false,
            ),
            (
                MigrationDurableTransition::Candidate,
                TransitionBoundary::Before,
                false,
            ),
            (
                MigrationDurableTransition::Candidate,
                TransitionBoundary::After,
                false,
            ),
            (
                MigrationDurableTransition::Target,
                TransitionBoundary::Before,
                false,
            ),
            (
                MigrationDurableTransition::Target,
                TransitionBoundary::After,
                true,
            ),
        ];
        let expected_migrated = manager()
            .migrate_v0(MIGRATION_V0_PROFILE)
            .expect("migration fixture is valid");
        let expected_migrated_sha256 = expected_migrated.configuration().sha256().to_owned();
        let previous_sha256 = sha256(MIGRATION_V0_PROFILE);

        for (transition, boundary, migrated_selected) in transition_points {
            let directory = temporary_directory();
            let target = directory.join("agentmage.json");
            fs::write(&target, MIGRATION_V0_PROFILE).expect("legacy fixture writes");
            let interruption = manager()
                .migrate_path_v0_with_observer(&target, |observed, position| {
                    if observed == transition && position == boundary {
                        return Err(ConfigurationError::new(
                            ErrorCode::Io,
                            "synthetic migration interruption",
                        ));
                    }
                    Ok(())
                })
                .expect_err("selected transition must interrupt");
            assert_eq!(interruption.code(), "configuration-io-failure");
            assert_eq!(
                sha256(&fs::read(&target).expect("selected target reads"))
                    == expected_migrated_sha256,
                migrated_selected,
                "interruption must select exactly the prior or complete migrated state",
            );

            let receipt = if migrated_selected {
                let backup = backup_path(&target, &previous_sha256).expect("backup path builds");
                assert!(backup.is_file());
                MigrationApplyReceipt {
                    previous_sha256: previous_sha256.clone(),
                    migrated_sha256: expected_migrated_sha256.clone(),
                    backup_path: backup,
                    changes: expected_migrated.changes().to_vec(),
                }
            } else {
                manager()
                    .migrate_path_v0(&target)
                    .expect("migration resumes from prior state")
            };
            assert_eq!(receipt.previous_sha256(), previous_sha256);
            assert_eq!(receipt.migrated_sha256(), expected_migrated_sha256);
            assert_eq!(receipt.changes(), expected_migrated.changes());

            let first = manager()
                .rollback_migration(&target, receipt.backup_path(), receipt.migrated_sha256())
                .expect("first rollback restores the exact legacy preimage");
            assert!(!first.already_restored());
            assert_eq!(first.restored_sha256(), previous_sha256);
            assert_eq!(
                fs::read(&target).expect("restored target reads"),
                MIGRATION_V0_PROFILE,
            );
            let repeated = manager()
                .rollback_migration(&target, receipt.backup_path(), receipt.migrated_sha256())
                .expect("identical rollback retry succeeds");
            assert!(repeated.already_restored());
            assert_eq!(repeated.restored_sha256(), previous_sha256);
            assert!(receipt.backup_path().is_file());
            fs::remove_dir_all(directory).expect("temporary directory removes");
        }

        let directory = temporary_directory();
        let target = directory.join("agentmage.json");
        fs::write(&target, MIGRATION_V0_PROFILE).expect("legacy fixture writes");
        let conflict = manager()
            .migrate_path_v0_with_observer(&target, |transition, boundary| {
                if transition == MigrationDurableTransition::Target
                    && boundary == TransitionBoundary::Before
                {
                    fs::write(&target, SYNTHETIC_PROFILE)
                        .expect("concurrent replacement fixture writes");
                }
                Ok(())
            })
            .expect_err("changed preimage must prevent migration publication");
        assert_eq!(conflict.code(), "configuration-state-conflict");
        assert_eq!(
            fs::read(&target).expect("concurrent replacement reads"),
            SYNTHETIC_PROFILE,
        );
        fs::remove_dir_all(directory).expect("temporary directory removes");
    }

    #[test]
    fn invalid_candidate_and_stale_rollback_preimage_preserve_current_file() {
        let directory = temporary_directory();
        let target = directory.join("agentmage.json");
        fs::write(&target, STRICT_LOCAL_PROFILE).expect("fixture write succeeds");
        let before = fs::read(&target).expect("fixture reads");
        let invalid = mutate(
            fixture_value(),
            &["permission", "default_effect"],
            Value::String("allow".to_owned()),
        );
        assert!(manager().apply_with_backup(&target, &invalid).is_err());
        assert_eq!(fs::read(&target).expect("target reads"), before);
        let receipt = manager()
            .apply_with_backup(&target, SYNTHETIC_PROFILE)
            .expect("valid apply succeeds");
        assert_eq!(
            manager()
                .rollback(&target, receipt.backup_path(), "0")
                .expect_err("stale rollback must fail")
                .code(),
            "configuration-state-conflict"
        );
        assert_eq!(
            manager().load_path(&target).expect("target loads").sha256(),
            receipt.applied_sha256()
        );
        fs::remove_dir_all(directory).expect("temporary directory removes");
    }
}
