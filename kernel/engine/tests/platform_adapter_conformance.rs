use agentmage_kernel_contracts::{
    PlatformAdapter, PlatformArchitecture, PlatformCapability, PlatformCapabilityObservation,
    PlatformCapabilityStatus, PlatformFamily, PlatformRuntimeIdentity, PlatformStartupError,
    PlatformStartupErrorKind, REQUIRED_PLATFORM_CAPABILITIES,
};
use agentmage_kernel_engine::platform_startup::{activate_platform, verify_platform_release};
use ed25519_dalek::{Signer, SigningKey};
use serde_json::json;

const SIGNATURE_DOMAIN: &[u8] = b"agentmage.platform-release-manifest.v2\0";

#[derive(Debug)]
struct ObservedAdapter {
    runtime: PlatformRuntimeIdentity,
}

impl PlatformAdapter for ObservedAdapter {
    fn runtime_identity(&self) -> Result<PlatformRuntimeIdentity, PlatformStartupError> {
        Ok(self.runtime.clone())
    }

    fn probe_capability(&self, capability: PlatformCapability) -> PlatformCapabilityObservation {
        PlatformCapabilityObservation::new(
            capability,
            PlatformCapabilityStatus::Verified,
            self.runtime.family(),
            mechanism_digest(capability),
        )
    }
}

const fn mechanism_digest(capability: PlatformCapability) -> [u8; 32] {
    [capability as u8 + 1; 32]
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

fn signed_release(
    family: PlatformFamily,
) -> agentmage_kernel_engine::platform_startup::VerifiedPlatformRelease {
    let family_name = match family {
        PlatformFamily::Fedora => "fedora",
        PlatformFamily::Ubuntu => "ubuntu",
        PlatformFamily::MacOsAppleSilicon => "macos-apple-silicon",
        PlatformFamily::DeterministicFake => panic!("fake is not a signed release target"),
    };
    let capabilities: Vec<_> = REQUIRED_PLATFORM_CAPABILITIES
        .iter()
        .map(|capability| {
            json!({
                "capability": capability_name(*capability),
                "mechanism_sha256": hex(&mechanism_digest(*capability)),
            })
        })
        .collect();
    let bytes = serde_json::to_vec(&json!({
        "schema_version": 2,
        "record_type": "platform-release-manifest",
        "status": "signed-release",
        "adapter_api_version": 1,
        "platform_family": family_name,
        "architecture": "x86_64",
        "os_build_sha256": hex(&[1; 32]),
        "toolchain_sha256": hex(&[2; 32]),
        "vscode_build_sha256": hex(&[3; 32]),
        "package_sha256": hex(&[4; 32]),
        "capabilities": capabilities,
    }))
    .expect("manifest serializes");
    let key = SigningKey::from_bytes(&[42; 32]);
    let mut material = SIGNATURE_DOMAIN.to_vec();
    material.extend_from_slice(&bytes);
    verify_platform_release(
        &bytes,
        &key.sign(&material).to_bytes(),
        &key.verifying_key().to_bytes(),
    )
    .expect("release verifies")
}

fn adapter(family: PlatformFamily) -> ObservedAdapter {
    ObservedAdapter {
        runtime: PlatformRuntimeIdentity::new(
            family,
            PlatformArchitecture::X86_64,
            [1; 32],
            [2; 32],
            [3; 32],
            [4; 32],
        ),
    }
}

fn hex(value: &[u8; 32]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn fedora_and_ubuntu_use_identical_trust_and_adapter_semantics() {
    for family in [PlatformFamily::Fedora, PlatformFamily::Ubuntu] {
        let verified = activate_platform(signed_release(family), adapter(family))
            .expect("available adapter contract");
        assert_eq!(verified.manifest_identity().target().family(), family);
        assert_eq!(
            verified.capability_observations().len(),
            REQUIRED_PLATFORM_CAPABILITIES.len()
        );
    }
}

#[test]
fn signed_platform_identity_is_never_portable_between_adapters() {
    assert_eq!(
        activate_platform(
            signed_release(PlatformFamily::Fedora),
            adapter(PlatformFamily::Ubuntu)
        )
        .expect_err("cross-platform activation")
        .kind(),
        PlatformStartupErrorKind::PlatformMismatch
    );
}
