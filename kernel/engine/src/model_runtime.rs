//! Candidate-neutral exact-profile admission and local runtime orchestration.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{
    CorrelationId, ExactModelProfile, LocalModelRuntime, ModelCancellationProbe,
    ModelCapabilityState, ModelContextPacket, ModelFamilyCodec, ModelHealth, ModelHealthState,
    ModelLifecycleState, ModelLoadReceipt, ModelManifestObservation, ModelModality, ModelProfileId,
    ModelResourceReport, ModelRunRequest, ModelRunResult, ModelRunTerminalState,
    ModelRuntimeFailure, ModelRuntimeIdentity, ModelStreamSink, ModelUnloadReceipt,
    StreamedModelFragment, TaskId, TokenCountResult,
};
use sha2::{Digest, Sha256};

const MAX_TEXT_BYTES: usize = 256;
const MAX_LINEAGE: usize = 32;
const MAX_TRANSFORMATIONS: usize = 32;
const MAX_END_TOKENS: usize = 32;
const MAX_SAMPLERS: usize = 16;
const MAX_HARDWARE_ENVELOPES: usize = 16;
const MAX_LIMITATIONS: usize = 64;
const MAX_STREAM_FRAGMENTS: u32 = 65_536;
const MAX_STREAM_BYTES: usize = 32 * 1024 * 1024;

/// Purpose for which an exact profile may be loaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelUsePurpose {
    /// Deterministic fake contract testing.
    ContractTest,
    /// Isolated candidate evaluation with no product activation.
    Evaluation,
    /// User-selectable product inference after complete admission.
    Product,
}

/// Fail-closed exact-profile or runtime orchestration error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelRuntimeGateError {
    /// Exact profile fields do not satisfy the closed contract.
    ProfileInvalid,
    /// Exact profile is absent from the immutable catalog.
    ProfileNotRegistered,
    /// Candidate differs from its immutable registered tuple.
    ProfileChanged,
    /// Lifecycle state prohibits the requested purpose.
    ProfileUnavailable,
    /// Profile attempts to permit automatic fallback.
    AutomaticFallbackProhibited,
    /// Adapter identity differs from the exact profile runtime.
    RuntimeMismatch,
    /// Runtime manifest observation differs from the exact tuple.
    ManifestObservationMismatch,
    /// Runtime observed network, workspace, authority, or credential material.
    IsolationViolation,
    /// A profile is already loaded.
    AlreadyLoaded,
    /// No matching profile is loaded.
    NotLoaded,
    /// Context packet or run request does not match the selected tuple.
    RequestMismatch,
    /// Token accounting does not match the packet and profile.
    TokenCountMismatch,
    /// Stream fragments are malformed, noncontiguous, oversized, or stale.
    StreamInvalid,
    /// Terminal result does not match the exact run or stream.
    ResultMismatch,
    /// Runtime returned a typed non-success.
    RuntimeFailure,
}

impl ModelRuntimeGateError {
    /// Returns the stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ProfileInvalid => "model.profile.invalid",
            Self::ProfileNotRegistered => "model.profile.not-registered",
            Self::ProfileChanged => "model.profile.changed",
            Self::ProfileUnavailable => "model.profile.unavailable",
            Self::AutomaticFallbackProhibited => "model.profile.automatic-fallback-prohibited",
            Self::RuntimeMismatch => "model.runtime.identity-mismatch",
            Self::ManifestObservationMismatch => "model.runtime.manifest-observation-mismatch",
            Self::IsolationViolation => "model.runtime.isolation-violation",
            Self::AlreadyLoaded => "model.runtime.already-loaded",
            Self::NotLoaded => "model.runtime.not-loaded",
            Self::RequestMismatch => "model.runtime.request-mismatch",
            Self::TokenCountMismatch => "model.runtime.token-count-mismatch",
            Self::StreamInvalid => "model.runtime.stream-invalid",
            Self::ResultMismatch => "model.runtime.result-mismatch",
            Self::RuntimeFailure => "model.runtime.failed",
        }
    }
}

/// Immutable exact-profile catalog used before any adapter operation.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelAdmissionCatalog {
    profiles: Vec<ExactModelProfile>,
}

impl ModelAdmissionCatalog {
    /// Constructs a catalog only when every tuple is valid and identities are unique.
    pub fn new(profiles: Vec<ExactModelProfile>) -> Result<Self, ModelRuntimeGateError> {
        if profiles.is_empty() || profiles.iter().any(|profile| !valid_profile(profile)) {
            return Err(ModelRuntimeGateError::ProfileInvalid);
        }
        let mut identities = BTreeSet::new();
        if profiles.iter().any(|profile| {
            !identities.insert((
                profile.profile_id.as_str().to_owned(),
                profile.manifest_sha256.clone(),
            ))
        }) {
            return Err(ModelRuntimeGateError::ProfileInvalid);
        }
        Ok(Self { profiles })
    }

    /// Resolves one exact tuple for an explicit purpose without fallback.
    pub fn admit(
        &self,
        candidate: &ExactModelProfile,
        purpose: ModelUsePurpose,
    ) -> Result<AdmittedModelProfile, ModelRuntimeGateError> {
        let Some(registered) = self
            .profiles
            .iter()
            .find(|profile| profile.profile_id == candidate.profile_id)
        else {
            return Err(ModelRuntimeGateError::ProfileNotRegistered);
        };
        if registered != candidate {
            return Err(ModelRuntimeGateError::ProfileChanged);
        }
        if candidate.automatic_fallback {
            return Err(ModelRuntimeGateError::AutomaticFallbackProhibited);
        }
        let eligible = match purpose {
            ModelUsePurpose::ContractTest => {
                candidate.runtime.kind
                    == agentmage_kernel_contracts::ModelRuntimeKind::DeterministicFake
                    && candidate.lifecycle != ModelLifecycleState::Rejected
                    && candidate.lifecycle != ModelLifecycleState::Quarantined
            }
            ModelUsePurpose::Evaluation => {
                matches!(
                    candidate.lifecycle,
                    ModelLifecycleState::Candidate | ModelLifecycleState::Evaluating
                ) && !candidate.enabled
            }
            ModelUsePurpose::Product => {
                matches!(
                    candidate.lifecycle,
                    ModelLifecycleState::Approved | ModelLifecycleState::Degraded
                ) && candidate.enabled
            }
        };
        if !eligible {
            return Err(ModelRuntimeGateError::ProfileUnavailable);
        }
        Ok(AdmittedModelProfile {
            profile: candidate.clone(),
            purpose,
        })
    }
}

/// Kernel-owned proof that one complete profile matched the immutable catalog.
#[derive(Clone, Debug, PartialEq)]
pub struct AdmittedModelProfile {
    profile: ExactModelProfile,
    purpose: ModelUsePurpose,
}

impl AdmittedModelProfile {
    /// Returns the exact profile identity.
    #[must_use]
    pub fn profile_id(&self) -> &ModelProfileId {
        &self.profile.profile_id
    }

    /// Returns the exact manifest digest.
    #[must_use]
    pub fn manifest_sha256(&self) -> &str {
        &self.profile.manifest_sha256
    }

    /// Returns the explicitly admitted purpose.
    #[must_use]
    pub const fn purpose(&self) -> ModelUsePurpose {
        self.purpose
    }
}

/// Kernel-owned controller for one exact selected profile and local adapter.
pub struct LocalModelController<R: LocalModelRuntime, C: ModelFamilyCodec> {
    runtime: R,
    codec: C,
    admitted: AdmittedModelProfile,
    loaded: bool,
}

impl<R: LocalModelRuntime, C: ModelFamilyCodec> LocalModelController<R, C> {
    /// Binds one admitted profile to one exact runtime identity.
    pub fn new(
        runtime: R,
        codec: C,
        admitted: AdmittedModelProfile,
    ) -> Result<Self, ModelRuntimeGateError> {
        if runtime.identity() != &admitted.profile.runtime
            || codec.identity() != &admitted.profile.codec
        {
            return Err(ModelRuntimeGateError::RuntimeMismatch);
        }
        Ok(Self {
            runtime,
            codec,
            admitted,
            loaded: false,
        })
    }

    /// Verifies and loads the exact selected tuple.
    pub fn load(&mut self) -> Result<ModelLoadReceipt, ModelRuntimeGateError> {
        if self.loaded {
            return Err(ModelRuntimeGateError::AlreadyLoaded);
        }
        let observation = self
            .runtime
            .verify_manifest(&self.admitted.profile)
            .map_err(|_| ModelRuntimeGateError::RuntimeFailure)?;
        validate_manifest_observation(&self.admitted.profile, &observation)?;
        let receipt = self
            .runtime
            .load(&self.admitted.profile)
            .map_err(|_| ModelRuntimeGateError::RuntimeFailure)?;
        validate_load_receipt(&self.admitted.profile, &receipt)?;
        self.loaded = true;
        Ok(receipt)
    }

    /// Returns health only when it remains bound to the exact loaded tuple.
    pub fn health(&self) -> Result<ModelHealth, ModelRuntimeGateError> {
        if !self.loaded {
            return Err(ModelRuntimeGateError::NotLoaded);
        }
        let health = self.runtime.health();
        if health.adapter_id != self.admitted.profile.runtime.adapter_id
            || health.profile_id.as_ref() != Some(&self.admitted.profile.profile_id)
            || health.state != ModelHealthState::Ready
        {
            return Err(ModelRuntimeGateError::RuntimeMismatch);
        }
        Ok(health)
    }

    /// Counts an exact bounded context packet with no fallback or substitution.
    pub fn count_tokens(
        &self,
        packet: &ModelContextPacket,
    ) -> Result<TokenCountResult, ModelRuntimeGateError> {
        self.validate_packet(packet)?;
        let context = self
            .codec
            .encode_context(&self.admitted.profile, packet)
            .map_err(|_| ModelRuntimeGateError::RequestMismatch)?;
        let result = self
            .runtime
            .count_tokens(&context)
            .map_err(|_| ModelRuntimeGateError::RuntimeFailure)?;
        if result.profile_id != self.admitted.profile.profile_id
            || result.context_packet_id != packet.context_packet_id
            || result.packet_sha256 != context.sha256
            || result.counter != self.admitted.profile.context.token_counter
            || result.tokens != packet.input_tokens
        {
            return Err(ModelRuntimeGateError::TokenCountMismatch);
        }
        Ok(result)
    }

    /// Runs one exact bounded request and validates its complete inert stream.
    pub fn stream(
        &mut self,
        request: &ModelRunRequest,
        packet: &ModelContextPacket,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<ModelRunResult, ModelRuntimeGateError> {
        self.validate_packet(packet)?;
        if request.profile_id != self.admitted.profile.profile_id
            || request.manifest_sha256 != self.admitted.profile.manifest_sha256
            || request.adapter_id != self.admitted.profile.runtime.adapter_id
            || request.context_packet_id != packet.context_packet_id
            || request.decoding_profile_id != self.admitted.profile.decoding.profile_id
            || request.max_output_tokens == 0
            || request.max_output_tokens > self.admitted.profile.decoding.max_output_tokens
            || request.timeout_ms == 0
        {
            return Err(ModelRuntimeGateError::RequestMismatch);
        }
        let bound_cancellation = cancellation.map(|source| BoundCancellationProbe {
            source,
            task_id: &packet.task_id,
            correlation_id: &request.correlation_id,
        });
        let context = self
            .codec
            .encode_context(&self.admitted.profile, packet)
            .map_err(|_| ModelRuntimeGateError::RequestMismatch)?;
        let mut capture = StreamCapture::new(request);
        let mut result = self
            .runtime
            .stream(
                request,
                &context,
                bound_cancellation
                    .as_ref()
                    .map(|probe| probe as &dyn ModelCancellationProbe),
                &mut capture,
            )
            .map_err(|_| ModelRuntimeGateError::RuntimeFailure)?;
        capture.finish(&result)?;
        if result.response_sha256 != sha256_hex(&capture.bytes) {
            return Err(ModelRuntimeGateError::ResultMismatch);
        }
        if result.terminal_state == ModelRunTerminalState::Proposed {
            let decoded = self
                .codec
                .decode_proposal(&self.admitted.profile, request, &capture.bytes)
                .map_err(|_| ModelRuntimeGateError::ResultMismatch)?;
            if result.proposal.is_some() {
                return Err(ModelRuntimeGateError::ResultMismatch);
            }
            result.proposal = Some(decoded);
        } else if result.terminal_state == ModelRunTerminalState::AdvisoryText
            && !crate::model_response::plain_text_advisory(&capture.bytes)
        {
            return Err(ModelRuntimeGateError::ResultMismatch);
        }
        validate_result(&self.admitted.profile, request, &result)?;
        Ok(result)
    }

    /// Returns resource accounting only for the exact loaded tuple.
    pub fn resources(&self) -> Result<ModelResourceReport, ModelRuntimeGateError> {
        if !self.loaded {
            return Err(ModelRuntimeGateError::NotLoaded);
        }
        let report = self
            .runtime
            .resources()
            .map_err(|_| ModelRuntimeGateError::RuntimeFailure)?;
        if report.adapter_id != self.admitted.profile.runtime.adapter_id
            || report.profile_id != self.admitted.profile.profile_id
        {
            return Err(ModelRuntimeGateError::RuntimeMismatch);
        }
        Ok(report)
    }

    /// Unloads exactly the selected tuple and leaves no alternate selected profile.
    pub fn unload(&mut self) -> Result<ModelUnloadReceipt, ModelRuntimeGateError> {
        if !self.loaded {
            return Err(ModelRuntimeGateError::NotLoaded);
        }
        let receipt = self
            .runtime
            .unload(&self.admitted.profile.profile_id)
            .map_err(|_| ModelRuntimeGateError::RuntimeFailure)?;
        if receipt.profile_id != self.admitted.profile.profile_id
            || receipt.adapter_id != self.admitted.profile.runtime.adapter_id
            || !receipt.empty
        {
            return Err(ModelRuntimeGateError::ResultMismatch);
        }
        self.loaded = false;
        Ok(receipt)
    }

    fn validate_packet(&self, packet: &ModelContextPacket) -> Result<(), ModelRuntimeGateError> {
        if !self.loaded {
            return Err(ModelRuntimeGateError::NotLoaded);
        }
        if packet.profile_id != self.admitted.profile.profile_id
            || packet.manifest_sha256 != self.admitted.profile.manifest_sha256
            || packet.messages.is_empty()
            || packet.messages.len() > self.admitted.profile.context.max_messages as usize
            || packet.input_bytes == 0
            || packet.input_bytes > self.admitted.profile.context.max_input_bytes
            || packet.input_tokens == 0
            || packet.input_tokens > self.admitted.profile.context.max_context_tokens
            || !valid_sha256(&packet.packet_sha256)
        {
            return Err(ModelRuntimeGateError::RequestMismatch);
        }
        Ok(())
    }
}

struct BoundCancellationProbe<'a> {
    source: &'a dyn ModelCancellationProbe,
    task_id: &'a TaskId,
    correlation_id: &'a CorrelationId,
}

impl ModelCancellationProbe for BoundCancellationProbe<'_> {
    fn observe(
        &self,
    ) -> Result<Option<agentmage_kernel_contracts::CancellationSignal>, ModelRuntimeFailure> {
        let signal = self.source.observe()?;
        if signal.as_ref().is_some_and(|signal| {
            signal.task_id != *self.task_id || signal.correlation_id != *self.correlation_id
        }) {
            return Err(runtime_failure("model.cancellation.identity-mismatch"));
        }
        Ok(signal)
    }
}

struct StreamCapture<'a> {
    request: &'a ModelRunRequest,
    stream_id: Option<String>,
    next_sequence: u32,
    total_bytes: usize,
    terminal_seen: bool,
    bytes: Vec<u8>,
}

impl<'a> StreamCapture<'a> {
    fn new(request: &'a ModelRunRequest) -> Self {
        Self {
            request,
            stream_id: None,
            next_sequence: 0,
            total_bytes: 0,
            terminal_seen: false,
            bytes: Vec::new(),
        }
    }

    fn finish(&self, result: &ModelRunResult) -> Result<(), ModelRuntimeGateError> {
        if self.next_sequence == 0
            || !self.terminal_seen
            || result.fragment_count != self.next_sequence
            || self.stream_id.as_deref() != Some(result.stream_id.as_str())
        {
            return Err(ModelRuntimeGateError::StreamInvalid);
        }
        Ok(())
    }
}

impl ModelStreamSink for StreamCapture<'_> {
    fn accept(&mut self, fragment: StreamedModelFragment) -> Result<(), ModelRuntimeFailure> {
        let invalid = self.terminal_seen
            || fragment.model_run_id != self.request.model_run_id
            || fragment.correlation_id != self.request.correlation_id
            || fragment.sequence != self.next_sequence
            || (fragment.bytes.is_empty() && !fragment.terminal)
            || !valid_sha256(&fragment.sha256)
            || sha256_hex(&fragment.bytes) != fragment.sha256
            || self.next_sequence >= MAX_STREAM_FRAGMENTS
            || self.total_bytes.saturating_add(fragment.bytes.len()) > MAX_STREAM_BYTES
            || self
                .stream_id
                .as_deref()
                .is_some_and(|identity| identity != fragment.stream_id.as_str());
        if invalid {
            return Err(runtime_failure("model.stream.fragment-invalid"));
        }
        if self.stream_id.is_none() {
            self.stream_id = Some(fragment.stream_id.as_str().to_owned());
        }
        self.next_sequence += 1;
        self.total_bytes += fragment.bytes.len();
        self.bytes.extend_from_slice(&fragment.bytes);
        self.terminal_seen = fragment.terminal;
        Ok(())
    }
}

fn validate_manifest_observation(
    profile: &ExactModelProfile,
    observation: &ModelManifestObservation,
) -> Result<(), ModelRuntimeGateError> {
    if observation.profile_id != profile.profile_id
        || observation.manifest_sha256 != profile.manifest_sha256
        || observation.artifact_sha256 != profile.artifact.sha256
        || observation.tokenizer_sha256 != profile.codec.tokenizer_sha256
        || observation.template_sha256 != profile.codec.template_sha256
        || observation.codec_sha256 != profile.codec.codec_sha256
        || observation.runtime != profile.runtime
    {
        return Err(ModelRuntimeGateError::ManifestObservationMismatch);
    }
    Ok(())
}

fn validate_load_receipt(
    profile: &ExactModelProfile,
    receipt: &ModelLoadReceipt,
) -> Result<(), ModelRuntimeGateError> {
    if receipt.profile_id != profile.profile_id
        || receipt.manifest_sha256 != profile.manifest_sha256
        || receipt.adapter_id != profile.runtime.adapter_id
    {
        return Err(ModelRuntimeGateError::ResultMismatch);
    }
    let isolation = &receipt.isolation;
    if isolation.adapter_id != profile.runtime.adapter_id
        || isolation.profile_id != profile.profile_id
        || !valid_sha256(&isolation.observation_sha256)
    {
        return Err(ModelRuntimeGateError::ResultMismatch);
    }
    if isolation.network_available
        || isolation.workspace_available
        || isolation.authority_material_available
        || isolation.credential_material_available
    {
        return Err(ModelRuntimeGateError::IsolationViolation);
    }
    Ok(())
}

fn validate_result(
    profile: &ExactModelProfile,
    request: &ModelRunRequest,
    result: &ModelRunResult,
) -> Result<(), ModelRuntimeGateError> {
    if result.model_run_id != request.model_run_id
        || result.correlation_id != request.correlation_id
        || !valid_sha256(&result.response_sha256)
        || result.resources.adapter_id != profile.runtime.adapter_id
        || result.resources.profile_id != profile.profile_id
        || result.resources.model_run_id.as_ref() != Some(&request.model_run_id)
        || result.resources.output_tokens > request.max_output_tokens
    {
        return Err(ModelRuntimeGateError::ResultMismatch);
    }
    match result.terminal_state {
        ModelRunTerminalState::Proposed => {
            let Some(proposal) = &result.proposal else {
                return Err(ModelRuntimeGateError::ResultMismatch);
            };
            if result.failure.is_some()
                || proposal.model_run_id != request.model_run_id
                || proposal.context_packet_id != request.context_packet_id
                || proposal.profile_id != profile.profile_id
                || proposal.codec_id != profile.codec.codec_id
                || proposal.correlation_id != request.correlation_id
                || !valid_sha256(&proposal.proposal_sha256)
            {
                return Err(ModelRuntimeGateError::ResultMismatch);
            }
        }
        ModelRunTerminalState::AdvisoryText => {
            if result.proposal.is_some() || result.failure.is_some() {
                return Err(ModelRuntimeGateError::ResultMismatch);
            }
        }
        _ => {
            if result.proposal.is_some() || result.failure.is_none() {
                return Err(ModelRuntimeGateError::ResultMismatch);
            }
        }
    }
    Ok(())
}

fn valid_profile(profile: &ExactModelProfile) -> bool {
    if profile.schema_version != agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION
        || !valid_text(profile.profile_id.as_str())
        || !valid_text(profile.manifest_id.as_str())
        || !valid_sha256(&profile.manifest_sha256)
        || !valid_text(&profile.display_name)
        || !valid_text(&profile.family)
        || !valid_text(&profile.publisher_control)
        || profile.lineage.is_empty()
        || profile.lineage.len() > MAX_LINEAGE
        || profile.lineage.iter().any(|item| !valid_text(item))
        || !valid_text(&profile.license_spdx)
        || !valid_sha256(&profile.license_terms_sha256)
        || !valid_text(&profile.artifact.artifact_id)
        || !valid_text(&profile.artifact.publisher)
        || !valid_text(&profile.artifact.source_revision)
        || !valid_text(&profile.artifact.format)
        || profile.artifact.bytes == 0
        || !valid_sha256(&profile.artifact.sha256)
        || profile.transformations.len() > MAX_TRANSFORMATIONS
        || !valid_codec(profile)
        || !valid_runtime(&profile.runtime)
        || !valid_text(&profile.quantization)
        || profile.modalities.is_empty()
        || !unique(&profile.modalities)
        || !profile.modalities.contains(&ModelModality::Text)
        || !valid_context(profile)
        || !valid_decoding(profile)
        || profile.hardware.is_empty()
        || profile.hardware.len() > MAX_HARDWARE_ENVELOPES
        || !profile.hardware.iter().any(|item| {
            item.platform == profile.runtime.platform
                && item.architecture == profile.runtime.architecture
        })
        || !valid_capabilities(profile)
        || !valid_sha256(&profile.policy_sha256)
        || profile.automatic_fallback
    {
        return false;
    }
    let mut previous_output: Option<&str> = None;
    for transformation in &profile.transformations {
        if !valid_text(&transformation.transformation_id)
            || !valid_text(&transformation.tool)
            || !valid_sha256(&transformation.arguments_sha256)
            || !valid_sha256(&transformation.input_sha256)
            || !valid_sha256(&transformation.output_sha256)
            || previous_output.is_some_and(|output| output != transformation.input_sha256)
        {
            return false;
        }
        previous_output = Some(&transformation.output_sha256);
    }
    previous_output.is_none_or(|output| output == profile.artifact.sha256)
}

fn valid_codec(profile: &ExactModelProfile) -> bool {
    let codec = &profile.codec;
    valid_text(codec.codec_id.as_str())
        && valid_text(&codec.codec_version)
        && valid_sha256(&codec.codec_sha256)
        && valid_text(&codec.tokenizer)
        && valid_sha256(&codec.tokenizer_sha256)
        && valid_text(&codec.template)
        && valid_sha256(&codec.template_sha256)
        && valid_text(&codec.tool_protocol_version)
        && !codec.end_tokens.is_empty()
        && codec.end_tokens.len() <= MAX_END_TOKENS
        && unique(&codec.end_tokens)
}

fn valid_runtime(runtime: &ModelRuntimeIdentity) -> bool {
    valid_text(runtime.adapter_id.as_str())
        && runtime.contract_version == 1
        && valid_text(&runtime.runtime_build)
        && valid_sha256(&runtime.runtime_sha256)
}

fn valid_context(profile: &ExactModelProfile) -> bool {
    let context = &profile.context;
    context.max_context_tokens > 0
        && context.max_input_bytes > 0
        && context.max_messages > 0
        && valid_text(&context.token_counter)
        && valid_sha256(&context.token_counter_sha256)
}

fn valid_decoding(profile: &ExactModelProfile) -> bool {
    let decoding = &profile.decoding;
    valid_text(&decoding.profile_id)
        && !decoding.sampler_order.is_empty()
        && decoding.sampler_order.len() <= MAX_SAMPLERS
        && decoding.sampler_order.iter().all(|item| valid_text(item))
        && unique(&decoding.sampler_order)
        && decoding.temperature.is_finite()
        && decoding.temperature >= 0.0
        && decoding.top_p.is_finite()
        && (0.0..=1.0).contains(&decoding.top_p)
        && decoding.top_k > 0
        && decoding.repeat_penalty.is_finite()
        && decoding.repeat_penalty > 0.0
        && decoding.max_output_tokens > 0
}

fn valid_capabilities(profile: &ExactModelProfile) -> bool {
    if profile.capabilities.is_empty() {
        return false;
    }
    let mut roles = BTreeSet::new();
    profile.capabilities.iter().all(|capability| {
        roles.insert(capability.role)
            && capability.limitations.len() <= MAX_LIMITATIONS
            && capability.limitations.iter().all(|item| valid_text(item))
            && match capability.state {
                ModelCapabilityState::Passed | ModelCapabilityState::Failed => {
                    capability
                        .evaluation_profile
                        .as_deref()
                        .is_some_and(valid_text)
                        && capability
                            .result_sha256
                            .as_deref()
                            .is_some_and(valid_sha256)
                }
                ModelCapabilityState::NotEvaluated
                | ModelCapabilityState::Blocked
                | ModelCapabilityState::NotApplicable => {
                    capability.evaluation_profile.is_none() && capability.result_sha256.is_none()
                }
            }
    })
}

fn valid_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_TEXT_BYTES && !value.chars().any(char::is_control)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn unique<T: Ord + Clone>(values: &[T]) -> bool {
    values.iter().cloned().collect::<BTreeSet<_>>().len() == values.len()
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn runtime_failure(code: &str) -> ModelRuntimeFailure {
    ModelRuntimeFailure {
        code: code.to_owned(),
        retryable_after_correction: false,
        dependency_recovery_required: false,
        contract_error: None,
    }
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        BoundaryKind, CONTRACT_SCHEMA_VERSION, CancellationId, CancellationReason,
        CancellationSignal, ContextBudget, ContextPacketId, CorrelationId, DecodingProfile,
        EncodedModelContext, ExactModelProfile, FamilyCodecIdentity, HardwareEnvelope,
        LocalModelRuntime, ModelAdapterId, ModelArtifact, ModelCancellationProbe, ModelCapability,
        ModelCapabilityState, ModelCodecId, ModelContextPacket, ModelHealth, ModelHealthState,
        ModelLifecycleState, ModelLoadReceipt, ModelManifestId, ModelManifestObservation,
        ModelMessage, ModelMessageId, ModelMessageRole, ModelModality, ModelProfileId,
        ModelProposalKind, ModelResourceReport, ModelRole, ModelRunId, ModelRunRequest,
        ModelRunResult, ModelRunTerminalState, ModelRuntimeFailure, ModelRuntimeIdentity,
        ModelRuntimeKind, ModelStreamId, ModelStreamSink, ModelTransformation, ModelUnloadReceipt,
        PlatformArchitecture, PlatformFamily, RuntimeIsolationObservation, SessionId,
        StreamedModelFragment, TaskId, TokenCountResult, ToolCatalogId,
    };

    use super::{
        LocalModelController, ModelAdmissionCatalog, ModelRuntimeGateError, ModelUsePurpose,
        runtime_failure, sha256_hex,
    };
    use crate::model_codec::ClosedJsonFamilyCodec;

    const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum FakeScenario {
        Happy,
        Malformed,
        Delayed,
        Cancelled,
        Crashed,
        ResourceExhausted,
        Replay,
        FalseCompletion,
    }

    #[derive(Clone)]
    struct FakeRuntime {
        identity: ModelRuntimeIdentity,
        token_counter: String,
        loaded: Option<ModelProfileId>,
        manifest_drift: bool,
        isolation_drift: bool,
        scenario: FakeScenario,
    }

    impl FakeRuntime {
        fn new(profile: &ExactModelProfile) -> Self {
            Self {
                identity: profile.runtime.clone(),
                token_counter: profile.context.token_counter.clone(),
                loaded: None,
                manifest_drift: false,
                isolation_drift: false,
                scenario: FakeScenario::Happy,
            }
        }
    }

    impl LocalModelRuntime for FakeRuntime {
        fn identity(&self) -> &ModelRuntimeIdentity {
            &self.identity
        }

        fn verify_manifest(
            &self,
            profile: &ExactModelProfile,
        ) -> Result<ModelManifestObservation, ModelRuntimeFailure> {
            Ok(ModelManifestObservation {
                profile_id: profile.profile_id.clone(),
                manifest_sha256: if self.manifest_drift {
                    "b".repeat(64)
                } else {
                    profile.manifest_sha256.clone()
                },
                artifact_sha256: profile.artifact.sha256.clone(),
                tokenizer_sha256: profile.codec.tokenizer_sha256.clone(),
                template_sha256: profile.codec.template_sha256.clone(),
                codec_sha256: profile.codec.codec_sha256.clone(),
                runtime: self.identity.clone(),
            })
        }

        fn load(
            &mut self,
            profile: &ExactModelProfile,
        ) -> Result<ModelLoadReceipt, ModelRuntimeFailure> {
            self.loaded = Some(profile.profile_id.clone());
            Ok(ModelLoadReceipt {
                profile_id: profile.profile_id.clone(),
                manifest_sha256: profile.manifest_sha256.clone(),
                adapter_id: self.identity.adapter_id.clone(),
                elapsed_ms: 1,
                isolation: RuntimeIsolationObservation {
                    adapter_id: self.identity.adapter_id.clone(),
                    profile_id: profile.profile_id.clone(),
                    network_available: self.isolation_drift,
                    workspace_available: false,
                    authority_material_available: false,
                    credential_material_available: false,
                    observation_sha256: SHA.to_owned(),
                },
            })
        }

        fn unload(
            &mut self,
            profile_id: &ModelProfileId,
        ) -> Result<ModelUnloadReceipt, ModelRuntimeFailure> {
            self.loaded = None;
            Ok(ModelUnloadReceipt {
                profile_id: profile_id.clone(),
                adapter_id: self.identity.adapter_id.clone(),
                empty: true,
                elapsed_ms: 1,
            })
        }

        fn health(&self) -> ModelHealth {
            ModelHealth {
                adapter_id: self.identity.adapter_id.clone(),
                profile_id: self.loaded.clone(),
                state: if self.loaded.is_some() {
                    ModelHealthState::Ready
                } else {
                    ModelHealthState::Unloaded
                },
                reason_code: "fixture.ready".to_owned(),
                observed_at_ms: 1,
            }
        }

        fn count_tokens(
            &self,
            context: &EncodedModelContext,
        ) -> Result<TokenCountResult, ModelRuntimeFailure> {
            Ok(TokenCountResult {
                profile_id: context.profile_id.clone(),
                context_packet_id: context.context_packet_id.clone(),
                tokens: 1,
                counter: self.token_counter.clone(),
                packet_sha256: context.sha256.clone(),
            })
        }

        fn stream(
            &mut self,
            request: &ModelRunRequest,
            _context: &EncodedModelContext,
            cancellation: Option<&dyn ModelCancellationProbe>,
            sink: &mut dyn ModelStreamSink,
        ) -> Result<ModelRunResult, ModelRuntimeFailure> {
            if self.scenario == FakeScenario::Crashed {
                return Err(runtime_failure("fixture.runtime-crashed"));
            }
            let cancellation = cancellation
                .map(ModelCancellationProbe::observe)
                .transpose()?;
            if self.scenario == FakeScenario::Cancelled
                && cancellation.as_ref().is_none_or(Option::is_none)
            {
                return Err(runtime_failure("fixture.cancellation-missing"));
            }
            let candidate = agentmage_kernel_contracts::ModelProposalWireCandidate {
                schema_version: CONTRACT_SCHEMA_VERSION,
                kind: ModelProposalKind::CompletionCandidate,
                payload: None,
                tool_call: None,
            };
            let bytes = if self.scenario == FakeScenario::FalseCompletion {
                b"{\"schema_version\":2,\"kind\":\"completion_candidate\"}".to_vec()
            } else {
                agentmage_kernel_contracts::to_canonical_json(&candidate)
                    .expect("fixture candidate bytes")
            };
            let response_sha256 = sha256_hex(&bytes);
            let stream_id = ModelStreamId::from_raw("stream-1");
            sink.accept(StreamedModelFragment {
                schema_version: CONTRACT_SCHEMA_VERSION,
                stream_id: stream_id.clone(),
                model_run_id: request.model_run_id.clone(),
                correlation_id: request.correlation_id.clone(),
                sequence: if self.scenario == FakeScenario::Replay {
                    1
                } else {
                    0
                },
                sha256: if self.scenario == FakeScenario::Malformed {
                    "invalid".to_owned()
                } else {
                    sha256_hex(&bytes)
                },
                bytes,
                terminal: true,
            })?;
            let (terminal_state, proposal, failure) = match self.scenario {
                FakeScenario::Happy => (ModelRunTerminalState::Proposed, None, None),
                FakeScenario::FalseCompletion => (ModelRunTerminalState::Proposed, None, None),
                FakeScenario::Delayed => (
                    ModelRunTerminalState::TimedOut,
                    None,
                    Some(runtime_failure("fixture.timed-out")),
                ),
                FakeScenario::Cancelled => (
                    ModelRunTerminalState::Cancelled,
                    None,
                    Some(runtime_failure("fixture.cancelled")),
                ),
                FakeScenario::ResourceExhausted => (
                    ModelRunTerminalState::ResourceExhausted,
                    None,
                    Some(runtime_failure("fixture.resource-exhausted")),
                ),
                FakeScenario::Malformed | FakeScenario::Crashed | FakeScenario::Replay => {
                    unreachable!("scenario returns before result construction")
                }
            };
            Ok(ModelRunResult {
                schema_version: CONTRACT_SCHEMA_VERSION,
                model_run_id: request.model_run_id.clone(),
                stream_id,
                correlation_id: request.correlation_id.clone(),
                terminal_state,
                fragment_count: 1,
                response_sha256,
                proposal,
                failure,
                resources: ModelResourceReport {
                    adapter_id: self.identity.adapter_id.clone(),
                    profile_id: request.profile_id.clone(),
                    model_run_id: Some(request.model_run_id.clone()),
                    resident_memory_bytes: 1,
                    accelerator_memory_bytes: 0,
                    input_tokens: 1,
                    output_tokens: 1,
                    elapsed_ms: 1,
                },
            })
        }

        fn resources(&self) -> Result<ModelResourceReport, ModelRuntimeFailure> {
            Ok(ModelResourceReport {
                adapter_id: self.identity.adapter_id.clone(),
                profile_id: self.loaded.clone().expect("loaded fixture"),
                model_run_id: None,
                resident_memory_bytes: 1,
                accelerator_memory_bytes: 0,
                input_tokens: 0,
                output_tokens: 0,
                elapsed_ms: 1,
            })
        }
    }

    fn runtime_identity() -> ModelRuntimeIdentity {
        ModelRuntimeIdentity {
            adapter_id: ModelAdapterId::from_raw("fixture-adapter"),
            kind: ModelRuntimeKind::DeterministicFake,
            contract_version: 1,
            runtime_build: "fixture-runtime-v1".to_owned(),
            runtime_sha256: SHA.to_owned(),
            platform: PlatformFamily::DeterministicFake,
            architecture: PlatformArchitecture::X86_64,
        }
    }

    fn profile() -> ExactModelProfile {
        ExactModelProfile {
            schema_version: CONTRACT_SCHEMA_VERSION,
            profile_id: ModelProfileId::from_raw("fixture-profile"),
            manifest_id: ModelManifestId::from_raw("fixture-manifest"),
            manifest_sha256: SHA.to_owned(),
            display_name: "Fixture profile".to_owned(),
            family: "fixture".to_owned(),
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
            transformations: vec![ModelTransformation {
                transformation_id: "fixture-transform".to_owned(),
                tool: "fixture-tool-v1".to_owned(),
                arguments_sha256: SHA.to_owned(),
                input_sha256: "b".repeat(64),
                output_sha256: SHA.to_owned(),
                reproducible: true,
            }],
            codec: FamilyCodecIdentity {
                codec_id: ModelCodecId::from_raw("fixture-codec"),
                codec_version: "1.0.0".to_owned(),
                codec_sha256: SHA.to_owned(),
                tokenizer: "fixture-tokenizer".to_owned(),
                tokenizer_sha256: SHA.to_owned(),
                template: "fixture-template".to_owned(),
                template_sha256: SHA.to_owned(),
                tool_protocol_version: "fixture-tool-v1".to_owned(),
                end_tokens: vec![1],
                reasoning_enabled: false,
            },
            runtime: runtime_identity(),
            quantization: "fixture-q".to_owned(),
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
                sampler_order: vec!["temperature".to_owned()],
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
            capabilities: vec![ModelCapability {
                role: ModelRole::CodingPlanner,
                state: ModelCapabilityState::NotEvaluated,
                evaluation_profile: None,
                result_sha256: None,
                limitations: vec!["fixture-only".to_owned()],
            }],
            policy_sha256: SHA.to_owned(),
            lifecycle: ModelLifecycleState::Candidate,
            enabled: false,
            automatic_fallback: false,
        }
    }

    fn family_profile(family: &str) -> ExactModelProfile {
        let mut value = profile();
        value.profile_id = ModelProfileId::from_raw(format!("fixture-{family}-profile"));
        value.manifest_id = ModelManifestId::from_raw(format!("fixture-{family}-manifest"));
        value.display_name = format!("Fixture {family} profile");
        value.family = format!("deterministic_fake_{family}");
        value.codec.codec_id = ModelCodecId::from_raw(format!("fixture-{family}-codec"));
        value.codec.tokenizer = format!("fixture-{family}-tokenizer");
        value.codec.template = format!("fixture-{family}-template");
        value.runtime.adapter_id = ModelAdapterId::from_raw(format!("fixture-{family}-adapter"));
        value
    }

    fn packet(profile: &ExactModelProfile) -> ModelContextPacket {
        ModelContextPacket {
            schema_version: CONTRACT_SCHEMA_VERSION,
            context_packet_id: ContextPacketId::from_raw("context-1"),
            session_id: SessionId::from_raw("session-1"),
            task_id: TaskId::from_raw("task-1"),
            profile_id: profile.profile_id.clone(),
            manifest_sha256: profile.manifest_sha256.clone(),
            tool_catalog_id: ToolCatalogId::from_raw("tools-1"),
            messages: vec![ModelMessage {
                message_id: ModelMessageId::from_raw("message-1"),
                role: ModelMessageRole::User,
                content: agentmage_kernel_contracts::ContractPayload {
                    schema: agentmage_kernel_contracts::SchemaReference {
                        schema_id: agentmage_kernel_contracts::SchemaId::from_raw("text-v1"),
                        schema_version: 1,
                        schema_sha256: SHA.to_owned(),
                    },
                    media_type: "text/plain".to_owned(),
                    bytes: b"fixture".to_vec(),
                    sha256: SHA.to_owned(),
                },
            }],
            input_bytes: 7,
            input_tokens: 1,
            packet_sha256: SHA.to_owned(),
        }
    }

    fn request(profile: &ExactModelProfile) -> ModelRunRequest {
        ModelRunRequest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            model_run_id: ModelRunId::from_raw("run-1"),
            correlation_id: CorrelationId::from_raw("correlation-1"),
            context_packet_id: ContextPacketId::from_raw("context-1"),
            profile_id: profile.profile_id.clone(),
            manifest_sha256: profile.manifest_sha256.clone(),
            adapter_id: profile.runtime.adapter_id.clone(),
            decoding_profile_id: profile.decoding.profile_id.clone(),
            max_output_tokens: 16,
            timeout_ms: 100,
        }
    }

    fn cancellation() -> CancellationSignal {
        CancellationSignal {
            schema_version: CONTRACT_SCHEMA_VERSION,
            cancellation_id: CancellationId::from_raw("cancel-1"),
            correlation_id: CorrelationId::from_raw("correlation-1"),
            task_id: TaskId::from_raw("task-1"),
            reason: CancellationReason::UserRequested,
            requested_by: BoundaryKind::Shell,
        }
    }

    #[test]
    fn exact_fake_profile_runs_full_contract_without_authority_or_fallback() {
        let profile = profile();
        let catalog = ModelAdmissionCatalog::new(vec![profile.clone()]).expect("catalog");
        let admitted = catalog
            .admit(&profile, ModelUsePurpose::ContractTest)
            .expect("admitted");
        let mut controller = LocalModelController::new(
            FakeRuntime::new(&profile),
            ClosedJsonFamilyCodec::new(profile.codec.clone()),
            admitted,
        )
        .expect("controller");

        controller.load().expect("load");
        assert_eq!(
            controller.health().expect("health").state,
            ModelHealthState::Ready
        );
        let packet = packet(&profile);
        assert_eq!(controller.count_tokens(&packet).expect("count").tokens, 1);
        let result = controller
            .stream(&request(&profile), &packet, None)
            .expect("stream");
        assert_eq!(result.terminal_state, ModelRunTerminalState::Proposed);
        assert_eq!(
            controller
                .resources()
                .expect("resources")
                .resident_memory_bytes,
            1
        );
        assert!(controller.unload().expect("unload").empty);
        assert_eq!(controller.health(), Err(ModelRuntimeGateError::NotLoaded));
    }

    #[test]
    fn changed_unknown_rejected_enabled_and_fallback_profiles_fail_closed() {
        let profile = profile();
        let catalog = ModelAdmissionCatalog::new(vec![profile.clone()]).expect("catalog");

        let mut changed = profile.clone();
        changed.codec.template_sha256 = "b".repeat(64);
        assert_eq!(
            catalog.admit(&changed, ModelUsePurpose::ContractTest),
            Err(ModelRuntimeGateError::ProfileChanged)
        );
        let mut unknown = profile.clone();
        unknown.profile_id = ModelProfileId::from_raw("unknown");
        assert_eq!(
            catalog.admit(&unknown, ModelUsePurpose::ContractTest),
            Err(ModelRuntimeGateError::ProfileNotRegistered)
        );
        let mut rejected = profile.clone();
        rejected.lifecycle = ModelLifecycleState::Rejected;
        let rejected_catalog = ModelAdmissionCatalog::new(vec![rejected.clone()]).expect("catalog");
        assert_eq!(
            rejected_catalog.admit(&rejected, ModelUsePurpose::ContractTest),
            Err(ModelRuntimeGateError::ProfileUnavailable)
        );
        let mut fallback = profile.clone();
        fallback.automatic_fallback = true;
        assert_eq!(
            ModelAdmissionCatalog::new(vec![fallback]),
            Err(ModelRuntimeGateError::ProfileInvalid)
        );
        assert_eq!(
            catalog.admit(&profile, ModelUsePurpose::Product),
            Err(ModelRuntimeGateError::ProfileUnavailable)
        );
    }

    #[test]
    fn manifest_runtime_isolation_packet_and_stream_drift_fail_before_success() {
        let profile = profile();
        let catalog = ModelAdmissionCatalog::new(vec![profile.clone()]).expect("catalog");

        for (manifest_drift, isolation_drift, expected) in [
            (
                true,
                false,
                ModelRuntimeGateError::ManifestObservationMismatch,
            ),
            (false, true, ModelRuntimeGateError::IsolationViolation),
        ] {
            let admitted = catalog
                .admit(&profile, ModelUsePurpose::ContractTest)
                .expect("admitted");
            let mut fake = FakeRuntime::new(&profile);
            fake.manifest_drift = manifest_drift;
            fake.isolation_drift = isolation_drift;
            let mut controller = LocalModelController::new(
                fake,
                ClosedJsonFamilyCodec::new(profile.codec.clone()),
                admitted,
            )
            .expect("controller");
            assert_eq!(controller.load(), Err(expected));
        }

        let admitted = catalog
            .admit(&profile, ModelUsePurpose::ContractTest)
            .expect("admitted");
        let mut fake = FakeRuntime::new(&profile);
        fake.scenario = FakeScenario::Replay;
        let mut controller = LocalModelController::new(
            fake,
            ClosedJsonFamilyCodec::new(profile.codec.clone()),
            admitted,
        )
        .expect("controller");
        controller.load().expect("load");
        assert_eq!(
            controller.stream(&request(&profile), &packet(&profile), None),
            Err(ModelRuntimeGateError::RuntimeFailure)
        );

        let mut wrong_packet = packet(&profile);
        wrong_packet.manifest_sha256 = "b".repeat(64);
        assert_eq!(
            controller.count_tokens(&wrong_packet),
            Err(ModelRuntimeGateError::RequestMismatch)
        );
    }

    #[test]
    fn fake_muse_and_gemma_cover_every_runtime_terminal_and_hostile_response_state() {
        for family in ["muse", "gemma"] {
            for (scenario, expected) in [
                (FakeScenario::Happy, Ok(ModelRunTerminalState::Proposed)),
                (FakeScenario::Delayed, Ok(ModelRunTerminalState::TimedOut)),
                (
                    FakeScenario::Cancelled,
                    Ok(ModelRunTerminalState::Cancelled),
                ),
                (
                    FakeScenario::ResourceExhausted,
                    Ok(ModelRunTerminalState::ResourceExhausted),
                ),
                (
                    FakeScenario::Malformed,
                    Err(ModelRuntimeGateError::RuntimeFailure),
                ),
                (
                    FakeScenario::Crashed,
                    Err(ModelRuntimeGateError::RuntimeFailure),
                ),
                (
                    FakeScenario::Replay,
                    Err(ModelRuntimeGateError::RuntimeFailure),
                ),
                (
                    FakeScenario::FalseCompletion,
                    Err(ModelRuntimeGateError::ResultMismatch),
                ),
            ] {
                let profile = family_profile(family);
                let admitted = ModelAdmissionCatalog::new(vec![profile.clone()])
                    .expect("catalog")
                    .admit(&profile, ModelUsePurpose::ContractTest)
                    .expect("admitted");
                let mut fake = FakeRuntime::new(&profile);
                fake.scenario = scenario;
                let mut controller = LocalModelController::new(
                    fake,
                    ClosedJsonFamilyCodec::new(profile.codec.clone()),
                    admitted,
                )
                .expect("controller");
                controller.load().expect("load");
                let packet = packet(&profile);
                let signal = cancellation();
                let observed = controller
                    .stream(
                        &request(&profile),
                        &packet,
                        (scenario == FakeScenario::Cancelled).then_some(&signal),
                    )
                    .map(|result| result.terminal_state);
                assert_eq!(observed, expected, "family={family} scenario={scenario:?}");
                assert!(controller.unload().expect("unload").empty);
            }
        }
    }

    #[test]
    fn every_exact_profile_dimension_is_validated_without_merging() {
        type ProfileMutation = Box<dyn Fn(&mut ExactModelProfile)>;

        let base = profile();
        let mut mutations: Vec<ProfileMutation> = vec![
            Box::new(|item| item.manifest_sha256 = "invalid".to_owned()),
            Box::new(|item| item.artifact.bytes = 0),
            Box::new(|item| item.transformations[0].output_sha256 = "b".repeat(64)),
            Box::new(|item| item.codec.tokenizer_sha256 = "invalid".to_owned()),
            Box::new(|item| item.codec.end_tokens.clear()),
            Box::new(|item| item.runtime.contract_version = 2),
            Box::new(|item| item.modalities.clear()),
            Box::new(|item| item.context.max_context_tokens = 0),
            Box::new(|item| item.decoding.temperature = f32::NAN),
            Box::new(|item| item.hardware.clear()),
            Box::new(|item| item.capabilities[0].limitations = vec!["x".repeat(257)]),
            Box::new(|item| item.policy_sha256 = "invalid".to_owned()),
            Box::new(|item| item.automatic_fallback = true),
        ];
        for mutate in mutations.drain(..) {
            let mut changed = base.clone();
            mutate(&mut changed);
            assert_eq!(
                ModelAdmissionCatalog::new(vec![changed]),
                Err(ModelRuntimeGateError::ProfileInvalid)
            );
        }
    }

    #[test]
    fn runtime_failure_codes_are_closed_and_content_free() {
        let codes = [
            ModelRuntimeGateError::ProfileInvalid,
            ModelRuntimeGateError::ProfileNotRegistered,
            ModelRuntimeGateError::ProfileChanged,
            ModelRuntimeGateError::ProfileUnavailable,
            ModelRuntimeGateError::AutomaticFallbackProhibited,
            ModelRuntimeGateError::RuntimeMismatch,
            ModelRuntimeGateError::ManifestObservationMismatch,
            ModelRuntimeGateError::IsolationViolation,
            ModelRuntimeGateError::AlreadyLoaded,
            ModelRuntimeGateError::NotLoaded,
            ModelRuntimeGateError::RequestMismatch,
            ModelRuntimeGateError::TokenCountMismatch,
            ModelRuntimeGateError::StreamInvalid,
            ModelRuntimeGateError::ResultMismatch,
            ModelRuntimeGateError::RuntimeFailure,
        ];
        assert_eq!(codes.len(), 15);
        for code in codes {
            assert!(code.code().starts_with("model."));
            assert!(!code.code().contains('/'));
        }
    }
}
