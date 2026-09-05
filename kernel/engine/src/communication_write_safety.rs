//! Cross-provider exact communication preview, approval, intent, and reconciliation.
use std::collections::BTreeSet;

/// Operations remain distinct through preview, effect, and receipt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CommunicationOperation {
    /// Read.
    Read,
    /// Draft.
    Draft,
    /// Send.
    Send,
    /// Reply.
    Reply,
    /// Forward.
    Forward,
    /// Edit.
    Edit,
    /// Delete.
    Delete,
    /// Reaction.
    Reaction,
    /// Upload.
    Upload,
    /// Download.
    Download,
    /// Move.
    Move,
    /// Label.
    Label,
    /// Flag.
    Flag,
    /// Archive.
    Archive,
}

/// Every field frozen before approval.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommunicationPreview {
    /// Operation.
    pub operation: CommunicationOperation,
    /// Provider identity.
    pub provider_id: String,
    /// Tenant identity.
    pub tenant_id: String,
    /// Account identity.
    pub account_id: String,
    /// Acting identity.
    pub actor_id: String,
    /// Sender identity.
    pub sender: String,
    /// Exact recipients.
    pub recipients: Vec<String>,
    /// External recipient domains.
    pub external_domains: BTreeSet<String>,
    /// Exact destination.
    pub destination_id: String,
    /// Thread identity.
    pub thread_id: String,
    /// Visibility class.
    pub visibility: String,
    /// Payload digest.
    pub payload_sha256: String,
    /// Formatting digest.
    pub formatting_sha256: String,
    /// Quote digest.
    pub quote_sha256: Option<String>,
    /// Expanded mentions.
    pub mentions: Vec<String>,
    /// Exact links.
    pub links: Vec<String>,
    /// Attachment digests.
    pub attachment_sha256: Vec<String>,
    /// Data classification.
    pub classification: String,
    /// Transformation identity.
    pub transformation_id: Option<String>,
    /// Expected postcondition digest.
    pub postcondition_sha256: String,
    /// Policy revision.
    pub policy_revision: u64,
    /// Permission revision.
    pub permission_revision: u64,
    /// Source revision.
    pub source_revision: u64,
    /// Destination/provider-state revision.
    pub provider_revision: u64,
}

/// Approval is bound to the exact preview and revisions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommunicationApproval {
    /// Approval identity.
    pub approval_id: String,
    /// Canonical preview digest.
    pub preview_sha256: String,
    /// Policy revision.
    pub policy_revision: u64,
    /// Permission revision.
    pub permission_revision: u64,
    /// Source revision.
    pub source_revision: u64,
    /// Provider revision.
    pub provider_revision: u64,
}

/// Intent persisted before any provider effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommunicationIntent {
    /// Stable intent identity.
    pub intent_id: String,
    /// Deterministic operation fingerprint/idempotency key.
    pub operation_fingerprint: String,
    /// Preview digest.
    pub preview_sha256: String,
    /// Whether durable persistence happened before dispatch.
    pub persisted: bool,
}

/// Closed result classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommunicationResult {
    /// Denied before effect.
    Denied,
    /// User or emergency cancellation.
    Cancelled,
    /// Proved failure/non-effect.
    Failed,
    /// Provider timeout.
    Timeout,
    /// Partial effect requiring reconciliation.
    Partial,
    /// Unknown effect requiring reconciliation.
    Unknown,
    /// Duplicate was detected without repeating effect.
    Duplicate,
    /// Effect and postcondition were proved.
    Succeeded,
}

/// Immutable verifier-owned receipt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommunicationReceipt {
    /// Intent identity.
    pub intent_id: String,
    /// Exact operation.
    pub operation: CommunicationOperation,
    /// Exact preview digest.
    pub preview_sha256: String,
    /// Result classification.
    pub result: CommunicationResult,
    /// Observed postcondition digest when proved.
    pub observed_postcondition_sha256: Option<String>,
}

/// Stable fail-closed refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommunicationWriteError {
    /// Identity or digest is malformed.
    Invalid,
    /// Approval does not bind the preview.
    ApprovalMismatch,
    /// Fresh policy, permission, source, destination, or provider state changed.
    StalePrecondition,
    /// Intent was not persisted before dispatch.
    IntentNotPersisted,
    /// Fingerprint was already consumed.
    Duplicate,
    /// Emergency disablement is active.
    Disabled,
    /// Unknown or partial outcome needs reconciliation.
    Uncertain,
    /// Observed postcondition differs.
    PostconditionMismatch,
}

/// Cross-provider write admission and reconciliation controller.
pub struct CommunicationWriteController {
    consumed: BTreeSet<String>,
    disabled: bool,
}

fn id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 512
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

impl CommunicationWriteController {
    /// Creates an enabled empty controller.
    pub fn new() -> Self {
        Self {
            consumed: BTreeSet::new(),
            disabled: false,
        }
    }

    /// Performs the final fresh precondition and exact-approval admission.
    pub fn submit(
        &mut self,
        preview: &CommunicationPreview,
        canonical_preview_sha256: &str,
        approval: &CommunicationApproval,
        intent: &CommunicationIntent,
        fresh_revisions: (u64, u64, u64, u64),
    ) -> Result<(), CommunicationWriteError> {
        if self.disabled {
            return Err(CommunicationWriteError::Disabled);
        }
        if !id(&approval.approval_id)
            || !id(&intent.intent_id)
            || !digest(canonical_preview_sha256)
            || !digest(&intent.operation_fingerprint)
            || !digest(&preview.payload_sha256)
            || !digest(&preview.formatting_sha256)
            || !digest(&preview.postcondition_sha256)
            || preview.attachment_sha256.iter().any(|v| !digest(v))
        {
            return Err(CommunicationWriteError::Invalid);
        }
        if approval.preview_sha256 != canonical_preview_sha256
            || intent.preview_sha256 != canonical_preview_sha256
            || (
                approval.policy_revision,
                approval.permission_revision,
                approval.source_revision,
                approval.provider_revision,
            ) != (
                preview.policy_revision,
                preview.permission_revision,
                preview.source_revision,
                preview.provider_revision,
            )
        {
            return Err(CommunicationWriteError::ApprovalMismatch);
        }
        if fresh_revisions
            != (
                preview.policy_revision,
                preview.permission_revision,
                preview.source_revision,
                preview.provider_revision,
            )
        {
            return Err(CommunicationWriteError::StalePrecondition);
        }
        if !intent.persisted {
            return Err(CommunicationWriteError::IntentNotPersisted);
        }
        if !self.consumed.insert(intent.operation_fingerprint.clone()) {
            return Err(CommunicationWriteError::Duplicate);
        }
        Ok(())
    }

    /// Produces a terminal receipt only when uncertainty and postconditions are resolved.
    pub fn reconcile(
        &self,
        preview: &CommunicationPreview,
        preview_sha256: &str,
        intent: &CommunicationIntent,
        provider_result: CommunicationResult,
        observed_postcondition_sha256: Option<&str>,
    ) -> Result<CommunicationReceipt, CommunicationWriteError> {
        if matches!(
            provider_result,
            CommunicationResult::Timeout
                | CommunicationResult::Partial
                | CommunicationResult::Unknown
        ) {
            return Err(CommunicationWriteError::Uncertain);
        }
        if provider_result == CommunicationResult::Succeeded
            && observed_postcondition_sha256 != Some(preview.postcondition_sha256.as_str())
        {
            return Err(CommunicationWriteError::PostconditionMismatch);
        }
        Ok(CommunicationReceipt {
            intent_id: intent.intent_id.clone(),
            operation: preview.operation,
            preview_sha256: preview_sha256.into(),
            result: provider_result,
            observed_postcondition_sha256: observed_postcondition_sha256.map(str::to_owned),
        })
    }

    /// Prevents any later dispatch at the emergency boundary.
    pub fn disable(&mut self) {
        self.disabled = true;
    }
}

impl Default for CommunicationWriteController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn preview() -> CommunicationPreview {
        CommunicationPreview {
            operation: CommunicationOperation::Send,
            provider_id: "p".into(),
            tenant_id: "t".into(),
            account_id: "a".into(),
            actor_id: "actor".into(),
            sender: "s@example.test".into(),
            recipients: vec!["r@example.test".into()],
            external_domains: BTreeSet::new(),
            destination_id: "d".into(),
            thread_id: "th".into(),
            visibility: "private".into(),
            payload_sha256: "a".repeat(64),
            formatting_sha256: "b".repeat(64),
            quote_sha256: None,
            mentions: vec![],
            links: vec![],
            attachment_sha256: vec!["c".repeat(64)],
            classification: "internal".into(),
            transformation_id: None,
            postcondition_sha256: "d".repeat(64),
            policy_revision: 1,
            permission_revision: 2,
            source_revision: 3,
            provider_revision: 4,
        }
    }
    fn approval(hash: &str) -> CommunicationApproval {
        CommunicationApproval {
            approval_id: "approval".into(),
            preview_sha256: hash.into(),
            policy_revision: 1,
            permission_revision: 2,
            source_revision: 3,
            provider_revision: 4,
        }
    }
    fn intent(hash: &str) -> CommunicationIntent {
        CommunicationIntent {
            intent_id: "intent".into(),
            operation_fingerprint: "e".repeat(64),
            preview_sha256: hash.into(),
            persisted: true,
        }
    }
    #[test]
    fn stale_and_changed_approval_deny() {
        let mut c = CommunicationWriteController::new();
        let p = preview();
        let h = "f".repeat(64);
        assert_eq!(
            c.submit(&p, &h, &approval(&h), &intent(&h), (1, 2, 3, 5)),
            Err(CommunicationWriteError::StalePrecondition)
        );
        let mut a = approval(&h);
        a.preview_sha256 = "0".repeat(64);
        assert_eq!(
            c.submit(&p, &h, &a, &intent(&h), (1, 2, 3, 4)),
            Err(CommunicationWriteError::ApprovalMismatch)
        );
    }
    #[test]
    fn persisted_intent_is_once_only() {
        let mut c = CommunicationWriteController::new();
        let p = preview();
        let h = "f".repeat(64);
        assert_eq!(
            c.submit(&p, &h, &approval(&h), &intent(&h), (1, 2, 3, 4)),
            Ok(())
        );
        assert_eq!(
            c.submit(&p, &h, &approval(&h), &intent(&h), (1, 2, 3, 4)),
            Err(CommunicationWriteError::Duplicate)
        );
    }
    #[test]
    fn uncertainty_never_becomes_success() {
        let c = CommunicationWriteController::new();
        let p = preview();
        let h = "f".repeat(64);
        assert_eq!(
            c.reconcile(&p, &h, &intent(&h), CommunicationResult::Unknown, None),
            Err(CommunicationWriteError::Uncertain)
        );
        assert!(
            c.reconcile(
                &p,
                &h,
                &intent(&h),
                CommunicationResult::Succeeded,
                Some(&p.postcondition_sha256)
            )
            .is_ok()
        );
    }
}
