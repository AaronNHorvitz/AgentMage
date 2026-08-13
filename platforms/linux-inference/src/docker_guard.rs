//! Private Docker Model Runner raw-endpoint guard contract.

use std::fmt;
use std::net::Ipv4Addr;

use agentmage_kernel_contracts::{LocalEndpointIdentity, LocalTransport, NetworkComponent};

use crate::{
    DOCKER_MODEL_RUNNER_BIND_HOST, DOCKER_MODEL_RUNNER_CONNECT_HOST,
    DOCKER_MODEL_RUNNER_IMAGE_DIGEST, DOCKER_MODEL_RUNNER_PORT,
};

/// SHA-256 of the checked Docker raw-endpoint guard profile.
pub const DOCKER_GUARD_PROFILE_SHA256: [u8; 32] = [
    0xa7, 0x44, 0xeb, 0x31, 0xf4, 0x94, 0x9e, 0xc7, 0xd9, 0x9d, 0xfa, 0xe8, 0xf6, 0x2e, 0xcc, 0xb5,
    0x31, 0x26, 0x9c, 0x35, 0x49, 0xee, 0x51, 0xf3, 0x97, 0x7e, 0x0e, 0xa6, 0x2a, 0x8f, 0x88, 0xc8,
];

/// Lowercase hexadecimal identity of the checked Docker guard profile.
pub const DOCKER_GUARD_PROFILE_SHA256_HEX: &str =
    "a744eb31f4949ec7d99dfae8f62eccb531269c3549ee51f3977e0ea62a8f88c8";

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
        runner_bind_host: Ipv4Addr,
        guard_connect_host: Ipv4Addr,
        raw_port: u16,
        private_namespace_sha256: [u8; 32],
        namespace_active_interface_count: u8,
        loopback_interface_up: bool,
        namespace_non_local_route_count: u8,
        host_tcp_listener_count: u8,
        non_loopback_listener_count: u8,
        container_bridge_route_count: u8,
        guard_uid: u32,
        runtime_uid: u32,
        runtime_gid: u32,
        guard_executable_sha256: [u8; 32],
        guard_cgroup_identity_sha256: [u8; 32],
        docker_socket_mount_count: u8,
        workspace_mount_count: u8,
        kernel_socket_identity_sha256: [u8; 32],
        kernel_socket_owner_uid: u32,
        kernel_socket_group_gid: u32,
        kernel_socket_parent_mode: u32,
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
            || runner_bind_host != DOCKER_MODEL_RUNNER_BIND_HOST
            || guard_connect_host != DOCKER_MODEL_RUNNER_CONNECT_HOST
            || raw_port != DOCKER_MODEL_RUNNER_PORT
            || private_namespace_sha256 == [0; 32]
            || namespace_active_interface_count != 1
            || !loopback_interface_up
            || namespace_non_local_route_count != 0
            || host_tcp_listener_count != 0
            || non_loopback_listener_count != 0
            || container_bridge_route_count != 0
        {
            return Err(DockerEndpointGuardError::RawEndpointExposure);
        }
        if guard_uid == 0
            || runtime_uid == 0
            || runtime_gid == 0
            || guard_uid == runtime_uid
            || guard_executable_sha256 == [0; 32]
            || guard_cgroup_identity_sha256 == [0; 32]
            || docker_socket_mount_count != 0
            || workspace_mount_count != 0
        {
            return Err(DockerEndpointGuardError::GuardProcessIdentity);
        }
        if kernel_socket_identity_sha256 == [0; 32]
            || kernel_socket_owner_uid != guard_uid
            || kernel_socket_group_gid != runtime_gid
            || kernel_socket_parent_mode != 0o710
            || kernel_socket_mode != 0o660
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

    #[derive(Clone)]
    struct GuardInput {
        profile_sha256: [u8; 32],
        runner_image_digest: [u8; 32],
        endpoint: LocalEndpointIdentity,
        runner_bind_host: Ipv4Addr,
        guard_connect_host: Ipv4Addr,
        raw_port: u16,
        private_namespace_sha256: [u8; 32],
        namespace_active_interface_count: u8,
        loopback_interface_up: bool,
        namespace_non_local_route_count: u8,
        host_tcp_listener_count: u8,
        non_loopback_listener_count: u8,
        container_bridge_route_count: u8,
        guard_uid: u32,
        runtime_uid: u32,
        runtime_gid: u32,
        guard_executable_sha256: [u8; 32],
        guard_cgroup_identity_sha256: [u8; 32],
        docker_socket_mount_count: u8,
        workspace_mount_count: u8,
        kernel_socket_identity_sha256: [u8; 32],
        kernel_socket_owner_uid: u32,
        kernel_socket_group_gid: u32,
        kernel_socket_parent_mode: u32,
        kernel_socket_mode: u32,
        kernel_socket_peer_authenticated: bool,
    }

    fn endpoint() -> LocalEndpointIdentity {
        LocalEndpointIdentity::new(
            NetworkComponent::KernelDockerInferenceAdapter,
            LocalTransport::GuardedLoopbackTcp,
            [8; 32],
        )
        .expect("guarded raw endpoint")
    }

    fn input() -> GuardInput {
        GuardInput {
            profile_sha256: DOCKER_GUARD_PROFILE_SHA256,
            runner_image_digest: DOCKER_MODEL_RUNNER_IMAGE_DIGEST,
            endpoint: endpoint(),
            runner_bind_host: Ipv4Addr::UNSPECIFIED,
            guard_connect_host: Ipv4Addr::LOCALHOST,
            raw_port: 12_434,
            private_namespace_sha256: [1; 32],
            namespace_active_interface_count: 1,
            loopback_interface_up: true,
            namespace_non_local_route_count: 0,
            host_tcp_listener_count: 0,
            non_loopback_listener_count: 0,
            container_bridge_route_count: 0,
            guard_uid: 991,
            runtime_uid: 1000,
            runtime_gid: 1000,
            guard_executable_sha256: [2; 32],
            guard_cgroup_identity_sha256: [3; 32],
            docker_socket_mount_count: 0,
            workspace_mount_count: 0,
            kernel_socket_identity_sha256: [4; 32],
            kernel_socket_owner_uid: 991,
            kernel_socket_group_gid: 1000,
            kernel_socket_parent_mode: 0o710,
            kernel_socket_mode: 0o660,
            kernel_socket_peer_authenticated: true,
        }
    }

    fn verify(input: GuardInput) -> Result<DockerEndpointGuard, DockerEndpointGuardError> {
        DockerEndpointGuard::verify(
            input.profile_sha256,
            input.runner_image_digest,
            input.endpoint,
            input.runner_bind_host,
            input.guard_connect_host,
            input.raw_port,
            input.private_namespace_sha256,
            input.namespace_active_interface_count,
            input.loopback_interface_up,
            input.namespace_non_local_route_count,
            input.host_tcp_listener_count,
            input.non_loopback_listener_count,
            input.container_bridge_route_count,
            input.guard_uid,
            input.runtime_uid,
            input.runtime_gid,
            input.guard_executable_sha256,
            input.guard_cgroup_identity_sha256,
            input.docker_socket_mount_count,
            input.workspace_mount_count,
            input.kernel_socket_identity_sha256,
            input.kernel_socket_owner_uid,
            input.kernel_socket_group_gid,
            input.kernel_socket_parent_mode,
            input.kernel_socket_mode,
            input.kernel_socket_peer_authenticated,
        )
    }

    fn guard() -> DockerEndpointGuard {
        verify(input()).expect("closed guard topology")
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
        let mutations: &[fn(&mut GuardInput)] = &[
            |value| value.runner_bind_host = Ipv4Addr::LOCALHOST,
            |value| value.guard_connect_host = Ipv4Addr::UNSPECIFIED,
            |value| value.raw_port = 12_435,
            |value| value.private_namespace_sha256 = [0; 32],
            |value| value.namespace_active_interface_count = 2,
            |value| value.loopback_interface_up = false,
            |value| value.namespace_non_local_route_count = 1,
            |value| value.host_tcp_listener_count = 1,
            |value| value.non_loopback_listener_count = 1,
            |value| value.container_bridge_route_count = 1,
        ];
        for mutate in mutations {
            let mut changed = input();
            mutate(&mut changed);
            assert_eq!(
                verify(changed),
                Err(DockerEndpointGuardError::RawEndpointExposure)
            );
        }
        let mut changed = input();
        changed.endpoint = native;
        assert_eq!(
            verify(changed),
            Err(DockerEndpointGuardError::RawEndpointExposure)
        );
    }

    #[test]
    fn process_mount_and_kernel_transport_mutations_fail_closed() {
        let process_mutations: &[fn(&mut GuardInput)] = &[
            |value| value.guard_uid = 0,
            |value| value.guard_uid = value.runtime_uid,
            |value| value.runtime_gid = 0,
            |value| value.guard_executable_sha256 = [0; 32],
            |value| value.guard_cgroup_identity_sha256 = [0; 32],
            |value| value.docker_socket_mount_count = 1,
            |value| value.workspace_mount_count = 1,
        ];
        for mutate in process_mutations {
            let mut changed = input();
            mutate(&mut changed);
            assert_eq!(
                verify(changed),
                Err(DockerEndpointGuardError::GuardProcessIdentity)
            );
        }
        let transport_mutations: &[fn(&mut GuardInput)] = &[
            |value| value.kernel_socket_identity_sha256 = [0; 32],
            |value| value.kernel_socket_owner_uid = 1000,
            |value| value.kernel_socket_group_gid = 1001,
            |value| value.kernel_socket_parent_mode = 0o770,
            |value| value.kernel_socket_mode = 0o600,
            |value| value.kernel_socket_peer_authenticated = false,
        ];
        for mutate in transport_mutations {
            let mut changed = input();
            mutate(&mut changed);
            assert_eq!(
                verify(changed),
                Err(DockerEndpointGuardError::KernelTransport)
            );
        }
    }

    #[test]
    fn profile_identity_mutations_fail_before_topology() {
        for mutate in [
            |value: &mut GuardInput| value.profile_sha256 = [0; 32],
            |value: &mut GuardInput| value.runner_image_digest = [0; 32],
        ] {
            let mut changed = input();
            mutate(&mut changed);
            assert_eq!(
                verify(changed),
                Err(DockerEndpointGuardError::ProfileIdentity)
            );
        }
    }
}
