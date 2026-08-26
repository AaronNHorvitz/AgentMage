//! Canonical failure taxonomy used by retry and recovery policy.
//!
//! Every failed operation attempt is exactly one closed [`FailureClass`], and each
//! class fixes one default [`FailureDisposition`]. The disposition is derived from
//! deterministic state alone: a model proposes no class and no disposition, and an
//! untrusted record that restates a rule differently is rejected rather than
//! admitted. There is no custom, wildcard, inherited, or model-created class; a
//! failure whose outcome cannot be established is exactly
//! [`FailureClass::UncertainEffect`] and fails closed.
//!
//! The taxonomy is independent of the [`EffectClass`](crate::EffectClass) taxonomy
//! and never replaces it. A failure class describes what happened to one attempt;
//! an effect class describes what an attempt does to state. The effective
//! disposition of a failed attempt is the more restrictive of the two, so an
//! effect that admits no automatic repetition can never be widened by the failure
//! class that interrupted it.

use serde::{Deserialize, Deserializer, Serialize};

use crate::{CanonicalRetryDisposition, EffectClass, EffectRepetition};

/// Current version of the canonical failure taxonomy.
pub const FAILURE_TAXONOMY_VERSION: u16 = 1;

/// Closed recovery disposition fixed by one failure class.
///
/// Variants are declared from least to most restrictive. The derived ordering is
/// meaningful: combining declarations may only move toward the end of this list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureDisposition {
    /// One fresh attempt is admitted from deterministic state and remaining budget.
    FreshAttempt,
    /// A new attempt is considered only after current effect state is reconciled.
    AfterReconciliation,
    /// Only an explicit new user decision can admit a further attempt.
    UserDecision,
    /// No further attempt is admitted; the workflow terminates with a diagnosis.
    Terminal,
}

impl FailureDisposition {
    /// Every disposition from least to most restrictive.
    pub const ALL: [Self; 4] = [
        Self::FreshAttempt,
        Self::AfterReconciliation,
        Self::UserDecision,
        Self::Terminal,
    ];

    /// Returns the restriction rank of this disposition.
    ///
    /// A higher rank is never less restrictive in any derived rule. The ranks form
    /// a closed total order so that combining declarations stays deterministic.
    #[must_use]
    pub const fn restriction_rank(self) -> u8 {
        match self {
            Self::FreshAttempt => 0,
            Self::AfterReconciliation => 1,
            Self::UserDecision => 2,
            Self::Terminal => 3,
        }
    }

    /// Returns the more restrictive of two dispositions.
    ///
    /// Combining can only raise the restriction rank, so no additional source can
    /// broaden a disposition that is already declared.
    #[must_use]
    pub const fn most_restrictive(self, other: Self) -> Self {
        if other.restriction_rank() > self.restriction_rank() {
            other
        } else {
            self
        }
    }

    /// Reports whether any further attempt of the same operation is admitted.
    #[must_use]
    pub const fn permits_new_attempt(self) -> bool {
        !matches!(self, Self::Terminal)
    }

    /// Returns the least restrictive disposition one effect class can produce.
    ///
    /// The floor is fixed by the effect's repetition rule, so an effect that admits
    /// no automatic repetition never returns anything weaker than
    /// [`Self::UserDecision`] regardless of which failure class was observed.
    #[must_use]
    pub const fn floor_for_effect(effect: EffectClass) -> Self {
        match effect.repetition() {
            EffectRepetition::SafeToRepeat => Self::FreshAttempt,
            EffectRepetition::IdentityBound => Self::FreshAttempt,
            EffectRepetition::ReconciliationRequired => Self::AfterReconciliation,
            EffectRepetition::NeverAutomatic => Self::UserDecision,
        }
    }

    /// Binds one recorded attempt-observation disposition to its exact value.
    #[must_use]
    pub const fn from_canonical(recorded: CanonicalRetryDisposition) -> Self {
        match recorded {
            CanonicalRetryDisposition::EligibleFreshAttempt => Self::FreshAttempt,
            CanonicalRetryDisposition::ReconcileFirst => Self::AfterReconciliation,
            CanonicalRetryDisposition::UserDecisionRequired => Self::UserDecision,
            CanonicalRetryDisposition::NotEligible => Self::Terminal,
        }
    }

    /// Returns the exact recorded disposition for one tool-attempt observation.
    ///
    /// The recorded vocabulary names the most restrictive value `not_eligible`
    /// because no further attempt is eligible. The kernel names it
    /// [`Self::Terminal`] because the workflow terminates with one diagnosis
    /// instead of waiting for an eligibility that cannot arrive.
    #[must_use]
    pub const fn to_canonical(self) -> CanonicalRetryDisposition {
        match self {
            Self::FreshAttempt => CanonicalRetryDisposition::EligibleFreshAttempt,
            Self::AfterReconciliation => CanonicalRetryDisposition::ReconcileFirst,
            Self::UserDecision => CanonicalRetryDisposition::UserDecisionRequired,
            Self::Terminal => CanonicalRetryDisposition::NotEligible,
        }
    }
}

/// Closed failure class of one operation attempt.
///
/// The class describes only how one attempt failed. It never describes who may
/// authorize a further attempt and never describes what the operation does to
/// state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureClass {
    /// The request failed a closed input contract before it could be dispatched.
    MalformedInput,
    /// A deterministic precondition did not hold, so nothing was dispatched.
    Preflight,
    /// Current policy denied the attempt before any authority was consumed.
    PolicyDenied,
    /// The attempt requires an approval that is absent, stale, or already consumed.
    ApprovalRequired,
    /// A required dependency was unavailable, so the attempt never proceeded.
    DependencyUnavailable,
    /// The attempt failed without changing state, and that is established.
    Transient,
    /// A concurrent change invalidated the state the attempt depended on.
    Conflict,
    /// The attempt exceeded its deadline with an unobserved outcome.
    Timeout,
    /// The attempt was cancelled.
    Cancellation,
    /// The attempt or its worker terminated abnormally.
    Crash,
    /// The attempt completed with an outcome that cannot be established.
    UncertainEffect,
    /// A deterministic postcondition observed that the attempt did not converge.
    Verification,
    /// A declared resource or budget bound was reached.
    ResourceExhausted,
    /// An internal invariant failed without exposing private state.
    Internal,
}

impl FailureClass {
    /// Every failure class in stable taxonomy order.
    pub const ALL: [Self; 14] = [
        Self::MalformedInput,
        Self::Preflight,
        Self::PolicyDenied,
        Self::ApprovalRequired,
        Self::DependencyUnavailable,
        Self::Transient,
        Self::Conflict,
        Self::Timeout,
        Self::Cancellation,
        Self::Crash,
        Self::UncertainEffect,
        Self::Verification,
        Self::ResourceExhausted,
        Self::Internal,
    ];

    /// Returns the one default disposition fixed for this class.
    ///
    /// The default is the disposition of the failure alone. The effective
    /// disposition of a failed attempt is [`Self::disposition_for`], which also
    /// applies the floor fixed by the operation's effect class.
    #[must_use]
    pub const fn disposition(self) -> FailureDisposition {
        match self {
            Self::MalformedInput | Self::Preflight => FailureDisposition::FreshAttempt,
            Self::DependencyUnavailable | Self::Transient => FailureDisposition::FreshAttempt,
            Self::Conflict | Self::Timeout => FailureDisposition::AfterReconciliation,
            Self::Crash | Self::Verification => FailureDisposition::AfterReconciliation,
            Self::ApprovalRequired | Self::UncertainEffect => FailureDisposition::UserDecision,
            Self::PolicyDenied | Self::Cancellation => FailureDisposition::Terminal,
            Self::ResourceExhausted | Self::Internal => FailureDisposition::Terminal,
        }
    }

    /// Reports whether an effect attempt may already have started for this class.
    ///
    /// Six classes report `false`: four cannot reach the operation at all, and
    /// [`Self::DependencyUnavailable`] and [`Self::Transient`] are admitted only
    /// when the attempt is established to have changed no state. Every other class
    /// reports `true` and fails closed, so an outcome that is merely assumed to be
    /// state-preserving is exactly [`Self::UncertainEffect`].
    #[must_use]
    pub const fn may_have_attempted_effect(self) -> bool {
        !matches!(
            self,
            Self::MalformedInput
                | Self::Preflight
                | Self::PolicyDenied
                | Self::ApprovalRequired
                | Self::DependencyUnavailable
                | Self::Transient
        )
    }

    /// Reports whether current effect state must be reconciled after this failure.
    ///
    /// Reconciliation is required exactly when an effect attempt may already have
    /// started, including when no further attempt is admitted at all.
    #[must_use]
    pub const fn requires_reconciliation(self) -> bool {
        self.may_have_attempted_effect()
    }

    /// Reports whether any further attempt of the same operation is admitted.
    #[must_use]
    pub const fn permits_new_attempt(self) -> bool {
        self.disposition().permits_new_attempt()
    }

    /// Reports whether only an explicit new user decision can admit a further attempt.
    #[must_use]
    pub const fn requires_user_decision(self) -> bool {
        matches!(self.disposition(), FailureDisposition::UserDecision)
    }

    /// Reports whether one bounded model repair of the request is permitted.
    ///
    /// Repair is permitted only for a request that never reached the operation and
    /// only after deterministic normalization. Repair itself attempts no effect, so
    /// it stays permitted even when the repaired attempt requires a user decision.
    #[must_use]
    pub const fn permits_model_repair(self) -> bool {
        matches!(self, Self::MalformedInput)
    }

    /// Returns the effective disposition for this failure of one effect class.
    ///
    /// The result is the more restrictive of the class default and the floor fixed
    /// by the effect, so neither taxonomy can broaden the other.
    #[must_use]
    pub const fn disposition_for(self, effect: EffectClass) -> FailureDisposition {
        self.disposition()
            .most_restrictive(FailureDisposition::floor_for_effect(effect))
    }
}

/// Stable reason one failure declaration cannot be admitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureTaxonomyError {
    /// The record names a taxonomy version this build does not implement.
    UnsupportedVersion,
    /// The disposition does not equal the canonical default for the class.
    DispositionMismatch,
    /// The effect-attempt rule does not equal the canonical rule for the class.
    EffectAttemptMismatch,
    /// The new-attempt rule does not equal the canonical rule for the class.
    NewAttemptMismatch,
    /// The user-decision rule does not equal the canonical rule for the class.
    UserDecisionMismatch,
    /// The model-repair rule does not equal the canonical rule for the class.
    ModelRepairMismatch,
}

impl FailureTaxonomyError {
    /// Returns a stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::UnsupportedVersion => "failure.taxonomy.version_unsupported",
            Self::DispositionMismatch => "failure.taxonomy.disposition_mismatch",
            Self::EffectAttemptMismatch => "failure.taxonomy.effect_attempt_mismatch",
            Self::NewAttemptMismatch => "failure.taxonomy.new_attempt_mismatch",
            Self::UserDecisionMismatch => "failure.taxonomy.user_decision_mismatch",
            Self::ModelRepairMismatch => "failure.taxonomy.model_repair_mismatch",
        }
    }
}

/// Versioned failure class together with the exact default rules it fixes.
///
/// Every rule is derived from the class. Construction and deserialization reject
/// any caller-selected rule that differs from the canonical derivation, so an
/// untrusted record may restate one declaration but can never relax one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FailureDeclaration {
    taxonomy_version: u16,
    failure_class: FailureClass,
    disposition: FailureDisposition,
    may_have_attempted_effect: bool,
    permits_new_attempt: bool,
    requires_user_decision: bool,
    permits_model_repair: bool,
}

impl FailureDeclaration {
    /// Creates the current canonical declaration for one failure class.
    #[must_use]
    pub const fn new(failure_class: FailureClass) -> Self {
        Self {
            taxonomy_version: FAILURE_TAXONOMY_VERSION,
            failure_class,
            disposition: failure_class.disposition(),
            may_have_attempted_effect: failure_class.may_have_attempted_effect(),
            permits_new_attempt: failure_class.permits_new_attempt(),
            requires_user_decision: failure_class.requires_user_decision(),
            permits_model_repair: failure_class.permits_model_repair(),
        }
    }

    /// Validates and constructs one fully specified declaration.
    pub fn from_parts(
        taxonomy_version: u16,
        failure_class: FailureClass,
        disposition: FailureDisposition,
        may_have_attempted_effect: bool,
        permits_new_attempt: bool,
        requires_user_decision: bool,
        permits_model_repair: bool,
    ) -> Result<Self, FailureTaxonomyError> {
        if taxonomy_version != FAILURE_TAXONOMY_VERSION {
            return Err(FailureTaxonomyError::UnsupportedVersion);
        }
        if disposition != failure_class.disposition() {
            return Err(FailureTaxonomyError::DispositionMismatch);
        }
        if may_have_attempted_effect != failure_class.may_have_attempted_effect() {
            return Err(FailureTaxonomyError::EffectAttemptMismatch);
        }
        if permits_new_attempt != failure_class.permits_new_attempt() {
            return Err(FailureTaxonomyError::NewAttemptMismatch);
        }
        if requires_user_decision != failure_class.requires_user_decision() {
            return Err(FailureTaxonomyError::UserDecisionMismatch);
        }
        if permits_model_repair != failure_class.permits_model_repair() {
            return Err(FailureTaxonomyError::ModelRepairMismatch);
        }
        Ok(Self::new(failure_class))
    }

    /// Returns the exact taxonomy version.
    #[must_use]
    pub const fn taxonomy_version(self) -> u16 {
        self.taxonomy_version
    }

    /// Returns the exact declared failure class.
    #[must_use]
    pub const fn failure_class(self) -> FailureClass {
        self.failure_class
    }

    /// Returns the default disposition fixed by the declared class.
    #[must_use]
    pub const fn disposition(self) -> FailureDisposition {
        self.disposition
    }

    /// Returns the effective disposition of this failure for one effect class.
    #[must_use]
    pub const fn disposition_for(self, effect: EffectClass) -> FailureDisposition {
        self.failure_class.disposition_for(effect)
    }

    /// Reports whether an effect attempt may already have started.
    #[must_use]
    pub const fn may_have_attempted_effect(self) -> bool {
        self.may_have_attempted_effect
    }

    /// Reports whether current effect state must be reconciled after this failure.
    #[must_use]
    pub const fn requires_reconciliation(self) -> bool {
        self.may_have_attempted_effect
    }

    /// Reports whether any further attempt of the same operation is admitted.
    #[must_use]
    pub const fn permits_new_attempt(self) -> bool {
        self.permits_new_attempt
    }

    /// Reports whether only an explicit new user decision admits a further attempt.
    #[must_use]
    pub const fn requires_user_decision(self) -> bool {
        self.requires_user_decision
    }

    /// Reports whether one bounded model repair of the request is permitted.
    #[must_use]
    pub const fn permits_model_repair(self) -> bool {
        self.permits_model_repair
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CurrentFailureDeclaration {
    taxonomy_version: u16,
    failure_class: FailureClass,
    disposition: FailureDisposition,
    may_have_attempted_effect: bool,
    permits_new_attempt: bool,
    requires_user_decision: bool,
    permits_model_repair: bool,
}

impl<'de> Deserialize<'de> for FailureDeclaration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let current = CurrentFailureDeclaration::deserialize(deserializer)?;
        Self::from_parts(
            current.taxonomy_version,
            current.failure_class,
            current.disposition,
            current.may_have_attempted_effect,
            current.permits_new_attempt,
            current.requires_user_decision,
            current.permits_model_repair,
        )
        .map_err(|error| serde::de::Error::custom(error.code()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        FAILURE_TAXONOMY_VERSION, FailureClass, FailureDeclaration, FailureDisposition,
        FailureTaxonomyError,
    };
    use crate::{CanonicalRetryDisposition, EffectClass, EffectRepetition};

    const CANONICAL: [CanonicalRetryDisposition; 4] = [
        CanonicalRetryDisposition::NotEligible,
        CanonicalRetryDisposition::EligibleFreshAttempt,
        CanonicalRetryDisposition::ReconcileFirst,
        CanonicalRetryDisposition::UserDecisionRequired,
    ];

    /// The exact default disposition and pre-dispatch status of every class.
    const DEFAULTS: [(FailureClass, FailureDisposition, bool); 14] = [
        (FailureClass::MalformedInput, FailureDisposition::FreshAttempt, false),
        (FailureClass::Preflight, FailureDisposition::FreshAttempt, false),
        (FailureClass::PolicyDenied, FailureDisposition::Terminal, false),
        (FailureClass::ApprovalRequired, FailureDisposition::UserDecision, false),
        (FailureClass::DependencyUnavailable, FailureDisposition::FreshAttempt, false),
        (FailureClass::Transient, FailureDisposition::FreshAttempt, false),
        (FailureClass::Conflict, FailureDisposition::AfterReconciliation, true),
        (FailureClass::Timeout, FailureDisposition::AfterReconciliation, true),
        (FailureClass::Cancellation, FailureDisposition::Terminal, true),
        (FailureClass::Crash, FailureDisposition::AfterReconciliation, true),
        (FailureClass::UncertainEffect, FailureDisposition::UserDecision, true),
        (FailureClass::Verification, FailureDisposition::AfterReconciliation, true),
        (FailureClass::ResourceExhausted, FailureDisposition::Terminal, true),
        (FailureClass::Internal, FailureDisposition::Terminal, true),
    ];

    const UNSAFE_EFFECTS: [EffectClass; 4] = [
        EffectClass::NonIdempotent,
        EffectClass::Destructive,
        EffectClass::External,
        EffectClass::Unknown,
    ];

    fn token<T: serde::Serialize>(value: T) -> String {
        let encoded = serde_json::to_value(value).expect("closed class must encode");
        encoded.as_str().expect("token must be a string").to_owned()
    }

    fn parse(value: serde_json::Value) -> Result<FailureDeclaration, serde_json::Error> {
        serde_json::from_value(value)
    }

    fn parse_class(candidate: &str) -> Result<FailureClass, serde_json::Error> {
        serde_json::from_value(serde_json::Value::from(candidate))
    }

    #[test]
    fn every_failure_class_has_exactly_one_default_disposition() {
        assert_eq!(FailureClass::ALL.len(), 14);
        assert_eq!(DEFAULTS.len(), FailureClass::ALL.len());

        let mut tokens = BTreeSet::new();
        for (index, class) in FailureClass::ALL.into_iter().enumerate() {
            let (declared, disposition, attempted) = DEFAULTS[index];
            assert_eq!(declared, class);
            assert_eq!(class.disposition(), disposition);
            assert_eq!(class.may_have_attempted_effect(), attempted);
            assert_eq!(class.requires_reconciliation(), attempted);
            assert!(tokens.insert(token(class)));

            let permits = disposition != FailureDisposition::Terminal;
            assert_eq!(class.permits_new_attempt(), permits);
            let decision = disposition == FailureDisposition::UserDecision;
            assert_eq!(class.requires_user_decision(), decision);
        }
        assert_eq!(tokens.len(), FailureClass::ALL.len());

        let malformed = FailureClass::MalformedInput;
        for class in FailureClass::ALL {
            assert_eq!(class.permits_model_repair(), class == malformed);
        }
    }

    #[test]
    fn a_started_effect_never_returns_a_disposition_weaker_than_reconciliation() {
        let floor = FailureDisposition::AfterReconciliation.restriction_rank();
        for class in FailureClass::ALL {
            if class.may_have_attempted_effect() {
                assert!(class.disposition().restriction_rank() >= floor);
            }
            if class.permits_model_repair() {
                assert!(!class.may_have_attempted_effect());
                assert_eq!(class.disposition(), FailureDisposition::FreshAttempt);
            }
        }
    }

    #[test]
    fn dispositions_are_exactly_ranked_and_workflow_bound() {
        assert_eq!(FailureDisposition::ALL.len(), 4);

        let mut ranks = BTreeSet::new();
        for disposition in FailureDisposition::ALL {
            assert!(ranks.insert(disposition.restriction_rank()));
            let canonical = disposition.to_canonical();
            assert_eq!(FailureDisposition::from_canonical(canonical), disposition);
            let terminal = disposition == FailureDisposition::Terminal;
            assert_eq!(disposition.permits_new_attempt(), !terminal);
        }
        assert_eq!(ranks, BTreeSet::from([0, 1, 2, 3]));

        for canonical in CANONICAL {
            let disposition = FailureDisposition::from_canonical(canonical);
            assert_eq!(disposition.to_canonical(), canonical);
        }

        for first in FailureDisposition::ALL {
            for second in FailureDisposition::ALL {
                let combined = first.most_restrictive(second);
                assert_eq!(combined, second.most_restrictive(first));
                assert!(combined.restriction_rank() >= first.restriction_rank());
                assert!(combined.restriction_rank() >= second.restriction_rank());
                assert!(combined.permits_new_attempt() <= first.permits_new_attempt());
            }
        }
    }

    #[test]
    fn effect_class_narrowing_never_relaxes_a_default_disposition() {
        for class in FailureClass::ALL {
            for effect in EffectClass::ALL {
                let effective = class.disposition_for(effect).restriction_rank();
                let floor = FailureDisposition::floor_for_effect(effect);
                assert!(effective >= class.disposition().restriction_rank());
                assert!(effective >= floor.restriction_rank());
                let declared = FailureDeclaration::new(class).disposition_for(effect);
                assert_eq!(declared.restriction_rank(), effective);

                if effect.requires_reconciliation() {
                    let reconciled = FailureDisposition::AfterReconciliation;
                    assert!(effective >= reconciled.restriction_rank());
                }
                if effect.requires_fresh_approval() {
                    let decision = FailureDisposition::UserDecision;
                    assert!(effective >= decision.restriction_rank());
                }
            }

            for effect in UNSAFE_EFFECTS {
                assert_eq!(effect.repetition(), EffectRepetition::NeverAutomatic);
                let effective = class.disposition_for(effect).restriction_rank();
                let decision = FailureDisposition::UserDecision;
                assert!(effective >= decision.restriction_rank());
            }

            let read_only = class.disposition_for(EffectClass::ReadOnly);
            assert_eq!(read_only, class.disposition());
        }
    }

    #[test]
    fn declarations_round_trip_and_reject_version_or_rule_drift() {
        for class in FailureClass::ALL {
            let declaration = FailureDeclaration::new(class);
            let encoded = serde_json::to_value(declaration).expect("must encode");
            assert_eq!(parse(encoded.clone()).ok(), Some(declaration));
            assert_eq!(declaration.taxonomy_version(), FAILURE_TAXONOMY_VERSION);
            assert_eq!(declaration.failure_class(), class);
            assert_eq!(declaration.disposition(), class.disposition());
            assert_eq!(declaration.requires_reconciliation(), class.requires_reconciliation());

            for disposition in FailureDisposition::ALL {
                let mut drifted = encoded.clone();
                drifted["disposition"] = serde_json::to_value(disposition).expect("rule");
                let admitted = parse(drifted).is_ok();
                assert_eq!(admitted, disposition == class.disposition());
            }

            for rule in [
                "may_have_attempted_effect",
                "permits_new_attempt",
                "requires_user_decision",
                "permits_model_repair",
            ] {
                let mut drifted = encoded.clone();
                let current = drifted[rule].as_bool().expect("rule must be a bool");
                drifted[rule] = serde_json::Value::Bool(!current);
                assert!(parse(drifted).is_err());
            }

            let mut stale = encoded;
            stale["taxonomy_version"] = serde_json::json!(FAILURE_TAXONOMY_VERSION + 1);
            assert!(parse(stale).is_err());
        }

        let relaxed = FailureDeclaration::from_parts(
            FAILURE_TAXONOMY_VERSION,
            FailureClass::UncertainEffect,
            FailureDisposition::FreshAttempt,
            true,
            true,
            true,
            false,
        );
        assert_eq!(relaxed, Err(FailureTaxonomyError::DispositionMismatch));

        let repaired = FailureDeclaration::from_parts(
            FAILURE_TAXONOMY_VERSION,
            FailureClass::Crash,
            FailureDisposition::AfterReconciliation,
            true,
            true,
            false,
            true,
        );
        assert_eq!(repaired, Err(FailureTaxonomyError::ModelRepairMismatch));

        let unsupported = FailureDeclaration::from_parts(
            FAILURE_TAXONOMY_VERSION + 1,
            FailureClass::Transient,
            FailureDisposition::FreshAttempt,
            false,
            true,
            false,
            false,
        );
        assert_eq!(unsupported, Err(FailureTaxonomyError::UnsupportedVersion));

        let codes = [
            FailureTaxonomyError::UnsupportedVersion.code(),
            FailureTaxonomyError::DispositionMismatch.code(),
            FailureTaxonomyError::EffectAttemptMismatch.code(),
            FailureTaxonomyError::NewAttemptMismatch.code(),
            FailureTaxonomyError::UserDecisionMismatch.code(),
            FailureTaxonomyError::ModelRepairMismatch.code(),
        ];
        let unique: BTreeSet<&str> = codes.into_iter().collect();
        assert_eq!(unique.len(), codes.len());
        for code in codes {
            assert!(code.starts_with("failure.taxonomy."));
        }
        let disposition_code = FailureTaxonomyError::DispositionMismatch.code();
        assert_eq!(disposition_code, "failure.taxonomy.disposition_mismatch");
    }

    #[test]
    fn custom_wildcard_and_model_created_failure_classes_are_not_representable() {
        let base = FailureDeclaration::new(FailureClass::UncertainEffect);
        let encoded = serde_json::to_value(base).expect("declaration must encode");
        for candidate in [
            "all",
            "any",
            "custom",
            "inherit",
            "*",
            "none",
            "retryable",
            "error",
            "failure",
            "model_declared",
            "read_only",
            "destructive",
        ] {
            assert!(parse_class(candidate).is_err());
            let mut drifted = encoded.clone();
            drifted["failure_class"] = serde_json::Value::from(candidate);
            assert!(parse(drifted).is_err());
        }

        assert!(serde_json::from_str::<FailureDeclaration>("\"transient\"").is_err());
        let mut smuggled = encoded;
        smuggled["retry_budget"] = serde_json::json!({"claimed": 99});
        assert!(parse(smuggled).is_err());
    }

    #[test]
    fn failure_and_effect_vocabularies_share_no_token() {
        let mut failures = BTreeSet::new();
        for class in FailureClass::ALL {
            failures.insert(token(class));
        }
        let mut foreign = BTreeSet::new();
        for class in EffectClass::ALL {
            foreign.insert(token(class));
        }
        for repetition in EffectRepetition::ALL {
            foreign.insert(token(repetition));
        }
        for disposition in FailureDisposition::ALL {
            foreign.insert(token(disposition));
        }
        assert_eq!(failures.len(), 14);
        assert!(failures.is_disjoint(&foreign));

        for candidate in &foreign {
            assert!(parse_class(candidate).is_err());
        }
        for candidate in &failures {
            let value = serde_json::Value::from(candidate.as_str());
            assert!(serde_json::from_value::<EffectClass>(value.clone()).is_err());
            assert!(serde_json::from_value::<FailureDisposition>(value).is_err());
        }
    }
}
