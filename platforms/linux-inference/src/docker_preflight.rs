//! Fail-closed Docker Model Runner topology and drift preflight contract.

use std::fmt;
use std::net::{IpAddr, Ipv4Addr};

use crate::{
    DOCKER_GUARD_PROFILE_SHA256, DOCKER_MODEL_ARTIFACT_DIGEST, DOCKER_MODEL_RUNNER_BIND_HOST,
    DOCKER_MODEL_RUNNER_CONNECT_HOST, DOCKER_MODEL_RUNNER_IMAGE_DIGEST, DOCKER_MODEL_RUNNER_PORT,
    DOCKER_RUNTIME_PROFILE_SHA256,
};

const DOCKER_SOCKET_MODE: u32 = 0o660;
const MEMORY_BYTES: u64 = 64 * 1024 * 1024 * 1024;
const TASKS: u32 = 64;
const CPU_PERCENT: u16 = 3200;
const RUNTIME_SECONDS: u16 = 3600;
const OUTPUT_BYTES: u32 = 16 * 1024 * 1024;

/// Version of the exact Docker topology preflight contract.
pub const DOCKER_PREFLIGHT_CONTRACT_VERSION: u16 = 3;

/// Version of the complete trusted collector observation protocol.
pub const DOCKER_TOPOLOGY_COLLECTOR_PROTOCOL_VERSION: u16 = 1;

/// Stable content-free reason that Docker mode was refused before activation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockerPreflightError {
    /// The runtime or guard profile identity changed.
    ProfileIdentity,
    /// The observation was incomplete, stale, replayed, or collected by another binary.
    ObservationIdentity,
    /// Daemon privilege or runtime-user Docker authority changed.
    DaemonPrivilege,
    /// Docker socket identity, ownership, type, or mode changed.
    SocketOwnership,
    /// Raw or management API binding changed.
    ApiBinding,
    /// Runner/guard namespace placement or foreign reachability changed.
    ContainerReachability,
    /// Runner or model OCI identity changed.
    ImageIdentity,
    /// Container privilege, mounts, or resource limits changed.
    ResourceLimits,
    /// Tracking, acquisition, name resolution, registry, firewall, or egress state changed.
    ZeroEgress,
}

impl DockerPreflightError {
    /// Returns a stable refusal code without observed values.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ProfileIdentity => "docker-preflight.profile.identity",
            Self::ObservationIdentity => "docker-preflight.observation.identity",
            Self::DaemonPrivilege => "docker-preflight.daemon.privilege",
            Self::SocketOwnership => "docker-preflight.socket.ownership",
            Self::ApiBinding => "docker-preflight.api.binding",
            Self::ContainerReachability => "docker-preflight.container.reachability",
            Self::ImageIdentity => "docker-preflight.image.identity",
            Self::ResourceLimits => "docker-preflight.resources.invalid",
            Self::ZeroEgress => "docker-preflight.egress.nonzero",
        }
    }
}

impl fmt::Display for DockerPreflightError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for DockerPreflightError {}

/// Exact identities configured by a separate administrator before observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DockerPreflightBaseline {
    collector_executable_sha256: [u8; 32],
    daemon_executable_sha256: [u8; 32],
    daemon_socket_identity_sha256: [u8; 32],
    docker_socket_gid: u32,
    runtime_uid: u32,
    runtime_gid: u32,
    guard_uid: u32,
    private_namespace_sha256: [u8; 32],
    guard_executable_sha256: [u8; 32],
    guard_cgroup_sha256: [u8; 32],
}

impl DockerPreflightBaseline {
    /// Verifies the non-ambient identities against which one fresh observation is compared.
    #[allow(clippy::too_many_arguments)]
    pub fn verify(
        runtime_profile_sha256: [u8; 32],
        guard_profile_sha256: [u8; 32],
        collector_executable_sha256: [u8; 32],
        daemon_executable_sha256: [u8; 32],
        daemon_socket_identity_sha256: [u8; 32],
        docker_socket_gid: u32,
        runtime_uid: u32,
        runtime_gid: u32,
        guard_uid: u32,
        private_namespace_sha256: [u8; 32],
        guard_executable_sha256: [u8; 32],
        guard_cgroup_sha256: [u8; 32],
    ) -> Result<Self, DockerPreflightError> {
        if runtime_profile_sha256 != DOCKER_RUNTIME_PROFILE_SHA256
            || guard_profile_sha256 != DOCKER_GUARD_PROFILE_SHA256
        {
            return Err(DockerPreflightError::ProfileIdentity);
        }
        if collector_executable_sha256 == [0; 32]
            || daemon_executable_sha256 == [0; 32]
            || daemon_socket_identity_sha256 == [0; 32]
            || docker_socket_gid == 0
            || runtime_uid == 0
            || runtime_gid == 0
            || guard_uid == 0
            || runtime_uid == guard_uid
            || private_namespace_sha256 == [0; 32]
            || guard_executable_sha256 == [0; 32]
            || guard_cgroup_sha256 == [0; 32]
        {
            return Err(DockerPreflightError::ObservationIdentity);
        }
        Ok(Self {
            collector_executable_sha256,
            daemon_executable_sha256,
            daemon_socket_identity_sha256,
            docker_socket_gid,
            runtime_uid,
            runtime_gid,
            guard_uid,
            private_namespace_sha256,
            guard_executable_sha256,
            guard_cgroup_sha256,
        })
    }

    /// Returns the exact collector executable identity.
    #[must_use]
    pub const fn collector_executable_sha256(&self) -> &[u8; 32] {
        &self.collector_executable_sha256
    }

    /// Returns the exact Docker daemon executable identity.
    #[must_use]
    pub const fn daemon_executable_sha256(&self) -> &[u8; 32] {
        &self.daemon_executable_sha256
    }

    /// Returns the exact Docker socket object identity.
    #[must_use]
    pub const fn daemon_socket_identity_sha256(&self) -> &[u8; 32] {
        &self.daemon_socket_identity_sha256
    }

    /// Returns the exact Docker socket group.
    #[must_use]
    pub const fn docker_socket_gid(&self) -> u32 {
        self.docker_socket_gid
    }

    /// Returns the exact runtime user.
    #[must_use]
    pub const fn runtime_uid(&self) -> u32 {
        self.runtime_uid
    }

    /// Returns the exact runtime primary group.
    #[must_use]
    pub const fn runtime_gid(&self) -> u32 {
        self.runtime_gid
    }

    /// Returns the exact dedicated guard user.
    #[must_use]
    pub const fn guard_uid(&self) -> u32 {
        self.guard_uid
    }

    /// Returns the exact private namespace identity.
    #[must_use]
    pub const fn private_namespace_sha256(&self) -> &[u8; 32] {
        &self.private_namespace_sha256
    }

    /// Returns the exact guard executable identity.
    #[must_use]
    pub const fn guard_executable_sha256(&self) -> &[u8; 32] {
        &self.guard_executable_sha256
    }

    /// Returns the exact guard cgroup identity.
    #[must_use]
    pub const fn guard_cgroup_sha256(&self) -> &[u8; 32] {
        &self.guard_cgroup_sha256
    }
}

/// Complete daemon privilege and socket observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DockerDaemonObservation {
    daemon_executable_sha256: [u8; 32],
    daemon_uid: u32,
    rootless: bool,
    runtime_uid: u32,
    runtime_user_has_socket_group: bool,
    socket_identity_sha256: [u8; 32],
    socket_is_unix_stream: bool,
    socket_owner_uid: u32,
    socket_group_gid: u32,
    socket_mode: u32,
}

impl DockerDaemonObservation {
    /// Constructs a content-free daemon observation supplied by the trusted collector.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub const fn new(
        daemon_executable_sha256: [u8; 32],
        daemon_uid: u32,
        rootless: bool,
        runtime_uid: u32,
        runtime_user_has_socket_group: bool,
        socket_identity_sha256: [u8; 32],
        socket_is_unix_stream: bool,
        socket_owner_uid: u32,
        socket_group_gid: u32,
        socket_mode: u32,
    ) -> Self {
        Self {
            daemon_executable_sha256,
            daemon_uid,
            rootless,
            runtime_uid,
            runtime_user_has_socket_group,
            socket_identity_sha256,
            socket_is_unix_stream,
            socket_owner_uid,
            socket_group_gid,
            socket_mode,
        }
    }
}

/// Complete raw API binding observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DockerApiObservation {
    private_namespace_sha256: [u8; 32],
    runner_bind_host: IpAddr,
    guard_connect_host: Ipv4Addr,
    raw_port: u16,
    namespace_active_interface_count: u8,
    loopback_interface_up: bool,
    namespace_non_local_route_count: u8,
    raw_listener_count: u8,
    host_listener_count: u8,
    non_loopback_listener_count: u8,
    management_listener_count: u8,
}

impl DockerApiObservation {
    /// Constructs a bounded raw API observation.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub const fn new(
        private_namespace_sha256: [u8; 32],
        runner_bind_host: IpAddr,
        guard_connect_host: Ipv4Addr,
        raw_port: u16,
        namespace_active_interface_count: u8,
        loopback_interface_up: bool,
        namespace_non_local_route_count: u8,
        raw_listener_count: u8,
        host_listener_count: u8,
        non_loopback_listener_count: u8,
        management_listener_count: u8,
    ) -> Self {
        Self {
            private_namespace_sha256,
            runner_bind_host,
            guard_connect_host,
            raw_port,
            namespace_active_interface_count,
            loopback_interface_up,
            namespace_non_local_route_count,
            raw_listener_count,
            host_listener_count,
            non_loopback_listener_count,
            management_listener_count,
        }
    }
}

/// Complete runner, guard, mount, and foreign-reachability observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DockerContainerObservation {
    runner_count: u8,
    guard_count: u8,
    runner_and_guard_share_namespace: bool,
    guard_uid: u32,
    guard_executable_sha256: [u8; 32],
    guard_cgroup_sha256: [u8; 32],
    kernel_socket_owner_uid: u32,
    kernel_socket_group_gid: u32,
    kernel_socket_parent_mode: u32,
    kernel_socket_mode: u32,
    kernel_socket_peer_authentication: bool,
    host_route_count: u8,
    bridge_route_count: u8,
    foreign_reachable_peer_count: u8,
    workspace_mount_count: u8,
    credential_mount_count: u8,
    host_root_mount_count: u8,
    docker_socket_mount_count: u8,
    private_runtime_tmpfs: bool,
    model_content_store_writable: bool,
}

impl DockerContainerObservation {
    /// Constructs a content-free process, namespace, reachability, and mount observation.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub const fn new(
        runner_count: u8,
        guard_count: u8,
        runner_and_guard_share_namespace: bool,
        guard_uid: u32,
        guard_executable_sha256: [u8; 32],
        guard_cgroup_sha256: [u8; 32],
        kernel_socket_owner_uid: u32,
        kernel_socket_group_gid: u32,
        kernel_socket_parent_mode: u32,
        kernel_socket_mode: u32,
        kernel_socket_peer_authentication: bool,
        host_route_count: u8,
        bridge_route_count: u8,
        foreign_reachable_peer_count: u8,
        workspace_mount_count: u8,
        credential_mount_count: u8,
        host_root_mount_count: u8,
        docker_socket_mount_count: u8,
        private_runtime_tmpfs: bool,
        model_content_store_writable: bool,
    ) -> Self {
        Self {
            runner_count,
            guard_count,
            runner_and_guard_share_namespace,
            guard_uid,
            guard_executable_sha256,
            guard_cgroup_sha256,
            kernel_socket_owner_uid,
            kernel_socket_group_gid,
            kernel_socket_parent_mode,
            kernel_socket_mode,
            kernel_socket_peer_authentication,
            host_route_count,
            bridge_route_count,
            foreign_reachable_peer_count,
            workspace_mount_count,
            credential_mount_count,
            host_root_mount_count,
            docker_socket_mount_count,
            private_runtime_tmpfs,
            model_content_store_writable,
        }
    }
}

/// Exact immutable runner and model identities observed after launch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DockerImageObservation {
    runner_manifest_digest: [u8; 32],
    model_manifest_digest: [u8; 32],
    mutable_tag_used_for_admission: bool,
    image_repull_allowed: bool,
}

impl DockerImageObservation {
    /// Constructs an immutable image observation.
    #[must_use]
    pub const fn new(
        runner_manifest_digest: [u8; 32],
        model_manifest_digest: [u8; 32],
        mutable_tag_used_for_admission: bool,
        image_repull_allowed: bool,
    ) -> Self {
        Self {
            runner_manifest_digest,
            model_manifest_digest,
            mutable_tag_used_for_admission,
            image_repull_allowed,
        }
    }
}

/// Exact container privilege and resource-limit observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DockerResourceObservation {
    privileged: bool,
    capabilities_added: u8,
    no_new_privileges: bool,
    read_only_root: bool,
    memory_bytes: u64,
    tasks: u32,
    cpu_percent: u16,
    runtime_seconds: u16,
    output_bytes: u32,
    swap_bytes: u64,
    parallel_slots: u8,
}

impl DockerResourceObservation {
    /// Constructs an exact privilege and cgroup observation.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub const fn new(
        privileged: bool,
        capabilities_added: u8,
        no_new_privileges: bool,
        read_only_root: bool,
        memory_bytes: u64,
        tasks: u32,
        cpu_percent: u16,
        runtime_seconds: u16,
        output_bytes: u32,
        swap_bytes: u64,
        parallel_slots: u8,
    ) -> Self {
        Self {
            privileged,
            capabilities_added,
            no_new_privileges,
            read_only_root,
            memory_bytes,
            tasks,
            cpu_percent,
            runtime_seconds,
            output_bytes,
            swap_bytes,
            parallel_slots,
        }
    }
}

/// Exact acquisition, tracking, firewall, name-resolution, and byte observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DockerEgressObservation {
    do_not_track: bool,
    acquisition_allowed: bool,
    registry_access: bool,
    ambient_proxy: bool,
    ambient_dns: bool,
    firewall_default_deny: bool,
    egress_interface_count: u8,
    outbound_bytes: u64,
}

impl DockerEgressObservation {
    /// Constructs a complete zero-egress observation.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub const fn new(
        do_not_track: bool,
        acquisition_allowed: bool,
        registry_access: bool,
        ambient_proxy: bool,
        ambient_dns: bool,
        firewall_default_deny: bool,
        egress_interface_count: u8,
        outbound_bytes: u64,
    ) -> Self {
        Self {
            do_not_track,
            acquisition_allowed,
            registry_access,
            ambient_proxy,
            ambient_dns,
            firewall_default_deny,
            egress_interface_count,
            outbound_bytes,
        }
    }
}

/// One fresh complete observation made before Docker-mode activation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DockerTopologyObservation {
    collector_executable_sha256: [u8; 32],
    collector_protocol_version: u16,
    collector_uid: u32,
    session_identity_sha256: [u8; 32],
    complete: bool,
    fresh: bool,
    replayed: bool,
    daemon: DockerDaemonObservation,
    api: DockerApiObservation,
    containers: DockerContainerObservation,
    images: DockerImageObservation,
    resources: DockerResourceObservation,
    egress: DockerEgressObservation,
}

impl DockerTopologyObservation {
    /// Constructs one content-free topology snapshot from the future trusted collector.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub const fn new(
        collector_executable_sha256: [u8; 32],
        collector_protocol_version: u16,
        collector_uid: u32,
        session_identity_sha256: [u8; 32],
        complete: bool,
        fresh: bool,
        replayed: bool,
        daemon: DockerDaemonObservation,
        api: DockerApiObservation,
        containers: DockerContainerObservation,
        images: DockerImageObservation,
        resources: DockerResourceObservation,
        egress: DockerEgressObservation,
    ) -> Self {
        Self {
            collector_executable_sha256,
            collector_protocol_version,
            collector_uid,
            session_identity_sha256,
            complete,
            fresh,
            replayed,
            daemon,
            api,
            containers,
            images,
            resources,
            egress,
        }
    }
}

/// Unforgeable in-module proof that all declared preflight classes matched exactly.
pub struct DockerModeAdmission<'baseline> {
    baseline: &'baseline DockerPreflightBaseline,
    session_identity_sha256: [u8; 32],
}

impl DockerModeAdmission<'_> {
    /// Returns the fresh observation identity without exposing topology details.
    #[must_use]
    pub const fn session_identity_sha256(&self) -> &[u8; 32] {
        &self.session_identity_sha256
    }

    /// Returns the admitted runtime user without exposing Docker authority.
    #[must_use]
    pub const fn runtime_uid(&self) -> u32 {
        self.baseline.runtime_uid
    }
}

impl fmt::Debug for DockerModeAdmission<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DockerModeAdmission")
            .finish_non_exhaustive()
    }
}

/// Evaluates the complete Docker observation in a stable fail-closed order.
pub fn admit_docker_mode(
    baseline: &DockerPreflightBaseline,
    observed: DockerTopologyObservation,
) -> Result<DockerModeAdmission<'_>, DockerPreflightError> {
    if observed.collector_executable_sha256 != baseline.collector_executable_sha256
        || observed.collector_protocol_version != DOCKER_TOPOLOGY_COLLECTOR_PROTOCOL_VERSION
        || observed.collector_uid != 0
        || observed.session_identity_sha256 == [0; 32]
        || !observed.complete
        || !observed.fresh
        || observed.replayed
    {
        return Err(DockerPreflightError::ObservationIdentity);
    }
    if observed.daemon.daemon_executable_sha256 != baseline.daemon_executable_sha256
        || observed.daemon.daemon_uid != 0
        || observed.daemon.rootless
        || observed.daemon.runtime_uid != baseline.runtime_uid
        || observed.daemon.runtime_user_has_socket_group
    {
        return Err(DockerPreflightError::DaemonPrivilege);
    }
    if observed.daemon.socket_identity_sha256 != baseline.daemon_socket_identity_sha256
        || !observed.daemon.socket_is_unix_stream
        || observed.daemon.socket_owner_uid != 0
        || observed.daemon.socket_group_gid != baseline.docker_socket_gid
        || observed.daemon.socket_mode != DOCKER_SOCKET_MODE
    {
        return Err(DockerPreflightError::SocketOwnership);
    }
    if observed.api.private_namespace_sha256 != baseline.private_namespace_sha256
        || observed.api.runner_bind_host != DOCKER_MODEL_RUNNER_BIND_HOST
        || observed.api.guard_connect_host != DOCKER_MODEL_RUNNER_CONNECT_HOST
        || observed.api.raw_port != DOCKER_MODEL_RUNNER_PORT
        || observed.api.namespace_active_interface_count != 1
        || !observed.api.loopback_interface_up
        || observed.api.namespace_non_local_route_count != 0
        || observed.api.raw_listener_count != 1
        || observed.api.host_listener_count != 0
        || observed.api.non_loopback_listener_count != 0
        || observed.api.management_listener_count != 0
    {
        return Err(DockerPreflightError::ApiBinding);
    }
    if observed.containers.runner_count != 1
        || observed.containers.guard_count != 1
        || !observed.containers.runner_and_guard_share_namespace
        || observed.containers.guard_uid != baseline.guard_uid
        || observed.containers.guard_executable_sha256 != baseline.guard_executable_sha256
        || observed.containers.guard_cgroup_sha256 != baseline.guard_cgroup_sha256
        || observed.containers.kernel_socket_owner_uid != baseline.guard_uid
        || observed.containers.kernel_socket_group_gid != baseline.runtime_gid
        || observed.containers.kernel_socket_parent_mode != 0o710
        || observed.containers.kernel_socket_mode != 0o660
        || !observed.containers.kernel_socket_peer_authentication
        || observed.containers.host_route_count != 0
        || observed.containers.bridge_route_count != 0
        || observed.containers.foreign_reachable_peer_count != 0
    {
        return Err(DockerPreflightError::ContainerReachability);
    }
    if observed.images.runner_manifest_digest != DOCKER_MODEL_RUNNER_IMAGE_DIGEST
        || observed.images.model_manifest_digest != DOCKER_MODEL_ARTIFACT_DIGEST
        || observed.images.mutable_tag_used_for_admission
        || observed.images.image_repull_allowed
    {
        return Err(DockerPreflightError::ImageIdentity);
    }
    if observed.containers.workspace_mount_count != 0
        || observed.containers.credential_mount_count != 0
        || observed.containers.host_root_mount_count != 0
        || observed.containers.docker_socket_mount_count != 0
        || !observed.containers.private_runtime_tmpfs
        || observed.containers.model_content_store_writable
        || observed.resources.privileged
        || observed.resources.capabilities_added != 0
        || !observed.resources.no_new_privileges
        || !observed.resources.read_only_root
        || observed.resources.memory_bytes != MEMORY_BYTES
        || observed.resources.tasks != TASKS
        || observed.resources.cpu_percent != CPU_PERCENT
        || observed.resources.runtime_seconds != RUNTIME_SECONDS
        || observed.resources.output_bytes != OUTPUT_BYTES
        || observed.resources.swap_bytes != 0
        || observed.resources.parallel_slots != 1
    {
        return Err(DockerPreflightError::ResourceLimits);
    }
    if !observed.egress.do_not_track
        || observed.egress.acquisition_allowed
        || observed.egress.registry_access
        || observed.egress.ambient_proxy
        || observed.egress.ambient_dns
        || !observed.egress.firewall_default_deny
        || observed.egress.egress_interface_count != 0
        || observed.egress.outbound_bytes != 0
    {
        return Err(DockerPreflightError::ZeroEgress);
    }
    Ok(DockerModeAdmission {
        baseline,
        session_identity_sha256: observed.session_identity_sha256,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline() -> DockerPreflightBaseline {
        DockerPreflightBaseline::verify(
            DOCKER_RUNTIME_PROFILE_SHA256,
            DOCKER_GUARD_PROFILE_SHA256,
            [1; 32],
            [2; 32],
            [3; 32],
            971,
            1000,
            1000,
            991,
            [4; 32],
            [5; 32],
            [6; 32],
        )
        .expect("exact preflight baseline")
    }

    fn observation() -> DockerTopologyObservation {
        DockerTopologyObservation::new(
            [1; 32],
            DOCKER_TOPOLOGY_COLLECTOR_PROTOCOL_VERSION,
            0,
            [7; 32],
            true,
            true,
            false,
            DockerDaemonObservation::new(
                [2; 32], 0, false, 1000, false, [3; 32], true, 0, 971, 0o660,
            ),
            DockerApiObservation::new(
                [4; 32],
                DOCKER_MODEL_RUNNER_BIND_HOST,
                Ipv4Addr::LOCALHOST,
                12_434,
                1,
                true,
                0,
                1,
                0,
                0,
                0,
            ),
            DockerContainerObservation::new(
                1, 1, true, 991, [5; 32], [6; 32], 991, 1000, 0o710, 0o660, true, 0, 0, 0, 0, 0, 0,
                0, true, false,
            ),
            DockerImageObservation::new(
                DOCKER_MODEL_RUNNER_IMAGE_DIGEST,
                DOCKER_MODEL_ARTIFACT_DIGEST,
                false,
                false,
            ),
            DockerResourceObservation::new(
                false,
                0,
                true,
                true,
                MEMORY_BYTES,
                TASKS,
                CPU_PERCENT,
                RUNTIME_SECONDS,
                OUTPUT_BYTES,
                0,
                1,
            ),
            DockerEgressObservation::new(true, false, false, false, false, true, 0, 0),
        )
    }

    #[test]
    fn exact_complete_fresh_observation_is_the_only_admission() {
        let baseline = baseline();
        let admitted = admit_docker_mode(&baseline, observation()).expect("exact admission");
        assert_eq!(admitted.session_identity_sha256(), &[7; 32]);
        assert_eq!(admitted.runtime_uid(), 1000);
        assert_eq!(format!("{admitted:?}"), "DockerModeAdmission { .. }");
    }

    #[test]
    fn profile_and_observation_identity_drift_refuse_first() {
        assert_eq!(
            DockerPreflightBaseline::verify(
                [0; 32],
                DOCKER_GUARD_PROFILE_SHA256,
                [1; 32],
                [2; 32],
                [3; 32],
                971,
                1000,
                1000,
                991,
                [4; 32],
                [5; 32],
                [6; 32],
            ),
            Err(DockerPreflightError::ProfileIdentity)
        );
        let baseline = baseline();
        for mutate in [
            |value: &mut DockerTopologyObservation| value.collector_executable_sha256 = [8; 32],
            |value: &mut DockerTopologyObservation| value.collector_protocol_version = 2,
            |value: &mut DockerTopologyObservation| value.collector_uid = 1000,
            |value: &mut DockerTopologyObservation| value.session_identity_sha256 = [0; 32],
            |value: &mut DockerTopologyObservation| value.complete = false,
            |value: &mut DockerTopologyObservation| value.fresh = false,
            |value: &mut DockerTopologyObservation| value.replayed = true,
        ] {
            let mut changed = observation();
            mutate(&mut changed);
            assert_eq!(
                admit_docker_mode(&baseline, changed).unwrap_err(),
                DockerPreflightError::ObservationIdentity
            );
        }
    }

    #[test]
    fn daemon_and_socket_drift_are_distinct_refusals() {
        let baseline = baseline();
        for mutate in [
            |value: &mut DockerTopologyObservation| value.daemon.daemon_executable_sha256 = [8; 32],
            |value: &mut DockerTopologyObservation| value.daemon.daemon_uid = 1000,
            |value: &mut DockerTopologyObservation| value.daemon.rootless = true,
            |value: &mut DockerTopologyObservation| value.daemon.runtime_uid = 1001,
            |value: &mut DockerTopologyObservation| {
                value.daemon.runtime_user_has_socket_group = true
            },
        ] {
            let mut changed = observation();
            mutate(&mut changed);
            assert_eq!(
                admit_docker_mode(&baseline, changed).unwrap_err(),
                DockerPreflightError::DaemonPrivilege
            );
        }
        for mutate in [
            |value: &mut DockerTopologyObservation| value.daemon.socket_identity_sha256 = [8; 32],
            |value: &mut DockerTopologyObservation| value.daemon.socket_is_unix_stream = false,
            |value: &mut DockerTopologyObservation| value.daemon.socket_owner_uid = 1000,
            |value: &mut DockerTopologyObservation| value.daemon.socket_group_gid = 972,
            |value: &mut DockerTopologyObservation| value.daemon.socket_mode = 0o666,
        ] {
            let mut changed = observation();
            mutate(&mut changed);
            assert_eq!(
                admit_docker_mode(&baseline, changed).unwrap_err(),
                DockerPreflightError::SocketOwnership
            );
        }
    }

    #[test]
    fn api_and_reachability_drift_refuse_without_fallback() {
        let baseline = baseline();
        for mutate in [
            |value: &mut DockerTopologyObservation| value.api.private_namespace_sha256 = [8; 32],
            |value: &mut DockerTopologyObservation| {
                value.api.runner_bind_host = IpAddr::V4(Ipv4Addr::UNSPECIFIED)
            },
            |value: &mut DockerTopologyObservation| {
                value.api.guard_connect_host = Ipv4Addr::UNSPECIFIED
            },
            |value: &mut DockerTopologyObservation| value.api.raw_port = 12_435,
            |value: &mut DockerTopologyObservation| value.api.namespace_active_interface_count = 2,
            |value: &mut DockerTopologyObservation| value.api.loopback_interface_up = false,
            |value: &mut DockerTopologyObservation| value.api.namespace_non_local_route_count = 1,
            |value: &mut DockerTopologyObservation| value.api.raw_listener_count = 0,
            |value: &mut DockerTopologyObservation| value.api.host_listener_count = 1,
            |value: &mut DockerTopologyObservation| value.api.non_loopback_listener_count = 1,
            |value: &mut DockerTopologyObservation| value.api.management_listener_count = 1,
        ] {
            let mut changed = observation();
            mutate(&mut changed);
            assert_eq!(
                admit_docker_mode(&baseline, changed).unwrap_err(),
                DockerPreflightError::ApiBinding
            );
        }
        for mutate in [
            |value: &mut DockerTopologyObservation| value.containers.runner_count = 2,
            |value: &mut DockerTopologyObservation| value.containers.guard_count = 0,
            |value: &mut DockerTopologyObservation| {
                value.containers.runner_and_guard_share_namespace = false
            },
            |value: &mut DockerTopologyObservation| value.containers.guard_uid = 992,
            |value: &mut DockerTopologyObservation| {
                value.containers.guard_executable_sha256 = [8; 32]
            },
            |value: &mut DockerTopologyObservation| value.containers.guard_cgroup_sha256 = [8; 32],
            |value: &mut DockerTopologyObservation| value.containers.kernel_socket_owner_uid = 1000,
            |value: &mut DockerTopologyObservation| value.containers.kernel_socket_group_gid = 1001,
            |value: &mut DockerTopologyObservation| {
                value.containers.kernel_socket_parent_mode = 0o770
            },
            |value: &mut DockerTopologyObservation| value.containers.kernel_socket_mode = 0o600,
            |value: &mut DockerTopologyObservation| {
                value.containers.kernel_socket_peer_authentication = false
            },
            |value: &mut DockerTopologyObservation| value.containers.host_route_count = 1,
            |value: &mut DockerTopologyObservation| {
                value.containers.foreign_reachable_peer_count = 1
            },
            |value: &mut DockerTopologyObservation| value.containers.bridge_route_count = 1,
        ] {
            let mut changed = observation();
            mutate(&mut changed);
            assert_eq!(
                admit_docker_mode(&baseline, changed).unwrap_err(),
                DockerPreflightError::ContainerReachability
            );
        }
    }

    #[test]
    fn image_resource_and_egress_drift_are_terminal() {
        let baseline = baseline();
        for mutate in [
            |value: &mut DockerTopologyObservation| value.images.runner_manifest_digest = [9; 32],
            |value: &mut DockerTopologyObservation| value.images.model_manifest_digest = [9; 32],
            |value: &mut DockerTopologyObservation| {
                value.images.mutable_tag_used_for_admission = true
            },
            |value: &mut DockerTopologyObservation| value.images.image_repull_allowed = true,
        ] {
            let mut changed = observation();
            mutate(&mut changed);
            assert_eq!(
                admit_docker_mode(&baseline, changed).unwrap_err(),
                DockerPreflightError::ImageIdentity
            );
        }
        for mutate in [
            |value: &mut DockerTopologyObservation| value.resources.memory_bytes -= 1,
            |value: &mut DockerTopologyObservation| value.resources.privileged = true,
            |value: &mut DockerTopologyObservation| value.resources.capabilities_added = 1,
            |value: &mut DockerTopologyObservation| value.resources.no_new_privileges = false,
            |value: &mut DockerTopologyObservation| value.resources.read_only_root = false,
            |value: &mut DockerTopologyObservation| value.resources.tasks -= 1,
            |value: &mut DockerTopologyObservation| value.resources.cpu_percent -= 1,
            |value: &mut DockerTopologyObservation| value.resources.runtime_seconds -= 1,
            |value: &mut DockerTopologyObservation| value.resources.output_bytes -= 1,
            |value: &mut DockerTopologyObservation| value.resources.swap_bytes = 1,
            |value: &mut DockerTopologyObservation| value.resources.parallel_slots = 2,
            |value: &mut DockerTopologyObservation| value.containers.workspace_mount_count = 1,
            |value: &mut DockerTopologyObservation| value.containers.credential_mount_count = 1,
            |value: &mut DockerTopologyObservation| value.containers.host_root_mount_count = 1,
            |value: &mut DockerTopologyObservation| value.containers.docker_socket_mount_count = 1,
            |value: &mut DockerTopologyObservation| value.containers.private_runtime_tmpfs = false,
            |value: &mut DockerTopologyObservation| {
                value.containers.model_content_store_writable = true
            },
        ] {
            let mut changed = observation();
            mutate(&mut changed);
            assert_eq!(
                admit_docker_mode(&baseline, changed).unwrap_err(),
                DockerPreflightError::ResourceLimits
            );
        }
        for mutate in [
            |value: &mut DockerTopologyObservation| value.egress.do_not_track = false,
            |value: &mut DockerTopologyObservation| value.egress.acquisition_allowed = true,
            |value: &mut DockerTopologyObservation| value.egress.firewall_default_deny = false,
            |value: &mut DockerTopologyObservation| value.egress.registry_access = true,
            |value: &mut DockerTopologyObservation| value.egress.ambient_proxy = true,
            |value: &mut DockerTopologyObservation| value.egress.ambient_dns = true,
            |value: &mut DockerTopologyObservation| value.egress.egress_interface_count = 1,
            |value: &mut DockerTopologyObservation| value.egress.outbound_bytes = 1,
        ] {
            let mut changed = observation();
            mutate(&mut changed);
            assert_eq!(
                admit_docker_mode(&baseline, changed).unwrap_err(),
                DockerPreflightError::ZeroEgress
            );
        }
    }
}
