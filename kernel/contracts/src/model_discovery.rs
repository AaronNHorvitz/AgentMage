//! Content-free model-picker discovery and exact-selection contracts.

use crate::{
    ModelAdapterId, ModelCapabilityState, ModelHealthState, ModelLifecycleState, ModelModality,
    ModelProfileId, ModelRole, ModelRuntimeKind, PlatformArchitecture, PlatformFamily,
};

/// Activation state observed for one exact profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelActivationState {
    /// The exact profile completed product activation.
    Activated,
    /// The profile is retained but has no active product registration.
    Inactive,
    /// A named activation prerequisite failed.
    Blocked,
    /// The activation record no longer matches current inputs.
    Stale,
}

/// Platform and hardware compatibility observed for one exact profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelCompatibilityState {
    /// The current platform and hardware satisfy the recorded envelope.
    Compatible,
    /// The current machine is outside the recorded envelope.
    Incompatible,
    /// Compatibility could not be established because a prerequisite failed.
    Blocked,
    /// Compatibility evidence no longer matches the current machine.
    Stale,
}

/// Current support disposition for one exact profile tuple.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelSupportState {
    /// The complete tuple is supported without an active limitation.
    Supported,
    /// The tuple is supported only within displayed limitations.
    Limited,
    /// The tuple is not supported for ordinary product use.
    Unsupported,
    /// Support evidence is no longer current.
    Stale,
}

/// Where one profile may be displayed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelPickerDisposition {
    /// The exact profile may appear in the ordinary native model picker.
    Selectable,
    /// The profile may appear only in a management or diagnostic surface.
    ManagementOnly,
}

/// Role-specific capability projected without evaluation content.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelPickerCapability {
    /// Evaluated role.
    pub role: ModelRole,
    /// Current role disposition.
    pub state: ModelCapabilityState,
    /// Stable limitation codes.
    pub limitations: Vec<String>,
}

/// Exact content-free profile information safe for a native model picker.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelPickerEntry {
    /// Exact profile identity.
    pub profile_id: ModelProfileId,
    /// Human-readable label from the signed profile tuple.
    pub display_name: String,
    /// Family label for display only, never admission.
    pub family: String,
    /// Exact manifest digest.
    pub manifest_sha256: String,
    /// Exact artifact digest.
    pub artifact_sha256: String,
    /// Exact codec identity.
    pub codec_id: String,
    /// Exact codec digest.
    pub codec_sha256: String,
    /// Exact tokenizer digest.
    pub tokenizer_sha256: String,
    /// Exact template digest.
    pub template_sha256: String,
    /// Exact runtime adapter identity.
    pub runtime_adapter_id: ModelAdapterId,
    /// Runtime implementation family.
    pub runtime_kind: ModelRuntimeKind,
    /// Exact runtime build digest.
    pub runtime_sha256: String,
    /// Runtime platform.
    pub platform: PlatformFamily,
    /// Runtime architecture.
    pub architecture: PlatformArchitecture,
    /// Declared modalities.
    pub modalities: Vec<ModelModality>,
    /// Exact maximum context token count.
    pub max_context_tokens: u32,
    /// Exact maximum input bytes.
    pub max_input_bytes: u64,
    /// Exact maximum messages.
    pub max_messages: u32,
    /// Exact maximum output tokens.
    pub max_output_tokens: u32,
    /// Whether an admitted role supports typed tool selection.
    pub tool_calling: bool,
    /// Role-specific capability records.
    pub capabilities: Vec<ModelPickerCapability>,
    /// Exact lifecycle state.
    pub lifecycle: ModelLifecycleState,
    /// Current runtime health.
    pub runtime_health: ModelHealthState,
    /// Current activation state.
    pub activation: ModelActivationState,
    /// Current compatibility state.
    pub compatibility: ModelCompatibilityState,
    /// Current support state.
    pub support: ModelSupportState,
    /// Complete stable limitation and refusal reason codes.
    pub limitations: Vec<String>,
    /// Whether choosing this profile requires a new explicit user decision.
    pub requires_user_decision: bool,
    /// Ordinary-picker or management-only disposition.
    pub disposition: ModelPickerDisposition,
    /// Digest of every preceding field in this entry.
    pub entry_sha256: String,
}

/// One current, signed-catalog-bound discovery result.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelPickerSnapshot {
    /// Contract schema version.
    pub schema_version: u16,
    /// Digest of the exact signed catalog bytes.
    pub catalog_sha256: String,
    /// Whether the catalog signature was verified before projection.
    pub catalog_signature_verified: bool,
    /// Logical observation time in milliseconds.
    pub observed_at_ms: u64,
    /// Exact entries in stable profile-identity order.
    pub entries: Vec<ModelPickerEntry>,
    /// Digest of every preceding field in this snapshot.
    pub snapshot_sha256: String,
}

/// Result of checking a selected picker entry immediately before use.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelSelectionRevalidation {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact selected profile.
    pub profile_id: ModelProfileId,
    /// Previously displayed entry digest.
    pub expected_entry_sha256: String,
    /// Current discovery snapshot digest.
    pub current_snapshot_sha256: String,
    /// Whether the exact entry remains selectable and unchanged.
    pub admitted: bool,
    /// Stable content-free outcome code.
    pub result_code: String,
}
