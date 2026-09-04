//! Exact external-effect lifecycle and untrusted event reconciliation.

use std::collections::BTreeSet;

/// Closed externally observable effect outcomes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectState {
    /// Inert local draft.
    Draft,
    /// Current remote state was read.
    Refreshed,
    /// Exact effect digest is ready for review.
    Previewed,
    /// One single-use grant was consumed before submission.
    Submitted,
    /// Postcondition proves the effect.
    Effect,
    /// Reconciliation proves no effect.
    NoEffect,
    /// Provider duplicated the same logical effect.
    Duplicate,
    /// Provider applied only part of the effect.
    Partial,
    /// Current effect cannot be established.
    Unknown,
    /// Operation was denied before submission.
    Denied,
    /// Operation was cancelled before submission.
    Cancelled,
    /// Validation failed before submission.
    ValidationFailed,
}

/// Canonical effect plan with every user-visible and recovery boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalEffectPlan {
    /// Actor identity.
    pub actor: String,
    /// Exact destination identity.
    pub target: String,
    /// Immutable object identity.
    pub object: String,
    /// Payload digest.
    pub payload_sha256: String,
    /// Ordered attachment digests.
    pub attachment_sha256: Vec<String>,
    /// Visibility identity.
    pub visibility: String,
    /// Immutable source revision.
    pub source_revision: String,
    /// Environment identity.
    pub environment: String,
    /// Expected change digest.
    pub expected_change_sha256: String,
    /// Exact maximum cost in minor units.
    pub maximum_cost_minor_units: u64,
    /// Exact operation budget.
    pub operation_budget: u64,
    /// Preconditions digest.
    pub preconditions_sha256: String,
    /// Approval expiry epoch second.
    pub expires_at_epoch_seconds: u64,
    /// Recovery policy identity.
    pub recovery: String,
    /// Provider idempotency key.
    pub idempotency_key: String,
    /// Deterministic operation fingerprint.
    pub operation_fingerprint_sha256: String,
}

/// Stateful exact-effect admission and reconciliation record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalEffectLifecycle {
    /// Canonical plan.
    pub plan: ExternalEffectPlan,
    /// Current state.
    pub state: EffectState,
    /// Last refreshed remote-state digest.
    pub remote_state_sha256: Option<String>,
    /// Exact preview digest.
    pub preview_sha256: Option<String>,
    /// Whether the one grant has been consumed.
    pub grant_consumed: bool,
    /// Exactly one terminal receipt digest, when terminal.
    pub receipt_sha256: Option<String>,
}

/// Canonical untrusted provider event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderEvent {
    /// Provider event identity.
    pub event_id: String,
    /// Exact provider host.
    pub host: String,
    /// Exact tenant.
    pub tenant: String,
    /// Signature digest.
    pub signature_sha256: String,
    /// Signature-key identity for bounded rotation.
    pub signing_key_id: String,
    /// Event time as an epoch second.
    pub timestamp_epoch_seconds: u64,
    /// Replay-protection nonce.
    pub nonce: String,
    /// Monotonic provider sequence.
    pub sequence: u64,
    /// Cursor identity.
    pub cursor: String,
    /// Untrusted content digest.
    pub content_sha256: String,
    /// Tombstone state.
    pub tombstone: bool,
    /// Backfill state.
    pub backfill: bool,
}

/// Bounded event-ingestion state without operation authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventLedger {
    /// Exact provider host.
    pub host: String,
    /// Exact tenant.
    pub tenant: String,
    /// Accepted signing-key identities.
    pub signing_key_ids: BTreeSet<String>,
    /// Largest applied sequence.
    pub last_sequence: u64,
    /// Last applied cursor.
    pub last_cursor: Option<String>,
    /// Seen nonces within the replay window.
    pub seen_nonces: BTreeSet<String>,
    /// Bounded replay-window seconds.
    pub replay_window_seconds: u64,
    /// Maximum retained nonces.
    pub maximum_nonces: usize,
}

/// Observable event-ingestion dispositions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventDisposition {
    /// Event applied without authority.
    Applied,
    /// Exact duplicate ignored.
    Duplicate,
    /// Sequence gap remains visible.
    Gap,
}

/// Stable fail-closed effect/event refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalEffectError {
    /// A closed record is malformed.
    InvalidRecord,
    /// Remote state changed after review.
    StaleApproval,
    /// Transition is not legal from the current state.
    InvalidTransition,
    /// Unknown or partial effects block retry.
    ReconciliationRequired,
    /// A stale compensation would overwrite later work.
    StaleCompensation,
    /// Event signature, identity, time, nonce, sequence, or cursor is invalid.
    InvalidEvent,
    /// Event storage ceiling was reached.
    EventCapacityExceeded,
}

impl ExternalEffectLifecycle {
    /// Construct a validated inert effect draft.
    pub fn new(plan: ExternalEffectPlan) -> Result<Self, ExternalEffectError> {
        validate_plan(&plan)?;
        Ok(Self {
            plan,
            state: EffectState::Draft,
            remote_state_sha256: None,
            preview_sha256: None,
            grant_consumed: false,
            receipt_sha256: None,
        })
    }

    /// Bind one current remote read before preview.
    pub fn refresh(&mut self, remote_state_sha256: String) -> Result<(), ExternalEffectError> {
        if !matches!(self.state, EffectState::Draft | EffectState::NoEffect)
            || !valid_sha256(&remote_state_sha256)
        {
            return Err(ExternalEffectError::InvalidTransition);
        }
        self.remote_state_sha256 = Some(remote_state_sha256);
        self.state = EffectState::Refreshed;
        Ok(())
    }

    /// Bind an exact preview to current state and plan.
    pub fn preview(&mut self, preview_sha256: String) -> Result<(), ExternalEffectError> {
        if self.state != EffectState::Refreshed || !valid_sha256(&preview_sha256) {
            return Err(ExternalEffectError::InvalidTransition);
        }
        self.preview_sha256 = Some(preview_sha256);
        self.state = EffectState::Previewed;
        Ok(())
    }

    /// Re-read state, consume one grant, and admit exactly one submission.
    pub fn submit(
        &mut self,
        current_remote_sha256: &str,
        grant_fingerprint_sha256: &str,
        now: u64,
    ) -> Result<(), ExternalEffectError> {
        if self.state != EffectState::Previewed || self.grant_consumed {
            return Err(ExternalEffectError::InvalidTransition);
        }
        if self.remote_state_sha256.as_deref() != Some(current_remote_sha256)
            || grant_fingerprint_sha256 != self.plan.operation_fingerprint_sha256
            || now >= self.plan.expires_at_epoch_seconds
        {
            return Err(ExternalEffectError::StaleApproval);
        }
        self.grant_consumed = true;
        self.state = EffectState::Submitted;
        Ok(())
    }

    /// Record one reconciled terminal outcome and exactly one receipt.
    pub fn reconcile(
        &mut self,
        outcome: EffectState,
        receipt_sha256: String,
    ) -> Result<(), ExternalEffectError> {
        if self.state != EffectState::Submitted
            || !matches!(
                outcome,
                EffectState::Effect
                    | EffectState::NoEffect
                    | EffectState::Duplicate
                    | EffectState::Partial
                    | EffectState::Unknown
            )
            || !valid_sha256(&receipt_sha256)
            || self.receipt_sha256.is_some()
        {
            return Err(ExternalEffectError::InvalidTransition);
        }
        self.state = outcome;
        self.receipt_sha256 = Some(receipt_sha256);
        Ok(())
    }

    /// Permit a fresh attempt only after reconciliation proves no effect.
    pub fn retry_permitted(&self) -> Result<(), ExternalEffectError> {
        if self.state == EffectState::NoEffect {
            Ok(())
        } else {
            Err(ExternalEffectError::ReconciliationRequired)
        }
    }

    /// Require compensation to be a fresh plan that preserves later remote work.
    pub fn validate_compensation(
        &self,
        observed_remote_sha256: &str,
        effect_postcondition_sha256: &str,
        fresh_plan: &ExternalEffectPlan,
    ) -> Result<(), ExternalEffectError> {
        validate_plan(fresh_plan)?;
        if self.state != EffectState::Effect
            || observed_remote_sha256 != effect_postcondition_sha256
            || fresh_plan.operation_fingerprint_sha256 == self.plan.operation_fingerprint_sha256
        {
            Err(ExternalEffectError::StaleCompensation)
        } else {
            Ok(())
        }
    }
}

impl EventLedger {
    /// Validate and apply one untrusted event without creating authority.
    pub fn ingest(
        &mut self,
        event: &ProviderEvent,
        now: u64,
    ) -> Result<EventDisposition, ExternalEffectError> {
        if event.host != self.host
            || event.tenant != self.tenant
            || !valid_id(&event.event_id)
            || !valid_sha256(&event.signature_sha256)
            || !valid_sha256(&event.content_sha256)
            || !self.signing_key_ids.contains(&event.signing_key_id)
            || !valid_id(&event.nonce)
            || !valid_id(&event.cursor)
            || now.saturating_sub(event.timestamp_epoch_seconds) > self.replay_window_seconds
        {
            return Err(ExternalEffectError::InvalidEvent);
        }
        if self.seen_nonces.contains(&event.nonce) || event.sequence <= self.last_sequence {
            return Ok(EventDisposition::Duplicate);
        }
        if self.seen_nonces.len() >= self.maximum_nonces {
            return Err(ExternalEffectError::EventCapacityExceeded);
        }
        let disposition = if event.sequence == self.last_sequence + 1 {
            EventDisposition::Applied
        } else {
            EventDisposition::Gap
        };
        self.seen_nonces.insert(event.nonce.clone());
        self.last_sequence = event.sequence;
        self.last_cursor = Some(event.cursor.clone());
        Ok(disposition)
    }
}

fn validate_plan(plan: &ExternalEffectPlan) -> Result<(), ExternalEffectError> {
    let ids = [
        &plan.actor,
        &plan.target,
        &plan.object,
        &plan.visibility,
        &plan.source_revision,
        &plan.environment,
        &plan.recovery,
        &plan.idempotency_key,
    ];
    let hashes = [
        &plan.payload_sha256,
        &plan.expected_change_sha256,
        &plan.preconditions_sha256,
        &plan.operation_fingerprint_sha256,
    ];
    if ids.into_iter().any(|v| !valid_id(v))
        || hashes.into_iter().any(|v| !valid_sha256(v))
        || plan.attachment_sha256.iter().any(|v| !valid_sha256(v))
        || plan.operation_budget == 0
        || plan.expires_at_epoch_seconds == 0
    {
        Err(ExternalEffectError::InvalidRecord)
    } else {
        Ok(())
    }
}
fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':' | b'/'))
}
fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn plan() -> ExternalEffectPlan {
        let h = "a".repeat(64);
        ExternalEffectPlan {
            actor: "actor".into(),
            target: "target".into(),
            object: "object".into(),
            payload_sha256: h.clone(),
            attachment_sha256: vec![h.clone()],
            visibility: "private".into(),
            source_revision: "revision".into(),
            environment: "test".into(),
            expected_change_sha256: h.clone(),
            maximum_cost_minor_units: 1,
            operation_budget: 1,
            preconditions_sha256: h.clone(),
            expires_at_epoch_seconds: 200,
            recovery: "reconcile".into(),
            idempotency_key: "key".into(),
            operation_fingerprint_sha256: h,
        }
    }
    #[test]
    fn exact_effect_lifecycle() {
        let mut v = ExternalEffectLifecycle::new(plan()).unwrap();
        v.refresh("b".repeat(64)).unwrap();
        v.preview("c".repeat(64)).unwrap();
        v.submit(&"b".repeat(64), &"a".repeat(64), 100).unwrap();
        v.reconcile(EffectState::Effect, "d".repeat(64)).unwrap();
        assert_eq!(v.state, EffectState::Effect);
    }
    #[test]
    fn stale_state_blocks_submission() {
        let mut v = ExternalEffectLifecycle::new(plan()).unwrap();
        v.refresh("b".repeat(64)).unwrap();
        v.preview("c".repeat(64)).unwrap();
        assert_eq!(
            v.submit(&"c".repeat(64), &"a".repeat(64), 100),
            Err(ExternalEffectError::StaleApproval)
        );
    }
    #[test]
    fn unknown_effect_blocks_retry() {
        let mut v = ExternalEffectLifecycle::new(plan()).unwrap();
        v.refresh("b".repeat(64)).unwrap();
        v.preview("c".repeat(64)).unwrap();
        v.submit(&"b".repeat(64), &"a".repeat(64), 100).unwrap();
        v.reconcile(EffectState::Unknown, "d".repeat(64)).unwrap();
        assert_eq!(
            v.retry_permitted(),
            Err(ExternalEffectError::ReconciliationRequired)
        );
    }
    #[test]
    fn events_are_bounded_deduplicated_and_gap_visible() {
        let mut l = EventLedger {
            host: "host".into(),
            tenant: "tenant".into(),
            signing_key_ids: BTreeSet::from(["key".into()]),
            last_sequence: 0,
            last_cursor: None,
            seen_nonces: BTreeSet::new(),
            replay_window_seconds: 60,
            maximum_nonces: 2,
        };
        let e = ProviderEvent {
            event_id: "event".into(),
            host: "host".into(),
            tenant: "tenant".into(),
            signature_sha256: "a".repeat(64),
            signing_key_id: "key".into(),
            timestamp_epoch_seconds: 90,
            nonce: "nonce".into(),
            sequence: 2,
            cursor: "cursor".into(),
            content_sha256: "b".repeat(64),
            tombstone: false,
            backfill: false,
        };
        assert_eq!(l.ingest(&e, 100), Ok(EventDisposition::Gap));
        assert_eq!(l.ingest(&e, 100), Ok(EventDisposition::Duplicate));
    }
}
