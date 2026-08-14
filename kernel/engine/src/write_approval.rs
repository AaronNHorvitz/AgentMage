//! Exact-preimage shadow changes, review previews, and bounded write grants.

use std::{collections::BTreeSet, fmt::Write as _};

use agentmage_kernel_contracts::{
    ActionId, ActionKind, ApprovalId, CapabilityGrant, GrantClass, GrantId, GrantNonce,
    GrantOperation, GrantPreimage, GrantSideEffect, GrantStatus, GrantTarget, OperationBinding,
    ToolId, WorkspaceObjectKind,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::grants::{DerivedOperationGrantRequest, GrantIssueError, GrantIssuer};

const MAX_OPERATIONS: usize = 128;
const MAX_CHANGE_BYTES: usize = 8 * 1024 * 1024;
const MAX_REVIEW_FIELD_BYTES: usize = 4_096;
const MAX_REVIEW_ITEMS: usize = 64;
const MAX_VERIFICATION_ITEMS: usize = 32;
const MAX_WRITE_GRANT_LIFETIME_MS: u64 = 300_000;
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Stable reason an exact-preimage write proposal cannot advance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteApprovalError {
    /// An identifier, bound, review field, or operation shape is invalid.
    InvalidInput,
    /// The current parent grant cannot authorize observation of every target.
    ParentUnavailable,
    /// A target is outside the current read scope or inside an exclusion.
    ScopeDenied,
    /// Observed bytes do not match the exact held-object preimage.
    PreimageMismatch,
    /// Proposed text is not valid UTF-8.
    InvalidEncoding,
    /// Proposed structured text does not satisfy its declared syntax.
    InvalidSyntax,
    /// Proposed text does not satisfy its declared line-ending contract.
    InvalidLineEndings,
    /// An operation identity or exact target appears more than once.
    DuplicateOperation,
    /// A generated file was proposed without exact generated-file permission.
    GeneratedFileDenied,
    /// The caller-provided expected postimage differs from the proposed bytes.
    PostimageMismatch,
    /// Approval does not bind the exact current change set and preview.
    ApprovalMismatch,
    /// Existing kernel grant issuance rejected the requested authority.
    GrantIssue,
    /// A fresh preapply observation is stale and the issued grant was invalidated.
    GrantStale,
    /// Retained grant state is absent, changed, or internally inconsistent.
    GrantState,
}

impl WriteApprovalError {
    /// Returns a stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "write.approval.invalid_input",
            Self::ParentUnavailable => "write.approval.parent_unavailable",
            Self::ScopeDenied => "write.approval.scope_denied",
            Self::PreimageMismatch => "write.approval.preimage_mismatch",
            Self::InvalidEncoding => "write.approval.encoding_invalid",
            Self::InvalidSyntax => "write.approval.syntax_invalid",
            Self::InvalidLineEndings => "write.approval.line_endings_invalid",
            Self::DuplicateOperation => "write.approval.operation_duplicate",
            Self::GeneratedFileDenied => "write.approval.generated_file_denied",
            Self::PostimageMismatch => "write.approval.postimage_mismatch",
            Self::ApprovalMismatch => "write.approval.decision_mismatch",
            Self::GrantIssue => "write.approval.grant_issue_failed",
            Self::GrantStale => "write.approval.grant_stale",
            Self::GrantState => "write.approval.grant_state_invalid",
        }
    }
}

impl From<GrantIssueError> for WriteApprovalError {
    fn from(_: GrantIssueError) -> Self {
        Self::GrantIssue
    }
}

/// Closed syntax validation available before language-specific editors exist.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteSyntax {
    /// Validate only bounded UTF-8 text.
    Utf8,
    /// Validate bounded UTF-8 containing one complete JSON value.
    Json,
}

/// Exact line-ending contract for one proposed postimage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteLineEndings {
    /// Text contains no line break.
    None,
    /// Every line break is LF and no carriage return is present.
    Lf,
    /// Every line break is CRLF and no bare carriage return or LF is present.
    CrLf,
}

/// Complete human-review narrative bound into one shadow change set.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WriteReviewNarrative {
    /// Why this exact change is proposed.
    pub rationale: String,
    /// User-visible or system behavior expected to change.
    pub behavior_change: String,
    /// Ordered checks proposed after a separately authorized application.
    pub verification_plan: Vec<String>,
    /// Known risks presented before approval.
    pub risks: Vec<String>,
    /// Bounded recovery approach presented before approval.
    pub rollback: String,
    /// Facts not yet verified and therefore excluded from completion claims.
    pub unverified_assumptions: Vec<String>,
}

/// One exact observed source and proposed postimage held outside user-owned files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShadowWriteDraft {
    /// Stable operation identity inside the requested change set.
    pub operation_id: String,
    /// Exact continuously held regular-file target under the current read grant.
    pub target: GrantTarget,
    /// Bytes read through that exact held target.
    pub observed_bytes: Vec<u8>,
    /// Proposed replacement bytes retained only in the shadow object.
    pub proposed_bytes: Vec<u8>,
    /// Caller-declared digest expected for the complete postimage.
    pub expected_postimage_sha256: String,
    /// Closed syntax validation to apply.
    pub syntax: WriteSyntax,
    /// Exact line endings expected in the postimage.
    pub line_endings: WriteLineEndings,
    /// Whether repository evidence identifies this target as generated.
    pub generated_file: bool,
    /// Exact per-operation exception allowing a generated target.
    pub allow_generated_file: bool,
}

/// Input for one bounded multi-file shadow change set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShadowChangeSetRequest {
    /// Stable caller-selected change-set identity.
    pub change_set_id: String,
    /// Kernel-clock observation time used to reject an expired read parent.
    pub observed_at_epoch_ms: u64,
    /// Ordered exact replacement drafts.
    pub operations: Vec<ShadowWriteDraft>,
    /// Complete human-review narrative.
    pub review: WriteReviewNarrative,
    /// Exact post-write verification labels shown before approval.
    pub permitted_verification: Vec<String>,
}

/// One validated exact replacement retained only in a shadow change set.
#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ShadowWriteOperation {
    operation_id: String,
    path: String,
    target: GrantTarget,
    preimage_sha256: String,
    preimage_byte_len: u64,
    expected_postimage_sha256: String,
    expected_postimage_byte_len: u64,
    syntax: WriteSyntax,
    line_endings: WriteLineEndings,
    generated_file: bool,
    complete_diff: String,
    #[serde(with = "byte_serialization")]
    proposed_bytes: Vec<u8>,
    operation_sha256: String,
}

impl std::fmt::Debug for ShadowWriteOperation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ShadowWriteOperation")
            .field("operation_id", &self.operation_id)
            .field("path", &self.path)
            .field("preimage_sha256", &self.preimage_sha256)
            .field("expected_postimage_sha256", &self.expected_postimage_sha256)
            .field(
                "expected_postimage_byte_len",
                &self.expected_postimage_byte_len,
            )
            .field("operation_sha256", &self.operation_sha256)
            .finish_non_exhaustive()
    }
}

impl ShadowWriteOperation {
    /// Returns the stable operation identity.
    #[must_use]
    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }

    /// Returns the canonical workspace-relative display path.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the exact held-object target.
    #[must_use]
    pub const fn target(&self) -> &GrantTarget {
        &self.target
    }

    /// Returns the exact observed source digest.
    #[must_use]
    pub fn preimage_sha256(&self) -> &str {
        &self.preimage_sha256
    }

    /// Returns the exact expected postimage digest.
    #[must_use]
    pub fn expected_postimage_sha256(&self) -> &str {
        &self.expected_postimage_sha256
    }

    /// Returns the proposed complete postimage bytes.
    #[must_use]
    pub fn proposed_bytes(&self) -> &[u8] {
        &self.proposed_bytes
    }

    /// Returns the complete escaped before/after diff shown for review.
    #[must_use]
    pub fn complete_diff(&self) -> &str {
        &self.complete_diff
    }

    /// Returns the canonical identity of this operation.
    #[must_use]
    pub fn operation_sha256(&self) -> &str {
        &self.operation_sha256
    }
}

/// Validated shadow change set that has no file-write authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ShadowChangeSet {
    change_set_id: String,
    workspace_id: String,
    parent_grant_id: GrantId,
    parent_grant_sha256: String,
    observed_at_epoch_ms: u64,
    operations: Vec<ShadowWriteOperation>,
    review: WriteReviewNarrative,
    permitted_verification: Vec<String>,
    change_set_sha256: String,
}

impl ShadowChangeSet {
    /// Returns the stable change-set identity.
    #[must_use]
    pub fn change_set_id(&self) -> &str {
        &self.change_set_id
    }

    /// Returns the exact workspace identity.
    #[must_use]
    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    /// Returns the ordered validated operations.
    #[must_use]
    pub fn operations(&self) -> &[ShadowWriteOperation] {
        &self.operations
    }

    /// Returns the complete review narrative.
    #[must_use]
    pub const fn review(&self) -> &WriteReviewNarrative {
        &self.review
    }

    /// Returns the exact post-write verification labels included in the preview.
    #[must_use]
    pub fn permitted_verification(&self) -> &[String] {
        &self.permitted_verification
    }

    /// Returns the canonical digest of every change-set field and postimage byte.
    #[must_use]
    pub fn change_set_sha256(&self) -> &str {
        &self.change_set_sha256
    }
}

/// Complete deterministic display object shown before write authority is requested.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WriteApprovalPreview {
    /// Exact shadow change-set identity.
    pub change_set_id: String,
    /// Canonical digest of the exact shadow change set.
    pub change_set_sha256: String,
    /// Exact workspace identity.
    pub workspace_id: String,
    /// Ordered affected paths.
    pub files: Vec<String>,
    /// Ordered stable operation identities.
    pub operations: Vec<String>,
    /// Complete escaped before/after diffs.
    pub complete_diffs: Vec<String>,
    /// Human-review narrative.
    pub review: WriteReviewNarrative,
    /// Exact post-write verification labels requested for separate authorization.
    pub permitted_verification: Vec<String>,
    /// Canonical digest of the complete preview with this field zeroed.
    pub preview_sha256: String,
}

/// Explicit user decision bound to one exact write preview.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriteApprovalDecision {
    /// Stable approval identity.
    pub approval_id: ApprovalId,
    /// Exact approved change-set digest.
    pub approved_change_set_sha256: String,
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
}

/// Kernel-selected identities needed to derive one existing operation grant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriteGrantRequest {
    /// Current session-read parent grant.
    pub parent_grant_id: GrantId,
    /// New single-use operation-grant identity.
    pub grant_id: GrantId,
    /// Exact action receiving the bounded authority.
    pub action_id: ActionId,
    /// Descriptive action class.
    pub action_kind: ActionKind,
    /// Exact registered write-tool identity.
    pub tool_id: ToolId,
    /// Exact registered write-tool version.
    pub tool_version: String,
    /// Unique anti-replay nonce.
    pub nonce: GrantNonce,
    /// Current policy digest, which must equal the parent policy.
    pub policy_sha256: String,
}

/// Non-authoritative proof that one exact write grant was issued.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriteApprovalReceipt {
    /// Existing kernel-enforced single-use operation grant.
    pub grant: CapabilityGrant,
    /// Exact change-set digest carried as the grant argument identity.
    pub change_set_sha256: String,
    /// Exact reviewed preview digest.
    pub preview_sha256: String,
    /// Exact verification labels cryptographically included in side-effect details.
    pub permitted_verification: Vec<String>,
    /// Canonical digest of this non-authoritative binding summary.
    pub binding_sha256: String,
}

/// Fresh observation used immediately before a future atomic application.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurrentWriteTarget {
    /// Fresh exact held-object target.
    pub target: GrantTarget,
    /// Fresh bytes read from that continuously held object.
    pub bytes: Vec<u8>,
}

/// Non-authoritative proof that current preimages still match an issued write grant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WritePreapplyReceipt {
    /// Exact single-use grant identity still awaiting atomic consumption.
    pub grant_id: GrantId,
    /// Exact shadow change-set identity.
    pub change_set_sha256: String,
    /// Ordered freshly observed preimage digests.
    pub current_preimages: Vec<String>,
    /// Kernel-clock validation time.
    pub validated_at_epoch_ms: u64,
    /// Canonical digest of this validation result.
    pub validation_sha256: String,
}

/// Constructs a deterministic, authority-free shadow change set.
pub fn build_shadow_change_set(
    parent: &CapabilityGrant,
    request: ShadowChangeSetRequest,
) -> Result<ShadowChangeSet, WriteApprovalError> {
    validate_identifier(&request.change_set_id)?;
    validate_review(&request.review)?;
    validate_verification(&request.permitted_verification)?;
    if parent.grant_class != GrantClass::SessionRead
        || parent.status != GrantStatus::Issued
        || request.observed_at_epoch_ms < parent.issued_at_epoch_ms
        || request.observed_at_epoch_ms >= parent.expires_at_epoch_ms
        || request.operations.is_empty()
        || request.operations.len() > MAX_OPERATIONS
    {
        return Err(WriteApprovalError::ParentUnavailable);
    }
    let parent_grant_sha256 = canonical_sha256(parent)?;
    let mut operation_ids = BTreeSet::new();
    let mut target_keys = BTreeSet::new();
    let mut total_bytes = 0_usize;
    let mut operations = Vec::with_capacity(request.operations.len());
    let workspace_id = request.operations[0]
        .target
        .workspace_id()
        .as_str()
        .to_owned();

    for draft in request.operations {
        validate_identifier(&draft.operation_id)?;
        if !operation_ids.insert(draft.operation_id.clone()) {
            return Err(WriteApprovalError::DuplicateOperation);
        }
        if draft.target.workspace_id().as_str() != workspace_id
            || draft.target.workspace_path().is_none()
            || draft.target.object_kind() != Some(WorkspaceObjectKind::RegularFile)
        {
            return Err(WriteApprovalError::InvalidInput);
        }
        if !parent
            .targets
            .iter()
            .any(|scope| scope.contains(&draft.target))
            || parent
                .excluded_targets
                .iter()
                .any(|excluded| excluded.contains(&draft.target))
        {
            return Err(WriteApprovalError::ScopeDenied);
        }
        let path = display_path(&draft.target);
        if !target_keys.insert((workspace_id.clone(), path.clone())) {
            return Err(WriteApprovalError::DuplicateOperation);
        }
        let Some(preimage) = draft.target.preimage() else {
            return Err(WriteApprovalError::PreimageMismatch);
        };
        if preimage.byte_len() != u64::try_from(draft.observed_bytes.len()).unwrap_or(u64::MAX)
            || hex_sha256(&draft.observed_bytes) != hex_bytes(preimage.content_sha256())
        {
            return Err(WriteApprovalError::PreimageMismatch);
        }
        if draft.generated_file && !draft.allow_generated_file {
            return Err(WriteApprovalError::GeneratedFileDenied);
        }
        let before = std::str::from_utf8(&draft.observed_bytes)
            .map_err(|_| WriteApprovalError::InvalidEncoding)?;
        let after = std::str::from_utf8(&draft.proposed_bytes)
            .map_err(|_| WriteApprovalError::InvalidEncoding)?;
        validate_syntax(draft.syntax, after)?;
        validate_line_endings(draft.line_endings, after)?;
        if draft.observed_bytes == draft.proposed_bytes {
            return Err(WriteApprovalError::InvalidInput);
        }
        let postimage_sha256 = hex_sha256(&draft.proposed_bytes);
        if !valid_sha256(&draft.expected_postimage_sha256)
            || draft.expected_postimage_sha256 != postimage_sha256
        {
            return Err(WriteApprovalError::PostimageMismatch);
        }
        total_bytes = total_bytes
            .checked_add(draft.observed_bytes.len())
            .and_then(|value| value.checked_add(draft.proposed_bytes.len()))
            .ok_or(WriteApprovalError::InvalidInput)?;
        if total_bytes > MAX_CHANGE_BYTES {
            return Err(WriteApprovalError::InvalidInput);
        }
        let preimage_sha256 = hex_sha256(&draft.observed_bytes);
        let complete_diff = exact_diff(&path, &preimage_sha256, &postimage_sha256, before, after);
        let mut operation = ShadowWriteOperation {
            operation_id: draft.operation_id,
            path,
            target: draft.target,
            preimage_sha256,
            preimage_byte_len: u64::try_from(draft.observed_bytes.len())
                .map_err(|_| WriteApprovalError::InvalidInput)?,
            expected_postimage_sha256: postimage_sha256,
            expected_postimage_byte_len: u64::try_from(draft.proposed_bytes.len())
                .map_err(|_| WriteApprovalError::InvalidInput)?,
            syntax: draft.syntax,
            line_endings: draft.line_endings,
            generated_file: draft.generated_file,
            complete_diff,
            proposed_bytes: draft.proposed_bytes,
            operation_sha256: ZERO_SHA256.to_owned(),
        };
        operation.operation_sha256 = canonical_sha256(&operation)?;
        operations.push(operation);
    }

    let mut change_set = ShadowChangeSet {
        change_set_id: request.change_set_id,
        workspace_id,
        parent_grant_id: parent.grant_id.clone(),
        parent_grant_sha256,
        observed_at_epoch_ms: request.observed_at_epoch_ms,
        operations,
        review: request.review,
        permitted_verification: request.permitted_verification,
        change_set_sha256: ZERO_SHA256.to_owned(),
    };
    change_set.change_set_sha256 = canonical_sha256(&change_set)?;
    Ok(change_set)
}

/// Renders the complete deterministic review display for one exact change set.
pub fn render_write_preview(
    change_set: &ShadowChangeSet,
) -> Result<WriteApprovalPreview, WriteApprovalError> {
    verify_change_set(change_set)?;
    let mut preview = WriteApprovalPreview {
        change_set_id: change_set.change_set_id.clone(),
        change_set_sha256: change_set.change_set_sha256.clone(),
        workspace_id: change_set.workspace_id.clone(),
        files: change_set
            .operations
            .iter()
            .map(|operation| operation.path.clone())
            .collect(),
        operations: change_set
            .operations
            .iter()
            .map(|operation| operation.operation_id.clone())
            .collect(),
        complete_diffs: change_set
            .operations
            .iter()
            .map(|operation| operation.complete_diff.clone())
            .collect(),
        review: change_set.review.clone(),
        permitted_verification: change_set.permitted_verification.clone(),
        preview_sha256: ZERO_SHA256.to_owned(),
    };
    preview.preview_sha256 = canonical_sha256(&preview)?;
    Ok(preview)
}

/// Verifies an exact decision and derives one short-lived single-use kernel grant.
pub fn issue_write_grant(
    issuer: &mut GrantIssuer,
    change_set: &ShadowChangeSet,
    preview: &WriteApprovalPreview,
    decision: &WriteApprovalDecision,
    request: WriteGrantRequest,
) -> Result<WriteApprovalReceipt, WriteApprovalError> {
    verify_change_set(change_set)?;
    verify_preview(change_set, preview)?;
    validate_identifier(decision.approval_id.as_str())?;
    validate_verification(&decision.permitted_verification)?;
    let current_parent = issuer
        .current(&request.parent_grant_id)
        .ok_or(WriteApprovalError::ParentUnavailable)?;
    let current_parent_sha256 = issuer
        .revision_hash(&current_parent.grant_id, current_parent.revision)
        .ok_or(WriteApprovalError::GrantState)?;
    if !decision.user_confirmed
        || decision.approved_change_set_sha256 != change_set.change_set_sha256
        || decision.approved_preview_sha256 != preview.preview_sha256
        || decision.permitted_verification != change_set.permitted_verification
        || decision.approved_at_epoch_ms < change_set.observed_at_epoch_ms
        || decision.expires_at_epoch_ms <= decision.approved_at_epoch_ms
        || decision.expires_at_epoch_ms - decision.approved_at_epoch_ms
            > MAX_WRITE_GRANT_LIFETIME_MS
        || request.parent_grant_id != change_set.parent_grant_id
        || current_parent_sha256 != change_set.parent_grant_sha256
        || request.policy_sha256.is_empty()
    {
        return Err(WriteApprovalError::ApprovalMismatch);
    }
    let verification_sha256 = canonical_sha256(&decision.permitted_verification)?;
    let targets: Vec<_> = change_set
        .operations
        .iter()
        .map(|operation| operation.target.clone())
        .collect();
    let preimages = targets
        .iter()
        .enumerate()
        .map(|(index, target)| {
            let index = u32::try_from(index).map_err(|_| WriteApprovalError::InvalidInput)?;
            GrantPreimage::for_target(index, target).ok_or(WriteApprovalError::PreimageMismatch)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let operation = OperationBinding::new(GrantOperation::WorkspaceWrite);
    let expected_side_effects = change_set
        .operations
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let target_index =
                u32::try_from(index).map_err(|_| WriteApprovalError::InvalidInput)?;
            let details_sha256 = canonical_sha256(&(
                item.operation_sha256.as_str(),
                item.expected_postimage_sha256.as_str(),
                verification_sha256.as_str(),
            ))?;
            Ok(GrantSideEffect {
                operation,
                target_indexes: vec![target_index],
                details_sha256,
            })
        })
        .collect::<Result<Vec<_>, WriteApprovalError>>()?;
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
            argument_sha256: change_set.change_set_sha256.clone(),
            preimages,
            expected_side_effects,
            rollback_description: change_set.review.rollback.clone(),
            issued_at_epoch_ms: decision.approved_at_epoch_ms,
            expires_at_epoch_ms: decision.expires_at_epoch_ms,
            nonce: request.nonce,
            preview_sha256: preview.preview_sha256.clone(),
            policy_sha256: request.policy_sha256,
        },
    )?;
    let binding_sha256 = canonical_sha256(&(
        grant.grant_id.as_str(),
        change_set.change_set_sha256.as_str(),
        preview.preview_sha256.as_str(),
        decision.permitted_verification.as_slice(),
    ))?;
    Ok(WriteApprovalReceipt {
        grant,
        change_set_sha256: change_set.change_set_sha256.clone(),
        preview_sha256: preview.preview_sha256.clone(),
        permitted_verification: decision.permitted_verification.clone(),
        binding_sha256,
    })
}

/// Revalidates every exact preimage and invalidates the grant on any mismatch.
///
/// Success is not write authority and does not consume the grant. Sprint 36 must
/// preserve held-object continuity and atomically consume the same grant before
/// any target bytes can change.
pub fn revalidate_before_apply(
    issuer: &mut GrantIssuer,
    change_set: &ShadowChangeSet,
    approval: &WriteApprovalReceipt,
    current: &[CurrentWriteTarget],
    now_epoch_ms: u64,
) -> Result<WritePreapplyReceipt, WriteApprovalError> {
    let issued = issuer
        .current(&approval.grant.grant_id)
        .cloned()
        .ok_or(WriteApprovalError::GrantState)?;
    let valid_binding = issued == approval.grant
        && issued.status == GrantStatus::Issued
        && issued.operation == OperationBinding::new(GrantOperation::WorkspaceWrite)
        && issued.argument_sha256 == change_set.change_set_sha256
        && issued.preview_sha256 == approval.preview_sha256
        && now_epoch_ms >= issued.issued_at_epoch_ms
        && now_epoch_ms < issued.expires_at_epoch_ms
        && current.len() == change_set.operations.len()
        && issued.targets.len() == current.len()
        && issued.preimages.len() == current.len();
    let mut current_preimages = Vec::with_capacity(current.len());
    let observations_match = valid_binding
        && current.iter().enumerate().all(|(index, observation)| {
            let operation = &change_set.operations[index];
            let digest = hex_sha256(&observation.bytes);
            let target_preimage_matches = observation.target.preimage().is_some_and(|preimage| {
                preimage.byte_len() == u64::try_from(observation.bytes.len()).unwrap_or(u64::MAX)
                    && hex_bytes(preimage.content_sha256()) == digest
            });
            let matches = observation.target == operation.target
                && observation.target == issued.targets[index]
                && target_preimage_matches
                && digest == operation.preimage_sha256
                && issued.preimages[index].matches_target(
                    u32::try_from(index).unwrap_or(u32::MAX),
                    &observation.target,
                );
            current_preimages.push(digest);
            matches
        });
    if !observations_match {
        invalidate_issued_grant(issuer, &issued, now_epoch_ms)?;
        return Err(WriteApprovalError::GrantStale);
    }
    let validation_sha256 = canonical_sha256(&(
        issued.grant_id.as_str(),
        change_set.change_set_sha256.as_str(),
        current_preimages.as_slice(),
        now_epoch_ms,
    ))?;
    Ok(WritePreapplyReceipt {
        grant_id: issued.grant_id,
        change_set_sha256: change_set.change_set_sha256.clone(),
        current_preimages,
        validated_at_epoch_ms: now_epoch_ms,
        validation_sha256,
    })
}

fn invalidate_issued_grant(
    issuer: &mut GrantIssuer,
    grant: &CapabilityGrant,
    now_epoch_ms: u64,
) -> Result<(), WriteApprovalError> {
    let digest = issuer
        .revision_hash(&grant.grant_id, grant.revision)
        .map(str::to_owned)
        .ok_or(WriteApprovalError::GrantState)?;
    issuer
        .cancel_before_execution(&grant.grant_id, &digest, now_epoch_ms)
        .map_err(|_| WriteApprovalError::GrantState)?;
    Ok(())
}

fn verify_change_set(change_set: &ShadowChangeSet) -> Result<(), WriteApprovalError> {
    if !valid_sha256(&change_set.change_set_sha256) {
        return Err(WriteApprovalError::InvalidInput);
    }
    let mut candidate = change_set.clone();
    candidate.change_set_sha256 = ZERO_SHA256.to_owned();
    if canonical_sha256(&candidate)? != change_set.change_set_sha256 {
        return Err(WriteApprovalError::PostimageMismatch);
    }
    for operation in &change_set.operations {
        let mut candidate = operation.clone();
        candidate.operation_sha256 = ZERO_SHA256.to_owned();
        if canonical_sha256(&candidate)? != operation.operation_sha256
            || hex_sha256(&operation.proposed_bytes) != operation.expected_postimage_sha256
        {
            return Err(WriteApprovalError::PostimageMismatch);
        }
    }
    Ok(())
}

fn verify_preview(
    change_set: &ShadowChangeSet,
    preview: &WriteApprovalPreview,
) -> Result<(), WriteApprovalError> {
    let expected = render_write_preview(change_set)?;
    if &expected != preview {
        return Err(WriteApprovalError::ApprovalMismatch);
    }
    Ok(())
}

fn validate_review(review: &WriteReviewNarrative) -> Result<(), WriteApprovalError> {
    for field in [
        review.rationale.as_str(),
        review.behavior_change.as_str(),
        review.rollback.as_str(),
    ] {
        if field.trim().is_empty() || field.len() > MAX_REVIEW_FIELD_BYTES {
            return Err(WriteApprovalError::InvalidInput);
        }
    }
    if review.verification_plan.is_empty()
        || review.verification_plan.len() > MAX_REVIEW_ITEMS
        || review.risks.is_empty()
        || review.risks.len() > MAX_REVIEW_ITEMS
        || review.unverified_assumptions.len() > MAX_REVIEW_ITEMS
    {
        return Err(WriteApprovalError::InvalidInput);
    }
    for item in review
        .verification_plan
        .iter()
        .chain(&review.risks)
        .chain(&review.unverified_assumptions)
    {
        if item.trim().is_empty() || item.len() > MAX_REVIEW_FIELD_BYTES {
            return Err(WriteApprovalError::InvalidInput);
        }
    }
    Ok(())
}

fn validate_verification(items: &[String]) -> Result<(), WriteApprovalError> {
    if items.len() > MAX_VERIFICATION_ITEMS {
        return Err(WriteApprovalError::InvalidInput);
    }
    for item in items {
        validate_identifier(item)?;
    }
    Ok(())
}

fn validate_identifier(value: &str) -> Result<(), WriteApprovalError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
    {
        return Err(WriteApprovalError::InvalidInput);
    }
    Ok(())
}

fn validate_syntax(syntax: WriteSyntax, value: &str) -> Result<(), WriteApprovalError> {
    match syntax {
        WriteSyntax::Utf8 => Ok(()),
        WriteSyntax::Json => serde_json::from_str::<serde_json::Value>(value)
            .map(|_| ())
            .map_err(|_| WriteApprovalError::InvalidSyntax),
    }
}

fn validate_line_endings(
    expected: WriteLineEndings,
    value: &str,
) -> Result<(), WriteApprovalError> {
    let bytes = value.as_bytes();
    let valid = match expected {
        WriteLineEndings::None => !bytes.contains(&b'\r') && !bytes.contains(&b'\n'),
        WriteLineEndings::Lf => !bytes.contains(&b'\r'),
        WriteLineEndings::CrLf => {
            let mut index = 0;
            let mut valid = true;
            while index < bytes.len() {
                if bytes[index] == b'\r' {
                    if bytes.get(index + 1) != Some(&b'\n') {
                        valid = false;
                        break;
                    }
                    index += 2;
                } else if bytes[index] == b'\n' {
                    valid = false;
                    break;
                } else {
                    index += 1;
                }
            }
            valid
        }
    };
    valid
        .then_some(())
        .ok_or(WriteApprovalError::InvalidLineEndings)
}

fn display_path(target: &GrantTarget) -> String {
    target
        .path_components()
        .iter()
        .map(|component| component.as_str())
        .collect::<Vec<_>>()
        .join("/")
}

fn exact_diff(
    path: &str,
    before_hash: &str,
    after_hash: &str,
    before: &str,
    after: &str,
) -> String {
    format!(
        "--- {path} sha256:{before_hash}\n+++ {path} sha256:{after_hash}\n@@ exact-complete-postimage @@\n-{before:?}\n+{after:?}\n"
    )
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn canonical_sha256(value: &impl Serialize) -> Result<String, WriteApprovalError> {
    serde_json::to_vec(value)
        .map(|bytes| hex_sha256(&bytes))
        .map_err(|_| WriteApprovalError::InvalidInput)
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

mod byte_serialization {
    use serde::{Serialize, Serializer};

    pub(super) fn serialize<S>(value: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value.serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{grants::SessionReadGrantRequest, test_target::scope};
    use agentmage_kernel_contracts::{
        ActorId, DataSensitivity, GrantNonce, PathPlatform, SessionId, TaskId,
    };

    type RequestMutation = Box<dyn Fn(&mut ShadowChangeSetRequest)>;

    fn target(path: &[&str], bytes: &[u8]) -> GrantTarget {
        let digest: [u8; 32] = Sha256::digest(bytes).into();
        serde_json::from_value(serde_json::json!({
            "target_kind": "held_object",
            "path": {"workspace_id": "workspace-0001", "components": path},
            "authorization_id": "authorization-0001",
            "adapter_instance_id": "adapter-0001",
            "platform": "deterministic_fake",
            "object_kind": "regular_file",
            "object_identity": {
                "platform": "deterministic_fake",
                "mount_identity_sha256": vec![1_u8; 32],
                "object_identity_sha256": hex_sha256(path.join("/").as_bytes())
                    .as_bytes()[..32].to_vec()
            },
            "preimage": {"byte_len": bytes.len(), "content_sha256": digest}
        }))
        .expect("synthetic exact write target")
    }

    fn parent() -> (GrantIssuer, CapabilityGrant) {
        let mut issuer = GrantIssuer::new();
        let parent = issuer
            .issue_session_read(SessionReadGrantRequest {
                grant_id: GrantId::from_raw("grant-parent-write"),
                actor_id: ActorId::from_raw("actor-local"),
                session_id: SessionId::from_raw("session-write"),
                task_id: TaskId::from_raw("task-write"),
                targets: vec![scope(&[])],
                excluded_targets: vec![scope(&["private"])],
                sensitivity: DataSensitivity::Restricted,
                issued_at_epoch_ms: 1_000,
                expires_at_epoch_ms: 100_000,
                nonce: GrantNonce::from_raw("nonce-parent-write"),
                maximum_derived_operations: 8,
                preview_sha256: "a".repeat(64),
                policy_sha256: "b".repeat(64),
            })
            .expect("session parent");
        (issuer, parent)
    }

    fn review() -> WriteReviewNarrative {
        WriteReviewNarrative {
            rationale: "Update the exact fixture behavior".to_owned(),
            behavior_change: "The fixture reports the approved value".to_owned(),
            verification_plan: vec!["Run the focused fixture test".to_owned()],
            risks: vec!["A caller may rely on the old fixture value".to_owned()],
            rollback: "Restore the reviewed preimage in a new transaction".to_owned(),
            unverified_assumptions: vec!["No external consumer was inspected".to_owned()],
        }
    }

    fn request() -> ShadowChangeSetRequest {
        let before = b"{\"enabled\":false}\n".to_vec();
        let after = b"{\"enabled\":true}\n".to_vec();
        ShadowChangeSetRequest {
            change_set_id: "change-set-0001".to_owned(),
            observed_at_epoch_ms: 2_000,
            operations: vec![ShadowWriteDraft {
                operation_id: "operation-0001".to_owned(),
                target: target(&["src", "fixture.json"], &before),
                observed_bytes: before,
                proposed_bytes: after.clone(),
                expected_postimage_sha256: hex_sha256(&after),
                syntax: WriteSyntax::Json,
                line_endings: WriteLineEndings::Lf,
                generated_file: false,
                allow_generated_file: false,
            }],
            review: review(),
            permitted_verification: vec!["cargo-test-focused".to_owned()],
        }
    }

    fn decision(preview: &WriteApprovalPreview) -> WriteApprovalDecision {
        WriteApprovalDecision {
            approval_id: ApprovalId::from_raw("approval-write-0001"),
            approved_change_set_sha256: preview.change_set_sha256.clone(),
            approved_preview_sha256: preview.preview_sha256.clone(),
            approved_at_epoch_ms: 3_000,
            expires_at_epoch_ms: 30_000,
            permitted_verification: vec!["cargo-test-focused".to_owned()],
            user_confirmed: true,
        }
    }

    fn grant_request() -> WriteGrantRequest {
        WriteGrantRequest {
            parent_grant_id: GrantId::from_raw("grant-parent-write"),
            grant_id: GrantId::from_raw("grant-write-0001"),
            action_id: ActionId::from_raw("action-write-0001"),
            action_kind: ActionKind::DeterministicTool,
            tool_id: ToolId::from_raw("workspace.write"),
            tool_version: "1.0.0".to_owned(),
            nonce: GrantNonce::from_raw("nonce-write-0001"),
            policy_sha256: "b".repeat(64),
        }
    }

    #[test]
    fn shadow_change_preview_and_grant_bind_every_exact_input() {
        let (mut issuer, parent) = parent();
        let change_set = build_shadow_change_set(&parent, request()).expect("shadow change");
        let preview = render_write_preview(&change_set).expect("preview");
        let receipt = issue_write_grant(
            &mut issuer,
            &change_set,
            &preview,
            &decision(&preview),
            grant_request(),
        )
        .expect("single-use grant");

        assert_eq!(receipt.grant.use_limit, 1);
        assert_eq!(
            receipt.grant.operation.operation(),
            GrantOperation::WorkspaceWrite
        );
        assert_eq!(
            receipt.grant.argument_sha256,
            change_set.change_set_sha256()
        );
        assert_eq!(receipt.grant.preview_sha256, preview.preview_sha256);
        assert!(
            change_set.operations()[0]
                .complete_diff()
                .contains("enabled")
        );
        assert_eq!(receipt.permitted_verification, ["cargo-test-focused"]);

        let mut replay = grant_request();
        replay.grant_id = GrantId::from_raw("grant-write-0002");
        replay.nonce = GrantNonce::from_raw("nonce-write-0002");
        assert_eq!(
            issue_write_grant(
                &mut issuer,
                &change_set,
                &preview,
                &decision(&preview),
                replay,
            ),
            Err(WriteApprovalError::ApprovalMismatch)
        );
    }

    #[test]
    fn stale_partial_excluded_generated_duplicate_and_invalid_proposals_fail_closed() {
        let (_, parent) = parent();
        let mutations: Vec<RequestMutation> = vec![
            Box::new(|value| value.operations[0].observed_bytes.push(b'x')),
            Box::new(|value| value.operations[0].expected_postimage_sha256 = "c".repeat(64)),
            Box::new(|value| value.operations[0].generated_file = true),
            Box::new(|value| value.operations[0].proposed_bytes = b"{invalid}\n".to_vec()),
            Box::new(|value| value.operations[0].line_endings = WriteLineEndings::CrLf),
            Box::new(|value| value.operations.push(value.operations[0].clone())),
            Box::new(|value| {
                let before = value.operations[0].observed_bytes.clone();
                value.operations[0].target = target(&["private", "fixture.json"], &before);
            }),
        ];
        for mutate in mutations {
            let mut candidate = request();
            mutate(&mut candidate);
            assert!(build_shadow_change_set(&parent, candidate).is_err());
        }
    }

    #[test]
    fn changed_preview_decision_expiry_and_policy_cannot_issue_authority() {
        let (mut issuer, parent) = parent();
        let change_set = build_shadow_change_set(&parent, request()).expect("shadow change");
        let preview = render_write_preview(&change_set).expect("preview");

        let mut changed_preview = preview.clone();
        changed_preview.review.rationale.push('!');
        assert_eq!(
            issue_write_grant(
                &mut issuer,
                &change_set,
                &changed_preview,
                &decision(&preview),
                grant_request(),
            ),
            Err(WriteApprovalError::ApprovalMismatch)
        );
        let mut expired = decision(&preview);
        expired.expires_at_epoch_ms = expired.approved_at_epoch_ms + 300_001;
        assert_eq!(
            issue_write_grant(
                &mut issuer,
                &change_set,
                &preview,
                &expired,
                grant_request(),
            ),
            Err(WriteApprovalError::ApprovalMismatch)
        );
        let mut cancelled = decision(&preview);
        cancelled.user_confirmed = false;
        assert_eq!(
            issue_write_grant(
                &mut issuer,
                &change_set,
                &preview,
                &cancelled,
                grant_request(),
            ),
            Err(WriteApprovalError::ApprovalMismatch)
        );
        let mut broader_verification = decision(&preview);
        broader_verification
            .permitted_verification
            .push("cargo-test-all".to_owned());
        assert_eq!(
            issue_write_grant(
                &mut issuer,
                &change_set,
                &preview,
                &broader_verification,
                grant_request(),
            ),
            Err(WriteApprovalError::ApprovalMismatch)
        );
        let mut wrong_policy = grant_request();
        wrong_policy.policy_sha256 = "c".repeat(64);
        assert_eq!(
            issue_write_grant(
                &mut issuer,
                &change_set,
                &preview,
                &decision(&preview),
                wrong_policy,
            ),
            Err(WriteApprovalError::GrantIssue)
        );
    }

    #[test]
    fn fresh_preimages_validate_without_consuming_or_writing() {
        let (mut issuer, parent) = parent();
        let change_set = build_shadow_change_set(&parent, request()).expect("shadow change");
        let preview = render_write_preview(&change_set).expect("preview");
        let approval = issue_write_grant(
            &mut issuer,
            &change_set,
            &preview,
            &decision(&preview),
            grant_request(),
        )
        .expect("grant");
        let current = CurrentWriteTarget {
            target: change_set.operations()[0].target().clone(),
            bytes: b"{\"enabled\":false}\n".to_vec(),
        };
        let receipt =
            revalidate_before_apply(&mut issuer, &change_set, &approval, &[current], 4_000)
                .expect("fresh validation");
        assert_eq!(
            receipt.current_preimages,
            [change_set.operations()[0].preimage_sha256()]
        );
        assert_eq!(
            issuer
                .current(&approval.grant.grant_id)
                .map(|grant| grant.status),
            Some(GrantStatus::Issued)
        );
    }

    #[test]
    fn changed_preimage_invalidates_the_single_use_grant() {
        let (mut issuer, parent) = parent();
        let change_set = build_shadow_change_set(&parent, request()).expect("shadow change");
        let preview = render_write_preview(&change_set).expect("preview");
        let approval = issue_write_grant(
            &mut issuer,
            &change_set,
            &preview,
            &decision(&preview),
            grant_request(),
        )
        .expect("grant");
        let changed = b"{\"enabled\":\"changed\"}\n".to_vec();
        let current = CurrentWriteTarget {
            target: target(&["src", "fixture.json"], &changed),
            bytes: changed,
        };
        assert_eq!(
            revalidate_before_apply(&mut issuer, &change_set, &approval, &[current], 4_000,),
            Err(WriteApprovalError::GrantStale)
        );
        assert_eq!(
            issuer
                .current(&approval.grant.grant_id)
                .map(|grant| grant.status),
            Some(GrantStatus::Invalidated)
        );
    }

    #[test]
    fn line_endings_accept_only_the_exact_declared_form() {
        assert_eq!(
            validate_line_endings(WriteLineEndings::Lf, "a\nb\n"),
            Ok(())
        );
        assert_eq!(
            validate_line_endings(WriteLineEndings::Lf, "a\r\nb\r\n"),
            Err(WriteApprovalError::InvalidLineEndings)
        );
        assert_eq!(
            validate_line_endings(WriteLineEndings::CrLf, "a\r\nb\r\n"),
            Ok(())
        );
        assert_eq!(
            validate_line_endings(WriteLineEndings::None, "a\n"),
            Err(WriteApprovalError::InvalidLineEndings)
        );
        let _ = PathPlatform::DeterministicFake;
    }
}
