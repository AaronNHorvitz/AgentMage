//! Signed-package-bound production bootstrap for native model discovery.

use agentmage_kernel_contracts::{
    ExactModelProfile, ModelActivationState, ModelCompatibilityState, ModelHealth,
    ModelHealthState, ModelPickerSnapshot, ModelSupportState,
};
use agentmage_kernel_engine::model_discovery::{
    ModelDiscoveryCandidate, build_model_picker_snapshot,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::package_verify::VerifiedPackageRoot;

const CATALOG_PATH: &str = "usr/share/agentmage/model-profiles/exact-profile-catalog.json";
const MAX_CATALOG_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExactProfileCatalog {
    schema_version: u16,
    policy_path: String,
    policy_sha256: String,
    enabled_profile_count: usize,
    historical_records: Vec<HistoricalRecord>,
    profiles: Vec<ExactModelProfile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HistoricalRecord {
    profile_family: String,
    path: String,
    sha256: String,
    disposition: String,
    preserved: bool,
}

/// Stable fail-closed catalog bootstrap result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ModelCatalogBootstrapError {
    PackageBinding,
    CatalogInvalid,
    ProjectionInvalid,
}

impl ModelCatalogBootstrapError {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::PackageBinding => "agentmage.model-catalog.package-binding-denied",
            Self::CatalogInvalid => "agentmage.model-catalog.invalid",
            Self::ProjectionInvalid => "agentmage.model-catalog.projection-invalid",
        }
    }
}

/// Loads the exact profile catalog through a continuously held signed-package proof.
///
/// Startup does not infer activation, compatibility, support, or runtime readiness. Until those
/// current observers are composed, every valid signed profile remains management-only.
pub(crate) fn load_signed_model_picker_snapshot(
    package: &VerifiedPackageRoot,
    observed_at_ms: u64,
) -> Result<ModelPickerSnapshot, ModelCatalogBootstrapError> {
    if observed_at_ms == 0 || package.manifest_sha256().len() != 64 {
        return Err(ModelCatalogBootstrapError::PackageBinding);
    }
    let bytes = package
        .read_verified_file(CATALOG_PATH, MAX_CATALOG_BYTES)
        .map_err(|_| ModelCatalogBootstrapError::PackageBinding)?;
    let catalog: ExactProfileCatalog =
        serde_json::from_slice(&bytes).map_err(|_| ModelCatalogBootstrapError::CatalogInvalid)?;
    validate_catalog(&catalog)?;
    let digest = lower_hex(&Sha256::digest(&bytes));
    let candidates = catalog
        .profiles
        .into_iter()
        .map(|profile| ModelDiscoveryCandidate {
            runtime_health: ModelHealth {
                adapter_id: profile.runtime.adapter_id.clone(),
                profile_id: Some(profile.profile_id.clone()),
                state: ModelHealthState::Unloaded,
                reason_code: "model.bootstrap.runtime-not-observed".to_owned(),
                observed_at_ms,
            },
            profile,
            activation: ModelActivationState::Inactive,
            compatibility: ModelCompatibilityState::Blocked,
            support: ModelSupportState::Unsupported,
            policy_current: true,
            limitations: vec![
                "model.bootstrap.activation-absent".to_owned(),
                "model.bootstrap.compatibility-not-observed".to_owned(),
                "model.bootstrap.runtime-not-observed".to_owned(),
            ],
            requires_user_decision: true,
        })
        .collect();
    build_model_picker_snapshot(digest, true, observed_at_ms, candidates)
        .map_err(|_| ModelCatalogBootstrapError::ProjectionInvalid)
}

fn validate_catalog(catalog: &ExactProfileCatalog) -> Result<(), ModelCatalogBootstrapError> {
    let enabled = catalog
        .profiles
        .iter()
        .filter(|profile| profile.enabled)
        .count();
    let historical_valid = !catalog.historical_records.is_empty()
        && catalog.historical_records.iter().all(|record| {
            !record.profile_family.is_empty()
                && record.path.starts_with("model-profiles/candidates/")
                && record.path.ends_with(".json")
                && valid_sha256(&record.sha256)
                && matches!(record.disposition.as_str(), "BLOCKED" | "REJECTED")
                && record.preserved
        });
    if catalog.schema_version != 1
        || catalog.policy_path != "MODEL-PROVENANCE-POLICY.md"
        || !valid_sha256(&catalog.policy_sha256)
        || catalog.profiles.is_empty()
        || catalog.profiles.len() > 64
        || enabled != catalog.enabled_profile_count
        || catalog
            .profiles
            .iter()
            .any(|profile| profile.policy_sha256 != catalog.policy_sha256)
        || !historical_valid
    {
        return Err(ModelCatalogBootstrapError::CatalogInvalid);
    }
    Ok(())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn lower_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}
