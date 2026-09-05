//! Final candidate-truth, composed-authority, and audit-finding contracts.
#![allow(missing_docs)]

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceDisposition {
    Verified,
    Unavailable,
    Contradictory,
    Stale,
    SecondaryOnly,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateTruth {
    pub profile_id: String,
    pub family_id: String,
    pub exact_artifact_sha256: String,
    pub role_ids: BTreeSet<String>,
    pub evidence_disposition: EvidenceDisposition,
    pub reason_code: Option<String>,
    pub support_claimed: bool,
    pub activation_allowed: bool,
    pub borrowed_family_result: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposedStep {
    pub step_id: String,
    pub capability_id: String,
    pub grant_ids: BTreeSet<String>,
    pub credential_ids: BTreeSet<String>,
    pub network_classes: BTreeSet<String>,
    pub storage_classes: BTreeSet<String>,
    pub completion_evidence_ids: BTreeSet<String>,
    pub input_digest: String,
    pub preview_digest: String,
    pub snapshot_digest: String,
    pub current: bool,
    pub externally_requested_authority: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReconciliationFinding {
    pub finding_id: String,
    pub category: String,
    pub severity: String,
    pub confidence_basis_points: u16,
    pub impact: String,
    pub evidence_ids: BTreeSet<String>,
    pub graph_path_ids: BTreeSet<String>,
    pub counterevidence_ids: BTreeSet<String>,
    pub uncertainty_codes: BTreeSet<String>,
    pub recommendation: String,
    pub conflict_ids: BTreeSet<String>,
    pub supersedes_ids: BTreeSet<String>,
    pub status: String,
    pub current: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditCompletion {
    pub census_digest: String,
    pub graph_digest: String,
    pub requirement_digest: String,
    pub decision_digest: String,
    pub test_digest: String,
    pub report_digest: String,
    pub missing_count: u64,
    pub stale_count: u64,
    pub failed_count: u64,
    pub unsupported_count: u64,
    pub unavailable_count: u64,
    pub excluded_required_count: u64,
    pub unreconciled_count: u64,
    pub read_only_attested: bool,
    pub comprehensive_claim: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReconciliationError {
    Invalid,
    UnsupportedClaim,
    AuthorityUnion,
    Stale,
    Incomplete,
}

fn id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 4096
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub fn validate_candidate(value: &CandidateTruth) -> Result<(), ReconciliationError> {
    if !id(&value.profile_id)
        || !id(&value.family_id)
        || !digest(&value.exact_artifact_sha256)
        || value.role_ids.is_empty()
        || value.borrowed_family_result
    {
        return Err(ReconciliationError::Invalid);
    }
    if value.evidence_disposition != EvidenceDisposition::Verified {
        if value
            .reason_code
            .as_deref()
            .is_none_or(|reason| !id(reason))
        {
            return Err(ReconciliationError::Invalid);
        }
        if value.support_claimed || value.activation_allowed {
            return Err(ReconciliationError::UnsupportedClaim);
        }
    }
    Ok(())
}

pub fn validate_step(value: &ComposedStep) -> Result<(), ReconciliationError> {
    if !id(&value.step_id)
        || !id(&value.capability_id)
        || !digest(&value.input_digest)
        || !digest(&value.preview_digest)
        || !digest(&value.snapshot_digest)
    {
        return Err(ReconciliationError::Invalid);
    }
    if !value.current {
        return Err(ReconciliationError::Stale);
    }
    if value.externally_requested_authority
        || value.grant_ids.len() > 1
        || value.credential_ids.len() > 1
        || value.network_classes.len() > 1
        || value.storage_classes.len() > 1
        || value.completion_evidence_ids.len() > 1
    {
        return Err(ReconciliationError::AuthorityUnion);
    }
    Ok(())
}

pub fn validate_finding(value: &ReconciliationFinding) -> Result<(), ReconciliationError> {
    if !id(&value.finding_id)
        || !id(&value.category)
        || !id(&value.severity)
        || value.confidence_basis_points > 10_000
        || !id(&value.impact)
        || value.evidence_ids.is_empty()
        || value.graph_path_ids.is_empty()
        || !id(&value.recommendation)
        || !id(&value.status)
        || !value.current
    {
        Err(ReconciliationError::Invalid)
    } else {
        Ok(())
    }
}

pub fn validate_completion(value: &AuditCompletion) -> Result<(), ReconciliationError> {
    let digests = [
        &value.census_digest,
        &value.graph_digest,
        &value.requirement_digest,
        &value.decision_digest,
        &value.test_digest,
        &value.report_digest,
    ];
    if digests.iter().any(|item| !digest(item)) {
        return Err(ReconciliationError::Invalid);
    }
    let blockers = value.missing_count
        + value.stale_count
        + value.failed_count
        + value.unsupported_count
        + value.unavailable_count
        + value.excluded_required_count
        + value.unreconciled_count;
    if value.comprehensive_claim && (blockers != 0 || !value.read_only_attested) {
        return Err(ReconciliationError::Incomplete);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_candidate_evidence_cannot_become_a_claim() {
        for disposition in [
            EvidenceDisposition::Unavailable,
            EvidenceDisposition::Contradictory,
            EvidenceDisposition::Stale,
            EvidenceDisposition::SecondaryOnly,
            EvidenceDisposition::Failed,
        ] {
            let mut value = candidate(disposition);
            assert_eq!(validate_candidate(&value), Ok(()));
            value.support_claimed = true;
            assert_eq!(
                validate_candidate(&value),
                Err(ReconciliationError::UnsupportedClaim)
            );
        }
    }
    #[test]
    fn composed_steps_cannot_borrow_authority() {
        let mut value = step();
        assert_eq!(validate_step(&value), Ok(()));
        value.grant_ids.insert("second".into());
        assert_eq!(
            validate_step(&value),
            Err(ReconciliationError::AuthorityUnion)
        );
        value.grant_ids.clear();
        value.externally_requested_authority = true;
        assert_eq!(
            validate_step(&value),
            Err(ReconciliationError::AuthorityUnion)
        );
    }
    #[test]
    fn incomplete_audit_cannot_claim_comprehensive() {
        let mut value = completion();
        value.comprehensive_claim = true;
        assert_eq!(validate_completion(&value), Ok(()));
        value.unreconciled_count = 1;
        assert_eq!(
            validate_completion(&value),
            Err(ReconciliationError::Incomplete)
        );
    }
    fn candidate(disposition: EvidenceDisposition) -> CandidateTruth {
        CandidateTruth {
            profile_id: "profile".into(),
            family_id: "family".into(),
            exact_artifact_sha256: "a".repeat(64),
            role_ids: BTreeSet::from(["coding".into()]),
            evidence_disposition: disposition,
            reason_code: Some("evidence-unavailable".into()),
            support_claimed: false,
            activation_allowed: false,
            borrowed_family_result: false,
        }
    }
    fn step() -> ComposedStep {
        ComposedStep {
            step_id: "step".into(),
            capability_id: "research".into(),
            grant_ids: BTreeSet::from(["grant".into()]),
            credential_ids: BTreeSet::new(),
            network_classes: BTreeSet::from(["public".into()]),
            storage_classes: BTreeSet::new(),
            completion_evidence_ids: BTreeSet::from(["receipt".into()]),
            input_digest: "a".repeat(64),
            preview_digest: "b".repeat(64),
            snapshot_digest: "c".repeat(64),
            current: true,
            externally_requested_authority: false,
        }
    }
    fn completion() -> AuditCompletion {
        AuditCompletion {
            census_digest: "a".repeat(64),
            graph_digest: "b".repeat(64),
            requirement_digest: "c".repeat(64),
            decision_digest: "d".repeat(64),
            test_digest: "e".repeat(64),
            report_digest: "f".repeat(64),
            missing_count: 0,
            stale_count: 0,
            failed_count: 0,
            unsupported_count: 0,
            unavailable_count: 0,
            excluded_required_count: 0,
            unreconciled_count: 0,
            read_only_attested: true,
            comprehensive_claim: false,
        }
    }
}
