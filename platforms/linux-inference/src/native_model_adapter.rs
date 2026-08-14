//! Candidate-neutral Linux `LocalModelRuntime` adapter boundary.

use agentmage_kernel_contracts::{
    EncodedModelContext, ExactModelProfile, LocalModelRuntime, ModelCancellationProbe, ModelHealth,
    ModelHealthState, ModelLoadReceipt, ModelManifestObservation, ModelProfileId,
    ModelResourceReport, ModelRunRequest, ModelRunResult, ModelRuntimeFailure,
    ModelRuntimeIdentity, ModelRuntimeKind, ModelStreamSink, ModelUnloadReceipt,
    PlatformArchitecture, PlatformFamily, RuntimeIsolationObservation, TokenCountResult,
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
        cancellation: Option<&dyn ModelCancellationProbe>,
        sink: &mut dyn ModelStreamSink,
    ) -> Result<ModelRunResult, ModelRuntimeFailure>;

    /// Returns content-free resource accounting.
    fn resources(&self) -> Result<ModelResourceReport, ModelRuntimeFailure>;
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
}

impl<D: NativeModelDriver> LocalModelRuntime for LinuxNativeModelAdapter<D> {
    fn identity(&self) -> &ModelRuntimeIdentity {
        &self.identity
    }

    fn verify_manifest(
        &self,
        profile: &ExactModelProfile,
    ) -> Result<ModelManifestObservation, ModelRuntimeFailure> {
        if !self.exact_profile(profile) || self.loaded.is_some() {
            return Err(failure("model.linux-adapter.profile-invalid"));
        }
        let observation = self.driver.verify_manifest(profile)?;
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
        Ok(observation)
    }

    fn load(
        &mut self,
        profile: &ExactModelProfile,
    ) -> Result<ModelLoadReceipt, ModelRuntimeFailure> {
        if self.loaded.is_some() {
            return Err(failure("model.linux-adapter.already-loaded"));
        }
        let observation = self.verify_manifest(profile)?;
        self.verified = Some((observation.profile_id, observation.manifest_sha256));
        let receipt = self.driver.load(profile, &self.isolation)?;
        if receipt.profile_id != profile.profile_id
            || receipt.manifest_sha256 != profile.manifest_sha256
            || receipt.adapter_id != self.identity.adapter_id
            || receipt.isolation != self.isolation
        {
            self.verified = None;
            return Err(failure("model.linux-adapter.load-receipt-drift"));
        }
        self.loaded = Some(profile.profile_id.clone());
        Ok(receipt)
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
        let valid = health.adapter_id == self.identity.adapter_id
            && health.profile_id == self.loaded
            && ((self.loaded.is_some() && health.state == ModelHealthState::Ready)
                || (self.loaded.is_none() && health.state == ModelHealthState::Unloaded));
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

    fn count_tokens(
        &self,
        context: &EncodedModelContext,
    ) -> Result<TokenCountResult, ModelRuntimeFailure> {
        self.loaded_profile(&context.profile_id)?;
        self.driver.count_tokens(context)
    }

    fn stream(
        &mut self,
        request: &ModelRunRequest,
        context: &EncodedModelContext,
        cancellation: Option<&dyn ModelCancellationProbe>,
        sink: &mut dyn ModelStreamSink,
    ) -> Result<ModelRunResult, ModelRuntimeFailure> {
        self.loaded_profile(&request.profile_id)?;
        if request.profile_id != context.profile_id
            || request.context_packet_id != context.context_packet_id
            || request.adapter_id != self.identity.adapter_id
            || self.verified.as_ref()
                != Some(&(request.profile_id.clone(), request.manifest_sha256.clone()))
        {
            return Err(failure("model.linux-adapter.request-drift"));
        }
        self.driver.stream(request, context, cancellation, sink)
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
        dependency_recovery_required: false,
        contract_error: None,
    }
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        EncodedModelContext, ExactModelProfile, LocalModelRuntime, ModelHealth, ModelHealthState,
        ModelLoadReceipt, ModelManifestObservation, ModelProfileId, ModelResourceReport,
        ModelRuntimeFailure, ModelStreamSink, ModelUnloadReceipt, RuntimeIsolationObservation,
        TokenCountResult,
    };

    use super::{LinuxNativeModelAdapter, NativeModelDriver, failure};

    struct FakeDriver {
        profile: ExactModelProfile,
        loaded: bool,
        drift_manifest: bool,
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
            Ok(ModelLoadReceipt {
                profile_id: profile.profile_id.clone(),
                manifest_sha256: profile.manifest_sha256.clone(),
                adapter_id: profile.runtime.adapter_id.clone(),
                elapsed_ms: 1,
                isolation: isolation.clone(),
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
            _cancellation: Option<&dyn agentmage_kernel_contracts::ModelCancellationProbe>,
            _sink: &mut dyn ModelStreamSink,
        ) -> Result<agentmage_kernel_contracts::ModelRunResult, ModelRuntimeFailure> {
            Err(failure("fixture.not-used"))
        }

        fn resources(&self) -> Result<ModelResourceReport, ModelRuntimeFailure> {
            Err(failure("fixture.not-used"))
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
}
