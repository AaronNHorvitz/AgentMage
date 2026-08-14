//! Deterministic material-claim proof and truthful final-response construction.

use std::collections::{BTreeMap, BTreeSet};

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, ClaimBoundFinalResponse, ClaimEvidence, ClaimEvidenceRole,
    EvidenceKind, MaterialClaim, MaterialClaimKind, TaskId, VerifiedMaterialClaim,
};

const MAX_CLAIMS: usize = 128;
const MAX_TEXT_BYTES: usize = 4_096;
const MAX_IDENTIFIER_BYTES: usize = 256;

/// Typed reason a material claim or final response is refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClaimEvidenceError {
    /// The proposed claim is malformed, duplicated, or outside the ledger task.
    InvalidClaim,
    /// Proof roles, evidence kinds, identities, or revisions do not match the claim.
    InvalidProof,
    /// A declared prerequisite is absent or not yet verified.
    MissingPrerequisite,
    /// A claim has already reached a terminal verified state.
    AlreadyVerified,
    /// The final response is malformed or references an incomplete claim set.
    InvalidFinalResponse,
}

/// Bounded task-local ledger of proposed and deterministically verified material claims.
pub struct ClaimEvidenceLedger {
    task_id: TaskId,
    proposed: BTreeMap<String, MaterialClaim>,
    verified: BTreeMap<String, VerifiedMaterialClaim>,
}

impl ClaimEvidenceLedger {
    /// Starts an empty ledger for one exact task.
    pub fn new(task_id: TaskId) -> Result<Self, ClaimEvidenceError> {
        if !valid_identifier(task_id.as_str()) {
            return Err(ClaimEvidenceError::InvalidClaim);
        }
        Ok(Self {
            task_id,
            proposed: BTreeMap::new(),
            verified: BTreeMap::new(),
        })
    }

    /// Registers one bounded material claim without treating it as true.
    pub fn propose(&mut self, claim: MaterialClaim) -> Result<(), ClaimEvidenceError> {
        validate_claim(&claim)?;
        if claim.task_id != self.task_id
            || self.proposed.len() >= MAX_CLAIMS
            || self.proposed.contains_key(&claim.claim_id)
        {
            return Err(ClaimEvidenceError::InvalidClaim);
        }
        if claim
            .prerequisite_claim_ids
            .iter()
            .any(|claim_id| !self.proposed.contains_key(claim_id))
        {
            return Err(ClaimEvidenceError::MissingPrerequisite);
        }
        self.proposed.insert(claim.claim_id.clone(), claim);
        Ok(())
    }

    /// Verifies one proposed claim against the exact closed proof contract.
    pub fn verify(
        &mut self,
        claim_id: &str,
        proof: Vec<ClaimEvidence>,
    ) -> Result<&VerifiedMaterialClaim, ClaimEvidenceError> {
        if self.verified.contains_key(claim_id) {
            return Err(ClaimEvidenceError::AlreadyVerified);
        }
        let claim = self
            .proposed
            .get(claim_id)
            .ok_or(ClaimEvidenceError::InvalidClaim)?;
        if claim
            .prerequisite_claim_ids
            .iter()
            .any(|required| !self.verified.contains_key(required))
        {
            return Err(ClaimEvidenceError::MissingPrerequisite);
        }
        let consumed_evidence = self
            .verified
            .values()
            .flat_map(|verified| verified.proof.iter())
            .map(|item| item.evidence.evidence_id.as_str())
            .collect::<BTreeSet<_>>();
        if proof
            .iter()
            .any(|item| consumed_evidence.contains(item.evidence.evidence_id.as_str()))
        {
            return Err(ClaimEvidenceError::InvalidProof);
        }
        validate_proof(claim, &proof)?;
        self.verified.insert(
            claim_id.to_owned(),
            VerifiedMaterialClaim {
                claim: claim.clone(),
                proof,
            },
        );
        Ok(self
            .verified
            .get(claim_id)
            .expect("verified claim was inserted"))
    }

    /// Returns one verified claim when present.
    #[must_use]
    pub fn verified(&self, claim_id: &str) -> Option<&VerifiedMaterialClaim> {
        self.verified.get(claim_id)
    }

    /// Builds a final response only from a complete exact set of verified claim identities.
    pub fn final_response(
        &self,
        summary: String,
        represented_claim_ids: Vec<String>,
    ) -> Result<ClaimBoundFinalResponse, ClaimEvidenceError> {
        if !valid_text(&summary)
            || represented_claim_ids.is_empty()
            || represented_claim_ids.len() > MAX_CLAIMS
            || has_duplicates(&represented_claim_ids)
            || represented_claim_ids
                .iter()
                .any(|claim_id| !valid_identifier(claim_id))
        {
            return Err(ClaimEvidenceError::InvalidFinalResponse);
        }
        let claims = represented_claim_ids
            .iter()
            .map(|claim_id| {
                self.verified
                    .get(claim_id)
                    .cloned()
                    .ok_or(ClaimEvidenceError::InvalidFinalResponse)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let completion = claims
            .iter()
            .filter(|claim| claim.claim.kind == MaterialClaimKind::Complete)
            .collect::<Vec<_>>();
        if completion.len() != 1
            || completion[0].claim.prerequisite_claim_ids
                != represented_claim_ids[..represented_claim_ids.len() - 1]
            || represented_claim_ids.last() != Some(&completion[0].claim.claim_id)
        {
            return Err(ClaimEvidenceError::InvalidFinalResponse);
        }
        Ok(ClaimBoundFinalResponse {
            schema_version: CONTRACT_SCHEMA_VERSION,
            task_id: self.task_id.clone(),
            summary,
            verified_claims: claims,
        })
    }
}

fn validate_claim(claim: &MaterialClaim) -> Result<(), ClaimEvidenceError> {
    if claim.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_identifier(claim.claim_id.as_str())
        || !valid_identifier(claim.task_id.as_str())
        || !valid_text(&claim.statement)
        || !valid_identifier(&claim.subject_id)
        || !valid_identifier(&claim.expected_revision)
        || claim.prerequisite_claim_ids.len() >= MAX_CLAIMS
        || claim
            .prerequisite_claim_ids
            .iter()
            .any(|claim_id| !valid_identifier(claim_id))
        || has_duplicates(&claim.prerequisite_claim_ids)
        || claim
            .prerequisite_claim_ids
            .iter()
            .any(|claim_id| claim_id == &claim.claim_id)
        || (claim.kind == MaterialClaimKind::Complete && claim.prerequisite_claim_ids.is_empty())
        || (claim.kind != MaterialClaimKind::Complete && !claim.prerequisite_claim_ids.is_empty())
    {
        return Err(ClaimEvidenceError::InvalidClaim);
    }
    Ok(())
}

fn validate_proof(
    claim: &MaterialClaim,
    proof: &[ClaimEvidence],
) -> Result<(), ClaimEvidenceError> {
    let required = required_roles(claim.kind);
    let actual = proof.iter().map(|item| item.role).collect::<BTreeSet<_>>();
    let identities = proof
        .iter()
        .map(|item| item.evidence.evidence_id.as_str())
        .collect::<BTreeSet<_>>();
    if proof.len() != required.len()
        || actual != required
        || identities.len() != proof.len()
        || proof.iter().any(|item| {
            let evidence = &item.evidence;
            evidence.schema_version != CONTRACT_SCHEMA_VERSION
                || !valid_identifier(evidence.evidence_id.as_str())
                || !valid_text(&evidence.source_id)
                || evidence.object_id != claim.subject_id
                || evidence.observed_revision.as_deref() != Some(&claim.expected_revision)
                || !valid_sha256(&evidence.content_sha256)
                || evidence_kind_for(item.role) != evidence.kind
        })
    {
        return Err(ClaimEvidenceError::InvalidProof);
    }
    Ok(())
}

fn required_roles(kind: MaterialClaimKind) -> BTreeSet<ClaimEvidenceRole> {
    let roles: &[ClaimEvidenceRole] = match kind {
        MaterialClaimKind::Read => &[
            ClaimEvidenceRole::OperationReceipt,
            ClaimEvidenceRole::ReadObservation,
        ],
        MaterialClaimKind::Change => &[
            ClaimEvidenceRole::OperationReceipt,
            ClaimEvidenceRole::Postcondition,
        ],
        MaterialClaimKind::Test => &[
            ClaimEvidenceRole::TestOutput,
            ClaimEvidenceRole::TestValidation,
        ],
        MaterialClaimKind::Commit => &[
            ClaimEvidenceRole::OperationReceipt,
            ClaimEvidenceRole::CommitObservation,
        ],
        MaterialClaimKind::Push => &[
            ClaimEvidenceRole::OperationReceipt,
            ClaimEvidenceRole::RemoteObservation,
        ],
        MaterialClaimKind::Publish => &[
            ClaimEvidenceRole::OperationReceipt,
            ClaimEvidenceRole::PublicationObservation,
        ],
        MaterialClaimKind::Complete => &[ClaimEvidenceRole::AcceptanceVerification],
    };
    roles.iter().copied().collect()
}

const fn evidence_kind_for(role: ClaimEvidenceRole) -> EvidenceKind {
    match role {
        ClaimEvidenceRole::OperationReceipt => EvidenceKind::Receipt,
        ClaimEvidenceRole::TestOutput => EvidenceKind::ToolOutput,
        ClaimEvidenceRole::ReadObservation
        | ClaimEvidenceRole::Postcondition
        | ClaimEvidenceRole::CommitObservation
        | ClaimEvidenceRole::RemoteObservation
        | ClaimEvidenceRole::PublicationObservation => EvidenceKind::Observation,
        ClaimEvidenceRole::TestValidation | ClaimEvidenceRole::AcceptanceVerification => {
            EvidenceKind::Validation
        }
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_TEXT_BYTES
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn has_duplicates(values: &[String]) -> bool {
    values.iter().collect::<BTreeSet<_>>().len() != values.len()
}

#[cfg(test)]
mod tests {
    use super::{ClaimEvidenceError, ClaimEvidenceLedger};
    use agentmage_kernel_contracts::{
        CONTRACT_SCHEMA_VERSION, ClaimEvidence, ClaimEvidenceRole, EvidenceId, EvidenceKind,
        EvidenceReference, MaterialClaim, MaterialClaimKind, TaskId,
    };

    fn claim(id: &str, kind: MaterialClaimKind, prerequisites: Vec<String>) -> MaterialClaim {
        MaterialClaim {
            schema_version: CONTRACT_SCHEMA_VERSION,
            claim_id: id.to_owned(),
            task_id: TaskId::from_raw("task-claims"),
            kind,
            statement: format!("Verified {id}"),
            subject_id: "subject-1".to_owned(),
            expected_revision: "revision-1".to_owned(),
            prerequisite_claim_ids: prerequisites,
        }
    }

    fn evidence(id: &str, role: ClaimEvidenceRole, kind: EvidenceKind) -> ClaimEvidence {
        ClaimEvidence {
            role,
            evidence: EvidenceReference {
                schema_version: CONTRACT_SCHEMA_VERSION,
                evidence_id: EvidenceId::from_raw(id),
                kind,
                source_id: "synthetic".to_owned(),
                object_id: "subject-1".to_owned(),
                fragment: None,
                content_sha256: "a".repeat(64),
                observed_revision: Some("revision-1".to_owned()),
            },
        }
    }

    fn proof(kind: MaterialClaimKind) -> Vec<ClaimEvidence> {
        match kind {
            MaterialClaimKind::Read => vec![
                evidence(
                    "receipt",
                    ClaimEvidenceRole::OperationReceipt,
                    EvidenceKind::Receipt,
                ),
                evidence(
                    "read",
                    ClaimEvidenceRole::ReadObservation,
                    EvidenceKind::Observation,
                ),
            ],
            MaterialClaimKind::Change => vec![
                evidence(
                    "receipt",
                    ClaimEvidenceRole::OperationReceipt,
                    EvidenceKind::Receipt,
                ),
                evidence(
                    "post",
                    ClaimEvidenceRole::Postcondition,
                    EvidenceKind::Observation,
                ),
            ],
            MaterialClaimKind::Test => vec![
                evidence(
                    "output",
                    ClaimEvidenceRole::TestOutput,
                    EvidenceKind::ToolOutput,
                ),
                evidence(
                    "validation",
                    ClaimEvidenceRole::TestValidation,
                    EvidenceKind::Validation,
                ),
            ],
            MaterialClaimKind::Commit => vec![
                evidence(
                    "receipt",
                    ClaimEvidenceRole::OperationReceipt,
                    EvidenceKind::Receipt,
                ),
                evidence(
                    "commit",
                    ClaimEvidenceRole::CommitObservation,
                    EvidenceKind::Observation,
                ),
            ],
            MaterialClaimKind::Push => vec![
                evidence(
                    "receipt",
                    ClaimEvidenceRole::OperationReceipt,
                    EvidenceKind::Receipt,
                ),
                evidence(
                    "remote",
                    ClaimEvidenceRole::RemoteObservation,
                    EvidenceKind::Observation,
                ),
            ],
            MaterialClaimKind::Publish => vec![
                evidence(
                    "receipt",
                    ClaimEvidenceRole::OperationReceipt,
                    EvidenceKind::Receipt,
                ),
                evidence(
                    "publish",
                    ClaimEvidenceRole::PublicationObservation,
                    EvidenceKind::Observation,
                ),
            ],
            MaterialClaimKind::Complete => vec![evidence(
                "acceptance",
                ClaimEvidenceRole::AcceptanceVerification,
                EvidenceKind::Validation,
            )],
        }
    }

    #[test]
    fn every_material_claim_requires_its_exact_current_role_set() {
        for (index, kind) in [
            MaterialClaimKind::Read,
            MaterialClaimKind::Change,
            MaterialClaimKind::Test,
            MaterialClaimKind::Commit,
            MaterialClaimKind::Push,
            MaterialClaimKind::Publish,
        ]
        .into_iter()
        .enumerate()
        {
            let id = format!("claim-{index}");
            let mut ledger = ClaimEvidenceLedger::new(TaskId::from_raw("task-claims")).unwrap();
            ledger.propose(claim(&id, kind, Vec::new())).unwrap();
            assert_eq!(
                ledger.verify(&id, Vec::new()),
                Err(ClaimEvidenceError::InvalidProof)
            );
            assert!(ledger.verify(&id, proof(kind)).is_ok());
        }
    }

    #[test]
    fn stale_wrong_kind_duplicate_and_extra_evidence_fail_closed() {
        let mut ledger = ClaimEvidenceLedger::new(TaskId::from_raw("task-claims")).unwrap();
        ledger
            .propose(claim("claim-test", MaterialClaimKind::Test, Vec::new()))
            .unwrap();
        let mut stale = proof(MaterialClaimKind::Test);
        stale[0].evidence.observed_revision = Some("revision-old".to_owned());
        assert_eq!(
            ledger.verify("claim-test", stale),
            Err(ClaimEvidenceError::InvalidProof)
        );
        let mut wrong = proof(MaterialClaimKind::Test);
        wrong[0].evidence.kind = EvidenceKind::Observation;
        assert_eq!(
            ledger.verify("claim-test", wrong),
            Err(ClaimEvidenceError::InvalidProof)
        );
        let mut extra = proof(MaterialClaimKind::Test);
        extra.push(evidence(
            "extra",
            ClaimEvidenceRole::OperationReceipt,
            EvidenceKind::Receipt,
        ));
        assert_eq!(
            ledger.verify("claim-test", extra),
            Err(ClaimEvidenceError::InvalidProof)
        );
    }

    #[test]
    fn completion_requires_every_declared_verified_prerequisite() {
        let mut ledger = ClaimEvidenceLedger::new(TaskId::from_raw("task-claims")).unwrap();
        ledger
            .propose(claim("claim-read", MaterialClaimKind::Read, Vec::new()))
            .unwrap();
        ledger
            .propose(claim(
                "claim-complete",
                MaterialClaimKind::Complete,
                vec!["claim-read".to_owned()],
            ))
            .unwrap();
        assert_eq!(
            ledger.verify("claim-complete", proof(MaterialClaimKind::Complete)),
            Err(ClaimEvidenceError::MissingPrerequisite)
        );
        ledger
            .verify("claim-read", proof(MaterialClaimKind::Read))
            .unwrap();
        ledger
            .verify("claim-complete", proof(MaterialClaimKind::Complete))
            .unwrap();
        assert!(
            ledger
                .final_response(
                    "Read and completed".to_owned(),
                    vec!["claim-read".to_owned(), "claim-complete".to_owned()],
                )
                .is_ok()
        );
    }

    #[test]
    fn final_response_rejects_omitted_unverified_reordered_or_duplicate_claims() {
        let mut ledger = ClaimEvidenceLedger::new(TaskId::from_raw("task-claims")).unwrap();
        ledger
            .propose(claim("claim-read", MaterialClaimKind::Read, Vec::new()))
            .unwrap();
        ledger
            .propose(claim(
                "claim-complete",
                MaterialClaimKind::Complete,
                vec!["claim-read".to_owned()],
            ))
            .unwrap();
        ledger
            .verify("claim-read", proof(MaterialClaimKind::Read))
            .unwrap();
        assert_eq!(
            ledger.final_response("Unsupported".to_owned(), vec!["claim-read".to_owned()]),
            Err(ClaimEvidenceError::InvalidFinalResponse)
        );
        ledger
            .verify("claim-complete", proof(MaterialClaimKind::Complete))
            .unwrap();
        for claims in [
            vec!["claim-complete".to_owned(), "claim-read".to_owned()],
            vec!["claim-read".to_owned(), "claim-read".to_owned()],
            vec!["claim-read".to_owned(), "claim-missing".to_owned()],
        ] {
            assert_eq!(
                ledger.final_response("Unsupported".to_owned(), claims),
                Err(ClaimEvidenceError::InvalidFinalResponse)
            );
        }
    }

    #[test]
    fn one_evidence_identity_cannot_be_reused_for_another_claim() {
        let mut ledger = ClaimEvidenceLedger::new(TaskId::from_raw("task-claims")).unwrap();
        ledger
            .propose(claim("claim-read-1", MaterialClaimKind::Read, Vec::new()))
            .unwrap();
        ledger
            .propose(claim("claim-read-2", MaterialClaimKind::Read, Vec::new()))
            .unwrap();
        ledger
            .verify("claim-read-1", proof(MaterialClaimKind::Read))
            .unwrap();
        assert_eq!(
            ledger.verify("claim-read-2", proof(MaterialClaimKind::Read)),
            Err(ClaimEvidenceError::InvalidProof)
        );
    }

    #[test]
    fn claim_records_are_descriptive_and_cannot_grant_authority() {
        let record = claim("claim-read", MaterialClaimKind::Read, Vec::new());
        assert_eq!(
            crate::authority::reject_as_authority(&record).artifact_kind,
            crate::authority::DescriptiveArtifactKind::ClaimRecord
        );
    }
}
