//! Authenticated, mode-restricted Linux local IPC.

use std::fmt;
use std::fs::{self, File};
use std::io::Read;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use rustix::net::sockopt::socket_peercred;
use rustix::rand::{GetRandomFlags, getrandom};
use sha2::{Digest, Sha256};

/// Version of the fixed Linux host/bridge authentication frame.
pub const LINUX_IPC_PROTOCOL_VERSION: u32 = 1;

const HANDSHAKE_FRAME_BYTES: usize = 68;
const MAX_PEER_EXECUTABLE_BYTES: u64 = 64 * 1024 * 1024;

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
    executable_sha256: [u8; 32],
}

impl LinuxPeerIdentity {
    /// Creates an exact peer identity from owner, process, and executable digest.
    #[must_use]
    pub const fn new(uid: u32, pid: i32, executable_sha256: [u8; 32]) -> Self {
        Self {
            uid,
            pid,
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

/// Evidence returned only after one fresh local peer authenticates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxAuthenticatedPeer {
    identity: LinuxPeerIdentity,
    protocol_version: u32,
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
pub struct PrivateUnixListener {
    listener: UnixListener,
}

impl fmt::Debug for PrivateUnixListener {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PrivateUnixListener")
            .finish_non_exhaustive()
    }
}

impl PrivateUnixListener {
    /// Binds a new `0600` socket after validating its private owner-only parent.
    pub fn bind(path: &Path) -> Result<Self, LinuxIpcError> {
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
        Ok(Self { listener })
    }

    /// Accepts, identifies, and authenticates exactly one peer connection.
    pub fn accept_authenticated(
        &self,
        authenticator: &LinuxIpcAuthenticator,
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
        let observed = LinuxPeerIdentity::new(
            credentials.uid.as_raw(),
            pid,
            process_executable_sha256(pid)?,
        );
        let mut frame = [0_u8; HANDSHAKE_FRAME_BYTES];
        stream
            .read_exact(&mut frame)
            .map_err(|_| ipc_error(LinuxIpcErrorKind::MalformedFrame))?;
        let peer = authenticator.authenticate(observed, &LinuxHandshakeRequest::decode(&frame))?;
        Ok((stream, peer))
    }
}

fn process_executable_sha256(pid: i32) -> Result<[u8; 32], LinuxIpcError> {
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
        if observed > MAX_PEER_EXECUTABLE_BYTES {
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
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixStream;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    use rustix::process::{getpid, getuid};

    use super::{
        LINUX_IPC_PROTOCOL_VERSION, LinuxHandshakeRequest, LinuxIpcAuthenticator,
        LinuxIpcErrorKind, LinuxPeerIdentity, PrivateUnixListener, process_executable_sha256,
    };

    fn peer() -> LinuxPeerIdentity {
        let pid = getpid().as_raw_pid();
        LinuxPeerIdentity::new(
            getuid().as_raw(),
            pid,
            process_executable_sha256(pid).expect("test executable digest"),
        )
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
                LinuxPeerIdentity::new(exact.uid() + 1, exact.pid(), *exact.executable_sha256()),
                LinuxIpcErrorKind::WrongUser,
            ),
            (
                LinuxPeerIdentity::new(exact.uid(), exact.pid() + 1, *exact.executable_sha256()),
                LinuxIpcErrorKind::WrongProcess,
            ),
            (
                LinuxPeerIdentity::new(exact.uid(), exact.pid(), [9; 32]),
                LinuxIpcErrorKind::WrongExecutable,
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
            .accept_authenticated(&authenticator())
            .expect("authenticated socket peer");
        assert_eq!(authenticated.identity(), &identity);
        client.join().expect("client thread");
        std::fs::remove_file(socket).expect("remove socket");
        std::fs::remove_dir(directory).expect("remove directory");
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
                .accept_authenticated(&authenticator())
                .expect_err("short frame")
                .kind(),
            LinuxIpcErrorKind::MalformedFrame
        );
        client.join().expect("client thread");
        std::fs::remove_file(socket).expect("remove socket");
        std::fs::remove_dir(directory).expect("remove directory");
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
