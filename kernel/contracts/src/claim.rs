//! Material-claim and evidence-role contracts for truthful completion.

use crate::{EvidenceReference, ModelManifestObservation, ModelRunId, Receipt, TaskId};

/// Exactly one user-visible evidence state assigned to a material claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialClaimEvidenceStateKind {
    /// Direct output from an authorized deterministic observation.
    Observed,
    /// Output computed by a versioned deterministic method from observations.
    Derived,
    /// A model interpretation supported by citations but not mechanically proven.
    Inferred,
    /// Evidence is unavailable, unsafe to use, or outside the authorized scope.
    UnknownBlocked,
}

/// Closed reason an evidence assignment must remain Unknown/Blocked.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum UnknownBlockedReason {
    /// Required evidence or a dependency is unavailable.
    Unavailable,
    /// Policy or authority denied the required observation.
    Denied,
    /// The deterministic attempt failed.
    Failed,
    /// Current evidence sources conflict.
    Conflict,
    /// A supporting source no longer matches its observed identity.
    StaleEvidence,
    /// The source format or structure is unsupported by the approved parser set.
    UnsupportedParsing,
    /// Available data cannot be verified under the current contracts.
    UnverifiableData,
    /// The required source or action lies outside the authorized task scope.
    ScopeExcluded,
}

/// Exact identity of one approved deterministic derivation implementation.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeterministicMethodIdentity {
    /// Stable method identity.
    pub method_id: String,
    /// Immutable method-contract version.
    pub method_version: String,
    /// Lowercase SHA-256 digest of the exact implementation or specification.
    pub implementation_sha256: String,
}

/// Provenance for a direct observed claim.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedClaimProvenance {
    /// Authority-bound terminal receipt for the deterministic observation.
    pub receipt: Receipt,
    /// Exact content-addressed identities returned by that receipt.
    pub sources: Vec<EvidenceReference>,
}

/// Provenance for a deterministic derived claim.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DerivedClaimProvenance {
    /// Registered deterministic method used for the derivation.
    pub method: DeterministicMethodIdentity,
    /// Exact Observed assignment identities consumed as inputs.
    pub observed_input_assignment_ids: Vec<String>,
}

/// Exact model and runtime identity supporting an inferred claim.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InferenceRuntimeProvenance {
    /// Exact bounded model run that produced the interpretation.
    pub model_run_id: ModelRunId,
    /// Runtime observation for the exact model, artifacts, codec, and adapter.
    pub manifest: ModelManifestObservation,
    /// Lowercase SHA-256 digest of the complete model response bytes.
    pub response_sha256: String,
}

/// Provenance for a model-inferred claim.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InferredClaimProvenance {
    /// Supporting content-addressed citations; these support but do not prove the claim.
    pub citations: Vec<EvidenceReference>,
    /// Exact model-run and runtime manifest identity.
    pub runtime: InferenceRuntimeProvenance,
}

/// Reasoned provenance for an Unknown/Blocked claim.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnknownBlockedClaimProvenance {
    /// Closed reason that prevents stronger evidence classification.
    pub reason: UnknownBlockedReason,
    /// Stable, content-free detail code suitable for audit and user explanation.
    pub detail_code: String,
    /// Available supporting identities, including conflicting or stale references.
    pub evidence: Vec<EvidenceReference>,
}

/// Closed state-specific provenance payload for one material claim.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(
    tag = "state",
    content = "provenance",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum MaterialClaimEvidenceState {
    /// Direct deterministic observation.
    Observed(Box<ObservedClaimProvenance>),
    /// Deterministic transformation of Observed inputs.
    Derived(Box<DerivedClaimProvenance>),
    /// Model interpretation with supporting citations.
    Inferred(Box<InferredClaimProvenance>),
    /// Reasoned inability to establish a stronger state.
    UnknownBlocked(Box<UnknownBlockedClaimProvenance>),
}

impl MaterialClaimEvidenceState {
    /// Returns the one closed user-visible state represented by this payload.
    #[must_use]
    pub const fn kind(&self) -> MaterialClaimEvidenceStateKind {
        match self {
            Self::Observed(_) => MaterialClaimEvidenceStateKind::Observed,
            Self::Derived(_) => MaterialClaimEvidenceStateKind::Derived,
            Self::Inferred(_) => MaterialClaimEvidenceStateKind::Inferred,
            Self::UnknownBlocked(_) => MaterialClaimEvidenceStateKind::UnknownBlocked,
        }
    }
}

/// One validated evidence-state assignment for one exact material claim.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialClaimEvidenceAssignment {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable task-local assignment identity.
    pub assignment_id: String,
    /// Exact material claim receiving the state.
    pub claim: MaterialClaim,
    /// Exactly one closed evidence state and its required provenance.
    pub evidence_state: MaterialClaimEvidenceState,
}

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
