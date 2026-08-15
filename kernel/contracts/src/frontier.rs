//! Measured frontier-recommendation and content-free receipt contracts.

/// Closed task execution or escalation tier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrontierTaskTier {
    /// A deterministic registered operation is sufficient.
    DeterministicScript,
    /// An admitted local model remains the appropriate bounded path.
    LocalModel,
    /// A material decision or clarification must come from the user.
    AskUser,
    /// External consultation may be recommended for manual user handling.
    FrontierRecommended,
}

/// Closed evidence-backed reason external consultation may be recommended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrontierRecommendationTrigger {
    /// The attempted local path failed or cannot satisfy one named acceptance check.
    MeasuredCapabilityFailure,
    /// At least two exact validation attempts failed the same acceptance check.
    RepeatedValidationFailure,
    /// Current local evidence contains an unresolved material contradiction.
    Contradiction,
    /// Deterministic verification rejected the local result.
    RejectedVerification,
    /// The exact local task budget was exhausted without verified completion.
    ExhaustedBudget,
    /// External expertise is needed after separating any user-owned decision.
    MaterialClarificationNeed,
}

/// Closed clarification ownership that separates user decisions from external advice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrontierClarificationClass {
    /// No material clarification is currently required.
    None,
    /// The user must supply or choose the missing fact.
    UserDecision,
    /// The remaining question may benefit from external expertise.
    ExternalExpertise,
}

/// Closed state of one exact local acceptance check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrontierAcceptanceState {
    /// Deterministic evidence verifies the check.
    Passed,
    /// Current deterministic evidence disproves the check.
    Failed,
    /// Required evidence was not obtained.
    Unmeasured,
    /// The admitted local capability cannot perform the check.
    Unsupported,
}

/// Exact local evidence used for one deterministic tier decision.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontierTierEvidence {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable decision identity.
    pub decision_id: String,
    /// Whether one deterministic registered operation applies.
    pub deterministic_available: bool,
    /// Whether that deterministic operation completed the acceptance check.
    pub deterministic_succeeded: bool,
    /// Whether an admitted local-model path was actually attempted.
    pub local_model_attempted: bool,
    /// Exact acceptance-check identity whose state controls the decision.
    pub local_acceptance_check_id: String,
    /// Current evidence-backed state of that check.
    pub local_acceptance_state: FrontierAcceptanceState,
    /// Number of exact failed validation attempts for the same check.
    pub validation_failure_count: u8,
    /// Number of unresolved material contradictions.
    pub contradiction_count: u8,
    /// Whether deterministic verification rejected the local result.
    pub verification_rejected: bool,
    /// Whether the exact local task budget was exhausted.
    pub budget_exhausted: bool,
    /// Ownership of any remaining material clarification.
    pub clarification: FrontierClarificationClass,
    /// Sorted content hashes of the exact evidence supporting the fields above.
    pub evidence_sha256: Vec<String>,
}

/// Content-free deterministic tier decision with no external authority.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontierTierDecision {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable decision identity.
    pub decision_id: String,
    /// Selected closed tier.
    pub tier: FrontierTaskTier,
    /// Exact approved recommendation trigger, if any.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub trigger: Option<FrontierRecommendationTrigger>,
    /// Exact acceptance check cited by the decision.
    pub local_acceptance_check_id: String,
    /// Sorted supporting evidence hashes.
    pub evidence_sha256: Vec<String>,
    /// Stable content-free explanation code.
    pub reason_code: String,
    /// True only for a direct user clarification decision.
    pub user_clarification_required: bool,
    /// True only when the result is a recommendation rather than an action.
    pub recommendation_only: bool,
    /// Always false; a tier decision cannot create an external effect.
    pub external_effect_allowed: bool,
    /// Digest of every preceding field and the complete source evidence.
    pub decision_sha256: String,
}

/// Content-free record of a local recommendation and optional user-recorded destination.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontierRecommendationReceipt {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable receipt identity.
    pub receipt_id: String,
    /// Exact tier-decision digest.
    pub decision_sha256: String,
    /// Stable recommendation reason code.
    pub reason_code: String,
    /// Exact locally reviewed packet digest.
    pub packet_sha256: String,
    /// Digest of the redaction and disclosure result without excerpt bytes.
    pub disclosure_result_sha256: String,
    /// Optional destination label entered by the user for record keeping only.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub user_recorded_destination: Option<String>,
    /// True only when the user explicitly chose to retain the destination label.
    pub destination_recording_requested: bool,
    /// Always false; no packet delivery occurred.
    pub external_delivery_attempted: bool,
    /// Digest of every preceding receipt field.
    pub receipt_sha256: String,
}
