//! One-session production transport for the private Docker Model Runner guard.

use std::fmt;
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddrV4, TcpStream};
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use rustix::net::sockopt::socket_peercred;
use rustix::rand::{GetRandomFlags, getrandom};
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use crate::{DOCKER_MODEL_RUNNER_CONNECT_HOST, DOCKER_MODEL_RUNNER_PORT};

/// Version of the fixed guard bootstrap and handshake protocol.
pub const DOCKER_GUARD_PROTOCOL_VERSION: u16 = 1;

/// Exact byte length of one inherited guard bootstrap frame.
pub const DOCKER_GUARD_BOOTSTRAP_BYTES: usize = 126;

const BOOTSTRAP_MAGIC: &[u8; 8] = b"AMDG0001";
const CHALLENGE_BYTES: usize = 34;
const RESPONSE_BYTES: usize = 66;
const MAX_REQUEST_BYTES: usize = 16 * 1024 * 1024;
const MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;
const SOCKET_MODE: u32 = 0o660;
const SOCKET_PARENT_MODE: u32 = 0o710;
const IO_TIMEOUT: Duration = Duration::from_secs(30);
const ALLOWED_PATH: &str = "/engines/llama.cpp/v1/chat/completions";

/// Stable content-free production guard failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockerGuardServiceError {
    /// The inherited bootstrap frame was malformed or unsafe.
    Bootstrap,
    /// The runtime directory or socket metadata was unsafe.
    SocketBoundary,
    /// The kernel-reported peer identity did not match the bootstrap.
    PeerIdentity,
    /// The one-use challenge response was invalid or replayed.
    Authentication,
    /// A request or response frame was malformed or exceeded its bound.
    Frame,
    /// The HTTP method, path, headers, or body were outside the inference allowlist.
    RequestPolicy,
    /// The private loopback upstream was unavailable or exceeded its bound.
    Upstream,
    /// A required operating-system operation failed.
    Platform,
}

impl DockerGuardServiceError {
    /// Returns the stable redacted error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Bootstrap => "docker-guard.service.bootstrap",
            Self::SocketBoundary => "docker-guard.service.socket-boundary",
            Self::PeerIdentity => "docker-guard.service.peer-identity",
            Self::Authentication => "docker-guard.service.authentication",
            Self::Frame => "docker-guard.service.frame",
            Self::RequestPolicy => "docker-guard.service.request-policy",
            Self::Upstream => "docker-guard.service.upstream",
            Self::Platform => "docker-guard.service.platform",
        }
    }
}

impl fmt::Display for DockerGuardServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for DockerGuardServiceError {}

/// Exact process identity admitted to one guard session.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ExpectedPeer {
    uid: u32,
    pid: i32,
    start_time_ticks: u64,
    executable_sha256: [u8; 32],
    cgroup_sha256: [u8; 32],
}

/// One inherited, fixed-size guard bootstrap record.
pub struct DockerGuardBootstrap {
    runtime_gid: u32,
    expected_peer: ExpectedPeer,
    session_secret: [u8; 32],
}

impl fmt::Debug for DockerGuardBootstrap {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DockerGuardBootstrap")
            .field("expected_peer", &self.expected_peer)
            .field("session_secret", &"redacted")
            .finish_non_exhaustive()
    }
}

impl DockerGuardBootstrap {
    /// Decodes the exact inherited bootstrap frame without accepting text or ambient defaults.
    pub fn decode(frame: &[u8]) -> Result<Self, DockerGuardServiceError> {
        if frame.len() != DOCKER_GUARD_BOOTSTRAP_BYTES || &frame[..8] != BOOTSTRAP_MAGIC {
            return Err(DockerGuardServiceError::Bootstrap);
        }
        let version = u16::from_be_bytes([frame[8], frame[9]]);
        let runtime_uid = u32_at(frame, 10)?;
        let runtime_gid = u32_at(frame, 14)?;
        let peer_pid_u32 = u32_at(frame, 18)?;
        let peer_pid =
            i32::try_from(peer_pid_u32).map_err(|_| DockerGuardServiceError::Bootstrap)?;
        let start_time_ticks = u64_at(frame, 22)?;
        let executable_sha256 = array_at(frame, 30)?;
        let cgroup_sha256 = array_at(frame, 62)?;
        let session_secret = array_at(frame, 94)?;
        if version != DOCKER_GUARD_PROTOCOL_VERSION
            || runtime_uid == 0
            || runtime_gid == 0
            || peer_pid <= 1
            || start_time_ticks == 0
            || executable_sha256 == [0; 32]
            || cgroup_sha256 == [0; 32]
            || session_secret == [0; 32]
        {
            return Err(DockerGuardServiceError::Bootstrap);
        }
        Ok(Self {
            runtime_gid,
            expected_peer: ExpectedPeer {
                uid: runtime_uid,
                pid: peer_pid,
                start_time_ticks,
                executable_sha256,
                cgroup_sha256,
            },
            session_secret,
        })
    }

    #[cfg(test)]
    fn encode_for_test(&self) -> [u8; DOCKER_GUARD_BOOTSTRAP_BYTES] {
        let mut frame = [0_u8; DOCKER_GUARD_BOOTSTRAP_BYTES];
        frame[..8].copy_from_slice(BOOTSTRAP_MAGIC);
        frame[8..10].copy_from_slice(&DOCKER_GUARD_PROTOCOL_VERSION.to_be_bytes());
        frame[10..14].copy_from_slice(&self.expected_peer.uid.to_be_bytes());
        frame[14..18].copy_from_slice(&self.runtime_gid.to_be_bytes());
        frame[18..22].copy_from_slice(&(self.expected_peer.pid as u32).to_be_bytes());
        frame[22..30].copy_from_slice(&self.expected_peer.start_time_ticks.to_be_bytes());
        frame[30..62].copy_from_slice(&self.expected_peer.executable_sha256);
        frame[62..94].copy_from_slice(&self.expected_peer.cgroup_sha256);
        frame[94..].copy_from_slice(&self.session_secret);
        frame
    }
}

impl Drop for DockerGuardBootstrap {
    fn drop(&mut self) {
        self.session_secret.zeroize();
    }
}

/// One-use client material delivered separately to the expected kernel peer.
pub struct DockerGuardSessionCredentials {
    secret: [u8; 32],
}

impl DockerGuardSessionCredentials {
    #[cfg(test)]
    fn response(&self, challenge: &[u8; 32], peer: &ExpectedPeer) -> [u8; 32] {
        authentication_digest(challenge, &self.secret, peer)
    }
}

impl fmt::Debug for DockerGuardSessionCredentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DockerGuardSessionCredentials")
            .field("secret", &"redacted")
            .finish()
    }
}

impl Drop for DockerGuardSessionCredentials {
    fn drop(&mut self) {
        self.secret.zeroize();
    }
}

/// Bound one-session Docker guard service.
pub struct DockerGuardService {
    listener: UnixListener,
    socket_path: PathBuf,
    socket_identity: ObjectIdentity,
    parent_identity: ObjectIdentity,
    bootstrap: DockerGuardBootstrap,
    consumed: AtomicBool,
}

impl fmt::Debug for DockerGuardService {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DockerGuardService")
            .field("bootstrap", &self.bootstrap)
            .field("consumed", &self.consumed.load(Ordering::SeqCst))
            .finish_non_exhaustive()
    }
}

impl DockerGuardService {
    /// Binds the exact guard socket beneath an administrator-created runtime directory.
    pub fn bind(
        socket_path: &Path,
        bootstrap: DockerGuardBootstrap,
    ) -> Result<Self, DockerGuardServiceError> {
        let parent = socket_path
            .parent()
            .ok_or(DockerGuardServiceError::SocketBoundary)?;
        let parent_metadata =
            fs::symlink_metadata(parent).map_err(|_| DockerGuardServiceError::SocketBoundary)?;
        let guard_uid = rustix::process::getuid().as_raw();
        let guard_gid = rustix::process::getgid().as_raw();
        if !parent_metadata.file_type().is_dir()
            || parent_metadata.uid() != guard_uid
            || parent_metadata.gid() != bootstrap.runtime_gid
            || guard_gid != bootstrap.runtime_gid
            || parent_metadata.mode() & 0o777 != SOCKET_PARENT_MODE
            || bootstrap.expected_peer.uid == guard_uid
            || fs::symlink_metadata(socket_path).is_ok()
        {
            return Err(DockerGuardServiceError::SocketBoundary);
        }
        let listener =
            UnixListener::bind(socket_path).map_err(|_| DockerGuardServiceError::Platform)?;
        fs::set_permissions(socket_path, fs::Permissions::from_mode(SOCKET_MODE))
            .map_err(|_| DockerGuardServiceError::Platform)?;
        let socket_metadata =
            fs::symlink_metadata(socket_path).map_err(|_| DockerGuardServiceError::Platform)?;
        if !socket_metadata.file_type().is_socket()
            || socket_metadata.uid() != guard_uid
            || socket_metadata.gid() != bootstrap.runtime_gid
            || socket_metadata.mode() & 0o777 != SOCKET_MODE
        {
            return Err(DockerGuardServiceError::SocketBoundary);
        }
        Ok(Self {
            listener,
            socket_path: socket_path.to_path_buf(),
            socket_identity: ObjectIdentity::from_metadata(&socket_metadata),
            parent_identity: ObjectIdentity::from_metadata(&parent_metadata),
            bootstrap,
            consumed: AtomicBool::new(false),
        })
    }

    /// Serves exactly one authenticated request and then consumes the session.
    pub fn serve_once(&self) -> Result<(), DockerGuardServiceError> {
        if self.consumed.swap(true, Ordering::AcqRel) {
            return Err(DockerGuardServiceError::Authentication);
        }
        self.revalidate_socket()?;
        let (mut stream, _) = self
            .listener
            .accept()
            .map_err(|_| DockerGuardServiceError::Platform)?;
        stream
            .set_read_timeout(Some(IO_TIMEOUT))
            .and_then(|()| stream.set_write_timeout(Some(IO_TIMEOUT)))
            .map_err(|_| DockerGuardServiceError::Platform)?;
        let peer = observe_peer(&stream, &self.bootstrap.expected_peer)?;
        authenticate(&mut stream, &peer, &self.bootstrap.session_secret)?;
        let request = read_frame(&mut stream, MAX_REQUEST_BYTES)?;
        validate_http_request(&request)?;
        let response = forward_to_runner(&request)?;
        write_frame(&mut stream, &response, MAX_RESPONSE_BYTES)
    }

    fn revalidate_socket(&self) -> Result<(), DockerGuardServiceError> {
        let parent = fs::symlink_metadata(
            self.socket_path
                .parent()
                .ok_or(DockerGuardServiceError::SocketBoundary)?,
        )
        .map_err(|_| DockerGuardServiceError::SocketBoundary)?;
        let socket = fs::symlink_metadata(&self.socket_path)
            .map_err(|_| DockerGuardServiceError::SocketBoundary)?;
        if ObjectIdentity::from_metadata(&parent) != self.parent_identity
            || ObjectIdentity::from_metadata(&socket) != self.socket_identity
            || !socket.file_type().is_socket()
            || parent.mode() & 0o777 != SOCKET_PARENT_MODE
            || socket.mode() & 0o777 != SOCKET_MODE
        {
            return Err(DockerGuardServiceError::SocketBoundary);
        }
        Ok(())
    }
}

impl Drop for DockerGuardService {
    fn drop(&mut self) {
        if let Ok(metadata) = fs::symlink_metadata(&self.socket_path)
            && metadata.file_type().is_socket()
            && ObjectIdentity::from_metadata(&metadata) == self.socket_identity
        {
            let _ = fs::remove_file(&self.socket_path);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ObjectIdentity {
    device: u64,
    inode: u64,
    uid: u32,
    gid: u32,
}

impl ObjectIdentity {
    fn from_metadata(metadata: &fs::Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            uid: metadata.uid(),
            gid: metadata.gid(),
        }
    }
}

fn observe_peer(
    stream: &UnixStream,
    expected: &ExpectedPeer,
) -> Result<ExpectedPeer, DockerGuardServiceError> {
    let credentials = socket_peercred(stream).map_err(|_| DockerGuardServiceError::Platform)?;
    let pid = credentials.pid.as_raw_nonzero().get();
    let uid = credentials.uid.as_raw();
    let start_time_ticks = process_start_time(pid)?;
    let cgroup = fs::read(format!("/proc/{pid}/cgroup"))
        .map_err(|_| DockerGuardServiceError::PeerIdentity)?;
    verify_kernel_peer(
        expected,
        uid,
        pid,
        start_time_ticks,
        Sha256::digest(cgroup).into(),
    )
}

fn verify_kernel_peer(
    expected: &ExpectedPeer,
    uid: u32,
    pid: i32,
    start_time_ticks: u64,
    cgroup_sha256: [u8; 32],
) -> Result<ExpectedPeer, DockerGuardServiceError> {
    if uid != expected.uid
        || pid != expected.pid
        || start_time_ticks != expected.start_time_ticks
        || cgroup_sha256 != expected.cgroup_sha256
    {
        return Err(DockerGuardServiceError::PeerIdentity);
    }
    Ok(expected.clone())
}

fn process_start_time(pid: i32) -> Result<u64, DockerGuardServiceError> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat"))
        .map_err(|_| DockerGuardServiceError::PeerIdentity)?;
    let command_end = stat
        .rfind(')')
        .ok_or(DockerGuardServiceError::PeerIdentity)?;
    stat.get(command_end + 2..)
        .and_then(|fields| fields.split_whitespace().nth(19))
        .and_then(|value| value.parse().ok())
        .filter(|value| *value > 0)
        .ok_or(DockerGuardServiceError::PeerIdentity)
}

fn authenticate(
    stream: &mut UnixStream,
    peer: &ExpectedPeer,
    secret: &[u8; 32],
) -> Result<(), DockerGuardServiceError> {
    let mut challenge = [0_u8; 32];
    if getrandom(&mut challenge, GetRandomFlags::empty())
        .map_err(|_| DockerGuardServiceError::Platform)?
        != challenge.len()
    {
        return Err(DockerGuardServiceError::Platform);
    }
    let mut challenge_frame = [0_u8; CHALLENGE_BYTES];
    challenge_frame[..2].copy_from_slice(&DOCKER_GUARD_PROTOCOL_VERSION.to_be_bytes());
    challenge_frame[2..].copy_from_slice(&challenge);
    stream
        .write_all(&challenge_frame)
        .and_then(|()| stream.flush())
        .map_err(|_| DockerGuardServiceError::Authentication)?;
    let mut response = [0_u8; RESPONSE_BYTES];
    stream
        .read_exact(&mut response)
        .map_err(|_| DockerGuardServiceError::Authentication)?;
    let version = u16::from_be_bytes([response[0], response[1]]);
    let mut echoed_challenge = [0_u8; 32];
    echoed_challenge.copy_from_slice(&response[2..34]);
    let expected = authentication_digest(&challenge, secret, peer);
    let accepted = version == DOCKER_GUARD_PROTOCOL_VERSION
        && constant_time_equal(&challenge, &echoed_challenge)
        && constant_time_equal(&expected, array_ref(&response[34..])?);
    response.zeroize();
    challenge.zeroize();
    if !accepted {
        return Err(DockerGuardServiceError::Authentication);
    }
    Ok(())
}

fn authentication_digest(challenge: &[u8; 32], secret: &[u8; 32], peer: &ExpectedPeer) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(b"agentmage-docker-guard-session-v1\0");
    digest.update(DOCKER_GUARD_PROTOCOL_VERSION.to_be_bytes());
    digest.update(challenge);
    digest.update(secret);
    digest.update(peer.uid.to_be_bytes());
    digest.update(peer.pid.to_be_bytes());
    digest.update(peer.start_time_ticks.to_be_bytes());
    digest.update(peer.executable_sha256);
    digest.update(peer.cgroup_sha256);
    digest.finalize().into()
}

fn constant_time_equal(left: &[u8; 32], right: &[u8; 32]) -> bool {
    let mut difference = 0_u8;
    for (left, right) in left.iter().zip(right.iter()) {
        difference |= left ^ right;
    }
    difference == 0
}

fn read_frame(stream: &mut UnixStream, maximum: usize) -> Result<Vec<u8>, DockerGuardServiceError> {
    let mut length = [0_u8; 4];
    stream
        .read_exact(&mut length)
        .map_err(|_| DockerGuardServiceError::Frame)?;
    let length =
        usize::try_from(u32::from_be_bytes(length)).map_err(|_| DockerGuardServiceError::Frame)?;
    if length == 0 || length > maximum {
        return Err(DockerGuardServiceError::Frame);
    }
    let mut frame = vec![0_u8; length];
    stream
        .read_exact(&mut frame)
        .map_err(|_| DockerGuardServiceError::Frame)?;
    Ok(frame)
}

fn write_frame(
    stream: &mut UnixStream,
    frame: &[u8],
    maximum: usize,
) -> Result<(), DockerGuardServiceError> {
    if frame.is_empty() || frame.len() > maximum {
        return Err(DockerGuardServiceError::Frame);
    }
    let length = u32::try_from(frame.len()).map_err(|_| DockerGuardServiceError::Frame)?;
    stream
        .write_all(&length.to_be_bytes())
        .and_then(|()| stream.write_all(frame))
        .and_then(|()| stream.flush())
        .map_err(|_| DockerGuardServiceError::Frame)
}

fn validate_http_request(request: &[u8]) -> Result<(), DockerGuardServiceError> {
    let header_end = request
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
        .ok_or(DockerGuardServiceError::RequestPolicy)?;
    if header_end > 32 * 1024 {
        return Err(DockerGuardServiceError::RequestPolicy);
    }
    let headers = std::str::from_utf8(&request[..header_end])
        .map_err(|_| DockerGuardServiceError::RequestPolicy)?;
    let mut lines = headers.split("\r\n");
    if lines.next() != Some(&format!("POST {ALLOWED_PATH} HTTP/1.1")) {
        return Err(DockerGuardServiceError::RequestPolicy);
    }
    let mut host = None;
    let mut content_length = None;
    let mut content_type = None;
    let mut connection = None;
    for line in lines.filter(|line| !line.is_empty()) {
        let (name, value) = line
            .split_once(':')
            .ok_or(DockerGuardServiceError::RequestPolicy)?;
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();
        match name.as_str() {
            "host" if host.is_none() => host = Some(value),
            "content-length" if content_length.is_none() => {
                content_length = Some(
                    value
                        .parse::<usize>()
                        .map_err(|_| DockerGuardServiceError::RequestPolicy)?,
                );
            }
            "content-type" if content_type.is_none() => content_type = Some(value),
            "connection" if connection.is_none() => connection = Some(value),
            "transfer-encoding" | "upgrade" | "proxy-authorization" => {
                return Err(DockerGuardServiceError::RequestPolicy);
            }
            _ => {}
        }
    }
    if host != Some("127.0.0.1:12434")
        || content_length != Some(request.len() - header_end)
        || content_type != Some("application/json")
        || connection != Some("close")
        || request.len() - header_end == 0
    {
        return Err(DockerGuardServiceError::RequestPolicy);
    }
    Ok(())
}

fn forward_to_runner(request: &[u8]) -> Result<Vec<u8>, DockerGuardServiceError> {
    let address = SocketAddrV4::new(DOCKER_MODEL_RUNNER_CONNECT_HOST, DOCKER_MODEL_RUNNER_PORT);
    let mut upstream = TcpStream::connect_timeout(&address.into(), IO_TIMEOUT)
        .map_err(|_| DockerGuardServiceError::Upstream)?;
    upstream
        .set_read_timeout(Some(IO_TIMEOUT))
        .and_then(|()| upstream.set_write_timeout(Some(IO_TIMEOUT)))
        .map_err(|_| DockerGuardServiceError::Upstream)?;
    upstream
        .write_all(request)
        .and_then(|()| upstream.flush())
        .map_err(|_| DockerGuardServiceError::Upstream)?;
    let mut response = Vec::new();
    upstream
        .take((MAX_RESPONSE_BYTES + 1) as u64)
        .read_to_end(&mut response)
        .map_err(|_| DockerGuardServiceError::Upstream)?;
    if response.is_empty() || response.len() > MAX_RESPONSE_BYTES {
        return Err(DockerGuardServiceError::Upstream);
    }
    Ok(response)
}

fn u32_at(frame: &[u8], offset: usize) -> Result<u32, DockerGuardServiceError> {
    let value = frame
        .get(offset..offset + 4)
        .ok_or(DockerGuardServiceError::Bootstrap)?;
    Ok(u32::from_be_bytes(
        value
            .try_into()
            .map_err(|_| DockerGuardServiceError::Bootstrap)?,
    ))
}

fn u64_at(frame: &[u8], offset: usize) -> Result<u64, DockerGuardServiceError> {
    let value = frame
        .get(offset..offset + 8)
        .ok_or(DockerGuardServiceError::Bootstrap)?;
    Ok(u64::from_be_bytes(
        value
            .try_into()
            .map_err(|_| DockerGuardServiceError::Bootstrap)?,
    ))
}

fn array_at(frame: &[u8], offset: usize) -> Result<[u8; 32], DockerGuardServiceError> {
    frame
        .get(offset..offset + 32)
        .ok_or(DockerGuardServiceError::Bootstrap)?
        .try_into()
        .map_err(|_| DockerGuardServiceError::Bootstrap)
}

fn array_ref(value: &[u8]) -> Result<&[u8; 32], DockerGuardServiceError> {
    value
        .try_into()
        .map_err(|_| DockerGuardServiceError::Authentication)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer() -> ExpectedPeer {
        ExpectedPeer {
            uid: 1000,
            pid: 1234,
            start_time_ticks: 5678,
            executable_sha256: [2; 32],
            cgroup_sha256: [3; 32],
        }
    }

    fn bootstrap() -> DockerGuardBootstrap {
        DockerGuardBootstrap {
            runtime_gid: 1000,
            expected_peer: peer(),
            session_secret: [4; 32],
        }
    }

    #[test]
    fn bootstrap_is_exact_fixed_size_and_rejects_every_identity_zero() {
        let frame = bootstrap().encode_for_test();
        let decoded = DockerGuardBootstrap::decode(&frame).expect("exact bootstrap");
        assert_eq!(decoded.runtime_gid, 1000);
        assert_eq!(decoded.expected_peer, peer());
        for range in [
            8..10,
            10..14,
            14..18,
            18..22,
            22..30,
            30..62,
            62..94,
            94..126,
        ] {
            let mut changed = frame;
            changed[range].fill(0);
            assert_eq!(
                DockerGuardBootstrap::decode(&changed).unwrap_err(),
                DockerGuardServiceError::Bootstrap
            );
        }
        assert_eq!(
            DockerGuardBootstrap::decode(&frame[..125]).unwrap_err(),
            DockerGuardServiceError::Bootstrap
        );
    }

    #[test]
    fn authentication_binds_challenge_secret_and_complete_peer() {
        let credentials = DockerGuardSessionCredentials { secret: [4; 32] };
        let challenge = [8; 32];
        let expected = credentials.response(&challenge, &peer());
        assert_eq!(
            expected,
            authentication_digest(&challenge, &[4; 32], &peer())
        );
        for changed in [
            authentication_digest(&[9; 32], &[4; 32], &peer()),
            authentication_digest(&challenge, &[5; 32], &peer()),
            authentication_digest(
                &challenge,
                &[4; 32],
                &ExpectedPeer {
                    uid: 1001,
                    ..peer()
                },
            ),
            authentication_digest(
                &challenge,
                &[4; 32],
                &ExpectedPeer {
                    pid: 1235,
                    ..peer()
                },
            ),
            authentication_digest(
                &challenge,
                &[4; 32],
                &ExpectedPeer {
                    start_time_ticks: 5679,
                    ..peer()
                },
            ),
            authentication_digest(
                &challenge,
                &[4; 32],
                &ExpectedPeer {
                    executable_sha256: [7; 32],
                    ..peer()
                },
            ),
            authentication_digest(
                &challenge,
                &[4; 32],
                &ExpectedPeer {
                    cgroup_sha256: [7; 32],
                    ..peer()
                },
            ),
        ] {
            assert_ne!(changed, expected);
        }
    }

    #[test]
    fn kernel_peer_verification_binds_readable_identity_before_challenge() {
        let expected = peer();
        assert_eq!(
            verify_kernel_peer(
                &expected,
                expected.uid,
                expected.pid,
                expected.start_time_ticks,
                expected.cgroup_sha256,
            ),
            Ok(expected.clone())
        );
        for changed in [
            verify_kernel_peer(
                &expected,
                expected.uid + 1,
                expected.pid,
                expected.start_time_ticks,
                expected.cgroup_sha256,
            ),
            verify_kernel_peer(
                &expected,
                expected.uid,
                expected.pid + 1,
                expected.start_time_ticks,
                expected.cgroup_sha256,
            ),
            verify_kernel_peer(
                &expected,
                expected.uid,
                expected.pid,
                expected.start_time_ticks + 1,
                expected.cgroup_sha256,
            ),
            verify_kernel_peer(
                &expected,
                expected.uid,
                expected.pid,
                expected.start_time_ticks,
                [7; 32],
            ),
        ] {
            assert_eq!(changed, Err(DockerGuardServiceError::PeerIdentity));
        }
    }

    fn request(path: &str, host: &str, body: &[u8]) -> Vec<u8> {
        format!(
            "POST {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
            body.len()
        )
        .into_bytes()
        .into_iter()
        .chain(body.iter().copied())
        .collect()
    }

    #[test]
    fn only_exact_bounded_chat_request_is_admitted() {
        let exact = request(ALLOWED_PATH, "127.0.0.1:12434", b"{}");
        assert_eq!(validate_http_request(&exact), Ok(()));
        for changed in [
            request("/models/create", "127.0.0.1:12434", b"{}"),
            request(ALLOWED_PATH, "localhost:12434", b"{}"),
            request(ALLOWED_PATH, "127.0.0.1:12434", b""),
            b"GET /engines/llama.cpp/v1/chat/completions HTTP/1.1\r\nHost: 127.0.0.1:12434\r\n\r\n".to_vec(),
            b"POST /engines/llama.cpp/v1/chat/completions HTTP/1.1\r\nHost: 127.0.0.1:12434\r\nTransfer-Encoding: chunked\r\n\r\n2\r\n{}\r\n0\r\n\r\n".to_vec(),
        ] {
            assert_eq!(
                validate_http_request(&changed),
                Err(DockerGuardServiceError::RequestPolicy)
            );
        }
    }

    #[test]
    fn debug_and_errors_are_content_free() {
        let text = format!("{:?}", bootstrap());
        assert!(text.contains("redacted"));
        assert!(!text.contains("04040404"));
        for error in [
            DockerGuardServiceError::Bootstrap,
            DockerGuardServiceError::SocketBoundary,
            DockerGuardServiceError::PeerIdentity,
            DockerGuardServiceError::Authentication,
            DockerGuardServiceError::Frame,
            DockerGuardServiceError::RequestPolicy,
            DockerGuardServiceError::Upstream,
            DockerGuardServiceError::Platform,
        ] {
            assert!(error.code().starts_with("docker-guard.service."));
            assert!(!error.code().contains('/'));
        }
    }

    #[test]
    fn socket_boundary_rejects_an_ordinary_same_user_guard() {
        let directory =
            std::env::temp_dir().join(format!("agentmage-docker-guard-{}", std::process::id()));
        fs::create_dir(&directory).expect("test directory");
        fs::set_permissions(&directory, fs::Permissions::from_mode(SOCKET_PARENT_MODE))
            .expect("test mode");
        let result = DockerGuardService::bind(&directory.join("guard.sock"), bootstrap());
        assert_eq!(result.unwrap_err(), DockerGuardServiceError::SocketBoundary);
        fs::remove_dir(directory).expect("remove test directory");
    }
}
