//! Content-free strict-local network and storage observations.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Product component attributed to a network observation.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum NetworkComponent {
    /// Visual Studio Code extension process.
    VisualStudioCodeExtension,
    /// Native protocol bridge.
    NativeBridge,
    /// Security-authoritative kernel.
    Kernel,
    /// Guarded kernel-owned native inference adapter.
    KernelNativeInferenceAdapter,
    /// Guarded kernel-owned Docker compatibility adapter.
    KernelDockerInferenceAdapter,
    /// Sandboxed deterministic tool worker.
    ToolWorker,
    /// Converter or parser worker.
    Converter,
    /// Repository index worker.
    Indexer,
    /// Separate model installer or importer.
    ModelInstaller,
    /// Component absent from the declared inventory.
    Undeclared,
}

/// Local transport classes representable by the strict-local contract.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum LocalTransport {
    /// Mode-restricted authenticated Unix-domain socket.
    AuthenticatedUnixSocket,
    /// Separately gated loopback TCP compatibility endpoint.
    GuardedLoopbackTcp,
}

/// Content-free destination class observed by a platform collector.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum NetworkDestinationClass {
    /// Authenticated private Unix-domain endpoint.
    AuthenticatedLocalSocket,
    /// IPv4 or IPv6 loopback.
    Loopback,
    /// Unspecified all-interface address.
    Unspecified,
    /// Link-local unicast address.
    LinkLocal,
    /// Private or unique-local local-area-network address.
    PrivateLan,
    /// Multicast address.
    Multicast,
    /// Container bridge, alias, or container-reachable address.
    ContainerNetwork,
    /// Ambient proxy or proxy-derived destination.
    Proxy,
    /// Ambient or overridden Domain Name System path.
    Dns,
    /// Public or otherwise external destination.
    External,
    /// Collector could not classify the destination.
    Unknown,
}

/// Invalid exact endpoint declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkEndpointError {
    /// Endpoint identity digest was all zeroes.
    ZeroIdentity,
    /// A native inference declaration used a non-Unix transport.
    NativeTransportMismatch,
    /// A Docker declaration used a non-loopback transport.
    DockerTransportMismatch,
    /// A component without guarded inference authority was declared.
    InvalidClient,
}

impl NetworkEndpointError {
    /// Returns a stable, content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ZeroIdentity => "zero_endpoint_identity",
            Self::NativeTransportMismatch => "native_transport_mismatch",
            Self::DockerTransportMismatch => "docker_transport_mismatch",
            Self::InvalidClient => "invalid_endpoint_client",
        }
    }
}

impl std::fmt::Display for NetworkEndpointError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for NetworkEndpointError {}

/// One exact guarded local inference endpoint.
#[derive(Clone, PartialEq, Eq, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalEndpointIdentity {
    client: NetworkComponent,
    transport: LocalTransport,
    identity_sha256: [u8; 32],
}

impl LocalEndpointIdentity {
    /// Declares one exact endpoint for a guarded kernel inference adapter.
    pub fn new(
        client: NetworkComponent,
        transport: LocalTransport,
        identity_sha256: [u8; 32],
    ) -> Result<Self, NetworkEndpointError> {
        if identity_sha256 == [0; 32] {
            return Err(NetworkEndpointError::ZeroIdentity);
        }
        match (client, transport) {
            (
                NetworkComponent::KernelNativeInferenceAdapter,
                LocalTransport::AuthenticatedUnixSocket,
            )
            | (
                NetworkComponent::KernelDockerInferenceAdapter,
                LocalTransport::GuardedLoopbackTcp,
            ) => Ok(Self {
                client,
                transport,
                identity_sha256,
            }),
            (
                NetworkComponent::KernelNativeInferenceAdapter,
                LocalTransport::GuardedLoopbackTcp,
            ) => Err(NetworkEndpointError::NativeTransportMismatch),
            (
                NetworkComponent::KernelDockerInferenceAdapter,
                LocalTransport::AuthenticatedUnixSocket,
            ) => Err(NetworkEndpointError::DockerTransportMismatch),
            _ => Err(NetworkEndpointError::InvalidClient),
        }
    }

    /// Returns the sole declared client.
    #[must_use]
    pub const fn client(&self) -> NetworkComponent {
        self.client
    }

    /// Returns the exact local transport.
    #[must_use]
    pub const fn transport(&self) -> LocalTransport {
        self.transport
    }

    /// Returns the content-free endpoint identity digest.
    #[must_use]
    pub const fn identity_sha256(&self) -> &[u8; 32] {
        &self.identity_sha256
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LocalEndpointIdentityWire {
    client: NetworkComponent,
    transport: LocalTransport,
    identity_sha256: [u8; 32],
}

impl<'de> serde::Deserialize<'de> for LocalEndpointIdentity {
    fn deserialize<Deserializer>(deserializer: Deserializer) -> Result<Self, Deserializer::Error>
    where
        Deserializer: serde::Deserializer<'de>,
    {
        let wire = LocalEndpointIdentityWire::deserialize(deserializer)?;
        Self::new(wire.client, wire.transport, wire.identity_sha256)
            .map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Debug for LocalEndpointIdentity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LocalEndpointIdentity")
            .field("client", &self.client)
            .field("transport", &self.transport)
            .field("identity", &"sha256:[REDACTED]")
            .finish()
    }
}

/// One platform observation evaluated without retaining a raw address or path.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkObservation {
    /// Component attributed by the process collector.
    pub component: NetworkComponent,
    /// Classified destination without a hostname or address.
    pub destination: NetworkDestinationClass,
    /// Local transport when the attempt targeted a declared local service.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub transport: Option<LocalTransport>,
    /// Observed endpoint identity digest, when available.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub endpoint_sha256: Option<[u8; 32]>,
    /// Whether the platform authenticated the exact local peer.
    pub peer_authenticated: bool,
    /// Attempted byte count, excluding payload content.
    pub attempted_bytes: u64,
}

/// Filesystem class observed for a proposed strict-local data root.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum StorageFilesystemClass {
    /// Locally attached filesystem with no observed remote transport.
    Local,
    /// Network or remote filesystem.
    Remote,
    /// Userspace filesystem, conservatively rejected in strict-local mode.
    Fuse,
    /// Collector could not identify the filesystem safely.
    Unknown,
}

/// Known synchronized-folder marker found in a proposed root ancestry.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum CloudSynchronizationMarker {
    /// Dropbox-managed path.
    Dropbox,
    /// Microsoft OneDrive-managed path.
    OneDrive,
    /// Google Drive-managed path.
    GoogleDrive,
    /// Nextcloud-managed path.
    Nextcloud,
    /// ownCloud-managed path.
    OwnCloud,
    /// Apple iCloud Drive-managed path.
    ICloudDrive,
    /// Syncthing-managed path.
    Syncthing,
    /// Known marker whose provider is not safely attributable.
    OtherKnownMarker,
}

/// Content-free storage observation for strict-local admission.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrictLocalStorageObservation {
    /// Filesystem class established by the platform adapter.
    pub filesystem: StorageFilesystemClass,
    /// Synchronized-folder marker, if one was observed.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub synchronization_marker: Option<CloudSynchronizationMarker>,
    /// Digest of the held root identity, never a path string.
    pub root_identity_sha256: [u8; 32],
    /// Whether every path component was resolved without symbolic links.
    pub symlink_free: bool,
}

/// Classifies an IP address without retaining it in the resulting contract.
#[must_use]
pub const fn classify_ip_destination(address: IpAddr) -> NetworkDestinationClass {
    match address {
        IpAddr::V4(address) => classify_ipv4(address),
        IpAddr::V6(address) => classify_ipv6(address),
    }
}

const fn classify_ipv4(address: Ipv4Addr) -> NetworkDestinationClass {
    let octets = address.octets();
    if address.is_unspecified() {
        NetworkDestinationClass::Unspecified
    } else if address.is_loopback() {
        NetworkDestinationClass::Loopback
    } else if address.is_link_local() {
        NetworkDestinationClass::LinkLocal
    } else if address.is_multicast() {
        NetworkDestinationClass::Multicast
    } else if address.is_private() || octets[0] == 100 && octets[1] & 0b1100_0000 == 64 {
        NetworkDestinationClass::PrivateLan
    } else {
        NetworkDestinationClass::External
    }
}

const fn classify_ipv6(address: Ipv6Addr) -> NetworkDestinationClass {
    let segments = address.segments();
    if address.is_unspecified() {
        NetworkDestinationClass::Unspecified
    } else if address.is_loopback() {
        NetworkDestinationClass::Loopback
    } else if address.is_multicast() {
        NetworkDestinationClass::Multicast
    } else if segments[0] & 0xffc0 == 0xfe80 {
        NetworkDestinationClass::LinkLocal
    } else if segments[0] & 0xfe00 == 0xfc00 {
        NetworkDestinationClass::PrivateLan
    } else {
        NetworkDestinationClass::External
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    use super::{
        LocalEndpointIdentity, LocalTransport, NetworkComponent, NetworkDestinationClass,
        NetworkEndpointError, classify_ip_destination,
    };

    #[test]
    fn endpoint_identity_admits_only_the_two_guarded_topologies() {
        assert!(
            LocalEndpointIdentity::new(
                NetworkComponent::KernelNativeInferenceAdapter,
                LocalTransport::AuthenticatedUnixSocket,
                [1; 32]
            )
            .is_ok()
        );
        assert!(
            LocalEndpointIdentity::new(
                NetworkComponent::KernelDockerInferenceAdapter,
                LocalTransport::GuardedLoopbackTcp,
                [2; 32]
            )
            .is_ok()
        );
        assert_eq!(
            LocalEndpointIdentity::new(
                NetworkComponent::Kernel,
                LocalTransport::AuthenticatedUnixSocket,
                [1; 32]
            ),
            Err(NetworkEndpointError::InvalidClient)
        );
        assert_eq!(
            LocalEndpointIdentity::new(
                NetworkComponent::KernelNativeInferenceAdapter,
                LocalTransport::AuthenticatedUnixSocket,
                [0; 32]
            ),
            Err(NetworkEndpointError::ZeroIdentity)
        );
    }

    #[test]
    fn endpoint_deserialization_reapplies_all_invariants() {
        let endpoint = LocalEndpointIdentity::new(
            NetworkComponent::KernelNativeInferenceAdapter,
            LocalTransport::AuthenticatedUnixSocket,
            [3; 32],
        )
        .expect("valid endpoint");
        let encoded = serde_json::to_value(&endpoint).expect("serialize endpoint");
        assert_eq!(
            serde_json::from_value::<LocalEndpointIdentity>(encoded).expect("deserialize endpoint"),
            endpoint
        );

        for malformed in [
            serde_json::json!({
                "client": "kernel_native_inference_adapter",
                "transport": "authenticated_unix_socket",
                "identity_sha256": vec![0_u8; 32],
            }),
            serde_json::json!({
                "client": "kernel_native_inference_adapter",
                "transport": "guarded_loopback_tcp",
                "identity_sha256": vec![1_u8; 32],
            }),
            serde_json::json!({
                "client": "kernel",
                "transport": "authenticated_unix_socket",
                "identity_sha256": vec![1_u8; 32],
            }),
            serde_json::json!({
                "client": "kernel_docker_inference_adapter",
                "transport": "guarded_loopback_tcp",
                "identity_sha256": vec![1_u8; 32],
                "unexpected": true,
            }),
        ] {
            assert!(serde_json::from_value::<LocalEndpointIdentity>(malformed).is_err());
        }
    }

    #[test]
    fn ip_classification_covers_local_lan_multicast_and_external_ranges() {
        for (address, expected) in [
            (
                IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                NetworkDestinationClass::Unspecified,
            ),
            (
                IpAddr::V4(Ipv4Addr::LOCALHOST),
                NetworkDestinationClass::Loopback,
            ),
            (
                IpAddr::V4(Ipv4Addr::new(169, 254, 1, 2)),
                NetworkDestinationClass::LinkLocal,
            ),
            (
                IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2)),
                NetworkDestinationClass::PrivateLan,
            ),
            (
                IpAddr::V4(Ipv4Addr::new(100, 64, 1, 2)),
                NetworkDestinationClass::PrivateLan,
            ),
            (
                IpAddr::V4(Ipv4Addr::new(224, 0, 0, 1)),
                NetworkDestinationClass::Multicast,
            ),
            (
                IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
                NetworkDestinationClass::External,
            ),
            (
                IpAddr::V6(Ipv6Addr::LOCALHOST),
                NetworkDestinationClass::Loopback,
            ),
            (
                "fe80::1".parse().expect("link-local IPv6"),
                NetworkDestinationClass::LinkLocal,
            ),
            (
                "fd00::1".parse().expect("unique-local IPv6"),
                NetworkDestinationClass::PrivateLan,
            ),
            (
                "ff02::1".parse().expect("multicast IPv6"),
                NetworkDestinationClass::Multicast,
            ),
            (
                "2001:4860:4860::8888".parse().expect("external IPv6"),
                NetworkDestinationClass::External,
            ),
        ] {
            assert_eq!(classify_ip_destination(address), expected);
        }
    }
}
