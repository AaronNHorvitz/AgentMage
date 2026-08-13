//! Bounded Linux Secret Service adapter with pipe-only secret transfer.

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io::{Read, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use agentmage_kernel_contracts::{GrantOperation, OperationOutcome, StateChange};
use agentmage_kernel_engine::authority_transaction::{
    EffectAuthorization, EffectDriver, EffectLaunch, EffectResult,
};
use agentmage_kernel_engine::operational_store::{
    OperationalStoreKeyError, OperationalStoreKeyLifecycle, OperationalStoreKeyProvider,
};
use rustix::fd::OwnedFd;
use rustix::fs::{FileType, Mode, OFlags, fstat, open};
use rustix::io::pread;
use rustix::process::getuid;
use rustix::rand::{GetRandomFlags, getrandom};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, Zeroizing};

const MAX_SECRET_BYTES: usize = 16 * 1024;
const MAX_DIAGNOSTIC_BYTES: usize = 64 * 1024;
const HASH_BUFFER_BYTES: usize = 64 * 1024;
const PROCESS_TIMEOUT: Duration = Duration::from_secs(10);
const FIXED_LABEL: &str = "AgentMage local credential";
const OPERATIONAL_STORE_KEY_PURPOSE: &str = "operational-store-key-v1";

/// Stable content-free Secret Service failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxSecretServiceErrorKind {
    /// A profile or purpose identifier is invalid.
    InvalidKey,
    /// A candidate secret is empty, oversized, or unsupported.
    InvalidValue,
    /// The trusted Secret Service client manifest is invalid.
    InvalidManifest,
    /// The Secret Service or its session bus is unavailable.
    ServiceUnavailable,
    /// The bounded client process did not complete in time.
    TimedOut,
    /// No credential matched the exact fixed attributes.
    NotFound,
    /// Explicit first-install provisioning found an existing key.
    AlreadyProvisioned,
    /// A newly stored operational key could not be retrieved exactly.
    VerificationFailed,
    /// The client returned output outside the closed protocol.
    ProtocolViolation,
    /// Client output exceeded the hard bound.
    OutputLimitExceeded,
}

impl LinuxSecretServiceErrorKind {
    /// Returns the stable machine-readable failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidKey => "linux.secret.key.invalid",
            Self::InvalidValue => "linux.secret.value.invalid",
            Self::InvalidManifest => "linux.secret.manifest.invalid",
            Self::ServiceUnavailable => "linux.secret.service.unavailable",
            Self::TimedOut => "linux.secret.service.timeout",
            Self::NotFound => "linux.secret.not-found",
            Self::AlreadyProvisioned => "linux.secret.operational-key.already-provisioned",
            Self::VerificationFailed => "linux.secret.operational-key.verification-failed",
            Self::ProtocolViolation => "linux.secret.protocol.invalid",
            Self::OutputLimitExceeded => "linux.secret.output.exceeded",
        }
    }
}

/// Redacted Secret Service error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxSecretServiceError {
    kind: LinuxSecretServiceErrorKind,
}

impl LinuxSecretServiceError {
    /// Returns the stable error category.
    #[must_use]
    pub const fn kind(self) -> LinuxSecretServiceErrorKind {
        self.kind
    }
}

impl fmt::Display for LinuxSecretServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.kind.code())
    }
}

impl std::error::Error for LinuxSecretServiceError {}

/// Exact non-secret metadata that identifies one local credential.
#[derive(Clone, PartialEq, Eq)]
pub struct LinuxSecretKey {
    profile_id: String,
    purpose: String,
}

impl LinuxSecretKey {
    /// Creates a key from bounded ASCII identifiers that are safe in process metadata.
    pub fn new(
        profile_id: impl Into<String>,
        purpose: impl Into<String>,
    ) -> Result<Self, LinuxSecretServiceError> {
        let profile_id = profile_id.into();
        let purpose = purpose.into();
        if !valid_identifier(&profile_id) || !valid_identifier(&purpose) {
            return Err(error(LinuxSecretServiceErrorKind::InvalidKey));
        }
        Ok(Self {
            profile_id,
            purpose,
        })
    }

    fn attributes(&self) -> [OsString; 6] {
        [
            "agentmage-schema".into(),
            "1".into(),
            "agentmage-profile".into(),
            self.profile_id.clone().into(),
            "agentmage-purpose".into(),
            self.purpose.clone().into(),
        ]
    }
}

impl fmt::Debug for LinuxSecretKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxSecretKey")
            .field("profile_id_bytes", &self.profile_id.len())
            .field("purpose_bytes", &self.purpose.len())
            .finish_non_exhaustive()
    }
}

/// Secret bytes that are zeroized when dropped and never implement serialization or display.
pub struct LinuxSecretValue {
    bytes: Vec<u8>,
}

impl LinuxSecretValue {
    /// Admits one bounded UTF-8 secret without NUL or line separators.
    pub fn new(bytes: impl Into<Vec<u8>>) -> Result<Self, LinuxSecretServiceError> {
        let mut bytes = bytes.into();
        if bytes.is_empty()
            || bytes.len() > MAX_SECRET_BYTES
            || std::str::from_utf8(&bytes).is_err()
            || bytes.iter().any(|byte| matches!(byte, 0 | b'\r' | b'\n'))
        {
            bytes.zeroize();
            return Err(error(LinuxSecretServiceErrorKind::InvalidValue));
        }
        Ok(Self { bytes })
    }

    /// Exposes the secret only for the duration of a caller-provided closure.
    pub fn with_exposed<T>(&self, operation: impl FnOnce(&[u8]) -> T) -> T {
        operation(&self.bytes)
    }

    /// Returns the secret byte count without exposing its content.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Reports whether this value contains no secret bytes.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

impl fmt::Debug for LinuxSecretValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxSecretValue")
            .field("secret_bytes", &"[REDACTED]")
            .finish()
    }
}

impl Drop for LinuxSecretValue {
    fn drop(&mut self) {
        self.bytes.zeroize();
    }
}

/// Secret Service operation recorded without key or value material.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxSecretOperation {
    /// Service availability probe with a fresh no-match attribute.
    Probe,
    /// Store one exact credential.
    Store,
    /// Retrieve one exact credential.
    Lookup,
    /// Remove every item matching the exact AgentMage attributes.
    Clear,
}

/// Content-free receipt for one completed Secret Service operation.
#[derive(Clone, PartialEq, Eq)]
pub struct LinuxSecretReceipt {
    operation: LinuxSecretOperation,
    stderr_sha256: [u8; 32],
    stderr_bytes: usize,
}

impl LinuxSecretReceipt {
    /// Returns the completed operation.
    #[must_use]
    pub const fn operation(&self) -> LinuxSecretOperation {
        self.operation
    }

    /// Returns the digest of hidden client diagnostics.
    #[must_use]
    pub const fn stderr_sha256(&self) -> &[u8; 32] {
        &self.stderr_sha256
    }

    /// Returns the hidden diagnostic byte count.
    #[must_use]
    pub const fn stderr_bytes(&self) -> usize {
        self.stderr_bytes
    }
}

impl fmt::Debug for LinuxSecretReceipt {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxSecretReceipt")
            .field("operation", &self.operation)
            .field("stderr_bytes", &self.stderr_bytes)
            .finish_non_exhaustive()
    }
}

/// Verified root-owned Secret Service client identity.
pub struct LinuxSecretServiceManifest {
    client_path: PathBuf,
    client_sha256: [u8; 32],
}

impl LinuxSecretServiceManifest {
    /// Verifies an immutable root-owned `secret-tool` client path.
    pub fn verify(client_path: impl AsRef<Path>) -> Result<Self, LinuxSecretServiceError> {
        let client_path = client_path.as_ref();
        let client_sha256 = verify_client(client_path)?;
        Ok(Self {
            client_path: client_path.to_path_buf(),
            client_sha256,
        })
    }
}

impl fmt::Debug for LinuxSecretServiceManifest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxSecretServiceManifest")
            .field("client_sha256", &hex_digest(&self.client_sha256))
            .finish_non_exhaustive()
    }
}

/// Narrow local credential adapter backed by `org.freedesktop.secrets`.
#[derive(Debug)]
pub struct LinuxSecretService {
    manifest: LinuxSecretServiceManifest,
    timeout: Duration,
}

impl LinuxSecretService {
    /// Creates the adapter without contacting the session bus.
    #[must_use]
    pub const fn new(manifest: LinuxSecretServiceManifest) -> Self {
        Self {
            manifest,
            timeout: PROCESS_TIMEOUT,
        }
    }

    /// Proves the session service responds to a fresh no-match query.
    pub(crate) fn probe(&self) -> Result<LinuxSecretReceipt, LinuxSecretServiceError> {
        let mut random = [0_u8; 16];
        getrandom(&mut random, GetRandomFlags::empty())
            .map_err(|_| error(LinuxSecretServiceErrorKind::ServiceUnavailable))?;
        let arguments = [
            OsString::from("search"),
            OsString::from("--all"),
            OsString::from("agentmage-probe"),
            OsString::from(hex_digest(&random)),
        ];
        let mut output = self.execute(&arguments, None)?;
        if !output.status.success() || !output.stdout.is_empty() {
            output.stdout.zeroize();
            return Err(error(LinuxSecretServiceErrorKind::ServiceUnavailable));
        }
        output.stdout.zeroize();
        Ok(output.receipt(LinuxSecretOperation::Probe))
    }

    /// Stores one credential through standard input and consumes its in-memory value.
    fn store(
        &self,
        key: &LinuxSecretKey,
        value: LinuxSecretValue,
    ) -> Result<LinuxSecretReceipt, LinuxSecretServiceError> {
        let mut arguments = vec![
            OsString::from("store"),
            OsString::from(format!("--label={FIXED_LABEL}")),
        ];
        arguments.extend(key.attributes());
        let mut output = self.execute(&arguments, Some(&value.bytes))?;
        if !output.status.success() {
            output.stdout.zeroize();
            return Err(error(LinuxSecretServiceErrorKind::ServiceUnavailable));
        }
        if !output.stdout.is_empty() {
            output.stdout.zeroize();
            return Err(error(LinuxSecretServiceErrorKind::ProtocolViolation));
        }
        Ok(output.receipt(LinuxSecretOperation::Store))
    }

    /// Retrieves one exact credential without exposing it in a receipt or diagnostic.
    fn lookup(
        &self,
        key: &LinuxSecretKey,
    ) -> Result<(LinuxSecretValue, LinuxSecretReceipt), LinuxSecretServiceError> {
        let mut arguments = vec![OsString::from("lookup")];
        arguments.extend(key.attributes());
        let mut output = self.execute(&arguments, None)?;
        if !output.status.success() {
            let service_available = self.probe().is_ok();
            output.stdout.zeroize();
            return Err(error(
                if output.status.code() == Some(1) && service_available {
                    LinuxSecretServiceErrorKind::NotFound
                } else {
                    LinuxSecretServiceErrorKind::ServiceUnavailable
                },
            ));
        }
        if output.stdout.last() == Some(&b'\n') {
            output.stdout.pop();
        }
        let value = LinuxSecretValue::new(std::mem::take(&mut output.stdout))?;
        let receipt = output.receipt(LinuxSecretOperation::Lookup);
        Ok((value, receipt))
    }

    /// Removes every item matching the exact fixed AgentMage attributes.
    fn clear(&self, key: &LinuxSecretKey) -> Result<LinuxSecretReceipt, LinuxSecretServiceError> {
        let mut arguments = vec![OsString::from("clear")];
        arguments.extend(key.attributes());
        let mut output = self.execute(&arguments, None)?;
        if !output.status.success() {
            output.stdout.zeroize();
            return Err(error(LinuxSecretServiceErrorKind::ServiceUnavailable));
        }
        if !output.stdout.is_empty() {
            output.stdout.zeroize();
            return Err(error(LinuxSecretServiceErrorKind::ProtocolViolation));
        }
        Ok(output.receipt(LinuxSecretOperation::Clear))
    }

    fn execute(
        &self,
        arguments: &[OsString],
        input: Option<&[u8]>,
    ) -> Result<SecretProcessOutput, LinuxSecretServiceError> {
        if verify_client(&self.manifest.client_path)? != self.manifest.client_sha256 {
            return Err(error(LinuxSecretServiceErrorKind::InvalidManifest));
        }
        let mut command = self.command(arguments, input.is_some());
        let mut child = command
            .spawn()
            .map_err(|_| error(LinuxSecretServiceErrorKind::ServiceUnavailable))?;
        let Some(stdout) = child.stdout.take() else {
            terminate(&mut child);
            return Err(error(LinuxSecretServiceErrorKind::ServiceUnavailable));
        };
        let Some(stderr) = child.stderr.take() else {
            terminate(&mut child);
            return Err(error(LinuxSecretServiceErrorKind::ServiceUnavailable));
        };
        let stdout_reader = thread::Builder::new()
            .name("agentmage-secret-stdout".to_owned())
            .spawn(move || read_bounded(stdout, MAX_SECRET_BYTES + 1, true))
            .map_err(|_| {
                terminate(&mut child);
                error(LinuxSecretServiceErrorKind::ServiceUnavailable)
            })?;
        let stderr_reader = match thread::Builder::new()
            .name("agentmage-secret-stderr".to_owned())
            .spawn(move || read_bounded(stderr, MAX_DIAGNOSTIC_BYTES, false))
        {
            Ok(reader) => reader,
            Err(_) => {
                terminate(&mut child);
                let _ = stdout_reader.join();
                return Err(error(LinuxSecretServiceErrorKind::ServiceUnavailable));
            }
        };
        let mut write_failed = false;
        if let Some(secret) = input {
            let write_result = child.stdin.take().map(|mut stdin| stdin.write_all(secret));
            if !matches!(write_result, Some(Ok(()))) {
                terminate(&mut child);
                write_failed = true;
            }
        }
        drop(child.stdin.take());
        let status = if write_failed {
            Err(error(LinuxSecretServiceErrorKind::ServiceUnavailable))
        } else {
            wait_bounded(&mut child, self.timeout)
        };
        let stdout_result = stdout_reader.join();
        let stderr_result = stderr_reader.join();
        let mut stdout =
            stdout_result.map_err(|_| error(LinuxSecretServiceErrorKind::ServiceUnavailable))??;
        let stderr =
            stderr_result.map_err(|_| error(LinuxSecretServiceErrorKind::ServiceUnavailable))??;
        let status = status?;
        if stdout.exceeded || stderr.exceeded {
            return Err(error(LinuxSecretServiceErrorKind::OutputLimitExceeded));
        }
        Ok(SecretProcessOutput {
            status,
            stdout: std::mem::take(&mut stdout.retained),
            stderr_sha256: stderr.sha256,
            stderr_bytes: stderr.total,
        })
    }

    fn command(&self, arguments: &[OsString], input_expected: bool) -> Command {
        let runtime_directory = format!("/run/user/{}", getuid().as_raw());
        let session_bus = format!("unix:path={runtime_directory}/bus");
        let mut command = Command::new(&self.manifest.client_path);
        command
            .env_clear()
            .env("XDG_RUNTIME_DIR", runtime_directory)
            .env("DBUS_SESSION_BUS_ADDRESS", session_bus)
            .args(arguments)
            .stdin(if input_expected {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    }
}

pub(crate) fn operational_store_key_exists(
    service: &LinuxSecretService,
    profile_id: impl Into<String>,
) -> Result<bool, LinuxSecretServiceError> {
    let key = LinuxSecretKey::new(profile_id, OPERATIONAL_STORE_KEY_PURPOSE)?;
    match service.lookup(&key) {
        Ok((value, _)) => {
            decode_operational_store_key_with_service_error(&value)?;
            Ok(true)
        }
        Err(failure) if failure.kind() == LinuxSecretServiceErrorKind::NotFound => Ok(false),
        Err(failure) => Err(failure),
    }
}

pub(crate) fn provision_operational_store_key(
    service: &LinuxSecretService,
    profile_id: impl Into<String>,
) -> Result<LinuxSecretReceipt, LinuxSecretServiceError> {
    let key = LinuxSecretKey::new(profile_id, OPERATIONAL_STORE_KEY_PURPOSE)?;
    match service.lookup(&key) {
        Ok((value, _)) => {
            decode_operational_store_key_with_service_error(&value)?;
            return Err(error(LinuxSecretServiceErrorKind::AlreadyProvisioned));
        }
        Err(failure) if failure.kind() == LinuxSecretServiceErrorKind::NotFound => {}
        Err(failure) => return Err(failure),
    }

    let mut raw = Zeroizing::new([0_u8; 32]);
    let generated = getrandom(&mut raw[..], GetRandomFlags::empty())
        .map_err(|_| error(LinuxSecretServiceErrorKind::ServiceUnavailable))?;
    if generated != raw.len() {
        return Err(error(LinuxSecretServiceErrorKind::ServiceUnavailable));
    }
    let encoded = Zeroizing::new(hex_digest(raw.as_slice()).into_bytes());
    let receipt = service.store(&key, LinuxSecretValue::new(encoded.to_vec())?)?;
    let (retrieved, _) = service.lookup(&key)?;
    let decoded = decode_operational_store_key_with_service_error(&retrieved)?;
    if !constant_time_equal(raw.as_slice(), decoded.as_slice()) {
        return Err(error(LinuxSecretServiceErrorKind::VerificationFailed));
    }
    Ok(receipt)
}

/// Fixed-purpose Secret Service key source for the encrypted operational store.
///
/// This startup boundary can only look up the one operational-store key for an
/// exact profile. It cannot list, select, mutate, or return general credentials.
pub struct LinuxOperationalStoreKeyProvider {
    service: LinuxSecretService,
    key: LinuxSecretKey,
}

impl LinuxOperationalStoreKeyProvider {
    /// Binds one profile to the fixed version-1 operational-store key identity.
    pub fn new(
        service: LinuxSecretService,
        profile_id: impl Into<String>,
    ) -> Result<Self, LinuxSecretServiceError> {
        Ok(Self {
            service,
            key: LinuxSecretKey::new(profile_id, OPERATIONAL_STORE_KEY_PURPOSE)?,
        })
    }
}

impl fmt::Debug for LinuxOperationalStoreKeyProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxOperationalStoreKeyProvider")
            .field("key", &self.key)
            .finish_non_exhaustive()
    }
}

impl OperationalStoreKeyProvider for LinuxOperationalStoreKeyProvider {
    fn with_key<T>(
        &mut self,
        operation: impl FnOnce(&[u8]) -> T,
    ) -> Result<T, OperationalStoreKeyError> {
        let (value, _) = self
            .service
            .lookup(&self.key)
            .map_err(|_| OperationalStoreKeyError::Unavailable)?;
        value.with_exposed(|encoded| with_decoded_operational_store_key(encoded, operation))
    }
}

impl OperationalStoreKeyLifecycle for LinuxOperationalStoreKeyProvider {
    fn destroy_key_and_verify_absent(&mut self) -> Result<(), OperationalStoreKeyError> {
        self.service
            .clear(&self.key)
            .map_err(|_| OperationalStoreKeyError::Unavailable)?;
        match self.service.lookup(&self.key) {
            Err(error) if error.kind() == LinuxSecretServiceErrorKind::NotFound => Ok(()),
            Ok((mut value, _)) => {
                value.bytes.zeroize();
                Err(OperationalStoreKeyError::Unavailable)
            }
            Err(_) => Err(OperationalStoreKeyError::Unavailable),
        }
    }
}

fn with_decoded_operational_store_key<T>(
    encoded: &[u8],
    operation: impl FnOnce(&[u8]) -> T,
) -> Result<T, OperationalStoreKeyError> {
    let decoded = decode_operational_store_key(encoded)?;
    Ok(operation(decoded.as_slice()))
}

fn decode_operational_store_key(
    encoded: &[u8],
) -> Result<Zeroizing<[u8; 32]>, OperationalStoreKeyError> {
    if encoded.len() != 64 {
        return Err(OperationalStoreKeyError::Unavailable);
    }
    let mut decoded = Zeroizing::new([0_u8; 32]);
    for (index, pair) in encoded.chunks_exact(2).enumerate() {
        let high = decode_hex(pair[0]).ok_or(OperationalStoreKeyError::Unavailable)?;
        let low = decode_hex(pair[1]).ok_or(OperationalStoreKeyError::Unavailable)?;
        decoded[index] = (high << 4) | low;
    }
    Ok(decoded)
}

fn decode_operational_store_key_with_service_error(
    value: &LinuxSecretValue,
) -> Result<Zeroizing<[u8; 32]>, LinuxSecretServiceError> {
    value
        .with_exposed(decode_operational_store_key)
        .map_err(|_| error(LinuxSecretServiceErrorKind::VerificationFailed))
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

const fn decode_hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

/// One exact Secret Service operation proposed for mediated execution.
pub enum LinuxSecretEffectRequest {
    /// Verify that the local Secret Service is available.
    Probe,
    /// Store one exact credential.
    Store {
        /// Exact non-secret credential identity.
        key: LinuxSecretKey,
        /// Secret value consumed by the operation.
        value: LinuxSecretValue,
    },
    /// Retrieve one exact credential.
    Lookup {
        /// Exact non-secret credential identity.
        key: LinuxSecretKey,
    },
    /// Remove one exact credential identity.
    Clear {
        /// Exact non-secret credential identity.
        key: LinuxSecretKey,
    },
}

impl fmt::Debug for LinuxSecretEffectRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let operation = match self {
            Self::Probe => LinuxSecretOperation::Probe,
            Self::Store { .. } => LinuxSecretOperation::Store,
            Self::Lookup { .. } => LinuxSecretOperation::Lookup,
            Self::Clear { .. } => LinuxSecretOperation::Clear,
        };
        formatter
            .debug_struct("LinuxSecretEffectRequest")
            .field("operation", &operation)
            .finish_non_exhaustive()
    }
}

/// Typed Secret Service output retained only after mediated execution.
pub enum LinuxSecretEffectOutput {
    /// Content-free metadata for a probe, store, or clear operation.
    Receipt(LinuxSecretReceipt),
    /// Retrieved secret and its content-free operation metadata.
    Lookup {
        /// Retrieved secret value, zeroized when dropped.
        value: LinuxSecretValue,
        /// Content-free lookup receipt.
        receipt: LinuxSecretReceipt,
    },
}

impl LinuxSecretEffectOutput {
    fn receipt(&self) -> &LinuxSecretReceipt {
        match self {
            Self::Receipt(receipt) | Self::Lookup { receipt, .. } => receipt,
        }
    }
}

impl fmt::Debug for LinuxSecretEffectOutput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxSecretEffectOutput")
            .field("operation", &self.receipt().operation())
            .finish_non_exhaustive()
    }
}

/// Secret Service driver callable only with a kernel-issued authorization.
///
/// The underlying service has no public lookup or mutation methods:
///
/// ```compile_fail
/// use agentmage_platform_linux::LinuxSecretService;
/// fn bypass(service: &LinuxSecretService) {
///     let _ = service.probe();
/// }
/// ```
pub struct LinuxSecretEffectDriver {
    service: LinuxSecretService,
    request: Option<LinuxSecretEffectRequest>,
    output: Option<LinuxSecretEffectOutput>,
    error: Option<LinuxSecretServiceError>,
}

impl LinuxSecretEffectDriver {
    /// Creates an inert driver without contacting the Secret Service.
    #[must_use]
    pub const fn new(service: LinuxSecretService, request: LinuxSecretEffectRequest) -> Self {
        Self {
            service,
            request: Some(request),
            output: None,
            error: None,
        }
    }

    /// Takes the output after the authority transaction closes.
    pub fn take_output(&mut self) -> Option<LinuxSecretEffectOutput> {
        self.output.take()
    }

    /// Takes the redacted service error after a failed mediated attempt.
    pub fn take_error(&mut self) -> Option<LinuxSecretServiceError> {
        self.error.take()
    }
}

impl fmt::Debug for LinuxSecretEffectDriver {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxSecretEffectDriver")
            .field("has_request", &self.request.is_some())
            .field("has_output", &self.output.is_some())
            .field("has_error", &self.error.is_some())
            .finish_non_exhaustive()
    }
}

impl EffectDriver for LinuxSecretEffectDriver {
    fn execute(&mut self, authorization: EffectAuthorization<'_>) -> EffectLaunch {
        if authorization.operation().operation() != GrantOperation::CredentialAccess {
            return EffectLaunch::failed();
        }
        let Some(request) = self.request.take() else {
            return EffectLaunch::failed();
        };
        let result = match request {
            LinuxSecretEffectRequest::Probe => {
                self.service.probe().map(LinuxSecretEffectOutput::Receipt)
            }
            LinuxSecretEffectRequest::Store { key, value } => self
                .service
                .store(&key, value)
                .map(LinuxSecretEffectOutput::Receipt),
            LinuxSecretEffectRequest::Lookup { key } => self
                .service
                .lookup(&key)
                .map(|(value, receipt)| LinuxSecretEffectOutput::Lookup { value, receipt }),
            LinuxSecretEffectRequest::Clear { key } => self
                .service
                .clear(&key)
                .map(LinuxSecretEffectOutput::Receipt),
        };
        match result {
            Ok(output) => {
                let receipt = output.receipt();
                let mut material = Vec::with_capacity(48);
                material.extend_from_slice(secret_operation_code(receipt.operation()).as_bytes());
                material.extend_from_slice(receipt.stderr_sha256());
                material.extend_from_slice(&receipt.stderr_bytes().to_be_bytes());
                let state_change = match receipt.operation() {
                    LinuxSecretOperation::Probe | LinuxSecretOperation::Lookup => {
                        StateChange::NotChanged
                    }
                    LinuxSecretOperation::Store | LinuxSecretOperation::Clear => {
                        StateChange::Changed
                    }
                };
                let effect_result = EffectResult::from_redacted_material(
                    OperationOutcome::Succeeded,
                    &material,
                    state_change,
                );
                self.output = Some(output);
                EffectLaunch::completed(effect_result)
            }
            Err(error) => {
                let effect_result = EffectResult::from_redacted_material(
                    OperationOutcome::Failed,
                    error.kind().code().as_bytes(),
                    StateChange::Uncertain,
                );
                self.error = Some(error);
                EffectLaunch::completed(effect_result)
            }
        }
    }
}

const fn secret_operation_code(operation: LinuxSecretOperation) -> &'static str {
    match operation {
        LinuxSecretOperation::Probe => "probe",
        LinuxSecretOperation::Store => "store",
        LinuxSecretOperation::Lookup => "lookup",
        LinuxSecretOperation::Clear => "clear",
    }
}

struct SecretProcessOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr_sha256: [u8; 32],
    stderr_bytes: usize,
}

impl SecretProcessOutput {
    fn receipt(&self, operation: LinuxSecretOperation) -> LinuxSecretReceipt {
        LinuxSecretReceipt {
            operation,
            stderr_sha256: self.stderr_sha256,
            stderr_bytes: self.stderr_bytes,
        }
    }
}

impl Drop for SecretProcessOutput {
    fn drop(&mut self) {
        self.stdout.zeroize();
    }
}

struct BoundedRead {
    retained: Vec<u8>,
    sha256: [u8; 32],
    total: usize,
    exceeded: bool,
}

impl Drop for BoundedRead {
    fn drop(&mut self) {
        self.retained.zeroize();
    }
}

fn read_bounded(
    mut input: impl Read,
    limit: usize,
    sensitive: bool,
) -> Result<BoundedRead, LinuxSecretServiceError> {
    let mut retained = Zeroizing::new(Vec::with_capacity(limit.min(HASH_BUFFER_BYTES)));
    let mut digest = Sha256::new();
    let mut total = 0_usize;
    let mut buffer = Zeroizing::new([0_u8; HASH_BUFFER_BYTES]);
    loop {
        let count = input
            .read(&mut buffer[..])
            .map_err(|_| error(LinuxSecretServiceErrorKind::ServiceUnavailable))?;
        if count == 0 {
            break;
        }
        if !sensitive {
            digest.update(&buffer[..count]);
        }
        total = total.saturating_add(count);
        if retained.len() < limit {
            let remaining = limit - retained.len();
            retained.extend_from_slice(&buffer[..count.min(remaining)]);
        }
        if sensitive {
            buffer[..count].zeroize();
        }
    }
    Ok(BoundedRead {
        retained: std::mem::take(&mut *retained),
        sha256: digest.finalize().into(),
        total,
        exceeded: total > limit,
    })
}

fn wait_bounded(
    child: &mut Child,
    timeout: Duration,
) -> Result<ExitStatus, LinuxSecretServiceError> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|_| error(LinuxSecretServiceErrorKind::ServiceUnavailable))?
        {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            terminate(child);
            return Err(error(LinuxSecretServiceErrorKind::TimedOut));
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn terminate(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn verify_client(path: &Path) -> Result<[u8; 32], LinuxSecretServiceError> {
    if !path.is_absolute() || path.file_name() != Some(OsStr::new("secret-tool")) {
        return Err(error(LinuxSecretServiceErrorKind::InvalidManifest));
    }
    let mut current = PathBuf::from("/");
    for component in path.components().skip(1) {
        let Component::Normal(component) = component else {
            return Err(error(LinuxSecretServiceErrorKind::InvalidManifest));
        };
        current.push(component);
        let metadata = std::fs::symlink_metadata(&current)
            .map_err(|_| error(LinuxSecretServiceErrorKind::InvalidManifest))?;
        if metadata.uid() != 0 || metadata.mode() & 0o022 != 0 || metadata.file_type().is_symlink()
        {
            return Err(error(LinuxSecretServiceErrorKind::InvalidManifest));
        }
    }
    let descriptor = open(
        path,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
        Mode::empty(),
    )
    .map_err(|_| error(LinuxSecretServiceErrorKind::InvalidManifest))?;
    let metadata =
        fstat(&descriptor).map_err(|_| error(LinuxSecretServiceErrorKind::InvalidManifest))?;
    if FileType::from_raw_mode(metadata.st_mode) != FileType::RegularFile
        || metadata.st_uid != 0
        || metadata.st_mode & 0o022 != 0
        || metadata.st_mode & 0o111 == 0
    {
        return Err(error(LinuxSecretServiceErrorKind::InvalidManifest));
    }
    hash_descriptor(&descriptor, metadata.st_size)
}

fn hash_descriptor(descriptor: &OwnedFd, size: i64) -> Result<[u8; 32], LinuxSecretServiceError> {
    let size =
        u64::try_from(size).map_err(|_| error(LinuxSecretServiceErrorKind::InvalidManifest))?;
    if size > 64 * 1024 * 1024 {
        return Err(error(LinuxSecretServiceErrorKind::InvalidManifest));
    }
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; HASH_BUFFER_BYTES];
    let mut offset = 0_u64;
    while offset < size {
        let requested = usize::try_from((size - offset).min(HASH_BUFFER_BYTES as u64))
            .map_err(|_| error(LinuxSecretServiceErrorKind::InvalidManifest))?;
        let count = pread(descriptor, &mut buffer[..requested], offset)
            .map_err(|_| error(LinuxSecretServiceErrorKind::InvalidManifest))?;
        if count == 0 {
            return Err(error(LinuxSecretServiceErrorKind::InvalidManifest));
        }
        digest.update(&buffer[..count]);
        offset = offset
            .checked_add(count as u64)
            .ok_or_else(|| error(LinuxSecretServiceErrorKind::InvalidManifest))?;
    }
    buffer.zeroize();
    Ok(digest.finalize().into())
}

fn valid_identifier(candidate: &str) -> bool {
    !candidate.is_empty()
        && candidate.len() <= 64
        && candidate
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

const fn error(kind: LinuxSecretServiceErrorKind) -> LinuxSecretServiceError {
    LinuxSecretServiceError { kind }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::ffi::{OsStr, OsString};

    use super::{
        FIXED_LABEL, LinuxSecretKey, LinuxSecretOperation, LinuxSecretService,
        LinuxSecretServiceErrorKind, LinuxSecretServiceManifest, LinuxSecretValue,
        MAX_SECRET_BYTES, OPERATIONAL_STORE_KEY_PURPOSE, decode_operational_store_key, hex_digest,
        operational_store_key_exists, provision_operational_store_key, read_bounded,
        with_decoded_operational_store_key,
    };
    use rustix::rand::{GetRandomFlags, getrandom};
    use sha2::Digest as _;

    #[test]
    fn keys_values_and_manifests_are_bounded_and_redacted() {
        for invalid in ["", "has space", "slash/value", "x&y"] {
            assert_eq!(
                LinuxSecretKey::new(invalid, "purpose")
                    .expect_err("invalid key")
                    .kind(),
                LinuxSecretServiceErrorKind::InvalidKey
            );
        }
        for invalid in [Vec::new(), vec![0], vec![b'x'; MAX_SECRET_BYTES + 1]] {
            assert_eq!(
                LinuxSecretValue::new(invalid)
                    .expect_err("invalid value")
                    .kind(),
                LinuxSecretServiceErrorKind::InvalidValue
            );
        }
        let value = LinuxSecretValue::new(b"synthetic-value".to_vec()).expect("value");
        assert_eq!(value.len(), 15);
        assert!(!format!("{value:?}").contains("synthetic"));
        assert!(
            LinuxSecretServiceManifest::verify("/bin/secret-tool")
                .expect_err("symbolic parent must fail")
                .kind()
                == LinuxSecretServiceErrorKind::InvalidManifest
        );
    }

    #[test]
    fn key_debug_and_errors_never_disclose_candidate_content() {
        let key = LinuxSecretKey::new("private-profile", "private-purpose").expect("key");
        let debug = format!("{key:?}");
        assert!(!debug.contains("private-profile"));
        assert!(!debug.contains("private-purpose"));
        let error = LinuxSecretValue::new(b"line\nbreak".to_vec()).expect_err("newline");
        assert_eq!(error.to_string(), "linux.secret.value.invalid");
    }

    #[test]
    fn sensitive_output_is_bounded_without_a_content_digest() {
        let output =
            read_bounded(std::io::Cursor::new(b"synthetic-secret"), 4, true).expect("bounded read");
        assert!(output.exceeded);
        assert_eq!(output.total, 16);
        assert_eq!(output.retained, b"synt");
        assert_eq!(output.sha256, sha2::Sha256::digest([]).as_slice());
    }

    #[test]
    fn operational_store_key_decode_is_exact_and_bounded() {
        let encoded = b"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        let decoded = decode_operational_store_key(encoded).expect("fixed key");
        assert_eq!(decoded.as_slice(), &(0_u8..32).collect::<Vec<_>>());
        assert!(decode_operational_store_key(b"too-short").is_err());
        assert!(
            decode_operational_store_key(
                b"zz0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
            )
            .is_err()
        );
    }

    #[test]
    fn invalid_operational_store_key_never_invokes_database_callback() {
        let mut invoked = false;
        let failure = with_decoded_operational_store_key(b"not-a-key", |_| invoked = true)
            .expect_err("invalid encoded key must fail");
        assert_eq!(
            failure,
            agentmage_kernel_engine::operational_store::OperationalStoreKeyError::Unavailable
        );
        assert!(!invoked);

        let encoded = b"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        with_decoded_operational_store_key(encoded, |_| invoked = true)
            .expect("exact key invokes callback");
        assert!(invoked);
    }

    #[test]
    fn secret_bytes_never_enter_process_arguments_or_environment() {
        let service = LinuxSecretService::new(
            LinuxSecretServiceManifest::verify("/usr/bin/secret-tool").expect("manifest"),
        );
        let key = LinuxSecretKey::new("profile-1", OPERATIONAL_STORE_KEY_PURPOSE).expect("key");
        let secret = "synthetic-secret-metadata-canary";
        let mut arguments = vec![
            OsString::from("store"),
            OsString::from(format!("--label={FIXED_LABEL}")),
        ];
        arguments.extend(key.attributes());
        let command = service.command(&arguments, true);

        assert_eq!(command.get_program(), OsStr::new("/usr/bin/secret-tool"));
        let argument_text = command
            .get_args()
            .map(|value| value.to_string_lossy())
            .collect::<Vec<_>>()
            .join("\0");
        assert!(!argument_text.contains(secret));
        let environment = command
            .get_envs()
            .map(|(key, value)| (key.to_os_string(), value.map(OsStr::to_os_string)))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            environment.keys().collect::<Vec<_>>(),
            [
                &OsString::from("DBUS_SESSION_BUS_ADDRESS"),
                &OsString::from("XDG_RUNTIME_DIR"),
            ]
        );
        assert!(
            environment
                .values()
                .flatten()
                .all(|value| !value.to_string_lossy().contains(secret))
        );
    }

    #[test]
    fn substituted_secret_client_digest_fails_before_service_contact() {
        let manifest =
            LinuxSecretServiceManifest::verify("/usr/bin/secret-tool").expect("manifest");
        let mut service = LinuxSecretService::new(manifest);
        service.manifest.client_sha256[0] ^= 0xff;
        assert_eq!(
            service
                .probe()
                .expect_err("substituted client identity must fail")
                .kind(),
            LinuxSecretServiceErrorKind::InvalidManifest
        );
    }

    #[test]
    #[ignore = "requires a live Secret Service session bus"]
    fn live_service_probe_returns_only_a_content_free_receipt() {
        let service = LinuxSecretService::new(
            LinuxSecretServiceManifest::verify("/usr/bin/secret-tool").expect("manifest"),
        );
        let receipt = service.probe().expect("live Secret Service probe");
        assert_eq!(receipt.operation(), LinuxSecretOperation::Probe);
        assert_eq!(receipt.stderr_bytes(), 0);
    }

    #[test]
    #[ignore = "requires an unlocked live Secret Service and writes then clears a synthetic item"]
    fn live_service_round_trip_is_exact_and_cleanup_is_verified() {
        let mut random = [0_u8; 16];
        getrandom(&mut random, GetRandomFlags::empty()).expect("random key");
        let key = LinuxSecretKey::new(hex_digest(&random), "integration-test").expect("key");
        let service = LinuxSecretService::new(
            LinuxSecretServiceManifest::verify("/usr/bin/secret-tool").expect("manifest"),
        );
        let value = LinuxSecretValue::new(b"synthetic-round-trip-value".to_vec()).expect("value");
        let store = service.store(&key, value);
        let lookup = service.lookup(&key);
        let clear = service.clear(&key);
        let store = store.expect("store");
        let (stored, lookup) = lookup.expect("lookup");
        let clear = clear.expect("clear");
        assert_eq!(store.operation(), LinuxSecretOperation::Store);
        assert_eq!(lookup.operation(), LinuxSecretOperation::Lookup);
        stored.with_exposed(|bytes| assert_eq!(bytes, b"synthetic-round-trip-value"));
        assert_eq!(clear.operation(), LinuxSecretOperation::Clear);
        assert_eq!(
            service
                .lookup(&key)
                .expect_err("cleared item must be absent")
                .kind(),
            LinuxSecretServiceErrorKind::NotFound
        );
    }

    #[test]
    #[ignore = "requires an unlocked live Secret Service and provisions then clears a synthetic operational key"]
    fn live_operational_key_provisioning_is_exact_non_overwriting_and_cleaned() {
        let mut random = [0_u8; 16];
        getrandom(&mut random, GetRandomFlags::empty()).expect("random profile");
        let profile_id = hex_digest(&random);
        let key = LinuxSecretKey::new(&profile_id, OPERATIONAL_STORE_KEY_PURPOSE).expect("key");
        let service = LinuxSecretService::new(
            LinuxSecretServiceManifest::verify("/usr/bin/secret-tool").expect("manifest"),
        );
        let receipt = provision_operational_store_key(&service, &profile_id).expect("provisions");
        let exists = operational_store_key_exists(&service, &profile_id);
        let overwrite = provision_operational_store_key(&service, &profile_id);
        let cleared = service.clear(&key);
        let absent = operational_store_key_exists(&service, &profile_id);

        assert_eq!(receipt.operation(), LinuxSecretOperation::Store);
        assert!(exists.expect("key exists"));
        assert_eq!(
            overwrite.expect_err("overwrite refuses").kind(),
            LinuxSecretServiceErrorKind::AlreadyProvisioned
        );
        cleared.expect("synthetic key clears");
        assert!(!absent.expect("key absent"));
    }
}
