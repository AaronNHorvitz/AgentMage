//! Material-claim and evidence-role contracts for truthful completion.

use crate::{EvidenceReference, TaskId};

/// Closed class of user-visible material claim requiring deterministic proof.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum MaterialClaimKind {
    /// Content or state was read.
    Read,
    /// User-owned state was changed.
    Change,
    /// A test or validation command ran to a terminal result.
    Test,
    /// A repository commit was created.
    Commit,
    /// A repository ref was pushed to a remote.
    Push,
    /// An artifact or release was published.
    Publish,
    /// The declared task acceptance checks completed.
    Complete,
}

/// Closed semantic role played by one evidence reference.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ClaimEvidenceRole {
    /// Terminal receipt for the exact attempted operation.
    OperationReceipt,
    /// Direct observation of the read object and revision.
    ReadObservation,
    /// Direct postcondition observation after a state change.
    Postcondition,
    /// Bounded raw output from the exact test attempt.
    TestOutput,
    /// Deterministic validation of the test result.
    TestValidation,
    /// Direct observation of the created commit identity.
    CommitObservation,
    /// Direct observation of the destination remote ref.
    RemoteObservation,
    /// Direct observation of the published artifact identity.
    PublicationObservation,
    /// Independent validation of all declared acceptance checks.
    AcceptanceVerification,
}

/// One evidence reference assigned one exact claim-proof role.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimEvidence {
    /// Exact semantic role in the proof.
    pub role: ClaimEvidenceRole,
    /// Content-addressed evidence reference.
    pub evidence: EvidenceReference,
}

/// One material claim proposed for deterministic verification.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialClaim {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable identity within the task claim ledger.
    pub claim_id: String,
    /// Exact task identity.
    pub task_id: TaskId,
    /// Closed claim class.
    pub kind: MaterialClaimKind,
    /// Concise user-visible claim statement.
    pub statement: String,
    /// Stable subject identity without ambient access authority.
    pub subject_id: String,
    /// Exact current revision every proof item must bind.
    pub expected_revision: String,
    /// Earlier verified material claims required by this claim.
    pub prerequisite_claim_ids: Vec<String>,
}

/// One deterministically verified material claim.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifiedMaterialClaim {
    /// Exact proposed claim.
    pub claim: MaterialClaim,
    /// Exact role-complete current proof.
    pub proof: Vec<ClaimEvidence>,
}

/// A concise final response paired with its complete verified claim set.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimBoundFinalResponse {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact task identity.
    pub task_id: TaskId,
    /// Bounded user-visible response text.
    pub summary: String,
    /// Exact verified claims represented by the response.
    pub verified_claims: Vec<VerifiedMaterialClaim>,
}
