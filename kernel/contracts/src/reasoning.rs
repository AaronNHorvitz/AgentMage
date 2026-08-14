//! Concise reasoning, clarification, contradiction, and verification contracts.

use crate::{AuthorityClass, EvidenceReference, TaskId};

/// Calibrated status of one user-visible claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStatus {
    /// Established by current authoritative records.
    Known,
    /// Established by a direct current observation.
    DirectlyObserved,
    /// Derived from evidence but not directly observed.
    Inferred,
    /// Current evidence materially disagrees.
    Disputed,
    /// No safe value is currently known.
    Unknown,
    /// A proposed value has not been checked.
    Unverified,
}

/// Closed qualitative risk of relying on an assumption.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssumptionRisk {
    /// A wrong assumption has limited reversible effect.
    Low,
    /// A wrong assumption can materially change local work.
    Moderate,
    /// A wrong assumption can materially change outcome or safety.
    High,
    /// A wrong assumption can cross a critical authority or safety boundary.
    Critical,
}

/// Lifecycle state of one explicit assumption.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssumptionStatus {
    /// The assumption has not yet been checked.
    Unverified,
    /// Current evidence confirms the assumption.
    Confirmed,
    /// Current evidence rejects the assumption.
    Rejected,
}

/// Lifecycle state of one competing hypothesis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HypothesisStatus {
    /// The hypothesis remains open.
    Open,
    /// Current discriminating evidence supports the hypothesis.
    Supported,
    /// Current discriminating evidence rejects the hypothesis.
    Rejected,
    /// Current tests cannot discriminate safely.
    Inconclusive,
}

/// One evidence-calibrated known fact in a compact problem frame.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProblemFact {
    /// Stable exact key used for deterministic consistency checks.
    pub claim_key: String,
    /// Concise user-visible claim.
    pub statement: String,
    /// Current calibrated status.
    pub status: ClaimStatus,
    /// Evidence supporting the current status.
    pub evidence: Vec<EvidenceReference>,
}

/// Compact bounded frame for substantial work.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProblemFrame {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact task identity.
    pub task_id: TaskId,
    /// Exact objective.
    pub objective: String,
    /// Current known or explicitly calibrated facts.
    pub known_facts: Vec<ProblemFact>,
    /// Explicit constraints.
    pub constraints: Vec<String>,
    /// Explicit unknowns.
    pub unknowns: Vec<String>,
    /// Exact acceptance checks.
    pub acceptance_checks: Vec<String>,
    /// Concise material risks.
    pub risks: Vec<String>,
    /// Descriptive authority boundary; this field grants nothing.
    pub authority_boundary: AuthorityClass,
}

/// One explicit material assumption and its verification state.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssumptionRecord {
    /// Stable identity within one task ledger.
    pub assumption_id: String,
    /// Concise assumption.
    pub statement: String,
    /// Why the assumption is currently needed.
    pub rationale: String,
    /// Consequence of relying on it incorrectly.
    pub risk: AssumptionRisk,
    /// Deterministic or human check that can resolve it.
    pub verification_method: String,
    /// Current lifecycle state.
    pub status: AssumptionStatus,
    /// Evidence used to confirm or reject it.
    pub evidence: Vec<EvidenceReference>,
}

/// One competing explanation and its discriminating evidence.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HypothesisRecord {
    /// Stable identity within one task ledger.
    pub hypothesis_id: String,
    /// Concise proposed explanation.
    pub statement: String,
    /// Evidence supporting the explanation.
    pub evidence_for: Vec<EvidenceReference>,
    /// Evidence weighing against the explanation.
    pub evidence_against: Vec<EvidenceReference>,
    /// Bounded tests capable of distinguishing explanations.
    pub discriminating_tests: Vec<String>,
    /// Current lifecycle state.
    pub status: HypothesisStatus,
}

/// One exact claim value supplied to deterministic consistency checking.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimAssertion {
    /// Stable exact semantic key.
    pub claim_key: String,
    /// Exact canonical candidate value.
    pub value: String,
    /// Current calibrated status.
    pub status: ClaimStatus,
    /// Evidence supporting the assertion.
    pub evidence: Vec<EvidenceReference>,
}

/// One deterministic conflict between two exact assertions.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContradictionRecord {
    /// Exact conflicting semantic key.
    pub claim_key: String,
    /// Zero-based first assertion index.
    pub left_index: u32,
    /// Zero-based conflicting assertion index.
    pub right_index: u32,
}

/// Material dimension changed by an unanswered question.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ClarificationImpact {
    /// Requested work scope can change.
    Scope,
    /// Required authority can change.
    Authority,
    /// Material cost can change.
    Cost,
    /// Safety can change.
    Safety,
    /// The resulting answer or artifact can materially change.
    Result,
}

/// One material question and optional explicit answer.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClarificationQuestion {
    /// Stable identity within one task ledger.
    pub question_id: String,
    /// Concise user-visible question.
    pub question: String,
    /// Nonempty set of material impact classes.
    pub impacts: Vec<ClarificationImpact>,
    /// Explicit answer when resolved.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub answer: Option<String>,
}

/// Closed result of the deterministic clarification gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClarificationState {
    /// No material unanswered question remains.
    Ready,
    /// At least one material question requires the user.
    UserDecisionRequired,
}

/// Independent verification input with no first-pass assumption or conclusion field.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndependentVerificationRequest {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact task identity.
    pub task_id: TaskId,
    /// Candidate result to verify independently.
    pub proposed_result: String,
    /// Exact acceptance checks from the problem frame.
    pub acceptance_checks: Vec<String>,
    /// Evidence available to the independent pass.
    pub evidence: Vec<EvidenceReference>,
}

/// Closed independent-verification disposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationDisposition {
    /// Every acceptance check independently passed.
    Pass,
    /// At least one acceptance check independently failed.
    Fail,
    /// Verification requires a missing decision or dependency.
    Blocked,
    /// Current evidence cannot establish a safe disposition.
    Unknown,
}

/// Bounded independent-verification result.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndependentVerificationResult {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact task identity.
    pub task_id: TaskId,
    /// Closed verification disposition.
    pub disposition: VerificationDisposition,
    /// Exact acceptance checks independently verified.
    pub verified_checks: Vec<String>,
    /// Concise failures, blockers, contradictions, or unknowns.
    pub findings: Vec<String>,
    /// Evidence used by the independent pass.
    pub evidence: Vec<EvidenceReference>,
}
