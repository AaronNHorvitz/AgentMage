//! Authority-free meeting records, minutes, and recurring continuity contracts.

use crate::executive::{ExecutiveEvidenceState, ExecutivePrivacyClass, ExecutiveSourceReference};

/// Explicit dependency and cancellation state checked before a meeting projection begins.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeetingProjectionPrecondition {
    /// True only after every declared local input dependency is available and verified.
    pub dependencies_ready: bool,
    /// Sticky cancellation state supplied by the owning coordinator.
    pub cancellation_requested: bool,
}

/// Closed local draft type for meeting preparation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeetingDraftKind {
    /// A local agenda proposal.
    Agenda,
    /// A local meeting-request proposal that cannot be sent or scheduled.
    MeetingRequest,
}

/// Closed invitation-response state that is never inferred from attendance or silence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeetingInvitationState {
    /// No invitation state was observed.
    NotObserved,
    /// An approved source reports that an invitation was issued.
    Invited,
    /// An approved source explicitly confirms acceptance.
    Accepted,
    /// An approved source explicitly confirms a decline.
    Declined,
    /// An approved source explicitly reports a tentative response.
    Tentative,
    /// The current response cannot be established.
    Unknown,
}

/// Closed attendance state that is never derived from an invitation response.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeetingAttendanceState {
    /// No attendance observation is available.
    NotObserved,
    /// An approved source confirms attendance.
    Attended,
    /// An approved source confirms attendance for only part of the meeting.
    PartiallyAttended,
    /// An approved source confirms absence.
    Absent,
    /// The attendance state cannot be established.
    Unknown,
}

/// One source-backed participant without inferred response or attendance.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeetingAttendee {
    /// Stable participant identity within approved local records.
    pub participant_id: String,
    /// User-visible participant label from an approved source.
    pub display_name: String,
    /// Exact observed invitation state.
    pub invitation_state: MeetingInvitationState,
    /// Truth state supporting the invitation state.
    pub invitation_evidence_state: ExecutiveEvidenceState,
    /// Exact observed attendance state.
    pub attendance_state: MeetingAttendanceState,
    /// Truth state supporting the attendance state.
    pub attendance_evidence_state: ExecutiveEvidenceState,
    /// Sorted exact source identities supporting the participant fields.
    pub source_ids: Vec<String>,
}

/// One source-linked item in an agenda or meeting-request draft.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeetingPlanItem {
    /// Stable item identity.
    pub item_id: String,
    /// Bounded source-faithful item text.
    pub text: String,
    /// Sorted exact source identities supporting the item.
    pub source_ids: Vec<String>,
}

/// One sealed local agenda or meeting-request draft with no external authority.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeetingPlanDraft {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable draft identity.
    pub draft_id: String,
    /// Stable meeting identity.
    pub meeting_id: String,
    /// Closed draft type.
    pub kind: MeetingDraftKind,
    /// Source-backed purpose statement.
    pub purpose: String,
    /// Sorted participants and their exact observed states.
    pub participants: Vec<MeetingAttendee>,
    /// Ordered proposed topics.
    pub topics: Vec<MeetingPlanItem>,
    /// Ordered decisions the meeting may need to address.
    pub decision_needs: Vec<MeetingPlanItem>,
    /// Ordered preparation items.
    pub preparation_items: Vec<MeetingPlanItem>,
    /// Ordered expected-output proposals.
    pub expected_outputs: Vec<MeetingPlanItem>,
    /// Non-empty exact local source references.
    pub sources: Vec<ExecutiveSourceReference>,
    /// SHA-256 digest of the sealed draft with this field empty.
    pub draft_sha256: String,
    /// Always true because the record cannot create or accept an invitation.
    pub proposal_only: bool,
    /// Always false; no invitation, notification, or message was sent.
    pub external_effects_performed: bool,
    /// Always false; no calendar event was created or changed.
    pub calendar_effect_performed: bool,
}

/// Closed source class for one note or transcript segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeetingTextSourceKind {
    /// User-provided live notes.
    LiveNote,
    /// User-provided local transcript text.
    Transcript,
}

/// One exact unclear-language range in the verbatim source text.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeetingUnclearMarker {
    /// Stable marker identity.
    pub marker_id: String,
    /// Inclusive UTF-8 byte offset in the verbatim text.
    pub start_byte: u32,
    /// Exclusive UTF-8 byte offset in the verbatim text.
    pub end_byte: u32,
    /// Exact source fragment covered by the offsets.
    pub verbatim_fragment: String,
    /// Bounded explanation of why the language remains unclear.
    pub reason: String,
}

/// One verbatim-preserving note or transcript cleanup segment.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeetingTranscriptSegment {
    /// Stable segment identity.
    pub segment_id: String,
    /// Source class supplied by the user.
    pub source_kind: MeetingTextSourceKind,
    /// Exact unmodified source text.
    pub verbatim_text: String,
    /// Local cleanup proposal; it never replaces the verbatim text.
    pub cleaned_text: String,
    /// Optional exact UTC segment start in `YYYY-MM-DDTHH:MM:SSZ` form.
    pub started_at: Option<String>,
    /// Optional exact UTC segment end in `YYYY-MM-DDTHH:MM:SSZ` form.
    pub ended_at: Option<String>,
    /// Optional source-reported speaker label.
    pub speaker_label: Option<String>,
    /// Source- or user-supplied attribution confidence in basis points.
    pub attribution_confidence_bps: u16,
    /// Truth state of the speaker attribution.
    pub attribution_evidence_state: ExecutiveEvidenceState,
    /// Ordered exact unclear-language markers.
    pub unclear_markers: Vec<MeetingUnclearMarker>,
    /// Sorted exact source identities supporting the segment.
    pub source_ids: Vec<String>,
}

/// A sealed local cleanup proposal that retains every verbatim segment.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeetingTranscriptCleanup {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable cleanup identity.
    pub cleanup_id: String,
    /// Stable meeting identity.
    pub meeting_id: String,
    /// Ordered source-preserving segments.
    pub segments: Vec<MeetingTranscriptSegment>,
    /// Non-empty exact local source references.
    pub sources: Vec<ExecutiveSourceReference>,
    /// SHA-256 digest of the sealed cleanup with this field empty.
    pub cleanup_sha256: String,
    /// Always true because cleaned text remains a reviewable proposal.
    pub proposal_only: bool,
    /// Always false; the source note or transcript was not changed.
    pub source_mutation_performed: bool,
}

/// Closed class for one statement in reviewable meeting minutes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeetingMinutesItemKind {
    /// A decision directly confirmed by approved evidence.
    ConfirmedDecision,
    /// A proposed decision that has not been promoted to fact.
    ProposedDecision,
    /// A source-backed action or action proposal.
    Action,
    /// An unresolved source-backed question.
    Question,
    /// A source-backed risk or concern.
    Risk,
    /// A proposed or confirmed next-meeting detail.
    NextMeeting,
}

/// Closed evidence state for an owner or date field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeetingFieldState {
    /// The field is confirmed by an exact source.
    Confirmed,
    /// The field is a visible proposal and not a confirmed fact.
    Proposed,
    /// The field is absent or cannot be established.
    Unknown,
}

/// One source-linked minutes entry with explicit owner and date truth states.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeetingMinutesItem {
    /// Stable item identity.
    pub item_id: String,
    /// Closed item class.
    pub kind: MeetingMinutesItemKind,
    /// Bounded source-faithful statement.
    pub text: String,
    /// Truth state of the statement itself.
    pub evidence_state: ExecutiveEvidenceState,
    /// Optional owner label; absence must be represented as unknown.
    pub owner: Option<String>,
    /// Exact owner-field truth state.
    pub owner_state: MeetingFieldState,
    /// Optional normalized local date in `YYYY-MM-DD` form.
    pub due_date: Option<String>,
    /// Exact due-date truth state.
    pub due_date_state: MeetingFieldState,
    /// Earlier minutes item carried into this meeting, if any.
    pub carried_from_item_id: Option<String>,
    /// Sorted exact source identities supporting the item.
    pub source_ids: Vec<String>,
}

/// Sealed local minutes with no notification, assignment, or scheduling authority.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeetingMinutes {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable minutes identity.
    pub minutes_id: String,
    /// Stable meeting identity.
    pub meeting_id: String,
    /// Sorted participant records with independent response and attendance states.
    pub participants: Vec<MeetingAttendee>,
    /// Ordered minutes items.
    pub items: Vec<MeetingMinutesItem>,
    /// Non-empty exact local source references.
    pub sources: Vec<ExecutiveSourceReference>,
    /// Privacy class applied before any projection or export.
    pub privacy_class: ExecutivePrivacyClass,
    /// SHA-256 digest of the sealed minutes with this field empty.
    pub minutes_sha256: String,
    /// Always true; the user must review the local minutes.
    pub proposal_only: bool,
    /// Always false; no action was assigned or commitment created.
    pub commitment_effect_performed: bool,
    /// Always false; no follow-up was sent or notification created.
    pub communication_effect_performed: bool,
    /// Always false; no calendar event was created or changed.
    pub calendar_effect_performed: bool,
}

/// Closed continuity state for one source-linked carry-forward item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeetingContinuityState {
    /// Item remains visibly open.
    Open,
    /// A later exact source confirms completion.
    Completed,
    /// A later exact source confirms cancellation.
    Cancelled,
    /// A later exact source supersedes the item.
    Superseded,
    /// Current state cannot be established.
    Unknown,
}

/// One immutable source-linked item carried through a recurring meeting series.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeetingContinuityItem {
    /// Stable continuity identity.
    pub continuity_item_id: String,
    /// Meeting where the item first appeared.
    pub originating_meeting_id: String,
    /// Exact originating minutes item.
    pub source_item_id: String,
    /// Current visible continuity state.
    pub state: MeetingContinuityState,
    /// Source-faithful item text.
    pub text: String,
    /// Optional owner label preserved from the minutes.
    pub owner: Option<String>,
    /// Owner truth state preserved from the minutes.
    pub owner_state: MeetingFieldState,
    /// Optional normalized local due date.
    pub due_date: Option<String>,
    /// Date truth state preserved from the minutes.
    pub due_date_state: MeetingFieldState,
    /// Sorted minutes-item identities contributing immutable history.
    pub history_item_ids: Vec<String>,
    /// Sorted exact source identities supporting the current item.
    pub source_ids: Vec<String>,
}

/// One exact source-backed state update for an existing continuity item.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeetingContinuityUpdate {
    /// Existing continuity-item identity.
    pub continuity_item_id: String,
    /// New state stated by the current approved source.
    pub state: MeetingContinuityState,
    /// Current minutes item providing the update.
    pub source_item_id: String,
    /// Sorted exact source identities supporting the update.
    pub source_ids: Vec<String>,
}

/// Recurring-meeting continuity and local follow-up draft.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeetingContinuityRecord {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable continuity-record identity.
    pub continuity_record_id: String,
    /// Stable recurring series identity.
    pub series_id: String,
    /// Current meeting identity.
    pub meeting_id: String,
    /// Exact prior continuity record, when supplied.
    pub prior_record_id: Option<String>,
    /// Sorted current and historical carry-forward items.
    pub items: Vec<MeetingContinuityItem>,
    /// Local follow-up draft for user review.
    pub follow_up_draft: String,
    /// SHA-256 digest of the sealed record with this field empty.
    pub continuity_sha256: String,
    /// Always true; the record and follow-up remain local proposals.
    pub proposal_only: bool,
    /// Always false; the follow-up draft was not sent.
    pub communication_effect_performed: bool,
    /// Always false; no source record was changed.
    pub source_mutation_performed: bool,
}

/// Deterministic content-free closeout counts for one meeting.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeetingCloseout {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable meeting identity.
    pub meeting_id: String,
    /// Exact sealed minutes digest.
    pub minutes_sha256: String,
    /// Total minutes-item count.
    pub item_count: u32,
    /// Confirmed-decision count.
    pub confirmed_decision_count: u32,
    /// Proposed-decision count.
    pub proposed_decision_count: u32,
    /// Action count.
    pub action_count: u32,
    /// Count of actions whose owner remains unknown.
    pub unknown_owner_action_count: u32,
    /// Count of actions whose due date remains unknown.
    pub unknown_date_action_count: u32,
    /// Count of open carry-forward items.
    pub open_continuity_count: u32,
    /// Always false; closeout performs no external action.
    pub external_effects_performed: bool,
}
