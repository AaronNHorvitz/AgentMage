//! Normal-admission-only transition and removal preservation.
#![allow(missing_docs)]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionEvidence {
    pub fresh_candidate: bool,
    pub identity_passed: bool,
    pub license_passed: bool,
    pub lineage_passed: bool,
    pub artifact_passed: bool,
    pub runtime_passed: bool,
    pub resource_passed: bool,
    pub quality_passed: bool,
    pub security_passed: bool,
    pub platform_passed: bool,
    pub independent_review_passed: bool,
    pub signed_catalog_transition: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PromotionRequest {
    pub direct_promote_operation: bool,
    pub chat_requested: bool,
    pub model_requested: bool,
    pub copied_manifest: bool,
    pub edited_catalog: bool,
    pub stale_approval: bool,
    pub alias_used: bool,
    pub lab_result_as_approval: bool,
    pub evidence: AdmissionEvidence,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RemovalObservation {
    pub evaluations: u32,
    pub descendants: u32,
    pub ipc_endpoints: u32,
    pub scratch_records: u32,
    pub index_records: u32,
    pub cache_records: u32,
    pub quarantine_records: u32,
    pub network_rules: u32,
    pub registrations: u32,
    pub selected_retained_artifacts: u32,
    pub approved_models_unchanged: bool,
    pub canonical_work_unchanged: bool,
    pub exported_evidence_unchanged: bool,
    pub dependencies_unchanged: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromotionError {
    DirectPath,
    Bypass,
    IncompleteAdmission,
    RemovalResidue,
    NeighborDamage,
}
pub fn admit(v: PromotionRequest) -> Result<(), PromotionError> {
    if v.direct_promote_operation {
        return Err(PromotionError::DirectPath);
    }
    if v.chat_requested
        || v.model_requested
        || v.copied_manifest
        || v.edited_catalog
        || v.stale_approval
        || v.alias_used
        || v.lab_result_as_approval
    {
        return Err(PromotionError::Bypass);
    }
    let e = v.evidence;
    let complete = e.fresh_candidate
        && e.identity_passed
        && e.license_passed
        && e.lineage_passed
        && e.artifact_passed
        && e.runtime_passed
        && e.resource_passed
        && e.quality_passed
        && e.security_passed
        && e.platform_passed
        && e.independent_review_passed
        && e.signed_catalog_transition;
    if complete {
        Ok(())
    } else {
        Err(PromotionError::IncompleteAdmission)
    }
}
pub fn validate_removal(v: RemovalObservation) -> Result<(), PromotionError> {
    let residue = v.evaluations
        + v.descendants
        + v.ipc_endpoints
        + v.scratch_records
        + v.index_records
        + v.cache_records
        + v.quarantine_records
        + v.network_rules
        + v.registrations
        + v.selected_retained_artifacts;
    if residue != 0 {
        return Err(PromotionError::RemovalResidue);
    }
    if !v.approved_models_unchanged
        || !v.canonical_work_unchanged
        || !v.exported_evidence_unchanged
        || !v.dependencies_unchanged
    {
        return Err(PromotionError::NeighborDamage);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_bypass_is_denied() {
        let base = request();
        for mutate in [
            |v: &mut PromotionRequest| v.direct_promote_operation = true,
            |v: &mut PromotionRequest| v.chat_requested = true,
            |v: &mut PromotionRequest| v.model_requested = true,
            |v: &mut PromotionRequest| v.copied_manifest = true,
            |v: &mut PromotionRequest| v.edited_catalog = true,
            |v: &mut PromotionRequest| v.stale_approval = true,
            |v: &mut PromotionRequest| v.alias_used = true,
            |v: &mut PromotionRequest| v.lab_result_as_approval = true,
        ] {
            let mut v = base;
            mutate(&mut v);
            assert!(admit(v).is_err())
        }
    }
    #[test]
    fn unsigned_transition_never_admits() {
        let mut v = request();
        v.evidence.signed_catalog_transition = false;
        assert_eq!(admit(v), Err(PromotionError::IncompleteAdmission));
    }
    #[test]
    fn removal_preserves_neighbors() {
        let v = RemovalObservation {
            evaluations: 0,
            descendants: 0,
            ipc_endpoints: 0,
            scratch_records: 0,
            index_records: 0,
            cache_records: 0,
            quarantine_records: 0,
            network_rules: 0,
            registrations: 0,
            selected_retained_artifacts: 0,
            approved_models_unchanged: true,
            canonical_work_unchanged: true,
            exported_evidence_unchanged: true,
            dependencies_unchanged: true,
        };
        assert_eq!(validate_removal(v), Ok(()));
    }
    fn request() -> PromotionRequest {
        PromotionRequest {
            direct_promote_operation: false,
            chat_requested: false,
            model_requested: false,
            copied_manifest: false,
            edited_catalog: false,
            stale_approval: false,
            alias_used: false,
            lab_result_as_approval: false,
            evidence: AdmissionEvidence {
                fresh_candidate: true,
                identity_passed: true,
                license_passed: true,
                lineage_passed: true,
                artifact_passed: true,
                runtime_passed: true,
                resource_passed: true,
                quality_passed: true,
                security_passed: true,
                platform_passed: true,
                independent_review_passed: true,
                signed_catalog_transition: true,
            },
        }
    }
}
