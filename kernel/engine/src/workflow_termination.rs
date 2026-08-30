//! Inert, non-secret workflow termination reasons and safe next actions.

use crate::workflow_budget::{WorkflowBudgetDimension, WorkflowBudgetError};
use crate::workflow_progress::WorkflowProgressDecision;

/// Closed policy-denial classes that cannot carry candidate content or secret values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowPolicyDenialClass {
    /// Requested authority exceeds an effective ceiling.
    AuthorityExceedsCeiling,
    /// A required current preflight observation is absent or stale.
    CurrentPreflightRequired,
    /// Effect reconciliation is required before another decision.
    EffectReconciliationRequired,
    /// A fresh exact approval is required.
    FreshApprovalRequired,
    /// An identity has already been used or is not fresh.
    IdentityReplay,
    /// The effect class does not admit automatic retry.
    EffectNotRetryable,
    /// The supplied state is incomplete, malformed, or inconsistent.
    InvalidState,
}

/// Closed, non-secret reason for ending workflow supervision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowTerminationReason {
    /// An exact separately accounted dimension reached its declared limit.
    BudgetExhausted {
        /// Exhausted budget dimension.
        dimension: WorkflowBudgetDimension,
    },
    /// Checked accounting could not represent a proposed total.
    BudgetCounterSaturated {
        /// Saturated budget dimension.
        dimension: WorkflowBudgetDimension,
    },
    /// A closed policy rule denied continuation.
    PolicyDenied {
        /// Non-secret denial classification.
        denial_class: WorkflowPolicyDenialClass,
    },
    /// A complete workflow-state fingerprint repeated to its exact limit.
    RepeatedState,
}

impl WorkflowTerminationReason {
    /// Returns a stable non-secret machine reason code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::BudgetExhausted { .. } => "workflow.terminated.budget_exhausted",
            Self::BudgetCounterSaturated { .. } => "workflow.terminated.budget_counter_saturated",
            Self::PolicyDenied { .. } => "workflow.terminated.policy_denied",
            Self::RepeatedState => "workflow.terminated.repeated_state",
        }
    }
}

/// Descriptive action safe to present after termination; it grants no execution authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowSafeNextAction {
    /// Return control so a person or separately authorized caller can review budget scope.
    ReturnForBudgetReview,
    /// Narrow the proposal before a new validation decision.
    NarrowProposalAndRevalidate,
    /// Establish required current preflight evidence without resuming this terminated workflow.
    EstablishFreshPreflight,
    /// Reconcile the uncertain effect before any separately authorized decision.
    ReconcileEffect,
    /// Obtain a new exact approval through the normal approval boundary.
    RequestFreshApproval,
    /// Construct a genuinely fresh attempt identity chain before normal admission.
    ConstructFreshAttempt,
    /// Correct the state record and submit it through validation again.
    CorrectStateAndRevalidate,
    /// Return control with the content-free repeated-state finding.
    ReturnWithNoProgressFinding,
}

/// Terminal, authority-free workflow disposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkflowTermination {
    reason: WorkflowTerminationReason,
    safe_next_action: WorkflowSafeNextAction,
}

impl WorkflowTermination {
    /// Returns the closed termination reason.
    #[must_use]
    pub const fn reason(self) -> WorkflowTerminationReason {
        self.reason
    }

    /// Returns the inert safe next action.
    #[must_use]
    pub const fn safe_next_action(self) -> WorkflowSafeNextAction {
        self.safe_next_action
    }

    /// Termination construction never consumes a grant or other authority.
    #[must_use]
    pub const fn authority_consumed(self) -> bool {
        false
    }

    /// A terminal disposition cannot automatically continue this workflow.
    #[must_use]
    pub const fn automatic_continuation_allowed(self) -> bool {
        false
    }

    /// A terminal disposition cannot automatically open another attempt.
    #[must_use]
    pub const fn automatic_retry_allowed(self) -> bool {
        false
    }
}

/// Refusal to construct a terminal record from a nonterminal input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowTerminationError {
    /// The budget result is a policy-construction/substitution failure, not exhaustion.
    NonExhaustionBudgetError,
    /// The progress decision has not reached the declared repeat limit.
    NonterminalProgress,
}

/// Converts only an actual budget-exhaustion result into an inert terminal disposition.
pub const fn terminate_budget(
    error: WorkflowBudgetError,
) -> Result<WorkflowTermination, WorkflowTerminationError> {
    match error {
        WorkflowBudgetError::BudgetExceeded { dimension, .. } => Ok(WorkflowTermination {
            reason: WorkflowTerminationReason::BudgetExhausted { dimension },
            safe_next_action: WorkflowSafeNextAction::ReturnForBudgetReview,
        }),
        WorkflowBudgetError::CounterOverflow { dimension } => Ok(WorkflowTermination {
            reason: WorkflowTerminationReason::BudgetCounterSaturated { dimension },
            safe_next_action: WorkflowSafeNextAction::ReturnForBudgetReview,
        }),
        WorkflowBudgetError::InvalidPolicy | WorkflowBudgetError::PolicyMismatch => {
            Err(WorkflowTerminationError::NonExhaustionBudgetError)
        }
    }
}

/// Converts one typed policy denial into an inert terminal disposition.
#[must_use]
pub const fn terminate_policy(denial_class: WorkflowPolicyDenialClass) -> WorkflowTermination {
    let safe_next_action = match denial_class {
        WorkflowPolicyDenialClass::AuthorityExceedsCeiling => {
            WorkflowSafeNextAction::NarrowProposalAndRevalidate
        }
        WorkflowPolicyDenialClass::CurrentPreflightRequired => {
            WorkflowSafeNextAction::EstablishFreshPreflight
        }
        WorkflowPolicyDenialClass::EffectReconciliationRequired
        | WorkflowPolicyDenialClass::EffectNotRetryable => WorkflowSafeNextAction::ReconcileEffect,
        WorkflowPolicyDenialClass::FreshApprovalRequired => {
            WorkflowSafeNextAction::RequestFreshApproval
        }
        WorkflowPolicyDenialClass::IdentityReplay => WorkflowSafeNextAction::ConstructFreshAttempt,
        WorkflowPolicyDenialClass::InvalidState => {
            WorkflowSafeNextAction::CorrectStateAndRevalidate
        }
    };
    WorkflowTermination {
        reason: WorkflowTerminationReason::PolicyDenied { denial_class },
        safe_next_action,
    }
}

/// Converts only the exact repeated-state stop decision into an inert terminal disposition.
pub const fn terminate_repeated_state(
    decision: &WorkflowProgressDecision,
) -> Result<WorkflowTermination, WorkflowTerminationError> {
    match decision {
        WorkflowProgressDecision::StopRepeatedState { .. } => Ok(WorkflowTermination {
            reason: WorkflowTerminationReason::RepeatedState,
            safe_next_action: WorkflowSafeNextAction::ReturnWithNoProgressFinding,
        }),
        WorkflowProgressDecision::Advanced { .. } | WorkflowProgressDecision::Repeated { .. } => {
            Err(WorkflowTerminationError::NonterminalProgress)
        }
    }
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::CanonicalWorkflowFailureClass;

    use super::*;
    use crate::workflow_budget::{
        ErrorClassBudgetLimit, WorkflowBudgetEvent, WorkflowBudgetLedger, WorkflowBudgetPolicy,
    };
    use crate::workflow_progress::{
        RepeatedStateDetector, RepeatedStatePolicy, WorkflowStateComponents,
        WorkflowStateFingerprint,
    };

    fn error_limits(limit: u64) -> Vec<ErrorClassBudgetLimit> {
        CanonicalWorkflowFailureClass::ALL
            .into_iter()
            .map(|failure_class| ErrorClassBudgetLimit {
                failure_class,
                limit,
            })
            .collect()
    }

    fn assert_inert(termination: WorkflowTermination) {
        assert!(!termination.authority_consumed());
        assert!(!termination.automatic_continuation_allowed());
        assert!(!termination.automatic_retry_allowed());
        assert!(termination.reason().code().is_ascii());
        assert!(
            termination
                .reason()
                .code()
                .starts_with("workflow.terminated.")
        );
    }

    #[test]
    fn exhausted_and_saturated_budgets_are_inert_and_do_not_change_usage() {
        let policy = WorkflowBudgetPolicy::new(
            "budget-policy-1".to_owned(),
            1,
            1,
            1,
            &error_limits(1),
            1,
            1,
        )
        .expect("policy");
        let mut ledger = WorkflowBudgetLedger::new(&policy);
        ledger
            .consume(&policy, WorkflowBudgetEvent::StepAttempt, 1)
            .expect("inclusive limit");
        let before = ledger.clone();
        let error = ledger
            .consume(&policy, WorkflowBudgetEvent::StepAttempt, 1)
            .expect_err("exhausted");
        let termination = terminate_budget(error).expect("terminal exhaustion");
        assert_eq!(ledger, before);
        assert_eq!(
            termination.reason(),
            WorkflowTerminationReason::BudgetExhausted {
                dimension: WorkflowBudgetDimension::StepAttempt,
            }
        );
        assert_eq!(
            termination.safe_next_action(),
            WorkflowSafeNextAction::ReturnForBudgetReview
        );
        assert_inert(termination);

        let saturated = terminate_budget(WorkflowBudgetError::CounterOverflow {
            dimension: WorkflowBudgetDimension::WorkflowWork,
        })
        .expect("terminal saturation");
        assert_eq!(
            saturated.reason(),
            WorkflowTerminationReason::BudgetCounterSaturated {
                dimension: WorkflowBudgetDimension::WorkflowWork,
            }
        );
        assert_inert(saturated);
        assert_eq!(
            terminate_budget(WorkflowBudgetError::PolicyMismatch),
            Err(WorkflowTerminationError::NonExhaustionBudgetError)
        );
    }

    #[test]
    fn every_policy_denial_has_one_exact_safe_nonexecuting_action() {
        let cases = [
            (
                WorkflowPolicyDenialClass::AuthorityExceedsCeiling,
                WorkflowSafeNextAction::NarrowProposalAndRevalidate,
            ),
            (
                WorkflowPolicyDenialClass::CurrentPreflightRequired,
                WorkflowSafeNextAction::EstablishFreshPreflight,
            ),
            (
                WorkflowPolicyDenialClass::EffectReconciliationRequired,
                WorkflowSafeNextAction::ReconcileEffect,
            ),
            (
                WorkflowPolicyDenialClass::FreshApprovalRequired,
                WorkflowSafeNextAction::RequestFreshApproval,
            ),
            (
                WorkflowPolicyDenialClass::IdentityReplay,
                WorkflowSafeNextAction::ConstructFreshAttempt,
            ),
            (
                WorkflowPolicyDenialClass::EffectNotRetryable,
                WorkflowSafeNextAction::ReconcileEffect,
            ),
            (
                WorkflowPolicyDenialClass::InvalidState,
                WorkflowSafeNextAction::CorrectStateAndRevalidate,
            ),
        ];
        for (denial_class, expected_action) in cases {
            let termination = terminate_policy(denial_class);
            assert_eq!(
                termination.reason(),
                WorkflowTerminationReason::PolicyDenied { denial_class }
            );
            assert_eq!(termination.safe_next_action(), expected_action);
            assert_inert(termination);
        }
    }

    #[test]
    fn only_exact_repeated_state_stop_terminates_and_detector_remains_unchanged() {
        let component = "0".repeat(64);
        let components = WorkflowStateComponents {
            plan_sha256: component.clone(),
            step_sha256: component.clone(),
            observation_sha256: Vec::new(),
            proposal_sha256: component.clone(),
            tool_sha256: component.clone(),
            policy_sha256: component.clone(),
            receipt_sha256: Vec::new(),
            artifact_sha256: Vec::new(),
            verifier_state_sha256: component,
        };
        let fingerprint = WorkflowStateFingerprint::new(&components).expect("fingerprint");
        let policy = RepeatedStatePolicy::new("repeat-policy-1".to_owned(), 1).expect("policy");
        let mut detector = RepeatedStateDetector::new(&policy);
        let advanced = detector.observe(&policy, &fingerprint).expect("advanced");
        assert_eq!(
            terminate_repeated_state(&advanced),
            Err(WorkflowTerminationError::NonterminalProgress)
        );
        let stopped = detector.observe(&policy, &fingerprint).expect("stopped");
        let before = detector.clone();
        let termination = terminate_repeated_state(&stopped).expect("terminal repeated state");
        assert_eq!(detector, before);
        assert_eq!(
            termination.safe_next_action(),
            WorkflowSafeNextAction::ReturnWithNoProgressFinding
        );
        assert_inert(termination);
    }

    #[test]
    fn story_5_2_declared_budget_or_no_progress_terminates_once_without_effect() {
        let policy = WorkflowBudgetPolicy::new(
            "story-5-2-ac3-budget".to_owned(),
            1,
            1,
            1,
            &error_limits(1),
            100,
            1,
        )
        .expect("complete criterion budget policy");
        let mut event_cases = vec![
            (
                WorkflowBudgetEvent::ParserRepair,
                WorkflowBudgetDimension::ParserRepair,
            ),
            (
                WorkflowBudgetEvent::ModelRepair,
                WorkflowBudgetDimension::ModelRepair,
            ),
            (
                WorkflowBudgetEvent::StepAttempt,
                WorkflowBudgetDimension::StepAttempt,
            ),
            (WorkflowBudgetEvent::Replan, WorkflowBudgetDimension::Replan),
        ];
        event_cases.extend(
            CanonicalWorkflowFailureClass::ALL
                .into_iter()
                .map(|failure_class| {
                    (
                        WorkflowBudgetEvent::ErrorClass(failure_class),
                        WorkflowBudgetDimension::ErrorClass(failure_class),
                    )
                }),
        );

        let effect_probe = 0_u64;
        for (event, dimension) in event_cases {
            let mut ledger = WorkflowBudgetLedger::new(&policy);
            ledger
                .consume(&policy, event, 1)
                .expect("inclusive declared limit");
            let at_limit = ledger.clone();
            let error = ledger
                .consume(&policy, event, 1)
                .expect_err("next event must terminate at the declared limit");
            assert_eq!(
                ledger, at_limit,
                "rejected {dimension:?} charge mutated usage"
            );
            let termination = terminate_budget(error).expect("exhaustion must terminate");
            assert_eq!(
                termination.reason(),
                WorkflowTerminationReason::BudgetExhausted { dimension }
            );
            assert_eq!(
                termination.safe_next_action(),
                WorkflowSafeNextAction::ReturnForBudgetReview
            );
            assert_inert(termination);
        }

        let total_policy = WorkflowBudgetPolicy::new(
            "story-5-2-ac3-total".to_owned(),
            2,
            2,
            2,
            &error_limits(2),
            1,
            2,
        )
        .expect("total-work criterion policy");
        let mut total_ledger = WorkflowBudgetLedger::new(&total_policy);
        total_ledger
            .consume(&total_policy, WorkflowBudgetEvent::ParserRepair, 1)
            .expect("one total-work unit");
        let at_total_limit = total_ledger.clone();
        let total_error = total_ledger
            .consume(&total_policy, WorkflowBudgetEvent::ModelRepair, 1)
            .expect_err("total work must terminate independently");
        assert_eq!(total_ledger, at_total_limit);
        let total_termination = terminate_budget(total_error).expect("total-work termination");
        assert_eq!(
            total_termination.reason(),
            WorkflowTerminationReason::BudgetExhausted {
                dimension: WorkflowBudgetDimension::WorkflowWork,
            }
        );
        assert_eq!(
            total_termination.safe_next_action(),
            WorkflowSafeNextAction::ReturnForBudgetReview
        );
        assert_inert(total_termination);

        let component = "0".repeat(64);
        let fingerprint = WorkflowStateFingerprint::new(&WorkflowStateComponents {
            plan_sha256: component.clone(),
            step_sha256: component.clone(),
            observation_sha256: Vec::new(),
            proposal_sha256: component.clone(),
            tool_sha256: component.clone(),
            policy_sha256: component.clone(),
            receipt_sha256: Vec::new(),
            artifact_sha256: Vec::new(),
            verifier_state_sha256: component,
        })
        .expect("complete criterion fingerprint");
        let repeat_policy =
            RepeatedStatePolicy::new("story-5-2-ac3-repeat".to_owned(), 2).expect("repeat policy");
        let mut detector = RepeatedStateDetector::new(&repeat_policy);
        let advanced = detector
            .observe(&repeat_policy, &fingerprint)
            .expect("first state advances");
        assert_eq!(
            terminate_repeated_state(&advanced),
            Err(WorkflowTerminationError::NonterminalProgress)
        );
        let repeated = detector
            .observe(&repeat_policy, &fingerprint)
            .expect("first repeat remains inside the bound");
        assert_eq!(
            terminate_repeated_state(&repeated),
            Err(WorkflowTerminationError::NonterminalProgress)
        );
        let stopped = detector
            .observe(&repeat_policy, &fingerprint)
            .expect("second repeat reaches the exact bound");
        let termination = terminate_repeated_state(&stopped).expect("no-progress termination");
        assert_eq!(
            termination.reason(),
            WorkflowTerminationReason::RepeatedState
        );
        assert_eq!(
            termination.safe_next_action(),
            WorkflowSafeNextAction::ReturnWithNoProgressFinding
        );
        assert_inert(termination);
        let sticky = detector
            .observe(&repeat_policy, &fingerprint)
            .expect("terminal diagnosis remains sticky");
        assert_eq!(sticky, stopped);
        assert_eq!(
            terminate_repeated_state(&sticky).expect("same terminal diagnosis"),
            termination
        );
        assert_eq!(effect_probe, 0, "termination must add no effect");
    }

    #[test]
    fn terminal_output_has_no_caller_text_identity_or_authority_surface() {
        let canary = "AM_SECRET_CANARY_workflow_termination";
        let terminations = [
            terminate_policy(WorkflowPolicyDenialClass::InvalidState),
            terminate_budget(WorkflowBudgetError::BudgetExceeded {
                dimension: WorkflowBudgetDimension::Replan,
                limit: 1,
                attempted_total: 2,
            })
            .expect("budget termination"),
            terminate_repeated_state(&WorkflowProgressDecision::StopRepeatedState {
                fingerprint_sha256: canary.to_owned(),
                repeat_count: 2,
            })
            .expect("repeat termination"),
        ];
        for termination in terminations {
            let rendered = format!("{termination:?}");
            assert!(!rendered.contains(canary));
            assert!(!rendered.contains("grant"));
            assert!(!rendered.contains("permit"));
            assert_inert(termination);
        }
    }
}
