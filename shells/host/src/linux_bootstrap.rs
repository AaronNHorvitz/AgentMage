//! Package-verified, authentication-only Linux host bootstrap.

use std::ffi::OsStr;
use std::fmt;
use std::fs::{self, DirBuilder};
use std::io::{self, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use agentmage_platform_linux::{
    LinuxHostIpcEndpoint, LinuxIpcAuthenticator, LinuxLaunchCredentials, LinuxPeerIdentity,
    observe_linux_process_identity, open_linux_bootstrap_ipc,
};

use crate::package_verify;

const BOOTSTRAP_SCHEMA_VERSION: u16 = 1;
const MAX_SELF_STAT_BYTES: u64 = 64 * 1024;

/// Fixed production package root.
pub const PRODUCTION_PACKAGE_ROOT: &str = "/";
/// Independently provisioned detached package-manifest signature.
pub const PRODUCTION_PACKAGE_SIGNATURE: &str = "/etc/agentmage/release/package-manifest.ed25519";
/// Independently provisioned package signing trust root.
pub const PRODUCTION_PACKAGE_PUBLIC_KEY: &str = "/etc/agentmage/trust/package-signing-ed25519.pub";

/// Stable content-free bootstrap failure class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxBootstrapError {
    /// The installed package or its detached trust inputs did not verify.
    PackageUntrusted,
    /// The exact parent process could not be observed safely.
    ParentUntrusted,
    /// The private runtime directory was absent or unsafe.
    RuntimeDenied,
    /// The private endpoint or one-use authentication material could not be created.
    TransportDenied,
    /// Direct bootstrap transfer failed.
    TransferFailed,
}

impl LinuxBootstrapError {
    /// Returns the stable redacted failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::PackageUntrusted => "agentmage.bootstrap.package_untrusted",
            Self::ParentUntrusted => "agentmage.bootstrap.parent_untrusted",
            Self::RuntimeDenied => "agentmage.bootstrap.runtime_denied",
            Self::TransportDenied => "agentmage.bootstrap.transport_denied",
            Self::TransferFailed => "agentmage.bootstrap.transfer_failed",
        }
    }
}

impl fmt::Display for LinuxBootstrapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for LinuxBootstrapError {}

/// Exact package and runtime paths consumed by one bootstrap attempt.
pub struct LinuxBootstrapPaths<'path> {
    package_root: &'path OsStr,
    signature: &'path OsStr,
    public_key: &'path OsStr,
    runtime_directory: &'path Path,
}

impl<'path> LinuxBootstrapPaths<'path> {
    /// Creates one explicit bootstrap input set.
    #[must_use]
    pub const fn new(
        package_root: &'path OsStr,
        signature: &'path OsStr,
        public_key: &'path OsStr,
        runtime_directory: &'path Path,
    ) -> Self {
        Self {
            package_root,
            signature,
            public_key,
            runtime_directory,
        }
    }
}

/// Package-verified endpoint and one-use launch material.
pub struct VerifiedLinuxBootstrap {
    endpoint: LinuxHostIpcEndpoint,
    authenticator: LinuxIpcAuthenticator,
    credentials: LinuxLaunchCredentials,
    peer: LinuxPeerIdentity,
}

impl fmt::Debug for VerifiedLinuxBootstrap {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VerifiedLinuxBootstrap")
            .field("endpoint", &"redacted")
            .field("authenticator", &self.authenticator)
            .finish_non_exhaustive()
    }
}

impl VerifiedLinuxBootstrap {
    /// Writes one bounded JSON line directly to the inherited standard-output pipe.
    ///
    /// No secret-bearing `String`, argument, environment variable, file, or log
    /// record is created by this serializer.
    pub fn write_launch_envelope(
        &self,
        output: &mut impl Write,
    ) -> Result<(), LinuxBootstrapError> {
        output
            .write_all(b"{\"schema_version\":1,\"endpoint\":\"")
            .and_then(|()| write_json_path(output, self.endpoint.path()))
            .and_then(|()| output.write_all(b"\",\"challenge\":\""))
            .and_then(|()| write_hex(output, self.credentials.challenge()))
            .and_then(|()| output.write_all(b"\",\"launch_secret\":\""))
            .and_then(|()| write_hex(output, self.credentials.launch_secret()))
            .and_then(|()| output.write_all(b"\",\"peer\":{\"uid\":"))
            .and_then(|()| write!(output, "{}", self.peer.uid()))
            .and_then(|()| output.write_all(b",\"pid\":"))
            .and_then(|()| write!(output, "{}", self.peer.pid()))
            .and_then(|()| output.write_all(b",\"start_time_ticks\":\""))
            .and_then(|()| write!(output, "{}", self.peer.start_time_ticks()))
            .and_then(|()| output.write_all(b"\",\"executable_sha256\":\""))
            .and_then(|()| write_hex(output, self.peer.executable_sha256()))
            .and_then(|()| output.write_all(b"\"}}\n"))
            .and_then(|()| output.flush())
            .map_err(|_| LinuxBootstrapError::TransferFailed)
    }

    /// Accepts and authenticates exactly the peer named in the launch envelope.
    pub fn accept(
        &self,
    ) -> Result<agentmage_platform_linux::LinuxAuthenticatedIpcSession, LinuxBootstrapError> {
        self.endpoint
            .accept(&self.authenticator)
            .map_err(|_| LinuxBootstrapError::TransportDenied)
    }
}

/// Verifies a signed package before creating any local endpoint or launch secret.
pub fn bootstrap_for_peer(
    paths: &LinuxBootstrapPaths<'_>,
    peer_pid: i32,
) -> Result<VerifiedLinuxBootstrap, LinuxBootstrapError> {
    package_verify::verify_signed_package_root(
        paths.package_root,
        paths.signature,
        paths.public_key,
    )
    .map_err(|_| LinuxBootstrapError::PackageUntrusted)?;
    let peer = observe_linux_process_identity(peer_pid)
        .map_err(|_| LinuxBootstrapError::ParentUntrusted)?;
    ensure_private_runtime_directory(paths.runtime_directory)?;
    let endpoint_path = paths.runtime_directory.join(format!(
        "host-{}-{}.sock",
        rustix::process::getpid().as_raw_pid(),
        peer.start_time_ticks()
    ));
    let endpoint = open_linux_bootstrap_ipc(&endpoint_path)
        .map_err(|_| LinuxBootstrapError::TransportDenied)?;
    let (authenticator, credentials) = LinuxIpcAuthenticator::generate(peer.clone())
        .map_err(|_| LinuxBootstrapError::TransportDenied)?;
    Ok(VerifiedLinuxBootstrap {
        endpoint,
        authenticator,
        credentials,
        peer,
    })
}

/// Returns the exact parent process from bounded Linux procfs state.
pub fn parent_process_id() -> Result<i32, LinuxBootstrapError> {
    let bytes = fs::read("/proc/self/stat").map_err(|_| LinuxBootstrapError::ParentUntrusted)?;
    if bytes.is_empty() || bytes.len() as u64 > MAX_SELF_STAT_BYTES {
        return Err(LinuxBootstrapError::ParentUntrusted);
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| LinuxBootstrapError::ParentUntrusted)?;
    let end = text
        .rfind(')')
        .ok_or(LinuxBootstrapError::ParentUntrusted)?;
    text[end + 1..]
        .split_ascii_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<i32>().ok())
        .filter(|value| *value > 1)
        .ok_or(LinuxBootstrapError::ParentUntrusted)
}

/// Returns the fixed owner-specific production runtime directory.
#[must_use]
pub fn production_runtime_directory() -> PathBuf {
    PathBuf::from(format!(
        "/run/user/{}/agentmage",
        rustix::process::getuid().as_raw()
    ))
}

fn ensure_private_runtime_directory(path: &Path) -> Result<(), LinuxBootstrapError> {
    let parent = path.parent().ok_or(LinuxBootstrapError::RuntimeDenied)?;
    let parent_metadata =
        fs::symlink_metadata(parent).map_err(|_| LinuxBootstrapError::RuntimeDenied)?;
    let owner = rustix::process::getuid().as_raw();
    if !parent_metadata.is_dir()
        || parent_metadata.uid() != owner
        || parent_metadata.mode() & 0o077 != 0
    {
        return Err(LinuxBootstrapError::RuntimeDenied);
    }
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.is_dir() || metadata.uid() != owner || metadata.mode() & 0o777 != 0o700 {
                return Err(LinuxBootstrapError::RuntimeDenied);
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            DirBuilder::new()
                .mode(0o700)
                .create(path)
                .map_err(|_| LinuxBootstrapError::RuntimeDenied)?;
            fs::set_permissions(path, fs::Permissions::from_mode(0o700))
                .map_err(|_| LinuxBootstrapError::RuntimeDenied)?;
            let metadata =
                fs::symlink_metadata(path).map_err(|_| LinuxBootstrapError::RuntimeDenied)?;
            if !metadata.is_dir() || metadata.uid() != owner || metadata.mode() & 0o777 != 0o700 {
                return Err(LinuxBootstrapError::RuntimeDenied);
            }
        }
        Err(_) => return Err(LinuxBootstrapError::RuntimeDenied),
    }
    Ok(())
}

fn write_json_path(output: &mut impl Write, path: &Path) -> io::Result<()> {
    let value = path
        .to_str()
        .ok_or_else(|| io::Error::other("non-utf8 path"))?;
    for byte in value.bytes() {
        match byte {
            b'"' | b'\\' | 0x00..=0x1f => return Err(io::Error::other("unsafe path")),
            _ => output.write_all(&[byte])?,
        }
    }
    Ok(())
}

fn write_hex(output: &mut impl Write, bytes: &[u8]) -> io::Result<()> {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    for byte in bytes {
        output.write_all(&[
            DIGITS[usize::from(byte >> 4)],
            DIGITS[usize::from(byte & 0x0f)],
        ])?;
    }
    Ok(())
}

const _: u16 = BOOTSTRAP_SCHEMA_VERSION;

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixStream;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use ed25519_dalek::{Signer, SigningKey};
    use serde_json::{Value, json};
    use sha2::{Digest, Sha256};

    use super::{LinuxBootstrapError, LinuxBootstrapPaths, bootstrap_for_peer, parent_process_id};

    const SIGNATURE_DOMAIN: &[u8] = b"agentmage.package-manifest.v2\0";
    static TEMP_ID: AtomicU64 = AtomicU64::new(1);

    #[test]
    fn verified_package_bootstrap_transfers_exact_one_use_identity_and_authenticates() {
        let fixture = signed_package_fixture();
        let runtime = fixture.root.join("runtime/agentmage");
        let paths = LinuxBootstrapPaths::new(
            fixture.root.as_os_str(),
            fixture.signature.as_os_str(),
            fixture.public_key.as_os_str(),
            &runtime,
        );
        let bootstrap = bootstrap_for_peer(&paths, std::process::id() as i32)
            .expect("verified package bootstrap");
        let mut envelope = Vec::new();
        bootstrap
            .write_launch_envelope(&mut envelope)
            .expect("direct envelope");
        assert!(envelope.len() < 1024);
        let parsed: Value = serde_json::from_slice(&envelope).expect("bounded JSON line");
        assert_eq!(parsed["schema_version"], 1);
        assert_eq!(parsed["peer"]["pid"], std::process::id());
        assert_eq!(
            parsed["peer"]["start_time_ticks"],
            bootstrap.peer.start_time_ticks().to_string()
        );
        assert_eq!(parsed["challenge"].as_str().map(str::len), Some(64));
        assert_eq!(parsed["launch_secret"].as_str().map(str::len), Some(64));

        let endpoint = bootstrap.endpoint.path().to_path_buf();
        let endpoint_after_drop = endpoint.clone();
        let frame = bootstrap.credentials.request(&bootstrap.peer).encode();
        let connector = std::thread::spawn(move || {
            let mut stream = UnixStream::connect(endpoint).expect("private endpoint");
            stream.write_all(&frame).expect("one-use handshake");
        });
        let session = bootstrap.accept().expect("authenticated exact peer");
        assert_eq!(session.peer().identity(), &bootstrap.peer);
        connector.join().expect("connector completes");
        drop(session);
        drop(bootstrap);
        assert!(!endpoint_after_drop.exists());
        fixture.cleanup();
    }

    #[test]
    fn package_failure_precedes_runtime_socket_and_launch_material() {
        let fixture = signed_package_fixture();
        fs::write(
            fixture.root.join("usr/libexec/agentmage/agentmage-host"),
            b"mutated",
        )
        .expect("mutate package");
        let runtime = fixture.root.join("runtime/agentmage");
        let paths = LinuxBootstrapPaths::new(
            fixture.root.as_os_str(),
            fixture.signature.as_os_str(),
            fixture.public_key.as_os_str(),
            &runtime,
        );
        assert_eq!(
            bootstrap_for_peer(&paths, std::process::id() as i32).unwrap_err(),
            LinuxBootstrapError::PackageUntrusted
        );
        assert!(!runtime.exists());
        fixture.cleanup();
    }

    #[test]
    fn unsafe_runtime_parent_and_wrong_peer_fail_closed() {
        let fixture = signed_package_fixture();
        let public_parent = fixture.root.join("public-runtime");
        fs::create_dir(&public_parent).expect("public parent");
        fs::set_permissions(&public_parent, fs::Permissions::from_mode(0o755))
            .expect("public mode");
        let runtime = public_parent.join("agentmage");
        let paths = LinuxBootstrapPaths::new(
            fixture.root.as_os_str(),
            fixture.signature.as_os_str(),
            fixture.public_key.as_os_str(),
            &runtime,
        );
        assert_eq!(
            bootstrap_for_peer(&paths, std::process::id() as i32).unwrap_err(),
            LinuxBootstrapError::RuntimeDenied
        );
        assert!(!runtime.exists());

        let private_runtime = fixture.root.join("runtime/agentmage");
        let private_paths = LinuxBootstrapPaths::new(
            fixture.root.as_os_str(),
            fixture.signature.as_os_str(),
            fixture.public_key.as_os_str(),
            &private_runtime,
        );
        assert_eq!(
            bootstrap_for_peer(&private_paths, i32::MAX).unwrap_err(),
            LinuxBootstrapError::ParentUntrusted
        );
        assert!(!private_runtime.exists());
        fixture.cleanup();
    }

    #[test]
    fn procfs_parent_parser_returns_the_real_parent_without_environment_input() {
        let parent = parent_process_id().expect("current parent");
        assert!(parent > 1);
        assert!(Path::new(&format!("/proc/{parent}")).is_dir());
    }

    struct PackageFixture {
        root: PathBuf,
        signature: PathBuf,
        public_key: PathBuf,
    }

    impl PackageFixture {
        fn cleanup(self) {
            fs::remove_dir_all(self.root).expect("fixture cleanup");
        }
    }

    fn signed_package_fixture() -> PackageFixture {
        let root = temp_root();
        let payloads = [
            (
                "usr/libexec/agentmage/agentmage-host",
                b"host".as_slice(),
                0o755,
            ),
            (
                "usr/share/agentmage/agentmage.vsix",
                b"vsix".as_slice(),
                0o644,
            ),
            (
                "usr/share/licenses/agentmage/LICENSE",
                b"license".as_slice(),
                0o644,
            ),
        ];
        let mut records = Vec::new();
        for (relative, bytes, mode) in payloads {
            let path = root.join(relative);
            fs::create_dir_all(path.parent().expect("payload parent")).expect("create parent");
            fs::write(&path, bytes).expect("write payload");
            fs::set_permissions(&path, fs::Permissions::from_mode(mode)).expect("payload mode");
            records.push(json!({
                "path": relative,
                "sha256": hex(&Sha256::digest(bytes)),
                "size": bytes.len(),
                "mode": mode,
            }));
        }
        let manifest = serde_json::to_vec(&json!({
            "schema_version": 2,
            "record_type": "agentmage-package-manifest",
            "status": "signed-release",
            "package_id": "agentmage-linux-test-release",
            "release_sequence": 1,
            "files": records,
        }))
        .expect("manifest");
        let manifest_path = root.join("usr/share/agentmage/package-manifest.json");
        fs::write(&manifest_path, &manifest).expect("write manifest");
        let trust = root.join("external-trust");
        fs::create_dir(&trust).expect("trust directory");
        let signing_key = SigningKey::from_bytes(&[91; 32]);
        let mut signed = Vec::with_capacity(SIGNATURE_DOMAIN.len() + manifest.len());
        signed.extend_from_slice(SIGNATURE_DOMAIN);
        signed.extend_from_slice(&manifest);
        let signature = trust.join("manifest.ed25519");
        let public_key = trust.join("signer.pub");
        fs::write(&signature, signing_key.sign(&signed).to_bytes()).expect("signature");
        fs::write(&public_key, signing_key.verifying_key().to_bytes()).expect("public key");
        fs::create_dir(root.join("runtime")).expect("runtime parent");
        fs::set_permissions(root.join("runtime"), fs::Permissions::from_mode(0o700))
            .expect("runtime mode");
        PackageFixture {
            root,
            signature,
            public_key,
        }
    }

    fn temp_root() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "agentmage-bootstrap-{}-{}",
            std::process::id(),
            TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("temp root");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).expect("temp mode");
        path
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}
