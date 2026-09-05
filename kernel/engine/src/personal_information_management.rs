//! Exact calendar, contact, task, and confirmed-UI Proton Calendar contracts.
#![allow(missing_docs)]
use std::collections::BTreeSet;

/// Personal-information object family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PimObjectKind {
    Event,
    Availability,
    Contact,
    Task,
}

/// Semantically distinct PIM operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PimOperation {
    Create,
    Update,
    Move,
    Invite,
    Respond,
    Cancel,
    Merge,
    Assign,
    Complete,
    Reopen,
    Delete,
    Read,
}

/// Exact PIM object and provider state frozen by preview.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PimEffect {
    pub kind: PimObjectKind,
    pub operation: PimOperation,
    pub provider_id: String,
    pub account_id: String,
    pub container_id: String,
    pub object_id: String,
    pub recurrence_master_id: Option<String>,
    pub recurrence_instance_id: Option<String>,
    pub time_zone: String,
    pub start: Option<String>,
    pub end: Option<String>,
    pub recurrence: Option<String>,
    pub attendees: Vec<String>,
    pub attendee_state: Vec<String>,
    pub reminders: Vec<String>,
    pub resources: Vec<String>,
    pub assignments: Vec<String>,
    pub dependencies: Vec<String>,
    pub due_date: Option<String>,
    pub status: String,
    pub visibility: String,
    pub conflict_revision: u64,
    pub permission_revision: u64,
    pub transformation_id: Option<String>,
    pub expected_postcondition_sha256: String,
    pub idempotency_key: String,
}

/// Exact provider capability result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PimCapabilities {
    pub provider_id: String,
    pub account_id: String,
    pub operations: BTreeSet<PimOperation>,
    pub limitations: BTreeSet<String>,
    pub transformations: BTreeSet<String>,
}

/// Stable PIM refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PimError {
    Invalid,
    Unsupported,
    ChangedEffect,
    StaleConflict,
    Duplicate,
    Unknown,
    Removed,
}

/// Capability-detected PIM controller.
pub struct PimController {
    capabilities: PimCapabilities,
    consumed: BTreeSet<String>,
    removed: bool,
}

fn id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 1024
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

impl PimController {
    pub fn new(capabilities: PimCapabilities) -> Result<Self, PimError> {
        if !id(&capabilities.provider_id) || !id(&capabilities.account_id) {
            return Err(PimError::Invalid);
        }
        Ok(Self {
            capabilities,
            consumed: BTreeSet::new(),
            removed: false,
        })
    }
    pub fn discover(&self) -> BTreeSet<PimOperation> {
        if self.removed {
            BTreeSet::new()
        } else {
            self.capabilities.operations.clone()
        }
    }
    pub fn admit(
        &mut self,
        effect: &PimEffect,
        preview: &PimEffect,
        current_conflict: u64,
        current_permission: u64,
    ) -> Result<(), PimError> {
        if self.removed {
            return Err(PimError::Removed);
        }
        if !self.capabilities.operations.contains(&effect.operation) {
            return Err(PimError::Unsupported);
        }
        if effect != preview
            || effect.provider_id != self.capabilities.provider_id
            || effect.account_id != self.capabilities.account_id
            || !id(&effect.container_id)
            || !id(&effect.object_id)
            || !id(&effect.time_zone)
            || !id(&effect.status)
            || !id(&effect.visibility)
            || !digest(&effect.expected_postcondition_sha256)
        {
            return Err(PimError::ChangedEffect);
        }
        if effect.conflict_revision != current_conflict
            || effect.permission_revision != current_permission
        {
            return Err(PimError::StaleConflict);
        }
        if !self.consumed.insert(effect.idempotency_key.clone()) {
            return Err(PimError::Duplicate);
        }
        Ok(())
    }
    pub fn reconcile(&self, proved_effect: Option<bool>) -> Result<bool, PimError> {
        proved_effect.ok_or(PimError::Unknown)
    }
    pub fn remove(&mut self) {
        self.capabilities.operations.clear();
        self.consumed.clear();
        self.removed = true
    }
}

/// Closed email-first response states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtonResponse {
    Affirmative,
    Negative,
    Tentative,
    Alternative,
    Ambiguous,
    Conflicting,
    Stale,
    Superseded,
    IdentityUncertain,
}

/// Exact confirmed-UI Proton surface without secret material.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtonCalendarSurface {
    pub origin: String,
    pub interface_version: String,
    pub authenticated_profile_reference: String,
    pub account_id: String,
    pub calendar_id: String,
    pub visible_foreground: bool,
    pub structured_controls: bool,
    pub accessibility_controls: bool,
    pub visual_fallback_declared: bool,
    pub session_current: bool,
}

/// One exact Proton event/invitation preview and grant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtonCalendarEffect {
    pub surface: ProtonCalendarSurface,
    pub title: String,
    pub start: String,
    pub end: String,
    pub time_zone: String,
    pub recurrence: Option<String>,
    pub location: String,
    pub description_sha256: String,
    pub attendees: Vec<String>,
    pub notifications: Vec<String>,
    pub policy_revision: u64,
    pub provider_revision: u64,
    pub grant_id: String,
    pub intent_persisted: bool,
}

/// Confirmed-UI refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtonCalendarError {
    InvalidSurface,
    ChangedEffect,
    SessionUnavailable,
    GrantConsumed,
    ReviewRequired,
    Unknown,
    Removed,
}

/// Operation-scoped Proton Calendar confirmed-UI controller.
pub struct ProtonCalendarController {
    origin: String,
    consumed: BTreeSet<String>,
    removed: bool,
}

impl ProtonCalendarController {
    pub fn new(origin: &str) -> Result<Self, ProtonCalendarError> {
        if origin != "https://calendar.proton.me" {
            return Err(ProtonCalendarError::InvalidSurface);
        }
        Ok(Self {
            origin: origin.into(),
            consumed: BTreeSet::new(),
            removed: false,
        })
    }
    pub fn classify_response(
        &self,
        response: ProtonResponse,
        correlated_identity: bool,
        current: bool,
    ) -> Result<(), ProtonCalendarError> {
        if response == ProtonResponse::Affirmative && correlated_identity && current {
            Ok(())
        } else {
            Err(ProtonCalendarError::ReviewRequired)
        }
    }
    pub fn admit(
        &mut self,
        effect: &ProtonCalendarEffect,
        preview: &ProtonCalendarEffect,
        current_policy: u64,
        current_provider: u64,
    ) -> Result<(), ProtonCalendarError> {
        if self.removed {
            return Err(ProtonCalendarError::Removed);
        }
        if effect.surface.origin != self.origin
            || !effect.surface.visible_foreground
            || !effect.surface.session_current
            || (!effect.surface.structured_controls && !effect.surface.accessibility_controls)
            || !id(&effect.surface.authenticated_profile_reference)
        {
            return Err(ProtonCalendarError::SessionUnavailable);
        }
        if effect != preview
            || effect.policy_revision != current_policy
            || effect.provider_revision != current_provider
            || !digest(&effect.description_sha256)
            || !effect.intent_persisted
        {
            return Err(ProtonCalendarError::ChangedEffect);
        }
        if !self.consumed.insert(effect.grant_id.clone()) {
            return Err(ProtonCalendarError::GrantConsumed);
        }
        Ok(())
    }
    pub fn reconcile(&self, verified: Option<bool>) -> Result<bool, ProtonCalendarError> {
        verified.ok_or(ProtonCalendarError::Unknown)
    }
    pub fn remove(&mut self) {
        self.consumed.clear();
        self.removed = true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pim() -> (PimController, PimEffect) {
        let c = PimController::new(PimCapabilities {
            provider_id: "google".into(),
            account_id: "a".into(),
            operations: BTreeSet::from([PimOperation::Invite]),
            limitations: BTreeSet::new(),
            transformations: BTreeSet::new(),
        })
        .unwrap();
        let e = PimEffect {
            kind: PimObjectKind::Event,
            operation: PimOperation::Invite,
            provider_id: "google".into(),
            account_id: "a".into(),
            container_id: "cal".into(),
            object_id: "event".into(),
            recurrence_master_id: None,
            recurrence_instance_id: None,
            time_zone: "UTC".into(),
            start: Some("start".into()),
            end: Some("end".into()),
            recurrence: None,
            attendees: vec!["r@example.test".into()],
            attendee_state: vec!["needs_action".into()],
            reminders: vec![],
            resources: vec![],
            assignments: vec![],
            dependencies: vec![],
            due_date: None,
            status: "confirmed".into(),
            visibility: "private".into(),
            conflict_revision: 1,
            permission_revision: 2,
            transformation_id: None,
            expected_postcondition_sha256: "a".repeat(64),
            idempotency_key: "once".into(),
        };
        (c, e)
    }
    fn proton() -> ProtonCalendarEffect {
        ProtonCalendarEffect {
            surface: ProtonCalendarSurface {
                origin: "https://calendar.proton.me".into(),
                interface_version: "v1".into(),
                authenticated_profile_reference: "profile".into(),
                account_id: "a".into(),
                calendar_id: "c".into(),
                visible_foreground: true,
                structured_controls: true,
                accessibility_controls: true,
                visual_fallback_declared: false,
                session_current: true,
            },
            title: "meeting".into(),
            start: "start".into(),
            end: "end".into(),
            time_zone: "UTC".into(),
            recurrence: None,
            location: "remote".into(),
            description_sha256: "b".repeat(64),
            attendees: vec!["r@example.test".into()],
            notifications: vec!["invite".into()],
            policy_revision: 1,
            provider_revision: 2,
            grant_id: "grant".into(),
            intent_persisted: true,
        }
    }
    #[test]
    fn exact_pim_effect_and_conflict_required() {
        let (mut c, e) = pim();
        assert_eq!(c.admit(&e, &e, 2, 2), Err(PimError::StaleConflict));
        assert_eq!(c.admit(&e, &e, 1, 2), Ok(()));
        assert_eq!(c.admit(&e, &e, 1, 2), Err(PimError::Duplicate));
    }
    #[test]
    fn unsupported_semantics_are_absent() {
        let (mut c, _) = pim();
        c.remove();
        assert!(c.discover().is_empty())
    }
    #[test]
    fn only_clear_correlated_affirmative_prepares_event() {
        let c = ProtonCalendarController::new("https://calendar.proton.me").unwrap();
        assert_eq!(
            c.classify_response(ProtonResponse::Affirmative, true, true),
            Ok(())
        );
        assert_eq!(
            c.classify_response(ProtonResponse::Ambiguous, true, true),
            Err(ProtonCalendarError::ReviewRequired)
        );
    }
    #[test]
    fn proton_grant_is_once_only_and_unknown_visible() {
        let mut c = ProtonCalendarController::new("https://calendar.proton.me").unwrap();
        let e = proton();
        assert_eq!(c.admit(&e, &e, 1, 2), Ok(()));
        assert_eq!(
            c.admit(&e, &e, 1, 2),
            Err(ProtonCalendarError::GrantConsumed)
        );
        assert_eq!(c.reconcile(None), Err(ProtonCalendarError::Unknown));
    }
}
