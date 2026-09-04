//! Exact supply-chain evidence and conflict-preserving security findings.
#![allow(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupplyEvidenceKind {
    Spdx,
    CycloneDx,
    SigstoreSignature,
    CosignAttestation,
    SlsaProvenance,
    PolicyResult,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SupplyEvidence {
    pub kind: SupplyEvidenceKind,
    pub schema_version: String,
    pub producer: String,
    pub subject_digest: String,
    pub component_graph_sha256: String,
    pub license_set_sha256: String,
    pub generation_context_sha256: String,
    pub signer_identity_sha256: Option<String>,
    pub transparency_sha256: Option<String>,
    pub certificate_valid: Option<bool>,
    pub source_sha256: String,
    pub build_sha256: String,
    pub artifact_digest: String,
    pub claimed_assurance_level: Option<u8>,
    pub proven_assurance_level: Option<u8>,
    pub policy_bundle_sha256: Option<String>,
    pub release_gate: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecurityFinding {
    pub finding_id: String,
    pub tool: String,
    pub tool_version: String,
    pub rule: String,
    pub immutable_source_sha256: String,
    pub artifact_digest: String,
    pub location_sha256: String,
    pub severity: String,
    pub confidence: String,
    pub reachability: String,
    pub suppression: String,
    pub remediation_sha256: String,
    pub untrusted_text: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindingGroup {
    pub correlation_key: String,
    pub original_findings: BTreeMap<String, SecurityFinding>,
    pub severities: BTreeSet<String>,
    pub locations: BTreeSet<String>,
    pub reachability: BTreeSet<String>,
    pub suppressions: BTreeSet<String>,
    pub blocking_finding_count: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupplyEvidenceError {
    InvalidRecord,
    SubjectMismatch,
    AssuranceOverclaim,
    StaleEvidence,
    FindingLoss,
}
pub fn validate_supply_evidence(
    v: &SupplyEvidence,
    current_artifact: &str,
) -> Result<(), SupplyEvidenceError> {
    if !valid_id(&v.schema_version)
        || !valid_id(&v.producer)
        || !valid_digest(&v.subject_digest)
        || !valid_digest(&v.artifact_digest)
        || v.subject_digest != current_artifact
        || v.artifact_digest != current_artifact
    {
        return Err(SupplyEvidenceError::SubjectMismatch);
    }
    if [
        &v.component_graph_sha256,
        &v.license_set_sha256,
        &v.generation_context_sha256,
        &v.source_sha256,
        &v.build_sha256,
    ]
    .into_iter()
    .any(|x| !valid_sha(x))
        || !valid_id(&v.release_gate)
    {
        return Err(SupplyEvidenceError::InvalidRecord);
    }
    if v.claimed_assurance_level.unwrap_or(0) > v.proven_assurance_level.unwrap_or(0) {
        return Err(SupplyEvidenceError::AssuranceOverclaim);
    }
    if matches!(
        v.kind,
        SupplyEvidenceKind::SigstoreSignature | SupplyEvidenceKind::CosignAttestation
    ) && (v
        .signer_identity_sha256
        .as_deref()
        .is_none_or(|x| !valid_sha(x))
        || v.certificate_valid != Some(true))
    {
        return Err(SupplyEvidenceError::InvalidRecord);
    }
    Ok(())
}
pub fn reconcile_findings(
    key: &str,
    findings: Vec<SecurityFinding>,
) -> Result<FindingGroup, SupplyEvidenceError> {
    if !valid_id(key) || findings.is_empty() {
        return Err(SupplyEvidenceError::InvalidRecord);
    }
    let mut originals = BTreeMap::new();
    let mut severities = BTreeSet::new();
    let mut locations = BTreeSet::new();
    let mut reachability = BTreeSet::new();
    let mut suppressions = BTreeSet::new();
    let mut blocking = 0;
    for f in findings {
        if !valid_id(&f.finding_id)
            || !valid_id(&f.tool)
            || !valid_id(&f.tool_version)
            || !valid_id(&f.rule)
            || [
                &f.immutable_source_sha256,
                &f.location_sha256,
                &f.remediation_sha256,
            ]
            .into_iter()
            .any(|x| !valid_sha(x))
            || !valid_digest(&f.artifact_digest)
            || !f.untrusted_text
            || originals.contains_key(&f.finding_id)
        {
            return Err(SupplyEvidenceError::InvalidRecord);
        }
        if f.suppression == "none" {
            blocking += 1
        }
        severities.insert(f.severity.clone());
        locations.insert(f.location_sha256.clone());
        reachability.insert(f.reachability.clone());
        suppressions.insert(f.suppression.clone());
        originals.insert(f.finding_id.clone(), f);
    }
    Ok(FindingGroup {
        correlation_key: key.into(),
        original_findings: originals,
        severities,
        locations,
        reachability,
        suppressions,
        blocking_finding_count: blocking,
    })
}
pub fn require_current_evidence(
    v: &SupplyEvidence,
    source: &str,
    build: &str,
    artifact: &str,
) -> Result<(), SupplyEvidenceError> {
    if v.source_sha256 != source || v.build_sha256 != build || v.artifact_digest != artifact {
        Err(SupplyEvidenceError::StaleEvidence)
    } else {
        Ok(())
    }
}
fn valid_id(v: &str) -> bool {
    !v.is_empty()
        && v.len() <= 512
        && v.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':' | b'/'))
}
fn valid_sha(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn valid_digest(v: &str) -> bool {
    v.starts_with("sha256:") && valid_sha(&v[7..])
}
#[cfg(test)]
mod tests {
    use super::*;
    fn evidence() -> SupplyEvidence {
        let h = "a".repeat(64);
        let d = format!("sha256:{h}");
        SupplyEvidence {
            kind: SupplyEvidenceKind::SlsaProvenance,
            schema_version: "1.0".into(),
            producer: "builder-1".into(),
            subject_digest: d.clone(),
            component_graph_sha256: h.clone(),
            license_set_sha256: h.clone(),
            generation_context_sha256: h.clone(),
            signer_identity_sha256: None,
            transparency_sha256: None,
            certificate_valid: None,
            source_sha256: h.clone(),
            build_sha256: h.clone(),
            artifact_digest: d,
            claimed_assurance_level: Some(2),
            proven_assurance_level: Some(2),
            policy_bundle_sha256: None,
            release_gate: "release-gate".into(),
        }
    }
    #[test]
    fn exact_subject_and_assurance_are_required() {
        let mut e = evidence();
        let d = e.artifact_digest.clone();
        assert!(validate_supply_evidence(&e, &d).is_ok());
        e.claimed_assurance_level = Some(3);
        assert_eq!(
            validate_supply_evidence(&e, &d),
            Err(SupplyEvidenceError::AssuranceOverclaim)
        );
    }
    #[test]
    fn conflicting_findings_are_preserved() {
        let h = "b".repeat(64);
        let make = |id: &str, severity: &str| SecurityFinding {
            finding_id: id.into(),
            tool: "semgrep".into(),
            tool_version: "1.0".into(),
            rule: "rule-1".into(),
            immutable_source_sha256: h.clone(),
            artifact_digest: format!("sha256:{h}"),
            location_sha256: h.clone(),
            severity: severity.into(),
            confidence: "high".into(),
            reachability: "unknown".into(),
            suppression: "none".into(),
            remediation_sha256: h.clone(),
            untrusted_text: true,
        };
        let g =
            reconcile_findings("group-1", vec![make("f-1", "high"), make("f-2", "low")]).unwrap();
        assert_eq!(g.original_findings.len(), 2);
        assert_eq!(g.severities.len(), 2);
        assert_eq!(g.blocking_finding_count, 2);
    }
    #[test]
    fn changed_source_is_stale() {
        let e = evidence();
        assert_eq!(
            require_current_evidence(&e, &"c".repeat(64), &e.build_sha256, &e.artifact_digest),
            Err(SupplyEvidenceError::StaleEvidence)
        );
    }
}
