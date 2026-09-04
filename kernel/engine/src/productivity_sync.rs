//! Deterministic provider synchronization with explicit incomplete-state truth.
use std::collections::{BTreeMap, BTreeSet};
/// Closed synchronization truth states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum SyncState {
    Current,
    Delayed,
    Incomplete,
    PermissionLimited,
    Reconciling,
    Revoked,
    Offline,
}
/// Exact provider stream boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncStream {
    /// Provider identity.
    pub provider_id: String,
    /// Tenant identity.
    pub tenant_id: String,
    /// Account identity.
    pub account_id: String,
    /// Object family.
    pub object_family: String,
    /// Exact scope.
    pub scope: String,
    /// Opaque credential reference.
    pub credential_reference: String,
    /// Closed schema version.
    pub schema_version: u16,
}
/// One provider event with monotonic sequence and opaque payload digest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncEvent {
    /// Unique event identity.
    pub event_id: String,
    /// Native object identity.
    pub object_id: String,
    /// Monotonic sequence.
    pub sequence: u64,
    /// Payload digest.
    pub payload_sha256: String,
    /// Deletion marker.
    pub deleted: bool,
}
/// User-visible coverage and recovery state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncStatus {
    /// Truth state.
    pub state: SyncState,
    /// Last complete synchronization.
    pub last_success: Option<u64>,
    /// Last attempt.
    pub last_attempt: u64,
    /// Complete coverage marker.
    pub coverage_complete: bool,
    /// Permission gap marker.
    pub permission_gap: bool,
    /// History gap marker.
    pub history_gap: bool,
    /// Throttling marker.
    pub throttled: bool,
    /// Stable visible reason.
    pub reason: String,
}
/// Canonical synchronized object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncObject {
    /// Native identity.
    pub object_id: String,
    /// Applied sequence.
    pub sequence: u64,
    /// Payload digest.
    pub payload_sha256: String,
    /// Tombstone marker.
    pub tombstoned: bool,
}
/// Stable fail-closed event result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum SyncError {
    InvalidStream,
    FutureVersion,
    ForgedEvent,
    SequenceGap,
    Revoked,
    Removed,
}
/// One bounded provider stream; events create no grants, schedules, workflows, or effects.
pub struct SyncEngine {
    stream: SyncStream,
    status: SyncStatus,
    objects: BTreeMap<String, SyncObject>,
    seen: BTreeSet<String>,
    next_sequence: u64,
    removed: bool,
}
fn id(v: &str) -> bool {
    !v.is_empty() && v.len() <= 128
}
fn digest(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
impl SyncEngine {
    /// Creates one exact version-one stream.
    pub fn new(stream: SyncStream, now: u64) -> Result<Self, SyncError> {
        if stream.schema_version != 1 {
            return Err(SyncError::FutureVersion);
        }
        if !id(&stream.provider_id)
            || !id(&stream.tenant_id)
            || !id(&stream.account_id)
            || !id(&stream.object_family)
            || !id(&stream.scope)
            || !id(&stream.credential_reference)
        {
            return Err(SyncError::InvalidStream);
        }
        Ok(Self {
            stream,
            status: SyncStatus {
                state: SyncState::Reconciling,
                last_success: None,
                last_attempt: now,
                coverage_complete: false,
                permission_gap: false,
                history_gap: false,
                throttled: false,
                reason: "initial_reconciliation".into(),
            },
            objects: BTreeMap::new(),
            seen: BTreeSet::new(),
            next_sequence: 1,
            removed: false,
        })
    }
    /// Applies duplicates idempotently, rejects forgery, and exposes sequence gaps.
    pub fn ingest(&mut self, event: SyncEvent, now: u64) -> Result<bool, SyncError> {
        if self.removed {
            return Err(SyncError::Removed);
        }
        if self.status.state == SyncState::Revoked {
            return Err(SyncError::Revoked);
        }
        self.status.last_attempt = now;
        if !id(&event.event_id) || !id(&event.object_id) || !digest(&event.payload_sha256) {
            return Err(SyncError::ForgedEvent);
        }
        if self.seen.contains(&event.event_id) {
            return Ok(false);
        }
        if event.sequence != self.next_sequence {
            self.status.state = SyncState::Incomplete;
            self.status.coverage_complete = false;
            self.status.history_gap = true;
            self.status.reason = "sequence_gap".into();
            return Err(SyncError::SequenceGap);
        }
        self.next_sequence += 1;
        self.seen.insert(event.event_id);
        self.objects.insert(
            event.object_id.clone(),
            SyncObject {
                object_id: event.object_id,
                sequence: event.sequence,
                payload_sha256: event.payload_sha256,
                tombstoned: event.deleted,
            },
        );
        self.status.state = SyncState::Current;
        self.status.coverage_complete = true;
        self.status.history_gap = false;
        self.status.last_success = Some(now);
        self.status.reason = "current".into();
        Ok(true)
    }
    /// Marks permission reduction without presenting absence as complete.
    pub fn permission_gap(&mut self, now: u64) {
        self.status = SyncStatus {
            state: SyncState::PermissionLimited,
            last_success: self.status.last_success,
            last_attempt: now,
            coverage_complete: false,
            permission_gap: true,
            history_gap: self.status.history_gap,
            throttled: false,
            reason: "permission_gap".into(),
        }
    }
    /// Revokes all further event and polling authority.
    pub fn revoke(&mut self, now: u64) {
        self.status.state = SyncState::Revoked;
        self.status.last_attempt = now;
        self.status.coverage_complete = false;
        self.status.reason = "revoked".into()
    }
    /// Removes stream state and every cursor/poller authority.
    pub fn remove(&mut self) {
        self.objects.clear();
        self.seen.clear();
        self.removed = true;
        self.status.state = SyncState::Revoked;
        self.status.coverage_complete = false;
        self.status.reason = "removed".into()
    }
    /// Returns visible truth state.
    pub fn status(&self) -> &SyncStatus {
        &self.status
    }
    /// Returns the exact provider stream boundary.
    pub fn stream(&self) -> &SyncStream {
        &self.stream
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn stream() -> SyncStream {
        SyncStream {
            provider_id: "p".into(),
            tenant_id: "t".into(),
            account_id: "a".into(),
            object_family: "message".into(),
            scope: "read".into(),
            credential_reference: "cred".into(),
            schema_version: 1,
        }
    }
    fn event(id: &str, seq: u64) -> SyncEvent {
        SyncEvent {
            event_id: id.into(),
            object_id: "o".into(),
            sequence: seq,
            payload_sha256: "a".repeat(64),
            deleted: false,
        }
    }
    #[test]
    fn duplicates_are_idempotent() {
        let mut e = SyncEngine::new(stream(), 0).unwrap();
        assert_eq!(e.ingest(event("e1", 1), 1), Ok(true));
        assert_eq!(e.ingest(event("e1", 1), 2), Ok(false));
    }
    #[test]
    fn gaps_are_visible_and_incomplete() {
        let mut e = SyncEngine::new(stream(), 0).unwrap();
        assert_eq!(e.ingest(event("e2", 2), 1), Err(SyncError::SequenceGap));
        assert!(e.status().history_gap);
        assert!(!e.status().coverage_complete)
    }
    #[test]
    fn revocation_and_removal_end_authority() {
        let mut e = SyncEngine::new(stream(), 0).unwrap();
        e.revoke(1);
        assert_eq!(e.ingest(event("e1", 1), 2), Err(SyncError::Revoked));
        let mut e = SyncEngine::new(stream(), 0).unwrap();
        e.remove();
        assert_eq!(e.ingest(event("e1", 1), 2), Err(SyncError::Removed));
        assert!(e.objects.is_empty() && e.seen.is_empty())
    }
}
