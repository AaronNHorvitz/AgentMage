//! Authority-free controlled filesystem plans, structured patches, and exact previews.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
};

use agentmage_kernel_contracts::{
    ActionId, ActionKind, ApprovalId, CapabilityGrant, GrantClass, GrantId, GrantNonce,
    GrantOperation, GrantPreimage, GrantSideEffect, GrantStatus, GrantTarget, OperationBinding,
    ToolId, WorkspaceObjectKind, WorkspacePath,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    grants::{
        DerivedOperationGrantRequest, GrantConsumeError, GrantConsumptionRecord, GrantIssueError,
        GrantIssuer,
    },
    policy::{PolicyEngine, PolicyEvaluationContext},
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

/// Stable reason a controlled-filesystem transaction cannot produce a reconciled result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilesystemTransactionError {
    /// Transaction input or a retained binding is malformed.
    InvalidInput,
    /// Fresh source or destination observations differ from the approved pre-state.
    PreapplyDenied,
    /// Current deterministic policy denied final grant consumption.
    PolicyDenied,
    /// The platform driver could not produce a bounded observation.
    ObservationFailed,
    /// The platform driver returned a malformed or contradictory report.
    DriverReportInvalid,
    /// Receipt hashing or lifecycle state became inconsistent.
    ReceiptIntegrity,
}

impl FilesystemTransactionError {
    /// Returns a stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "filesystem.transaction.invalid_input",
            Self::PreapplyDenied => "filesystem.transaction.preapply_denied",
            Self::PolicyDenied => "filesystem.transaction.policy_denied",
            Self::ObservationFailed => "filesystem.transaction.observation_failed",
            Self::DriverReportInvalid => "filesystem.transaction.driver_report_invalid",
            Self::ReceiptIntegrity => "filesystem.transaction.receipt_integrity",
        }
    }
}

impl From<GrantConsumeError> for FilesystemTransactionError {
    fn from(_: GrantConsumeError) -> Self {
        Self::PolicyDenied
    }
}

/// One freshly held regular-file observation at a source or destination path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservedFilesystemEntry {
    /// Exact held regular-file target.
    pub target: GrantTarget,
    /// Complete bytes read through that exact target.
    pub bytes: Vec<u8>,
    /// Exact observed POSIX-compatible permission bits.
    pub mode: u32,
}

/// Fresh source, destination-parent, sibling, and destination state for one operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilesystemOperationObservation {
    /// Current source entry; absent when a move or trash operation removed it.
    pub source: Option<ObservedFilesystemEntry>,
    /// Exact current destination parent for destination-bearing operations.
    pub destination_parent: Option<GrantTarget>,
    /// Complete bounded sibling names currently observed in the destination parent.
    pub destination_sibling_names: Vec<String>,
    /// Current destination entry, absent before every permitted operation.
    pub destination: Option<ObservedFilesystemEntry>,
}

/// Exact per-operation lifecycle state retained in receipt order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FilesystemOperationStatus {
    /// Exact filesystem operation exists without mutation authority.
    Proposed,
    /// Exact preview and single-use grant were approved.
    Approved,
    /// The driver reports that the approved effect occurred.
    Applied,
    /// Fresh observation verified the exact approved post-state.
    Verified,
    /// The operation or transaction failed.
    Failed,
    /// A fresh restoration returned the exact approved pre-state.
    RolledBack,
    /// An earlier failure prevented this approved operation from running.
    Superseded,
}

impl FilesystemOperationStatus {
    /// Every lifecycle state in stable order.
    pub const ALL: [Self; 7] = [
        Self::Proposed,
        Self::Approved,
        Self::Applied,
        Self::Verified,
        Self::Failed,
        Self::RolledBack,
        Self::Superseded,
    ];

    fn allows(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Proposed, Self::Approved | Self::Superseded)
                | (
                    Self::Approved,
                    Self::Applied | Self::Failed | Self::Superseded
                )
                | (Self::Applied, Self::Verified | Self::Failed)
                | (Self::Failed, Self::RolledBack)
        )
    }
}

/// One hash-chained, operation-specific filesystem lifecycle receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FilesystemOperationReceipt {
    /// Stable transaction identity.
    pub transaction_id: String,
    /// Monotonic receipt sequence within the transaction.
    pub sequence: u32,
    /// Stable filesystem operation identity.
    pub operation_id: String,
    /// Closed filesystem operation kind.
    pub kind: FilesystemOperationKind,
    /// Exact lifecycle state.
    pub status: FilesystemOperationStatus,
    /// Canonical source path when applicable.
    pub source_path: Option<String>,
    /// Canonical destination path when applicable.
    pub destination_path: Option<String>,
    /// Exact source digest, or zeroes for creation.
    pub source_sha256: String,
    /// Exact expected postimage digest.
    pub postimage_sha256: String,
    /// Exact source mode when applicable.
    pub source_mode: Option<u32>,
    /// Exact destination mode when applicable.
    pub destination_mode: Option<u32>,
    /// Exact consumed or pending single-use grant identity.
    pub grant_id: GrantId,
    /// Kernel-clock event time.
    pub occurred_at_epoch_ms: u64,
    /// Stable content-free failure code, absent outside failure transitions.
    pub failure_code: Option<String>,
    /// Previous receipt digest or zeroes for the first receipt.
    pub previous_receipt_sha256: String,
    /// Canonical digest of this receipt with this field zeroed.
    pub receipt_sha256: String,
}

/// Terminal disposition of one controlled-filesystem transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FilesystemTransactionOutcome {
    /// Every exact post-state was applied and freshly verified.
    Committed,
    /// The driver failed before any operation changed state.
    FailedNoChange,
    /// Known changes were restored to exact approved pre-state.
    Restored,
    /// State could not be reconciled without risking later user work.
    Uncertain,
}

/// Descriptive post-effect check that still requires separate command authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FilesystemVerificationRequirement {
    /// Exact label shown before filesystem approval.
    pub verification_id: String,
    /// This transaction never executes the check directly.
    pub executed: bool,
    /// A separate command grant is mandatory.
    pub requires_separate_command_grant: bool,
}

/// Reconciled terminal result with exact receipts and no raw file content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilesystemTransactionResult {
    /// Stable transaction identity.
    pub transaction_id: String,
    /// Terminal disposition.
    pub outcome: FilesystemTransactionOutcome,
    /// Kernel proof that the exact operation grant was consumed once.
    pub grant_consumption: GrantConsumptionRecord,
    /// Complete hash-chained per-operation receipt history.
    pub receipts: Vec<FilesystemOperationReceipt>,
    /// Descriptive checks that remain separately permissioned and unexecuted.
    pub verification_requirements: Vec<FilesystemVerificationRequirement>,
}

/// Exact transaction request supplied by the kernel-owned caller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilesystemTransactionRequest {
    /// Stable transaction identity.
    pub transaction_id: String,
    /// Kernel-clock time for final observation and grant consumption.
    pub now_epoch_ms: u64,
    /// Explicit cancellation observed before grant consumption.
    pub cancelled_before_consume: bool,
}

/// Stable platform-driver failure class retained without paths or content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilesystemDriverError {
    /// Fresh source or destination observation was unavailable.
    ObservationUnavailable,
    /// Staging or application failed with a known unchanged or partial state.
    ApplyFailed,
    /// Restoration failed with a known or unknown state.
    RestoreFailed,
    /// The platform cannot determine whether an effect occurred.
    Uncertain,
}

impl FilesystemDriverError {
    /// Returns a stable content-free code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ObservationUnavailable => "filesystem.driver.observation_unavailable",
            Self::ApplyFailed => "filesystem.driver.apply_failed",
            Self::RestoreFailed => "filesystem.driver.restore_failed",
            Self::Uncertain => "filesystem.driver.uncertain",
        }
    }
}

/// Driver report for one ordered filesystem application attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilesystemApplyReport {
    /// Whether the platform claims all-or-nothing application for this attempt.
    pub atomic: bool,
    /// Ordered indexes known to have received their complete approved post-state.
    pub applied_indexes: Vec<u32>,
    /// First operation index that failed, absent for complete success or uncertainty.
    pub failure_index: Option<u32>,
    /// Stable content-free failure code.
    pub failure_code: Option<String>,
    /// Whether the driver cannot determine exact resulting state.
    pub uncertain: bool,
}

/// Driver report for one bounded restoration attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilesystemRestoreReport {
    /// Ordered indexes reported restored to exact approved pre-state.
    pub restored_indexes: Vec<u32>,
    /// Stable content-free failure code.
    pub failure_code: Option<String>,
    /// Whether restoration left an indeterminate state.
    pub uncertain: bool,
}

/// Opaque one-use authorization passed only after exact grant consumption.
pub struct FilesystemApplyAuthorization<'transaction> {
    transaction_id: &'transaction str,
    consumed_grant_sha256: &'transaction str,
    plan: &'transaction FilesystemPlan,
}

impl FilesystemApplyAuthorization<'_> {
    /// Returns the stable transaction identity.
    #[must_use]
    pub const fn transaction_id(&self) -> &str {
        self.transaction_id
    }

    /// Returns the digest of the exact consumed grant revision.
    #[must_use]
    pub const fn consumed_grant_sha256(&self) -> &str {
        self.consumed_grant_sha256
    }

    /// Returns the exact approved filesystem plan.
    #[must_use]
    pub const fn plan(&self) -> &FilesystemPlan {
        self.plan
    }
}

/// Opaque restoration authorization limited to indexes known to have changed.
pub struct FilesystemRestoreAuthorization<'transaction> {
    transaction_id: &'transaction str,
    plan: &'transaction FilesystemPlan,
    restore_indexes: &'transaction [u32],
}

impl FilesystemRestoreAuthorization<'_> {
    /// Returns the stable transaction identity.
    #[must_use]
    pub const fn transaction_id(&self) -> &str {
        self.transaction_id
    }

    /// Returns the exact approved filesystem plan containing retained pre-state.
    #[must_use]
    pub const fn plan(&self) -> &FilesystemPlan {
        self.plan
    }

    /// Returns only operation indexes known to require restoration.
    #[must_use]
    pub const fn restore_indexes(&self) -> &[u32] {
        self.restore_indexes
    }
}

/// Platform effect boundary for exact controlled-filesystem transactions.
pub trait ControlledFilesystemDriver {
    /// Freshly observes every exact source and destination without changing it.
    fn observe(
        &mut self,
        plan: &FilesystemPlan,
    ) -> Result<Vec<FilesystemOperationObservation>, FilesystemDriverError>;

    /// Applies exact approved operations using consumed-grant authorization.
    fn apply(&mut self, authorization: FilesystemApplyAuthorization<'_>) -> FilesystemApplyReport;

    /// Restores exact approved pre-state using a fresh bounded authorization.
    fn restore(
        &mut self,
        authorization: FilesystemRestoreAuthorization<'_>,
    ) -> FilesystemRestoreReport;
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

/// Observes, consumes, applies, verifies, and restores one exact filesystem transaction.
pub fn execute_filesystem_transaction<D: ControlledFilesystemDriver>(
    issuer: &mut GrantIssuer,
    policy: &PolicyEngine,
    plan: &FilesystemPlan,
    approval: &FilesystemApprovalReceipt,
    request: FilesystemTransactionRequest,
    driver: &mut D,
) -> Result<FilesystemTransactionResult, FilesystemTransactionError> {
    validate_transaction_identifier(&request.transaction_id)?;
    validate_transaction_binding(issuer, plan, approval)?;
    if request.cancelled_before_consume {
        invalidate_filesystem_grant(issuer, &approval.grant.grant_id, request.now_epoch_ms)?;
        return Err(FilesystemTransactionError::PreapplyDenied);
    }
    let current = driver
        .observe(plan)
        .map_err(|_| FilesystemTransactionError::ObservationFailed)?;
    if !observations_match_prestate(plan, &current) {
        invalidate_filesystem_grant(issuer, &approval.grant.grant_id, request.now_epoch_ms)?;
        return Err(FilesystemTransactionError::PreapplyDenied);
    }
    let context = filesystem_policy_context(&approval.grant, request.now_epoch_ms)?;
    let consumption = issuer.consume_for_execution(&approval.grant.grant_id, policy, &context)?;
    let mut ledger = FilesystemReceiptLedger::new(
        &request.transaction_id,
        plan,
        &approval.grant.grant_id,
        request.now_epoch_ms,
    )?;
    let report = driver.apply(FilesystemApplyAuthorization {
        transaction_id: &request.transaction_id,
        consumed_grant_sha256: &consumption.consumed_grant_sha256,
        plan,
    });
    let disposition = classify_filesystem_report(plan.operations.len(), &report).unwrap_or(
        FilesystemApplyDisposition::Uncertain {
            failure_code: FilesystemTransactionError::DriverReportInvalid
                .code()
                .to_owned(),
        },
    );
    let outcome = match disposition {
        FilesystemApplyDisposition::Success => {
            for index in 0..plan.operations.len() {
                ledger.transition(
                    index,
                    FilesystemOperationStatus::Applied,
                    None,
                    request.now_epoch_ms,
                )?;
            }
            match driver.observe(plan) {
                Ok(poststate) if observations_match_poststate(plan, &poststate) => {
                    for index in 0..plan.operations.len() {
                        ledger.transition(
                            index,
                            FilesystemOperationStatus::Verified,
                            None,
                            request.now_epoch_ms,
                        )?;
                    }
                    FilesystemTransactionOutcome::Committed
                }
                _ => restore_filesystem_after_failure(
                    driver,
                    plan,
                    &request,
                    &mut ledger,
                    filesystem_indexes(plan.operations.len())?,
                    "filesystem.verify.poststate_mismatch",
                )?,
            }
        }
        FilesystemApplyDisposition::KnownFailure {
            applied_indexes,
            failure_index,
            failure_code,
        } => {
            for index in &applied_indexes {
                let index = usize::try_from(*index)
                    .map_err(|_| FilesystemTransactionError::DriverReportInvalid)?;
                ledger.transition(
                    index,
                    FilesystemOperationStatus::Applied,
                    None,
                    request.now_epoch_ms,
                )?;
                ledger.transition(
                    index,
                    FilesystemOperationStatus::Failed,
                    Some(&failure_code),
                    request.now_epoch_ms,
                )?;
            }
            let failure_index = usize::try_from(failure_index)
                .map_err(|_| FilesystemTransactionError::DriverReportInvalid)?;
            ledger.transition(
                failure_index,
                FilesystemOperationStatus::Failed,
                Some(&failure_code),
                request.now_epoch_ms,
            )?;
            for index in (failure_index + 1)..plan.operations.len() {
                ledger.transition(
                    index,
                    FilesystemOperationStatus::Superseded,
                    None,
                    request.now_epoch_ms,
                )?;
            }
            if applied_indexes.is_empty() {
                FilesystemTransactionOutcome::FailedNoChange
            } else {
                restore_filesystem_after_failure(
                    driver,
                    plan,
                    &request,
                    &mut ledger,
                    applied_indexes,
                    &failure_code,
                )?
            }
        }
        FilesystemApplyDisposition::Uncertain { failure_code } => {
            ledger.fail_every_nonterminal(&failure_code, request.now_epoch_ms)?;
            FilesystemTransactionOutcome::Uncertain
        }
    };

    if outcome == FilesystemTransactionOutcome::Uncertain {
        issuer
            .mark_execution_uncertain(
                &approval.grant.grant_id,
                &consumption.consumed_grant_sha256,
                request.now_epoch_ms,
            )
            .map_err(|_| FilesystemTransactionError::ReceiptIntegrity)?;
    }
    let result = FilesystemTransactionResult {
        transaction_id: request.transaction_id.clone(),
        outcome,
        grant_consumption: consumption,
        receipts: ledger.receipts,
        verification_requirements: plan
            .permitted_verification
            .iter()
            .map(|verification_id| FilesystemVerificationRequirement {
                verification_id: verification_id.clone(),
                executed: false,
                requires_separate_command_grant: true,
            })
            .collect(),
    };
    verify_filesystem_receipts(&result.receipts)?;
    Ok(result)
}

/// Recomputes one complete filesystem receipt chain from retained fields.
pub fn verify_filesystem_receipts(
    receipts: &[FilesystemOperationReceipt],
) -> Result<(), FilesystemTransactionError> {
    if receipts.is_empty() {
        return Err(FilesystemTransactionError::ReceiptIntegrity);
    }
    let transaction_id = receipts[0].transaction_id.as_str();
    let grant_id = &receipts[0].grant_id;
    let mut previous = ZERO_SHA256.to_owned();
    let mut states = BTreeMap::<String, FilesystemOperationStatus>::new();
    let mut identities = BTreeMap::<
        String,
        (
            FilesystemOperationKind,
            Option<String>,
            Option<String>,
            String,
            String,
            Option<u32>,
            Option<u32>,
        ),
    >::new();
    for (index, receipt) in receipts.iter().enumerate() {
        if receipt.transaction_id != transaction_id
            || &receipt.grant_id != grant_id
            || receipt.sequence != u32::try_from(index + 1).unwrap_or(u32::MAX)
            || receipt.previous_receipt_sha256 != previous
            || !valid_sha256(&receipt.source_sha256)
            || !valid_sha256(&receipt.postimage_sha256)
        {
            return Err(FilesystemTransactionError::ReceiptIntegrity);
        }
        let mut candidate = receipt.clone();
        candidate.receipt_sha256 = ZERO_SHA256.to_owned();
        if transaction_sha256(&candidate)? != receipt.receipt_sha256 {
            return Err(FilesystemTransactionError::ReceiptIntegrity);
        }
        let identity = (
            receipt.kind,
            receipt.source_path.clone(),
            receipt.destination_path.clone(),
            receipt.source_sha256.clone(),
            receipt.postimage_sha256.clone(),
            receipt.source_mode,
            receipt.destination_mode,
        );
        if identities
            .get(&receipt.operation_id)
            .is_some_and(|existing| existing != &identity)
        {
            return Err(FilesystemTransactionError::ReceiptIntegrity);
        }
        identities
            .entry(receipt.operation_id.clone())
            .or_insert(identity);
        match states.get(&receipt.operation_id).copied() {
            None if receipt.status == FilesystemOperationStatus::Proposed => {}
            Some(current) if current.allows(receipt.status) => {}
            _ => return Err(FilesystemTransactionError::ReceiptIntegrity),
        }
        if matches!(receipt.status, FilesystemOperationStatus::Failed)
            != receipt.failure_code.is_some()
        {
            return Err(FilesystemTransactionError::ReceiptIntegrity);
        }
        previous = receipt.receipt_sha256.clone();
        states.insert(receipt.operation_id.clone(), receipt.status);
    }
    if states.values().any(|status| {
        !matches!(
            status,
            FilesystemOperationStatus::Verified
                | FilesystemOperationStatus::Failed
                | FilesystemOperationStatus::RolledBack
                | FilesystemOperationStatus::Superseded
        )
    }) {
        return Err(FilesystemTransactionError::ReceiptIntegrity);
    }
    Ok(())
}

fn validate_transaction_binding(
    issuer: &GrantIssuer,
    plan: &FilesystemPlan,
    approval: &FilesystemApprovalReceipt,
) -> Result<(), FilesystemTransactionError> {
    verify_plan(plan).map_err(|_| FilesystemTransactionError::InvalidInput)?;
    let expected_operation = OperationBinding::new(if plan.requires_high_risk_delete_grant {
        GrantOperation::WorkspaceDelete
    } else {
        GrantOperation::WorkspaceWrite
    });
    let current = issuer
        .current(&approval.grant.grant_id)
        .ok_or(FilesystemTransactionError::InvalidInput)?;
    let valid = current == &approval.grant
        && current.status == GrantStatus::Issued
        && current.operation == expected_operation
        && current.argument_sha256 == plan.plan_sha256
        && current.preview_sha256 == approval.preview_sha256
        && approval.plan_sha256 == plan.plan_sha256
        && approval.permitted_verification == plan.permitted_verification
        && current.targets == filesystem_grant_targets(plan);
    if !valid {
        return Err(FilesystemTransactionError::InvalidInput);
    }
    Ok(())
}

fn filesystem_grant_targets(plan: &FilesystemPlan) -> Vec<GrantTarget> {
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
    targets
}

fn invalidate_filesystem_grant(
    issuer: &mut GrantIssuer,
    grant_id: &GrantId,
    now_epoch_ms: u64,
) -> Result<(), FilesystemTransactionError> {
    let current = issuer
        .current(grant_id)
        .ok_or(FilesystemTransactionError::InvalidInput)?;
    let digest = issuer
        .revision_hash(grant_id, current.revision)
        .ok_or(FilesystemTransactionError::ReceiptIntegrity)?
        .to_owned();
    issuer
        .cancel_before_execution(grant_id, &digest, now_epoch_ms)
        .map_err(|_| FilesystemTransactionError::ReceiptIntegrity)?;
    Ok(())
}

fn filesystem_policy_context(
    grant: &CapabilityGrant,
    now_epoch_ms: u64,
) -> Result<PolicyEvaluationContext, FilesystemTransactionError> {
    if grant.status != GrantStatus::Issued
        || !matches!(
            grant.operation.operation(),
            GrantOperation::WorkspaceWrite | GrantOperation::WorkspaceDelete
        )
    {
        return Err(FilesystemTransactionError::InvalidInput);
    }
    Ok(PolicyEvaluationContext {
        actor_id: grant.actor_id.clone(),
        session_id: grant.session_id.clone(),
        task_id: grant.task_id.clone(),
        action_id: grant
            .action_id
            .clone()
            .ok_or(FilesystemTransactionError::InvalidInput)?,
        action_kind: grant
            .action_kind
            .ok_or(FilesystemTransactionError::InvalidInput)?,
        tool_id: grant
            .tool_id
            .clone()
            .ok_or(FilesystemTransactionError::InvalidInput)?,
        tool_version: grant
            .tool_version
            .clone()
            .ok_or(FilesystemTransactionError::InvalidInput)?,
        targets: grant.targets.clone(),
        argument_sha256: grant.argument_sha256.clone(),
        preimages: grant.preimages.clone(),
        expected_side_effects: grant.expected_side_effects.clone(),
        preview_sha256: grant.preview_sha256.clone(),
        now_epoch_ms,
        network_scope: None,
        credential_scope: None,
        publication_scope: None,
    })
}

fn observations_match_prestate(
    plan: &FilesystemPlan,
    observations: &[FilesystemOperationObservation],
) -> bool {
    observations.len() == plan.operations.len()
        && observations
            .iter()
            .zip(&plan.operations)
            .all(|(observation, operation)| {
                let source_matches = match (&operation.source, &observation.source) {
                    (None, None) => true,
                    (Some(expected), Some(current)) => {
                        current.target == *expected
                            && current.bytes == operation.source_bytes
                            && current.mode == operation.source_mode.unwrap_or(u32::MAX)
                            && observed_entry_is_exact(current)
                    }
                    _ => false,
                };
                let destination_matches = match &operation.destination_parent {
                    None => {
                        observation.destination_parent.is_none()
                            && observation.destination_sibling_names.is_empty()
                            && observation.destination.is_none()
                    }
                    Some(parent) => {
                        observation.destination_parent.as_ref() == Some(parent)
                            && observation.destination.is_none()
                            && destination_is_collision_free(
                                operation,
                                &observation.destination_sibling_names,
                            )
                    }
                };
                source_matches && destination_matches
            })
}

fn observations_match_poststate(
    plan: &FilesystemPlan,
    observations: &[FilesystemOperationObservation],
) -> bool {
    observations.len() == plan.operations.len()
        && observations
            .iter()
            .zip(&plan.operations)
            .all(|(observation, operation)| match operation.kind {
                FilesystemOperationKind::Create => {
                    observation.source.is_none()
                        && destination_matches_postimage(operation, observation)
                }
                FilesystemOperationKind::ExactPatch => {
                    observation.destination_parent.is_none()
                        && observation.destination_sibling_names.is_empty()
                        && observation.destination.is_none()
                        && observation.source.as_ref().is_some_and(|source| {
                            entry_matches_postimage(
                                source,
                                operation.source_path.as_deref(),
                                operation.postimage_sha256.as_deref(),
                                operation.destination_mode,
                                &operation.postimage_bytes,
                            )
                        })
                }
                FilesystemOperationKind::Copy => {
                    source_matches_approved(operation, observation.source.as_ref())
                        && destination_matches_postimage(operation, observation)
                }
                FilesystemOperationKind::Move | FilesystemOperationKind::TrashDelete => {
                    observation.source.is_none()
                        && destination_matches_postimage(operation, observation)
                }
            })
}

fn source_matches_approved(
    operation: &FilesystemOperation,
    source: Option<&ObservedFilesystemEntry>,
) -> bool {
    source.is_some_and(|source| {
        operation.source.as_ref() == Some(&source.target)
            && source.bytes == operation.source_bytes
            && source.mode == operation.source_mode.unwrap_or(u32::MAX)
            && observed_entry_is_exact(source)
    })
}

fn destination_matches_postimage(
    operation: &FilesystemOperation,
    observation: &FilesystemOperationObservation,
) -> bool {
    observation.destination_parent.as_ref() == operation.destination_parent.as_ref()
        && observation.destination.as_ref().is_some_and(|destination| {
            entry_matches_postimage(
                destination,
                operation.destination_path.as_deref(),
                operation.postimage_sha256.as_deref(),
                operation.destination_mode,
                &operation.postimage_bytes,
            )
        })
}

fn entry_matches_postimage(
    entry: &ObservedFilesystemEntry,
    expected_path: Option<&str>,
    expected_sha256: Option<&str>,
    expected_mode: Option<u32>,
    expected_bytes: &[u8],
) -> bool {
    Some(display_target_path(&entry.target).as_str()) == expected_path
        && Some(hex_sha256(&entry.bytes).as_str()) == expected_sha256
        && Some(entry.mode) == expected_mode
        && entry.bytes == expected_bytes
        && observed_entry_is_exact(entry)
}

fn observed_entry_is_exact(entry: &ObservedFilesystemEntry) -> bool {
    entry.target.object_kind() == Some(WorkspaceObjectKind::RegularFile)
        && entry.target.preimage().is_some_and(|preimage| {
            preimage.byte_len() == u64::try_from(entry.bytes.len()).unwrap_or(u64::MAX)
                && hex_bytes(preimage.content_sha256()) == hex_sha256(&entry.bytes)
        })
}

fn destination_is_collision_free(
    operation: &FilesystemOperation,
    sibling_names: &[String],
) -> bool {
    if sibling_names.len() > MAX_SIBLING_NAMES {
        return false;
    }
    let Some(candidate) = operation
        .destination_path
        .as_deref()
        .and_then(|path| path.rsplit('/').next())
    else {
        return false;
    };
    let candidate_folded = candidate.to_lowercase();
    let mut unique = BTreeSet::new();
    sibling_names.iter().all(|sibling| {
        !sibling.is_empty()
            && sibling.len() <= agentmage_kernel_contracts::MAX_WORKSPACE_PATH_COMPONENT_BYTES
            && !sibling.contains('/')
            && !sibling.contains('\\')
            && unique.insert(sibling)
            && sibling.to_lowercase() != candidate_folded
    })
}

enum FilesystemApplyDisposition {
    Success,
    KnownFailure {
        applied_indexes: Vec<u32>,
        failure_index: u32,
        failure_code: String,
    },
    Uncertain {
        failure_code: String,
    },
}

fn classify_filesystem_report(
    operation_count: usize,
    report: &FilesystemApplyReport,
) -> Result<FilesystemApplyDisposition, FilesystemTransactionError> {
    if report.uncertain {
        let code = report
            .failure_code
            .as_deref()
            .unwrap_or(FilesystemDriverError::Uncertain.code());
        validate_transaction_failure_code(code)?;
        return Ok(FilesystemApplyDisposition::Uncertain {
            failure_code: code.to_owned(),
        });
    }
    let complete = filesystem_indexes(operation_count)?;
    if report.failure_index.is_none()
        && report.failure_code.is_none()
        && report.applied_indexes == complete
    {
        return Ok(FilesystemApplyDisposition::Success);
    }
    let (Some(failure_index), Some(failure_code)) =
        (report.failure_index, report.failure_code.as_deref())
    else {
        return Err(FilesystemTransactionError::DriverReportInvalid);
    };
    validate_transaction_failure_code(failure_code)?;
    let failure = usize::try_from(failure_index)
        .ok()
        .filter(|index| *index < operation_count)
        .ok_or(FilesystemTransactionError::DriverReportInvalid)?;
    let expected_prefix = filesystem_indexes(failure)?;
    if report.applied_indexes != expected_prefix || (report.atomic && !expected_prefix.is_empty()) {
        return Err(FilesystemTransactionError::DriverReportInvalid);
    }
    Ok(FilesystemApplyDisposition::KnownFailure {
        applied_indexes: report.applied_indexes.clone(),
        failure_index,
        failure_code: failure_code.to_owned(),
    })
}

fn restore_filesystem_after_failure<D: ControlledFilesystemDriver>(
    driver: &mut D,
    plan: &FilesystemPlan,
    request: &FilesystemTransactionRequest,
    ledger: &mut FilesystemReceiptLedger<'_>,
    indexes: Vec<u32>,
    failure_code: &str,
) -> Result<FilesystemTransactionOutcome, FilesystemTransactionError> {
    for index in &indexes {
        let index =
            usize::try_from(*index).map_err(|_| FilesystemTransactionError::DriverReportInvalid)?;
        if ledger.current(index) == Some(FilesystemOperationStatus::Applied) {
            ledger.transition(
                index,
                FilesystemOperationStatus::Failed,
                Some(failure_code),
                request.now_epoch_ms,
            )?;
        }
    }
    let report = driver.restore(FilesystemRestoreAuthorization {
        transaction_id: &request.transaction_id,
        plan,
        restore_indexes: &indexes,
    });
    if report.uncertain
        || report.failure_code.is_some()
        || report.restored_indexes != indexes
        || driver
            .observe(plan)
            .map_or(true, |values| !observations_match_prestate(plan, &values))
    {
        return Ok(FilesystemTransactionOutcome::Uncertain);
    }
    for index in indexes {
        ledger.transition(
            usize::try_from(index).map_err(|_| FilesystemTransactionError::DriverReportInvalid)?,
            FilesystemOperationStatus::RolledBack,
            None,
            request.now_epoch_ms,
        )?;
    }
    Ok(FilesystemTransactionOutcome::Restored)
}

fn filesystem_indexes(count: usize) -> Result<Vec<u32>, FilesystemTransactionError> {
    (0..count)
        .map(|index| u32::try_from(index).map_err(|_| FilesystemTransactionError::InvalidInput))
        .collect()
}

fn validate_transaction_identifier(value: &str) -> Result<(), FilesystemTransactionError> {
    if value.is_empty()
        || value.len() > MAX_IDENTIFIER_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
    {
        return Err(FilesystemTransactionError::InvalidInput);
    }
    Ok(())
}

fn validate_transaction_failure_code(value: &str) -> Result<(), FilesystemTransactionError> {
    if value.is_empty()
        || value.len() > MAX_IDENTIFIER_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
    {
        return Err(FilesystemTransactionError::DriverReportInvalid);
    }
    Ok(())
}

fn transaction_sha256(value: &impl Serialize) -> Result<String, FilesystemTransactionError> {
    serde_json::to_vec(value)
        .map(|bytes| hex_sha256(&bytes))
        .map_err(|_| FilesystemTransactionError::ReceiptIntegrity)
}

struct FilesystemReceiptLedger<'operation> {
    transaction_id: &'operation str,
    plan: &'operation FilesystemPlan,
    grant_id: &'operation GrantId,
    current: BTreeMap<usize, FilesystemOperationStatus>,
    receipts: Vec<FilesystemOperationReceipt>,
}

impl<'operation> FilesystemReceiptLedger<'operation> {
    fn new(
        transaction_id: &'operation str,
        plan: &'operation FilesystemPlan,
        grant_id: &'operation GrantId,
        occurred_at_epoch_ms: u64,
    ) -> Result<Self, FilesystemTransactionError> {
        let mut ledger = Self {
            transaction_id,
            plan,
            grant_id,
            current: BTreeMap::new(),
            receipts: Vec::new(),
        };
        for index in 0..plan.operations.len() {
            ledger.emit(
                index,
                FilesystemOperationStatus::Proposed,
                None,
                occurred_at_epoch_ms,
            )?;
            ledger.transition(
                index,
                FilesystemOperationStatus::Approved,
                None,
                occurred_at_epoch_ms,
            )?;
        }
        Ok(ledger)
    }

    fn current(&self, index: usize) -> Option<FilesystemOperationStatus> {
        self.current.get(&index).copied()
    }

    fn transition(
        &mut self,
        index: usize,
        status: FilesystemOperationStatus,
        failure_code: Option<&str>,
        occurred_at_epoch_ms: u64,
    ) -> Result<(), FilesystemTransactionError> {
        let Some(current) = self.current(index) else {
            return Err(FilesystemTransactionError::ReceiptIntegrity);
        };
        if !current.allows(status) {
            return Err(FilesystemTransactionError::ReceiptIntegrity);
        }
        self.emit(index, status, failure_code, occurred_at_epoch_ms)
    }

    fn fail_every_nonterminal(
        &mut self,
        failure_code: &str,
        occurred_at_epoch_ms: u64,
    ) -> Result<(), FilesystemTransactionError> {
        for index in 0..self.plan.operations.len() {
            if matches!(
                self.current(index),
                Some(FilesystemOperationStatus::Approved | FilesystemOperationStatus::Applied)
            ) {
                self.emit(
                    index,
                    FilesystemOperationStatus::Failed,
                    Some(failure_code),
                    occurred_at_epoch_ms,
                )?;
            }
        }
        Ok(())
    }

    fn emit(
        &mut self,
        index: usize,
        status: FilesystemOperationStatus,
        failure_code: Option<&str>,
        occurred_at_epoch_ms: u64,
    ) -> Result<(), FilesystemTransactionError> {
        if let Some(code) = failure_code {
            validate_transaction_failure_code(code)?;
        }
        if matches!(status, FilesystemOperationStatus::Failed) != failure_code.is_some() {
            return Err(FilesystemTransactionError::ReceiptIntegrity);
        }
        let operation = self
            .plan
            .operations
            .get(index)
            .ok_or(FilesystemTransactionError::ReceiptIntegrity)?;
        let previous = self
            .receipts
            .last()
            .map_or(ZERO_SHA256, |receipt| receipt.receipt_sha256.as_str())
            .to_owned();
        let mut receipt = FilesystemOperationReceipt {
            transaction_id: self.transaction_id.to_owned(),
            sequence: u32::try_from(self.receipts.len() + 1)
                .map_err(|_| FilesystemTransactionError::ReceiptIntegrity)?,
            operation_id: operation.operation_id.clone(),
            kind: operation.kind,
            status,
            source_path: operation.source_path.clone(),
            destination_path: operation.destination_path.clone(),
            source_sha256: operation
                .source_sha256
                .clone()
                .unwrap_or_else(|| ZERO_SHA256.to_owned()),
            postimage_sha256: operation
                .postimage_sha256
                .clone()
                .ok_or(FilesystemTransactionError::ReceiptIntegrity)?,
            source_mode: operation.source_mode,
            destination_mode: operation.destination_mode,
            grant_id: self.grant_id.clone(),
            occurred_at_epoch_ms,
            failure_code: failure_code.map(str::to_owned),
            previous_receipt_sha256: previous,
            receipt_sha256: ZERO_SHA256.to_owned(),
        };
        receipt.receipt_sha256 = transaction_sha256(&receipt)?;
        self.current.insert(index, status);
        self.receipts.push(receipt);
        Ok(())
    }
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
            if content.len() > MAX_FILE_BYTES || std::str::from_utf8(&content).is_err() {
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
    use std::collections::BTreeSet;

    use agentmage_kernel_contracts::{
        ActionId, ActionKind, ActorId, AdapterInstanceId, ApprovalId, AuthorizedWorkspaceHandle,
        DataSensitivity, GrantId, GrantNonce, GrantOperation, GrantStatus, GrantTarget,
        OperationBinding, PathPlatform, SessionId, TaskId, ToolId, WorkspaceAuthorizationId,
        WorkspaceId, WorkspacePath, WorkspaceScopePath,
    };
    use serde_json::json;
    use sha2::{Digest, Sha256};

    use super::{
        ControlledFilesystemDriver, ExistingSourceDraft, ExistingWorkDisposition,
        FileClassification, FilesystemApplyAuthorization, FilesystemApplyReport,
        FilesystemApprovalDecision, FilesystemApprovalPreview, FilesystemDriverError,
        FilesystemGrantRequest, FilesystemOperationDraft, FilesystemOperationKind,
        FilesystemOperationObservation, FilesystemOperationStatus, FilesystemPlan,
        FilesystemPlanError, FilesystemPlanRequest, FilesystemRestoreAuthorization,
        FilesystemRestoreReport, FilesystemTransactionError, FilesystemTransactionOutcome,
        FilesystemTransactionRequest, NewDestinationDraft, ObservedFilesystemEntry,
        StructuredPatch, StructuredPatchHunk, apply_structured_patch, build_filesystem_plan,
        execute_filesystem_transaction, filesystem_indexes, hex_sha256, issue_filesystem_grant,
        parse_structured_patch_json, render_filesystem_preview, verify_filesystem_receipts,
    };
    use crate::grants::{GrantIssuer, SessionReadGrantRequest};
    use crate::policy::{PolicyDocument, PolicyEngine, ScopeRules, ToolPolicyBinding};
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
        parent_with_policy("b".repeat(64))
    }

    fn parent_with_policy(
        policy_sha256: String,
    ) -> (GrantIssuer, agentmage_kernel_contracts::CapabilityGrant) {
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
                policy_sha256,
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

    fn rules<T: Ord>(values: impl IntoIterator<Item = T>) -> ScopeRules<T> {
        ScopeRules {
            allowed: values.into_iter().collect(),
            denied: BTreeSet::new(),
        }
    }

    struct TransactionFixture {
        issuer: GrantIssuer,
        policy: PolicyEngine,
        plan: FilesystemPlan,
        approval: super::FilesystemApprovalReceipt,
    }

    fn transaction_fixture(delete: bool) -> TransactionFixture {
        let drafts = if delete {
            vec![FilesystemOperationDraft::TrashDelete {
                operation_id: "operation-trash".to_owned(),
                source: source(
                    &["src", "obsolete.txt"],
                    b"obsolete\n",
                    ExistingWorkDisposition::Clean,
                ),
                trash_destination: destination(&["trash"], "obsolete.txt", &[]),
            }]
        } else {
            vec![
                FilesystemOperationDraft::Create {
                    operation_id: "operation-create".to_owned(),
                    destination: destination(&["new"], "created.txt", &[]),
                    content: b"created\n".to_vec(),
                    mode: 0o600,
                    classification: FileClassification::Documentation,
                },
                FilesystemOperationDraft::ExactPatch {
                    operation_id: "operation-patch".to_owned(),
                    source: source(
                        &["src", "patch.txt"],
                        b"alpha\nbeta\ngamma\n",
                        ExistingWorkDisposition::Clean,
                    ),
                    patch_json: patch("beta\n", "changed\n"),
                    expected_postimage_sha256: hex_sha256(b"alpha\nchanged\ngamma\n"),
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
                FilesystemOperationDraft::Move {
                    operation_id: "operation-move".to_owned(),
                    source: source(
                        &["src", "move.txt"],
                        b"move\n",
                        ExistingWorkDisposition::OwnedByCurrentTask,
                    ),
                    destination: destination(&["moved"], "move.txt", &[]),
                },
            ]
        };
        let policy_targets = drafts
            .iter()
            .flat_map(|draft| match draft {
                FilesystemOperationDraft::Create { destination, .. } => {
                    vec![destination.parent.clone()]
                }
                FilesystemOperationDraft::ExactPatch { source, .. } => {
                    vec![source.target.clone()]
                }
                FilesystemOperationDraft::Copy {
                    source,
                    destination,
                    ..
                }
                | FilesystemOperationDraft::Move {
                    source,
                    destination,
                    ..
                } => vec![source.target.clone(), destination.parent.clone()],
                FilesystemOperationDraft::TrashDelete {
                    source,
                    trash_destination,
                    ..
                } => vec![source.target.clone(), trash_destination.parent.clone()],
            })
            .collect::<Vec<_>>();
        let action_id = ActionId::from_raw("action-filesystem-transaction");
        let tool_id = ToolId::from_raw("workspace.filesystem");
        let operation = OperationBinding::new(if delete {
            GrantOperation::WorkspaceDelete
        } else {
            GrantOperation::WorkspaceWrite
        });
        let policy = PolicyEngine::new(PolicyDocument {
            schema_version: 1,
            revision: 1,
            actors: rules([ActorId::from_raw("actor-local")]),
            tasks: rules([TaskId::from_raw("task-filesystem")]),
            actions: rules([action_id.clone()]),
            tools: rules([ToolPolicyBinding {
                tool_id: tool_id.clone(),
                tool_version: "1.0.0".to_owned(),
            }]),
            operations: rules([operation]),
            targets: rules(policy_targets),
            denied_argument_sha256s: BTreeSet::new(),
            denied_preimage_sha256s: BTreeSet::new(),
            network_scopes: ScopeRules::deny_all(),
            credential_scopes: ScopeRules::deny_all(),
            publication_scopes: ScopeRules::deny_all(),
        })
        .expect("filesystem transaction policy");
        let (mut issuer, parent) = parent_with_policy(policy.policy_sha256().to_owned());
        let plan = build_filesystem_plan(&parent, request(drafts)).expect("transaction plan");
        let preview = render_filesystem_preview(&plan).expect("transaction preview");
        let approval = issue_filesystem_grant(
            &mut issuer,
            &plan,
            &preview,
            &approval_decision(&preview, delete),
            FilesystemGrantRequest {
                parent_grant_id: parent.grant_id,
                grant_id: GrantId::from_raw("grant-filesystem-transaction"),
                action_id,
                action_kind: ActionKind::DeterministicTool,
                tool_id,
                tool_version: "1.0.0".to_owned(),
                nonce: GrantNonce::from_raw("nonce-filesystem-transaction"),
                policy_sha256: policy.policy_sha256().to_owned(),
            },
        )
        .expect("filesystem transaction grant");
        TransactionFixture {
            issuer,
            policy,
            plan,
            approval,
        }
    }

    #[derive(Clone, Copy)]
    enum FilesystemDriverMode {
        Success,
        FailAt(usize),
        CorruptPoststate,
        Uncertain,
        MalformedAtomicPartial,
        RestoreFails,
    }

    struct MemoryFilesystemDriver {
        observations: Vec<FilesystemOperationObservation>,
        initial: Vec<FilesystemOperationObservation>,
        mode: FilesystemDriverMode,
        apply_calls: usize,
        restore_calls: usize,
    }

    impl MemoryFilesystemDriver {
        fn new(plan: &FilesystemPlan, mode: FilesystemDriverMode) -> Self {
            let observations = plan
                .operations()
                .iter()
                .map(|operation| FilesystemOperationObservation {
                    source: operation.source().map(|target| ObservedFilesystemEntry {
                        target: target.clone(),
                        bytes: operation.source_bytes().to_vec(),
                        mode: operation.source_mode().expect("source mode"),
                    }),
                    destination_parent: operation.destination_parent().cloned(),
                    destination_sibling_names: Vec::new(),
                    destination: None,
                })
                .collect::<Vec<_>>();
            Self {
                initial: observations.clone(),
                observations,
                mode,
                apply_calls: 0,
                restore_calls: 0,
            }
        }

        fn observed_entry(path: &str, bytes: &[u8], mode: u32) -> ObservedFilesystemEntry {
            let components = path.split('/').collect::<Vec<_>>();
            ObservedFilesystemEntry {
                target: source(&components, bytes, ExistingWorkDisposition::Clean).target,
                bytes: bytes.to_vec(),
                mode,
            }
        }

        fn apply_one(&mut self, plan: &FilesystemPlan, index: usize) {
            let operation = &plan.operations()[index];
            match operation.kind() {
                FilesystemOperationKind::Create => {
                    self.observations[index].destination = Some(Self::observed_entry(
                        operation.destination_path().expect("create destination"),
                        operation.postimage_bytes(),
                        operation.destination_mode().expect("create mode"),
                    ));
                }
                FilesystemOperationKind::ExactPatch => {
                    self.observations[index].source = Some(Self::observed_entry(
                        operation.source_path().expect("patch source"),
                        operation.postimage_bytes(),
                        operation.destination_mode().expect("patch mode"),
                    ));
                }
                FilesystemOperationKind::Copy => {
                    self.observations[index].destination = Some(Self::observed_entry(
                        operation.destination_path().expect("copy destination"),
                        operation.postimage_bytes(),
                        operation.destination_mode().expect("copy mode"),
                    ));
                }
                FilesystemOperationKind::Move | FilesystemOperationKind::TrashDelete => {
                    self.observations[index].source = None;
                    self.observations[index].destination = Some(Self::observed_entry(
                        operation.destination_path().expect("move destination"),
                        operation.postimage_bytes(),
                        operation.destination_mode().expect("move mode"),
                    ));
                }
            }
        }

        fn corrupt_first_changed_entry(&mut self) {
            let observation = &mut self.observations[0];
            let entry = match observation.destination.as_mut() {
                Some(destination) => destination,
                None => observation.source.as_mut().expect("changed entry"),
            };
            entry.bytes.push(b'!');
        }
    }

    impl ControlledFilesystemDriver for MemoryFilesystemDriver {
        fn observe(
            &mut self,
            _plan: &FilesystemPlan,
        ) -> Result<Vec<FilesystemOperationObservation>, FilesystemDriverError> {
            Ok(self.observations.clone())
        }

        fn apply(
            &mut self,
            authorization: FilesystemApplyAuthorization<'_>,
        ) -> FilesystemApplyReport {
            self.apply_calls += 1;
            assert_ne!(authorization.consumed_grant_sha256(), "0".repeat(64));
            let plan = authorization.plan();
            match self.mode {
                FilesystemDriverMode::Success | FilesystemDriverMode::RestoreFails => {
                    for index in 0..plan.operations().len() {
                        self.apply_one(plan, index);
                    }
                    if matches!(self.mode, FilesystemDriverMode::RestoreFails) {
                        self.corrupt_first_changed_entry();
                    }
                    FilesystemApplyReport {
                        atomic: true,
                        applied_indexes: filesystem_indexes(plan.operations().len())
                            .expect("bounded indexes"),
                        failure_index: None,
                        failure_code: None,
                        uncertain: false,
                    }
                }
                FilesystemDriverMode::FailAt(failure) => {
                    for index in 0..failure {
                        self.apply_one(plan, index);
                    }
                    FilesystemApplyReport {
                        atomic: false,
                        applied_indexes: filesystem_indexes(failure).expect("bounded prefix"),
                        failure_index: Some(u32::try_from(failure).expect("bounded failure")),
                        failure_code: Some("filesystem.driver.injected_failure".to_owned()),
                        uncertain: false,
                    }
                }
                FilesystemDriverMode::CorruptPoststate => {
                    for index in 0..plan.operations().len() {
                        self.apply_one(plan, index);
                    }
                    self.corrupt_first_changed_entry();
                    FilesystemApplyReport {
                        atomic: true,
                        applied_indexes: filesystem_indexes(plan.operations().len())
                            .expect("bounded indexes"),
                        failure_index: None,
                        failure_code: None,
                        uncertain: false,
                    }
                }
                FilesystemDriverMode::Uncertain => {
                    self.apply_one(plan, 0);
                    FilesystemApplyReport {
                        atomic: false,
                        applied_indexes: Vec::new(),
                        failure_index: None,
                        failure_code: Some(FilesystemDriverError::Uncertain.code().to_owned()),
                        uncertain: true,
                    }
                }
                FilesystemDriverMode::MalformedAtomicPartial => FilesystemApplyReport {
                    atomic: true,
                    applied_indexes: vec![0],
                    failure_index: Some(1),
                    failure_code: Some("filesystem.driver.invalid_atomic_claim".to_owned()),
                    uncertain: false,
                },
            }
        }

        fn restore(
            &mut self,
            authorization: FilesystemRestoreAuthorization<'_>,
        ) -> FilesystemRestoreReport {
            self.restore_calls += 1;
            if matches!(self.mode, FilesystemDriverMode::RestoreFails) {
                return FilesystemRestoreReport {
                    restored_indexes: Vec::new(),
                    failure_code: Some(FilesystemDriverError::RestoreFailed.code().to_owned()),
                    uncertain: true,
                };
            }
            for index in authorization.restore_indexes() {
                let index = usize::try_from(*index).expect("bounded restore index");
                self.observations[index] = self.initial[index].clone();
            }
            FilesystemRestoreReport {
                restored_indexes: authorization.restore_indexes().to_vec(),
                failure_code: None,
                uncertain: false,
            }
        }
    }

    fn execute_transaction(
        fixture: &mut TransactionFixture,
        driver: &mut MemoryFilesystemDriver,
    ) -> Result<super::FilesystemTransactionResult, FilesystemTransactionError> {
        execute_filesystem_transaction(
            &mut fixture.issuer,
            &fixture.policy,
            &fixture.plan,
            &fixture.approval,
            FilesystemTransactionRequest {
                transaction_id: "filesystem-transaction-0001".to_owned(),
                now_epoch_ms: 4_000,
                cancelled_before_consume: false,
            },
            driver,
        )
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

        let unicode_collision = FilesystemOperationDraft::Create {
            operation_id: "operation-unicode-collision".to_owned(),
            destination: destination(&["new"], "Résumé.md", &["RÉSUMÉ.MD"]),
            content: b"collision\n".to_vec(),
            mode: 0o600,
            classification: FileClassification::Documentation,
        };
        assert_eq!(
            build_filesystem_plan(&parent(), request(vec![unicode_collision])),
            Err(FilesystemPlanError::DestinationCollision)
        );
    }

    #[test]
    fn empty_file_creation_has_an_exact_zero_byte_postimage() {
        let operation = FilesystemOperationDraft::Create {
            operation_id: "operation-empty-create".to_owned(),
            destination: destination(&["new"], "empty.txt", &[]),
            content: Vec::new(),
            mode: 0o600,
            classification: FileClassification::Data,
        };
        let plan = build_filesystem_plan(&parent(), request(vec![operation])).expect("empty plan");
        let preview = render_filesystem_preview(&plan).expect("empty preview");
        assert_eq!(plan.operations()[0].postimage_bytes(), b"");
        assert_eq!(
            plan.operations()[0].postimage_sha256(),
            Some(hex_sha256(b"").as_str())
        );
        assert_eq!(
            preview.operations[0].complete_content_preview.as_deref(),
            Some("\"\"")
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

    #[test]
    fn controlled_filesystem_transaction_commits_every_operation_and_delete() {
        for delete in [false, true] {
            let mut fixture = transaction_fixture(delete);
            let mut driver =
                MemoryFilesystemDriver::new(&fixture.plan, FilesystemDriverMode::Success);
            let result = execute_transaction(&mut fixture, &mut driver).expect("committed");
            assert_eq!(result.outcome, FilesystemTransactionOutcome::Committed);
            assert_eq!(driver.apply_calls, 1);
            assert_eq!(driver.restore_calls, 0);
            assert!(
                result
                    .receipts
                    .iter()
                    .filter(|receipt| receipt.status == FilesystemOperationStatus::Verified)
                    .count()
                    == fixture.plan.operations().len()
            );
            assert_eq!(
                result.verification_requirements,
                [super::FilesystemVerificationRequirement {
                    verification_id: "cargo-test-filesystem-control".to_owned(),
                    executed: false,
                    requires_separate_command_grant: true,
                }]
            );
            assert_eq!(
                fixture
                    .issuer
                    .current(&fixture.approval.grant.grant_id)
                    .map(|grant| grant.status),
                Some(GrantStatus::Consumed)
            );
            verify_filesystem_receipts(&result.receipts).expect("receipt chain");
        }
    }

    #[test]
    fn stale_source_and_fresh_destination_collision_invalidate_before_apply() {
        let mut stale = transaction_fixture(false);
        let mut stale_driver =
            MemoryFilesystemDriver::new(&stale.plan, FilesystemDriverMode::Success);
        stale_driver.observations[1]
            .source
            .as_mut()
            .expect("patch source")
            .bytes
            .push(b'!');
        assert_eq!(
            execute_transaction(&mut stale, &mut stale_driver),
            Err(FilesystemTransactionError::PreapplyDenied)
        );
        assert_eq!(stale_driver.apply_calls, 0);
        assert_eq!(
            stale
                .issuer
                .current(&stale.approval.grant.grant_id)
                .map(|grant| grant.status),
            Some(GrantStatus::Invalidated)
        );

        let mut collision = transaction_fixture(false);
        let mut collision_driver =
            MemoryFilesystemDriver::new(&collision.plan, FilesystemDriverMode::Success);
        collision_driver.observations[0]
            .destination_sibling_names
            .push("CREATED.TXT".to_owned());
        assert_eq!(
            execute_transaction(&mut collision, &mut collision_driver),
            Err(FilesystemTransactionError::PreapplyDenied)
        );
        assert_eq!(collision_driver.apply_calls, 0);
    }

    #[test]
    fn cancellation_invalidates_before_observation_consumption_or_apply() {
        let mut fixture = transaction_fixture(false);
        let mut driver = MemoryFilesystemDriver::new(&fixture.plan, FilesystemDriverMode::Success);
        let result = execute_filesystem_transaction(
            &mut fixture.issuer,
            &fixture.policy,
            &fixture.plan,
            &fixture.approval,
            FilesystemTransactionRequest {
                transaction_id: "filesystem-transaction-cancelled".to_owned(),
                now_epoch_ms: 4_000,
                cancelled_before_consume: true,
            },
            &mut driver,
        );
        assert_eq!(result, Err(FilesystemTransactionError::PreapplyDenied));
        assert_eq!(driver.apply_calls, 0);
        assert_eq!(driver.restore_calls, 0);
        assert_eq!(driver.observations, driver.initial);
        assert_eq!(
            fixture
                .issuer
                .current(&fixture.approval.grant.grant_id)
                .map(|grant| grant.status),
            Some(GrantStatus::Invalidated)
        );
    }

    #[test]
    fn known_partial_failure_restores_exact_prestate_and_no_change_stays_failed() {
        let mut partial = transaction_fixture(false);
        let mut partial_driver =
            MemoryFilesystemDriver::new(&partial.plan, FilesystemDriverMode::FailAt(2));
        let restored = execute_transaction(&mut partial, &mut partial_driver).expect("restored");
        assert_eq!(restored.outcome, FilesystemTransactionOutcome::Restored);
        assert_eq!(partial_driver.restore_calls, 1);
        assert_eq!(partial_driver.observations, partial_driver.initial);
        assert!(restored.receipts.iter().any(|receipt| {
            receipt.status == FilesystemOperationStatus::RolledBack
                && receipt.kind == FilesystemOperationKind::Create
        }));

        let mut no_change = transaction_fixture(false);
        let mut no_change_driver =
            MemoryFilesystemDriver::new(&no_change.plan, FilesystemDriverMode::FailAt(0));
        let failed = execute_transaction(&mut no_change, &mut no_change_driver).expect("failed");
        assert_eq!(failed.outcome, FilesystemTransactionOutcome::FailedNoChange);
        assert_eq!(no_change_driver.restore_calls, 0);
        assert_eq!(no_change_driver.observations, no_change_driver.initial);
    }

    #[test]
    fn verification_mismatch_restores_while_uncertain_states_never_replay() {
        let mut mismatch = transaction_fixture(false);
        let mut mismatch_driver =
            MemoryFilesystemDriver::new(&mismatch.plan, FilesystemDriverMode::CorruptPoststate);
        let restored = execute_transaction(&mut mismatch, &mut mismatch_driver).expect("restored");
        assert_eq!(restored.outcome, FilesystemTransactionOutcome::Restored);
        assert_eq!(mismatch_driver.observations, mismatch_driver.initial);

        for mode in [
            FilesystemDriverMode::Uncertain,
            FilesystemDriverMode::MalformedAtomicPartial,
            FilesystemDriverMode::RestoreFails,
        ] {
            let mut fixture = transaction_fixture(false);
            let mut driver = MemoryFilesystemDriver::new(&fixture.plan, mode);
            let result = execute_transaction(&mut fixture, &mut driver).expect("uncertain result");
            assert_eq!(result.outcome, FilesystemTransactionOutcome::Uncertain);
            assert_eq!(
                fixture
                    .issuer
                    .current(&fixture.approval.grant.grant_id)
                    .map(|grant| grant.status),
                Some(GrantStatus::Uncertain)
            );
            assert_eq!(
                execute_transaction(&mut fixture, &mut driver),
                Err(FilesystemTransactionError::InvalidInput)
            );
            assert_eq!(driver.apply_calls, 1);
        }
    }

    #[test]
    fn filesystem_receipt_tampering_fails_closed() {
        type ReceiptMutation = Box<dyn Fn(&mut super::FilesystemOperationReceipt)>;

        let mut fixture = transaction_fixture(false);
        let mut driver = MemoryFilesystemDriver::new(&fixture.plan, FilesystemDriverMode::Success);
        let result = execute_transaction(&mut fixture, &mut driver).expect("committed");
        let mutations: Vec<ReceiptMutation> = vec![
            Box::new(|value| value.destination_path = Some("other/path".to_owned())),
            Box::new(|value| value.postimage_sha256 = "f".repeat(64)),
            Box::new(|value| value.destination_mode = Some(0o777)),
            Box::new(|value| value.previous_receipt_sha256 = "f".repeat(64)),
        ];
        for mutate in mutations {
            let mut receipts = result.receipts.clone();
            mutate(&mut receipts[0]);
            assert_eq!(
                verify_filesystem_receipts(&receipts),
                Err(FilesystemTransactionError::ReceiptIntegrity)
            );
        }
    }
}
