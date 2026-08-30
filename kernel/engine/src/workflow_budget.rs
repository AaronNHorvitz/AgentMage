//! Separate, immutable budgets for parser repair and workflow supervision.

use std::fmt::Write as _;

use agentmage_kernel_contracts::CanonicalWorkflowFailureClass;
use sha2::{Digest, Sha256};

const FAILURE_CLASS_COUNT: usize = 14;

/// One exact per-error-class limit supplied when constructing a policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ErrorClassBudgetLimit {
    /// Closed workflow failure class.
    pub failure_class: CanonicalWorkflowFailureClass,
    /// Inclusive number of failures admitted for this class.
    pub limit: u64,
}

/// Immutable limits and content-derived identity for one workflow-supervision policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkflowBudgetPolicy {
    policy_id: String,
    policy_sha256: String,
    parser_repairs: u64,
    step_attempts: u64,
    per_error_class: [u64; FAILURE_CLASS_COUNT],
    workflow_work: u64,
    replans: u64,
}

impl WorkflowBudgetPolicy {
    /// Constructs a policy only when every closed failure class is named exactly once.
    pub fn new(
        policy_id: String,
        parser_repairs: u64,
        step_attempts: u64,
        error_limits: &[ErrorClassBudgetLimit],
        workflow_work: u64,
        replans: u64,
    ) -> Result<Self, WorkflowBudgetError> {
        if !valid_identifier(&policy_id) || error_limits.len() != FAILURE_CLASS_COUNT {
            return Err(WorkflowBudgetError::InvalidPolicy);
        }
        let mut per_error_class = [0_u64; FAILURE_CLASS_COUNT];
        let mut seen = [false; FAILURE_CLASS_COUNT];
        for limit in error_limits {
            let index = failure_class_index(limit.failure_class);
            if seen[index] {
                return Err(WorkflowBudgetError::InvalidPolicy);
            }
            seen[index] = true;
            per_error_class[index] = limit.limit;
        }
        if seen.iter().any(|present| !present) {
            return Err(WorkflowBudgetError::InvalidPolicy);
        }
        let mut policy = Self {
            policy_id,
            policy_sha256: String::new(),
            parser_repairs,
            step_attempts,
            per_error_class,
            workflow_work,
            replans,
        };
        policy.policy_sha256 = policy_digest(&policy);
        Ok(policy)
    }

    /// Returns the stable descriptive policy identity.
    #[must_use]
    pub fn policy_id(&self) -> &str {
        &self.policy_id
    }

    /// Returns the content-derived identity binding every limit and class position.
    #[must_use]
    pub fn policy_sha256(&self) -> &str {
        &self.policy_sha256
    }

    /// Returns the inclusive parser-repair limit.
    #[must_use]
    pub const fn parser_repair_limit(&self) -> u64 {
        self.parser_repairs
    }

    /// Returns the inclusive step-attempt limit.
    #[must_use]
    pub const fn step_attempt_limit(&self) -> u64 {
        self.step_attempts
    }

    /// Returns the inclusive limit for one closed failure class.
    #[must_use]
    pub const fn error_class_limit(&self, failure_class: CanonicalWorkflowFailureClass) -> u64 {
        self.per_error_class[failure_class_index(failure_class)]
    }

    /// Returns the inclusive total workflow-work limit.
    #[must_use]
    pub const fn workflow_work_limit(&self) -> u64 {
        self.workflow_work
    }

    /// Returns the inclusive replan limit.
    #[must_use]
    pub const fn replan_limit(&self) -> u64 {
        self.replans
    }

    const fn limit(&self, dimension: WorkflowBudgetDimension) -> u64 {
        match dimension {
            WorkflowBudgetDimension::ParserRepair => self.parser_repairs,
            WorkflowBudgetDimension::StepAttempt => self.step_attempts,
            WorkflowBudgetDimension::ErrorClass(failure_class) => {
                self.per_error_class[failure_class_index(failure_class)]
            }
            WorkflowBudgetDimension::WorkflowWork => self.workflow_work,
            WorkflowBudgetDimension::Replan => self.replans,
        }
    }
}

/// One separately accounted supervision event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowBudgetEvent {
    /// One deterministic parser-repair operation.
    ParserRepair,
    /// One newly opened step attempt.
    StepAttempt,
    /// One observed failure in its exact closed class.
    ErrorClass(CanonicalWorkflowFailureClass),
    /// One admitted replan operation.
    Replan,
}

/// Exact budget dimension used in accounting decisions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowBudgetDimension {
    /// Parser repair count.
    ParserRepair,
    /// Step attempt count.
    StepAttempt,
    /// Count for one closed failure class.
    ErrorClass(CanonicalWorkflowFailureClass),
    /// Total supervised workflow work across all event families.
    WorkflowWork,
    /// Replan count.
    Replan,
}

/// Admitted usage returned after one atomic checked consumption.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkflowBudgetAdmission {
    /// Separately charged primary dimension.
    pub dimension: WorkflowBudgetDimension,
    /// Admitted usage in the primary dimension.
    pub dimension_used: u64,
    /// Remaining capacity in the primary dimension.
    pub dimension_remaining: u64,
    /// Admitted total workflow work.
    pub workflow_work_used: u64,
    /// Remaining total workflow-work capacity.
    pub workflow_work_remaining: u64,
}

/// Closed failure returned without changing any usage counter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowBudgetError {
    /// Policy identity, class coverage, or ordering input is invalid.
    InvalidPolicy,
    /// A different immutable policy was presented to an existing ledger.
    PolicyMismatch,
    /// Checked addition overflowed.
    CounterOverflow {
        /// Dimension whose addition overflowed.
        dimension: WorkflowBudgetDimension,
    },
    /// A proposed total exceeded its inclusive declared limit.
    BudgetExceeded {
        /// Exhausted dimension.
        dimension: WorkflowBudgetDimension,
        /// Inclusive limit.
        limit: u64,
        /// Rejected proposed total.
        attempted_total: u64,
    },
}

/// Mutable usage ledger permanently bound to one immutable policy identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkflowBudgetLedger {
    policy_id: String,
    policy_sha256: String,
    parser_repairs: u64,
    step_attempts: u64,
    per_error_class: [u64; FAILURE_CLASS_COUNT],
    workflow_work: u64,
    replans: u64,
}

impl WorkflowBudgetLedger {
    /// Creates a zero-usage ledger bound to one exact policy.
    #[must_use]
    pub fn new(policy: &WorkflowBudgetPolicy) -> Self {
        Self {
            policy_id: policy.policy_id.clone(),
            policy_sha256: policy.policy_sha256.clone(),
            parser_repairs: 0,
            step_attempts: 0,
            per_error_class: [0; FAILURE_CLASS_COUNT],
            workflow_work: 0,
            replans: 0,
        }
    }

    /// Atomically charges one event to its separate dimension and the total workflow dimension.
    pub fn consume(
        &mut self,
        policy: &WorkflowBudgetPolicy,
        event: WorkflowBudgetEvent,
        amount: u64,
    ) -> Result<WorkflowBudgetAdmission, WorkflowBudgetError> {
        if self.policy_id != policy.policy_id || self.policy_sha256 != policy.policy_sha256 {
            return Err(WorkflowBudgetError::PolicyMismatch);
        }
        let dimension = event_dimension(event);
        let current = self.usage(dimension);
        let attempted = current
            .checked_add(amount)
            .ok_or(WorkflowBudgetError::CounterOverflow { dimension })?;
        let limit = policy.limit(dimension);
        if attempted > limit {
            return Err(WorkflowBudgetError::BudgetExceeded {
                dimension,
                limit,
                attempted_total: attempted,
            });
        }
        let workflow_attempted =
            self.workflow_work
                .checked_add(amount)
                .ok_or(WorkflowBudgetError::CounterOverflow {
                    dimension: WorkflowBudgetDimension::WorkflowWork,
                })?;
        if workflow_attempted > policy.workflow_work {
            return Err(WorkflowBudgetError::BudgetExceeded {
                dimension: WorkflowBudgetDimension::WorkflowWork,
                limit: policy.workflow_work,
                attempted_total: workflow_attempted,
            });
        }

        self.set_usage(dimension, attempted);
        self.workflow_work = workflow_attempted;
        Ok(WorkflowBudgetAdmission {
            dimension,
            dimension_used: attempted,
            dimension_remaining: limit - attempted,
            workflow_work_used: workflow_attempted,
            workflow_work_remaining: policy.workflow_work - workflow_attempted,
        })
    }

    /// Returns admitted usage for one separate dimension.
    #[must_use]
    pub const fn usage(&self, dimension: WorkflowBudgetDimension) -> u64 {
        match dimension {
            WorkflowBudgetDimension::ParserRepair => self.parser_repairs,
            WorkflowBudgetDimension::StepAttempt => self.step_attempts,
            WorkflowBudgetDimension::ErrorClass(failure_class) => {
                self.per_error_class[failure_class_index(failure_class)]
            }
            WorkflowBudgetDimension::WorkflowWork => self.workflow_work,
            WorkflowBudgetDimension::Replan => self.replans,
        }
    }

    /// Returns the immutable policy digest retained by this ledger.
    #[must_use]
    pub fn policy_sha256(&self) -> &str {
        &self.policy_sha256
    }

    fn set_usage(&mut self, dimension: WorkflowBudgetDimension, value: u64) {
        match dimension {
            WorkflowBudgetDimension::ParserRepair => self.parser_repairs = value,
            WorkflowBudgetDimension::StepAttempt => self.step_attempts = value,
            WorkflowBudgetDimension::ErrorClass(failure_class) => {
                self.per_error_class[failure_class_index(failure_class)] = value;
            }
            WorkflowBudgetDimension::WorkflowWork => self.workflow_work = value,
            WorkflowBudgetDimension::Replan => self.replans = value,
        }
    }
}

const fn event_dimension(event: WorkflowBudgetEvent) -> WorkflowBudgetDimension {
    match event {
        WorkflowBudgetEvent::ParserRepair => WorkflowBudgetDimension::ParserRepair,
        WorkflowBudgetEvent::StepAttempt => WorkflowBudgetDimension::StepAttempt,
        WorkflowBudgetEvent::ErrorClass(failure_class) => {
            WorkflowBudgetDimension::ErrorClass(failure_class)
        }
        WorkflowBudgetEvent::Replan => WorkflowBudgetDimension::Replan,
    }
}

const fn failure_class_index(failure_class: CanonicalWorkflowFailureClass) -> usize {
    match failure_class {
        CanonicalWorkflowFailureClass::MalformedInput => 0,
        CanonicalWorkflowFailureClass::Preflight => 1,
        CanonicalWorkflowFailureClass::Policy => 2,
        CanonicalWorkflowFailureClass::Approval => 3,
        CanonicalWorkflowFailureClass::Dependency => 4,
        CanonicalWorkflowFailureClass::Transient => 5,
        CanonicalWorkflowFailureClass::Conflict => 6,
        CanonicalWorkflowFailureClass::Timeout => 7,
        CanonicalWorkflowFailureClass::Cancellation => 8,
        CanonicalWorkflowFailureClass::Crash => 9,
        CanonicalWorkflowFailureClass::UncertainEffect => 10,
        CanonicalWorkflowFailureClass::Verification => 11,
        CanonicalWorkflowFailureClass::Resource => 12,
        CanonicalWorkflowFailureClass::Internal => 13,
    }
}

fn policy_digest(policy: &WorkflowBudgetPolicy) -> String {
    let mut digest = Sha256::new();
    digest.update(b"agentmage.workflow-budget-policy.v1\0");
    digest.update(
        u64::try_from(policy.policy_id.len())
            .unwrap_or(u64::MAX)
            .to_be_bytes(),
    );
    digest.update(policy.policy_id.as_bytes());
    digest.update(policy.parser_repairs.to_be_bytes());
    digest.update(policy.step_attempts.to_be_bytes());
    for limit in policy.per_error_class {
        digest.update(limit.to_be_bytes());
    }
    digest.update(policy.workflow_work.to_be_bytes());
    digest.update(policy.replans.to_be_bytes());
    let mut output = String::with_capacity(64);
    for byte in digest.finalize() {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.is_ascii()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error_limits(limit: u64) -> Vec<ErrorClassBudgetLimit> {
        CanonicalWorkflowFailureClass::ALL
            .into_iter()
            .map(|failure_class| ErrorClassBudgetLimit {
                failure_class,
                limit,
            })
            .collect()
    }

    fn policy() -> WorkflowBudgetPolicy {
        WorkflowBudgetPolicy::new("budget-policy-1".to_owned(), 2, 3, &error_limits(2), 20, 2)
            .expect("complete budget policy")
    }

    #[test]
    fn every_dimension_is_separate_and_every_error_class_is_exact() {
        let policy = policy();
        let mut ledger = WorkflowBudgetLedger::new(&policy);
        let parser = ledger
            .consume(&policy, WorkflowBudgetEvent::ParserRepair, 2)
            .expect("parser budget");
        assert_eq!(parser.dimension_remaining, 0);
        assert_eq!(ledger.usage(WorkflowBudgetDimension::StepAttempt), 0);
        ledger
            .consume(&policy, WorkflowBudgetEvent::StepAttempt, 1)
            .expect("step budget");
        ledger
            .consume(&policy, WorkflowBudgetEvent::Replan, 1)
            .expect("replan budget");
        for failure_class in CanonicalWorkflowFailureClass::ALL {
            ledger
                .consume(&policy, WorkflowBudgetEvent::ErrorClass(failure_class), 1)
                .expect("independent error-class budget");
            assert_eq!(
                ledger.usage(WorkflowBudgetDimension::ErrorClass(failure_class)),
                1
            );
        }
        assert_eq!(ledger.usage(WorkflowBudgetDimension::WorkflowWork), 18);
    }

    #[test]
    fn missing_or_duplicate_error_classes_cannot_form_a_policy() {
        let mut missing = error_limits(1);
        missing.pop();
        assert_eq!(
            WorkflowBudgetPolicy::new("budget-policy-1".to_owned(), 1, 1, &missing, 10, 1),
            Err(WorkflowBudgetError::InvalidPolicy),
        );
        let mut repeated = error_limits(1);
        repeated[13] = repeated[0];
        assert_eq!(
            WorkflowBudgetPolicy::new("budget-policy-1".to_owned(), 1, 1, &repeated, 10, 1),
            Err(WorkflowBudgetError::InvalidPolicy),
        );
    }

    #[test]
    fn checked_arithmetic_and_total_budget_fail_without_partial_consumption() {
        let overflow_policy = WorkflowBudgetPolicy::new(
            "overflow-policy".to_owned(),
            u64::MAX,
            1,
            &error_limits(1),
            u64::MAX,
            1,
        )
        .expect("maximum limits are representable");
        let mut overflow = WorkflowBudgetLedger::new(&overflow_policy);
        overflow
            .consume(
                &overflow_policy,
                WorkflowBudgetEvent::ParserRepair,
                u64::MAX,
            )
            .expect("inclusive maximum");
        assert_eq!(
            overflow.consume(&overflow_policy, WorkflowBudgetEvent::ParserRepair, 1),
            Err(WorkflowBudgetError::CounterOverflow {
                dimension: WorkflowBudgetDimension::ParserRepair,
            }),
        );
        assert_eq!(
            overflow.usage(WorkflowBudgetDimension::ParserRepair),
            u64::MAX
        );

        let total_policy =
            WorkflowBudgetPolicy::new("total-policy".to_owned(), 5, 5, &error_limits(5), 1, 5)
                .expect("total policy");
        let mut total = WorkflowBudgetLedger::new(&total_policy);
        total
            .consume(&total_policy, WorkflowBudgetEvent::StepAttempt, 1)
            .expect("first workflow unit");
        assert_eq!(
            total.consume(&total_policy, WorkflowBudgetEvent::ParserRepair, 1),
            Err(WorkflowBudgetError::BudgetExceeded {
                dimension: WorkflowBudgetDimension::WorkflowWork,
                limit: 1,
                attempted_total: 2,
            }),
        );
        assert_eq!(total.usage(WorkflowBudgetDimension::ParserRepair), 0);
    }

    #[test]
    fn content_identity_is_immutable_and_substitution_is_denied() {
        let policy = policy();
        let changed =
            WorkflowBudgetPolicy::new("budget-policy-1".to_owned(), 3, 3, &error_limits(2), 20, 2)
                .expect("changed policy");
        assert_ne!(policy.policy_sha256(), changed.policy_sha256());
        let mut ledger = WorkflowBudgetLedger::new(&policy);
        assert_eq!(ledger.policy_sha256(), policy.policy_sha256());
        assert_eq!(
            ledger.consume(&changed, WorkflowBudgetEvent::ParserRepair, 1),
            Err(WorkflowBudgetError::PolicyMismatch),
        );
        assert_eq!(ledger.usage(WorkflowBudgetDimension::ParserRepair), 0);
    }
}
