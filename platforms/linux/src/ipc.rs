//! Authenticated, mode-restricted Linux local IPC.

use std::fmt;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use rustix::net::sockopt::socket_peercred;
use rustix::rand::{GetRandomFlags, getrandom};
use sha2::{Digest, Sha256};

/// Version of the fixed Linux host/bridge authentication frame.
pub const LINUX_IPC_PROTOCOL_VERSION: u32 = 1;

const HANDSHAKE_FRAME_BYTES: usize = 68;
const LAUNCH_ENVELOPE_SCHEMA_VERSION: u16 = 1;
const LAUNCH_ENVELOPE_MAGIC: &[u8; 4] = b"AGMB";
const MAX_ENDPOINT_BYTES: usize = 107;
#[cfg_attr(any(test, feature = "test-support"), allow(dead_code))]
const MAX_PEER_EXECUTABLE_BYTES: u64 = 64 * 1024 * 1024;
// Development and test binaries retain debug information and can exceed the production ceiling.
const MAX_DEVELOPMENT_PEER_EXECUTABLE_BYTES: u64 = 512 * 1024 * 1024;
#[cfg(any(test, feature = "test-support"))]
const DEFAULT_PEER_EXECUTABLE_BYTES: u64 = MAX_DEVELOPMENT_PEER_EXECUTABLE_BYTES;
#[cfg(not(any(test, feature = "test-support")))]
const DEFAULT_PEER_EXECUTABLE_BYTES: u64 = MAX_PEER_EXECUTABLE_BYTES;

/// Stable content-free Linux IPC authentication failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxIpcErrorKind {
    /// The private socket parent does not have the required owner or mode.
    UnsafeSocketParent,
    /// The socket path already exists or does not have mode `0600`.
    UnsafeSocketMode,
    /// The kernel-reported peer user differs from the launched peer.
    WrongUser,
    /// The kernel-reported peer process differs from the launched peer.
    WrongProcess,
    /// The peer process disappeared or changed start identity during observation.
    ProcessIdentityChanged,
    /// The peer executable digest differs from the launched executable.
    WrongExecutable,
    /// The peer used an unsupported protocol version.
    VersionMismatch,
    /// The peer did not return the current launch challenge.
    ChallengeMismatch,
    /// The challenge response did not authenticate.
    AuthenticationFailed,
    /// The launch authentication was already consumed.
    Replay,
    /// The fixed-size authentication frame was malformed or incomplete.
    MalformedFrame,
    /// A bounded peer executable read exceeded its limit.
    ResourceLimitExceeded,
    /// A required operating-system operation failed without safe detail.
    PlatformFailure,
}

impl LinuxIpcErrorKind {
    /// Returns the stable redacted failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::UnsafeSocketParent => "linux.ipc.unsafe_socket_parent",
            Self::UnsafeSocketMode => "linux.ipc.unsafe_socket_mode",
            Self::WrongUser => "linux.ipc.wrong_user",
            Self::WrongProcess => "linux.ipc.wrong_process",
            Self::ProcessIdentityChanged => "linux.ipc.process_identity_changed",
            Self::WrongExecutable => "linux.ipc.wrong_executable",
            Self::VersionMismatch => "linux.ipc.version_mismatch",
            Self::ChallengeMismatch => "linux.ipc.challenge_mismatch",
            Self::AuthenticationFailed => "linux.ipc.authentication_failed",
            Self::Replay => "linux.ipc.replay",
            Self::MalformedFrame => "linux.ipc.malformed_frame",
            Self::ResourceLimitExceeded => "linux.ipc.resource_limit_exceeded",
            Self::PlatformFailure => "linux.ipc.platform_failure",
        }
    }
}

/// Content-free error returned by Linux IPC setup or authentication.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxIpcError {
    kind: LinuxIpcErrorKind,
}

impl LinuxIpcError {
    const fn new(kind: LinuxIpcErrorKind) -> Self {
        Self { kind }
    }

    /// Returns the stable failure class.
    #[must_use]
    pub const fn kind(&self) -> LinuxIpcErrorKind {
        self.kind
    }
}

impl fmt::Display for LinuxIpcError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.kind.code())
    }
}

impl std::error::Error for LinuxIpcError {}

/// Exact expected or kernel-observed local peer identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxPeerIdentity {
    uid: u32,
    pid: i32,
    start_time_ticks: u64,
    executable_sha256: [u8; 32],
}

impl LinuxPeerIdentity {
    /// Creates an exact peer identity from owner, process, and executable digest.
    #[must_use]
    pub const fn new(
        uid: u32,
        pid: i32,
        start_time_ticks: u64,
        executable_sha256: [u8; 32],
    ) -> Self {
        Self {
            uid,
            pid,
            start_time_ticks,
            executable_sha256,
        }
    }

    /// Returns the expected numeric user identity.
    #[must_use]
    pub const fn uid(&self) -> u32 {
        self.uid
    }

    /// Returns the expected process identity.
    #[must_use]
    pub const fn pid(&self) -> i32 {
        self.pid
    }

    /// Returns the kernel process start time in clock ticks since boot.
    #[must_use]
    pub const fn start_time_ticks(&self) -> u64 {
        self.start_time_ticks
    }

    /// Returns the exact executable digest without revealing a path.
    #[must_use]
    pub const fn executable_sha256(&self) -> &[u8; 32] {
        &self.executable_sha256
    }
}

/// Fixed-size launch-authentication request sent by one local peer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxHandshakeRequest {
    protocol_version: u32,
    challenge: [u8; 32],
    response: [u8; 32],
}

impl LinuxHandshakeRequest {
    fn new(
        protocol_version: u32,
        challenge: [u8; 32],
        launch_secret: &[u8; 32],
        peer: &LinuxPeerIdentity,
    ) -> Self {
        Self {
            protocol_version,
            challenge,
            response: authentication_digest(protocol_version, &challenge, launch_secret, peer),
        }
    }

    /// Encodes the exact bounded wire frame.
    #[must_use]
    pub fn encode(&self) -> [u8; HANDSHAKE_FRAME_BYTES] {
        let mut frame = [0_u8; HANDSHAKE_FRAME_BYTES];
        frame[..4].copy_from_slice(&self.protocol_version.to_be_bytes());
        frame[4..36].copy_from_slice(&self.challenge);
        frame[36..].copy_from_slice(&self.response);
        frame
    }

    #[allow(dead_code)]
    fn decode(frame: &[u8; HANDSHAKE_FRAME_BYTES]) -> Self {
        let mut version = [0_u8; 4];
        version.copy_from_slice(&frame[..4]);
        let mut challenge = [0_u8; 32];
        challenge.copy_from_slice(&frame[4..36]);
        let mut response = [0_u8; 32];
        response.copy_from_slice(&frame[36..]);
        Self {
            protocol_version: u32::from_be_bytes(version),
            challenge,
            response,
        }
    }
}

/// Fresh launch material delivered directly to the declared peer, never by environment.
pub struct LinuxLaunchCredentials {
    challenge: [u8; 32],
    launch_secret: [u8; 32],
}

impl fmt::Debug for LinuxLaunchCredentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxLaunchCredentials")
            .field("challenge", &"redacted")
            .field("launch_secret", &"redacted")
            .finish()
    }
}

impl LinuxLaunchCredentials {
    /// Returns the public challenge for launch correlation.
    #[must_use]
    pub const fn challenge(&self) -> &[u8; 32] {
        &self.challenge
    }

    /// Returns the one-use secret for direct inherited-pipe transfer.
    ///
    /// Callers must not place these bytes in arguments, environment variables,
    /// files, diagnostics, or logs. The credential object erases both fields on
    /// drop; the receiving process must erase its copy after constructing the
    /// authenticated bridge.
    #[must_use]
    pub const fn launch_secret(&self) -> &[u8; 32] {
        &self.launch_secret
    }

    /// Builds the exact handshake for the peer receiving these launch credentials.
    #[must_use]
    pub fn request(&self, peer: &LinuxPeerIdentity) -> LinuxHandshakeRequest {
        LinuxHandshakeRequest::new(
            LINUX_IPC_PROTOCOL_VERSION,
            self.challenge,
            &self.launch_secret,
            peer,
        )
    }
}

impl Drop for LinuxLaunchCredentials {
    fn drop(&mut self) {
        self.challenge.fill(0);
        self.launch_secret.fill(0);
    }
}

/// One-use launch envelope transferred directly from a spawned host to its declared client.
///
/// The contained secret is never exposed as text and is erased when the envelope is consumed or
/// dropped. Reading an envelope does not grant authority; it only permits the exact named process
/// to authenticate to the private host endpoint.
pub struct LinuxLaunchEnvelope {
    endpoint: PathBuf,
    credentials: LinuxLaunchCredentials,
    peer: LinuxPeerIdentity,
}

impl fmt::Debug for LinuxLaunchEnvelope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxLaunchEnvelope")
            .field("endpoint", &"redacted")
            .field("credentials", &self.credentials)
            .field("peer", &self.peer)
            .finish()
    }
}

impl LinuxLaunchEnvelope {
    /// Reads one exact bounded binary launch envelope from an inherited pipe.
    pub fn read(input: &mut impl Read) -> Result<Self, LinuxIpcError> {
        let mut header = [0_u8; 8];
        input
            .read_exact(&mut header)
            .map_err(|_| ipc_error(LinuxIpcErrorKind::MalformedFrame))?;
        if &header[..4] != LAUNCH_ENVELOPE_MAGIC
            || u16::from_be_bytes([header[4], header[5]]) != LAUNCH_ENVELOPE_SCHEMA_VERSION
        {
            return Err(ipc_error(LinuxIpcErrorKind::VersionMismatch));
        }
        let endpoint_length = usize::from(u16::from_be_bytes([header[6], header[7]]));
        if endpoint_length == 0 || endpoint_length > MAX_ENDPOINT_BYTES {
            return Err(ipc_error(LinuxIpcErrorKind::ResourceLimitExceeded));
        }
        let mut endpoint = vec![0_u8; endpoint_length];
        let mut fixed = [0_u8; 112];
        input
            .read_exact(&mut endpoint)
            .and_then(|()| input.read_exact(&mut fixed))
            .map_err(|_| ipc_error(LinuxIpcErrorKind::MalformedFrame))?;
        if endpoint.contains(&0) {
            return Err(ipc_error(LinuxIpcErrorKind::MalformedFrame));
        }
        let endpoint = std::str::from_utf8(&endpoint)
            .ok()
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .ok_or_else(|| ipc_error(LinuxIpcErrorKind::MalformedFrame))?;
        let mut challenge = [0_u8; 32];
        challenge.copy_from_slice(&fixed[..32]);
        let mut launch_secret = [0_u8; 32];
        launch_secret.copy_from_slice(&fixed[32..64]);
        let uid = u32::from_be_bytes(fixed[64..68].try_into().expect("fixed slice"));
        let pid = i32::from_be_bytes(fixed[68..72].try_into().expect("fixed slice"));
        let start_time_ticks = u64::from_be_bytes(fixed[72..80].try_into().expect("fixed slice"));
        let mut executable_sha256 = [0_u8; 32];
        executable_sha256.copy_from_slice(&fixed[80..]);
        if pid <= 1 || start_time_ticks == 0 || challenge == [0; 32] || launch_secret == [0; 32] {
            launch_secret.fill(0);
            return Err(ipc_error(LinuxIpcErrorKind::MalformedFrame));
        }
        Ok(Self {
            endpoint,
            credentials: LinuxLaunchCredentials {
                challenge,
                launch_secret,
            },
            peer: LinuxPeerIdentity::new(uid, pid, start_time_ticks, executable_sha256),
        })
    }

    /// Connects and authenticates only when this process still matches the envelope identity.
    pub fn connect(self) -> Result<LinuxAuthenticatedIpcSession, LinuxIpcError> {
        self.connect_with_limit(DEFAULT_PEER_EXECUTABLE_BYTES)
    }

    /// Connects the explicit development harness while retaining a bounded debug-binary ceiling.
    pub fn connect_development(self) -> Result<LinuxAuthenticatedIpcSession, LinuxIpcError> {
        self.connect_with_limit(MAX_DEVELOPMENT_PEER_EXECUTABLE_BYTES)
    }

    fn connect_with_limit(
        self,
        maximum_executable_bytes: u64,
    ) -> Result<LinuxAuthenticatedIpcSession, LinuxIpcError> {
        let current = stable_peer_identity_with_limit(
            rustix::process::getuid().as_raw(),
            rustix::process::getpid().as_raw_pid(),
            maximum_executable_bytes,
        )?;
        if current != self.peer {
            return Err(ipc_error(LinuxIpcErrorKind::ProcessIdentityChanged));
        }
        verify_client_socket_path(&self.endpoint)?;
        let mut stream = UnixStream::connect(&self.endpoint)
            .map_err(|_| ipc_error(LinuxIpcErrorKind::PlatformFailure))?;
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .map_err(|_| ipc_error(LinuxIpcErrorKind::PlatformFailure))?;
        let request = self.credentials.request(&current).encode();
        stream
            .write_all(&request)
            .and_then(|()| stream.flush())
            .map_err(|_| ipc_error(LinuxIpcErrorKind::PlatformFailure))?;
        stream
            .set_write_timeout(None)
            .map_err(|_| ipc_error(LinuxIpcErrorKind::PlatformFailure))?;
        Ok(LinuxAuthenticatedIpcSession {
            stream,
            peer: LinuxAuthenticatedPeer {
                identity: current,
                protocol_version: LINUX_IPC_PROTOCOL_VERSION,
            },
        })
    }
}

fn verify_client_socket_path(path: &Path) -> Result<(), LinuxIpcError> {
    let parent = path
        .parent()
        .ok_or_else(|| ipc_error(LinuxIpcErrorKind::UnsafeSocketParent))?;
    let owner = rustix::process::getuid().as_raw();
    let parent_metadata = fs::symlink_metadata(parent)
        .map_err(|_| ipc_error(LinuxIpcErrorKind::UnsafeSocketParent))?;
    let socket_metadata =
        fs::symlink_metadata(path).map_err(|_| ipc_error(LinuxIpcErrorKind::UnsafeSocketMode))?;
    if !parent_metadata.is_dir()
        || parent_metadata.uid() != owner
        || parent_metadata.mode() & 0o077 != 0
        || !socket_metadata.file_type().is_socket()
        || socket_metadata.uid() != owner
        || socket_metadata.mode() & 0o777 != 0o600
    {
        return Err(ipc_error(LinuxIpcErrorKind::UnsafeSocketMode));
    }
    Ok(())
}

/// Evidence returned only after one fresh local peer authenticates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxAuthenticatedPeer {
    identity: LinuxPeerIdentity,
    protocol_version: u32,
}

/// Authenticated local byte channel with fixed length-bounded message framing.
///
/// Construction is private to the verified Linux endpoint. The channel carries
/// authenticated bytes but no capability grant or effect authority.
pub struct LinuxAuthenticatedIpcSession {
    stream: UnixStream,
    peer: LinuxAuthenticatedPeer,
}

impl fmt::Debug for LinuxAuthenticatedIpcSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxAuthenticatedIpcSession")
            .field("peer", &self.peer)
            .finish_non_exhaustive()
    }
}

impl LinuxAuthenticatedIpcSession {
    /// Returns the peer admitted by kernel credentials and the one-use handshake.
    #[must_use]
    pub const fn peer(&self) -> &LinuxAuthenticatedPeer {
        &self.peer
    }

    /// Reads one big-endian length-prefixed frame within the caller's closed bound.
    pub fn read_frame(&mut self, maximum_bytes: usize) -> Result<Vec<u8>, LinuxIpcError> {
        if maximum_bytes == 0 || maximum_bytes > 4 * 1024 * 1024 {
            return Err(ipc_error(LinuxIpcErrorKind::ResourceLimitExceeded));
        }
        let mut length = [0_u8; 4];
        self.stream
            .read_exact(&mut length)
            .map_err(|_| ipc_error(LinuxIpcErrorKind::MalformedFrame))?;
        let length = usize::try_from(u32::from_be_bytes(length))
            .map_err(|_| ipc_error(LinuxIpcErrorKind::ResourceLimitExceeded))?;
        if length == 0 || length > maximum_bytes {
            return Err(ipc_error(LinuxIpcErrorKind::ResourceLimitExceeded));
        }
        let mut frame = vec![0_u8; length];
        self.stream
            .read_exact(&mut frame)
            .map_err(|_| ipc_error(LinuxIpcErrorKind::MalformedFrame))?;
        Ok(frame)
    }

    /// Writes one big-endian length-prefixed frame within the caller's closed bound.
    pub fn write_frame(&mut self, frame: &[u8], maximum_bytes: usize) -> Result<(), LinuxIpcError> {
        if frame.is_empty() || frame.len() > maximum_bytes || maximum_bytes > 4 * 1024 * 1024 {
            return Err(ipc_error(LinuxIpcErrorKind::ResourceLimitExceeded));
        }
        let length = u32::try_from(frame.len())
            .map_err(|_| ipc_error(LinuxIpcErrorKind::ResourceLimitExceeded))?;
        self.stream
            .write_all(&length.to_be_bytes())
            .and_then(|()| self.stream.write_all(frame))
            .and_then(|()| self.stream.flush())
            .map_err(|_| ipc_error(LinuxIpcErrorKind::PlatformFailure))
    }
}

/// Verified aggregate-owned Linux host endpoint.
///
/// The underlying listener and socket identities remain private to this type.
pub struct LinuxHostIpcEndpoint {
    listener: PrivateUnixListener,
}

impl fmt::Debug for LinuxHostIpcEndpoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxHostIpcEndpoint")
            .finish_non_exhaustive()
    }
}

impl LinuxHostIpcEndpoint {
    pub(crate) fn bind(path: &Path) -> Result<Self, LinuxIpcError> {
        Ok(Self {
            listener: PrivateUnixListener::bind(path)?,
        })
    }

    /// Returns the private socket path for direct bootstrap transfer.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.listener.path
    }

    /// Accepts exactly one peer and consumes its one-use launch authenticator.
    pub fn accept(
        &self,
        authenticator: &LinuxIpcAuthenticator,
    ) -> Result<LinuxAuthenticatedIpcSession, LinuxIpcError> {
        self.accept_with_limit(authenticator, DEFAULT_PEER_EXECUTABLE_BYTES)
    }

    /// Accepts the explicit development peer under the bounded debug-binary ceiling.
    pub fn accept_development(
        &self,
        authenticator: &LinuxIpcAuthenticator,
    ) -> Result<LinuxAuthenticatedIpcSession, LinuxIpcError> {
        self.accept_with_limit(authenticator, MAX_DEVELOPMENT_PEER_EXECUTABLE_BYTES)
    }

    fn accept_with_limit(
        &self,
        authenticator: &LinuxIpcAuthenticator,
        maximum_executable_bytes: u64,
    ) -> Result<LinuxAuthenticatedIpcSession, LinuxIpcError> {
        let (stream, peer) = self
            .listener
            .accept_authenticated(authenticator, maximum_executable_bytes)?;
        Ok(LinuxAuthenticatedIpcSession { stream, peer })
    }
}

impl LinuxAuthenticatedPeer {
    /// Returns the kernel-observed peer identity.
    #[must_use]
    pub const fn identity(&self) -> &LinuxPeerIdentity {
        &self.identity
    }

    /// Returns the exact admitted protocol version.
    #[must_use]
    pub const fn protocol_version(&self) -> u32 {
        self.protocol_version
    }
}

/// One-launch authenticator whose successful use is consumed atomically.
pub struct LinuxIpcAuthenticator {
    expected_peer: LinuxPeerIdentity,
    challenge: [u8; 32],
    launch_secret: [u8; 32],
    consumed: AtomicBool,
}

impl fmt::Debug for LinuxIpcAuthenticator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxIpcAuthenticator")
            .field("expected_peer", &self.expected_peer)
            .field("consumed", &self.consumed.load(Ordering::SeqCst))
            .finish_non_exhaustive()
    }
}

impl LinuxIpcAuthenticator {
    /// Generates fresh one-launch authentication material from the operating system.
    pub fn generate(
        expected_peer: LinuxPeerIdentity,
    ) -> Result<(Self, LinuxLaunchCredentials), LinuxIpcError> {
        let mut challenge = [0_u8; 32];
        let mut launch_secret = [0_u8; 32];
        let challenge_bytes = getrandom(&mut challenge, GetRandomFlags::empty())
            .map_err(|_| ipc_error(LinuxIpcErrorKind::PlatformFailure))?;
        let secret_bytes = getrandom(&mut launch_secret, GetRandomFlags::empty())
            .map_err(|_| ipc_error(LinuxIpcErrorKind::PlatformFailure))?;
        if challenge_bytes != challenge.len() || secret_bytes != launch_secret.len() {
            return Err(ipc_error(LinuxIpcErrorKind::PlatformFailure));
        }
        let credentials = LinuxLaunchCredentials {
            challenge,
            launch_secret,
        };
        Ok((
            Self::from_launch_material(expected_peer, challenge, launch_secret),
            credentials,
        ))
    }

    const fn from_launch_material(
        expected_peer: LinuxPeerIdentity,
        challenge: [u8; 32],
        launch_secret: [u8; 32],
    ) -> Self {
        Self {
            expected_peer,
            challenge,
            launch_secret,
            consumed: AtomicBool::new(false),
        }
    }

    /// Returns the fresh public launch challenge.
    #[must_use]
    pub const fn challenge(&self) -> &[u8; 32] {
        &self.challenge
    }

    /// Validates and atomically consumes one exact request.
    pub fn authenticate(
        &self,
        observed_peer: LinuxPeerIdentity,
        request: &LinuxHandshakeRequest,
    ) -> Result<LinuxAuthenticatedPeer, LinuxIpcError> {
        if self.consumed.load(Ordering::Acquire) {
            return Err(ipc_error(LinuxIpcErrorKind::Replay));
        }
        if observed_peer.uid != self.expected_peer.uid {
            return Err(ipc_error(LinuxIpcErrorKind::WrongUser));
        }
        if observed_peer.pid != self.expected_peer.pid {
            return Err(ipc_error(LinuxIpcErrorKind::WrongProcess));
        }
        if observed_peer.start_time_ticks != self.expected_peer.start_time_ticks {
            return Err(ipc_error(LinuxIpcErrorKind::ProcessIdentityChanged));
        }
        if observed_peer.executable_sha256 != self.expected_peer.executable_sha256 {
            return Err(ipc_error(LinuxIpcErrorKind::WrongExecutable));
        }
        if request.protocol_version != LINUX_IPC_PROTOCOL_VERSION {
            return Err(ipc_error(LinuxIpcErrorKind::VersionMismatch));
        }
        if request.challenge != self.challenge {
            return Err(ipc_error(LinuxIpcErrorKind::ChallengeMismatch));
        }
        let expected_response = authentication_digest(
            request.protocol_version,
            &request.challenge,
            &self.launch_secret,
            &observed_peer,
        );
        if !constant_time_equal(&request.response, &expected_response) {
            return Err(ipc_error(LinuxIpcErrorKind::AuthenticationFailed));
        }
        self.consumed
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| ipc_error(LinuxIpcErrorKind::Replay))?;
        Ok(LinuxAuthenticatedPeer {
            identity: observed_peer,
            protocol_version: request.protocol_version,
        })
    }
}

impl Drop for LinuxIpcAuthenticator {
    fn drop(&mut self) {
        self.challenge.fill(0);
        self.launch_secret.fill(0);
    }
}

/// Mode-restricted Unix listener rooted in an already-private runtime directory.
#[allow(dead_code)]
struct PrivateUnixListener {
    listener: UnixListener,
    path: PathBuf,
    parent_identity: SocketPathIdentity,
    socket_identity: SocketPathIdentity,
    cleaned: bool,
}

impl fmt::Debug for PrivateUnixListener {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PrivateUnixListener")
            .finish_non_exhaustive()
    }
}

#[allow(dead_code)]
impl PrivateUnixListener {
    /// Binds a new `0600` socket after validating its private owner-only parent.
    fn bind(path: &Path) -> Result<Self, LinuxIpcError> {
        let parent = path
            .parent()
            .ok_or_else(|| ipc_error(LinuxIpcErrorKind::UnsafeSocketParent))?;
        let parent_metadata = fs::symlink_metadata(parent)
            .map_err(|_| ipc_error(LinuxIpcErrorKind::UnsafeSocketParent))?;
        if !parent_metadata.file_type().is_dir()
            || parent_metadata.uid() != rustix::process::getuid().as_raw()
            || parent_metadata.mode() & 0o077 != 0
        {
            return Err(ipc_error(LinuxIpcErrorKind::UnsafeSocketParent));
        }
        if fs::symlink_metadata(path).is_ok() {
            return Err(ipc_error(LinuxIpcErrorKind::UnsafeSocketMode));
        }
        let listener =
            UnixListener::bind(path).map_err(|_| ipc_error(LinuxIpcErrorKind::PlatformFailure))?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(|_| ipc_error(LinuxIpcErrorKind::PlatformFailure))?;
        let socket_metadata = fs::symlink_metadata(path)
            .map_err(|_| ipc_error(LinuxIpcErrorKind::PlatformFailure))?;
        if !socket_metadata.file_type().is_socket()
            || socket_metadata.uid() != rustix::process::getuid().as_raw()
            || socket_metadata.mode() & 0o777 != 0o600
        {
            return Err(ipc_error(LinuxIpcErrorKind::UnsafeSocketMode));
        }
        Ok(Self {
            listener,
            path: path.to_path_buf(),
            parent_identity: SocketPathIdentity::from_metadata(&parent_metadata),
            socket_identity: SocketPathIdentity::from_metadata(&socket_metadata),
            cleaned: false,
        })
    }

    /// Accepts, identifies, and authenticates exactly one peer connection.
    fn accept_authenticated(
        &self,
        authenticator: &LinuxIpcAuthenticator,
        maximum_executable_bytes: u64,
    ) -> Result<(UnixStream, LinuxAuthenticatedPeer), LinuxIpcError> {
        let (mut stream, _) = self
            .listener
            .accept()
            .map_err(|_| ipc_error(LinuxIpcErrorKind::PlatformFailure))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .map_err(|_| ipc_error(LinuxIpcErrorKind::PlatformFailure))?;
        let credentials =
            socket_peercred(&stream).map_err(|_| ipc_error(LinuxIpcErrorKind::PlatformFailure))?;
        let pid = credentials.pid.as_raw_pid();
        let observed = stable_peer_identity_with_limit(
            credentials.uid.as_raw(),
            pid,
            maximum_executable_bytes,
        )?;
        let mut frame = [0_u8; HANDSHAKE_FRAME_BYTES];
        stream
            .read_exact(&mut frame)
            .map_err(|_| ipc_error(LinuxIpcErrorKind::MalformedFrame))?;
        let peer = authenticator.authenticate(observed, &LinuxHandshakeRequest::decode(&frame))?;
        stream
            .set_read_timeout(None)
            .map_err(|_| ipc_error(LinuxIpcErrorKind::PlatformFailure))?;
        Ok((stream, peer))
    }

    fn cleanup(&mut self) -> Result<(), LinuxIpcError> {
        if self.cleaned {
            return Ok(());
        }
        let parent = self
            .path
            .parent()
            .ok_or_else(|| ipc_error(LinuxIpcErrorKind::UnsafeSocketParent))?;
        let parent_metadata = fs::symlink_metadata(parent)
            .map_err(|_| ipc_error(LinuxIpcErrorKind::UnsafeSocketParent))?;
        let socket_metadata = fs::symlink_metadata(&self.path)
            .map_err(|_| ipc_error(LinuxIpcErrorKind::UnsafeSocketMode))?;
        if SocketPathIdentity::from_metadata(&parent_metadata) != self.parent_identity
            || SocketPathIdentity::from_metadata(&socket_metadata) != self.socket_identity
            || !socket_metadata.file_type().is_socket()
        {
            return Err(ipc_error(LinuxIpcErrorKind::UnsafeSocketMode));
        }
        fs::remove_file(&self.path).map_err(|_| ipc_error(LinuxIpcErrorKind::PlatformFailure))?;
        self.cleaned = true;
        Ok(())
    }
}

impl Drop for PrivateUnixListener {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SocketPathIdentity {
    device: u64,
    inode: u64,
    owner: u32,
    mode: u32,
}

impl SocketPathIdentity {
    fn from_metadata(metadata: &fs::Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            owner: metadata.uid(),
            mode: metadata.mode() & 0o777,
        }
    }
}

fn stable_peer_identity(uid: u32, pid: i32) -> Result<LinuxPeerIdentity, LinuxIpcError> {
    stable_peer_identity_with_limit(uid, pid, DEFAULT_PEER_EXECUTABLE_BYTES)
}

fn stable_peer_identity_with_limit(
    uid: u32,
    pid: i32,
    maximum_executable_bytes: u64,
) -> Result<LinuxPeerIdentity, LinuxIpcError> {
    let start_time_ticks = process_start_time_ticks(pid)?;
    let executable_sha256 = process_executable_sha256_bounded(pid, maximum_executable_bytes)?;
    if process_start_time_ticks(pid)? != start_time_ticks {
        return Err(ipc_error(LinuxIpcErrorKind::ProcessIdentityChanged));
    }
    Ok(LinuxPeerIdentity::new(
        uid,
        pid,
        start_time_ticks,
        executable_sha256,
    ))
}

/// Observes one same-user Linux process for an exact launch identity.
///
/// The resulting identity is suitable only for the fresh one-use local IPC
/// authenticator. The kernel credentials observed at connection time are
/// compared with this process, start time, and executable digest.
pub fn observe_linux_process_identity(pid: i32) -> Result<LinuxPeerIdentity, LinuxIpcError> {
    stable_peer_identity(rustix::process::getuid().as_raw(), pid)
}

/// Observes one explicit development peer under the separate debug-binary size ceiling.
pub fn observe_linux_development_process_identity(
    pid: i32,
) -> Result<LinuxPeerIdentity, LinuxIpcError> {
    stable_peer_identity_with_limit(
        rustix::process::getuid().as_raw(),
        pid,
        MAX_DEVELOPMENT_PEER_EXECUTABLE_BYTES,
    )
}

pub(crate) fn process_start_time_ticks(pid: i32) -> Result<u64, LinuxIpcError> {
    let bytes = fs::read(format!("/proc/{pid}/stat"))
        .map_err(|_| ipc_error(LinuxIpcErrorKind::ProcessIdentityChanged))?;
    parse_process_start_time_ticks(&bytes)
}

fn parse_process_start_time_ticks(bytes: &[u8]) -> Result<u64, LinuxIpcError> {
    if bytes.len() > 64 * 1024 {
        return Err(ipc_error(LinuxIpcErrorKind::ResourceLimitExceeded));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| ipc_error(LinuxIpcErrorKind::ProcessIdentityChanged))?;
    let end = text
        .rfind(')')
        .ok_or_else(|| ipc_error(LinuxIpcErrorKind::ProcessIdentityChanged))?;
    text[end + 1..]
        .split_ascii_whitespace()
        .nth(19)
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| ipc_error(LinuxIpcErrorKind::ProcessIdentityChanged))
}

pub(crate) fn process_executable_sha256(pid: i32) -> Result<[u8; 32], LinuxIpcError> {
    process_executable_sha256_bounded(pid, DEFAULT_PEER_EXECUTABLE_BYTES)
}

fn process_executable_sha256_bounded(
    pid: i32,
    maximum_executable_bytes: u64,
) -> Result<[u8; 32], LinuxIpcError> {
    let mut file = File::open(format!("/proc/{pid}/exe"))
        .map_err(|_| ipc_error(LinuxIpcErrorKind::WrongExecutable))?;
    let mut hasher = Sha256::new();
    let mut observed = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|_| ipc_error(LinuxIpcErrorKind::WrongExecutable))?;
        if count == 0 {
            break;
        }
        observed = observed
            .checked_add(count as u64)
            .ok_or_else(|| ipc_error(LinuxIpcErrorKind::ResourceLimitExceeded))?;
        if observed > maximum_executable_bytes {
            return Err(ipc_error(LinuxIpcErrorKind::ResourceLimitExceeded));
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hasher.finalize().into())
}

fn authentication_digest(
    protocol_version: u32,
    challenge: &[u8; 32],
    launch_secret: &[u8; 32],
    peer: &LinuxPeerIdentity,
) -> [u8; 32] {
    let mut inner_key = [0x36_u8; 64];
    let mut outer_key = [0x5c_u8; 64];
    for (index, byte) in launch_secret.iter().enumerate() {
        inner_key[index] ^= byte;
        outer_key[index] ^= byte;
    }
    let mut inner = Sha256::new();
    inner.update(inner_key);
    inner.update(b"agentmage-linux-ipc-auth-v1\0");
    inner.update(protocol_version.to_be_bytes());
    inner.update(challenge);
    inner.update(peer.uid.to_be_bytes());
    inner.update(peer.pid.to_be_bytes());
    inner.update(peer.start_time_ticks.to_be_bytes());
    inner.update(peer.executable_sha256);
    let inner_digest = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(outer_key);
    outer.update(inner_digest);
    outer.finalize().into()
}

fn constant_time_equal(left: &[u8; 32], right: &[u8; 32]) -> bool {
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

const fn ipc_error(kind: LinuxIpcErrorKind) -> LinuxIpcError {
    LinuxIpcError::new(kind)
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    use rustix::process::{getpid, getuid};

    use super::{
        LINUX_IPC_PROTOCOL_VERSION, LinuxHandshakeRequest, LinuxHostIpcEndpoint,
        LinuxIpcAuthenticator, LinuxIpcErrorKind, LinuxLaunchEnvelope, LinuxPeerIdentity,
        PrivateUnixListener, parse_process_start_time_ticks, stable_peer_identity,
    };

    fn peer() -> LinuxPeerIdentity {
        let pid = getpid().as_raw_pid();
        stable_peer_identity(getuid().as_raw(), pid).expect("stable test peer")
    }

    fn authenticator() -> LinuxIpcAuthenticator {
        LinuxIpcAuthenticator::from_launch_material(peer(), [3; 32], [4; 32])
    }

    #[test]
    fn exact_peer_authenticates_once_and_replay_fails() {
        let identity = peer();
        let request =
            LinuxHandshakeRequest::new(LINUX_IPC_PROTOCOL_VERSION, [3; 32], &[4; 32], &identity);
        let authenticator = authenticator();
        let authenticated = authenticator
            .authenticate(identity.clone(), &request)
            .expect("first authentication");
        assert_eq!(authenticated.identity(), &identity);
        assert_eq!(authenticated.protocol_version(), LINUX_IPC_PROTOCOL_VERSION);
        assert_eq!(
            authenticator
                .authenticate(identity, &request)
                .expect_err("replay")
                .kind(),
            LinuxIpcErrorKind::Replay
        );
    }

    #[test]
    fn generated_launch_material_is_fresh_and_redacted() {
        let (first_authenticator, first_credentials) =
            LinuxIpcAuthenticator::generate(peer()).expect("first launch");
        let (second_authenticator, second_credentials) =
            LinuxIpcAuthenticator::generate(peer()).expect("second launch");
        assert_ne!(
            first_credentials.challenge(),
            second_credentials.challenge()
        );
        assert!(!format!("{first_credentials:?}").contains(&"03".repeat(32)));
        first_authenticator
            .authenticate(peer(), &first_credentials.request(&peer()))
            .expect("generated first handshake");
        second_authenticator
            .authenticate(peer(), &second_credentials.request(&peer()))
            .expect("generated second handshake");
    }

    #[test]
    fn every_peer_and_frame_mutation_fails_without_consuming_valid_request() {
        let exact = peer();
        let request =
            LinuxHandshakeRequest::new(LINUX_IPC_PROTOCOL_VERSION, [3; 32], &[4; 32], &exact);
        let peer_cases = [
            (
                LinuxPeerIdentity::new(
                    exact.uid() + 1,
                    exact.pid(),
                    exact.start_time_ticks(),
                    *exact.executable_sha256(),
                ),
                LinuxIpcErrorKind::WrongUser,
            ),
            (
                LinuxPeerIdentity::new(
                    exact.uid(),
                    exact.pid() + 1,
                    exact.start_time_ticks(),
                    *exact.executable_sha256(),
                ),
                LinuxIpcErrorKind::WrongProcess,
            ),
            (
                LinuxPeerIdentity::new(exact.uid(), exact.pid(), exact.start_time_ticks(), [9; 32]),
                LinuxIpcErrorKind::WrongExecutable,
            ),
            (
                LinuxPeerIdentity::new(
                    exact.uid(),
                    exact.pid(),
                    exact.start_time_ticks() + 1,
                    *exact.executable_sha256(),
                ),
                LinuxIpcErrorKind::ProcessIdentityChanged,
            ),
        ];
        for (candidate, expected) in peer_cases {
            assert_eq!(
                authenticator()
                    .authenticate(candidate, &request)
                    .expect_err("peer mutation")
                    .kind(),
                expected
            );
        }

        let frame_cases = [
            (
                LinuxHandshakeRequest::new(2, [3; 32], &[4; 32], &exact),
                LinuxIpcErrorKind::VersionMismatch,
            ),
            (
                LinuxHandshakeRequest::new(LINUX_IPC_PROTOCOL_VERSION, [8; 32], &[4; 32], &exact),
                LinuxIpcErrorKind::ChallengeMismatch,
            ),
            (
                LinuxHandshakeRequest::new(LINUX_IPC_PROTOCOL_VERSION, [3; 32], &[8; 32], &exact),
                LinuxIpcErrorKind::AuthenticationFailed,
            ),
        ];
        for (candidate, expected) in frame_cases {
            let authenticator = authenticator();
            assert_eq!(
                authenticator
                    .authenticate(exact.clone(), &candidate)
                    .expect_err("frame mutation")
                    .kind(),
                expected
            );
            authenticator
                .authenticate(exact.clone(), &request)
                .expect("invalid request did not consume launch");
        }
    }

    #[test]
    fn private_socket_uses_kernel_peer_credentials_and_exact_frame() {
        let directory = private_test_directory();
        let socket = directory.join("agentmage.sock");
        let listener = PrivateUnixListener::bind(&socket).expect("private listener");
        let mode = std::fs::symlink_metadata(&socket)
            .expect("socket metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);

        let identity = peer();
        let request =
            LinuxHandshakeRequest::new(LINUX_IPC_PROTOCOL_VERSION, [3; 32], &[4; 32], &identity);
        let client_socket = socket.clone();
        let client = thread::spawn(move || {
            let mut stream = UnixStream::connect(client_socket).expect("connect");
            stream.write_all(&request.encode()).expect("write frame");
        });
        let (_, authenticated) = listener
            .accept_authenticated(&authenticator(), super::DEFAULT_PEER_EXECUTABLE_BYTES)
            .expect("authenticated socket peer");
        assert_eq!(authenticated.identity(), &identity);
        client.join().expect("client thread");
        drop(listener);
        assert!(!socket.exists());
        std::fs::remove_dir(directory).expect("remove directory");
    }

    #[test]
    fn direct_launch_envelope_connects_the_exact_current_process() {
        let directory = private_test_directory();
        let socket = directory.join("launch.sock");
        let endpoint = LinuxHostIpcEndpoint::bind(&socket).expect("endpoint");
        let identity = peer();
        let (authenticator, credentials) =
            LinuxIpcAuthenticator::generate(identity.clone()).expect("launch material");
        let endpoint_bytes = socket.to_str().expect("UTF-8 path").as_bytes();
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"AGMB");
        bytes.extend_from_slice(&1_u16.to_be_bytes());
        bytes.extend_from_slice(
            &u16::try_from(endpoint_bytes.len())
                .expect("bounded path")
                .to_be_bytes(),
        );
        bytes.extend_from_slice(endpoint_bytes);
        bytes.extend_from_slice(credentials.challenge());
        bytes.extend_from_slice(credentials.launch_secret());
        bytes.extend_from_slice(&identity.uid().to_be_bytes());
        bytes.extend_from_slice(&identity.pid().to_be_bytes());
        bytes.extend_from_slice(&identity.start_time_ticks().to_be_bytes());
        bytes.extend_from_slice(identity.executable_sha256());

        let host = thread::spawn(move || endpoint.accept(&authenticator).expect("authenticated"));
        let envelope = LinuxLaunchEnvelope::read(&mut bytes.as_slice()).expect("envelope");
        let client = envelope.connect().expect("client session");
        let server = host.join().expect("host thread");
        assert_eq!(client.peer().identity(), &identity);
        assert_eq!(server.peer().identity(), &identity);
        drop(client);
        drop(server);
        assert!(!socket.exists());
        std::fs::remove_dir(directory).expect("remove directory");
    }

    #[test]
    fn authenticated_endpoint_exposes_only_bounded_product_frames() {
        let directory = private_test_directory();
        let socket = directory.join("agentmage-framed.sock");
        let endpoint = LinuxHostIpcEndpoint {
            listener: PrivateUnixListener::bind(&socket).expect("private listener"),
        };
        let expected = peer();
        let (authenticator, credentials) =
            LinuxIpcAuthenticator::generate(expected.clone()).expect("launch material");
        let client = thread::spawn(move || {
            let mut stream = UnixStream::connect(socket).expect("client connection");
            stream
                .write_all(&credentials.request(&expected).encode())
                .expect("handshake");
            stream.write_all(&4_u32.to_be_bytes()).expect("length");
            stream.write_all(b"ping").expect("request");
            let mut length = [0_u8; 4];
            stream.read_exact(&mut length).expect("response length");
            let mut response = vec![0_u8; u32::from_be_bytes(length) as usize];
            stream.read_exact(&mut response).expect("response");
            response
        });
        let mut session = endpoint.accept(&authenticator).expect("authenticated peer");
        assert_eq!(session.read_frame(16).expect("bounded frame"), b"ping");
        session.write_frame(b"pong", 16).expect("bounded response");
        assert_eq!(client.join().expect("client result"), b"pong");
        drop(session);
        drop(endpoint);
        std::fs::remove_dir(directory).expect("private directory cleanup");
    }

    #[test]
    fn unsafe_parent_existing_socket_and_short_frame_fail_closed() {
        let directory = private_test_directory();
        let socket = directory.join("agentmage.sock");
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o755))
            .expect("weaken fixture");
        assert_eq!(
            PrivateUnixListener::bind(&socket)
                .expect_err("public parent")
                .kind(),
            LinuxIpcErrorKind::UnsafeSocketParent
        );
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))
            .expect("restore fixture");
        let listener = PrivateUnixListener::bind(&socket).expect("listener");
        assert_eq!(
            PrivateUnixListener::bind(&socket)
                .expect_err("existing socket")
                .kind(),
            LinuxIpcErrorKind::UnsafeSocketMode
        );

        let client_socket = socket.clone();
        let client = thread::spawn(move || {
            let mut stream = UnixStream::connect(client_socket).expect("connect");
            stream.write_all(&[0; 8]).expect("short frame");
        });
        assert_eq!(
            listener
                .accept_authenticated(&authenticator(), super::DEFAULT_PEER_EXECUTABLE_BYTES)
                .expect_err("short frame")
                .kind(),
            LinuxIpcErrorKind::MalformedFrame
        );
        client.join().expect("client thread");
        std::fs::remove_file(socket).expect("remove socket");
        std::fs::remove_dir(directory).expect("remove directory");
    }

    #[test]
    fn listener_drop_removes_only_its_unchanged_socket_identity() {
        let directory = private_test_directory();
        let socket = directory.join("agentmage.sock");
        let listener = PrivateUnixListener::bind(&socket).expect("listener");
        std::fs::remove_file(&socket).expect("original socket removed");
        let replacement = UnixListener::bind(&socket).expect("replacement socket");
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))
            .expect("replacement mode");
        drop(listener);
        assert!(socket.exists(), "foreign replacement must be retained");
        drop(replacement);
        std::fs::remove_file(&socket).expect("replacement removes");
        std::fs::remove_dir(directory).expect("directory removes");
    }

    #[test]
    fn process_start_parser_handles_parentheses_and_rejects_malformed_records() {
        let mut record = String::from("42 (worker ) name) R");
        for _ in 0..18 {
            record.push_str(" 0");
        }
        record.push_str(" 912345 0\n");
        assert_eq!(
            parse_process_start_time_ticks(record.as_bytes()).expect("start time parses"),
            912_345
        );
        assert_eq!(
            parse_process_start_time_ticks(b"42 malformed")
                .expect_err("malformed record rejects")
                .kind(),
            LinuxIpcErrorKind::ProcessIdentityChanged
        );
    }

    fn private_test_directory() -> PathBuf {
        static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);
        let directory = std::env::temp_dir().join(format!(
            "am-ipc-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::SeqCst)
        ));
        if directory.exists() {
            std::fs::remove_dir_all(&directory).expect("remove stale fixture");
        }
        std::fs::create_dir(&directory).expect("create fixture");
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))
            .expect("private fixture");
        directory
    }
}
