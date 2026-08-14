//! Non-acquiring model review and machine-fit preflight.

use agentmage_kernel_contracts::{
    ExactModelProfile, ModelLifecycleState, ModelModality, ModelProfileId, ModelRuntimeIdentity,
    PlatformArchitecture, PlatformFamily,
};

const GIB: u64 = 1024 * 1024 * 1024;
const MAX_ARTIFACT_BYTES: u64 = 64 * GIB;
const INSTALL_RESERVE_BYTES: u64 = GIB;

/// Exact host facts used before any model source is opened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelAcquisitionHost {
    /// Observed platform family.
    pub platform: PlatformFamily,
    /// Observed processor architecture.
    pub architecture: PlatformArchitecture,
    /// Total system memory available to the machine.
    pub system_memory_bytes: u64,
    /// Total compatible accelerator memory.
    pub accelerator_memory_bytes: u64,
    /// Free bytes in the separately selected private model store.
    pub model_store_available_bytes: u64,
    /// Maximum context requested for the first install self-check.
    pub requested_context_tokens: u32,
    /// Exact runtime observed as locally installed and package-verified.
    pub runtime: Option<ModelRuntimeIdentity>,
}

/// Stable reason an exact candidate cannot begin acquisition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModelAcquisitionBlocker {
    /// Candidate profile structure or lifecycle is not installable.
    ProfilePolicy,
    /// Artifact format or byte identity is outside the initial importer contract.
    ArtifactPolicy,
    /// License, publisher, or lineage information is incomplete.
    ProvenancePolicy,
    /// No exact hardware envelope matches the observed platform and architecture.
    Platform,
    /// Required runtime is absent or differs from the exact profile.
    Runtime,
    /// Requested context exceeds the exact profile.
    Context,
    /// System memory is below the matching envelope.
    SystemMemory,
    /// Accelerator memory is below the matching envelope.
    AcceleratorMemory,
    /// Private model-store capacity cannot retain staging plus active bytes.
    Disk,
}

impl ModelAcquisitionBlocker {
    /// Returns the stable content-free blocker code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ProfilePolicy => "model.install.profile-policy",
            Self::ArtifactPolicy => "model.install.artifact-policy",
            Self::ProvenancePolicy => "model.install.provenance-policy",
            Self::Platform => "model.install.platform",
            Self::Runtime => "model.install.runtime",
            Self::Context => "model.install.context",
            Self::SystemMemory => "model.install.system-memory",
            Self::AcceleratorMemory => "model.install.accelerator-memory",
            Self::Disk => "model.install.disk",
        }
    }
}

/// Exact non-acquiring preflight disposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelAcquisitionDisposition {
    /// Every policy and machine-fit prerequisite passed.
    Eligible,
    /// Candidate identity, provenance, artifact, runtime, or context policy blocked.
    Blocked,
    /// Candidate is policy-eligible but this machine lacks a required resource.
    BlockedHardware,
}

/// Review record rendered before a source or destination can be selected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelAcquisitionReview {
    /// Exact candidate profile.
    pub profile_id: ModelProfileId,
    /// Exact profile manifest digest.
    pub manifest_sha256: String,
    /// Display-only exact profile label.
    pub display_name: String,
    /// Publisher controlling the selected artifact.
    pub publisher: String,
    /// Publisher-control jurisdiction or policy label.
    pub publisher_control: String,
    /// Ordered source-to-artifact lineage.
    pub lineage: Vec<String>,
    /// Exact SPDX expression.
    pub license_spdx: String,
    /// Exact license and use-term digest.
    pub license_terms_sha256: String,
    /// Exact immutable source revision.
    pub source_revision: String,
    /// Exact artifact format.
    pub artifact_format: String,
    /// Exact artifact byte count.
    pub artifact_bytes: u64,
    /// Exact artifact digest.
    pub artifact_sha256: String,
    /// Exact quantization label.
    pub quantization: String,
    /// Exact required runtime.
    pub runtime: ModelRuntimeIdentity,
    /// Maximum admitted context for this profile.
    pub maximum_context_tokens: u32,
    /// Minimum system memory from the matching hardware envelope, when present.
    pub minimum_system_memory_bytes: Option<u64>,
    /// Minimum accelerator memory from the matching envelope, when present.
    pub minimum_accelerator_memory_bytes: Option<u64>,
    /// Required private-store capacity for staging and active copies plus reserve.
    pub minimum_model_store_bytes: u64,
}

/// Complete non-acquiring result with ordered reasons and review material.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelAcquisitionPreflight {
    /// Exact review material displayed before acquisition.
    pub review: ModelAcquisitionReview,
    /// Truthful current disposition.
    pub disposition: ModelAcquisitionDisposition,
    /// Stable blockers in policy-first order.
    pub blockers: Vec<ModelAcquisitionBlocker>,
    /// No source was opened by this operation.
    pub source_opened: bool,
    /// No destination was changed by this operation.
    pub destination_changed: bool,
}

/// Builds an exact review and evaluates machine fit without acquiring bytes.
#[must_use]
pub fn preflight_model_acquisition(
    profile: &ExactModelProfile,
    host: &ModelAcquisitionHost,
) -> ModelAcquisitionPreflight {
    let minimum_model_store_bytes = profile
        .artifact
        .bytes
        .saturating_mul(2)
        .saturating_add(INSTALL_RESERVE_BYTES);
    let envelope = profile
        .hardware
        .iter()
        .find(|item| item.platform == host.platform && item.architecture == host.architecture);
    let review = ModelAcquisitionReview {
        profile_id: profile.profile_id.clone(),
        manifest_sha256: profile.manifest_sha256.clone(),
        display_name: profile.display_name.clone(),
        publisher: profile.artifact.publisher.clone(),
        publisher_control: profile.publisher_control.clone(),
        lineage: profile.lineage.clone(),
        license_spdx: profile.license_spdx.clone(),
        license_terms_sha256: profile.license_terms_sha256.clone(),
        source_revision: profile.artifact.source_revision.clone(),
        artifact_format: profile.artifact.format.clone(),
        artifact_bytes: profile.artifact.bytes,
        artifact_sha256: profile.artifact.sha256.clone(),
        quantization: profile.quantization.clone(),
        runtime: profile.runtime.clone(),
        maximum_context_tokens: profile.context.max_context_tokens,
        minimum_system_memory_bytes: envelope.map(|item| item.minimum_system_memory_bytes),
        minimum_accelerator_memory_bytes: envelope
            .map(|item| item.minimum_accelerator_memory_bytes),
        minimum_model_store_bytes,
    };
    let mut blockers = Vec::new();
    if profile.schema_version != 2
        || profile.lifecycle != ModelLifecycleState::Candidate
        || profile.enabled
        || profile.automatic_fallback
        || profile.modalities != [ModelModality::Text]
        || !valid_sha256(&profile.manifest_sha256)
    {
        blockers.push(ModelAcquisitionBlocker::ProfilePolicy);
    }
    if profile.artifact.format != "GGUF"
        || profile.artifact.bytes == 0
        || profile.artifact.bytes > MAX_ARTIFACT_BYTES
        || !valid_sha256(&profile.artifact.sha256)
    {
        blockers.push(ModelAcquisitionBlocker::ArtifactPolicy);
    }
    if profile.artifact.publisher.is_empty()
        || profile.publisher_control.is_empty()
        || profile.lineage.is_empty()
        || profile.license_spdx.is_empty()
        || !valid_sha256(&profile.license_terms_sha256)
        || profile.artifact.source_revision.is_empty()
    {
        blockers.push(ModelAcquisitionBlocker::ProvenancePolicy);
    }
    if envelope.is_none() {
        blockers.push(ModelAcquisitionBlocker::Platform);
    }
    if host.runtime.as_ref() != Some(&profile.runtime) {
        blockers.push(ModelAcquisitionBlocker::Runtime);
    }
    if host.requested_context_tokens == 0
        || host.requested_context_tokens > profile.context.max_context_tokens
    {
        blockers.push(ModelAcquisitionBlocker::Context);
    }
    if envelope.is_some_and(|item| host.system_memory_bytes < item.minimum_system_memory_bytes) {
        blockers.push(ModelAcquisitionBlocker::SystemMemory);
    }
    if envelope
        .is_some_and(|item| host.accelerator_memory_bytes < item.minimum_accelerator_memory_bytes)
    {
        blockers.push(ModelAcquisitionBlocker::AcceleratorMemory);
    }
    if host.model_store_available_bytes < minimum_model_store_bytes {
        blockers.push(ModelAcquisitionBlocker::Disk);
    }
    let policy_blocked = blockers.iter().any(|blocker| {
        matches!(
            blocker,
            ModelAcquisitionBlocker::ProfilePolicy
                | ModelAcquisitionBlocker::ArtifactPolicy
                | ModelAcquisitionBlocker::ProvenancePolicy
                | ModelAcquisitionBlocker::Platform
                | ModelAcquisitionBlocker::Runtime
                | ModelAcquisitionBlocker::Context
        )
    });
    let disposition = if blockers.is_empty() {
        ModelAcquisitionDisposition::Eligible
    } else if policy_blocked {
        ModelAcquisitionDisposition::Blocked
    } else {
        ModelAcquisitionDisposition::BlockedHardware
    };
    ModelAcquisitionPreflight {
        review,
        disposition,
        blockers,
        source_opened: false,
        destination_changed: false,
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{ExactModelProfile, ModelLifecycleState, PlatformFamily};
    use serde_json::Value;

    use super::{
        GIB, ModelAcquisitionBlocker, ModelAcquisitionDisposition, ModelAcquisitionHost,
        preflight_model_acquisition,
    };

    fn profile() -> ExactModelProfile {
        let catalog: Value = serde_json::from_str(include_str!(
            "../../../model-profiles/exact-profile-catalog.json"
        ))
        .expect("catalog JSON");
        serde_json::from_value(
            catalog["profiles"]
                .as_array()
                .expect("profiles")
                .iter()
                .find(|value| {
                    value["profile_id"]
                        == "muse-glimmer-30b-q4-k-m-text-8k-fedora-diagnostic-repeatability"
                })
                .expect("profile")
                .clone(),
        )
        .expect("exact profile")
    }

    fn host(profile: &ExactModelProfile) -> ModelAcquisitionHost {
        ModelAcquisitionHost {
            platform: PlatformFamily::Fedora,
            architecture: profile.runtime.architecture,
            system_memory_bytes: 32 * GIB,
            accelerator_memory_bytes: 20 * GIB,
            model_store_available_bytes: profile.artifact.bytes * 2 + GIB,
            requested_context_tokens: 8192,
            runtime: Some(profile.runtime.clone()),
        }
    }

    #[test]
    fn exact_boundary_is_eligible_and_review_is_complete_without_io() {
        let profile = profile();
        let result = preflight_model_acquisition(&profile, &host(&profile));
        assert_eq!(result.disposition, ModelAcquisitionDisposition::Eligible);
        assert!(result.blockers.is_empty());
        assert!(!result.source_opened);
        assert!(!result.destination_changed);
        assert_eq!(result.review.profile_id, profile.profile_id);
        assert_eq!(result.review.lineage, profile.lineage);
        assert_eq!(result.review.artifact_sha256, profile.artifact.sha256);
        assert_eq!(result.review.runtime, profile.runtime);
    }

    #[test]
    fn each_hardware_boundary_below_at_and_above_is_exact() {
        let profile = profile();
        let exact = host(&profile);
        for (blocker, mutate) in [
            (
                ModelAcquisitionBlocker::SystemMemory,
                (|value: &mut ModelAcquisitionHost| value.system_memory_bytes -= 1)
                    as fn(&mut ModelAcquisitionHost),
            ),
            (
                ModelAcquisitionBlocker::AcceleratorMemory,
                (|value: &mut ModelAcquisitionHost| value.accelerator_memory_bytes -= 1)
                    as fn(&mut ModelAcquisitionHost),
            ),
            (
                ModelAcquisitionBlocker::Disk,
                (|value: &mut ModelAcquisitionHost| value.model_store_available_bytes -= 1)
                    as fn(&mut ModelAcquisitionHost),
            ),
        ] {
            let mut below = exact.clone();
            mutate(&mut below);
            let result = preflight_model_acquisition(&profile, &below);
            assert_eq!(
                result.disposition,
                ModelAcquisitionDisposition::BlockedHardware
            );
            assert_eq!(result.blockers, [blocker]);
        }
        let at = preflight_model_acquisition(&profile, &exact);
        assert_eq!(at.disposition, ModelAcquisitionDisposition::Eligible);
        let mut above = exact;
        above.system_memory_bytes += 1;
        above.accelerator_memory_bytes += 1;
        above.model_store_available_bytes += 1;
        assert_eq!(
            preflight_model_acquisition(&profile, &above).disposition,
            ModelAcquisitionDisposition::Eligible
        );
    }

    #[test]
    fn platform_runtime_context_profile_and_artifact_drift_block_before_io() {
        let profile = profile();
        let mut changed_host = host(&profile);
        changed_host.platform = PlatformFamily::Ubuntu;
        changed_host.runtime = None;
        changed_host.requested_context_tokens = 8193;
        let result = preflight_model_acquisition(&profile, &changed_host);
        assert_eq!(result.disposition, ModelAcquisitionDisposition::Blocked);
        assert_eq!(
            result.blockers,
            [
                ModelAcquisitionBlocker::Platform,
                ModelAcquisitionBlocker::Runtime,
                ModelAcquisitionBlocker::Context,
            ]
        );
        assert!(!result.source_opened);
        assert!(!result.destination_changed);

        let mut changed_profile = profile;
        changed_profile.lifecycle = ModelLifecycleState::Evaluating;
        changed_profile.artifact.format = "executable".to_owned();
        changed_profile.license_terms_sha256 = "invalid".to_owned();
        let result = preflight_model_acquisition(&changed_profile, &host(&changed_profile));
        assert_eq!(result.disposition, ModelAcquisitionDisposition::Blocked);
        assert_eq!(
            &result.blockers[..3],
            [
                ModelAcquisitionBlocker::ProfilePolicy,
                ModelAcquisitionBlocker::ArtifactPolicy,
                ModelAcquisitionBlocker::ProvenancePolicy,
            ]
        );
    }
}
