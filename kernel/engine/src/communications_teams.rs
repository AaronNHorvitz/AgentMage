//! Exact Microsoft Teams capability discovery and effect admission.
use std::collections::BTreeSet;
/// Closed Teams object families.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[allow(missing_docs)]
pub enum TeamsObject {
    Tenant,
    Account,
    Team,
    Channel,
    Chat,
    Thread,
    Message,
    Reply,
    Mention,
    Reaction,
    File,
    Membership,
    Visibility,
}
/// Explicit Teams operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[allow(missing_docs)]
pub enum TeamsOperation {
    Read,
    Draft,
    Reply,
    React,
    Edit,
    Delete,
}
/// Exact account and tenant capability profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TeamsProfile {
    /// Tenant identity.
    pub tenant_id: String,
    /// Account identity.
    pub account_id: String,
    /// Exact delegated scopes.
    pub delegated_scopes: BTreeSet<String>,
    /// Operations permitted by tenant/account policy.
    pub operations: BTreeSet<TeamsOperation>,
    /// Personal account exclusion.
    pub personal_account: bool,
    /// Event support.
    pub events_supported: bool,
    /// Edit window seconds.
    pub edit_window_seconds: u64,
    /// Delete window seconds.
    pub delete_window_seconds: u64,
    /// Visible degradation reason.
    pub degradation: Option<String>,
}
/// Exact preview and effect fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TeamsEffect {
    /// Operation.
    pub operation: TeamsOperation,
    /// Tenant identity.
    pub tenant_id: String,
    /// Team or chat destination.
    pub destination_id: String,
    /// Thread identity.
    pub thread_id: String,
    /// Membership revision.
    pub membership_revision: u64,
    /// Visibility class.
    pub visibility: String,
    /// Content digest.
    pub content_sha256: String,
    /// Mention identities after expansion.
    pub mentions: Vec<String>,
    /// File digests.
    pub file_sha256: Vec<String>,
    /// Target source revision.
    pub target_revision: u64,
    /// One-use effect identity.
    pub idempotency_key: String,
}
/// Stable denial reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum TeamsError {
    InvalidProfile,
    Unsupported,
    PersonalAccount,
    ChangedEffect,
    StaleTarget,
    DuplicateEffect,
    Removed,
}
/// Operation-scoped Teams admission controller.
pub struct TeamsController {
    profile: TeamsProfile,
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
impl TeamsController {
    /// Creates a controller from one exact tenant/account profile.
    pub fn new(profile: TeamsProfile) -> Result<Self, TeamsError> {
        if !id(&profile.tenant_id) || !id(&profile.account_id) {
            return Err(TeamsError::InvalidProfile);
        }
        Ok(Self {
            profile,
            consumed: BTreeSet::new(),
            removed: false,
        })
    }
    /// Returns only operations actually supported by the exact profile.
    pub fn discover(&self) -> BTreeSet<TeamsOperation> {
        if self.removed || self.profile.personal_account {
            BTreeSet::new()
        } else {
            self.profile.operations.clone()
        }
    }
    /// Admits only byte-equivalent preview/effect fields against fresh membership and target state.
    pub fn admit(
        &mut self,
        effect: &TeamsEffect,
        preview: &TeamsEffect,
        current_membership: u64,
        current_target: u64,
    ) -> Result<(), TeamsError> {
        if self.removed {
            return Err(TeamsError::Removed);
        }
        if self.profile.personal_account {
            return Err(TeamsError::PersonalAccount);
        }
        if !self.profile.operations.contains(&effect.operation) {
            return Err(TeamsError::Unsupported);
        }
        if effect != preview
            || effect.tenant_id != self.profile.tenant_id
            || !id(&effect.destination_id)
            || !id(&effect.thread_id)
            || !digest(&effect.content_sha256)
            || effect.mentions.iter().any(|v| !id(v))
            || effect.file_sha256.iter().any(|v| !digest(v))
        {
            return Err(TeamsError::ChangedEffect);
        }
        if effect.membership_revision != current_membership
            || effect.target_revision != current_target
        {
            return Err(TeamsError::StaleTarget);
        }
        if !self.consumed.insert(effect.idempotency_key.clone()) {
            return Err(TeamsError::DuplicateEffect);
        }
        Ok(())
    }
    /// Revokes and removes all operation and event authority.
    pub fn remove(&mut self) {
        self.consumed.clear();
        self.removed = true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn controller() -> TeamsController {
        TeamsController::new(TeamsProfile {
            tenant_id: "tenant".into(),
            account_id: "account".into(),
            delegated_scopes: BTreeSet::from(["Chat.ReadWrite".into()]),
            operations: BTreeSet::from([TeamsOperation::Reply]),
            personal_account: false,
            events_supported: true,
            edit_window_seconds: 100,
            delete_window_seconds: 100,
            degradation: None,
        })
        .unwrap()
    }
    fn effect() -> TeamsEffect {
        TeamsEffect {
            operation: TeamsOperation::Reply,
            tenant_id: "tenant".into(),
            destination_id: "channel".into(),
            thread_id: "thread".into(),
            membership_revision: 3,
            visibility: "team".into(),
            content_sha256: "a".repeat(64),
            mentions: vec!["member".into()],
            file_sha256: vec!["b".repeat(64)],
            target_revision: 7,
            idempotency_key: "once".into(),
        }
    }
    #[test]
    fn exact_preview_and_membership_required() {
        let mut c = controller();
        let p = effect();
        let mut changed = effect();
        changed.mentions.push("hidden".into());
        assert_eq!(c.admit(&changed, &p, 3, 7), Err(TeamsError::ChangedEffect));
        assert_eq!(c.admit(&p, &p, 4, 7), Err(TeamsError::StaleTarget))
    }
    #[test]
    fn replay_denies() {
        let mut c = controller();
        let e = effect();
        assert_eq!(c.admit(&e, &e, 3, 7), Ok(()));
        assert_eq!(c.admit(&e, &e, 3, 7), Err(TeamsError::DuplicateEffect))
    }
    #[test]
    fn personal_and_removed_profiles_register_nothing() {
        let mut c = controller();
        c.profile.personal_account = true;
        assert!(c.discover().is_empty());
        let mut c = controller();
        c.remove();
        assert!(c.discover().is_empty())
    }
}
