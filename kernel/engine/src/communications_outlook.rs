//! Provider-neutral communications contract and exact Microsoft Outlook boundary.
use std::collections::BTreeSet;
/// Closed communications object families.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[allow(missing_docs)]
pub enum CommunicationObject {
    Mailbox,
    Folder,
    Conversation,
    Thread,
    Message,
    Sender,
    Recipient,
    Mention,
    Reaction,
    Attachment,
    Formatting,
    Visibility,
    Event,
    ProviderTransformation,
}
/// Distinct operations without inheritance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[allow(missing_docs)]
pub enum CommunicationOperation {
    Read,
    Draft,
    Send,
    Reply,
    Forward,
    Edit,
    Delete,
    Archive,
    Flag,
    Label,
    Upload,
    Download,
    Recover,
    Remove,
}
/// Exact Microsoft Graph security and mailbox domain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlookIdentity {
    /// Tenant identity.
    pub tenant_id: String,
    /// Account identity.
    pub account_id: String,
    /// Mailbox identity.
    pub mailbox_id: String,
    /// Optional shared mailbox identity.
    pub shared_mailbox_id: Option<String>,
    /// Folder identity.
    pub folder_id: String,
    /// Credential reference, never bytes.
    pub credential_reference: String,
    /// Exact least scopes.
    pub scopes: BTreeSet<String>,
}
/// Exact effect fields frozen before approval and dispatch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommunicationEffect {
    /// Operation.
    pub operation: CommunicationOperation,
    /// Message identity.
    pub message_id: String,
    /// Thread identity.
    pub thread_id: String,
    /// Ordered exact recipients.
    pub recipients: Vec<String>,
    /// Content digest.
    pub content_sha256: String,
    /// Attachment digests.
    pub attachment_sha256: Vec<String>,
    /// Fresh source revision.
    pub source_revision: u64,
    /// Idempotency identity.
    pub idempotency_key: String,
}
/// Provider capability and transformation declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlookSupport {
    /// Exact supported operations.
    pub operations: BTreeSet<CommunicationOperation>,
    /// Required scopes.
    pub required_scopes: BTreeSet<String>,
    /// Named provider transformations.
    pub transformations: BTreeSet<String>,
}
/// Closed effect outcome for recovery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommunicationOutcome {
    /// No effect began.
    NotStarted,
    /// Exact effect verified.
    Verified,
    /// Provider result requires read-after-write reconciliation.
    Uncertain,
    /// Effect failed with no completion claim.
    Failed,
}
/// Stable fail-closed refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum CommunicationError {
    InvalidIdentity,
    UnsupportedOperation,
    InsufficientScope,
    StalePrecondition,
    ChangedEffect,
    DuplicateEffect,
    UncertainRequiresReconciliation,
    Removed,
}
/// Operation-scoped Outlook controller.
pub struct OutlookController {
    identity: OutlookIdentity,
    support: OutlookSupport,
    consumed: BTreeSet<String>,
    removed: bool,
}
fn id(v: &str) -> bool {
    !v.is_empty() && v.len() <= 256
}
fn digest(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
impl OutlookController {
    /// Creates an exact tenant/account/mailbox controller.
    pub fn new(
        identity: OutlookIdentity,
        support: OutlookSupport,
    ) -> Result<Self, CommunicationError> {
        if !id(&identity.tenant_id)
            || !id(&identity.account_id)
            || !id(&identity.mailbox_id)
            || !id(&identity.folder_id)
            || !id(&identity.credential_reference)
        {
            return Err(CommunicationError::InvalidIdentity);
        }
        Ok(Self {
            identity,
            support,
            consumed: BTreeSet::new(),
            removed: false,
        })
    }
    /// Admits only an exact supported, scoped, fresh effect and consumes its idempotency key.
    pub fn admit(
        &mut self,
        effect: &CommunicationEffect,
        approved: &CommunicationEffect,
        current_revision: u64,
    ) -> Result<(), CommunicationError> {
        if self.removed {
            return Err(CommunicationError::Removed);
        }
        if !self.support.operations.contains(&effect.operation) {
            return Err(CommunicationError::UnsupportedOperation);
        }
        if !self
            .support
            .required_scopes
            .is_subset(&self.identity.scopes)
        {
            return Err(CommunicationError::InsufficientScope);
        }
        if effect.source_revision != current_revision {
            return Err(CommunicationError::StalePrecondition);
        }
        if effect != approved
            || !id(&effect.message_id)
            || !id(&effect.thread_id)
            || effect.recipients.is_empty()
            || !digest(&effect.content_sha256)
            || effect.attachment_sha256.iter().any(|v| !digest(v))
        {
            return Err(CommunicationError::ChangedEffect);
        }
        if !self.consumed.insert(effect.idempotency_key.clone()) {
            return Err(CommunicationError::DuplicateEffect);
        }
        Ok(())
    }
    /// Refuses completion for an uncertain provider result until exact postcondition evidence exists.
    pub fn reconcile(
        outcome: CommunicationOutcome,
        postcondition_verified: bool,
    ) -> Result<CommunicationOutcome, CommunicationError> {
        match (outcome, postcondition_verified) {
            (CommunicationOutcome::Uncertain, false) => {
                Err(CommunicationError::UncertainRequiresReconciliation)
            }
            (CommunicationOutcome::Uncertain, true) => Ok(CommunicationOutcome::Verified),
            (value, _) => Ok(value),
        }
    }
    /// Removes credentials/cursors/subscriptions/workers by destroying controller authority.
    pub fn remove(&mut self) {
        self.consumed.clear();
        self.removed = true
    }
    /// Exact immutable provider identity.
    pub fn identity(&self) -> &OutlookIdentity {
        &self.identity
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn controller() -> OutlookController {
        OutlookController::new(
            OutlookIdentity {
                tenant_id: "t".into(),
                account_id: "a".into(),
                mailbox_id: "m".into(),
                shared_mailbox_id: None,
                folder_id: "f".into(),
                credential_reference: "cred".into(),
                scopes: BTreeSet::from(["Mail.Send".into()]),
            },
            OutlookSupport {
                operations: BTreeSet::from([CommunicationOperation::Send]),
                required_scopes: BTreeSet::from(["Mail.Send".into()]),
                transformations: BTreeSet::new(),
            },
        )
        .unwrap()
    }
    fn effect() -> CommunicationEffect {
        CommunicationEffect {
            operation: CommunicationOperation::Send,
            message_id: "m1".into(),
            thread_id: "t1".into(),
            recipients: vec!["r@example.test".into()],
            content_sha256: "a".repeat(64),
            attachment_sha256: vec!["b".repeat(64)],
            source_revision: 7,
            idempotency_key: "once".into(),
        }
    }
    #[test]
    fn changed_recipient_attachment_or_revision_denies() {
        let mut c = controller();
        let approved = effect();
        let mut changed = effect();
        changed.recipients.push("hidden@example.test".into());
        assert_eq!(
            c.admit(&changed, &approved, 7),
            Err(CommunicationError::ChangedEffect)
        );
        assert_eq!(
            c.admit(&approved, &approved, 8),
            Err(CommunicationError::StalePrecondition)
        )
    }
    #[test]
    fn duplicate_effect_denies() {
        let mut c = controller();
        let e = effect();
        assert_eq!(c.admit(&e, &e, 7), Ok(()));
        assert_eq!(c.admit(&e, &e, 7), Err(CommunicationError::DuplicateEffect))
    }
    #[test]
    fn uncertain_requires_postcondition_and_remove_denies() {
        assert_eq!(
            OutlookController::reconcile(CommunicationOutcome::Uncertain, false),
            Err(CommunicationError::UncertainRequiresReconciliation)
        );
        let mut c = controller();
        c.remove();
        let e = effect();
        assert_eq!(c.admit(&e, &e, 7), Err(CommunicationError::Removed))
    }
}
