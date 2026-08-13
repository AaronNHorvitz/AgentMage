//! Root-only production collection from explicit Linux and Docker targets.

use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::docker_http_observer::{
    DockerContainerInspect, DockerDaemonObservationClient, DockerHttpObserverError, DockerMount,
};
use crate::docker_linux_observer::{
    LinuxObserverError, count_processes_by_executable, hash_regular_file, observe_current_network,
    observe_directory, observe_network_namespace, observe_process, observe_unix_socket,
};
use crate::docker_topology_collector::{
    ApiInput, BaselineInput, ContainersInput, DaemonInput, DockerCollectorInput,
    DockerCollectorOutput, EgressInput, ImagesInput, ResourcesInput, validate_topology,
};
use crate::{
    DOCKER_COLLECTOR_MAX_INPUT_BYTES, DOCKER_GUARD_PROFILE_SHA256,
    DOCKER_MODEL_ARTIFACT_DIGEST_HEX, DOCKER_MODEL_GGUF_SHA256, DOCKER_MODEL_PROJECTOR_SHA256,
    DOCKER_MODEL_RUNNER_IMAGE_DIGEST_HEX, DOCKER_MODEL_RUNNER_PORT,
    DOCKER_PREFLIGHT_CONTRACT_VERSION, DOCKER_RUNTIME_PROFILE_SHA256,
    DOCKER_TOPOLOGY_COLLECTOR_PROTOCOL_VERSION, DockerTopologyCollectorError,
};

/// Maximum accepted explicit-target request size for one live observation.
pub const DOCKER_LIVE_OBSERVER_MAX_INPUT_BYTES: u64 = 64 * 1024;

const MAX_OBSERVATION_AGE_SECONDS: u64 = 60;
const MAX_OS_RELEASE_BYTES: u64 = 64 * 1024;
const OS_RELEASE_PATH: &str = "/etc/os-release";
const GUARD_SOCKET_PATH: &str = "/run/agentmage-dmr/guard.sock";
const REPLAY_DIRECTORY: &str = "/run/agentmage-dmr/observations";
const RUNTIME_SECONDS: u16 = 3600;
const OUTPUT_BYTES: u32 = 16 * 1024 * 1024;
const PARALLEL_SLOTS: u8 = 1;
const MAX_MODEL_PAYLOAD_BYTES: u64 = 32 * 1024 * 1024 * 1024;

/// Stable content-free refusal from the production live collector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DockerLiveCollectorError {
    code: &'static str,
}

impl DockerLiveCollectorError {
    const fn new(code: &'static str) -> Self {
        Self { code }
    }

    fn linux(error: LinuxObserverError) -> Self {
        Self::new(error.code())
    }

    fn docker(error: DockerHttpObserverError) -> Self {
        Self::new(error.code())
    }

    fn validation(error: DockerTopologyCollectorError) -> Self {
        Self::new(error.code())
    }

    /// Returns the stable redacted refusal code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        self.code
    }
}

impl fmt::Display for DockerLiveCollectorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for DockerLiveCollectorError {}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct DockerLiveCollectorRequest {
    protocol_version: u16,
    preflight_contract_version: u16,
    source_revision: String,
    os_release_sha256: String,
    observation_started_unix_seconds: u64,
    session_identity_sha256: String,
    collector_executable_sha256: String,
    runtime_pid: i32,
    runtime_start_time_ticks: u64,
    guard_pid: i32,
    guard_start_time_ticks: u64,
    runner_container_id: String,
    baseline: BaselineInput,
}

/// Complete bounded production observation and its admission result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DockerLiveCollectorOutput {
    /// Stable evidence record type.
    pub record_type: &'static str,
    /// Exact observer protocol version.
    pub protocol_version: u16,
    /// Exact preflight contract version.
    pub preflight_contract_version: u16,
    /// Source revision supplied by the source-bound launcher.
    pub source_revision: String,
    /// Exact operating-system release record identity.
    pub os_release_sha256: String,
    /// Request creation time used by the freshness check.
    pub observation_started_unix_seconds: u64,
    /// Executing collector's verified digest.
    pub collector_executable_sha256: String,
    /// Complete bounded raw topology observation.
    pub observation: DockerCollectorInput,
    /// Content-free terminal preflight admission.
    pub admission: DockerCollectorOutput,
}

/// Collects one fresh topology from explicit targets and refuses non-administrators first.
pub fn observe_live_topology_from_reader(
    reader: impl Read,
    effective_uid: u32,
) -> Result<DockerLiveCollectorOutput, DockerLiveCollectorError> {
    if effective_uid != 0 {
        return Err(DockerLiveCollectorError::new(
            "docker-collector.administrator-identity",
        ));
    }
    let mut bytes = Vec::new();
    reader
        .take(DOCKER_LIVE_OBSERVER_MAX_INPUT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| DockerLiveCollectorError::new("docker-collector.input"))?;
    if bytes.is_empty() || bytes.len() as u64 > DOCKER_LIVE_OBSERVER_MAX_INPUT_BYTES {
        return Err(DockerLiveCollectorError::new("docker-collector.input"));
    }
    let request: DockerLiveCollectorRequest = serde_json::from_slice(&bytes)
        .map_err(|_| DockerLiveCollectorError::new("docker-collector.input"))?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| DockerLiveCollectorError::new("docker-collector.freshness"))?
        .as_secs();
    observe_live_topology(request, now, true)
}

fn observe_live_topology(
    request: DockerLiveCollectorRequest,
    now: u64,
    record_replay: bool,
) -> Result<DockerLiveCollectorOutput, DockerLiveCollectorError> {
    validate_request(&request, now)?;
    let collector_process = crate::docker_linux_observer::observe_current_process(
        i32::try_from(std::process::id())
            .map_err(|_| DockerLiveCollectorError::new("docker-collector.input"))?,
    )
    .map_err(DockerLiveCollectorError::linux)?;
    if hex(&collector_process.executable_sha256) != request.collector_executable_sha256 {
        return Err(DockerLiveCollectorError::new("docker-collector.input"));
    }
    let os_release = hash_regular_file(Path::new(OS_RELEASE_PATH), MAX_OS_RELEASE_BYTES)
        .map_err(DockerLiveCollectorError::linux)?;
    if hex(&os_release) != request.os_release_sha256 {
        return Err(DockerLiveCollectorError::new("docker-collector.input"));
    }

    let daemon =
        DockerDaemonObservationClient::connect().map_err(DockerLiveCollectorError::docker)?;
    let info = daemon.info().map_err(DockerLiveCollectorError::docker)?;
    let container = daemon
        .container(&request.runner_container_id)
        .map_err(DockerLiveCollectorError::docker)?;
    if container.id != request.runner_container_id
        || !container.state.running
        || container.state.pid <= 1
    {
        return Err(DockerLiveCollectorError::docker(
            DockerHttpObserverError::DockerState,
        ));
    }
    let running = daemon
        .running_containers()
        .map_err(DockerLiveCollectorError::docker)?;
    let image = daemon
        .image(&container.image)
        .map_err(DockerLiveCollectorError::docker)?;

    let runtime = observe_process(request.runtime_pid, request.runtime_start_time_ticks)
        .map_err(DockerLiveCollectorError::linux)?;
    let guard = observe_process(request.guard_pid, request.guard_start_time_ticks)
        .map_err(DockerLiveCollectorError::linux)?;
    let runner = crate::docker_linux_observer::observe_current_process(container.state.pid)
        .map_err(DockerLiveCollectorError::linux)?;
    let host_network = observe_current_network(DOCKER_MODEL_RUNNER_PORT)
        .map_err(DockerLiveCollectorError::linux)?;
    let private_network = observe_network_namespace(container.state.pid, DOCKER_MODEL_RUNNER_PORT)
        .map_err(DockerLiveCollectorError::linux)?;
    let guard_socket = observe_unix_socket(Path::new(GUARD_SOCKET_PATH))
        .map_err(DockerLiveCollectorError::linux)?;
    let guard_parent = observe_directory(
        Path::new(GUARD_SOCKET_PATH)
            .parent()
            .ok_or_else(|| DockerLiveCollectorError::new("docker-collector.input"))?,
    )
    .map_err(DockerLiveCollectorError::linux)?;

    let daemon_executable = parse_sha256(&request.baseline.daemon_executable_sha256)?;
    let daemon_socket_identity = parse_sha256(&request.baseline.daemon_socket_identity_sha256)?;
    let namespace_identity = parse_sha256(&request.baseline.private_namespace_sha256)?;
    let guard_executable = parse_sha256(&request.baseline.guard_executable_sha256)?;
    let guard_cgroup = parse_sha256(&request.baseline.guard_cgroup_sha256)?;

    if daemon.daemon().executable_sha256 != daemon_executable
        || daemon.socket().identity_sha256 != daemon_socket_identity
        || runtime.uid != request.baseline.runtime_uid
        || runtime.gid != request.baseline.runtime_gid
        || guard.uid != request.baseline.guard_uid
        || guard.gid != request.baseline.runtime_gid
        || guard.executable_sha256 != guard_executable
        || guard.cgroup_sha256 != guard_cgroup
        || guard.effective_capabilities != 0
        || !guard.no_new_privileges
        || runner.network_namespace_sha256 != namespace_identity
        || guard.network_namespace_sha256 != namespace_identity
        || guard.mount_namespace_sha256 == runner.mount_namespace_sha256
        || guard_parent.owner_uid != request.baseline.guard_uid
        || guard_parent.group_gid != request.baseline.runtime_gid
    {
        return Err(DockerLiveCollectorError::new("docker-collector.input"));
    }
    verify_model_store(container.state.pid)?;

    let observation = build_observation(
        &request,
        &container,
        &running,
        &image.repo_digests,
        &runtime.groups,
        &guard,
        &runner,
        daemon.daemon(),
        daemon.socket(),
        &info.security_options,
        host_network,
        private_network,
        guard_socket,
        guard_parent,
        count_processes_by_executable(guard.executable_sha256)
            .map_err(DockerLiveCollectorError::linux)?,
    )?;
    let admission =
        validate_topology(observation.clone()).map_err(DockerLiveCollectorError::validation)?;
    if record_replay {
        record_session_once(&request.session_identity_sha256)?;
    }
    Ok(DockerLiveCollectorOutput {
        record_type: "agentmage_docker_live_topology_observation",
        protocol_version: DOCKER_TOPOLOGY_COLLECTOR_PROTOCOL_VERSION,
        preflight_contract_version: DOCKER_PREFLIGHT_CONTRACT_VERSION,
        source_revision: request.source_revision,
        os_release_sha256: request.os_release_sha256,
        observation_started_unix_seconds: request.observation_started_unix_seconds,
        collector_executable_sha256: request.collector_executable_sha256,
        observation,
        admission,
    })
}

#[allow(clippy::too_many_arguments)]
fn build_observation(
    request: &DockerLiveCollectorRequest,
    container: &DockerContainerInspect,
    running: &[crate::docker_http_observer::DockerContainerSummary],
    repo_digests: &[String],
    runtime_groups: &[u32],
    guard: &crate::docker_linux_observer::ProcessObservation,
    runner: &crate::docker_linux_observer::ProcessObservation,
    daemon: &crate::docker_linux_observer::ProcessObservation,
    daemon_socket: &crate::docker_linux_observer::UnixSocketObservation,
    security_options: &[String],
    host_network: crate::docker_linux_observer::NetworkObservation,
    private_network: crate::docker_linux_observer::NetworkObservation,
    guard_socket: crate::docker_linux_observer::UnixSocketObservation,
    guard_parent: crate::docker_linux_observer::DirectoryObservation,
    guard_count: u8,
) -> Result<DockerCollectorInput, DockerLiveCollectorError> {
    let runner_digest = format!("sha256:{DOCKER_MODEL_RUNNER_IMAGE_DIGEST_HEX}");
    let runner_count = u8::try_from(
        running
            .iter()
            .filter(|item| item.image_id == container.image)
            .count(),
    )
    .map_err(|_| DockerLiveCollectorError::new("docker-collector.input"))?;
    if !running
        .iter()
        .any(|item| item.id == request.runner_container_id)
        || image_digest_absent(repo_digests, &runner_digest)
    {
        return Err(DockerLiveCollectorError::docker(
            DockerHttpObserverError::DockerState,
        ));
    }
    let model_label = container
        .config
        .labels
        .get("io.agentmage.model.manifest-digest")
        .cloned()
        .unwrap_or_default();
    let runtime_seconds = label_number(&container.config.labels, "io.agentmage.runtime-seconds");
    let output_bytes = label_number(&container.config.labels, "io.agentmage.output-bytes");
    let parallel_slots = label_number(&container.config.labels, "io.agentmage.parallel-slots");
    let image_repull_allowed = container
        .config
        .labels
        .get("io.agentmage.image-repull")
        .is_none_or(|value| value != "false");
    let mount_state = classify_mounts(&container.mounts, container.host_config.tmpfs.as_ref());
    let rootless = security_options
        .iter()
        .any(|value| value.to_ascii_lowercase().contains("rootless"));
    let no_new_privileges = runner.no_new_privileges
        && container
            .host_config
            .security_opt
            .as_ref()
            .is_some_and(|values| values.iter().any(|value| value == "no-new-privileges"));
    let proxy = container.config.env.iter().any(|value| {
        let key = value.split_once('=').map_or(value.as_str(), |(key, _)| key);
        matches!(
            key.to_ascii_lowercase().as_str(),
            "http_proxy" | "https_proxy" | "all_proxy" | "no_proxy"
        )
    });
    let do_not_track = container
        .config
        .env
        .iter()
        .filter(|value| value.as_str() == "DO_NOT_TRACK=1")
        .count()
        == 1;
    let network_closed = container.host_config.network_mode == "none"
        && private_network.active_interface_count == 1
        && private_network.loopback_up
        && private_network.non_loopback_interface_count == 0
        && private_network.non_local_route_count == 0;
    let privileged = container.host_config.privileged
        || matches!(container.config.user.as_str(), "" | "0" | "root")
        || runner.effective_capabilities != 0
        || container
            .host_config
            .port_bindings
            .as_ref()
            .is_some_and(|value| {
                !value.is_null() && value.as_object().is_none_or(|object| !object.is_empty())
            });
    let cap_add = container.host_config.cap_add.as_ref().map_or(0, Vec::len);
    let swap_bytes = if container.host_config.memory_swap >= 0
        && u64::try_from(container.host_config.memory_swap).ok()
            == Some(container.host_config.memory)
    {
        0
    } else {
        u64::MAX
    };
    let cpu_percent =
        u16::try_from(container.host_config.nano_cpus / 10_000_000).unwrap_or(u16::MAX);
    let tasks = container
        .host_config
        .pids_limit
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(0);
    let runner_bind_host = if private_network.raw_wildcard_v6_listener_count == 1
        && private_network.raw_wildcard_v4_listener_count == 0
        && private_network.raw_listener_count == 1
    {
        "::"
    } else {
        "::1"
    };
    Ok(DockerCollectorInput {
        protocol_version: DOCKER_TOPOLOGY_COLLECTOR_PROTOCOL_VERSION,
        preflight_contract_version: DOCKER_PREFLIGHT_CONTRACT_VERSION,
        collector_uid: 0,
        collector_executable_sha256: request.collector_executable_sha256.clone(),
        session_identity_sha256: request.session_identity_sha256.clone(),
        complete: true,
        fresh: true,
        replayed: false,
        baseline: request.baseline.clone(),
        daemon: DaemonInput {
            daemon_executable_sha256: hex(&daemon.executable_sha256),
            daemon_uid: daemon.uid,
            rootless,
            runtime_uid: request.baseline.runtime_uid,
            runtime_user_has_socket_group: runtime_groups
                .contains(&request.baseline.docker_socket_gid),
            socket_identity_sha256: hex(&daemon_socket.identity_sha256),
            socket_is_unix_stream: true,
            socket_owner_uid: daemon_socket.owner_uid,
            socket_group_gid: daemon_socket.group_gid,
            socket_mode: daemon_socket.mode,
        },
        api: ApiInput {
            private_namespace_sha256: hex(&runner.network_namespace_sha256),
            runner_bind_host: runner_bind_host.into(),
            guard_connect_host: "127.0.0.1".into(),
            raw_port: DOCKER_MODEL_RUNNER_PORT,
            namespace_active_interface_count: private_network.active_interface_count,
            loopback_interface_up: private_network.loopback_up,
            namespace_non_local_route_count: private_network.non_local_route_count,
            raw_listener_count: private_network.raw_listener_count,
            host_listener_count: host_network.raw_listener_count,
            non_loopback_listener_count: private_network.non_loopback_listener_count,
            management_listener_count: private_network.management_listener_count,
        },
        containers: ContainersInput {
            runner_count,
            guard_count,
            runner_and_guard_share_namespace: guard.network_namespace_sha256
                == runner.network_namespace_sha256,
            guard_uid: guard.uid,
            guard_executable_sha256: hex(&guard.executable_sha256),
            guard_cgroup_sha256: hex(&guard.cgroup_sha256),
            kernel_socket_owner_uid: guard_socket.owner_uid,
            kernel_socket_group_gid: guard_socket.group_gid,
            kernel_socket_parent_mode: guard_parent.mode,
            kernel_socket_mode: guard_socket.mode,
            kernel_socket_peer_authentication: true,
            host_route_count: private_network.non_local_route_count,
            bridge_route_count: private_network.non_loopback_interface_count,
            foreign_reachable_peer_count: private_network.non_loopback_interface_count,
            workspace_mount_count: 0,
            credential_mount_count: 0,
            host_root_mount_count: mount_state.forbidden,
            docker_socket_mount_count: mount_state.docker_socket,
            private_runtime_tmpfs: mount_state.private_runtime_tmpfs,
            model_content_store_writable: !mount_state.model_content_store_present
                || mount_state.model_content_store_writable,
        },
        images: ImagesInput {
            runner_manifest_digest: runner_digest.clone(),
            model_manifest_digest: model_label,
            mutable_tag_used_for_admission: container.config.image != runner_digest,
            image_repull_allowed,
        },
        resources: ResourcesInput {
            privileged,
            capabilities_added: u8::try_from(cap_add).unwrap_or(u8::MAX),
            no_new_privileges,
            read_only_root: container.host_config.readonly_rootfs,
            memory_bytes: container.host_config.memory,
            tasks,
            cpu_percent,
            runtime_seconds: u16::try_from(runtime_seconds).unwrap_or(0),
            output_bytes: u32::try_from(output_bytes).unwrap_or(0),
            swap_bytes,
            parallel_slots: u8::try_from(parallel_slots).unwrap_or(0),
        },
        egress: EgressInput {
            do_not_track,
            acquisition_allowed: !network_closed,
            registry_access: !network_closed,
            ambient_proxy: proxy,
            ambient_dns: !network_closed,
            firewall_default_deny: network_closed,
            egress_interface_count: private_network.non_loopback_interface_count,
            outbound_bytes: private_network.outbound_bytes,
        },
    })
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct MountState {
    private_runtime_tmpfs: bool,
    model_content_store_present: bool,
    model_content_store_writable: bool,
    docker_socket: u8,
    forbidden: u8,
}

fn classify_mounts(
    mounts: &[DockerMount],
    tmpfs: Option<&std::collections::BTreeMap<String, String>>,
) -> MountState {
    let mut state = MountState::default();
    for mount in mounts {
        if mount.destination == "/models" && mount.kind == "volume" {
            state.model_content_store_present = true;
            state.model_content_store_writable = mount.read_write;
        } else if mount.destination == "/run/docker.sock"
            || mount.source == "/run/docker.sock"
            || mount.source == "/var/run/docker.sock"
        {
            state.docker_socket = state.docker_socket.saturating_add(1);
        } else {
            state.forbidden = state.forbidden.saturating_add(1);
        }
    }
    if let Some(tmpfs) = tmpfs {
        for (destination, options) in tmpfs {
            if destination == "/run" && exact_private_runtime_tmpfs(options) {
                state.private_runtime_tmpfs = true;
            } else {
                state.forbidden = state.forbidden.saturating_add(1);
            }
        }
    }
    state
}

fn exact_private_runtime_tmpfs(options: &str) -> bool {
    let tokens = options
        .split(',')
        .collect::<std::collections::BTreeSet<_>>();
    tokens == std::collections::BTreeSet::from(["rw", "nosuid", "nodev", "noexec", "size=64m"])
}

fn verify_model_store(runner_pid: i32) -> Result<(), DockerLiveCollectorError> {
    let root = PathBuf::from(format!(
        "/proc/{runner_pid}/root/models/bundles/sha256/{DOCKER_MODEL_ARTIFACT_DIGEST_HEX}/model"
    ));
    let model = hash_regular_file(
        &root.join("gemma-4-E4B-it-Q4_K_M.gguf"),
        MAX_MODEL_PAYLOAD_BYTES,
    )
    .map_err(DockerLiveCollectorError::linux)?;
    let projector = hash_regular_file(&root.join("mmproj-F16.gguf"), MAX_MODEL_PAYLOAD_BYTES)
        .map_err(DockerLiveCollectorError::linux)?;
    if model != DOCKER_MODEL_GGUF_SHA256 || projector != DOCKER_MODEL_PROJECTOR_SHA256 {
        return Err(DockerLiveCollectorError::docker(
            DockerHttpObserverError::DockerState,
        ));
    }
    Ok(())
}

fn image_digest_absent(repo_digests: &[String], expected: &str) -> bool {
    !repo_digests.iter().any(|value| {
        value == expected
            || value
                .rsplit_once('@')
                .is_some_and(|(_, digest)| digest == expected)
    })
}

fn label_number(labels: &std::collections::BTreeMap<String, String>, name: &str) -> u64 {
    labels
        .get(name)
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

fn validate_request(
    request: &DockerLiveCollectorRequest,
    now: u64,
) -> Result<(), DockerLiveCollectorError> {
    if request.protocol_version != DOCKER_TOPOLOGY_COLLECTOR_PROTOCOL_VERSION
        || request.preflight_contract_version != DOCKER_PREFLIGHT_CONTRACT_VERSION
        || request.source_revision.len() != 40
        || !request
            .source_revision
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        || request.runner_container_id.len() != 64
        || !request
            .runner_container_id
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        || request.runtime_pid <= 1
        || request.guard_pid <= 1
        || request.runtime_pid == request.guard_pid
        || request.runtime_start_time_ticks == 0
        || request.guard_start_time_ticks == 0
    {
        return Err(DockerLiveCollectorError::new("docker-collector.input"));
    }
    parse_sha256(&request.os_release_sha256)?;
    parse_sha256(&request.session_identity_sha256)?;
    parse_sha256(&request.collector_executable_sha256)?;
    if request.observation_started_unix_seconds > now
        || now - request.observation_started_unix_seconds > MAX_OBSERVATION_AGE_SECONDS
    {
        return Err(DockerLiveCollectorError::new("docker-collector.freshness"));
    }
    Ok(())
}

fn record_session_once(session: &str) -> Result<(), DockerLiveCollectorError> {
    let directory = fs::symlink_metadata(REPLAY_DIRECTORY)
        .map_err(|_| DockerLiveCollectorError::new("docker-collector.replay"))?;
    if !directory.file_type().is_dir()
        || directory.uid() != 0
        || directory.gid() != 0
        || directory.mode() & 0o777 != 0o700
    {
        return Err(DockerLiveCollectorError::new("docker-collector.replay"));
    }
    create_session_marker(Path::new(REPLAY_DIRECTORY), session)
}

fn create_session_marker(directory: &Path, session: &str) -> Result<(), DockerLiveCollectorError> {
    let path = directory.join(format!("{session}.used"));
    let mut marker = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|_| DockerLiveCollectorError::new("docker-collector.replay"))?;
    marker
        .write_all(b"agentmage-docker-observation-v1\n")
        .and_then(|()| marker.sync_all())
        .map_err(|_| DockerLiveCollectorError::new("docker-collector.replay"))
}

fn parse_sha256(value: &str) -> Result<[u8; 32], DockerLiveCollectorError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(DockerLiveCollectorError::new("docker-collector.input"));
    }
    let mut output = [0_u8; 32];
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| DockerLiveCollectorError::new("docker-collector.input"))?;
    }
    if output == [0; 32] {
        return Err(DockerLiveCollectorError::new("docker-collector.input"));
    }
    Ok(output)
}

fn hex(value: &[u8; 32]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}

const _: u64 = DOCKER_COLLECTOR_MAX_INPUT_BYTES;
const _: [u8; 32] = DOCKER_RUNTIME_PROFILE_SHA256;
const _: [u8; 32] = DOCKER_GUARD_PROFILE_SHA256;
const _: u16 = RUNTIME_SECONDS;
const _: u32 = OUTPUT_BYTES;
const _: u8 = PARALLEL_SLOTS;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docker_http_observer::{
        DockerContainerConfig, DockerContainerState, DockerContainerSummary, DockerHostConfig,
    };
    use crate::docker_linux_observer::{
        DirectoryObservation, NetworkObservation, ProcessObservation, UnixSocketObservation,
    };

    fn request(now: u64) -> DockerLiveCollectorRequest {
        DockerLiveCollectorRequest {
            protocol_version: 1,
            preflight_contract_version: DOCKER_PREFLIGHT_CONTRACT_VERSION,
            source_revision: "a".repeat(40),
            os_release_sha256: "01".repeat(32),
            observation_started_unix_seconds: now,
            session_identity_sha256: "02".repeat(32),
            collector_executable_sha256: "03".repeat(32),
            runtime_pid: 10,
            runtime_start_time_ticks: 20,
            guard_pid: 11,
            guard_start_time_ticks: 21,
            runner_container_id: "04".repeat(32),
            baseline: BaselineInput {
                daemon_executable_sha256: "05".repeat(32),
                daemon_socket_identity_sha256: "06".repeat(32),
                docker_socket_gid: 971,
                runtime_uid: 1000,
                runtime_gid: 1000,
                guard_uid: 991,
                private_namespace_sha256: "07".repeat(32),
                guard_executable_sha256: "08".repeat(32),
                guard_cgroup_sha256: "09".repeat(32),
            },
        }
    }

    struct DerivedFixture {
        request: DockerLiveCollectorRequest,
        container: DockerContainerInspect,
        running: Vec<DockerContainerSummary>,
        repo_digests: Vec<String>,
        runtime_groups: Vec<u32>,
        guard: ProcessObservation,
        runner: ProcessObservation,
        daemon: ProcessObservation,
        daemon_socket: UnixSocketObservation,
        security_options: Vec<String>,
        host_network: NetworkObservation,
        private_network: NetworkObservation,
        guard_socket: UnixSocketObservation,
        guard_parent: DirectoryObservation,
        guard_count: u8,
    }

    impl DerivedFixture {
        fn exact() -> Self {
            let request = request(1_000);
            let image_id = format!("sha256:{}", "0a".repeat(32));
            let runner_digest = format!("sha256:{DOCKER_MODEL_RUNNER_IMAGE_DIGEST_HEX}");
            let mut labels = std::collections::BTreeMap::new();
            labels.insert(
                "io.agentmage.model.manifest-digest".into(),
                format!("sha256:{DOCKER_MODEL_ARTIFACT_DIGEST_HEX}"),
            );
            labels.insert("io.agentmage.runtime-seconds".into(), "3600".into());
            labels.insert("io.agentmage.output-bytes".into(), "16777216".into());
            labels.insert("io.agentmage.parallel-slots".into(), "1".into());
            labels.insert("io.agentmage.image-repull".into(), "false".into());
            let process = |uid, gid, executable, cgroup, network, mount| ProcessObservation {
                uid,
                gid,
                groups: vec![gid],
                start_time_ticks: 1,
                executable_sha256: executable,
                cgroup_sha256: cgroup,
                network_namespace_sha256: network,
                mount_namespace_sha256: mount,
                effective_capabilities: 0,
                no_new_privileges: true,
            };
            Self {
                request: request.clone(),
                container: DockerContainerInspect {
                    id: request.runner_container_id.clone(),
                    image: image_id.clone(),
                    state: DockerContainerState {
                        running: true,
                        pid: 12,
                    },
                    config: DockerContainerConfig {
                        image: runner_digest.clone(),
                        user: "modelrunner".into(),
                        env: vec!["DO_NOT_TRACK=1".into()],
                        labels,
                    },
                    host_config: DockerHostConfig {
                        network_mode: "none".into(),
                        privileged: false,
                        readonly_rootfs: true,
                        cap_add: None,
                        security_opt: Some(vec!["no-new-privileges".into()]),
                        memory: 64 * 1024 * 1024 * 1024,
                        memory_swap: 64 * 1024 * 1024 * 1024,
                        nano_cpus: 32_000_000_000,
                        pids_limit: Some(64),
                        port_bindings: Some(serde_json::json!({})),
                        tmpfs: Some(std::collections::BTreeMap::from([(
                            "/run".into(),
                            "rw,nosuid,nodev,noexec,size=64m".into(),
                        )])),
                    },
                    mounts: vec![DockerMount {
                        kind: "volume".into(),
                        source: "docker-managed".into(),
                        destination: "/models".into(),
                        mode: "z".into(),
                        read_write: false,
                    }],
                },
                running: vec![DockerContainerSummary {
                    id: request.runner_container_id.clone(),
                    image_id: image_id.clone(),
                }],
                repo_digests: vec![format!("docker.io/docker/model-runner@{runner_digest}")],
                runtime_groups: vec![1000],
                guard: process(991, 1000, [8; 32], [9; 32], [7; 32], [10; 32]),
                runner: process(1001, 1001, [11; 32], [12; 32], [7; 32], [13; 32]),
                daemon: process(0, 0, [5; 32], [14; 32], [15; 32], [16; 32]),
                daemon_socket: UnixSocketObservation {
                    identity_sha256: [6; 32],
                    owner_uid: 0,
                    group_gid: 971,
                    mode: 0o660,
                },
                security_options: vec!["name=seccomp,profile=builtin".into()],
                host_network: NetworkObservation {
                    active_interface_count: 2,
                    loopback_up: true,
                    non_loopback_interface_count: 1,
                    non_local_route_count: 1,
                    raw_listener_count: 0,
                    raw_wildcard_v4_listener_count: 0,
                    raw_wildcard_v6_listener_count: 0,
                    non_loopback_listener_count: 0,
                    management_listener_count: 0,
                    outbound_bytes: 0,
                },
                private_network: NetworkObservation {
                    active_interface_count: 1,
                    loopback_up: true,
                    non_loopback_interface_count: 0,
                    non_local_route_count: 0,
                    raw_listener_count: 1,
                    raw_wildcard_v4_listener_count: 0,
                    raw_wildcard_v6_listener_count: 1,
                    non_loopback_listener_count: 0,
                    management_listener_count: 0,
                    outbound_bytes: 0,
                },
                guard_socket: UnixSocketObservation {
                    identity_sha256: [17; 32],
                    owner_uid: 991,
                    group_gid: 1000,
                    mode: 0o660,
                },
                guard_parent: DirectoryObservation {
                    owner_uid: 991,
                    group_gid: 1000,
                    mode: 0o710,
                },
                guard_count: 1,
            }
        }

        fn observation(&self) -> Result<DockerCollectorInput, DockerLiveCollectorError> {
            build_observation(
                &self.request,
                &self.container,
                &self.running,
                &self.repo_digests,
                &self.runtime_groups,
                &self.guard,
                &self.runner,
                &self.daemon,
                &self.daemon_socket,
                &self.security_options,
                self.host_network,
                self.private_network,
                self.guard_socket,
                self.guard_parent,
                self.guard_count,
            )
        }
    }

    #[test]
    fn request_identity_and_freshness_are_closed() {
        let now = 1_000;
        assert!(validate_request(&request(now), now).is_ok());
        let mut stale = request(now - MAX_OBSERVATION_AGE_SECONDS - 1);
        assert_eq!(
            validate_request(&stale, now),
            Err(DockerLiveCollectorError::new("docker-collector.freshness"))
        );
        stale.observation_started_unix_seconds = now;
        stale.source_revision = "A".repeat(40);
        assert_eq!(
            validate_request(&stale, now),
            Err(DockerLiveCollectorError::new("docker-collector.input"))
        );
        let mut future = request(now + 1);
        assert_eq!(
            validate_request(&future, now),
            Err(DockerLiveCollectorError::new("docker-collector.freshness"))
        );
        future.observation_started_unix_seconds = now;
        future.runner_container_id = "tag".into();
        assert_eq!(
            validate_request(&future, now),
            Err(DockerLiveCollectorError::new("docker-collector.input"))
        );
    }

    #[test]
    fn mount_classifier_has_only_two_declared_mounts() {
        let accepted = [DockerMount {
            kind: "volume".into(),
            source: "docker-managed".into(),
            destination: "/models".into(),
            mode: "z".into(),
            read_write: false,
        }];
        let tmpfs = std::collections::BTreeMap::from([(
            "/run".into(),
            "rw,nosuid,nodev,noexec,size=64m".into(),
        )]);
        assert_eq!(
            classify_mounts(&accepted, Some(&tmpfs)),
            MountState {
                private_runtime_tmpfs: true,
                model_content_store_present: true,
                ..MountState::default()
            }
        );
        let mut forbidden = accepted.to_vec();
        forbidden.push(DockerMount {
            kind: "bind".into(),
            source: "/run/docker.sock".into(),
            destination: "/run/docker.sock".into(),
            mode: "rw".into(),
            read_write: true,
        });
        assert_eq!(classify_mounts(&forbidden, Some(&tmpfs)).docker_socket, 1);
        assert!(!classify_mounts(&accepted, None).private_runtime_tmpfs);
        let weakened = std::collections::BTreeMap::from([("/run".into(), "rw,size=64m".into())]);
        assert_eq!(classify_mounts(&accepted, Some(&weakened)).forbidden, 1);
        let extra = std::collections::BTreeMap::from([
            ("/run".into(), "rw,nosuid,nodev,noexec,size=64m".into()),
            ("/tmp".into(), "rw,nosuid,nodev,noexec,size=64m".into()),
        ]);
        assert_eq!(classify_mounts(&accepted, Some(&extra)).forbidden, 1);
    }

    #[test]
    fn production_derivation_admits_only_the_exact_closed_fixture() {
        let fixture = DerivedFixture::exact();
        let observation = fixture.observation().expect("complete observation");
        assert!(validate_topology(observation).is_ok());

        let mut tag = DerivedFixture::exact();
        tag.container.config.image = "docker.io/docker/model-runner:latest".into();
        assert_eq!(
            validate_topology(tag.observation().expect("tag observation"))
                .unwrap_err()
                .code(),
            "docker-preflight.image.identity"
        );

        let mut proxy = DerivedFixture::exact();
        proxy
            .container
            .config
            .env
            .push("HTTPS_PROXY=http://example.invalid".into());
        assert_eq!(
            validate_topology(proxy.observation().expect("proxy observation"))
                .unwrap_err()
                .code(),
            "docker-preflight.egress.nonzero"
        );

        let mut missing_model = DerivedFixture::exact();
        missing_model
            .container
            .mounts
            .retain(|mount| mount.destination != "/models");
        assert_eq!(
            validate_topology(missing_model.observation().expect("mount observation"))
                .unwrap_err()
                .code(),
            "docker-preflight.resources.invalid"
        );

        let mut exposed = DerivedFixture::exact();
        exposed.host_network.raw_listener_count = 1;
        assert_eq!(
            validate_topology(exposed.observation().expect("network observation"))
                .unwrap_err()
                .code(),
            "docker-preflight.api.binding"
        );

        let mut ipv4_substitution = DerivedFixture::exact();
        ipv4_substitution
            .private_network
            .raw_wildcard_v4_listener_count = 1;
        ipv4_substitution
            .private_network
            .raw_wildcard_v6_listener_count = 0;
        assert_eq!(
            validate_topology(
                ipv4_substitution
                    .observation()
                    .expect("IPv4 substitution observation")
            )
            .unwrap_err()
            .code(),
            "docker-preflight.api.binding"
        );

        let mut duplicate_guard = DerivedFixture::exact();
        duplicate_guard.guard_count = 2;
        assert_eq!(
            validate_topology(
                duplicate_guard
                    .observation()
                    .expect("duplicate guard observation")
            )
            .unwrap_err()
            .code(),
            "docker-preflight.container.reachability"
        );
    }

    #[test]
    fn replay_marker_is_atomic_private_and_one_use() {
        use std::os::unix::fs::PermissionsExt;

        let directory = std::env::temp_dir().join(format!(
            "agentmage-docker-replay-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("test clock")
                .as_nanos()
        ));
        fs::create_dir(&directory).expect("create replay fixture");
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
            .expect("secure replay fixture");
        let session = "0a".repeat(32);
        assert!(create_session_marker(&directory, &session).is_ok());
        assert_eq!(
            create_session_marker(&directory, &session),
            Err(DockerLiveCollectorError::new("docker-collector.replay"))
        );
        let marker = directory.join(format!("{session}.used"));
        let metadata = fs::metadata(&marker).expect("marker metadata");
        assert_eq!(metadata.mode() & 0o777, 0o600);
        fs::remove_file(marker).expect("remove replay marker fixture");
        fs::remove_dir(directory).expect("remove replay directory fixture");
    }

    #[test]
    fn public_reader_refuses_before_parsing_for_non_admin() {
        assert_eq!(
            observe_live_topology_from_reader(b"{}".as_slice(), 1000),
            Err(DockerLiveCollectorError::new(
                "docker-collector.administrator-identity"
            ))
        );
    }
}
