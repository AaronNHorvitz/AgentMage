//! Shared platform-adapter startup and capability contracts.

use std::fmt;

/// Version of the shared platform-adapter API implemented by every adapter.
pub const PLATFORM_ADAPTER_API_VERSION: u16 = 1;

/// Closed platform target selected by a release manifest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlatformFamily {
    /// Deterministic adapter used only by contract and fault tests.
    DeterministicFake,
    /// Supported Fedora Linux adapter.
    Fedora,
    /// Supported Ubuntu Linux adapter.
    Ubuntu,
    /// Apple Silicon macOS adapter.
    MacOsAppleSilicon,
}

/// Closed processor architecture selected by a release manifest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformArchitecture {
    /// 64-bit x86 architecture.
    X86_64,
    /// 64-bit Arm architecture.
    Aarch64,
}

/// Complete set of platform-owned capabilities required before kernel startup.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlatformCapability {
    /// User-mediated workspace authorization.
    WorkspaceAuthorization,
    /// Descriptor-relative secure path resolution.
    SecurePathResolution,
    /// Per-operation tool confinement.
    ToolConfinement,
    /// Operating-system-backed secret storage.
    SecretStorage,
    /// Per-process resource enforcement.
    ProcessLimits,
    /// Approved local inference runtime.
    LocalInference,
    /// Separate verified model installation or import.
    ModelInstallation,
    /// Platform package identity and verification.
    Packaging,
    /// Signed, fail-closed update and rollback handling.
    Updates,
    /// Enforced network isolation for offline runtime components.
    NetworkIsolation,
}

/// Exact closed capability order used by startup and conformance tests.
pub const REQUIRED_PLATFORM_CAPABILITIES: [PlatformCapability; 10] = [
    PlatformCapability::WorkspaceAuthorization,
    PlatformCapability::SecurePathResolution,
    PlatformCapability::ToolConfinement,
    PlatformCapability::SecretStorage,
    PlatformCapability::ProcessLimits,
    PlatformCapability::LocalInference,
    PlatformCapability::ModelInstallation,
    PlatformCapability::Packaging,
    PlatformCapability::Updates,
    PlatformCapability::NetworkIsolation,
];

/// Allowlisted runtime identity compared with one release manifest before startup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformRuntimeIdentity {
    family: PlatformFamily,
    architecture: PlatformArchitecture,
    os_build_sha256: [u8; 32],
    toolchain_sha256: [u8; 32],
    vscode_build_sha256: [u8; 32],
    package_sha256: [u8; 32],
}

impl PlatformRuntimeIdentity {
    /// Creates an allowlisted runtime identity from exact content digests.
    #[must_use]
    pub const fn new(
        family: PlatformFamily,
        architecture: PlatformArchitecture,
        os_build_sha256: [u8; 32],
        toolchain_sha256: [u8; 32],
        vscode_build_sha256: [u8; 32],
        package_sha256: [u8; 32],
    ) -> Self {
        Self {
            family,
            architecture,
            os_build_sha256,
            toolchain_sha256,
            vscode_build_sha256,
            package_sha256,
        }
    }

    /// Returns the exact platform family.
    #[must_use]
    pub const fn family(&self) -> PlatformFamily {
        self.family
    }

    /// Returns the exact processor architecture.
    #[must_use]
    pub const fn architecture(&self) -> PlatformArchitecture {
        self.architecture
    }

    /// Returns the digest of the normalized operating-system build identity.
    #[must_use]
    pub const fn os_build_sha256(&self) -> &[u8; 32] {
        &self.os_build_sha256
    }

    /// Returns the digest of the pinned product toolchain identity.
    #[must_use]
    pub const fn toolchain_sha256(&self) -> &[u8; 32] {
        &self.toolchain_sha256
    }

    /// Returns the digest of the supported Visual Studio Code build identity.
    #[must_use]
    pub const fn vscode_build_sha256(&self) -> &[u8; 32] {
        &self.vscode_build_sha256
    }

    /// Returns the exact installed AgentMage package digest.
    #[must_use]
    pub const fn package_sha256(&self) -> &[u8; 32] {
        &self.package_sha256
    }
}

/// Content-addressed identity of one immutable platform release manifest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformManifestIdentity {
    api_version: u16,
    manifest_sha256: [u8; 32],
    target: PlatformRuntimeIdentity,
}

impl PlatformManifestIdentity {
    /// Creates one immutable manifest identity and its exact runtime target.
    #[must_use]
    pub const fn new(
        api_version: u16,
        manifest_sha256: [u8; 32],
        target: PlatformRuntimeIdentity,
    ) -> Self {
        Self {
            api_version,
            manifest_sha256,
            target,
        }
    }

    /// Returns the platform-adapter API version required by the manifest.
    #[must_use]
    pub const fn api_version(&self) -> u16 {
        self.api_version
    }

    /// Returns the exact canonical-manifest digest.
    #[must_use]
    pub const fn manifest_sha256(&self) -> &[u8; 32] {
        &self.manifest_sha256
    }

    /// Returns the exact allowlisted runtime target.
    #[must_use]
    pub const fn target(&self) -> &PlatformRuntimeIdentity {
        &self.target
    }
}

/// Closed result of probing one platform capability before workspace access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformCapabilityStatus {
    /// The declared mechanism was independently observed and matched the manifest.
    Verified,
    /// The required primitive was not available.
    Unavailable,
    /// A primitive was present but failed identity or enforcement validation.
    Invalid,
}

/// Content-free identity evidence from one platform capability probe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformCapabilityObservation {
    capability: PlatformCapability,
    status: PlatformCapabilityStatus,
    platform: PlatformFamily,
    mechanism_sha256: [u8; 32],
}

impl PlatformCapabilityObservation {
    /// Creates one bounded capability observation without native paths or error text.
    #[must_use]
    pub const fn new(
        capability: PlatformCapability,
        status: PlatformCapabilityStatus,
        platform: PlatformFamily,
        mechanism_sha256: [u8; 32],
    ) -> Self {
        Self {
            capability,
            status,
            platform,
            mechanism_sha256,
        }
    }

    /// Returns the capability that was probed.
    #[must_use]
    pub const fn capability(&self) -> PlatformCapability {
        self.capability
    }

    /// Returns the closed probe result.
    #[must_use]
    pub const fn status(&self) -> PlatformCapabilityStatus {
        self.status
    }

    /// Returns the platform family that performed the probe.
    #[must_use]
    pub const fn platform(&self) -> PlatformFamily {
        self.platform
    }

    /// Returns the content-free digest of the observed mechanism identity.
    #[must_use]
    pub const fn mechanism_sha256(&self) -> &[u8; 32] {
        &self.mechanism_sha256
    }
}

/// Stable, content-free platform-startup failure class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformStartupErrorKind {
    /// Trusted release-manifest bytes exceeded the closed input bound.
    ManifestTooLarge,
    /// Trusted release-manifest bytes did not match the closed schema.
    ManifestMalformed,
    /// The release manifest used an unsupported schema, status, or target.
    ManifestUnsupported,
    /// The detached release-manifest signature did not verify.
    ManifestSignatureInvalid,
    /// The adapter implements an unsupported API version.
    ApiVersionMismatch,
    /// The observed operating-system family differs from the manifest.
    PlatformMismatch,
    /// The observed processor architecture differs from the manifest.
    ArchitectureMismatch,
    /// The operating-system build identity differs from the manifest.
    OsBuildMismatch,
    /// The product toolchain identity differs from the manifest.
    ToolchainMismatch,
    /// The Visual Studio Code build identity differs from the manifest.
    VisualStudioCodeMismatch,
    /// The installed package digest differs from the manifest.
    PackageMismatch,
    /// A probe returned evidence for a different capability.
    CapabilityMismatch,
    /// A probe returned evidence from another platform or manifest.
    ForeignCapabilityEvidence,
    /// A verified observation did not match the independently trusted mechanism.
    MechanismMismatch,
    /// A required platform primitive is unavailable.
    CapabilityUnavailable,
    /// A required platform primitive failed validation.
    CapabilityInvalid,
}

impl PlatformStartupErrorKind {
    /// Returns the stable redacted code for this failure class.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ManifestTooLarge => "platform.startup.manifest_too_large",
            Self::ManifestMalformed => "platform.startup.manifest_malformed",
            Self::ManifestUnsupported => "platform.startup.manifest_unsupported",
            Self::ManifestSignatureInvalid => "platform.startup.manifest_signature_invalid",
            Self::ApiVersionMismatch => "platform.startup.api_version_mismatch",
            Self::PlatformMismatch => "platform.startup.platform_mismatch",
            Self::ArchitectureMismatch => "platform.startup.architecture_mismatch",
            Self::OsBuildMismatch => "platform.startup.os_build_mismatch",
            Self::ToolchainMismatch => "platform.startup.toolchain_mismatch",
            Self::VisualStudioCodeMismatch => "platform.startup.vscode_mismatch",
            Self::PackageMismatch => "platform.startup.package_mismatch",
            Self::CapabilityMismatch => "platform.startup.capability_mismatch",
            Self::ForeignCapabilityEvidence => "platform.startup.foreign_capability_evidence",
            Self::MechanismMismatch => "platform.startup.mechanism_mismatch",
            Self::CapabilityUnavailable => "platform.startup.capability_unavailable",
            Self::CapabilityInvalid => "platform.startup.capability_invalid",
        }
    }
}

/// Content-free startup error with optional capability attribution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformStartupError {
    kind: PlatformStartupErrorKind,
    capability: Option<PlatformCapability>,
}

impl PlatformStartupError {
    /// Creates a bounded startup error without native platform details.
    #[must_use]
    pub const fn new(
        kind: PlatformStartupErrorKind,
        capability: Option<PlatformCapability>,
    ) -> Self {
        Self { kind, capability }
    }

    /// Returns the stable failure class.
    #[must_use]
    pub const fn kind(&self) -> PlatformStartupErrorKind {
        self.kind
    }

    /// Returns the failed capability, when failure occurred during a probe.
    #[must_use]
    pub const fn capability(&self) -> Option<PlatformCapability> {
        self.capability
    }
}

impl fmt::Display for PlatformStartupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.kind.code())
    }
}

impl std::error::Error for PlatformStartupError {}

/// Shared startup boundary implemented by deterministic and native platform adapters.
///
/// Probes run before any workspace handle is created. Product logic selects an adapter
/// only through the platform-independent kernel activation routine.
pub trait PlatformAdapter: fmt::Debug + Send + Sync {
    /// Observes the allowlisted runtime identity without retaining ambient values.
    fn runtime_identity(&self) -> Result<PlatformRuntimeIdentity, PlatformStartupError>;

    /// Probes one required capability without accessing a workspace.
    fn probe_capability(&self, capability: PlatformCapability) -> PlatformCapabilityObservation;
}
