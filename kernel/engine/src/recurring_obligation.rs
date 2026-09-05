//! Cited, uncertainty-preserving recurring-obligation tracking.
#![allow(missing_docs)]

use crate::financial_domain::{Money, UserDisposition};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObligationKind {
    RecurringExpense,
    RecurringIncome,
    Fee,
    Subscription,
    PriceChange,
    MissingExpectedRecord,
    DuplicateChargeCandidate,
    CancellationCandidate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetectionMethod {
    Deterministic,
    ModelAssisted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObligationStatus {
    Candidate,
    Confirmed,
    Uncertain,
    Dismissed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecurringObligation {
    pub obligation_id: String,
    pub kind: ObligationKind,
    pub merchant_or_payee_id: String,
    pub recurrence_rule: String,
    pub cadence_days_min: u16,
    pub cadence_days_max: u16,
    pub expected_amount_min: Money,
    pub expected_amount_max: Money,
    pub due_window_start: String,
    pub due_window_end: String,
    pub confidence_basis_points: u16,
    pub financial_source_ids: BTreeSet<String>,
    pub communication_source_ids: BTreeSet<String>,
    pub limitations: BTreeSet<String>,
    pub method: DetectionMethod,
    pub status: ObligationStatus,
    pub user_disposition: UserDisposition,
    pub reversible: bool,
    pub authority: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CommunicationEvidenceKind {
    Invoice,
    RenewalNotice,
    Receipt,
    PriceChangeNotice,
    CancellationTerms,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObligationEvidenceLink {
    pub link_id: String,
    pub obligation_id: String,
    pub financial_record_id: String,
    pub communication_record_id: String,
    pub communication_kind: CommunicationEvidenceKind,
    pub evidence_sha256: String,
    pub confidence_basis_points: u16,
    pub confirmed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReminderKind {
    Due,
    Renewal,
    PriceChange,
    MissingRecord,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalObligationReminder {
    pub reminder_id: String,
    pub obligation_id: String,
    pub kind: ReminderKind,
    pub source_ids: BTreeSet<String>,
    pub due_at: String,
    pub expires_at: String,
    pub acknowledged_at: Option<String>,
    pub dismissed_at: Option<String>,
    pub local_only: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApprovedCommunicationDraft {
    pub draft_id: String,
    pub obligation_id: String,
    pub exact_recipient_id: String,
    pub content_sha256: String,
    pub approval_id: String,
    pub approval_fresh: bool,
    pub external_effect: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObligationError {
    Invalid,
    Ambiguous,
    UntrustedContent,
    Unapproved,
    UnsupportedEffect,
}

fn identifier(value: &str) -> bool {
    !value.is_empty() && value.len() <= 1024
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub fn validate_detection(value: &RecurringObligation) -> Result<(), ObligationError> {
    if !identifier(&value.obligation_id)
        || !identifier(&value.merchant_or_payee_id)
        || !identifier(&value.recurrence_rule)
        || value.cadence_days_min == 0
        || value.cadence_days_min > value.cadence_days_max
        || value.confidence_basis_points > 10_000
        || value.financial_source_ids.is_empty()
        || value.expected_amount_min.currency() != value.expected_amount_max.currency()
        || value.expected_amount_min.scale() != value.expected_amount_max.scale()
        || value.expected_amount_min.minor_units() > value.expected_amount_max.minor_units()
        || value.authority
    {
        return Err(ObligationError::Invalid);
    }
    if value.method == DetectionMethod::ModelAssisted
        && (!value.reversible
            || value.status == ObligationStatus::Confirmed
            || value.user_disposition == UserDisposition::Accepted)
    {
        return Err(ObligationError::Ambiguous);
    }
    if (value.status == ObligationStatus::Uncertain || value.confidence_basis_points < 10_000)
        && value.limitations.is_empty()
    {
        return Err(ObligationError::Ambiguous);
    }
    Ok(())
}

pub fn validate_link(link: &ObligationEvidenceLink) -> Result<(), ObligationError> {
    if !identifier(&link.link_id)
        || !identifier(&link.obligation_id)
        || !identifier(&link.financial_record_id)
        || !identifier(&link.communication_record_id)
        || !digest(&link.evidence_sha256)
        || link.confidence_basis_points > 10_000
    {
        return Err(ObligationError::Invalid);
    }
    Ok(())
}

pub fn validate_reminder(reminder: &LocalObligationReminder) -> Result<(), ObligationError> {
    if !identifier(&reminder.reminder_id)
        || !identifier(&reminder.obligation_id)
        || reminder.source_ids.is_empty()
        || !identifier(&reminder.due_at)
        || !identifier(&reminder.expires_at)
        || !reminder.local_only
        || (reminder.acknowledged_at.is_some() && reminder.dismissed_at.is_some())
    {
        return Err(ObligationError::Invalid);
    }
    Ok(())
}

pub fn admit_communication_draft(
    draft: &ApprovedCommunicationDraft,
) -> Result<(), ObligationError> {
    if !identifier(&draft.draft_id)
        || !identifier(&draft.obligation_id)
        || !identifier(&draft.exact_recipient_id)
        || !digest(&draft.content_sha256)
        || !identifier(&draft.approval_id)
        || !draft.approval_fresh
        || draft.external_effect
    {
        return Err(ObligationError::Unapproved);
    }
    Ok(())
}

pub fn treat_external_content_as_data(_content: &str) -> Result<(), ObligationError> {
    Err(ObligationError::UntrustedContent)
}

pub fn reject_financial_or_account_effect(_request: &str) -> Result<(), ObligationError> {
    Err(ObligationError::UnsupportedEffect)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::financial_domain::Currency;

    fn obligation(method: DetectionMethod) -> RecurringObligation {
        let currency = Currency::new("USD").unwrap();
        RecurringObligation {
            obligation_id: "obligation".into(),
            kind: ObligationKind::Subscription,
            merchant_or_payee_id: "merchant".into(),
            recurrence_rule: "monthly".into(),
            cadence_days_min: 28,
            cadence_days_max: 31,
            expected_amount_min: Money::new(900, currency.clone(), 2).unwrap(),
            expected_amount_max: Money::new(1100, currency, 2).unwrap(),
            due_window_start: "2026-09-01".into(),
            due_window_end: "2026-09-05".into(),
            confidence_basis_points: if method == DetectionMethod::Deterministic {
                10_000
            } else {
                7_500
            },
            financial_source_ids: BTreeSet::from(["transaction".into()]),
            communication_source_ids: BTreeSet::from(["renewal".into()]),
            limitations: if method == DetectionMethod::Deterministic {
                BTreeSet::new()
            } else {
                BTreeSet::from(["model-assisted".into()])
            },
            method,
            status: if method == DetectionMethod::Deterministic {
                ObligationStatus::Confirmed
            } else {
                ObligationStatus::Candidate
            },
            user_disposition: if method == DetectionMethod::Deterministic {
                UserDisposition::Accepted
            } else {
                UserDisposition::Unresolved
            },
            reversible: true,
            authority: false,
        }
    }
    #[test]
    fn deterministic_and_model_assisted_states_are_separate() {
        assert_eq!(
            validate_detection(&obligation(DetectionMethod::Deterministic)),
            Ok(())
        );
        assert_eq!(
            validate_detection(&obligation(DetectionMethod::ModelAssisted)),
            Ok(())
        );
    }
    #[test]
    fn model_output_cannot_confirm_or_create_authority() {
        let mut value = obligation(DetectionMethod::ModelAssisted);
        value.status = ObligationStatus::Confirmed;
        assert_eq!(validate_detection(&value), Err(ObligationError::Ambiguous));
        value.status = ObligationStatus::Candidate;
        value.authority = true;
        assert_eq!(validate_detection(&value), Err(ObligationError::Invalid));
    }
    #[test]
    fn only_fresh_zero_effect_communication_drafts_are_admitted() {
        let mut draft = ApprovedCommunicationDraft {
            draft_id: "draft".into(),
            obligation_id: "obligation".into(),
            exact_recipient_id: "recipient".into(),
            content_sha256: "a".repeat(64),
            approval_id: "approval".into(),
            approval_fresh: true,
            external_effect: false,
        };
        assert_eq!(admit_communication_draft(&draft), Ok(()));
        draft.external_effect = true;
        assert_eq!(
            admit_communication_draft(&draft),
            Err(ObligationError::Unapproved)
        );
    }
    #[test]
    fn hostile_content_and_financial_effects_are_rejected() {
        assert_eq!(
            treat_external_content_as_data("cancel now"),
            Err(ObligationError::UntrustedContent)
        );
        for request in ["cancel service", "send funds", "change account"] {
            assert_eq!(
                reject_financial_or_account_effect(request),
                Err(ObligationError::UnsupportedEffect)
            );
        }
    }
}
