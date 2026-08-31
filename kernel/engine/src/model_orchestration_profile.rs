//! Checked model context allocation and authority-invariant orchestration adaptation.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{CONTRACT_SCHEMA_VERSION, ExactModelProfile, ModelLifecycleState};
use serde::Serialize;
use sha2::{Digest, Sha256};

const MAX_ID_BYTES: usize = 256;
const MAX_VISIBLE_TOOLS: usize = 64;
const MAX_QUALIFICATIONS: usize = 128;
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Stable refusal from model-specific context/orchestration compilation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelOrchestrationError {
    /// An identity, digest, bound, order, or collection is malformed.
    InvalidInput,
    /// The supplied counter is not the exact counter and tokenizer pinned by the profile.
    TokenCounterMismatch,
    /// Mandatory fixed partitions or authoritative source minimums exceed the exact window.
    ContextOvercommit,
    /// Required role/workflow qualification evidence is absent or stale.
    QualificationMissing,
    /// A candidate changed an invariant authority, effect, verification, or completion control.
    SafetyControlDrift,
    /// Canonical hashing failed.
    SerializationFailed,
}

/// Exact authoritative token-counter observation used for one allocation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ExactTokenCounterBinding {
    /// Token-counter implementation identity.
    pub token_counter_id: String,
    /// Digest of that implementation.
    pub token_counter_sha256: String,
    /// Digest of the exact profile tokenizer.
    pub tokenizer_sha256: String,
}

/// Requested and minimum tokens for one adaptable partition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct AdaptableTokenDemand {
    /// Tokens required for complete inclusion.
    pub requested_tokens: u32,
    /// Lowest useful token allocation.
    pub minimum_tokens: u32,
}

/// Complete input to deterministic context-window allocation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ContextWindowDemand {
    /// Exact authoritative counter binding.
    pub counter: ExactTokenCounterBinding,
    /// Fixed system instructions and tool definitions.
    pub system_and_tool_tokens: u32,
    /// Exact user input.
    pub user_input_tokens: u32,
    /// Authoritative source-artifact context; its minimum cannot be omitted.
    pub source_artifacts: AdaptableTokenDemand,
    /// Optional retrieved context; it may be summarized or omitted.
    pub retrieved_context: AdaptableTokenDemand,
    /// Fixed workflow and recovery reserve.
    pub workflow_recovery_reserve_tokens: u32,
    /// Exact output reserve; must equal the decoding profile ceiling.
    pub output_reserve_tokens: u32,
    /// Non-spendable safety margin.
    pub safety_margin_tokens: u32,
}

/// Visible deterministic disposition for an adaptable allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextPartitionDisposition {
    /// The complete requested partition fits.
    Included,
    /// Authoritative source context was reduced to its declared useful minimum.
    Truncated,
    /// Retrieved context was reduced to its declared useful minimum.
    Summarized,
    /// Optional retrieved context did not fit and was omitted.
    Omitted,
}

/// One allocated variable context partition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AllocatedContextPartition {
    /// Tokens requested.
    pub requested_tokens: u32,
    /// Declared lowest useful allocation, retained for identity and stale-profile detection.
    pub minimum_tokens: u32,
    /// Tokens actually allocated.
    pub allocated_tokens: u32,
    /// Deterministic disposition.
    pub disposition: ContextPartitionDisposition,
    /// Stable visible reason code for a non-complete disposition.
    pub reason_code: Option<&'static str>,
}

/// Exact checked model-context plan fed into later context composition.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ModelContextWindowPlan {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact selected model profile.
    pub model_profile_id: String,
    /// Exact profile manifest digest.
    pub model_manifest_sha256: String,
    /// Exact runtime digest.
    pub model_runtime_sha256: String,
    /// Exact tokenizer digest.
    pub tokenizer_sha256: String,
    /// Exact token counter digest.
    pub token_counter_sha256: String,
    /// Total profile window.
    pub total_window_tokens: u32,
    /// Fixed system/tool allocation.
    pub system_and_tool_tokens: u32,
    /// Fixed user-input allocation.
    pub user_input_tokens: u32,
    /// Source-artifact allocation and disposition.
    pub source_artifacts: AllocatedContextPartition,
    /// Retrieved-context allocation and disposition.
    pub retrieved_context: AllocatedContextPartition,
    /// Workflow/recovery reserve.
    pub workflow_recovery_reserve_tokens: u32,
    /// Output reserve.
    pub output_reserve_tokens: u32,
    /// Safety margin.
    pub safety_margin_tokens: u32,
    /// Unallocated tokens left after deterministic allocation.
    pub unallocated_tokens: u32,
    /// Digest of this plan with this field zeroed.
    pub plan_sha256: String,
}

/// Parser repair permitted only at the model proposal boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParserRepairAllowance {
    /// No repair is permitted.
    None,
    /// One schema-local targeted repair is permitted.
    OneTargetedRepair,
}

/// Amount of non-authoritative recovery structure shown to a model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryScaffolding {
    /// Only the current safe action and blocker codes.
    Minimal,
    /// Current safe action, blocker codes, and bounded prior-attempt identities.
    BoundedHistory,
}

/// Content-free diagnostic detail requested from the model edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticVerbosity {
    /// Stable reason codes only.
    CodesOnly,
    /// Codes plus bounded user-visible explanation.
    BoundedExplanation,
}

/// Model-varying presentation and planning shape with no authority fields.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OrchestrationShape {
    /// Maximum number of visible future plan steps.
    pub plan_horizon: u16,
    /// Ordered exact subset of descriptive tool identities shown to the model.
    pub visible_tool_ids: Vec<String>,
    /// Maximum tokens from one tool observation shown to the model.
    pub max_observation_tokens: u32,
    /// Permitted proposal-parser repair.
    pub parser_repair: ParserRepairAllowance,
    /// Recovery context presentation.
    pub recovery_scaffolding: RecoveryScaffolding,
    /// Requested diagnostic presentation.
    pub diagnostic_verbosity: DiagnosticVerbosity,
}

/// Hashes of every safety/authority control that must remain model invariant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ModelInvariantControls {
    /// Deterministic policy digest.
    pub policy_sha256: String,
    /// Capability-grant contract digest.
    pub grant_contract_sha256: String,
    /// Approval policy digest.
    pub approval_policy_sha256: String,
    /// Side-effect classification digest.
    pub side_effect_policy_sha256: String,
    /// Retry eligibility digest.
    pub retry_policy_sha256: String,
    /// Verifier policy digest.
    pub verifier_policy_sha256: String,
    /// Runtime budget policy digest.
    pub budget_policy_sha256: String,
    /// Completion authority digest.
    pub completion_policy_sha256: String,
}

/// Independent quality evidence for one exact role and workflow tuple.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OrchestrationQualification {
    /// Exact role identity.
    pub role_id: String,
    /// Exact workflow/corpus identity.
    pub workflow_id: String,
    /// Digest of current independent evidence.
    pub evidence_sha256: String,
    /// Whether all declared thresholds passed for this exact tuple.
    pub passed: bool,
}

/// One immutable context/orchestration profile.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CompiledModelOrchestrationProfile {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable orchestration-profile identity.
    pub orchestration_profile_id: String,
    /// Exact model profile.
    pub model_profile_id: String,
    /// Exact model manifest digest.
    pub model_manifest_sha256: String,
    /// Checked context allocation.
    pub context: ModelContextWindowPlan,
    /// Model-varying presentation shape.
    pub shape: OrchestrationShape,
    /// Invariant safety control tuple.
    pub invariant_controls: ModelInvariantControls,
    /// Exact independent role/workflow qualifications.
    pub qualifications: Vec<OrchestrationQualification>,
    /// True only when the model itself is selectable and every declared qualification passed.
    pub enabled: bool,
    /// Always false; profile compilation never enables fallback.
    pub automatic_fallback: bool,
    /// Digest of this profile with this field zeroed.
    pub profile_sha256: String,
}

/// Allocates the exact profile context window with checked arithmetic and visible degradation.
pub fn compile_context_window(
    profile: &ExactModelProfile,
    demand: &ContextWindowDemand,
) -> Result<ModelContextWindowPlan, ModelOrchestrationError> {
    validate_profile_binding(profile)?;
    if demand.counter.token_counter_id != profile.context.token_counter
        || demand.counter.token_counter_sha256 != profile.context.token_counter_sha256
        || demand.counter.tokenizer_sha256 != profile.codec.tokenizer_sha256
    {
        return Err(ModelOrchestrationError::TokenCounterMismatch);
    }
    if demand.output_reserve_tokens != profile.decoding.max_output_tokens
        || demand.source_artifacts.minimum_tokens > demand.source_artifacts.requested_tokens
        || demand.retrieved_context.minimum_tokens > demand.retrieved_context.requested_tokens
        || demand.system_and_tool_tokens == 0
        || demand.user_input_tokens == 0
        || demand.workflow_recovery_reserve_tokens == 0
        || demand.output_reserve_tokens == 0
        || demand.safety_margin_tokens == 0
    {
        return Err(ModelOrchestrationError::InvalidInput);
    }
    let total = profile.context.max_context_tokens;
    let fixed = [
        demand.system_and_tool_tokens,
        demand.user_input_tokens,
        demand.workflow_recovery_reserve_tokens,
        demand.output_reserve_tokens,
        demand.safety_margin_tokens,
    ]
    .into_iter()
    .try_fold(0_u32, u32::checked_add)
    .ok_or(ModelOrchestrationError::ContextOvercommit)?;
    let mut remaining = total
        .checked_sub(fixed)
        .ok_or(ModelOrchestrationError::ContextOvercommit)?;
    let source = allocate_source(demand.source_artifacts, remaining)?;
    remaining = remaining
        .checked_sub(source.allocated_tokens)
        .ok_or(ModelOrchestrationError::ContextOvercommit)?;
    let retrieved = allocate_retrieved(demand.retrieved_context, remaining);
    remaining = remaining
        .checked_sub(retrieved.allocated_tokens)
        .ok_or(ModelOrchestrationError::ContextOvercommit)?;
    let mut plan = ModelContextWindowPlan {
        schema_version: CONTRACT_SCHEMA_VERSION,
        model_profile_id: profile.profile_id.as_str().to_owned(),
        model_manifest_sha256: profile.manifest_sha256.clone(),
        model_runtime_sha256: profile.runtime.runtime_sha256.clone(),
        tokenizer_sha256: profile.codec.tokenizer_sha256.clone(),
        token_counter_sha256: profile.context.token_counter_sha256.clone(),
        total_window_tokens: total,
        system_and_tool_tokens: demand.system_and_tool_tokens,
        user_input_tokens: demand.user_input_tokens,
        source_artifacts: source,
        retrieved_context: retrieved,
        workflow_recovery_reserve_tokens: demand.workflow_recovery_reserve_tokens,
        output_reserve_tokens: demand.output_reserve_tokens,
        safety_margin_tokens: demand.safety_margin_tokens,
        unallocated_tokens: remaining,
        plan_sha256: ZERO_SHA256.to_owned(),
    };
    plan.plan_sha256 = digest(&plan)?;
    Ok(plan)
}

/// Compiles a model-specific presentation profile while preserving exact safety controls.
pub fn compile_orchestration_profile(
    orchestration_profile_id: String,
    profile: &ExactModelProfile,
    context: ModelContextWindowPlan,
    shape: OrchestrationShape,
    invariant_controls: ModelInvariantControls,
    qualifications: Vec<OrchestrationQualification>,
) -> Result<CompiledModelOrchestrationProfile, ModelOrchestrationError> {
    validate_profile_binding(profile)?;
    if !valid_id(&orchestration_profile_id)
        || context.model_profile_id != profile.profile_id.as_str()
        || context.model_manifest_sha256 != profile.manifest_sha256
        || context.tokenizer_sha256 != profile.codec.tokenizer_sha256
        || !valid_sha256(&context.plan_sha256)
        || plan_digest(&context)? != context.plan_sha256
        || shape.plan_horizon == 0
        || shape.plan_horizon > 64
        || shape.visible_tool_ids.len() > MAX_VISIBLE_TOOLS
        || shape.max_observation_tokens == 0
        || qualifications.is_empty()
        || qualifications.len() > MAX_QUALIFICATIONS
        || !valid_controls(&invariant_controls)
    {
        return Err(ModelOrchestrationError::InvalidInput);
    }
    let mut tools = BTreeSet::new();
    if shape
        .visible_tool_ids
        .iter()
        .any(|identity| !valid_id(identity) || !tools.insert(identity.as_str()))
    {
        return Err(ModelOrchestrationError::InvalidInput);
    }
    let mut qualification_keys = BTreeSet::new();
    for qualification in &qualifications {
        if !valid_id(&qualification.role_id)
            || !valid_id(&qualification.workflow_id)
            || !valid_sha256(&qualification.evidence_sha256)
            || !qualification_keys.insert((
                qualification.role_id.as_str(),
                qualification.workflow_id.as_str(),
            ))
        {
            return Err(ModelOrchestrationError::InvalidInput);
        }
    }
    if qualifications.iter().any(|item| !item.passed) {
        return Err(ModelOrchestrationError::QualificationMissing);
    }
    let enabled = profile.enabled
        && matches!(
            profile.lifecycle,
            ModelLifecycleState::Approved | ModelLifecycleState::Degraded
        );
    let mut compiled = CompiledModelOrchestrationProfile {
        schema_version: CONTRACT_SCHEMA_VERSION,
        orchestration_profile_id,
        model_profile_id: profile.profile_id.as_str().to_owned(),
        model_manifest_sha256: profile.manifest_sha256.clone(),
        context,
        shape,
        invariant_controls,
        qualifications,
        enabled,
        automatic_fallback: false,
        profile_sha256: ZERO_SHA256.to_owned(),
    };
    compiled.profile_sha256 = digest(&compiled)?;
    Ok(compiled)
}

/// Proves two candidate adaptations preserve the exact authority/safety control tuple.
pub fn verify_model_invariant_controls(
    expected: &ModelInvariantControls,
    candidate: &CompiledModelOrchestrationProfile,
) -> Result<(), ModelOrchestrationError> {
    if expected == &candidate.invariant_controls && valid_controls(expected) {
        Ok(())
    } else {
        Err(ModelOrchestrationError::SafetyControlDrift)
    }
}

fn allocate_source(
    demand: AdaptableTokenDemand,
    available: u32,
) -> Result<AllocatedContextPartition, ModelOrchestrationError> {
    if demand.requested_tokens <= available {
        Ok(partition(
            demand.requested_tokens,
            demand.minimum_tokens,
            demand.requested_tokens,
            ContextPartitionDisposition::Included,
            None,
        ))
    } else if demand.minimum_tokens <= available {
        Ok(partition(
            demand.requested_tokens,
            demand.minimum_tokens,
            available,
            ContextPartitionDisposition::Truncated,
            Some("context.source.truncated_to_window"),
        ))
    } else {
        Err(ModelOrchestrationError::ContextOvercommit)
    }
}

fn allocate_retrieved(demand: AdaptableTokenDemand, available: u32) -> AllocatedContextPartition {
    if demand.requested_tokens <= available {
        partition(
            demand.requested_tokens,
            demand.minimum_tokens,
            demand.requested_tokens,
            ContextPartitionDisposition::Included,
            None,
        )
    } else if demand.minimum_tokens > 0 && demand.minimum_tokens <= available {
        partition(
            demand.requested_tokens,
            demand.minimum_tokens,
            available,
            ContextPartitionDisposition::Summarized,
            Some("context.retrieval.summarized_to_window"),
        )
    } else {
        partition(
            demand.requested_tokens,
            demand.minimum_tokens,
            0,
            ContextPartitionDisposition::Omitted,
            Some("context.retrieval.omitted_for_window"),
        )
    }
}

fn partition(
    requested_tokens: u32,
    minimum_tokens: u32,
    allocated_tokens: u32,
    disposition: ContextPartitionDisposition,
    reason_code: Option<&'static str>,
) -> AllocatedContextPartition {
    AllocatedContextPartition {
        requested_tokens,
        minimum_tokens,
        allocated_tokens,
        disposition,
        reason_code,
    }
}

fn validate_profile_binding(profile: &ExactModelProfile) -> Result<(), ModelOrchestrationError> {
    if profile.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_id(profile.profile_id.as_str())
        || !valid_sha256(&profile.manifest_sha256)
        || !valid_sha256(&profile.codec.tokenizer_sha256)
        || !valid_sha256(&profile.context.token_counter_sha256)
        || !valid_sha256(&profile.runtime.runtime_sha256)
        || profile.context.max_context_tokens == 0
        || profile.context.max_input_bytes == 0
        || profile.context.max_messages == 0
        || !valid_id(&profile.context.token_counter)
    {
        Err(ModelOrchestrationError::InvalidInput)
    } else {
        Ok(())
    }
}

fn valid_controls(controls: &ModelInvariantControls) -> bool {
    [
        &controls.policy_sha256,
        &controls.grant_contract_sha256,
        &controls.approval_policy_sha256,
        &controls.side_effect_policy_sha256,
        &controls.retry_policy_sha256,
        &controls.verifier_policy_sha256,
        &controls.budget_policy_sha256,
        &controls.completion_policy_sha256,
    ]
    .into_iter()
    .all(|value| valid_sha256(value))
}

fn digest<T: Serialize>(value: &T) -> Result<String, ModelOrchestrationError> {
    let bytes =
        serde_json::to_vec(value).map_err(|_| ModelOrchestrationError::SerializationFailed)?;
    Ok(hex(&Sha256::digest(bytes)))
}

fn plan_digest(value: &ModelContextWindowPlan) -> Result<String, ModelOrchestrationError> {
    let mut candidate = value.clone();
    candidate.plan_sha256 = ZERO_SHA256.to_owned();
    digest(&candidate)
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ID_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':' | b'/')
        })
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        CONTRACT_SCHEMA_VERSION, ContextBudget, DecodingProfile, ExactModelProfile,
        FamilyCodecIdentity, ModelAdapterId, ModelArtifact, ModelCodecId, ModelLifecycleState,
        ModelManifestId, ModelModality, ModelProfileId, ModelRuntimeIdentity, ModelRuntimeKind,
        PlatformArchitecture, PlatformFamily,
    };

    use super::*;

    fn hash(byte: char) -> String {
        byte.to_string().repeat(64)
    }

    fn profile(enabled: bool) -> ExactModelProfile {
        ExactModelProfile {
            schema_version: CONTRACT_SCHEMA_VERSION,
            profile_id: ModelProfileId::from_raw("model-context-profile-1"),
            manifest_id: ModelManifestId::from_raw("model-context-manifest-1"),
            manifest_sha256: hash('a'),
            display_name: "Fixture".to_owned(),
            family: "fixture".to_owned(),
            publisher_control: "fixture".to_owned(),
            lineage: vec!["fixture".to_owned()],
            license_spdx: "Apache-2.0".to_owned(),
            license_terms_sha256: hash('b'),
            artifact: ModelArtifact {
                artifact_id: "artifact-1".to_owned(),
                publisher: "fixture".to_owned(),
                source_revision: "revision-1".to_owned(),
                format: "fixture".to_owned(),
                bytes: 1,
                sha256: hash('c'),
            },
            transformations: Vec::new(),
            codec: FamilyCodecIdentity {
                codec_id: ModelCodecId::from_raw("codec-1"),
                codec_version: "1".to_owned(),
                codec_sha256: hash('d'),
                tokenizer: "tokenizer-1".to_owned(),
                tokenizer_sha256: hash('e'),
                template: "template-1".to_owned(),
                template_sha256: hash('f'),
                tool_protocol_version: "1".to_owned(),
                end_tokens: vec![1],
                reasoning_enabled: false,
            },
            runtime: ModelRuntimeIdentity {
                adapter_id: ModelAdapterId::from_raw("adapter-1"),
                kind: ModelRuntimeKind::DeterministicFake,
                contract_version: 1,
                runtime_build: "runtime-1".to_owned(),
                runtime_sha256: hash('1'),
                platform: PlatformFamily::Fedora,
                architecture: PlatformArchitecture::X86_64,
            },
            quantization: "fixture".to_owned(),
            modalities: vec![ModelModality::Text],
            context: ContextBudget {
                max_context_tokens: 1024,
                max_input_bytes: 4096,
                max_messages: 16,
                token_counter: "counter-1".to_owned(),
                token_counter_sha256: hash('2'),
            },
            decoding: DecodingProfile {
                profile_id: "decoding-1".to_owned(),
                sampler_order: vec!["temperature".to_owned()],
                temperature: 0.0,
                top_p: 1.0,
                top_k: 1,
                repeat_penalty: 1.0,
                seed: 1,
                max_output_tokens: 128,
            },
            hardware: Vec::new(),
            capabilities: Vec::new(),
            policy_sha256: hash('3'),
            lifecycle: ModelLifecycleState::Approved,
            enabled,
            automatic_fallback: false,
        }
    }

    fn demand() -> ContextWindowDemand {
        ContextWindowDemand {
            counter: ExactTokenCounterBinding {
                token_counter_id: "counter-1".to_owned(),
                token_counter_sha256: hash('2'),
                tokenizer_sha256: hash('e'),
            },
            system_and_tool_tokens: 100,
            user_input_tokens: 100,
            source_artifacts: AdaptableTokenDemand {
                requested_tokens: 300,
                minimum_tokens: 200,
            },
            retrieved_context: AdaptableTokenDemand {
                requested_tokens: 200,
                minimum_tokens: 50,
            },
            workflow_recovery_reserve_tokens: 96,
            output_reserve_tokens: 128,
            safety_margin_tokens: 100,
        }
    }

    fn controls() -> ModelInvariantControls {
        ModelInvariantControls {
            policy_sha256: hash('1'),
            grant_contract_sha256: hash('2'),
            approval_policy_sha256: hash('3'),
            side_effect_policy_sha256: hash('4'),
            retry_policy_sha256: hash('5'),
            verifier_policy_sha256: hash('6'),
            budget_policy_sha256: hash('7'),
            completion_policy_sha256: hash('8'),
        }
    }

    fn qualification() -> Vec<OrchestrationQualification> {
        vec![OrchestrationQualification {
            role_id: "role-coding".to_owned(),
            workflow_id: "workflow-read-only".to_owned(),
            evidence_sha256: hash('9'),
            passed: true,
        }]
    }

    fn shape() -> OrchestrationShape {
        OrchestrationShape {
            plan_horizon: 4,
            visible_tool_ids: vec!["tool.read".to_owned()],
            max_observation_tokens: 128,
            parser_repair: ParserRepairAllowance::OneTargetedRepair,
            recovery_scaffolding: RecoveryScaffolding::BoundedHistory,
            diagnostic_verbosity: DiagnosticVerbosity::BoundedExplanation,
        }
    }

    #[test]
    fn story_13_4_exact_window_reconciles_without_overcommit() {
        let plan = compile_context_window(&profile(true), &demand()).expect("exact allocation");
        let spent = plan.system_and_tool_tokens
            + plan.user_input_tokens
            + plan.source_artifacts.allocated_tokens
            + plan.retrieved_context.allocated_tokens
            + plan.workflow_recovery_reserve_tokens
            + plan.output_reserve_tokens
            + plan.safety_margin_tokens
            + plan.unallocated_tokens;
        assert_eq!(spent, plan.total_window_tokens);
        assert_eq!(
            plan.source_artifacts.disposition,
            ContextPartitionDisposition::Included
        );
        assert_eq!(
            plan.retrieved_context.disposition,
            ContextPartitionDisposition::Included
        );
    }

    #[test]
    fn story_13_4_degradation_is_deterministic_visible_and_source_first() {
        let mut value = demand();
        value.source_artifacts.requested_tokens = 600;
        value.retrieved_context.requested_tokens = 300;
        let plan = compile_context_window(&profile(true), &value).expect("bounded degradation");
        assert_eq!(
            plan.source_artifacts.disposition,
            ContextPartitionDisposition::Truncated
        );
        assert_eq!(plan.source_artifacts.allocated_tokens, 500);
        assert_eq!(
            plan.retrieved_context.disposition,
            ContextPartitionDisposition::Omitted
        );
        assert_eq!(
            plan.retrieved_context.reason_code,
            Some("context.retrieval.omitted_for_window")
        );
    }

    #[test]
    fn story_13_4_missing_counter_or_authoritative_minimum_fails_closed() {
        let mut wrong = demand();
        wrong.counter.tokenizer_sha256 = hash('0');
        assert_eq!(
            compile_context_window(&profile(true), &wrong),
            Err(ModelOrchestrationError::TokenCounterMismatch)
        );
        let mut oversized = demand();
        oversized.source_artifacts.minimum_tokens = 800;
        oversized.source_artifacts.requested_tokens = 800;
        assert_eq!(
            compile_context_window(&profile(true), &oversized),
            Err(ModelOrchestrationError::ContextOvercommit)
        );
    }

    #[test]
    fn story_13_4_orchestration_changes_shape_but_not_safety_controls() {
        let model = profile(true);
        let context = compile_context_window(&model, &demand()).expect("context");
        let first = compile_orchestration_profile(
            "orchestration-1".to_owned(),
            &model,
            context.clone(),
            shape(),
            controls(),
            qualification(),
        )
        .expect("first profile");
        let mut compact = shape();
        compact.plan_horizon = 1;
        compact.visible_tool_ids.clear();
        let second = compile_orchestration_profile(
            "orchestration-2".to_owned(),
            &model,
            context,
            compact,
            controls(),
            qualification(),
        )
        .expect("second profile");
        assert_ne!(first.profile_sha256, second.profile_sha256);
        assert_eq!(first.invariant_controls, second.invariant_controls);
        assert_eq!(
            verify_model_invariant_controls(&first.invariant_controls, &second),
            Ok(())
        );
        let mut drifted = first.invariant_controls.clone();
        drifted.completion_policy_sha256 = hash('a');
        assert_eq!(
            verify_model_invariant_controls(&drifted, &second),
            Err(ModelOrchestrationError::SafetyControlDrift)
        );
    }

    #[test]
    fn story_13_4_quality_evidence_and_model_admission_are_both_required() {
        let model = profile(false);
        let context = compile_context_window(&model, &demand()).expect("context");
        let disabled = compile_orchestration_profile(
            "orchestration-disabled".to_owned(),
            &model,
            context.clone(),
            shape(),
            controls(),
            qualification(),
        )
        .expect("truthful disabled profile");
        assert!(!disabled.enabled);
        assert!(!disabled.automatic_fallback);
        let mut failed = qualification();
        failed[0].passed = false;
        assert_eq!(
            compile_orchestration_profile(
                "orchestration-failed".to_owned(),
                &model,
                context,
                shape(),
                controls(),
                failed
            ),
            Err(ModelOrchestrationError::QualificationMissing)
        );
    }

    #[test]
    fn story_13_4_every_profile_and_allocation_mutation_changes_identity_or_refuses() {
        let model = profile(true);
        let original_context = compile_context_window(&model, &demand()).expect("context");
        for changed in {
            let mut values = Vec::new();
            let mut value = demand();
            value.system_and_tool_tokens += 1;
            values.push(value);
            let mut value = demand();
            value.user_input_tokens += 1;
            values.push(value);
            let mut value = demand();
            value.source_artifacts.requested_tokens += 1;
            values.push(value);
            let mut value = demand();
            value.source_artifacts.minimum_tokens += 1;
            values.push(value);
            let mut value = demand();
            value.retrieved_context.requested_tokens += 1;
            values.push(value);
            let mut value = demand();
            value.retrieved_context.minimum_tokens += 1;
            values.push(value);
            let mut value = demand();
            value.workflow_recovery_reserve_tokens += 1;
            values.push(value);
            let mut value = demand();
            value.safety_margin_tokens += 1;
            values.push(value);
            values
        } {
            let changed = compile_context_window(&model, &changed).expect("changed allocation");
            assert_ne!(original_context.plan_sha256, changed.plan_sha256);
        }
        let original = compile_orchestration_profile(
            "orchestration-original".to_owned(),
            &model,
            original_context.clone(),
            shape(),
            controls(),
            qualification(),
        )
        .expect("original");
        let mut shapes = Vec::new();
        let mut changed = shape();
        changed.plan_horizon += 1;
        shapes.push(changed);
        let mut changed = shape();
        changed.visible_tool_ids.push("tool.hash".to_owned());
        shapes.push(changed);
        let mut changed = shape();
        changed.max_observation_tokens += 1;
        shapes.push(changed);
        let mut changed = shape();
        changed.parser_repair = ParserRepairAllowance::None;
        shapes.push(changed);
        let mut changed = shape();
        changed.recovery_scaffolding = RecoveryScaffolding::Minimal;
        shapes.push(changed);
        let mut changed = shape();
        changed.diagnostic_verbosity = DiagnosticVerbosity::CodesOnly;
        shapes.push(changed);
        for changed_shape in shapes {
            let changed = compile_orchestration_profile(
                "orchestration-original".to_owned(),
                &model,
                original_context.clone(),
                changed_shape,
                controls(),
                qualification(),
            )
            .expect("changed shape");
            assert_ne!(original.profile_sha256, changed.profile_sha256);
        }
        let mut changed_qualification = qualification();
        changed_qualification[0].evidence_sha256 = hash('a');
        let changed = compile_orchestration_profile(
            "orchestration-original".to_owned(),
            &model,
            original_context,
            shape(),
            controls(),
            changed_qualification,
        )
        .expect("changed qualification");
        assert_ne!(original.profile_sha256, changed.profile_sha256);
        let mut wrong_context = demand();
        wrong_context.output_reserve_tokens += 1;
        assert_eq!(
            compile_context_window(&model, &wrong_context),
            Err(ModelOrchestrationError::InvalidInput)
        );
    }
}
