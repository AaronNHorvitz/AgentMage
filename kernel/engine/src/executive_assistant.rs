//! Deterministic executive-assistant projections over approved local records.

use std::collections::{BTreeMap, BTreeSet};

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, ExecutiveCorrespondenceDraft, ExecutiveCorrespondenceIssue,
    ExecutiveCorrespondenceIssueKind, ExecutiveCorrespondenceReview, ExecutiveDueWindow,
    ExecutiveEvidenceState, ExecutiveLocalMessage, ExecutiveMessageTriageClass,
    ExecutiveMessageTriageEntry, ExecutivePortfolioSnapshot, ExecutivePriorityComponent,
    ExecutivePriorityComponentKind, ExecutivePriorityEntry, ExecutivePriorityRanking,
    ExecutivePrivacyClass, ExecutivePrivacyDecision, ExecutivePrivacyOperation,
    ExecutivePrivacyRequest, ExecutiveRecord, ExecutiveRecordKind, ExecutiveRecordStatus,
    ExecutiveReminder, ExecutiveReminderActionKind, ExecutiveReminderEvent, ExecutiveReminderState,
    ExecutiveSourceReference, ExecutiveSourceStore, ExecutiveTracker, ExecutiveTrackerEntry,
    ExecutiveTrackerKind, ExecutiveView, ExecutiveViewItem, ExecutiveViewKind,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const MAX_RECORDS: usize = 1_024;
const MAX_HISTORY_RECORDS: usize = 4_096;
const MAX_SOURCES: usize = 64;
const MAX_FIELDS: usize = 64;
const MAX_DEPENDENCIES: usize = 128;
const MAX_ITEMS: usize = 1_024;
const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_SHORT_TEXT_BYTES: usize = 512;
const MAX_TEXT_BYTES: usize = 32 * 1_024;
const MAX_DRAFT_BYTES: usize = 128 * 1_024;
const PRIORITY_METHOD_ID: &str = "executive-priority-fixed-v1";
const PRIORITY_METHOD_VERSION: &str = "1";
const PRIORITY_METHOD_SPEC: &str = "urgency*30 + importance*30 + user-preference*10 + consequence*15 + closed-due-window + schedule-conflict - unresolved-dependencies + bounded-shorter-effort - evidence-uncertainty; ties by record-id";

/// Stable fail-closed reason an executive-assistant projection was rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutiveAssistantError {
    /// An identity, text, date, score, list, or source reference is malformed.
    InvalidInput,
    /// A canonical record, source, dependency, or snapshot is duplicated or inconsistent.
    IntegrityFailure,
    /// The requested record class is outside the admitted privacy boundary.
    PrivacyDenied,
    /// The supplied portfolio snapshot no longer matches its exact digest.
    StaleSnapshot,
    /// A changes-since view lacks a valid earlier snapshot.
    PriorSnapshotRequired,
}

impl ExecutiveAssistantError {
    /// Returns a stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "executive.input.invalid",
            Self::IntegrityFailure => "executive.integrity.failed",
            Self::PrivacyDenied => "executive.privacy.denied",
            Self::StaleSnapshot => "executive.snapshot.stale",
            Self::PriorSnapshotRequired => "executive.prior-snapshot.required",
        }
    }
}

impl std::fmt::Display for ExecutiveAssistantError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for ExecutiveAssistantError {}

/// Current privacy and historical-state inputs for one deterministic view.
#[derive(Clone, Debug)]
pub struct ExecutiveViewRequest<'a> {
    /// Stable view identity.
    pub view_id: String,
    /// Requested view class.
    pub kind: ExecutiveViewKind,
    /// Privacy classes admitted by the caller's already-authorized local read.
    pub admitted_privacy_classes: Vec<ExecutivePrivacyClass>,
    /// Exact earlier snapshot for a changes-since view.
    pub prior_snapshot: Option<&'a ExecutivePortfolioSnapshot>,
}

/// Context used to review one exact local correspondence draft.
#[derive(Clone, Debug)]
pub struct CorrespondenceReviewContext<'a> {
    /// Canonical local records available to support draft claims.
    pub records: &'a [ExecutiveRecord],
    /// Exact source-question identities the draft is expected to answer.
    pub required_question_ids: &'a [String],
    /// Exact source-question identities answered by the draft.
    pub answered_question_ids: &'a [String],
    /// User-approved recipient and professional-name labels.
    pub confirmed_names: &'a [String],
    /// Strictest privacy class admitted to this draft.
    pub maximum_privacy_class: ExecutivePrivacyClass,
}

fn digest_bytes(value: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to a string cannot fail");
    }
    output
}

fn digest_record<T: Serialize>(value: &T) -> Result<String, ExecutiveAssistantError> {
    let bytes = serde_json::to_vec(value).map_err(|_| ExecutiveAssistantError::InvalidInput)?;
    Ok(digest_bytes(&bytes))
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value.is_ascii()
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn valid_code(value: &str) -> bool {
    valid_identifier(value)
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase())
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'.' | b'-')
        })
}

fn valid_text(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty()
        && value.len() <= maximum
        && !value.contains('\0')
        && !value
            .chars()
            .any(|character| character.is_control() && character != '\n' && character != '\t')
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    if !bytes
        .iter()
        .enumerate()
        .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
    {
        return false;
    }
    let year = value[0..4].parse::<u16>().ok();
    let month = value[5..7].parse::<u8>().ok();
    let day = value[8..10].parse::<u8>().ok();
    year.is_some_and(|year| year >= 1970)
        && month.is_some_and(|month| (1..=12).contains(&month))
        && day.is_some_and(|day| (1..=31).contains(&day))
}

fn sorted_unique(values: &[String]) -> bool {
    values
        .windows(2)
        .all(|window| window[0].as_str() < window[1].as_str())
}

fn validate_source(source: &ExecutiveSourceReference) -> bool {
    valid_identifier(&source.source_id)
        && valid_identifier(&source.object_id)
        && source
            .fragment
            .as_deref()
            .is_none_or(|value| valid_text(value, MAX_SHORT_TEXT_BYTES))
        && valid_sha256(&source.content_sha256)
        && source
            .observed_revision
            .as_deref()
            .is_none_or(valid_identifier)
}

fn source_key(source: &ExecutiveSourceReference) -> (&str, &str, &str) {
    (
        source.source_id.as_str(),
        source.object_id.as_str(),
        source.fragment.as_deref().unwrap_or(""),
    )
}

fn prohibited_professional_field(code: &str) -> bool {
    matches!(
        code,
        "biometric"
            | "disability"
            | "genetic"
            | "health"
            | "political_affiliation"
            | "race"
            | "religion"
            | "sexual_orientation"
    ) || code.starts_with("sensitive_")
}

fn validate_record(record: &ExecutiveRecord) -> Result<(), ExecutiveAssistantError> {
    if record.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_identifier(&record.record_id)
        || !valid_text(&record.title, MAX_SHORT_TEXT_BYTES)
        || !valid_text(&record.summary, MAX_TEXT_BYTES)
        || record.urgency_bps > 10_000
        || record.importance_bps > 10_000
        || record.user_preference_bps > 10_000
        || record.consequence_bps > 10_000
        || record.sources.is_empty()
        || record.sources.len() > MAX_SOURCES
        || record.fields.len() > MAX_FIELDS
        || record.dependency_ids.len() > MAX_DEPENDENCIES
    {
        return Err(ExecutiveAssistantError::InvalidInput);
    }
    for value in [
        record.owner.as_deref(),
        record.counterparty.as_deref(),
        record.project_id.as_deref(),
    ] {
        if value.is_some_and(|candidate| !valid_text(candidate, MAX_SHORT_TEXT_BYTES)) {
            return Err(ExecutiveAssistantError::InvalidInput);
        }
    }
    if record
        .due_date
        .as_deref()
        .is_some_and(|date| !valid_date(date))
        || (record.due_date.is_none() != (record.due_window == ExecutiveDueWindow::Unknown))
        || record.estimated_effort_minutes == Some(0)
        || record
            .supersedes_record_id
            .as_deref()
            .is_some_and(|value| !valid_identifier(value) || value == record.record_id)
        || !sorted_unique(&record.dependency_ids)
        || record
            .dependency_ids
            .iter()
            .any(|dependency| !valid_identifier(dependency) || dependency == &record.record_id)
    {
        return Err(ExecutiveAssistantError::InvalidInput);
    }
    if matches!(record.status, ExecutiveRecordStatus::Completed)
        && record.evidence_state != ExecutiveEvidenceState::Confirmed
    {
        return Err(ExecutiveAssistantError::IntegrityFailure);
    }
    if matches!(
        record.kind,
        ExecutiveRecordKind::Commitment | ExecutiveRecordKind::Decision
    ) && matches!(
        record.status,
        ExecutiveRecordStatus::Active | ExecutiveRecordStatus::Completed
    ) && record.evidence_state != ExecutiveEvidenceState::Confirmed
    {
        return Err(ExecutiveAssistantError::IntegrityFailure);
    }
    if !record.sources.iter().all(validate_source)
        || !record
            .sources
            .windows(2)
            .all(|window| source_key(&window[0]) < source_key(&window[1]))
    {
        return Err(ExecutiveAssistantError::IntegrityFailure);
    }
    let source_ids = record
        .sources
        .iter()
        .map(|source| source.source_id.as_str())
        .collect::<BTreeSet<_>>();
    if !record
        .fields
        .windows(2)
        .all(|window| window[0].field_code < window[1].field_code)
        || record.fields.iter().any(|field| {
            !valid_code(&field.field_code)
                || !valid_text(&field.value, MAX_TEXT_BYTES)
                || field.source_ids.is_empty()
                || !sorted_unique(&field.source_ids)
                || field
                    .source_ids
                    .iter()
                    .any(|source_id| !source_ids.contains(source_id.as_str()))
                || (matches!(
                    record.kind,
                    ExecutiveRecordKind::Person | ExecutiveRecordKind::Organization
                ) && prohibited_professional_field(&field.field_code))
        })
    {
        return Err(ExecutiveAssistantError::IntegrityFailure);
    }
    Ok(())
}

fn canonical_records(
    mut records: Vec<ExecutiveRecord>,
) -> Result<Vec<ExecutiveRecord>, ExecutiveAssistantError> {
    if records.len() > MAX_RECORDS {
        return Err(ExecutiveAssistantError::InvalidInput);
    }
    for record in &records {
        validate_record(record)?;
    }
    records.sort_by(|left, right| left.record_id.cmp(&right.record_id));
    if records
        .windows(2)
        .any(|window| window[0].record_id == window[1].record_id)
    {
        return Err(ExecutiveAssistantError::IntegrityFailure);
    }
    Ok(records)
}

fn canonical_history(
    mut records: Vec<ExecutiveRecord>,
) -> Result<Vec<ExecutiveRecord>, ExecutiveAssistantError> {
    if records.len() > MAX_HISTORY_RECORDS {
        return Err(ExecutiveAssistantError::InvalidInput);
    }
    for record in &records {
        validate_record(record)?;
    }
    records.sort_by(|left, right| {
        left.record_id
            .cmp(&right.record_id)
            .then_with(|| source_key(&left.sources[0]).cmp(&source_key(&right.sources[0])))
            .then_with(|| left.summary.cmp(&right.summary))
    });
    if records.windows(2).any(|window| window[0] == window[1]) {
        return Err(ExecutiveAssistantError::IntegrityFailure);
    }
    Ok(records)
}

fn seal_snapshot(
    mut snapshot: ExecutivePortfolioSnapshot,
) -> Result<ExecutivePortfolioSnapshot, ExecutiveAssistantError> {
    snapshot.snapshot_sha256.clear();
    snapshot.snapshot_sha256 = digest_record(&snapshot)?;
    Ok(snapshot)
}

fn verify_snapshot(snapshot: &ExecutivePortfolioSnapshot) -> Result<(), ExecutiveAssistantError> {
    let mut candidate = snapshot.clone();
    let supplied = candidate.snapshot_sha256.clone();
    candidate.snapshot_sha256.clear();
    if !valid_sha256(&supplied) || digest_record(&candidate)? != supplied {
        return Err(ExecutiveAssistantError::StaleSnapshot);
    }
    if candidate.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_identifier(&candidate.snapshot_id)
        || candidate.source_mutation_allowed
    {
        return Err(ExecutiveAssistantError::IntegrityFailure);
    }
    let current = canonical_records(candidate.current_records.clone())?;
    let history = canonical_history(candidate.history.clone())?;
    if current != candidate.current_records || history != candidate.history {
        return Err(ExecutiveAssistantError::IntegrityFailure);
    }
    Ok(())
}

/// Builds one immutable canonical current-and-history portfolio snapshot.
pub fn build_portfolio_snapshot(
    snapshot_id: String,
    current_records: Vec<ExecutiveRecord>,
    history: Vec<ExecutiveRecord>,
) -> Result<ExecutivePortfolioSnapshot, ExecutiveAssistantError> {
    if !valid_identifier(&snapshot_id) {
        return Err(ExecutiveAssistantError::InvalidInput);
    }
    let current_records = canonical_records(current_records)?;
    if current_records
        .iter()
        .any(|record| record.status == ExecutiveRecordStatus::Superseded)
    {
        return Err(ExecutiveAssistantError::IntegrityFailure);
    }
    seal_snapshot(ExecutivePortfolioSnapshot {
        schema_version: CONTRACT_SCHEMA_VERSION,
        snapshot_id,
        current_records,
        history: canonical_history(history)?,
        source_mutation_allowed: false,
        snapshot_sha256: String::new(),
    })
}

/// Reconciles source corrections, supersessions, completion, and conflicts while preserving history.
pub fn reconcile_portfolio(
    previous: &ExecutivePortfolioSnapshot,
    updates: Vec<ExecutiveRecord>,
    next_snapshot_id: String,
) -> Result<ExecutivePortfolioSnapshot, ExecutiveAssistantError> {
    verify_snapshot(previous)?;
    let updates = canonical_records(updates)?;
    let mut current = previous
        .current_records
        .iter()
        .cloned()
        .map(|record| (record.record_id.clone(), record))
        .collect::<BTreeMap<_, _>>();
    let mut history = previous.history.clone();
    for update in updates {
        if let Some(superseded_id) = &update.supersedes_record_id {
            let superseded = current
                .remove(superseded_id)
                .ok_or(ExecutiveAssistantError::IntegrityFailure)?;
            history.push(superseded);
        }
        if let Some(earlier) = current.remove(&update.record_id) {
            history.push(earlier);
        }
        if update.status == ExecutiveRecordStatus::Superseded {
            history.push(update);
        } else {
            current.insert(update.record_id.clone(), update);
        }
    }
    build_portfolio_snapshot(next_snapshot_id, current.into_values().collect(), history)
}

fn require_admitted_records(
    records: &[ExecutiveRecord],
    admitted: &[ExecutivePrivacyClass],
) -> Result<(), ExecutiveAssistantError> {
    let classes = admitted.iter().copied().collect::<BTreeSet<_>>();
    if classes.len() != admitted.len()
        || records
            .iter()
            .any(|record| !classes.contains(&record.privacy_class))
    {
        return Err(ExecutiveAssistantError::PrivacyDenied);
    }
    Ok(())
}

fn due_points(window: ExecutiveDueWindow) -> (i64, &'static str) {
    match window {
        ExecutiveDueWindow::Overdue => (150_000, "priority.due.overdue"),
        ExecutiveDueWindow::Today => (140_000, "priority.due.today"),
        ExecutiveDueWindow::WithinSevenDays => (110_000, "priority.due.within-seven-days"),
        ExecutiveDueWindow::WithinThirtyDays => (60_000, "priority.due.within-thirty-days"),
        ExecutiveDueWindow::Later => (15_000, "priority.due.later"),
        ExecutiveDueWindow::Unknown => (0, "priority.due.unknown"),
    }
}

fn evidence_points(state: ExecutiveEvidenceState) -> (i64, &'static str) {
    match state {
        ExecutiveEvidenceState::Confirmed => (0, "priority.evidence.confirmed"),
        ExecutiveEvidenceState::Historical => (-30_000, "priority.evidence.historical"),
        ExecutiveEvidenceState::Inferred => (-40_000, "priority.evidence.inferred"),
        ExecutiveEvidenceState::Disputed => (-75_000, "priority.evidence.disputed"),
        ExecutiveEvidenceState::Unknown => (-100_000, "priority.evidence.unknown"),
    }
}

fn source_ids(record: &ExecutiveRecord) -> Vec<String> {
    record
        .sources
        .iter()
        .map(|source| source.source_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Ranks active records with an exact integer method, visible components, and deterministic ties.
pub fn rank_priorities(
    records: &[ExecutiveRecord],
    admitted_privacy_classes: &[ExecutivePrivacyClass],
) -> Result<Vec<ExecutivePriorityEntry>, ExecutiveAssistantError> {
    if records.len() > MAX_RECORDS {
        return Err(ExecutiveAssistantError::InvalidInput);
    }
    for record in records {
        validate_record(record)?;
    }
    require_admitted_records(records, admitted_privacy_classes)?;
    let statuses = records
        .iter()
        .map(|record| (record.record_id.as_str(), record.status))
        .collect::<BTreeMap<_, _>>();
    let mut entries = Vec::new();
    for record in records.iter().filter(|record| {
        !matches!(
            record.status,
            ExecutiveRecordStatus::Completed
                | ExecutiveRecordStatus::Superseded
                | ExecutiveRecordStatus::Cancelled
        )
    }) {
        let unresolved = record
            .dependency_ids
            .iter()
            .filter(|dependency| {
                statuses.get(dependency.as_str()) != Some(&ExecutiveRecordStatus::Completed)
            })
            .count();
        let (due, due_reason) = due_points(record.due_window);
        let (evidence, evidence_reason) = evidence_points(record.evidence_state);
        let effort = record.estimated_effort_minutes.map_or(0, |minutes| {
            20_000_i64.saturating_sub(i64::from(minutes).min(20_000))
        });
        let schedule = if record.schedule_conflict { 50_000 } else { 0 };
        let dependency = -(i64::try_from(unresolved).unwrap_or(i64::MAX).min(8) * 75_000);
        let components = vec![
            ExecutivePriorityComponent {
                kind: ExecutivePriorityComponentKind::Urgency,
                points: i64::from(record.urgency_bps) * 30,
                reason_code: "priority.urgency.weight-30".to_owned(),
            },
            ExecutivePriorityComponent {
                kind: ExecutivePriorityComponentKind::Importance,
                points: i64::from(record.importance_bps) * 30,
                reason_code: "priority.importance.weight-30".to_owned(),
            },
            ExecutivePriorityComponent {
                kind: ExecutivePriorityComponentKind::UserPreference,
                points: i64::from(record.user_preference_bps) * 10,
                reason_code: "priority.user-preference.weight-10".to_owned(),
            },
            ExecutivePriorityComponent {
                kind: ExecutivePriorityComponentKind::Consequence,
                points: i64::from(record.consequence_bps) * 15,
                reason_code: "priority.consequence.weight-15".to_owned(),
            },
            ExecutivePriorityComponent {
                kind: ExecutivePriorityComponentKind::DueWindow,
                points: due,
                reason_code: due_reason.to_owned(),
            },
            ExecutivePriorityComponent {
                kind: ExecutivePriorityComponentKind::Schedule,
                points: schedule,
                reason_code: if record.schedule_conflict {
                    "priority.schedule.conflict"
                } else {
                    "priority.schedule.clear"
                }
                .to_owned(),
            },
            ExecutivePriorityComponent {
                kind: ExecutivePriorityComponentKind::Dependencies,
                points: dependency,
                reason_code: format!("priority.dependencies.unresolved-{unresolved}"),
            },
            ExecutivePriorityComponent {
                kind: ExecutivePriorityComponentKind::Effort,
                points: effort,
                reason_code: if record.estimated_effort_minutes.is_some() {
                    "priority.effort.known"
                } else {
                    "priority.effort.unknown"
                }
                .to_owned(),
            },
            ExecutivePriorityComponent {
                kind: ExecutivePriorityComponentKind::EvidenceState,
                points: evidence,
                reason_code: evidence_reason.to_owned(),
            },
        ];
        let mut limitations = Vec::new();
        if record.due_date.is_none() {
            limitations.push("priority.limit.due-date-unknown".to_owned());
        }
        if record.estimated_effort_minutes.is_none() {
            limitations.push("priority.limit.effort-unknown".to_owned());
        }
        if record.owner.is_none() {
            limitations.push("priority.limit.owner-unknown".to_owned());
        }
        if record.schedule_conflict {
            limitations.push("priority.limit.schedule-conflict".to_owned());
        }
        if record.evidence_state != ExecutiveEvidenceState::Confirmed {
            limitations.push(
                format!("priority.limit.evidence-{:?}", record.evidence_state).to_ascii_lowercase(),
            );
        }
        limitations.sort();
        entries.push(ExecutivePriorityEntry {
            record_id: record.record_id.clone(),
            rank: 0,
            score: components.iter().map(|component| component.points).sum(),
            components,
            source_ids: source_ids(record),
            limitations,
        });
    }
    entries.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.record_id.cmp(&right.record_id))
    });
    for (index, entry) in entries.iter_mut().enumerate() {
        entry.rank = u32::try_from(index + 1).map_err(|_| ExecutiveAssistantError::InvalidInput)?;
    }
    Ok(entries)
}

/// Builds a hash-bound authority-free priority recommendation envelope.
pub fn build_priority_ranking(
    ranking_id: String,
    records: &[ExecutiveRecord],
    admitted_privacy_classes: &[ExecutivePrivacyClass],
) -> Result<ExecutivePriorityRanking, ExecutiveAssistantError> {
    if !valid_identifier(&ranking_id) {
        return Err(ExecutiveAssistantError::InvalidInput);
    }
    let mut ranking = ExecutivePriorityRanking {
        schema_version: CONTRACT_SCHEMA_VERSION,
        ranking_id,
        method_id: PRIORITY_METHOD_ID.to_owned(),
        method_version: PRIORITY_METHOD_VERSION.to_owned(),
        method_sha256: digest_bytes(PRIORITY_METHOD_SPEC.as_bytes()),
        entries: rank_priorities(records, admitted_privacy_classes)?,
        proposal_only: true,
        external_effect_allowed: false,
        ranking_sha256: String::new(),
    };
    ranking.ranking_sha256 = digest_record(&ranking)?;
    Ok(ranking)
}

fn tracker_accepts(kind: ExecutiveTrackerKind, record: &ExecutiveRecord) -> bool {
    match kind {
        ExecutiveTrackerKind::Commitments => record.kind == ExecutiveRecordKind::Commitment,
        ExecutiveTrackerKind::Decisions => record.kind == ExecutiveRecordKind::Decision,
        ExecutiveTrackerKind::Waiting => record.kind == ExecutiveRecordKind::Waiting,
        ExecutiveTrackerKind::Approvals => record.kind == ExecutiveRecordKind::Approval,
        ExecutiveTrackerKind::Deadlines => record.kind == ExecutiveRecordKind::Deadline,
        ExecutiveTrackerKind::Reminders => {
            record.due_date.is_some()
                && matches!(
                    record.kind,
                    ExecutiveRecordKind::Task
                        | ExecutiveRecordKind::Commitment
                        | ExecutiveRecordKind::Waiting
                        | ExecutiveRecordKind::Approval
                        | ExecutiveRecordKind::Deadline
                        | ExecutiveRecordKind::Meeting
                )
                && !matches!(
                    record.status,
                    ExecutiveRecordStatus::Completed
                        | ExecutiveRecordStatus::Cancelled
                        | ExecutiveRecordStatus::Superseded
                )
        }
    }
}

/// Builds one deterministic tracker without creating a notification or reminder effect.
pub fn build_tracker(
    tracker_id: String,
    kind: ExecutiveTrackerKind,
    records: &[ExecutiveRecord],
    admitted_privacy_classes: &[ExecutivePrivacyClass],
) -> Result<ExecutiveTracker, ExecutiveAssistantError> {
    if !valid_identifier(&tracker_id) || records.len() > MAX_RECORDS {
        return Err(ExecutiveAssistantError::InvalidInput);
    }
    for record in records {
        validate_record(record)?;
    }
    require_admitted_records(records, admitted_privacy_classes)?;
    let mut entries = records
        .iter()
        .filter(|record| tracker_accepts(kind, record))
        .map(|record| ExecutiveTrackerEntry {
            record_id: record.record_id.clone(),
            title: record.title.clone(),
            status: record.status,
            evidence_state: record.evidence_state,
            due_date: record.due_date.clone(),
            person: record.counterparty.clone().or_else(|| record.owner.clone()),
            source_ids: source_ids(record),
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        left.due_date
            .is_none()
            .cmp(&right.due_date.is_none())
            .then_with(|| left.due_date.cmp(&right.due_date))
            .then_with(|| left.record_id.cmp(&right.record_id))
    });
    let mut tracker = ExecutiveTracker {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tracker_id,
        kind,
        entries,
        notification_allowed: false,
        tracker_sha256: String::new(),
    };
    tracker.tracker_sha256 = digest_record(&tracker)?;
    Ok(tracker)
}

fn reminder_event_digest(
    event: &ExecutiveReminderEvent,
) -> Result<String, ExecutiveAssistantError> {
    let mut candidate = event.clone();
    candidate.event_sha256.clear();
    digest_record(&candidate)
}

fn reminder_digest(reminder: &ExecutiveReminder) -> Result<String, ExecutiveAssistantError> {
    let mut candidate = reminder.clone();
    candidate.reminder_sha256.clear();
    digest_record(&candidate)
}

/// Verifies a complete durable reminder chain and its authority-free current projection.
pub fn verify_reminder(reminder: &ExecutiveReminder) -> Result<(), ExecutiveAssistantError> {
    if reminder.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_identifier(&reminder.reminder_id)
        || !valid_identifier(&reminder.record_id)
        || !valid_sha256(&reminder.record_sha256)
        || reminder.source_ids.is_empty()
        || !sorted_unique(&reminder.source_ids)
        || reminder.notification_allowed
        || reminder.events.is_empty()
        || reminder.reminder_sha256 != reminder_digest(reminder)?
    {
        return Err(ExecutiveAssistantError::IntegrityFailure);
    }
    let mut prior = None;
    let mut prior_state = None;
    let mut prior_time = None;
    for (index, event) in reminder.events.iter().enumerate() {
        let revision =
            u64::try_from(index + 1).map_err(|_| ExecutiveAssistantError::InvalidInput)?;
        if !valid_identifier(&event.event_id)
            || event.reminder_id != reminder.reminder_id
            || event.revision != revision
            || event.occurred_at_epoch_ms == 0
            || event.external_effect_allowed
            || event.previous_event_sha256 != prior
            || event.event_sha256 != reminder_event_digest(event)?
            || (index == 0 && event.action != ExecutiveReminderActionKind::Create)
            || (index > 0 && event.action == ExecutiveReminderActionKind::Create)
            || prior_time.is_some_and(|time| event.occurred_at_epoch_ms <= time)
            || (matches!(
                event.state,
                ExecutiveReminderState::Scheduled | ExecutiveReminderState::Snoozed
            ) != event.scheduled_for_epoch_ms.is_some())
            || event
                .scheduled_for_epoch_ms
                .is_some_and(|due| due <= event.occurred_at_epoch_ms)
            || !matches!(
                (prior_state, event.action, event.state),
                (
                    None,
                    ExecutiveReminderActionKind::Create,
                    ExecutiveReminderState::Scheduled
                ) | (
                    Some(ExecutiveReminderState::Scheduled | ExecutiveReminderState::Snoozed),
                    ExecutiveReminderActionKind::Snooze,
                    ExecutiveReminderState::Snoozed
                ) | (
                    Some(
                        ExecutiveReminderState::Scheduled
                            | ExecutiveReminderState::Snoozed
                            | ExecutiveReminderState::Acknowledged
                    ),
                    ExecutiveReminderActionKind::Reschedule,
                    ExecutiveReminderState::Scheduled
                ) | (
                    Some(ExecutiveReminderState::Scheduled | ExecutiveReminderState::Snoozed),
                    ExecutiveReminderActionKind::Acknowledge,
                    ExecutiveReminderState::Acknowledged
                ) | (
                    Some(
                        ExecutiveReminderState::Scheduled
                            | ExecutiveReminderState::Snoozed
                            | ExecutiveReminderState::Acknowledged
                    ),
                    ExecutiveReminderActionKind::Complete,
                    ExecutiveReminderState::Completed
                )
            )
        {
            return Err(ExecutiveAssistantError::IntegrityFailure);
        }
        prior = Some(event.event_sha256.clone());
        prior_state = Some(event.state);
        prior_time = Some(event.occurred_at_epoch_ms);
    }
    let terminal = reminder
        .events
        .last()
        .ok_or(ExecutiveAssistantError::IntegrityFailure)?;
    if terminal.state != reminder.state
        || terminal.scheduled_for_epoch_ms != reminder.scheduled_for_epoch_ms
    {
        return Err(ExecutiveAssistantError::IntegrityFailure);
    }
    Ok(())
}

/// Creates one durable local reminder from an exact source-backed canonical record.
pub fn create_reminder(
    reminder_id: String,
    event_id: String,
    record: &ExecutiveRecord,
    scheduled_for_epoch_ms: u64,
    occurred_at_epoch_ms: u64,
) -> Result<ExecutiveReminder, ExecutiveAssistantError> {
    validate_record(record)?;
    if !valid_identifier(&reminder_id)
        || !valid_identifier(&event_id)
        || occurred_at_epoch_ms == 0
        || scheduled_for_epoch_ms <= occurred_at_epoch_ms
        || !tracker_accepts(ExecutiveTrackerKind::Reminders, record)
    {
        return Err(ExecutiveAssistantError::InvalidInput);
    }
    let mut event = ExecutiveReminderEvent {
        event_id,
        reminder_id: reminder_id.clone(),
        revision: 1,
        action: ExecutiveReminderActionKind::Create,
        state: ExecutiveReminderState::Scheduled,
        scheduled_for_epoch_ms: Some(scheduled_for_epoch_ms),
        occurred_at_epoch_ms,
        previous_event_sha256: None,
        external_effect_allowed: false,
        event_sha256: String::new(),
    };
    event.event_sha256 = reminder_event_digest(&event)?;
    let mut reminder = ExecutiveReminder {
        schema_version: CONTRACT_SCHEMA_VERSION,
        reminder_id,
        record_id: record.record_id.clone(),
        record_sha256: digest_record(record)?,
        source_ids: source_ids(record),
        state: ExecutiveReminderState::Scheduled,
        scheduled_for_epoch_ms: Some(scheduled_for_epoch_ms),
        events: vec![event],
        notification_allowed: false,
        reminder_sha256: String::new(),
    };
    reminder.reminder_sha256 = reminder_digest(&reminder)?;
    verify_reminder(&reminder)?;
    Ok(reminder)
}

/// Applies one exact user action while retaining every prior reminder state in its hash chain.
pub fn apply_reminder_action(
    reminder: &ExecutiveReminder,
    expected_reminder_sha256: &str,
    event_id: String,
    action: ExecutiveReminderActionKind,
    scheduled_for_epoch_ms: Option<u64>,
    occurred_at_epoch_ms: u64,
) -> Result<ExecutiveReminder, ExecutiveAssistantError> {
    verify_reminder(reminder)?;
    let previous = reminder.events.last().expect("verified non-empty history");
    if reminder.reminder_sha256 != expected_reminder_sha256
        || !valid_identifier(&event_id)
        || reminder
            .events
            .iter()
            .any(|event| event.event_id == event_id)
        || occurred_at_epoch_ms <= previous.occurred_at_epoch_ms
        || action == ExecutiveReminderActionKind::Create
        || reminder.state == ExecutiveReminderState::Completed
    {
        return Err(ExecutiveAssistantError::StaleSnapshot);
    }
    let (state, next_due) = match action {
        ExecutiveReminderActionKind::Snooze => {
            if !matches!(
                reminder.state,
                ExecutiveReminderState::Scheduled | ExecutiveReminderState::Snoozed
            ) || scheduled_for_epoch_ms.is_none_or(|due| due <= occurred_at_epoch_ms)
            {
                return Err(ExecutiveAssistantError::InvalidInput);
            }
            (ExecutiveReminderState::Snoozed, scheduled_for_epoch_ms)
        }
        ExecutiveReminderActionKind::Reschedule => {
            if scheduled_for_epoch_ms.is_none_or(|due| due <= occurred_at_epoch_ms) {
                return Err(ExecutiveAssistantError::InvalidInput);
            }
            (ExecutiveReminderState::Scheduled, scheduled_for_epoch_ms)
        }
        ExecutiveReminderActionKind::Acknowledge => {
            if !matches!(
                reminder.state,
                ExecutiveReminderState::Scheduled | ExecutiveReminderState::Snoozed
            ) || scheduled_for_epoch_ms.is_some()
            {
                return Err(ExecutiveAssistantError::InvalidInput);
            }
            (ExecutiveReminderState::Acknowledged, None)
        }
        ExecutiveReminderActionKind::Complete => {
            if scheduled_for_epoch_ms.is_some() {
                return Err(ExecutiveAssistantError::InvalidInput);
            }
            (ExecutiveReminderState::Completed, None)
        }
        ExecutiveReminderActionKind::Create => unreachable!("rejected above"),
    };
    let mut event = ExecutiveReminderEvent {
        event_id,
        reminder_id: reminder.reminder_id.clone(),
        revision: previous
            .revision
            .checked_add(1)
            .ok_or(ExecutiveAssistantError::InvalidInput)?,
        action,
        state,
        scheduled_for_epoch_ms: next_due,
        occurred_at_epoch_ms,
        previous_event_sha256: Some(previous.event_sha256.clone()),
        external_effect_allowed: false,
        event_sha256: String::new(),
    };
    event.event_sha256 = reminder_event_digest(&event)?;
    let mut updated = reminder.clone();
    updated.state = state;
    updated.scheduled_for_epoch_ms = next_due;
    updated.events.push(event);
    updated.reminder_sha256.clear();
    updated.reminder_sha256 = reminder_digest(&updated)?;
    verify_reminder(&updated)?;
    Ok(updated)
}

/// Canonical persistence boundary for durable local reminders.
pub trait ExecutiveReminderStore {
    /// Loads one exact reminder, or returns absence without inventing state.
    fn load_reminder(
        &self,
        reminder_id: &str,
    ) -> Result<Option<ExecutiveReminder>, ExecutiveAssistantError>;

    /// Atomically creates or replaces one reminder against its exact prior digest.
    fn compare_and_store_reminder(
        &mut self,
        expected_reminder_sha256: Option<&str>,
        reminder: &ExecutiveReminder,
    ) -> Result<(), ExecutiveAssistantError>;
}

/// Store-backed lifecycle that survives client or host reconstruction without scheduling effects.
pub struct DurableExecutiveReminderLifecycle<S: ExecutiveReminderStore> {
    store: S,
}

impl<S: ExecutiveReminderStore> DurableExecutiveReminderLifecycle<S> {
    /// Binds the lifecycle to one caller-supplied canonical store.
    #[must_use]
    pub const fn new(store: S) -> Self {
        Self { store }
    }

    /// Creates and atomically persists one exact source-backed reminder.
    pub fn create(
        &mut self,
        reminder_id: String,
        event_id: String,
        record: &ExecutiveRecord,
        scheduled_for_epoch_ms: u64,
        occurred_at_epoch_ms: u64,
    ) -> Result<ExecutiveReminder, ExecutiveAssistantError> {
        if self.store.load_reminder(&reminder_id)?.is_some() {
            return Err(ExecutiveAssistantError::StaleSnapshot);
        }
        let reminder = create_reminder(
            reminder_id,
            event_id,
            record,
            scheduled_for_epoch_ms,
            occurred_at_epoch_ms,
        )?;
        self.store.compare_and_store_reminder(None, &reminder)?;
        Ok(reminder)
    }

    /// Loads and verifies one exact reminder after process or client reconstruction.
    pub fn reopen(&self, reminder_id: &str) -> Result<ExecutiveReminder, ExecutiveAssistantError> {
        let reminder = self
            .store
            .load_reminder(reminder_id)?
            .ok_or(ExecutiveAssistantError::StaleSnapshot)?;
        verify_reminder(&reminder)?;
        Ok(reminder)
    }

    /// Applies and atomically persists one transition against the caller's observed digest.
    pub fn apply(
        &mut self,
        reminder_id: &str,
        expected_reminder_sha256: &str,
        event_id: String,
        action: ExecutiveReminderActionKind,
        scheduled_for_epoch_ms: Option<u64>,
        occurred_at_epoch_ms: u64,
    ) -> Result<ExecutiveReminder, ExecutiveAssistantError> {
        let current = self.reopen(reminder_id)?;
        let updated = apply_reminder_action(
            &current,
            expected_reminder_sha256,
            event_id,
            action,
            scheduled_for_epoch_ms,
            occurred_at_epoch_ms,
        )?;
        self.store
            .compare_and_store_reminder(Some(expected_reminder_sha256), &updated)?;
        Ok(updated)
    }

    /// Returns the persistence adapter for trusted host recomposition or testing.
    #[must_use]
    pub fn into_store(self) -> S {
        self.store
    }
}

fn view_accepts(kind: ExecutiveViewKind, record: &ExecutiveRecord) -> bool {
    match kind {
        ExecutiveViewKind::StartOfCycle => !matches!(
            record.status,
            ExecutiveRecordStatus::Completed
                | ExecutiveRecordStatus::Cancelled
                | ExecutiveRecordStatus::Superseded
        ),
        ExecutiveViewKind::Closeout => matches!(
            record.status,
            ExecutiveRecordStatus::Completed
                | ExecutiveRecordStatus::Active
                | ExecutiveRecordStatus::Waiting
                | ExecutiveRecordStatus::Blocked
        ),
        ExecutiveViewKind::RecurringReview
        | ExecutiveViewKind::BriefingPack
        | ExecutiveViewKind::Audit => true,
        ExecutiveViewKind::Portfolio => record.kind == ExecutiveRecordKind::Project,
        ExecutiveViewKind::Status => !matches!(record.status, ExecutiveRecordStatus::Superseded),
        ExecutiveViewKind::MeetingBrief => record.kind == ExecutiveRecordKind::Meeting,
        ExecutiveViewKind::DecisionBrief => record.kind == ExecutiveRecordKind::Decision,
        ExecutiveViewKind::PersonBrief => record.kind == ExecutiveRecordKind::Person,
        ExecutiveViewKind::OrganizationBrief => record.kind == ExecutiveRecordKind::Organization,
        ExecutiveViewKind::ChangesSince => true,
        ExecutiveViewKind::ForgottenItems => {
            record.due_window == ExecutiveDueWindow::Overdue
                || (matches!(
                    record.kind,
                    ExecutiveRecordKind::Task | ExecutiveRecordKind::Commitment
                ) && record.owner.is_none())
                || (matches!(
                    record.kind,
                    ExecutiveRecordKind::Commitment
                        | ExecutiveRecordKind::Approval
                        | ExecutiveRecordKind::Deadline
                ) && record.due_date.is_none())
                || (record.kind == ExecutiveRecordKind::Project
                    && !record
                        .fields
                        .iter()
                        .any(|field| field.field_code == "next_action"))
        }
    }
}

fn view_section(kind: ExecutiveViewKind, record: &ExecutiveRecord) -> String {
    match kind {
        ExecutiveViewKind::StartOfCycle => match record.kind {
            ExecutiveRecordKind::Schedule | ExecutiveRecordKind::Meeting => "schedule",
            ExecutiveRecordKind::Waiting => "waiting",
            ExecutiveRecordKind::Decision => "decisions",
            ExecutiveRecordKind::Deadline => "deadlines",
            _ => "priorities",
        },
        ExecutiveViewKind::Closeout => match record.status {
            ExecutiveRecordStatus::Completed => "completed",
            ExecutiveRecordStatus::Waiting => "waiting",
            ExecutiveRecordStatus::Blocked => "blocked",
            _ => "unfinished",
        },
        ExecutiveViewKind::RecurringReview => match record.kind {
            ExecutiveRecordKind::Decision => "decisions",
            ExecutiveRecordKind::Meeting => "upcoming-meetings",
            ExecutiveRecordKind::Project => "projects",
            _ if record.due_window == ExecutiveDueWindow::Overdue => "aging",
            _ => "current",
        },
        ExecutiveViewKind::Portfolio => "projects",
        ExecutiveViewKind::Status => "status",
        ExecutiveViewKind::MeetingBrief => "meeting",
        ExecutiveViewKind::DecisionBrief => "decision",
        ExecutiveViewKind::PersonBrief => "professional-context",
        ExecutiveViewKind::OrganizationBrief => "relationships",
        ExecutiveViewKind::BriefingPack => "sources",
        ExecutiveViewKind::ChangesSince => "changed",
        ExecutiveViewKind::ForgottenItems => "needs-review",
        ExecutiveViewKind::Audit => "audit",
    }
    .to_owned()
}

fn view_warnings(record: &ExecutiveRecord) -> Vec<String> {
    let mut warnings = Vec::new();
    if record.evidence_state != ExecutiveEvidenceState::Confirmed {
        warnings.push(format!("evidence.{:?}", record.evidence_state).to_ascii_lowercase());
    }
    if record.owner.is_none()
        && matches!(
            record.kind,
            ExecutiveRecordKind::Task
                | ExecutiveRecordKind::Commitment
                | ExecutiveRecordKind::Approval
        )
    {
        warnings.push("owner.unknown".to_owned());
    }
    if record.due_date.is_none()
        && matches!(
            record.kind,
            ExecutiveRecordKind::Commitment
                | ExecutiveRecordKind::Approval
                | ExecutiveRecordKind::Deadline
        )
    {
        warnings.push("date.unknown".to_owned());
    }
    if record.schedule_conflict {
        warnings.push("schedule.conflict".to_owned());
    }
    warnings.sort();
    warnings
}

fn record_fingerprint(record: &ExecutiveRecord) -> Result<String, ExecutiveAssistantError> {
    digest_record(record)
}

/// Builds one source-backed local brief, cycle view, portfolio view, or audit view.
pub fn build_executive_view(
    snapshot: &ExecutivePortfolioSnapshot,
    request: ExecutiveViewRequest<'_>,
) -> Result<ExecutiveView, ExecutiveAssistantError> {
    verify_snapshot(snapshot)?;
    if !valid_identifier(&request.view_id) {
        return Err(ExecutiveAssistantError::InvalidInput);
    }
    let admitted = request
        .admitted_privacy_classes
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if admitted.len() != request.admitted_privacy_classes.len() {
        return Err(ExecutiveAssistantError::InvalidInput);
    }
    let changed_ids = if request.kind == ExecutiveViewKind::ChangesSince {
        let prior = request
            .prior_snapshot
            .ok_or(ExecutiveAssistantError::PriorSnapshotRequired)?;
        verify_snapshot(prior)?;
        let prior_records = prior
            .current_records
            .iter()
            .map(|record| Ok((record.record_id.clone(), record_fingerprint(record)?)))
            .collect::<Result<BTreeMap<_, _>, ExecutiveAssistantError>>()?;
        Some(
            snapshot
                .current_records
                .iter()
                .filter_map(|record| {
                    let current = record_fingerprint(record).ok()?;
                    (prior_records.get(&record.record_id) != Some(&current))
                        .then(|| record.record_id.clone())
                })
                .collect::<BTreeSet<_>>(),
        )
    } else {
        None
    };
    let privacy_filtered = snapshot
        .current_records
        .iter()
        .any(|record| !admitted.contains(&record.privacy_class));
    let mut items = snapshot
        .current_records
        .iter()
        .filter(|record| {
            admitted.contains(&record.privacy_class)
                && view_accepts(request.kind, record)
                && changed_ids
                    .as_ref()
                    .is_none_or(|ids| ids.contains(&record.record_id))
        })
        .map(|record| ExecutiveViewItem {
            record_id: record.record_id.clone(),
            section: view_section(request.kind, record),
            text: format!("{}: {}", record.title, record.summary),
            evidence_state: record.evidence_state,
            source_ids: source_ids(record),
            warnings: view_warnings(record),
        })
        .collect::<Vec<_>>();
    if items.len() > MAX_ITEMS {
        return Err(ExecutiveAssistantError::InvalidInput);
    }
    items.sort_by(|left, right| {
        left.section
            .cmp(&right.section)
            .then_with(|| left.record_id.cmp(&right.record_id))
    });
    let mut limitations = Vec::new();
    if privacy_filtered {
        limitations.push("privacy.policy.filtered".to_owned());
    }
    if items.is_empty() {
        limitations.push("view.no-admitted-records".to_owned());
    }
    let mut view = ExecutiveView {
        schema_version: CONTRACT_SCHEMA_VERSION,
        view_id: request.view_id,
        kind: request.kind,
        snapshot_sha256: snapshot.snapshot_sha256.clone(),
        items,
        limitations,
        proposal_only: true,
        external_effect_allowed: false,
        view_sha256: String::new(),
    };
    view.view_sha256 = digest_record(&view)?;
    Ok(view)
}

fn verify_draft(draft: &ExecutiveCorrespondenceDraft) -> Result<(), ExecutiveAssistantError> {
    let mut candidate = draft.clone();
    let supplied = candidate.draft_sha256.clone();
    candidate.draft_sha256.clear();
    if candidate.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_identifier(&candidate.draft_id)
        || !valid_identifier(&candidate.tone_profile_id)
        || !valid_text(&candidate.subject, MAX_SHORT_TEXT_BYTES)
        || !valid_text(&candidate.body, MAX_DRAFT_BYTES)
        || candidate.send_allowed
        || candidate.commitment_created
        || !valid_sha256(&supplied)
        || digest_record(&candidate)? != supplied
        || !sorted_unique(&candidate.recipients)
        || !sorted_unique(&candidate.attachment_names)
        || !candidate
            .claims
            .windows(2)
            .all(|window| window[0].claim_id < window[1].claim_id)
        || candidate.claims.iter().any(|claim| {
            !valid_identifier(&claim.claim_id)
                || !valid_text(&claim.statement, MAX_TEXT_BYTES)
                || !sorted_unique(&claim.source_ids)
        })
    {
        return Err(ExecutiveAssistantError::IntegrityFailure);
    }
    Ok(())
}

/// Validates and seals one local draft without recipient selection, commitment, or send authority.
pub fn seal_correspondence_draft(
    mut draft: ExecutiveCorrespondenceDraft,
) -> Result<ExecutiveCorrespondenceDraft, ExecutiveAssistantError> {
    draft.draft_sha256.clear();
    if draft.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_identifier(&draft.draft_id)
        || !valid_identifier(&draft.tone_profile_id)
        || !valid_text(&draft.subject, MAX_SHORT_TEXT_BYTES)
        || !valid_text(&draft.body, MAX_DRAFT_BYTES)
        || draft.send_allowed
        || draft.commitment_created
        || !sorted_unique(&draft.recipients)
        || !sorted_unique(&draft.attachment_names)
        || !draft
            .claims
            .windows(2)
            .all(|window| window[0].claim_id < window[1].claim_id)
    {
        return Err(ExecutiveAssistantError::InvalidInput);
    }
    for claim in &draft.claims {
        if !valid_identifier(&claim.claim_id)
            || !valid_text(&claim.statement, MAX_TEXT_BYTES)
            || !sorted_unique(&claim.source_ids)
        {
            return Err(ExecutiveAssistantError::InvalidInput);
        }
    }
    draft.draft_sha256 = digest_record(&draft)?;
    Ok(draft)
}

fn privacy_rank(class: ExecutivePrivacyClass) -> u8 {
    match class {
        ExecutivePrivacyClass::Ordinary => 0,
        ExecutivePrivacyClass::Private => 1,
        ExecutivePrivacyClass::Confidential => 2,
        ExecutivePrivacyClass::HighlyRestricted => 3,
    }
}

/// Reviews unanswered questions, commitments, dates, attachments, claims, privacy, and names.
pub fn review_correspondence(
    draft: &ExecutiveCorrespondenceDraft,
    context: CorrespondenceReviewContext<'_>,
) -> Result<ExecutiveCorrespondenceReview, ExecutiveAssistantError> {
    verify_draft(draft)?;
    for record in context.records {
        validate_record(record)?;
    }
    if !sorted_unique(context.required_question_ids)
        || !sorted_unique(context.answered_question_ids)
        || !sorted_unique(context.confirmed_names)
        || context
            .answered_question_ids
            .iter()
            .any(|answer| !context.required_question_ids.contains(answer))
    {
        return Err(ExecutiveAssistantError::InvalidInput);
    }
    let source_privacy = context
        .records
        .iter()
        .flat_map(|record| {
            record
                .sources
                .iter()
                .map(move |source| (source.source_id.as_str(), record.privacy_class))
        })
        .collect::<BTreeMap<_, _>>();
    let available_sources = source_privacy.keys().copied().collect::<BTreeSet<_>>();
    let mut issues = Vec::new();
    for question in context.required_question_ids {
        if !context.answered_question_ids.contains(question) {
            issues.push(ExecutiveCorrespondenceIssue {
                kind: ExecutiveCorrespondenceIssueKind::UnansweredQuestion,
                reason_code: "correspondence.question.unanswered".to_owned(),
                claim_id: Some(question.clone()),
            });
        }
    }
    let lowercase = draft.body.to_ascii_lowercase();
    if ["i will", "we will", "i promise", "we commit"]
        .iter()
        .any(|phrase| lowercase.contains(phrase))
    {
        issues.push(ExecutiveCorrespondenceIssue {
            kind: ExecutiveCorrespondenceIssueKind::AccidentalCommitment,
            reason_code: "correspondence.commitment.review-required".to_owned(),
            claim_id: None,
        });
    }
    if ["soon", "tomorrow", "next week", "later"]
        .iter()
        .any(|phrase| lowercase.contains(phrase))
    {
        issues.push(ExecutiveCorrespondenceIssue {
            kind: ExecutiveCorrespondenceIssueKind::UnclearDate,
            reason_code: "correspondence.date.ambiguous".to_owned(),
            claim_id: None,
        });
    }
    if lowercase.contains("attach") && draft.attachment_names.is_empty() {
        issues.push(ExecutiveCorrespondenceIssue {
            kind: ExecutiveCorrespondenceIssueKind::MissingAttachment,
            reason_code: "correspondence.attachment.missing".to_owned(),
            claim_id: None,
        });
    }
    if draft.recipients.is_empty()
        || draft
            .recipients
            .iter()
            .any(|name| !context.confirmed_names.contains(name))
    {
        issues.push(ExecutiveCorrespondenceIssue {
            kind: ExecutiveCorrespondenceIssueKind::UncertainName,
            reason_code: "correspondence.name.unconfirmed".to_owned(),
            claim_id: None,
        });
    }
    for claim in &draft.claims {
        if claim.source_ids.is_empty()
            || claim
                .source_ids
                .iter()
                .any(|source| !available_sources.contains(source.as_str()))
        {
            issues.push(ExecutiveCorrespondenceIssue {
                kind: ExecutiveCorrespondenceIssueKind::UnsupportedClaim,
                reason_code: "correspondence.claim.unsupported".to_owned(),
                claim_id: Some(claim.claim_id.clone()),
            });
        }
        if claim.source_ids.iter().any(|source| {
            source_privacy.get(source.as_str()).is_some_and(|class| {
                privacy_rank(*class) > privacy_rank(context.maximum_privacy_class)
            })
        }) {
            issues.push(ExecutiveCorrespondenceIssue {
                kind: ExecutiveCorrespondenceIssueKind::SensitiveContent,
                reason_code: "correspondence.privacy.exceeds-admitted-class".to_owned(),
                claim_id: Some(claim.claim_id.clone()),
            });
        }
        if claim.evidence_state != ExecutiveEvidenceState::Confirmed {
            issues.push(ExecutiveCorrespondenceIssue {
                kind: ExecutiveCorrespondenceIssueKind::UnsupportedClaim,
                reason_code: "correspondence.claim.not-confirmed".to_owned(),
                claim_id: Some(claim.claim_id.clone()),
            });
        }
    }
    issues.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.claim_id.cmp(&right.claim_id))
            .then_with(|| left.reason_code.cmp(&right.reason_code))
    });
    issues.dedup();
    let mut review = ExecutiveCorrespondenceReview {
        schema_version: CONTRACT_SCHEMA_VERSION,
        draft_sha256: draft.draft_sha256.clone(),
        locally_complete: issues.is_empty(),
        issues,
        send_allowed: false,
        review_sha256: String::new(),
    };
    review.review_sha256 = digest_record(&review)?;
    Ok(review)
}

/// Classifies user-provided local message exports without inbox access or send authority.
pub fn triage_local_messages(
    messages: &[ExecutiveLocalMessage],
    admitted_privacy_classes: &[ExecutivePrivacyClass],
) -> Result<Vec<ExecutiveMessageTriageEntry>, ExecutiveAssistantError> {
    if messages.len() > MAX_RECORDS {
        return Err(ExecutiveAssistantError::InvalidInput);
    }
    let admitted = admitted_privacy_classes
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if admitted.len() != admitted_privacy_classes.len() {
        return Err(ExecutiveAssistantError::InvalidInput);
    }
    let mut ids = BTreeSet::new();
    let all_ids = messages
        .iter()
        .map(|message| message.message_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut output = Vec::new();
    for message in messages {
        if !valid_identifier(&message.message_id)
            || !ids.insert(message.message_id.as_str())
            || message.source.store != ExecutiveSourceStore::LocalMessageExport
            || !validate_source(&message.source)
            || !valid_text(&message.sender, MAX_SHORT_TEXT_BYTES)
            || !valid_text(&message.subject, MAX_SHORT_TEXT_BYTES)
            || !valid_text(&message.body, MAX_DRAFT_BYTES)
            || !admitted.contains(&message.privacy_class)
            || message.duplicate_of.as_deref() == Some(message.message_id.as_str())
            || message
                .duplicate_of
                .as_deref()
                .is_some_and(|value| !all_ids.contains(value))
        {
            return Err(ExecutiveAssistantError::InvalidInput);
        }
        let cue_count = u8::from(message.asks_question)
            + u8::from(message.requests_action)
            + u8::from(message.requests_decision);
        let (classification, reason_code) = if message.duplicate_of.is_some() {
            (
                ExecutiveMessageTriageClass::Duplicate,
                "message.duplicate.confirmed",
            )
        } else if cue_count > 1 && !message.response_confirmed {
            (
                ExecutiveMessageTriageClass::Uncertain,
                "message.cues.conflict",
            )
        } else if message.requests_decision && !message.response_confirmed {
            (
                ExecutiveMessageTriageClass::DecisionRequired,
                "message.decision.requested",
            )
        } else if message.requests_action && !message.response_confirmed {
            (
                ExecutiveMessageTriageClass::ActionRequired,
                "message.action.requested",
            )
        } else if message.asks_question && !message.response_confirmed {
            (
                ExecutiveMessageTriageClass::ResponseRequired,
                "message.response.requested",
            )
        } else if message.response_confirmed {
            (
                ExecutiveMessageTriageClass::Waiting,
                "message.response.confirmed-waiting",
            )
        } else {
            (
                ExecutiveMessageTriageClass::ReferenceOnly,
                "message.reference-only",
            )
        };
        output.push(ExecutiveMessageTriageEntry {
            message_id: message.message_id.clone(),
            classification,
            reason_code: reason_code.to_owned(),
            source_id: message.source.source_id.clone(),
            inbox_access_allowed: false,
            send_allowed: false,
        });
    }
    output.sort_by(|left, right| {
        left.classification
            .cmp(&right.classification)
            .then_with(|| left.message_id.cmp(&right.message_id))
    });
    Ok(output)
}

/// Evaluates retrieval, indexing, retention, and export under the four privacy classes.
#[must_use]
pub fn evaluate_executive_privacy(request: &ExecutivePrivacyRequest) -> ExecutivePrivacyDecision {
    let valid = valid_identifier(&request.record_id)
        && request.retention_days != Some(0)
        && (request.operation == ExecutivePrivacyOperation::Retain)
            == request.retention_days.is_some();
    let (allowed, approval, general_index, separate_index, reason) = if !valid {
        (
            false,
            true,
            false,
            false,
            "executive.privacy.request-invalid",
        )
    } else {
        match (request.privacy_class, request.operation) {
            (
                ExecutivePrivacyClass::Ordinary,
                ExecutivePrivacyOperation::Retrieve | ExecutivePrivacyOperation::Index,
            ) => (true, false, true, false, "executive.privacy.ordinary-local"),
            (ExecutivePrivacyClass::Ordinary, ExecutivePrivacyOperation::Retain) => (
                request.retention_days.is_some_and(|days| days <= 365),
                false,
                true,
                false,
                "executive.privacy.ordinary-retention",
            ),
            (ExecutivePrivacyClass::Ordinary, ExecutivePrivacyOperation::Export) => (
                request.explicit_user_approval,
                true,
                false,
                false,
                "executive.privacy.export-user-review",
            ),
            (ExecutivePrivacyClass::Private, ExecutivePrivacyOperation::Retrieve) => (
                request.exact_source,
                false,
                false,
                true,
                "executive.privacy.private-exact-retrieval",
            ),
            (ExecutivePrivacyClass::Private, ExecutivePrivacyOperation::Index) => (
                true,
                false,
                false,
                true,
                "executive.privacy.private-separated-index",
            ),
            (ExecutivePrivacyClass::Private, ExecutivePrivacyOperation::Retain) => (
                request.retention_days.is_some_and(|days| days <= 90),
                false,
                false,
                true,
                "executive.privacy.private-retention",
            ),
            (ExecutivePrivacyClass::Private, ExecutivePrivacyOperation::Export) => (
                request.exact_source && request.explicit_user_approval,
                true,
                false,
                true,
                "executive.privacy.private-export-review",
            ),
            (ExecutivePrivacyClass::Confidential, ExecutivePrivacyOperation::Retrieve) => (
                request.exact_source && request.explicit_user_approval,
                true,
                false,
                true,
                "executive.privacy.confidential-exact-review",
            ),
            (ExecutivePrivacyClass::Confidential, ExecutivePrivacyOperation::Index) => (
                request.exact_source,
                false,
                false,
                true,
                "executive.privacy.confidential-separated-index",
            ),
            (ExecutivePrivacyClass::Confidential, ExecutivePrivacyOperation::Retain) => (
                request.exact_source && request.retention_days.is_some_and(|days| days <= 30),
                false,
                false,
                true,
                "executive.privacy.confidential-retention",
            ),
            (ExecutivePrivacyClass::Confidential, ExecutivePrivacyOperation::Export) => (
                request.exact_source && request.explicit_user_approval,
                true,
                false,
                true,
                "executive.privacy.confidential-export-review",
            ),
            (ExecutivePrivacyClass::HighlyRestricted, ExecutivePrivacyOperation::Retrieve) => (
                request.exact_source && request.explicit_user_approval,
                true,
                false,
                false,
                "executive.privacy.highly-restricted-exact-review",
            ),
            (
                ExecutivePrivacyClass::HighlyRestricted,
                ExecutivePrivacyOperation::Index | ExecutivePrivacyOperation::Export,
            ) => (
                false,
                true,
                false,
                false,
                "executive.privacy.highly-restricted-prohibited",
            ),
            (ExecutivePrivacyClass::HighlyRestricted, ExecutivePrivacyOperation::Retain) => (
                request.exact_source
                    && request.explicit_user_approval
                    && request.retention_days.is_some_and(|days| days <= 7),
                true,
                false,
                false,
                "executive.privacy.highly-restricted-retention",
            ),
        }
    };
    ExecutivePrivacyDecision {
        record_id: request.record_id.clone(),
        operation: request.operation,
        proposal_allowed: allowed,
        user_approval_required: approval,
        general_index_allowed: general_index,
        separate_index_required: separate_index,
        reason_code: reason.to_owned(),
        effect_performed: false,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use agentmage_kernel_contracts::{
        CONTRACT_SCHEMA_VERSION, ExecutiveCorrespondenceDraft, ExecutiveCorrespondenceKind,
        ExecutiveDraftClaim, ExecutiveDueWindow, ExecutiveEvidenceState, ExecutiveField,
        ExecutiveLocalMessage, ExecutiveMessageTriageClass, ExecutivePrivacyClass,
        ExecutivePrivacyOperation, ExecutivePrivacyRequest, ExecutiveRecord, ExecutiveRecordKind,
        ExecutiveRecordStatus, ExecutiveReminderActionKind, ExecutiveReminderState,
        ExecutiveSourceReference, ExecutiveSourceStore, ExecutiveTrackerKind, ExecutiveViewKind,
    };

    use super::{
        CorrespondenceReviewContext, DurableExecutiveReminderLifecycle, ExecutiveAssistantError,
        ExecutiveReminderStore, ExecutiveViewRequest, apply_reminder_action, build_executive_view,
        build_portfolio_snapshot, build_priority_ranking, build_tracker, create_reminder,
        evaluate_executive_privacy, rank_priorities, reconcile_portfolio, review_correspondence,
        seal_correspondence_draft, triage_local_messages, verify_reminder,
    };

    #[derive(Default)]
    struct MemoryReminderStore(
        std::collections::BTreeMap<String, agentmage_kernel_contracts::ExecutiveReminder>,
    );

    impl ExecutiveReminderStore for MemoryReminderStore {
        fn load_reminder(
            &self,
            reminder_id: &str,
        ) -> Result<Option<agentmage_kernel_contracts::ExecutiveReminder>, ExecutiveAssistantError>
        {
            Ok(self.0.get(reminder_id).cloned())
        }

        fn compare_and_store_reminder(
            &mut self,
            expected_reminder_sha256: Option<&str>,
            reminder: &agentmage_kernel_contracts::ExecutiveReminder,
        ) -> Result<(), ExecutiveAssistantError> {
            let observed = self
                .0
                .get(&reminder.reminder_id)
                .map(|current| current.reminder_sha256.as_str());
            if observed != expected_reminder_sha256 {
                return Err(ExecutiveAssistantError::StaleSnapshot);
            }
            verify_reminder(reminder)?;
            self.0
                .insert(reminder.reminder_id.clone(), reminder.clone());
            Ok(())
        }
    }

    fn source(record_id: &str, store: ExecutiveSourceStore) -> ExecutiveSourceReference {
        ExecutiveSourceReference {
            source_id: format!("source-{record_id}"),
            object_id: format!("object-{record_id}"),
            fragment: Some("heading-1".to_owned()),
            content_sha256: "1".repeat(64),
            observed_revision: Some("revision-1".to_owned()),
            store,
        }
    }

    fn record(
        record_id: &str,
        kind: ExecutiveRecordKind,
        status: ExecutiveRecordStatus,
        evidence_state: ExecutiveEvidenceState,
        store: ExecutiveSourceStore,
    ) -> ExecutiveRecord {
        ExecutiveRecord {
            schema_version: CONTRACT_SCHEMA_VERSION,
            record_id: record_id.to_owned(),
            kind,
            title: format!("Title {record_id}"),
            summary: format!("Summary for {record_id}"),
            status,
            evidence_state,
            owner: Some("Owner One".to_owned()),
            counterparty: Some("Person One".to_owned()),
            project_id: Some("project-one".to_owned()),
            due_date: Some("2026-08-20".to_owned()),
            due_window: ExecutiveDueWindow::WithinSevenDays,
            estimated_effort_minutes: Some(60),
            urgency_bps: 5_000,
            importance_bps: 5_000,
            user_preference_bps: 5_000,
            consequence_bps: 5_000,
            schedule_conflict: false,
            dependency_ids: vec![],
            fields: vec![],
            sources: vec![source(record_id, store)],
            privacy_class: ExecutivePrivacyClass::Ordinary,
            supersedes_record_id: None,
        }
    }

    fn ordinary() -> Vec<ExecutivePrivacyClass> {
        vec![ExecutivePrivacyClass::Ordinary]
    }

    #[test]
    fn priorities_expose_every_component_and_use_stable_ties() {
        let mut first = record(
            "task-a",
            ExecutiveRecordKind::Task,
            ExecutiveRecordStatus::Active,
            ExecutiveEvidenceState::Confirmed,
            ExecutiveSourceStore::PlainFolder,
        );
        first.urgency_bps = 9_000;
        first.importance_bps = 8_000;
        first.due_date = Some("2026-08-15".to_owned());
        first.due_window = ExecutiveDueWindow::Today;
        first.schedule_conflict = true;
        first.dependency_ids = vec!["task-c".to_owned()];
        let mut second = first.clone();
        second.record_id = "task-b".to_owned();
        second.sources = vec![source("task-b", ExecutiveSourceStore::PlainFolder)];
        let completed = record(
            "task-c",
            ExecutiveRecordKind::Task,
            ExecutiveRecordStatus::Completed,
            ExecutiveEvidenceState::Confirmed,
            ExecutiveSourceStore::PlainFolder,
        );
        let ranked = rank_priorities(&[second, completed, first], &ordinary()).expect("ranked");
        assert_eq!(ranked.len(), 2);
        assert_eq!(ranked[0].record_id, "task-a");
        assert_eq!(ranked[1].record_id, "task-b");
        assert_eq!(ranked[0].components.len(), 9);
        assert_eq!(ranked[0].rank, 1);
        assert!(
            ranked[0]
                .limitations
                .contains(&"priority.limit.schedule-conflict".to_owned())
        );
        let ranking = build_priority_ranking(
            "ranking-a".to_owned(),
            &[record(
                "task-ranked",
                ExecutiveRecordKind::Task,
                ExecutiveRecordStatus::Active,
                ExecutiveEvidenceState::Confirmed,
                ExecutiveSourceStore::PlainFolder,
            )],
            &ordinary(),
        )
        .expect("ranking envelope");
        assert_eq!(ranking.method_id, "executive-priority-fixed-v1");
        assert!(!ranking.method_sha256.is_empty());
        assert!(ranking.proposal_only);
        assert!(!ranking.external_effect_allowed);
    }

    #[test]
    fn unknown_inputs_and_unresolved_dependencies_are_visible_not_invented() {
        let mut task = record(
            "task-unknown",
            ExecutiveRecordKind::Task,
            ExecutiveRecordStatus::Blocked,
            ExecutiveEvidenceState::Unknown,
            ExecutiveSourceStore::PlainFolder,
        );
        task.owner = None;
        task.due_date = None;
        task.due_window = ExecutiveDueWindow::Unknown;
        task.estimated_effort_minutes = None;
        task.dependency_ids = vec!["missing-task".to_owned()];
        let ranked = rank_priorities(&[task], &ordinary()).expect("ranked unknown");
        assert_eq!(ranked[0].limitations.len(), 4);
        assert!(
            ranked[0]
                .limitations
                .contains(&"priority.limit.evidence-unknown".to_owned())
        );
        assert!(ranked[0].components.iter().any(|component| {
            component.reason_code == "priority.dependencies.unresolved-1" && component.points < 0
        }));
    }

    #[test]
    fn trackers_and_views_preserve_truth_states_and_have_zero_effects() {
        let commitment = record(
            "commitment-a",
            ExecutiveRecordKind::Commitment,
            ExecutiveRecordStatus::Active,
            ExecutiveEvidenceState::Confirmed,
            ExecutiveSourceStore::PlainFolder,
        );
        let decision = record(
            "decision-a",
            ExecutiveRecordKind::Decision,
            ExecutiveRecordStatus::Proposed,
            ExecutiveEvidenceState::Inferred,
            ExecutiveSourceStore::PlainFolder,
        );
        let tracker = build_tracker(
            "tracker-commitments".to_owned(),
            ExecutiveTrackerKind::Commitments,
            &[decision.clone(), commitment.clone()],
            &ordinary(),
        )
        .expect("tracker");
        assert_eq!(tracker.entries.len(), 1);
        assert!(!tracker.notification_allowed);
        let snapshot =
            build_portfolio_snapshot("snapshot-a".to_owned(), vec![commitment, decision], vec![])
                .expect("snapshot");
        for kind in [
            ExecutiveViewKind::StartOfCycle,
            ExecutiveViewKind::Closeout,
            ExecutiveViewKind::RecurringReview,
            ExecutiveViewKind::Status,
            ExecutiveViewKind::BriefingPack,
            ExecutiveViewKind::Audit,
        ] {
            let view = build_executive_view(
                &snapshot,
                ExecutiveViewRequest {
                    view_id: format!("view-{kind:?}").to_ascii_lowercase(),
                    kind,
                    admitted_privacy_classes: ordinary(),
                    prior_snapshot: None,
                },
            )
            .expect("view");
            assert!(view.proposal_only);
            assert!(!view.external_effect_allowed);
        }
    }

    #[test]
    fn durable_reminder_supports_snooze_reschedule_acknowledge_and_complete() {
        let source_record = record(
            "reminder-task",
            ExecutiveRecordKind::Task,
            ExecutiveRecordStatus::Active,
            ExecutiveEvidenceState::Confirmed,
            ExecutiveSourceStore::PlainFolder,
        );
        let created = create_reminder(
            "reminder-a".to_owned(),
            "reminder-event-1".to_owned(),
            &source_record,
            2_000,
            1_000,
        )
        .expect("create");
        assert!(!created.notification_allowed);
        let snoozed = apply_reminder_action(
            &created,
            &created.reminder_sha256,
            "reminder-event-2".to_owned(),
            ExecutiveReminderActionKind::Snooze,
            Some(4_000),
            2_500,
        )
        .expect("snooze");
        let rescheduled = apply_reminder_action(
            &snoozed,
            &snoozed.reminder_sha256,
            "reminder-event-3".to_owned(),
            ExecutiveReminderActionKind::Reschedule,
            Some(6_000),
            3_000,
        )
        .expect("reschedule");
        let acknowledged = apply_reminder_action(
            &rescheduled,
            &rescheduled.reminder_sha256,
            "reminder-event-4".to_owned(),
            ExecutiveReminderActionKind::Acknowledge,
            None,
            4_000,
        )
        .expect("acknowledge");
        let completed = apply_reminder_action(
            &acknowledged,
            &acknowledged.reminder_sha256,
            "reminder-event-5".to_owned(),
            ExecutiveReminderActionKind::Complete,
            None,
            5_000,
        )
        .expect("complete");
        assert_eq!(completed.state, ExecutiveReminderState::Completed);
        assert_eq!(completed.events.len(), 5);
        assert!(
            completed
                .events
                .iter()
                .all(|event| !event.external_effect_allowed)
        );
        verify_reminder(&completed).expect("restorable chain");
    }

    #[test]
    fn durable_reminder_rejects_stale_replay_tamper_and_automatic_effects() {
        let source_record = record(
            "reminder-secure",
            ExecutiveRecordKind::Commitment,
            ExecutiveRecordStatus::Active,
            ExecutiveEvidenceState::Confirmed,
            ExecutiveSourceStore::Obsidian,
        );
        let created = create_reminder(
            "reminder-secure".to_owned(),
            "reminder-secure-event-1".to_owned(),
            &source_record,
            20_000,
            10_000,
        )
        .expect("create");
        assert_eq!(
            apply_reminder_action(
                &created,
                &"0".repeat(64),
                "reminder-secure-event-2".to_owned(),
                ExecutiveReminderActionKind::Complete,
                None,
                11_000,
            ),
            Err(ExecutiveAssistantError::StaleSnapshot)
        );
        let mut tampered = created.clone();
        tampered.events[0].external_effect_allowed = true;
        assert_eq!(
            verify_reminder(&tampered),
            Err(ExecutiveAssistantError::IntegrityFailure)
        );
        assert_eq!(
            apply_reminder_action(
                &created,
                &created.reminder_sha256,
                "reminder-secure-event-1".to_owned(),
                ExecutiveReminderActionKind::Snooze,
                Some(30_000),
                12_000,
            ),
            Err(ExecutiveAssistantError::StaleSnapshot)
        );
    }

    #[test]
    fn durable_reminder_reopens_and_compare_swaps_across_lifecycle_reconstruction() {
        let source_record = record(
            "reminder-restart",
            ExecutiveRecordKind::Waiting,
            ExecutiveRecordStatus::Waiting,
            ExecutiveEvidenceState::Confirmed,
            ExecutiveSourceStore::PlainFolder,
        );
        let mut first = DurableExecutiveReminderLifecycle::new(MemoryReminderStore::default());
        let created = first
            .create(
                "reminder-restart".to_owned(),
                "reminder-restart-event-1".to_owned(),
                &source_record,
                20_000,
                10_000,
            )
            .expect("persist creation");
        let store = first.into_store();
        let mut reopened = DurableExecutiveReminderLifecycle::new(store);
        assert_eq!(
            reopened
                .reopen("reminder-restart")
                .expect("reopen after reconstruction"),
            created
        );
        let completed = reopened
            .apply(
                "reminder-restart",
                &created.reminder_sha256,
                "reminder-restart-event-2".to_owned(),
                ExecutiveReminderActionKind::Complete,
                None,
                11_000,
            )
            .expect("compare and store");
        assert_eq!(completed.state, ExecutiveReminderState::Completed);
        assert_eq!(
            reopened.apply(
                "reminder-restart",
                &created.reminder_sha256,
                "reminder-restart-event-3".to_owned(),
                ExecutiveReminderActionKind::Reschedule,
                Some(30_000),
                12_000,
            ),
            Err(ExecutiveAssistantError::StaleSnapshot)
        );
    }

    #[test]
    fn every_specialized_brief_preserves_confirmed_inferred_historical_disputed_and_unknown() {
        let fixtures = [
            (
                "meeting-a",
                ExecutiveRecordKind::Meeting,
                ExecutiveEvidenceState::Confirmed,
                ExecutiveViewKind::MeetingBrief,
            ),
            (
                "decision-a",
                ExecutiveRecordKind::Decision,
                ExecutiveEvidenceState::Historical,
                ExecutiveViewKind::DecisionBrief,
            ),
            (
                "person-a",
                ExecutiveRecordKind::Person,
                ExecutiveEvidenceState::Inferred,
                ExecutiveViewKind::PersonBrief,
            ),
            (
                "organization-a",
                ExecutiveRecordKind::Organization,
                ExecutiveEvidenceState::Disputed,
                ExecutiveViewKind::OrganizationBrief,
            ),
            (
                "project-a",
                ExecutiveRecordKind::Project,
                ExecutiveEvidenceState::Unknown,
                ExecutiveViewKind::Portfolio,
            ),
        ];
        let records = fixtures
            .iter()
            .map(|(id, kind, state, _)| {
                record(
                    id,
                    *kind,
                    ExecutiveRecordStatus::Proposed,
                    *state,
                    ExecutiveSourceStore::PlainFolder,
                )
            })
            .collect::<Vec<_>>();
        let snapshot = build_portfolio_snapshot("snapshot-briefs".to_owned(), records, vec![])
            .expect("snapshot");
        for (_, _, expected, kind) in fixtures {
            let view = build_executive_view(
                &snapshot,
                ExecutiveViewRequest {
                    view_id: format!("brief-{kind:?}").to_ascii_lowercase(),
                    kind,
                    admitted_privacy_classes: ordinary(),
                    prior_snapshot: None,
                },
            )
            .expect("brief");
            assert_eq!(view.items.len(), 1);
            assert_eq!(view.items[0].evidence_state, expected);
            assert!(!view.items[0].source_ids.is_empty());
        }
    }

    #[test]
    fn plain_folder_and_obsidian_inputs_have_projection_parity() {
        let plain = record(
            "task-parity",
            ExecutiveRecordKind::Task,
            ExecutiveRecordStatus::Active,
            ExecutiveEvidenceState::Confirmed,
            ExecutiveSourceStore::PlainFolder,
        );
        let obsidian = record(
            "task-parity",
            ExecutiveRecordKind::Task,
            ExecutiveRecordStatus::Active,
            ExecutiveEvidenceState::Confirmed,
            ExecutiveSourceStore::Obsidian,
        );
        let plain_rank =
            rank_priorities(std::slice::from_ref(&plain), &ordinary()).expect("plain rank");
        let obsidian_rank =
            rank_priorities(std::slice::from_ref(&obsidian), &ordinary()).expect("vault rank");
        assert_eq!(plain_rank, obsidian_rank);
        let plain_tracker = build_tracker(
            "tracker-parity".to_owned(),
            ExecutiveTrackerKind::Reminders,
            &[plain],
            &ordinary(),
        )
        .expect("plain tracker");
        let vault_tracker = build_tracker(
            "tracker-parity".to_owned(),
            ExecutiveTrackerKind::Reminders,
            &[obsidian],
            &ordinary(),
        )
        .expect("vault tracker");
        assert_eq!(plain_tracker, vault_tracker);
    }

    #[test]
    fn portfolio_reconciliation_preserves_corrections_supersessions_and_completion_history() {
        let original = record(
            "task-a",
            ExecutiveRecordKind::Task,
            ExecutiveRecordStatus::Active,
            ExecutiveEvidenceState::Confirmed,
            ExecutiveSourceStore::PlainFolder,
        );
        let earlier_decision = record(
            "decision-old",
            ExecutiveRecordKind::Decision,
            ExecutiveRecordStatus::Active,
            ExecutiveEvidenceState::Confirmed,
            ExecutiveSourceStore::PlainFolder,
        );
        let previous = build_portfolio_snapshot(
            "snapshot-1".to_owned(),
            vec![original, earlier_decision],
            vec![],
        )
        .expect("previous");
        let mut corrected = record(
            "task-a",
            ExecutiveRecordKind::Task,
            ExecutiveRecordStatus::Completed,
            ExecutiveEvidenceState::Confirmed,
            ExecutiveSourceStore::PlainFolder,
        );
        corrected.summary = "Corrected and completed".to_owned();
        corrected.sources[0].observed_revision = Some("revision-2".to_owned());
        let mut disputed = record(
            "decision-new",
            ExecutiveRecordKind::Decision,
            ExecutiveRecordStatus::Proposed,
            ExecutiveEvidenceState::Disputed,
            ExecutiveSourceStore::PlainFolder,
        );
        disputed.supersedes_record_id = Some("decision-old".to_owned());
        let next = reconcile_portfolio(
            &previous,
            vec![corrected, disputed],
            "snapshot-2".to_owned(),
        )
        .expect("reconciled");
        assert_eq!(next.current_records.len(), 2);
        assert_eq!(next.history.len(), 2);
        assert!(next.current_records.iter().any(|record| {
            record.record_id == "task-a" && record.status == ExecutiveRecordStatus::Completed
        }));
        assert!(next.current_records.iter().any(|record| {
            record.record_id == "decision-new"
                && record.evidence_state == ExecutiveEvidenceState::Disputed
        }));
        assert!(
            next.history
                .iter()
                .any(|record| record.record_id == "decision-old")
        );
        let changed = build_executive_view(
            &next,
            ExecutiveViewRequest {
                view_id: "view-changed".to_owned(),
                kind: ExecutiveViewKind::ChangesSince,
                admitted_privacy_classes: ordinary(),
                prior_snapshot: Some(&previous),
            },
        )
        .expect("changed");
        assert_eq!(changed.items.len(), 2);
        assert!(changed.items.iter().any(|item| {
            item.record_id == "decision-new"
                && item.warnings.contains(&"evidence.disputed".to_owned())
        }));
    }

    #[test]
    fn stale_snapshots_inferred_decisions_and_sensitive_person_traits_fail_closed() {
        let mut inferred = record(
            "decision-inferred",
            ExecutiveRecordKind::Decision,
            ExecutiveRecordStatus::Active,
            ExecutiveEvidenceState::Inferred,
            ExecutiveSourceStore::PlainFolder,
        );
        assert_eq!(
            build_portfolio_snapshot(
                "snapshot-inferred".to_owned(),
                vec![inferred.clone()],
                vec![]
            ),
            Err(ExecutiveAssistantError::IntegrityFailure)
        );
        inferred.kind = ExecutiveRecordKind::Person;
        inferred.status = ExecutiveRecordStatus::Proposed;
        inferred.fields = vec![ExecutiveField {
            field_code: "religion".to_owned(),
            value: "not admitted professional context".to_owned(),
            evidence_state: ExecutiveEvidenceState::Inferred,
            source_ids: vec!["source-decision-inferred".to_owned()],
        }];
        assert_eq!(
            build_portfolio_snapshot("snapshot-sensitive".to_owned(), vec![inferred], vec![]),
            Err(ExecutiveAssistantError::IntegrityFailure)
        );
        let snapshot =
            build_portfolio_snapshot("snapshot-good".to_owned(), vec![], vec![]).expect("snapshot");
        let mut stale = snapshot.clone();
        stale.snapshot_id = "snapshot-mutated".to_owned();
        assert_eq!(
            build_executive_view(
                &stale,
                ExecutiveViewRequest {
                    view_id: "view-stale".to_owned(),
                    kind: ExecutiveViewKind::Audit,
                    admitted_privacy_classes: ordinary(),
                    prior_snapshot: None,
                }
            ),
            Err(ExecutiveAssistantError::StaleSnapshot)
        );
    }

    fn draft(
        body: &str,
        recipients: Vec<String>,
        claims: Vec<ExecutiveDraftClaim>,
    ) -> ExecutiveCorrespondenceDraft {
        seal_correspondence_draft(ExecutiveCorrespondenceDraft {
            schema_version: CONTRACT_SCHEMA_VERSION,
            draft_id: "draft-1".to_owned(),
            kind: ExecutiveCorrespondenceKind::Email,
            tone_profile_id: "tone-professional".to_owned(),
            recipients,
            subject: "Project update".to_owned(),
            body: body.to_owned(),
            attachment_names: vec![],
            claims,
            send_allowed: false,
            commitment_created: false,
            draft_sha256: String::new(),
        })
        .expect("draft")
    }

    #[test]
    fn correspondence_review_covers_all_seven_checks_and_never_sends() {
        let mut confidential = record(
            "task-draft",
            ExecutiveRecordKind::Task,
            ExecutiveRecordStatus::Active,
            ExecutiveEvidenceState::Confirmed,
            ExecutiveSourceStore::PlainFolder,
        );
        confidential.privacy_class = ExecutivePrivacyClass::Confidential;
        let bad = draft(
            "I will finish this tomorrow. The attachment is included.",
            vec!["Unconfirmed Person".to_owned()],
            vec![
                ExecutiveDraftClaim {
                    claim_id: "claim-1".to_owned(),
                    statement: "Unsupported".to_owned(),
                    evidence_state: ExecutiveEvidenceState::Inferred,
                    source_ids: vec![],
                },
                ExecutiveDraftClaim {
                    claim_id: "claim-2".to_owned(),
                    statement: "Sensitive".to_owned(),
                    evidence_state: ExecutiveEvidenceState::Confirmed,
                    source_ids: vec!["source-task-draft".to_owned()],
                },
            ],
        );
        let review = review_correspondence(
            &bad,
            CorrespondenceReviewContext {
                records: &[confidential],
                required_question_ids: &["question-1".to_owned()],
                answered_question_ids: &[],
                confirmed_names: &["Confirmed Person".to_owned()],
                maximum_privacy_class: ExecutivePrivacyClass::Ordinary,
            },
        )
        .expect("review");
        let kinds = review
            .issues
            .iter()
            .map(|issue| issue.kind)
            .collect::<BTreeSet<_>>();
        assert_eq!(kinds.len(), 7);
        assert!(!review.locally_complete);
        assert!(!review.send_allowed);
        assert!(!bad.send_allowed);
        assert!(!bad.commitment_created);
    }

    #[test]
    fn complete_correspondence_still_has_no_send_authority() {
        let record = record(
            "task-good-draft",
            ExecutiveRecordKind::Task,
            ExecutiveRecordStatus::Active,
            ExecutiveEvidenceState::Confirmed,
            ExecutiveSourceStore::PlainFolder,
        );
        let draft = draft(
            "Thank you for the confirmed project update.",
            vec!["Person One".to_owned()],
            vec![ExecutiveDraftClaim {
                claim_id: "claim-good".to_owned(),
                statement: "The project update is confirmed.".to_owned(),
                evidence_state: ExecutiveEvidenceState::Confirmed,
                source_ids: vec!["source-task-good-draft".to_owned()],
            }],
        );
        let review = review_correspondence(
            &draft,
            CorrespondenceReviewContext {
                records: &[record],
                required_question_ids: &[],
                answered_question_ids: &[],
                confirmed_names: &["Person One".to_owned()],
                maximum_privacy_class: ExecutivePrivacyClass::Ordinary,
            },
        )
        .expect("review");
        assert!(review.locally_complete);
        assert!(review.issues.is_empty());
        assert!(!review.send_allowed);
    }

    fn message(
        id: &str,
        asks: bool,
        action: bool,
        decision: bool,
        responded: bool,
        duplicate_of: Option<&str>,
    ) -> ExecutiveLocalMessage {
        ExecutiveLocalMessage {
            message_id: id.to_owned(),
            source: source(id, ExecutiveSourceStore::LocalMessageExport),
            sender: "Sender One".to_owned(),
            subject: format!("Subject {id}"),
            body: "User-provided local export body".to_owned(),
            asks_question: asks,
            requests_action: action,
            requests_decision: decision,
            response_confirmed: responded,
            duplicate_of: duplicate_of.map(str::to_owned),
            privacy_class: ExecutivePrivacyClass::Ordinary,
        }
    }

    #[test]
    fn local_message_export_triage_covers_every_class_without_inbox_or_send_access() {
        let messages = vec![
            message("message-action", false, true, false, false, None),
            message("message-decision", false, false, true, false, None),
            message(
                "message-duplicate",
                false,
                false,
                false,
                false,
                Some("message-reference"),
            ),
            message("message-reference", false, false, false, false, None),
            message("message-response", true, false, false, false, None),
            message("message-uncertain", true, true, false, false, None),
            message("message-waiting", true, false, false, true, None),
        ];
        let results = triage_local_messages(&messages, &ordinary()).expect("triage");
        let classes = results
            .iter()
            .map(|entry| entry.classification)
            .collect::<BTreeSet<_>>();
        assert_eq!(classes.len(), 7);
        assert!(classes.contains(&ExecutiveMessageTriageClass::Uncertain));
        assert!(
            results
                .iter()
                .all(|entry| !entry.inbox_access_allowed && !entry.send_allowed)
        );
    }

    #[test]
    fn privacy_classes_have_distinct_fail_closed_retrieval_index_retention_and_export_rules() {
        let decide = |class, operation, exact_source, approval, retention_days| {
            evaluate_executive_privacy(&ExecutivePrivacyRequest {
                record_id: "record-private".to_owned(),
                privacy_class: class,
                operation,
                exact_source,
                explicit_user_approval: approval,
                retention_days,
            })
        };
        assert!(
            decide(
                ExecutivePrivacyClass::Ordinary,
                ExecutivePrivacyOperation::Index,
                false,
                false,
                None
            )
            .general_index_allowed
        );
        assert!(
            decide(
                ExecutivePrivacyClass::Private,
                ExecutivePrivacyOperation::Index,
                false,
                false,
                None
            )
            .separate_index_required
        );
        assert!(
            !decide(
                ExecutivePrivacyClass::Confidential,
                ExecutivePrivacyOperation::Retrieve,
                true,
                false,
                None
            )
            .proposal_allowed
        );
        assert!(
            decide(
                ExecutivePrivacyClass::Confidential,
                ExecutivePrivacyOperation::Retrieve,
                true,
                true,
                None
            )
            .proposal_allowed
        );
        assert!(
            !decide(
                ExecutivePrivacyClass::HighlyRestricted,
                ExecutivePrivacyOperation::Index,
                true,
                true,
                None
            )
            .proposal_allowed
        );
        assert!(
            !decide(
                ExecutivePrivacyClass::HighlyRestricted,
                ExecutivePrivacyOperation::Export,
                true,
                true,
                None
            )
            .proposal_allowed
        );
        assert!(
            decide(
                ExecutivePrivacyClass::HighlyRestricted,
                ExecutivePrivacyOperation::Retain,
                true,
                true,
                Some(7)
            )
            .proposal_allowed
        );
        assert!(
            !decide(
                ExecutivePrivacyClass::HighlyRestricted,
                ExecutivePrivacyOperation::Retain,
                true,
                true,
                Some(8)
            )
            .proposal_allowed
        );
        for class in [
            ExecutivePrivacyClass::Ordinary,
            ExecutivePrivacyClass::Private,
            ExecutivePrivacyClass::Confidential,
            ExecutivePrivacyClass::HighlyRestricted,
        ] {
            assert!(
                !decide(class, ExecutivePrivacyOperation::Retrieve, true, true, None)
                    .effect_performed
            );
        }
    }

    #[test]
    fn privacy_scope_and_changes_since_require_exact_admitted_inputs() {
        let mut private = record(
            "task-private",
            ExecutiveRecordKind::Task,
            ExecutiveRecordStatus::Active,
            ExecutiveEvidenceState::Confirmed,
            ExecutiveSourceStore::PlainFolder,
        );
        private.privacy_class = ExecutivePrivacyClass::Private;
        assert_eq!(
            rank_priorities(&[private.clone()], &ordinary()),
            Err(ExecutiveAssistantError::PrivacyDenied)
        );
        let snapshot =
            build_portfolio_snapshot("snapshot-private".to_owned(), vec![private], vec![])
                .expect("snapshot");
        let view = build_executive_view(
            &snapshot,
            ExecutiveViewRequest {
                view_id: "view-filtered".to_owned(),
                kind: ExecutiveViewKind::Status,
                admitted_privacy_classes: ordinary(),
                prior_snapshot: None,
            },
        )
        .expect("filtered view");
        assert!(view.items.is_empty());
        assert_eq!(
            view.limitations,
            vec!["privacy.policy.filtered", "view.no-admitted-records"]
        );
        assert_eq!(
            build_executive_view(
                &snapshot,
                ExecutiveViewRequest {
                    view_id: "view-changes".to_owned(),
                    kind: ExecutiveViewKind::ChangesSince,
                    admitted_privacy_classes: vec![ExecutivePrivacyClass::Private],
                    prior_snapshot: None,
                }
            ),
            Err(ExecutiveAssistantError::PriorSnapshotRequired)
        );
    }
}
