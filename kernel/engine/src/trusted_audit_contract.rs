//! Closed trusted-operation and whole-codebase audit contracts.
#![allow(missing_docs)]
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrustedCapability {
    Command,
    Research,
    Credential,
    Continuity,
    ModelManager,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityLifecycle {
    Enabled,
    Starting,
    Active,
    Cancelling,
    Uncertain,
    Reconciling,
    Stopped,
    Disabled,
    Removed,
    Recovered,
    Residue,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformKind {
    Linux,
    Windows,
    MacOsRetained,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathDisposition {
    Analyzed,
    Generated,
    Vendored,
    Binary,
    Excluded,
    Unavailable,
    Unsupported,
    Changed,
    Failed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryRecordKind {
    File,
    Symbol,
    Module,
    GraphEdge,
    SemanticPacket,
    EvidenceCard,
    Contradiction,
    Finding,
    Checkpoint,
    Invalidation,
    Coverage,
    Resource,
    Report,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustedOperation {
    pub schema_version: u16,
    pub operation_id: String,
    pub capability: TrustedCapability,
    pub worker_id: String,
    pub authority_level: String,
    pub input_digest: String,
    pub limit_profile_id: String,
    pub lifecycle: CapabilityLifecycle,
    pub receipt_schema_id: String,
    pub grant_ids: BTreeSet<String>,
    pub credential_references: BTreeSet<String>,
    pub network_classes: BTreeSet<String>,
    pub storage_classes: BTreeSet<String>,
    pub policy_ids: BTreeSet<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopologyEntry {
    pub platform: PlatformKind,
    pub component_id: String,
    pub process_id: String,
    pub package_id: String,
    pub ipc_edges: BTreeSet<String>,
    pub sockets: BTreeSet<String>,
    pub writable_paths: BTreeSet<String>,
    pub network_destinations: BTreeSet<String>,
    pub secret_edges: BTreeSet<String>,
    pub durable_objects: BTreeSet<String>,
    pub cleanup_owner: String,
    pub removal_disposition: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditRequest {
    pub schema_version: u16,
    pub audit_id: String,
    pub repository_root_digest: String,
    pub revision: String,
    pub index_digest: String,
    pub dirty: bool,
    pub include_rules: BTreeSet<String>,
    pub exclude_rules: BTreeSet<String>,
    pub depth: u32,
    pub languages: BTreeSet<String>,
    pub parser_catalog_digest: String,
    pub include_history: bool,
    pub include_submodules: bool,
    pub include_worktrees: bool,
    pub command_policy_id: String,
    pub network_policy_id: String,
    pub resource_profile_id: String,
    pub retention_policy_id: String,
    pub cancellation_id: String,
    pub output_schema_id: String,
    pub platform_id: String,
    pub release_id: String,
    pub policy_digest: String,
    pub model_runtime_profile_id: String,
    pub start_state_digest: String,
    pub predecessor_audit_id: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditMemoryRecord {
    pub record_id: String,
    pub kind: MemoryRecordKind,
    pub source_path_digest: String,
    pub source_sha256: String,
    pub start_byte: u64,
    pub end_byte: u64,
    pub provenance_digest: String,
    pub observed: bool,
    pub inferred: bool,
    pub confidence_basis_points: u16,
    pub uncertainty_codes: BTreeSet<String>,
    pub counterevidence_ids: BTreeSet<String>,
    pub reverse_dependency_ids: BTreeSet<String>,
    pub current: bool,
    pub canonical_identity_from_source: bool,
    pub authority_count: u32,
    pub completeness_claim: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditCoverage {
    pub audit_id: String,
    pub path_digest: String,
    pub disposition: PathDisposition,
    pub reason_code: Option<String>,
    pub evidence_digest: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrustedAuditError {
    Invalid,
    Union,
    UnsupportedVersion,
    Coverage,
    Authority,
}
fn id(v: &str) -> bool {
    !v.is_empty() && v.len() <= 2048
}
fn digest(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
pub fn validate_operation(v: &TrustedOperation) -> Result<(), TrustedAuditError> {
    if v.schema_version != 1 {
        return Err(TrustedAuditError::UnsupportedVersion);
    }
    if !id(&v.operation_id)
        || !id(&v.worker_id)
        || !id(&v.authority_level)
        || !digest(&v.input_digest)
        || !id(&v.limit_profile_id)
        || !id(&v.receipt_schema_id)
    {
        return Err(TrustedAuditError::Invalid);
    }
    let union = v.grant_ids.len() > 1
        || v.credential_references.len() > 1
        || v.network_classes.len() > 1
        || v.storage_classes.len() > 1
        || v.policy_ids.len() != 1;
    if union {
        return Err(TrustedAuditError::Union);
    }
    Ok(())
}
pub fn validate_topology(v: &TopologyEntry) -> Result<(), TrustedAuditError> {
    if !id(&v.component_id)
        || !id(&v.process_id)
        || !id(&v.package_id)
        || !id(&v.cleanup_owner)
        || !id(&v.removal_disposition)
    {
        return Err(TrustedAuditError::Invalid);
    }
    Ok(())
}
pub fn validate_audit_request(v: &AuditRequest) -> Result<(), TrustedAuditError> {
    if v.schema_version != 1 {
        return Err(TrustedAuditError::UnsupportedVersion);
    }
    if !id(&v.audit_id)
        || !digest(&v.repository_root_digest)
        || !id(&v.revision)
        || !digest(&v.index_digest)
        || v.include_rules.is_empty()
        || v.depth == 0
        || v.languages.is_empty()
        || !digest(&v.parser_catalog_digest)
        || !id(&v.command_policy_id)
        || !id(&v.network_policy_id)
        || !id(&v.resource_profile_id)
        || !id(&v.retention_policy_id)
        || !id(&v.cancellation_id)
        || !id(&v.output_schema_id)
        || !id(&v.platform_id)
        || !id(&v.release_id)
        || !digest(&v.policy_digest)
        || !id(&v.model_runtime_profile_id)
        || !digest(&v.start_state_digest)
    {
        return Err(TrustedAuditError::Invalid);
    }
    Ok(())
}
pub fn validate_coverage(v: &AuditCoverage) -> Result<(), TrustedAuditError> {
    if !id(&v.audit_id) || !digest(&v.path_digest) {
        return Err(TrustedAuditError::Coverage);
    }
    if v.disposition == PathDisposition::Analyzed {
        if v.evidence_digest.as_deref().is_none_or(|d| !digest(d)) {
            return Err(TrustedAuditError::Coverage);
        }
    } else if v.reason_code.as_deref().is_none_or(|r| !id(r)) {
        return Err(TrustedAuditError::Coverage);
    }
    Ok(())
}
pub fn validate_memory(v: &AuditMemoryRecord) -> Result<(), TrustedAuditError> {
    if !id(&v.record_id)
        || !digest(&v.source_path_digest)
        || !digest(&v.source_sha256)
        || v.start_byte >= v.end_byte
        || !digest(&v.provenance_digest)
        || v.observed == v.inferred
        || v.confidence_basis_points > 10_000
        || !v.canonical_identity_from_source
    {
        return Err(TrustedAuditError::Invalid);
    }
    if v.authority_count != 0 || v.completeness_claim {
        return Err(TrustedAuditError::Authority);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn op() -> TrustedOperation {
        TrustedOperation {
            schema_version: 1,
            operation_id: "op".into(),
            capability: TrustedCapability::Research,
            worker_id: "worker".into(),
            authority_level: "network-read".into(),
            input_digest: "a".repeat(64),
            limit_profile_id: "limits".into(),
            lifecycle: CapabilityLifecycle::Enabled,
            receipt_schema_id: "receipt".into(),
            grant_ids: BTreeSet::new(),
            credential_references: BTreeSet::new(),
            network_classes: BTreeSet::from(["public".into()]),
            storage_classes: BTreeSet::new(),
            policy_ids: BTreeSet::from(["policy".into()]),
        }
    }
    #[test]
    fn exact_capability_passes() {
        assert_eq!(validate_operation(&op()), Ok(()))
    }
    #[test]
    fn authority_union_fails() {
        let mut v = op();
        v.network_classes.insert("private".into());
        assert_eq!(validate_operation(&v), Err(TrustedAuditError::Union))
    }
    #[test]
    fn coverage_never_hides_omission() {
        let v = AuditCoverage {
            audit_id: "audit".into(),
            path_digest: "a".repeat(64),
            disposition: PathDisposition::Failed,
            reason_code: None,
            evidence_digest: None,
        };
        assert_eq!(validate_coverage(&v), Err(TrustedAuditError::Coverage))
    }
    #[test]
    fn model_memory_has_no_authority() {
        let v = AuditMemoryRecord {
            record_id: "r".into(),
            kind: MemoryRecordKind::EvidenceCard,
            source_path_digest: "a".repeat(64),
            source_sha256: "b".repeat(64),
            start_byte: 1,
            end_byte: 2,
            provenance_digest: "c".repeat(64),
            observed: true,
            inferred: false,
            confidence_basis_points: 10_000,
            uncertainty_codes: BTreeSet::new(),
            counterevidence_ids: BTreeSet::new(),
            reverse_dependency_ids: BTreeSet::new(),
            current: true,
            canonical_identity_from_source: true,
            authority_count: 0,
            completeness_claim: false,
        };
        assert_eq!(validate_memory(&v), Ok(()))
    }
}
