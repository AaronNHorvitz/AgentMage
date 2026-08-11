use agentmage_kernel_contracts::{
    PLATFORM_ADAPTER_API_VERSION, PlatformAdapter, PlatformArchitecture, PlatformCapability,
    PlatformCapabilityObservation, PlatformCapabilityStatus, PlatformFamily,
    PlatformManifestIdentity, PlatformRuntimeIdentity, PlatformStartupError,
    REQUIRED_PLATFORM_CAPABILITIES,
};
use agentmage_kernel_engine::platform_startup::activate_platform;

#[derive(Debug)]
struct ManifestBackedAdapter {
    manifest: PlatformManifestIdentity,
    runtime: PlatformRuntimeIdentity,
}

impl ManifestBackedAdapter {
    fn new(family: PlatformFamily) -> Self {
        let runtime = PlatformRuntimeIdentity::new(
            family,
            PlatformArchitecture::X86_64,
            [1; 32],
            [2; 32],
            [3; 32],
            [4; 32],
        );
        Self {
            manifest: PlatformManifestIdentity::new(
                PLATFORM_ADAPTER_API_VERSION,
                manifest_digest(family),
                runtime.clone(),
            ),
            runtime,
        }
    }
}

impl PlatformAdapter for ManifestBackedAdapter {
    fn manifest_identity(&self) -> &PlatformManifestIdentity {
        &self.manifest
    }

    fn runtime_identity(&self) -> Result<PlatformRuntimeIdentity, PlatformStartupError> {
        Ok(self.runtime.clone())
    }

    fn probe_capability(&self, capability: PlatformCapability) -> PlatformCapabilityObservation {
        PlatformCapabilityObservation::new(
            capability,
            PlatformCapabilityStatus::Verified,
            self.runtime.family(),
            *self.manifest.manifest_sha256(),
            [capability as u8 + 1; 32],
        )
    }
}

const fn manifest_digest(family: PlatformFamily) -> [u8; 32] {
    match family {
        PlatformFamily::DeterministicFake => [7; 32],
        PlatformFamily::Fedora => [8; 32],
        PlatformFamily::Ubuntu => [9; 32],
        PlatformFamily::MacOsAppleSilicon => [10; 32],
    }
}

#[test]
fn fake_fedora_and_ubuntu_use_identical_contract_semantics() {
    for family in [
        PlatformFamily::DeterministicFake,
        PlatformFamily::Fedora,
        PlatformFamily::Ubuntu,
    ] {
        let verified = activate_platform(ManifestBackedAdapter::new(family))
            .expect("available adapter contract");
        assert_eq!(verified.manifest_identity().target().family(), family);
        assert_eq!(
            verified.capability_observations().len(),
            REQUIRED_PLATFORM_CAPABILITIES.len()
        );
        for (expected, observation) in REQUIRED_PLATFORM_CAPABILITIES
            .iter()
            .zip(verified.capability_observations())
        {
            assert_eq!(observation.capability(), *expected);
            assert_eq!(observation.status(), PlatformCapabilityStatus::Verified);
            assert_eq!(observation.platform(), family);
        }
    }
}

#[test]
fn platform_manifest_identity_is_never_portable_between_adapters() {
    let fedora = ManifestBackedAdapter::new(PlatformFamily::Fedora);
    let ubuntu = ManifestBackedAdapter::new(PlatformFamily::Ubuntu);
    assert_ne!(
        fedora.manifest_identity().manifest_sha256(),
        ubuntu.manifest_identity().manifest_sha256()
    );
    assert_ne!(
        fedora.manifest_identity().target().family(),
        ubuntu.manifest_identity().target().family()
    );
}
