#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Isolated Linux native-inference process boundary.

use std::ffi::OsString;
use std::fmt;

use agentmage_kernel_contracts::{
    LocalEndpointIdentity, LocalTransport, NetworkComponent, NetworkEndpointError,
};

/// Stable component identity included in package and process inventories.
pub const COMPONENT_ID: &str = "platform-linux-native-inference";

/// Version of the package/process boundary implemented before model-runtime work.
pub const PROCESS_BOUNDARY_VERSION: u16 = 1;

/// Exact content-free descriptor emitted by the inactive packaged adapter.
pub const BOUNDARY_DESCRIPTION: &[u8] = b"{\"accepted_operation\":\"self-check-only\",\"authority_inputs\":[],\"component_id\":\"platform-linux-native-inference\",\"contract\":\"authenticated-local-endpoint-v1\",\"enabled_models\":0,\"inference_available\":false,\"network_listener\":false,\"process_boundary_version\":1}\n";

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
        BOUNDARY_DESCRIPTION, LinuxInferenceBoundaryError, authenticated_endpoint_identity,
        evaluate_arguments,
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
            "\"enabled_models\":0",
            "\"inference_available\":false",
            "\"network_listener\":false",
        ] {
            assert!(text.contains(required));
        }
        for prohibited in ["/home/", "/var/home/", "token", "password", "secret"] {
            assert!(!text.contains(prohibited));
        }
    }
}
