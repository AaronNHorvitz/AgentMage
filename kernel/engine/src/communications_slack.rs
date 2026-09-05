//! Workspace-, membership-, visibility-, and effect-exact Slack contracts.
use std::collections::BTreeSet;

/// Explicit Slack operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SlackOperation {
    /// Read bounded history.
    History,
    /// Read a thread.
    Thread,
    /// Post a message or reply.
    Post,
    /// Add or remove a reaction.
    React,
    /// Upload a file.
    File,
    /// Edit a message.
    Edit,
    /// Delete a message.
    Delete,
    /// Receive events.
    Event,
}

/// Exact workspace/account capability and authority profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlackProfile {
    /// Workspace identity.
    pub workspace_id: String,
    /// Enterprise identity, if any.
    pub enterprise_id: Option<String>,
    /// Account identity.
    pub account_id: String,
    /// Opaque token reference.
    pub token_reference: String,
    /// Exact scopes.
    pub scopes: BTreeSet<String>,
    /// Membership revision.
    pub membership_revision: u64,
    /// Supported operations.
    pub operations: BTreeSet<SlackOperation>,
    /// Event subscription identity.
    pub event_subscription_id: Option<String>,
    /// Pagination ceiling.
    pub page_limit: u32,
    /// Provider rate tier.
    pub rate_tier: u8,
}

/// Frozen destination, membership, visibility, content, mention, and file effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlackEffect {
    /// Operation.
    pub operation: SlackOperation,
    /// Workspace identity.
    pub workspace_id: String,
    /// Channel, direct-message, or group-message identity.
    pub destination_id: String,
    /// Channel type.
    pub destination_type: String,
    /// Thread timestamp or root identity.
    pub thread_id: Option<String>,
    /// Message identity.
    pub message_id: String,
    /// Membership revision.
    pub membership_revision: u64,
    /// Exact visibility, including shared/external state.
    pub visibility: String,
    /// Expanded member mentions.
    pub mentions: Vec<String>,
    /// Content digest.
    pub content_sha256: String,
    /// File digests.
    pub file_sha256: Vec<String>,
    /// Fresh target revision.
    pub target_revision: u64,
    /// One-use receipt key.
    pub idempotency_key: String,
}

/// Visible event and pagination state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlackSyncState {
    /// Cursor identity.
    pub cursor: Option<String>,
    /// Last event identity.
    pub event_id: Option<String>,
    /// Whether synchronized state is complete.
    pub complete: bool,
    /// Visible degradation reason.
    pub reason: String,
}

/// Stable fail-closed refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlackError {
    /// Profile identity is malformed.
    InvalidProfile,
    /// Operation is unsupported by scope, membership, or provider.
    Unsupported,
    /// Frozen effect changed.
    ChangedEffect,
    /// Workspace or membership differs.
    WrongAuthority,
    /// Target revision is stale.
    StaleTarget,
    /// Receipt or event was replayed.
    Replay,
    /// Result is rate-limited or otherwise uncertain.
    Uncertain,
    /// Adapter authority was removed.
    Removed,
}

/// Exact workspace Slack controller.
pub struct SlackController {
    profile: SlackProfile,
    consumed: BTreeSet<String>,
    events: BTreeSet<String>,
    sync: SlackSyncState,
    removed: bool,
}

fn id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

impl SlackController {
    /// Creates one exact workspace and token authority boundary.
    pub fn new(profile: SlackProfile) -> Result<Self, SlackError> {
        if !id(&profile.workspace_id)
            || !id(&profile.account_id)
            || !id(&profile.token_reference)
            || profile.enterprise_id.as_deref().is_some_and(|v| !id(v))
        {
            return Err(SlackError::InvalidProfile);
        }
        Ok(Self {
            profile,
            consumed: BTreeSet::new(),
            events: BTreeSet::new(),
            sync: SlackSyncState {
                cursor: None,
                event_id: None,
                complete: false,
                reason: "initial_sync".into(),
            },
            removed: false,
        })
    }

    /// Returns only exact workspace- and membership-supported operations.
    pub fn discover(&self, membership_present: bool) -> BTreeSet<SlackOperation> {
        if self.removed || !membership_present || self.profile.scopes.is_empty() {
            BTreeSet::new()
        } else {
            self.profile.operations.clone()
        }
    }

    /// Admits a byte-equivalent preview against current membership and target state.
    pub fn admit(
        &mut self,
        effect: &SlackEffect,
        preview: &SlackEffect,
        current_membership: u64,
        current_target: u64,
    ) -> Result<(), SlackError> {
        if self.removed {
            return Err(SlackError::Removed);
        }
        if effect.workspace_id != self.profile.workspace_id
            || effect.membership_revision != self.profile.membership_revision
            || effect.membership_revision != current_membership
        {
            return Err(SlackError::WrongAuthority);
        }
        if !self.profile.operations.contains(&effect.operation) {
            return Err(SlackError::Unsupported);
        }
        if effect != preview
            || !id(&effect.destination_id)
            || !id(&effect.destination_type)
            || effect.thread_id.as_deref().is_some_and(|v| !id(v))
            || !id(&effect.message_id)
            || !id(&effect.visibility)
            || effect.mentions.iter().any(|v| !id(v))
            || !digest(&effect.content_sha256)
            || effect.file_sha256.iter().any(|v| !digest(v))
        {
            return Err(SlackError::ChangedEffect);
        }
        if effect.target_revision != current_target {
            return Err(SlackError::StaleTarget);
        }
        if !self.consumed.insert(effect.idempotency_key.clone()) {
            return Err(SlackError::Replay);
        }
        Ok(())
    }

    /// Accepts an event once and makes cursor/event completeness visible.
    pub fn accept_event(&mut self, event_id: &str, cursor: &str) -> Result<(), SlackError> {
        if self.removed {
            return Err(SlackError::Removed);
        }
        if !id(event_id) || !id(cursor) || !self.events.insert(event_id.into()) {
            self.sync.complete = false;
            self.sync.reason = "event_replay_or_invalid".into();
            return Err(SlackError::Replay);
        }
        self.sync.event_id = Some(event_id.into());
        self.sync.cursor = Some(cursor.into());
        self.sync.complete = true;
        self.sync.reason = "current".into();
        Ok(())
    }

    /// Exposes rate, timeout, outage, or permission-loss uncertainty.
    pub fn mark_uncertain(&mut self, reason: &str) -> Result<(), SlackError> {
        self.sync.complete = false;
        self.sync.reason = reason.into();
        Err(SlackError::Uncertain)
    }

    /// Revokes subscriptions, tokens, caches, cursors, workers, and retained authority.
    pub fn remove(&mut self) {
        self.profile.operations.clear();
        self.profile.scopes.clear();
        self.profile.token_reference.clear();
        self.profile.event_subscription_id = None;
        self.consumed.clear();
        self.events.clear();
        self.sync.cursor = None;
        self.sync.event_id = None;
        self.sync.complete = false;
        self.sync.reason = "removed".into();
        self.removed = true;
    }

    /// Visible synchronization truth.
    pub fn sync_state(&self) -> &SlackSyncState {
        &self.sync
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn controller() -> SlackController {
        SlackController::new(SlackProfile {
            workspace_id: "workspace".into(),
            enterprise_id: Some("enterprise".into()),
            account_id: "account".into(),
            token_reference: "token".into(),
            scopes: BTreeSet::from(["chat:write".into()]),
            membership_revision: 3,
            operations: BTreeSet::from([SlackOperation::Post]),
            event_subscription_id: Some("events".into()),
            page_limit: 100,
            rate_tier: 2,
        })
        .unwrap()
    }
    fn effect() -> SlackEffect {
        SlackEffect {
            operation: SlackOperation::Post,
            workspace_id: "workspace".into(),
            destination_id: "channel".into(),
            destination_type: "public_channel".into(),
            thread_id: Some("thread".into()),
            message_id: "message".into(),
            membership_revision: 3,
            visibility: "workspace".into(),
            mentions: vec!["member".into()],
            content_sha256: "a".repeat(64),
            file_sha256: vec!["b".repeat(64)],
            target_revision: 7,
            idempotency_key: "once".into(),
        }
    }
    #[test]
    fn exact_workspace_effect_and_replay_are_enforced() {
        let mut c = controller();
        let p = effect();
        let mut changed = effect();
        changed.mentions.push("everyone".into());
        assert_eq!(c.admit(&changed, &p, 3, 7), Err(SlackError::ChangedEffect));
        assert_eq!(c.admit(&p, &p, 3, 7), Ok(()));
        assert_eq!(c.admit(&p, &p, 3, 7), Err(SlackError::Replay));
    }
    #[test]
    fn events_are_once_only_and_uncertainty_visible() {
        let mut c = controller();
        assert_eq!(c.accept_event("event", "cursor"), Ok(()));
        assert_eq!(c.accept_event("event", "cursor"), Err(SlackError::Replay));
        assert!(!c.sync_state().complete);
    }
    #[test]
    fn membership_loss_and_removal_register_nothing() {
        let mut c = controller();
        assert!(c.discover(false).is_empty());
        c.remove();
        assert!(c.discover(true).is_empty());
        assert_eq!(c.sync_state().reason, "removed");
    }
}
