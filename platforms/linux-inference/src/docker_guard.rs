//! Private Docker Model Runner raw-endpoint guard contract.

use std::fmt;
use std::net::Ipv4Addr;

use agentmage_kernel_contracts::{LocalEndpointIdentity, LocalTransport, NetworkComponent};

use crate::{DOCKER_MODEL_RUNNER_HOST, DOCKER_MODEL_RUNNER_IMAGE_DIGEST, DOCKER_MODEL_RUNNER_PORT};

/// SHA-256 of the checked Docker raw-endpoint guard profile.
pub const DOCKER_GUARD_PROFILE_SHA256: [u8; 32] = [
    0x88, 0xfb, 0x0d, 0x5a, 0x78, 0x82, 0x9c, 0xbd, 0xfc, 0x34, 0xaf, 0x5c, 0xbc, 0xbf, 0xe3, 0xca,
    0x2a, 0x80, 0xf5, 0x50, 0x88, 0x99, 0x47, 0xe6, 0x47, 0x8f, 0xdb, 0x66, 0xbf, 0x7e, 0xdb, 0x2a,
];

/// Lowercase hexadecimal identity of the checked Docker guard profile.
pub const DOCKER_GUARD_PROFILE_SHA256_HEX: &str =
    "88fb0d5a78829cbdfc34af5cbcbfe3ca2a80f550889947e6478fdb66bf7edb2a";

/// Stable caller classes evaluated at the private raw-endpoint boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockerEndpointCallerClass {
    /// Exact dedicated process acting for the authenticated AgentMage kernel.
    GuardedKernelDockerAdapter,
    /// Visual Studio Code extension host.
    VisualStudioCodeExtension,
    /// Sandboxed deterministic tool worker.
    ToolWorker,
    /// Unrelated process running as the ordinary desktop user.
    UnrelatedSameUserProcess,
    /// Container outside the private runner network namespace.
    ArbitraryContainer,
    /// Caller absent from the closed process inventory.
    Undeclared,
}

/// Content-free Docker endpoint-guard failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockerEndpointGuardError {
    /// The guard policy profile identity changed.
    ProfileIdentity,
    /// The raw listener was not private loopback inside the declared namespace.
    RawEndpointExposure,
    /// The dedicated guard process identity or authority closure changed.
    GuardProcessIdentity,
    /// The kernel-facing socket was not private and authenticated.
    KernelTransport,
    /// The caller class is never admitted to the raw runtime endpoint.
    CallerDenied,
    /// The declared guard caller did not match the verified process/session identity.
    CallerIdentity,
}

impl DockerEndpointGuardError {
    /// Returns a stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ProfileIdentity => "docker-guard.profile.identity",
            Self::RawEndpointExposure => "docker-guard.raw-endpoint.exposed",
            Self::GuardProcessIdentity => "docker-guard.process.identity",
            Self::KernelTransport => "docker-guard.kernel-transport.invalid",
            Self::CallerDenied => "docker-guard.caller.denied",
            Self::CallerIdentity => "docker-guard.caller.identity",
        }
    }
}

impl fmt::Display for DockerEndpointGuardError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for DockerEndpointGuardError {}

/// Exact, content-free process observation for one attempted raw-endpoint caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DockerEndpointCallerObservation {
    class: DockerEndpointCallerClass,
    uid: u32,
    executable_sha256: [u8; 32],
    cgroup_identity_sha256: [u8; 32],
    authenticated_kernel_session: bool,
}

impl DockerEndpointCallerObservation {
    /// Constructs one bounded platform observation without a path, address, or payload.
    #[must_use]
    pub const fn new(
        class: DockerEndpointCallerClass,
        uid: u32,
        executable_sha256: [u8; 32],
        cgroup_identity_sha256: [u8; 32],
        authenticated_kernel_session: bool,
    ) -> Self {
        Self {
            class,
            uid,
            executable_sha256,
            cgroup_identity_sha256,
            authenticated_kernel_session,
        }
    }
}

/// Verified two-hop isolation topology for the Docker Model Runner raw API.
#[derive(Clone, PartialEq, Eq)]
pub struct DockerEndpointGuard {
    guard_uid: u32,
    runtime_uid: u32,
    guard_executable_sha256: [u8; 32],
    guard_cgroup_identity_sha256: [u8; 32],
    private_namespace_sha256: [u8; 32],
    kernel_socket_identity_sha256: [u8; 32],
    raw_endpoint: LocalEndpointIdentity,
}

impl DockerEndpointGuard {
    /// Verifies the complete private namespace, process, socket, and authority closure.
    #[allow(clippy::too_many_arguments)]
    pub fn verify(
        profile_sha256: [u8; 32],
        runner_image_digest: [u8; 32],
        raw_endpoint: LocalEndpointIdentity,
        raw_host: Ipv4Addr,
        raw_port: u16,
        private_namespace_sha256: [u8; 32],
        host_tcp_listener_count: u8,
        non_loopback_listener_count: u8,
        container_bridge_route_count: u8,
        guard_uid: u32,
        runtime_uid: u32,
        guard_executable_sha256: [u8; 32],
        guard_cgroup_identity_sha256: [u8; 32],
        docker_socket_mount_count: u8,
        workspace_mount_count: u8,
        kernel_socket_identity_sha256: [u8; 32],
        kernel_socket_owner_uid: u32,
        kernel_socket_mode: u32,
        kernel_socket_peer_authenticated: bool,
    ) -> Result<Self, DockerEndpointGuardError> {
        if profile_sha256 != DOCKER_GUARD_PROFILE_SHA256
            || runner_image_digest != DOCKER_MODEL_RUNNER_IMAGE_DIGEST
        {
            return Err(DockerEndpointGuardError::ProfileIdentity);
        }
        if raw_endpoint.client() != NetworkComponent::KernelDockerInferenceAdapter
            || raw_endpoint.transport() != LocalTransport::GuardedLoopbackTcp
            || raw_host != DOCKER_MODEL_RUNNER_HOST
            || raw_port != DOCKER_MODEL_RUNNER_PORT
            || private_namespace_sha256 == [0; 32]
            || host_tcp_listener_count != 0
            || non_loopback_listener_count != 0
            || container_bridge_route_count != 0
        {
            return Err(DockerEndpointGuardError::RawEndpointExposure);
        }
        if guard_uid == 0
            || runtime_uid == 0
            || guard_uid == runtime_uid
            || guard_executable_sha256 == [0; 32]
            || guard_cgroup_identity_sha256 == [0; 32]
            || docker_socket_mount_count != 0
            || workspace_mount_count != 0
        {
            return Err(DockerEndpointGuardError::GuardProcessIdentity);
        }
        if kernel_socket_identity_sha256 == [0; 32]
            || kernel_socket_owner_uid != runtime_uid
            || kernel_socket_mode != 0o600
            || !kernel_socket_peer_authenticated
        {
            return Err(DockerEndpointGuardError::KernelTransport);
        }
        Ok(Self {
            guard_uid,
            runtime_uid,
            guard_executable_sha256,
            guard_cgroup_identity_sha256,
            private_namespace_sha256,
            kernel_socket_identity_sha256,
            raw_endpoint,
        })
    }

    /// Admits only the exact dedicated guard acting for a fresh authenticated kernel session.
    pub fn authorize(
        &self,
        caller: DockerEndpointCallerObservation,
    ) -> Result<DockerRawEndpointPermit<'_>, DockerEndpointGuardError> {
        if caller.class != DockerEndpointCallerClass::GuardedKernelDockerAdapter {
            return Err(DockerEndpointGuardError::CallerDenied);
        }
        if caller.uid != self.guard_uid
            || caller.executable_sha256 != self.guard_executable_sha256
            || caller.cgroup_identity_sha256 != self.guard_cgroup_identity_sha256
            || !caller.authenticated_kernel_session
        {
            return Err(DockerEndpointGuardError::CallerIdentity);
        }
        Ok(DockerRawEndpointPermit { guard: self })
    }

    /// Returns the ordinary runtime user that owns the authenticated outer socket.
    #[must_use]
    pub const fn runtime_uid(&self) -> u32 {
        self.runtime_uid
    }

    /// Returns the private namespace identity without exposing a namespace path.
    #[must_use]
    pub const fn private_namespace_sha256(&self) -> &[u8; 32] {
        &self.private_namespace_sha256
    }

    /// Returns the authenticated outer socket identity without exposing its path.
    #[must_use]
    pub const fn kernel_socket_identity_sha256(&self) -> &[u8; 32] {
        &self.kernel_socket_identity_sha256
    }
}

impl fmt::Debug for DockerEndpointGuard {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DockerEndpointGuard")
            .field("guard_uid", &self.guard_uid)
            .field("runtime_uid", &self.runtime_uid)
            .finish_non_exhaustive()
    }
}

/// Unforgeable capability to the verified private raw endpoint.
pub struct DockerRawEndpointPermit<'guard> {
    guard: &'guard DockerEndpointGuard,
}

impl DockerRawEndpointPermit<'_> {
    /// Returns only the content-free endpoint identity for the guarded connector.
    #[must_use]
    pub const fn endpoint_identity(&self) -> &LocalEndpointIdentity {
        &self.guard.raw_endpoint
    }
}

impl fmt::Debug for DockerRawEndpointPermit<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DockerRawEndpointPermit")
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{LocalEndpointIdentity, LocalTransport, NetworkComponent};

    use super::*;

    fn endpoint() -> LocalEndpointIdentity {
        LocalEndpointIdentity::new(
            NetworkComponent::KernelDockerInferenceAdapter,
            LocalTransport::GuardedLoopbackTcp,
            [8; 32],
        )
        .expect("guarded raw endpoint")
    }

    fn guard() -> DockerEndpointGuard {
        DockerEndpointGuard::verify(
            DOCKER_GUARD_PROFILE_SHA256,
            DOCKER_MODEL_RUNNER_IMAGE_DIGEST,
            endpoint(),
            Ipv4Addr::LOCALHOST,
            12_434,
            [1; 32],
            0,
            0,
            0,
            991,
            1000,
            [2; 32],
            [3; 32],
            0,
            0,
            [4; 32],
            1000,
            0o600,
            true,
        )
        .expect("closed guard topology")
    }

    fn caller(class: DockerEndpointCallerClass) -> DockerEndpointCallerObservation {
        DockerEndpointCallerObservation::new(class, 991, [2; 32], [3; 32], true)
    }

    #[test]
    fn only_exact_guarded_kernel_adapter_receives_a_permit() {
        let guard = guard();
        let permit = guard
            .authorize(caller(
                DockerEndpointCallerClass::GuardedKernelDockerAdapter,
            ))
            .expect("exact guard admitted");
        assert_eq!(
            permit.endpoint_identity().client(),
            NetworkComponent::KernelDockerInferenceAdapter
        );
        for class in [
            DockerEndpointCallerClass::VisualStudioCodeExtension,
            DockerEndpointCallerClass::ToolWorker,
            DockerEndpointCallerClass::UnrelatedSameUserProcess,
            DockerEndpointCallerClass::ArbitraryContainer,
            DockerEndpointCallerClass::Undeclared,
        ] {
            assert_eq!(
                guard.authorize(caller(class)).unwrap_err(),
                DockerEndpointGuardError::CallerDenied
            );
        }
    }

    #[test]
    fn guard_caller_identity_and_session_mutations_fail_closed() {
        let guard = guard();
        for observation in [
            DockerEndpointCallerObservation::new(
                DockerEndpointCallerClass::GuardedKernelDockerAdapter,
                992,
                [2; 32],
                [3; 32],
                true,
            ),
            DockerEndpointCallerObservation::new(
                DockerEndpointCallerClass::GuardedKernelDockerAdapter,
                991,
                [0; 32],
                [3; 32],
                true,
            ),
            DockerEndpointCallerObservation::new(
                DockerEndpointCallerClass::GuardedKernelDockerAdapter,
                991,
                [2; 32],
                [0; 32],
                true,
            ),
            DockerEndpointCallerObservation::new(
                DockerEndpointCallerClass::GuardedKernelDockerAdapter,
                991,
                [2; 32],
                [3; 32],
                false,
            ),
        ] {
            assert_eq!(
                guard.authorize(observation).unwrap_err(),
                DockerEndpointGuardError::CallerIdentity
            );
        }
    }

    #[test]
    fn raw_endpoint_exposure_mutations_fail_closed() {
        let native = LocalEndpointIdentity::new(
            NetworkComponent::KernelNativeInferenceAdapter,
            LocalTransport::AuthenticatedUnixSocket,
            [8; 32],
        )
        .expect("native endpoint");
        let mutations = [
            (endpoint(), Ipv4Addr::UNSPECIFIED, 12_434, [1; 32], 0, 0, 0),
            (endpoint(), Ipv4Addr::LOCALHOST, 12_435, [1; 32], 0, 0, 0),
            (endpoint(), Ipv4Addr::LOCALHOST, 12_434, [0; 32], 0, 0, 0),
            (endpoint(), Ipv4Addr::LOCALHOST, 12_434, [1; 32], 1, 0, 0),
            (endpoint(), Ipv4Addr::LOCALHOST, 12_434, [1; 32], 0, 1, 0),
            (endpoint(), Ipv4Addr::LOCALHOST, 12_434, [1; 32], 0, 0, 1),
            (native, Ipv4Addr::LOCALHOST, 12_434, [1; 32], 0, 0, 0),
        ];
        for values in mutations {
            assert_eq!(
                DockerEndpointGuard::verify(
                    DOCKER_GUARD_PROFILE_SHA256,
                    DOCKER_MODEL_RUNNER_IMAGE_DIGEST,
                    values.0,
                    values.1,
                    values.2,
                    values.3,
                    values.4,
                    values.5,
                    values.6,
                    991,
                    1000,
                    [2; 32],
                    [3; 32],
                    0,
                    0,
                    [4; 32],
                    1000,
                    0o600,
                    true,
                ),
                Err(DockerEndpointGuardError::RawEndpointExposure)
            );
        }
    }

    #[test]
    fn process_mount_and_kernel_transport_mutations_fail_closed() {
        for values in [
            (0, 1000, [2; 32], [3; 32], 0, 0),
            (1000, 1000, [2; 32], [3; 32], 0, 0),
            (991, 1000, [0; 32], [3; 32], 0, 0),
            (991, 1000, [2; 32], [0; 32], 0, 0),
            (991, 1000, [2; 32], [3; 32], 1, 0),
            (991, 1000, [2; 32], [3; 32], 0, 1),
        ] {
            assert_eq!(
                DockerEndpointGuard::verify(
                    DOCKER_GUARD_PROFILE_SHA256,
                    DOCKER_MODEL_RUNNER_IMAGE_DIGEST,
                    endpoint(),
                    Ipv4Addr::LOCALHOST,
                    12_434,
                    [1; 32],
                    0,
                    0,
                    0,
                    values.0,
                    values.1,
                    values.2,
                    values.3,
                    values.4,
                    values.5,
                    [4; 32],
                    1000,
                    0o600,
                    true,
                ),
                Err(DockerEndpointGuardError::GuardProcessIdentity)
            );
        }
        for values in [
            ([0; 32], 1000, 0o600, true),
            ([4; 32], 1001, 0o600, true),
            ([4; 32], 1000, 0o660, true),
            ([4; 32], 1000, 0o600, false),
        ] {
            assert_eq!(
                DockerEndpointGuard::verify(
                    DOCKER_GUARD_PROFILE_SHA256,
                    DOCKER_MODEL_RUNNER_IMAGE_DIGEST,
                    endpoint(),
                    Ipv4Addr::LOCALHOST,
                    12_434,
                    [1; 32],
                    0,
                    0,
                    0,
                    991,
                    1000,
                    [2; 32],
                    [3; 32],
                    0,
                    0,
                    values.0,
                    values.1,
                    values.2,
                    values.3,
                ),
                Err(DockerEndpointGuardError::KernelTransport)
            );
        }
    }
}
