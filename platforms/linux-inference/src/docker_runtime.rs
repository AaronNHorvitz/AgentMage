//! Exact Docker Model Runner compatibility identity and authority-free topology.

use std::fmt;
use std::net::Ipv4Addr;

use agentmage_kernel_contracts::{LocalEndpointIdentity, LocalTransport, NetworkComponent};

/// Exact Docker Model Runner plugin package admitted by this compatibility profile.
pub const DOCKER_MODEL_PLUGIN_PACKAGE_ID: &str = "docker-model-plugin";

/// Exact Docker Model Runner plugin package version admitted by this profile.
pub const DOCKER_MODEL_PLUGIN_VERSION: &str = "1.2.6";

/// Exact Docker Model Runner image manifest admitted by this profile.
pub const DOCKER_MODEL_RUNNER_IMAGE_DIGEST: [u8; 32] = [
    0xbd, 0x94, 0x09, 0x5b, 0xbc, 0x1d, 0xdc, 0x42, 0x66, 0xc3, 0xa8, 0x8f, 0x58, 0x2a, 0x92, 0x56,
    0x2c, 0x6b, 0x63, 0xec, 0xeb, 0x17, 0x55, 0x72, 0xc9, 0xa6, 0x00, 0x45, 0x66, 0x37, 0x27, 0xc9,
];

/// Lowercase OCI digest of the exact Docker Model Runner image manifest.
pub const DOCKER_MODEL_RUNNER_IMAGE_DIGEST_HEX: &str =
    "bd94095bbc1ddc4266c3a88f582a92562c6b63eceb175572c9a60045663727c9";

/// Exact quarantined Gemma 4 E4B OCI manifest bound for compatibility testing.
pub const DOCKER_MODEL_ARTIFACT_DIGEST: [u8; 32] = [
    0x08, 0xfa, 0x7b, 0x1d, 0x44, 0xf2, 0x55, 0xbe, 0x48, 0xcf, 0xc1, 0x23, 0x59, 0x21, 0x17, 0x25,
    0xbf, 0xd6, 0x59, 0x74, 0x26, 0x12, 0xed, 0x4b, 0x22, 0x1c, 0xd5, 0xbe, 0x90, 0xd1, 0x44, 0x44,
];

/// Lowercase OCI digest of the quarantined Gemma 4 E4B model manifest.
pub const DOCKER_MODEL_ARTIFACT_DIGEST_HEX: &str =
    "08fa7b1d44f255be48cfc12359211725bfd659742612ed4b221cd5be90d14444";

/// SHA-256 of the checked Docker compatibility profile.
pub const DOCKER_RUNTIME_PROFILE_SHA256: [u8; 32] = [
    0xab, 0x8c, 0xde, 0x6b, 0xc1, 0x44, 0x0f, 0x8a, 0x00, 0x13, 0x39, 0x0a, 0xa2, 0xe2, 0x91, 0xa3,
    0x15, 0xcd, 0xeb, 0xcf, 0xe4, 0x4a, 0xa1, 0xd3, 0x39, 0xf0, 0xaa, 0x0b, 0x1d, 0x70, 0x89, 0x9c,
];

/// Lowercase hexadecimal identity of the checked Docker compatibility profile.
pub const DOCKER_RUNTIME_PROFILE_SHA256_HEX: &str =
    "ab8cde6bc1440f8a0013390aa2e291a315cdebcfe44aa1d339f0aa0b1d70899c";

/// Fixed Docker Engine loopback endpoint required by this compatibility profile.
pub const DOCKER_MODEL_RUNNER_HOST: Ipv4Addr = Ipv4Addr::LOCALHOST;

/// Fixed Docker Engine Model Runner port required by this compatibility profile.
pub const DOCKER_MODEL_RUNNER_PORT: u16 = 12_434;

/// Content-free Docker compatibility contract failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockerRuntimeContractError {
    /// The package, profile, runner image, or model OCI identity changed.
    RuntimeIdentity,
    /// Docker Engine or its socket/group privilege observation was unsafe.
    DaemonPrerequisite,
    /// The container mount set exposed an undeclared host object.
    MountPolicy,
    /// One or more resource settings exceeded the closed envelope.
    ResourceEnvelope,
    /// The runtime network policy allowed acquisition, tracking, or egress.
    NetworkPolicy,
    /// The local endpoint was not the guarded kernel Docker path.
    EndpointIdentity,
}

impl DockerRuntimeContractError {
    /// Returns a stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::RuntimeIdentity => "docker-inference.runtime.identity",
            Self::DaemonPrerequisite => "docker-inference.daemon.prerequisite",
            Self::MountPolicy => "docker-inference.mount-policy.invalid",
            Self::ResourceEnvelope => "docker-inference.resources.invalid",
            Self::NetworkPolicy => "docker-inference.network-policy.invalid",
            Self::EndpointIdentity => "docker-inference.endpoint.invalid",
        }
    }
}

impl fmt::Display for DockerRuntimeContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for DockerRuntimeContractError {}

/// Verified identity of the one Docker compatibility tuple admitted here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PinnedDockerRuntimeIdentity {
    _private: (),
}

impl PinnedDockerRuntimeIdentity {
    /// Verifies the plugin, profile, runner image, and model artifact without fallback.
    pub fn verify(
        package_id: &str,
        package_version: &str,
        profile_sha256: [u8; 32],
        runner_image_digest: [u8; 32],
        model_artifact_digest: [u8; 32],
    ) -> Result<Self, DockerRuntimeContractError> {
        if package_id != DOCKER_MODEL_PLUGIN_PACKAGE_ID
            || package_version != DOCKER_MODEL_PLUGIN_VERSION
            || profile_sha256 != DOCKER_RUNTIME_PROFILE_SHA256
            || runner_image_digest != DOCKER_MODEL_RUNNER_IMAGE_DIGEST
            || model_artifact_digest != DOCKER_MODEL_ARTIFACT_DIGEST
        {
            return Err(DockerRuntimeContractError::RuntimeIdentity);
        }
        Ok(Self { _private: () })
    }

    /// Returns the exact plugin package identifier.
    #[must_use]
    pub const fn package_id(self) -> &'static str {
        DOCKER_MODEL_PLUGIN_PACKAGE_ID
    }

    /// Returns the exact plugin package version.
    #[must_use]
    pub const fn package_version(self) -> &'static str {
        DOCKER_MODEL_PLUGIN_VERSION
    }

    /// Returns the exact runner image manifest digest.
    #[must_use]
    pub const fn runner_image_digest(self) -> &'static [u8; 32] {
        &DOCKER_MODEL_RUNNER_IMAGE_DIGEST
    }

    /// Returns the exact quarantined model artifact manifest digest.
    #[must_use]
    pub const fn model_artifact_digest(self) -> &'static [u8; 32] {
        &DOCKER_MODEL_ARTIFACT_DIGEST
    }
}

/// Content-free observation of the separately administered Docker daemon boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DockerDaemonPrerequisites {
    daemon_binary_sha256: [u8; 32],
    daemon_socket_identity_sha256: [u8; 32],
    socket_group_gid: u32,
    launch_uid: u32,
}

impl DockerDaemonPrerequisites {
    /// Verifies rootful Docker Engine and explicit standard-user group membership.
    #[allow(clippy::too_many_arguments)]
    pub fn verify(
        daemon_binary_sha256: [u8; 32],
        daemon_socket_identity_sha256: [u8; 32],
        daemon_uid: u32,
        socket_group_gid: u32,
        launch_uid: u32,
        launch_user_has_socket_group: bool,
        rootless_engine: bool,
    ) -> Result<Self, DockerRuntimeContractError> {
        if daemon_binary_sha256 == [0; 32]
            || daemon_socket_identity_sha256 == [0; 32]
            || daemon_uid != 0
            || socket_group_gid == 0
            || launch_uid == 0
            || launch_user_has_socket_group
            || rootless_engine
        {
            return Err(DockerRuntimeContractError::DaemonPrerequisite);
        }
        Ok(Self {
            daemon_binary_sha256,
            daemon_socket_identity_sha256,
            socket_group_gid,
            launch_uid,
        })
    }

    /// Returns the exact daemon executable identity.
    #[must_use]
    pub const fn daemon_binary_sha256(&self) -> &[u8; 32] {
        &self.daemon_binary_sha256
    }

    /// Returns the exact Docker socket object identity without exposing its path.
    #[must_use]
    pub const fn daemon_socket_identity_sha256(&self) -> &[u8; 32] {
        &self.daemon_socket_identity_sha256
    }

    /// Returns the explicitly accepted Docker socket group identifier.
    #[must_use]
    pub const fn socket_group_gid(self) -> u32 {
        self.socket_group_gid
    }

    /// Returns the standard user admitted to the compatibility adapter.
    #[must_use]
    pub const fn launch_uid(self) -> u32 {
        self.launch_uid
    }
}

/// Exact mount closure for the Docker Model Runner container.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DockerMountPolicy {
    _private: (),
}

impl DockerMountPolicy {
    /// Admits only a private tmpfs plus the Docker-managed immutable model artifact.
    pub fn verify(
        private_runtime_tmpfs: bool,
        model_content_store_write: bool,
        workspace_mounts: u8,
        credential_mounts: u8,
        host_root_mounts: u8,
        docker_socket_mounts: u8,
    ) -> Result<Self, DockerRuntimeContractError> {
        if !private_runtime_tmpfs
            || model_content_store_write
            || workspace_mounts != 0
            || credential_mounts != 0
            || host_root_mounts != 0
            || docker_socket_mounts != 0
        {
            return Err(DockerRuntimeContractError::MountPolicy);
        }
        Ok(Self { _private: () })
    }
}

/// Exact cgroup, duration, output, and concurrency envelope for Docker inference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DockerInferenceResourceEnvelope {
    memory_bytes: u64,
    tasks: u32,
    cpu_percent: u16,
    runtime_seconds: u16,
    output_bytes: u32,
}

impl DockerInferenceResourceEnvelope {
    /// Verifies one no-swap, one-slot resource envelope below every hard ceiling.
    #[allow(clippy::too_many_arguments)]
    pub fn verify(
        memory_bytes: u64,
        tasks: u32,
        cpu_percent: u16,
        runtime_seconds: u16,
        output_bytes: u32,
        swap_bytes: u64,
        parallel_slots: u8,
    ) -> Result<Self, DockerRuntimeContractError> {
        if !(256 * 1024 * 1024..=64 * 1024 * 1024 * 1024).contains(&memory_bytes)
            || !(1..=64).contains(&tasks)
            || !(1..=3200).contains(&cpu_percent)
            || !(1..=3600).contains(&runtime_seconds)
            || !(1..=16 * 1024 * 1024).contains(&output_bytes)
            || swap_bytes != 0
            || parallel_slots != 1
        {
            return Err(DockerRuntimeContractError::ResourceEnvelope);
        }
        Ok(Self {
            memory_bytes,
            tasks,
            cpu_percent,
            runtime_seconds,
            output_bytes,
        })
    }

    /// Returns the cgroup memory ceiling in bytes.
    #[must_use]
    pub const fn memory_bytes(self) -> u64 {
        self.memory_bytes
    }

    /// Returns the cgroup task ceiling.
    #[must_use]
    pub const fn tasks(self) -> u32 {
        self.tasks
    }

    /// Returns the cgroup CPU percentage ceiling.
    #[must_use]
    pub const fn cpu_percent(self) -> u16 {
        self.cpu_percent
    }

    /// Returns the process lifetime ceiling in seconds.
    #[must_use]
    pub const fn runtime_seconds(self) -> u16 {
        self.runtime_seconds
    }

    /// Returns the retained output ceiling in bytes.
    #[must_use]
    pub const fn output_bytes(self) -> u32 {
        self.output_bytes
    }

    /// Reports the immutable no-swap requirement.
    #[must_use]
    pub const fn swap_bytes(self) -> u64 {
        0
    }

    /// Reports the immutable one-slot requirement.
    #[must_use]
    pub const fn parallel_slots(self) -> u8 {
        1
    }
}

/// Exact no-acquisition, no-tracking, zero-egress network declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DockerOfflineNetworkPolicy {
    _private: (),
}

impl DockerOfflineNetworkPolicy {
    /// Verifies fixed loopback service state with every ambient network path disabled.
    #[allow(clippy::too_many_arguments)]
    pub fn verify(
        host: Ipv4Addr,
        port: u16,
        do_not_track: bool,
        acquisition_allowed: bool,
        registry_access: bool,
        ambient_proxy: bool,
        ambient_dns: bool,
        outbound_bytes: u64,
    ) -> Result<Self, DockerRuntimeContractError> {
        if host != DOCKER_MODEL_RUNNER_HOST
            || port != DOCKER_MODEL_RUNNER_PORT
            || !do_not_track
            || acquisition_allowed
            || registry_access
            || ambient_proxy
            || ambient_dns
            || outbound_bytes != 0
        {
            return Err(DockerRuntimeContractError::NetworkPolicy);
        }
        Ok(Self { _private: () })
    }
}

/// Complete authority-free topology passed to the future Docker compatibility supervisor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DockerInferenceTopology {
    runtime: PinnedDockerRuntimeIdentity,
    daemon: DockerDaemonPrerequisites,
    mounts: DockerMountPolicy,
    resources: DockerInferenceResourceEnvelope,
    network: DockerOfflineNetworkPolicy,
    endpoint: LocalEndpointIdentity,
}

impl DockerInferenceTopology {
    /// Binds exact identities, explicit daemon privilege, closed mounts, limits, and IPC.
    pub fn verify(
        runtime: PinnedDockerRuntimeIdentity,
        daemon: DockerDaemonPrerequisites,
        mounts: DockerMountPolicy,
        resources: DockerInferenceResourceEnvelope,
        network: DockerOfflineNetworkPolicy,
        endpoint: LocalEndpointIdentity,
    ) -> Result<Self, DockerRuntimeContractError> {
        if endpoint.client() != NetworkComponent::KernelDockerInferenceAdapter
            || endpoint.transport() != LocalTransport::GuardedLoopbackTcp
        {
            return Err(DockerRuntimeContractError::EndpointIdentity);
        }
        Ok(Self {
            runtime,
            daemon,
            mounts,
            resources,
            network,
            endpoint,
        })
    }

    /// Returns the verified compatibility runtime identity.
    #[must_use]
    pub const fn runtime(&self) -> PinnedDockerRuntimeIdentity {
        self.runtime
    }

    /// Returns the observed Docker daemon prerequisites.
    #[must_use]
    pub const fn daemon(&self) -> DockerDaemonPrerequisites {
        self.daemon
    }

    /// Returns the closed mount policy.
    #[must_use]
    pub const fn mounts(&self) -> DockerMountPolicy {
        self.mounts
    }

    /// Returns the resource ceiling.
    #[must_use]
    pub const fn resources(&self) -> DockerInferenceResourceEnvelope {
        self.resources
    }

    /// Returns the zero-egress network policy.
    #[must_use]
    pub const fn network(&self) -> DockerOfflineNetworkPolicy {
        self.network
    }

    /// Returns the guarded local endpoint identity.
    #[must_use]
    pub const fn endpoint(&self) -> &LocalEndpointIdentity {
        &self.endpoint
    }
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{LocalEndpointIdentity, LocalTransport, NetworkComponent};

    use super::*;

    fn runtime() -> PinnedDockerRuntimeIdentity {
        PinnedDockerRuntimeIdentity::verify(
            DOCKER_MODEL_PLUGIN_PACKAGE_ID,
            DOCKER_MODEL_PLUGIN_VERSION,
            DOCKER_RUNTIME_PROFILE_SHA256,
            DOCKER_MODEL_RUNNER_IMAGE_DIGEST,
            DOCKER_MODEL_ARTIFACT_DIGEST,
        )
        .expect("pinned Docker identity")
    }

    fn daemon() -> DockerDaemonPrerequisites {
        DockerDaemonPrerequisites::verify([1; 32], [2; 32], 0, 971, 1000, false, false)
            .expect("explicit rootful daemon prerequisites")
    }

    fn mounts() -> DockerMountPolicy {
        DockerMountPolicy::verify(true, false, 0, 0, 0, 0).expect("closed mounts")
    }

    fn resources() -> DockerInferenceResourceEnvelope {
        DockerInferenceResourceEnvelope::verify(
            16 * 1024 * 1024 * 1024,
            32,
            1600,
            900,
            4 * 1024 * 1024,
            0,
            1,
        )
        .expect("bounded resources")
    }

    fn network() -> DockerOfflineNetworkPolicy {
        DockerOfflineNetworkPolicy::verify(
            Ipv4Addr::LOCALHOST,
            12_434,
            true,
            false,
            false,
            false,
            false,
            0,
        )
        .expect("strict offline network")
    }

    #[test]
    fn runtime_identity_has_no_tag_fallback() {
        let identity = runtime();
        assert_eq!(identity.package_id(), DOCKER_MODEL_PLUGIN_PACKAGE_ID);
        assert_eq!(identity.package_version(), DOCKER_MODEL_PLUGIN_VERSION);
        assert_eq!(
            identity.runner_image_digest(),
            &DOCKER_MODEL_RUNNER_IMAGE_DIGEST
        );
        assert_eq!(
            identity.model_artifact_digest(),
            &DOCKER_MODEL_ARTIFACT_DIGEST
        );
        for (package, version, profile, image, model) in [
            (
                "docker-model-plugin-latest",
                DOCKER_MODEL_PLUGIN_VERSION,
                DOCKER_RUNTIME_PROFILE_SHA256,
                DOCKER_MODEL_RUNNER_IMAGE_DIGEST,
                DOCKER_MODEL_ARTIFACT_DIGEST,
            ),
            (
                DOCKER_MODEL_PLUGIN_PACKAGE_ID,
                "latest",
                DOCKER_RUNTIME_PROFILE_SHA256,
                DOCKER_MODEL_RUNNER_IMAGE_DIGEST,
                DOCKER_MODEL_ARTIFACT_DIGEST,
            ),
            (
                DOCKER_MODEL_PLUGIN_PACKAGE_ID,
                DOCKER_MODEL_PLUGIN_VERSION,
                [0; 32],
                DOCKER_MODEL_RUNNER_IMAGE_DIGEST,
                DOCKER_MODEL_ARTIFACT_DIGEST,
            ),
            (
                DOCKER_MODEL_PLUGIN_PACKAGE_ID,
                DOCKER_MODEL_PLUGIN_VERSION,
                DOCKER_RUNTIME_PROFILE_SHA256,
                [0; 32],
                DOCKER_MODEL_ARTIFACT_DIGEST,
            ),
            (
                DOCKER_MODEL_PLUGIN_PACKAGE_ID,
                DOCKER_MODEL_PLUGIN_VERSION,
                DOCKER_RUNTIME_PROFILE_SHA256,
                DOCKER_MODEL_RUNNER_IMAGE_DIGEST,
                [0; 32],
            ),
        ] {
            assert_eq!(
                PinnedDockerRuntimeIdentity::verify(package, version, profile, image, model),
                Err(DockerRuntimeContractError::RuntimeIdentity)
            );
        }
    }

    #[test]
    fn daemon_prerequisites_make_privilege_explicit_and_fail_closed() {
        let accepted = daemon();
        assert_eq!(accepted.daemon_binary_sha256(), &[1; 32]);
        assert_eq!(accepted.daemon_socket_identity_sha256(), &[2; 32]);
        assert_eq!(accepted.socket_group_gid(), 971);
        assert_eq!(accepted.launch_uid(), 1000);
        for values in [
            ([0; 32], [2; 32], 0, 971, 1000, false, false),
            ([1; 32], [0; 32], 0, 971, 1000, false, false),
            ([1; 32], [2; 32], 1000, 971, 1000, false, false),
            ([1; 32], [2; 32], 0, 0, 1000, false, false),
            ([1; 32], [2; 32], 0, 971, 0, false, false),
            ([1; 32], [2; 32], 0, 971, 1000, true, false),
            ([1; 32], [2; 32], 0, 971, 1000, false, true),
        ] {
            assert_eq!(
                DockerDaemonPrerequisites::verify(
                    values.0, values.1, values.2, values.3, values.4, values.5, values.6,
                ),
                Err(DockerRuntimeContractError::DaemonPrerequisite)
            );
        }
    }

    #[test]
    fn mounts_and_resources_reject_every_authority_or_limit_expansion() {
        for values in [
            (false, false, 0, 0, 0, 0),
            (true, true, 0, 0, 0, 0),
            (true, false, 1, 0, 0, 0),
            (true, false, 0, 1, 0, 0),
            (true, false, 0, 0, 1, 0),
            (true, false, 0, 0, 0, 1),
        ] {
            assert_eq!(
                DockerMountPolicy::verify(
                    values.0, values.1, values.2, values.3, values.4, values.5,
                ),
                Err(DockerRuntimeContractError::MountPolicy)
            );
        }
        for values in [
            (1, 32, 1600, 900, 4 * 1024 * 1024, 0, 1),
            (
                16 * 1024 * 1024 * 1024,
                65,
                1600,
                900,
                4 * 1024 * 1024,
                0,
                1,
            ),
            (
                16 * 1024 * 1024 * 1024,
                32,
                3201,
                900,
                4 * 1024 * 1024,
                0,
                1,
            ),
            (
                16 * 1024 * 1024 * 1024,
                32,
                1600,
                3601,
                4 * 1024 * 1024,
                0,
                1,
            ),
            (
                16 * 1024 * 1024 * 1024,
                32,
                1600,
                900,
                16 * 1024 * 1024 + 1,
                0,
                1,
            ),
            (
                16 * 1024 * 1024 * 1024,
                32,
                1600,
                900,
                4 * 1024 * 1024,
                1,
                1,
            ),
            (
                16 * 1024 * 1024 * 1024,
                32,
                1600,
                900,
                4 * 1024 * 1024,
                0,
                2,
            ),
        ] {
            assert_eq!(
                DockerInferenceResourceEnvelope::verify(
                    values.0, values.1, values.2, values.3, values.4, values.5, values.6,
                ),
                Err(DockerRuntimeContractError::ResourceEnvelope)
            );
        }
    }

    #[test]
    fn network_policy_accepts_only_fixed_offline_loopback() {
        for values in [
            (
                Ipv4Addr::UNSPECIFIED,
                12_434,
                true,
                false,
                false,
                false,
                false,
                0,
            ),
            (
                Ipv4Addr::LOCALHOST,
                12_435,
                true,
                false,
                false,
                false,
                false,
                0,
            ),
            (
                Ipv4Addr::LOCALHOST,
                12_434,
                false,
                false,
                false,
                false,
                false,
                0,
            ),
            (
                Ipv4Addr::LOCALHOST,
                12_434,
                true,
                true,
                false,
                false,
                false,
                0,
            ),
            (
                Ipv4Addr::LOCALHOST,
                12_434,
                true,
                false,
                true,
                false,
                false,
                0,
            ),
            (
                Ipv4Addr::LOCALHOST,
                12_434,
                true,
                false,
                false,
                true,
                false,
                0,
            ),
            (
                Ipv4Addr::LOCALHOST,
                12_434,
                true,
                false,
                false,
                false,
                true,
                0,
            ),
            (
                Ipv4Addr::LOCALHOST,
                12_434,
                true,
                false,
                false,
                false,
                false,
                1,
            ),
        ] {
            assert_eq!(
                DockerOfflineNetworkPolicy::verify(
                    values.0, values.1, values.2, values.3, values.4, values.5, values.6, values.7,
                ),
                Err(DockerRuntimeContractError::NetworkPolicy)
            );
        }
    }

    #[test]
    fn topology_accepts_only_the_guarded_kernel_docker_endpoint() {
        let docker = LocalEndpointIdentity::new(
            NetworkComponent::KernelDockerInferenceAdapter,
            LocalTransport::GuardedLoopbackTcp,
            [8; 32],
        )
        .expect("Docker endpoint");
        let topology = DockerInferenceTopology::verify(
            runtime(),
            daemon(),
            mounts(),
            resources(),
            network(),
            docker,
        )
        .expect("closed Docker topology");
        assert_eq!(topology.runtime(), runtime());
        assert_eq!(topology.daemon(), daemon());
        assert_eq!(topology.mounts(), mounts());
        assert_eq!(topology.resources(), resources());
        assert_eq!(topology.network(), network());
        assert_eq!(
            topology.endpoint().client(),
            NetworkComponent::KernelDockerInferenceAdapter
        );

        let native = LocalEndpointIdentity::new(
            NetworkComponent::KernelNativeInferenceAdapter,
            LocalTransport::AuthenticatedUnixSocket,
            [8; 32],
        )
        .expect("native endpoint");
        assert_eq!(
            DockerInferenceTopology::verify(
                runtime(),
                daemon(),
                mounts(),
                resources(),
                network(),
                native,
            ),
            Err(DockerRuntimeContractError::EndpointIdentity)
        );
    }
}
