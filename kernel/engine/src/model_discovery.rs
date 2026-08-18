//! Candidate-neutral native-picker projection and stale-selection refusal.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, ExactModelProfile, ModelActivationState, ModelCapabilityState,
    ModelCompatibilityState, ModelHealth, ModelHealthState, ModelLifecycleState,
    ModelPickerCapability, ModelPickerDisposition, ModelPickerEntry, ModelPickerSnapshot,
    ModelProfileId, ModelRole, ModelSelectionRevalidation, ModelSupportState,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::model_runtime::{ModelAdmissionCatalog, ModelRuntimeGateError};

const MAX_PROFILES: usize = 64;
const MAX_CODES: usize = 128;
const MAX_CODE_BYTES: usize = 128;

/// Content-free current facts joined to one exact signed-catalog profile.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelDiscoveryCandidate {
    /// Complete exact profile tuple from the verified catalog.
    pub profile: ExactModelProfile,
    /// Current health from the profile's exact runtime adapter.
    pub runtime_health: ModelHealth,
    /// Current product-activation state.
    pub activation: ModelActivationState,
    /// Current platform and hardware compatibility state.
    pub compatibility: ModelCompatibilityState,
    /// Current support disposition.
    pub support: ModelSupportState,
    /// Whether the profile policy digest is the active policy digest.
    pub policy_current: bool,
    /// Additional stable support or compatibility limitation codes.
    pub limitations: Vec<String>,
    /// Whether selection requires a fresh explicit user decision.
    pub requires_user_decision: bool,
}

/// Stable fail-closed discovery error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelDiscoveryError {
    /// Catalog signature verification was absent or failed.
    CatalogUnverified,
    /// A digest, profile, health join, code, count, or identity was invalid.
    InvalidInput,
    /// A duplicate exact profile identity was supplied.
    DuplicateProfile,
    /// A supplied snapshot failed its content digest or ordering checks.
    SnapshotInvalid,
}

impl ModelDiscoveryError {
    /// Returns the stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::CatalogUnverified => "model.discovery.catalog-unverified",
            Self::InvalidInput => "model.discovery.input-invalid",
            Self::DuplicateProfile => "model.discovery.profile-duplicate",
            Self::SnapshotInvalid => "model.discovery.snapshot-invalid",
        }
    }
}

/// Projects exact profiles into a content-free, stable native-picker snapshot.
pub fn build_model_picker_snapshot(
    catalog_sha256: String,
    catalog_signature_verified: bool,
    observed_at_ms: u64,
    mut candidates: Vec<ModelDiscoveryCandidate>,
) -> Result<ModelPickerSnapshot, ModelDiscoveryError> {
    if !catalog_signature_verified {
        return Err(ModelDiscoveryError::CatalogUnverified);
    }
    if !valid_sha256(&catalog_sha256) || observed_at_ms == 0 || candidates.len() > MAX_PROFILES {
        return Err(ModelDiscoveryError::InvalidInput);
    }
    candidates.sort_by(|left, right| {
        left.profile
            .profile_id
            .as_str()
            .cmp(right.profile.profile_id.as_str())
    });
    let mut identities = BTreeSet::new();
    let mut entries = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        if !identities.insert(candidate.profile.profile_id.as_str().to_owned()) {
            return Err(ModelDiscoveryError::DuplicateProfile);
        }
        entries.push(project_candidate(candidate)?);
    }
    let snapshot_sha256 = snapshot_digest(
        &catalog_sha256,
        catalog_signature_verified,
        observed_at_ms,
        &entries,
    )?;
    Ok(ModelPickerSnapshot {
        schema_version: CONTRACT_SCHEMA_VERSION,
        catalog_sha256,
        catalog_signature_verified,
        observed_at_ms,
        entries,
        snapshot_sha256,
    })
}

/// Verifies ordering and every digest in a received picker snapshot.
pub fn verify_model_picker_snapshot(
    snapshot: &ModelPickerSnapshot,
) -> Result<(), ModelDiscoveryError> {
    if snapshot.schema_version != CONTRACT_SCHEMA_VERSION
        || !snapshot.catalog_signature_verified
        || !valid_sha256(&snapshot.catalog_sha256)
        || snapshot.observed_at_ms == 0
        || snapshot.entries.len() > MAX_PROFILES
    {
        return Err(ModelDiscoveryError::SnapshotInvalid);
    }
    let mut previous: Option<&str> = None;
    for entry in &snapshot.entries {
        if previous.is_some_and(|value| value >= entry.profile_id.as_str())
            || !valid_entry(entry)
            || entry.entry_sha256 != entry_digest(entry)?
        {
            return Err(ModelDiscoveryError::SnapshotInvalid);
        }
        previous = Some(entry.profile_id.as_str());
    }
    if snapshot.snapshot_sha256
        != snapshot_digest(
            &snapshot.catalog_sha256,
            snapshot.catalog_signature_verified,
            snapshot.observed_at_ms,
            &snapshot.entries,
        )?
    {
        return Err(ModelDiscoveryError::SnapshotInvalid);
    }
    Ok(())
}

/// Revalidates one exact displayed entry immediately before use without fallback.
pub fn revalidate_model_selection(
    profile_id: ModelProfileId,
    expected_entry_sha256: String,
    current: &ModelPickerSnapshot,
) -> Result<ModelSelectionRevalidation, ModelDiscoveryError> {
    verify_model_picker_snapshot(current)?;
    if !valid_sha256(&expected_entry_sha256) {
        return Err(ModelDiscoveryError::InvalidInput);
    }
    let entry = current
        .entries
        .iter()
        .find(|entry| entry.profile_id == profile_id);
    let (admitted, result_code) = match entry {
        None => (false, "model.selection.profile-unavailable"),
        Some(entry) if entry.entry_sha256 != expected_entry_sha256 => {
            (false, "model.selection.profile-changed")
        }
        Some(entry) if entry.disposition != ModelPickerDisposition::Selectable => {
            (false, "model.selection.profile-blocked")
        }
        Some(_) => (true, "model.selection.revalidated"),
    };
    Ok(ModelSelectionRevalidation {
        schema_version: CONTRACT_SCHEMA_VERSION,
        profile_id,
        expected_entry_sha256,
        current_snapshot_sha256: current.snapshot_sha256.clone(),
        admitted,
        result_code: result_code.to_owned(),
    })
}

fn project_candidate(
    candidate: ModelDiscoveryCandidate,
) -> Result<ModelPickerEntry, ModelDiscoveryError> {
    ModelAdmissionCatalog::new(vec![candidate.profile.clone()]).map_err(map_profile_error)?;
    if candidate.runtime_health.adapter_id != candidate.profile.runtime.adapter_id
        || candidate.runtime_health.profile_id.as_ref() != Some(&candidate.profile.profile_id)
        || candidate.runtime_health.observed_at_ms == 0
        || candidate.limitations.len() > MAX_CODES
        || candidate.limitations.iter().any(|code| !valid_code(code))
    {
        return Err(ModelDiscoveryError::InvalidInput);
    }

    let profile = candidate.profile;
    let context_sha256 = digest(&profile.context)?;
    let decoding_sha256 = digest(&profile.decoding)?;
    let hardware_sha256 = digest(&profile.hardware)?;
    let selectable = matches!(
        profile.lifecycle,
        ModelLifecycleState::Approved | ModelLifecycleState::Degraded
    ) && profile.enabled
        && candidate.activation == ModelActivationState::Activated
        && candidate.compatibility == ModelCompatibilityState::Compatible
        && matches!(
            candidate.support,
            ModelSupportState::Supported | ModelSupportState::Limited
        )
        && candidate.policy_current
        && candidate.requires_user_decision
        && candidate.runtime_health.state == ModelHealthState::Ready
        && profile
            .capabilities
            .iter()
            .any(|capability| capability.state == ModelCapabilityState::Passed);

    let mut limitations = candidate.limitations;
    for capability in &profile.capabilities {
        limitations.extend(capability.limitations.iter().cloned());
    }
    if !profile.enabled {
        limitations.push("model.profile.disabled".to_owned());
    }
    if !matches!(
        profile.lifecycle,
        ModelLifecycleState::Approved | ModelLifecycleState::Degraded
    ) {
        limitations.push(format!("model.lifecycle.{}", enum_code(&profile.lifecycle)));
    }
    if candidate.activation != ModelActivationState::Activated {
        limitations.push(format!(
            "model.activation.{}",
            enum_code(&candidate.activation)
        ));
    }
    if candidate.compatibility != ModelCompatibilityState::Compatible {
        limitations.push(format!(
            "model.compatibility.{}",
            enum_code(&candidate.compatibility)
        ));
    }
    if !matches!(
        candidate.support,
        ModelSupportState::Supported | ModelSupportState::Limited
    ) {
        limitations.push(format!("model.support.{}", enum_code(&candidate.support)));
    }
    if !candidate.policy_current {
        limitations.push("model.policy.stale".to_owned());
    }
    if candidate.runtime_health.state != ModelHealthState::Ready {
        limitations.push(format!(
            "model.runtime.{}",
            enum_code(&candidate.runtime_health.state)
        ));
    }
    if !selectable
        && !profile
            .capabilities
            .iter()
            .any(|capability| capability.state == ModelCapabilityState::Passed)
    {
        limitations.push("model.capability.no-passed-role".to_owned());
    }
    limitations.sort();
    limitations.dedup();
    if limitations.len() > MAX_CODES || limitations.iter().any(|code| !valid_code(code)) {
        return Err(ModelDiscoveryError::InvalidInput);
    }

    let capabilities = profile
        .capabilities
        .iter()
        .map(|capability| ModelPickerCapability {
            role: capability.role,
            state: capability.state,
            limitations: capability.limitations.clone(),
        })
        .collect::<Vec<_>>();
    let tool_calling = profile.capabilities.iter().any(|capability| {
        capability.role == ModelRole::ToolSelection
            && capability.state == ModelCapabilityState::Passed
    });
    let mut entry = ModelPickerEntry {
        profile_id: profile.profile_id,
        display_name: profile.display_name,
        family: profile.family,
        manifest_sha256: profile.manifest_sha256,
        artifact_sha256: profile.artifact.sha256,
        publisher: profile.artifact.publisher,
        publisher_control: profile.publisher_control,
        lineage: profile.lineage,
        license_spdx: profile.license_spdx,
        license_terms_sha256: profile.license_terms_sha256,
        source_revision: profile.artifact.source_revision,
        artifact_format: profile.artifact.format,
        artifact_bytes: profile.artifact.bytes,
        quantization: profile.quantization,
        codec_id: profile.codec.codec_id.as_str().to_owned(),
        codec_sha256: profile.codec.codec_sha256,
        tokenizer_sha256: profile.codec.tokenizer_sha256,
        template_sha256: profile.codec.template_sha256,
        runtime_adapter_id: profile.runtime.adapter_id,
        runtime_kind: profile.runtime.kind,
        runtime_contract_version: profile.runtime.contract_version,
        runtime_build: profile.runtime.runtime_build,
        runtime_sha256: profile.runtime.runtime_sha256,
        platform: profile.runtime.platform,
        architecture: profile.runtime.architecture,
        modalities: profile.modalities,
        max_context_tokens: profile.context.max_context_tokens,
        max_input_bytes: profile.context.max_input_bytes,
        max_messages: profile.context.max_messages,
        context_sha256,
        max_output_tokens: profile.decoding.max_output_tokens,
        decoding_sha256,
        hardware_sha256,
        policy_sha256: profile.policy_sha256,
        tool_calling,
        capabilities,
        lifecycle: profile.lifecycle,
        runtime_health: candidate.runtime_health.state,
        activation: candidate.activation,
        compatibility: candidate.compatibility,
        support: candidate.support,
        limitations,
        requires_user_decision: candidate.requires_user_decision,
        disposition: if selectable {
            ModelPickerDisposition::Selectable
        } else {
            ModelPickerDisposition::ManagementOnly
        },
        entry_sha256: String::new(),
    };
    entry.entry_sha256 = entry_digest(&entry)?;
    Ok(entry)
}

fn map_profile_error(_: ModelRuntimeGateError) -> ModelDiscoveryError {
    ModelDiscoveryError::InvalidInput
}

fn entry_digest(entry: &ModelPickerEntry) -> Result<String, ModelDiscoveryError> {
    #[derive(Serialize)]
    struct Unsigned<'a> {
        profile_id: &'a ModelProfileId,
        display_name: &'a str,
        family: &'a str,
        manifest_sha256: &'a str,
        artifact_sha256: &'a str,
        publisher: &'a str,
        publisher_control: &'a str,
        lineage: &'a [String],
        license_spdx: &'a str,
        license_terms_sha256: &'a str,
        source_revision: &'a str,
        artifact_format: &'a str,
        artifact_bytes: u64,
        quantization: &'a str,
        codec_id: &'a str,
        codec_sha256: &'a str,
        tokenizer_sha256: &'a str,
        template_sha256: &'a str,
        runtime_adapter_id: &'a agentmage_kernel_contracts::ModelAdapterId,
        runtime_kind: agentmage_kernel_contracts::ModelRuntimeKind,
        runtime_contract_version: u16,
        runtime_build: &'a str,
        runtime_sha256: &'a str,
        platform: agentmage_kernel_contracts::PlatformFamily,
        architecture: agentmage_kernel_contracts::PlatformArchitecture,
        modalities: &'a [agentmage_kernel_contracts::ModelModality],
        max_context_tokens: u32,
        max_input_bytes: u64,
        max_messages: u32,
        context_sha256: &'a str,
        max_output_tokens: u32,
        decoding_sha256: &'a str,
        hardware_sha256: &'a str,
        policy_sha256: &'a str,
        tool_calling: bool,
        capabilities: &'a [ModelPickerCapability],
        lifecycle: ModelLifecycleState,
        runtime_health: ModelHealthState,
        activation: ModelActivationState,
        compatibility: ModelCompatibilityState,
        support: ModelSupportState,
        limitations: &'a [String],
        requires_user_decision: bool,
        disposition: ModelPickerDisposition,
    }
    digest(&Unsigned {
        profile_id: &entry.profile_id,
        display_name: &entry.display_name,
        family: &entry.family,
        manifest_sha256: &entry.manifest_sha256,
        artifact_sha256: &entry.artifact_sha256,
        publisher: &entry.publisher,
        publisher_control: &entry.publisher_control,
        lineage: &entry.lineage,
        license_spdx: &entry.license_spdx,
        license_terms_sha256: &entry.license_terms_sha256,
        source_revision: &entry.source_revision,
        artifact_format: &entry.artifact_format,
        artifact_bytes: entry.artifact_bytes,
        quantization: &entry.quantization,
        codec_id: &entry.codec_id,
        codec_sha256: &entry.codec_sha256,
        tokenizer_sha256: &entry.tokenizer_sha256,
        template_sha256: &entry.template_sha256,
        runtime_adapter_id: &entry.runtime_adapter_id,
        runtime_kind: entry.runtime_kind,
        runtime_contract_version: entry.runtime_contract_version,
        runtime_build: &entry.runtime_build,
        runtime_sha256: &entry.runtime_sha256,
        platform: entry.platform,
        architecture: entry.architecture,
        modalities: &entry.modalities,
        max_context_tokens: entry.max_context_tokens,
        max_input_bytes: entry.max_input_bytes,
        max_messages: entry.max_messages,
        context_sha256: &entry.context_sha256,
        max_output_tokens: entry.max_output_tokens,
        decoding_sha256: &entry.decoding_sha256,
        hardware_sha256: &entry.hardware_sha256,
        policy_sha256: &entry.policy_sha256,
        tool_calling: entry.tool_calling,
        capabilities: &entry.capabilities,
        lifecycle: entry.lifecycle,
        runtime_health: entry.runtime_health,
        activation: entry.activation,
        compatibility: entry.compatibility,
        support: entry.support,
        limitations: &entry.limitations,
        requires_user_decision: entry.requires_user_decision,
        disposition: entry.disposition,
    })
}

fn snapshot_digest(
    catalog_sha256: &str,
    catalog_signature_verified: bool,
    observed_at_ms: u64,
    entries: &[ModelPickerEntry],
) -> Result<String, ModelDiscoveryError> {
    #[derive(Serialize)]
    struct Unsigned<'a> {
        schema_version: u16,
        catalog_sha256: &'a str,
        catalog_signature_verified: bool,
        observed_at_ms: u64,
        entries: &'a [ModelPickerEntry],
    }
    digest(&Unsigned {
        schema_version: CONTRACT_SCHEMA_VERSION,
        catalog_sha256,
        catalog_signature_verified,
        observed_at_ms,
        entries,
    })
}

fn digest<T: Serialize>(value: &T) -> Result<String, ModelDiscoveryError> {
    let bytes = serde_json::to_vec(value).map_err(|_| ModelDiscoveryError::InvalidInput)?;
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write;
        write!(&mut encoded, "{byte:02x}").expect("writing to a string cannot fail");
    }
    Ok(encoded)
}

fn enum_code<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .expect("closed enum serialization cannot fail")
        .trim_matches('"')
        .replace('_', "-")
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_entry(entry: &ModelPickerEntry) -> bool {
    let mut modalities = BTreeSet::new();
    let mut roles = BTreeSet::new();
    let mut lineage = BTreeSet::new();
    let limitations_sorted = entry.limitations.windows(2).all(|pair| pair[0] < pair[1]);
    let has_passed_role = entry
        .capabilities
        .iter()
        .any(|capability| capability.state == ModelCapabilityState::Passed);
    let expected_tool_calling = entry.capabilities.iter().any(|capability| {
        capability.role == ModelRole::ToolSelection
            && capability.state == ModelCapabilityState::Passed
    });
    let selectable_facts = matches!(
        entry.lifecycle,
        ModelLifecycleState::Approved | ModelLifecycleState::Degraded
    ) && entry.activation == ModelActivationState::Activated
        && entry.compatibility == ModelCompatibilityState::Compatible
        && matches!(
            entry.support,
            ModelSupportState::Supported | ModelSupportState::Limited
        )
        && entry.runtime_health == ModelHealthState::Ready
        && has_passed_role
        && entry.requires_user_decision;

    valid_identifier(entry.profile_id.as_str())
        && valid_text(&entry.display_name)
        && valid_text(&entry.family)
        && valid_sha256(&entry.manifest_sha256)
        && valid_sha256(&entry.artifact_sha256)
        && valid_text(&entry.publisher)
        && valid_text(&entry.publisher_control)
        && !entry.lineage.is_empty()
        && entry.lineage.len() <= 32
        && entry
            .lineage
            .iter()
            .all(|item| valid_text(item) && lineage.insert(item.as_str()))
        && valid_text(&entry.license_spdx)
        && valid_sha256(&entry.license_terms_sha256)
        && valid_text(&entry.source_revision)
        && valid_text(&entry.artifact_format)
        && entry.artifact_bytes > 0
        && valid_text(&entry.quantization)
        && valid_identifier(&entry.codec_id)
        && valid_sha256(&entry.codec_sha256)
        && valid_sha256(&entry.tokenizer_sha256)
        && valid_sha256(&entry.template_sha256)
        && valid_identifier(entry.runtime_adapter_id.as_str())
        && entry.runtime_contract_version > 0
        && valid_text(&entry.runtime_build)
        && valid_sha256(&entry.runtime_sha256)
        && !entry.modalities.is_empty()
        && entry
            .modalities
            .iter()
            .all(|modality| modalities.insert(*modality))
        && entry.max_context_tokens > 0
        && entry.max_input_bytes > 0
        && entry.max_messages > 0
        && valid_sha256(&entry.context_sha256)
        && entry.max_output_tokens > 0
        && valid_sha256(&entry.decoding_sha256)
        && valid_sha256(&entry.hardware_sha256)
        && valid_sha256(&entry.policy_sha256)
        && !entry.capabilities.is_empty()
        && entry.capabilities.iter().all(|capability| {
            roles.insert(capability.role)
                && capability.limitations.len() <= MAX_CODES
                && capability.limitations.iter().all(|code| valid_code(code))
        })
        && entry.tool_calling == expected_tool_calling
        && entry.limitations.len() <= MAX_CODES
        && entry.limitations.iter().all(|code| valid_code(code))
        && limitations_sorted
        && (entry.disposition != ModelPickerDisposition::Selectable || selectable_facts)
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_CODE_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':'))
}

fn valid_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}

fn valid_code(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_CODE_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'_')
        })
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        ContextBudget, DecodingProfile, FamilyCodecIdentity, HardwareEnvelope, ModelAdapterId,
        ModelArtifact, ModelCapability, ModelCodecId, ModelHealth, ModelHealthState,
        ModelLifecycleState, ModelManifestId, ModelModality, ModelProfileId, ModelRole,
        ModelRuntimeIdentity, ModelRuntimeKind, PlatformArchitecture, PlatformFamily,
    };

    use super::*;

    const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    type CandidateMutation = Box<dyn Fn(&mut ModelDiscoveryCandidate)>;

    fn profile(id: &str, family: &str) -> ExactModelProfile {
        ExactModelProfile {
            schema_version: CONTRACT_SCHEMA_VERSION,
            profile_id: ModelProfileId::from_raw(id),
            manifest_id: ModelManifestId::from_raw(format!("{id}-manifest")),
            manifest_sha256: SHA.to_owned(),
            display_name: format!("{family} exact profile"),
            family: family.to_owned(),
            publisher_control: "fixture-publisher".to_owned(),
            lineage: vec!["fixture-source".to_owned()],
            license_spdx: "Apache-2.0".to_owned(),
            license_terms_sha256: SHA.to_owned(),
            artifact: ModelArtifact {
                artifact_id: format!("{id}-artifact"),
                publisher: "fixture-publisher".to_owned(),
                source_revision: "fixture-revision".to_owned(),
                format: "GGUF".to_owned(),
                bytes: 1,
                sha256: SHA.to_owned(),
            },
            transformations: Vec::new(),
            codec: FamilyCodecIdentity {
                codec_id: ModelCodecId::from_raw(format!("{id}-codec")),
                codec_version: "1.0.0".to_owned(),
                codec_sha256: SHA.to_owned(),
                tokenizer: "fixture-tokenizer".to_owned(),
                tokenizer_sha256: SHA.to_owned(),
                template: "fixture-template".to_owned(),
                template_sha256: SHA.to_owned(),
                tool_protocol_version: "closed-proposal-v1".to_owned(),
                end_tokens: vec![1],
                reasoning_enabled: false,
            },
            runtime: ModelRuntimeIdentity {
                adapter_id: ModelAdapterId::from_raw(format!("{id}-adapter")),
                kind: ModelRuntimeKind::NativeLlamaCpp,
                contract_version: 1,
                runtime_build: "fixture-runtime".to_owned(),
                runtime_sha256: SHA.to_owned(),
                platform: PlatformFamily::Fedora,
                architecture: PlatformArchitecture::X86_64,
            },
            quantization: "fixture-q4".to_owned(),
            modalities: vec![ModelModality::Text],
            context: ContextBudget {
                max_context_tokens: 8_192,
                max_input_bytes: 32_768,
                max_messages: 32,
                token_counter: "fixture-counter".to_owned(),
                token_counter_sha256: SHA.to_owned(),
            },
            decoding: DecodingProfile {
                profile_id: "fixture-decoding".to_owned(),
                sampler_order: vec!["temperature".to_owned()],
                temperature: 0.0,
                top_p: 1.0,
                top_k: 1,
                repeat_penalty: 1.0,
                seed: 1,
                max_output_tokens: 256,
            },
            hardware: vec![HardwareEnvelope {
                platform: PlatformFamily::Fedora,
                architecture: PlatformArchitecture::X86_64,
                minimum_system_memory_bytes: 1,
                minimum_accelerator_memory_bytes: 0,
                accelerator: "cpu".to_owned(),
                driver_constraint: "none".to_owned(),
            }],
            capabilities: vec![
                ModelCapability {
                    role: ModelRole::Dialogue,
                    state: ModelCapabilityState::Passed,
                    evaluation_profile: Some("dialogue-v1".to_owned()),
                    result_sha256: Some(SHA.to_owned()),
                    limitations: Vec::new(),
                },
                ModelCapability {
                    role: ModelRole::ToolSelection,
                    state: ModelCapabilityState::Passed,
                    evaluation_profile: Some("tool-v1".to_owned()),
                    result_sha256: Some(SHA.to_owned()),
                    limitations: Vec::new(),
                },
            ],
            policy_sha256: SHA.to_owned(),
            lifecycle: ModelLifecycleState::Approved,
            enabled: true,
            automatic_fallback: false,
        }
    }

    fn candidate(id: &str, family: &str) -> ModelDiscoveryCandidate {
        let profile = profile(id, family);
        ModelDiscoveryCandidate {
            runtime_health: ModelHealth {
                adapter_id: profile.runtime.adapter_id.clone(),
                profile_id: Some(profile.profile_id.clone()),
                state: ModelHealthState::Ready,
                reason_code: "runtime.ready".to_owned(),
                observed_at_ms: 7,
            },
            profile,
            activation: ModelActivationState::Activated,
            compatibility: ModelCompatibilityState::Compatible,
            support: ModelSupportState::Supported,
            policy_current: true,
            limitations: Vec::new(),
            requires_user_decision: true,
        }
    }

    fn snapshot(candidates: Vec<ModelDiscoveryCandidate>) -> ModelPickerSnapshot {
        build_model_picker_snapshot(SHA.to_owned(), true, 10, candidates).expect("snapshot")
    }

    #[test]
    fn zero_and_multiple_profiles_are_stable_and_family_neutral() {
        let empty = snapshot(Vec::new());
        assert!(empty.entries.is_empty());
        verify_model_picker_snapshot(&empty).expect("empty snapshot");

        let value = snapshot(vec![
            candidate("muse-1", "muse"),
            candidate("gemma-1", "gemma"),
        ]);
        assert_eq!(value.entries.len(), 2);
        assert_eq!(value.entries[0].profile_id.as_str(), "gemma-1");
        assert_eq!(value.entries[1].profile_id.as_str(), "muse-1");
        assert!(value.entries.iter().all(|entry| {
            entry.disposition == ModelPickerDisposition::Selectable
                && entry.tool_calling
                && entry.requires_user_decision
        }));
    }

    #[test]
    fn every_non_current_lifecycle_or_dependency_is_management_only() {
        let lifecycle_states = [
            ModelLifecycleState::Candidate,
            ModelLifecycleState::Evaluating,
            ModelLifecycleState::Quarantined,
            ModelLifecycleState::Rejected,
            ModelLifecycleState::Retired,
        ];
        for (index, lifecycle) in lifecycle_states.into_iter().enumerate() {
            let mut value = candidate(&format!("lifecycle-{index}"), "additional");
            value.profile.lifecycle = lifecycle;
            value.profile.enabled = false;
            assert_eq!(
                snapshot(vec![value]).entries[0].disposition,
                ModelPickerDisposition::ManagementOnly
            );
        }

        let mutations: Vec<CandidateMutation> = vec![
            Box::new(|value| value.activation = ModelActivationState::Blocked),
            Box::new(|value| value.activation = ModelActivationState::Stale),
            Box::new(|value| value.compatibility = ModelCompatibilityState::Incompatible),
            Box::new(|value| value.compatibility = ModelCompatibilityState::Blocked),
            Box::new(|value| value.compatibility = ModelCompatibilityState::Stale),
            Box::new(|value| value.support = ModelSupportState::Unsupported),
            Box::new(|value| value.support = ModelSupportState::Stale),
            Box::new(|value| value.policy_current = false),
            Box::new(|value| value.runtime_health.state = ModelHealthState::Degraded),
            Box::new(|value| value.runtime_health.state = ModelHealthState::Quarantined),
            Box::new(|value| value.runtime_health.state = ModelHealthState::Failed),
        ];
        for (index, mutate) in mutations.into_iter().enumerate() {
            let mut value = candidate(&format!("dependency-{index}"), "additional");
            mutate(&mut value);
            let entry = &snapshot(vec![value]).entries[0];
            assert_eq!(entry.disposition, ModelPickerDisposition::ManagementOnly);
            assert!(!entry.limitations.is_empty());
        }
    }

    #[test]
    fn limited_degraded_profile_is_selectable_only_with_passed_role() {
        let mut value = candidate("degraded-1", "additional");
        value.profile.lifecycle = ModelLifecycleState::Degraded;
        value.support = ModelSupportState::Limited;
        value.limitations = vec!["support.known-limitation".to_owned()];
        assert_eq!(
            snapshot(vec![value.clone()]).entries[0].disposition,
            ModelPickerDisposition::Selectable
        );
        for capability in &mut value.profile.capabilities {
            capability.state = ModelCapabilityState::Blocked;
            capability.evaluation_profile = None;
            capability.result_sha256 = None;
        }
        assert_eq!(
            snapshot(vec![value]).entries[0].disposition,
            ModelPickerDisposition::ManagementOnly
        );
    }

    #[test]
    fn every_exact_profile_evidence_mutation_invalidates_selection() {
        let original = snapshot(vec![candidate("selected", "muse")]);
        let selected = original.entries[0].clone();
        let mutations: Vec<CandidateMutation> = vec![
            Box::new(|value| value.profile.family = "changed-family".to_owned()),
            Box::new(|value| value.profile.display_name = "Changed profile".to_owned()),
            Box::new(|value| value.profile.artifact.sha256 = "b".repeat(64)),
            Box::new(|value| value.profile.artifact.publisher = "Changed publisher".to_owned()),
            Box::new(|value| value.profile.publisher_control = "changed-control".to_owned()),
            Box::new(|value| value.profile.lineage.push("changed-lineage".to_owned())),
            Box::new(|value| value.profile.license_spdx = "MIT".to_owned()),
            Box::new(|value| value.profile.license_terms_sha256 = "b".repeat(64)),
            Box::new(|value| value.profile.artifact.source_revision = "changed".to_owned()),
            Box::new(|value| value.profile.artifact.format = "changed".to_owned()),
            Box::new(|value| value.profile.artifact.bytes += 1),
            Box::new(|value| value.profile.quantization = "changed".to_owned()),
            Box::new(|value| value.profile.codec.tokenizer_sha256 = "b".repeat(64)),
            Box::new(|value| value.profile.codec.template_sha256 = "b".repeat(64)),
            Box::new(|value| value.profile.codec.codec_sha256 = "b".repeat(64)),
            Box::new(|value| value.profile.runtime.runtime_build = "changed-runtime".to_owned()),
            Box::new(|value| value.profile.runtime.runtime_sha256 = "b".repeat(64)),
            Box::new(|value| value.profile.context.token_counter_sha256 = "b".repeat(64)),
            Box::new(|value| value.profile.decoding.seed += 1),
            Box::new(|value| value.profile.hardware[0].driver_constraint = "changed".to_owned()),
            Box::new(|value| value.profile.policy_sha256 = "b".repeat(64)),
            Box::new(|value| value.activation = ModelActivationState::Stale),
            Box::new(|value| value.support = ModelSupportState::Stale),
        ];

        for mutate in mutations {
            let mut changed = candidate("selected", "muse");
            mutate(&mut changed);
            let current = snapshot(vec![changed]);
            let result = revalidate_model_selection(
                selected.profile_id.clone(),
                selected.entry_sha256.clone(),
                &current,
            )
            .expect("closed revalidation result");
            assert!(!result.admitted);
            assert_eq!(result.result_code, "model.selection.profile-changed");
        }
    }

    #[test]
    fn unsigned_duplicate_and_mismatched_runtime_inputs_fail_closed() {
        assert_eq!(
            build_model_picker_snapshot(SHA.to_owned(), false, 10, Vec::new()),
            Err(ModelDiscoveryError::CatalogUnverified)
        );
        let duplicate = candidate("same", "muse");
        assert_eq!(
            build_model_picker_snapshot(
                SHA.to_owned(),
                true,
                10,
                vec![duplicate.clone(), duplicate]
            ),
            Err(ModelDiscoveryError::DuplicateProfile)
        );
        let mut mismatch = candidate("mismatch", "muse");
        mismatch.runtime_health.adapter_id = ModelAdapterId::from_raw("other-adapter");
        assert_eq!(
            build_model_picker_snapshot(SHA.to_owned(), true, 10, vec![mismatch]),
            Err(ModelDiscoveryError::InvalidInput)
        );
    }

    #[test]
    fn exact_entry_mutation_blocks_revalidation_without_fallback() {
        let original = snapshot(vec![
            candidate("selected", "muse"),
            candidate("other", "gemma"),
        ]);
        let selected = original
            .entries
            .iter()
            .find(|entry| entry.profile_id.as_str() == "selected")
            .expect("selected");
        let admitted = revalidate_model_selection(
            selected.profile_id.clone(),
            selected.entry_sha256.clone(),
            &original,
        )
        .expect("revalidation");
        assert!(admitted.admitted);

        let mut changed_candidate = candidate("selected", "muse");
        changed_candidate.profile.codec.template_sha256 = "b".repeat(64);
        let changed = snapshot(vec![changed_candidate, candidate("other", "gemma")]);
        let refused = revalidate_model_selection(
            selected.profile_id.clone(),
            selected.entry_sha256.clone(),
            &changed,
        )
        .expect("refusal");
        assert!(!refused.admitted);
        assert_eq!(refused.result_code, "model.selection.profile-changed");
        assert_ne!(refused.profile_id.as_str(), "other");
    }

    #[test]
    fn removal_failure_and_tampering_never_select_an_alternative() {
        let original = snapshot(vec![candidate("selected", "muse")]);
        let entry = &original.entries[0];
        let empty = snapshot(Vec::new());
        let removed = revalidate_model_selection(
            entry.profile_id.clone(),
            entry.entry_sha256.clone(),
            &empty,
        )
        .expect("removed profile");
        assert!(!removed.admitted);
        assert_eq!(removed.result_code, "model.selection.profile-unavailable");

        let mut tampered = original;
        tampered.entries[0].max_context_tokens += 1;
        assert_eq!(
            verify_model_picker_snapshot(&tampered),
            Err(ModelDiscoveryError::SnapshotInvalid)
        );
    }
}
