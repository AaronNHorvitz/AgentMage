//! Authority-free executive-assistant records and deterministic local projections.

/// Closed local knowledge-store class supplying an executive record.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ExecutiveSourceStore {
    /// An ordinary user-owned folder of approved local files.
    PlainFolder,
    /// A user-owned Obsidian vault containing ordinary Markdown notes.
    Obsidian,
    /// A user-provided local message export, never a connected inbox.
    LocalMessageExport,
    /// A user-provided local schedule or calendar export.
    LocalScheduleExport,
}

/// Closed user-visible truth state for one executive record or field.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ExecutiveEvidenceState {
    /// The cited source directly confirms the represented fact.
    Confirmed,
    /// The represented statement is an interpretation and not a confirmed fact.
    Inferred,
    /// The statement was once confirmed but is not asserted as current.
    Historical,
    /// Current approved sources disagree materially.
    Disputed,
    /// Current approved sources cannot establish the statement.
    Unknown,
}

/// Closed record class admitted to the executive-assistant projections.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ExecutiveRecordKind {
    /// One bounded task or next action.
    Task,
    /// A promise made by an identified person to another identified party.
    Commitment,
    /// A decision, proposal, preference, question, assumption, or reported statement.
    Decision,
    /// An item awaiting another person or source.
    Waiting,
    /// A version-bound approval request and response.
    Approval,
    /// A source-backed deadline or reminder.
    Deadline,
    /// A user-provided local schedule record.
    Schedule,
    /// An active project or portfolio item.
    Project,
    /// A meeting record or preparation item.
    Meeting,
    /// Approved professional context about one person.
    Person,
    /// An organization or reporting relationship.
    Organization,
    /// A local correspondence draft or source record.
    Correspondence,
}

/// Closed lifecycle state for a canonical executive record.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ExecutiveRecordStatus {
    /// A proposal that has not become a confirmed decision or commitment.
    Proposed,
    /// Current work or an active confirmed record.
    Active,
    /// Waiting on a named person, source, decision, or approval.
    Waiting,
    /// Unable to progress because of a visible dependency or conflict.
    Blocked,
    /// Completed with source evidence.
    Completed,
    /// Preserved history replaced by a named later record.
    Superseded,
    /// Explicitly cancelled without implying completion.
    Cancelled,
    /// Current state cannot be established.
    Unknown,
}

/// Executive-record privacy class with distinct retrieval and export policy.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ExecutivePrivacyClass {
    /// Ordinary approved local work information.
    Ordinary,
    /// Private information restricted to its owning local workspace.
    Private,
    /// Confidential information requiring exact-source retrieval and separate indexing.
    Confidential,
    /// Highly restricted information excluded from general indexes and exports.
    HighlyRestricted,
}

/// Closed due-date proximity supplied by deterministic date normalization.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ExecutiveDueWindow {
    /// The confirmed due date has passed.
    Overdue,
    /// The confirmed due date is the current local date.
    Today,
    /// The confirmed due date is within seven local dates.
    WithinSevenDays,
    /// The confirmed due date is within thirty local dates.
    WithinThirtyDays,
    /// The confirmed due date is later than thirty local dates.
    Later,
    /// No confirmed due date is available.
    Unknown,
}

/// Content-addressed source identity with no ambient read authority.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutiveSourceReference {
    /// Stable source identity resolved by an existing authorized local adapter.
    pub source_id: String,
    /// Stable object identity within the source.
    pub object_id: String,
    /// Optional heading, field, message, or bounded fragment identity.
    pub fragment: Option<String>,
    /// Lowercase SHA-256 digest of the exact source bytes.
    pub content_sha256: String,
    /// Exact source revision observed when the record was constructed.
    pub observed_revision: Option<String>,
    /// Local storage grammar from which the source was read.
    pub store: ExecutiveSourceStore,
}

/// One source-backed field retained without promoting interpretation to fact.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutiveField {
    /// Stable lowercase field code.
    pub field_code: String,
    /// Bounded user-visible field value.
    pub value: String,
    /// Exact truth state shown with the field.
    pub evidence_state: ExecutiveEvidenceState,
    /// Sorted source identities supporting or disputing the field.
    pub source_ids: Vec<String>,
}

/// One canonical local record consumed by executive-assistant projections.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutiveRecord {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable record identity.
    pub record_id: String,
    /// Closed record class.
    pub kind: ExecutiveRecordKind,
    /// Bounded title.
    pub title: String,
    /// Bounded source-faithful summary.
    pub summary: String,
    /// Current lifecycle state.
    pub status: ExecutiveRecordStatus,
    /// Truth state of the record-level representation.
    pub evidence_state: ExecutiveEvidenceState,
    /// Optional confirmed or explicitly unknown owner label.
    pub owner: Option<String>,
    /// Optional counterparty or waiting-on label.
    pub counterparty: Option<String>,
    /// Optional project identity.
    pub project_id: Option<String>,
    /// Optional normalized local date in `YYYY-MM-DD` form.
    pub due_date: Option<String>,
    /// Deterministically normalized due proximity.
    pub due_window: ExecutiveDueWindow,
    /// Estimated effort in minutes when supplied by an approved record.
    pub estimated_effort_minutes: Option<u32>,
    /// User- or rule-supplied urgency in basis points.
    pub urgency_bps: u16,
    /// User- or rule-supplied importance in basis points.
    pub importance_bps: u16,
    /// Explicit user preference in basis points.
    pub user_preference_bps: u16,
    /// Consequence-of-delay magnitude in basis points.
    pub consequence_bps: u16,
    /// Whether approved schedule records conflict with the represented work.
    pub schedule_conflict: bool,
    /// Sorted record identities that must complete first.
    pub dependency_ids: Vec<String>,
    /// Sorted source-backed fields.
    pub fields: Vec<ExecutiveField>,
    /// Non-empty exact local source references.
    pub sources: Vec<ExecutiveSourceReference>,
    /// Privacy policy applied before retrieval, indexing, retention, or export.
    pub privacy_class: ExecutivePrivacyClass,
    /// Earlier record replaced by this immutable revision, if any.
    pub supersedes_record_id: Option<String>,
}

/// Closed priority-score component disclosed to the user.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ExecutivePriorityComponentKind {
    /// Urgency contribution.
    Urgency,
    /// Importance contribution.
    Importance,
    /// Explicit user-preference contribution.
    UserPreference,
    /// Consequence-of-delay contribution.
    Consequence,
    /// Confirmed due-window contribution.
    DueWindow,
    /// Schedule-conflict contribution or penalty.
    Schedule,
    /// Unresolved dependency penalty.
    Dependencies,
    /// Estimated-effort tie-break contribution.
    Effort,
    /// Evidence-state confidence penalty.
    EvidenceState,
}

/// One visible component in a deterministic priority score.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutivePriorityComponent {
    /// Closed component class.
    pub kind: ExecutivePriorityComponentKind,
    /// Signed score contribution in fixed integer points.
    pub points: i64,
    /// Stable user-visible reason code.
    pub reason_code: String,
}

/// One deterministically ranked record and its complete explanation.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutivePriorityEntry {
    /// Exact canonical record identity.
    pub record_id: String,
    /// One-based rank after deterministic tie breaking.
    pub rank: u32,
    /// Complete fixed-point score.
    pub score: i64,
    /// Ordered complete score decomposition.
    pub components: Vec<ExecutivePriorityComponent>,
    /// Sorted source identities supporting the ranking inputs.
    pub source_ids: Vec<String>,
    /// Visible assumptions or unknown inputs that constrained the score.
    pub limitations: Vec<String>,
}

/// Closed tracker projection over canonical records.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ExecutiveTrackerKind {
    /// Confirmed and proposed commitments.
    Commitments,
    /// Confirmed decisions and non-confirmed decision-like records.
    Decisions,
    /// Items awaiting another person or source.
    Waiting,
    /// Version-bound approval requests and responses.
    Approvals,
    /// Confirmed and unknown deadlines.
    Deadlines,
    /// Local reminders derived from canonical records.
    Reminders,
}

/// One source-backed row in a tracker.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutiveTrackerEntry {
    /// Exact canonical record identity.
    pub record_id: String,
    /// Display title.
    pub title: String,
    /// Current lifecycle state.
    pub status: ExecutiveRecordStatus,
    /// Truth state rendered with the row.
    pub evidence_state: ExecutiveEvidenceState,
    /// Confirmed date or explicit absence.
    pub due_date: Option<String>,
    /// Confirmed person label or explicit absence.
    pub person: Option<String>,
    /// Sorted exact source identities.
    pub source_ids: Vec<String>,
}

/// Deterministic tracker projection with no reminder or notification effect.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutiveTracker {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable tracker identity.
    pub tracker_id: String,
    /// Closed tracker class.
    pub kind: ExecutiveTrackerKind,
    /// Deterministically ordered entries.
    pub entries: Vec<ExecutiveTrackerEntry>,
    /// Always false; rendering a tracker cannot notify another system.
    pub notification_allowed: bool,
    /// Digest of the complete canonical tracker.
    pub tracker_sha256: String,
}

/// Closed executive view class.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ExecutiveViewKind {
    /// Start-of-cycle schedule, priority, deadline, waiting, and preparation view.
    StartOfCycle,
    /// Completed, unfinished, changed, new, preparation, and handoff view.
    Closeout,
    /// Recurring accomplishment, risk, aging, project, and focus review.
    RecurringReview,
    /// Active-project portfolio view.
    Portfolio,
    /// Concise current status view.
    Status,
    /// Meeting-preparation brief.
    MeetingBrief,
    /// Evidence, options, risk, dissent, and deadline decision brief.
    DecisionBrief,
    /// Approved professional context for one person.
    PersonBrief,
    /// Confirmed, inferred, historical, disputed, and unknown organization view.
    OrganizationBrief,
    /// Table-of-contents style pack over approved local sources.
    BriefingPack,
    /// Source-backed changes after an exact prior snapshot.
    ChangesSince,
    /// Overdue, ownerless, dateless, unapproved, and next-action review.
    ForgottenItems,
    /// Exact source and method audit view.
    Audit,
}

/// One source-backed row in an executive view.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutiveViewItem {
    /// Exact canonical record identity.
    pub record_id: String,
    /// Display section code.
    pub section: String,
    /// Bounded display text.
    pub text: String,
    /// Truth state rendered with the item.
    pub evidence_state: ExecutiveEvidenceState,
    /// Sorted source identities.
    pub source_ids: Vec<String>,
    /// Visible warnings, missing fields, conflicts, or stale-state markers.
    pub warnings: Vec<String>,
}

/// Deterministic local executive view with no effect authority.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutiveView {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable view identity.
    pub view_id: String,
    /// Closed view class.
    pub kind: ExecutiveViewKind,
    /// Exact canonical snapshot digest.
    pub snapshot_sha256: String,
    /// Deterministically ordered source-backed items.
    pub items: Vec<ExecutiveViewItem>,
    /// Visible view-level limitations and missing evidence.
    pub limitations: Vec<String>,
    /// Always true; the view can only inform a later user decision.
    pub proposal_only: bool,
    /// Always false; rendering cannot write, send, schedule, or notify.
    pub external_effect_allowed: bool,
    /// Digest of the complete canonical view.
    pub view_sha256: String,
}

/// Closed local correspondence format.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ExecutiveCorrespondenceKind {
    /// Electronic mail draft.
    Email,
    /// Local chat-message draft.
    Chat,
    /// Memorandum draft.
    Memorandum,
    /// Briefing-note draft.
    BriefingNote,
    /// Request draft.
    Request,
    /// Thank-you draft.
    ThankYou,
    /// Follow-up draft.
    FollowUp,
    /// Escalation draft.
    Escalation,
    /// Status-update draft.
    StatusUpdate,
}

/// One material draft claim and its supporting source identities.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutiveDraftClaim {
    /// Stable draft-local claim identity.
    pub claim_id: String,
    /// Exact represented statement.
    pub statement: String,
    /// Truth state that must remain visible to the user.
    pub evidence_state: ExecutiveEvidenceState,
    /// Sorted supporting source identities.
    pub source_ids: Vec<String>,
}

/// Local correspondence draft that cannot select recipients or send itself.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutiveCorrespondenceDraft {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable draft identity.
    pub draft_id: String,
    /// Closed draft format.
    pub kind: ExecutiveCorrespondenceKind,
    /// User-supplied tone-profile identity.
    pub tone_profile_id: String,
    /// User-reviewed recipient labels; an empty list remains visibly incomplete.
    pub recipients: Vec<String>,
    /// Bounded subject.
    pub subject: String,
    /// Bounded local draft body.
    pub body: String,
    /// User-declared attachment names available locally.
    pub attachment_names: Vec<String>,
    /// Material claims represented in the body.
    pub claims: Vec<ExecutiveDraftClaim>,
    /// Always false; a draft carries no send authority.
    pub send_allowed: bool,
    /// Always false; a draft cannot create a commitment.
    pub commitment_created: bool,
    /// Digest of the complete canonical draft.
    pub draft_sha256: String,
}

/// Closed issue detected during deterministic correspondence review.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ExecutiveCorrespondenceIssueKind {
    /// A source question is not answered by the draft.
    UnansweredQuestion,
    /// Language may create an unsupported commitment.
    AccidentalCommitment,
    /// A date expression is ambiguous or lacks a confirmed date.
    UnclearDate,
    /// The draft refers to an attachment that is not declared.
    MissingAttachment,
    /// A material draft claim lacks supporting source evidence.
    UnsupportedClaim,
    /// The draft contains material outside the admitted privacy class.
    SensitiveContent,
    /// A recipient or named person is absent or not confirmed.
    UncertainName,
}

/// One deterministic content-free correspondence finding.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutiveCorrespondenceIssue {
    /// Closed issue class.
    pub kind: ExecutiveCorrespondenceIssueKind,
    /// Stable content-free reason code.
    pub reason_code: String,
    /// Exact related claim identity when applicable.
    pub claim_id: Option<String>,
}

/// Deterministic review of one exact correspondence draft.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutiveCorrespondenceReview {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact reviewed draft digest.
    pub draft_sha256: String,
    /// Deterministically ordered findings.
    pub issues: Vec<ExecutiveCorrespondenceIssue>,
    /// True only when no issue remains; this is not send approval.
    pub locally_complete: bool,
    /// Always false; review cannot send or notify.
    pub send_allowed: bool,
    /// Digest of the complete canonical review.
    pub review_sha256: String,
}

/// One user-provided message-export record with no inbox authority.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutiveLocalMessage {
    /// Stable local message identity.
    pub message_id: String,
    /// Exact content-addressed local source.
    pub source: ExecutiveSourceReference,
    /// User-provided sender label.
    pub sender: String,
    /// Bounded subject.
    pub subject: String,
    /// Bounded local body.
    pub body: String,
    /// Whether the export contains a direct question for the user.
    pub asks_question: bool,
    /// Whether the export contains a requested user action.
    pub requests_action: bool,
    /// Whether the export asks the user to make a decision.
    pub requests_decision: bool,
    /// Whether an approved local record confirms a response was sent.
    pub response_confirmed: bool,
    /// Earlier exact message identity when this is a confirmed duplicate.
    pub duplicate_of: Option<String>,
    /// Privacy policy applied to local triage.
    pub privacy_class: ExecutivePrivacyClass,
}

/// Closed local-message triage class.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ExecutiveMessageTriageClass {
    /// A confirmed requested action is present.
    ActionRequired,
    /// A confirmed unanswered question is present.
    ResponseRequired,
    /// A confirmed requested decision is present.
    DecisionRequired,
    /// No current action, response, decision, waiting, or duplicate state is present.
    ReferenceOnly,
    /// A response is confirmed and the item remains useful as waiting evidence.
    Waiting,
    /// The message is a confirmed duplicate of an exact local record.
    Duplicate,
    /// The available export cannot support a stronger classification.
    Uncertain,
}

/// One authority-free local-message triage result.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutiveMessageTriageEntry {
    /// Exact local message identity.
    pub message_id: String,
    /// Closed triage class.
    pub classification: ExecutiveMessageTriageClass,
    /// Stable deterministic reason code.
    pub reason_code: String,
    /// Exact local source identity.
    pub source_id: String,
    /// Always false; triage cannot connect to or mutate an inbox.
    pub inbox_access_allowed: bool,
    /// Always false; triage cannot send a response.
    pub send_allowed: bool,
}

/// Closed privacy-policy operation for executive records.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ExecutivePrivacyOperation {
    /// Retrieve an exact record for one approved local purpose.
    Retrieve,
    /// Add content-free search metadata to the class-specific local index.
    Index,
    /// Retain the record under a bounded local policy.
    Retain,
    /// Prepare a user-reviewed local export proposal.
    Export,
}

/// Exact request evaluated by the executive privacy policy.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutivePrivacyRequest {
    /// Exact record identity.
    pub record_id: String,
    /// Record privacy class.
    pub privacy_class: ExecutivePrivacyClass,
    /// Requested closed operation.
    pub operation: ExecutivePrivacyOperation,
    /// Whether the request addresses an exact source rather than a broad search.
    pub exact_source: bool,
    /// Whether the current user explicitly approved this exact operation.
    pub explicit_user_approval: bool,
    /// Requested retention in whole days when retaining.
    pub retention_days: Option<u16>,
}

/// Fail-closed privacy-policy result with no operation authority.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutivePrivacyDecision {
    /// Exact record identity.
    pub record_id: String,
    /// Requested operation.
    pub operation: ExecutivePrivacyOperation,
    /// Whether policy permits a later controlled operation to be proposed.
    pub proposal_allowed: bool,
    /// Whether exact current user approval remains required.
    pub user_approval_required: bool,
    /// Whether a general index may contain this record.
    pub general_index_allowed: bool,
    /// Whether class-separated indexing is required.
    pub separate_index_required: bool,
    /// Stable content-free policy reason.
    pub reason_code: String,
    /// Always false; a policy decision does not perform the operation.
    pub effect_performed: bool,
}

/// One immutable current-and-history portfolio snapshot.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutivePortfolioSnapshot {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable snapshot identity.
    pub snapshot_id: String,
    /// Current canonical record revision for each identity.
    pub current_records: Vec<ExecutiveRecord>,
    /// Superseded or completed earlier revisions preserved for audit.
    pub history: Vec<ExecutiveRecord>,
    /// Always false; reconciliation cannot mutate any source file.
    pub source_mutation_allowed: bool,
    /// Digest of the complete canonical snapshot.
    pub snapshot_sha256: String,
}
