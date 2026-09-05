//! Disposable, authority-free experimental model laboratory.
#![forbid(unsafe_code)]
#![allow(missing_docs)]
use std::collections::BTreeSet;

pub const PROCESS_ID: &str = "agentmage-experimental-model-lab";
pub const DATA_ROOT_CLASS: &str = "experimental-model-lab";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuthoritySurface {
    pub network: bool,
    pub credentials: bool,
    pub commands: bool,
    pub connectors: bool,
    pub messaging: bool,
    pub finance: bool,
    pub delivery: bool,
    pub cloud: bool,
    pub backup: bool,
    pub operational_memory: bool,
    pub approved_store: bool,
    pub canonical_workspace_write: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExperimentalArtifact {
    pub artifact_id: String,
    pub user_source_digest: String,
    pub artifact_sha256: String,
    pub observed_license_digest: Option<String>,
    pub developer: String,
    pub publisher: String,
    pub provenance_gap_codes: BTreeSet<String>,
    pub format: String,
    pub transformation_sha256: String,
    pub warning_codes: BTreeSet<String>,
    pub policy_eligible: bool,
    pub lineage_verified: bool,
    pub experimental_label: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceLimits {
    pub processor_millis: u64,
    pub memory_bytes: u64,
    pub graphics_bytes: u64,
    pub disk_bytes: u64,
    pub context_tokens: u64,
    pub output_tokens: u64,
    pub duration_millis: u64,
    pub process_count: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabStage {
    Imported,
    Quarantined,
    Parsed,
    Loaded,
    Evaluated,
    Cancelling,
    Cleaned,
    Removed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LabObservation {
    pub stage: LabStage,
    pub within_limits: bool,
    pub hostile_tool_request: bool,
    pub exfiltration_request: bool,
    pub prompt_injection: bool,
    pub interrupted: bool,
    pub process_count: u32,
    pub socket_count: u32,
    pub residue_count: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabError {
    Invalid,
    ProhibitedAuthority,
    PolicyExcluded,
    LimitExceeded,
    HostileOutput,
    Interrupted,
    Residue,
}
fn id(v: &str) -> bool {
    !v.is_empty() && v.len() <= 4096
}
fn digest(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
pub fn validate_authority(v: AuthoritySurface) -> Result<(), LabError> {
    if v.network
        || v.credentials
        || v.commands
        || v.connectors
        || v.messaging
        || v.finance
        || v.delivery
        || v.cloud
        || v.backup
        || v.operational_memory
        || v.approved_store
        || v.canonical_workspace_write
    {
        Err(LabError::ProhibitedAuthority)
    } else {
        Ok(())
    }
}
pub fn validate_artifact(v: &ExperimentalArtifact) -> Result<(), LabError> {
    if !id(&v.artifact_id)
        || !digest(&v.user_source_digest)
        || !digest(&v.artifact_sha256)
        || v.observed_license_digest
            .as_deref()
            .is_some_and(|x| !digest(x))
        || !id(&v.developer)
        || !id(&v.publisher)
        || !id(&v.format)
        || !digest(&v.transformation_sha256)
        || !v.experimental_label
    {
        return Err(LabError::Invalid);
    }
    if !v.policy_eligible || !v.lineage_verified {
        return Err(LabError::PolicyExcluded);
    }
    Ok(())
}
pub fn validate_limits(v: ResourceLimits) -> Result<(), LabError> {
    if v.processor_millis == 0
        || v.memory_bytes == 0
        || v.graphics_bytes == 0
        || v.disk_bytes == 0
        || v.context_tokens == 0
        || v.output_tokens == 0
        || v.duration_millis == 0
        || v.process_count == 0
    {
        Err(LabError::Invalid)
    } else {
        Ok(())
    }
}
pub fn observe(v: LabObservation) -> Result<(), LabError> {
    if !v.within_limits {
        return Err(LabError::LimitExceeded);
    }
    if v.hostile_tool_request || v.exfiltration_request || v.prompt_injection {
        return Err(LabError::HostileOutput);
    }
    if v.interrupted {
        return Err(LabError::Interrupted);
    }
    if matches!(v.stage, LabStage::Cleaned | LabStage::Removed)
        && (v.process_count != 0 || v.socket_count != 0 || v.residue_count != 0)
    {
        return Err(LabError::Residue);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authority_is_structurally_empty() {
        let v = AuthoritySurface {
            network: false,
            credentials: false,
            commands: false,
            connectors: false,
            messaging: false,
            finance: false,
            delivery: false,
            cloud: false,
            backup: false,
            operational_memory: false,
            approved_store: false,
            canonical_workspace_write: false,
        };
        assert_eq!(validate_authority(v), Ok(()));
        assert_eq!(
            validate_authority(AuthoritySurface { network: true, ..v }),
            Err(LabError::ProhibitedAuthority)
        );
    }
    #[test]
    fn unverifiable_lineage_is_blocked() {
        let mut v = artifact();
        v.lineage_verified = false;
        assert_eq!(validate_artifact(&v), Err(LabError::PolicyExcluded));
    }
    #[test]
    fn hostile_output_and_removal_residue_are_inert() {
        let mut v = observation(LabStage::Evaluated);
        v.hostile_tool_request = true;
        assert_eq!(observe(v), Err(LabError::HostileOutput));
        let mut v = observation(LabStage::Removed);
        v.residue_count = 1;
        assert_eq!(observe(v), Err(LabError::Residue));
    }
    fn artifact() -> ExperimentalArtifact {
        ExperimentalArtifact {
            artifact_id: "artifact".into(),
            user_source_digest: "a".repeat(64),
            artifact_sha256: "b".repeat(64),
            observed_license_digest: None,
            developer: "unknown".into(),
            publisher: "unknown".into(),
            provenance_gap_codes: BTreeSet::from(["license-unobserved".into()]),
            format: "gguf".into(),
            transformation_sha256: "c".repeat(64),
            warning_codes: BTreeSet::from(["experimental".into()]),
            policy_eligible: true,
            lineage_verified: true,
            experimental_label: true,
        }
    }
    fn observation(stage: LabStage) -> LabObservation {
        LabObservation {
            stage,
            within_limits: true,
            hostile_tool_request: false,
            exfiltration_request: false,
            prompt_injection: false,
            interrupted: false,
            process_count: 0,
            socket_count: 0,
            residue_count: 0,
        }
    }
}
