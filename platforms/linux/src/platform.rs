//! Independently admitted aggregate Linux platform composition.

use std::fmt;
use std::fs::{self, File};
use std::io::Read;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::Path;

use agentmage_kernel_contracts::{
    AdapterInstanceId, PathResolutionIntent, PlatformAdapter, PlatformArchitecture,
    PlatformCapability, PlatformCapabilityObservation, PlatformCapabilityStatus, PlatformFamily,
    PlatformPathAdapter, PlatformRuntimeIdentity, PlatformStartupError, WorkspaceAuthorizationId,
    WorkspaceId, WorkspacePath,
};
use agentmage_kernel_engine::operational_store::{
    DurableAuthorityError, DurableAuthorityRuntime, OperationalStoreKeyProvider,
};
use agentmage_kernel_engine::platform_startup::VerifiedPlatformAdapter;
use sha2::{Digest, Sha256};

use crate::security_controls::{LinuxSecurityControl, LinuxSecurityControls};
use crate::{
    DEFAULT_MAX_PREIMAGE_BYTES, LinuxAuthorizedWorkspace, LinuxHeldObject, LinuxHostIpcEndpoint,
    LinuxIpcError, LinuxOperationalStoreKeyProvider, LinuxPathAdapter, LinuxStrictLocalRoot,
    LinuxStrictLocalRootInspector, authorize_workspace_root,
};

const MAX_IDENTITY_FILE_BYTES: u64 = 256 * 1024 * 1024;
const TOOLCHAIN_IDENTITY: &[u8] = b"agentmage.rust-toolchain.1.95.0.v1";

/// Stable, content-free Linux platform discovery failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxPlatformDiscoveryErrorKind {
    /// The running distribution is not an admitted Linux family.
    UnsupportedPlatform,
    /// The running processor architecture is unsupported.
    UnsupportedArchitecture,
    /// A mandatory runtime identity could not be read safely.
    IdentityUnavailable,
    /// A mandatory identity exceeded its closed resource bound.
    ResourceLimitExceeded,
}

/// Content-free Linux platform discovery error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinuxPlatformDiscoveryError {
    kind: LinuxPlatformDiscoveryErrorKind,
}

impl LinuxPlatformDiscoveryError {
    /// Returns the stable failure class.
    #[must_use]
    pub const fn kind(self) -> LinuxPlatformDiscoveryErrorKind {
        self.kind
    }
}

impl fmt::Display for LinuxPlatformDiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.kind {
            LinuxPlatformDiscoveryErrorKind::UnsupportedPlatform => {
                "linux.platform.unsupported_platform"
            }
            LinuxPlatformDiscoveryErrorKind::UnsupportedArchitecture => {
                "linux.platform.unsupported_architecture"
            }
            LinuxPlatformDiscoveryErrorKind::IdentityUnavailable => {
                "linux.platform.identity_unavailable"
            }
            LinuxPlatformDiscoveryErrorKind::ResourceLimitExceeded => {
                "linux.platform.resource_limit"
            }
        })
    }
}

impl std::error::Error for LinuxPlatformDiscoveryError {}

/// Production aggregate for Fedora and Ubuntu platform observations and handles.
pub struct LinuxPlatformAdapter {
    family: PlatformFamily,
    adapter_instance_id: AdapterInstanceId,
    runtime_identity: PlatformRuntimeIdentity,
    observations: [PlatformCapabilityObservation; 10],
    path_adapter: LinuxPathAdapter,
}

impl fmt::Debug for LinuxPlatformAdapter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxPlatformAdapter")
            .field("family", &self.family)
            .field("adapter_instance_id", &self.adapter_instance_id)
            .finish_non_exhaustive()
    }
}

impl LinuxPlatformAdapter {
    /// Observes this process and fixed native mechanisms without authorizing a workspace.
    pub fn discover(
        adapter_instance_id: AdapterInstanceId,
    ) -> Result<Self, LinuxPlatformDiscoveryError> {
        let os_release = read_first_identity_bytes(&[
            Path::new("/usr/lib/os-release"),
            Path::new("/etc/os-release"),
        ])?;
        let family = parse_distribution(&os_release)?;
        let architecture = native_architecture()?;
        let os_build_sha256 = Sha256::digest(&os_release).into();
        let toolchain_sha256 = Sha256::digest(TOOLCHAIN_IDENTITY).into();
        let vscode_sha256 = read_first_identity(&[
            Path::new("/usr/share/code/code"),
            Path::new("/usr/bin/code"),
        ])?;
        let executable = std::env::current_exe()
            .map_err(|_| discovery_error(LinuxPlatformDiscoveryErrorKind::IdentityUnavailable))?;
        let package_sha256 = hash_identity_file(&executable)?;
        let runtime_identity = PlatformRuntimeIdentity::new(
            family,
            architecture,
            os_build_sha256,
            toolchain_sha256,
            vscode_sha256,
            package_sha256,
        );

        let package = observe_binary(&executable, true);
        let bubblewrap = observe_binary(Path::new("/usr/bin/bwrap"), true);
        let secret_tool = observe_binary(Path::new("/usr/bin/secret-tool"), true);
        let systemd_run = observe_binary(Path::new("/usr/bin/systemd-run"), true);
        let inference_adapter = observe_binary(
            Path::new("/usr/libexec/agentmage/agentmage-native-inference"),
            true,
        );
        let model_installer = observe_binary(
            Path::new("/usr/libexec/agentmage/agentmage-model-installer"),
            true,
        );
        let updater = observe_binary(Path::new("/usr/libexec/agentmage/agentmage-updater"), true);
        let controls = LinuxSecurityControls::discover(
            bubblewrap.status == PlatformCapabilityStatus::Verified,
            systemd_run.status == PlatformCapabilityStatus::Verified,
            secret_tool.status == PlatformCapabilityStatus::Verified,
        );
        let mechanisms = LinuxMechanismObservations {
            package,
            bubblewrap,
            secret_tool,
            systemd_run,
            inference_adapter,
            model_installer,
            updater,
            controls,
        };
        let observations = linux_capability_observations(family, &mechanisms);
        Ok(Self {
            family,
            adapter_instance_id: adapter_instance_id.clone(),
            runtime_identity,
            observations,
            path_adapter: LinuxPathAdapter::new(adapter_instance_id, DEFAULT_MAX_PREIMAGE_BYTES),
        })
    }

    /// Returns the exact aggregate-adapter instance identity.
    #[must_use]
    pub const fn adapter_instance_id(&self) -> &AdapterInstanceId {
        &self.adapter_instance_id
    }
}

impl PlatformAdapter for LinuxPlatformAdapter {
    fn runtime_identity(&self) -> Result<PlatformRuntimeIdentity, PlatformStartupError> {
        Ok(self.runtime_identity.clone())
    }

    fn probe_capability(&self, capability: PlatformCapability) -> PlatformCapabilityObservation {
        self.observations[capability as usize].clone()
    }
}

/// Authorizes one explicit user-selected Linux workspace after aggregate activation.
pub fn select_linux_workspace(
    verified: &VerifiedPlatformAdapter<LinuxPlatformAdapter>,
    root: &Path,
    workspace_id: WorkspaceId,
    authorization_id: WorkspaceAuthorizationId,
) -> Result<LinuxAuthorizedWorkspace, agentmage_kernel_contracts::PathAdapterError> {
    authorize_workspace_root(
        root,
        workspace_id,
        authorization_id,
        verified.adapter().adapter_instance_id.clone(),
    )
}

/// Resolves one canonical object through the verified aggregate path adapter.
pub fn resolve_linux_workspace_object(
    verified: &VerifiedPlatformAdapter<LinuxPlatformAdapter>,
    workspace: &LinuxAuthorizedWorkspace,
    path: &WorkspacePath,
    intent: PathResolutionIntent,
) -> Result<LinuxHeldObject, agentmage_kernel_contracts::PathAdapterError> {
    verified
        .adapter()
        .path_adapter
        .resolve(workspace, path, intent)
}

/// Binds one private authenticated host endpoint after aggregate activation.
pub fn open_linux_host_ipc(
    verified: &VerifiedPlatformAdapter<LinuxPlatformAdapter>,
    path: &Path,
) -> Result<LinuxHostIpcEndpoint, LinuxIpcError> {
    let _verified_adapter = verified.adapter();
    LinuxHostIpcEndpoint::bind(path)
}

/// Binds an authentication-only endpoint for a package-verified bootstrap.
///
/// This transport conveys no workspace handle, capability grant, tool permit,
/// model authority, or effect authority. Product requests must remain denied
/// until the caller separately verifies and activates the complete platform
/// release through [`open_linux_host_ipc`].
pub fn open_linux_bootstrap_ipc(path: &Path) -> Result<LinuxHostIpcEndpoint, LinuxIpcError> {
    LinuxHostIpcEndpoint::bind(path)
}

/// Selects a Linux workspace without release activation for isolated test harnesses.
///
/// This function is absent from normal builds and cannot be used as production
/// release or platform evidence.
#[cfg(feature = "test-support")]
pub fn select_test_linux_workspace(
    root: &Path,
    workspace_id: WorkspaceId,
    authorization_id: WorkspaceAuthorizationId,
    adapter_instance_id: agentmage_kernel_contracts::AdapterInstanceId,
) -> Result<LinuxAuthorizedWorkspace, agentmage_kernel_contracts::PathAdapterError> {
    authorize_workspace_root(root, workspace_id, authorization_id, adapter_instance_id)
}

/// Resolves one held object through a synthetic test-only Linux adapter identity.
///
/// This function is absent from normal builds and cannot satisfy release trust.
#[cfg(feature = "test-support")]
pub fn resolve_test_linux_workspace_object(
    workspace: &LinuxAuthorizedWorkspace,
    adapter_instance_id: agentmage_kernel_contracts::AdapterInstanceId,
    path: &WorkspacePath,
    intent: PathResolutionIntent,
) -> Result<LinuxHeldObject, agentmage_kernel_contracts::PathAdapterError> {
    LinuxPathAdapter::new(adapter_instance_id, crate::DEFAULT_MAX_PREIMAGE_BYTES)
        .resolve(workspace, path, intent)
}

/// Durable authority runtime that retains its descriptor-held private Linux root.
pub struct LinuxAuthorityRuntime {
    root: LinuxStrictLocalRoot,
    runtime: DurableAuthorityRuntime,
}

impl fmt::Debug for LinuxAuthorityRuntime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LinuxAuthorityRuntime")
            .field("root", &self.root)
            .field("runtime", &self.runtime)
            .finish_non_exhaustive()
    }
}

impl LinuxAuthorityRuntime {
    /// Returns the durable kernel authority while this wrapper retains its root.
    #[must_use]
    pub const fn authority(&self) -> &DurableAuthorityRuntime {
        &self.runtime
    }

    /// Returns mutable durable authority while this wrapper retains its root.
    #[must_use]
    pub const fn authority_mut(&mut self) -> &mut DurableAuthorityRuntime {
        &mut self.runtime
    }

    /// Revalidates the private root around an explicit lifecycle checkpoint.
    pub fn revalidate_root(&self) -> Result<(), crate::LinuxStrictLocalRootError> {
        self.root.revalidate()
    }
}

/// Opens SQLCipher state through a verified adapter and continuously held private root.
pub fn open_linux_authority(
    verified: &VerifiedPlatformAdapter<LinuxPlatformAdapter>,
    state_root: &Path,
    provider: &mut LinuxOperationalStoreKeyProvider,
    recovery_epoch_ms: u64,
) -> Result<LinuxAuthorityRuntime, LinuxAuthorityOpenError> {
    if verified.adapter().family != verified.manifest_identity().target().family() {
        return Err(LinuxAuthorityOpenError::Platform);
    }
    let root = LinuxStrictLocalRootInspector::inspect(state_root)
        .map_err(LinuxAuthorityOpenError::Root)?;
    open_linux_authority_in_root(root, provider, recovery_epoch_ms)
}

fn open_linux_authority_in_root<P: OperationalStoreKeyProvider>(
    root: LinuxStrictLocalRoot,
    provider: &mut P,
    recovery_epoch_ms: u64,
) -> Result<LinuxAuthorityRuntime, LinuxAuthorityOpenError> {
    let database = root.authority_database_path();
    root.revalidate().map_err(LinuxAuthorityOpenError::Root)?;
    let runtime =
        DurableAuthorityRuntime::open(&database, root.observation(), provider, recovery_epoch_ms)
            .map_err(LinuxAuthorityOpenError::Authority)?;
    root.revalidate().map_err(LinuxAuthorityOpenError::Root)?;
    Ok(LinuxAuthorityRuntime { root, runtime })
}

/// Opens a private Linux authority root for isolated test harnesses only.
///
/// This function is absent from normal builds and cannot satisfy platform
/// activation, release, package, or support evidence.
#[cfg(feature = "test-support")]
pub fn open_test_linux_authority<P: OperationalStoreKeyProvider>(
    state_root: &Path,
    provider: &mut P,
    recovery_epoch_ms: u64,
) -> Result<LinuxAuthorityRuntime, LinuxAuthorityOpenError> {
    let root = LinuxStrictLocalRootInspector::inspect(state_root)
        .map_err(LinuxAuthorityOpenError::Root)?;
    open_linux_authority_in_root(root, provider, recovery_epoch_ms)
}

/// Closed failure from Linux private-root and durable-authority composition.
#[derive(Debug)]
pub enum LinuxAuthorityOpenError {
    /// Private strict-local root admission failed.
    Root(crate::LinuxStrictLocalRootError),
    /// Encrypted authority startup or recovery failed.
    Authority(DurableAuthorityError),
    /// The admitted release and aggregate adapter did not agree.
    Platform,
}

impl fmt::Display for LinuxAuthorityOpenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Root(_) => "linux.authority.root",
            Self::Authority(_) => "linux.authority.store",
            Self::Platform => "linux.authority.platform",
        })
    }
}

impl std::error::Error for LinuxAuthorityOpenError {}

#[derive(Clone, Copy)]
struct BinaryObservation {
    status: PlatformCapabilityStatus,
    sha256: [u8; 32],
}

#[derive(Clone)]
struct LinuxMechanismObservations {
    package: BinaryObservation,
    bubblewrap: BinaryObservation,
    secret_tool: BinaryObservation,
    systemd_run: BinaryObservation,
    inference_adapter: BinaryObservation,
    model_installer: BinaryObservation,
    updater: BinaryObservation,
    controls: LinuxSecurityControls,
}

fn linux_capability_observations(
    family: PlatformFamily,
    mechanisms: &LinuxMechanismObservations,
) -> [PlatformCapabilityObservation; 10] {
    let control = |control| {
        let observed = mechanisms.controls.observation(control);
        BinaryObservation {
            status: observed.status(),
            sha256: observed.mechanism_sha256(),
        }
    };
    let bubblewrap_control = control(LinuxSecurityControl::Bubblewrap);
    let user_namespaces = control(LinuxSecurityControl::UserNamespaces);
    let seccomp = control(LinuxSecurityControl::Seccomp);
    let cgroups = control(LinuxSecurityControl::Cgroups);
    let secret_service = control(LinuxSecurityControl::SecretService);
    let descriptor_paths = control(LinuxSecurityControl::DescriptorSafePaths);
    let network = control(LinuxSecurityControl::NetworkIsolation);

    [
        observation(
            family,
            PlatformCapability::WorkspaceAuthorization,
            &[mechanisms.package, descriptor_paths],
        ),
        observation(
            family,
            PlatformCapability::SecurePathResolution,
            &[mechanisms.package, descriptor_paths],
        ),
        observation(
            family,
            PlatformCapability::ToolConfinement,
            &[
                mechanisms.package,
                mechanisms.bubblewrap,
                bubblewrap_control,
                user_namespaces,
                seccomp,
            ],
        ),
        observation(
            family,
            PlatformCapability::SecretStorage,
            &[mechanisms.secret_tool, secret_service],
        ),
        observation(
            family,
            PlatformCapability::ProcessLimits,
            &[mechanisms.systemd_run, cgroups],
        ),
        observation(
            family,
            PlatformCapability::LocalInference,
            &[mechanisms.inference_adapter],
        ),
        observation(
            family,
            PlatformCapability::ModelInstallation,
            &[mechanisms.model_installer],
        ),
        observation(family, PlatformCapability::Packaging, &[mechanisms.package]),
        observation(family, PlatformCapability::Updates, &[mechanisms.updater]),
        observation(
            family,
            PlatformCapability::NetworkIsolation,
            &[
                mechanisms.package,
                mechanisms.bubblewrap,
                bubblewrap_control,
                user_namespaces,
                network,
            ],
        ),
    ]
}

fn observation(
    family: PlatformFamily,
    capability: PlatformCapability,
    binaries: &[BinaryObservation],
) -> PlatformCapabilityObservation {
    let status = binaries
        .iter()
        .fold(PlatformCapabilityStatus::Verified, |current, binary| {
            combine_status(current, binary.status)
        });
    let mut digest = Sha256::new();
    digest.update(b"agentmage.linux-capability-mechanism.v1\0");
    digest.update([capability as u8]);
    for binary in binaries {
        digest.update(binary.sha256);
    }
    PlatformCapabilityObservation::new(capability, status, family, digest.finalize().into())
}

const fn combine_status(
    left: PlatformCapabilityStatus,
    right: PlatformCapabilityStatus,
) -> PlatformCapabilityStatus {
    match (left, right) {
        (PlatformCapabilityStatus::Invalid, _) | (_, PlatformCapabilityStatus::Invalid) => {
            PlatformCapabilityStatus::Invalid
        }
        (PlatformCapabilityStatus::Unavailable, _) | (_, PlatformCapabilityStatus::Unavailable) => {
            PlatformCapabilityStatus::Unavailable
        }
        _ => PlatformCapabilityStatus::Verified,
    }
}

fn observe_binary(path: &Path, root_owned: bool) -> BinaryObservation {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => {
            return BinaryObservation {
                status: PlatformCapabilityStatus::Unavailable,
                sha256: [0; 32],
            };
        }
    };
    let valid = metadata.file_type().is_file()
        && !metadata.file_type().is_symlink()
        && !metadata.file_type().is_socket()
        && metadata.mode() & 0o022 == 0
        && (!root_owned || metadata.uid() == 0);
    match hash_identity_file(path) {
        Ok(sha256) => BinaryObservation {
            status: if valid {
                PlatformCapabilityStatus::Verified
            } else {
                PlatformCapabilityStatus::Invalid
            },
            sha256,
        },
        Err(_) => BinaryObservation {
            status: PlatformCapabilityStatus::Invalid,
            sha256: [0; 32],
        },
    }
}

fn read_first_identity(paths: &[&Path]) -> Result<[u8; 32], LinuxPlatformDiscoveryError> {
    paths
        .iter()
        .find_map(|path| hash_identity_file(path).ok())
        .ok_or_else(|| discovery_error(LinuxPlatformDiscoveryErrorKind::IdentityUnavailable))
}

fn read_first_identity_bytes(paths: &[&Path]) -> Result<Vec<u8>, LinuxPlatformDiscoveryError> {
    paths
        .iter()
        .find_map(|path| read_identity_file(path).ok())
        .ok_or_else(|| discovery_error(LinuxPlatformDiscoveryErrorKind::IdentityUnavailable))
}

fn read_identity_file(path: &Path) -> Result<Vec<u8>, LinuxPlatformDiscoveryError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| discovery_error(LinuxPlatformDiscoveryErrorKind::IdentityUnavailable))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(discovery_error(
            LinuxPlatformDiscoveryErrorKind::IdentityUnavailable,
        ));
    }
    if metadata.len() > MAX_IDENTITY_FILE_BYTES {
        return Err(discovery_error(
            LinuxPlatformDiscoveryErrorKind::ResourceLimitExceeded,
        ));
    }
    fs::read(path)
        .map_err(|_| discovery_error(LinuxPlatformDiscoveryErrorKind::IdentityUnavailable))
}

fn hash_identity_file(path: &Path) -> Result<[u8; 32], LinuxPlatformDiscoveryError> {
    let mut file = File::open(path)
        .map_err(|_| discovery_error(LinuxPlatformDiscoveryErrorKind::IdentityUnavailable))?;
    let metadata = file
        .metadata()
        .map_err(|_| discovery_error(LinuxPlatformDiscoveryErrorKind::IdentityUnavailable))?;
    if !metadata.file_type().is_file() || metadata.len() > MAX_IDENTITY_FILE_BYTES {
        return Err(discovery_error(
            LinuxPlatformDiscoveryErrorKind::ResourceLimitExceeded,
        ));
    }
    let mut digest = Sha256::new();
    let mut observed = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|_| discovery_error(LinuxPlatformDiscoveryErrorKind::IdentityUnavailable))?;
        if count == 0 {
            break;
        }
        observed = observed.checked_add(count as u64).ok_or_else(|| {
            discovery_error(LinuxPlatformDiscoveryErrorKind::ResourceLimitExceeded)
        })?;
        if observed > MAX_IDENTITY_FILE_BYTES {
            return Err(discovery_error(
                LinuxPlatformDiscoveryErrorKind::ResourceLimitExceeded,
            ));
        }
        digest.update(&buffer[..count]);
    }
    Ok(digest.finalize().into())
}

fn parse_distribution(bytes: &[u8]) -> Result<PlatformFamily, LinuxPlatformDiscoveryError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| discovery_error(LinuxPlatformDiscoveryErrorKind::UnsupportedPlatform))?;
    if text.lines().any(|line| line == "ID=fedora") {
        Ok(PlatformFamily::Fedora)
    } else if text.lines().any(|line| line == "ID=ubuntu") {
        Ok(PlatformFamily::Ubuntu)
    } else {
        Err(discovery_error(
            LinuxPlatformDiscoveryErrorKind::UnsupportedPlatform,
        ))
    }
}

const fn native_architecture() -> Result<PlatformArchitecture, LinuxPlatformDiscoveryError> {
    #[cfg(target_arch = "x86_64")]
    {
        Ok(PlatformArchitecture::X86_64)
    }
    #[cfg(target_arch = "aarch64")]
    {
        Ok(PlatformArchitecture::Aarch64)
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        Err(discovery_error(
            LinuxPlatformDiscoveryErrorKind::UnsupportedArchitecture,
        ))
    }
}

const fn discovery_error(kind: LinuxPlatformDiscoveryErrorKind) -> LinuxPlatformDiscoveryError {
    LinuxPlatformDiscoveryError { kind }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};

    use agentmage_kernel_contracts::{
        AdapterInstanceId, PlatformAdapter, PlatformCapability, PlatformCapabilityStatus,
        PlatformFamily, REQUIRED_PLATFORM_CAPABILITIES,
    };
    use agentmage_kernel_engine::operational_store::{
        OperationalStoreKeyError, OperationalStoreKeyProvider,
    };

    use super::{
        BinaryObservation, LinuxAuthorityOpenError, LinuxMechanismObservations,
        LinuxPlatformAdapter, combine_status, linux_capability_observations,
        open_linux_authority_in_root, parse_distribution,
    };
    use crate::security_controls::{
        LinuxSecurityControl, LinuxSecurityControls, REQUIRED_LINUX_SECURITY_CONTROLS,
    };
    use crate::{LinuxStrictLocalRootErrorKind, LinuxStrictLocalRootInspector};

    static TEMP_ID: AtomicU64 = AtomicU64::new(1);

    struct TestKey([u8; 32]);

    impl OperationalStoreKeyProvider for TestKey {
        fn with_key<T>(
            &mut self,
            operation: impl FnOnce(&[u8]) -> T,
        ) -> Result<T, OperationalStoreKeyError> {
            Ok(operation(&self.0))
        }
    }

    #[test]
    fn distribution_parser_is_closed() {
        assert_eq!(
            parse_distribution(b"ID=fedora\nVERSION_ID=44\n").expect("fedora"),
            PlatformFamily::Fedora
        );
        assert_eq!(
            parse_distribution(b"ID=ubuntu\nVERSION_ID=26.04\n").expect("ubuntu"),
            PlatformFamily::Ubuntu
        );
        assert!(parse_distribution(b"ID=other\n").is_err());
    }

    #[test]
    fn invalid_status_dominates_unavailable_and_verified() {
        assert_eq!(
            combine_status(
                PlatformCapabilityStatus::Unavailable,
                PlatformCapabilityStatus::Invalid
            ),
            PlatformCapabilityStatus::Invalid
        );
    }

    #[test]
    fn every_linux_control_disablement_maps_to_fail_closed_startup_capabilities() {
        for family in [PlatformFamily::Fedora, PlatformFamily::Ubuntu] {
            for control in REQUIRED_LINUX_SECURITY_CONTROLS {
                for status in [
                    PlatformCapabilityStatus::Unavailable,
                    PlatformCapabilityStatus::Invalid,
                ] {
                    let mut mechanisms = synthetic_mechanisms();
                    mechanisms.controls.set_status(control, status);
                    let observations = linux_capability_observations(family, &mechanisms);
                    let affected = affected_capabilities(control);

                    assert!(!affected.is_empty(), "{}", control.id());
                    for capability in REQUIRED_PLATFORM_CAPABILITIES {
                        assert_eq!(
                            observations[capability as usize].status(),
                            if affected.contains(&capability) {
                                status
                            } else {
                                PlatformCapabilityStatus::Verified
                            },
                            "{}:{capability:?}:{status:?}",
                            control.id()
                        );
                    }
                }
            }
        }
    }

    #[test]
    #[ignore = "requires a supported Fedora or Ubuntu desktop with Visual Studio Code installed"]
    fn production_discovery_never_self_supplies_release_trust() {
        let adapter =
            LinuxPlatformAdapter::discover(AdapterInstanceId::from_raw("linux-platform-test-0001"))
                .expect("development Linux identities are observable");
        assert!(matches!(
            adapter.runtime_identity().expect("runtime").family(),
            PlatformFamily::Fedora | PlatformFamily::Ubuntu
        ));
        assert_eq!(
            adapter
                .probe_capability(PlatformCapability::ModelInstallation)
                .capability(),
            PlatformCapability::ModelInstallation
        );
    }

    #[test]
    fn aggregate_authority_composition_opens_through_held_private_root() {
        let directory = std::env::temp_dir().join(format!(
            "agentmage-linux-authority-{}-{}",
            std::process::id(),
            TEMP_ID.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir(&directory).expect("state root creates");
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))
            .expect("state root private");
        let root = LinuxStrictLocalRootInspector::inspect(&directory).expect("state root inspects");
        let runtime = open_linux_authority_in_root(root, &mut TestKey([61; 32]), 1)
            .expect("held-root authority opens");
        let _authority = runtime.authority();
        runtime.revalidate_root().expect("root remains stable");
        drop(runtime);
        std::fs::remove_dir_all(directory).expect("fixture removes");
    }

    #[test]
    fn strict_local_state_root_authority_open_rejects_sync_before_store_io() {
        let directory = std::env::temp_dir().join(format!(
            "agentmage-linux-authority-sync-{}-{}",
            std::process::id(),
            TEMP_ID.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir(&directory).expect("state root creates");
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))
            .expect("state root private");
        std::fs::create_dir(directory.join(".stfolder")).expect("sync marker creates");
        let root = LinuxStrictLocalRootInspector::inspect(&directory).expect("root observes");
        let error = open_linux_authority_in_root(root, &mut TestKey([62; 32]), 1)
            .expect_err("synchronized root rejects before store open");
        assert!(matches!(
            error,
            LinuxAuthorityOpenError::Root(root_error)
                if root_error.kind() == LinuxStrictLocalRootErrorKind::CloudSynchronized
        ));
        assert!(!directory.join("authority.db").exists());
        std::fs::remove_dir_all(directory).expect("fixture removes");
    }

    fn synthetic_mechanisms() -> LinuxMechanismObservations {
        LinuxMechanismObservations {
            package: verified_binary(1),
            bubblewrap: verified_binary(2),
            secret_tool: verified_binary(3),
            systemd_run: verified_binary(4),
            inference_adapter: verified_binary(5),
            model_installer: verified_binary(6),
            updater: verified_binary(7),
            controls: LinuxSecurityControls::all_verified(),
        }
    }

    const fn verified_binary(identity: u8) -> BinaryObservation {
        BinaryObservation {
            status: PlatformCapabilityStatus::Verified,
            sha256: [identity; 32],
        }
    }

    const fn affected_capabilities(control: LinuxSecurityControl) -> &'static [PlatformCapability] {
        match control {
            LinuxSecurityControl::Bubblewrap | LinuxSecurityControl::UserNamespaces => &[
                PlatformCapability::ToolConfinement,
                PlatformCapability::NetworkIsolation,
            ],
            LinuxSecurityControl::Seccomp => &[PlatformCapability::ToolConfinement],
            LinuxSecurityControl::Cgroups => &[PlatformCapability::ProcessLimits],
            LinuxSecurityControl::SecretService => &[PlatformCapability::SecretStorage],
            LinuxSecurityControl::DescriptorSafePaths => &[
                PlatformCapability::WorkspaceAuthorization,
                PlatformCapability::SecurePathResolution,
            ],
            LinuxSecurityControl::NetworkIsolation => &[PlatformCapability::NetworkIsolation],
        }
    }
}
