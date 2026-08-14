//! Authority-free controlled filesystem plans, structured patches, and exact previews.

use std::{collections::BTreeSet, fmt::Write as _};

use agentmage_kernel_contracts::{
    ActionId, ActionKind, ApprovalId, CapabilityGrant, GrantClass, GrantId, GrantNonce,
    GrantOperation, GrantPreimage, GrantSideEffect, GrantStatus, GrantTarget, OperationBinding,
    ToolId, WorkspaceObjectKind, WorkspacePath,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    grants::{DerivedOperationGrantRequest, GrantIssueError, GrantIssuer},
    write_approval::WriteReviewNarrative,
};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_OPERATIONS: usize = 128;
const MAX_FILE_BYTES: usize = 8 * 1024 * 1024;
const MAX_TRANSACTION_BYTES: usize = 32 * 1024 * 1024;
const MAX_PATH_DEPTH: usize = 64;
const MAX_SIBLING_NAMES: usize = 4_096;
const MAX_PATCH_HUNKS: usize = 4_096;
const MAX_PATCH_LINES: usize = 131_072;
const MAX_REVIEW_ITEMS: usize = 64;
const MAX_REVIEW_TEXT_BYTES: usize = 4_096;
const MAX_FILESYSTEM_GRANT_LIFETIME_MS: u64 = 300_000;

/// Stable reason a controlled filesystem proposal was denied before authority existed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilesystemPlanError {
    /// A closed identifier, list, mode, digest, or resource bound is invalid.
    InvalidInput,
    /// The supplied parent is not a current session-scoped read grant.
    ParentUnavailable,
    /// A source or destination lies outside the parent or inside an exclusion.
    ScopeDenied,
    /// An exact source preimage does not match its held target.
    PreimageMismatch,
    /// A structured patch is malformed, stale, overlapping, or has a wrong result.
    PatchRejected,
    /// A destination already exists or collides by case or canonical spelling.
    DestinationCollision,
    /// A protected path cannot be changed by the generic filesystem pack.
    ProtectedPath,
    /// Existing work was not classified as clean or owned by this exact task.
    UnrelatedExistingWork,
    /// Two proposed operations overlap or conflict.
    OperationConflict,
    /// The explicit approval did not bind the exact current plan and preview.
    ApprovalMismatch,
    /// Existing kernel grant issuance rejected the requested authority.
    GrantIssue,
    /// Retained parent-grant state is absent or internally inconsistent.
    GrantState,
}

impl FilesystemPlanError {
    /// Returns a stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "filesystem.plan.invalid_input",
            Self::ParentUnavailable => "filesystem.plan.parent_unavailable",
            Self::ScopeDenied => "filesystem.plan.scope_denied",
            Self::PreimageMismatch => "filesystem.plan.preimage_mismatch",
            Self::PatchRejected => "filesystem.plan.patch_rejected",
            Self::DestinationCollision => "filesystem.plan.destination_collision",
            Self::ProtectedPath => "filesystem.plan.protected_path",
            Self::UnrelatedExistingWork => "filesystem.plan.unrelated_existing_work",
            Self::OperationConflict => "filesystem.plan.operation_conflict",
            Self::ApprovalMismatch => "filesystem.approval.decision_mismatch",
            Self::GrantIssue => "filesystem.approval.grant_issue_failed",
            Self::GrantState => "filesystem.approval.grant_state_invalid",
        }
    }
}

impl From<GrantIssueError> for FilesystemPlanError {
    fn from(_: GrantIssueError) -> Self {
        Self::GrantIssue
    }
}

/// Closed operation class supported by the first controlled filesystem pack.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FilesystemOperationKind {
    /// Create one exact absent UTF-8 file.
    Create,
    /// Apply one parsed exact-preimage structured patch.
    ExactPatch,
    /// Copy one exact regular-file preimage to one absent destination.
    Copy,
    /// Move one exact regular-file preimage to one absent destination.
    Move,
    /// Move one exact regular file into an existing approved trash directory.
    TrashDelete,
}

/// User-visible file classification bound into a create or destination preview.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileClassification {
    /// Program source or a source-adjacent text file.
    SourceCode,
    /// Human-readable project documentation.
    Documentation,
    /// Non-secret application or project configuration.
    Configuration,
    /// User or project data that is not a protected source record.
    Data,
    /// Generated content explicitly identified as generated.
    Generated,
}

/// Current Git/work-packet ownership classification for an existing source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExistingWorkDisposition {
    /// The source is clean under current repository evidence.
    Clean,
    /// The source change is explicitly owned by this exact approved task.
    OwnedByCurrentTask,
    /// The source contains unrelated user-owned modifications.
    UnrelatedModified,
    /// Repository or work-packet ownership could not be established.
    Unknown,
}

impl ExistingWorkDisposition {
    fn permits_mutation(self) -> bool {
        matches!(self, Self::Clean | Self::OwnedByCurrentTask)
    }
}

/// One exact destination observation beneath a continuously held parent directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewDestinationDraft {
    /// Exact held parent-directory target.
    pub parent: GrantTarget,
    /// Canonical absent destination directly beneath `parent`.
    pub path: WorkspacePath,
    /// Complete bounded sibling-name snapshot observed before preview.
    pub observed_sibling_names: Vec<String>,
}

/// One exact existing source observation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExistingSourceDraft {
    /// Exact held regular-file target.
    pub target: GrantTarget,
    /// Bytes read through that exact held target.
    pub observed_bytes: Vec<u8>,
    /// Current repository and task ownership classification.
    pub work_disposition: ExistingWorkDisposition,
    /// Exact observed POSIX-compatible permission bits displayed before mutation.
    pub mode: u32,
}

/// Authority-free request for one controlled filesystem operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FilesystemOperationDraft {
    /// Propose creation of one exact absent file.
    Create {
        /// Stable operation identity.
        operation_id: String,
        /// Exact absent destination observation.
        destination: NewDestinationDraft,
        /// Complete proposed UTF-8 bytes.
        content: Vec<u8>,
        /// Exact permission bits for the new file.
        mode: u32,
        /// User-visible file classification.
        classification: FileClassification,
    },
    /// Propose one exact structured patch.
    ExactPatch {
        /// Stable operation identity.
        operation_id: String,
        /// Exact existing source observation.
        source: ExistingSourceDraft,
        /// Closed JSON representation of [`StructuredPatch`].
        patch_json: Vec<u8>,
        /// Caller-declared digest of the complete expected postimage.
        expected_postimage_sha256: String,
    },
    /// Propose copying one exact source into one absent destination.
    Copy {
        /// Stable operation identity.
        operation_id: String,
        /// Exact existing source observation.
        source: ExistingSourceDraft,
        /// Exact absent destination observation.
        destination: NewDestinationDraft,
        /// User-visible destination classification.
        classification: FileClassification,
    },
    /// Propose moving one exact source into one absent destination.
    Move {
        /// Stable operation identity.
        operation_id: String,
        /// Exact existing source observation.
        source: ExistingSourceDraft,
        /// Exact absent destination observation.
        destination: NewDestinationDraft,
    },
    /// Propose moving one exact source into an existing approved trash directory.
    TrashDelete {
        /// Stable operation identity.
        operation_id: String,
        /// Exact existing source observation.
        source: ExistingSourceDraft,
        /// Exact absent trash destination observation.
        trash_destination: NewDestinationDraft,
    },
}

/// Complete request for one bounded authority-free filesystem plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilesystemPlanRequest {
    /// Stable plan identity.
    pub plan_id: String,
    /// Kernel-clock observation time.
    pub observed_at_epoch_ms: u64,
    /// Ordered operation drafts.
    pub operations: Vec<FilesystemOperationDraft>,
    /// Complete review narrative.
    pub review: WriteReviewNarrative,
    /// Exact separately authorized verification labels.
    pub permitted_verification: Vec<String>,
}

/// Closed line-oriented patch format; no path or command syntax is accepted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredPatch {
    /// Current patch schema version, exactly one.
    pub schema_version: u16,
    /// Ordered, non-overlapping exact hunks.
    pub hunks: Vec<StructuredPatchHunk>,
}

/// One one-based exact-old-lines patch hunk.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredPatchHunk {
    /// One-based source line at which `old_lines` must begin.
    pub old_start_line: u32,
    /// Exact old line strings, including line terminators when present.
    pub old_lines: Vec<String>,
    /// Exact replacement line strings, including line terminators when intended.
    pub new_lines: Vec<String>,
}

/// One validated operation retained outside user-owned files.
#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FilesystemOperation {
    operation_id: String,
    kind: FilesystemOperationKind,
    source: Option<GrantTarget>,
    destination_parent: Option<GrantTarget>,
    source_path: Option<String>,
    destination_path: Option<String>,
    source_sha256: Option<String>,
    postimage_sha256: Option<String>,
    source_mode: Option<u32>,
    destination_mode: Option<u32>,
    classification: Option<FileClassification>,
    work_disposition: Option<ExistingWorkDisposition>,
    sibling_snapshot_sha256: Option<String>,
    structured_patch_sha256: Option<String>,
    #[serde(skip_serializing)]
    source_bytes: Vec<u8>,
    #[serde(skip_serializing)]
    postimage_bytes: Vec<u8>,
    operation_sha256: String,
}

impl std::fmt::Debug for FilesystemOperation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FilesystemOperation")
            .field("operation_id", &self.operation_id)
            .field("kind", &self.kind)
            .field("source_path", &self.source_path)
            .field("destination_path", &self.destination_path)
            .field("source_bytes", &"[REDACTED]")
            .field("postimage_bytes", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

impl FilesystemOperation {
    /// Returns the stable operation identity.
    #[must_use]
    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }

    /// Returns the closed operation kind.
    #[must_use]
    pub const fn kind(&self) -> FilesystemOperationKind {
        self.kind
    }

    /// Returns the exact existing source target when applicable.
    #[must_use]
    pub const fn source(&self) -> Option<&GrantTarget> {
        self.source.as_ref()
    }

    /// Returns the exact destination-parent target when applicable.
    #[must_use]
    pub const fn destination_parent(&self) -> Option<&GrantTarget> {
        self.destination_parent.as_ref()
    }

    /// Returns the canonical source display path when applicable.
    #[must_use]
    pub fn source_path(&self) -> Option<&str> {
        self.source_path.as_deref()
    }

    /// Returns the canonical destination display path when applicable.
    #[must_use]
    pub fn destination_path(&self) -> Option<&str> {
        self.destination_path.as_deref()
    }

    /// Returns the exact retained source bytes when applicable.
    #[must_use]
    pub fn source_bytes(&self) -> &[u8] {
        &self.source_bytes
    }

    /// Returns the exact complete postimage bytes when applicable.
    #[must_use]
    pub fn postimage_bytes(&self) -> &[u8] {
        &self.postimage_bytes
    }

    /// Returns the exact source digest when applicable.
    #[must_use]
    pub fn source_sha256(&self) -> Option<&str> {
        self.source_sha256.as_deref()
    }

    /// Returns the exact postimage digest when applicable.
    #[must_use]
    pub fn postimage_sha256(&self) -> Option<&str> {
        self.postimage_sha256.as_deref()
    }

    /// Returns the exact observed source mode when applicable.
    #[must_use]
    pub const fn source_mode(&self) -> Option<u32> {
        self.source_mode
    }

    /// Returns the exact requested destination mode when applicable.
    #[must_use]
    pub const fn destination_mode(&self) -> Option<u32> {
        self.destination_mode
    }

    /// Returns the canonical operation identity.
    #[must_use]
    pub fn operation_sha256(&self) -> &str {
        &self.operation_sha256
    }
}

/// Complete validated filesystem plan with no authority or effect method.
#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FilesystemPlan {
    plan_id: String,
    workspace_id: String,
    parent_grant_id: GrantId,
    parent_grant_sha256: String,
    observed_at_epoch_ms: u64,
    operations: Vec<FilesystemOperation>,
    review: WriteReviewNarrative,
    permitted_verification: Vec<String>,
    requires_high_risk_delete_grant: bool,
    plan_sha256: String,
}

impl std::fmt::Debug for FilesystemPlan {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FilesystemPlan")
            .field("plan_id", &self.plan_id)
            .field("workspace_id", &self.workspace_id)
            .field("operation_count", &self.operations.len())
            .field(
                "requires_high_risk_delete_grant",
                &self.requires_high_risk_delete_grant,
            )
            .finish_non_exhaustive()
    }
}

impl FilesystemPlan {
    /// Returns the stable plan identity.
    #[must_use]
    pub fn plan_id(&self) -> &str {
        &self.plan_id
    }

    /// Returns the exact workspace identity.
    #[must_use]
    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    /// Returns the exact parent read-grant identity.
    #[must_use]
    pub const fn parent_grant_id(&self) -> &GrantId {
        &self.parent_grant_id
    }

    /// Returns the ordered validated operations.
    #[must_use]
    pub fn operations(&self) -> &[FilesystemOperation] {
        &self.operations
    }

    /// Returns the exact review narrative.
    #[must_use]
    pub const fn review(&self) -> &WriteReviewNarrative {
        &self.review
    }

    /// Returns the separately authorized verification labels.
    #[must_use]
    pub fn permitted_verification(&self) -> &[String] {
        &self.permitted_verification
    }

    /// Reports whether this is an all-delete plan requiring separate high-risk authority.
    #[must_use]
    pub const fn requires_high_risk_delete_grant(&self) -> bool {
        self.requires_high_risk_delete_grant
    }

    /// Returns the canonical identity of the complete plan and retained bytes.
    #[must_use]
    pub fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }
}

/// One operation-specific display entry bound into an exact approval preview.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FilesystemPreviewEntry {
    /// Stable operation identity.
    pub operation_id: String,
    /// Closed operation class.
    pub kind: FilesystemOperationKind,
    /// Canonical source path when applicable.
    pub source_path: Option<String>,
    /// Canonical destination path when applicable.
    pub destination_path: Option<String>,
    /// Exact source digest when applicable.
    pub source_sha256: Option<String>,
    /// Exact postimage digest when applicable.
    pub postimage_sha256: Option<String>,
    /// Exact source mode when applicable.
    pub source_mode: Option<u32>,
    /// Exact destination mode when applicable.
    pub destination_mode: Option<u32>,
    /// Exact escaped complete create or patch postimage, otherwise absent.
    pub complete_content_preview: Option<String>,
    /// Collision behavior is always refusal.
    pub overwrite_allowed: bool,
    /// Parent creation is never implicit.
    pub implicit_parent_creation: bool,
    /// Deletion is always a move into an approved trash directory.
    pub permanent_delete: bool,
    /// Canonical operation digest.
    pub operation_sha256: String,
}

/// Complete deterministic preview; parsing or displaying it grants no authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FilesystemApprovalPreview {
    /// Stable plan identity.
    pub plan_id: String,
    /// Canonical plan digest.
    pub plan_sha256: String,
    /// Exact workspace identity.
    pub workspace_id: String,
    /// Ordered operation-specific entries.
    pub operations: Vec<FilesystemPreviewEntry>,
    /// Complete review narrative.
    pub review: WriteReviewNarrative,
    /// Exact separately authorized verification labels.
    pub permitted_verification: Vec<String>,
    /// Whether a separate high-risk delete grant is mandatory.
    pub requires_high_risk_delete_grant: bool,
    /// Canonical preview digest.
    pub preview_sha256: String,
}

/// Explicit user decision bound to one exact controlled-filesystem preview.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilesystemApprovalDecision {
    /// Stable approval identity.
    pub approval_id: ApprovalId,
    /// Exact approved filesystem-plan digest.
    pub approved_plan_sha256: String,
    /// Exact approved preview digest.
    pub approved_preview_sha256: String,
    /// Kernel-clock approval time.
    pub approved_at_epoch_ms: u64,
    /// Exact short-lived approval expiry.
    pub expires_at_epoch_ms: u64,
    /// Exact verification labels allowed after a future apply.
    pub permitted_verification: Vec<String>,
    /// False decisions are inert.
    pub user_confirmed: bool,
    /// Separate explicit confirmation required exactly for trash-delete plans.
    pub high_risk_delete_confirmed: bool,
}

/// Kernel-selected identities needed to derive one controlled-filesystem grant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilesystemGrantRequest {
    /// Current session-read parent grant.
    pub parent_grant_id: GrantId,
    /// New single-use operation-grant identity.
    pub grant_id: GrantId,
    /// Exact action receiving the bounded authority.
    pub action_id: ActionId,
    /// Descriptive action class.
    pub action_kind: ActionKind,
    /// Exact registered filesystem-tool identity.
    pub tool_id: ToolId,
    /// Exact registered filesystem-tool version.
    pub tool_version: String,
    /// Unique anti-replay nonce.
    pub nonce: GrantNonce,
    /// Current policy digest, which must equal the parent policy.
    pub policy_sha256: String,
}

/// Non-authoritative proof that one exact filesystem grant was issued.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilesystemApprovalReceipt {
    /// Existing kernel-enforced single-use operation grant.
    pub grant: CapabilityGrant,
    /// Exact filesystem-plan digest carried as the grant argument identity.
    pub plan_sha256: String,
    /// Exact reviewed preview digest.
    pub preview_sha256: String,
    /// Exact verification labels cryptographically included in side-effect details.
    pub permitted_verification: Vec<String>,
    /// Canonical digest of this non-authoritative binding summary.
    pub binding_sha256: String,
}

/// Parses one closed JSON structured patch without accepting target or command syntax.
pub fn parse_structured_patch_json(bytes: &[u8]) -> Result<StructuredPatch, FilesystemPlanError> {
    if bytes.is_empty() || bytes.len() > MAX_FILE_BYTES {
        return Err(FilesystemPlanError::PatchRejected);
    }
    serde_json::from_slice(bytes).map_err(|_| FilesystemPlanError::PatchRejected)
}

/// Applies one exact line patch to one complete UTF-8 preimage in memory.
pub fn apply_structured_patch(
    source: &[u8],
    patch: &StructuredPatch,
) -> Result<Vec<u8>, FilesystemPlanError> {
    if patch.schema_version != 1 || patch.hunks.is_empty() || patch.hunks.len() > MAX_PATCH_HUNKS {
        return Err(FilesystemPlanError::PatchRejected);
    }
    let source = std::str::from_utf8(source).map_err(|_| FilesystemPlanError::PatchRejected)?;
    let source_lines: Vec<&str> = source.split_inclusive('\n').collect();
    let mut cursor = 0_usize;
    let mut output = String::new();
    let mut total_patch_lines = 0_usize;
    for hunk in &patch.hunks {
        if hunk.old_start_line == 0 || hunk.old_lines.is_empty() && hunk.new_lines.is_empty() {
            return Err(FilesystemPlanError::PatchRejected);
        }
        total_patch_lines = total_patch_lines
            .checked_add(hunk.old_lines.len())
            .and_then(|value| value.checked_add(hunk.new_lines.len()))
            .ok_or(FilesystemPlanError::PatchRejected)?;
        if total_patch_lines > MAX_PATCH_LINES
            || hunk
                .old_lines
                .iter()
                .chain(&hunk.new_lines)
                .any(|line| line.contains('\0') || line.len() > MAX_FILE_BYTES)
        {
            return Err(FilesystemPlanError::PatchRejected);
        }
        let start = usize::try_from(hunk.old_start_line - 1)
            .map_err(|_| FilesystemPlanError::PatchRejected)?;
        let end = start
            .checked_add(hunk.old_lines.len())
            .ok_or(FilesystemPlanError::PatchRejected)?;
        if start < cursor
            || end > source_lines.len()
            || source_lines[start..end]
                .iter()
                .copied()
                .ne(hunk.old_lines.iter().map(String::as_str))
        {
            return Err(FilesystemPlanError::PatchRejected);
        }
        for line in &source_lines[cursor..start] {
            output.push_str(line);
        }
        for line in &hunk.new_lines {
            output.push_str(line);
        }
        cursor = end;
        if output.len() > MAX_FILE_BYTES {
            return Err(FilesystemPlanError::PatchRejected);
        }
    }
    for line in &source_lines[cursor..] {
        output.push_str(line);
    }
    if output.len() > MAX_FILE_BYTES || output.as_bytes() == source.as_bytes() {
        return Err(FilesystemPlanError::PatchRejected);
    }
    Ok(output.into_bytes())
}

/// Builds one deterministic authority-free controlled filesystem plan.
pub fn build_filesystem_plan(
    parent: &CapabilityGrant,
    request: FilesystemPlanRequest,
) -> Result<FilesystemPlan, FilesystemPlanError> {
    validate_identifier(&request.plan_id)?;
    validate_review(&request.review)?;
    validate_verification(&request.permitted_verification)?;
    if parent.grant_class != GrantClass::SessionRead
        || parent.status != GrantStatus::Issued
        || request.observed_at_epoch_ms < parent.issued_at_epoch_ms
        || request.observed_at_epoch_ms >= parent.expires_at_epoch_ms
        || request.operations.is_empty()
        || request.operations.len() > MAX_OPERATIONS
    {
        return Err(FilesystemPlanError::ParentUnavailable);
    }
    let parent_grant_sha256 = canonical_sha256(parent)?;
    let mut operation_ids = BTreeSet::new();
    let mut mutated_paths = BTreeSet::new();
    let mut operations = Vec::with_capacity(request.operations.len());
    let mut total_bytes = 0_usize;
    let mut workspace_id = None;
    let delete_plan = request
        .operations
        .iter()
        .all(|draft| matches!(draft, FilesystemOperationDraft::TrashDelete { .. }));
    if !delete_plan
        && request
            .operations
            .iter()
            .any(|draft| matches!(draft, FilesystemOperationDraft::TrashDelete { .. }))
    {
        return Err(FilesystemPlanError::OperationConflict);
    }

    for draft in request.operations {
        let operation = validate_operation(parent, draft, &mut total_bytes)?;
        validate_identifier(&operation.operation_id)?;
        if !operation_ids.insert(operation.operation_id.clone()) {
            return Err(FilesystemPlanError::OperationConflict);
        }
        let operation_workspace = operation
            .source
            .as_ref()
            .or(operation.destination_parent.as_ref())
            .ok_or(FilesystemPlanError::InvalidInput)?
            .workspace_id()
            .as_str();
        match &workspace_id {
            Some(expected) if expected != operation_workspace => {
                return Err(FilesystemPlanError::ScopeDenied);
            }
            None => workspace_id = Some(operation_workspace.to_owned()),
            _ => {}
        }
        for path in [
            operation.source_path.as_deref(),
            operation.destination_path.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if !mutated_paths.insert(path.to_owned()) {
                return Err(FilesystemPlanError::OperationConflict);
            }
        }
        operations.push(operation);
    }

    let mut plan = FilesystemPlan {
        plan_id: request.plan_id,
        workspace_id: workspace_id.ok_or(FilesystemPlanError::InvalidInput)?,
        parent_grant_id: parent.grant_id.clone(),
        parent_grant_sha256,
        observed_at_epoch_ms: request.observed_at_epoch_ms,
        operations,
        review: request.review,
        permitted_verification: request.permitted_verification,
        requires_high_risk_delete_grant: delete_plan,
        plan_sha256: ZERO_SHA256.to_owned(),
    };
    plan.plan_sha256 = plan_digest(&plan)?;
    Ok(plan)
}

/// Renders one complete operation-specific approval preview.
pub fn render_filesystem_preview(
    plan: &FilesystemPlan,
) -> Result<FilesystemApprovalPreview, FilesystemPlanError> {
    verify_plan(plan)?;
    let operations = plan
        .operations
        .iter()
        .map(|operation| FilesystemPreviewEntry {
            operation_id: operation.operation_id.clone(),
            kind: operation.kind,
            source_path: operation.source_path.clone(),
            destination_path: operation.destination_path.clone(),
            source_sha256: operation.source_sha256.clone(),
            postimage_sha256: operation.postimage_sha256.clone(),
            source_mode: operation.source_mode,
            destination_mode: operation.destination_mode,
            complete_content_preview: matches!(
                operation.kind,
                FilesystemOperationKind::Create | FilesystemOperationKind::ExactPatch
            )
            .then(|| format!("{:?}", String::from_utf8_lossy(&operation.postimage_bytes))),
            overwrite_allowed: false,
            implicit_parent_creation: false,
            permanent_delete: false,
            operation_sha256: operation.operation_sha256.clone(),
        })
        .collect();
    let mut preview = FilesystemApprovalPreview {
        plan_id: plan.plan_id.clone(),
        plan_sha256: plan.plan_sha256.clone(),
        workspace_id: plan.workspace_id.clone(),
        operations,
        review: plan.review.clone(),
        permitted_verification: plan.permitted_verification.clone(),
        requires_high_risk_delete_grant: plan.requires_high_risk_delete_grant,
        preview_sha256: ZERO_SHA256.to_owned(),
    };
    preview.preview_sha256 = canonical_sha256(&preview)?;
    Ok(preview)
}

/// Verifies an exact decision and derives one short-lived single-use filesystem grant.
pub fn issue_filesystem_grant(
    issuer: &mut GrantIssuer,
    plan: &FilesystemPlan,
    preview: &FilesystemApprovalPreview,
    decision: &FilesystemApprovalDecision,
    request: FilesystemGrantRequest,
) -> Result<FilesystemApprovalReceipt, FilesystemPlanError> {
    verify_plan(plan)?;
    verify_preview(plan, preview)?;
    validate_identifier(decision.approval_id.as_str())?;
    validate_verification(&decision.permitted_verification)?;
    let current_parent = issuer
        .current(&request.parent_grant_id)
        .ok_or(FilesystemPlanError::ParentUnavailable)?;
    let current_parent_sha256 = issuer
        .revision_hash(&current_parent.grant_id, current_parent.revision)
        .ok_or(FilesystemPlanError::GrantState)?;
    if !decision.user_confirmed
        || decision.high_risk_delete_confirmed != plan.requires_high_risk_delete_grant
        || decision.approved_plan_sha256 != plan.plan_sha256
        || decision.approved_preview_sha256 != preview.preview_sha256
        || decision.permitted_verification != plan.permitted_verification
        || decision.approved_at_epoch_ms < plan.observed_at_epoch_ms
        || decision.expires_at_epoch_ms <= decision.approved_at_epoch_ms
        || decision.expires_at_epoch_ms - decision.approved_at_epoch_ms
            > MAX_FILESYSTEM_GRANT_LIFETIME_MS
        || request.parent_grant_id != plan.parent_grant_id
        || current_parent_sha256 != plan.parent_grant_sha256
        || request.policy_sha256.is_empty()
    {
        return Err(FilesystemPlanError::ApprovalMismatch);
    }

    let operation = OperationBinding::new(if plan.requires_high_risk_delete_grant {
        GrantOperation::WorkspaceDelete
    } else {
        GrantOperation::WorkspaceWrite
    });
    let verification_sha256 = canonical_sha256(&decision.permitted_verification)?;
    let mut targets = Vec::new();
    for item in &plan.operations {
        for target in [item.source.as_ref(), item.destination_parent.as_ref()]
            .into_iter()
            .flatten()
        {
            if !targets.contains(target) {
                targets.push(target.clone());
            }
        }
    }
    let preimages = targets
        .iter()
        .enumerate()
        .filter_map(|(index, target)| {
            target.preimage().map(|_| {
                let index = u32::try_from(index).map_err(|_| FilesystemPlanError::InvalidInput)?;
                GrantPreimage::for_target(index, target)
                    .ok_or(FilesystemPlanError::PreimageMismatch)
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let expected_side_effects = plan
        .operations
        .iter()
        .map(|item| {
            let mut target_indexes = Vec::new();
            for target in [item.source.as_ref(), item.destination_parent.as_ref()]
                .into_iter()
                .flatten()
            {
                let index = targets
                    .iter()
                    .position(|candidate| candidate == target)
                    .ok_or(FilesystemPlanError::GrantState)?;
                target_indexes
                    .push(u32::try_from(index).map_err(|_| FilesystemPlanError::InvalidInput)?);
            }
            let details_sha256 = canonical_sha256(&(
                item.operation_sha256.as_str(),
                item.kind,
                item.source_path.as_deref(),
                item.destination_path.as_deref(),
                item.source_sha256.as_deref(),
                item.postimage_sha256.as_deref(),
                item.source_mode,
                item.destination_mode,
                item.sibling_snapshot_sha256.as_deref(),
                item.structured_patch_sha256.as_deref(),
                verification_sha256.as_str(),
                false,
                false,
                false,
            ))?;
            Ok(GrantSideEffect {
                operation,
                target_indexes,
                details_sha256,
            })
        })
        .collect::<Result<Vec<_>, FilesystemPlanError>>()?;
    let grant = issuer.derive_operation(
        &request.parent_grant_id,
        DerivedOperationGrantRequest {
            grant_id: request.grant_id,
            approval_id: decision.approval_id.clone(),
            action_id: request.action_id,
            action_kind: request.action_kind,
            operation,
            tool_id: request.tool_id,
            tool_version: request.tool_version,
            targets,
            argument_sha256: plan.plan_sha256.clone(),
            preimages,
            expected_side_effects,
            rollback_description: plan.review.rollback.clone(),
            issued_at_epoch_ms: decision.approved_at_epoch_ms,
            expires_at_epoch_ms: decision.expires_at_epoch_ms,
            nonce: request.nonce,
            preview_sha256: preview.preview_sha256.clone(),
            policy_sha256: request.policy_sha256,
        },
    )?;
    let binding_sha256 = canonical_sha256(&(
        grant.grant_id.as_str(),
        plan.plan_sha256.as_str(),
        preview.preview_sha256.as_str(),
        decision.permitted_verification.as_slice(),
        operation,
    ))?;
    Ok(FilesystemApprovalReceipt {
        grant,
        plan_sha256: plan.plan_sha256.clone(),
        preview_sha256: preview.preview_sha256.clone(),
        permitted_verification: decision.permitted_verification.clone(),
        binding_sha256,
    })
}

fn verify_preview(
    plan: &FilesystemPlan,
    preview: &FilesystemApprovalPreview,
) -> Result<(), FilesystemPlanError> {
    let expected = render_filesystem_preview(plan)?;
    if expected != *preview {
        return Err(FilesystemPlanError::ApprovalMismatch);
    }
    Ok(())
}

fn validate_operation(
    parent: &CapabilityGrant,
    draft: FilesystemOperationDraft,
    total_bytes: &mut usize,
) -> Result<FilesystemOperation, FilesystemPlanError> {
    let mut operation = match draft {
        FilesystemOperationDraft::Create {
            operation_id,
            destination,
            content,
            mode,
            classification,
        } => {
            validate_destination(parent, &destination, true)?;
            validate_mode(mode)?;
            if content.is_empty()
                || content.len() > MAX_FILE_BYTES
                || std::str::from_utf8(&content).is_err()
            {
                return Err(FilesystemPlanError::InvalidInput);
            }
            account_bytes(total_bytes, content.len())?;
            FilesystemOperation {
                operation_id,
                kind: FilesystemOperationKind::Create,
                source: None,
                destination_parent: Some(destination.parent),
                source_path: None,
                destination_path: Some(display_workspace_path(&destination.path)),
                source_sha256: None,
                postimage_sha256: Some(hex_sha256(&content)),
                source_mode: None,
                destination_mode: Some(mode),
                classification: Some(classification),
                work_disposition: None,
                sibling_snapshot_sha256: Some(canonical_sha256(
                    &destination.observed_sibling_names,
                )?),
                structured_patch_sha256: None,
                source_bytes: Vec::new(),
                postimage_bytes: content,
                operation_sha256: ZERO_SHA256.to_owned(),
            }
        }
        FilesystemOperationDraft::ExactPatch {
            operation_id,
            source,
            patch_json,
            expected_postimage_sha256,
        } => {
            validate_source(parent, &source, true)?;
            let patch = parse_structured_patch_json(&patch_json)?;
            let postimage = apply_structured_patch(&source.observed_bytes, &patch)?;
            if !valid_sha256(&expected_postimage_sha256)
                || hex_sha256(&postimage) != expected_postimage_sha256
            {
                return Err(FilesystemPlanError::PatchRejected);
            }
            account_bytes(total_bytes, source.observed_bytes.len())?;
            account_bytes(total_bytes, postimage.len())?;
            FilesystemOperation {
                operation_id,
                kind: FilesystemOperationKind::ExactPatch,
                source_path: Some(display_target_path(&source.target)),
                source_sha256: Some(hex_sha256(&source.observed_bytes)),
                postimage_sha256: Some(expected_postimage_sha256),
                source_mode: Some(source.mode),
                destination_mode: Some(source.mode),
                classification: None,
                work_disposition: Some(source.work_disposition),
                sibling_snapshot_sha256: None,
                structured_patch_sha256: Some(hex_sha256(&patch_json)),
                source_bytes: source.observed_bytes,
                postimage_bytes: postimage,
                source: Some(source.target),
                destination_parent: None,
                destination_path: None,
                operation_sha256: ZERO_SHA256.to_owned(),
            }
        }
        FilesystemOperationDraft::Copy {
            operation_id,
            source,
            destination,
            classification,
        } => {
            validate_source(parent, &source, false)?;
            validate_destination(parent, &destination, true)?;
            account_bytes(total_bytes, source.observed_bytes.len())?;
            FilesystemOperation {
                operation_id,
                kind: FilesystemOperationKind::Copy,
                source_path: Some(display_target_path(&source.target)),
                destination_path: Some(display_workspace_path(&destination.path)),
                source_sha256: Some(hex_sha256(&source.observed_bytes)),
                postimage_sha256: Some(hex_sha256(&source.observed_bytes)),
                source_mode: Some(source.mode),
                destination_mode: Some(source.mode),
                classification: Some(classification),
                work_disposition: Some(source.work_disposition),
                sibling_snapshot_sha256: Some(canonical_sha256(
                    &destination.observed_sibling_names,
                )?),
                structured_patch_sha256: None,
                postimage_bytes: source.observed_bytes.clone(),
                source_bytes: source.observed_bytes,
                source: Some(source.target),
                destination_parent: Some(destination.parent),
                operation_sha256: ZERO_SHA256.to_owned(),
            }
        }
        FilesystemOperationDraft::Move {
            operation_id,
            source,
            destination,
        } => {
            validate_source(parent, &source, true)?;
            validate_destination(parent, &destination, true)?;
            if display_target_path(&source.target) == display_workspace_path(&destination.path) {
                return Err(FilesystemPlanError::OperationConflict);
            }
            account_bytes(total_bytes, source.observed_bytes.len())?;
            move_like_operation(
                operation_id,
                FilesystemOperationKind::Move,
                source,
                destination,
            )?
        }
        FilesystemOperationDraft::TrashDelete {
            operation_id,
            source,
            trash_destination,
        } => {
            validate_source(parent, &source, true)?;
            validate_destination(parent, &trash_destination, true)?;
            if display_target_path(&source.target)
                == display_workspace_path(&trash_destination.path)
            {
                return Err(FilesystemPlanError::OperationConflict);
            }
            account_bytes(total_bytes, source.observed_bytes.len())?;
            move_like_operation(
                operation_id,
                FilesystemOperationKind::TrashDelete,
                source,
                trash_destination,
            )?
        }
    };
    operation.operation_sha256 = operation_digest(&operation)?;
    Ok(operation)
}

fn move_like_operation(
    operation_id: String,
    kind: FilesystemOperationKind,
    source: ExistingSourceDraft,
    destination: NewDestinationDraft,
) -> Result<FilesystemOperation, FilesystemPlanError> {
    Ok(FilesystemOperation {
        operation_id,
        kind,
        source_path: Some(display_target_path(&source.target)),
        destination_path: Some(display_workspace_path(&destination.path)),
        source_sha256: Some(hex_sha256(&source.observed_bytes)),
        postimage_sha256: Some(hex_sha256(&source.observed_bytes)),
        source_mode: Some(source.mode),
        destination_mode: Some(source.mode),
        classification: None,
        work_disposition: Some(source.work_disposition),
        sibling_snapshot_sha256: Some(canonical_sha256(&destination.observed_sibling_names)?),
        structured_patch_sha256: None,
        postimage_bytes: source.observed_bytes.clone(),
        source_bytes: source.observed_bytes,
        source: Some(source.target),
        destination_parent: Some(destination.parent),
        operation_sha256: ZERO_SHA256.to_owned(),
    })
}

fn validate_source(
    parent: &CapabilityGrant,
    source: &ExistingSourceDraft,
    mutation: bool,
) -> Result<(), FilesystemPlanError> {
    if source.target.workspace_path().is_none()
        || source.target.object_kind() != Some(WorkspaceObjectKind::RegularFile)
        || source.target.path_components().len() > MAX_PATH_DEPTH
        || source.observed_bytes.len() > MAX_FILE_BYTES
    {
        return Err(FilesystemPlanError::InvalidInput);
    }
    validate_mode(source.mode)?;
    validate_inside_parent(parent, &source.target)?;
    let preimage = source
        .target
        .preimage()
        .ok_or(FilesystemPlanError::PreimageMismatch)?;
    if preimage.byte_len() != u64::try_from(source.observed_bytes.len()).unwrap_or(u64::MAX)
        || hex_bytes(preimage.content_sha256()) != hex_sha256(&source.observed_bytes)
    {
        return Err(FilesystemPlanError::PreimageMismatch);
    }
    if mutation {
        if protected_path(&source.target) {
            return Err(FilesystemPlanError::ProtectedPath);
        }
        if !source.work_disposition.permits_mutation() {
            return Err(FilesystemPlanError::UnrelatedExistingWork);
        }
    }
    Ok(())
}

fn validate_destination(
    parent: &CapabilityGrant,
    destination: &NewDestinationDraft,
    protect_name: bool,
) -> Result<(), FilesystemPlanError> {
    if destination.parent.workspace_path().is_none()
        || destination.parent.object_kind() != Some(WorkspaceObjectKind::Directory)
        || destination.parent.preimage().is_some()
        || destination.path.components().len() > MAX_PATH_DEPTH
        || destination.observed_sibling_names.len() > MAX_SIBLING_NAMES
    {
        return Err(FilesystemPlanError::InvalidInput);
    }
    validate_inside_parent(parent, &destination.parent)?;
    if destination.path.workspace_id() != destination.parent.workspace_id()
        || destination.path.components().len() != destination.parent.path_components().len() + 1
        || !destination
            .path
            .components()
            .starts_with(destination.parent.path_components())
    {
        return Err(FilesystemPlanError::ScopeDenied);
    }
    let candidate = destination
        .path
        .components()
        .last()
        .ok_or(FilesystemPlanError::InvalidInput)?
        .as_str();
    if protect_name && protected_components(destination.path.components()) {
        return Err(FilesystemPlanError::ProtectedPath);
    }
    let candidate_folded = candidate.to_lowercase();
    let mut exact_names = BTreeSet::new();
    for sibling in &destination.observed_sibling_names {
        let sibling_path =
            WorkspacePath::new(destination.path.workspace_id().clone(), [sibling.as_str()])
                .map_err(|_| FilesystemPlanError::InvalidInput)?;
        let normalized = sibling_path.components()[0].as_str();
        if !exact_names.insert(normalized.to_owned())
            || normalized == candidate
            || normalized.to_lowercase() == candidate_folded
        {
            return Err(FilesystemPlanError::DestinationCollision);
        }
    }
    Ok(())
}

fn validate_inside_parent(
    parent: &CapabilityGrant,
    target: &GrantTarget,
) -> Result<(), FilesystemPlanError> {
    if !parent.targets.iter().any(|scope| scope.contains(target))
        || parent
            .excluded_targets
            .iter()
            .any(|excluded| excluded.contains(target))
    {
        return Err(FilesystemPlanError::ScopeDenied);
    }
    Ok(())
}

fn protected_path(target: &GrantTarget) -> bool {
    protected_components(target.path_components())
}

fn protected_components(components: &[agentmage_kernel_contracts::WorkspacePathComponent]) -> bool {
    components.iter().any(|component| {
        let value = component.as_str().to_lowercase();
        matches!(
            value.as_str(),
            ".git"
                | ".codex"
                | ".agentmage"
                | ".ssh"
                | "agents.md"
                | "claude.md"
                | "memory.md"
                | "ai_memory.md"
                | "project_memory.md"
                | "handoff.md"
                | ".env"
                | "credentials"
                | "credentials.json"
                | "secrets"
                | "secrets.json"
                | "source-records"
                | "source_records"
        ) || value.starts_with(".env.")
            || value.ends_with(".pem")
            || value.ends_with(".key")
            || value.ends_with(".p12")
            || value.ends_with(".pfx")
            || value.ends_with("-handoff.md")
            || value.ends_with("_handoff.md")
            || value.contains("instructions.md")
    })
}

fn verify_plan(plan: &FilesystemPlan) -> Result<(), FilesystemPlanError> {
    if plan.operations.is_empty()
        || plan.operations.len() > MAX_OPERATIONS
        || plan.plan_sha256 == ZERO_SHA256
        || plan_digest(plan)? != plan.plan_sha256
        || plan.operations.iter().any(|operation| {
            operation_digest(operation).as_deref() != Ok(operation.operation_sha256.as_str())
        })
    {
        return Err(FilesystemPlanError::InvalidInput);
    }
    Ok(())
}

fn operation_digest(operation: &FilesystemOperation) -> Result<String, FilesystemPlanError> {
    let mut canonical_operation = operation.clone();
    canonical_operation.operation_sha256 = ZERO_SHA256.to_owned();
    canonical_sha256(&(
        canonical_operation,
        hex_sha256(&operation.source_bytes),
        hex_sha256(&operation.postimage_bytes),
    ))
}

fn plan_digest(plan: &FilesystemPlan) -> Result<String, FilesystemPlanError> {
    canonical_sha256(&(
        &plan.plan_id,
        &plan.workspace_id,
        &plan.parent_grant_id,
        &plan.parent_grant_sha256,
        plan.observed_at_epoch_ms,
        &plan.operations,
        plan.operations
            .iter()
            .map(|operation| {
                (
                    hex_sha256(&operation.source_bytes),
                    hex_sha256(&operation.postimage_bytes),
                )
            })
            .collect::<Vec<_>>(),
        &plan.review,
        &plan.permitted_verification,
        plan.requires_high_risk_delete_grant,
    ))
}

fn account_bytes(total: &mut usize, additional: usize) -> Result<(), FilesystemPlanError> {
    *total = total
        .checked_add(additional)
        .ok_or(FilesystemPlanError::InvalidInput)?;
    if *total > MAX_TRANSACTION_BYTES {
        return Err(FilesystemPlanError::InvalidInput);
    }
    Ok(())
}

fn validate_identifier(value: &str) -> Result<(), FilesystemPlanError> {
    if value.is_empty()
        || value.len() > MAX_IDENTIFIER_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
    {
        return Err(FilesystemPlanError::InvalidInput);
    }
    Ok(())
}

fn validate_mode(mode: u32) -> Result<(), FilesystemPlanError> {
    if mode > 0o777 {
        return Err(FilesystemPlanError::InvalidInput);
    }
    Ok(())
}

fn validate_review(review: &WriteReviewNarrative) -> Result<(), FilesystemPlanError> {
    for value in [&review.rationale, &review.behavior_change, &review.rollback] {
        if value.is_empty() || value.len() > MAX_REVIEW_TEXT_BYTES {
            return Err(FilesystemPlanError::InvalidInput);
        }
    }
    for values in [
        &review.verification_plan,
        &review.risks,
        &review.unverified_assumptions,
    ] {
        if values.is_empty()
            || values.len() > MAX_REVIEW_ITEMS
            || values
                .iter()
                .any(|value| value.is_empty() || value.len() > MAX_REVIEW_TEXT_BYTES)
        {
            return Err(FilesystemPlanError::InvalidInput);
        }
    }
    Ok(())
}

fn validate_verification(values: &[String]) -> Result<(), FilesystemPlanError> {
    if values.is_empty() || values.len() > MAX_REVIEW_ITEMS {
        return Err(FilesystemPlanError::InvalidInput);
    }
    let mut unique = BTreeSet::new();
    for value in values {
        validate_identifier(value)?;
        if !unique.insert(value) {
            return Err(FilesystemPlanError::InvalidInput);
        }
    }
    Ok(())
}

fn display_target_path(target: &GrantTarget) -> String {
    target
        .path_components()
        .iter()
        .map(|component| component.as_str())
        .collect::<Vec<_>>()
        .join("/")
}

fn display_workspace_path(path: &WorkspacePath) -> String {
    path.components()
        .iter()
        .map(|component| component.as_str())
        .collect::<Vec<_>>()
        .join("/")
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn canonical_sha256(value: &impl Serialize) -> Result<String, FilesystemPlanError> {
    serde_json::to_vec(value)
        .map(|bytes| hex_sha256(&bytes))
        .map_err(|_| FilesystemPlanError::InvalidInput)
}

fn hex_sha256(value: &[u8]) -> String {
    hex_bytes(&Sha256::digest(value))
}

fn hex_bytes(value: &[u8]) -> String {
    let mut encoded = String::with_capacity(value.len() * 2);
    for byte in value {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        ActionId, ActionKind, ActorId, AdapterInstanceId, ApprovalId, AuthorizedWorkspaceHandle,
        DataSensitivity, GrantId, GrantNonce, GrantOperation, GrantTarget, PathPlatform, SessionId,
        TaskId, ToolId, WorkspaceAuthorizationId, WorkspaceId, WorkspacePath, WorkspaceScopePath,
    };
    use serde_json::json;
    use sha2::{Digest, Sha256};

    use super::{
        ExistingSourceDraft, ExistingWorkDisposition, FileClassification,
        FilesystemApprovalDecision, FilesystemApprovalPreview, FilesystemGrantRequest,
        FilesystemOperationDraft, FilesystemOperationKind, FilesystemPlan, FilesystemPlanError,
        FilesystemPlanRequest, NewDestinationDraft, StructuredPatch, StructuredPatchHunk,
        apply_structured_patch, build_filesystem_plan, hex_sha256, issue_filesystem_grant,
        parse_structured_patch_json, render_filesystem_preview,
    };
    use crate::grants::{GrantIssuer, SessionReadGrantRequest};
    use crate::write_approval::WriteReviewNarrative;

    #[derive(Debug)]
    struct FakeWorkspace;

    impl AuthorizedWorkspaceHandle for FakeWorkspace {
        fn workspace_id(&self) -> &WorkspaceId {
            static ID: std::sync::OnceLock<WorkspaceId> = std::sync::OnceLock::new();
            ID.get_or_init(|| WorkspaceId::from_raw("workspace-filesystem"))
        }

        fn authorization_id(&self) -> &WorkspaceAuthorizationId {
            static ID: std::sync::OnceLock<WorkspaceAuthorizationId> = std::sync::OnceLock::new();
            ID.get_or_init(|| WorkspaceAuthorizationId::from_raw("authorization-filesystem"))
        }

        fn adapter_instance_id(&self) -> &AdapterInstanceId {
            static ID: std::sync::OnceLock<AdapterInstanceId> = std::sync::OnceLock::new();
            ID.get_or_init(|| AdapterInstanceId::from_raw("adapter-filesystem"))
        }

        fn platform(&self) -> PathPlatform {
            PathPlatform::DeterministicFake
        }
    }

    fn parent_with_issuer() -> (GrantIssuer, agentmage_kernel_contracts::CapabilityGrant) {
        let mut issuer = GrantIssuer::new();
        let root = GrantTarget::workspace_scope(
            &FakeWorkspace,
            WorkspaceScopePath::new(
                WorkspaceId::from_raw("workspace-filesystem"),
                Vec::<String>::new(),
            )
            .expect("root scope"),
        )
        .expect("root target");
        let private = GrantTarget::workspace_scope(
            &FakeWorkspace,
            WorkspaceScopePath::new(WorkspaceId::from_raw("workspace-filesystem"), ["private"])
                .expect("private scope"),
        )
        .expect("private target");
        let parent = issuer
            .issue_session_read(SessionReadGrantRequest {
                grant_id: GrantId::from_raw("grant-parent-filesystem"),
                actor_id: ActorId::from_raw("actor-local"),
                session_id: SessionId::from_raw("session-filesystem"),
                task_id: TaskId::from_raw("task-filesystem"),
                targets: vec![root],
                excluded_targets: vec![private],
                sensitivity: DataSensitivity::Restricted,
                issued_at_epoch_ms: 1_000,
                expires_at_epoch_ms: 100_000,
                nonce: GrantNonce::from_raw("nonce-parent-filesystem"),
                maximum_derived_operations: 10,
                preview_sha256: "a".repeat(64),
                policy_sha256: "b".repeat(64),
            })
            .expect("parent");
        (issuer, parent)
    }

    fn parent() -> agentmage_kernel_contracts::CapabilityGrant {
        parent_with_issuer().1
    }

    fn source(path: &[&str], bytes: &[u8], work: ExistingWorkDisposition) -> ExistingSourceDraft {
        let content: [u8; 32] = Sha256::digest(bytes).into();
        let identity: [u8; 32] = Sha256::digest(path.join("/").as_bytes()).into();
        let target: GrantTarget = serde_json::from_value(json!({
            "target_kind": "held_object",
            "path": {"workspace_id": "workspace-filesystem", "components": path},
            "authorization_id": "authorization-filesystem",
            "adapter_instance_id": "adapter-filesystem",
            "platform": "deterministic_fake",
            "object_kind": "regular_file",
            "object_identity": {
                "platform": "deterministic_fake",
                "mount_identity_sha256": vec![1_u8; 32],
                "object_identity_sha256": identity
            },
            "preimage": {"byte_len": bytes.len(), "content_sha256": content}
        }))
        .expect("source target");
        ExistingSourceDraft {
            target,
            observed_bytes: bytes.to_vec(),
            work_disposition: work,
            mode: 0o640,
        }
    }

    fn destination(parent_path: &[&str], name: &str, siblings: &[&str]) -> NewDestinationDraft {
        let identity: [u8; 32] = Sha256::digest(parent_path.join("/").as_bytes()).into();
        let parent: GrantTarget = serde_json::from_value(json!({
            "target_kind": "held_object",
            "path": {"workspace_id": "workspace-filesystem", "components": parent_path},
            "authorization_id": "authorization-filesystem",
            "adapter_instance_id": "adapter-filesystem",
            "platform": "deterministic_fake",
            "object_kind": "directory",
            "object_identity": {
                "platform": "deterministic_fake",
                "mount_identity_sha256": vec![1_u8; 32],
                "object_identity_sha256": identity
            },
            "preimage": null
        }))
        .expect("destination parent");
        let components: Vec<_> = parent_path
            .iter()
            .copied()
            .chain(std::iter::once(name))
            .collect();
        NewDestinationDraft {
            parent,
            path: WorkspacePath::new(WorkspaceId::from_raw("workspace-filesystem"), components)
                .expect("destination path"),
            observed_sibling_names: siblings.iter().map(|value| (*value).to_owned()).collect(),
        }
    }

    fn review() -> WriteReviewNarrative {
        WriteReviewNarrative {
            rationale: "Apply the exact controlled filesystem fixture".to_owned(),
            behavior_change: "Only the reviewed fixture paths change".to_owned(),
            verification_plan: vec!["Run the focused filesystem checks".to_owned()],
            risks: vec!["A fixture may depend on the prior path".to_owned()],
            rollback: "Apply a fresh reviewed inverse transaction".to_owned(),
            unverified_assumptions: vec!["No external fixture consumer was inspected".to_owned()],
        }
    }

    fn request(operations: Vec<FilesystemOperationDraft>) -> FilesystemPlanRequest {
        FilesystemPlanRequest {
            plan_id: "filesystem-plan-0001".to_owned(),
            observed_at_epoch_ms: 2_000,
            operations,
            review: review(),
            permitted_verification: vec!["cargo-test-filesystem-control".to_owned()],
        }
    }

    fn patch(old: &str, new: &str) -> Vec<u8> {
        serde_json::to_vec(&StructuredPatch {
            schema_version: 1,
            hunks: vec![StructuredPatchHunk {
                old_start_line: 2,
                old_lines: vec![old.to_owned()],
                new_lines: vec![new.to_owned()],
            }],
        })
        .expect("patch json")
    }

    fn approval_decision(
        preview: &FilesystemApprovalPreview,
        high_risk_delete_confirmed: bool,
    ) -> FilesystemApprovalDecision {
        FilesystemApprovalDecision {
            approval_id: ApprovalId::from_raw("approval-filesystem-0001"),
            approved_plan_sha256: preview.plan_sha256.clone(),
            approved_preview_sha256: preview.preview_sha256.clone(),
            approved_at_epoch_ms: 3_000,
            expires_at_epoch_ms: 30_000,
            permitted_verification: vec!["cargo-test-filesystem-control".to_owned()],
            user_confirmed: true,
            high_risk_delete_confirmed,
        }
    }

    fn grant_request() -> FilesystemGrantRequest {
        FilesystemGrantRequest {
            parent_grant_id: GrantId::from_raw("grant-parent-filesystem"),
            grant_id: GrantId::from_raw("grant-filesystem-0001"),
            action_id: ActionId::from_raw("action-filesystem-0001"),
            action_kind: ActionKind::DeterministicTool,
            tool_id: ToolId::from_raw("workspace.filesystem"),
            tool_version: "1.0.0".to_owned(),
            nonce: GrantNonce::from_raw("nonce-filesystem-0001"),
            policy_sha256: "b".repeat(64),
        }
    }

    fn write_approval_fixture() -> (GrantIssuer, FilesystemPlan, FilesystemApprovalPreview) {
        let (issuer, parent) = parent_with_issuer();
        let operations = vec![
            FilesystemOperationDraft::Create {
                operation_id: "operation-create".to_owned(),
                destination: destination(&["new"], "created.txt", &[]),
                content: b"created\n".to_vec(),
                mode: 0o600,
                classification: FileClassification::Documentation,
            },
            FilesystemOperationDraft::Copy {
                operation_id: "operation-copy".to_owned(),
                source: source(
                    &["src", "copy.txt"],
                    b"copy\n",
                    ExistingWorkDisposition::Clean,
                ),
                destination: destination(&["copies"], "copy.txt", &[]),
                classification: FileClassification::Data,
            },
        ];
        let plan = build_filesystem_plan(&parent, request(operations)).expect("write plan");
        let preview = render_filesystem_preview(&plan).expect("write preview");
        (issuer, plan, preview)
    }

    fn delete_approval_fixture() -> (GrantIssuer, FilesystemPlan, FilesystemApprovalPreview) {
        let (issuer, parent) = parent_with_issuer();
        let operation = FilesystemOperationDraft::TrashDelete {
            operation_id: "operation-trash".to_owned(),
            source: source(
                &["src", "obsolete.txt"],
                b"obsolete\n",
                ExistingWorkDisposition::Clean,
            ),
            trash_destination: destination(&["trash"], "obsolete.txt", &[]),
        };
        let plan = build_filesystem_plan(&parent, request(vec![operation])).expect("delete plan");
        let preview = render_filesystem_preview(&plan).expect("delete preview");
        (issuer, plan, preview)
    }

    #[test]
    fn structured_patch_is_closed_exact_and_non_overlapping() {
        let source = b"alpha\nbeta\ngamma\n";
        let bytes = patch("beta\n", "changed\n");
        let parsed = parse_structured_patch_json(&bytes).expect("parsed patch");
        assert_eq!(
            apply_structured_patch(source, &parsed),
            Ok(b"alpha\nchanged\ngamma\n".to_vec())
        );

        let mut stale = parsed.clone();
        stale.hunks[0].old_lines[0] = "wrong\n".to_owned();
        assert_eq!(
            apply_structured_patch(source, &stale),
            Err(FilesystemPlanError::PatchRejected)
        );
        let overlapping = StructuredPatch {
            schema_version: 1,
            hunks: vec![
                StructuredPatchHunk {
                    old_start_line: 1,
                    old_lines: vec!["alpha\n".to_owned(), "beta\n".to_owned()],
                    new_lines: vec!["first\n".to_owned()],
                },
                StructuredPatchHunk {
                    old_start_line: 2,
                    old_lines: vec!["beta\n".to_owned()],
                    new_lines: vec!["second\n".to_owned()],
                },
            ],
        };
        assert_eq!(
            apply_structured_patch(source, &overlapping),
            Err(FilesystemPlanError::PatchRejected)
        );
        let unknown = br#"{"schema_version":1,"hunks":[],"command":"rm"}"#;
        assert_eq!(
            parse_structured_patch_json(unknown),
            Err(FilesystemPlanError::PatchRejected)
        );
    }

    #[test]
    fn create_patch_copy_and_move_render_exact_non_overwriting_preview() {
        let original = b"alpha\nbeta\ngamma\n";
        let patched = b"alpha\nchanged\ngamma\n";
        let operations = vec![
            FilesystemOperationDraft::Create {
                operation_id: "operation-create".to_owned(),
                destination: destination(&["new"], "created.txt", &["existing.txt"]),
                content: b"created\n".to_vec(),
                mode: 0o640,
                classification: FileClassification::Documentation,
            },
            FilesystemOperationDraft::ExactPatch {
                operation_id: "operation-patch".to_owned(),
                source: source(
                    &["src", "patch.txt"],
                    original,
                    ExistingWorkDisposition::Clean,
                ),
                patch_json: patch("beta\n", "changed\n"),
                expected_postimage_sha256: hex_sha256(patched),
            },
            FilesystemOperationDraft::Copy {
                operation_id: "operation-copy".to_owned(),
                source: source(
                    &["src", "copy.txt"],
                    b"copy\n",
                    ExistingWorkDisposition::UnrelatedModified,
                ),
                destination: destination(&["copies"], "copy.txt", &[]),
                classification: FileClassification::Data,
            },
            FilesystemOperationDraft::Move {
                operation_id: "operation-move".to_owned(),
                source: source(
                    &["src", "move.txt"],
                    b"move\n",
                    ExistingWorkDisposition::OwnedByCurrentTask,
                ),
                destination: destination(&["moved"], "move.txt", &[]),
            },
        ];
        let plan = build_filesystem_plan(&parent(), request(operations)).expect("complete plan");
        let preview = render_filesystem_preview(&plan).expect("complete preview");

        assert_eq!(preview.operations.len(), 4);
        assert_eq!(preview.operations[0].kind, FilesystemOperationKind::Create);
        assert_eq!(
            preview.operations[1].kind,
            FilesystemOperationKind::ExactPatch
        );
        assert!(preview.operations[0].complete_content_preview.is_some());
        assert!(preview.operations[1].complete_content_preview.is_some());
        assert!(preview.operations[2].complete_content_preview.is_none());
        assert!(preview.operations.iter().all(|item| {
            !item.overwrite_allowed && !item.implicit_parent_creation && !item.permanent_delete
        }));
        assert!(!preview.requires_high_risk_delete_grant);
        assert_eq!(preview.plan_sha256, plan.plan_sha256());
    }

    #[test]
    fn trash_delete_is_isolated_high_risk_and_never_permanent() {
        let operation = FilesystemOperationDraft::TrashDelete {
            operation_id: "operation-trash".to_owned(),
            source: source(
                &["src", "obsolete.txt"],
                b"obsolete\n",
                ExistingWorkDisposition::Clean,
            ),
            trash_destination: destination(&["trash"], "obsolete.txt", &[]),
        };
        let plan = build_filesystem_plan(&parent(), request(vec![operation])).expect("trash plan");
        let preview = render_filesystem_preview(&plan).expect("trash preview");
        assert!(plan.requires_high_risk_delete_grant());
        assert!(preview.requires_high_risk_delete_grant);
        assert_eq!(
            preview.operations[0].kind,
            FilesystemOperationKind::TrashDelete
        );
        assert!(!preview.operations[0].permanent_delete);

        let mixed = vec![
            FilesystemOperationDraft::TrashDelete {
                operation_id: "operation-trash".to_owned(),
                source: source(
                    &["src", "obsolete.txt"],
                    b"obsolete\n",
                    ExistingWorkDisposition::Clean,
                ),
                trash_destination: destination(&["trash"], "obsolete.txt", &[]),
            },
            FilesystemOperationDraft::Create {
                operation_id: "operation-create".to_owned(),
                destination: destination(&["new"], "created.txt", &[]),
                content: b"created\n".to_vec(),
                mode: 0o600,
                classification: FileClassification::Data,
            },
        ];
        assert_eq!(
            build_filesystem_plan(&parent(), request(mixed)),
            Err(FilesystemPlanError::OperationConflict)
        );
    }

    #[test]
    fn protected_dirty_colliding_stale_and_duplicate_operations_fail_closed() {
        let protected = FilesystemOperationDraft::ExactPatch {
            operation_id: "operation-protected".to_owned(),
            source: source(&["AGENTS.md"], b"old\n", ExistingWorkDisposition::Clean),
            patch_json: serde_json::to_vec(&StructuredPatch {
                schema_version: 1,
                hunks: vec![StructuredPatchHunk {
                    old_start_line: 1,
                    old_lines: vec!["old\n".to_owned()],
                    new_lines: vec!["new\n".to_owned()],
                }],
            })
            .expect("protected patch"),
            expected_postimage_sha256: hex_sha256(b"new\n"),
        };
        assert_eq!(
            build_filesystem_plan(&parent(), request(vec![protected])),
            Err(FilesystemPlanError::ProtectedPath)
        );

        let dirty = FilesystemOperationDraft::Move {
            operation_id: "operation-dirty".to_owned(),
            source: source(
                &["src", "dirty.txt"],
                b"dirty\n",
                ExistingWorkDisposition::UnrelatedModified,
            ),
            destination: destination(&["moved"], "dirty.txt", &[]),
        };
        assert_eq!(
            build_filesystem_plan(&parent(), request(vec![dirty])),
            Err(FilesystemPlanError::UnrelatedExistingWork)
        );

        let collision = FilesystemOperationDraft::Create {
            operation_id: "operation-collision".to_owned(),
            destination: destination(&["new"], "Readme.md", &["README.md"]),
            content: b"collision\n".to_vec(),
            mode: 0o600,
            classification: FileClassification::Documentation,
        };
        assert_eq!(
            build_filesystem_plan(&parent(), request(vec![collision])),
            Err(FilesystemPlanError::DestinationCollision)
        );

        let mut stale_source = source(
            &["src", "stale.txt"],
            b"original\n",
            ExistingWorkDisposition::Clean,
        );
        stale_source.observed_bytes = b"changed\n".to_vec();
        let stale = FilesystemOperationDraft::Copy {
            operation_id: "operation-stale".to_owned(),
            source: stale_source,
            destination: destination(&["copies"], "stale.txt", &[]),
            classification: FileClassification::Data,
        };
        assert_eq!(
            build_filesystem_plan(&parent(), request(vec![stale])),
            Err(FilesystemPlanError::PreimageMismatch)
        );

        let duplicates = vec![
            FilesystemOperationDraft::Create {
                operation_id: "operation-first".to_owned(),
                destination: destination(&["new"], "same.txt", &[]),
                content: b"first\n".to_vec(),
                mode: 0o600,
                classification: FileClassification::Data,
            },
            FilesystemOperationDraft::Create {
                operation_id: "operation-second".to_owned(),
                destination: destination(&["new"], "same.txt", &[]),
                content: b"second\n".to_vec(),
                mode: 0o600,
                classification: FileClassification::Data,
            },
        ];
        assert_eq!(
            build_filesystem_plan(&parent(), request(duplicates)),
            Err(FilesystemPlanError::OperationConflict)
        );
    }

    #[test]
    fn preview_rejects_plan_and_operation_digest_tampering() {
        let operation = FilesystemOperationDraft::Create {
            operation_id: "operation-create".to_owned(),
            destination: destination(&["new"], "created.txt", &[]),
            content: b"created\n".to_vec(),
            mode: 0o600,
            classification: FileClassification::Data,
        };
        let plan = build_filesystem_plan(&parent(), request(vec![operation])).expect("plan");

        let mut changed_plan = plan.clone();
        changed_plan.plan_sha256 = "f".repeat(64);
        assert_eq!(
            render_filesystem_preview(&changed_plan),
            Err(FilesystemPlanError::InvalidInput)
        );
        let mut changed_operation = plan;
        changed_operation.operations[0].postimage_bytes.push(b'!');
        assert_eq!(
            render_filesystem_preview(&changed_operation),
            Err(FilesystemPlanError::InvalidInput)
        );
    }

    #[test]
    fn exact_approval_issues_one_single_use_write_grant() {
        let (mut issuer, plan, preview) = write_approval_fixture();
        let receipt = issue_filesystem_grant(
            &mut issuer,
            &plan,
            &preview,
            &approval_decision(&preview, false),
            grant_request(),
        )
        .expect("write grant");

        assert_eq!(receipt.grant.use_limit, 1);
        assert_eq!(receipt.grant.use_count, 0);
        assert_eq!(
            receipt.grant.operation.operation(),
            GrantOperation::WorkspaceWrite
        );
        assert_eq!(receipt.grant.argument_sha256, plan.plan_sha256());
        assert_eq!(receipt.grant.preview_sha256, preview.preview_sha256);
        assert_eq!(receipt.grant.targets.len(), 3);
        assert_eq!(receipt.grant.preimages.len(), 1);
        assert_eq!(receipt.grant.preimages[0].target_index, 1);
        assert_eq!(receipt.grant.expected_side_effects.len(), 2);
        assert_eq!(receipt.grant.expected_side_effects[0].target_indexes, [0]);
        assert_eq!(
            receipt.grant.expected_side_effects[1].target_indexes,
            [1, 2]
        );
        assert_eq!(
            receipt.permitted_verification,
            ["cargo-test-filesystem-control"]
        );

        let mut replay = grant_request();
        replay.grant_id = GrantId::from_raw("grant-filesystem-0002");
        replay.nonce = GrantNonce::from_raw("nonce-filesystem-0002");
        assert_eq!(
            issue_filesystem_grant(
                &mut issuer,
                &plan,
                &preview,
                &approval_decision(&preview, false),
                replay,
            ),
            Err(FilesystemPlanError::ApprovalMismatch)
        );
    }

    #[test]
    fn trash_delete_requires_separate_confirmation_and_delete_authority() {
        let (mut issuer, plan, preview) = delete_approval_fixture();
        assert_eq!(
            issue_filesystem_grant(
                &mut issuer,
                &plan,
                &preview,
                &approval_decision(&preview, false),
                grant_request(),
            ),
            Err(FilesystemPlanError::ApprovalMismatch)
        );

        let (mut issuer, plan, preview) = delete_approval_fixture();
        let receipt = issue_filesystem_grant(
            &mut issuer,
            &plan,
            &preview,
            &approval_decision(&preview, true),
            grant_request(),
        )
        .expect("delete grant");
        assert_eq!(
            receipt.grant.operation.operation(),
            GrantOperation::WorkspaceDelete
        );
        assert_eq!(receipt.grant.targets.len(), 2);
        assert_eq!(receipt.grant.preimages.len(), 1);
        assert_eq!(
            receipt.grant.expected_side_effects[0].target_indexes,
            [0, 1]
        );
    }

    #[test]
    fn approval_mutations_cannot_issue_filesystem_authority() {
        type DecisionMutation = Box<dyn Fn(&mut FilesystemApprovalDecision)>;

        let (mut issuer, plan, preview) = write_approval_fixture();
        let mut changed_preview = preview.clone();
        changed_preview.operations[0].destination_path = Some("new/other.txt".to_owned());
        assert_eq!(
            issue_filesystem_grant(
                &mut issuer,
                &plan,
                &changed_preview,
                &approval_decision(&preview, false),
                grant_request(),
            ),
            Err(FilesystemPlanError::ApprovalMismatch)
        );

        let (mut issuer, plan, preview) = write_approval_fixture();
        let mut changed_plan = plan.clone();
        changed_plan.operations[0].destination_mode = Some(0o644);
        assert_eq!(
            issue_filesystem_grant(
                &mut issuer,
                &changed_plan,
                &preview,
                &approval_decision(&preview, false),
                grant_request(),
            ),
            Err(FilesystemPlanError::InvalidInput)
        );

        let decision_mutations: Vec<DecisionMutation> = vec![
            Box::new(|value| value.user_confirmed = false),
            Box::new(|value| value.approved_plan_sha256 = "c".repeat(64)),
            Box::new(|value| value.approved_preview_sha256 = "c".repeat(64)),
            Box::new(|value| value.expires_at_epoch_ms = value.approved_at_epoch_ms + 300_001),
            Box::new(|value| value.high_risk_delete_confirmed = true),
            Box::new(|value| {
                value
                    .permitted_verification
                    .push("cargo-test-all".to_owned())
            }),
        ];
        for mutate in decision_mutations {
            let (mut issuer, plan, preview) = write_approval_fixture();
            let mut decision = approval_decision(&preview, false);
            mutate(&mut decision);
            assert_eq!(
                issue_filesystem_grant(&mut issuer, &plan, &preview, &decision, grant_request(),),
                Err(FilesystemPlanError::ApprovalMismatch)
            );
        }

        let (mut issuer, plan, preview) = write_approval_fixture();
        let mut wrong_policy = grant_request();
        wrong_policy.policy_sha256 = "c".repeat(64);
        assert_eq!(
            issue_filesystem_grant(
                &mut issuer,
                &plan,
                &preview,
                &approval_decision(&preview, false),
                wrong_policy,
            ),
            Err(FilesystemPlanError::GrantIssue)
        );
    }
}
