//! Read-only bounded Docker Engine API observations over the fixed Unix socket.

use std::fmt;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use rustix::net::sockopt::socket_peercred;
use serde::Deserialize;

use crate::docker_linux_observer::{
    ProcessObservation, UnixSocketObservation, observe_current_process, observe_unix_socket,
};

const DOCKER_SOCKET: &str = "/run/docker.sock";
const API_PREFIX: &str = "/v1.47";
const MAX_REQUEST_PATH_BYTES: usize = 512;
const MAX_RESPONSE_BYTES: u64 = 4 * 1024 * 1024;
const IO_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DockerHttpObserverError {
    SocketIdentity,
    DaemonIdentity,
    Request,
    Response,
    DockerState,
}

impl DockerHttpObserverError {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::SocketIdentity => "docker-collector.docker-socket",
            Self::DaemonIdentity => "docker-collector.docker-daemon",
            Self::Request => "docker-collector.docker-request",
            Self::Response => "docker-collector.docker-response",
            Self::DockerState => "docker-collector.docker-state",
        }
    }
}

impl fmt::Display for DockerHttpObserverError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for DockerHttpObserverError {}

#[derive(Debug)]
pub(crate) struct DockerDaemonObservationClient {
    socket: UnixSocketObservation,
    daemon: ProcessObservation,
    socket_path: &'static Path,
}

impl DockerDaemonObservationClient {
    pub(crate) fn connect() -> Result<Self, DockerHttpObserverError> {
        Self::connect_at(Path::new(DOCKER_SOCKET))
    }

    fn connect_at(socket_path: &'static Path) -> Result<Self, DockerHttpObserverError> {
        let socket = observe_unix_socket(socket_path)
            .map_err(|_| DockerHttpObserverError::SocketIdentity)?;
        let stream = connect(socket_path)?;
        let credentials =
            socket_peercred(&stream).map_err(|_| DockerHttpObserverError::DaemonIdentity)?;
        let pid = credentials.pid.as_raw_nonzero().get();
        let daemon =
            observe_current_process(pid).map_err(|_| DockerHttpObserverError::DaemonIdentity)?;
        if daemon.uid != credentials.uid.as_raw() {
            return Err(DockerHttpObserverError::DaemonIdentity);
        }
        Ok(Self {
            socket,
            daemon,
            socket_path,
        })
    }

    pub(crate) const fn socket(&self) -> &UnixSocketObservation {
        &self.socket
    }

    pub(crate) const fn daemon(&self) -> &ProcessObservation {
        &self.daemon
    }

    pub(crate) fn info(&self) -> Result<DockerInfo, DockerHttpObserverError> {
        self.get_json(&format!("{API_PREFIX}/info"))
    }

    pub(crate) fn container(
        &self,
        container_id: &str,
    ) -> Result<DockerContainerInspect, DockerHttpObserverError> {
        validate_hex_identity(container_id)?;
        self.get_json(&format!("{API_PREFIX}/containers/{container_id}/json"))
    }

    pub(crate) fn running_containers(
        &self,
    ) -> Result<Vec<DockerContainerSummary>, DockerHttpObserverError> {
        self.get_json(&format!("{API_PREFIX}/containers/json?all=0"))
    }

    pub(crate) fn image(
        &self,
        image_id: &str,
    ) -> Result<DockerImageInspect, DockerHttpObserverError> {
        validate_digest_or_hex_identity(image_id)?;
        self.get_json(&format!("{API_PREFIX}/images/{image_id}/json"))
    }

    fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
    ) -> Result<T, DockerHttpObserverError> {
        let body = request(self.socket_path, path)?;
        serde_json::from_slice(&body).map_err(|_| DockerHttpObserverError::DockerState)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct DockerInfo {
    pub(crate) security_options: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct DockerContainerInspect {
    pub(crate) id: String,
    pub(crate) image: String,
    pub(crate) state: DockerContainerState,
    pub(crate) config: DockerContainerConfig,
    pub(crate) host_config: DockerHostConfig,
    pub(crate) mounts: Vec<DockerMount>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct DockerContainerState {
    pub(crate) running: bool,
    pub(crate) pid: i32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct DockerContainerConfig {
    pub(crate) image: String,
    pub(crate) user: String,
    pub(crate) env: Vec<String>,
    pub(crate) labels: std::collections::BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct DockerHostConfig {
    pub(crate) network_mode: String,
    pub(crate) privileged: bool,
    pub(crate) readonly_rootfs: bool,
    pub(crate) cap_add: Option<Vec<String>>,
    pub(crate) security_opt: Option<Vec<String>>,
    pub(crate) memory: u64,
    pub(crate) memory_swap: i64,
    pub(crate) nano_cpus: u64,
    pub(crate) pids_limit: Option<i64>,
    pub(crate) port_bindings: Option<serde_json::Value>,
    pub(crate) tmpfs: Option<std::collections::BTreeMap<String, String>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct DockerMount {
    #[serde(rename = "Type")]
    pub(crate) kind: String,
    pub(crate) source: String,
    pub(crate) destination: String,
    pub(crate) mode: String,
    #[serde(rename = "RW")]
    pub(crate) read_write: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct DockerContainerSummary {
    pub(crate) id: String,
    #[serde(rename = "ImageID")]
    pub(crate) image_id: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct DockerImageInspect {
    pub(crate) id: String,
    pub(crate) repo_digests: Vec<String>,
}

fn request(socket_path: &Path, path: &str) -> Result<Vec<u8>, DockerHttpObserverError> {
    validate_request_path(path)?;
    let mut stream = connect(socket_path)?;
    let request = format!("GET {path} HTTP/1.1\r\nHost: docker\r\nConnection: close\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .and_then(|()| stream.flush())
        .map_err(|_| DockerHttpObserverError::Request)?;
    let mut response = Vec::new();
    stream
        .take(MAX_RESPONSE_BYTES + 1)
        .read_to_end(&mut response)
        .map_err(|_| DockerHttpObserverError::Response)?;
    if response.is_empty() || response.len() as u64 > MAX_RESPONSE_BYTES {
        return Err(DockerHttpObserverError::Response);
    }
    parse_response(&response)
}

fn connect(path: &Path) -> Result<UnixStream, DockerHttpObserverError> {
    let stream = UnixStream::connect(path).map_err(|_| DockerHttpObserverError::SocketIdentity)?;
    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .and_then(|()| stream.set_write_timeout(Some(IO_TIMEOUT)))
        .map_err(|_| DockerHttpObserverError::SocketIdentity)?;
    Ok(stream)
}

fn validate_request_path(path: &str) -> Result<(), DockerHttpObserverError> {
    if !path.starts_with(API_PREFIX)
        || path.len() > MAX_REQUEST_PATH_BYTES
        || path.bytes().any(|byte| {
            !(byte.is_ascii_alphanumeric()
                || matches!(byte, b'/' | b'.' | b':' | b'?' | b'=' | b'_' | b'-'))
        })
    {
        return Err(DockerHttpObserverError::Request);
    }
    let relative = path
        .strip_prefix(API_PREFIX)
        .ok_or(DockerHttpObserverError::Request)?;
    if matches!(relative, "/info" | "/containers/json?all=0") {
        return Ok(());
    }
    if let Some(identity) = relative
        .strip_prefix("/containers/")
        .and_then(|value| value.strip_suffix("/json"))
    {
        return validate_hex_identity(identity);
    }
    if let Some(identity) = relative
        .strip_prefix("/images/")
        .and_then(|value| value.strip_suffix("/json"))
    {
        return validate_digest_or_hex_identity(identity);
    }
    Err(DockerHttpObserverError::Request)
}

fn validate_hex_identity(value: &str) -> Result<(), DockerHttpObserverError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(DockerHttpObserverError::Request);
    }
    Ok(())
}

fn validate_digest_or_hex_identity(value: &str) -> Result<(), DockerHttpObserverError> {
    let value = value.strip_prefix("sha256:").unwrap_or(value);
    validate_hex_identity(value)
}

fn parse_response(response: &[u8]) -> Result<Vec<u8>, DockerHttpObserverError> {
    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or(DockerHttpObserverError::Response)?;
    let headers = std::str::from_utf8(&response[..header_end])
        .map_err(|_| DockerHttpObserverError::Response)?;
    let mut lines = headers.split("\r\n");
    if lines.next() != Some("HTTP/1.1 200 OK") {
        return Err(DockerHttpObserverError::Response);
    }
    let mut content_length = None;
    let mut chunked = false;
    for line in lines {
        let (name, value) = line
            .split_once(':')
            .ok_or(DockerHttpObserverError::Response)?;
        if name.eq_ignore_ascii_case("content-length") {
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| DockerHttpObserverError::Response)?,
            );
        } else if name.eq_ignore_ascii_case("transfer-encoding") {
            if !value.trim().eq_ignore_ascii_case("chunked") {
                return Err(DockerHttpObserverError::Response);
            }
            chunked = true;
        }
    }
    if chunked == content_length.is_some() {
        return Err(DockerHttpObserverError::Response);
    }
    let body = &response[header_end + 4..];
    if chunked {
        decode_chunks(body)
    } else if content_length == Some(body.len()) {
        Ok(body.to_vec())
    } else {
        Err(DockerHttpObserverError::Response)
    }
}

fn decode_chunks(mut bytes: &[u8]) -> Result<Vec<u8>, DockerHttpObserverError> {
    let mut output = Vec::new();
    loop {
        let line_end = bytes
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or(DockerHttpObserverError::Response)?;
        let size = std::str::from_utf8(&bytes[..line_end])
            .ok()
            .and_then(|value| usize::from_str_radix(value, 16).ok())
            .ok_or(DockerHttpObserverError::Response)?;
        bytes = &bytes[line_end + 2..];
        if size == 0 {
            return if bytes == b"\r\n" {
                Ok(output)
            } else {
                Err(DockerHttpObserverError::Response)
            };
        }
        if bytes.len() < size + 2 || &bytes[size..size + 2] != b"\r\n" {
            return Err(DockerHttpObserverError::Response);
        }
        output.extend_from_slice(&bytes[..size]);
        if output.len() as u64 > MAX_RESPONSE_BYTES {
            return Err(DockerHttpObserverError::Response);
        }
        bytes = &bytes[size + 2..];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_paths_accept_only_fixed_read_only_api_grammar() {
        assert!(validate_request_path("/v1.47/info").is_ok());
        assert!(validate_request_path("/v1.47/containers/json?all=0").is_ok());
        for path in [
            "/containers/json",
            "/v1.47/containers/x/start",
            "/v1.47/info HTTP/1.1",
            "/v1.47/info%0aPOST",
            "/v1.47/info#fragment",
        ] {
            assert_eq!(
                validate_request_path(path),
                Err(DockerHttpObserverError::Request)
            );
        }
    }

    #[test]
    fn exact_length_and_chunked_responses_are_bounded() {
        let length = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}";
        assert_eq!(parse_response(length), Ok(b"{}".to_vec()));
        let chunks = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2\r\n{}\r\n0\r\n\r\n";
        assert_eq!(parse_response(chunks), Ok(b"{}".to_vec()));
        for invalid in [
            b"HTTP/1.1 201 Created\r\nContent-Length: 2\r\n\r\n{}".as_slice(),
            b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\n{}".as_slice(),
            b"HTTP/1.1 200 OK\r\n\r\n{}".as_slice(),
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: gzip\r\n\r\n{}".as_slice(),
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n3\r\n{}\r\n0\r\n\r\n".as_slice(),
        ] {
            assert_eq!(
                parse_response(invalid),
                Err(DockerHttpObserverError::Response)
            );
        }
    }

    #[test]
    fn docker_records_require_all_security_relevant_fields() {
        let fixture = r#"{
          "Id":"a","Image":"sha256:b","State":{"Running":true,"Pid":42},
          "Config":{"Image":"sha256:c","User":"modelrunner","Env":[],"Labels":{}},
          "HostConfig":{"NetworkMode":"none","Privileged":false,"ReadonlyRootfs":true,"CapAdd":null,"SecurityOpt":["no-new-privileges"],"Memory":1,"MemorySwap":1,"NanoCpus":1,"PidsLimit":1,"PortBindings":null,"Tmpfs":{"/run":"rw,nosuid,nodev,noexec,size=64m"}},
          "Mounts":[]
        }"#;
        assert!(serde_json::from_str::<DockerContainerInspect>(fixture).is_ok());
        assert!(
            serde_json::from_str::<DockerContainerInspect>(&fixture.replace("\"Pid\":42", ""))
                .is_err()
        );
        assert!(
            serde_json::from_str::<DockerContainerInspect>(
                &fixture.replace("\"ReadonlyRootfs\":true,", "")
            )
            .is_err()
        );
    }
}
