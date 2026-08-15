//! Deterministic meeting preparation, minutes, closeout, and continuity projections.

use std::collections::{BTreeMap, BTreeSet};

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, ExecutiveEvidenceState, ExecutiveSourceReference,
    MeetingAttendanceState, MeetingAttendee, MeetingCloseout, MeetingContinuityItem,
    MeetingContinuityRecord, MeetingContinuityState, MeetingContinuityUpdate, MeetingFieldState,
    MeetingInvitationState, MeetingMinutes, MeetingMinutesItem, MeetingMinutesItemKind,
    MeetingPlanDraft, MeetingPlanItem, MeetingProjectionPrecondition, MeetingTranscriptCleanup,
    MeetingTranscriptSegment,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_SHORT_TEXT_BYTES: usize = 512;
const MAX_TEXT_BYTES: usize = 128 * 1_024;
const MAX_ITEMS: usize = 2_048;
const MAX_SOURCES: usize = 128;

/// Stable fail-closed reason a meeting projection was rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeetingContinuityError {
    /// An identity, text, timestamp, list, or source reference is malformed.
    InvalidInput,
    /// Evidence, ordering, history, or an authority boundary is inconsistent.
    IntegrityFailure,
    /// A supplied sealed record does not match its exact digest.
    StaleRecord,
    /// A required local source, prior record, or validation dependency is unavailable.
    DependencyUnavailable,
    /// The owning coordinator requested cancellation before the projection began.
    Cancelled,
}

impl MeetingContinuityError {
    /// Returns a stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "meeting.input.invalid",
            Self::IntegrityFailure => "meeting.integrity.failed",
            Self::StaleRecord => "meeting.record.stale",
            Self::DependencyUnavailable => "meeting.dependency.unavailable",
            Self::Cancelled => "meeting.cancelled",
        }
    }
}

impl std::fmt::Display for MeetingContinuityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for MeetingContinuityError {}

/// Admits a meeting projection only when dependencies are ready and cancellation is absent.
pub fn admit_meeting_projection(
    precondition: MeetingProjectionPrecondition,
) -> Result<(), MeetingContinuityError> {
    if precondition.cancellation_requested {
        return Err(MeetingContinuityError::Cancelled);
    }
    if !precondition.dependencies_ready {
        return Err(MeetingContinuityError::DependencyUnavailable);
    }
    Ok(())
}

fn sha256(value: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to a string cannot fail");
    }
    output
}

fn digest_record<T: Serialize>(value: &T) -> Result<String, MeetingContinuityError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| MeetingContinuityError::InvalidInput)
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
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
        && value[5..7]
            .parse::<u8>()
            .is_ok_and(|month| (1..=12).contains(&month))
        && value[8..10]
            .parse::<u8>()
            .is_ok_and(|day| (1..=31).contains(&day))
}

fn valid_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 20
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'T'
        && bytes[13] == b':'
        && bytes[16] == b':'
        && bytes[19] == b'Z'
        && bytes.iter().enumerate().all(|(index, byte)| {
            matches!(index, 4 | 7 | 10 | 13 | 16 | 19) || byte.is_ascii_digit()
        })
        && valid_date(&value[..10])
        && value[11..13].parse::<u8>().is_ok_and(|hour| hour <= 23)
        && value[14..16].parse::<u8>().is_ok_and(|minute| minute <= 59)
        && value[17..19].parse::<u8>().is_ok_and(|second| second <= 59)
}

fn sorted_unique(values: &[String]) -> bool {
    values
        .windows(2)
        .all(|window| window[0].as_str() < window[1].as_str())
}

fn source_key(source: &ExecutiveSourceReference) -> (&str, &str, &str) {
    (
        source.source_id.as_str(),
        source.object_id.as_str(),
        source.fragment.as_deref().unwrap_or(""),
    )
}

fn validate_sources(
    sources: &[ExecutiveSourceReference],
) -> Result<BTreeSet<&str>, MeetingContinuityError> {
    if sources.is_empty()
        || sources.len() > MAX_SOURCES
        || !sources
            .windows(2)
            .all(|window| source_key(&window[0]) < source_key(&window[1]))
    {
        return Err(MeetingContinuityError::IntegrityFailure);
    }
    for source in sources {
        if !valid_identifier(&source.source_id)
            || !valid_identifier(&source.object_id)
            || !valid_sha256(&source.content_sha256)
            || source
                .observed_revision
                .as_deref()
                .is_some_and(|value| !valid_identifier(value))
            || source
                .fragment
                .as_deref()
                .is_some_and(|value| !valid_text(value, MAX_SHORT_TEXT_BYTES))
        {
            return Err(MeetingContinuityError::InvalidInput);
        }
    }
    Ok(sources
        .iter()
        .map(|source| source.source_id.as_str())
        .collect())
}

fn validate_source_ids(
    source_ids: &[String],
    available: &BTreeSet<&str>,
) -> Result<(), MeetingContinuityError> {
    if source_ids.is_empty()
        || !sorted_unique(source_ids)
        || source_ids.iter().any(|source_id| {
            !valid_identifier(source_id) || !available.contains(source_id.as_str())
        })
    {
        return Err(MeetingContinuityError::IntegrityFailure);
    }
    Ok(())
}

fn validate_attendee(
    attendee: &MeetingAttendee,
    available: &BTreeSet<&str>,
) -> Result<(), MeetingContinuityError> {
    if !valid_identifier(&attendee.participant_id)
        || !valid_text(&attendee.display_name, MAX_SHORT_TEXT_BYTES)
    {
        return Err(MeetingContinuityError::InvalidInput);
    }
    validate_source_ids(&attendee.source_ids, available)?;
    let invitation_observed = !matches!(
        attendee.invitation_state,
        MeetingInvitationState::NotObserved | MeetingInvitationState::Unknown
    );
    let attendance_observed = !matches!(
        attendee.attendance_state,
        MeetingAttendanceState::NotObserved | MeetingAttendanceState::Unknown
    );
    if invitation_observed
        != (attendee.invitation_evidence_state == ExecutiveEvidenceState::Confirmed)
        || attendance_observed
            != (attendee.attendance_evidence_state == ExecutiveEvidenceState::Confirmed)
    {
        return Err(MeetingContinuityError::IntegrityFailure);
    }
    Ok(())
}

fn validate_plan_item(
    item: &MeetingPlanItem,
    available: &BTreeSet<&str>,
) -> Result<(), MeetingContinuityError> {
    if !valid_identifier(&item.item_id) || !valid_text(&item.text, MAX_TEXT_BYTES) {
        return Err(MeetingContinuityError::InvalidInput);
    }
    validate_source_ids(&item.source_ids, available)
}

fn unique_item_ids<'a>(items: impl IntoIterator<Item = &'a MeetingPlanItem>) -> bool {
    let mut identities = BTreeSet::new();
    items
        .into_iter()
        .all(|item| identities.insert(item.item_id.as_str()))
}

fn validate_plan(draft: &MeetingPlanDraft) -> Result<(), MeetingContinuityError> {
    let available = validate_sources(&draft.sources)?;
    if draft.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_identifier(&draft.draft_id)
        || !valid_identifier(&draft.meeting_id)
        || !valid_text(&draft.purpose, MAX_TEXT_BYTES)
        || draft.participants.len() > MAX_ITEMS
        || draft.topics.is_empty()
        || draft.topics.len() > MAX_ITEMS
        || draft.decision_needs.len() > MAX_ITEMS
        || draft.preparation_items.len() > MAX_ITEMS
        || draft.expected_outputs.len() > MAX_ITEMS
        || !draft.proposal_only
        || draft.external_effects_performed
        || draft.calendar_effect_performed
        || !draft
            .participants
            .windows(2)
            .all(|window| window[0].participant_id < window[1].participant_id)
        || !unique_item_ids(
            draft
                .topics
                .iter()
                .chain(&draft.decision_needs)
                .chain(&draft.preparation_items)
                .chain(&draft.expected_outputs),
        )
    {
        return Err(MeetingContinuityError::IntegrityFailure);
    }
    for attendee in &draft.participants {
        validate_attendee(attendee, &available)?;
    }
    for item in draft
        .topics
        .iter()
        .chain(&draft.decision_needs)
        .chain(&draft.preparation_items)
        .chain(&draft.expected_outputs)
    {
        validate_plan_item(item, &available)?;
    }
    Ok(())
}

/// Validates and seals one local agenda or meeting-request draft.
pub fn seal_meeting_plan_draft(
    mut draft: MeetingPlanDraft,
) -> Result<MeetingPlanDraft, MeetingContinuityError> {
    draft.draft_sha256.clear();
    validate_plan(&draft)?;
    draft.draft_sha256 = digest_record(&draft)?;
    Ok(draft)
}

/// Verifies an exact sealed agenda or meeting-request draft.
pub fn verify_meeting_plan_draft(draft: &MeetingPlanDraft) -> Result<(), MeetingContinuityError> {
    let supplied = draft.draft_sha256.clone();
    let mut candidate = draft.clone();
    candidate.draft_sha256.clear();
    validate_plan(&candidate)?;
    if !valid_sha256(&supplied) || digest_record(&candidate)? != supplied {
        return Err(MeetingContinuityError::StaleRecord);
    }
    Ok(())
}

fn validate_segment(
    segment: &MeetingTranscriptSegment,
    available: &BTreeSet<&str>,
) -> Result<(), MeetingContinuityError> {
    if !valid_identifier(&segment.segment_id)
        || !valid_text(&segment.verbatim_text, MAX_TEXT_BYTES)
        || !valid_text(&segment.cleaned_text, MAX_TEXT_BYTES)
        || segment.attribution_confidence_bps > 10_000
        || segment.unclear_markers.len() > MAX_ITEMS
    {
        return Err(MeetingContinuityError::InvalidInput);
    }
    validate_source_ids(&segment.source_ids, available)?;
    match (&segment.started_at, &segment.ended_at) {
        (Some(start), Some(end))
            if valid_timestamp(start) && valid_timestamp(end) && start <= end => {}
        (None, None) => {}
        _ => return Err(MeetingContinuityError::InvalidInput),
    }
    if segment.speaker_label.is_none()
        != (segment.attribution_evidence_state == ExecutiveEvidenceState::Unknown)
        || (segment.speaker_label.is_none() && segment.attribution_confidence_bps != 0)
        || segment
            .speaker_label
            .as_deref()
            .is_some_and(|value| !valid_text(value, MAX_SHORT_TEXT_BYTES))
    {
        return Err(MeetingContinuityError::IntegrityFailure);
    }
    let bytes = segment.verbatim_text.as_bytes();
    let mut last_end = 0usize;
    for marker in &segment.unclear_markers {
        let start = marker.start_byte as usize;
        let end = marker.end_byte as usize;
        if !valid_identifier(&marker.marker_id)
            || !valid_text(&marker.verbatim_fragment, MAX_TEXT_BYTES)
            || !valid_text(&marker.reason, MAX_SHORT_TEXT_BYTES)
            || start < last_end
            || start >= end
            || end > bytes.len()
            || !segment.verbatim_text.is_char_boundary(start)
            || !segment.verbatim_text.is_char_boundary(end)
            || bytes.get(start..end) != Some(marker.verbatim_fragment.as_bytes())
        {
            return Err(MeetingContinuityError::IntegrityFailure);
        }
        last_end = end;
    }
    Ok(())
}

fn validate_cleanup(cleanup: &MeetingTranscriptCleanup) -> Result<(), MeetingContinuityError> {
    let available = validate_sources(&cleanup.sources)?;
    if cleanup.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_identifier(&cleanup.cleanup_id)
        || !valid_identifier(&cleanup.meeting_id)
        || cleanup.segments.is_empty()
        || cleanup.segments.len() > MAX_ITEMS
        || !cleanup.proposal_only
        || cleanup.source_mutation_performed
    {
        return Err(MeetingContinuityError::IntegrityFailure);
    }
    let mut identities = BTreeSet::new();
    for segment in &cleanup.segments {
        if !identities.insert(segment.segment_id.as_str()) {
            return Err(MeetingContinuityError::IntegrityFailure);
        }
        validate_segment(segment, &available)?;
    }
    Ok(())
}

/// Validates and seals a cleanup proposal while retaining exact verbatim text.
pub fn seal_transcript_cleanup(
    mut cleanup: MeetingTranscriptCleanup,
) -> Result<MeetingTranscriptCleanup, MeetingContinuityError> {
    cleanup.cleanup_sha256.clear();
    validate_cleanup(&cleanup)?;
    cleanup.cleanup_sha256 = digest_record(&cleanup)?;
    Ok(cleanup)
}

/// Verifies an exact sealed note or transcript cleanup.
pub fn verify_transcript_cleanup(
    cleanup: &MeetingTranscriptCleanup,
) -> Result<(), MeetingContinuityError> {
    let supplied = cleanup.cleanup_sha256.clone();
    let mut candidate = cleanup.clone();
    candidate.cleanup_sha256.clear();
    validate_cleanup(&candidate)?;
    if !valid_sha256(&supplied) || digest_record(&candidate)? != supplied {
        return Err(MeetingContinuityError::StaleRecord);
    }
    Ok(())
}

fn validate_optional_field(
    value: Option<&str>,
    state: MeetingFieldState,
    is_date: bool,
) -> Result<(), MeetingContinuityError> {
    if value.is_none() != (state == MeetingFieldState::Unknown) {
        return Err(MeetingContinuityError::IntegrityFailure);
    }
    if let Some(value) = value {
        let valid = if is_date {
            valid_date(value)
        } else {
            valid_text(value, MAX_SHORT_TEXT_BYTES)
        };
        if !valid {
            return Err(MeetingContinuityError::InvalidInput);
        }
    }
    Ok(())
}

fn validate_minutes_item(
    item: &MeetingMinutesItem,
    available: &BTreeSet<&str>,
) -> Result<(), MeetingContinuityError> {
    if !valid_identifier(&item.item_id)
        || !valid_text(&item.text, MAX_TEXT_BYTES)
        || item
            .carried_from_item_id
            .as_deref()
            .is_some_and(|value| !valid_identifier(value) || value == item.item_id)
    {
        return Err(MeetingContinuityError::InvalidInput);
    }
    if item.kind == MeetingMinutesItemKind::ConfirmedDecision
        && item.evidence_state != ExecutiveEvidenceState::Confirmed
        || item.kind == MeetingMinutesItemKind::ProposedDecision
            && item.evidence_state == ExecutiveEvidenceState::Confirmed
    {
        return Err(MeetingContinuityError::IntegrityFailure);
    }
    validate_optional_field(item.owner.as_deref(), item.owner_state, false)?;
    validate_optional_field(item.due_date.as_deref(), item.due_date_state, true)?;
    validate_source_ids(&item.source_ids, available)
}

fn validate_minutes(minutes: &MeetingMinutes) -> Result<(), MeetingContinuityError> {
    let available = validate_sources(&minutes.sources)?;
    if minutes.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_identifier(&minutes.minutes_id)
        || !valid_identifier(&minutes.meeting_id)
        || minutes.items.is_empty()
        || minutes.items.len() > MAX_ITEMS
        || minutes.participants.len() > MAX_ITEMS
        || !minutes.proposal_only
        || minutes.commitment_effect_performed
        || minutes.communication_effect_performed
        || minutes.calendar_effect_performed
        || !minutes
            .participants
            .windows(2)
            .all(|window| window[0].participant_id < window[1].participant_id)
    {
        return Err(MeetingContinuityError::IntegrityFailure);
    }
    for attendee in &minutes.participants {
        validate_attendee(attendee, &available)?;
    }
    let mut identities = BTreeSet::new();
    for item in &minutes.items {
        if !identities.insert(item.item_id.as_str()) {
            return Err(MeetingContinuityError::IntegrityFailure);
        }
        validate_minutes_item(item, &available)?;
    }
    Ok(())
}

/// Validates and seals reviewable local minutes without creating assignments or events.
pub fn seal_meeting_minutes(
    mut minutes: MeetingMinutes,
) -> Result<MeetingMinutes, MeetingContinuityError> {
    minutes.minutes_sha256.clear();
    validate_minutes(&minutes)?;
    minutes.minutes_sha256 = digest_record(&minutes)?;
    Ok(minutes)
}

/// Verifies exact sealed local minutes.
pub fn verify_meeting_minutes(minutes: &MeetingMinutes) -> Result<(), MeetingContinuityError> {
    let supplied = minutes.minutes_sha256.clone();
    let mut candidate = minutes.clone();
    candidate.minutes_sha256.clear();
    validate_minutes(&candidate)?;
    if !valid_sha256(&supplied) || digest_record(&candidate)? != supplied {
        return Err(MeetingContinuityError::StaleRecord);
    }
    Ok(())
}

fn validate_continuity(record: &MeetingContinuityRecord) -> Result<(), MeetingContinuityError> {
    if record.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_identifier(&record.continuity_record_id)
        || !valid_identifier(&record.series_id)
        || !valid_identifier(&record.meeting_id)
        || record
            .prior_record_id
            .as_deref()
            .is_some_and(|value| !valid_identifier(value) || value == record.continuity_record_id)
        || record.items.len() > MAX_ITEMS
        || !valid_text(&record.follow_up_draft, MAX_TEXT_BYTES)
        || !record.proposal_only
        || record.communication_effect_performed
        || record.source_mutation_performed
        || !record
            .items
            .windows(2)
            .all(|window| window[0].continuity_item_id < window[1].continuity_item_id)
    {
        return Err(MeetingContinuityError::IntegrityFailure);
    }
    for item in &record.items {
        if !valid_identifier(&item.continuity_item_id)
            || !valid_identifier(&item.originating_meeting_id)
            || !valid_identifier(&item.source_item_id)
            || !valid_text(&item.text, MAX_TEXT_BYTES)
            || !sorted_unique(&item.history_item_ids)
            || !sorted_unique(&item.source_ids)
            || item.history_item_ids.is_empty()
            || item.source_ids.is_empty()
        {
            return Err(MeetingContinuityError::InvalidInput);
        }
        validate_optional_field(item.owner.as_deref(), item.owner_state, false)?;
        validate_optional_field(item.due_date.as_deref(), item.due_date_state, true)?;
    }
    Ok(())
}

/// Verifies an exact recurring-meeting continuity record.
pub fn verify_meeting_continuity(
    record: &MeetingContinuityRecord,
) -> Result<(), MeetingContinuityError> {
    let supplied = record.continuity_sha256.clone();
    let mut candidate = record.clone();
    candidate.continuity_sha256.clear();
    validate_continuity(&candidate)?;
    if !valid_sha256(&supplied) || digest_record(&candidate)? != supplied {
        return Err(MeetingContinuityError::StaleRecord);
    }
    Ok(())
}

fn continuity_kind(kind: MeetingMinutesItemKind) -> bool {
    matches!(
        kind,
        MeetingMinutesItemKind::Action
            | MeetingMinutesItemKind::Question
            | MeetingMinutesItemKind::Risk
    )
}

/// Builds immutable recurring continuity and a local follow-up draft from sealed minutes.
pub fn build_meeting_continuity(
    continuity_record_id: String,
    series_id: String,
    minutes: &MeetingMinutes,
    prior: Option<&MeetingContinuityRecord>,
    updates: &[MeetingContinuityUpdate],
    follow_up_draft: String,
) -> Result<MeetingContinuityRecord, MeetingContinuityError> {
    verify_meeting_minutes(minutes)?;
    if !valid_identifier(&continuity_record_id)
        || !valid_identifier(&series_id)
        || !valid_text(&follow_up_draft, MAX_TEXT_BYTES)
        || updates.len() > MAX_ITEMS
        || !updates
            .windows(2)
            .all(|window| window[0].continuity_item_id < window[1].continuity_item_id)
    {
        return Err(MeetingContinuityError::InvalidInput);
    }
    if let Some(record) = prior {
        verify_meeting_continuity(record)?;
        if record.series_id != series_id || record.meeting_id == minutes.meeting_id {
            return Err(MeetingContinuityError::IntegrityFailure);
        }
    }
    let current_items = minutes
        .items
        .iter()
        .map(|item| (item.item_id.as_str(), item))
        .collect::<BTreeMap<_, _>>();
    let current_sources = minutes
        .sources
        .iter()
        .map(|source| source.source_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut output = prior
        .map(|record| {
            record
                .items
                .iter()
                .cloned()
                .map(|item| (item.continuity_item_id.clone(), item))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    for update in updates {
        let existing = output
            .get_mut(&update.continuity_item_id)
            .ok_or(MeetingContinuityError::IntegrityFailure)?;
        let source = current_items
            .get(update.source_item_id.as_str())
            .ok_or(MeetingContinuityError::IntegrityFailure)?;
        if !continuity_kind(source.kind)
            || update.state == MeetingContinuityState::Open
            || update.source_ids != source.source_ids
            || update
                .source_ids
                .iter()
                .any(|source_id| !current_sources.contains(source_id.as_str()))
        {
            return Err(MeetingContinuityError::IntegrityFailure);
        }
        existing.state = update.state;
        existing.source_item_id = source.item_id.clone();
        existing.text = source.text.clone();
        existing.owner.clone_from(&source.owner);
        existing.owner_state = source.owner_state;
        existing.due_date.clone_from(&source.due_date);
        existing.due_date_state = source.due_date_state;
        existing.history_item_ids.push(source.item_id.clone());
        existing.history_item_ids.sort();
        existing.history_item_ids.dedup();
        existing.source_ids.extend(source.source_ids.clone());
        existing.source_ids.sort();
        existing.source_ids.dedup();
    }
    for item in minutes
        .items
        .iter()
        .filter(|item| continuity_kind(item.kind))
    {
        if updates
            .iter()
            .any(|update| update.source_item_id == item.item_id)
        {
            continue;
        }
        if let Some(prior_item_id) = &item.carried_from_item_id {
            let existing = output
                .values_mut()
                .find(|candidate| candidate.history_item_ids.contains(prior_item_id))
                .ok_or(MeetingContinuityError::IntegrityFailure)?;
            if existing.state != MeetingContinuityState::Open {
                return Err(MeetingContinuityError::IntegrityFailure);
            }
            existing.source_item_id = item.item_id.clone();
            existing.text = item.text.clone();
            existing.owner.clone_from(&item.owner);
            existing.owner_state = item.owner_state;
            existing.due_date.clone_from(&item.due_date);
            existing.due_date_state = item.due_date_state;
            existing.history_item_ids.push(item.item_id.clone());
            existing.history_item_ids.sort();
            existing.history_item_ids.dedup();
            existing.source_ids.extend(item.source_ids.clone());
            existing.source_ids.sort();
            existing.source_ids.dedup();
            continue;
        }
        let continuity_item_id = format!("continuity:{}", item.item_id);
        if output.contains_key(&continuity_item_id) {
            return Err(MeetingContinuityError::IntegrityFailure);
        }
        output.insert(
            continuity_item_id.clone(),
            MeetingContinuityItem {
                continuity_item_id,
                originating_meeting_id: minutes.meeting_id.clone(),
                source_item_id: item.item_id.clone(),
                state: MeetingContinuityState::Open,
                text: item.text.clone(),
                owner: item.owner.clone(),
                owner_state: item.owner_state,
                due_date: item.due_date.clone(),
                due_date_state: item.due_date_state,
                history_item_ids: vec![item.item_id.clone()],
                source_ids: item.source_ids.clone(),
            },
        );
    }
    let mut record = MeetingContinuityRecord {
        schema_version: CONTRACT_SCHEMA_VERSION,
        continuity_record_id,
        series_id,
        meeting_id: minutes.meeting_id.clone(),
        prior_record_id: prior.map(|record| record.continuity_record_id.clone()),
        items: output.into_values().collect(),
        follow_up_draft,
        continuity_sha256: String::new(),
        proposal_only: true,
        communication_effect_performed: false,
        source_mutation_performed: false,
    };
    validate_continuity(&record)?;
    record.continuity_sha256 = digest_record(&record)?;
    Ok(record)
}

/// Derives content-free closeout counts from exact sealed minutes and continuity.
pub fn build_meeting_closeout(
    minutes: &MeetingMinutes,
    continuity: &MeetingContinuityRecord,
) -> Result<MeetingCloseout, MeetingContinuityError> {
    verify_meeting_minutes(minutes)?;
    verify_meeting_continuity(continuity)?;
    if minutes.meeting_id != continuity.meeting_id {
        return Err(MeetingContinuityError::IntegrityFailure);
    }
    let count = |kind| {
        minutes
            .items
            .iter()
            .filter(|item| item.kind == kind)
            .count() as u32
    };
    Ok(MeetingCloseout {
        schema_version: CONTRACT_SCHEMA_VERSION,
        meeting_id: minutes.meeting_id.clone(),
        minutes_sha256: minutes.minutes_sha256.clone(),
        item_count: minutes.items.len() as u32,
        confirmed_decision_count: count(MeetingMinutesItemKind::ConfirmedDecision),
        proposed_decision_count: count(MeetingMinutesItemKind::ProposedDecision),
        action_count: count(MeetingMinutesItemKind::Action),
        unknown_owner_action_count: minutes
            .items
            .iter()
            .filter(|item| {
                item.kind == MeetingMinutesItemKind::Action
                    && item.owner_state == MeetingFieldState::Unknown
            })
            .count() as u32,
        unknown_date_action_count: minutes
            .items
            .iter()
            .filter(|item| {
                item.kind == MeetingMinutesItemKind::Action
                    && item.due_date_state == MeetingFieldState::Unknown
            })
            .count() as u32,
        open_continuity_count: continuity
            .items
            .iter()
            .filter(|item| item.state == MeetingContinuityState::Open)
            .count() as u32,
        external_effects_performed: false,
    })
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        ExecutivePrivacyClass, ExecutiveSourceStore, MeetingDraftKind, MeetingPlanItem,
        MeetingTextSourceKind, MeetingTranscriptSegment, MeetingUnclearMarker,
    };

    use super::*;

    fn source() -> ExecutiveSourceReference {
        ExecutiveSourceReference {
            source_id: "source-1".to_owned(),
            object_id: "object-1".to_owned(),
            fragment: Some("Meeting".to_owned()),
            content_sha256: "a".repeat(64),
            observed_revision: Some("revision-1".to_owned()),
            store: agentmage_kernel_contracts::ExecutiveSourceStore::PlainFolder,
        }
    }

    fn attendee() -> MeetingAttendee {
        MeetingAttendee {
            participant_id: "person-1".to_owned(),
            display_name: "Person One".to_owned(),
            invitation_state: MeetingInvitationState::Accepted,
            invitation_evidence_state: ExecutiveEvidenceState::Confirmed,
            attendance_state: MeetingAttendanceState::Unknown,
            attendance_evidence_state: ExecutiveEvidenceState::Unknown,
            source_ids: vec!["source-1".to_owned()],
        }
    }

    fn plan(kind: MeetingDraftKind) -> MeetingPlanDraft {
        MeetingPlanDraft {
            schema_version: CONTRACT_SCHEMA_VERSION,
            draft_id: "draft-1".to_owned(),
            meeting_id: "meeting-1".to_owned(),
            kind,
            purpose: "Decide the bounded next step.".to_owned(),
            participants: vec![attendee()],
            topics: vec![MeetingPlanItem {
                item_id: "topic-1".to_owned(),
                text: "Review current evidence.".to_owned(),
                source_ids: vec!["source-1".to_owned()],
            }],
            decision_needs: vec![],
            preparation_items: vec![],
            expected_outputs: vec![],
            sources: vec![source()],
            draft_sha256: String::new(),
            proposal_only: true,
            external_effects_performed: false,
            calendar_effect_performed: false,
        }
    }

    fn minutes(meeting_id: &str, carried_from: Option<&str>) -> MeetingMinutes {
        MeetingMinutes {
            schema_version: CONTRACT_SCHEMA_VERSION,
            minutes_id: format!("minutes-{meeting_id}"),
            meeting_id: meeting_id.to_owned(),
            participants: vec![attendee()],
            items: vec![
                MeetingMinutesItem {
                    item_id: format!("action-{meeting_id}"),
                    kind: MeetingMinutesItemKind::Action,
                    text: "Investigate the open question.".to_owned(),
                    evidence_state: ExecutiveEvidenceState::Confirmed,
                    owner: None,
                    owner_state: MeetingFieldState::Unknown,
                    due_date: None,
                    due_date_state: MeetingFieldState::Unknown,
                    carried_from_item_id: carried_from.map(str::to_owned),
                    source_ids: vec!["source-1".to_owned()],
                },
                MeetingMinutesItem {
                    item_id: format!("proposed-{meeting_id}"),
                    kind: MeetingMinutesItemKind::ProposedDecision,
                    text: "Consider the proposed option.".to_owned(),
                    evidence_state: ExecutiveEvidenceState::Inferred,
                    owner: None,
                    owner_state: MeetingFieldState::Unknown,
                    due_date: None,
                    due_date_state: MeetingFieldState::Unknown,
                    carried_from_item_id: None,
                    source_ids: vec!["source-1".to_owned()],
                },
            ],
            sources: vec![source()],
            privacy_class: ExecutivePrivacyClass::Private,
            minutes_sha256: String::new(),
            proposal_only: true,
            commitment_effect_performed: false,
            communication_effect_performed: false,
            calendar_effect_performed: false,
        }
    }

    #[test]
    fn agenda_and_request_drafts_are_hash_bound_and_authority_free() {
        for kind in [MeetingDraftKind::Agenda, MeetingDraftKind::MeetingRequest] {
            let sealed = seal_meeting_plan_draft(plan(kind)).expect("sealed plan");
            verify_meeting_plan_draft(&sealed).expect("verified plan");
            assert!(sealed.proposal_only);
            assert!(!sealed.external_effects_performed);
            assert!(!sealed.calendar_effect_performed);
        }
    }

    #[test]
    fn invitation_and_attendance_states_cannot_be_inferred() {
        let mut candidate = plan(MeetingDraftKind::Agenda);
        candidate.participants[0].attendance_state = MeetingAttendanceState::Attended;
        candidate.participants[0].attendance_evidence_state = ExecutiveEvidenceState::Inferred;
        assert_eq!(
            seal_meeting_plan_draft(candidate).expect_err("inferred attendance"),
            MeetingContinuityError::IntegrityFailure
        );
    }

    #[test]
    fn transcript_cleanup_preserves_verbatim_offsets_and_unknown_attribution() {
        let cleanup = MeetingTranscriptCleanup {
            schema_version: CONTRACT_SCHEMA_VERSION,
            cleanup_id: "cleanup-1".to_owned(),
            meeting_id: "meeting-1".to_owned(),
            segments: vec![MeetingTranscriptSegment {
                segment_id: "segment-1".to_owned(),
                source_kind: MeetingTextSourceKind::Transcript,
                verbatim_text: "Decision maybe later".to_owned(),
                cleaned_text: "Decision: [unclear] later.".to_owned(),
                started_at: Some("2026-08-15T12:00:00Z".to_owned()),
                ended_at: Some("2026-08-15T12:00:10Z".to_owned()),
                speaker_label: None,
                attribution_confidence_bps: 0,
                attribution_evidence_state: ExecutiveEvidenceState::Unknown,
                unclear_markers: vec![MeetingUnclearMarker {
                    marker_id: "unclear-1".to_owned(),
                    start_byte: 9,
                    end_byte: 14,
                    verbatim_fragment: "maybe".to_owned(),
                    reason: "The source does not establish intent.".to_owned(),
                }],
                source_ids: vec!["source-1".to_owned()],
            }],
            sources: vec![source()],
            cleanup_sha256: String::new(),
            proposal_only: true,
            source_mutation_performed: false,
        };
        let sealed = seal_transcript_cleanup(cleanup).expect("sealed cleanup");
        verify_transcript_cleanup(&sealed).expect("verified cleanup");
        assert_eq!(sealed.segments[0].verbatim_text, "Decision maybe later");
        assert!(!sealed.source_mutation_performed);
    }

    #[test]
    fn transcript_cleanup_rejects_a_marker_that_changes_the_source_fragment() {
        let mut marker_plan = plan(MeetingDraftKind::Agenda);
        marker_plan.sources[0].store = ExecutiveSourceStore::PlainFolder;
        let mut cleanup = MeetingTranscriptCleanup {
            schema_version: CONTRACT_SCHEMA_VERSION,
            cleanup_id: "cleanup-1".to_owned(),
            meeting_id: "meeting-1".to_owned(),
            segments: vec![MeetingTranscriptSegment {
                segment_id: "segment-1".to_owned(),
                source_kind: MeetingTextSourceKind::LiveNote,
                verbatim_text: "unclear".to_owned(),
                cleaned_text: "[unclear]".to_owned(),
                started_at: None,
                ended_at: None,
                speaker_label: None,
                attribution_confidence_bps: 0,
                attribution_evidence_state: ExecutiveEvidenceState::Unknown,
                unclear_markers: vec![MeetingUnclearMarker {
                    marker_id: "marker-1".to_owned(),
                    start_byte: 0,
                    end_byte: 7,
                    verbatim_fragment: "changed".to_owned(),
                    reason: "Unclear source.".to_owned(),
                }],
                source_ids: vec!["source-1".to_owned()],
            }],
            sources: marker_plan.sources,
            cleanup_sha256: String::new(),
            proposal_only: true,
            source_mutation_performed: false,
        };
        assert!(seal_transcript_cleanup(cleanup.clone()).is_err());
        cleanup.segments[0].unclear_markers[0].verbatim_fragment = "unclear".to_owned();
        assert!(seal_transcript_cleanup(cleanup).is_ok());
    }

    #[test]
    fn minutes_keep_missing_owner_and_date_unknown() {
        let sealed = seal_meeting_minutes(minutes("meeting-1", None)).expect("sealed minutes");
        verify_meeting_minutes(&sealed).expect("verified minutes");
        assert_eq!(sealed.items[0].owner_state, MeetingFieldState::Unknown);
        assert_eq!(sealed.items[0].due_date_state, MeetingFieldState::Unknown);

        let mut guessed = minutes("meeting-1", None);
        guessed.items[0].owner = Some("Guessed Owner".to_owned());
        assert_eq!(
            seal_meeting_minutes(guessed).expect_err("guessed owner"),
            MeetingContinuityError::IntegrityFailure
        );
    }

    #[test]
    fn proposed_decision_cannot_be_promoted_to_confirmed() {
        let mut candidate = minutes("meeting-1", None);
        candidate.items[1].evidence_state = ExecutiveEvidenceState::Confirmed;
        assert_eq!(
            seal_meeting_minutes(candidate).expect_err("promoted proposal"),
            MeetingContinuityError::IntegrityFailure
        );
    }

    #[test]
    fn recurring_continuity_preserves_history_and_never_sends() {
        let first_minutes = seal_meeting_minutes(minutes("meeting-1", None)).expect("minutes");
        let first = build_meeting_continuity(
            "record-1".to_owned(),
            "series-1".to_owned(),
            &first_minutes,
            None,
            &[],
            "Review and edit the follow-up.".to_owned(),
        )
        .expect("first continuity");
        let second_minutes = seal_meeting_minutes(minutes("meeting-2", Some("action-meeting-1")))
            .expect("second minutes");
        let second = build_meeting_continuity(
            "record-2".to_owned(),
            "series-1".to_owned(),
            &second_minutes,
            Some(&first),
            &[],
            "Review the next follow-up draft.".to_owned(),
        )
        .expect("second continuity");
        let carried = second
            .items
            .iter()
            .find(|item| item.continuity_item_id == "continuity:action-meeting-1")
            .expect("carried item");
        assert_eq!(
            carried.history_item_ids,
            ["action-meeting-1", "action-meeting-2"]
        );
        assert!(!second.communication_effect_performed);
        assert!(!second.source_mutation_performed);
    }

    #[test]
    fn exact_source_update_can_close_a_continuity_item() {
        let first_minutes = seal_meeting_minutes(minutes("meeting-1", None)).expect("minutes");
        let first = build_meeting_continuity(
            "record-1".to_owned(),
            "series-1".to_owned(),
            &first_minutes,
            None,
            &[],
            "Review the follow-up.".to_owned(),
        )
        .expect("continuity");
        let second_minutes = seal_meeting_minutes(minutes("meeting-2", None)).expect("minutes");
        let updates = [MeetingContinuityUpdate {
            continuity_item_id: "continuity:action-meeting-1".to_owned(),
            state: MeetingContinuityState::Completed,
            source_item_id: "action-meeting-2".to_owned(),
            source_ids: vec!["source-1".to_owned()],
        }];
        let second = build_meeting_continuity(
            "record-2".to_owned(),
            "series-1".to_owned(),
            &second_minutes,
            Some(&first),
            &updates,
            "Review the follow-up.".to_owned(),
        )
        .expect("updated continuity");
        let closed = second
            .items
            .iter()
            .find(|item| item.continuity_item_id == "continuity:action-meeting-1")
            .expect("closed item");
        assert_eq!(closed.state, MeetingContinuityState::Completed);
        let closeout = build_meeting_closeout(&second_minutes, &second).expect("closeout");
        assert_eq!(closeout.unknown_owner_action_count, 1);
        assert!(!closeout.external_effects_performed);
    }

    #[test]
    fn stale_minutes_and_continuity_are_rejected() {
        let mut sealed = seal_meeting_minutes(minutes("meeting-1", None)).expect("minutes");
        sealed.items[0].text.push_str(" changed");
        assert_eq!(
            verify_meeting_minutes(&sealed).expect_err("stale minutes"),
            MeetingContinuityError::StaleRecord
        );
    }

    #[test]
    fn dependency_failure_and_cancellation_fail_before_projection() {
        assert_eq!(
            admit_meeting_projection(MeetingProjectionPrecondition {
                dependencies_ready: false,
                cancellation_requested: false,
            })
            .expect_err("missing dependency"),
            MeetingContinuityError::DependencyUnavailable
        );
        assert_eq!(
            admit_meeting_projection(MeetingProjectionPrecondition {
                dependencies_ready: true,
                cancellation_requested: true,
            })
            .expect_err("cancelled"),
            MeetingContinuityError::Cancelled
        );
        assert_eq!(
            admit_meeting_projection(MeetingProjectionPrecondition {
                dependencies_ready: false,
                cancellation_requested: true,
            })
            .expect_err("sticky cancellation wins"),
            MeetingContinuityError::Cancelled
        );
        admit_meeting_projection(MeetingProjectionPrecondition {
            dependencies_ready: true,
            cancellation_requested: false,
        })
        .expect("admitted");
    }
}
