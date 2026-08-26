//! Closed preconditions that admit exactly one new operation attempt.
//!
//! A new attempt is a fresh admission, never a replay. Every precondition named here
//! is stated over current deterministic facts, and each precondition that names an
//! object distinguishes a freshly minted object from one retained by an earlier
//! attempt. The vocabulary is closed: there is no custom, wildcard, inherited, or
//! model-created variant, so an untrusted proposal may restate one fact but can never
//! broaden one. Every unestablished fact has an explicit variant that fails closed.

use serde::{Deserialize, Deserializer, Serialize};

use crate::{EffectDeclaration, OperationBinding};

/// Current version of the canonical new-attempt admission taxonomy.
pub const ATTEMPT_ADMISSION_VERSION: u16 = 1;

/// Closed precondition that must hold before one new operation attempt.
///
/// Variants are declared in the exact order they are evaluated, so at most one
/// precondition is reported for any denied attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptPrecondition {
    /// Deterministic preflight was re-evaluated against current state.
    Preflight,
    /// The prior attempt's effect on canonical state was reconciled.
    EffectReconciliation,
    /// At least one attempt remains within every declared budget.
    Budget,
    /// The attempt carries a call identity that no earlier attempt used.
    CallIdentity,
    /// The attempt carries a current, freshly issued, single-use operation grant.
    Grant,
    /// The attempt carries a fresh user decision when the operation requires one.
    Approval,
}

impl AttemptPrecondition {
    /// Every precondition in the exact order it is evaluated.
    pub const ALL: [Self; 6] = [
        Self::Preflight,
        Self::EffectReconciliation,
        Self::Budget,
        Self::CallIdentity,
        Self::Grant,
        Self::Approval,
    ];

    /// Returns a stable content-free precondition code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Preflight => "attempt.precondition.preflight",
            Self::EffectReconciliation => "attempt.precondition.effect_reconciliation",
            Self::Budget => "attempt.precondition.budget",
            Self::CallIdentity => "attempt.precondition.call_identity",
            Self::Grant => "attempt.precondition.grant",
            Self::Approval => "attempt.precondition.approval",
        }
    }
}

/// Freshness of the deterministic preflight evaluated for one candidate attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptPreflightState {
    /// Preflight was re-evaluated against current state and admits this attempt.
    CurrentAdmitted,
    /// Preflight was re-evaluated against current state and denies this attempt.
    CurrentDenied,
    /// The retained preflight result belongs to an earlier attempt, policy, or observation.
    Stale,
    /// No preflight was evaluated for this attempt.
    Absent,
}

/// Reconciled state of the prior attempt's effect on canonical state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptReconciliationState {
    /// No prior attempt of this operation crossed its launch boundary.
    NoPriorEffect,
    /// Fresh observation proves the prior attempt changed no canonical state.
    FailedNoChangeVerified,
    /// Fresh observation proves the prior attempt completed its exact effect.
    CompletedVerified,
    /// The prior attempt's effect on canonical state cannot be established.
    Unreconciled,
}

/// Remaining declared budget for one more attempt of this operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptBudgetState {
    /// At least one attempt remains within every declared budget.
    Remaining,
    /// A declared budget leaves no attempt for this operation.
    Exhausted,
    /// No budget is declared for this operation.
    Undeclared,
}

/// Freshness of the call identity bound to one candidate attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptCallIdentityState {
    /// A call identity minted for this exact attempt was never used before.
    Fresh,
    /// The candidate identity already names an earlier attempt.
    Reused,
    /// No call identity is bound to this attempt.
    Absent,
    /// The identity's binding to this exact attempt cannot be established.
    Unverified,
}

/// State of the operation grant bound to one candidate attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptGrantState {
    /// One current, freshly issued, single-use operation grant is bound to this attempt.
    FreshSingleUse,
    /// The bound grant authorizes more than one admission.
    MultiUse,
    /// The bound grant already recorded an admission or served an earlier attempt.
    Reused,
    /// The bound grant is not currently issued for this exact operation.
    NotCurrent,
    /// No operation grant is bound to this attempt.
    Absent,
}

/// State of the user decision bound to one candidate attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptApprovalState {
    /// A narrow user decision was recorded for this exact attempt.
    Fresh,
    /// The bound decision approved an earlier attempt.
    Reused,
    /// No user decision is bound to this attempt.
    Absent,
}

/// Idempotency identity carried across attempts of one identity-bound effect.
///
/// This identity is the one attempt-scoped fact that must stay stable rather than
/// become fresh: it is what makes a repeated identity-bound write converge instead of
/// duplicating an effect. It applies only to
/// [`EffectRepetition::IdentityBound`](crate::EffectRepetition::IdentityBound).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptIdempotencyIdentity {
    /// One verified identity binds this attempt to the prior attempt's exact effect.
    VerifiedStable,
    /// The identity differs from the prior attempt, so a new attempt would duplicate the effect.
    Changed,
    /// No verified idempotency identity exists.
    Absent,
}

/// Closed reason exactly one candidate attempt was not admitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptDenial {
    /// The attempt ordinal is not a one-based count.
    OrdinalInvalid,
    /// A first attempt cannot follow a prior effect of the same operation.
    FirstAttemptWithPriorEffect,
    /// An idempotency identity was supplied for an effect that is not identity bound.
    IdempotencyIdentityNotApplicable,
    /// No preflight was evaluated for this attempt.
    PreflightAbsent,
    /// The retained preflight result is not current for this attempt.
    PreflightStale,
    /// Current preflight denies this attempt.
    PreflightDenied,
    /// The prior attempt already completed its exact effect.
    EffectAlreadyCompleted,
    /// The prior attempt's effect on canonical state cannot be established.
    EffectUnreconciled,
    /// An identity-bound effect has no verified idempotency identity.
    IdempotencyIdentityAbsent,
    /// An identity-bound effect changed idempotency identity between attempts.
    IdempotencyIdentityChanged,
    /// No budget is declared for this operation.
    BudgetUndeclared,
    /// A declared budget leaves no attempt for this operation.
    BudgetExhausted,
    /// No call identity is bound to this attempt.
    CallIdentityAbsent,
    /// The bound call identity already names an earlier attempt.
    CallIdentityReused,
    /// The bound call identity cannot be verified against this attempt.
    CallIdentityUnverified,
    /// No operation grant is bound to this attempt.
    GrantAbsent,
    /// The bound grant is not currently issued for this exact operation.
    GrantNotCurrent,
    /// The bound grant authorizes more than one admission.
    GrantMultiUse,
    /// The bound grant already recorded an admission or served an earlier attempt.
    GrantReused,
    /// The operation requires a user decision and none is bound to this attempt.
    ApprovalAbsent,
    /// The bound user decision approved an earlier attempt.
    ApprovalReused,
}

impl AttemptDenial {
    /// Every denial reason in stable taxonomy order.
    pub const ALL: [Self; 21] = [
        Self::OrdinalInvalid,
        Self::FirstAttemptWithPriorEffect,
        Self::IdempotencyIdentityNotApplicable,
        Self::PreflightAbsent,
        Self::PreflightStale,
        Self::PreflightDenied,
        Self::EffectAlreadyCompleted,
        Self::EffectUnreconciled,
        Self::IdempotencyIdentityAbsent,
        Self::IdempotencyIdentityChanged,
        Self::BudgetUndeclared,
        Self::BudgetExhausted,
        Self::CallIdentityAbsent,
        Self::CallIdentityReused,
        Self::CallIdentityUnverified,
        Self::GrantAbsent,
        Self::GrantNotCurrent,
        Self::GrantMultiUse,
        Self::GrantReused,
        Self::ApprovalAbsent,
        Self::ApprovalReused,
    ];

    /// Returns the precondition this reason denied.
    ///
    /// The first three reasons return `None` because a malformed request is rejected
    /// before any precondition is evaluated.
    #[must_use]
    pub const fn precondition(self) -> Option<AttemptPrecondition> {
        match self {
            Self::OrdinalInvalid
            | Self::FirstAttemptWithPriorEffect
            | Self::IdempotencyIdentityNotApplicable => None,
            Self::PreflightAbsent | Self::PreflightStale | Self::PreflightDenied => {
                Some(AttemptPrecondition::Preflight)
            }
            Self::EffectAlreadyCompleted
            | Self::EffectUnreconciled
            | Self::IdempotencyIdentityAbsent
            | Self::IdempotencyIdentityChanged => Some(AttemptPrecondition::EffectReconciliation),
            Self::BudgetUndeclared | Self::BudgetExhausted => Some(AttemptPrecondition::Budget),
            Self::CallIdentityAbsent | Self::CallIdentityReused | Self::CallIdentityUnverified => {
                Some(AttemptPrecondition::CallIdentity)
            }
            Self::GrantAbsent | Self::GrantNotCurrent | Self::GrantMultiUse | Self::GrantReused => {
                Some(AttemptPrecondition::Grant)
            }
            Self::ApprovalAbsent | Self::ApprovalReused => Some(AttemptPrecondition::Approval),
        }
    }

    /// Returns a stable content-free denial code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::OrdinalInvalid => "attempt.deny.request.ordinal_invalid",
            Self::FirstAttemptWithPriorEffect => {
                "attempt.deny.request.first_attempt_with_prior_effect"
            }
            Self::IdempotencyIdentityNotApplicable => {
                "attempt.deny.request.idempotency_identity_not_applicable"
            }
            Self::PreflightAbsent => "attempt.deny.preflight.absent",
            Self::PreflightStale => "attempt.deny.preflight.stale",
            Self::PreflightDenied => "attempt.deny.preflight.denied",
            Self::EffectAlreadyCompleted => "attempt.deny.reconciliation.effect_already_completed",
            Self::EffectUnreconciled => "attempt.deny.reconciliation.unreconciled",
            Self::IdempotencyIdentityAbsent => {
                "attempt.deny.reconciliation.idempotency_identity_absent"
            }
            Self::IdempotencyIdentityChanged => {
                "attempt.deny.reconciliation.idempotency_identity_changed"
            }
            Self::BudgetUndeclared => "attempt.deny.budget.undeclared",
            Self::BudgetExhausted => "attempt.deny.budget.exhausted",
            Self::CallIdentityAbsent => "attempt.deny.call_identity.absent",
            Self::CallIdentityReused => "attempt.deny.call_identity.reused",
            Self::CallIdentityUnverified => "attempt.deny.call_identity.unverified",
            Self::GrantAbsent => "attempt.deny.grant.absent",
            Self::GrantNotCurrent => "attempt.deny.grant.not_current",
            Self::GrantMultiUse => "attempt.deny.grant.multi_use",
            Self::GrantReused => "attempt.deny.grant.reused",
            Self::ApprovalAbsent => "attempt.deny.approval.absent",
            Self::ApprovalReused => "attempt.deny.approval.reused",
        }
    }
}

/// Complete deterministic facts used to decide one candidate new attempt.
///
/// The facts carry no operation authority, no arguments, and no observed content. A
/// value of this type describes what is currently true; it never carries the objects
/// whose freshness it reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NewAttemptFacts {
    /// Exact registered operation the candidate attempt would perform.
    pub operation: OperationBinding,
    /// Exact declared effect of that registered operation.
    pub effect: EffectDeclaration,
    /// One-based ordinal of the attempt being considered.
    pub attempt_ordinal: u32,
    /// Whether the operation's registration requires an explicit user decision.
    pub registration_requires_approval: bool,
    /// Freshness of the deterministic preflight evaluated for this attempt.
    pub preflight: AttemptPreflightState,
    /// Reconciled state of the prior attempt's effect on canonical state.
    pub reconciliation: AttemptReconciliationState,
    /// Remaining declared budget for one more attempt.
    pub budget: AttemptBudgetState,
    /// Freshness of the call identity bound to this attempt.
    pub call_identity: AttemptCallIdentityState,
    /// State of the operation grant bound to this attempt.
    pub grant: AttemptGrantState,
    /// State of the user decision bound to this attempt.
    pub approval: AttemptApprovalState,
    /// Idempotency identity carried across attempts of an identity-bound effect.
    pub idempotency_identity: AttemptIdempotencyIdentity,
}

impl NewAttemptFacts {
    /// Reports whether this attempt requires its own fresh user decision.
    ///
    /// The declared effect and the operation registration are combined by narrowing
    /// only: either source can require a decision, and neither can waive one.
    #[must_use]
    pub const fn requires_fresh_approval(&self) -> bool {
        self.registration_requires_approval || self.effect.requires_fresh_approval()
    }
}

/// Closed outcome of one new-attempt admission decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NewAttemptDisposition {
    /// One new attempt may be prepared under the exact fresh objects already verified.
    Admitted,
    /// No new attempt is admitted.
    Denied,
}

/// Stable reason one admission decision record cannot be admitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttemptAdmissionError {
    /// The record names an admission version this build does not implement.
    UnsupportedVersion,
    /// The disposition does not agree with the presence of a denial reason.
    DispositionMismatch,
    /// The record claims a replay is permitted.
    ReplayNotPermitted,
}

impl AttemptAdmissionError {
    /// Returns a stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::UnsupportedVersion => "attempt.admission.version_unsupported",
            Self::DispositionMismatch => "attempt.admission.disposition_mismatch",
            Self::ReplayNotPermitted => "attempt.admission.replay_not_permitted",
        }
    }
}

/// One versioned admission decision that carries no operation authority.
///
/// An admitted decision states only that the six preconditions were satisfied from
/// current deterministic facts. It never authorizes an operation, and it can never
/// permit a replay: `replay_permitted` is structurally always `false`, and
/// construction and deserialization reject any record that claims otherwise.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NewAttemptDecision {
    admission_version: u16,
    disposition: NewAttemptDisposition,
    denial: Option<AttemptDenial>,
    approval_required: bool,
    replay_permitted: bool,
}

impl NewAttemptDecision {
    /// Creates the admitted decision for one attempt whose preconditions all held.
    #[must_use]
    pub const fn admitted(approval_required: bool) -> Self {
        Self {
            admission_version: ATTEMPT_ADMISSION_VERSION,
            disposition: NewAttemptDisposition::Admitted,
            denial: None,
            approval_required,
            replay_permitted: false,
        }
    }

    /// Creates the denied decision carrying exactly one reason.
    #[must_use]
    pub const fn denied(denial: AttemptDenial, approval_required: bool) -> Self {
        Self {
            admission_version: ATTEMPT_ADMISSION_VERSION,
            disposition: NewAttemptDisposition::Denied,
            denial: Some(denial),
            approval_required,
            replay_permitted: false,
        }
    }

    /// Validates and constructs one fully specified decision.
    pub fn from_parts(
        admission_version: u16,
        disposition: NewAttemptDisposition,
        denial: Option<AttemptDenial>,
        approval_required: bool,
        replay_permitted: bool,
    ) -> Result<Self, AttemptAdmissionError> {
        if admission_version != ATTEMPT_ADMISSION_VERSION {
            return Err(AttemptAdmissionError::UnsupportedVersion);
        }
        if replay_permitted {
            return Err(AttemptAdmissionError::ReplayNotPermitted);
        }
        match (disposition, denial) {
            (NewAttemptDisposition::Admitted, None) => Ok(Self::admitted(approval_required)),
            (NewAttemptDisposition::Denied, Some(reason)) => {
                Ok(Self::denied(reason, approval_required))
            }
            _ => Err(AttemptAdmissionError::DispositionMismatch),
        }
    }

    /// Returns the exact admission taxonomy version.
    #[must_use]
    pub const fn admission_version(self) -> u16 {
        self.admission_version
    }

    /// Returns the closed disposition.
    #[must_use]
    pub const fn disposition(self) -> NewAttemptDisposition {
        self.disposition
    }

    /// Reports whether one new attempt may be prepared.
    #[must_use]
    pub const fn is_admitted(self) -> bool {
        matches!(self.disposition, NewAttemptDisposition::Admitted)
    }

    /// Returns the single denial reason, when the attempt was denied.
    #[must_use]
    pub const fn denial(self) -> Option<AttemptDenial> {
        self.denial
    }

    /// Returns the precondition that denied the attempt, when one did.
    #[must_use]
    pub const fn failed_precondition(self) -> Option<AttemptPrecondition> {
        match self.denial {
            Some(denial) => denial.precondition(),
            None => None,
        }
    }

    /// Returns a stable content-free decision code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self.denial {
            Some(denial) => denial.code(),
            None => "attempt.admit.eligible",
        }
    }

    /// Reports whether this operation requires its own fresh user decision.
    #[must_use]
    pub const fn approval_required(self) -> bool {
        self.approval_required
    }

    /// Reports whether any prior call, grant, approval, or effect may be replayed.
    ///
    /// This is always `false`.
    #[must_use]
    pub const fn replay_permitted(self) -> bool {
        self.replay_permitted
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CurrentNewAttemptDecision {
    admission_version: u16,
    disposition: NewAttemptDisposition,
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    denial: Option<AttemptDenial>,
    approval_required: bool,
    replay_permitted: bool,
}

impl<'de> Deserialize<'de> for NewAttemptDecision {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let current = CurrentNewAttemptDecision::deserialize(deserializer)?;
        Self::from_parts(
            current.admission_version,
            current.disposition,
            current.denial,
            current.approval_required,
            current.replay_permitted,
        )
        .map_err(|error| serde::de::Error::custom(error.code()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        ATTEMPT_ADMISSION_VERSION, AttemptAdmissionError, AttemptApprovalState, AttemptBudgetState,
        AttemptCallIdentityState, AttemptDenial, AttemptGrantState, AttemptIdempotencyIdentity,
        AttemptPrecondition, AttemptPreflightState, AttemptReconciliationState, NewAttemptDecision,
        NewAttemptDisposition, NewAttemptFacts,
    };
    use crate::{
        EffectClass, EffectDeclaration, GrantOperation, OperationBinding, RetryDisposition,
    };

    fn facts(class: EffectClass) -> NewAttemptFacts {
        NewAttemptFacts {
            operation: OperationBinding::new(GrantOperation::WorkspaceWrite),
            effect: EffectDeclaration::new(class),
            attempt_ordinal: 2,
            registration_requires_approval: false,
            preflight: AttemptPreflightState::CurrentAdmitted,
            reconciliation: AttemptReconciliationState::FailedNoChangeVerified,
            budget: AttemptBudgetState::Remaining,
            call_identity: AttemptCallIdentityState::Fresh,
            grant: AttemptGrantState::FreshSingleUse,
            approval: AttemptApprovalState::Fresh,
            idempotency_identity: AttemptIdempotencyIdentity::Absent,
        }
    }

    #[test]
    fn every_denial_names_one_precondition_and_one_distinct_code() {
        assert_eq!(AttemptDenial::ALL.len(), 21);
        assert_eq!(AttemptPrecondition::ALL.len(), 6);

        let mut codes = BTreeSet::new();
        let mut covered = BTreeSet::new();
        for denial in AttemptDenial::ALL {
            assert!(codes.insert(denial.code()), "duplicate {denial:?}");
            assert!(denial.code().starts_with("attempt.deny."));
            if let Some(precondition) = denial.precondition() {
                covered.insert(precondition);
            }
        }
        assert_eq!(codes.len(), AttemptDenial::ALL.len());
        assert_eq!(covered, BTreeSet::from(AttemptPrecondition::ALL));

        let mut precondition_codes = BTreeSet::new();
        for precondition in AttemptPrecondition::ALL {
            assert!(precondition_codes.insert(precondition.code()));
            assert!(precondition.code().starts_with("attempt.precondition."));
        }
        assert_eq!(precondition_codes.len(), AttemptPrecondition::ALL.len());
    }

    #[test]
    fn only_the_three_request_reasons_are_evaluated_before_any_precondition() {
        let unbound = [
            AttemptDenial::OrdinalInvalid,
            AttemptDenial::FirstAttemptWithPriorEffect,
            AttemptDenial::IdempotencyIdentityNotApplicable,
        ];
        for denial in AttemptDenial::ALL {
            assert_eq!(
                denial.precondition().is_none(),
                unbound.contains(&denial),
                "denial {denial:?}"
            );
            assert_eq!(
                denial.code().starts_with("attempt.deny.request."),
                unbound.contains(&denial)
            );
        }
    }

    #[test]
    fn approval_requirement_narrows_and_is_never_waived_by_registration() {
        for class in EffectClass::ALL {
            let mut value = facts(class);
            assert_eq!(
                value.requires_fresh_approval(),
                class.requires_fresh_approval()
            );

            value.registration_requires_approval = true;
            assert!(value.requires_fresh_approval());
        }
    }

    #[test]
    fn decisions_round_trip_and_never_admit_a_replay() {
        let admitted = NewAttemptDecision::admitted(true);
        assert!(admitted.is_admitted());
        assert_eq!(admitted.denial(), None);
        assert_eq!(admitted.failed_precondition(), None);
        assert_eq!(admitted.code(), "attempt.admit.eligible");
        assert!(admitted.approval_required());
        assert!(!admitted.replay_permitted());
        assert_eq!(admitted.admission_version(), ATTEMPT_ADMISSION_VERSION);

        for denial in AttemptDenial::ALL {
            let decision = NewAttemptDecision::denied(denial, false);
            assert!(!decision.is_admitted());
            assert_eq!(decision.denial(), Some(denial));
            assert_eq!(decision.failed_precondition(), denial.precondition());
            assert_eq!(decision.code(), denial.code());
            assert!(!decision.replay_permitted());

            let encoded = serde_json::to_value(decision).expect("decision must encode");
            let decoded: NewAttemptDecision =
                serde_json::from_value(encoded.clone()).expect("decision must decode");
            assert_eq!(decoded, decision);

            let mut replayed = encoded.clone();
            replayed["replay_permitted"] = serde_json::json!(true);
            assert!(serde_json::from_value::<NewAttemptDecision>(replayed).is_err());

            let mut widened = encoded;
            widened["disposition"] = serde_json::json!("admitted");
            assert!(serde_json::from_value::<NewAttemptDecision>(widened).is_err());
        }

        let dropped = serde_json::json!({
            "admission_version": ATTEMPT_ADMISSION_VERSION,
            "disposition": "denied",
            "denial": serde_json::Value::Null,
            "approval_required": true,
            "replay_permitted": false
        });
        assert!(serde_json::from_value::<NewAttemptDecision>(dropped).is_err());

        let stale = serde_json::json!({
            "admission_version": ATTEMPT_ADMISSION_VERSION + 1,
            "disposition": "admitted",
            "denial": serde_json::Value::Null,
            "approval_required": false,
            "replay_permitted": false
        });
        assert!(serde_json::from_value::<NewAttemptDecision>(stale).is_err());
    }

    #[test]
    fn malformed_decision_parts_fail_closed_with_distinct_codes() {
        assert_eq!(
            NewAttemptDecision::from_parts(
                ATTEMPT_ADMISSION_VERSION + 1,
                NewAttemptDisposition::Admitted,
                None,
                false,
                false,
            ),
            Err(AttemptAdmissionError::UnsupportedVersion)
        );
        assert_eq!(
            NewAttemptDecision::from_parts(
                ATTEMPT_ADMISSION_VERSION,
                NewAttemptDisposition::Admitted,
                None,
                false,
                true,
            ),
            Err(AttemptAdmissionError::ReplayNotPermitted)
        );
        assert_eq!(
            NewAttemptDecision::from_parts(
                ATTEMPT_ADMISSION_VERSION,
                NewAttemptDisposition::Admitted,
                Some(AttemptDenial::BudgetExhausted),
                false,
                false,
            ),
            Err(AttemptAdmissionError::DispositionMismatch)
        );
        assert_eq!(
            NewAttemptDecision::from_parts(
                ATTEMPT_ADMISSION_VERSION,
                NewAttemptDisposition::Denied,
                None,
                false,
                false,
            ),
            Err(AttemptAdmissionError::DispositionMismatch)
        );

        let codes = [
            AttemptAdmissionError::UnsupportedVersion.code(),
            AttemptAdmissionError::DispositionMismatch.code(),
            AttemptAdmissionError::ReplayNotPermitted.code(),
        ];
        let unique: BTreeSet<&str> = codes.into_iter().collect();
        assert_eq!(unique.len(), codes.len());
        for code in codes {
            assert!(code.starts_with("attempt.admission."));
        }
    }

    #[test]
    fn attempt_vocabulary_shares_no_token_with_the_caller_retry_contract() {
        let mut attempt = BTreeSet::new();
        for state in [
            serde_json::to_value(AttemptPreflightState::CurrentAdmitted),
            serde_json::to_value(AttemptReconciliationState::NoPriorEffect),
            serde_json::to_value(AttemptBudgetState::Remaining),
            serde_json::to_value(AttemptCallIdentityState::Fresh),
            serde_json::to_value(AttemptGrantState::FreshSingleUse),
            serde_json::to_value(AttemptApprovalState::Fresh),
        ] {
            let encoded = state.expect("closed state must encode");
            attempt.insert(encoded.as_str().expect("token").to_owned());
        }

        let mut foreign = BTreeSet::new();
        for disposition in [
            RetryDisposition::Never,
            RetryDisposition::AfterCorrection,
            RetryDisposition::AfterDependencyRecovery,
            RetryDisposition::AfterUserDecision,
        ] {
            let encoded = serde_json::to_value(disposition).expect("closed value must encode");
            foreign.insert(encoded.as_str().expect("token").to_owned());
        }
        assert!(attempt.is_disjoint(&foreign));
    }

    #[test]
    fn custom_wildcard_and_inherited_admission_values_are_not_representable() {
        let base = serde_json::to_value(NewAttemptDecision::admitted(false)).expect("encode");
        for candidate in [
            "all",
            "any",
            "custom",
            "inherit",
            "*",
            "retry",
            "model_declared",
            "approve_all",
        ] {
            let mut drifted = base.clone();
            drifted["disposition"] = serde_json::Value::from(candidate);
            assert!(serde_json::from_value::<NewAttemptDecision>(drifted).is_err());

            let mut denial = base.clone();
            denial["denial"] = serde_json::Value::from(candidate);
            assert!(serde_json::from_value::<NewAttemptDecision>(denial).is_err());
        }

        let mut smuggled = base;
        smuggled["capability_grant"] = serde_json::json!({"claimed": true});
        assert!(serde_json::from_value::<NewAttemptDecision>(smuggled).is_err());
    }
}
