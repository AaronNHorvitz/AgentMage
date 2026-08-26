//! Closed denial of automatic new attempts and of every spent operation identity.
//!
//! These rules only deny. A candidate that no rule here denies is still not eligible:
//! current preflight, effect reconciliation, remaining budget, and fresh authority are
//! established separately, so [`AutomaticAttemptScreen::NotDeniedHere`] carries no
//! permission and no authority.
//!
//! The effect rule is read from [`EffectRepetition::NeverAutomatic`] rather than from a
//! second list of classes, so `non_idempotent`, `destructive`, `external`, and `unknown`
//! stay denied by the canonical taxonomy and no class can drift out of the rule. An
//! effect whose prior result cannot be established is denied for every class, including
//! classes that would otherwise be safe to repeat.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{
    ApprovalId, EffectClass, EffectDeclaration, EffectRepetition, GrantId, GrantNonce,
    OperationAttemptId, ReceiptId, ToolCallId,
};

/// Closed kind of one single-use identity that may never be presented twice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SpentIdentityKind {
    /// Identity of one tool call already presented for dispatch.
    ToolCall,
    /// Identity of one capability grant already consumed.
    Grant,
    /// Single-use nonce of one issued grant.
    GrantNonce,
    /// Identity of one explicit user approval decision.
    Approval,
    /// Identity of one retained operation receipt.
    Receipt,
    /// Idempotency key bound to one exact converging attempt.
    IdempotencyKey,
    /// Identity of one non-replayable operation attempt.
    OperationAttempt,
}

impl SpentIdentityKind {
    /// Every single-use identity kind in stable order.
    pub const ALL: [Self; 7] = [
        Self::ToolCall,
        Self::Grant,
        Self::GrantNonce,
        Self::Approval,
        Self::Receipt,
        Self::IdempotencyKey,
        Self::OperationAttempt,
    ];

    /// Returns the stable content-free denial code for replaying this kind.
    #[must_use]
    pub const fn replay_code(self) -> &'static str {
        match self {
            Self::ToolCall => "retry_denial.identity.tool_call_replayed",
            Self::Grant => "retry_denial.identity.grant_replayed",
            Self::GrantNonce => "retry_denial.identity.grant_nonce_replayed",
            Self::Approval => "retry_denial.identity.approval_replayed",
            Self::Receipt => "retry_denial.identity.receipt_replayed",
            Self::IdempotencyKey => "retry_denial.identity.idempotency_key_replayed",
            Self::OperationAttempt => "retry_denial.identity.operation_attempt_replayed",
        }
    }
}

/// One single-use identity together with the exact kind it belongs to.
///
/// Two identities are equal only when both the kind and the value are equal, so one
/// wire value used for a grant and for a receipt remains two separate identities. The
/// value is retained privately and never appears in a denial.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpentIdentity {
    kind: SpentIdentityKind,
    value: String,
}

impl SpentIdentity {
    fn new(kind: SpentIdentityKind, value: &str) -> Self {
        Self {
            kind,
            value: value.to_owned(),
        }
    }

    /// Binds one tool-call identity.
    #[must_use]
    pub fn tool_call(tool_call_id: &ToolCallId) -> Self {
        Self::new(SpentIdentityKind::ToolCall, tool_call_id.as_str())
    }

    /// Binds one consumed capability-grant identity.
    #[must_use]
    pub fn grant(grant_id: &GrantId) -> Self {
        Self::new(SpentIdentityKind::Grant, grant_id.as_str())
    }

    /// Binds one single-use grant nonce.
    #[must_use]
    pub fn grant_nonce(nonce: &GrantNonce) -> Self {
        Self::new(SpentIdentityKind::GrantNonce, nonce.as_str())
    }

    /// Binds one explicit approval-decision identity.
    #[must_use]
    pub fn approval(approval_id: &ApprovalId) -> Self {
        Self::new(SpentIdentityKind::Approval, approval_id.as_str())
    }

    /// Binds one retained operation-receipt identity.
    #[must_use]
    pub fn receipt(receipt_id: &ReceiptId) -> Self {
        Self::new(SpentIdentityKind::Receipt, receipt_id.as_str())
    }

    /// Binds one idempotency key.
    #[must_use]
    pub fn idempotency_key(key: &str) -> Self {
        Self::new(SpentIdentityKind::IdempotencyKey, key)
    }

    /// Binds one operation-attempt identity.
    #[must_use]
    pub fn operation_attempt(attempt_id: &OperationAttemptId) -> Self {
        Self::new(SpentIdentityKind::OperationAttempt, attempt_id.as_str())
    }

    /// Returns the exact kind of this identity.
    #[must_use]
    pub const fn kind(&self) -> SpentIdentityKind {
        self.kind
    }
}

/// Independently observed result of the immediately prior attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PriorAttemptOutcome {
    /// No prior attempt of this operation crossed its launch boundary.
    NotAttempted,
    /// Fresh observation proves the prior attempt changed no canonical state.
    FailedNoChangeVerified,
    /// Fresh observation and receipts prove the exact prior attempt completed.
    CompletedVerified,
    /// The prior attempt's effect cannot be established.
    Uncertain,
}

impl PriorAttemptOutcome {
    /// Every prior-attempt outcome in stable order.
    pub const ALL: [Self; 4] = [
        Self::NotAttempted,
        Self::FailedNoChangeVerified,
        Self::CompletedVerified,
        Self::Uncertain,
    ];
}

/// Exact reason no automatic new attempt may be considered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutomaticAttemptDenial {
    /// The declared effect admits no automatic new attempt from deterministic state.
    EffectNeverAutomatic {
        /// Exact declared class that fixed the repetition rule.
        effect_class: EffectClass,
    },
    /// The prior attempt's effect cannot be established, so nothing may repeat.
    PriorEffectUncertain,
    /// A presented identity was already spent and may never be presented again.
    IdentityReplayed {
        /// Kind of the spent identity; the identity value is never disclosed.
        kind: SpentIdentityKind,
    },
}

impl AutomaticAttemptDenial {
    /// Returns a stable content-free denial code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::EffectNeverAutomatic { .. } => "retry_denial.effect.never_automatic",
            Self::PriorEffectUncertain => "retry_denial.effect.prior_outcome_uncertain",
            Self::IdentityReplayed { kind } => kind.replay_code(),
        }
    }
}

/// Result of the closed denial rules for one candidate automatic attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutomaticAttemptScreen {
    /// One exact rule denies the candidate and no automatic new attempt is considered.
    Denied(AutomaticAttemptDenial),
    /// These rules deny nothing; eligibility is still established elsewhere.
    NotDeniedHere,
}

impl AutomaticAttemptScreen {
    /// Reports whether a denial applies.
    #[must_use]
    pub const fn is_denied(self) -> bool {
        matches!(self, Self::Denied(_))
    }

    /// Returns the exact denial, when one applies.
    #[must_use]
    pub const fn denial(self) -> Option<AutomaticAttemptDenial> {
        match self {
            Self::Denied(denial) => Some(denial),
            Self::NotDeniedHere => None,
        }
    }
}

/// Complete facts required to screen one candidate automatic new attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutomaticAttemptRequest {
    /// Declared effect of the operation whose new attempt is considered.
    pub effect: EffectDeclaration,
    /// Independently observed result of the immediately prior attempt.
    pub prior_outcome: PriorAttemptOutcome,
    /// Every single-use identity the candidate attempt would present.
    pub presented_identities: Vec<SpentIdentity>,
}

/// Append-only record of every single-use identity that has already been spent.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpentIdentityLedger {
    spent: BTreeSet<SpentIdentity>,
}

impl SpentIdentityLedger {
    /// Creates an empty ledger.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records one identity as spent, denying any second presentation of it.
    pub fn record(&mut self, identity: SpentIdentity) -> Result<(), AutomaticAttemptDenial> {
        let kind = identity.kind();
        if self.spent.insert(identity) {
            Ok(())
        } else {
            Err(AutomaticAttemptDenial::IdentityReplayed { kind })
        }
    }

    /// Records a whole set of identities, retaining nothing when any one is a replay.
    ///
    /// Duplicates within the same set are themselves a replay, so one attempt can never
    /// spend one identity twice.
    pub fn record_all(
        &mut self,
        identities: impl IntoIterator<Item = SpentIdentity>,
    ) -> Result<(), AutomaticAttemptDenial> {
        let candidates: Vec<SpentIdentity> = identities.into_iter().collect();
        if let Some(denial) = replayed_identity(self, &candidates) {
            return Err(denial);
        }
        self.spent.extend(candidates);
        Ok(())
    }

    /// Reports whether one exact identity was already spent.
    #[must_use]
    pub fn is_spent(&self, identity: &SpentIdentity) -> bool {
        self.spent.contains(identity)
    }

    /// Returns how many distinct identities are retained as spent.
    #[must_use]
    pub fn spent_count(&self) -> usize {
        self.spent.len()
    }
}

/// Reports whether one declared effect class denies every automatic new attempt.
#[must_use]
pub const fn denies_automatic_attempt(effect_class: EffectClass) -> bool {
    matches!(effect_class.repetition(), EffectRepetition::NeverAutomatic)
}

/// Applies every closed denial rule to one candidate automatic new attempt.
///
/// Rules are evaluated in a fixed order: a replayed identity first, because presenting a
/// spent identity is invalid for every effect class; then the declared effect class; then
/// an unestablished prior result.
#[must_use]
pub fn screen_automatic_attempt(
    ledger: &SpentIdentityLedger,
    request: &AutomaticAttemptRequest,
) -> AutomaticAttemptScreen {
    if let Some(denial) = replayed_identity(ledger, &request.presented_identities) {
        return AutomaticAttemptScreen::Denied(denial);
    }
    let effect_class = request.effect.effect_class();
    if denies_automatic_attempt(effect_class) {
        return AutomaticAttemptScreen::Denied(AutomaticAttemptDenial::EffectNeverAutomatic {
            effect_class,
        });
    }
    if request.prior_outcome == PriorAttemptOutcome::Uncertain {
        return AutomaticAttemptScreen::Denied(AutomaticAttemptDenial::PriorEffectUncertain);
    }
    AutomaticAttemptScreen::NotDeniedHere
}

fn replayed_identity(
    ledger: &SpentIdentityLedger,
    presented: &[SpentIdentity],
) -> Option<AutomaticAttemptDenial> {
    let mut seen: BTreeSet<&SpentIdentity> = BTreeSet::new();
    for identity in presented {
        if ledger.is_spent(identity) || !seen.insert(identity) {
            return Some(AutomaticAttemptDenial::IdentityReplayed {
                kind: identity.kind(),
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use agentmage_kernel_contracts::{
        ApprovalId, EffectClass, EffectDeclaration, GrantId, GrantNonce, OperationAttemptId,
        ReceiptId, ToolCallId,
    };

    use super::{
        AutomaticAttemptDenial, AutomaticAttemptRequest, AutomaticAttemptScreen,
        PriorAttemptOutcome, SpentIdentity, SpentIdentityKind, SpentIdentityLedger,
        denies_automatic_attempt, screen_automatic_attempt,
    };

    const NEVER_AUTOMATIC: [EffectClass; 4] = [
        EffectClass::NonIdempotent,
        EffectClass::Destructive,
        EffectClass::External,
        EffectClass::Unknown,
    ];

    fn identity(kind: SpentIdentityKind, value: &str) -> SpentIdentity {
        match kind {
            SpentIdentityKind::ToolCall => SpentIdentity::tool_call(&ToolCallId::from_raw(value)),
            SpentIdentityKind::Grant => SpentIdentity::grant(&GrantId::from_raw(value)),
            SpentIdentityKind::GrantNonce => {
                SpentIdentity::grant_nonce(&GrantNonce::from_raw(value))
            }
            SpentIdentityKind::Approval => SpentIdentity::approval(&ApprovalId::from_raw(value)),
            SpentIdentityKind::Receipt => SpentIdentity::receipt(&ReceiptId::from_raw(value)),
            SpentIdentityKind::IdempotencyKey => SpentIdentity::idempotency_key(value),
            SpentIdentityKind::OperationAttempt => {
                SpentIdentity::operation_attempt(&OperationAttemptId::from_raw(value))
            }
        }
    }

    fn request(class: EffectClass, outcome: PriorAttemptOutcome) -> AutomaticAttemptRequest {
        AutomaticAttemptRequest {
            effect: EffectDeclaration::new(class),
            prior_outcome: outcome,
            presented_identities: Vec::new(),
        }
    }

    fn effect_denial(effect_class: EffectClass) -> AutomaticAttemptDenial {
        AutomaticAttemptDenial::EffectNeverAutomatic { effect_class }
    }

    fn replay_denial(kind: SpentIdentityKind) -> AutomaticAttemptDenial {
        AutomaticAttemptDenial::IdentityReplayed { kind }
    }

    #[test]
    fn task_5_2_2_3_non_idempotent_destructive_external_and_unknown_effects_are_denied() {
        let ledger = SpentIdentityLedger::new();
        for class in EffectClass::ALL {
            let denied = NEVER_AUTOMATIC.contains(&class);
            assert_eq!(denies_automatic_attempt(class), denied, "class {class:?}");

            let candidate = request(class, PriorAttemptOutcome::FailedNoChangeVerified);
            let screen = screen_automatic_attempt(&ledger, &candidate);
            if denied {
                let expected = Some(effect_denial(class));
                assert_eq!(screen.denial(), expected, "class {class:?}");
            } else {
                let admitted = AutomaticAttemptScreen::NotDeniedHere;
                assert_eq!(screen, admitted, "class {class:?}");
            }
        }
    }

    #[test]
    fn task_5_2_2_3_uncertain_prior_effect_denies_every_effect_class() {
        let ledger = SpentIdentityLedger::new();
        for class in EffectClass::ALL {
            let candidate = request(class, PriorAttemptOutcome::Uncertain);
            let screen = screen_automatic_attempt(&ledger, &candidate);
            assert!(screen.is_denied(), "class {class:?}");
            if !denies_automatic_attempt(class) {
                let expected = Some(AutomaticAttemptDenial::PriorEffectUncertain);
                assert_eq!(screen.denial(), expected, "class {class:?}");
            }
        }

        for outcome in PriorAttemptOutcome::ALL {
            let candidate = request(EffectClass::ReadOnly, outcome);
            let screen = screen_automatic_attempt(&ledger, &candidate);
            let uncertain = outcome == PriorAttemptOutcome::Uncertain;
            assert_eq!(screen.is_denied(), uncertain, "outcome {outcome:?}");
        }
    }

    #[test]
    fn task_5_2_2_3_every_spent_identity_kind_is_denied_on_second_presentation() {
        for kind in SpentIdentityKind::ALL {
            let spent = identity(kind, "shared-value");
            let mut ledger = SpentIdentityLedger::new();
            assert_eq!(ledger.record(spent.clone()), Ok(()));
            assert!(ledger.is_spent(&spent));
            assert_eq!(ledger.record(spent.clone()), Err(replay_denial(kind)));
            assert_eq!(ledger.spent_count(), 1);

            let outcome = PriorAttemptOutcome::NotAttempted;
            let mut candidate = request(EffectClass::ReadOnly, outcome);
            candidate.presented_identities = vec![spent];
            let screen = screen_automatic_attempt(&ledger, &candidate);
            assert_eq!(screen.denial(), Some(replay_denial(kind)), "kind {kind:?}");

            candidate.presented_identities = vec![identity(kind, "fresh-value")];
            let fresh = screen_automatic_attempt(&ledger, &candidate);
            assert_eq!(fresh, AutomaticAttemptScreen::NotDeniedHere, "{kind:?}");

            for other in SpentIdentityKind::ALL {
                let same_value = identity(other, "shared-value");
                assert_eq!(ledger.is_spent(&same_value), other == kind);
            }
        }
    }

    #[test]
    fn task_5_2_2_3_one_attempt_cannot_present_or_spend_an_identity_twice() {
        let repeated = identity(SpentIdentityKind::Grant, "grant-1");
        let expected = replay_denial(SpentIdentityKind::Grant);

        let ledger = SpentIdentityLedger::new();
        let outcome = PriorAttemptOutcome::NotAttempted;
        let mut candidate = request(EffectClass::ReadOnly, outcome);
        candidate.presented_identities = vec![repeated.clone(), repeated.clone()];
        let screen = screen_automatic_attempt(&ledger, &candidate);
        assert_eq!(screen.denial(), Some(expected));

        let mut accumulating = SpentIdentityLedger::new();
        let duplicated = [repeated.clone(), repeated.clone()];
        assert_eq!(accumulating.record_all(duplicated), Err(expected));
        assert_eq!(accumulating.spent_count(), 0);

        let receipt = identity(SpentIdentityKind::Receipt, "receipt-1");
        let first = [repeated, receipt.clone()];
        assert_eq!(accumulating.record_all(first), Ok(()));
        assert_eq!(accumulating.spent_count(), 2);

        let attempt = identity(SpentIdentityKind::OperationAttempt, "attempt-1");
        let overlapping = [attempt, receipt];
        let denial = replay_denial(SpentIdentityKind::Receipt);
        assert_eq!(accumulating.record_all(overlapping), Err(denial));
        assert_eq!(accumulating.spent_count(), 2);
    }

    #[test]
    fn task_5_2_2_3_replayed_identity_is_denied_before_any_effect_rule() {
        let spent = identity(SpentIdentityKind::Approval, "approval-1");
        let mut ledger = SpentIdentityLedger::new();
        assert_eq!(ledger.record(spent.clone()), Ok(()));

        let expected = Some(replay_denial(SpentIdentityKind::Approval));
        for class in EffectClass::ALL {
            let mut candidate = request(class, PriorAttemptOutcome::Uncertain);
            candidate.presented_identities = vec![spent.clone()];
            let screen = screen_automatic_attempt(&ledger, &candidate);
            assert_eq!(screen.denial(), expected, "class {class:?}");
        }
    }

    #[test]
    fn task_5_2_2_3_denial_codes_are_unique_stable_and_disclose_no_identity_value() {
        let mut codes = Vec::new();
        for kind in SpentIdentityKind::ALL {
            let denial = replay_denial(kind);
            assert_eq!(denial.code(), kind.replay_code());
            codes.push(denial.code());
        }
        codes.push(AutomaticAttemptDenial::PriorEffectUncertain.code());
        codes.push(effect_denial(EffectClass::Destructive).code());

        let unique: BTreeSet<&str> = codes.iter().copied().collect();
        assert_eq!(unique.len(), codes.len());
        for code in &codes {
            assert!(code.starts_with("retry_denial."), "code {code}");
            assert!(!code.contains("secret-value"), "code {code}");
        }
        assert_eq!(
            AutomaticAttemptDenial::PriorEffectUncertain.code(),
            "retry_denial.effect.prior_outcome_uncertain"
        );

        let mut ledger = SpentIdentityLedger::new();
        let secret = identity(SpentIdentityKind::IdempotencyKey, "secret-value");
        assert_eq!(ledger.record(secret.clone()), Ok(()));
        let outcome = PriorAttemptOutcome::NotAttempted;
        let mut candidate = request(EffectClass::IdempotentWrite, outcome);
        candidate.presented_identities = vec![secret];
        let denial = screen_automatic_attempt(&ledger, &candidate)
            .denial()
            .expect("a replayed idempotency key must be denied");
        let key_code = "retry_denial.identity.idempotency_key_replayed";
        assert_eq!(denial.code(), key_code);

        let empty = SpentIdentityLedger::new();
        for class in NEVER_AUTOMATIC {
            let candidate = request(class, PriorAttemptOutcome::NotAttempted);
            let denial = screen_automatic_attempt(&empty, &candidate)
                .denial()
                .expect("a never-automatic class must be denied");
            assert_eq!(denial.code(), "retry_denial.effect.never_automatic");
        }
    }

    #[test]
    fn task_5_2_2_3_absence_of_a_denial_admits_nothing() {
        let screen = AutomaticAttemptScreen::NotDeniedHere;
        assert!(!screen.is_denied());
        assert_eq!(screen.denial(), None);

        let ledger = SpentIdentityLedger::new();
        let outcome = PriorAttemptOutcome::FailedNoChangeVerified;
        let candidate = request(EffectClass::IdempotentWrite, outcome);
        let result = screen_automatic_attempt(&ledger, &candidate);
        assert_eq!(result, AutomaticAttemptScreen::NotDeniedHere);
        assert_eq!(ledger.spent_count(), 0);
    }
}
