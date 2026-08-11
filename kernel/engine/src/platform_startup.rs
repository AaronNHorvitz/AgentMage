//! Fail-closed platform-adapter selection before workspace access.

use agentmage_kernel_contracts::{
    PLATFORM_ADAPTER_API_VERSION, PlatformAdapter, PlatformCapability,
    PlatformCapabilityObservation, PlatformCapabilityStatus, PlatformManifestIdentity,
    PlatformStartupError, PlatformStartupErrorKind, REQUIRED_PLATFORM_CAPABILITIES,
};

/// Adapter wrapper constructible only after exact runtime and capability validation.
#[derive(Debug)]
pub struct VerifiedPlatformAdapter<A: PlatformAdapter> {
    adapter: A,
    observations: [PlatformCapabilityObservation; REQUIRED_PLATFORM_CAPABILITIES.len()],
}

impl<A: PlatformAdapter> VerifiedPlatformAdapter<A> {
    /// Returns the verified adapter for later capability-specific dispatch.
    #[must_use]
    pub const fn adapter(&self) -> &A {
        &self.adapter
    }

    /// Returns the immutable manifest selected at startup.
    #[must_use]
    pub fn manifest_identity(&self) -> &PlatformManifestIdentity {
        self.adapter.manifest_identity()
    }

    /// Returns exact bounded evidence for every required startup capability.
    #[must_use]
    pub const fn capability_observations(
        &self,
    ) -> &[PlatformCapabilityObservation; REQUIRED_PLATFORM_CAPABILITIES.len()] {
        &self.observations
    }
}

/// Validates one adapter against its manifest and every required capability.
///
/// The routine contains no operating-system branch. Native differences remain behind
/// the adapter's runtime-identity and capability-probe methods.
pub fn activate_platform<A: PlatformAdapter>(
    adapter: A,
) -> Result<VerifiedPlatformAdapter<A>, PlatformStartupError> {
    let manifest = adapter.manifest_identity();
    if manifest.api_version() != PLATFORM_ADAPTER_API_VERSION {
        return Err(startup_error(
            PlatformStartupErrorKind::ApiVersionMismatch,
            None,
        ));
    }

    let observed = adapter.runtime_identity()?;
    let target = manifest.target();
    if observed.family() != target.family() {
        return Err(startup_error(
            PlatformStartupErrorKind::PlatformMismatch,
            None,
        ));
    }
    if observed.architecture() != target.architecture() {
        return Err(startup_error(
            PlatformStartupErrorKind::ArchitectureMismatch,
            None,
        ));
    }
    if observed.os_build_sha256() != target.os_build_sha256() {
        return Err(startup_error(
            PlatformStartupErrorKind::OsBuildMismatch,
            None,
        ));
    }
    if observed.toolchain_sha256() != target.toolchain_sha256() {
        return Err(startup_error(
            PlatformStartupErrorKind::ToolchainMismatch,
            None,
        ));
    }
    if observed.vscode_build_sha256() != target.vscode_build_sha256() {
        return Err(startup_error(
            PlatformStartupErrorKind::VisualStudioCodeMismatch,
            None,
        ));
    }
    if observed.package_sha256() != target.package_sha256() {
        return Err(startup_error(
            PlatformStartupErrorKind::PackageMismatch,
            None,
        ));
    }

    let mut observations = Vec::with_capacity(REQUIRED_PLATFORM_CAPABILITIES.len());
    for capability in REQUIRED_PLATFORM_CAPABILITIES {
        let observation = adapter.probe_capability(capability);
        validate_observation(manifest, capability, &observation)?;
        observations.push(observation);
    }

    let observations = observations
        .try_into()
        .map_err(|_| startup_error(PlatformStartupErrorKind::CapabilityMismatch, None))?;
    Ok(VerifiedPlatformAdapter {
        adapter,
        observations,
    })
}

fn validate_observation(
    manifest: &PlatformManifestIdentity,
    expected: PlatformCapability,
    observation: &PlatformCapabilityObservation,
) -> Result<(), PlatformStartupError> {
    if observation.capability() != expected {
        return Err(startup_error(
            PlatformStartupErrorKind::CapabilityMismatch,
            Some(expected),
        ));
    }
    if observation.platform() != manifest.target().family()
        || observation.manifest_sha256() != manifest.manifest_sha256()
    {
        return Err(startup_error(
            PlatformStartupErrorKind::ForeignCapabilityEvidence,
            Some(expected),
        ));
    }
    match observation.status() {
        PlatformCapabilityStatus::Verified => Ok(()),
        PlatformCapabilityStatus::Unavailable => Err(startup_error(
            PlatformStartupErrorKind::CapabilityUnavailable,
            Some(expected),
        )),
        PlatformCapabilityStatus::Invalid => Err(startup_error(
            PlatformStartupErrorKind::CapabilityInvalid,
            Some(expected),
        )),
    }
}

const fn startup_error(
    kind: PlatformStartupErrorKind,
    capability: Option<PlatformCapability>,
) -> PlatformStartupError {
    PlatformStartupError::new(kind, capability)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use agentmage_kernel_contracts::{
        PLATFORM_ADAPTER_API_VERSION, PlatformAdapter, PlatformArchitecture, PlatformCapability,
        PlatformCapabilityObservation, PlatformCapabilityStatus, PlatformFamily,
        PlatformManifestIdentity, PlatformRuntimeIdentity, PlatformStartupError,
        PlatformStartupErrorKind, REQUIRED_PLATFORM_CAPABILITIES,
    };

    use super::activate_platform;

    #[derive(Debug)]
    struct FakeAdapter {
        manifest: PlatformManifestIdentity,
        runtime: PlatformRuntimeIdentity,
        failed_capability: Option<(PlatformCapability, PlatformCapabilityStatus)>,
        returned_capability: Option<PlatformCapability>,
        observation_platform: Option<PlatformFamily>,
        observation_manifest: Option<[u8; 32]>,
        probes: AtomicUsize,
        workspace_observations: AtomicUsize,
    }

    impl PlatformAdapter for FakeAdapter {
        fn manifest_identity(&self) -> &PlatformManifestIdentity {
            &self.manifest
        }

        fn runtime_identity(&self) -> Result<PlatformRuntimeIdentity, PlatformStartupError> {
            Ok(self.runtime.clone())
        }

        fn probe_capability(
            &self,
            capability: PlatformCapability,
        ) -> PlatformCapabilityObservation {
            self.probes.fetch_add(1, Ordering::SeqCst);
            let status = self
                .failed_capability
                .filter(|(candidate, _)| *candidate == capability)
                .map_or(PlatformCapabilityStatus::Verified, |(_, status)| status);
            PlatformCapabilityObservation::new(
                self.returned_capability.unwrap_or(capability),
                status,
                self.observation_platform.unwrap_or(self.runtime.family()),
                self.observation_manifest
                    .unwrap_or(*self.manifest.manifest_sha256()),
                mechanism_digest(capability),
            )
        }
    }

    fn runtime() -> PlatformRuntimeIdentity {
        PlatformRuntimeIdentity::new(
            PlatformFamily::DeterministicFake,
            PlatformArchitecture::X86_64,
            [1; 32],
            [2; 32],
            [3; 32],
            [4; 32],
        )
    }

    fn adapter() -> FakeAdapter {
        let runtime = runtime();
        FakeAdapter {
            manifest: PlatformManifestIdentity::new(
                PLATFORM_ADAPTER_API_VERSION,
                [9; 32],
                runtime.clone(),
            ),
            runtime,
            failed_capability: None,
            returned_capability: None,
            observation_platform: None,
            observation_manifest: None,
            probes: AtomicUsize::new(0),
            workspace_observations: AtomicUsize::new(0),
        }
    }

    const fn mechanism_digest(capability: PlatformCapability) -> [u8; 32] {
        [capability as u8 + 1; 32]
    }

    #[test]
    fn exact_runtime_and_complete_capability_set_activate_once() {
        let verified = activate_platform(adapter()).expect("verified fake platform");
        for (expected, observed) in REQUIRED_PLATFORM_CAPABILITIES
            .iter()
            .zip(verified.capability_observations())
        {
            assert_eq!(observed.capability(), *expected);
            assert_eq!(observed.status(), PlatformCapabilityStatus::Verified);
            assert_ne!(observed.mechanism_sha256(), &[0; 32]);
        }
        assert_eq!(
            verified.adapter().probes.load(Ordering::SeqCst),
            REQUIRED_PLATFORM_CAPABILITIES.len()
        );
        assert_eq!(
            verified
                .adapter()
                .workspace_observations
                .load(Ordering::SeqCst),
            0
        );
    }

    #[test]
    fn every_missing_or_invalid_capability_refuses_before_workspace_access() {
        for capability in REQUIRED_PLATFORM_CAPABILITIES {
            for (status, expected_kind) in [
                (
                    PlatformCapabilityStatus::Unavailable,
                    PlatformStartupErrorKind::CapabilityUnavailable,
                ),
                (
                    PlatformCapabilityStatus::Invalid,
                    PlatformStartupErrorKind::CapabilityInvalid,
                ),
            ] {
                let mut candidate = adapter();
                candidate.failed_capability = Some((capability, status));
                let error = activate_platform(candidate).expect_err("startup must fail");
                assert_eq!(error.kind(), expected_kind);
                assert_eq!(error.capability(), Some(capability));
            }
        }
    }

    #[test]
    fn every_runtime_identity_mutation_refuses_before_capability_probes() {
        let cases = [
            (
                PlatformRuntimeIdentity::new(
                    PlatformFamily::Fedora,
                    PlatformArchitecture::X86_64,
                    [1; 32],
                    [2; 32],
                    [3; 32],
                    [4; 32],
                ),
                PlatformStartupErrorKind::PlatformMismatch,
            ),
            (
                PlatformRuntimeIdentity::new(
                    PlatformFamily::DeterministicFake,
                    PlatformArchitecture::Aarch64,
                    [1; 32],
                    [2; 32],
                    [3; 32],
                    [4; 32],
                ),
                PlatformStartupErrorKind::ArchitectureMismatch,
            ),
            (
                PlatformRuntimeIdentity::new(
                    PlatformFamily::DeterministicFake,
                    PlatformArchitecture::X86_64,
                    [8; 32],
                    [2; 32],
                    [3; 32],
                    [4; 32],
                ),
                PlatformStartupErrorKind::OsBuildMismatch,
            ),
            (
                PlatformRuntimeIdentity::new(
                    PlatformFamily::DeterministicFake,
                    PlatformArchitecture::X86_64,
                    [1; 32],
                    [8; 32],
                    [3; 32],
                    [4; 32],
                ),
                PlatformStartupErrorKind::ToolchainMismatch,
            ),
            (
                PlatformRuntimeIdentity::new(
                    PlatformFamily::DeterministicFake,
                    PlatformArchitecture::X86_64,
                    [1; 32],
                    [2; 32],
                    [8; 32],
                    [4; 32],
                ),
                PlatformStartupErrorKind::VisualStudioCodeMismatch,
            ),
            (
                PlatformRuntimeIdentity::new(
                    PlatformFamily::DeterministicFake,
                    PlatformArchitecture::X86_64,
                    [1; 32],
                    [2; 32],
                    [3; 32],
                    [8; 32],
                ),
                PlatformStartupErrorKind::PackageMismatch,
            ),
        ];

        for (runtime, expected_kind) in cases {
            let mut candidate = adapter();
            candidate.runtime = runtime;
            assert_eq!(
                activate_platform(candidate)
                    .expect_err("identity must fail")
                    .kind(),
                expected_kind
            );
        }
    }

    #[test]
    fn wrong_api_capability_or_evidence_identity_never_falls_back() {
        let mut wrong_api = adapter();
        wrong_api.manifest = PlatformManifestIdentity::new(2, [9; 32], runtime());
        assert_eq!(
            activate_platform(wrong_api)
                .expect_err("API must fail")
                .kind(),
            PlatformStartupErrorKind::ApiVersionMismatch
        );

        let mut wrong_capability = adapter();
        wrong_capability.returned_capability = Some(PlatformCapability::Updates);
        assert_eq!(
            activate_platform(wrong_capability)
                .expect_err("capability must fail")
                .kind(),
            PlatformStartupErrorKind::CapabilityMismatch
        );

        let mut wrong_platform = adapter();
        wrong_platform.observation_platform = Some(PlatformFamily::Ubuntu);
        assert_eq!(
            activate_platform(wrong_platform)
                .expect_err("platform evidence must fail")
                .kind(),
            PlatformStartupErrorKind::ForeignCapabilityEvidence
        );

        let mut wrong_manifest = adapter();
        wrong_manifest.observation_manifest = Some([7; 32]);
        assert_eq!(
            activate_platform(wrong_manifest)
                .expect_err("manifest evidence must fail")
                .kind(),
            PlatformStartupErrorKind::ForeignCapabilityEvidence
        );
    }
}
