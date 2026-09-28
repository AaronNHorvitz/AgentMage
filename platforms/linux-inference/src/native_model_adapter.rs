//! Candidate-neutral Linux `LocalModelRuntime` adapter boundary.

use agentmage_kernel_contracts::{
    EncodedModelContext, ExactModelProfile, LocalModelRuntime, ModelCancellationProbe,
    ModelDispatchPreflight, ModelHealth, ModelHealthState, ModelLoadReceipt,
    ModelManifestObservation, ModelOperationControl, ModelProfileId, ModelResourceReport,
    ModelRunRequest, ModelRunResult, ModelRuntimeFailure, ModelRuntimeIdentity, ModelRuntimeKind,
    ModelServingCapabilities, ModelStreamSink, ModelUnloadReceipt, PlatformArchitecture,
    PlatformFamily, RuntimeIsolationObservation, TokenCountResult,
};

/// Driver operations available behind the Linux runtime adapter.
///
/// A driver has no workspace, tool, grant, credential, shell, destination, or
/// fallback input. Process and transport implementations remain platform-owned.
pub trait NativeModelDriver {
    /// Observes one exact manifest and artifact tuple without loading it.
    fn verify_manifest(
        &self,
        profile: &ExactModelProfile,
    ) -> Result<ModelManifestObservation, ModelRuntimeFailure>;

    /// Loads exactly one verified tuple.
    fn load(
        &mut self,
        profile: &ExactModelProfile,
        isolation: &RuntimeIsolationObservation,
    ) -> Result<ModelLoadReceipt, ModelRuntimeFailure>;

    /// Unloads exactly one named profile.
    fn unload(
        &mut self,
        profile_id: &ModelProfileId,
    ) -> Result<ModelUnloadReceipt, ModelRuntimeFailure>;

    /// Returns a content-free health observation.
    fn health(&self) -> ModelHealth;

    /// Re-observes the effective immutable serving tuple of the loaded process.
    fn serving_capabilities(&self) -> Result<ModelServingCapabilities, ModelRuntimeFailure>;

    /// Counts tokens for one exact bounded packet.
    fn count_tokens(
        &self,
        context: &EncodedModelContext,
    ) -> Result<TokenCountResult, ModelRuntimeFailure>;

    /// Streams inert response fragments for one exact request.
    fn stream(
        &mut self,
        request: &ModelRunRequest,
        context: &EncodedModelContext,
        preflight: &ModelDispatchPreflight,
        cancellation: Option<&dyn ModelCancellationProbe>,
        sink: &mut dyn ModelStreamSink,
    ) -> Result<ModelRunResult, ModelRuntimeFailure>;

    /// Returns content-free resource accounting.
    fn resources(&self) -> Result<ModelResourceReport, ModelRuntimeFailure>;
    /// Runs `verify_manifest` with borrowed run control; unsupported adapters refuse before work.
    fn verify_manifest_controlled(
        &self,
        _profile: &ExactModelProfile,
        control: &dyn ModelOperationControl,
    ) -> Result<ModelManifestObservation, ModelRuntimeFailure> {
        control.remaining_ms()?;
        Err(ModelRuntimeFailure {
            code: "model.operation.control-unsupported".to_owned(),
            retryable_after_correction: false,
            dependency_recovery_required: false,
            contract_error: None,
        })
    }

    /// Runs `load` with borrowed run control; unsupported adapters refuse before work.
    fn load_controlled(
        &mut self,
        _profile: &ExactModelProfile,
        _isolation: &RuntimeIsolationObservation,
        control: &dyn ModelOperationControl,
    ) -> Result<ModelLoadReceipt, ModelRuntimeFailure> {
        control.remaining_ms()?;
        Err(ModelRuntimeFailure {
            code: "model.operation.control-unsupported".to_owned(),
            retryable_after_correction: false,
            dependency_recovery_required: false,
            contract_error: None,
        })
    }

    /// Runs `health` with borrowed run control; unsupported adapters refuse before work.
    fn health_controlled(
        &self,
        control: &dyn ModelOperationControl,
    ) -> Result<ModelHealth, ModelRuntimeFailure> {
        control.remaining_ms()?;
        Err(ModelRuntimeFailure {
            code: "model.operation.control-unsupported".to_owned(),
            retryable_after_correction: false,
            dependency_recovery_required: false,
            contract_error: None,
        })
    }

    /// Runs `serving_capabilities` with borrowed run control; unsupported adapters refuse before work.
    fn serving_capabilities_controlled(
        &self,
        control: &dyn ModelOperationControl,
    ) -> Result<ModelServingCapabilities, ModelRuntimeFailure> {
        control.remaining_ms()?;
        Err(ModelRuntimeFailure {
            code: "model.operation.control-unsupported".to_owned(),
            retryable_after_correction: false,
            dependency_recovery_required: false,
            contract_error: None,
        })
    }

    /// Runs `count_tokens` with borrowed run control; unsupported adapters refuse before work.
    fn count_tokens_controlled(
        &self,
        _context: &EncodedModelContext,
        control: &dyn ModelOperationControl,
    ) -> Result<TokenCountResult, ModelRuntimeFailure> {
        control.remaining_ms()?;
        Err(ModelRuntimeFailure {
            code: "model.operation.control-unsupported".to_owned(),
            retryable_after_correction: false,
            dependency_recovery_required: false,
            contract_error: None,
        })
    }

    /// Runs `stream` with borrowed run control; unsupported adapters refuse before work.
    fn stream_controlled(
        &mut self,
        _request: &ModelRunRequest,
        _context: &EncodedModelContext,
        _preflight: &ModelDispatchPreflight,
        _cancellation: Option<&dyn ModelCancellationProbe>,
        _sink: &mut dyn ModelStreamSink,
        control: &dyn ModelOperationControl,
    ) -> Result<ModelRunResult, ModelRuntimeFailure> {
        control.remaining_ms()?;
        Err(ModelRuntimeFailure {
            code: "model.operation.control-unsupported".to_owned(),
            retryable_after_correction: false,
            dependency_recovery_required: false,
            contract_error: None,
        })
    }
}

/// One Linux native runtime bound to a pre-observed zero-authority boundary.
pub struct LinuxNativeModelAdapter<D: NativeModelDriver> {
    identity: ModelRuntimeIdentity,
    isolation: RuntimeIsolationObservation,
    driver: D,
    verified: Option<(ModelProfileId, String)>,
    loaded: Option<ModelProfileId>,
}

impl<D: NativeModelDriver> LinuxNativeModelAdapter<D> {
    /// Constructs an adapter only for an exact Fedora or Ubuntu native runtime
    /// and an isolation observation containing no reachable authority material.
    pub fn new(
        identity: ModelRuntimeIdentity,
        isolation: RuntimeIsolationObservation,
        driver: D,
    ) -> Result<Self, ModelRuntimeFailure> {
        if identity.kind != ModelRuntimeKind::NativeLlamaCpp
            || identity.architecture != PlatformArchitecture::X86_64
            || !matches!(
                identity.platform,
                PlatformFamily::Fedora | PlatformFamily::Ubuntu
            )
            || isolation.adapter_id != identity.adapter_id
            || isolation.network_available
            || isolation.workspace_available
            || isolation.authority_material_available
            || isolation.credential_material_available
            || !valid_sha256(&isolation.observation_sha256)
        {
            return Err(failure("model.linux-adapter.isolation-or-identity-invalid"));
        }
        Ok(Self {
            identity,
            isolation,
            driver,
            verified: None,
            loaded: None,
        })
    }

    fn exact_profile(&self, profile: &ExactModelProfile) -> bool {
        profile.runtime == self.identity
            && !profile.automatic_fallback
            && valid_sha256(&profile.manifest_sha256)
            && valid_sha256(&profile.artifact.sha256)
            && valid_sha256(&profile.codec.tokenizer_sha256)
            && valid_sha256(&profile.codec.template_sha256)
            && valid_sha256(&profile.codec.codec_sha256)
    }

    fn loaded_profile(&self, profile_id: &ModelProfileId) -> Result<(), ModelRuntimeFailure> {
        if self.loaded.as_ref() == Some(profile_id) {
            Ok(())
        } else {
            Err(failure("model.linux-adapter.profile-not-loaded"))
        }
    }

    fn exact_serving_capabilities(
        &self,
        profile: &ExactModelProfile,
        served: &ModelServingCapabilities,
    ) -> bool {
        served.schema_version == 1
            && served.profile_id == profile.profile_id
            && served.manifest_sha256 == profile.manifest_sha256
            && served.artifact_sha256 == profile.artifact.sha256
            && served.adapter_id == self.identity.adapter_id
            && served.runtime == self.identity
            && !served.endpoint.trim().is_empty()
            && served.process_id != 0
            && served.process_generation != 0
            && served.load_generation != 0
            && served.context_capacity_tokens >= profile.context.max_context_tokens
            && served.parallel_slots != 0
            && served.tokenizer_sha256 == profile.codec.tokenizer_sha256
            && served.template_sha256 == profile.codec.template_sha256
            && served.codec_sha256 == profile.codec.codec_sha256
            && served.reasoning_supported == profile.codec.reasoning_enabled
            && !served.context_shift_supported
            && valid_sha256(&served.launch_configuration_sha256)
            && valid_sha256(&served.observation_sha256)
    }
}

fn check_operation_control(
    control: Option<&dyn ModelOperationControl>,
) -> Result<(), ModelRuntimeFailure> {
    if let Some(control) = control {
        control.remaining_ms()?;
    }
    Ok(())
}

impl<D: NativeModelDriver> LinuxNativeModelAdapter<D> {
    fn verify_manifest_using(
        &self,
        profile: &ExactModelProfile,
        control: Option<&dyn ModelOperationControl>,
    ) -> Result<ModelManifestObservation, ModelRuntimeFailure> {
        check_operation_control(control)?;
        if !self.exact_profile(profile) || self.loaded.is_some() {
            return Err(failure("model.linux-adapter.profile-invalid"));
        }
        let observation = match control {
            Some(control) => self.driver.verify_manifest_controlled(profile, control)?,
            None => self.driver.verify_manifest(profile)?,
        };
        if observation.profile_id != profile.profile_id
            || observation.manifest_sha256 != profile.manifest_sha256
            || observation.artifact_sha256 != profile.artifact.sha256
            || observation.tokenizer_sha256 != profile.codec.tokenizer_sha256
            || observation.template_sha256 != profile.codec.template_sha256
            || observation.codec_sha256 != profile.codec.codec_sha256
            || observation.runtime != self.identity
        {
            return Err(failure("model.linux-adapter.manifest-drift"));
        }
        check_operation_control(control)?;
        Ok(observation)
    }

    fn load_using(
        &mut self,
        profile: &ExactModelProfile,
        control: Option<&dyn ModelOperationControl>,
    ) -> Result<ModelLoadReceipt, ModelRuntimeFailure> {
        check_operation_control(control)?;
        if self.loaded.is_some() {
            return Err(failure("model.linux-adapter.already-loaded"));
        }
        let observation = self.verify_manifest_using(profile, control)?;
        self.verified = Some((observation.profile_id, observation.manifest_sha256));
        let receipt = match control {
            Some(control) => self
                .driver
                .load_controlled(profile, &self.isolation, control)?,
            None => self.driver.load(profile, &self.isolation)?,
        };
        if receipt.profile_id != profile.profile_id
            || receipt.manifest_sha256 != profile.manifest_sha256
            || receipt.adapter_id != self.identity.adapter_id
            || receipt.isolation != self.isolation
            || !self.exact_serving_capabilities(profile, &receipt.served_capabilities)
        {
            self.verified = None;
            if control.is_some() {
                self.cleanup_failed_load(profile)?;
            } else {
                let _ = self.driver.unload(&profile.profile_id);
            }
            return Err(failure("model.linux-adapter.load-receipt-drift"));
        }
        if let Err(stop) = check_operation_control(control) {
            self.verified = None;
            self.cleanup_failed_load(profile)?;
            return Err(stop);
        }
        self.loaded = Some(profile.profile_id.clone());
        Ok(receipt)
    }

    fn cleanup_failed_load(
        &mut self,
        profile: &ExactModelProfile,
    ) -> Result<(), ModelRuntimeFailure> {
        let receipt = self
            .driver
            .unload(&profile.profile_id)
            .map_err(|_| failure("model.operation.cleanup-uncertain"))?;
        if receipt.profile_id != profile.profile_id
            || receipt.adapter_id != self.identity.adapter_id
            || !receipt.empty
        {
            return Err(failure("model.operation.cleanup-uncertain"));
        }
        Ok(())
    }

    fn serving_capabilities_using(
        &self,
        control: Option<&dyn ModelOperationControl>,
    ) -> Result<ModelServingCapabilities, ModelRuntimeFailure> {
        check_operation_control(control)?;
        let profile_id = self
            .loaded
            .as_ref()
            .ok_or_else(|| failure("model.served-capability.missing"))?;
        let served = match control {
            Some(control) => self.driver.serving_capabilities_controlled(control)?,
            None => self.driver.serving_capabilities()?,
        };
        if served.adapter_id != self.identity.adapter_id || served.profile_id != *profile_id {
            return Err(failure("model.served-capability.drift"));
        }
        check_operation_control(control)?;
        Ok(served)
    }

    fn count_tokens_using(
        &self,
        context: &EncodedModelContext,
        control: Option<&dyn ModelOperationControl>,
    ) -> Result<TokenCountResult, ModelRuntimeFailure> {
        check_operation_control(control)?;
        self.loaded_profile(&context.profile_id)?;
        let result = match control {
            Some(control) => self.driver.count_tokens_controlled(context, control)?,
            None => self.driver.count_tokens(context)?,
        };
        check_operation_control(control)?;
        Ok(result)
    }

    fn stream_using(
        &mut self,
        request: &ModelRunRequest,
        context: &EncodedModelContext,
        preflight: &ModelDispatchPreflight,
        cancellation: Option<&dyn ModelCancellationProbe>,
        sink: &mut dyn ModelStreamSink,
        control: Option<&dyn ModelOperationControl>,
    ) -> Result<ModelRunResult, ModelRuntimeFailure> {
        check_operation_control(control)?;
        self.loaded_profile(&request.profile_id)?;
        if request.profile_id != context.profile_id
            || request.context_packet_id != context.context_packet_id
            || request.adapter_id != self.identity.adapter_id
            || self.verified.as_ref()
                != Some(&(request.profile_id.clone(), request.manifest_sha256.clone()))
        {
            return Err(failure("model.linux-adapter.request-drift"));
        }
        match control {
            Some(control) => self.driver.stream_controlled(
                request,
                context,
                preflight,
                cancellation,
                sink,
                control,
            ),
            None => self
                .driver
                .stream(request, context, preflight, cancellation, sink),
        }
    }
}

impl<D: NativeModelDriver> LocalModelRuntime for LinuxNativeModelAdapter<D> {
    fn identity(&self) -> &ModelRuntimeIdentity {
        &self.identity
    }

    fn verify_manifest(
        &self,
        profile: &ExactModelProfile,
    ) -> Result<ModelManifestObservation, ModelRuntimeFailure> {
        self.verify_manifest_using(profile, None)
    }

    fn verify_manifest_controlled(
        &self,
        profile: &ExactModelProfile,
        control: &dyn ModelOperationControl,
    ) -> Result<ModelManifestObservation, ModelRuntimeFailure> {
        self.verify_manifest_using(profile, Some(control))
    }

    fn load(
        &mut self,
        profile: &ExactModelProfile,
    ) -> Result<ModelLoadReceipt, ModelRuntimeFailure> {
        self.load_using(profile, None)
    }

    fn load_controlled(
        &mut self,
        profile: &ExactModelProfile,
        control: &dyn ModelOperationControl,
    ) -> Result<ModelLoadReceipt, ModelRuntimeFailure> {
        self.load_using(profile, Some(control))
    }

    fn unload(
        &mut self,
        profile_id: &ModelProfileId,
    ) -> Result<ModelUnloadReceipt, ModelRuntimeFailure> {
        self.loaded_profile(profile_id)?;
        let receipt = self.driver.unload(profile_id)?;
        if receipt.profile_id != *profile_id
            || receipt.adapter_id != self.identity.adapter_id
            || !receipt.empty
        {
            return Err(failure("model.linux-adapter.unload-receipt-drift"));
        }
        self.loaded = None;
        self.verified = None;
        Ok(receipt)
    }

    fn health(&self) -> ModelHealth {
        let health = self.driver.health();
        let capabilities_valid = self.loaded.as_ref().is_none_or(|_| {
            self.driver
                .serving_capabilities()
                .ok()
                .is_some_and(|served| {
                    served.adapter_id == self.identity.adapter_id
                        && health.profile_id.as_ref() == Some(&served.profile_id)
                        && valid_sha256(&served.observation_sha256)
                })
        });
        let valid = health.adapter_id == self.identity.adapter_id
            && health.profile_id == self.loaded
            && ((self.loaded.is_some() && health.state == ModelHealthState::Ready)
                || (self.loaded.is_none() && health.state == ModelHealthState::Unloaded))
            && capabilities_valid;
        if valid {
            health
        } else {
            ModelHealth {
                adapter_id: self.identity.adapter_id.clone(),
                profile_id: self.loaded.clone(),
                state: ModelHealthState::Failed,
                reason_code: "model.linux-adapter.health-drift".to_owned(),
                observed_at_ms: health.observed_at_ms,
            }
        }
    }

    fn health_controlled(
        &self,
        control: &dyn ModelOperationControl,
    ) -> Result<ModelHealth, ModelRuntimeFailure> {
        control.remaining_ms()?;
        let health = self.driver.health_controlled(control)?;
        let capabilities_valid = if self.loaded.is_some() {
            let served = self.driver.serving_capabilities_controlled(control)?;
            served.adapter_id == self.identity.adapter_id
                && health.profile_id.as_ref() == Some(&served.profile_id)
                && valid_sha256(&served.observation_sha256)
        } else {
            true
        };
        control.remaining_ms()?;
        let valid = health.adapter_id == self.identity.adapter_id
            && health.profile_id == self.loaded
            && ((self.loaded.is_some() && health.state == ModelHealthState::Ready)
                || (self.loaded.is_none() && health.state == ModelHealthState::Unloaded))
            && capabilities_valid;
        Ok(if valid {
            health
        } else {
            ModelHealth {
                adapter_id: self.identity.adapter_id.clone(),
                profile_id: self.loaded.clone(),
                state: ModelHealthState::Failed,
                reason_code: "model.linux-adapter.health-drift".to_owned(),
                observed_at_ms: health.observed_at_ms,
            }
        })
    }

    fn serving_capabilities(&self) -> Result<ModelServingCapabilities, ModelRuntimeFailure> {
        self.serving_capabilities_using(None)
    }

    fn serving_capabilities_controlled(
        &self,
        control: &dyn ModelOperationControl,
    ) -> Result<ModelServingCapabilities, ModelRuntimeFailure> {
        self.serving_capabilities_using(Some(control))
    }

    fn count_tokens(
        &self,
        context: &EncodedModelContext,
    ) -> Result<TokenCountResult, ModelRuntimeFailure> {
        self.count_tokens_using(context, None)
    }

    fn count_tokens_controlled(
        &self,
        context: &EncodedModelContext,
        control: &dyn ModelOperationControl,
    ) -> Result<TokenCountResult, ModelRuntimeFailure> {
        self.count_tokens_using(context, Some(control))
    }

    fn stream(
        &mut self,
        request: &ModelRunRequest,
        context: &EncodedModelContext,
        preflight: &ModelDispatchPreflight,
        cancellation: Option<&dyn ModelCancellationProbe>,
        sink: &mut dyn ModelStreamSink,
    ) -> Result<ModelRunResult, ModelRuntimeFailure> {
        self.stream_using(request, context, preflight, cancellation, sink, None)
    }

    fn stream_controlled(
        &mut self,
        request: &ModelRunRequest,
        context: &EncodedModelContext,
        preflight: &ModelDispatchPreflight,
        cancellation: Option<&dyn ModelCancellationProbe>,
        sink: &mut dyn ModelStreamSink,
        control: &dyn ModelOperationControl,
    ) -> Result<ModelRunResult, ModelRuntimeFailure> {
        self.stream_using(
            request,
            context,
            preflight,
            cancellation,
            sink,
            Some(control),
        )
    }

    fn resources(&self) -> Result<ModelResourceReport, ModelRuntimeFailure> {
        let profile_id = self
            .loaded
            .as_ref()
            .ok_or_else(|| failure("model.linux-adapter.profile-not-loaded"))?;
        let report = self.driver.resources()?;
        if report.adapter_id != self.identity.adapter_id || report.profile_id != *profile_id {
            return Err(failure("model.linux-adapter.resource-drift"));
        }
        Ok(report)
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn failure(code: &str) -> ModelRuntimeFailure {
    ModelRuntimeFailure {
        code: code.to_owned(),
        retryable_after_correction: false,
        dependency_recovery_required: code == "model.operation.cleanup-uncertain",
        contract_error: None,
    }
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        EncodedModelContext, ExactModelProfile, LocalModelRuntime, ModelHealth, ModelHealthState,
        ModelLoadReceipt, ModelManifestObservation, ModelProfileId, ModelResourceReport,
        ModelRuntimeFailure, ModelServingCachePolicy, ModelServingCapabilities, ModelStreamSink,
        ModelUnloadReceipt, RuntimeIsolationObservation, TokenCountResult,
    };

    use super::{LinuxNativeModelAdapter, NativeModelDriver, failure};

    struct FakeDriver {
        profile: ExactModelProfile,
        loaded: bool,
        drift_manifest: bool,
        drift_served: bool,
    }

    impl NativeModelDriver for FakeDriver {
        fn verify_manifest(
            &self,
            profile: &ExactModelProfile,
        ) -> Result<ModelManifestObservation, ModelRuntimeFailure> {
            Ok(ModelManifestObservation {
                profile_id: profile.profile_id.clone(),
                manifest_sha256: if self.drift_manifest {
                    "0".repeat(64)
                } else {
                    profile.manifest_sha256.clone()
                },
                artifact_sha256: profile.artifact.sha256.clone(),
                tokenizer_sha256: profile.codec.tokenizer_sha256.clone(),
                template_sha256: profile.codec.template_sha256.clone(),
                codec_sha256: profile.codec.codec_sha256.clone(),
                runtime: profile.runtime.clone(),
            })
        }

        fn load(
            &mut self,
            profile: &ExactModelProfile,
            isolation: &RuntimeIsolationObservation,
        ) -> Result<ModelLoadReceipt, ModelRuntimeFailure> {
            self.loaded = true;
            let mut served_capabilities = served(profile);
            if self.drift_served {
                served_capabilities.context_capacity_tokens -= 1;
            }
            Ok(ModelLoadReceipt {
                profile_id: profile.profile_id.clone(),
                manifest_sha256: profile.manifest_sha256.clone(),
                adapter_id: profile.runtime.adapter_id.clone(),
                elapsed_ms: 1,
                isolation: isolation.clone(),
                served_capabilities,
            })
        }

        fn unload(
            &mut self,
            profile_id: &ModelProfileId,
        ) -> Result<ModelUnloadReceipt, ModelRuntimeFailure> {
            self.loaded = false;
            Ok(ModelUnloadReceipt {
                profile_id: profile_id.clone(),
                adapter_id: self.profile.runtime.adapter_id.clone(),
                empty: true,
                elapsed_ms: 1,
            })
        }

        fn health(&self) -> ModelHealth {
            ModelHealth {
                adapter_id: self.profile.runtime.adapter_id.clone(),
                profile_id: self.loaded.then(|| self.profile.profile_id.clone()),
                state: if self.loaded {
                    ModelHealthState::Ready
                } else {
                    ModelHealthState::Unloaded
                },
                reason_code: "fixture".to_owned(),
                observed_at_ms: 1,
            }
        }

        fn serving_capabilities(&self) -> Result<ModelServingCapabilities, ModelRuntimeFailure> {
            if self.loaded {
                let mut value = served(&self.profile);
                if self.drift_served {
                    value.context_capacity_tokens -= 1;
                }
                Ok(value)
            } else {
                Err(failure("model.served-capability.missing"))
            }
        }

        fn count_tokens(
            &self,
            _context: &EncodedModelContext,
        ) -> Result<TokenCountResult, ModelRuntimeFailure> {
            Err(failure("fixture.not-used"))
        }

        fn stream(
            &mut self,
            _request: &agentmage_kernel_contracts::ModelRunRequest,
            _context: &EncodedModelContext,
            _preflight: &agentmage_kernel_contracts::ModelDispatchPreflight,
            _cancellation: Option<&dyn agentmage_kernel_contracts::ModelCancellationProbe>,
            _sink: &mut dyn ModelStreamSink,
        ) -> Result<agentmage_kernel_contracts::ModelRunResult, ModelRuntimeFailure> {
            Err(failure("fixture.not-used"))
        }

        fn resources(&self) -> Result<ModelResourceReport, ModelRuntimeFailure> {
            Err(failure("fixture.not-used"))
        }
    }

    fn served(profile: &ExactModelProfile) -> ModelServingCapabilities {
        ModelServingCapabilities {
            schema_version: 1,
            profile_id: profile.profile_id.clone(),
            manifest_sha256: profile.manifest_sha256.clone(),
            artifact_sha256: profile.artifact.sha256.clone(),
            adapter_id: profile.runtime.adapter_id.clone(),
            runtime: profile.runtime.clone(),
            endpoint: "fixture://native-model".to_owned(),
            process_id: 1,
            process_generation: 1,
            load_generation: 1,
            launch_configuration_sha256: "a".repeat(64),
            context_capacity_tokens: profile.context.max_context_tokens,
            parallel_slots: 1,
            cache_policy: ModelServingCachePolicy::Disabled,
            tokenizer_sha256: profile.codec.tokenizer_sha256.clone(),
            template_sha256: profile.codec.template_sha256.clone(),
            codec_sha256: profile.codec.codec_sha256.clone(),
            reasoning_supported: profile.codec.reasoning_enabled,
            context_shift_supported: false,
            observed_at_ms: 1,
            observation_sha256: "a".repeat(64),
        }
    }

    fn profile() -> ExactModelProfile {
        let catalog: serde_json::Value = serde_json::from_str(include_str!(
            "../../../model-profiles/exact-profile-catalog.json"
        ))
        .expect("catalog");
        serde_json::from_value(
            catalog["profiles"]
                .as_array()
                .expect("profiles")
                .iter()
                .find(|profile| {
                    profile["profile_id"]
                        == "muse-glimmer-30b-q4-k-m-text-8k-fedora-first-party-quality"
                })
                .expect("Muse profile")
                .clone(),
        )
        .expect("exact profile")
    }

    fn isolation(profile: &ExactModelProfile) -> RuntimeIsolationObservation {
        RuntimeIsolationObservation {
            adapter_id: profile.runtime.adapter_id.clone(),
            profile_id: profile.profile_id.clone(),
            network_available: false,
            workspace_available: false,
            authority_material_available: false,
            credential_material_available: false,
            observation_sha256: "a".repeat(64),
        }
    }

    fn adapter(drift_manifest: bool) -> LinuxNativeModelAdapter<FakeDriver> {
        let profile = profile();
        LinuxNativeModelAdapter::new(
            profile.runtime.clone(),
            isolation(&profile),
            FakeDriver {
                profile,
                loaded: false,
                drift_manifest,
                drift_served: false,
            },
        )
        .expect("adapter")
    }

    #[test]
    fn exact_profile_verifies_loads_reports_health_and_unloads() {
        let profile = profile();
        let mut adapter = adapter(false);
        assert_eq!(
            adapter
                .verify_manifest(&profile)
                .expect("manifest")
                .profile_id,
            profile.profile_id
        );
        assert_eq!(
            adapter.load(&profile).expect("load").isolation,
            isolation(&profile)
        );
        assert_eq!(adapter.health().state, ModelHealthState::Ready);
        assert!(adapter.unload(&profile.profile_id).expect("unload").empty);
        assert_eq!(adapter.health().state, ModelHealthState::Unloaded);
    }

    #[test]
    fn isolation_authority_and_manifest_drift_fail_closed() {
        let profile = profile();
        for mutate in [
            |value: &mut RuntimeIsolationObservation| value.network_available = true,
            |value: &mut RuntimeIsolationObservation| value.workspace_available = true,
            |value: &mut RuntimeIsolationObservation| value.authority_material_available = true,
            |value: &mut RuntimeIsolationObservation| value.credential_material_available = true,
        ] {
            let mut changed = isolation(&profile);
            mutate(&mut changed);
            assert!(
                LinuxNativeModelAdapter::new(
                    profile.runtime.clone(),
                    changed,
                    FakeDriver {
                        profile: profile.clone(),
                        loaded: false,
                        drift_manifest: false,
                        drift_served: false,
                    },
                )
                .is_err()
            );
        }
        assert!(adapter(true).verify_manifest(&profile).is_err());
    }

    #[test]
    fn changed_fallback_hash_and_foreign_profiles_fail_before_driver_load() {
        for mutate in [
            |value: &mut ExactModelProfile| value.automatic_fallback = true,
            |value: &mut ExactModelProfile| value.runtime.runtime_sha256 = "0".repeat(64),
            |value: &mut ExactModelProfile| value.artifact.sha256 = "invalid".to_owned(),
        ] {
            let mut changed = profile();
            mutate(&mut changed);
            assert!(adapter(false).load(&changed).is_err());
        }
    }

    #[test]
    fn undersized_served_capacity_refuses_and_retains_no_loaded_binding() {
        let profile = profile();
        let mut adapter = LinuxNativeModelAdapter::new(
            profile.runtime.clone(),
            isolation(&profile),
            FakeDriver {
                profile: profile.clone(),
                loaded: false,
                drift_manifest: false,
                drift_served: true,
            },
        )
        .expect("adapter");
        assert_eq!(
            adapter.load(&profile).expect_err("undersized refusal").code,
            "model.linux-adapter.load-receipt-drift"
        );
        assert_eq!(adapter.health().state, ModelHealthState::Unloaded);
    }
    mod controlled_preparation_tests {
        use super::*;
        use agentmage_kernel_contracts::{ModelOperationControl, ModelOperationStop};
        use std::cell::Cell;
        use std::rc::Rc;

        #[derive(Clone)]
        struct Control(Rc<Cell<bool>>);
        impl ModelOperationControl for Control {
            fn remaining_ms(&self) -> Result<std::num::NonZeroU64, ModelOperationStop> {
                if self.0.get() {
                    Err(ModelOperationStop::Cancelled)
                } else {
                    Ok(std::num::NonZeroU64::new(100).unwrap())
                }
            }
        }
        struct ControlledDriver {
            inner: FakeDriver,
            control: Control,
            cancel_after_load: bool,
            cleanup_fails: bool,
            cleanup_receipt_mutation: u8,
            loads: usize,
            unloads: usize,
        }
        impl NativeModelDriver for ControlledDriver {
            fn verify_manifest(
                &self,
                _: &ExactModelProfile,
            ) -> Result<ModelManifestObservation, ModelRuntimeFailure> {
                panic!("legacy manifest")
            }
            fn load(
                &mut self,
                _: &ExactModelProfile,
                _: &RuntimeIsolationObservation,
            ) -> Result<ModelLoadReceipt, ModelRuntimeFailure> {
                panic!("legacy load")
            }
            fn health(&self) -> ModelHealth {
                panic!("legacy health")
            }
            fn serving_capabilities(
                &self,
            ) -> Result<ModelServingCapabilities, ModelRuntimeFailure> {
                panic!("legacy capabilities")
            }
            fn count_tokens(
                &self,
                _: &EncodedModelContext,
            ) -> Result<TokenCountResult, ModelRuntimeFailure> {
                panic!("legacy tokens")
            }
            fn stream(
                &mut self,
                _: &agentmage_kernel_contracts::ModelRunRequest,
                _: &EncodedModelContext,
                _: &agentmage_kernel_contracts::ModelDispatchPreflight,
                _: Option<&dyn agentmage_kernel_contracts::ModelCancellationProbe>,
                _: &mut dyn ModelStreamSink,
            ) -> Result<agentmage_kernel_contracts::ModelRunResult, ModelRuntimeFailure>
            {
                panic!("legacy stream")
            }
            fn resources(&self) -> Result<ModelResourceReport, ModelRuntimeFailure> {
                self.inner.resources()
            }
            fn unload(
                &mut self,
                profile: &ModelProfileId,
            ) -> Result<ModelUnloadReceipt, ModelRuntimeFailure> {
                self.unloads += 1;
                if self.cleanup_fails {
                    Err(failure("fixture.cleanup-uncertain"))
                } else {
                    let mut receipt = self.inner.unload(profile)?;
                    match self.cleanup_receipt_mutation {
                        1 => receipt.empty = false,
                        2 => {
                            receipt.profile_id = ModelProfileId::from_raw("foreign-cleanup-profile")
                        }
                        3 => {
                            receipt.adapter_id =
                                agentmage_kernel_contracts::ModelAdapterId::from_raw(
                                    "foreign-cleanup-adapter",
                                )
                        }
                        _ => {}
                    }
                    Ok(receipt)
                }
            }
            fn verify_manifest_controlled(
                &self,
                profile: &ExactModelProfile,
                control: &dyn ModelOperationControl,
            ) -> Result<ModelManifestObservation, ModelRuntimeFailure> {
                control.remaining_ms()?;
                self.inner.verify_manifest(profile)
            }
            fn load_controlled(
                &mut self,
                profile: &ExactModelProfile,
                isolation: &RuntimeIsolationObservation,
                control: &dyn ModelOperationControl,
            ) -> Result<ModelLoadReceipt, ModelRuntimeFailure> {
                control.remaining_ms()?;
                self.loads += 1;
                let result = self.inner.load(profile, isolation);
                if self.cancel_after_load {
                    self.control.0.set(true);
                }
                result
            }
            fn health_controlled(
                &self,
                control: &dyn ModelOperationControl,
            ) -> Result<ModelHealth, ModelRuntimeFailure> {
                control.remaining_ms()?;
                Ok(self.inner.health())
            }
            fn serving_capabilities_controlled(
                &self,
                control: &dyn ModelOperationControl,
            ) -> Result<ModelServingCapabilities, ModelRuntimeFailure> {
                control.remaining_ms()?;
                self.inner.serving_capabilities()
            }
        }
        fn fixture(
            drift: bool,
            cancel: bool,
            cleanup_fails: bool,
        ) -> (LinuxNativeModelAdapter<ControlledDriver>, Control) {
            let profile = profile();
            let control = Control(Rc::new(Cell::new(false)));
            let driver = ControlledDriver {
                inner: FakeDriver {
                    profile: profile.clone(),
                    loaded: false,
                    drift_manifest: false,
                    drift_served: drift,
                },
                control: control.clone(),
                cancel_after_load: cancel,
                cleanup_fails,
                cleanup_receipt_mutation: 0,
                loads: 0,
                unloads: 0,
            };
            (
                LinuxNativeModelAdapter::new(profile.runtime.clone(), isolation(&profile), driver)
                    .unwrap(),
                control,
            )
        }

        #[test]
        fn native_adapter_controlled_defaults_never_fall_through_to_legacy_driver_load() {
            let mut adapter = adapter(false);
            let control = Control(Rc::new(Cell::new(false)));
            assert_eq!(
                adapter
                    .verify_manifest_controlled(&profile(), &control)
                    .unwrap_err()
                    .code,
                "model.operation.control-unsupported"
            );
            assert_eq!(
                adapter
                    .load_controlled(&profile(), &control)
                    .unwrap_err()
                    .code,
                "model.operation.control-unsupported"
            );
            assert!(!adapter.driver.loaded);
            assert!(adapter.loaded.is_none());
        }

        #[test]
        fn native_adapter_controlled_load_keeps_exact_receipt_and_serving_checks() {
            let (mut adapter, control) = fixture(false, false, false);
            let receipt = adapter.load_controlled(&profile(), &control).unwrap();
            assert_eq!(receipt.isolation, isolation(&profile()));
            assert_eq!(
                adapter.health_controlled(&control).unwrap().state,
                ModelHealthState::Ready
            );
            assert_eq!(
                adapter.serving_capabilities_controlled(&control).unwrap(),
                receipt.served_capabilities
            );
            assert_eq!(adapter.driver.loads, 1);
            assert!(adapter.unload(&profile().profile_id).unwrap().empty);
        }

        #[test]
        fn native_adapter_cancelled_or_invalid_load_preserves_cleanup_priority() {
            for drift in [false, true] {
                for cleanup_fails in [false, true] {
                    let (mut adapter, control) = fixture(drift, !drift, cleanup_fails);
                    let error = adapter.load_controlled(&profile(), &control).unwrap_err();
                    let expected = if cleanup_fails {
                        "model.operation.cleanup-uncertain"
                    } else if drift {
                        "model.linux-adapter.load-receipt-drift"
                    } else {
                        "model.operation.cancelled"
                    };
                    assert_eq!(error.code, expected);
                    assert_eq!(adapter.driver.loads, 1);
                    assert_eq!(adapter.driver.unloads, 1);
                    assert_eq!(adapter.driver.inner.loaded, cleanup_fails);
                    assert!(adapter.loaded.is_none());
                    assert!(adapter.verified.is_none());
                }
            }
        }
        #[test]
        fn native_adapter_failed_load_requires_an_exact_empty_unload_receipt() {
            for drift in [false, true] {
                for mutation in 1..=3 {
                    let (mut adapter, control) = fixture(drift, !drift, false);
                    adapter.driver.cleanup_receipt_mutation = mutation;
                    let error = adapter.load_controlled(&profile(), &control).unwrap_err();
                    assert_eq!(
                        error.code, "model.operation.cleanup-uncertain",
                        "drift={drift} mutation={mutation}"
                    );
                    assert!(error.dependency_recovery_required);
                    assert!(!error.retryable_after_correction);
                    assert_eq!(adapter.driver.loads, 1);
                    assert_eq!(adapter.driver.unloads, 1);
                    assert!(adapter.loaded.is_none());
                    assert!(adapter.verified.is_none());
                }
            }
        }
    }
}
