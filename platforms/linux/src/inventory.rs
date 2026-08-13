//! Content-free Linux process, socket, port, and writable-descriptor inventory.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use agentmage_kernel_contracts::{
    LocalEndpointIdentity, NetworkComponent, NetworkDestinationClass, ToolDefinition, ToolId,
    classify_ip_destination, to_canonical_json,
};
use agentmage_kernel_engine::{strict_local::StrictLocalNetworkPolicy, tooling::ToolRegistry};
use rustix::io::Errno;
use rustix::process::{Pid, PidfdFlags, pidfd_open};
use sha2::{Digest, Sha256};

use crate::ipc::{process_executable_sha256, process_start_time_ticks};

/// Maximum number of explicitly attributed processes in one session snapshot.
pub const MAX_INVENTORY_PROCESSES: usize = 128;
/// Maximum sockets retained in one session snapshot.
pub const MAX_INVENTORY_SOCKETS: usize = 4096;
/// Maximum writable descriptors retained in one session snapshot.
pub const MAX_INVENTORY_WRITABLES: usize = 4096;
/// Maximum number of exact listeners declared for one strict-local session.
pub const MAX_DECLARED_LISTENERS: usize = 32;
/// Maximum number of exact tool definitions in one strict-local session manifest.
pub const MAX_DECLARED_TOOLS: usize = 256;

const MAX_PROC_RECORD_BYTES: u64 = 8 * 1024 * 1024;

/// One explicitly attributed process requested for collection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LinuxInventoryTarget {
    /// Linux process identifier.
    pub pid: u32,
    /// Closed product component attribution.
    pub component: NetworkComponent,
}

/// Linux socket protocol represented without an address or payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LinuxSocketProtocol {
    /// Unix stream socket.
    UnixStream,
    /// Unix datagram socket.
    UnixDatagram,
    /// Unix sequenced-packet socket.
    UnixSequencedPacket,
    /// Other Unix socket type.
    UnixOther,
    /// IPv4 TCP socket.
    Tcp4,
    /// IPv6 TCP socket.
    Tcp6,
    /// IPv4 UDP socket.
    Udp4,
    /// IPv6 UDP socket.
    Udp6,
    /// Socket descriptor absent from the supported kernel tables.
    Other,
}

/// Content-free kernel socket state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LinuxSocketState {
    /// Socket accepts inbound connections.
    Listening,
    /// Socket has a connected peer.
    Connected,
    /// Socket is bound or unconnected.
    Bound,
    /// State could not be safely mapped.
    Other,
}

/// One process-attributed socket descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxSocketObservation {
    pid: u32,
    descriptor: u32,
    inode: u64,
    protocol: LinuxSocketProtocol,
    state: LinuxSocketState,
    local_destination: NetworkDestinationClass,
    remote_destination: NetworkDestinationClass,
    local_port: Option<u16>,
    remote_port: Option<u16>,
    endpoint_sha256: Option<[u8; 32]>,
}

impl LinuxSocketObservation {
    /// Returns the owning process identifier.
    #[must_use]
    pub const fn pid(&self) -> u32 {
        self.pid
    }

    /// Returns the owning descriptor number.
    #[must_use]
    pub const fn descriptor(&self) -> u32 {
        self.descriptor
    }

    /// Returns the kernel socket inode.
    #[must_use]
    pub const fn inode(&self) -> u64 {
        self.inode
    }

    /// Returns the protocol class.
    #[must_use]
    pub const fn protocol(&self) -> LinuxSocketProtocol {
        self.protocol
    }

    /// Returns the state class.
    #[must_use]
    pub const fn state(&self) -> LinuxSocketState {
        self.state
    }

    /// Returns the local address class without the address.
    #[must_use]
    pub const fn local_destination(&self) -> NetworkDestinationClass {
        self.local_destination
    }

    /// Returns the peer address class without the address.
    #[must_use]
    pub const fn remote_destination(&self) -> NetworkDestinationClass {
        self.remote_destination
    }

    /// Returns the local port for an internet socket.
    #[must_use]
    pub const fn local_port(&self) -> Option<u16> {
        self.local_port
    }

    /// Returns the peer port for an internet socket.
    #[must_use]
    pub const fn remote_port(&self) -> Option<u16> {
        self.remote_port
    }

    /// Returns a Unix endpoint digest when its kernel table had a name.
    #[must_use]
    pub const fn endpoint_sha256(&self) -> Option<&[u8; 32]> {
        self.endpoint_sha256.as_ref()
    }
}

/// Class of a writable descriptor target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LinuxWritableTargetClass {
    /// Absolute filesystem target.
    AbsolutePath,
    /// Filesystem target unlinked after it was opened.
    DeletedPath,
    /// Anonymous in-memory descriptor.
    AnonymousMemory,
    /// Pipe or other non-socket writable descriptor.
    Other,
}

/// One content-free writable descriptor observation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxWritableObservation {
    pid: u32,
    descriptor: u32,
    target_class: LinuxWritableTargetClass,
    target_sha256: [u8; 32],
}

impl LinuxWritableObservation {
    /// Returns the owning process identifier.
    #[must_use]
    pub const fn pid(&self) -> u32 {
        self.pid
    }

    /// Returns the descriptor number.
    #[must_use]
    pub const fn descriptor(&self) -> u32 {
        self.descriptor
    }

    /// Returns the target class.
    #[must_use]
    pub const fn target_class(&self) -> LinuxWritableTargetClass {
        self.target_class
    }

    /// Returns the digest of the kernel-rendered target without returning the target.
    #[must_use]
    pub const fn target_sha256(&self) -> &[u8; 32] {
        &self.target_sha256
    }
}

/// One process identity captured without command-line or environment content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxSessionProcessObservation {
    pid: u32,
    parent_pid: u32,
    uid: u32,
    component: NetworkComponent,
    start_time_ticks: u64,
    identity_binding: LinuxProcessIdentityBinding,
    executable_sha256: [u8; 32],
}

/// Strength of the kernel process identity retained during inventory collection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxProcessIdentityBinding {
    /// A pidfd retained the original process while start time rejected PID reuse.
    PidFdAndStartTime,
    /// The kernel lacks pidfd support; start time still rejected PID reuse.
    StartTimeOnly,
}

impl LinuxSessionProcessObservation {
    /// Returns the process identifier.
    #[must_use]
    pub const fn pid(&self) -> u32 {
        self.pid
    }

    /// Returns the observed parent process identifier.
    #[must_use]
    pub const fn parent_pid(&self) -> u32 {
        self.parent_pid
    }

    /// Returns the real user identifier from `/proc`.
    #[must_use]
    pub const fn uid(&self) -> u32 {
        self.uid
    }

    /// Returns the declared component attribution.
    #[must_use]
    pub const fn component(&self) -> NetworkComponent {
        self.component
    }

    /// Returns the kernel process start time in clock ticks since boot.
    #[must_use]
    pub const fn start_time_ticks(&self) -> u64 {
        self.start_time_ticks
    }

    /// Returns the process identity mechanism available for this observation.
    #[must_use]
    pub const fn identity_binding(&self) -> LinuxProcessIdentityBinding {
        self.identity_binding
    }

    /// Returns the executable content digest.
    #[must_use]
    pub const fn executable_sha256(&self) -> &[u8; 32] {
        &self.executable_sha256
    }
}

/// One bounded content-free Linux session inventory snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxSessionInventory {
    scope: LinuxInventoryScope,
    processes: Vec<LinuxSessionProcessObservation>,
    sockets: Vec<LinuxSocketObservation>,
    writable_descriptors: Vec<LinuxWritableObservation>,
}

/// Completeness boundary used to produce one Linux session inventory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxInventoryScope {
    /// Only caller-supplied process identifiers were observed.
    ExplicitTargets,
    /// Unified-cgroup membership matched the complete target set before and after collection.
    CompleteUnifiedCgroup,
}

impl LinuxSessionInventory {
    /// Returns the completeness boundary used for this snapshot.
    #[must_use]
    pub const fn scope(&self) -> LinuxInventoryScope {
        self.scope
    }

    /// Returns process identities in ascending PID order.
    #[must_use]
    pub fn processes(&self) -> &[LinuxSessionProcessObservation] {
        &self.processes
    }

    /// Returns process-attributed sockets in stable order.
    #[must_use]
    pub fn sockets(&self) -> &[LinuxSocketObservation] {
        &self.sockets
    }

    /// Returns writable descriptors in stable order.
    #[must_use]
    pub fn writable_descriptors(&self) -> &[LinuxWritableObservation] {
        &self.writable_descriptors
    }
}

/// One exact listener allowed by a strict-local Linux session manifest.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LinuxDeclaredListener {
    component: NetworkComponent,
    protocol: LinuxSocketProtocol,
    local_destination: NetworkDestinationClass,
    local_port: Option<u16>,
    endpoint_sha256: Option<[u8; 32]>,
}

impl LinuxDeclaredListener {
    /// Declares one exact loopback TCP listener owned by an approved component.
    pub fn loopback_tcp(
        component: NetworkComponent,
        protocol: LinuxSocketProtocol,
        port: u16,
    ) -> Result<Self, LinuxListenerBoundaryError> {
        if !listener_component_allowed(component)
            || !matches!(
                protocol,
                LinuxSocketProtocol::Tcp4 | LinuxSocketProtocol::Tcp6
            )
            || port == 0
        {
            return Err(listener_error(
                LinuxListenerBoundaryErrorKind::InvalidDeclaration,
            ));
        }
        Ok(Self {
            component,
            protocol,
            local_destination: NetworkDestinationClass::Loopback,
            local_port: Some(port),
            endpoint_sha256: None,
        })
    }

    /// Declares one exact named Unix-stream listener owned by an approved component.
    pub fn unix_stream(
        component: NetworkComponent,
        endpoint_sha256: [u8; 32],
    ) -> Result<Self, LinuxListenerBoundaryError> {
        if !listener_component_allowed(component) || endpoint_sha256 == [0; 32] {
            return Err(listener_error(
                LinuxListenerBoundaryErrorKind::InvalidDeclaration,
            ));
        }
        Ok(Self {
            component,
            protocol: LinuxSocketProtocol::UnixStream,
            local_destination: NetworkDestinationClass::LocalSocket,
            local_port: None,
            endpoint_sha256: Some(endpoint_sha256),
        })
    }

    /// Returns the declared owner component.
    #[must_use]
    pub const fn component(&self) -> NetworkComponent {
        self.component
    }

    /// Returns the declared listener protocol.
    #[must_use]
    pub const fn protocol(&self) -> LinuxSocketProtocol {
        self.protocol
    }
}

impl fmt::Debug for LinuxDeclaredListener {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxDeclaredListener")
            .field("component", &self.component)
            .field("protocol", &self.protocol)
            .field("local_destination", &self.local_destination)
            .field("local_port", &self.local_port)
            .field(
                "endpoint_identity",
                &self.endpoint_sha256.map(|_| "sha256:[REDACTED]"),
            )
            .finish()
    }
}

/// Stable refusal class for strict-local listener reconciliation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxListenerBoundaryErrorKind {
    /// A declaration used an unapproved component, protocol, port, or identity.
    InvalidDeclaration,
    /// The declared-listener set exceeded its closed bound.
    ResourceLimitExceeded,
    /// The same exact listener was declared more than once.
    DuplicateDeclaration,
    /// An observed listener did not belong to an inventoried process.
    UnknownProcess,
    /// A listener used an unspecified, LAN, container, external, or unknown address class.
    UnsafeDestination,
    /// A bound socket existed without an exact supported listener declaration.
    UndeclaredBinding,
    /// A socket state could not be proven non-listening.
    IndeterminateSocketState,
    /// A locally scoped listener was absent from the exact manifest.
    UndeclaredListener,
    /// One declared listener was absent from the session inventory.
    MissingDeclaredListener,
    /// More than one distinct socket object matched one declaration.
    DuplicateObservedListener,
}

/// Content-free strict-local listener refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxListenerBoundaryError {
    kind: LinuxListenerBoundaryErrorKind,
}

impl LinuxListenerBoundaryError {
    /// Returns the stable refusal class.
    #[must_use]
    pub const fn kind(self) -> LinuxListenerBoundaryErrorKind {
        self.kind
    }
}

impl fmt::Display for LinuxListenerBoundaryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            LinuxListenerBoundaryErrorKind::InvalidDeclaration => {
                "strict_local_listener.invalid_declaration"
            }
            LinuxListenerBoundaryErrorKind::ResourceLimitExceeded => {
                "strict_local_listener.resource_limit"
            }
            LinuxListenerBoundaryErrorKind::DuplicateDeclaration => {
                "strict_local_listener.duplicate_declaration"
            }
            LinuxListenerBoundaryErrorKind::UnknownProcess => {
                "strict_local_listener.unknown_process"
            }
            LinuxListenerBoundaryErrorKind::UnsafeDestination => {
                "strict_local_listener.unsafe_destination"
            }
            LinuxListenerBoundaryErrorKind::UndeclaredBinding => {
                "strict_local_listener.undeclared_binding"
            }
            LinuxListenerBoundaryErrorKind::IndeterminateSocketState => {
                "strict_local_listener.indeterminate_socket_state"
            }
            LinuxListenerBoundaryErrorKind::UndeclaredListener => {
                "strict_local_listener.undeclared"
            }
            LinuxListenerBoundaryErrorKind::MissingDeclaredListener => {
                "strict_local_listener.missing_declared"
            }
            LinuxListenerBoundaryErrorKind::DuplicateObservedListener => {
                "strict_local_listener.duplicate_observed"
            }
        })
    }
}

impl std::error::Error for LinuxListenerBoundaryError {}

/// Successful exact listener reconciliation without retaining addresses or paths.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxListenerBoundaryReceipt {
    declared_listeners: usize,
    observed_listeners: usize,
}

impl LinuxListenerBoundaryReceipt {
    /// Returns the number of exact manifest declarations.
    #[must_use]
    pub const fn declared_listeners(self) -> usize {
        self.declared_listeners
    }

    /// Returns the number of distinct matching socket objects.
    #[must_use]
    pub const fn observed_listeners(self) -> usize {
        self.observed_listeners
    }
}

/// Exact fail-closed listener policy for one Linux strict-local session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxSessionListenerPolicy {
    declared: Vec<LinuxDeclaredListener>,
}

impl LinuxSessionListenerPolicy {
    /// Constructs one bounded unique declaration set.
    pub fn new(declared: &[LinuxDeclaredListener]) -> Result<Self, LinuxListenerBoundaryError> {
        if declared.len() > MAX_DECLARED_LISTENERS {
            return Err(listener_error(
                LinuxListenerBoundaryErrorKind::ResourceLimitExceeded,
            ));
        }
        let mut declared = declared.to_vec();
        declared.sort_unstable();
        if declared.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(listener_error(
                LinuxListenerBoundaryErrorKind::DuplicateDeclaration,
            ));
        }
        Ok(Self { declared })
    }

    /// Requires every observed listener to match one declaration exactly and vice versa.
    pub fn reconcile(
        &self,
        inventory: &LinuxSessionInventory,
    ) -> Result<LinuxListenerBoundaryReceipt, LinuxListenerBoundaryError> {
        let processes = inventory
            .processes
            .iter()
            .map(|process| (process.pid, process.component))
            .collect::<BTreeMap<_, _>>();
        let mut matched = vec![None; self.declared.len()];
        let mut observed_objects = BTreeSet::new();
        for socket in &inventory.sockets {
            match socket.state {
                LinuxSocketState::Connected => continue,
                LinuxSocketState::Bound => {
                    return Err(listener_error(
                        LinuxListenerBoundaryErrorKind::UndeclaredBinding,
                    ));
                }
                LinuxSocketState::Other => {
                    return Err(listener_error(
                        LinuxListenerBoundaryErrorKind::IndeterminateSocketState,
                    ));
                }
                LinuxSocketState::Listening => {}
            }
            if !observed_objects.insert((socket.pid, socket.inode)) {
                continue;
            }
            let component = processes
                .get(&socket.pid)
                .copied()
                .ok_or_else(|| listener_error(LinuxListenerBoundaryErrorKind::UnknownProcess))?;
            if !matches!(
                socket.local_destination,
                NetworkDestinationClass::Loopback | NetworkDestinationClass::LocalSocket
            ) {
                return Err(listener_error(
                    LinuxListenerBoundaryErrorKind::UnsafeDestination,
                ));
            }
            let observed = LinuxDeclaredListener {
                component,
                protocol: socket.protocol,
                local_destination: socket.local_destination,
                local_port: socket.local_port,
                endpoint_sha256: socket.endpoint_sha256,
            };
            let index = self
                .declared
                .binary_search(&observed)
                .map_err(|_| listener_error(LinuxListenerBoundaryErrorKind::UndeclaredListener))?;
            if matched[index].replace((socket.pid, socket.inode)).is_some() {
                return Err(listener_error(
                    LinuxListenerBoundaryErrorKind::DuplicateObservedListener,
                ));
            }
        }
        if matched.iter().any(Option::is_none) {
            return Err(listener_error(
                LinuxListenerBoundaryErrorKind::MissingDeclaredListener,
            ));
        }
        Ok(LinuxListenerBoundaryReceipt {
            declared_listeners: self.declared.len(),
            observed_listeners: observed_objects.len(),
        })
    }
}

/// One expected process shape in a strict-local session topology.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LinuxDeclaredProcess {
    component: NetworkComponent,
    parent_component: Option<NetworkComponent>,
    uid: u32,
    executable_sha256: [u8; 32],
    count: usize,
}

impl LinuxDeclaredProcess {
    /// Declares one or more processes with an exact owner, executable, and parent class.
    #[must_use]
    pub const fn new(
        component: NetworkComponent,
        parent_component: Option<NetworkComponent>,
        uid: u32,
        executable_sha256: [u8; 32],
        count: usize,
    ) -> Self {
        Self {
            component,
            parent_component,
            uid,
            executable_sha256,
            count,
        }
    }

    /// Returns the declared component.
    #[must_use]
    pub const fn component(&self) -> NetworkComponent {
        self.component
    }

    /// Returns the parent component, or `None` for a parent outside the session.
    #[must_use]
    pub const fn parent_component(&self) -> Option<NetworkComponent> {
        self.parent_component
    }

    /// Returns the declared process count.
    #[must_use]
    pub const fn count(&self) -> usize {
        self.count
    }
}

/// One expected unique kernel socket shape in a strict-local session topology.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LinuxDeclaredSocket {
    component: NetworkComponent,
    protocol: LinuxSocketProtocol,
    state: LinuxSocketState,
    local_destination: NetworkDestinationClass,
    remote_destination: NetworkDestinationClass,
    local_port: Option<u16>,
    remote_port: Option<u16>,
    endpoint_sha256: Option<[u8; 32]>,
    count: usize,
}

impl LinuxDeclaredSocket {
    /// Declares one or more sockets without retaining an address or Unix path.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        component: NetworkComponent,
        protocol: LinuxSocketProtocol,
        state: LinuxSocketState,
        local_destination: NetworkDestinationClass,
        remote_destination: NetworkDestinationClass,
        local_port: Option<u16>,
        remote_port: Option<u16>,
        endpoint_sha256: Option<[u8; 32]>,
        count: usize,
    ) -> Self {
        Self {
            component,
            protocol,
            state,
            local_destination,
            remote_destination,
            local_port,
            remote_port,
            endpoint_sha256,
            count,
        }
    }

    /// Returns the declared owner component.
    #[must_use]
    pub const fn component(&self) -> NetworkComponent {
        self.component
    }

    /// Returns the declared socket count after descriptor aliases are collapsed.
    #[must_use]
    pub const fn count(&self) -> usize {
        self.count
    }
}

/// One expected writable-descriptor target shape in a strict-local session topology.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LinuxDeclaredWritable {
    component: NetworkComponent,
    target_class: LinuxWritableTargetClass,
    target_sha256: [u8; 32],
    count: usize,
}

impl LinuxDeclaredWritable {
    /// Declares one or more writable descriptors for one content-free target identity.
    #[must_use]
    pub const fn new(
        component: NetworkComponent,
        target_class: LinuxWritableTargetClass,
        target_sha256: [u8; 32],
        count: usize,
    ) -> Self {
        Self {
            component,
            target_class,
            target_sha256,
            count,
        }
    }

    /// Returns the declared owner component.
    #[must_use]
    pub const fn component(&self) -> NetworkComponent {
        self.component
    }

    /// Returns the declared writable-descriptor count.
    #[must_use]
    pub const fn count(&self) -> usize {
        self.count
    }
}

/// One exact registered tool definition retained by a session manifest.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LinuxDeclaredTool {
    tool_id: ToolId,
    tool_version: String,
    definition_sha256: [u8; 32],
}

impl LinuxDeclaredTool {
    /// Creates an exact tool declaration from its canonical validated definition.
    pub fn from_definition(definition: &ToolDefinition) -> Result<Self, LinuxSessionBoundaryError> {
        let bytes = to_canonical_json(definition)
            .map_err(|_| boundary_error(LinuxSessionBoundaryErrorKind::InvalidManifest))?;
        Ok(Self {
            tool_id: definition.tool_id.clone(),
            tool_version: definition.tool_version.clone(),
            definition_sha256: Sha256::digest(bytes).into(),
        })
    }

    /// Returns the stable tool identity.
    #[must_use]
    pub const fn tool_id(&self) -> &ToolId {
        &self.tool_id
    }

    /// Returns the exact tool-contract version.
    #[must_use]
    pub fn tool_version(&self) -> &str {
        &self.tool_version
    }

    /// Returns the canonical definition digest.
    #[must_use]
    pub const fn definition_sha256(&self) -> &[u8; 32] {
        &self.definition_sha256
    }
}

/// The sole exact network allow rule retained by a strict-local session manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxDeclaredNetworkRule {
    endpoint: LocalEndpointIdentity,
}

impl LinuxDeclaredNetworkRule {
    /// Captures the exact endpoint identity from the active kernel policy.
    #[must_use]
    pub fn from_policy(policy: &StrictLocalNetworkPolicy) -> Self {
        Self {
            endpoint: policy.endpoint().clone(),
        }
    }

    /// Returns the exact guarded endpoint declaration.
    #[must_use]
    pub const fn endpoint(&self) -> &LocalEndpointIdentity {
        &self.endpoint
    }
}

/// Stable refusal class for complete Linux session-boundary reconciliation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxSessionBoundaryErrorKind {
    /// A declaration was empty, duplicated, internally inconsistent, or unsafe.
    InvalidManifest,
    /// A closed declaration or observation bound was exceeded.
    ResourceLimitExceeded,
    /// Observed process topology did not equal the exact declaration.
    ProcessInventoryMismatch,
    /// Observed unique socket and port shapes did not equal the exact declaration.
    SocketInventoryMismatch,
    /// Observed writable targets did not equal the exact declaration.
    WritableInventoryMismatch,
    /// The active tool registry did not equal the exact declaration.
    ToolInventoryMismatch,
    /// The active strict-local network rule did not equal the exact declaration.
    NetworkRuleMismatch,
    /// Listener reconciliation failed within the complete session boundary.
    ListenerBoundary,
}

/// Content-free complete Linux session-boundary refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxSessionBoundaryError {
    kind: LinuxSessionBoundaryErrorKind,
}

impl LinuxSessionBoundaryError {
    /// Returns the stable refusal class.
    #[must_use]
    pub const fn kind(self) -> LinuxSessionBoundaryErrorKind {
        self.kind
    }
}

impl fmt::Display for LinuxSessionBoundaryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            LinuxSessionBoundaryErrorKind::InvalidManifest => "session_boundary.invalid_manifest",
            LinuxSessionBoundaryErrorKind::ResourceLimitExceeded => {
                "session_boundary.resource_limit"
            }
            LinuxSessionBoundaryErrorKind::ProcessInventoryMismatch => {
                "session_boundary.process_mismatch"
            }
            LinuxSessionBoundaryErrorKind::SocketInventoryMismatch => {
                "session_boundary.socket_mismatch"
            }
            LinuxSessionBoundaryErrorKind::WritableInventoryMismatch => {
                "session_boundary.writable_mismatch"
            }
            LinuxSessionBoundaryErrorKind::ToolInventoryMismatch => {
                "session_boundary.tool_mismatch"
            }
            LinuxSessionBoundaryErrorKind::NetworkRuleMismatch => {
                "session_boundary.network_rule_mismatch"
            }
            LinuxSessionBoundaryErrorKind::ListenerBoundary => "session_boundary.listener_mismatch",
        })
    }
}

impl std::error::Error for LinuxSessionBoundaryError {}

/// Exact expected process, socket, path, tool, listener, and network-rule topology.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxSessionBoundaryManifest {
    processes: Vec<LinuxDeclaredProcess>,
    sockets: Vec<LinuxDeclaredSocket>,
    writables: Vec<LinuxDeclaredWritable>,
    tools: Vec<LinuxDeclaredTool>,
    listener_policy: LinuxSessionListenerPolicy,
    network_rule: LinuxDeclaredNetworkRule,
}

impl LinuxSessionBoundaryManifest {
    /// Creates a bounded exact topology declaration for one strict-local session.
    pub fn new(
        mut processes: Vec<LinuxDeclaredProcess>,
        mut sockets: Vec<LinuxDeclaredSocket>,
        mut writables: Vec<LinuxDeclaredWritable>,
        mut tools: Vec<LinuxDeclaredTool>,
        listener_policy: LinuxSessionListenerPolicy,
        network_rule: LinuxDeclaredNetworkRule,
    ) -> Result<Self, LinuxSessionBoundaryError> {
        validate_declaration_counts(&processes, &sockets, &writables, &tools)?;
        processes.sort_unstable();
        sockets.sort_unstable();
        writables.sort_unstable();
        tools.sort_unstable();
        if adjacent_process_identity_matches(&processes)
            || adjacent_socket_identity_matches(&sockets)
            || adjacent_writable_identity_matches(&writables)
            || tools.windows(2).any(|pair| pair[0] == pair[1])
        {
            return Err(boundary_error(
                LinuxSessionBoundaryErrorKind::InvalidManifest,
            ));
        }
        Ok(Self {
            processes,
            sockets,
            writables,
            tools,
            listener_policy,
            network_rule,
        })
    }

    /// Reconciles every declared boundary against one identity-stable observation.
    pub fn reconcile(
        &self,
        inventory: &LinuxSessionInventory,
        registry: &ToolRegistry,
        policy: &StrictLocalNetworkPolicy,
    ) -> Result<LinuxSessionBoundaryReport, LinuxSessionBoundaryError> {
        if inventory.scope != LinuxInventoryScope::CompleteUnifiedCgroup {
            return Err(boundary_error(
                LinuxSessionBoundaryErrorKind::ProcessInventoryMismatch,
            ));
        }
        let observed_processes = normalized_process_inventory(inventory)?;
        if observed_processes != self.processes {
            return Err(boundary_error(
                LinuxSessionBoundaryErrorKind::ProcessInventoryMismatch,
            ));
        }
        let observed_sockets = normalized_socket_inventory(inventory)?;
        if observed_sockets != self.sockets {
            return Err(boundary_error(
                LinuxSessionBoundaryErrorKind::SocketInventoryMismatch,
            ));
        }
        let observed_writables = normalized_writable_inventory(inventory)?;
        if observed_writables != self.writables {
            return Err(boundary_error(
                LinuxSessionBoundaryErrorKind::WritableInventoryMismatch,
            ));
        }
        let observed_tools = registered_tools(registry)?;
        if observed_tools != self.tools {
            return Err(boundary_error(
                LinuxSessionBoundaryErrorKind::ToolInventoryMismatch,
            ));
        }
        let observed_network_rule = LinuxDeclaredNetworkRule::from_policy(policy);
        if observed_network_rule != self.network_rule {
            return Err(boundary_error(
                LinuxSessionBoundaryErrorKind::NetworkRuleMismatch,
            ));
        }
        let listener_receipt = self
            .listener_policy
            .reconcile(inventory)
            .map_err(|_| boundary_error(LinuxSessionBoundaryErrorKind::ListenerBoundary))?;
        Ok(LinuxSessionBoundaryReport {
            inventory: inventory.clone(),
            tools: observed_tools,
            network_rule: observed_network_rule,
            listener_receipt,
        })
    }
}

/// Content-free startup report for one exactly reconciled Linux session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxSessionBoundaryReport {
    inventory: LinuxSessionInventory,
    tools: Vec<LinuxDeclaredTool>,
    network_rule: LinuxDeclaredNetworkRule,
    listener_receipt: LinuxListenerBoundaryReceipt,
}

impl LinuxSessionBoundaryReport {
    /// Returns every process, socket, port, and writable-target observation.
    #[must_use]
    pub const fn inventory(&self) -> &LinuxSessionInventory {
        &self.inventory
    }

    /// Returns every exact enabled tool definition in stable order.
    #[must_use]
    pub fn tools(&self) -> &[LinuxDeclaredTool] {
        &self.tools
    }

    /// Returns the sole exact strict-local network allow rule.
    #[must_use]
    pub const fn network_rule(&self) -> &LinuxDeclaredNetworkRule {
        &self.network_rule
    }

    /// Returns the exact listener-reconciliation counts.
    #[must_use]
    pub const fn listener_receipt(&self) -> LinuxListenerBoundaryReceipt {
        self.listener_receipt
    }
}

fn validate_declaration_counts(
    processes: &[LinuxDeclaredProcess],
    sockets: &[LinuxDeclaredSocket],
    writables: &[LinuxDeclaredWritable],
    tools: &[LinuxDeclaredTool],
) -> Result<(), LinuxSessionBoundaryError> {
    if processes.is_empty()
        || processes.len() > MAX_INVENTORY_PROCESSES
        || sockets.len() > MAX_INVENTORY_SOCKETS
        || writables.len() > MAX_INVENTORY_WRITABLES
        || tools.len() > MAX_DECLARED_TOOLS
    {
        return Err(boundary_error(
            LinuxSessionBoundaryErrorKind::ResourceLimitExceeded,
        ));
    }
    if processes.iter().any(|declaration| {
        declaration.count == 0
            || declaration.executable_sha256 == [0; 32]
            || declaration.component == NetworkComponent::Undeclared
            || declaration.parent_component == Some(NetworkComponent::Undeclared)
    }) || sockets.iter().any(|declaration| {
        declaration.count == 0
            || declaration.component == NetworkComponent::Undeclared
            || declaration.endpoint_sha256 == Some([0; 32])
            || declaration.state == LinuxSocketState::Other
    }) || writables.iter().any(|declaration| {
        declaration.count == 0
            || declaration.component == NetworkComponent::Undeclared
            || declaration.target_sha256 == [0; 32]
    }) || tools.iter().any(|declaration| {
        declaration.tool_id.as_str().is_empty()
            || declaration.tool_version.is_empty()
            || declaration.definition_sha256 == [0; 32]
    }) {
        return Err(boundary_error(
            LinuxSessionBoundaryErrorKind::InvalidManifest,
        ));
    }
    if checked_declared_total(processes.iter().map(|entry| entry.count))
        .is_none_or(|count| count > MAX_INVENTORY_PROCESSES)
        || checked_declared_total(sockets.iter().map(|entry| entry.count))
            .is_none_or(|count| count > MAX_INVENTORY_SOCKETS)
        || checked_declared_total(writables.iter().map(|entry| entry.count))
            .is_none_or(|count| count > MAX_INVENTORY_WRITABLES)
    {
        return Err(boundary_error(
            LinuxSessionBoundaryErrorKind::ResourceLimitExceeded,
        ));
    }
    Ok(())
}

fn checked_declared_total(mut counts: impl Iterator<Item = usize>) -> Option<usize> {
    counts.try_fold(0_usize, usize::checked_add)
}

fn adjacent_process_identity_matches(entries: &[LinuxDeclaredProcess]) -> bool {
    entries.windows(2).any(|pair| {
        pair[0].component == pair[1].component
            && pair[0].parent_component == pair[1].parent_component
            && pair[0].uid == pair[1].uid
            && pair[0].executable_sha256 == pair[1].executable_sha256
    })
}

fn adjacent_socket_identity_matches(entries: &[LinuxDeclaredSocket]) -> bool {
    entries.windows(2).any(|pair| {
        pair[0].component == pair[1].component
            && pair[0].protocol == pair[1].protocol
            && pair[0].state == pair[1].state
            && pair[0].local_destination == pair[1].local_destination
            && pair[0].remote_destination == pair[1].remote_destination
            && pair[0].local_port == pair[1].local_port
            && pair[0].remote_port == pair[1].remote_port
            && pair[0].endpoint_sha256 == pair[1].endpoint_sha256
    })
}

fn adjacent_writable_identity_matches(entries: &[LinuxDeclaredWritable]) -> bool {
    entries.windows(2).any(|pair| {
        pair[0].component == pair[1].component
            && pair[0].target_class == pair[1].target_class
            && pair[0].target_sha256 == pair[1].target_sha256
    })
}

fn normalized_process_inventory(
    inventory: &LinuxSessionInventory,
) -> Result<Vec<LinuxDeclaredProcess>, LinuxSessionBoundaryError> {
    let components = inventory
        .processes
        .iter()
        .map(|process| (process.pid, process.component))
        .collect::<BTreeMap<_, _>>();
    let mut entries = Vec::new();
    for process in &inventory.processes {
        if process.component == NetworkComponent::Undeclared || process.executable_sha256 == [0; 32]
        {
            return Err(boundary_error(
                LinuxSessionBoundaryErrorKind::ProcessInventoryMismatch,
            ));
        }
        entries.push(LinuxDeclaredProcess::new(
            process.component,
            components.get(&process.parent_pid).copied(),
            process.uid,
            process.executable_sha256,
            1,
        ));
    }
    entries.sort_unstable();
    Ok(aggregate_processes(entries))
}

fn normalized_socket_inventory(
    inventory: &LinuxSessionInventory,
) -> Result<Vec<LinuxDeclaredSocket>, LinuxSessionBoundaryError> {
    let components = inventory
        .processes
        .iter()
        .map(|process| (process.pid, process.component))
        .collect::<BTreeMap<_, _>>();
    let mut observed_objects = BTreeSet::new();
    let mut entries = Vec::new();
    for socket in &inventory.sockets {
        if !observed_objects.insert((socket.pid, socket.inode)) {
            continue;
        }
        let component = components.get(&socket.pid).copied().ok_or_else(|| {
            boundary_error(LinuxSessionBoundaryErrorKind::SocketInventoryMismatch)
        })?;
        entries.push(LinuxDeclaredSocket::new(
            component,
            socket.protocol,
            socket.state,
            socket.local_destination,
            socket.remote_destination,
            socket.local_port,
            socket.remote_port,
            socket.endpoint_sha256,
            1,
        ));
    }
    entries.sort_unstable();
    Ok(aggregate_sockets(entries))
}

fn normalized_writable_inventory(
    inventory: &LinuxSessionInventory,
) -> Result<Vec<LinuxDeclaredWritable>, LinuxSessionBoundaryError> {
    let components = inventory
        .processes
        .iter()
        .map(|process| (process.pid, process.component))
        .collect::<BTreeMap<_, _>>();
    let mut entries = Vec::new();
    for writable in &inventory.writable_descriptors {
        let component = components.get(&writable.pid).copied().ok_or_else(|| {
            boundary_error(LinuxSessionBoundaryErrorKind::WritableInventoryMismatch)
        })?;
        entries.push(LinuxDeclaredWritable::new(
            component,
            writable.target_class,
            writable.target_sha256,
            1,
        ));
    }
    entries.sort_unstable();
    Ok(aggregate_writables(entries))
}

fn registered_tools(
    registry: &ToolRegistry,
) -> Result<Vec<LinuxDeclaredTool>, LinuxSessionBoundaryError> {
    let mut tools = registry
        .list_tools()
        .into_iter()
        .map(LinuxDeclaredTool::from_definition)
        .collect::<Result<Vec<_>, _>>()?;
    if tools.len() > MAX_DECLARED_TOOLS {
        return Err(boundary_error(
            LinuxSessionBoundaryErrorKind::ResourceLimitExceeded,
        ));
    }
    tools.sort_unstable();
    Ok(tools)
}

fn aggregate_processes(entries: Vec<LinuxDeclaredProcess>) -> Vec<LinuxDeclaredProcess> {
    let mut aggregated: Vec<LinuxDeclaredProcess> = Vec::new();
    for entry in entries {
        if let Some(last) = aggregated.last_mut().filter(|last| {
            last.component == entry.component
                && last.parent_component == entry.parent_component
                && last.uid == entry.uid
                && last.executable_sha256 == entry.executable_sha256
        }) {
            last.count += 1;
        } else {
            aggregated.push(entry);
        }
    }
    aggregated
}

fn aggregate_sockets(entries: Vec<LinuxDeclaredSocket>) -> Vec<LinuxDeclaredSocket> {
    let mut aggregated: Vec<LinuxDeclaredSocket> = Vec::new();
    for entry in entries {
        if let Some(last) = aggregated
            .last_mut()
            .filter(|last| same_socket_identity(last, &entry))
        {
            last.count += 1;
        } else {
            aggregated.push(entry);
        }
    }
    aggregated
}

fn same_socket_identity(left: &LinuxDeclaredSocket, right: &LinuxDeclaredSocket) -> bool {
    left.component == right.component
        && left.protocol == right.protocol
        && left.state == right.state
        && left.local_destination == right.local_destination
        && left.remote_destination == right.remote_destination
        && left.local_port == right.local_port
        && left.remote_port == right.remote_port
        && left.endpoint_sha256 == right.endpoint_sha256
}

fn aggregate_writables(entries: Vec<LinuxDeclaredWritable>) -> Vec<LinuxDeclaredWritable> {
    let mut aggregated: Vec<LinuxDeclaredWritable> = Vec::new();
    for entry in entries {
        if let Some(last) = aggregated.last_mut().filter(|last| {
            last.component == entry.component
                && last.target_class == entry.target_class
                && last.target_sha256 == entry.target_sha256
        }) {
            last.count += 1;
        } else {
            aggregated.push(entry);
        }
    }
    aggregated
}

const fn boundary_error(kind: LinuxSessionBoundaryErrorKind) -> LinuxSessionBoundaryError {
    LinuxSessionBoundaryError { kind }
}

const fn listener_component_allowed(component: NetworkComponent) -> bool {
    matches!(
        component,
        NetworkComponent::NativeBridge
            | NetworkComponent::Kernel
            | NetworkComponent::KernelNativeInferenceAdapter
            | NetworkComponent::KernelDockerInferenceAdapter
    )
}

const fn listener_error(kind: LinuxListenerBoundaryErrorKind) -> LinuxListenerBoundaryError {
    LinuxListenerBoundaryError { kind }
}

/// Stable Linux inventory failure class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxInventoryErrorKind {
    /// At least one explicit process target is required.
    EmptyTargets,
    /// A closed inventory bound was exceeded.
    ResourceLimitExceeded,
    /// The same PID was attributed more than once.
    DuplicateProcess,
    /// A PID could not be represented by the Linux collector.
    InvalidProcess,
    /// Required process state disappeared or could not be read.
    ProcessUnavailable,
    /// The PID referred to a different process before collection completed.
    ProcessIdentityChanged,
    /// A kernel pseudo-file contained an unsupported or malformed record.
    InvalidKernelRecord,
    /// A target did not expose one valid shared unified-cgroup identity.
    UnifiedCgroupUnavailable,
    /// Declared PIDs did not equal the complete cgroup process set.
    SessionProcessSetMismatch,
    /// Cgroup process membership changed while the snapshot was collected.
    SessionProcessSetChanged,
}

/// Content-free Linux inventory failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxInventoryError {
    kind: LinuxInventoryErrorKind,
    pid: Option<u32>,
}

impl LinuxInventoryError {
    /// Returns the stable failure class.
    #[must_use]
    pub const fn kind(self) -> LinuxInventoryErrorKind {
        self.kind
    }

    /// Returns the process attribution when available.
    #[must_use]
    pub const fn pid(self) -> Option<u32> {
        self.pid
    }
}

impl fmt::Display for LinuxInventoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            LinuxInventoryErrorKind::EmptyTargets => "inventory.empty_targets",
            LinuxInventoryErrorKind::ResourceLimitExceeded => "inventory.resource_limit",
            LinuxInventoryErrorKind::DuplicateProcess => "inventory.duplicate_process",
            LinuxInventoryErrorKind::InvalidProcess => "inventory.invalid_process",
            LinuxInventoryErrorKind::ProcessUnavailable => "inventory.process_unavailable",
            LinuxInventoryErrorKind::ProcessIdentityChanged => "inventory.process_identity_changed",
            LinuxInventoryErrorKind::InvalidKernelRecord => "inventory.invalid_kernel_record",
            LinuxInventoryErrorKind::UnifiedCgroupUnavailable => {
                "inventory.unified_cgroup_unavailable"
            }
            LinuxInventoryErrorKind::SessionProcessSetMismatch => {
                "inventory.session_process_set_mismatch"
            }
            LinuxInventoryErrorKind::SessionProcessSetChanged => {
                "inventory.session_process_set_changed"
            }
        })
    }
}

impl std::error::Error for LinuxInventoryError {}

/// Linux session inventory collector.
#[derive(Clone, Copy, Debug, Default)]
pub struct LinuxSessionInventoryCollector;

impl LinuxSessionInventoryCollector {
    /// Captures only the explicitly attributed target PIDs and their owned descriptors.
    pub fn collect(
        targets: &[LinuxInventoryTarget],
    ) -> Result<LinuxSessionInventory, LinuxInventoryError> {
        validate_inventory_targets(targets)?;
        let mut ordered = targets.to_vec();
        ordered.sort_unstable();

        let mut processes = Vec::with_capacity(ordered.len());
        let mut sockets = Vec::new();
        let mut writables = Vec::new();
        for target in ordered {
            let pid = i32::try_from(target.pid)
                .ok()
                .filter(|pid| *pid > 0)
                .ok_or_else(|| error(LinuxInventoryErrorKind::InvalidProcess, Some(target.pid)))?;
            let process_pid = Pid::from_raw(pid)
                .ok_or_else(|| error(LinuxInventoryErrorKind::InvalidProcess, Some(target.pid)))?;
            let start_time_ticks = observed_start_time(pid, target.pid)?;
            let (pidfd, identity_binding) = retain_process_identity(process_pid, target.pid)?;
            let process_root = PathBuf::from(format!("/proc/{pid}"));
            let (parent_pid, uid) = parse_status(
                &read_bounded(&process_root.join("status"), target.pid)?,
                target.pid,
            )?;
            let executable_sha256 = process_executable_sha256(pid).map_err(|_| {
                error(
                    LinuxInventoryErrorKind::ProcessUnavailable,
                    Some(target.pid),
                )
            })?;
            processes.push(LinuxSessionProcessObservation {
                pid: target.pid,
                parent_pid,
                uid,
                component: target.component,
                start_time_ticks,
                identity_binding,
                executable_sha256,
            });

            let descriptors = collect_descriptors(&process_root, target.pid)?;
            let mut process_sockets =
                collect_sockets(&process_root, target.pid, &descriptors.sockets)?;
            sockets.append(&mut process_sockets);
            writables.extend(descriptors.writables);
            if sockets.len() > MAX_INVENTORY_SOCKETS || writables.len() > MAX_INVENTORY_WRITABLES {
                return Err(error(
                    LinuxInventoryErrorKind::ResourceLimitExceeded,
                    Some(target.pid),
                ));
            }
            ensure_process_identity(pid, target.pid, start_time_ticks)?;
            drop(pidfd);
        }
        sockets.sort_by_key(|entry| (entry.pid, entry.descriptor, entry.inode, entry.protocol));
        writables.sort_by_key(|entry| (entry.pid, entry.descriptor));
        Ok(LinuxSessionInventory {
            scope: LinuxInventoryScope::ExplicitTargets,
            processes,
            sockets,
            writable_descriptors: writables,
        })
    }

    /// Captures every process in one unified cgroup and rejects membership drift.
    pub fn collect_complete_cgroup(
        targets: &[LinuxInventoryTarget],
    ) -> Result<LinuxSessionInventory, LinuxInventoryError> {
        validate_inventory_targets(targets)?;
        let expected = targets
            .iter()
            .map(|target| target.pid)
            .collect::<BTreeSet<_>>();
        let first = targets
            .first()
            .ok_or_else(|| error(LinuxInventoryErrorKind::EmptyTargets, None))?;
        let cgroup = process_unified_cgroup(first.pid)?;
        for target in targets.iter().skip(1) {
            if process_unified_cgroup(target.pid)? != cgroup {
                return Err(error(
                    LinuxInventoryErrorKind::SessionProcessSetMismatch,
                    Some(target.pid),
                ));
            }
        }
        let before = unified_cgroup_members(&cgroup, first.pid)?;
        if before != expected {
            return Err(error(
                LinuxInventoryErrorKind::SessionProcessSetMismatch,
                None,
            ));
        }
        let mut inventory = Self::collect(targets)?;
        let after = unified_cgroup_members(&cgroup, first.pid)?;
        if after != before {
            return Err(error(
                LinuxInventoryErrorKind::SessionProcessSetChanged,
                None,
            ));
        }
        inventory.scope = LinuxInventoryScope::CompleteUnifiedCgroup;
        Ok(inventory)
    }
}

fn validate_inventory_targets(targets: &[LinuxInventoryTarget]) -> Result<(), LinuxInventoryError> {
    if targets.is_empty() {
        return Err(error(LinuxInventoryErrorKind::EmptyTargets, None));
    }
    if targets.len() > MAX_INVENTORY_PROCESSES {
        return Err(error(LinuxInventoryErrorKind::ResourceLimitExceeded, None));
    }
    let mut pids = BTreeSet::new();
    for target in targets {
        if i32::try_from(target.pid)
            .ok()
            .filter(|pid| *pid > 0)
            .is_none()
        {
            return Err(error(
                LinuxInventoryErrorKind::InvalidProcess,
                Some(target.pid),
            ));
        }
        if !pids.insert(target.pid) {
            return Err(error(
                LinuxInventoryErrorKind::DuplicateProcess,
                Some(target.pid),
            ));
        }
    }
    Ok(())
}

fn process_unified_cgroup(pid: u32) -> Result<PathBuf, LinuxInventoryError> {
    let content = read_bounded(&PathBuf::from(format!("/proc/{pid}/cgroup")), pid)?;
    parse_unified_cgroup_path(&content, pid)
}

fn parse_unified_cgroup_path(content: &[u8], pid: u32) -> Result<PathBuf, LinuxInventoryError> {
    let content = std::str::from_utf8(content)
        .map_err(|_| error(LinuxInventoryErrorKind::UnifiedCgroupUnavailable, Some(pid)))?;
    let mut unified = content.lines().filter(|line| line.starts_with("0::/"));
    let line = unified
        .next()
        .ok_or_else(|| error(LinuxInventoryErrorKind::UnifiedCgroupUnavailable, Some(pid)))?;
    if unified.next().is_some() {
        return Err(error(
            LinuxInventoryErrorKind::UnifiedCgroupUnavailable,
            Some(pid),
        ));
    }
    let relative = line
        .strip_prefix("0::/")
        .ok_or_else(|| error(LinuxInventoryErrorKind::UnifiedCgroupUnavailable, Some(pid)))?;
    let mut path = PathBuf::from("/sys/fs/cgroup");
    if !relative.is_empty() {
        for component in relative.split('/') {
            if component.is_empty() || matches!(component, "." | "..") {
                return Err(error(
                    LinuxInventoryErrorKind::UnifiedCgroupUnavailable,
                    Some(pid),
                ));
            }
            path.push(component);
        }
    }
    Ok(path)
}

fn unified_cgroup_members(
    cgroup: &Path,
    attributed_pid: u32,
) -> Result<BTreeSet<u32>, LinuxInventoryError> {
    let content = read_bounded(&cgroup.join("cgroup.procs"), attributed_pid).map_err(|_| {
        error(
            LinuxInventoryErrorKind::UnifiedCgroupUnavailable,
            Some(attributed_pid),
        )
    })?;
    parse_unified_cgroup_members(&content, attributed_pid)
}

fn parse_unified_cgroup_members(
    content: &[u8],
    pid: u32,
) -> Result<BTreeSet<u32>, LinuxInventoryError> {
    let content = std::str::from_utf8(content)
        .map_err(|_| error(LinuxInventoryErrorKind::InvalidKernelRecord, Some(pid)))?;
    let mut members = BTreeSet::new();
    for line in content.lines() {
        let member = line
            .parse::<u32>()
            .ok()
            .filter(|member| *member > 0)
            .ok_or_else(|| error(LinuxInventoryErrorKind::InvalidKernelRecord, Some(pid)))?;
        if !members.insert(member) {
            return Err(error(
                LinuxInventoryErrorKind::InvalidKernelRecord,
                Some(pid),
            ));
        }
    }
    if members.is_empty() {
        return Err(error(
            LinuxInventoryErrorKind::InvalidKernelRecord,
            Some(pid),
        ));
    }
    Ok(members)
}

fn observed_start_time(pid: i32, display_pid: u32) -> Result<u64, LinuxInventoryError> {
    process_start_time_ticks(pid).map_err(|_| {
        error(
            LinuxInventoryErrorKind::ProcessUnavailable,
            Some(display_pid),
        )
    })
}

fn ensure_process_identity(
    pid: i32,
    display_pid: u32,
    expected_start_time_ticks: u64,
) -> Result<(), LinuxInventoryError> {
    if observed_start_time(pid, display_pid)? != expected_start_time_ticks {
        return Err(error(
            LinuxInventoryErrorKind::ProcessIdentityChanged,
            Some(display_pid),
        ));
    }
    Ok(())
}

fn retain_process_identity(
    pid: Pid,
    display_pid: u32,
) -> Result<(Option<std::os::fd::OwnedFd>, LinuxProcessIdentityBinding), LinuxInventoryError> {
    match pidfd_open(pid, PidfdFlags::empty()) {
        Ok(pidfd) => Ok((Some(pidfd), LinuxProcessIdentityBinding::PidFdAndStartTime)),
        Err(error) if pidfd_is_unsupported(error) => {
            Ok((None, LinuxProcessIdentityBinding::StartTimeOnly))
        }
        Err(_) => Err(error(
            LinuxInventoryErrorKind::ProcessUnavailable,
            Some(display_pid),
        )),
    }
}

const fn pidfd_is_unsupported(error: Errno) -> bool {
    matches!(error, Errno::NOSYS | Errno::INVAL | Errno::OPNOTSUPP)
}

struct DescriptorCollection {
    sockets: BTreeMap<u64, Vec<u32>>,
    writables: Vec<LinuxWritableObservation>,
}

fn collect_descriptors(
    process_root: &Path,
    pid: u32,
) -> Result<DescriptorCollection, LinuxInventoryError> {
    let directory = fs::read_dir(process_root.join("fd"))
        .map_err(|_| error(LinuxInventoryErrorKind::ProcessUnavailable, Some(pid)))?;
    let mut sockets: BTreeMap<u64, Vec<u32>> = BTreeMap::new();
    let mut writables = Vec::new();
    for entry in directory {
        let entry =
            entry.map_err(|_| error(LinuxInventoryErrorKind::ProcessUnavailable, Some(pid)))?;
        let Some(descriptor) = entry
            .file_name()
            .to_str()
            .and_then(|value| value.parse::<u32>().ok())
        else {
            continue;
        };
        let target = fs::read_link(entry.path())
            .map_err(|_| error(LinuxInventoryErrorKind::ProcessUnavailable, Some(pid)))?;
        if let Some(inode) = socket_inode(target.as_os_str()) {
            sockets.entry(inode).or_default().push(descriptor);
            continue;
        }
        let fdinfo = read_bounded(
            &process_root.join("fdinfo").join(descriptor.to_string()),
            pid,
        )?;
        if descriptor_is_writable(&fdinfo)? {
            let bytes = target.as_os_str().as_bytes();
            writables.push(LinuxWritableObservation {
                pid,
                descriptor,
                target_class: writable_target_class(bytes),
                target_sha256: Sha256::digest(bytes).into(),
            });
        }
    }
    for descriptors in sockets.values_mut() {
        descriptors.sort_unstable();
    }
    Ok(DescriptorCollection { sockets, writables })
}

fn collect_sockets(
    process_root: &Path,
    pid: u32,
    owned: &BTreeMap<u64, Vec<u32>>,
) -> Result<Vec<LinuxSocketObservation>, LinuxInventoryError> {
    let mut observations = Vec::new();
    let mut matched = BTreeSet::new();
    for (name, protocol, ipv6) in [
        ("tcp", LinuxSocketProtocol::Tcp4, false),
        ("tcp6", LinuxSocketProtocol::Tcp6, true),
        ("udp", LinuxSocketProtocol::Udp4, false),
        ("udp6", LinuxSocketProtocol::Udp6, true),
    ] {
        let table = read_bounded(&process_root.join("net").join(name), pid)?;
        for parsed in parse_internet_table(&table, protocol, ipv6, pid)? {
            if let Some(descriptors) = owned.get(&parsed.inode) {
                matched.insert(parsed.inode);
                for descriptor in descriptors {
                    observations.push(parsed.with_owner(pid, *descriptor));
                }
            }
        }
    }
    let unix_table = read_bounded(&process_root.join("net/unix"), pid)?;
    for parsed in parse_unix_table(&unix_table, pid)? {
        if let Some(descriptors) = owned.get(&parsed.inode) {
            matched.insert(parsed.inode);
            for descriptor in descriptors {
                observations.push(parsed.with_owner(pid, *descriptor));
            }
        }
    }
    for (inode, descriptors) in owned {
        if matched.contains(inode) {
            continue;
        }
        for descriptor in descriptors {
            observations.push(LinuxSocketObservation {
                pid,
                descriptor: *descriptor,
                inode: *inode,
                protocol: LinuxSocketProtocol::Other,
                state: LinuxSocketState::Other,
                local_destination: NetworkDestinationClass::Unknown,
                remote_destination: NetworkDestinationClass::Unknown,
                local_port: None,
                remote_port: None,
                endpoint_sha256: None,
            });
        }
    }
    Ok(observations)
}

impl LinuxSocketObservation {
    fn with_owner(&self, pid: u32, descriptor: u32) -> Self {
        let mut attributed = self.clone();
        attributed.pid = pid;
        attributed.descriptor = descriptor;
        attributed
    }
}

fn parse_status(content: &[u8], pid: u32) -> Result<(u32, u32), LinuxInventoryError> {
    let text = std::str::from_utf8(content)
        .map_err(|_| error(LinuxInventoryErrorKind::InvalidKernelRecord, Some(pid)))?;
    let mut parent = None;
    let mut uid = None;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("PPid:") {
            parent = value.split_whitespace().next().and_then(|v| v.parse().ok());
        } else if let Some(value) = line.strip_prefix("Uid:") {
            uid = value.split_whitespace().next().and_then(|v| v.parse().ok());
        }
    }
    parent
        .zip(uid)
        .ok_or_else(|| error(LinuxInventoryErrorKind::InvalidKernelRecord, Some(pid)))
}

fn parse_internet_table(
    content: &[u8],
    protocol: LinuxSocketProtocol,
    ipv6: bool,
    pid: u32,
) -> Result<Vec<LinuxSocketObservation>, LinuxInventoryError> {
    let text = std::str::from_utf8(content)
        .map_err(|_| error(LinuxInventoryErrorKind::InvalidKernelRecord, Some(pid)))?;
    let mut observations = Vec::new();
    for line in text.lines().skip(1).filter(|line| !line.trim().is_empty()) {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() < 10 {
            return Err(error(
                LinuxInventoryErrorKind::InvalidKernelRecord,
                Some(pid),
            ));
        }
        let (local_address, local_port) = parse_ip_port(fields[1], ipv6, pid)?;
        let (remote_address, remote_port) = parse_ip_port(fields[2], ipv6, pid)?;
        let inode = fields[9]
            .parse()
            .map_err(|_| error(LinuxInventoryErrorKind::InvalidKernelRecord, Some(pid)))?;
        let state = match fields[3] {
            "0A" => LinuxSocketState::Listening,
            "01" => LinuxSocketState::Connected,
            "07" => LinuxSocketState::Bound,
            _ => LinuxSocketState::Other,
        };
        observations.push(LinuxSocketObservation {
            pid: 0,
            descriptor: 0,
            inode,
            protocol,
            state,
            local_destination: classify_ip_destination(local_address),
            remote_destination: classify_ip_destination(remote_address),
            local_port: Some(local_port),
            remote_port: Some(remote_port),
            endpoint_sha256: None,
        });
    }
    Ok(observations)
}

fn parse_ip_port(value: &str, ipv6: bool, pid: u32) -> Result<(IpAddr, u16), LinuxInventoryError> {
    let (address, port) = value
        .split_once(':')
        .ok_or_else(|| error(LinuxInventoryErrorKind::InvalidKernelRecord, Some(pid)))?;
    let port = u16::from_str_radix(port, 16)
        .map_err(|_| error(LinuxInventoryErrorKind::InvalidKernelRecord, Some(pid)))?;
    if ipv6 {
        if address.len() != 32 {
            return Err(error(
                LinuxInventoryErrorKind::InvalidKernelRecord,
                Some(pid),
            ));
        }
        let mut bytes = [0_u8; 16];
        for (index, chunk) in address.as_bytes().chunks_exact(8).enumerate() {
            let text = std::str::from_utf8(chunk)
                .map_err(|_| error(LinuxInventoryErrorKind::InvalidKernelRecord, Some(pid)))?;
            let word = u32::from_str_radix(text, 16)
                .map_err(|_| error(LinuxInventoryErrorKind::InvalidKernelRecord, Some(pid)))?;
            bytes[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        Ok((IpAddr::V6(Ipv6Addr::from(bytes)), port))
    } else {
        let word = u32::from_str_radix(address, 16)
            .map_err(|_| error(LinuxInventoryErrorKind::InvalidKernelRecord, Some(pid)))?;
        Ok((IpAddr::V4(Ipv4Addr::from(word.to_le_bytes())), port))
    }
}

fn parse_unix_table(
    content: &[u8],
    pid: u32,
) -> Result<Vec<LinuxSocketObservation>, LinuxInventoryError> {
    let text = std::str::from_utf8(content)
        .map_err(|_| error(LinuxInventoryErrorKind::InvalidKernelRecord, Some(pid)))?;
    let mut observations = Vec::new();
    for line in text.lines().skip(1).filter(|line| !line.trim().is_empty()) {
        let (fields, endpoint) = split_unix_record(line);
        if fields.len() != 7 {
            return Err(error(
                LinuxInventoryErrorKind::InvalidKernelRecord,
                Some(pid),
            ));
        }
        let protocol = match fields[4] {
            "0001" => LinuxSocketProtocol::UnixStream,
            "0002" => LinuxSocketProtocol::UnixDatagram,
            "0005" => LinuxSocketProtocol::UnixSequencedPacket,
            _ => LinuxSocketProtocol::UnixOther,
        };
        let state = if fields[3] == "00010000" {
            LinuxSocketState::Listening
        } else if fields[5] == "03" {
            LinuxSocketState::Connected
        } else {
            LinuxSocketState::Bound
        };
        let inode = fields[6]
            .parse()
            .map_err(|_| error(LinuxInventoryErrorKind::InvalidKernelRecord, Some(pid)))?;
        observations.push(LinuxSocketObservation {
            pid: 0,
            descriptor: 0,
            inode,
            protocol,
            state,
            local_destination: NetworkDestinationClass::LocalSocket,
            remote_destination: NetworkDestinationClass::LocalSocket,
            local_port: None,
            remote_port: None,
            endpoint_sha256: endpoint.map(|path| Sha256::digest(path.as_bytes()).into()),
        });
    }
    Ok(observations)
}

fn split_unix_record(line: &str) -> (Vec<&str>, Option<&str>) {
    let bytes = line.as_bytes();
    let mut fields = Vec::with_capacity(7);
    let mut cursor = 0;
    while fields.len() < 7 {
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        let start = cursor;
        while cursor < bytes.len() && !bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if start == cursor {
            break;
        }
        fields.push(&line[start..cursor]);
    }
    while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
        cursor += 1;
    }
    let endpoint = (cursor < bytes.len()).then_some(&line[cursor..]);
    (fields, endpoint)
}

fn descriptor_is_writable(content: &[u8]) -> Result<bool, LinuxInventoryError> {
    let text = std::str::from_utf8(content)
        .map_err(|_| error(LinuxInventoryErrorKind::InvalidKernelRecord, None))?;
    let flags = text
        .lines()
        .find_map(|line| line.strip_prefix("flags:"))
        .and_then(|value| u64::from_str_radix(value.trim(), 8).ok())
        .ok_or_else(|| error(LinuxInventoryErrorKind::InvalidKernelRecord, None))?;
    Ok(matches!(flags & 0b11, 1 | 2))
}

fn socket_inode(target: &OsStr) -> Option<u64> {
    let text = target.to_str()?;
    text.strip_prefix("socket:[")
        .and_then(|value| value.strip_suffix(']'))
        .and_then(|value| value.parse().ok())
}

fn writable_target_class(target: &[u8]) -> LinuxWritableTargetClass {
    if target.starts_with(b"/memfd:") || target.starts_with(b"anon_inode:") {
        LinuxWritableTargetClass::AnonymousMemory
    } else if target.ends_with(b" (deleted)") {
        LinuxWritableTargetClass::DeletedPath
    } else if target.starts_with(b"/") {
        LinuxWritableTargetClass::AbsolutePath
    } else {
        LinuxWritableTargetClass::Other
    }
}

fn read_bounded(path: &Path, pid: u32) -> Result<Vec<u8>, LinuxInventoryError> {
    let metadata = fs::metadata(path)
        .map_err(|_| error(LinuxInventoryErrorKind::ProcessUnavailable, Some(pid)))?;
    if metadata.len() > MAX_PROC_RECORD_BYTES {
        return Err(error(
            LinuxInventoryErrorKind::ResourceLimitExceeded,
            Some(pid),
        ));
    }
    let content = fs::read(path)
        .map_err(|_| error(LinuxInventoryErrorKind::ProcessUnavailable, Some(pid)))?;
    if content.len() as u64 > MAX_PROC_RECORD_BYTES {
        return Err(error(
            LinuxInventoryErrorKind::ResourceLimitExceeded,
            Some(pid),
        ));
    }
    Ok(content)
}

const fn error(kind: LinuxInventoryErrorKind, pid: Option<u32>) -> LinuxInventoryError {
    LinuxInventoryError { kind, pid }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use std::os::fd::AsRawFd;
    use std::path::PathBuf;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;
    use std::time::Duration;

    use agentmage_kernel_contracts::{
        CONTRACT_SCHEMA_VERSION, GrantOperation, LocalEndpointIdentity, LocalTransport,
        NetworkComponent, NetworkDestinationClass, OperationBinding, RequiredGrantTemplate,
        SchemaId, SchemaReference, ToolDefinition, ToolId, ToolRiskLevel,
    };
    use agentmage_kernel_engine::{
        strict_local::StrictLocalNetworkPolicy,
        tooling::{Tool, ToolRegistry},
    };
    use rustix::process::getuid;
    use sha2::{Digest, Sha256};

    use super::{
        LinuxDeclaredListener, LinuxDeclaredNetworkRule, LinuxDeclaredProcess, LinuxDeclaredSocket,
        LinuxDeclaredTool, LinuxDeclaredWritable, LinuxInventoryErrorKind, LinuxInventoryScope,
        LinuxInventoryTarget, LinuxListenerBoundaryErrorKind, LinuxProcessIdentityBinding,
        LinuxSessionBoundaryErrorKind, LinuxSessionBoundaryManifest, LinuxSessionInventory,
        LinuxSessionInventoryCollector, LinuxSessionListenerPolicy, LinuxSessionProcessObservation,
        LinuxSocketObservation, LinuxSocketProtocol, LinuxSocketState, LinuxWritableObservation,
        LinuxWritableTargetClass, descriptor_is_writable, parse_internet_table, parse_ip_port,
        parse_unified_cgroup_members, parse_unified_cgroup_path, parse_unix_table,
        pidfd_is_unsupported,
    };

    static TEMP_ID: AtomicU64 = AtomicU64::new(1);

    fn synthetic_process(pid: u32, component: NetworkComponent) -> LinuxSessionProcessObservation {
        LinuxSessionProcessObservation {
            pid,
            parent_pid: 1,
            uid: 1000,
            component,
            start_time_ticks: 1,
            identity_binding: LinuxProcessIdentityBinding::PidFdAndStartTime,
            executable_sha256: [1; 32],
        }
    }

    fn synthetic_listener(
        pid: u32,
        descriptor: u32,
        inode: u64,
        protocol: LinuxSocketProtocol,
        destination: NetworkDestinationClass,
        port: Option<u16>,
        endpoint_sha256: Option<[u8; 32]>,
    ) -> LinuxSocketObservation {
        LinuxSocketObservation {
            pid,
            descriptor,
            inode,
            protocol,
            state: LinuxSocketState::Listening,
            local_destination: destination,
            remote_destination: NetworkDestinationClass::Unspecified,
            local_port: port,
            remote_port: port.map(|_| 0),
            endpoint_sha256,
        }
    }

    fn synthetic_inventory(
        processes: Vec<LinuxSessionProcessObservation>,
        sockets: Vec<LinuxSocketObservation>,
    ) -> LinuxSessionInventory {
        LinuxSessionInventory {
            scope: LinuxInventoryScope::ExplicitTargets,
            processes,
            sockets,
            writable_descriptors: Vec::new(),
        }
    }

    struct FixtureTool(ToolDefinition);

    impl Tool for FixtureTool {
        fn definition(&self) -> &ToolDefinition {
            &self.0
        }
    }

    fn schema(identity: &str) -> SchemaReference {
        SchemaReference {
            schema_id: SchemaId::from_raw(identity),
            schema_version: 1,
            schema_sha256: "a".repeat(64),
        }
    }

    fn tool_definition(identity: &str) -> ToolDefinition {
        ToolDefinition {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_id: ToolId::from_raw(identity),
            tool_version: "1.0.0".to_owned(),
            display_name: "Synthetic fixture reader".to_owned(),
            description: "Reads one synthetic fixture".to_owned(),
            input_schema: schema("fixture.input"),
            output_schema: schema("fixture.output"),
            risk_level: ToolRiskLevel::Low,
            declared_effects: vec![OperationBinding::new(GrantOperation::WorkspaceRead)],
            required_grant: RequiredGrantTemplate {
                operation: OperationBinding::new(GrantOperation::WorkspaceRead),
                target_scope: "workspace-file".to_owned(),
                single_use: true,
            },
            timeout_ms: 1_000,
        }
    }

    fn tool_registry(identity: &str) -> ToolRegistry {
        let mut registry = ToolRegistry::new();
        registry
            .register_tool(Box::new(FixtureTool(tool_definition(identity))))
            .expect("fixture tool registers");
        registry
    }

    fn session_policy(identity: u8) -> StrictLocalNetworkPolicy {
        StrictLocalNetworkPolicy::new(
            LocalEndpointIdentity::new(
                NetworkComponent::KernelDockerInferenceAdapter,
                LocalTransport::GuardedLoopbackTcp,
                [identity; 32],
            )
            .expect("network endpoint is valid"),
        )
    }

    fn exact_boundary_fixture() -> (
        LinuxSessionInventory,
        ToolRegistry,
        StrictLocalNetworkPolicy,
        LinuxSessionBoundaryManifest,
    ) {
        let mut parent = synthetic_process(10, NetworkComponent::KernelDockerInferenceAdapter);
        parent.executable_sha256 = [10; 32];
        let mut child = synthetic_process(11, NetworkComponent::ToolWorker);
        child.parent_pid = 10;
        child.executable_sha256 = [11; 32];
        let listener = synthetic_listener(
            10,
            3,
            100,
            LinuxSocketProtocol::Tcp4,
            NetworkDestinationClass::Loopback,
            Some(12434),
            None,
        );
        let mut duplicate_descriptor = listener.clone();
        duplicate_descriptor.descriptor = 4;
        let inventory = LinuxSessionInventory {
            scope: LinuxInventoryScope::CompleteUnifiedCgroup,
            processes: vec![parent, child],
            sockets: vec![listener, duplicate_descriptor],
            writable_descriptors: vec![
                LinuxWritableObservation {
                    pid: 10,
                    descriptor: 5,
                    target_class: LinuxWritableTargetClass::AbsolutePath,
                    target_sha256: [20; 32],
                },
                LinuxWritableObservation {
                    pid: 11,
                    descriptor: 6,
                    target_class: LinuxWritableTargetClass::AnonymousMemory,
                    target_sha256: [21; 32],
                },
            ],
        };
        let registry = tool_registry("fixture.read");
        let policy = session_policy(30);
        let listener = LinuxDeclaredListener::loopback_tcp(
            NetworkComponent::KernelDockerInferenceAdapter,
            LinuxSocketProtocol::Tcp4,
            12434,
        )
        .expect("listener declaration");
        let manifest = LinuxSessionBoundaryManifest::new(
            vec![
                LinuxDeclaredProcess::new(
                    NetworkComponent::KernelDockerInferenceAdapter,
                    None,
                    1000,
                    [10; 32],
                    1,
                ),
                LinuxDeclaredProcess::new(
                    NetworkComponent::ToolWorker,
                    Some(NetworkComponent::KernelDockerInferenceAdapter),
                    1000,
                    [11; 32],
                    1,
                ),
            ],
            vec![LinuxDeclaredSocket::new(
                NetworkComponent::KernelDockerInferenceAdapter,
                LinuxSocketProtocol::Tcp4,
                LinuxSocketState::Listening,
                NetworkDestinationClass::Loopback,
                NetworkDestinationClass::Unspecified,
                Some(12434),
                Some(0),
                None,
                1,
            )],
            vec![
                LinuxDeclaredWritable::new(
                    NetworkComponent::KernelDockerInferenceAdapter,
                    LinuxWritableTargetClass::AbsolutePath,
                    [20; 32],
                    1,
                ),
                LinuxDeclaredWritable::new(
                    NetworkComponent::ToolWorker,
                    LinuxWritableTargetClass::AnonymousMemory,
                    [21; 32],
                    1,
                ),
            ],
            vec![
                LinuxDeclaredTool::from_definition(&tool_definition("fixture.read"))
                    .expect("tool declaration"),
            ],
            LinuxSessionListenerPolicy::new(&[listener]).expect("listener policy"),
            LinuxDeclaredNetworkRule::from_policy(&policy),
        )
        .expect("session manifest");
        (inventory, registry, policy, manifest)
    }

    #[test]
    fn strict_local_session_boundary_reconciles_every_declared_inventory_family() {
        let (inventory, registry, policy, manifest) = exact_boundary_fixture();
        let report = manifest
            .reconcile(&inventory, &registry, &policy)
            .expect("exact session topology reconciles");
        assert_eq!(report.inventory().processes().len(), 2);
        assert_eq!(report.inventory().sockets().len(), 2);
        assert_eq!(report.inventory().writable_descriptors().len(), 2);
        assert_eq!(report.tools().len(), 1);
        assert_eq!(report.tools()[0].tool_id().as_str(), "fixture.read");
        assert_eq!(report.tools()[0].tool_version(), "1.0.0");
        assert_ne!(report.tools()[0].definition_sha256(), &[0; 32]);
        assert_eq!(
            report.network_rule().endpoint().client(),
            NetworkComponent::KernelDockerInferenceAdapter
        );
        assert_eq!(report.listener_receipt().declared_listeners(), 1);
        assert_eq!(report.listener_receipt().observed_listeners(), 1);
        assert!(!format!("{report:?}").contains("workspace-file"));
    }

    #[test]
    fn strict_local_session_boundary_rejects_each_observed_inventory_substitution() {
        let (inventory, registry, policy, manifest) = exact_boundary_fixture();

        let mut changed = inventory.clone();
        changed.processes[0].executable_sha256 = [99; 32];
        assert_eq!(
            manifest
                .reconcile(&changed, &registry, &policy)
                .expect_err("executable substitution rejects")
                .kind(),
            LinuxSessionBoundaryErrorKind::ProcessInventoryMismatch
        );

        let mut changed = inventory.clone();
        changed.processes[1].parent_pid = 1;
        assert_eq!(
            manifest
                .reconcile(&changed, &registry, &policy)
                .expect_err("parent topology substitution rejects")
                .kind(),
            LinuxSessionBoundaryErrorKind::ProcessInventoryMismatch
        );

        let mut changed = inventory.clone();
        changed.sockets[0].local_port = Some(12435);
        changed.sockets[1].local_port = Some(12435);
        assert_eq!(
            manifest
                .reconcile(&changed, &registry, &policy)
                .expect_err("port substitution rejects")
                .kind(),
            LinuxSessionBoundaryErrorKind::SocketInventoryMismatch
        );

        let mut changed = inventory.clone();
        changed.writable_descriptors[0].target_sha256 = [98; 32];
        assert_eq!(
            manifest
                .reconcile(&changed, &registry, &policy)
                .expect_err("writable target substitution rejects")
                .kind(),
            LinuxSessionBoundaryErrorKind::WritableInventoryMismatch
        );

        assert_eq!(
            manifest
                .reconcile(&inventory, &tool_registry("fixture.changed"), &policy)
                .expect_err("tool substitution rejects")
                .kind(),
            LinuxSessionBoundaryErrorKind::ToolInventoryMismatch
        );
        assert_eq!(
            manifest
                .reconcile(&inventory, &registry, &session_policy(31))
                .expect_err("network rule substitution rejects")
                .kind(),
            LinuxSessionBoundaryErrorKind::NetworkRuleMismatch
        );
    }

    #[test]
    fn strict_local_session_boundary_rejects_extra_and_unattributed_observations() {
        let (inventory, registry, policy, manifest) = exact_boundary_fixture();

        let mut changed = inventory.clone();
        changed
            .processes
            .push(synthetic_process(12, NetworkComponent::Converter));
        assert_eq!(
            manifest
                .reconcile(&changed, &registry, &policy)
                .expect_err("extra process rejects")
                .kind(),
            LinuxSessionBoundaryErrorKind::ProcessInventoryMismatch
        );

        let mut changed = inventory.clone();
        let mut extra_socket = changed.sockets[0].clone();
        extra_socket.descriptor = 7;
        extra_socket.inode = 101;
        changed.sockets.push(extra_socket);
        assert_eq!(
            manifest
                .reconcile(&changed, &registry, &policy)
                .expect_err("extra socket rejects")
                .kind(),
            LinuxSessionBoundaryErrorKind::SocketInventoryMismatch
        );

        let mut changed = inventory.clone();
        changed.writable_descriptors.push(LinuxWritableObservation {
            pid: 11,
            descriptor: 8,
            target_class: LinuxWritableTargetClass::Other,
            target_sha256: [22; 32],
        });
        assert_eq!(
            manifest
                .reconcile(&changed, &registry, &policy)
                .expect_err("extra writable target rejects")
                .kind(),
            LinuxSessionBoundaryErrorKind::WritableInventoryMismatch
        );

        let mut changed = inventory;
        changed.sockets[0].pid = 99;
        changed.sockets[1].pid = 99;
        assert_eq!(
            manifest
                .reconcile(&changed, &registry, &policy)
                .expect_err("unattributed socket rejects")
                .kind(),
            LinuxSessionBoundaryErrorKind::SocketInventoryMismatch
        );
    }

    #[test]
    fn strict_local_session_boundary_manifest_is_bounded_unique_and_closed() {
        let (inventory, registry, policy, mut manifest) = exact_boundary_fixture();
        manifest.listener_policy =
            LinuxSessionListenerPolicy::new(&[]).expect("empty listener policy");
        assert_eq!(
            manifest
                .reconcile(&inventory, &registry, &policy)
                .expect_err("listener omission rejects")
                .kind(),
            LinuxSessionBoundaryErrorKind::ListenerBoundary
        );

        let process = LinuxDeclaredProcess::new(NetworkComponent::Kernel, None, 1000, [1; 32], 1);
        assert_eq!(
            LinuxSessionBoundaryManifest::new(
                vec![process.clone(), process],
                Vec::new(),
                Vec::new(),
                Vec::new(),
                LinuxSessionListenerPolicy::new(&[]).expect("empty listener policy"),
                LinuxDeclaredNetworkRule::from_policy(&policy),
            )
            .expect_err("duplicate declaration rejects")
            .kind(),
            LinuxSessionBoundaryErrorKind::InvalidManifest
        );
        assert_eq!(
            LinuxSessionBoundaryManifest::new(
                vec![LinuxDeclaredProcess::new(
                    NetworkComponent::Undeclared,
                    None,
                    1000,
                    [1; 32],
                    1,
                )],
                Vec::new(),
                Vec::new(),
                Vec::new(),
                LinuxSessionListenerPolicy::new(&[]).expect("empty listener policy"),
                LinuxDeclaredNetworkRule::from_policy(&policy),
            )
            .expect_err("undeclared component rejects")
            .kind(),
            LinuxSessionBoundaryErrorKind::InvalidManifest
        );

        let (mut inventory, registry, policy, manifest) = exact_boundary_fixture();
        inventory.scope = LinuxInventoryScope::ExplicitTargets;
        assert_eq!(
            manifest
                .reconcile(&inventory, &registry, &policy)
                .expect_err("caller-selected process subset rejects")
                .kind(),
            LinuxSessionBoundaryErrorKind::ProcessInventoryMismatch
        );
    }

    #[test]
    fn strict_local_session_boundary_unified_cgroup_parsers_are_closed() {
        assert_eq!(
            parse_unified_cgroup_path(b"0::/user.slice/agentmage.scope\n", 10)
                .expect("unified path parses"),
            PathBuf::from("/sys/fs/cgroup/user.slice/agentmage.scope")
        );
        assert_eq!(
            parse_unified_cgroup_path(b"1:net_cls:/\n0::/user.slice/agentmage.scope\n", 10,)
                .expect("hybrid hierarchy unified path parses"),
            PathBuf::from("/sys/fs/cgroup/user.slice/agentmage.scope")
        );
        assert_eq!(
            parse_unified_cgroup_path(b"0::/\n", 10).expect("root cgroup parses"),
            PathBuf::from("/sys/fs/cgroup")
        );
        for malformed in [
            b"".as_slice(),
            b"1:name=/legacy\n".as_slice(),
            b"0::/valid\n0::/second\n".as_slice(),
            b"0::/../escape\n".as_slice(),
            b"0::/./ambiguous\n".as_slice(),
        ] {
            assert_eq!(
                parse_unified_cgroup_path(malformed, 10)
                    .expect_err("malformed cgroup path rejects")
                    .kind(),
                LinuxInventoryErrorKind::UnifiedCgroupUnavailable
            );
        }
        assert_eq!(
            parse_unified_cgroup_members(b"12\n10\n11\n", 10)
                .expect("members parse")
                .into_iter()
                .collect::<Vec<_>>(),
            vec![10, 11, 12]
        );
        for malformed in [
            b"".as_slice(),
            b"0\n".as_slice(),
            b"10\n10\n".as_slice(),
            b"ten\n".as_slice(),
        ] {
            assert_eq!(
                parse_unified_cgroup_members(malformed, 10)
                    .expect_err("malformed cgroup members reject")
                    .kind(),
                LinuxInventoryErrorKind::InvalidKernelRecord
            );
        }
    }

    #[test]
    #[ignore = "requires a supported Linux systemd user session"]
    fn strict_local_session_boundary_live_complete_cgroup_inventory() {
        struct UnitGuard {
            unit: String,
            pid_path: PathBuf,
        }

        impl Drop for UnitGuard {
            fn drop(&mut self) {
                let _ = Command::new("/usr/bin/systemctl")
                    .args(["--user", "stop", &self.unit])
                    .output();
                let _ = Command::new("/usr/bin/systemctl")
                    .args(["--user", "reset-failed", &self.unit])
                    .output();
                let _ = std::fs::remove_file(&self.pid_path);
            }
        }

        let id = TEMP_ID.fetch_add(1, Ordering::SeqCst);
        let unit = format!("agentmage-inventory-{}-{id}.service", std::process::id());
        let pid_path = std::env::temp_dir().join(format!(
            "agentmage-inventory-pid-{}-{id}",
            std::process::id()
        ));
        let guard = UnitGuard {
            unit: unit.clone(),
            pid_path: pid_path.clone(),
        };
        let command = format!(
            "printf '%s\\n' $$ > {}; /usr/bin/sleep 30 & child=$!; printf '%s\\n' \"$child\" >> {}; wait \"$child\"",
            pid_path.display(),
            pid_path.display()
        );
        let status = Command::new("/usr/bin/systemd-run")
            .args([
                "--user",
                "--quiet",
                "--expand-environment=no",
                "--unit",
                &unit,
                "--property=Type=exec",
                "/bin/sh",
                "-c",
                &command,
            ])
            .status()
            .expect("systemd-run launches");
        assert!(status.success());
        let pids = (0..100)
            .find_map(|_| {
                let value = std::fs::read_to_string(&pid_path).ok();
                if value
                    .as_ref()
                    .is_none_or(|value| value.lines().count() != 2)
                {
                    thread::sleep(Duration::from_millis(20));
                    return None;
                }
                value
            })
            .expect("service publishes both PIDs")
            .lines()
            .map(|value| value.parse::<u32>().expect("service PID parses"))
            .collect::<Vec<_>>();
        assert_eq!(
            LinuxSessionInventoryCollector::collect_complete_cgroup(&[LinuxInventoryTarget {
                pid: pids[0],
                component: NetworkComponent::Kernel,
            },])
            .expect_err("partial cgroup declaration rejects")
            .kind(),
            LinuxInventoryErrorKind::SessionProcessSetMismatch
        );
        let inventory = LinuxSessionInventoryCollector::collect_complete_cgroup(&[
            LinuxInventoryTarget {
                pid: pids[0],
                component: NetworkComponent::Kernel,
            },
            LinuxInventoryTarget {
                pid: pids[1],
                component: NetworkComponent::ToolWorker,
            },
        ])
        .expect("complete service cgroup inventories");
        assert_eq!(
            inventory.scope(),
            LinuxInventoryScope::CompleteUnifiedCgroup
        );
        assert_eq!(inventory.processes().len(), 2);
        assert_eq!(
            inventory
                .processes()
                .iter()
                .map(LinuxSessionProcessObservation::pid)
                .collect::<BTreeSet<_>>(),
            pids.into_iter().collect()
        );
        drop(guard);
    }

    #[test]
    fn strict_local_listener_exact_loopback_and_unix_reconcile_once() {
        let declarations = [
            LinuxDeclaredListener::loopback_tcp(
                NetworkComponent::KernelDockerInferenceAdapter,
                LinuxSocketProtocol::Tcp4,
                12434,
            )
            .expect("loopback declaration"),
            LinuxDeclaredListener::unix_stream(NetworkComponent::NativeBridge, [7; 32])
                .expect("Unix declaration"),
        ];
        let policy = LinuxSessionListenerPolicy::new(&declarations).expect("listener policy");
        let inventory = synthetic_inventory(
            vec![
                synthetic_process(10, NetworkComponent::KernelDockerInferenceAdapter),
                synthetic_process(11, NetworkComponent::NativeBridge),
            ],
            vec![
                synthetic_listener(
                    10,
                    3,
                    100,
                    LinuxSocketProtocol::Tcp4,
                    NetworkDestinationClass::Loopback,
                    Some(12434),
                    None,
                ),
                synthetic_listener(
                    10,
                    4,
                    100,
                    LinuxSocketProtocol::Tcp4,
                    NetworkDestinationClass::Loopback,
                    Some(12434),
                    None,
                ),
                synthetic_listener(
                    11,
                    5,
                    101,
                    LinuxSocketProtocol::UnixStream,
                    NetworkDestinationClass::LocalSocket,
                    None,
                    Some([7; 32]),
                ),
            ],
        );
        let receipt = policy.reconcile(&inventory).expect("exact inventory");
        assert_eq!(receipt.declared_listeners(), 2);
        assert_eq!(receipt.observed_listeners(), 2);
        assert!(!format!("{declarations:?}").contains("7, 7"));
    }

    #[test]
    fn strict_local_listener_unsafe_destination_matrix_fails_before_manifest_matching() {
        let policy = LinuxSessionListenerPolicy::new(&[]).expect("empty exact policy");
        for destination in [
            NetworkDestinationClass::AuthenticatedLocalSocket,
            NetworkDestinationClass::Unspecified,
            NetworkDestinationClass::LinkLocal,
            NetworkDestinationClass::PrivateLan,
            NetworkDestinationClass::Multicast,
            NetworkDestinationClass::ContainerNetwork,
            NetworkDestinationClass::Proxy,
            NetworkDestinationClass::Dns,
            NetworkDestinationClass::External,
            NetworkDestinationClass::Unknown,
        ] {
            let inventory = synthetic_inventory(
                vec![synthetic_process(10, NetworkComponent::Kernel)],
                vec![synthetic_listener(
                    10,
                    3,
                    100,
                    LinuxSocketProtocol::Tcp4,
                    destination,
                    Some(12434),
                    None,
                )],
            );
            assert_eq!(
                policy
                    .reconcile(&inventory)
                    .expect_err("unsafe destination rejects")
                    .kind(),
                LinuxListenerBoundaryErrorKind::UnsafeDestination
            );
        }
    }

    #[test]
    fn strict_local_listener_undeclared_missing_duplicate_and_unknown_fail_closed() {
        let declaration = LinuxDeclaredListener::loopback_tcp(
            NetworkComponent::KernelNativeInferenceAdapter,
            LinuxSocketProtocol::Tcp4,
            8080,
        )
        .expect("declaration");
        let policy = LinuxSessionListenerPolicy::new(std::slice::from_ref(&declaration))
            .expect("listener policy");
        let process = synthetic_process(10, NetworkComponent::KernelNativeInferenceAdapter);
        let expected = synthetic_listener(
            10,
            3,
            100,
            LinuxSocketProtocol::Tcp4,
            NetworkDestinationClass::Loopback,
            Some(8080),
            None,
        );

        assert_eq!(
            policy
                .reconcile(&synthetic_inventory(vec![process.clone()], Vec::new()))
                .expect_err("missing listener rejects")
                .kind(),
            LinuxListenerBoundaryErrorKind::MissingDeclaredListener
        );
        let mut undeclared = expected.clone();
        undeclared.local_port = Some(8081);
        assert_eq!(
            policy
                .reconcile(&synthetic_inventory(
                    vec![process.clone()],
                    vec![undeclared]
                ))
                .expect_err("undeclared listener rejects")
                .kind(),
            LinuxListenerBoundaryErrorKind::UndeclaredListener
        );
        let mut duplicate = expected.clone();
        duplicate.inode = 101;
        duplicate.descriptor = 4;
        assert_eq!(
            policy
                .reconcile(&synthetic_inventory(
                    vec![process],
                    vec![expected.clone(), duplicate],
                ))
                .expect_err("duplicate socket objects reject")
                .kind(),
            LinuxListenerBoundaryErrorKind::DuplicateObservedListener
        );
        assert_eq!(
            policy
                .reconcile(&synthetic_inventory(Vec::new(), vec![expected]))
                .expect_err("unknown process rejects")
                .kind(),
            LinuxListenerBoundaryErrorKind::UnknownProcess
        );
    }

    #[test]
    fn strict_local_listener_malformed_duplicate_and_unapproved_declarations_fail_closed() {
        assert_eq!(
            LinuxDeclaredListener::loopback_tcp(
                NetworkComponent::VisualStudioCodeExtension,
                LinuxSocketProtocol::Tcp4,
                12434,
            )
            .expect_err("extension listener rejects")
            .kind(),
            LinuxListenerBoundaryErrorKind::InvalidDeclaration
        );
        assert_eq!(
            LinuxDeclaredListener::loopback_tcp(
                NetworkComponent::Kernel,
                LinuxSocketProtocol::Udp4,
                12434,
            )
            .expect_err("UDP listener rejects")
            .kind(),
            LinuxListenerBoundaryErrorKind::InvalidDeclaration
        );
        assert_eq!(
            LinuxDeclaredListener::unix_stream(NetworkComponent::Kernel, [0; 32])
                .expect_err("zero identity rejects")
                .kind(),
            LinuxListenerBoundaryErrorKind::InvalidDeclaration
        );
        let declaration = LinuxDeclaredListener::unix_stream(NetworkComponent::Kernel, [3; 32])
            .expect("declaration");
        assert_eq!(
            LinuxSessionListenerPolicy::new(&[declaration.clone(), declaration])
                .expect_err("duplicate declaration rejects")
                .kind(),
            LinuxListenerBoundaryErrorKind::DuplicateDeclaration
        );
        let too_many = (0..=super::MAX_DECLARED_LISTENERS)
            .map(|index| {
                LinuxDeclaredListener::loopback_tcp(
                    NetworkComponent::Kernel,
                    LinuxSocketProtocol::Tcp4,
                    u16::try_from(index + 1).expect("bounded port"),
                )
                .expect("bounded declaration")
            })
            .collect::<Vec<_>>();
        assert_eq!(
            LinuxSessionListenerPolicy::new(&too_many)
                .expect_err("declaration limit rejects")
                .kind(),
            LinuxListenerBoundaryErrorKind::ResourceLimitExceeded
        );
    }

    #[test]
    fn strict_local_listener_bound_and_indeterminate_states_fail_closed() {
        let policy = LinuxSessionListenerPolicy::new(&[]).expect("empty exact policy");
        for (state, expected) in [
            (
                LinuxSocketState::Bound,
                LinuxListenerBoundaryErrorKind::UndeclaredBinding,
            ),
            (
                LinuxSocketState::Other,
                LinuxListenerBoundaryErrorKind::IndeterminateSocketState,
            ),
        ] {
            let mut socket = synthetic_listener(
                10,
                3,
                100,
                LinuxSocketProtocol::Udp4,
                NetworkDestinationClass::Loopback,
                Some(12434),
                None,
            );
            socket.state = state;
            let inventory = synthetic_inventory(
                vec![synthetic_process(10, NetworkComponent::Kernel)],
                vec![socket],
            );
            assert_eq!(
                policy
                    .reconcile(&inventory)
                    .expect_err("unproven socket state rejects")
                    .kind(),
                expected
            );
        }
    }

    #[test]
    #[ignore = "requires a quiescent process descriptor table during the live /proc snapshot"]
    fn strict_local_listener_live_loopback_passes_and_wildcard_fails() {
        let loopback =
            std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("loopback listener");
        let port = loopback.local_addr().expect("loopback address").port();
        let target = LinuxInventoryTarget {
            pid: std::process::id(),
            component: NetworkComponent::Kernel,
        };
        let inventory = LinuxSessionInventoryCollector::collect(&[target])
            .expect("loopback inventory collects");
        let declaration = LinuxDeclaredListener::loopback_tcp(
            NetworkComponent::Kernel,
            LinuxSocketProtocol::Tcp4,
            port,
        )
        .expect("loopback declaration");
        LinuxSessionListenerPolicy::new(&[declaration])
            .expect("loopback policy")
            .reconcile(&inventory)
            .expect("exact loopback listener passes");
        drop(loopback);

        let wildcard =
            std::net::TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0)).expect("wildcard listener");
        let inventory = LinuxSessionInventoryCollector::collect(&[target])
            .expect("wildcard inventory collects");
        assert_eq!(
            LinuxSessionListenerPolicy::new(&[])
                .expect("empty exact policy")
                .reconcile(&inventory)
                .expect_err("wildcard listener rejects")
                .kind(),
            LinuxListenerBoundaryErrorKind::UnsafeDestination
        );
        drop(wildcard);
    }

    #[test]
    fn proc_ip_encoding_maps_loopback_unspecified_and_external_addresses() {
        assert_eq!(
            parse_ip_port("0100007F:1F90", false, 1).expect("IPv4 parses"),
            (IpAddr::V4(Ipv4Addr::LOCALHOST), 8080)
        );
        assert_eq!(
            parse_ip_port("00000000000000000000000001000000:01BB", true, 1).expect("IPv6 parses"),
            (IpAddr::V6(Ipv6Addr::LOCALHOST), 443)
        );
    }

    #[test]
    fn synthetic_socket_tables_preserve_inode_port_state_and_only_endpoint_digest() {
        let tcp = b"sl local_address rem_address st tx rx tr when retr uid timeout inode\n 0: 0100007F:1F90 00000000:0000 0A 0:0 00:0 0 1000 0 42\n";
        let parsed = parse_internet_table(tcp, LinuxSocketProtocol::Tcp4, false, 1)
            .expect("TCP table parses");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].inode(), 42);
        assert_eq!(parsed[0].state(), LinuxSocketState::Listening);
        assert_eq!(parsed[0].local_port(), Some(8080));
        assert_eq!(
            parsed[0].local_destination(),
            NetworkDestinationClass::Loopback
        );

        let unix = b"Num RefCount Protocol Flags Type St Inode Path\n0: 3 0 00010000 0001 01 84 /run/user/1000/agent mage.sock\n";
        let parsed = parse_unix_table(unix, 1).expect("Unix table parses");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].inode(), 84);
        assert_eq!(parsed[0].protocol(), LinuxSocketProtocol::UnixStream);
        assert_eq!(parsed[0].state(), LinuxSocketState::Listening);
        let expected: [u8; 32] = Sha256::digest(b"/run/user/1000/agent mage.sock").into();
        assert_eq!(parsed[0].endpoint_sha256(), Some(&expected));
        assert!(!format!("{:?}", parsed[0]).contains("agentmage.sock"));
    }

    #[test]
    fn descriptor_flags_are_parsed_as_octal_and_fail_closed() {
        assert!(!descriptor_is_writable(b"pos:\t0\nflags:\t0100000\n").expect("read flags"));
        assert!(descriptor_is_writable(b"pos:\t0\nflags:\t0100001\n").expect("write flags"));
        assert_eq!(
            descriptor_is_writable(b"pos:\t0\n")
                .expect_err("missing flags rejects")
                .kind(),
            LinuxInventoryErrorKind::InvalidKernelRecord
        );
    }

    #[test]
    #[ignore = "requires a quiescent process descriptor table during the live /proc snapshot"]
    fn live_self_inventory_attributes_executable_and_open_writable_descriptor() {
        let id = TEMP_ID.fetch_add(1, Ordering::SeqCst);
        let path: PathBuf =
            std::env::temp_dir().join(format!("agentmage-inventory-{}-{id}", std::process::id()));
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .expect("test file opens");
        file.write_all(b"synthetic").expect("test file writes");
        let descriptor = u32::try_from(file.as_raw_fd()).expect("descriptor is nonnegative");

        let inventory = LinuxSessionInventoryCollector::collect(&[LinuxInventoryTarget {
            pid: std::process::id(),
            component: NetworkComponent::Kernel,
        }])
        .expect("self inventory collects");
        assert_eq!(inventory.processes().len(), 1);
        assert_eq!(inventory.processes()[0].uid(), getuid().as_raw());
        assert!(inventory.processes()[0].start_time_ticks() > 0);
        assert_ne!(inventory.processes()[0].executable_sha256(), &[0; 32]);
        let writable = inventory
            .writable_descriptors()
            .iter()
            .find(|entry| entry.descriptor() == descriptor)
            .expect("open writable file is inventoried");
        assert_eq!(
            writable.target_class(),
            LinuxWritableTargetClass::AbsolutePath
        );
        assert_ne!(writable.target_sha256(), &[0; 32]);
        drop(file);
        std::fs::remove_file(path).expect("test file removes");
    }

    #[test]
    fn duplicate_empty_and_impossible_target_sets_fail_before_collection() {
        assert_eq!(
            LinuxSessionInventoryCollector::collect(&[])
                .expect_err("empty rejects")
                .kind(),
            LinuxInventoryErrorKind::EmptyTargets
        );
        let target = LinuxInventoryTarget {
            pid: std::process::id(),
            component: NetworkComponent::Kernel,
        };
        assert_eq!(
            LinuxSessionInventoryCollector::collect(&[target, target])
                .expect_err("duplicate rejects")
                .kind(),
            LinuxInventoryErrorKind::DuplicateProcess
        );
        assert_eq!(
            LinuxSessionInventoryCollector::collect(&[LinuxInventoryTarget {
                pid: 0,
                component: NetworkComponent::Kernel,
            }])
            .expect_err("zero PID rejects")
            .kind(),
            LinuxInventoryErrorKind::InvalidProcess
        );
        assert_eq!(
            LinuxSessionInventoryCollector::collect(&[LinuxInventoryTarget {
                pid: 2_000_000_000,
                component: NetworkComponent::Kernel,
            }])
            .expect_err("absent PID rejects")
            .kind(),
            LinuxInventoryErrorKind::ProcessUnavailable
        );
    }

    #[test]
    fn pidfd_fallback_is_limited_to_unsupported_kernel_errors() {
        assert!(pidfd_is_unsupported(rustix::io::Errno::NOSYS));
        assert!(pidfd_is_unsupported(rustix::io::Errno::INVAL));
        assert!(pidfd_is_unsupported(rustix::io::Errno::OPNOTSUPP));
        assert!(!pidfd_is_unsupported(rustix::io::Errno::ACCESS));
    }
}
