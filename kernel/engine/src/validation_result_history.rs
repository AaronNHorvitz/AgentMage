// SPDX-License-Identifier: MIT OR Apache-2.0
//! Exact process-retained validation history, without current-source acceptance.

use std::collections::BTreeSet;

use crate::command_runner::{CommandReceipt, PreparedCommand};
use crate::validation_template::ValidationTemplate;

use super::{ValidationReceipt, ValidationRerunPlan, verify_rerun_plan, verify_validation_receipt};

/// Original command preparation and terminal evidence for one validation.
/// A caller-supplied record is inert; verification checks every field against
/// the registered template and original command receipt before using it.
#[derive(Clone, Debug)]
pub struct ValidationAttemptEvidence {
    /// Original exact command preparation, carrying no execution authority.
    pub prepared: PreparedCommand,
    /// Original terminal command receipt.
    pub command: CommandReceipt,
    /// Original normalized validation receipt, including failures.
    pub validation: ValidationReceipt,
}

/// Checks one complete series of at most three separately approved attempts.
/// Every earlier failure must link to its immediate successor through the
/// original rerun plan. The final result must be a full pass. This does not
/// erase failed evidence, establish current source, or grade a coding task.
/// Inputs must come from the trusted executor; hashes do not prove provenance.
#[must_use]
pub fn verify_bounded_validation_history(
    template: &ValidationTemplate,
    attempts: &[ValidationAttemptEvidence],
    reruns: &[ValidationRerunPlan],
) -> bool {
    if !(1..=3).contains(&attempts.len()) || reruns.len() != attempts.len() - 1 {
        return false;
    }
    let mut validation_ids = BTreeSet::new();
    let mut command_ids = BTreeSet::new();
    let mut approvals = BTreeSet::new();
    let mut transactions = BTreeSet::new();
    let mut operations = BTreeSet::new();
    for attempt in attempts {
        let receipt = &attempt.validation;
        if !verify_validation_receipt(template, &attempt.prepared, &attempt.command, receipt)
            || !validation_ids.insert(&receipt.validation_attempt_id)
            || !command_ids.insert(&receipt.command_attempt_id)
            || !approvals.insert(&receipt.approval_sha256)
            || !transactions.insert(&receipt.authority_transaction_id)
            || !operations.insert(&receipt.operation_attempt_id)
        {
            return false;
        }
    }
    for (pair, plan) in attempts.windows(2).zip(reruns) {
        let next = &pair[1].validation;
        if !verify_rerun_plan(template, &pair[0].validation, plan)
            || plan.validation_attempt_id != next.validation_attempt_id
            || plan.approval_sha256 != next.approval_sha256
        {
            return false;
        }
    }
    attempts
        .last()
        .is_some_and(|last| last.validation.is_full_pass())
}
