//! Independently trusted, fail-closed platform activation before workspace access.

use agentmage_kernel_contracts::{
    PLATFORM_ADAPTER_API_VERSION, PlatformAdapter, PlatformArchitecture, PlatformCapability,
    PlatformCapabilityObservation, PlatformCapabilityStatus, PlatformFamily,
    PlatformManifestIdentity, PlatformRuntimeIdentity, PlatformStartupError,
    PlatformStartupErrorKind, REQUIRED_PLATFORM_CAPABILITIES,
};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::Deserialize;
use sha2::{Digest, Sha256};

const RELEASE_MANIFEST_SCHEMA_VERSION: u16 = 2;
const MAX_RELEASE_MANIFEST_BYTES: usize = 1024 * 1024;
const RELEASE_SIGNATURE_DOMAIN: &[u8] = b"agentmage.platform-release-manifest.v2\0";

/// Independently signature-verified expected platform and mechanism identity.
pub struct VerifiedPlatformRelease {
    identity: PlatformManifestIdentity,
    mechanism_sha256: [[u8; 32]; REQUIRED_PLATFORM_CAPABILITIES.len()],
    signer_sha256: [u8; 32],
}

impl std::fmt::Debug for VerifiedPlatformRelease {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("VerifiedPlatformRelease")
            .field("family", &self.identity.target().family())
            .field("architecture", &self.identity.target().architecture())
            .field("manifest", &"sha256:[REDACTED]")
            .field("signer", &"sha256:[REDACTED]")
            .finish_non_exhaustive()
    }
}

impl VerifiedPlatformRelease {
    /// Returns the exact signed manifest identity and runtime target.
    #[must_use]
    pub const fn identity(&self) -> &PlatformManifestIdentity {
        &self.identity
    }

    /// Returns the trusted signing-key digest without returning the key.
    #[must_use]
    pub const fn signer_sha256(&self) -> &[u8; 32] {
        &self.signer_sha256
    }

    fn expected_mechanism(&self, capability: PlatformCapability) -> &[u8; 32] {
        &self.mechanism_sha256[capability_index(capability)]
    }
}

/// Verifies exact release-manifest bytes before any native adapter observation.
pub fn verify_platform_release(
    manifest_bytes: &[u8],
    signature_bytes: &[u8; 64],
    verifying_key_bytes: &[u8; 32],
) -> Result<VerifiedPlatformRelease, PlatformStartupError> {
    if manifest_bytes.is_empty() || manifest_bytes.len() > MAX_RELEASE_MANIFEST_BYTES {
        return Err(startup_error(
            PlatformStartupErrorKind::ManifestTooLarge,
            None,
        ));
    }
    let verifying_key = VerifyingKey::from_bytes(verifying_key_bytes)
        .map_err(|_| startup_error(PlatformStartupErrorKind::ManifestSignatureInvalid, None))?;
    let mut signed = Vec::with_capacity(RELEASE_SIGNATURE_DOMAIN.len() + manifest_bytes.len());
    signed.extend_from_slice(RELEASE_SIGNATURE_DOMAIN);
    signed.extend_from_slice(manifest_bytes);
    verifying_key
        .verify_strict(&signed, &Signature::from_bytes(signature_bytes))
        .map_err(|_| startup_error(PlatformStartupErrorKind::ManifestSignatureInvalid, None))?;

    let wire: ReleaseManifestWire = serde_json::from_slice(manifest_bytes)
        .map_err(|_| startup_error(PlatformStartupErrorKind::ManifestMalformed, None))?;
    if wire.schema_version != RELEASE_MANIFEST_SCHEMA_VERSION
        || wire.record_type != "platform-release-manifest"
        || wire.status != "signed-release"
        || wire.adapter_api_version != PLATFORM_ADAPTER_API_VERSION
    {
        return Err(startup_error(
            PlatformStartupErrorKind::ManifestUnsupported,
            None,
        ));
    }
    let family = parse_family(&wire.platform_family)?;
    let architecture = parse_architecture(&wire.architecture)?;
    let runtime = PlatformRuntimeIdentity::new(
        family,
        architecture,
        decode_nonzero_sha256(&wire.os_build_sha256)?,
        decode_nonzero_sha256(&wire.toolchain_sha256)?,
        decode_nonzero_sha256(&wire.vscode_build_sha256)?,
        decode_nonzero_sha256(&wire.package_sha256)?,
    );
    if wire.capabilities.len() != REQUIRED_PLATFORM_CAPABILITIES.len() {
        return Err(startup_error(
            PlatformStartupErrorKind::ManifestUnsupported,
            None,
        ));
    }
    let mut mechanism_sha256 = [[0_u8; 32]; REQUIRED_PLATFORM_CAPABILITIES.len()];
    for ((expected, candidate), retained) in REQUIRED_PLATFORM_CAPABILITIES
        .iter()
        .zip(&wire.capabilities)
        .zip(&mut mechanism_sha256)
    {
        if candidate.capability != capability_name(*expected) {
            return Err(startup_error(
                PlatformStartupErrorKind::ManifestUnsupported,
                Some(*expected),
            ));
        }
        *retained = decode_sha256(&candidate.mechanism_sha256)?;
        if *retained == [0; 32] {
            return Err(startup_error(
                PlatformStartupErrorKind::ManifestUnsupported,
                Some(*expected),
            ));
        }
    }
    Ok(VerifiedPlatformRelease {
        identity: PlatformManifestIdentity::new(
            wire.adapter_api_version,
            Sha256::digest(manifest_bytes).into(),
            runtime,
        ),
        mechanism_sha256,
        signer_sha256: Sha256::digest(verifying_key_bytes).into(),
    })
}

/// Adapter wrapper constructible only after release, runtime, and capability validation.
#[derive(Debug)]
pub struct VerifiedPlatformAdapter<A: PlatformAdapter> {
    adapter: A,
    release: VerifiedPlatformRelease,
    observations: [PlatformCapabilityObservation; REQUIRED_PLATFORM_CAPABILITIES.len()],
}

impl<A: PlatformAdapter> VerifiedPlatformAdapter<A> {
    /// Returns the verified adapter for later capability-specific dispatch.
    #[must_use]
    pub const fn adapter(&self) -> &A {
        &self.adapter
    }

    /// Returns the independently verified immutable release manifest.
    #[must_use]
    pub const fn release(&self) -> &VerifiedPlatformRelease {
        &self.release
    }

    /// Returns the immutable manifest selected at startup.
    #[must_use]
    pub const fn manifest_identity(&self) -> &PlatformManifestIdentity {
        self.release.identity()
    }

    /// Returns exact bounded evidence for every required startup capability.
    #[must_use]
    pub const fn capability_observations(
        &self,
    ) -> &[PlatformCapabilityObservation; REQUIRED_PLATFORM_CAPABILITIES.len()] {
        &self.observations
    }
}

/// Validates one adapter against independent release trust and every required capability.
pub fn activate_platform<A: PlatformAdapter>(
    release: VerifiedPlatformRelease,
    adapter: A,
) -> Result<VerifiedPlatformAdapter<A>, PlatformStartupError> {
    let manifest = release.identity();
    if manifest.api_version() != PLATFORM_ADAPTER_API_VERSION {
        return Err(startup_error(
            PlatformStartupErrorKind::ApiVersionMismatch,
            None,
        ));
    }

    let observed = adapter.runtime_identity()?;
    let target = manifest.target();
    validate_runtime(target, &observed)?;

    let mut observations = Vec::with_capacity(REQUIRED_PLATFORM_CAPABILITIES.len());
    for capability in REQUIRED_PLATFORM_CAPABILITIES {
        let observation = adapter.probe_capability(capability);
        validate_observation(&release, capability, &observation)?;
        observations.push(observation);
    }

    let observations = observations
        .try_into()
        .map_err(|_| startup_error(PlatformStartupErrorKind::CapabilityMismatch, None))?;
    Ok(VerifiedPlatformAdapter {
        adapter,
        release,
        observations,
    })
}

fn validate_runtime(
    target: &PlatformRuntimeIdentity,
    observed: &PlatformRuntimeIdentity,
) -> Result<(), PlatformStartupError> {
    let mismatch = if observed.family() != target.family() {
        Some(PlatformStartupErrorKind::PlatformMismatch)
    } else if observed.architecture() != target.architecture() {
        Some(PlatformStartupErrorKind::ArchitectureMismatch)
    } else if observed.os_build_sha256() != target.os_build_sha256() {
        Some(PlatformStartupErrorKind::OsBuildMismatch)
    } else if observed.toolchain_sha256() != target.toolchain_sha256() {
        Some(PlatformStartupErrorKind::ToolchainMismatch)
    } else if observed.vscode_build_sha256() != target.vscode_build_sha256() {
        Some(PlatformStartupErrorKind::VisualStudioCodeMismatch)
    } else if observed.package_sha256() != target.package_sha256() {
        Some(PlatformStartupErrorKind::PackageMismatch)
    } else {
        None
    };
    match mismatch {
        Some(kind) => Err(startup_error(kind, None)),
        None => Ok(()),
    }
}

fn validate_observation(
    release: &VerifiedPlatformRelease,
    expected: PlatformCapability,
    observation: &PlatformCapabilityObservation,
) -> Result<(), PlatformStartupError> {
    if observation.capability() != expected {
        return Err(startup_error(
            PlatformStartupErrorKind::CapabilityMismatch,
            Some(expected),
        ));
    }
    if observation.platform() != release.identity().target().family() {
        return Err(startup_error(
            PlatformStartupErrorKind::ForeignCapabilityEvidence,
            Some(expected),
        ));
    }
    match observation.status() {
        PlatformCapabilityStatus::Unavailable => Err(startup_error(
            PlatformStartupErrorKind::CapabilityUnavailable,
            Some(expected),
        )),
        PlatformCapabilityStatus::Invalid => Err(startup_error(
            PlatformStartupErrorKind::CapabilityInvalid,
            Some(expected),
        )),
        PlatformCapabilityStatus::Verified
            if observation.mechanism_sha256() != release.expected_mechanism(expected) =>
        {
            Err(startup_error(
                PlatformStartupErrorKind::MechanismMismatch,
                Some(expected),
            ))
        }
        PlatformCapabilityStatus::Verified => Ok(()),
    }
}

const fn capability_index(capability: PlatformCapability) -> usize {
    capability as usize
}

const fn capability_name(capability: PlatformCapability) -> &'static str {
    match capability {
        PlatformCapability::WorkspaceAuthorization => "workspace-authorization",
        PlatformCapability::SecurePathResolution => "secure-path-resolution",
        PlatformCapability::ToolConfinement => "tool-confinement",
        PlatformCapability::SecretStorage => "secret-storage",
        PlatformCapability::ProcessLimits => "process-limits",
        PlatformCapability::LocalInference => "local-inference",
        PlatformCapability::ModelInstallation => "model-installation",
        PlatformCapability::Packaging => "packaging",
        PlatformCapability::Updates => "updates",
        PlatformCapability::NetworkIsolation => "network-isolation",
    }
}

fn parse_family(value: &str) -> Result<PlatformFamily, PlatformStartupError> {
    match value {
        "fedora" => Ok(PlatformFamily::Fedora),
        "ubuntu" => Ok(PlatformFamily::Ubuntu),
        _ => Err(startup_error(
            PlatformStartupErrorKind::ManifestUnsupported,
            None,
        )),
    }
}

fn parse_architecture(value: &str) -> Result<PlatformArchitecture, PlatformStartupError> {
    match value {
        "x86_64" => Ok(PlatformArchitecture::X86_64),
        "aarch64" => Ok(PlatformArchitecture::Aarch64),
        _ => Err(startup_error(
            PlatformStartupErrorKind::ManifestUnsupported,
            None,
        )),
    }
}

fn decode_sha256(value: &str) -> Result<[u8; 32], PlatformStartupError> {
    if value.len() != 64 {
        return Err(startup_error(
            PlatformStartupErrorKind::ManifestMalformed,
            None,
        ));
    }
    let mut decoded = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = decode_hex(pair[0])
            .ok_or_else(|| startup_error(PlatformStartupErrorKind::ManifestMalformed, None))?;
        let low = decode_hex(pair[1])
            .ok_or_else(|| startup_error(PlatformStartupErrorKind::ManifestMalformed, None))?;
        decoded[index] = (high << 4) | low;
    }
    Ok(decoded)
}

fn decode_nonzero_sha256(value: &str) -> Result<[u8; 32], PlatformStartupError> {
    let decoded = decode_sha256(value)?;
    if decoded == [0; 32] {
        return Err(startup_error(
            PlatformStartupErrorKind::ManifestUnsupported,
            None,
        ));
    }
    Ok(decoded)
}

const fn decode_hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

const fn startup_error(
    kind: PlatformStartupErrorKind,
    capability: Option<PlatformCapability>,
) -> PlatformStartupError {
    PlatformStartupError::new(kind, capability)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReleaseManifestWire {
    schema_version: u16,
    record_type: String,
    status: String,
    adapter_api_version: u16,
    platform_family: String,
    architecture: String,
    os_build_sha256: String,
    toolchain_sha256: String,
    vscode_build_sha256: String,
    package_sha256: String,
    capabilities: Vec<ReleaseCapabilityWire>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReleaseCapabilityWire {
    capability: String,
    mechanism_sha256: String,
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use agentmage_kernel_contracts::{
        PlatformAdapter, PlatformArchitecture, PlatformCapability, PlatformCapabilityObservation,
        PlatformCapabilityStatus, PlatformFamily, PlatformRuntimeIdentity, PlatformStartupError,
        PlatformStartupErrorKind, REQUIRED_PLATFORM_CAPABILITIES,
    };
    use ed25519_dalek::{Signer, SigningKey};
    use serde_json::json;

    use super::{RELEASE_SIGNATURE_DOMAIN, activate_platform, verify_platform_release};

    #[derive(Debug)]
    struct FakeAdapter {
        runtime: PlatformRuntimeIdentity,
        failed_capability: Option<(PlatformCapability, PlatformCapabilityStatus)>,
        returned_capability: Option<PlatformCapability>,
        observation_platform: Option<PlatformFamily>,
        mechanism_mutation: Option<PlatformCapability>,
        probes: AtomicUsize,
    }

    impl PlatformAdapter for FakeAdapter {
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
                if self.mechanism_mutation == Some(capability) {
                    [99; 32]
                } else {
                    mechanism_digest(capability)
                },
            )
        }
    }

    fn runtime() -> PlatformRuntimeIdentity {
        PlatformRuntimeIdentity::new(
            PlatformFamily::Fedora,
            PlatformArchitecture::X86_64,
            [1; 32],
            [2; 32],
            [3; 32],
            [4; 32],
        )
    }

    fn adapter() -> FakeAdapter {
        FakeAdapter {
            runtime: runtime(),
            failed_capability: None,
            returned_capability: None,
            observation_platform: None,
            mechanism_mutation: None,
            probes: AtomicUsize::new(0),
        }
    }

    const fn mechanism_digest(capability: PlatformCapability) -> [u8; 32] {
        [capability as u8 + 1; 32]
    }

    fn signed_release(
        mutation: impl FnOnce(&mut serde_json::Value),
    ) -> (Vec<u8>, [u8; 64], [u8; 32]) {
        let capabilities: Vec<_> = REQUIRED_PLATFORM_CAPABILITIES
            .iter()
            .map(|capability| {
                json!({
                    "capability": super::capability_name(*capability),
                    "mechanism_sha256": hex(&mechanism_digest(*capability)),
                })
            })
            .collect();
        let mut value = json!({
            "schema_version": 2,
            "record_type": "platform-release-manifest",
            "status": "signed-release",
            "adapter_api_version": 1,
            "platform_family": "fedora",
            "architecture": "x86_64",
            "os_build_sha256": hex(&[1; 32]),
            "toolchain_sha256": hex(&[2; 32]),
            "vscode_build_sha256": hex(&[3; 32]),
            "package_sha256": hex(&[4; 32]),
            "capabilities": capabilities,
        });
        mutation(&mut value);
        let bytes = serde_json::to_vec(&value).expect("manifest serializes");
        let key = SigningKey::from_bytes(&[42; 32]);
        let mut material = RELEASE_SIGNATURE_DOMAIN.to_vec();
        material.extend_from_slice(&bytes);
        let signature = key.sign(&material).to_bytes();
        (bytes, signature, key.verifying_key().to_bytes())
    }

    fn release() -> super::VerifiedPlatformRelease {
        let (bytes, signature, key) = signed_release(|_| {});
        verify_platform_release(&bytes, &signature, &key).expect("signed release")
    }

    fn hex(value: &[u8; 32]) -> String {
        value.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn exact_independently_signed_release_activates_once() {
        let candidate = adapter();
        let verified = activate_platform(release(), candidate).expect("verified platform");
        assert_eq!(
            verified.adapter().probes.load(Ordering::SeqCst),
            REQUIRED_PLATFORM_CAPABILITIES.len()
        );
        assert_ne!(verified.release().signer_sha256(), &[0; 32]);
    }

    #[test]
    fn manifest_signature_schema_status_and_capability_order_fail_closed() {
        let (bytes, mut signature, key) = signed_release(|_| {});
        signature[0] ^= 1;
        assert_eq!(
            verify_platform_release(&bytes, &signature, &key)
                .expect_err("signature mutation")
                .kind(),
            PlatformStartupErrorKind::ManifestSignatureInvalid
        );
        let foreign_key = SigningKey::from_bytes(&[43; 32]).verifying_key().to_bytes();
        let (bytes, signature, _) = signed_release(|_| {});
        assert_eq!(
            verify_platform_release(&bytes, &signature, &foreign_key)
                .expect_err("foreign signer")
                .kind(),
            PlatformStartupErrorKind::ManifestSignatureInvalid
        );

        for mutation in [
            |value: &mut serde_json::Value| value["schema_version"] = json!(3),
            |value: &mut serde_json::Value| value["status"] = json!("contract-fixture"),
            |value: &mut serde_json::Value| {
                value["capabilities"]
                    .as_array_mut()
                    .expect("array")
                    .swap(0, 1);
            },
        ] {
            let (bytes, signature, key) = signed_release(mutation);
            assert_eq!(
                verify_platform_release(&bytes, &signature, &key)
                    .expect_err("unsupported manifest")
                    .kind(),
                PlatformStartupErrorKind::ManifestUnsupported
            );
        }
    }

    #[test]
    fn every_signed_runtime_identity_mutation_refuses_native_activation() {
        for (field, expected) in [
            ("os_build_sha256", PlatformStartupErrorKind::OsBuildMismatch),
            (
                "toolchain_sha256",
                PlatformStartupErrorKind::ToolchainMismatch,
            ),
            (
                "vscode_build_sha256",
                PlatformStartupErrorKind::VisualStudioCodeMismatch,
            ),
            ("package_sha256", PlatformStartupErrorKind::PackageMismatch),
        ] {
            let (bytes, signature, key) = signed_release(|value| {
                value[field] = json!(hex(&[88; 32]));
            });
            let release = verify_platform_release(&bytes, &signature, &key).expect("signed");
            assert_eq!(
                activate_platform(release, adapter())
                    .expect_err("runtime mutation")
                    .kind(),
                expected
            );
        }

        let (bytes, signature, key) = signed_release(|value| {
            value["architecture"] = json!("aarch64");
        });
        let release = verify_platform_release(&bytes, &signature, &key).expect("signed");
        assert_eq!(
            activate_platform(release, adapter())
                .expect_err("architecture mutation")
                .kind(),
            PlatformStartupErrorKind::ArchitectureMismatch
        );
    }

    #[test]
    fn zero_runtime_identity_and_unimplemented_platform_are_not_signed_release_targets() {
        for field in [
            "os_build_sha256",
            "toolchain_sha256",
            "vscode_build_sha256",
            "package_sha256",
        ] {
            let (bytes, signature, key) = signed_release(|value| {
                value[field] = json!(hex(&[0; 32]));
            });
            assert_eq!(
                verify_platform_release(&bytes, &signature, &key)
                    .expect_err("zero identity")
                    .kind(),
                PlatformStartupErrorKind::ManifestUnsupported
            );
        }
        let (bytes, signature, key) = signed_release(|value| {
            value["platform_family"] = json!("macos-apple-silicon");
        });
        assert_eq!(
            verify_platform_release(&bytes, &signature, &key)
                .expect_err("unimplemented platform")
                .kind(),
            PlatformStartupErrorKind::ManifestUnsupported
        );
    }

    #[test]
    fn every_missing_invalid_or_substituted_mechanism_refuses() {
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
                let error = activate_platform(release(), candidate).expect_err("startup fails");
                assert_eq!(error.kind(), expected_kind);
                assert_eq!(error.capability(), Some(capability));
            }
            let mut candidate = adapter();
            candidate.mechanism_mutation = Some(capability);
            let error = activate_platform(release(), candidate).expect_err("mechanism fails");
            assert_eq!(error.kind(), PlatformStartupErrorKind::MechanismMismatch);
            assert_eq!(error.capability(), Some(capability));
        }
    }

    #[test]
    fn runtime_and_foreign_observation_mutations_refuse_before_authority() {
        let mut wrong_runtime = adapter();
        wrong_runtime.runtime = PlatformRuntimeIdentity::new(
            PlatformFamily::Ubuntu,
            PlatformArchitecture::X86_64,
            [1; 32],
            [2; 32],
            [3; 32],
            [4; 32],
        );
        assert_eq!(
            activate_platform(release(), wrong_runtime)
                .expect_err("runtime mismatch")
                .kind(),
            PlatformStartupErrorKind::PlatformMismatch
        );

        let mut wrong_capability = adapter();
        wrong_capability.returned_capability = Some(PlatformCapability::Updates);
        assert_eq!(
            activate_platform(release(), wrong_capability)
                .expect_err("capability mismatch")
                .kind(),
            PlatformStartupErrorKind::CapabilityMismatch
        );

        let mut foreign = adapter();
        foreign.observation_platform = Some(PlatformFamily::Ubuntu);
        assert_eq!(
            activate_platform(release(), foreign)
                .expect_err("foreign observation")
                .kind(),
            PlatformStartupErrorKind::ForeignCapabilityEvidence
        );
    }
}
