//! Closed content-free contracts for local product diagnostics.

/// Product boundary represented by one diagnostic observation.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticComponent {
    /// Exact package and release-manifest identity.
    Package,
    /// Supported local platform adapter.
    Platform,
    /// Exact selected model profile.
    Model,
    /// Exact local inference runtime.
    Runtime,
    /// Reference-machine fit for the selected profile.
    HardwareFit,
    /// Strict-local network posture.
    OfflineBoundary,
    /// Platform sandbox and privileged helper boundary.
    SandboxHelper,
    /// Current explicit workspace grant.
    WorkspaceGrant,
    /// Installed capability contract versions.
    Capabilities,
    /// Repository-map freshness and integrity.
    RepositoryMap,
    /// Encrypted operational-store availability.
    EncryptedStore,
    /// Receipt sequence and hash-chain continuity.
    ReceiptChain,
    /// Interrupted-session recovery state.
    Recovery,
}

impl DiagnosticComponent {
    /// Every component in stable display and reconciliation order.
    pub const ALL: [Self; 13] = [
        Self::Package,
        Self::Platform,
        Self::Model,
        Self::Runtime,
        Self::HardwareFit,
        Self::OfflineBoundary,
        Self::SandboxHelper,
        Self::WorkspaceGrant,
        Self::Capabilities,
        Self::RepositoryMap,
        Self::EncryptedStore,
        Self::ReceiptChain,
        Self::Recovery,
    ];
}

/// Closed diagnostic state used by Chat, test harnesses, and reviewed exports.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticState {
    /// The exact component is present, current, and usable.
    Healthy,
    /// The component remains usable within a named limitation.
    Degraded,
    /// Policy or a failed prerequisite prevents use.
    Blocked,
    /// The component is absent or cannot currently be observed.
    Unavailable,
    /// The component was isolated after an integrity or behavior failure.
    Quarantined,
    /// The observed implementation is not supported by this product build.
    Unsupported,
}

/// Content-free observation supplied to the trusted diagnostic provider.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticObservation {
    /// Component represented by this exact observation.
    pub component: DiagnosticComponent,
    /// Closed state observed for the component.
    pub state: DiagnosticState,
    /// Stable product-defined reason code.
    pub reason_code: String,
    /// Optional lowercase SHA-256 identity for the observed manifest or state.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub identity_sha256: Option<String>,
    /// Whether the observation exceeded its product-defined freshness limit.
    pub stale: bool,
}

/// One redacted diagnostic item safe for local display and reviewed export.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticItem {
    /// Component in stable report order.
    pub component: DiagnosticComponent,
    /// Effective closed state after freshness rules are applied.
    pub state: DiagnosticState,
    /// Stable product-defined reason code.
    pub reason_code: String,
    /// Stable product-defined local remediation code.
    pub remediation_code: String,
    /// Optional lowercase SHA-256 identity; never raw content or a local path.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub identity_sha256: Option<String>,
}

/// Complete redacted local doctor result.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DoctorReport {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable report format identity.
    pub report_kind: String,
    /// Worst effective state in the report.
    pub overall_state: DiagnosticState,
    /// Exactly one item for every diagnostic component.
    pub items: Vec<DiagnosticItem>,
    /// Lowercase SHA-256 digest of the canonical report before this field.
    pub report_sha256: String,
}
