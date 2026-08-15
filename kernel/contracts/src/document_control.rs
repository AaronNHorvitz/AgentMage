//! Authority-free document registers, quality reports, and exact filing previews.

use crate::{ExecutiveEvidenceState, ExecutiveSourceReference, MeetingFieldState};

/// Closed register class for one controlled local record.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum DocumentRegisterKind {
    /// A local document, memorandum, report, agenda, or minutes record.
    Document,
    /// A local inbound source or outbound correspondence draft.
    Correspondence,
}

/// Closed lifecycle state that cannot itself authorize filing or disposition.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum DocumentLifecycleState {
    /// Editable local draft.
    Draft,
    /// Draft awaiting review.
    InReview,
    /// Exact version approved by a named approval record.
    Approved,
    /// Exact approved final copy; no filing is implied.
    Final,
    /// Historical copy replaced by a named later record.
    Superseded,
    /// Current lifecycle state cannot be established.
    Unknown,
}

/// Closed statement class preserving source, derivation, conflict, and approval distinctions.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum DocumentStatementClass {
    /// Exact quoted or copied source language.
    VerbatimSource,
    /// Fact directly observed in an approved source.
    ObservedFact,
    /// Action derived from an approved record but not assigned by this system.
    DerivedAction,
    /// Explicitly labeled inferred summary.
    InferredSummary,
    /// Conflicting source language that remains unresolved.
    UnresolvedConflict,
    /// Exact final language approved by the user or designated reviewer.
    UserApprovedFinalLanguage,
}

/// Closed attachment review state.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum DocumentAttachmentReviewState {
    /// Attachment has not completed required local review.
    Pending,
    /// Exact attachment digest was approved for this local record.
    Approved,
    /// Exact attachment was rejected.
    Rejected,
    /// Review state cannot be established.
    Unknown,
}

/// One exact local attachment identity without execution or delivery authority.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentRegisterAttachment {
    /// Stable attachment identity.
    pub attachment_id: String,
    /// User-visible filename.
    pub filename: String,
    /// Exact workspace-relative source path.
    pub source_path: String,
    /// Exact content digest.
    pub content_sha256: String,
    /// Current review state.
    pub review_state: DocumentAttachmentReviewState,
    /// Optional exact approval identity for the attachment digest.
    pub approval_id: Option<String>,
}

/// One source-backed register statement, commitment, deadline, or final-language item.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentRegisterStatement {
    /// Stable statement identity.
    pub statement_id: String,
    /// Closed source and derivation class.
    pub class: DocumentStatementClass,
    /// Bounded source-faithful statement.
    pub text: String,
    /// Truth state of the statement.
    pub evidence_state: ExecutiveEvidenceState,
    /// Optional source-backed owner.
    pub owner: Option<String>,
    /// Explicit owner-field state.
    pub owner_state: MeetingFieldState,
    /// Optional normalized local date in `YYYY-MM-DD` form.
    pub due_date: Option<String>,
    /// Explicit due-date field state.
    pub due_date_state: MeetingFieldState,
    /// Sorted exact source identities.
    pub source_ids: Vec<String>,
}

/// One exact source-confirmed participant, correspondent, signer, owner, or reviewer name.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentNamedParty {
    /// Stable local party identity.
    pub party_id: String,
    /// Exact source-reported display name.
    pub display_name: String,
    /// Stable role code such as `participant`, `sender`, `recipient`, or `reviewer`.
    pub role_code: String,
    /// Truth state of the identity and role attribution.
    pub evidence_state: ExecutiveEvidenceState,
    /// Sorted exact source identities supporting the attribution.
    pub source_ids: Vec<String>,
}

/// One immutable versioned entry in a document or correspondence register.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentRegisterEntry {
    /// Stable record identity.
    pub record_id: String,
    /// Closed register class.
    pub kind: DocumentRegisterKind,
    /// Bounded title.
    pub title: String,
    /// User- or records-owner-supplied category; never guessed.
    pub record_category: Option<String>,
    /// Exact version label.
    pub version: String,
    /// Current local lifecycle state.
    pub lifecycle_state: DocumentLifecycleState,
    /// Exact approval identity for approved or final versions.
    pub approval_id: Option<String>,
    /// Sorted exact source-confirmed named parties.
    pub named_parties: Vec<DocumentNamedParty>,
    /// Optional meeting quorum or workflow-status statement.
    pub quorum_or_status: Option<String>,
    /// Truth state of the quorum or status statement.
    pub quorum_or_status_evidence_state: ExecutiveEvidenceState,
    /// Sorted exact attachment records.
    pub attachments: Vec<DocumentRegisterAttachment>,
    /// Sorted source-backed commitments.
    pub commitments: Vec<DocumentRegisterStatement>,
    /// Sorted source-backed deadlines.
    pub deadlines: Vec<DocumentRegisterStatement>,
    /// Sorted statements preserving source and truth distinctions.
    pub statements: Vec<DocumentRegisterStatement>,
    /// Exact content digest for this version.
    pub content_sha256: String,
    /// Exact workspace-relative source path.
    pub source_path: String,
    /// Optional records-owner-supplied retention schedule identity.
    pub retention_schedule_id: Option<String>,
    /// Whether the exact version completed its required local accessibility review.
    pub accessibility_review_complete: bool,
    /// Earlier record version replaced by this entry, if any.
    pub supersedes_record_id: Option<String>,
    /// Later record version replacing this entry, if known.
    pub superseded_by_record_id: Option<String>,
    /// Non-empty exact local source references.
    pub sources: Vec<ExecutiveSourceReference>,
}

/// Sealed local document and correspondence register.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentRegister {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable register identity.
    pub register_id: String,
    /// Entries sorted by record identity.
    pub entries: Vec<DocumentRegisterEntry>,
    /// SHA-256 digest of the sealed register with this field empty.
    pub register_sha256: String,
    /// Always true; the register itself grants no filing authority.
    pub proposal_only: bool,
    /// Always false; no file or external system was changed.
    pub external_effects_performed: bool,
    /// Always false; no retention or disposition decision was made.
    pub records_disposition_performed: bool,
}

/// Closed local-only document-control workflow class.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum DocumentWorkflowKind {
    /// Filename and naming-rule review.
    Naming,
    /// Content-digest duplicate review.
    Duplicate,
    /// Superseded-version review.
    Superseded,
    /// Final-copy and approval review.
    FinalCopy,
    /// General records-quality review.
    Quality,
    /// Deadline-field review.
    Deadline,
    /// Local routing-slip preview.
    RoutingSlip,
    /// Local mail-merge preview.
    MailMergePreview,
    /// Local calendar-file draft.
    CalendarFileDraft,
    /// Local filing suggestion requiring exact later approval.
    FilingSuggestion,
}

/// Closed finding class for deterministic records review.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum DocumentControlFindingKind {
    /// Required field is absent or explicitly unknown.
    MissingOrUnknown,
    /// Two exact fields or sources conflict.
    ConflictingValue,
    /// Filename does not match the supplied naming rule.
    NamingMismatch,
    /// Exact content digest duplicates another record.
    DuplicateContent,
    /// Version or supersession relationship is inconsistent.
    VersionMismatch,
    /// Approval is absent, stale, or does not match the exact version.
    ApprovalMissing,
    /// Attachment is absent, unreviewed, rejected, or digest-mismatched.
    AttachmentIssue,
    /// Deadline is missing, malformed, disputed, or inconsistent.
    DeadlineIssue,
    /// Filing category, destination, or retention field needs records-owner review.
    FilingReviewRequired,
    /// Accessibility or output-quality review remains incomplete.
    QualityReviewRequired,
    /// Untrusted source content attempted to direct an operation.
    UntrustedInstruction,
    /// Recipient identity or disclosure scope is hidden or unsupported.
    RecipientOrDisclosureIssue,
}

/// One content-minimized deterministic records-quality finding.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentControlFinding {
    /// Stable finding identity.
    pub finding_id: String,
    /// Closed finding class.
    pub kind: DocumentControlFindingKind,
    /// Exact affected register record.
    pub record_id: String,
    /// Stable field or rule code.
    pub field_code: String,
    /// Stable content-free reason code.
    pub reason_code: String,
    /// Sorted exact source identities supporting the finding.
    pub source_ids: Vec<String>,
}

/// Closed file-affecting action that always requires a separate approval and executor.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum DocumentActionKind {
    /// Save a local draft.
    SaveDraft,
    /// Rename an exact existing file.
    Rename,
    /// Move an exact existing file.
    Move,
    /// File an exact version at a records-owner-approved destination.
    File,
}

/// Exact local preview for a save, rename, move, or filing proposal.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentActionPreview {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable preview identity.
    pub preview_id: String,
    /// Closed proposed action.
    pub action: DocumentActionKind,
    /// Exact register record identity.
    pub record_id: String,
    /// Exact existing workspace-relative path, if one exists.
    pub source_path: Option<String>,
    /// Exact proposed workspace-relative destination.
    pub destination_path: String,
    /// Exact current content digest, if one exists.
    pub source_sha256: Option<String>,
    /// Exact proposed content digest.
    pub proposed_sha256: String,
    /// Exact canonical metadata digest.
    pub metadata_sha256: String,
    /// Bounded exact local content preview.
    pub content_preview: String,
    /// Bounded exact local metadata preview.
    pub metadata_preview: String,
    /// Optional records-owner-supplied category.
    pub record_category: Option<String>,
    /// Optional records-owner-supplied retention schedule identity.
    pub retention_schedule_id: Option<String>,
    /// SHA-256 digest of the sealed preview with this field empty.
    pub preview_sha256: String,
    /// Always true for every action in this contract.
    pub approval_required: bool,
    /// Always false; this record is not an approval.
    pub approval_granted: bool,
    /// Always false; preview construction never performs the action.
    pub effect_performed: bool,
}

/// Exact user or designated-reviewer confirmation of one sealed preview.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentActionApproval {
    /// Stable approval identity.
    pub approval_id: String,
    /// Exact sealed preview digest being reviewed.
    pub preview_sha256: String,
    /// Whether the exact destination was confirmed.
    pub destination_confirmed: bool,
    /// Whether the exact content digest and preview were confirmed.
    pub content_confirmed: bool,
    /// Whether the exact metadata digest and preview were confirmed.
    pub metadata_confirmed: bool,
    /// Whether the records category was supplied and confirmed when filing.
    pub record_category_confirmed: bool,
    /// Whether the retention field was supplied and confirmed when filing.
    pub retention_confirmed: bool,
    /// Final explicit local approval decision.
    pub approved: bool,
}

/// Review result for one exact action preview; it never executes the action.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentActionReview {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact preview digest.
    pub preview_sha256: String,
    /// Exact approval identity, when supplied.
    pub approval_id: Option<String>,
    /// True only when every action-specific field was explicitly confirmed.
    pub approval_complete: bool,
    /// Stable reasons approval remains incomplete.
    pub missing_confirmation_codes: Vec<String>,
    /// Always false; approval review creates no execution grant.
    pub execution_authority_created: bool,
    /// Always false; no save, rename, move, or filing occurred.
    pub effect_performed: bool,
}

/// One local-only rendered workflow report over an exact sealed register.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentWorkflowReport {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable report identity.
    pub report_id: String,
    /// Closed workflow class.
    pub kind: DocumentWorkflowKind,
    /// Exact sealed register digest.
    pub register_sha256: String,
    /// Sorted deterministic findings.
    pub findings: Vec<DocumentControlFinding>,
    /// Sorted exact action-preview digests included in the report.
    pub preview_sha256: Vec<String>,
    /// Bounded local routing slip, merge preview, calendar draft, or filing suggestion.
    pub rendered_preview: String,
    /// Always true; the report is local and reviewable.
    pub local_preview_only: bool,
    /// Always false; no recipients were selected or contacted.
    pub communication_effect_performed: bool,
    /// Always false; no calendar was changed.
    pub calendar_effect_performed: bool,
    /// Always false; no file was saved, renamed, moved, or filed.
    pub filesystem_effect_performed: bool,
    /// Always false; no records disposition decision was made.
    pub records_disposition_performed: bool,
}
