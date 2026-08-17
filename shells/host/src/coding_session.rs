//! Immutable composition profile for one bounded local coding session.

use std::fmt;
use std::fmt::Write;

use agentmage_kernel_contracts::{
    ExactModelProfile, ModelCapabilityState, ModelRole, RepositorySnapshotId, RuntimeRunLimits,
    RuntimeToolReference, ToolCatalogId,
};
use agentmage_kernel_engine::{
    command_runner::{CommandRegistry, CommandRequest, prepare_command},
    model_runtime::{AdmittedModelProfile, ModelUsePurpose},
    repository_safety::{OwnedWorktreeRecord, WorktreeDisposition},
    runtime_coordinator::{runtime_tool_catalog_sha256, validate_runtime_run_limits},
    runtime_loop::runtime_tool_references,
    strict_local::{AcquisitionExitDisposition, OfflineProofReceipt},
    tooling::ToolRegistry,
    validation_template::ValidationTemplateRegistry,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    coding_changes::CodingWriteScope,
    coding_tools::{CodingToolCatalogError, native_coding_runtime_registry},
};

/// Explicitly unavailable capability families in the first native coding profile.
pub const MVP_PROHIBITED_CAPABILITIES: [&str; 12] = [
    "autonomous-publication",
    "browser",
    "child-agent",
    "git-commit",
    "git-push",
    "mcp-dependency",
    "network-access",
    "package-install",
    "remote-git",
    "release",
    "workflow-execution",
    "workspace-delete",
];

/// Complete trusted inputs for one immutable coding-session profile.
pub struct CodingSessionProfileInput {
    /// Stable profile identity.
    pub profile_id: String,
    /// Exact frozen native tool-catalog identity.
    pub tool_catalog_id: ToolCatalogId,
    /// Exact repository observation identity.
    pub repository_snapshot_id: RepositorySnapshotId,
    /// Digest of the complete repository observation.
    pub repository_snapshot_sha256: String,
    /// Immutable source commit from which the owned worktree was created.
    pub immutable_base_commit: String,
    /// Verified active AgentMage-owned worktree record.
    pub worktree: OwnedWorktreeRecord,
    /// Exact admitted local model tuple with no fallback.
    pub model: AdmittedModelProfile,
    /// Exact offline proof whose session boundary owns this profile.
    pub offline_proof: OfflineProofReceipt,
    /// Expected offline session-boundary identity.
    pub session_boundary_sha256: [u8; 32],
    /// Exact writable path roots in the owned worktree.
    pub write_scope: CodingWriteScope,
    /// Frozen bounded command templates.
    pub commands: CommandRegistry,
    /// Frozen targeted validation templates.
    pub validations: ValidationTemplateRegistry,
    /// Inclusive coordinator ceilings.
    pub limits: RuntimeRunLimits,
}

/// Stable content-free profile construction refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodingSessionProfileError {
    /// A profile, repository, digest, base, or runtime-limit value is invalid.
    InvalidInput,
    /// The worktree is stale, inactive, mismatched, or not quiescent.
    WorktreeDenied,
    /// The selected model lacks exact admitted coding and tool-selection capability.
    ModelDenied,
    /// The offline proof does not bind the expected current session boundary.
    OfflineProofDenied,
    /// A validation template is stale or not backed by the exact command registry.
    ValidationDenied,
    /// The exact native tool catalog could not be composed.
    CatalogDenied,
}

impl CodingSessionProfileError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "coding-session.input.invalid",
            Self::WorktreeDenied => "coding-session.worktree.denied",
            Self::ModelDenied => "coding-session.model.denied",
            Self::OfflineProofDenied => "coding-session.offline-proof.denied",
            Self::ValidationDenied => "coding-session.validation.denied",
            Self::CatalogDenied => "coding-session.catalog.denied",
        }
    }
}

impl From<CodingToolCatalogError> for CodingSessionProfileError {
    fn from(_: CodingToolCatalogError) -> Self {
        Self::CatalogDenied
    }
}

/// Immutable, content-addressed native coding-session profile.
pub struct CodingSessionProfile {
    profile_id: String,
    tool_catalog_id: ToolCatalogId,
    tool_catalog_sha256: String,
    visible_tools: Vec<RuntimeToolReference>,
    repository_snapshot_id: RepositorySnapshotId,
    repository_snapshot_sha256: String,
    immutable_base_commit: String,
    worktree: OwnedWorktreeRecord,
    model: AdmittedModelProfile,
    offline_proof: OfflineProofReceipt,
    write_scope: CodingWriteScope,
    commands: CommandRegistry,
    validations: ValidationTemplateRegistry,
    limits: RuntimeRunLimits,
    registry: ToolRegistry,
    profile_sha256: String,
}

impl fmt::Debug for CodingSessionProfile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CodingSessionProfile")
            .field("profile_id", &self.profile_id)
            .field("tool_catalog_id", &self.tool_catalog_id)
            .field("repository_snapshot_id", &self.repository_snapshot_id)
            .field("worktree_id", &self.worktree.worktree_id)
            .field("model_profile_id", self.model.profile_id())
            .field("tool_count", &self.visible_tools.len())
            .field("profile_sha256", &self.profile_sha256)
            .finish_non_exhaustive()
    }
}

impl CodingSessionProfile {
    /// Validates and freezes one native coding-session composition.
    pub fn build(input: CodingSessionProfileInput) -> Result<Self, CodingSessionProfileError> {
        validate_static_input(&input)?;
        let registry = native_coding_runtime_registry(
            input.write_scope.clone(),
            input.commands.clone(),
            input.validations.clone(),
        )?;
        let visible_tools = runtime_tool_references(&registry)
            .map_err(|_| CodingSessionProfileError::CatalogDenied)?;
        let tool_catalog_sha256 =
            runtime_tool_catalog_sha256(&input.tool_catalog_id, &visible_tools)
                .map_err(|_| CodingSessionProfileError::CatalogDenied)?;
        let profile_sha256 = sha256_json(&ProfileMaterial {
            profile_id: &input.profile_id,
            tool_catalog_id: input.tool_catalog_id.as_str(),
            tool_catalog_sha256: &tool_catalog_sha256,
            visible_tools: &visible_tools,
            repository_snapshot_id: input.repository_snapshot_id.as_str(),
            repository_snapshot_sha256: &input.repository_snapshot_sha256,
            immutable_base_commit: &input.immutable_base_commit,
            worktree: &input.worktree,
            model: input.model.exact_profile(),
            model_purpose: model_purpose(input.model.purpose()),
            offline_proof_sha256: hex_bytes(input.offline_proof.proof_sha256()),
            session_boundary_sha256: hex_bytes(input.offline_proof.session_boundary_sha256()),
            writable_roots: input.write_scope.writable_roots(),
            commands: input.commands.commands(),
            validations: &input.validations,
            limits: &input.limits,
            prohibited_capabilities: &MVP_PROHIBITED_CAPABILITIES,
        })?;
        Ok(Self {
            profile_id: input.profile_id,
            tool_catalog_id: input.tool_catalog_id,
            tool_catalog_sha256,
            visible_tools,
            repository_snapshot_id: input.repository_snapshot_id,
            repository_snapshot_sha256: input.repository_snapshot_sha256,
            immutable_base_commit: input.immutable_base_commit,
            worktree: input.worktree,
            model: input.model,
            offline_proof: input.offline_proof,
            write_scope: input.write_scope,
            commands: input.commands,
            validations: input.validations,
            limits: input.limits,
            registry,
            profile_sha256,
        })
    }

    /// Returns the stable coding-profile identity.
    #[must_use]
    pub fn profile_id(&self) -> &str {
        &self.profile_id
    }

    /// Returns the exact frozen native tool-catalog identity.
    #[must_use]
    pub const fn tool_catalog_id(&self) -> &ToolCatalogId {
        &self.tool_catalog_id
    }

    /// Returns the digest of the complete visible tool catalog.
    #[must_use]
    pub fn tool_catalog_sha256(&self) -> &str {
        &self.tool_catalog_sha256
    }

    /// Returns stable exact references to every visible native tool.
    #[must_use]
    pub fn visible_tools(&self) -> &[RuntimeToolReference] {
        &self.visible_tools
    }

    /// Returns the exact repository observation identity.
    #[must_use]
    pub const fn repository_snapshot_id(&self) -> &RepositorySnapshotId {
        &self.repository_snapshot_id
    }

    /// Returns the digest of the complete repository observation.
    #[must_use]
    pub fn repository_snapshot_sha256(&self) -> &str {
        &self.repository_snapshot_sha256
    }

    /// Returns the immutable source commit for the owned worktree.
    #[must_use]
    pub fn immutable_base_commit(&self) -> &str {
        &self.immutable_base_commit
    }

    /// Returns the exact verified owned-worktree record.
    #[must_use]
    pub const fn worktree(&self) -> &OwnedWorktreeRecord {
        &self.worktree
    }

    /// Returns the exact admitted model tuple.
    #[must_use]
    pub const fn model_profile(&self) -> &ExactModelProfile {
        self.model.exact_profile()
    }

    /// Returns the exact offline proof bound to the profile.
    #[must_use]
    pub const fn offline_proof(&self) -> &OfflineProofReceipt {
        &self.offline_proof
    }

    /// Returns the exact writable scope.
    #[must_use]
    pub const fn write_scope(&self) -> &CodingWriteScope {
        &self.write_scope
    }

    /// Returns the exact frozen command registry.
    #[must_use]
    pub const fn commands(&self) -> &CommandRegistry {
        &self.commands
    }

    /// Returns the exact frozen validation registry.
    #[must_use]
    pub const fn validations(&self) -> &ValidationTemplateRegistry {
        &self.validations
    }

    /// Returns the inclusive runtime ceilings.
    #[must_use]
    pub const fn limits(&self) -> &RuntimeRunLimits {
        &self.limits
    }

    /// Returns the common non-executing native tool registry.
    #[must_use]
    pub const fn registry(&self) -> &ToolRegistry {
        &self.registry
    }

    /// Returns the canonical digest of every profile input and prohibition.
    #[must_use]
    pub fn profile_sha256(&self) -> &str {
        &self.profile_sha256
    }

    /// Consumes the profile and returns its exact common tool registry.
    #[must_use]
    pub fn into_registry(self) -> ToolRegistry {
        self.registry
    }
}

#[derive(Serialize)]
struct ProfileMaterial<'a> {
    profile_id: &'a str,
    tool_catalog_id: &'a str,
    tool_catalog_sha256: &'a str,
    visible_tools: &'a [RuntimeToolReference],
    repository_snapshot_id: &'a str,
    repository_snapshot_sha256: &'a str,
    immutable_base_commit: &'a str,
    worktree: &'a OwnedWorktreeRecord,
    model: &'a ExactModelProfile,
    model_purpose: &'static str,
    offline_proof_sha256: String,
    session_boundary_sha256: String,
    writable_roots: &'a [Vec<String>],
    commands: Vec<&'a agentmage_kernel_engine::command_runner::CommandSpec>,
    validations: &'a ValidationTemplateRegistry,
    limits: &'a RuntimeRunLimits,
    prohibited_capabilities: &'a [&'static str],
}

fn validate_static_input(
    input: &CodingSessionProfileInput,
) -> Result<(), CodingSessionProfileError> {
    if !valid_identifier(&input.profile_id)
        || !valid_identifier(input.tool_catalog_id.as_str())
        || !valid_identifier(input.repository_snapshot_id.as_str())
        || !is_sha256(&input.repository_snapshot_sha256)
        || !valid_object_id(&input.immutable_base_commit)
        || input.write_scope.workspace_id().as_str().is_empty()
        || validate_runtime_run_limits(&input.limits).is_err()
    {
        return Err(CodingSessionProfileError::InvalidInput);
    }
    input
        .worktree
        .verify()
        .map_err(|_| CodingSessionProfileError::WorktreeDenied)?;
    if input.worktree.disposition != WorktreeDisposition::Active
        || input.worktree.source_object != input.immutable_base_commit
        || input.worktree.live_process_count != 0
        || input.worktree.resource_budget_sha256 != sha256_json(&input.limits)?
    {
        return Err(CodingSessionProfileError::WorktreeDenied);
    }
    if input.model.exact_profile().automatic_fallback
        || ![ModelRole::CodingPlanner, ModelRole::ToolSelection]
            .into_iter()
            .all(|role| {
                input
                    .model
                    .exact_profile()
                    .capabilities
                    .iter()
                    .any(|capability| {
                        capability.role == role && capability.state == ModelCapabilityState::Passed
                    })
            })
    {
        return Err(CodingSessionProfileError::ModelDenied);
    }
    if input.offline_proof.disposition() != AcquisitionExitDisposition::Completed
        || input.offline_proof.session_boundary_sha256() != &input.session_boundary_sha256
    {
        return Err(CodingSessionProfileError::OfflineProofDenied);
    }
    if !input.validations.verify()
        || input.validations.templates.iter().any(|template| {
            template.execution_scope_sha256 != input.worktree.record_sha256
                || prepare_command(
                    &input.commands,
                    CommandRequest::new("coding-profile-validation", &template.command),
                )
                .is_err()
        })
    {
        return Err(CodingSessionProfileError::ValidationDenied);
    }
    Ok(())
}

const fn model_purpose(purpose: ModelUsePurpose) -> &'static str {
    match purpose {
        ModelUsePurpose::ContractTest => "contract_test",
        ModelUsePurpose::Evaluation => "evaluation",
        ModelUsePurpose::Product => "product",
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_alphanumeric() || (index > 0 && matches!(byte, b'.' | b'_' | b':' | b'-'))
        })
}

fn valid_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn sha256_json<T: Serialize>(value: &T) -> Result<String, CodingSessionProfileError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| CodingSessionProfileError::InvalidInput)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn hex_bytes(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use agentmage_kernel_contracts::{
        CONTRACT_SCHEMA_VERSION, ContextBudget, DecodingProfile, FamilyCodecIdentity,
        HardwareEnvelope, LocalEndpointIdentity, LocalTransport, ModelAdapterId, ModelArtifact,
        ModelCapability, ModelCodecId, ModelLifecycleState, ModelManifestId, ModelModality,
        ModelProfileId, ModelRuntimeIdentity, ModelRuntimeKind, NetworkComponent,
        NetworkDestinationClass, NetworkObservation, PlatformArchitecture, PlatformFamily,
        RuntimeRunLimits, WorkspaceId, WorkspacePath,
    };
    use agentmage_kernel_engine::{
        command_runner::{CommandBounds, CommandRisk, CommandSpec, CommandWorkingDirectory},
        model_runtime::ModelAdmissionCatalog,
        strict_local::{
            AcquisitionExitDisposition, NetworkAttemptLedger, OfflinePreflightObservation,
            OfflineProofWorkflow, StagedArtifactState, StrictLocalNetworkPolicy,
        },
        validation_template::{
            ValidationKind, ValidationParserKind, ValidationTemplateInput,
            ValidationTemplateSource, seal_validation_template,
        },
    };

    use super::*;

    const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn limits() -> RuntimeRunLimits {
        RuntimeRunLimits {
            max_turns: 16,
            max_model_calls: 32,
            max_tool_calls: 32,
            max_repeated_tool_calls: 4,
            max_tool_call_depth: 2,
            max_no_progress_turns: 4,
            max_context_refreshes: 16,
            max_events: 2_048,
            max_elapsed_ms: 600_000,
            max_output_bytes: 4 * 1024 * 1024,
        }
    }

    fn worktree(limits: &RuntimeRunLimits) -> OwnedWorktreeRecord {
        OwnedWorktreeRecord::seal(OwnedWorktreeRecord {
            schema_version: 0,
            worktree_id: "worktree-coding-0001".to_owned(),
            task_id: "task-coding-0001".to_owned(),
            source_object: "1".repeat(40),
            branch_ref: "refs/heads/agentmage/tasks/task-coding-0001".to_owned(),
            worktree_path_sha256: "2".repeat(64),
            owner_sha256: "3".repeat(64),
            grant_ids: Vec::new(),
            file_ownership_sha256: "4".repeat(64),
            live_process_count: 0,
            resource_budget_sha256: sha256_json(limits).expect("limit digest"),
            retain_until_epoch_ms: 10_000,
            clean: true,
            recovery_retained: false,
            disposition: WorktreeDisposition::Active,
            record_sha256: "0".repeat(64),
        })
        .expect("worktree")
    }

    fn command() -> CommandSpec {
        CommandSpec::seal(
            "fixture.cargo-test",
            "1.0.0",
            "/usr/bin/cargo",
            "5".repeat(64),
            vec!["test".to_owned(), "fixture".to_owned()],
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

    fn validations(
        command: CommandSpec,
        execution_scope_sha256: String,
    ) -> ValidationTemplateRegistry {
        let template = seal_validation_template(ValidationTemplateInput {
            validation_id: "validation-unit".to_owned(),
            kind: ValidationKind::Unit,
            command,
            source: ValidationTemplateSource::TrustedProjectConfiguration,
            source_sha256: "6".repeat(64),
            source_path: Some(
                WorkspacePath::new(WorkspaceId::from_raw("workspace-coding"), ["Cargo.toml"])
                    .expect("source path"),
            ),
            user_input_approval_sha256: None,
            execution_scope_sha256,
            parser: ValidationParserKind::AgentMageJsonV1,
            parser_version: "1.0.0".to_owned(),
            parser_sha256: "7".repeat(64),
            minimum_test_count: 1,
            focused: true,
            fail_fast: true,
            failed_test_rerun_allowed: true,
            setup_idempotent: true,
            expected_artifacts: Vec::new(),
        })
        .expect("validation template");
        ValidationTemplateRegistry::build(vec![template]).expect("validation registry")
    }

    fn admitted_model(with_tool_selection: bool) -> AdmittedModelProfile {
        let mut capabilities = vec![ModelCapability {
            role: ModelRole::CodingPlanner,
            state: ModelCapabilityState::Passed,
            evaluation_profile: Some("coding-eval-v1".to_owned()),
            result_sha256: Some("8".repeat(64)),
            limitations: vec!["fixture-only".to_owned()],
        }];
        if with_tool_selection {
            capabilities.push(ModelCapability {
                role: ModelRole::ToolSelection,
                state: ModelCapabilityState::Passed,
                evaluation_profile: Some("tool-eval-v1".to_owned()),
                result_sha256: Some("9".repeat(64)),
                limitations: vec!["fixture-only".to_owned()],
            });
        }
        let profile = ExactModelProfile {
            schema_version: CONTRACT_SCHEMA_VERSION,
            profile_id: ModelProfileId::from_raw("fixture-coding-profile"),
            manifest_id: ModelManifestId::from_raw("fixture-coding-manifest"),
            manifest_sha256: SHA.to_owned(),
            display_name: "Fixture coding profile".to_owned(),
            family: "deterministic_fake_coding".to_owned(),
            publisher_control: "fixture".to_owned(),
            lineage: vec!["fixture-source".to_owned()],
            license_spdx: "Apache-2.0".to_owned(),
            license_terms_sha256: SHA.to_owned(),
            artifact: ModelArtifact {
                artifact_id: "fixture-artifact".to_owned(),
                publisher: "fixture".to_owned(),
                source_revision: "fixture-revision".to_owned(),
                format: "fixture".to_owned(),
                bytes: 1,
                sha256: SHA.to_owned(),
            },
            transformations: Vec::new(),
            codec: FamilyCodecIdentity {
                codec_id: ModelCodecId::from_raw("fixture-coding-codec"),
                codec_version: "1.0.0".to_owned(),
                codec_sha256: SHA.to_owned(),
                tokenizer: "fixture-tokenizer".to_owned(),
                tokenizer_sha256: SHA.to_owned(),
                template: "fixture-template".to_owned(),
                template_sha256: SHA.to_owned(),
                tool_protocol_version: "closed-v1".to_owned(),
                end_tokens: vec![1],
                reasoning_enabled: false,
            },
            runtime: ModelRuntimeIdentity {
                adapter_id: ModelAdapterId::from_raw("fixture-coding-adapter"),
                kind: ModelRuntimeKind::DeterministicFake,
                contract_version: 1,
                runtime_build: "fixture-runtime-v1".to_owned(),
                runtime_sha256: SHA.to_owned(),
                platform: PlatformFamily::DeterministicFake,
                architecture: PlatformArchitecture::X86_64,
            },
            quantization: "none".to_owned(),
            modalities: vec![ModelModality::Text],
            context: ContextBudget {
                max_context_tokens: 128,
                max_input_bytes: 1_024,
                max_messages: 4,
                token_counter: "fixture-counter-v1".to_owned(),
                token_counter_sha256: SHA.to_owned(),
            },
            decoding: DecodingProfile {
                profile_id: "fixture-decoding".to_owned(),
                sampler_order: vec!["exact".to_owned()],
                temperature: 0.0,
                top_p: 1.0,
                top_k: 1,
                repeat_penalty: 1.0,
                seed: 1,
                max_output_tokens: 16,
            },
            hardware: vec![HardwareEnvelope {
                platform: PlatformFamily::DeterministicFake,
                architecture: PlatformArchitecture::X86_64,
                minimum_system_memory_bytes: 1,
                minimum_accelerator_memory_bytes: 0,
                accelerator: "none".to_owned(),
                driver_constraint: "none".to_owned(),
            }],
            capabilities,
            policy_sha256: SHA.to_owned(),
            lifecycle: ModelLifecycleState::Candidate,
            enabled: false,
            automatic_fallback: false,
        };
        ModelAdmissionCatalog::new(vec![profile.clone()])
            .expect("model catalog")
            .admit(&profile, ModelUsePurpose::ContractTest)
            .expect("admitted model")
    }

    fn endpoint() -> LocalEndpointIdentity {
        LocalEndpointIdentity::new(
            NetworkComponent::KernelNativeInferenceAdapter,
            LocalTransport::AuthenticatedUnixSocket,
            [7; 32],
        )
        .expect("endpoint")
    }

    fn offline_proof() -> OfflineProofReceipt {
        let policy = StrictLocalNetworkPolicy::new(endpoint());
        let mut ledger = NetworkAttemptLedger::new();
        ledger
            .evaluate_and_record(
                &policy,
                &NetworkObservation {
                    component: NetworkComponent::KernelNativeInferenceAdapter,
                    destination: NetworkDestinationClass::AuthenticatedLocalSocket,
                    transport: Some(LocalTransport::AuthenticatedUnixSocket),
                    endpoint_sha256: Some([7; 32]),
                    peer_authenticated: true,
                    attempted_bytes: 128,
                },
                [31; 32],
                110,
            )
            .expect("local inference attempt");
        let mut workflow = OfflineProofWorkflow::new(7, 100, 0);
        workflow
            .record_exit(AcquisitionExitDisposition::Completed, 200)
            .expect("acquisition exit");
        workflow
            .prove(
                &OfflinePreflightObservation {
                    observed_at_millis: 201,
                    acquisition_process_count: 0,
                    acquisition_socket_count: 0,
                    external_network_rule_count: 0,
                    observed_outbound_bytes: 0,
                    observed_dns_attempts: 0,
                    staged_artifact_state: StagedArtifactState::ActivatedVerified,
                    session_boundary_sha256: [21; 32],
                    firewall_policy_sha256: [22; 32],
                },
                &ledger,
            )
            .expect("offline proof")
    }

    fn input() -> CodingSessionProfileInput {
        let limits = limits();
        let worktree = worktree(&limits);
        let command = command();
        CodingSessionProfileInput {
            profile_id: "coding-profile-0001".to_owned(),
            tool_catalog_id: ToolCatalogId::from_raw("coding-tools-0001"),
            repository_snapshot_id: RepositorySnapshotId::from_raw("repository-snapshot-0001"),
            repository_snapshot_sha256: "a".repeat(64),
            immutable_base_commit: worktree.source_object.clone(),
            validations: validations(command.clone(), worktree.record_sha256.clone()),
            commands: CommandRegistry::build(vec![command]).expect("command registry"),
            worktree,
            model: admitted_model(true),
            offline_proof: offline_proof(),
            session_boundary_sha256: [21; 32],
            write_scope: CodingWriteScope::new(
                WorkspaceId::from_raw("workspace-coding"),
                vec![vec!["src".to_owned()], vec!["tests".to_owned()]],
            )
            .expect("write scope"),
            limits,
        }
    }

    #[test]
    fn story_48_2_profile_binds_repository_worktree_model_offline_state_and_tools() {
        let profile = CodingSessionProfile::build(input()).expect("coding profile");
        assert_eq!(profile.visible_tools().len(), 15);
        assert_eq!(profile.immutable_base_commit(), "1".repeat(40));
        assert_eq!(profile.worktree().disposition, WorktreeDisposition::Active);
        assert_eq!(
            profile.model_profile().profile_id.as_str(),
            "fixture-coding-profile"
        );
        assert_eq!(profile.profile_sha256().len(), 64);
        assert_eq!(profile.tool_catalog_sha256().len(), 64);
        assert!(
            profile
                .visible_tools()
                .iter()
                .all(|tool| !tool.tool_id.as_str().contains("mcp"))
        );

        let second = CodingSessionProfile::build(input()).expect("second profile");
        assert_eq!(profile.profile_sha256(), second.profile_sha256());
    }

    #[test]
    fn story_48_2_profile_rejects_every_stale_foundational_binding() {
        let mut wrong_base = input();
        wrong_base.immutable_base_commit = "f".repeat(40);
        assert_eq!(
            CodingSessionProfile::build(wrong_base).expect_err("wrong base"),
            CodingSessionProfileError::WorktreeDenied
        );

        let mut wrong_offline = input();
        wrong_offline.session_boundary_sha256 = [99; 32];
        assert_eq!(
            CodingSessionProfile::build(wrong_offline).expect_err("wrong offline boundary"),
            CodingSessionProfileError::OfflineProofDenied
        );

        let mut wrong_model = input();
        wrong_model.model = admitted_model(false);
        assert_eq!(
            CodingSessionProfile::build(wrong_model).expect_err("missing tool capability"),
            CodingSessionProfileError::ModelDenied
        );

        let mut stale_validation = input();
        let command = command();
        stale_validation.validations = validations(command, "e".repeat(64));
        assert_eq!(
            CodingSessionProfile::build(stale_validation).expect_err("stale validation"),
            CodingSessionProfileError::ValidationDenied
        );
    }
}
