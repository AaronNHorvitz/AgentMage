//! Evidence-linked incidents, separately granted effects, and duplicate-safe recovery.
#![allow(missing_docs)]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    Fact,
    Inference,
    Conflict,
    Unknown,
    Decision,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IncidentEffect {
    State,
    Severity,
    Assignment,
    WorkItem,
    Message,
    Rollback,
    Flag,
    Closure,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectOutcome {
    Completed,
    Rejected,
    Unknown,
    Cancelled,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recovery {
    Complete,
    NoEffect,
    ReconcileOnly,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IncidentEvidence {
    pub incident_sha256: String,
    pub service_sha256: String,
    pub tenant_sha256: String,
    pub timeline_sha256: String,
    pub source_sha256: String,
    pub class: EvidenceClass,
    pub content_untrusted: bool,
    pub secret_scan_passed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectPreview {
    pub incident_sha256: String,
    pub effect: IncidentEffect,
    pub provider_sha256: String,
    pub destination_sha256: String,
    pub recipients_sha256: String,
    pub membership_sha256: String,
    pub precondition_sha256: String,
    pub payload_sha256: String,
    pub idempotency_sha256: String,
    pub disclosure_warning_sha256: String,
    pub granted_effect: IncidentEffect,
    pub approval_sha256: String,
    pub reread_preconditions: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectReceipt {
    pub incident_sha256: String,
    pub effect: IncidentEffect,
    pub idempotency_sha256: String,
    pub provider_receipt_sha256: String,
    pub post_state_sha256: String,
    pub duplicate_effect_count: u32,
    pub hidden_recipient_count: u32,
    pub cross_tenant_count: u32,
    pub secret_disclosure_count: u32,
    pub outcome: EffectOutcome,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IncidentError {
    InvalidEvidence,
    InvalidPreview,
    AuthorityMismatch,
    InvalidReceipt,
}

pub fn validate_evidence(value: &IncidentEvidence) -> Result<(), IncidentError> {
    if [
        &value.incident_sha256,
        &value.service_sha256,
        &value.tenant_sha256,
        &value.timeline_sha256,
        &value.source_sha256,
    ]
    .into_iter()
    .any(|field| !valid_sha(field))
        || !value.content_untrusted
        || !value.secret_scan_passed
    {
        return Err(IncidentError::InvalidEvidence);
    }
    Ok(())
}

pub fn validate_preview(value: &EffectPreview) -> Result<(), IncidentError> {
    if [
        &value.incident_sha256,
        &value.provider_sha256,
        &value.destination_sha256,
        &value.recipients_sha256,
        &value.membership_sha256,
        &value.precondition_sha256,
        &value.payload_sha256,
        &value.idempotency_sha256,
        &value.disclosure_warning_sha256,
        &value.approval_sha256,
    ]
    .into_iter()
    .any(|field| !valid_sha(field))
        || !value.reread_preconditions
    {
        return Err(IncidentError::InvalidPreview);
    }
    if value.effect != value.granted_effect {
        return Err(IncidentError::AuthorityMismatch);
    }
    Ok(())
}

pub fn verify_receipt(
    preview: &EffectPreview,
    receipt: &EffectReceipt,
) -> Result<(), IncidentError> {
    validate_preview(preview)?;
    if receipt.incident_sha256 != preview.incident_sha256
        || receipt.effect != preview.effect
        || receipt.idempotency_sha256 != preview.idempotency_sha256
        || !valid_sha(&receipt.provider_receipt_sha256)
        || !valid_sha(&receipt.post_state_sha256)
        || receipt.duplicate_effect_count != 0
        || receipt.hidden_recipient_count != 0
        || receipt.cross_tenant_count != 0
        || receipt.secret_disclosure_count != 0
    {
        return Err(IncidentError::InvalidReceipt);
    }
    Ok(())
}

pub fn recovery(receipt: &EffectReceipt) -> Recovery {
    match receipt.outcome {
        EffectOutcome::Completed => Recovery::Complete,
        EffectOutcome::Rejected => Recovery::NoEffect,
        EffectOutcome::Unknown | EffectOutcome::Cancelled => Recovery::ReconcileOnly,
    }
}

pub fn autonomous_effect_allowed(_: IncidentEffect) -> bool {
    false
}

fn valid_sha(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn preview() -> EffectPreview {
        let h = "a".repeat(64);
        EffectPreview {
            incident_sha256: h.clone(),
            effect: IncidentEffect::Message,
            provider_sha256: h.clone(),
            destination_sha256: h.clone(),
            recipients_sha256: h.clone(),
            membership_sha256: h.clone(),
            precondition_sha256: h.clone(),
            payload_sha256: h.clone(),
            idempotency_sha256: h.clone(),
            disclosure_warning_sha256: h.clone(),
            granted_effect: IncidentEffect::Message,
            approval_sha256: h,
            reread_preconditions: true,
        }
    }
    #[test]
    fn separate_effect_grant_is_valid() {
        assert_eq!(validate_preview(&preview()), Ok(()));
    }
    #[test]
    fn authority_cannot_cross_effects() {
        let mut p = preview();
        p.granted_effect = IncidentEffect::Rollback;
        assert_eq!(validate_preview(&p), Err(IncidentError::AuthorityMismatch));
    }
    #[test]
    fn uncertain_effect_requires_reconciliation() {
        let h = "b".repeat(64);
        let r = EffectReceipt {
            incident_sha256: h.clone(),
            effect: IncidentEffect::Message,
            idempotency_sha256: h.clone(),
            provider_receipt_sha256: h.clone(),
            post_state_sha256: h,
            duplicate_effect_count: 0,
            hidden_recipient_count: 0,
            cross_tenant_count: 0,
            secret_disclosure_count: 0,
            outcome: EffectOutcome::Unknown,
        };
        assert_eq!(recovery(&r), Recovery::ReconcileOnly);
    }
    #[test]
    fn no_incident_content_can_trigger_an_effect() {
        for e in [
            IncidentEffect::State,
            IncidentEffect::Severity,
            IncidentEffect::Assignment,
            IncidentEffect::WorkItem,
            IncidentEffect::Message,
            IncidentEffect::Rollback,
            IncidentEffect::Flag,
            IncidentEffect::Closure,
        ] {
            assert!(!autonomous_effect_allowed(e));
        }
    }
}
