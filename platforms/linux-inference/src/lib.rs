#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Isolated Linux native-inference process boundary.

use std::ffi::OsString;
use std::fmt;

use agentmage_kernel_contracts::{
    LocalEndpointIdentity, LocalTransport, NetworkComponent, NetworkEndpointError,
};

mod docker_guard;
mod docker_guard_service;
mod docker_http_observer;
mod docker_linux_observer;
mod docker_live_collector;
mod docker_preflight;
mod docker_runtime;
mod docker_topology_collector;
mod native_model_adapter;
mod native_runtime;

pub use docker_guard::{
    DOCKER_GUARD_PROFILE_SHA256, DOCKER_GUARD_PROFILE_SHA256_HEX, DockerEndpointCallerClass,
    DockerEndpointCallerObservation, DockerEndpointGuard, DockerEndpointGuardError,
    DockerRawEndpointPermit,
};
pub use docker_guard_service::{
    DOCKER_GUARD_BOOTSTRAP_BYTES, DOCKER_GUARD_PROTOCOL_VERSION, DockerGuardBootstrap,
    DockerGuardService, DockerGuardServiceError, DockerGuardSessionCredentials,
};
pub use docker_live_collector::{
    DOCKER_LIVE_OBSERVER_MAX_INPUT_BYTES, DockerLiveCollectorError, DockerLiveCollectorOutput,
    observe_live_topology_from_reader,
};
pub use docker_preflight::{
    DOCKER_PREFLIGHT_CONTRACT_VERSION, DOCKER_TOPOLOGY_COLLECTOR_PROTOCOL_VERSION,
    DockerApiObservation, DockerContainerObservation, DockerDaemonObservation,
    DockerEgressObservation, DockerImageObservation, DockerModeAdmission, DockerPreflightBaseline,
    DockerPreflightError, DockerResourceObservation, DockerTopologyObservation, admit_docker_mode,
};
pub use docker_runtime::{
    DOCKER_MODEL_ARTIFACT_DIGEST, DOCKER_MODEL_ARTIFACT_DIGEST_HEX, DOCKER_MODEL_GGUF_SHA256,
    DOCKER_MODEL_PLUGIN_PACKAGE_ID, DOCKER_MODEL_PLUGIN_VERSION, DOCKER_MODEL_PROJECTOR_SHA256,
    DOCKER_MODEL_RUNNER_BIND_HOST, DOCKER_MODEL_RUNNER_CONNECT_HOST,
    DOCKER_MODEL_RUNNER_IMAGE_DIGEST, DOCKER_MODEL_RUNNER_IMAGE_DIGEST_HEX,
    DOCKER_MODEL_RUNNER_PORT, DOCKER_RUNTIME_PROFILE_SHA256, DOCKER_RUNTIME_PROFILE_SHA256_HEX,
    DockerDaemonPrerequisites, DockerInferenceResourceEnvelope, DockerInferenceTopology,
    DockerMountPolicy, DockerOfflineNetworkPolicy, DockerRuntimeContractError,
    PinnedDockerRuntimeIdentity,
};
pub use docker_topology_collector::{
    DOCKER_COLLECTOR_MAX_INPUT_BYTES, DockerCollectorInput, DockerCollectorOutput,
    DockerTopologyCollectorError, validate_topology_from_reader,
};

pub use native_model_adapter::{LinuxNativeModelAdapter, NativeModelDriver};
pub use native_runtime::{
    NATIVE_RUNTIME_PACKAGE_ID, NATIVE_RUNTIME_PROFILE_SHA256, NATIVE_RUNTIME_PROFILE_SHA256_HEX,
    NATIVE_RUNTIME_SOURCE_ARCHIVE_SHA256, NativeInferenceResourceEnvelope, NativeInferenceTopology,
    NativeRuntimeContractError, PinnedNativeRuntimeIdentity, RestrictedModelStoreIdentity,
};

/// Stable component identity included in package and process inventories.
pub const COMPONENT_ID: &str = "platform-linux-native-inference";

/// Version of the package/process boundary including mandatory Docker drift preflight.
pub const PROCESS_BOUNDARY_VERSION: u16 = 7;

/// Exact content-free descriptor emitted by the inactive packaged adapter.
pub const BOUNDARY_DESCRIPTION: &[u8] = b"{\"accepted_operation\":\"self-check-only\",\"authority_inputs\":[],\"component_id\":\"platform-linux-native-inference\",\"contract\":\"authenticated-local-endpoint-v1\",\"docker_compatibility_available\":false,\"docker_guard_profile_sha256\":\"86d5cd6860d7e1efe7e4630197912f2b2d40c7a8694f9032c6c67ab9e6ff4a82\",\"docker_model_artifact_digest\":\"sha256:08fa7b1d44f255be48cfc12359211725bfd659742612ed4b221cd5be90d14444\",\"docker_model_runner_image_digest\":\"sha256:bd94095bbc1ddc4266c3a88f582a92562c6b63eceb175572c9a60045663727c9\",\"docker_preflight_contract_version\":3,\"docker_runtime_profile_sha256\":\"b754cd50d0bc278e4513c81f045dea2a36cd6fe73495519ba642db1debc1edbd\",\"enabled_models\":0,\"inference_available\":false,\"native_runtime_package\":\"agentmage-llama-cpp-b10333-cpu-linux-x86_64\",\"native_runtime_profile_sha256\":\"21346c06fb86b418706326b186609f8e1f690d6b57b53e73d02b4ad8e28e53ea\",\"network_listener\":false,\"process_boundary_version\":7}\n";

/// Stable refusal from the pre-runtime adapter process.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxInferenceBoundaryError {
    /// No operation except the exact self-check is representable yet.
    OperationUnavailable,
}

impl LinuxInferenceBoundaryError {
    /// Returns the content-free process error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::OperationUnavailable => {
                "agentmage.native-inference.operation-unavailable-no-admitted-profile"
            }
        }
    }
}

impl fmt::Display for LinuxInferenceBoundaryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for LinuxInferenceBoundaryError {}

/// Evaluates the executable's exact argument closure without reading ambient state.
pub fn evaluate_arguments(
    arguments: &[OsString],
) -> Result<&'static [u8], LinuxInferenceBoundaryError> {
    match arguments {
        [command] if command == "--self-check" => Ok(BOUNDARY_DESCRIPTION),
        _ => Err(LinuxInferenceBoundaryError::OperationUnavailable),
    }
}

/// Constructs the only shared endpoint class this native adapter may use later.
///
/// This function does not open a socket. It binds a platform-observed endpoint
/// digest to the shared authenticated Unix-socket contract and rejects every
/// zero or incompatible identity through the shared constructor.
pub fn authenticated_endpoint_identity(
    endpoint_sha256: [u8; 32],
) -> Result<LocalEndpointIdentity, NetworkEndpointError> {
    LocalEndpointIdentity::new(
        NetworkComponent::KernelNativeInferenceAdapter,
        LocalTransport::AuthenticatedUnixSocket,
        endpoint_sha256,
    )
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use agentmage_kernel_contracts::{LocalTransport, NetworkComponent, NetworkEndpointError};

    use super::{
        BOUNDARY_DESCRIPTION, DOCKER_GUARD_PROFILE_SHA256_HEX, DOCKER_MODEL_ARTIFACT_DIGEST_HEX,
        DOCKER_MODEL_RUNNER_IMAGE_DIGEST_HEX, DOCKER_RUNTIME_PROFILE_SHA256_HEX,
        LinuxInferenceBoundaryError, NATIVE_RUNTIME_PACKAGE_ID, NATIVE_RUNTIME_PROFILE_SHA256_HEX,
        authenticated_endpoint_identity, evaluate_arguments,
    };

    #[test]
    fn only_exact_self_check_is_representable() {
        assert_eq!(
            evaluate_arguments(&[OsString::from("--self-check")]),
            Ok(BOUNDARY_DESCRIPTION)
        );
        for arguments in [
            vec![],
            vec![OsString::from("--serve")],
            vec![OsString::from("--self-check"), OsString::from("extra")],
            vec![OsString::from("--model"), OsString::from("anything")],
        ] {
            assert_eq!(
                evaluate_arguments(&arguments),
                Err(LinuxInferenceBoundaryError::OperationUnavailable)
            );
        }
    }

    #[test]
    fn endpoint_identity_is_shared_authenticated_and_nonzero() {
        let identity = authenticated_endpoint_identity([7; 32]).expect("valid endpoint");
        assert_eq!(
            identity.client(),
            NetworkComponent::KernelNativeInferenceAdapter
        );
        assert_eq!(
            identity.transport(),
            LocalTransport::AuthenticatedUnixSocket
        );
        assert_eq!(identity.identity_sha256(), &[7; 32]);
        assert_eq!(
            authenticated_endpoint_identity([0; 32]),
            Err(NetworkEndpointError::ZeroIdentity)
        );
    }

    #[test]
    fn descriptor_is_inactive_content_free_and_authority_empty() {
        let text = std::str::from_utf8(BOUNDARY_DESCRIPTION).expect("ASCII descriptor");
        for required in [
            "\"authority_inputs\":[]",
            "\"docker_compatibility_available\":false",
            "\"enabled_models\":0",
            "\"inference_available\":false",
            NATIVE_RUNTIME_PACKAGE_ID,
            NATIVE_RUNTIME_PROFILE_SHA256_HEX,
            DOCKER_MODEL_RUNNER_IMAGE_DIGEST_HEX,
            DOCKER_MODEL_ARTIFACT_DIGEST_HEX,
            DOCKER_RUNTIME_PROFILE_SHA256_HEX,
            DOCKER_GUARD_PROFILE_SHA256_HEX,
            "\"docker_preflight_contract_version\":3",
            "\"network_listener\":false",
        ] {
            assert!(text.contains(required));
        }
        for prohibited in ["/home/", "/var/home/", "token", "password", "secret"] {
            assert!(!text.contains(prohibited));
        }
    }
}
