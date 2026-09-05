//! Truthful exact-profile catalog and model-independent semantic structure contracts.
#![allow(missing_docs)]
use std::collections::BTreeSet;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatalogState {
    Candidate,
    Evaluating,
    Approved,
    Degraded,
    Quarantined,
    Rejected,
    Retired,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compatibility {
    Compatible,
    CompatibleWithLimitations,
    Incompatible,
    Unknown,
    Stale,
    Revoked,
    Blocked,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructureDisposition {
    Parsed,
    Ambiguous,
    Unsupported,
    VersionSkew,
    GeneratedBoundary,
    UnknownEdge,
    Failed,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogEntry {
    pub profile_id: String,
    pub developer: String,
    pub publisher: String,
    pub model_revision: String,
    pub artifact_revision: String,
    pub origin: String,
    pub lineage_sha256: String,
    pub license_sha256: String,
    pub format: String,
    pub transformation_sha256: String,
    pub artifact_sha256: String,
    pub size_bytes: u64,
    pub tokenizer_sha256: String,
    pub template_sha256: String,
    pub runtime_id: String,
    pub platform_id: String,
    pub hardware_profile_id: String,
    pub quality_evidence_sha256: String,
    pub security_evidence_sha256: String,
    pub support_state: String,
    pub rereview_evidence_sha256: String,
    pub state: CatalogState,
    pub compatibility: Compatibility,
    pub decision_history: Vec<String>,
    pub ordinary_selection_allowed: bool,
    pub exact_artifact_identity: bool,
    pub family_inherited_approval: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformFacts {
    pub disk_bytes: u64,
    pub memory_bytes: u64,
    pub acceleration: String,
    pub context_tokens: u64,
    pub concurrency: u32,
    pub latency_limit_ms: u64,
    pub runtime_id: String,
    pub measured: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructuralRecord {
    pub record_id: String,
    pub kind: String,
    pub source_sha256: String,
    pub parser_id: String,
    pub parser_version: String,
    pub start_byte: u64,
    pub end_byte: u64,
    pub disposition: StructureDisposition,
    pub reason_code: Option<String>,
    pub model_inferred: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticPacket {
    pub packet_id: String,
    pub partition_class: String,
    pub source_span_digests: BTreeSet<String>,
    pub structural_fact_ids: BTreeSet<String>,
    pub neighbor_interface_ids: BTreeSet<String>,
    pub test_ids: BTreeSet<String>,
    pub decision_ids: BTreeSet<String>,
    pub conflict_ids: BTreeSet<String>,
    pub audit_question_digest: String,
    pub context_budget: u64,
    pub resource_profile_id: String,
    pub redacted: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceCard {
    pub card_id: String,
    pub source_ids: BTreeSet<String>,
    pub packet_id: String,
    pub model_profile_id: String,
    pub artifact_sha256: String,
    pub tokenizer_sha256: String,
    pub template_sha256: String,
    pub codec_id: String,
    pub runtime_id: String,
    pub context_id: String,
    pub decoding_id: String,
    pub platform_hardware_id: String,
    pub proposal_digest: String,
    pub assumption_digests: BTreeSet<String>,
    pub uncertainty_codes: BTreeSet<String>,
    pub confidence_basis_points: u16,
    pub conflict_ids: BTreeSet<String>,
    pub followup_digest: String,
    pub verifier_id: String,
    pub reverse_dependency_ids: BTreeSet<String>,
    pub provisional: bool,
    pub authority_count: u32,
    pub completeness_count: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatalogSemanticError {
    Invalid,
    Unusable,
    Inherited,
    Structure,
    Authority,
}
fn id(v: &str) -> bool {
    !v.is_empty() && v.len() <= 4096
}
fn digest(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
pub fn validate_entry(v: &CatalogEntry) -> Result<(), CatalogSemanticError> {
    let hashes = [
        &v.lineage_sha256,
        &v.license_sha256,
        &v.transformation_sha256,
        &v.artifact_sha256,
        &v.tokenizer_sha256,
        &v.template_sha256,
        &v.quality_evidence_sha256,
        &v.security_evidence_sha256,
        &v.rereview_evidence_sha256,
    ];
    if !id(&v.profile_id)
        || !id(&v.developer)
        || !id(&v.publisher)
        || !id(&v.model_revision)
        || !id(&v.artifact_revision)
        || !id(&v.origin)
        || hashes.iter().any(|x| !digest(x))
        || v.size_bytes == 0
        || !id(&v.format)
        || !id(&v.runtime_id)
        || !id(&v.platform_id)
        || !id(&v.hardware_profile_id)
        || !id(&v.support_state)
        || v.decision_history.is_empty()
        || !v.exact_artifact_identity
    {
        return Err(CatalogSemanticError::Invalid);
    }
    if v.family_inherited_approval {
        return Err(CatalogSemanticError::Inherited);
    }
    let usable = v.state == CatalogState::Approved
        && matches!(
            v.compatibility,
            Compatibility::Compatible | Compatibility::CompatibleWithLimitations
        );
    if v.ordinary_selection_allowed != usable {
        return Err(CatalogSemanticError::Unusable);
    }
    Ok(())
}
pub fn validate_structure(v: &StructuralRecord) -> Result<(), CatalogSemanticError> {
    if !id(&v.record_id)
        || !id(&v.kind)
        || !digest(&v.source_sha256)
        || !id(&v.parser_id)
        || !id(&v.parser_version)
        || v.start_byte >= v.end_byte
        || v.model_inferred
    {
        return Err(CatalogSemanticError::Structure);
    }
    if v.disposition != StructureDisposition::Parsed
        && v.reason_code.as_deref().is_none_or(|x| !id(x))
    {
        return Err(CatalogSemanticError::Structure);
    }
    Ok(())
}
pub fn validate_packet(v: &SemanticPacket) -> Result<(), CatalogSemanticError> {
    if !id(&v.packet_id)
        || !id(&v.partition_class)
        || v.source_span_digests.is_empty()
        || v.source_span_digests.iter().any(|x| !digest(x))
        || v.structural_fact_ids.is_empty()
        || !digest(&v.audit_question_digest)
        || v.context_budget == 0
        || !id(&v.resource_profile_id)
        || !v.redacted
    {
        Err(CatalogSemanticError::Invalid)
    } else {
        Ok(())
    }
}
pub fn validate_card(v: &EvidenceCard) -> Result<(), CatalogSemanticError> {
    if !id(&v.card_id)
        || v.source_ids.is_empty()
        || !id(&v.packet_id)
        || !id(&v.model_profile_id)
        || !digest(&v.artifact_sha256)
        || !digest(&v.tokenizer_sha256)
        || !digest(&v.template_sha256)
        || !id(&v.codec_id)
        || !id(&v.runtime_id)
        || !id(&v.context_id)
        || !id(&v.decoding_id)
        || !id(&v.platform_hardware_id)
        || !digest(&v.proposal_digest)
        || v.confidence_basis_points > 10_000
        || !digest(&v.followup_digest)
        || !id(&v.verifier_id)
        || !v.provisional
    {
        return Err(CatalogSemanticError::Invalid);
    }
    if v.authority_count != 0 || v.completeness_count != 0 {
        return Err(CatalogSemanticError::Authority);
    }
    Ok(())
}
pub fn deterministic_structure_identity(
    records: &[StructuralRecord],
) -> Result<Vec<String>, CatalogSemanticError> {
    for r in records {
        validate_structure(r)?
    }
    let mut ids = records
        .iter()
        .map(|r| r.record_id.clone())
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    if ids.len() != records.len() {
        return Err(CatalogSemanticError::Structure);
    }
    Ok(ids)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inaccessible_profiles_never_select() {
        for state in [
            CatalogState::Candidate,
            CatalogState::Evaluating,
            CatalogState::Degraded,
            CatalogState::Quarantined,
            CatalogState::Rejected,
            CatalogState::Retired,
        ] {
            let mut v = entry();
            v.state = state;
            assert_eq!(validate_entry(&v), Ok(()));
            v.ordinary_selection_allowed = true;
            assert_eq!(validate_entry(&v), Err(CatalogSemanticError::Unusable));
        }
    }
    #[test]
    fn parsed_structure_is_source_not_model_owned() {
        let v = StructuralRecord {
            record_id: "r".into(),
            kind: "symbol".into(),
            source_sha256: "a".repeat(64),
            parser_id: "rust".into(),
            parser_version: "1".into(),
            start_byte: 1,
            end_byte: 2,
            disposition: StructureDisposition::Parsed,
            reason_code: None,
            model_inferred: false,
        };
        assert_eq!(validate_structure(&v), Ok(()))
    }
    fn entry() -> CatalogEntry {
        CatalogEntry {
            profile_id: "p".into(),
            developer: "d".into(),
            publisher: "p".into(),
            model_revision: "m".into(),
            artifact_revision: "a".into(),
            origin: "o".into(),
            lineage_sha256: "a".repeat(64),
            license_sha256: "b".repeat(64),
            format: "gguf".into(),
            transformation_sha256: "c".repeat(64),
            artifact_sha256: "d".repeat(64),
            size_bytes: 1,
            tokenizer_sha256: "e".repeat(64),
            template_sha256: "f".repeat(64),
            runtime_id: "r".into(),
            platform_id: "linux".into(),
            hardware_profile_id: "h".into(),
            quality_evidence_sha256: "1".repeat(64),
            security_evidence_sha256: "2".repeat(64),
            support_state: "unsupported-pre-release".into(),
            rereview_evidence_sha256: "3".repeat(64),
            state: CatalogState::Candidate,
            compatibility: Compatibility::Unknown,
            decision_history: vec!["candidate".into()],
            ordinary_selection_allowed: false,
            exact_artifact_identity: true,
            family_inherited_approval: false,
        }
    }
}
