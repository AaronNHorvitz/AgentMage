//! Account-exact Gmail operations and completeness-aware history synchronization.
use std::collections::BTreeSet;
/// Explicit Gmail operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[allow(missing_docs)]
pub enum GmailOperation {
    Read,
    Search,
    Thread,
    Label,
    Draft,
    Send,
    Reply,
    Forward,
    Attachment,
    History,
    Push,
    Poll,
}
/// Exact Gmail account and synchronization identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GmailIdentity {
    /// Account identity.
    pub account_id: String,
    /// Mailbox identity.
    pub mailbox_id: String,
    /// Opaque OAuth credential reference.
    pub credential_reference: String,
    /// Exact admitted OAuth scopes.
    pub scopes: BTreeSet<String>,
    /// History identity.
    pub history_id: u64,
    /// Watch identity.
    pub watch_id: Option<String>,
    /// Cursor identity.
    pub cursor: Option<String>,
}
/// Exact capability declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GmailSupport {
    /// Supported operations.
    pub operations: BTreeSet<GmailOperation>,
    /// Required least scopes.
    pub required_scopes: BTreeSet<String>,
    /// Quota ceiling.
    pub quota: u64,
    /// Page ceiling.
    pub page_limit: u32,
    /// Push availability.
    pub push_supported: bool,
    /// Poll fallback availability.
    pub poll_supported: bool,
}
/// Exact message effect frozen by preview.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GmailEffect {
    /// Operation.
    pub operation: GmailOperation,
    /// Account identity.
    pub account_id: String,
    /// Message identity.
    pub message_id: String,
    /// Thread identity.
    pub thread_id: String,
    /// Exact labels.
    pub labels: BTreeSet<String>,
    /// Exact recipients.
    pub recipients: Vec<String>,
    /// Raw MIME digest.
    pub mime_sha256: String,
    /// Attachment digests.
    pub attachment_sha256: Vec<String>,
    /// Fresh draft/source revision.
    pub source_revision: u64,
    /// One-use key.
    pub idempotency_key: String,
}
/// Visible history and watch truth.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GmailSyncStatus {
    /// Complete-history marker.
    pub complete: bool,
    /// History gap marker.
    pub history_gap: bool,
    /// Watch expiry marker.
    pub watch_expired: bool,
    /// Permission reduction marker.
    pub permission_reduced: bool,
    /// Stable visible reason.
    pub reason: String,
}
/// Stable fail-closed refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum GmailError {
    InvalidIdentity,
    BroadScope,
    CrossAccount,
    Unsupported,
    ChangedEffect,
    StaleSource,
    DuplicateDelivery,
    HistoryGap,
    Removed,
}
/// Account-scoped Gmail controller.
pub struct GmailController {
    identity: GmailIdentity,
    support: GmailSupport,
    consumed: BTreeSet<String>,
    status: GmailSyncStatus,
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
impl GmailController {
    /// Creates an exact least-scope account controller.
    pub fn new(identity: GmailIdentity, support: GmailSupport) -> Result<Self, GmailError> {
        if !id(&identity.account_id)
            || !id(&identity.mailbox_id)
            || !id(&identity.credential_reference)
        {
            return Err(GmailError::InvalidIdentity);
        }
        if identity.scopes != support.required_scopes {
            return Err(GmailError::BroadScope);
        }
        Ok(Self {
            identity,
            support,
            consumed: BTreeSet::new(),
            status: GmailSyncStatus {
                complete: false,
                history_gap: false,
                watch_expired: false,
                permission_reduced: false,
                reason: "initial_sync".into(),
            },
            removed: false,
        })
    }
    /// Admits only exact account, content, recipient, thread, label, and attachment fields.
    pub fn admit(
        &mut self,
        effect: &GmailEffect,
        preview: &GmailEffect,
        current_revision: u64,
    ) -> Result<(), GmailError> {
        if self.removed {
            return Err(GmailError::Removed);
        }
        if effect.account_id != self.identity.account_id {
            return Err(GmailError::CrossAccount);
        }
        if !self.support.operations.contains(&effect.operation) {
            return Err(GmailError::Unsupported);
        }
        if effect != preview
            || !id(&effect.message_id)
            || !id(&effect.thread_id)
            || effect.recipients.is_empty()
            || !digest(&effect.mime_sha256)
            || effect.attachment_sha256.iter().any(|v| !digest(v))
        {
            return Err(GmailError::ChangedEffect);
        }
        if effect.source_revision != current_revision {
            return Err(GmailError::StaleSource);
        }
        if !self.consumed.insert(effect.idempotency_key.clone()) {
            return Err(GmailError::DuplicateDelivery);
        }
        Ok(())
    }
    /// Applies the next exact history checkpoint or exposes incompleteness.
    pub fn advance_history(&mut self, next: u64) -> Result<(), GmailError> {
        if next != self.identity.history_id + 1 {
            self.status.complete = false;
            self.status.history_gap = true;
            self.status.reason = "history_gap".into();
            return Err(GmailError::HistoryGap);
        }
        self.identity.history_id = next;
        self.status.complete = true;
        self.status.history_gap = false;
        self.status.reason = "current".into();
        Ok(())
    }
    /// Exposes watch expiry and prevents completeness claims.
    pub fn expire_watch(&mut self) {
        self.status.complete = false;
        self.status.watch_expired = true;
        self.status.reason = "watch_expired".into()
    }
    /// Removes watches, tokens, caches, cursors, schedules, workers, and retained authority.
    pub fn remove(&mut self) {
        self.identity.watch_id = None;
        self.identity.cursor = None;
        self.identity.scopes.clear();
        self.consumed.clear();
        self.removed = true;
        self.status.complete = false;
        self.status.reason = "removed".into()
    }
    /// Visible synchronization truth.
    pub fn status(&self) -> &GmailSyncStatus {
        &self.status
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn controller() -> GmailController {
        GmailController::new(
            GmailIdentity {
                account_id: "a".into(),
                mailbox_id: "m".into(),
                credential_reference: "cred".into(),
                scopes: BTreeSet::from(["gmail.send".into()]),
                history_id: 1,
                watch_id: Some("w".into()),
                cursor: Some("c".into()),
            },
            GmailSupport {
                operations: BTreeSet::from([GmailOperation::Send]),
                required_scopes: BTreeSet::from(["gmail.send".into()]),
                quota: 10,
                page_limit: 10,
                push_supported: true,
                poll_supported: true,
            },
        )
        .unwrap()
    }
    fn effect() -> GmailEffect {
        GmailEffect {
            operation: GmailOperation::Send,
            account_id: "a".into(),
            message_id: "m1".into(),
            thread_id: "t1".into(),
            labels: BTreeSet::from(["INBOX".into()]),
            recipients: vec!["r@example.test".into()],
            mime_sha256: "a".repeat(64),
            attachment_sha256: vec!["b".repeat(64)],
            source_revision: 7,
            idempotency_key: "once".into(),
        }
    }
    #[test]
    fn cross_account_and_changed_mime_deny() {
        let mut c = controller();
        let p = effect();
        let mut x = effect();
        x.account_id = "other".into();
        assert_eq!(c.admit(&x, &p, 7), Err(GmailError::CrossAccount));
        let mut x = effect();
        x.mime_sha256 = "c".repeat(64);
        assert_eq!(c.admit(&x, &p, 7), Err(GmailError::ChangedEffect))
    }
    #[test]
    fn history_gap_and_watch_expiry_are_visible() {
        let mut c = controller();
        assert_eq!(c.advance_history(3), Err(GmailError::HistoryGap));
        assert!(!c.status().complete && c.status().history_gap);
        c.expire_watch();
        assert!(c.status().watch_expired)
    }
    #[test]
    fn replay_and_removed_authority_deny() {
        let mut c = controller();
        let e = effect();
        assert_eq!(c.admit(&e, &e, 7), Ok(()));
        assert_eq!(c.admit(&e, &e, 7), Err(GmailError::DuplicateDelivery));
        c.remove();
        assert_eq!(c.admit(&e, &e, 7), Err(GmailError::Removed));
        assert!(c.identity.scopes.is_empty())
    }
}
