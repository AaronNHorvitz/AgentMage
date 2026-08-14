//! Untrusted typed postcondition result contracts.

use crate::{
    EvidenceReference, PostconditionId, ProposalId, RepositorySnapshotId, TaskId, VerifierId,
    VerifierRecordId,
};

/// Claimed producer class for one verifier candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerifierSource {
    /// Registered deterministic postcondition implementation.
    DeterministicPostcondition,
    /// Natural-language model statement.
    ModelProse,
    /// Model or classifier confidence value.
    Confidence,
    /// Model self-review output.
    SelfReview,
    /// Separate model-judge output.
    ModelJudge,
    /// Advisory classifier output.
    Classifier,
}

/// Claimed terminal disposition of a verifier candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerifierDisposition {
    /// All registered postconditions claim a changed successful result.
    Success,
    /// All registered postconditions claim an already-satisfied no-op result.
    NoOp,
    /// At least one postcondition failed or completion is not established.
    Failed,
}

/// Result for one exact registered postcondition.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostconditionResult {
    /// Registered postcondition identity.
    pub postcondition_id: PostconditionId,
    /// Whether the deterministic check passed.
    pub passed: bool,
    /// Current evidence supporting this exact check.
    pub evidence: Vec<EvidenceReference>,
}

/// Untrusted candidate record evaluated by the deterministic verifier registry.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifierCandidate {
    /// Contract schema version.
    pub schema_version: u16,
    /// Unique result-record identity.
    pub verifier_record_id: VerifierRecordId,
    /// Exact registered verifier identity.
    pub verifier_id: VerifierId,
    /// Exact task identity.
    pub task_id: TaskId,
    /// Exact admitted proposal identity.
    pub proposal_id: ProposalId,
    /// Exact current repository snapshot.
    pub repository_snapshot_id: RepositorySnapshotId,
    /// Exact state revision evaluated by the verifier.
    pub state_revision: u64,
    /// Claimed producer class.
    pub source: VerifierSource,
    /// Claimed terminal disposition.
    pub disposition: VerifierDisposition,
    /// Complete registered postcondition results.
    pub postconditions: Vec<PostconditionResult>,
}
