//! Separate retry, repair, and replan budgets bound to one immutable policy identity.
//!
//! Every budget is its own counter. Parser repair, step attempts, each error class,
//! workflow recovery, and replanning never share a ceiling, so charging one budget
//! can never reduce what remains in another and no scope borrows capacity from a
//! neighboring scope. Ceilings and counters move only through checked arithmetic: a
//! declared total that would overflow, a counter that already exceeds its ceiling,
//! and a ledger bound to a different policy revision each fail closed instead of
//! admitting one more attempt.
//!
//! A policy revision is immutable. Its identity is the exact policy identifier and
//! contract version fixed at construction, there is no widening accessor, and a
//! ledger answers only for the identity it recorded. No later declaration can raise
//! a ceiling that a running workflow is already spending against.

use serde::{Deserialize, Deserializer, Serialize};

use crate::{BudgetState, ErrorCategory, PolicyId};

/// Current version of the closed retry-budget policy contract.
pub const RETRY_BUDGET_POLICY_VERSION: u16 = 1;

/// Closed family of independently declared retry budgets.
///
/// Scopes are separate accounting families, not a hierarchy. No scope inherits,
/// contains, or refills another.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryBudgetScope {
    /// Deterministic normalization and profile-bound repair of one malformed call.
    ParserRepair,
    /// New attempts of one workflow step.
    StepAttempt,
    /// New attempts that follow one exact error class.
    ErrorClass,
    /// Recovery attempts across one workflow.
    Workflow,
    /// Regenerated plans for the current goal.
    Replan,
}

impl RetryBudgetScope {
    /// Every budget scope in stable contract order.
    pub const ALL: [Self; 5] = [
        Self::ParserRepair,
        Self::StepAttempt,
        Self::ErrorClass,
        Self::Workflow,
        Self::Replan,
    ];
}

/// Closed key of one exact declared budget.
///
/// The per-error-class scope is keyed by one closed [`ErrorCategory`], so every error
/// class carries its own ceiling and no class can spend another class's budget. There
/// is deliberately no custom, wildcard, aggregate, or model-created key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryBudgetKey {
    /// Bounded repair of malformed calls before any effect attempt.
    ParserRepair,
    /// Bounded new attempts of one workflow step.
    StepAttempt,
    /// Bounded new attempts that follow one exact error class.
    ErrorClass(ErrorCategory),
    /// Bounded recovery attempts across one workflow.
    Workflow,
    /// Bounded regenerated plans for the current goal.
    Replan,
}

impl RetryBudgetKey {
    /// Every declared budget in stable contract order.
    ///
    /// The per-error-class scope maps every closed error category exactly once, so a
    /// declaration can neither omit a class nor declare one twice.
    pub const ALL: [Self; 12] = [
        Self::ParserRepair,
        Self::StepAttempt,
        Self::ErrorClass(ErrorCategory::Validation),
        Self::ErrorClass(ErrorCategory::Policy),
        Self::ErrorClass(ErrorCategory::Dependency),
        Self::ErrorClass(ErrorCategory::Resource),
        Self::ErrorClass(ErrorCategory::Cancellation),
        Self::ErrorClass(ErrorCategory::Timeout),
        Self::ErrorClass(ErrorCategory::Uncertain),
        Self::ErrorClass(ErrorCategory::Internal),
        Self::Workflow,
        Self::Replan,
    ];

    /// Returns the one scope this budget belongs to.
    #[must_use]
    pub const fn scope(self) -> RetryBudgetScope {
        match self {
            Self::ParserRepair => RetryBudgetScope::ParserRepair,
            Self::StepAttempt => RetryBudgetScope::StepAttempt,
            Self::ErrorClass(_) => RetryBudgetScope::ErrorClass,
            Self::Workflow => RetryBudgetScope::Workflow,
            Self::Replan => RetryBudgetScope::Replan,
        }
    }

    /// Returns the exact error category when this budget belongs to that scope.
    #[must_use]
    pub const fn error_category(self) -> Option<ErrorCategory> {
        match self {
            Self::ErrorClass(category) => Some(category),
            _ => None,
        }
    }
}

/// One declared inclusive ceiling for one exact budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetryBudgetLimit {
    /// Exact budget bounded by this entry.
    pub key: RetryBudgetKey,
    /// Inclusive maximum attempts admitted for that budget.
    ///
    /// Zero is a valid declaration and admits no attempt at all.
    pub limit: u32,
}

/// One recorded counter for one exact budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetryBudgetConsumption {
    /// Exact budget counted by this entry.
    pub key: RetryBudgetKey,
    /// Attempts already charged to that budget.
    pub consumed: u32,
}

/// Immutable identity of one retry-budget policy revision.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetryBudgetPolicyIdentity {
    /// Exact contract version of the policy revision.
    pub policy_version: u16,
    /// Stable identity of the policy revision.
    pub policy_id: PolicyId,
}

/// Stable reason one budget record or one charge cannot be admitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetryBudgetError {
    /// The record names a contract version this build does not implement.
    UnsupportedVersion,
    /// The record does not declare every budget exactly once in contract order.
    ScopeCoverageMismatch,
    /// The declared ceilings do not sum within checked arithmetic.
    LimitOverflow,
    /// The restated declared total does not equal the checked sum of the ceilings.
    DeclaredTotalMismatch,
    /// The ledger is bound to a different immutable policy identity.
    PolicyIdentityMismatch,
    /// A recorded counter already exceeds the ceiling declared for that budget.
    ConsumptionExceedsLimit,
    /// The budget has no remaining capacity for another attempt.
    Exhausted,
    /// A counter cannot advance within checked arithmetic.
    CounterOverflow,
}

impl RetryBudgetError {
    /// Returns a stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::UnsupportedVersion => "retry.budget.version_unsupported",
            Self::ScopeCoverageMismatch => "retry.budget.scope_coverage_mismatch",
            Self::LimitOverflow => "retry.budget.limit_overflow",
            Self::DeclaredTotalMismatch => "retry.budget.declared_total_mismatch",
            Self::PolicyIdentityMismatch => "retry.budget.policy_identity_mismatch",
            Self::ConsumptionExceedsLimit => "retry.budget.consumption_exceeds_limit",
            Self::Exhausted => "retry.budget.exhausted",
            Self::CounterOverflow => "retry.budget.counter_overflow",
        }
    }
}

/// Reports whether entries declare every budget exactly once in contract order.
fn coverage_is_canonical<T: Copy>(entries: &[T], key_of: fn(T) -> RetryBudgetKey) -> bool {
    if entries.len() != RetryBudgetKey::ALL.len() {
        return false;
    }
    for (entry, key) in entries.iter().zip(RetryBudgetKey::ALL) {
        if key_of(*entry) != key {
            return false;
        }
    }
    true
}

/// Sums declared ceilings with checked arithmetic.
fn checked_total(limits: &[RetryBudgetLimit]) -> Result<u32, RetryBudgetError> {
    let mut total: u32 = 0;
    for entry in limits {
        total = match total.checked_add(entry.limit) {
            Some(next) => next,
            None => return Err(RetryBudgetError::LimitOverflow),
        };
    }
    Ok(total)
}

/// Immutable declaration of every separate retry budget for one policy revision.
///
/// The declaration carries no authority. It states only how many attempts each
/// separate budget admits, and it cannot be widened after construction: there is no
/// mutating accessor, and a record that restates a different total is rejected.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RetryBudgetPolicy {
    policy_version: u16,
    policy_id: PolicyId,
    limits: Vec<RetryBudgetLimit>,
    declared_total: u32,
}

impl RetryBudgetPolicy {
    /// Declares the current policy revision from one ceiling per budget.
    ///
    /// Coverage is exact by construction; only a total that overflows is rejected.
    pub fn declare(
        policy_id: PolicyId,
        limit_for: impl Fn(RetryBudgetKey) -> u32,
    ) -> Result<Self, RetryBudgetError> {
        let limits = RetryBudgetKey::ALL
            .into_iter()
            .map(|key| RetryBudgetLimit {
                key,
                limit: limit_for(key),
            })
            .collect::<Vec<_>>();
        let declared_total = checked_total(&limits)?;
        Ok(Self {
            policy_version: RETRY_BUDGET_POLICY_VERSION,
            policy_id,
            limits,
            declared_total,
        })
    }

    /// Validates and constructs one fully specified policy revision.
    pub fn from_parts(
        policy_version: u16,
        policy_id: PolicyId,
        limits: Vec<RetryBudgetLimit>,
        declared_total: u32,
    ) -> Result<Self, RetryBudgetError> {
        if policy_version != RETRY_BUDGET_POLICY_VERSION {
            return Err(RetryBudgetError::UnsupportedVersion);
        }
        if !coverage_is_canonical(&limits, |entry| entry.key) {
            return Err(RetryBudgetError::ScopeCoverageMismatch);
        }
        if checked_total(&limits)? != declared_total {
            return Err(RetryBudgetError::DeclaredTotalMismatch);
        }
        Ok(Self {
            policy_version,
            policy_id,
            limits,
            declared_total,
        })
    }

    /// Returns the exact contract version.
    #[must_use]
    pub const fn policy_version(&self) -> u16 {
        self.policy_version
    }

    /// Returns the stable identity of this policy revision.
    #[must_use]
    pub const fn policy_id(&self) -> &PolicyId {
        &self.policy_id
    }

    /// Returns the immutable identity a ledger binds itself to.
    #[must_use]
    pub fn identity(&self) -> RetryBudgetPolicyIdentity {
        RetryBudgetPolicyIdentity {
            policy_version: self.policy_version,
            policy_id: self.policy_id.clone(),
        }
    }

    /// Returns every declared ceiling in contract order.
    #[must_use]
    pub fn limits(&self) -> &[RetryBudgetLimit] {
        &self.limits
    }

    /// Returns the inclusive ceiling declared for one exact budget.
    #[must_use]
    pub fn limit(&self, key: RetryBudgetKey) -> u32 {
        let declared = self.limits.iter().find(|entry| entry.key == key);
        declared.map_or(0, |entry| entry.limit)
    }

    /// Returns the checked upper bound on attempts across every separate budget.
    ///
    /// This is the exact bound within which a workflow governed by this revision
    /// terminates, because no budget refills and no budget spends another.
    #[must_use]
    pub const fn declared_total(&self) -> u32 {
        self.declared_total
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CurrentRetryBudgetPolicy {
    policy_version: u16,
    policy_id: PolicyId,
    limits: Vec<RetryBudgetLimit>,
    declared_total: u32,
}

impl<'de> Deserialize<'de> for RetryBudgetPolicy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let current = CurrentRetryBudgetPolicy::deserialize(deserializer)?;
        Self::from_parts(
            current.policy_version,
            current.policy_id,
            current.limits,
            current.declared_total,
        )
        .map_err(|error| serde::de::Error::custom(error.code()))
    }
}

/// Counters charged against one immutable policy revision.
///
/// A ledger is a value: charging a budget returns a new ledger and leaves every other
/// counter exactly as it was. The recorded identity is the only policy this ledger
/// answers for, so a question asked against another revision fails closed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RetryBudgetLedger {
    policy_version: u16,
    policy_id: PolicyId,
    consumed: Vec<RetryBudgetConsumption>,
}

impl RetryBudgetLedger {
    /// Opens an unspent ledger bound to one policy revision.
    #[must_use]
    pub fn opened(policy: &RetryBudgetPolicy) -> Self {
        Self {
            policy_version: policy.policy_version,
            policy_id: policy.policy_id.clone(),
            consumed: RetryBudgetKey::ALL
                .into_iter()
                .map(|key| RetryBudgetConsumption { key, consumed: 0 })
                .collect(),
        }
    }

    /// Validates and constructs one fully specified ledger.
    pub fn from_parts(
        policy_version: u16,
        policy_id: PolicyId,
        consumed: Vec<RetryBudgetConsumption>,
    ) -> Result<Self, RetryBudgetError> {
        if policy_version != RETRY_BUDGET_POLICY_VERSION {
            return Err(RetryBudgetError::UnsupportedVersion);
        }
        if !coverage_is_canonical(&consumed, |entry| entry.key) {
            return Err(RetryBudgetError::ScopeCoverageMismatch);
        }
        Ok(Self {
            policy_version,
            policy_id,
            consumed,
        })
    }

    /// Returns the immutable policy identity this ledger is bound to.
    #[must_use]
    pub fn identity(&self) -> RetryBudgetPolicyIdentity {
        RetryBudgetPolicyIdentity {
            policy_version: self.policy_version,
            policy_id: self.policy_id.clone(),
        }
    }

    /// Reports whether this ledger is bound to the named policy revision.
    #[must_use]
    pub fn is_bound_to(&self, policy: &RetryBudgetPolicy) -> bool {
        self.policy_version == policy.policy_version && self.policy_id == policy.policy_id
    }

    /// Returns every recorded counter in contract order.
    #[must_use]
    pub fn counters(&self) -> &[RetryBudgetConsumption] {
        &self.consumed
    }

    /// Returns the attempts already charged to one exact budget.
    #[must_use]
    pub fn consumed(&self, key: RetryBudgetKey) -> u32 {
        let counter = self.consumed.iter().find(|entry| entry.key == key);
        counter.map_or(0, |entry| entry.consumed)
    }

    /// Returns the checked sum of every recorded counter.
    pub fn consumed_total(&self) -> Result<u32, RetryBudgetError> {
        let mut total: u32 = 0;
        for entry in &self.consumed {
            total = match total.checked_add(entry.consumed) {
                Some(next) => next,
                None => return Err(RetryBudgetError::CounterOverflow),
            };
        }
        Ok(total)
    }

    /// Returns the attempts one exact budget still admits.
    pub fn remaining(
        &self,
        policy: &RetryBudgetPolicy,
        key: RetryBudgetKey,
    ) -> Result<u32, RetryBudgetError> {
        if !self.is_bound_to(policy) {
            return Err(RetryBudgetError::PolicyIdentityMismatch);
        }
        match policy.limit(key).checked_sub(self.consumed(key)) {
            Some(remaining) => Ok(remaining),
            None => Err(RetryBudgetError::ConsumptionExceedsLimit),
        }
    }

    /// Returns the one closed budget disposition for one exact budget.
    ///
    /// A ledger bound to another revision reports [`BudgetState::Unknown`], because a
    /// state that cannot be established never admits another attempt.
    #[must_use]
    pub fn state(&self, policy: &RetryBudgetPolicy, key: RetryBudgetKey) -> BudgetState {
        if !self.is_bound_to(policy) {
            return BudgetState::Unknown;
        }
        let limit = policy.limit(key);
        let consumed = self.consumed(key);
        if consumed < limit {
            BudgetState::Within
        } else if consumed == limit {
            BudgetState::AtLimit
        } else {
            BudgetState::Exceeded
        }
    }

    /// Reports whether one exact budget admits another attempt.
    #[must_use]
    pub fn admits(&self, policy: &RetryBudgetPolicy, key: RetryBudgetKey) -> bool {
        matches!(self.state(policy, key), BudgetState::Within)
    }

    /// Charges one attempt to one exact budget and returns the resulting ledger.
    ///
    /// Only the named counter changes. Exhaustion, counter drift, and identity
    /// mismatch each return their own stable reason and leave this ledger untouched.
    pub fn consume(
        &self,
        policy: &RetryBudgetPolicy,
        key: RetryBudgetKey,
    ) -> Result<Self, RetryBudgetError> {
        if self.remaining(policy, key)? == 0 {
            return Err(RetryBudgetError::Exhausted);
        }
        let next = match self.consumed(key).checked_add(1) {
            Some(next) => next,
            None => return Err(RetryBudgetError::CounterOverflow),
        };
        let mut charged = self.clone();
        for entry in &mut charged.consumed {
            if entry.key == key {
                entry.consumed = next;
            }
        }
        Ok(charged)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CurrentRetryBudgetLedger {
    policy_version: u16,
    policy_id: PolicyId,
    consumed: Vec<RetryBudgetConsumption>,
}

impl<'de> Deserialize<'de> for RetryBudgetLedger {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let current = CurrentRetryBudgetLedger::deserialize(deserializer)?;
        Self::from_parts(current.policy_version, current.policy_id, current.consumed)
            .map_err(|error| serde::de::Error::custom(error.code()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        RETRY_BUDGET_POLICY_VERSION, RetryBudgetConsumption, RetryBudgetError, RetryBudgetKey,
        RetryBudgetLedger, RetryBudgetPolicy, RetryBudgetScope,
    };
    use crate::{BudgetState, ErrorCategory, PolicyId};

    fn policy_id(value: &str) -> PolicyId {
        PolicyId::from_raw(value)
    }

    fn uniform(value: &str, limit: u32) -> RetryBudgetPolicy {
        RetryBudgetPolicy::declare(policy_id(value), |_| limit).expect("policy must declare")
    }

    fn scoped_limit(key: RetryBudgetKey) -> u32 {
        match key.scope() {
            RetryBudgetScope::ParserRepair => 1,
            RetryBudgetScope::StepAttempt => 3,
            RetryBudgetScope::ErrorClass => 2,
            RetryBudgetScope::Workflow => 8,
            RetryBudgetScope::Replan => 4,
        }
    }

    fn bounded_limit(key: RetryBudgetKey) -> u32 {
        match key.scope() {
            RetryBudgetScope::ParserRepair => 1,
            RetryBudgetScope::ErrorClass => 0,
            _ => 2,
        }
    }

    fn overflowing_limit(key: RetryBudgetKey) -> u32 {
        match key {
            RetryBudgetKey::ParserRepair | RetryBudgetKey::Replan => u32::MAX,
            _ => 0,
        }
    }

    fn token<T: serde::Serialize>(value: T) -> String {
        serde_json::to_string(&value).expect("closed key must encode")
    }

    fn parse_policy(value: serde_json::Value) -> Result<RetryBudgetPolicy, serde_json::Error> {
        serde_json::from_value(value)
    }

    fn parse_ledger(value: serde_json::Value) -> Result<RetryBudgetLedger, serde_json::Error> {
        serde_json::from_value(value)
    }

    #[test]
    fn every_scope_is_separately_keyed_and_covered_exactly_once() {
        assert_eq!(RetryBudgetKey::ALL.len(), 12);
        assert_eq!(RetryBudgetScope::ALL.len(), 5);

        let mut tokens = BTreeSet::new();
        let mut keys = BTreeSet::new();
        for key in RetryBudgetKey::ALL {
            assert!(tokens.insert(token(key)));
            assert!(keys.insert(key));
            let is_error_class = key.scope() == RetryBudgetScope::ErrorClass;
            assert_eq!(key.error_category().is_some(), is_error_class);
        }

        let scopes = RetryBudgetKey::ALL.map(RetryBudgetKey::scope);
        assert_eq!(
            BTreeSet::from(scopes),
            BTreeSet::from(RetryBudgetScope::ALL)
        );

        let categories = RetryBudgetKey::ALL
            .into_iter()
            .filter_map(RetryBudgetKey::error_category)
            .collect::<BTreeSet<_>>();
        assert_eq!(categories, BTreeSet::from(ErrorCategory::ALL));
        assert_eq!(categories.len(), 8);

        let timeout = RetryBudgetKey::ErrorClass(ErrorCategory::Timeout);
        assert_eq!(token(timeout), r#"{"error_class":"timeout"}"#);
        assert_eq!(token(RetryBudgetKey::ParserRepair), r#""parser_repair""#);
    }

    #[test]
    fn declared_policies_round_trip_and_reject_version_coverage_or_total_drift() {
        let policy = RetryBudgetPolicy::declare(policy_id("policy.retry.v1"), scoped_limit)
            .expect("policy must declare");
        assert_eq!(policy.policy_version(), RETRY_BUDGET_POLICY_VERSION);
        assert_eq!(policy.policy_id().as_str(), "policy.retry.v1");
        assert_eq!(policy.limits().len(), RetryBudgetKey::ALL.len());
        assert_eq!(policy.limit(RetryBudgetKey::ParserRepair), 1);
        assert_eq!(policy.limit(RetryBudgetKey::Workflow), 8);
        assert_eq!(policy.declared_total(), 1 + 3 + 16 + 8 + 4);

        let encoded = serde_json::to_value(&policy).expect("policy must encode");
        assert_eq!(parse_policy(encoded.clone()).ok(), Some(policy.clone()));

        let mut wrong_total = encoded.clone();
        wrong_total["declared_total"] = serde_json::json!(policy.declared_total() + 1);
        assert!(parse_policy(wrong_total).is_err());

        let mut stale = encoded.clone();
        stale["policy_version"] = serde_json::json!(RETRY_BUDGET_POLICY_VERSION + 1);
        assert!(parse_policy(stale).is_err());

        let mut omitted = encoded.clone();
        omitted["limits"] = serde_json::json!(&policy.limits()[1..]);
        assert!(parse_policy(omitted).is_err());

        let mut duplicated_entries = policy.limits().to_vec();
        duplicated_entries[1] = duplicated_entries[0];
        let mut duplicated = encoded.clone();
        duplicated["limits"] = serde_json::json!(duplicated_entries);
        assert!(parse_policy(duplicated).is_err());

        let mut reversed_entries = policy.limits().to_vec();
        reversed_entries.reverse();
        let mut reordered = encoded.clone();
        reordered["limits"] = serde_json::json!(reversed_entries);
        assert!(parse_policy(reordered).is_err());

        let mut smuggled = encoded;
        smuggled["unlimited"] = serde_json::json!(true);
        assert!(parse_policy(smuggled).is_err());

        let overflow = RetryBudgetPolicy::declare(policy_id("overflow"), overflowing_limit);
        assert_eq!(overflow.err(), Some(RetryBudgetError::LimitOverflow));

        let mismatched = RetryBudgetPolicy::from_parts(
            RETRY_BUDGET_POLICY_VERSION,
            policy_id("policy.retry.v1"),
            policy.limits().to_vec(),
            0,
        );
        assert_eq!(
            mismatched.err(),
            Some(RetryBudgetError::DeclaredTotalMismatch)
        );
    }

    #[test]
    fn charging_one_budget_never_reduces_another() {
        let policy = uniform("policy.retry.separate", 2);
        let opened = RetryBudgetLedger::opened(&policy);

        for charged in RetryBudgetKey::ALL {
            let ledger = opened.consume(&policy, charged).expect("first attempt");
            assert_eq!(ledger.consumed(charged), 1);
            assert_eq!(ledger.remaining(&policy, charged).ok(), Some(1));
            assert_eq!(opened.consumed(charged), 0);

            for other in RetryBudgetKey::ALL {
                if other == charged {
                    continue;
                }
                assert_eq!(ledger.consumed(other), 0);
                assert_eq!(ledger.remaining(&policy, other).ok(), Some(2));
                assert_eq!(ledger.state(&policy, other), BudgetState::Within);
            }
        }
    }

    #[test]
    fn each_budget_terminates_within_its_declared_bound() {
        let policy = RetryBudgetPolicy::declare(policy_id("bounded"), bounded_limit)
            .expect("policy must declare");
        let mut ledger = RetryBudgetLedger::opened(&policy);
        assert_eq!(policy.declared_total(), 7);
        assert_eq!(ledger.consumed_total().ok(), Some(0));

        for key in RetryBudgetKey::ALL {
            let limit = policy.limit(key);
            let mut admitted = 0_u32;
            while ledger.admits(&policy, key) {
                ledger = ledger.consume(&policy, key).expect("attempt is admitted");
                admitted += 1;
                assert!(admitted <= limit, "budget exceeded its declared bound");
            }
            assert_eq!(admitted, limit);
            assert_eq!(ledger.state(&policy, key), BudgetState::AtLimit);
            assert_eq!(ledger.remaining(&policy, key).ok(), Some(0));
            let refused = ledger.consume(&policy, key).err();
            assert_eq!(refused, Some(RetryBudgetError::Exhausted));
        }
        assert_eq!(ledger.consumed_total().ok(), Some(policy.declared_total()));

        let denied = uniform("policy.retry.denied", 0);
        let closed = RetryBudgetLedger::opened(&denied);
        let key = RetryBudgetKey::ParserRepair;
        assert_eq!(denied.declared_total(), 0);
        assert_eq!(closed.state(&denied, key), BudgetState::AtLimit);
        assert!(!closed.admits(&denied, key));
        assert_eq!(
            closed.consume(&denied, key).err(),
            Some(RetryBudgetError::Exhausted)
        );
    }

    #[test]
    fn a_ledger_answers_only_for_its_immutable_policy_identity() {
        let policy = uniform("policy.retry.identity", 2);
        let widened = uniform("policy.retry.widened", 99);
        let ledger = RetryBudgetLedger::opened(&policy);
        let key = RetryBudgetKey::StepAttempt;

        assert!(ledger.is_bound_to(&policy));
        assert_eq!(ledger.identity(), policy.identity());
        assert!(!ledger.is_bound_to(&widened));
        assert_eq!(ledger.state(&widened, key), BudgetState::Unknown);
        assert!(!ledger.admits(&widened, key));

        let mismatch = Some(RetryBudgetError::PolicyIdentityMismatch);
        assert_eq!(ledger.remaining(&widened, key).err(), mismatch);
        assert_eq!(ledger.consume(&widened, key).err(), mismatch);

        let encoded = serde_json::to_value(&ledger).expect("ledger must encode");
        assert_eq!(parse_ledger(encoded.clone()).ok(), Some(ledger));

        let mut restamped = encoded.clone();
        restamped["policy_version"] = serde_json::json!(RETRY_BUDGET_POLICY_VERSION + 1);
        assert!(parse_ledger(restamped).is_err());

        let mut smuggled = encoded;
        smuggled["policy_override"] = serde_json::json!("policy.retry.widened");
        assert!(parse_ledger(smuggled).is_err());
    }

    #[test]
    fn drifted_counters_fail_closed_under_checked_arithmetic() {
        let policy = uniform("policy.retry.checked", 1);
        let saturated = RetryBudgetKey::ALL
            .into_iter()
            .map(|key| RetryBudgetConsumption {
                key,
                consumed: u32::MAX,
            })
            .collect::<Vec<_>>();
        let drifted = RetryBudgetLedger::from_parts(
            RETRY_BUDGET_POLICY_VERSION,
            policy_id("policy.retry.checked"),
            saturated,
        )
        .expect("coverage is canonical");
        let key = RetryBudgetKey::Workflow;

        let exceeded = Some(RetryBudgetError::ConsumptionExceedsLimit);
        assert_eq!(drifted.counters().len(), RetryBudgetKey::ALL.len());
        assert_eq!(drifted.remaining(&policy, key).err(), exceeded);
        assert_eq!(drifted.consume(&policy, key).err(), exceeded);
        assert_eq!(drifted.state(&policy, key), BudgetState::Exceeded);
        assert!(!drifted.admits(&policy, key));

        let overflow = Some(RetryBudgetError::CounterOverflow);
        assert_eq!(drifted.consumed_total().err(), overflow);

        let short = RetryBudgetLedger::from_parts(
            RETRY_BUDGET_POLICY_VERSION,
            policy_id("policy.retry.checked"),
            Vec::new(),
        );
        assert_eq!(short.err(), Some(RetryBudgetError::ScopeCoverageMismatch));

        let codes = [
            RetryBudgetError::UnsupportedVersion.code(),
            RetryBudgetError::ScopeCoverageMismatch.code(),
            RetryBudgetError::LimitOverflow.code(),
            RetryBudgetError::DeclaredTotalMismatch.code(),
            RetryBudgetError::PolicyIdentityMismatch.code(),
            RetryBudgetError::ConsumptionExceedsLimit.code(),
            RetryBudgetError::Exhausted.code(),
            RetryBudgetError::CounterOverflow.code(),
        ];
        let unique = codes.into_iter().collect::<BTreeSet<_>>();
        assert_eq!(unique.len(), codes.len());
        for code in codes {
            assert!(code.starts_with("retry.budget."));
        }
        assert_eq!(RetryBudgetError::Exhausted.code(), "retry.budget.exhausted");
    }

    #[test]
    fn custom_wildcard_and_aggregate_budget_keys_are_not_representable() {
        let policy = uniform("policy.retry.closed", 1);
        let encoded = serde_json::to_value(&policy).expect("policy must encode");
        for candidate in [
            "all",
            "any",
            "custom",
            "inherit",
            "*",
            "shared",
            "error_class",
            "retry",
            "model_declared",
            "unlimited",
        ] {
            let mut drifted = encoded.clone();
            drifted["limits"][0]["key"] = serde_json::Value::from(candidate);
            assert!(parse_policy(drifted).is_err());
        }

        let mut unknown_class = encoded.clone();
        unknown_class["limits"][2]["key"] = serde_json::json!({"error_class": "transient"});
        assert!(parse_policy(unknown_class).is_err());

        let mut refillable = encoded;
        refillable["limits"][0]["refill"] = serde_json::json!(true);
        assert!(parse_policy(refillable).is_err());
    }
}
