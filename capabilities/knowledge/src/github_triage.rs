//! Pure read-only GitHub triage, correlation, draft, and event-admission contracts.
#![allow(missing_docs)]

use agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION;
use serde::{Deserialize, Serialize};

use crate::hosted_repository::{
    HostedContentClassification, HostedCoverageState, HostedSourceIdentity,
};
use crate::word_generation::valid_identifier;
use crate::word_ooxml::word_sha256;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriageObjectKind {
    Issue,
    PullRequest,
    Review,
    ReviewThread,
    CheckRun,
    WorkflowRun,
    Notification,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriageState {
    Open,
    Closed,
    Draft,
    Pending,
    Passing,
    Failing,
    Cancelled,
    Deleted,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriageReason {
    UserSelected,
    Unassigned,
    Stale,
    Blocked,
    Duplicate,
    DependencyLinked,
    RecentlyChanged,
    ReviewRequested,
    Mentioned,
    FailingCheck,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalDraftKind {
    Issue,
    PullRequest,
    Review,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedTriageObservation {
    pub object_id: String,
    pub kind: TriageObjectKind,
    pub state: TriageState,
    pub source: HostedSourceIdentity,
    pub base_revision: Option<String>,
    pub head_revision: Option<String>,
    pub updated_epoch_milliseconds: u64,
    pub edited: bool,
    pub moved_lines: bool,
    pub permission_scopes: Vec<String>,
    pub coverage: HostedCoverageState,
    pub content_sha256: String,
    pub classification: HostedContentClassification,
    pub uncertainty_codes: Vec<String>,
    pub imported_authority: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedLocalRelationship {
    pub relationship_id: String,
    pub hosted_object_id: String,
    pub hosted_repository_id: String,
    pub hosted_revision: String,
    pub local_repository_id: String,
    pub local_worktree_id: Option<String>,
    pub local_revision: Option<String>,
    pub task_id: Option<String>,
    pub decision_id: Option<String>,
    pub evidence_id: Option<String>,
    pub exact_revision_match: Option<bool>,
    pub resolved: bool,
    pub reason_code: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalDraftPackage {
    pub draft_id: String,
    pub kind: LocalDraftKind,
    pub hosted_object_id: String,
    pub base_revision: String,
    pub content_sha256: String,
    pub created_epoch_milliseconds: u64,
    pub sensitivity: HostedContentClassification,
    pub published: bool,
    pub provider_effect_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TriageItem {
    pub schema_version: u16,
    pub triage_id: String,
    pub object: HostedTriageObservation,
    pub reasons: Vec<TriageReason>,
    pub relationship: HostedLocalRelationship,
    pub draft: Option<LocalDraftPackage>,
    pub current: bool,
    pub provider_state_changed: bool,
    pub triage_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedEventObservation {
    pub delivery_id: String,
    pub event_kind: String,
    pub payload_sha256: String,
    pub externally_verified_signature_sha256: String,
    pub signature_valid: bool,
    pub received_epoch_milliseconds: u64,
    pub provider_timestamp_epoch_milliseconds: u64,
    pub user_initiated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedEventReceipt {
    pub schema_version: u16,
    pub delivery_id: String,
    pub event_kind: String,
    pub payload_sha256: String,
    pub accepted: bool,
    pub duplicate: bool,
    pub stale: bool,
    pub provider_state_changed: bool,
    pub local_task_created: bool,
    pub receipt_sha256: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GithubTriageError {
    InvalidInput,
    IdentityMismatch,
    Stale,
    InvalidSignature,
    Replay,
    AuthorityEscalation,
}
impl GithubTriageError {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "github-triage.input.invalid",
            Self::IdentityMismatch => "github-triage.identity.mismatch",
            Self::Stale => "github-triage.event.stale",
            Self::InvalidSignature => "github-triage.signature.invalid",
            Self::Replay => "github-triage.event.replay",
            Self::AuthorityEscalation => "github-triage.authority.denied",
        }
    }
}
impl std::fmt::Display for GithubTriageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for GithubTriageError {}

fn sha(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
}
fn sorted_unique<T: Ord>(values: &[T]) -> bool {
    values.windows(2).all(|w| w[0] < w[1])
}

pub fn build_triage_item(
    triage_id: &str,
    object: HostedTriageObservation,
    reasons: Vec<TriageReason>,
    relationship: HostedLocalRelationship,
    draft: Option<LocalDraftPackage>,
    now: u64,
) -> Result<TriageItem, GithubTriageError> {
    if !valid_identifier(triage_id)
        || !valid_identifier(&object.object_id)
        || !sha(&object.source.immutable_revision)
        || !sha(&object.source.immutable_object_id)
        || !sha(&object.content_sha256)
        || object.updated_epoch_milliseconds == 0
        || !sorted_unique(&object.permission_scopes)
        || !sorted_unique(&object.uncertainty_codes)
        || reasons.is_empty()
    {
        return Err(GithubTriageError::InvalidInput);
    }
    if object.imported_authority
        || relationship.hosted_object_id != object.object_id
        || relationship.hosted_repository_id != object.source.repository_id
        || relationship.hosted_revision != object.source.immutable_revision
    {
        return Err(GithubTriageError::IdentityMismatch);
    }
    if let Some(value) = &draft
        && (value.hosted_object_id != object.object_id
            || value.base_revision != object.source.immutable_revision
            || !sha(&value.content_sha256)
            || value.published
            || value.provider_effect_count != 0)
    {
        return Err(GithubTriageError::AuthorityEscalation);
    }
    let current = now >= object.updated_epoch_milliseconds
        && object.coverage == HostedCoverageState::Complete
        && !object.edited
        && !object.moved_lines
        && object.uncertainty_codes.is_empty();
    let mut item = TriageItem {
        schema_version: CONTRACT_SCHEMA_VERSION,
        triage_id: triage_id.into(),
        object,
        reasons,
        relationship,
        draft,
        current,
        provider_state_changed: false,
        triage_sha256: String::new(),
    };
    item.triage_sha256 =
        word_sha256(&serde_json::to_vec(&item).map_err(|_| GithubTriageError::InvalidInput)?);
    Ok(item)
}

pub fn admit_hosted_event(
    event: &HostedEventObservation,
    seen_delivery_ids: &[String],
    now: u64,
    max_age_milliseconds: u64,
) -> Result<HostedEventReceipt, GithubTriageError> {
    if !valid_identifier(&event.delivery_id)
        || !valid_identifier(&event.event_kind)
        || !sha(&event.payload_sha256)
        || !sha(&event.externally_verified_signature_sha256)
        || event.received_epoch_milliseconds == 0
        || event.provider_timestamp_epoch_milliseconds == 0
        || max_age_milliseconds == 0
    {
        return Err(GithubTriageError::InvalidInput);
    }
    if !event.user_initiated || !event.signature_valid {
        return Err(GithubTriageError::InvalidSignature);
    }
    if seen_delivery_ids.iter().any(|v| v == &event.delivery_id) {
        return Err(GithubTriageError::Replay);
    }
    if now < event.provider_timestamp_epoch_milliseconds
        || now - event.provider_timestamp_epoch_milliseconds > max_age_milliseconds
    {
        return Err(GithubTriageError::Stale);
    }
    let mut receipt = HostedEventReceipt {
        schema_version: CONTRACT_SCHEMA_VERSION,
        delivery_id: event.delivery_id.clone(),
        event_kind: event.event_kind.clone(),
        payload_sha256: event.payload_sha256.clone(),
        accepted: true,
        duplicate: false,
        stale: false,
        provider_state_changed: false,
        local_task_created: false,
        receipt_sha256: String::new(),
    };
    receipt.receipt_sha256 =
        word_sha256(&serde_json::to_vec(&receipt).map_err(|_| GithubTriageError::InvalidInput)?);
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> HostedSourceIdentity {
        HostedSourceIdentity {
            host: "github.com".into(),
            repository_id: "org-repo".into(),
            account_id: "account-1".into(),
            immutable_revision: "a".repeat(64),
            immutable_object_id: "b".repeat(64),
            path: None,
            line_start: None,
            line_end: None,
            retrieved_epoch_milliseconds: 100,
            canonical_url_sha256: "c".repeat(64),
        }
    }
    fn observation() -> HostedTriageObservation {
        HostedTriageObservation {
            object_id: "issue-1".into(),
            kind: TriageObjectKind::Issue,
            state: TriageState::Open,
            source: source(),
            base_revision: None,
            head_revision: None,
            updated_epoch_milliseconds: 100,
            edited: false,
            moved_lines: false,
            permission_scopes: vec!["issues:read".into()],
            coverage: HostedCoverageState::Complete,
            content_sha256: "d".repeat(64),
            classification: HostedContentClassification::UntrustedInstructions,
            uncertainty_codes: vec![],
            imported_authority: false,
        }
    }
    fn relationship() -> HostedLocalRelationship {
        HostedLocalRelationship {
            relationship_id: "relationship-1".into(),
            hosted_object_id: "issue-1".into(),
            hosted_repository_id: "org-repo".into(),
            hosted_revision: "a".repeat(64),
            local_repository_id: "local-repo".into(),
            local_worktree_id: None,
            local_revision: Some("a".repeat(64)),
            task_id: Some("task-1".into()),
            decision_id: None,
            evidence_id: Some("evidence-1".into()),
            exact_revision_match: Some(true),
            resolved: true,
            reason_code: "exact-match".into(),
        }
    }
    #[test]
    fn triage_and_unpublished_draft_are_exact_and_effect_free() {
        let draft = LocalDraftPackage {
            draft_id: "draft-1".into(),
            kind: LocalDraftKind::Issue,
            hosted_object_id: "issue-1".into(),
            base_revision: "a".repeat(64),
            content_sha256: "e".repeat(64),
            created_epoch_milliseconds: 110,
            sensitivity: HostedContentClassification::Private,
            published: false,
            provider_effect_count: 0,
        };
        let item = build_triage_item(
            "triage-1",
            observation(),
            vec![TriageReason::UserSelected],
            relationship(),
            Some(draft),
            120,
        )
        .unwrap();
        assert!(item.current && !item.provider_state_changed);
    }
    #[test]
    fn edits_moved_lines_permissions_and_uncertainty_are_not_current() {
        for n in 0..4 {
            let mut o = observation();
            match n {
                0 => o.edited = true,
                1 => o.moved_lines = true,
                2 => o.coverage = HostedCoverageState::Partial,
                _ => o.uncertainty_codes = vec!["provider-field-unknown".into()],
            };
            assert!(
                !build_triage_item(
                    "triage-1",
                    o,
                    vec![TriageReason::Stale],
                    relationship(),
                    None,
                    120
                )
                .unwrap()
                .current
            );
        }
    }
    #[test]
    fn drafts_cannot_publish_and_hostile_content_cannot_gain_authority() {
        let mut o = observation();
        o.imported_authority = true;
        assert!(
            build_triage_item(
                "triage-1",
                o,
                vec![TriageReason::UserSelected],
                relationship(),
                None,
                120
            )
            .is_err()
        );
        let draft = LocalDraftPackage {
            draft_id: "draft-1".into(),
            kind: LocalDraftKind::Review,
            hosted_object_id: "issue-1".into(),
            base_revision: "a".repeat(64),
            content_sha256: "e".repeat(64),
            created_epoch_milliseconds: 110,
            sensitivity: HostedContentClassification::Restricted,
            published: true,
            provider_effect_count: 1,
        };
        assert_eq!(
            build_triage_item(
                "triage-1",
                observation(),
                vec![TriageReason::ReviewRequested],
                relationship(),
                Some(draft),
                120
            ),
            Err(GithubTriageError::AuthorityEscalation)
        );
    }
    #[test]
    fn event_signature_replay_reorder_and_staleness_fail_closed() {
        let event = HostedEventObservation {
            delivery_id: "delivery-1".into(),
            event_kind: "issues".into(),
            payload_sha256: "f".repeat(64),
            externally_verified_signature_sha256: "e".repeat(64),
            signature_valid: true,
            received_epoch_milliseconds: 200,
            provider_timestamp_epoch_milliseconds: 190,
            user_initiated: true,
        };
        assert!(admit_hosted_event(&event, &[], 200, 20).unwrap().accepted);
        assert_eq!(
            admit_hosted_event(&event, &["delivery-1".into()], 200, 20),
            Err(GithubTriageError::Replay)
        );
        let mut invalid = event.clone();
        invalid.signature_valid = false;
        assert_eq!(
            admit_hosted_event(&invalid, &[], 200, 20),
            Err(GithubTriageError::InvalidSignature)
        );
        assert_eq!(
            admit_hosted_event(&event, &[], 220, 20),
            Err(GithubTriageError::Stale)
        );
    }
}
