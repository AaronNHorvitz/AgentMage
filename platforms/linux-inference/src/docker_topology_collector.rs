//! Closed structured validation boundary for Docker topology observations.

use std::fmt;
use std::io::Read;
use std::net::Ipv4Addr;

use serde::{Deserialize, Serialize};

use crate::{
    DOCKER_GUARD_PROFILE_SHA256, DOCKER_PREFLIGHT_CONTRACT_VERSION, DOCKER_RUNTIME_PROFILE_SHA256,
    DOCKER_TOPOLOGY_COLLECTOR_PROTOCOL_VERSION, DockerApiObservation, DockerContainerObservation,
    DockerDaemonObservation, DockerEgressObservation, DockerImageObservation,
    DockerPreflightBaseline, DockerPreflightError, DockerResourceObservation,
    DockerTopologyObservation, admit_docker_mode,
};

/// Maximum accepted structured collector observation input.
pub const DOCKER_COLLECTOR_MAX_INPUT_BYTES: u64 = 128 * 1024;

/// Stable content-free trusted collector failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockerTopologyCollectorError {
    /// The collector did not run as the separately authorized administrator.
    AdministratorIdentity,
    /// The input exceeded the fixed bound or was not a closed valid record.
    Input,
    /// The configured exact baseline was malformed or drifted.
    Baseline,
    /// The complete observation did not satisfy preflight.
    Preflight(DockerPreflightError),
}

impl DockerTopologyCollectorError {
    /// Returns the stable redacted error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::AdministratorIdentity => "docker-collector.administrator-identity",
            Self::Input => "docker-collector.input",
            Self::Baseline => "docker-collector.baseline",
            Self::Preflight(error) => error.code(),
        }
    }
}

impl fmt::Display for DockerTopologyCollectorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for DockerTopologyCollectorError {}

/// Complete closed collector input; every field is bounded metadata, never source content.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DockerCollectorInput {
    /// Exact collector protocol version.
    pub protocol_version: u16,
    /// Exact Docker preflight contract version.
    pub preflight_contract_version: u16,
    /// Numeric collector user observed by the launcher.
    pub collector_uid: u32,
    /// Exact collector executable SHA-256.
    pub collector_executable_sha256: String,
    /// Fresh nonzero observation identity.
    pub session_identity_sha256: String,
    /// Complete-observation marker.
    pub complete: bool,
    /// Fresh-observation marker.
    pub fresh: bool,
    /// Replay marker.
    pub replayed: bool,
    /// Exact administrator-configured baseline.
    pub baseline: BaselineInput,
    /// Docker daemon and socket observation.
    pub daemon: DaemonInput,
    /// Private API and namespace observation.
    pub api: ApiInput,
    /// Runner, guard, reachability, and mount observation.
    pub containers: ContainersInput,
    /// Immutable runner and model identities.
    pub images: ImagesInput,
    /// Exact privilege and resource observation.
    pub resources: ResourcesInput,
    /// Exact offline and egress observation.
    pub egress: EgressInput,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineInput {
    pub(crate) daemon_executable_sha256: String,
    pub(crate) daemon_socket_identity_sha256: String,
    pub(crate) docker_socket_gid: u32,
    pub(crate) runtime_uid: u32,
    pub(crate) runtime_gid: u32,
    pub(crate) guard_uid: u32,
    pub(crate) private_namespace_sha256: String,
    pub(crate) guard_executable_sha256: String,
    pub(crate) guard_cgroup_sha256: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DaemonInput {
    pub(crate) daemon_executable_sha256: String,
    pub(crate) daemon_uid: u32,
    pub(crate) rootless: bool,
    pub(crate) runtime_uid: u32,
    pub(crate) runtime_user_has_socket_group: bool,
    pub(crate) socket_identity_sha256: String,
    pub(crate) socket_is_unix_stream: bool,
    pub(crate) socket_owner_uid: u32,
    pub(crate) socket_group_gid: u32,
    pub(crate) socket_mode: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApiInput {
    pub(crate) private_namespace_sha256: String,
    pub(crate) runner_bind_host: String,
    pub(crate) guard_connect_host: String,
    pub(crate) raw_port: u16,
    pub(crate) namespace_active_interface_count: u8,
    pub(crate) loopback_interface_up: bool,
    pub(crate) namespace_non_local_route_count: u8,
    pub(crate) raw_listener_count: u8,
    pub(crate) host_listener_count: u8,
    pub(crate) non_loopback_listener_count: u8,
    pub(crate) management_listener_count: u8,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContainersInput {
    pub(crate) runner_count: u8,
    pub(crate) guard_count: u8,
    pub(crate) runner_and_guard_share_namespace: bool,
    pub(crate) guard_uid: u32,
    pub(crate) guard_executable_sha256: String,
    pub(crate) guard_cgroup_sha256: String,
    pub(crate) kernel_socket_owner_uid: u32,
    pub(crate) kernel_socket_group_gid: u32,
    pub(crate) kernel_socket_parent_mode: u32,
    pub(crate) kernel_socket_mode: u32,
    pub(crate) kernel_socket_peer_authentication: bool,
    pub(crate) host_route_count: u8,
    pub(crate) bridge_route_count: u8,
    pub(crate) foreign_reachable_peer_count: u8,
    pub(crate) workspace_mount_count: u8,
    pub(crate) credential_mount_count: u8,
    pub(crate) host_root_mount_count: u8,
    pub(crate) docker_socket_mount_count: u8,
    pub(crate) private_runtime_tmpfs: bool,
    pub(crate) model_content_store_writable: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImagesInput {
    pub(crate) runner_manifest_digest: String,
    pub(crate) model_manifest_digest: String,
    pub(crate) mutable_tag_used_for_admission: bool,
    pub(crate) image_repull_allowed: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResourcesInput {
    pub(crate) privileged: bool,
    pub(crate) capabilities_added: u8,
    pub(crate) no_new_privileges: bool,
    pub(crate) read_only_root: bool,
    pub(crate) memory_bytes: u64,
    pub(crate) tasks: u32,
    pub(crate) cpu_percent: u16,
    pub(crate) runtime_seconds: u16,
    pub(crate) output_bytes: u32,
    pub(crate) swap_bytes: u64,
    pub(crate) parallel_slots: u8,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EgressInput {
    pub(crate) do_not_track: bool,
    pub(crate) acquisition_allowed: bool,
    pub(crate) registry_access: bool,
    pub(crate) ambient_proxy: bool,
    pub(crate) ambient_dns: bool,
    pub(crate) firewall_default_deny: bool,
    pub(crate) egress_interface_count: u8,
    pub(crate) outbound_bytes: u64,
}

/// Content-free admission output emitted only after the complete preflight passes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DockerCollectorOutput {
    /// Stable output record type.
    pub record_type: &'static str,
    /// Collector protocol version.
    pub protocol_version: u16,
    /// Docker preflight contract version.
    pub preflight_contract_version: u16,
    /// Fresh admitted session identity.
    pub session_identity_sha256: String,
    /// Content-free terminal status.
    pub status: &'static str,
}

/// Validates one bounded closed record and returns only a content-free admission result.
pub fn validate_topology_from_reader(
    reader: impl Read,
    effective_uid: u32,
) -> Result<DockerCollectorOutput, DockerTopologyCollectorError> {
    if effective_uid != 0 {
        return Err(DockerTopologyCollectorError::AdministratorIdentity);
    }
    let mut bytes = Vec::new();
    reader
        .take(DOCKER_COLLECTOR_MAX_INPUT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| DockerTopologyCollectorError::Input)?;
    if bytes.is_empty() || bytes.len() as u64 > DOCKER_COLLECTOR_MAX_INPUT_BYTES {
        return Err(DockerTopologyCollectorError::Input);
    }
    let input: DockerCollectorInput =
        serde_json::from_slice(&bytes).map_err(|_| DockerTopologyCollectorError::Input)?;
    validate_topology(input)
}

pub(crate) fn validate_topology(
    input: DockerCollectorInput,
) -> Result<DockerCollectorOutput, DockerTopologyCollectorError> {
    if input.protocol_version != DOCKER_TOPOLOGY_COLLECTOR_PROTOCOL_VERSION
        || input.preflight_contract_version != DOCKER_PREFLIGHT_CONTRACT_VERSION
        || input.collector_uid != 0
    {
        return Err(DockerTopologyCollectorError::Input);
    }
    let collector_sha256 = parse_sha256(&input.collector_executable_sha256)?;
    let baseline = DockerPreflightBaseline::verify(
        DOCKER_RUNTIME_PROFILE_SHA256,
        DOCKER_GUARD_PROFILE_SHA256,
        collector_sha256,
        parse_sha256(&input.baseline.daemon_executable_sha256)?,
        parse_sha256(&input.baseline.daemon_socket_identity_sha256)?,
        input.baseline.docker_socket_gid,
        input.baseline.runtime_uid,
        input.baseline.runtime_gid,
        input.baseline.guard_uid,
        parse_sha256(&input.baseline.private_namespace_sha256)?,
        parse_sha256(&input.baseline.guard_executable_sha256)?,
        parse_sha256(&input.baseline.guard_cgroup_sha256)?,
    )
    .map_err(|_| DockerTopologyCollectorError::Baseline)?;
    if collector_sha256 != *baseline.collector_executable_sha256() {
        return Err(DockerTopologyCollectorError::Baseline);
    }
    let session = parse_sha256(&input.session_identity_sha256)?;
    let observation = DockerTopologyObservation::new(
        collector_sha256,
        input.protocol_version,
        input.collector_uid,
        session,
        input.complete,
        input.fresh,
        input.replayed,
        DockerDaemonObservation::new(
            parse_sha256(&input.daemon.daemon_executable_sha256)?,
            input.daemon.daemon_uid,
            input.daemon.rootless,
            input.daemon.runtime_uid,
            input.daemon.runtime_user_has_socket_group,
            parse_sha256(&input.daemon.socket_identity_sha256)?,
            input.daemon.socket_is_unix_stream,
            input.daemon.socket_owner_uid,
            input.daemon.socket_group_gid,
            input.daemon.socket_mode,
        ),
        DockerApiObservation::new(
            parse_sha256(&input.api.private_namespace_sha256)?,
            parse_ipv4(&input.api.runner_bind_host)?,
            parse_ipv4(&input.api.guard_connect_host)?,
            input.api.raw_port,
            input.api.namespace_active_interface_count,
            input.api.loopback_interface_up,
            input.api.namespace_non_local_route_count,
            input.api.raw_listener_count,
            input.api.host_listener_count,
            input.api.non_loopback_listener_count,
            input.api.management_listener_count,
        ),
        DockerContainerObservation::new(
            input.containers.runner_count,
            input.containers.guard_count,
            input.containers.runner_and_guard_share_namespace,
            input.containers.guard_uid,
            parse_sha256(&input.containers.guard_executable_sha256)?,
            parse_sha256(&input.containers.guard_cgroup_sha256)?,
            input.containers.kernel_socket_owner_uid,
            input.containers.kernel_socket_group_gid,
            input.containers.kernel_socket_parent_mode,
            input.containers.kernel_socket_mode,
            input.containers.kernel_socket_peer_authentication,
            input.containers.host_route_count,
            input.containers.bridge_route_count,
            input.containers.foreign_reachable_peer_count,
            input.containers.workspace_mount_count,
            input.containers.credential_mount_count,
            input.containers.host_root_mount_count,
            input.containers.docker_socket_mount_count,
            input.containers.private_runtime_tmpfs,
            input.containers.model_content_store_writable,
        ),
        DockerImageObservation::new(
            parse_digest(&input.images.runner_manifest_digest)?,
            parse_digest(&input.images.model_manifest_digest)?,
            input.images.mutable_tag_used_for_admission,
            input.images.image_repull_allowed,
        ),
        DockerResourceObservation::new(
            input.resources.privileged,
            input.resources.capabilities_added,
            input.resources.no_new_privileges,
            input.resources.read_only_root,
            input.resources.memory_bytes,
            input.resources.tasks,
            input.resources.cpu_percent,
            input.resources.runtime_seconds,
            input.resources.output_bytes,
            input.resources.swap_bytes,
            input.resources.parallel_slots,
        ),
        DockerEgressObservation::new(
            input.egress.do_not_track,
            input.egress.acquisition_allowed,
            input.egress.registry_access,
            input.egress.ambient_proxy,
            input.egress.ambient_dns,
            input.egress.firewall_default_deny,
            input.egress.egress_interface_count,
            input.egress.outbound_bytes,
        ),
    );
    let admission = admit_docker_mode(&baseline, observation)
        .map_err(DockerTopologyCollectorError::Preflight)?;
    Ok(DockerCollectorOutput {
        record_type: "agentmage_docker_topology_admission",
        protocol_version: DOCKER_TOPOLOGY_COLLECTOR_PROTOCOL_VERSION,
        preflight_contract_version: DOCKER_PREFLIGHT_CONTRACT_VERSION,
        session_identity_sha256: hex(admission.session_identity_sha256()),
        status: "admitted",
    })
}

fn parse_sha256(value: &str) -> Result<[u8; 32], DockerTopologyCollectorError> {
    if value.len() != 64 || !value.bytes().all(|value| value.is_ascii_hexdigit()) {
        return Err(DockerTopologyCollectorError::Input);
    }
    let mut output = [0_u8; 32];
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| DockerTopologyCollectorError::Input)?;
    }
    if output == [0; 32] {
        return Err(DockerTopologyCollectorError::Input);
    }
    Ok(output)
}

fn parse_digest(value: &str) -> Result<[u8; 32], DockerTopologyCollectorError> {
    value
        .strip_prefix("sha256:")
        .ok_or(DockerTopologyCollectorError::Input)
        .and_then(parse_sha256)
}

fn parse_ipv4(value: &str) -> Result<Ipv4Addr, DockerTopologyCollectorError> {
    value
        .parse()
        .map_err(|_| DockerTopologyCollectorError::Input)
}

fn hex(value: &[u8; 32]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DOCKER_MODEL_ARTIFACT_DIGEST, DOCKER_MODEL_RUNNER_IMAGE_DIGEST};

    fn exact_json() -> String {
        let hash = "01".repeat(32);
        format!(
            r#"{{
              "protocol_version":1,"preflight_contract_version":2,"collector_uid":0,
              "collector_executable_sha256":"{hash}","session_identity_sha256":"{session}",
              "complete":true,"fresh":true,"replayed":false,
              "baseline":{{"daemon_executable_sha256":"{daemon}","daemon_socket_identity_sha256":"{socket}","docker_socket_gid":971,"runtime_uid":1000,"runtime_gid":1000,"guard_uid":991,"private_namespace_sha256":"{namespace}","guard_executable_sha256":"{guard}","guard_cgroup_sha256":"{cgroup}"}},
              "daemon":{{"daemon_executable_sha256":"{daemon}","daemon_uid":0,"rootless":false,"runtime_uid":1000,"runtime_user_has_socket_group":false,"socket_identity_sha256":"{socket}","socket_is_unix_stream":true,"socket_owner_uid":0,"socket_group_gid":971,"socket_mode":432}},
              "api":{{"private_namespace_sha256":"{namespace}","runner_bind_host":"0.0.0.0","guard_connect_host":"127.0.0.1","raw_port":12434,"namespace_active_interface_count":1,"loopback_interface_up":true,"namespace_non_local_route_count":0,"raw_listener_count":1,"host_listener_count":0,"non_loopback_listener_count":0,"management_listener_count":0}},
              "containers":{{"runner_count":1,"guard_count":1,"runner_and_guard_share_namespace":true,"guard_uid":991,"guard_executable_sha256":"{guard}","guard_cgroup_sha256":"{cgroup}","kernel_socket_owner_uid":991,"kernel_socket_group_gid":1000,"kernel_socket_parent_mode":456,"kernel_socket_mode":432,"kernel_socket_peer_authentication":true,"host_route_count":0,"bridge_route_count":0,"foreign_reachable_peer_count":0,"workspace_mount_count":0,"credential_mount_count":0,"host_root_mount_count":0,"docker_socket_mount_count":0,"private_runtime_tmpfs":true,"model_content_store_writable":false}},
              "images":{{"runner_manifest_digest":"sha256:{runner}","model_manifest_digest":"sha256:{model}","mutable_tag_used_for_admission":false,"image_repull_allowed":false}},
              "resources":{{"privileged":false,"capabilities_added":0,"no_new_privileges":true,"read_only_root":true,"memory_bytes":68719476736,"tasks":64,"cpu_percent":3200,"runtime_seconds":3600,"output_bytes":16777216,"swap_bytes":0,"parallel_slots":1}},
              "egress":{{"do_not_track":true,"acquisition_allowed":false,"registry_access":false,"ambient_proxy":false,"ambient_dns":false,"firewall_default_deny":true,"egress_interface_count":0,"outbound_bytes":0}}
            }}"#,
            session = "07".repeat(32),
            daemon = "02".repeat(32),
            socket = "03".repeat(32),
            namespace = "04".repeat(32),
            guard = "05".repeat(32),
            cgroup = "06".repeat(32),
            runner = hex(&DOCKER_MODEL_RUNNER_IMAGE_DIGEST),
            model = hex(&DOCKER_MODEL_ARTIFACT_DIGEST),
        )
    }

    #[test]
    fn exact_complete_observation_is_admitted_for_root_only() {
        let output = validate_topology_from_reader(exact_json().as_bytes(), 0).expect("admitted");
        assert_eq!(output.status, "admitted");
        assert_eq!(output.session_identity_sha256, "07".repeat(32));
        assert_eq!(
            validate_topology_from_reader(exact_json().as_bytes(), 1000).unwrap_err(),
            DockerTopologyCollectorError::AdministratorIdentity
        );
    }

    #[test]
    fn malformed_unknown_oversized_and_partial_input_fail_closed() {
        for input in [
            Vec::new(),
            b"not-json".to_vec(),
            exact_json()
                .replace("\"complete\":true", "\"complete\":false")
                .into_bytes(),
            exact_json()
                .replace("\"fresh\":true", "\"fresh\":false")
                .into_bytes(),
            exact_json()
                .replace("\"replayed\":false", "\"replayed\":true")
                .into_bytes(),
            exact_json()
                .replace("\"protocol_version\":1", "\"protocol_version\":2")
                .into_bytes(),
            exact_json()
                .replace("\n            }", ",\"unknown\":true\n            }")
                .into_bytes(),
            vec![b' '; DOCKER_COLLECTOR_MAX_INPUT_BYTES as usize + 1],
        ] {
            assert!(validate_topology_from_reader(input.as_slice(), 0).is_err());
        }
    }

    #[test]
    fn every_preflight_class_is_preserved_without_observed_values() {
        let mutations = [
            (
                "\"daemon_uid\":0",
                "\"daemon_uid\":1000",
                DockerPreflightError::DaemonPrivilege,
            ),
            (
                "\"socket_mode\":432",
                "\"socket_mode\":438",
                DockerPreflightError::SocketOwnership,
            ),
            (
                "\"raw_port\":12434",
                "\"raw_port\":12435",
                DockerPreflightError::ApiBinding,
            ),
            (
                "\"runner_count\":1",
                "\"runner_count\":2",
                DockerPreflightError::ContainerReachability,
            ),
            (
                "\"mutable_tag_used_for_admission\":false",
                "\"mutable_tag_used_for_admission\":true",
                DockerPreflightError::ImageIdentity,
            ),
            (
                "\"privileged\":false",
                "\"privileged\":true",
                DockerPreflightError::ResourceLimits,
            ),
            (
                "\"outbound_bytes\":0",
                "\"outbound_bytes\":1",
                DockerPreflightError::ZeroEgress,
            ),
        ];
        for (old, new, expected) in mutations {
            let error = validate_topology_from_reader(exact_json().replace(old, new).as_bytes(), 0)
                .unwrap_err();
            assert_eq!(error, DockerTopologyCollectorError::Preflight(expected));
            assert_eq!(error.code(), expected.code());
        }
    }

    #[test]
    fn digest_parser_is_exact_lower_or_upper_hex_but_no_prefix_fallback() {
        assert_eq!(parse_sha256(&"01".repeat(32)), Ok([1; 32]));
        assert_eq!(
            parse_sha256(&"01".repeat(31)),
            Err(DockerTopologyCollectorError::Input)
        );
        assert_eq!(
            parse_sha256(&"00".repeat(32)),
            Err(DockerTopologyCollectorError::Input)
        );
        assert_eq!(
            parse_digest(&format!("sha256:{}", "01".repeat(32))),
            Ok([1; 32])
        );
        assert_eq!(
            parse_digest(&"01".repeat(32)),
            Err(DockerTopologyCollectorError::Input)
        );
    }
}
