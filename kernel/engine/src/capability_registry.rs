//! Versioned executable capability admission and deterministic execution.

use std::collections::{BTreeMap, BTreeSet};

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, CapabilityId, CapabilityManifest, EvidenceReference, ToolId,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_CAPABILITIES: usize = 256;
const MAX_CAPABILITY_ITEMS: usize = 64;
const MAX_CAPABILITY_TEXT_BYTES: usize = 128;
const ALLOWED_AUTHORITY_CLASSES: [&str; 6] = [
    "read_only",
    "controlled_write",
    "controlled_command",
    "controlled_git",
    "network_read",
    "remote_write",
];

/// Stable capability-admission or execution refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityRegistryError {
    /// Manifest shape, version, digest, or closed values are invalid.
    InvalidManifest,
    /// The capability identity and version are already registered.
    Duplicate,
    /// Registry capacity was reached.
    Capacity,
    /// The requested capability is absent.
    NotFound,
    /// A required workflow step failed.
    StepFailed,
    /// Deterministic postconditions did not pass.
    VerificationFailed,
}

impl CapabilityRegistryError {
    /// Returns one stable redacted error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidManifest => "capability-registry.manifest.invalid",
            Self::Duplicate => "capability-registry.manifest.duplicate",
            Self::Capacity => "capability-registry.capacity.exceeded",
            Self::NotFound => "capability-registry.capability.not-found",
            Self::StepFailed => "capability-registry.step.failed",
            Self::VerificationFailed => "capability-registry.verification.failed",
        }
    }
}

/// One deterministic step result returned by the runtime implementation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityStepResult {
    /// Exact workflow-step identity.
    pub step_id: String,
    /// Deterministic evidence created by this step.
    pub evidence: Vec<EvidenceReference>,
}

/// Complete verified capability execution result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityExecutionResult {
    /// Exact capability identity.
    pub capability_id: CapabilityId,
    /// Exact manifest version.
    pub version: String,
    /// Ordered step results.
    pub steps: Vec<CapabilityStepResult>,
    /// Complete evidence accepted by the deterministic verifier.
    pub evidence: Vec<EvidenceReference>,
    /// Digest of the manifest executed.
    pub manifest_sha256: String,
}

/// Runtime boundary for one manifest-defined workflow.
pub trait CapabilityExecutionPort {
    /// Executes one exact declared step without expanding manifest authority.
    fn execute_step(
        &mut self,
        manifest: &CapabilityManifest,
        step_id: &str,
    ) -> Result<CapabilityStepResult, CapabilityRegistryError>;

    /// Verifies every declared postcondition from current exact evidence.
    fn verify_postconditions(
        &mut self,
        manifest: &CapabilityManifest,
        evidence: &[EvidenceReference],
    ) -> Result<bool, CapabilityRegistryError>;
}

/// Immutable capability registry keyed by stable ID and semantic version.
#[derive(Default)]
pub struct CapabilityRegistry {
    manifests: BTreeMap<(CapabilityId, String), CapabilityManifest>,
}

impl CapabilityRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers one verified manifest without granting or executing authority.
    pub fn register(
        &mut self,
        manifest: CapabilityManifest,
    ) -> Result<(), CapabilityRegistryError> {
        verify_capability_manifest(&manifest)?;
        if self.manifests.len() >= MAX_CAPABILITIES {
            return Err(CapabilityRegistryError::Capacity);
        }
        let key = (manifest.capability_id.clone(), manifest.version.clone());
        if self.manifests.contains_key(&key) {
            return Err(CapabilityRegistryError::Duplicate);
        }
        self.manifests.insert(key, manifest);
        Ok(())
    }

    /// Returns one exact registered manifest.
    #[must_use]
    pub fn get(&self, capability_id: &CapabilityId, version: &str) -> Option<&CapabilityManifest> {
        self.manifests
            .get(&(capability_id.clone(), version.to_owned()))
    }

    /// Executes one exact registered manifest through a separately supplied runtime port.
    pub fn execute<P: CapabilityExecutionPort>(
        &self,
        capability_id: &CapabilityId,
        version: &str,
        port: &mut P,
    ) -> Result<CapabilityExecutionResult, CapabilityRegistryError> {
        let manifest = self
            .get(capability_id, version)
            .ok_or(CapabilityRegistryError::NotFound)?;
        let mut steps = Vec::with_capacity(manifest.workflow_steps.len());
        let mut evidence = Vec::new();
        for step_id in &manifest.workflow_steps {
            let result = port.execute_step(manifest, step_id)?;
            if result.step_id != *step_id {
                return Err(CapabilityRegistryError::StepFailed);
            }
            evidence.extend(result.evidence.iter().cloned());
            steps.push(result);
        }
        if !port.verify_postconditions(manifest, &evidence)? {
            return Err(CapabilityRegistryError::VerificationFailed);
        }
        Ok(CapabilityExecutionResult {
            capability_id: manifest.capability_id.clone(),
            version: manifest.version.clone(),
            steps,
            evidence,
            manifest_sha256: manifest.manifest_sha256.clone(),
        })
    }

    /// Returns all manifests in stable ID and version order.
    #[must_use]
    pub fn manifests(&self) -> Vec<&CapabilityManifest> {
        self.manifests.values().collect()
    }
}

/// Seals one otherwise valid executable capability manifest.
pub fn seal_capability_manifest(
    mut manifest: CapabilityManifest,
) -> Result<CapabilityManifest, CapabilityRegistryError> {
    manifest.manifest_sha256 = ZERO_SHA256.to_owned();
    validate_manifest_fields(&manifest)?;
    manifest.manifest_sha256 = canonical_sha256(&manifest)?;
    Ok(manifest)
}

/// Verifies one exact capability manifest and its immutable digest.
pub fn verify_capability_manifest(
    manifest: &CapabilityManifest,
) -> Result<(), CapabilityRegistryError> {
    validate_manifest_fields(manifest)?;
    let mut candidate = manifest.clone();
    candidate.manifest_sha256 = ZERO_SHA256.to_owned();
    if manifest.manifest_sha256 != canonical_sha256(&candidate)? {
        return Err(CapabilityRegistryError::InvalidManifest);
    }
    Ok(())
}

/// Returns the initial Repository Review capability as a real executable manifest.
pub fn repository_review_manifest() -> CapabilityManifest {
    seal_capability_manifest(CapabilityManifest {
        schema_version: CONTRACT_SCHEMA_VERSION,
        capability_id: CapabilityId::from_raw("agentmage.repository-review"),
        version: "1.0.0".to_owned(),
        display_name: "Repository Review".to_owned(),
        input_schema_sha256: "1".repeat(64),
        output_schema_sha256: "2".repeat(64),
        required_tools: vec![
            ToolId::from_raw("repository-map"),
            ToolId::from_raw("repository-read"),
            ToolId::from_raw("git-inspect"),
        ],
        required_model_roles: vec!["repository_review".to_owned()],
        authority_classes: vec!["read_only".to_owned()],
        workflow_steps: vec![
            "inventory".to_owned(),
            "inspect".to_owned(),
            "analyze".to_owned(),
            "verify-report".to_owned(),
        ],
        postconditions: vec![
            "report-cites-observed-files".to_owned(),
            "findings-bind-current-snapshot".to_owned(),
        ],
        approval_points: Vec::new(),
        max_turns: 16,
        max_workers: 1,
        manifest_sha256: ZERO_SHA256.to_owned(),
    })
    .expect("built-in Repository Review manifest is static and valid")
}

fn validate_manifest_fields(manifest: &CapabilityManifest) -> Result<(), CapabilityRegistryError> {
    if manifest.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_identifier(manifest.capability_id.as_str())
        || !valid_semver(&manifest.version)
        || !valid_text(&manifest.display_name)
        || !valid_sha256(&manifest.input_schema_sha256)
        || !valid_sha256(&manifest.output_schema_sha256)
        || !valid_sha256(&manifest.manifest_sha256)
        || manifest.required_tools.is_empty()
        || manifest.required_tools.len() > MAX_CAPABILITY_ITEMS
        || manifest.required_model_roles.is_empty()
        || manifest.required_model_roles.len() > MAX_CAPABILITY_ITEMS
        || manifest.authority_classes.is_empty()
        || manifest.authority_classes.len() > MAX_CAPABILITY_ITEMS
        || manifest.workflow_steps.is_empty()
        || manifest.workflow_steps.len() > MAX_CAPABILITY_ITEMS
        || manifest.postconditions.is_empty()
        || manifest.postconditions.len() > MAX_CAPABILITY_ITEMS
        || manifest.approval_points.len() > MAX_CAPABILITY_ITEMS
        || manifest.max_turns == 0
        || manifest.max_workers == 0
        || manifest.max_workers > 5
    {
        return Err(CapabilityRegistryError::InvalidManifest);
    }
    if !unique(manifest.required_tools.iter().map(ToolId::as_str))
        || !unique(manifest.required_model_roles.iter().map(String::as_str))
        || !unique(manifest.authority_classes.iter().map(String::as_str))
        || !unique(manifest.workflow_steps.iter().map(String::as_str))
        || !unique(manifest.postconditions.iter().map(String::as_str))
        || !unique(manifest.approval_points.iter().map(String::as_str))
        || manifest
            .required_tools
            .iter()
            .any(|item| !valid_identifier(item.as_str()))
        || manifest
            .required_model_roles
            .iter()
            .any(|item| !valid_identifier(item))
        || manifest
            .authority_classes
            .iter()
            .any(|item| !ALLOWED_AUTHORITY_CLASSES.contains(&item.as_str()))
        || manifest
            .workflow_steps
            .iter()
            .any(|item| !valid_identifier(item))
        || manifest
            .postconditions
            .iter()
            .any(|item| !valid_identifier(item))
        || manifest
            .approval_points
            .iter()
            .any(|item| !valid_identifier(item))
    {
        return Err(CapabilityRegistryError::InvalidManifest);
    }
    Ok(())
}

fn unique<'a>(mut values: impl Iterator<Item = &'a str>) -> bool {
    let mut seen = BTreeSet::new();
    values.all(|value| seen.insert(value))
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_CAPABILITY_TEXT_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

fn valid_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_CAPABILITY_TEXT_BYTES && !value.contains('\0')
}

fn valid_semver(value: &str) -> bool {
    let parts = value.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn canonical_sha256<T: Serialize>(value: &T) -> Result<String, CapabilityRegistryError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| CapabilityRegistryError::InvalidManifest)
}

fn sha256(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    let digest = digest.finalize();
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{
        CapabilityExecutionPort, CapabilityRegistry, CapabilityRegistryError, CapabilityStepResult,
        repository_review_manifest, seal_capability_manifest,
    };
    use agentmage_kernel_contracts::{CapabilityManifest, EvidenceReference};

    struct FakeRuntime;

    impl CapabilityExecutionPort for FakeRuntime {
        fn execute_step(
            &mut self,
            _manifest: &CapabilityManifest,
            step_id: &str,
        ) -> Result<CapabilityStepResult, CapabilityRegistryError> {
            Ok(CapabilityStepResult {
                step_id: step_id.to_owned(),
                evidence: Vec::<EvidenceReference>::new(),
            })
        }

        fn verify_postconditions(
            &mut self,
            _manifest: &CapabilityManifest,
            _evidence: &[EvidenceReference],
        ) -> Result<bool, CapabilityRegistryError> {
            Ok(true)
        }
    }

    #[test]
    fn repository_review_executes_from_the_registered_manifest() {
        let manifest = repository_review_manifest();
        let capability_id = manifest.capability_id.clone();
        let mut registry = CapabilityRegistry::new();
        registry.register(manifest).unwrap();
        let result = registry
            .execute(&capability_id, "1.0.0", &mut FakeRuntime)
            .unwrap();
        assert_eq!(result.steps.len(), 4);
        assert_eq!(result.capability_id, capability_id);
    }

    #[test]
    fn manifest_cannot_invent_an_authority_class_or_exceed_team_limit() {
        let mut manifest = repository_review_manifest();
        manifest.authority_classes = vec!["mint_grant".to_owned()];
        manifest.max_workers = 6;
        manifest.manifest_sha256 = "0".repeat(64);
        assert_eq!(
            seal_capability_manifest(manifest),
            Err(CapabilityRegistryError::InvalidManifest)
        );
    }
}
