//! Closed engineering-capability lifecycle and exact dependency qualification.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// Closed capability package lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityLifecycleState {
    /// Editable and inadmissible.
    Draft,
    /// Manifest is valid but not enabled.
    Admitted,
    /// Separately qualified and enabled.
    Enabled,
    /// Available with exact declared limitations.
    Degraded,
    /// Disabled without deletion.
    Disabled,
    /// Rejected immediately after integrity/security failure.
    Quarantined,
    /// Historical migration/removal record only.
    Retired,
}

/// Exact dependency reference and qualification state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityDependency {
    /// Stable dependency identity.
    pub capability_id: String,
    /// Exact version, never a range.
    pub version: String,
    /// Exact manifest digest.
    pub manifest_sha256: String,
    /// Current qualification evidence digest.
    pub qualification_sha256: String,
    /// Qualification is current and passed.
    pub qualified: bool,
}

/// Complete additive lifecycle fields around the executable manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityLifecycleManifest {
    /// Stable capability identity.
    pub capability_id: String,
    /// Exact semantic version.
    pub version: String,
    /// Closed workload classification.
    pub classification: String,
    /// Exact context ceiling.
    pub context_items: u32,
    /// Exact elapsed ceiling.
    pub elapsed_milliseconds: u64,
    /// Exact artifact ceiling.
    pub artifact_count: u32,
    /// Exact output ceiling.
    pub output_bytes: u64,
    /// Deterministic verifier identities.
    pub verifiers: Vec<String>,
    /// Synthetic fixture identities.
    pub fixtures: Vec<String>,
    /// Exact dependencies.
    pub dependencies: Vec<CapabilityDependency>,
    /// Declared degradation behavior.
    pub degradation: String,
    /// Declared compatibility contract.
    pub compatibility: String,
    /// Declared migration contract.
    pub migration: String,
    /// Declared removal contract.
    pub removal: String,
    /// Package lifecycle.
    pub lifecycle: CapabilityLifecycleState,
    /// Exact admission review digest.
    pub admission_sha256: String,
    /// Registration, prose, or model output attempted to mint authority.
    pub authority_minted: bool,
}

/// Stable lifecycle refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityLifecycleError {
    /// Manifest shape, identity, budget, or contract invalid.
    InvalidManifest,
    /// Dependency is absent, mismatched, or unqualified.
    DependencyUnavailable,
    /// Lifecycle does not permit invocation.
    Disabled,
    /// Duplicate exact package.
    Duplicate,
}

/// Lifecycle registry keyed by exact capability/version.
#[derive(Default)]
pub struct CapabilityLifecycleRegistry {
    manifests: BTreeMap<(String, String), CapabilityLifecycleManifest>,
}

impl CapabilityLifecycleRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Admits one valid package without granting authority or enabling it.
    pub fn admit(
        &mut self,
        manifest: CapabilityLifecycleManifest,
    ) -> Result<(), CapabilityLifecycleError> {
        validate_capability_lifecycle(&manifest)?;
        let key = (manifest.capability_id.clone(), manifest.version.clone());
        if self.manifests.insert(key, manifest).is_some() {
            return Err(CapabilityLifecycleError::Duplicate);
        }
        Ok(())
    }

    /// Resolves an exact package only when enabled and all dependencies remain qualified.
    pub fn resolve(
        &self,
        capability_id: &str,
        version: &str,
    ) -> Result<&CapabilityLifecycleManifest, CapabilityLifecycleError> {
        let manifest = self
            .manifests
            .get(&(capability_id.to_owned(), version.to_owned()))
            .ok_or(CapabilityLifecycleError::Disabled)?;
        if manifest.lifecycle != CapabilityLifecycleState::Enabled {
            return Err(CapabilityLifecycleError::Disabled);
        }
        if manifest
            .dependencies
            .iter()
            .any(|dependency| !dependency.qualified)
        {
            return Err(CapabilityLifecycleError::DependencyUnavailable);
        }
        Ok(manifest)
    }

    /// Immediately disables or quarantines one exact package without residual registration.
    pub fn transition(
        &mut self,
        capability_id: &str,
        version: &str,
        state: CapabilityLifecycleState,
    ) -> Result<(), CapabilityLifecycleError> {
        if !matches!(
            state,
            CapabilityLifecycleState::Disabled
                | CapabilityLifecycleState::Quarantined
                | CapabilityLifecycleState::Retired
        ) {
            return Err(CapabilityLifecycleError::InvalidManifest);
        }
        let mut manifest = self
            .manifests
            .remove(&(capability_id.to_owned(), version.to_owned()))
            .ok_or(CapabilityLifecycleError::Disabled)?;
        manifest.lifecycle = state;
        if state != CapabilityLifecycleState::Retired {
            self.manifests
                .insert((capability_id.to_owned(), version.to_owned()), manifest);
        }
        Ok(())
    }
}

/// Returns nine disabled candidate packages in stable order.
#[must_use]
pub fn initial_capability_candidates() -> Vec<CapabilityLifecycleManifest> {
    [
        "repository-inspection",
        "coding-change",
        "pull-request-review",
        "issue-bug-workflow",
        "test-diagnosis",
        "release-readiness",
        "document-workflow",
        "research",
        "whole-codebase-audit",
    ]
    .into_iter()
    .map(candidate)
    .collect()
}

/// Validates exact lifecycle metadata without treating it as runtime authority.
pub fn validate_capability_lifecycle(
    value: &CapabilityLifecycleManifest,
) -> Result<(), CapabilityLifecycleError> {
    let texts = [
        &value.capability_id,
        &value.version,
        &value.classification,
        &value.degradation,
        &value.compatibility,
        &value.migration,
        &value.removal,
    ];
    if texts.iter().any(|text| !valid_id(text))
        || value.context_items == 0
        || value.context_items > 4096
        || value.elapsed_milliseconds == 0
        || value.elapsed_milliseconds > 86_400_000
        || value.artifact_count == 0
        || value.artifact_count > 4096
        || value.output_bytes == 0
        || value.output_bytes > 1024 * 1024 * 1024
        || value.verifiers.is_empty()
        || value.fixtures.is_empty()
        || !unique(value.verifiers.iter().map(String::as_str))
        || !unique(value.fixtures.iter().map(String::as_str))
        || !valid_sha256(&value.admission_sha256)
        || value.authority_minted
        || value.dependencies.iter().any(|dependency| {
            !valid_id(&dependency.capability_id)
                || !valid_version(&dependency.version)
                || !valid_sha256(&dependency.manifest_sha256)
                || !valid_sha256(&dependency.qualification_sha256)
        })
        || !unique(
            value
                .dependencies
                .iter()
                .map(|item| item.capability_id.as_str()),
        )
    {
        return Err(CapabilityLifecycleError::InvalidManifest);
    }
    if !valid_version(&value.version) {
        return Err(CapabilityLifecycleError::InvalidManifest);
    }
    if value.lifecycle == CapabilityLifecycleState::Enabled
        && value
            .dependencies
            .iter()
            .any(|dependency| !dependency.qualified)
    {
        return Err(CapabilityLifecycleError::DependencyUnavailable);
    }
    Ok(())
}

fn candidate(name: &str) -> CapabilityLifecycleManifest {
    CapabilityLifecycleManifest {
        capability_id: format!("agentmage.{name}"),
        version: "1.0.0".to_owned(),
        classification: "local-proposal".to_owned(),
        context_items: 64,
        elapsed_milliseconds: 300_000,
        artifact_count: 32,
        output_bytes: 65_536,
        verifiers: vec!["deterministic-postcondition".to_owned()],
        fixtures: vec!["synthetic-v1".to_owned()],
        dependencies: vec![CapabilityDependency {
            capability_id: "agentmage.shared-runtime".to_owned(),
            version: "1.0.0".to_owned(),
            manifest_sha256: "a".repeat(64),
            qualification_sha256: "b".repeat(64),
            qualified: false,
        }],
        degradation: "block-with-attributable-dependency".to_owned(),
        compatibility: "exact-version-and-qualification".to_owned(),
        migration: "explicit-versioned-migration".to_owned(),
        removal: "remove-registration-grant-worker-cache".to_owned(),
        lifecycle: CapabilityLifecycleState::Disabled,
        admission_sha256: "c".repeat(64),
        authority_minted: false,
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}
fn valid_version(value: &str) -> bool {
    value.split('.').count() == 3 && value.split('.').all(|part| part.parse::<u32>().is_ok())
}
fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
fn unique<'a>(values: impl Iterator<Item = &'a str>) -> bool {
    let values = values.collect::<Vec<_>>();
    values.iter().copied().collect::<BTreeSet<_>>().len() == values.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nine_candidates_are_exact_disabled_and_authority_free() {
        let candidates = initial_capability_candidates();
        assert_eq!(candidates.len(), 9);
        assert!(
            candidates
                .iter()
                .all(|item| item.lifecycle == CapabilityLifecycleState::Disabled
                    && !item.authority_minted
                    && validate_capability_lifecycle(item).is_ok())
        );
    }
    #[test]
    fn admission_does_not_enable_and_exact_resolution_blocks() {
        let mut registry = CapabilityLifecycleRegistry::new();
        let candidate = initial_capability_candidates().remove(0);
        let id = candidate.capability_id.clone();
        registry.admit(candidate).unwrap();
        assert_eq!(
            registry.resolve(&id, "1.0.0"),
            Err(CapabilityLifecycleError::Disabled)
        );
    }
    #[test]
    fn unqualified_dependency_and_authority_minting_fail_closed() {
        let mut candidate = initial_capability_candidates().remove(0);
        candidate.lifecycle = CapabilityLifecycleState::Enabled;
        assert_eq!(
            validate_capability_lifecycle(&candidate),
            Err(CapabilityLifecycleError::DependencyUnavailable)
        );
        candidate.lifecycle = CapabilityLifecycleState::Disabled;
        candidate.authority_minted = true;
        assert_eq!(
            validate_capability_lifecycle(&candidate),
            Err(CapabilityLifecycleError::InvalidManifest)
        );
    }
    #[test]
    fn quarantine_disable_and_retirement_remove_invocation() {
        for state in [
            CapabilityLifecycleState::Disabled,
            CapabilityLifecycleState::Quarantined,
            CapabilityLifecycleState::Retired,
        ] {
            let mut registry = CapabilityLifecycleRegistry::new();
            let candidate = initial_capability_candidates().remove(0);
            let id = candidate.capability_id.clone();
            registry.admit(candidate).unwrap();
            registry.transition(&id, "1.0.0", state).unwrap();
            assert_eq!(
                registry.resolve(&id, "1.0.0"),
                Err(CapabilityLifecycleError::Disabled)
            );
        }
    }
}
