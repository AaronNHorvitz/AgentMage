//! Candidate-neutral exact-profile admission and local runtime orchestration.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{
    CorrelationId, ExactModelProfile, LocalModelRuntime, ModelCancellationProbe,
    ModelCapabilityState, ModelContextPacket, ModelDispatchPreflight, ModelFamilyCodec,
    ModelFinishReason, ModelHealth, ModelHealthState, ModelLifecycleState, ModelLoadReceipt,
    ModelManifestObservation, ModelModality, ModelProfileId, ModelResourceReport, ModelRunRequest,
    ModelRunResult, ModelRunTerminalState, ModelRuntimeFailure, ModelRuntimeIdentity,
    ModelServingCachePolicy, ModelServingCapabilities, ModelStreamSink, ModelUnloadReceipt,
    StreamedModelFragment, TaskId, TokenCountResult,
};
use sha2::{Digest, Sha256};

use crate::model_orchestration_profile::{
    ModelContextWindowPlan, verify_model_context_window_plan,
};
use crate::model_selection::{ModelResourceGovernor, ModelResourceObservation, ResourceDecision};

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
    /// Effective served capabilities are absent for the loaded process.
    ServedCapabilityMissing,
    /// Effective served capabilities name a different admitted tuple.
    ServedCapabilityProfileMismatch,
    /// Effective served capabilities changed after load or contain invalid facts.
    ServedCapabilityDrift,
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
    /// Exact rendered input plus output reserve and safety margin exceeds effective capacity.
    DispatchCapacityExceeded,
    /// A prepared request no longer matches the current process, load, slot, or configuration.
    PreparedRequestStale,
    /// The effective tokenizer count changed between preparation and dispatch.
    DispatchTokenDrift,
    /// A prepared request or its digest does not match its immutable request/context tuple.
    PreparedRequestMismatch,
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
            Self::ServedCapabilityMissing => "model.served-capability.missing",
            Self::ServedCapabilityProfileMismatch => "model.served-capability.profile-mismatch",
            Self::ServedCapabilityDrift => "model.served-capability.drift",
            Self::IsolationViolation => "model.runtime.isolation-violation",
            Self::AlreadyLoaded => "model.runtime.already-loaded",
            Self::NotLoaded => "model.runtime.not-loaded",
            Self::RequestMismatch => "model.runtime.request-mismatch",
            Self::TokenCountMismatch => "model.runtime.token-count-mismatch",
            Self::DispatchCapacityExceeded => "model.prepared-request.capacity-exceeded",
            Self::PreparedRequestStale => "model.prepared-request.stale-binding",
            Self::DispatchTokenDrift => "model.prepared-request.token-drift",
            Self::PreparedRequestMismatch => "model.prepared-request.mismatch",
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

    /// Resolves one user-named exact profile without family lookup or fallback.
    pub fn admit_by_id(
        &self,
        profile_id: &ModelProfileId,
        purpose: ModelUsePurpose,
    ) -> Result<AdmittedModelProfile, ModelRuntimeGateError> {
        let candidate = self
            .profiles
            .iter()
            .find(|profile| profile.profile_id == *profile_id)
            .ok_or(ModelRuntimeGateError::ProfileNotRegistered)?;
        self.admit(candidate, purpose)
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

    /// Returns the complete exact admitted tuple for trusted kernel composition.
    #[must_use]
    pub const fn exact_profile(&self) -> &ExactModelProfile {
        &self.profile
    }
}

/// Kernel-owned controller for one exact selected profile and local adapter.
pub struct LocalModelController<R: LocalModelRuntime, C: ModelFamilyCodec> {
    runtime: R,
    codec: C,
    admitted: AdmittedModelProfile,
    loaded: bool,
    served_capabilities: Option<ModelServingCapabilities>,
    safety_margin_tokens: u32,
}

/// Move-only immutable generation preparation produced by the trusted controller.
///
/// Dispatch consumes this value, so retries, summaries, and recovery runs must render, count, and
/// revalidate a fresh request rather than replaying a stale preparation.
#[derive(Debug, PartialEq, Eq)]
pub struct PreparedModelRequest {
    request: ModelRunRequest,
    context: agentmage_kernel_contracts::EncodedModelContext,
    preflight: ModelDispatchPreflight,
    task_id: TaskId,
}

impl PreparedModelRequest {
    /// Returns content-free preflight facts for diagnostics and evidence.
    #[must_use]
    pub const fn preflight(&self) -> &ModelDispatchPreflight {
        &self.preflight
    }

    /// Returns the exact run request bound by this preparation.
    #[must_use]
    pub const fn request(&self) -> &ModelRunRequest {
        &self.request
    }
}

/// Complete model result plus the exact response bytes validated by the controller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedModelOutput {
    /// Content-free terminal result and optional closed proposal.
    pub result: ModelRunResult,
    /// Exact contiguous response bytes whose digest matches the result.
    pub response_bytes: Vec<u8>,
}

impl<R: LocalModelRuntime, C: ModelFamilyCodec> LocalModelController<R, C> {
    /// Binds one admitted profile to one exact runtime identity.
    pub fn new(
        runtime: R,
        codec: C,
        admitted: AdmittedModelProfile,
        safety_margin_tokens: u32,
    ) -> Result<Self, ModelRuntimeGateError> {
        if runtime.identity() != &admitted.profile.runtime
            || codec.identity() != &admitted.profile.codec
        {
            return Err(ModelRuntimeGateError::RuntimeMismatch);
        }
        if safety_margin_tokens == 0
            || safety_margin_tokens >= admitted.profile.context.max_context_tokens
        {
            return Err(ModelRuntimeGateError::DispatchCapacityExceeded);
        }
        Ok(Self {
            runtime,
            codec,
            admitted,
            loaded: false,
            served_capabilities: None,
            safety_margin_tokens,
        })
    }

    /// Returns the exact admitted profile bound to this controller.
    #[must_use]
    pub const fn exact_profile(&self) -> &ExactModelProfile {
        self.admitted.exact_profile()
    }

    /// Returns the exact purpose admitted before controller construction.
    #[must_use]
    pub const fn purpose(&self) -> ModelUsePurpose {
        self.admitted.purpose()
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
        self.served_capabilities = Some(receipt.served_capabilities.clone());
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
        let current = self.runtime.serving_capabilities().map_err(|error| {
            if error.code == "model.served-capability.missing" {
                ModelRuntimeGateError::ServedCapabilityMissing
            } else {
                ModelRuntimeGateError::ServedCapabilityDrift
            }
        })?;
        validate_served_capabilities(&self.admitted.profile, &current)?;
        if self.served_capabilities.as_ref() != Some(&current) {
            return Err(ModelRuntimeGateError::ServedCapabilityDrift);
        }
        Ok(health)
    }

    /// Returns the current effective served-capability binding after revalidation.
    pub fn serving_capabilities(&self) -> Result<ModelServingCapabilities, ModelRuntimeGateError> {
        self.health()?;
        self.served_capabilities
            .clone()
            .ok_or(ModelRuntimeGateError::ServedCapabilityMissing)
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

    /// Counts and binds tokens for a structurally valid packet before dispatch.
    ///
    /// This is the trusted bridge for callers that cannot run the exact profile tokenizer.
    /// The adapter-produced count is accepted only when every other packet binding and the
    /// encoded-context digest match the loaded tuple.
    pub fn bind_token_count(
        &self,
        packet: &mut ModelContextPacket,
    ) -> Result<TokenCountResult, ModelRuntimeGateError> {
        if !self.loaded {
            return Err(ModelRuntimeGateError::NotLoaded);
        }
        if packet.profile_id != self.admitted.profile.profile_id
            || packet.manifest_sha256 != self.admitted.profile.manifest_sha256
            || packet.messages.is_empty()
            || packet.messages.len() > self.admitted.profile.context.max_messages as usize
            || packet.input_bytes == 0
            || packet.input_bytes > self.admitted.profile.context.max_input_bytes
            || !valid_sha256(&packet.packet_sha256)
        {
            return Err(ModelRuntimeGateError::RequestMismatch);
        }
        packet.input_tokens = 1;
        for _ in 0..4 {
            packet.packet_sha256 = model_packet_digest(packet)?;
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
                || result.tokens == 0
                || result.tokens > self.admitted.profile.context.max_context_tokens
            {
                return Err(ModelRuntimeGateError::TokenCountMismatch);
            }
            if packet.input_tokens == result.tokens {
                return Ok(result);
            }
            packet.input_tokens = result.tokens;
        }
        Err(ModelRuntimeGateError::TokenCountMismatch)
    }

    /// Renders once, counts with the effective tokenizer, and binds current serving capacity.
    pub fn prepare(
        &self,
        request: &ModelRunRequest,
        packet: &ModelContextPacket,
    ) -> Result<PreparedModelRequest, ModelRuntimeGateError> {
        let served = self.serving_capabilities()?;
        self.validate_packet(packet)?;
        self.validate_request(request, packet)?;
        let context = self
            .codec
            .encode_context(&self.admitted.profile, packet)
            .map_err(|_| ModelRuntimeGateError::RequestMismatch)?;
        let count = self
            .runtime
            .count_tokens(&context)
            .map_err(|_| ModelRuntimeGateError::RuntimeFailure)?;
        if count.profile_id != self.admitted.profile.profile_id
            || count.context_packet_id != packet.context_packet_id
            || count.packet_sha256 != context.sha256
            || count.counter != self.admitted.profile.context.token_counter
            || count.tokens != packet.input_tokens
        {
            return Err(ModelRuntimeGateError::DispatchTokenDrift);
        }
        let approved = self.admitted.profile.context.max_context_tokens;
        let effective = approved.min(served.context_capacity_tokens);
        let usable_input = effective
            .checked_sub(request.max_output_tokens)
            .and_then(|remaining| remaining.checked_sub(self.safety_margin_tokens))
            .ok_or(ModelRuntimeGateError::DispatchCapacityExceeded)?;
        if count.tokens > usable_input {
            return Err(ModelRuntimeGateError::DispatchCapacityExceeded);
        }
        let mut preflight = ModelDispatchPreflight {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            context_manifest_sha256: packet.packet_sha256.clone(),
            orchestration_plan_sha256: packet.packet_sha256.clone(),
            rendered_prompt_sha256: context.sha256.clone(),
            rendered_prompt_tokens: count.tokens,
            token_counter: count.counter,
            token_counter_sha256: self.admitted.profile.context.token_counter_sha256.clone(),
            approved_profile_capacity_tokens: approved,
            served_capacity_tokens: served.context_capacity_tokens,
            effective_capacity_tokens: effective,
            total_output_reserve_tokens: request.max_output_tokens,
            safety_margin_tokens: self.safety_margin_tokens,
            usable_input_tokens: usable_input,
            process_generation: served.process_generation,
            load_generation: served.load_generation,
            launch_configuration_sha256: served.launch_configuration_sha256,
            serving_observation_sha256: served.observation_sha256,
            parallel_slots: served.parallel_slots,
            reserved_slot: 0,
            cache_policy: served.cache_policy,
            preflight_sha256: "0".repeat(64),
        };
        preflight.preflight_sha256 = prepared_request_digest(request, &context, &preflight)?;
        Ok(PreparedModelRequest {
            request: request.clone(),
            context,
            preflight,
            task_id: packet.task_id.clone(),
        })
    }

    /// Prepares one request only after its earlier context allocation reconciles exactly.
    pub fn prepare_with_plan(
        &self,
        request: &ModelRunRequest,
        packet: &ModelContextPacket,
        plan: &ModelContextWindowPlan,
    ) -> Result<PreparedModelRequest, ModelRuntimeGateError> {
        let planned_input = [
            plan.system_and_tool_tokens,
            plan.user_input_tokens,
            plan.source_artifacts.allocated_tokens,
            plan.retrieved_context.allocated_tokens,
        ]
        .into_iter()
        .try_fold(0_u32, u32::checked_add)
        .ok_or(ModelRuntimeGateError::DispatchCapacityExceeded)?;
        if verify_model_context_window_plan(plan).is_err()
            || plan.model_profile_id != self.admitted.profile.profile_id.as_str()
            || plan.model_manifest_sha256 != self.admitted.profile.manifest_sha256
            || plan.model_runtime_sha256 != self.admitted.profile.runtime.runtime_sha256
            || plan.tokenizer_sha256 != self.admitted.profile.codec.tokenizer_sha256
            || plan.token_counter_sha256 != self.admitted.profile.context.token_counter_sha256
            || plan.total_window_tokens != self.admitted.profile.context.max_context_tokens
            || plan.output_reserve_tokens != request.max_output_tokens
            || plan.safety_margin_tokens != self.safety_margin_tokens
            || planned_input != packet.input_tokens
        {
            return Err(ModelRuntimeGateError::PreparedRequestMismatch);
        }
        let mut prepared = self.prepare(request, packet)?;
        prepared.preflight.orchestration_plan_sha256 = plan.plan_sha256.clone();
        prepared.preflight.preflight_sha256 =
            prepared_request_digest(&prepared.request, &prepared.context, &prepared.preflight)?;
        Ok(prepared)
    }

    /// Consumes one immutable preparation and validates its complete inert stream.
    pub fn dispatch(
        &mut self,
        prepared: PreparedModelRequest,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<ModelRunResult, ModelRuntimeGateError> {
        self.dispatch_with_output(prepared, cancellation)
            .map(|output| output.result)
    }

    /// Consumes one immutable preparation and returns its validated inert response bytes.
    pub fn dispatch_with_output(
        &mut self,
        prepared: PreparedModelRequest,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<VerifiedModelOutput, ModelRuntimeGateError> {
        let PreparedModelRequest {
            request,
            context,
            preflight,
            task_id,
        } = prepared;
        validate_prepared_request(&self.admitted.profile, &request, &context, &preflight)?;
        let current = self
            .runtime
            .serving_capabilities()
            .map_err(|_| ModelRuntimeGateError::PreparedRequestStale)?;
        if validate_served_capabilities(&self.admitted.profile, &current).is_err()
            || self.served_capabilities.as_ref() != Some(&current)
            || !preflight_matches_serving(&preflight, &current)
        {
            return Err(ModelRuntimeGateError::PreparedRequestStale);
        }
        let bound_cancellation = cancellation.map(|source| BoundCancellationProbe {
            source,
            task_id: &task_id,
            correlation_id: &request.correlation_id,
        });
        let mut capture = StreamCapture::new(&request);
        let mut result = self
            .runtime
            .stream(
                &request,
                &context,
                &preflight,
                bound_cancellation
                    .as_ref()
                    .map(|probe| probe as &dyn ModelCancellationProbe),
                &mut capture,
            )
            .map_err(|error| match error.code.as_str() {
                "model.prepared-request.token-drift" => ModelRuntimeGateError::DispatchTokenDrift,
                "model.prepared-request.stale-binding" => {
                    ModelRuntimeGateError::PreparedRequestStale
                }
                _ => ModelRuntimeGateError::RuntimeFailure,
            })?;
        capture.finish(&result)?;
        if result.response_sha256 != sha256_hex(&capture.bytes) {
            return Err(ModelRuntimeGateError::ResultMismatch);
        }
        if !valid_usage(
            preflight.rendered_prompt_tokens,
            preflight.effective_capacity_tokens,
            &request,
            &result,
        ) || (matches!(
            result.terminal_state,
            ModelRunTerminalState::Proposed | ModelRunTerminalState::AdvisoryText
        ) && !result.finish_reason.is_complete())
        {
            return Err(ModelRuntimeGateError::ResultMismatch);
        }
        if result.terminal_state == ModelRunTerminalState::Proposed {
            let decoded = self
                .codec
                .decode_proposal(&self.admitted.profile, &request, &capture.bytes)
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
        validate_result(&self.admitted.profile, &request, &result)?;
        Ok(VerifiedModelOutput {
            result,
            response_bytes: capture.bytes,
        })
    }

    fn validate_request(
        &self,
        request: &ModelRunRequest,
        packet: &ModelContextPacket,
    ) -> Result<(), ModelRuntimeGateError> {
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
        Ok(())
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

    /// Applies one exact resource observation and unloads this profile on the first breach.
    pub fn enforce_resource_limits(
        &mut self,
        governor: &mut ModelResourceGovernor,
        cpu_time_ms: u64,
        gpu_time_ms: u64,
        inference_slots: u16,
        disk_bytes: u64,
        process_count: u32,
    ) -> Result<ResourceDecision, ModelRuntimeGateError> {
        let report = self.resources()?;
        let decision = governor.observe(ModelResourceObservation::from_runtime(
            &report,
            cpu_time_ms,
            gpu_time_ms,
            inference_slots,
            disk_bytes,
            process_count,
        ));
        if matches!(decision, ResourceDecision::Stop { .. }) {
            self.unload()?;
        }
        Ok(decision)
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
        self.served_capabilities = None;
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
            || model_packet_digest(packet)? != packet.packet_sha256
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
    validate_served_capabilities(profile, &receipt.served_capabilities)?;
    Ok(())
}

fn validate_served_capabilities(
    profile: &ExactModelProfile,
    served: &ModelServingCapabilities,
) -> Result<(), ModelRuntimeGateError> {
    if served.profile_id != profile.profile_id
        || served.manifest_sha256 != profile.manifest_sha256
        || served.artifact_sha256 != profile.artifact.sha256
        || served.adapter_id != profile.runtime.adapter_id
        || served.runtime != profile.runtime
        || served.tokenizer_sha256 != profile.codec.tokenizer_sha256
        || served.template_sha256 != profile.codec.template_sha256
        || served.codec_sha256 != profile.codec.codec_sha256
        || served.reasoning_supported != profile.codec.reasoning_enabled
    {
        return Err(ModelRuntimeGateError::ServedCapabilityProfileMismatch);
    }
    if served.schema_version != 1
        || !valid_text(&served.endpoint)
        || served.process_id == 0
        || served.process_generation == 0
        || served.load_generation == 0
        || !valid_sha256(&served.launch_configuration_sha256)
        || served.context_capacity_tokens < profile.context.max_context_tokens
        || served.parallel_slots == 0
        || served.context_shift_supported
        || !valid_sha256(&served.observation_sha256)
        || (served.parallel_slots > 1
            && served.cache_policy == ModelServingCachePolicy::QualifiedShared)
    {
        return Err(ModelRuntimeGateError::ServedCapabilityDrift);
    }
    Ok(())
}

#[derive(serde::Serialize)]
struct PreparedRequestDigest<'a> {
    request: &'a ModelRunRequest,
    context: &'a agentmage_kernel_contracts::EncodedModelContext,
    preflight: &'a ModelDispatchPreflight,
}

fn prepared_request_digest(
    request: &ModelRunRequest,
    context: &agentmage_kernel_contracts::EncodedModelContext,
    preflight: &ModelDispatchPreflight,
) -> Result<String, ModelRuntimeGateError> {
    let mut candidate = preflight.clone();
    candidate.preflight_sha256 = "0".repeat(64);
    let bytes = serde_json::to_vec(&PreparedRequestDigest {
        request,
        context,
        preflight: &candidate,
    })
    .map_err(|_| ModelRuntimeGateError::PreparedRequestMismatch)?;
    Ok(sha256_hex(&bytes))
}

fn validate_prepared_request(
    profile: &ExactModelProfile,
    request: &ModelRunRequest,
    context: &agentmage_kernel_contracts::EncodedModelContext,
    preflight: &ModelDispatchPreflight,
) -> Result<(), ModelRuntimeGateError> {
    let usable = preflight
        .effective_capacity_tokens
        .checked_sub(preflight.total_output_reserve_tokens)
        .and_then(|remaining| remaining.checked_sub(preflight.safety_margin_tokens))
        .ok_or(ModelRuntimeGateError::DispatchCapacityExceeded)?;
    if preflight.schema_version != agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION
        || context.profile_id != profile.profile_id
        || context.context_packet_id != request.context_packet_id
        || preflight.context_manifest_sha256.is_empty()
        || !valid_sha256(&preflight.orchestration_plan_sha256)
        || preflight.rendered_prompt_sha256 != context.sha256
        || preflight.rendered_prompt_tokens == 0
        || preflight.token_counter != profile.context.token_counter
        || preflight.token_counter_sha256 != profile.context.token_counter_sha256
        || preflight.approved_profile_capacity_tokens != profile.context.max_context_tokens
        || preflight.effective_capacity_tokens
            != preflight
                .approved_profile_capacity_tokens
                .min(preflight.served_capacity_tokens)
        || preflight.total_output_reserve_tokens != request.max_output_tokens
        || preflight.safety_margin_tokens == 0
        || preflight.usable_input_tokens != usable
        || preflight.rendered_prompt_tokens > usable
        || preflight.process_generation == 0
        || preflight.load_generation == 0
        || !valid_sha256(&preflight.context_manifest_sha256)
        || !valid_sha256(&preflight.rendered_prompt_sha256)
        || !valid_sha256(&preflight.launch_configuration_sha256)
        || !valid_sha256(&preflight.serving_observation_sha256)
        || !valid_sha256(&preflight.preflight_sha256)
        || preflight.parallel_slots == 0
        || preflight.reserved_slot >= preflight.parallel_slots
        || prepared_request_digest(request, context, preflight)? != preflight.preflight_sha256
    {
        return Err(ModelRuntimeGateError::PreparedRequestMismatch);
    }
    Ok(())
}

fn preflight_matches_serving(
    preflight: &ModelDispatchPreflight,
    served: &ModelServingCapabilities,
) -> bool {
    preflight.served_capacity_tokens == served.context_capacity_tokens
        && preflight.process_generation == served.process_generation
        && preflight.load_generation == served.load_generation
        && preflight.launch_configuration_sha256 == served.launch_configuration_sha256
        && preflight.serving_observation_sha256 == served.observation_sha256
        && preflight.parallel_slots == served.parallel_slots
        && preflight.reserved_slot < served.parallel_slots
        && preflight.cache_policy == served.cache_policy
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
        || result.resources.input_tokens != result.usage.rendered_prompt_tokens
        || result.resources.output_tokens != result.usage.generated_output_tokens
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
    match result.finish_reason {
        ModelFinishReason::Cancelled
            if result.terminal_state != ModelRunTerminalState::Cancelled =>
        {
            return Err(ModelRuntimeGateError::ResultMismatch);
        }
        ModelFinishReason::DeadlineExceeded
            if result.terminal_state != ModelRunTerminalState::TimedOut =>
        {
            return Err(ModelRuntimeGateError::ResultMismatch);
        }
        ModelFinishReason::TransportFailure
            if result.terminal_state != ModelRunTerminalState::Failed =>
        {
            return Err(ModelRuntimeGateError::ResultMismatch);
        }
        reason
            if !reason.is_complete()
                && matches!(
                    result.terminal_state,
                    ModelRunTerminalState::Proposed | ModelRunTerminalState::AdvisoryText
                ) =>
        {
            return Err(ModelRuntimeGateError::ResultMismatch);
        }
        _ => {}
    }
    Ok(())
}

fn valid_usage(
    packet_input_tokens: u32,
    served_capacity: u32,
    request: &ModelRunRequest,
    result: &ModelRunResult,
) -> bool {
    let usage = &result.usage;
    if usage.rendered_prompt_tokens != packet_input_tokens
        || usage.generated_output_tokens != result.resources.output_tokens
        || usage.output_token_reserve != request.max_output_tokens
        || usage.generated_output_tokens > usage.output_token_reserve
        || usage
            .reasoning_output_tokens
            .is_some_and(|tokens| tokens > usage.generated_output_tokens)
        || usage.cached_input_tokens.is_some() != usage.evaluated_input_tokens.is_some()
    {
        return false;
    }
    if let (Some(cached), Some(evaluated)) =
        (usage.cached_input_tokens, usage.evaluated_input_tokens)
        && (cached > evaluated
            || if result.finish_reason == ModelFinishReason::ContextTruncation {
                evaluated > usage.rendered_prompt_tokens
            } else {
                evaluated != usage.rendered_prompt_tokens
            })
    {
        return false;
    }
    usage.remaining_capacity_tokens
        == Some(
            served_capacity.saturating_sub(
                usage
                    .rendered_prompt_tokens
                    .saturating_add(usage.generated_output_tokens),
            ),
        )
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

fn model_packet_digest(packet: &ModelContextPacket) -> Result<String, ModelRuntimeGateError> {
    let mut candidate = packet.clone();
    candidate.packet_sha256 = "0".repeat(64);
    serde_json::to_vec(&candidate)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| ModelRuntimeGateError::RequestMismatch)
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
    use std::cell::Cell;
    use std::rc::Rc;

    use agentmage_kernel_contracts::{
        BoundaryKind, CONTRACT_SCHEMA_VERSION, CancellationId, CancellationReason,
        CancellationSignal, ContextBudget, ContextPacketId, CorrelationId, DecodingProfile,
        EncodedModelContext, ExactModelProfile, FamilyCodecIdentity, HardwareEnvelope,
        LocalModelRuntime, ModelAdapterId, ModelArtifact, ModelCancellationProbe, ModelCapability,
        ModelCapabilityState, ModelCodecId, ModelContextPacket, ModelFinishReason, ModelHealth,
        ModelHealthState, ModelLifecycleState, ModelLoadReceipt, ModelManifestId,
        ModelManifestObservation, ModelMessage, ModelMessageId, ModelMessageRole, ModelModality,
        ModelProfileId, ModelProposalKind, ModelResourceReport, ModelRole, ModelRunId,
        ModelRunRequest, ModelRunResult, ModelRunTerminalState, ModelRuntimeFailure,
        ModelRuntimeIdentity, ModelRuntimeKind, ModelServingCachePolicy, ModelServingCapabilities,
        ModelStreamId, ModelStreamSink, ModelTokenUsage, ModelTransformation, ModelUnloadReceipt,
        PlatformArchitecture, PlatformFamily, RuntimeIsolationObservation, SessionId,
        StreamedModelFragment, TaskId, TokenCountResult, ToolCatalogId,
    };

    use super::{
        LocalModelController, ModelAdmissionCatalog, ModelRuntimeGateError, ModelUsePurpose,
        runtime_failure, sha256_hex, validate_served_capabilities,
    };
    use crate::model_codec::ClosedJsonFamilyCodec;
    use crate::model_orchestration_profile::{
        AdaptableTokenDemand, ContextWindowDemand, ExactTokenCounterBinding, compile_context_window,
    };

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

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum ServedDrift {
        Missing,
        Endpoint,
        ProcessGeneration,
        LoadGeneration,
        Tokenizer,
        Template,
        Slots,
        CachePolicy,
    }

    #[derive(Clone)]
    struct FakeRuntime {
        identity: ModelRuntimeIdentity,
        token_counter: String,
        loaded: Option<ModelProfileId>,
        served: Option<ModelServingCapabilities>,
        manifest_drift: bool,
        isolation_drift: bool,
        served_drift_after_load: Option<ServedDrift>,
        scenario: FakeScenario,
        resident_memory_bytes: u64,
        load_generation: u64,
        stream_calls: Rc<Cell<u32>>,
        generation_calls: Rc<Cell<u32>>,
        counted_tokens: Rc<Cell<u32>>,
    }

    impl FakeRuntime {
        fn new(profile: &ExactModelProfile) -> Self {
            Self {
                identity: profile.runtime.clone(),
                token_counter: profile.context.token_counter.clone(),
                loaded: None,
                served: None,
                manifest_drift: false,
                isolation_drift: false,
                served_drift_after_load: None,
                scenario: FakeScenario::Happy,
                resident_memory_bytes: 1,
                load_generation: 0,
                stream_calls: Rc::new(Cell::new(0)),
                generation_calls: Rc::new(Cell::new(0)),
                counted_tokens: Rc::new(Cell::new(1)),
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
            self.load_generation += 1;
            self.loaded = Some(profile.profile_id.clone());
            let served = ModelServingCapabilities {
                schema_version: 1,
                profile_id: profile.profile_id.clone(),
                manifest_sha256: profile.manifest_sha256.clone(),
                artifact_sha256: profile.artifact.sha256.clone(),
                adapter_id: self.identity.adapter_id.clone(),
                runtime: self.identity.clone(),
                endpoint: "fixture://served-profile".to_owned(),
                process_id: 1,
                process_generation: 1,
                load_generation: self.load_generation,
                launch_configuration_sha256: SHA.to_owned(),
                context_capacity_tokens: profile.context.max_context_tokens,
                parallel_slots: 1,
                cache_policy: ModelServingCachePolicy::Disabled,
                tokenizer_sha256: profile.codec.tokenizer_sha256.clone(),
                template_sha256: profile.codec.template_sha256.clone(),
                codec_sha256: profile.codec.codec_sha256.clone(),
                reasoning_supported: profile.codec.reasoning_enabled,
                context_shift_supported: false,
                observed_at_ms: 1,
                observation_sha256: SHA.to_owned(),
            };
            self.served = Some(served.clone());
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
                served_capabilities: served,
            })
        }

        fn unload(
            &mut self,
            profile_id: &ModelProfileId,
        ) -> Result<ModelUnloadReceipt, ModelRuntimeFailure> {
            self.loaded = None;
            self.served = None;
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

        fn serving_capabilities(&self) -> Result<ModelServingCapabilities, ModelRuntimeFailure> {
            if self.served_drift_after_load == Some(ServedDrift::Missing) {
                return Err(runtime_failure("model.served-capability.missing"));
            }
            let mut served = self
                .served
                .clone()
                .ok_or_else(|| runtime_failure("model.served-capability.missing"))?;
            match self.served_drift_after_load {
                Some(ServedDrift::Endpoint) => served.endpoint.push_str("-changed"),
                Some(ServedDrift::ProcessGeneration) => served.process_generation += 1,
                Some(ServedDrift::LoadGeneration) => served.load_generation += 1,
                Some(ServedDrift::Tokenizer) => served.tokenizer_sha256 = "b".repeat(64),
                Some(ServedDrift::Template) => served.template_sha256 = "b".repeat(64),
                Some(ServedDrift::Slots) => served.parallel_slots += 1,
                Some(ServedDrift::CachePolicy) => {
                    served.cache_policy = ModelServingCachePolicy::IsolatedPerSlot
                }
                Some(ServedDrift::Missing) | None => {}
            }
            Ok(served)
        }

        fn count_tokens(
            &self,
            context: &EncodedModelContext,
        ) -> Result<TokenCountResult, ModelRuntimeFailure> {
            Ok(TokenCountResult {
                profile_id: context.profile_id.clone(),
                context_packet_id: context.context_packet_id.clone(),
                tokens: self.counted_tokens.get(),
                counter: self.token_counter.clone(),
                packet_sha256: context.sha256.clone(),
            })
        }

        fn stream(
            &mut self,
            request: &ModelRunRequest,
            _context: &EncodedModelContext,
            preflight: &agentmage_kernel_contracts::ModelDispatchPreflight,
            cancellation: Option<&dyn ModelCancellationProbe>,
            sink: &mut dyn ModelStreamSink,
        ) -> Result<ModelRunResult, ModelRuntimeFailure> {
            self.stream_calls.set(self.stream_calls.get() + 1);
            if self.counted_tokens.get() != preflight.rendered_prompt_tokens {
                return Err(runtime_failure("model.prepared-request.token-drift"));
            }
            self.generation_calls.set(self.generation_calls.get() + 1);
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
                finish_reason: match self.scenario {
                    FakeScenario::FalseCompletion => ModelFinishReason::ReasoningExhausted,
                    _ => match terminal_state {
                        ModelRunTerminalState::Proposed => ModelFinishReason::EndOfSequence,
                        ModelRunTerminalState::TimedOut => ModelFinishReason::DeadlineExceeded,
                        ModelRunTerminalState::Cancelled => ModelFinishReason::Cancelled,
                        _ => ModelFinishReason::Unknown,
                    },
                },
                fragment_count: 1,
                response_sha256,
                proposal,
                failure,
                usage: ModelTokenUsage {
                    rendered_prompt_tokens: self.counted_tokens.get(),
                    cached_input_tokens: None,
                    evaluated_input_tokens: None,
                    generated_output_tokens: 1,
                    reasoning_output_tokens: None,
                    output_token_reserve: request.max_output_tokens,
                    remaining_capacity_tokens: self.served.as_ref().and_then(|served| {
                        served
                            .context_capacity_tokens
                            .checked_sub(self.counted_tokens.get().checked_add(1)?)
                    }),
                },
                resources: ModelResourceReport {
                    adapter_id: self.identity.adapter_id.clone(),
                    profile_id: request.profile_id.clone(),
                    model_run_id: Some(request.model_run_id.clone()),
                    resident_memory_bytes: 1,
                    accelerator_memory_bytes: 0,
                    input_tokens: self.counted_tokens.get(),
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
                resident_memory_bytes: self.resident_memory_bytes,
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
        let mut packet = ModelContextPacket {
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
        };
        packet.packet_sha256 = super::model_packet_digest(&packet).expect("packet digest");
        packet
    }

    fn reseal_packet(packet: &mut ModelContextPacket) {
        packet.packet_sha256 = super::model_packet_digest(packet).expect("packet digest");
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
            1,
        )
        .expect("controller");

        controller.load().expect("load");
        assert_eq!(
            controller.health().expect("health").state,
            ModelHealthState::Ready
        );
        let mut packet = packet(&profile);
        packet.input_tokens = 77;
        controller
            .bind_token_count(&mut packet)
            .expect("runtime-bound count");
        assert_eq!(packet.input_tokens, 1);
        assert_eq!(
            packet.packet_sha256,
            super::model_packet_digest(&packet).expect("packet digest")
        );
        assert_eq!(controller.count_tokens(&packet).expect("count").tokens, 1);
        let prepared = controller
            .prepare(&request(&profile), &packet)
            .expect("prepare");
        let output = controller
            .dispatch_with_output(prepared, None)
            .expect("stream");
        assert_eq!(
            output.result.terminal_state,
            ModelRunTerminalState::Proposed
        );
        assert!(!output.response_bytes.is_empty());
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
    fn dispatch_preflight_accepts_exact_fit_and_refuses_one_token_over_before_stream() {
        for (input_tokens, expected) in [
            (111, Ok(ModelRunTerminalState::Proposed)),
            (112, Err(ModelRuntimeGateError::DispatchCapacityExceeded)),
        ] {
            let profile = profile();
            let admitted = ModelAdmissionCatalog::new(vec![profile.clone()])
                .expect("catalog")
                .admit(&profile, ModelUsePurpose::ContractTest)
                .expect("admitted");
            let runtime = FakeRuntime::new(&profile);
            runtime.counted_tokens.set(input_tokens);
            let stream_calls = Rc::clone(&runtime.stream_calls);
            let generation_calls = Rc::clone(&runtime.generation_calls);
            let mut controller = LocalModelController::new(
                runtime,
                ClosedJsonFamilyCodec::new(profile.codec.clone()),
                admitted,
                1,
            )
            .expect("controller");
            controller.load().expect("load");
            let mut packet = packet(&profile);
            packet.input_tokens = input_tokens;
            reseal_packet(&mut packet);
            let observed = controller
                .prepare(&request(&profile), &packet)
                .and_then(|prepared| controller.dispatch(prepared, None))
                .map(|result| result.terminal_state);
            assert_eq!(observed, expected, "input_tokens={input_tokens}");
            let dispatched = u32::from(expected.is_ok());
            assert_eq!(stream_calls.get(), dispatched);
            assert_eq!(generation_calls.get(), dispatched);
        }
    }

    #[test]
    fn dispatch_preflight_refuses_7000_plus_2048_against_8192_and_invalid_margin() {
        let mut bounded_profile = profile();
        bounded_profile.context.max_context_tokens = 8_192;
        bounded_profile.decoding.max_output_tokens = 2_048;
        let admitted = ModelAdmissionCatalog::new(vec![bounded_profile.clone()])
            .expect("catalog")
            .admit(&bounded_profile, ModelUsePurpose::ContractTest)
            .expect("admitted");
        assert!(matches!(
            LocalModelController::new(
                FakeRuntime::new(&bounded_profile),
                ClosedJsonFamilyCodec::new(bounded_profile.codec.clone()),
                admitted.clone(),
                0,
            ),
            Err(ModelRuntimeGateError::DispatchCapacityExceeded)
        ));
        let runtime = FakeRuntime::new(&bounded_profile);
        runtime.counted_tokens.set(7_000);
        let generation_calls = Rc::clone(&runtime.generation_calls);
        let mut controller = LocalModelController::new(
            runtime,
            ClosedJsonFamilyCodec::new(bounded_profile.codec.clone()),
            admitted,
            1,
        )
        .expect("controller");
        controller.load().expect("load");
        let mut bounded_packet = packet(&bounded_profile);
        bounded_packet.input_tokens = 7_000;
        reseal_packet(&mut bounded_packet);
        let mut run = request(&bounded_profile);
        run.max_output_tokens = 2_048;
        assert_eq!(
            controller.prepare(&run, &bounded_packet),
            Err(ModelRuntimeGateError::DispatchCapacityExceeded)
        );
        assert_eq!(generation_calls.get(), 0);

        let mut maximum = profile();
        maximum.context.max_context_tokens = u32::MAX;
        maximum.decoding.max_output_tokens = u32::MAX;
        let admitted = ModelAdmissionCatalog::new(vec![maximum.clone()])
            .expect("maximum-bound catalog")
            .admit(&maximum, ModelUsePurpose::ContractTest)
            .expect("maximum-bound admission");
        let runtime = FakeRuntime::new(&maximum);
        let generation_calls = Rc::clone(&runtime.generation_calls);
        let mut controller = LocalModelController::new(
            runtime,
            ClosedJsonFamilyCodec::new(maximum.codec.clone()),
            admitted,
            1,
        )
        .expect("maximum-bound controller");
        controller.load().expect("maximum-bound load");
        let mut run = request(&maximum);
        run.max_output_tokens = u32::MAX;
        assert_eq!(
            controller.prepare(&run, &packet(&maximum)),
            Err(ModelRuntimeGateError::DispatchCapacityExceeded)
        );
        assert_eq!(generation_calls.get(), 0);
    }

    #[test]
    fn prepared_request_mutation_stale_binding_and_token_drift_send_no_generation_bytes() {
        let profile = profile();
        let admitted = ModelAdmissionCatalog::new(vec![profile.clone()])
            .expect("catalog")
            .admit(&profile, ModelUsePurpose::ContractTest)
            .expect("admitted");

        let runtime = FakeRuntime::new(&profile);
        let stream_calls = Rc::clone(&runtime.stream_calls);
        let generation_calls = Rc::clone(&runtime.generation_calls);
        let mut controller = LocalModelController::new(
            runtime,
            ClosedJsonFamilyCodec::new(profile.codec.clone()),
            admitted.clone(),
            1,
        )
        .expect("controller");
        controller.load().expect("load");
        let mut prepared = controller
            .prepare(&request(&profile), &packet(&profile))
            .expect("prepare");
        prepared.context.bytes.push(b'!');
        assert_eq!(
            controller.dispatch(prepared, None),
            Err(ModelRuntimeGateError::PreparedRequestMismatch)
        );
        assert_eq!(stream_calls.get(), 0);
        assert_eq!(generation_calls.get(), 0);

        let runtime = FakeRuntime::new(&profile);
        let stream_calls = Rc::clone(&runtime.stream_calls);
        let mut controller = LocalModelController::new(
            runtime,
            ClosedJsonFamilyCodec::new(profile.codec.clone()),
            admitted.clone(),
            1,
        )
        .expect("controller");
        controller.load().expect("load");
        let prepared = controller
            .prepare(&request(&profile), &packet(&profile))
            .expect("prepare");
        controller.runtime.served_drift_after_load = Some(ServedDrift::Slots);
        assert_eq!(
            controller.dispatch(prepared, None),
            Err(ModelRuntimeGateError::PreparedRequestStale)
        );
        assert_eq!(stream_calls.get(), 0);

        let runtime = FakeRuntime::new(&profile);
        let stream_calls = Rc::clone(&runtime.stream_calls);
        let generation_calls = Rc::clone(&runtime.generation_calls);
        let counted_tokens = Rc::clone(&runtime.counted_tokens);
        let mut controller = LocalModelController::new(
            runtime,
            ClosedJsonFamilyCodec::new(profile.codec.clone()),
            admitted,
            1,
        )
        .expect("controller");
        controller.load().expect("load");
        let prepared = controller
            .prepare(&request(&profile), &packet(&profile))
            .expect("prepare");
        counted_tokens.set(2);
        assert_eq!(
            controller.dispatch(prepared, None),
            Err(ModelRuntimeGateError::DispatchTokenDrift)
        );
        assert_eq!(stream_calls.get(), 1);
        assert_eq!(generation_calls.get(), 0);
    }

    #[test]
    fn orchestration_plan_and_tool_heavy_unicode_render_reconcile_once() {
        let profile = profile();
        let admitted = ModelAdmissionCatalog::new(vec![profile.clone()])
            .expect("catalog")
            .admit(&profile, ModelUsePurpose::ContractTest)
            .expect("admitted");
        let runtime = FakeRuntime::new(&profile);
        runtime.counted_tokens.set(4);
        let mut controller = LocalModelController::new(
            runtime,
            ClosedJsonFamilyCodec::new(profile.codec.clone()),
            admitted,
            1,
        )
        .expect("controller");
        controller.load().expect("load");
        let plan = compile_context_window(
            &profile,
            &ContextWindowDemand {
                counter: ExactTokenCounterBinding {
                    token_counter_id: profile.context.token_counter.clone(),
                    token_counter_sha256: profile.context.token_counter_sha256.clone(),
                    tokenizer_sha256: profile.codec.tokenizer_sha256.clone(),
                },
                system_and_tool_tokens: 1,
                user_input_tokens: 1,
                source_artifacts: AdaptableTokenDemand {
                    requested_tokens: 1,
                    minimum_tokens: 1,
                },
                retrieved_context: AdaptableTokenDemand {
                    requested_tokens: 1,
                    minimum_tokens: 1,
                },
                workflow_recovery_reserve_tokens: 1,
                output_reserve_tokens: profile.decoding.max_output_tokens,
                safety_margin_tokens: 1,
            },
        )
        .expect("context plan");
        let mut packet = packet(&profile);
        let tool_bytes = "{\"schema\":\"工具🧰\",\"special\":\"<|end|>\"}"
            .as_bytes()
            .to_vec();
        packet.input_bytes += u64::try_from(tool_bytes.len()).expect("bounded fixture");
        packet.messages.push(ModelMessage {
            message_id: ModelMessageId::from_raw("message-tool-unicode"),
            role: ModelMessageRole::Tool,
            content: agentmage_kernel_contracts::ContractPayload {
                schema: agentmage_kernel_contracts::SchemaReference {
                    schema_id: agentmage_kernel_contracts::SchemaId::from_raw("tool-result-v1"),
                    schema_version: 1,
                    schema_sha256: SHA.to_owned(),
                },
                media_type: "application/json".to_owned(),
                sha256: sha256_hex(&tool_bytes),
                bytes: tool_bytes,
            },
        });
        packet.input_tokens = 4;
        reseal_packet(&mut packet);
        let prepared = controller
            .prepare_with_plan(&request(&profile), &packet, &plan)
            .expect("plan-bound preparation");
        assert_eq!(
            prepared.preflight().orchestration_plan_sha256,
            plan.plan_sha256
        );
        assert_eq!(prepared.preflight().rendered_prompt_tokens, 4);
        assert_eq!(
            controller
                .dispatch(prepared, None)
                .map(|value| value.terminal_state),
            Ok(ModelRunTerminalState::Proposed)
        );
    }

    #[test]
    fn served_capabilities_reject_missing_forged_undersized_and_unsafe_facts() {
        let profile = profile();
        let mut runtime = FakeRuntime::new(&profile);
        let baseline = runtime
            .load(&profile)
            .expect("fixture load")
            .served_capabilities;
        assert_eq!(validate_served_capabilities(&profile, &baseline), Ok(()));

        macro_rules! rejects {
            ($field:ident, $value:expr, $expected:expr) => {{
                let mut changed = baseline.clone();
                changed.$field = $value;
                assert_eq!(
                    validate_served_capabilities(&profile, &changed),
                    Err($expected),
                    "field {}",
                    stringify!($field)
                );
            }};
        }
        rejects!(
            endpoint,
            String::new(),
            ModelRuntimeGateError::ServedCapabilityDrift
        );
        rejects!(
            schema_version,
            0,
            ModelRuntimeGateError::ServedCapabilityDrift
        );
        rejects!(
            profile_id,
            ModelProfileId::from_raw("foreign"),
            ModelRuntimeGateError::ServedCapabilityProfileMismatch
        );
        rejects!(
            manifest_sha256,
            "b".repeat(64),
            ModelRuntimeGateError::ServedCapabilityProfileMismatch
        );
        rejects!(
            artifact_sha256,
            "b".repeat(64),
            ModelRuntimeGateError::ServedCapabilityProfileMismatch
        );
        rejects!(process_id, 0, ModelRuntimeGateError::ServedCapabilityDrift);
        rejects!(
            process_generation,
            0,
            ModelRuntimeGateError::ServedCapabilityDrift
        );
        rejects!(
            load_generation,
            0,
            ModelRuntimeGateError::ServedCapabilityDrift
        );
        rejects!(
            launch_configuration_sha256,
            String::new(),
            ModelRuntimeGateError::ServedCapabilityDrift
        );
        rejects!(
            context_capacity_tokens,
            profile.context.max_context_tokens - 1,
            ModelRuntimeGateError::ServedCapabilityDrift
        );
        rejects!(
            parallel_slots,
            0,
            ModelRuntimeGateError::ServedCapabilityDrift
        );
        rejects!(
            tokenizer_sha256,
            "b".repeat(64),
            ModelRuntimeGateError::ServedCapabilityProfileMismatch
        );
        rejects!(
            template_sha256,
            "b".repeat(64),
            ModelRuntimeGateError::ServedCapabilityProfileMismatch
        );
        rejects!(
            codec_sha256,
            "b".repeat(64),
            ModelRuntimeGateError::ServedCapabilityProfileMismatch
        );
        rejects!(
            reasoning_supported,
            !profile.codec.reasoning_enabled,
            ModelRuntimeGateError::ServedCapabilityProfileMismatch
        );
        rejects!(
            observation_sha256,
            String::new(),
            ModelRuntimeGateError::ServedCapabilityDrift
        );
        rejects!(
            context_shift_supported,
            true,
            ModelRuntimeGateError::ServedCapabilityDrift
        );

        let mut unsafe_shared = baseline;
        unsafe_shared.parallel_slots = 2;
        unsafe_shared.cache_policy = ModelServingCachePolicy::QualifiedShared;
        assert_eq!(
            validate_served_capabilities(&profile, &unsafe_shared),
            Err(ModelRuntimeGateError::ServedCapabilityDrift)
        );
        let mut isolated_parallel = unsafe_shared;
        isolated_parallel.cache_policy = ModelServingCachePolicy::IsolatedPerSlot;
        assert_eq!(
            validate_served_capabilities(&profile, &isolated_parallel),
            Ok(())
        );
    }

    #[test]
    fn health_rejects_every_served_capability_drift_before_dispatch() {
        let profile = profile();
        let admitted = ModelAdmissionCatalog::new(vec![profile.clone()])
            .expect("catalog")
            .admit(&profile, ModelUsePurpose::ContractTest)
            .expect("admitted");
        for (drift, expected) in [
            (
                ServedDrift::Missing,
                ModelRuntimeGateError::ServedCapabilityMissing,
            ),
            (
                ServedDrift::Endpoint,
                ModelRuntimeGateError::ServedCapabilityDrift,
            ),
            (
                ServedDrift::ProcessGeneration,
                ModelRuntimeGateError::ServedCapabilityDrift,
            ),
            (
                ServedDrift::LoadGeneration,
                ModelRuntimeGateError::ServedCapabilityDrift,
            ),
            (
                ServedDrift::Tokenizer,
                ModelRuntimeGateError::ServedCapabilityProfileMismatch,
            ),
            (
                ServedDrift::Template,
                ModelRuntimeGateError::ServedCapabilityProfileMismatch,
            ),
            (
                ServedDrift::Slots,
                ModelRuntimeGateError::ServedCapabilityDrift,
            ),
            (
                ServedDrift::CachePolicy,
                ModelRuntimeGateError::ServedCapabilityDrift,
            ),
        ] {
            let mut runtime = FakeRuntime::new(&profile);
            runtime.served_drift_after_load = Some(drift);
            runtime.scenario = FakeScenario::Crashed;
            let stream_calls = Rc::clone(&runtime.stream_calls);
            let mut controller = LocalModelController::new(
                runtime,
                ClosedJsonFamilyCodec::new(profile.codec.clone()),
                admitted.clone(),
                1,
            )
            .expect("controller");
            controller.load().expect("load");
            assert_eq!(controller.health(), Err(expected), "drift={drift:?}");
            assert_eq!(
                controller.prepare(&request(&profile), &packet(&profile)),
                Err(expected),
                "drift={drift:?}"
            );
            assert_eq!(stream_calls.get(), 0, "drift={drift:?}");
        }
    }

    #[test]
    fn served_capability_stable_sleep_reobservation_and_reload_generation_are_explicit() {
        let profile = profile();
        let admitted = ModelAdmissionCatalog::new(vec![profile.clone()])
            .expect("catalog")
            .admit(&profile, ModelUsePurpose::ContractTest)
            .expect("admitted");
        let mut controller = LocalModelController::new(
            FakeRuntime::new(&profile),
            ClosedJsonFamilyCodec::new(profile.codec.clone()),
            admitted,
            1,
        )
        .expect("controller");
        let first = controller.load().expect("first load").served_capabilities;
        std::thread::sleep(std::time::Duration::from_millis(1));
        assert_eq!(controller.serving_capabilities(), Ok(first.clone()));
        controller.unload().expect("unload");
        let second = controller.load().expect("reload").served_capabilities;
        assert_eq!(second.process_generation, first.process_generation);
        assert_eq!(second.load_generation, first.load_generation + 1);
        assert_ne!(second, first);
    }

    #[test]
    fn resource_breach_unloads_only_the_exact_active_profile() {
        let profile = profile();
        let admitted = ModelAdmissionCatalog::new(vec![profile.clone()])
            .expect("catalog")
            .admit(&profile, ModelUsePurpose::ContractTest)
            .expect("admitted");
        let mut runtime = FakeRuntime::new(&profile);
        runtime.resident_memory_bytes = 11;
        let mut controller = LocalModelController::new(
            runtime,
            ClosedJsonFamilyCodec::new(profile.codec.clone()),
            admitted,
            1,
        )
        .expect("controller");
        controller.load().expect("load");
        let mut governor = crate::model_selection::ModelResourceGovernor::new(
            crate::model_selection::ModelResourceLimits {
                resident_memory_bytes: 10,
                accelerator_memory_bytes: 10,
                cpu_time_ms: 10,
                gpu_time_ms: 10,
                context_tokens: 10,
                output_tokens: 10,
                inference_slots: 1,
                disk_bytes: 10,
                process_count: 1,
            },
        )
        .expect("governor");

        assert_eq!(
            controller
                .enforce_resource_limits(&mut governor, 1, 1, 1, 1, 1)
                .expect("pressure decision"),
            crate::model_selection::ResourceDecision::Stop {
                resource: crate::model_selection::ModelResourceKind::ResidentMemory,
            }
        );
        assert_eq!(controller.health(), Err(ModelRuntimeGateError::NotLoaded));
        assert_eq!(
            controller.enforce_resource_limits(&mut governor, 1, 1, 1, 1, 1),
            Err(ModelRuntimeGateError::NotLoaded)
        );
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
                1,
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
            1,
        )
        .expect("controller");
        controller.load().expect("load");
        let prepared = controller
            .prepare(&request(&profile), &packet(&profile))
            .expect("prepare");
        assert_eq!(
            controller.dispatch(prepared, None),
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
                    1,
                )
                .expect("controller");
                controller.load().expect("load");
                let packet = packet(&profile);
                let signal = cancellation();
                let prepared = controller
                    .prepare(&request(&profile), &packet)
                    .expect("prepare");
                let observed = controller
                    .dispatch(
                        prepared,
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
