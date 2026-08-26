//! Canonical side-effect taxonomy used by retry and recovery policy.
//!
//! The taxonomy is independent of [`AuthorityClass`](crate::AuthorityClass) and
//! [`ToolRiskLevel`](crate::ToolRiskLevel). No effect class is derived from, implies,
//! or is implied by an authority class or a review risk level, and the three wire
//! vocabularies share no token, so no classification can substitute for another.
//! There is no custom, wildcard, inherited, or model-created variant: an effect that
//! cannot be established is exactly [`EffectClass::Unknown`] and fails closed.
//!
//! Every canonical [`GrantOperation`] maps to exactly one class through
//! [`EffectClass::for_operation`]. The map is total and kernel-owned, so a registered
//! tool operation can neither omit its class nor carry two, and no manifest, record,
//! or model proposal can add, remove, or re-point an entry.

use serde::{Deserialize, Deserializer, Serialize};

use crate::{CanonicalEffectClass, GrantOperation, StateChange};

/// Current version of the canonical effect taxonomy.
pub const EFFECT_TAXONOMY_VERSION: u16 = 1;

/// Closed repetition rule fixed by one effect class.
///
/// Variants are declared from least to most restrictive. The derived ordering is
/// meaningful: combining declarations may only move toward the end of this list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectRepetition {
    /// The identical attempt may repeat because it changes no state.
    SafeToRepeat,
    /// A new attempt is admitted only under a verified idempotency identity.
    IdentityBound,
    /// Current state must be reconciled before a new attempt is considered.
    ReconciliationRequired,
    /// No automatic new attempt is admitted from deterministic state alone.
    NeverAutomatic,
}

impl EffectRepetition {
    /// Every repetition rule from least to most restrictive.
    pub const ALL: [Self; 4] = [
        Self::SafeToRepeat,
        Self::IdentityBound,
        Self::ReconciliationRequired,
        Self::NeverAutomatic,
    ];
}

/// Closed side-effect class of one operation attempt.
///
/// The class describes only what an attempt does to state. It never describes who
/// may authorize the attempt and never describes how a reviewer rated the tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectClass {
    /// The attempt observes state without changing it.
    ReadOnly,
    /// The attempt converges one exact desired state under a verified identity.
    IdempotentWrite,
    /// The attempt changes state only while verified preconditions still hold.
    Conditional,
    /// The attempt changes state again on every repetition.
    NonIdempotent,
    /// The attempt removes or overwrites state that repetition cannot restore.
    Destructive,
    /// The attempt changes state owned outside this machine.
    External,
    /// The effect cannot be established and must fail closed.
    Unknown,
}

impl EffectClass {
    /// Every effect class in stable taxonomy order.
    pub const ALL: [Self; 7] = [
        Self::ReadOnly,
        Self::IdempotentWrite,
        Self::Conditional,
        Self::NonIdempotent,
        Self::Destructive,
        Self::External,
        Self::Unknown,
    ];

    /// Returns the one effect class fixed for one canonical operation.
    ///
    /// The map is total and closed: every [`GrantOperation`] appears in exactly one
    /// arm and there is no default, wildcard, or inherited arm, so a new operation
    /// cannot compile without an explicit class and no operation can carry two.
    /// No canonical operation maps to [`Self::Unknown`]; that class stays reserved
    /// for an effect this taxonomy cannot establish.
    ///
    /// [`GrantOperation::ModelInference`] is read-only because a local inference
    /// changes no persisted state; repeated inference is bounded by declared budgets
    /// rather than by this taxonomy.
    #[must_use]
    pub const fn for_operation(operation: GrantOperation) -> Self {
        match operation {
            GrantOperation::WorkspaceRead
            | GrantOperation::DatabaseRead
            | GrantOperation::ModelInference => Self::ReadOnly,
            GrantOperation::GitFetch | GrantOperation::DraftCreate => Self::IdempotentWrite,
            GrantOperation::WorkspaceWrite
            | GrantOperation::GitClone
            | GrantOperation::GitWorktreeCreate
            | GrantOperation::GitBranchFastForward => Self::Conditional,
            GrantOperation::CommandExecute | GrantOperation::GitCommit => Self::NonIdempotent,
            GrantOperation::WorkspaceDelete
            | GrantOperation::GitWorktreeRemove
            | GrantOperation::Administration => Self::Destructive,
            GrantOperation::NetworkAccess
            | GrantOperation::GitPush
            | GrantOperation::Publish
            | GrantOperation::Send
            | GrantOperation::Upload
            | GrantOperation::Deploy
            | GrantOperation::DatabaseWrite
            | GrantOperation::CredentialAccess => Self::External,
        }
    }

    /// Reports whether one observed state-change disposition is admissible here.
    ///
    /// A read-only attempt can neither report a change nor report an uncertain
    /// change, so an untrusted record claiming either for a read-only class fails
    /// closed. Any class may report that state did not change, because an attempt
    /// may be denied before it runs.
    #[must_use]
    pub const fn admits_observed_change(self, observed: StateChange) -> bool {
        match observed {
            StateChange::NotChanged => true,
            StateChange::Changed | StateChange::Uncertain => self.changes_state(),
        }
    }

    /// Returns the one repetition rule fixed for this class.
    #[must_use]
    pub const fn repetition(self) -> EffectRepetition {
        match self {
            Self::ReadOnly => EffectRepetition::SafeToRepeat,
            Self::IdempotentWrite => EffectRepetition::IdentityBound,
            Self::Conditional => EffectRepetition::ReconciliationRequired,
            Self::NonIdempotent | Self::Destructive | Self::External | Self::Unknown => {
                EffectRepetition::NeverAutomatic
            }
        }
    }

    /// Reports whether an attempt of this class may change state.
    ///
    /// [`Self::Unknown`] reports `true` because an unestablished effect fails closed.
    #[must_use]
    pub const fn changes_state(self) -> bool {
        !matches!(self, Self::ReadOnly)
    }

    /// Reports whether current state must be reconciled before recovery is considered.
    #[must_use]
    pub const fn requires_reconciliation(self) -> bool {
        !matches!(self, Self::ReadOnly | Self::IdempotentWrite)
    }

    /// Reports whether every new attempt requires its own fresh narrow approval.
    #[must_use]
    pub const fn requires_fresh_approval(self) -> bool {
        matches!(self.repetition(), EffectRepetition::NeverAutomatic)
    }

    /// Returns the restriction rank of this class.
    ///
    /// A higher rank is never less restrictive in any derived rule. The ranks form a
    /// closed total order so that combining declarations stays deterministic.
    /// [`Self::Unknown`] holds the highest rank because an unestablished effect is
    /// never replaced by a concrete class.
    #[must_use]
    pub const fn restriction_rank(self) -> u8 {
        match self {
            Self::ReadOnly => 0,
            Self::IdempotentWrite => 1,
            Self::Conditional => 2,
            Self::NonIdempotent => 3,
            Self::External => 4,
            Self::Destructive => 5,
            Self::Unknown => 6,
        }
    }

    /// Returns the more restrictive of two declared classes.
    ///
    /// Combining declarations can only raise the restriction rank, so no additional
    /// declaration source can broaden an effect that is already declared. Because
    /// [`Self::Unknown`] outranks every concrete class, it is absorbing: no proposal
    /// can establish a concrete effect for an unestablished declaration.
    #[must_use]
    pub const fn most_restrictive(self, other: Self) -> Self {
        if other.restriction_rank() > self.restriction_rank() {
            other
        } else {
            self
        }
    }

    /// Binds one descriptive workflow-record class to its exact canonical class.
    #[must_use]
    pub const fn from_canonical(class: CanonicalEffectClass) -> Self {
        match class {
            CanonicalEffectClass::ReadOnly => Self::ReadOnly,
            CanonicalEffectClass::IdempotentWrite => Self::IdempotentWrite,
            CanonicalEffectClass::Conditional => Self::Conditional,
            CanonicalEffectClass::NonIdempotent => Self::NonIdempotent,
            CanonicalEffectClass::Destructive => Self::Destructive,
            CanonicalEffectClass::External => Self::External,
            CanonicalEffectClass::Unknown => Self::Unknown,
        }
    }

    /// Returns the exact descriptive workflow-record class for this class.
    #[must_use]
    pub const fn to_canonical(self) -> CanonicalEffectClass {
        match self {
            Self::ReadOnly => CanonicalEffectClass::ReadOnly,
            Self::IdempotentWrite => CanonicalEffectClass::IdempotentWrite,
            Self::Conditional => CanonicalEffectClass::Conditional,
            Self::NonIdempotent => CanonicalEffectClass::NonIdempotent,
            Self::Destructive => CanonicalEffectClass::Destructive,
            Self::External => CanonicalEffectClass::External,
            Self::Unknown => CanonicalEffectClass::Unknown,
        }
    }
}

/// Stable reason one effect declaration cannot be admitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectTaxonomyError {
    /// The record names a taxonomy version this build does not implement.
    UnsupportedVersion,
    /// The repetition rule does not equal the canonical rule for the class.
    RepetitionMismatch,
    /// The state-change rule does not equal the canonical rule for the class.
    StateChangeMismatch,
    /// The reconciliation rule does not equal the canonical rule for the class.
    ReconciliationMismatch,
    /// The approval rule does not equal the canonical rule for the class.
    ApprovalMismatch,
    /// The class does not equal the canonical class mapped to the named operation.
    OperationEffectMismatch,
}

impl EffectTaxonomyError {
    /// Returns a stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::UnsupportedVersion => "effect.taxonomy.version_unsupported",
            Self::RepetitionMismatch => "effect.taxonomy.repetition_mismatch",
            Self::StateChangeMismatch => "effect.taxonomy.state_change_mismatch",
            Self::ReconciliationMismatch => "effect.taxonomy.reconciliation_mismatch",
            Self::ApprovalMismatch => "effect.taxonomy.approval_mismatch",
            Self::OperationEffectMismatch => "effect.taxonomy.operation_effect_mismatch",
        }
    }
}

/// Versioned effect class together with the exact rules it fixes.
///
/// Every rule is derived from the class. Construction and deserialization reject any
/// caller-selected rule that differs from the canonical derivation, so an untrusted
/// record may restate one declaration but can never relax one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EffectDeclaration {
    taxonomy_version: u16,
    effect_class: EffectClass,
    repetition: EffectRepetition,
    changes_state: bool,
    requires_reconciliation: bool,
    requires_fresh_approval: bool,
}

impl EffectDeclaration {
    /// Creates the current canonical declaration for one effect class.
    #[must_use]
    pub const fn new(effect_class: EffectClass) -> Self {
        Self {
            taxonomy_version: EFFECT_TAXONOMY_VERSION,
            effect_class,
            repetition: effect_class.repetition(),
            changes_state: effect_class.changes_state(),
            requires_reconciliation: effect_class.requires_reconciliation(),
            requires_fresh_approval: effect_class.requires_fresh_approval(),
        }
    }

    /// Creates the current canonical declaration for one canonical operation.
    #[must_use]
    pub const fn for_operation(operation: GrantOperation) -> Self {
        Self::new(EffectClass::for_operation(operation))
    }

    /// Validates and constructs one fully specified declaration.
    pub fn from_parts(
        taxonomy_version: u16,
        effect_class: EffectClass,
        repetition: EffectRepetition,
        changes_state: bool,
        requires_reconciliation: bool,
        requires_fresh_approval: bool,
    ) -> Result<Self, EffectTaxonomyError> {
        if taxonomy_version != EFFECT_TAXONOMY_VERSION {
            return Err(EffectTaxonomyError::UnsupportedVersion);
        }
        if repetition != effect_class.repetition() {
            return Err(EffectTaxonomyError::RepetitionMismatch);
        }
        if changes_state != effect_class.changes_state() {
            return Err(EffectTaxonomyError::StateChangeMismatch);
        }
        if requires_reconciliation != effect_class.requires_reconciliation() {
            return Err(EffectTaxonomyError::ReconciliationMismatch);
        }
        if requires_fresh_approval != effect_class.requires_fresh_approval() {
            return Err(EffectTaxonomyError::ApprovalMismatch);
        }
        Ok(Self::new(effect_class))
    }

    /// Returns this declaration narrowed by one untrusted proposed class.
    ///
    /// A proposal can only raise the restriction rank; it can never broaden the
    /// declaration it is combined with.
    #[must_use]
    pub const fn narrowed_by(self, proposed: EffectClass) -> Self {
        Self::new(self.effect_class.most_restrictive(proposed))
    }

    /// Returns the exact taxonomy version.
    #[must_use]
    pub const fn taxonomy_version(self) -> u16 {
        self.taxonomy_version
    }

    /// Returns the exact declared effect class.
    #[must_use]
    pub const fn effect_class(self) -> EffectClass {
        self.effect_class
    }

    /// Returns the repetition rule fixed by the declared class.
    #[must_use]
    pub const fn repetition(self) -> EffectRepetition {
        self.repetition
    }

    /// Reports whether an attempt of the declared class may change state.
    #[must_use]
    pub const fn changes_state(self) -> bool {
        self.changes_state
    }

    /// Reports whether reconciliation is required before recovery is considered.
    #[must_use]
    pub const fn requires_reconciliation(self) -> bool {
        self.requires_reconciliation
    }

    /// Reports whether every new attempt requires its own fresh narrow approval.
    #[must_use]
    pub const fn requires_fresh_approval(self) -> bool {
        self.requires_fresh_approval
    }
}

/// Versioned binding of one canonical operation to its one canonical effect class.
///
/// The class is derived from the operation and is never supplied by the record's
/// author. Deserialization rejects an omitted class, a class outside the closed
/// taxonomy, and any class that differs from the canonical mapping, so an untrusted
/// manifest or model proposal may restate one binding but can never introduce a
/// custom, wildcard, inherited, or invented one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperationEffectBinding {
    taxonomy_version: u16,
    operation: GrantOperation,
    effect_class: EffectClass,
}

impl OperationEffectBinding {
    /// Creates the current canonical binding for one canonical operation.
    #[must_use]
    pub const fn new(operation: GrantOperation) -> Self {
        Self {
            taxonomy_version: EFFECT_TAXONOMY_VERSION,
            operation,
            effect_class: EffectClass::for_operation(operation),
        }
    }

    /// Validates and constructs one fully specified binding.
    pub fn from_parts(
        taxonomy_version: u16,
        operation: GrantOperation,
        effect_class: EffectClass,
    ) -> Result<Self, EffectTaxonomyError> {
        if taxonomy_version != EFFECT_TAXONOMY_VERSION {
            return Err(EffectTaxonomyError::UnsupportedVersion);
        }
        if effect_class != EffectClass::for_operation(operation) {
            return Err(EffectTaxonomyError::OperationEffectMismatch);
        }
        Ok(Self::new(operation))
    }

    /// Returns the exact taxonomy version.
    #[must_use]
    pub const fn taxonomy_version(self) -> u16 {
        self.taxonomy_version
    }

    /// Returns the exact bound canonical operation.
    #[must_use]
    pub const fn operation(self) -> GrantOperation {
        self.operation
    }

    /// Returns the one effect class mapped to the bound operation.
    #[must_use]
    pub const fn effect_class(self) -> EffectClass {
        self.effect_class
    }

    /// Returns the full declaration, including every rule fixed by the class.
    #[must_use]
    pub const fn declaration(self) -> EffectDeclaration {
        EffectDeclaration::new(self.effect_class)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CurrentOperationEffectBinding {
    taxonomy_version: u16,
    operation: GrantOperation,
    effect_class: EffectClass,
}

impl<'de> Deserialize<'de> for OperationEffectBinding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let current = CurrentOperationEffectBinding::deserialize(deserializer)?;
        Self::from_parts(
            current.taxonomy_version,
            current.operation,
            current.effect_class,
        )
        .map_err(|error| serde::de::Error::custom(error.code()))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CurrentEffectDeclaration {
    taxonomy_version: u16,
    effect_class: EffectClass,
    repetition: EffectRepetition,
    changes_state: bool,
    requires_reconciliation: bool,
    requires_fresh_approval: bool,
}

impl<'de> Deserialize<'de> for EffectDeclaration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let current = CurrentEffectDeclaration::deserialize(deserializer)?;
        Self::from_parts(
            current.taxonomy_version,
            current.effect_class,
            current.repetition,
            current.changes_state,
            current.requires_reconciliation,
            current.requires_fresh_approval,
        )
        .map_err(|error| serde::de::Error::custom(error.code()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::{
        EFFECT_TAXONOMY_VERSION, EffectClass, EffectDeclaration, EffectRepetition,
        EffectTaxonomyError, OperationEffectBinding,
    };
    use crate::{AuthorityClass, CanonicalEffectClass, GrantOperation, StateChange, ToolRiskLevel};

    const CANONICAL: [CanonicalEffectClass; 7] = [
        CanonicalEffectClass::ReadOnly,
        CanonicalEffectClass::IdempotentWrite,
        CanonicalEffectClass::Conditional,
        CanonicalEffectClass::NonIdempotent,
        CanonicalEffectClass::Destructive,
        CanonicalEffectClass::External,
        CanonicalEffectClass::Unknown,
    ];

    const RISKS: [ToolRiskLevel; 4] = [
        ToolRiskLevel::Low,
        ToolRiskLevel::Moderate,
        ToolRiskLevel::High,
        ToolRiskLevel::Critical,
    ];

    fn token<T: serde::Serialize>(value: T) -> String {
        let encoded = serde_json::to_value(value).expect("closed class must encode");
        encoded.as_str().expect("token must be a string").to_owned()
    }

    fn parse(value: serde_json::Value) -> Result<EffectDeclaration, serde_json::Error> {
        serde_json::from_value(value)
    }

    fn parse_class(candidate: &str) -> Result<EffectClass, serde_json::Error> {
        serde_json::from_value(serde_json::Value::from(candidate))
    }

    fn bind(value: serde_json::Value) -> Result<OperationEffectBinding, serde_json::Error> {
        serde_json::from_value(value)
    }

    fn checked(
        operation: GrantOperation,
        class: EffectClass,
    ) -> Result<OperationEffectBinding, EffectTaxonomyError> {
        OperationEffectBinding::from_parts(EFFECT_TAXONOMY_VERSION, operation, class)
    }

    /// Asserts one combined class is at least as restrictive as one source class;
    /// each rule holds when the combined value is greater than or equal.
    fn assert_not_relaxed(combined: EffectClass, source: EffectClass) {
        assert!(combined.restriction_rank() >= source.restriction_rank());
        assert!(combined.repetition() >= source.repetition());
        assert!(combined.changes_state() >= source.changes_state());
        assert!(combined.requires_reconciliation() >= source.requires_reconciliation());
        assert!(combined.requires_fresh_approval() >= source.requires_fresh_approval());
    }

    #[test]
    fn every_effect_class_is_distinct_exactly_ranked_and_workflow_bound() {
        assert_eq!(EffectClass::ALL.len(), 7);

        let mut ranks = BTreeSet::new();
        let mut tokens = BTreeSet::new();
        for class in EffectClass::ALL {
            assert!(ranks.insert(class.restriction_rank()));
            assert!(tokens.insert(token(class)));
            let canonical = class.to_canonical();
            assert_eq!(EffectClass::from_canonical(canonical), class);
            assert_eq!(token(canonical), token(class));
        }
        assert_eq!(ranks, BTreeSet::from([0, 1, 2, 3, 4, 5, 6]));
        assert_eq!(tokens.len(), EffectClass::ALL.len());

        for canonical in CANONICAL {
            let class = EffectClass::from_canonical(canonical);
            assert_eq!(class.to_canonical(), canonical);
        }

        let read_only = EffectClass::ReadOnly;
        assert_eq!(read_only.repetition(), EffectRepetition::SafeToRepeat);
        assert!(!read_only.changes_state());
        assert!(!read_only.requires_reconciliation());
        assert!(!read_only.requires_fresh_approval());

        let write = EffectClass::IdempotentWrite;
        assert_eq!(write.repetition(), EffectRepetition::IdentityBound);
        assert!(write.changes_state());
        assert!(!write.requires_reconciliation());
        assert!(!write.requires_fresh_approval());

        let conditional = EffectClass::Conditional;
        let reconciled = EffectRepetition::ReconciliationRequired;
        assert_eq!(conditional.repetition(), reconciled);
        assert!(conditional.requires_reconciliation());
        assert!(!conditional.requires_fresh_approval());

        for class in [
            EffectClass::NonIdempotent,
            EffectClass::Destructive,
            EffectClass::External,
            EffectClass::Unknown,
        ] {
            assert_eq!(class.repetition(), EffectRepetition::NeverAutomatic);
            assert!(class.changes_state());
            assert!(class.requires_reconciliation());
            assert!(class.requires_fresh_approval());
        }
    }

    #[test]
    fn combining_declarations_never_relaxes_any_derived_rule() {
        for first in EffectClass::ALL {
            for second in EffectClass::ALL {
                let combined = first.most_restrictive(second);
                assert_eq!(combined, second.most_restrictive(first));
                assert_not_relaxed(combined, first);
                assert_not_relaxed(combined, second);

                let narrowed = EffectDeclaration::new(first).narrowed_by(second);
                assert_eq!(narrowed, EffectDeclaration::new(combined));
                assert_eq!(narrowed.effect_class(), combined);
            }
        }
    }

    #[test]
    fn unknown_absorbs_every_proposed_class_in_both_operand_orders() {
        for class in EffectClass::ALL {
            let unknown = EffectClass::Unknown;
            assert_eq!(unknown.most_restrictive(class), unknown);
            assert_eq!(class.most_restrictive(unknown), unknown);

            let narrowed = EffectDeclaration::new(unknown).narrowed_by(class);
            assert_eq!(narrowed.effect_class(), unknown);
        }

        let destructive = EffectDeclaration::new(EffectClass::Unknown)
            .narrowed_by(EffectClass::Destructive)
            .effect_class();
        assert_eq!(destructive, EffectClass::Unknown);
    }

    #[test]
    fn declarations_round_trip_and_reject_version_or_rule_drift() {
        for class in EffectClass::ALL {
            let declaration = EffectDeclaration::new(class);
            let encoded = serde_json::to_value(declaration).expect("must encode");
            assert_eq!(parse(encoded.clone()).ok(), Some(declaration));
            assert_eq!(declaration.taxonomy_version(), EFFECT_TAXONOMY_VERSION);

            for repetition in EffectRepetition::ALL {
                let mut drifted = encoded.clone();
                drifted["repetition"] = serde_json::to_value(repetition).expect("rule");
                let admitted = parse(drifted).is_ok();
                assert_eq!(admitted, repetition == class.repetition());
            }

            for rule in [
                "changes_state",
                "requires_reconciliation",
                "requires_fresh_approval",
            ] {
                let mut drifted = encoded.clone();
                let current = drifted[rule].as_bool().expect("rule must be a bool");
                drifted[rule] = serde_json::Value::Bool(!current);
                assert!(parse(drifted).is_err());
            }

            let mut stale = encoded;
            stale["taxonomy_version"] = serde_json::json!(EFFECT_TAXONOMY_VERSION + 1);
            assert!(parse(stale).is_err());
        }

        let relaxed = EffectDeclaration::from_parts(
            EFFECT_TAXONOMY_VERSION,
            EffectClass::Destructive,
            EffectRepetition::SafeToRepeat,
            true,
            true,
            true,
        );
        assert_eq!(relaxed, Err(EffectTaxonomyError::RepetitionMismatch));

        let unsupported = EffectDeclaration::from_parts(
            EFFECT_TAXONOMY_VERSION + 1,
            EffectClass::ReadOnly,
            EffectRepetition::SafeToRepeat,
            false,
            false,
            false,
        );
        assert_eq!(unsupported, Err(EffectTaxonomyError::UnsupportedVersion));

        let codes = [
            EffectTaxonomyError::UnsupportedVersion.code(),
            EffectTaxonomyError::RepetitionMismatch.code(),
            EffectTaxonomyError::StateChangeMismatch.code(),
            EffectTaxonomyError::ReconciliationMismatch.code(),
            EffectTaxonomyError::ApprovalMismatch.code(),
            EffectTaxonomyError::OperationEffectMismatch.code(),
        ];
        let unique: BTreeSet<&str> = codes.into_iter().collect();
        assert_eq!(unique.len(), codes.len());
        for code in codes {
            assert!(code.starts_with("effect.taxonomy."));
        }
        let repetition_code = EffectTaxonomyError::RepetitionMismatch.code();
        assert_eq!(repetition_code, "effect.taxonomy.repetition_mismatch");
    }

    #[test]
    fn effect_classes_are_independent_of_authority_and_risk_vocabularies() {
        let mut effects = BTreeSet::new();
        for class in EffectClass::ALL {
            effects.insert(token(class));
        }
        let mut foreign = BTreeSet::new();
        for class in AuthorityClass::ALL {
            foreign.insert(token(class));
        }
        for level in RISKS {
            foreign.insert(token(level));
        }
        assert_eq!(effects.len(), 7);
        assert_eq!(foreign.len(), 12);
        assert!(effects.is_disjoint(&foreign));

        for candidate in &foreign {
            assert!(parse_class(candidate).is_err());
        }
        for candidate in &effects {
            let value = serde_json::Value::from(candidate.as_str());
            let authority = serde_json::from_value::<AuthorityClass>(value.clone());
            let risk = serde_json::from_value::<ToolRiskLevel>(value);
            assert!(authority.is_err());
            assert!(risk.is_err());
        }
    }

    #[test]
    fn custom_wildcard_and_inherited_effect_classes_are_not_representable() {
        let base = EffectDeclaration::new(EffectClass::Unknown);
        let encoded = serde_json::to_value(base).expect("declaration must encode");
        for candidate in [
            "all",
            "any",
            "custom",
            "inherit",
            "*",
            "none",
            "safe",
            "write",
            "read",
            "side_effect",
            "model_declared",
        ] {
            assert!(parse_class(candidate).is_err());
            let mut drifted = encoded.clone();
            drifted["effect_class"] = serde_json::Value::from(candidate);
            assert!(parse(drifted).is_err());
        }

        assert!(serde_json::from_str::<EffectDeclaration>("\"unknown\"").is_err());
        let mut smuggled = encoded;
        smuggled["capability_grant"] = serde_json::json!({"claimed": true});
        assert!(parse(smuggled).is_err());
    }

    #[test]
    fn every_canonical_operation_maps_to_exactly_one_effect_class() {
        let mut mapped = BTreeMap::new();
        for operation in GrantOperation::ALL {
            let binding = OperationEffectBinding::new(operation);
            let class = binding.effect_class();
            assert!(mapped.insert(operation, class).is_none());
            assert_eq!(binding.operation(), operation);
            assert_eq!(binding.taxonomy_version(), EFFECT_TAXONOMY_VERSION);
            assert_eq!(class, EffectClass::for_operation(operation));
            assert_eq!(binding.declaration(), EffectDeclaration::new(class));
            assert_eq!(binding.declaration().repetition(), class.repetition());
            assert_ne!(class, EffectClass::Unknown, "no operation is unknown");
        }
        assert_eq!(mapped.len(), GrantOperation::ALL.len());

        let mut covered = BTreeSet::new();
        for class in mapped.values() {
            covered.insert(token(*class));
        }
        let mut concrete = BTreeSet::new();
        for class in EffectClass::ALL {
            if class != EffectClass::Unknown {
                concrete.insert(token(class));
            }
        }
        assert_eq!(covered, concrete);

        let read = EffectClass::for_operation(GrantOperation::WorkspaceRead);
        let delete = EffectClass::for_operation(GrantOperation::WorkspaceDelete);
        let commit = EffectClass::for_operation(GrantOperation::GitCommit);
        let push = EffectClass::for_operation(GrantOperation::GitPush);
        let fetch = EffectClass::for_operation(GrantOperation::GitFetch);
        let advance = EffectClass::for_operation(GrantOperation::GitBranchFastForward);
        assert_eq!(read, EffectClass::ReadOnly);
        assert_eq!(delete, EffectClass::Destructive);
        assert_eq!(commit, EffectClass::NonIdempotent);
        assert_eq!(push, EffectClass::External);
        assert_eq!(fetch, EffectClass::IdempotentWrite);
        assert_eq!(advance, EffectClass::Conditional);
    }

    #[test]
    fn operation_bindings_reject_custom_wildcard_inherited_or_omitted_classes() {
        for operation in GrantOperation::ALL {
            let binding = OperationEffectBinding::new(operation);
            let canonical = binding.effect_class();
            let encoded = serde_json::to_value(binding).expect("must encode");
            assert_eq!(bind(encoded.clone()).ok(), Some(binding));

            for class in EffectClass::ALL {
                let admitted = checked(operation, class);
                assert_eq!(admitted.is_ok(), class == canonical);
                if class == canonical {
                    continue;
                }
                let mismatch = EffectTaxonomyError::OperationEffectMismatch;
                assert_eq!(admitted, Err(mismatch));
                let mut drifted = encoded.clone();
                drifted["effect_class"] = serde_json::to_value(class).expect("class");
                assert!(bind(drifted).is_err());
            }

            for candidate in [
                "all",
                "any",
                "custom",
                "inherit",
                "*",
                "none",
                "safe",
                "write",
                "read",
                "side_effect",
                "model_declared",
                "",
            ] {
                let mut drifted = encoded.clone();
                drifted["effect_class"] = serde_json::Value::from(candidate);
                assert!(bind(drifted).is_err());
            }

            let mut omitted = encoded.clone();
            let object = omitted.as_object_mut().expect("binding is an object");
            assert!(object.remove("effect_class").is_some());
            assert!(bind(omitted).is_err());

            let mut stale = encoded.clone();
            stale["taxonomy_version"] = serde_json::json!(EFFECT_TAXONOMY_VERSION + 1);
            assert!(bind(stale).is_err());

            let legacy = OperationEffectBinding::from_parts(0, operation, canonical);
            assert_eq!(legacy, Err(EffectTaxonomyError::UnsupportedVersion));

            let mut smuggled = encoded;
            smuggled["capability_grant"] = serde_json::json!({"claimed": true});
            assert!(bind(smuggled).is_err());
        }

        let bare = serde_json::from_str::<OperationEffectBinding>("\"git_push\"");
        assert!(bare.is_err());
        let unmapped = serde_json::json!({
            "taxonomy_version": EFFECT_TAXONOMY_VERSION,
            "operation": "git_force_push",
            "effect_class": "read_only"
        });
        assert!(bind(unmapped).is_err());
    }

    #[test]
    fn observed_state_change_claims_cannot_exceed_the_mapped_effect_class() {
        for operation in GrantOperation::ALL {
            let class = EffectClass::for_operation(operation);
            assert!(class.admits_observed_change(StateChange::NotChanged));
            assert_eq!(
                class.admits_observed_change(StateChange::Changed),
                class.changes_state()
            );
            assert_eq!(
                class.admits_observed_change(StateChange::Uncertain),
                class.changes_state()
            );
        }

        let read_only = EffectClass::ReadOnly;
        assert!(!read_only.admits_observed_change(StateChange::Changed));
        assert!(!read_only.admits_observed_change(StateChange::Uncertain));
        for class in EffectClass::ALL {
            assert!(class.admits_observed_change(StateChange::NotChanged));
            assert_eq!(
                class.admits_observed_change(StateChange::Changed),
                class != EffectClass::ReadOnly
            );
        }
    }
}
