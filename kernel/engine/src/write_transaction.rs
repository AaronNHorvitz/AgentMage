//! Grant-consuming atomic write coordination, receipts, restoration, and rollback proposals.

use std::{collections::BTreeMap, fmt::Write as _};

use agentmage_kernel_contracts::{
    CapabilityGrant, GrantId, GrantOperation, GrantStatus, GrantTarget, OperationBinding,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    grants::{GrantConsumeError, GrantConsumptionRecord, GrantIssuer},
    policy::{PolicyEngine, PolicyEvaluationContext},
    write_approval::{
        CurrentWriteTarget, ShadowChangeSet, ShadowChangeSetRequest, ShadowWriteDraft,
        WriteApprovalError, WriteApprovalReceipt, WriteReviewNarrative, revalidate_before_apply,
    },
};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_FAILURE_CODE_BYTES: usize = 128;

/// Stable reason an atomic write transaction cannot produce a reconciled result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteTransactionError {
    /// Transaction input or a retained binding is malformed.
    InvalidInput,
    /// Fresh preapply validation failed before grant consumption.
    PreapplyDenied,
    /// Current deterministic policy denied final grant consumption.
    PolicyDenied,
    /// The platform driver could not produce a bounded observation.
    ObservationFailed,
    /// The platform driver returned a malformed or contradictory effect report.
    DriverReportInvalid,
    /// Receipt hashing or transition state became inconsistent.
    ReceiptIntegrity,
}

impl WriteTransactionError {
    /// Returns a stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "write.transaction.invalid_input",
            Self::PreapplyDenied => "write.transaction.preapply_denied",
            Self::PolicyDenied => "write.transaction.policy_denied",
            Self::ObservationFailed => "write.transaction.observation_failed",
            Self::DriverReportInvalid => "write.transaction.driver_report_invalid",
            Self::ReceiptIntegrity => "write.transaction.receipt_integrity",
        }
    }
}

impl From<WriteApprovalError> for WriteTransactionError {
    fn from(_: WriteApprovalError) -> Self {
        Self::PreapplyDenied
    }
}

impl From<GrantConsumeError> for WriteTransactionError {
    fn from(_: GrantConsumeError) -> Self {
        Self::PolicyDenied
    }
}

/// Exact per-operation lifecycle states retained in receipt order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteOperationStatus {
    /// Exact shadow operation exists without write authority.
    Proposed,
    /// Exact preview and single-use grant were approved.
    Approved,
    /// The driver reports that the approved postimage was applied.
    Applied,
    /// Fresh observation verified the exact approved postimage.
    Verified,
    /// The operation or transaction failed.
    Failed,
    /// A fresh restoration returned the exact original preimage.
    RolledBack,
    /// An earlier failure prevented this approved operation from running.
    Superseded,
}

impl WriteOperationStatus {
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

/// One hash-chained content-free operation lifecycle receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WriteOperationReceipt {
    /// Stable transaction identity.
    pub transaction_id: String,
    /// Monotonic receipt sequence within the transaction.
    pub sequence: u32,
    /// Stable shadow operation identity.
    pub operation_id: String,
    /// Exact operation lifecycle state.
    pub status: WriteOperationStatus,
    /// Exact source preimage digest.
    pub preimage_sha256: String,
    /// Exact approved postimage digest.
    pub postimage_sha256: String,
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

/// Terminal disposition of one attempted atomic write transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteTransactionOutcome {
    /// Every exact postimage was applied and freshly verified.
    Committed,
    /// The driver failed before any target changed.
    FailedNoChange,
    /// Known partial changes were restored to exact reviewed preimages.
    Restored,
    /// State could not be reconciled without risking later user work.
    Uncertain,
}

/// Descriptive post-write check that still requires a separate command grant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SeparateVerificationRequirement {
    /// Exact label shown before the write approval.
    pub verification_id: String,
    /// This transaction never executes the check directly.
    pub executed: bool,
    /// A separate command grant is mandatory.
    pub requires_separate_command_grant: bool,
}

/// Reconciled terminal result with exact receipts and no raw target content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriteTransactionResult {
    /// Stable transaction identity.
    pub transaction_id: String,
    /// Terminal disposition.
    pub outcome: WriteTransactionOutcome,
    /// Kernel proof that the exact operation grant was consumed once.
    pub grant_consumption: GrantConsumptionRecord,
    /// Complete hash-chained per-operation receipt history.
    pub receipts: Vec<WriteOperationReceipt>,
    /// Descriptive checks that remain separately permissioned and unexecuted.
    pub verification_requirements: Vec<SeparateVerificationRequirement>,
}

/// Exact transaction request supplied by the kernel-owned caller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriteTransactionRequest {
    /// Stable transaction identity.
    pub transaction_id: String,
    /// Kernel-clock time for final preimage validation and grant consumption.
    pub now_epoch_ms: u64,
}

/// Stable platform-driver failure class retained without path or content values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteDriverError {
    /// Fresh target observation was unavailable.
    ObservationUnavailable,
    /// Staging or application failed with a known unchanged or partial state.
    ApplyFailed,
    /// Restoration failed with a known or unknown state.
    RestoreFailed,
    /// The platform cannot determine whether an effect occurred.
    Uncertain,
}

impl WriteDriverError {
    /// Returns a stable content-free code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ObservationUnavailable => "write.driver.observation_unavailable",
            Self::ApplyFailed => "write.driver.apply_failed",
            Self::RestoreFailed => "write.driver.restore_failed",
            Self::Uncertain => "write.driver.uncertain",
        }
    }
}

/// Driver report for one ordered application attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriteApplyReport {
    /// Whether the platform claims all-or-nothing application support for this attempt.
    pub atomic: bool,
    /// Ordered indexes known to have received their complete approved postimages.
    pub applied_indexes: Vec<u32>,
    /// First operation index that failed, absent only for complete success or uncertainty.
    pub failure_index: Option<u32>,
    /// Stable content-free failure code.
    pub failure_code: Option<String>,
    /// Whether the driver cannot determine exact resulting target state.
    pub uncertain: bool,
}

/// Driver report for one bounded restoration attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriteRestoreReport {
    /// Ordered indexes reported restored to their complete reviewed preimages.
    pub restored_indexes: Vec<u32>,
    /// Stable content-free failure code.
    pub failure_code: Option<String>,
    /// Whether restoration left an indeterminate state.
    pub uncertain: bool,
}

/// Opaque one-use authorization passed only after grant consumption.
pub struct WriteApplyAuthorization<'transaction> {
    transaction_id: &'transaction str,
    consumed_grant_sha256: &'transaction str,
    change_set: &'transaction ShadowChangeSet,
}

impl WriteApplyAuthorization<'_> {
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

    /// Returns the exact approved change set.
    #[must_use]
    pub const fn change_set(&self) -> &ShadowChangeSet {
        self.change_set
    }
}

/// Opaque restoration authorization limited to indexes known to have changed.
pub struct WriteRestoreAuthorization<'transaction> {
    transaction_id: &'transaction str,
    change_set: &'transaction ShadowChangeSet,
    restore_indexes: &'transaction [u32],
}

impl WriteRestoreAuthorization<'_> {
    /// Returns the stable transaction identity.
    #[must_use]
    pub const fn transaction_id(&self) -> &str {
        self.transaction_id
    }

    /// Returns the exact approved change set containing reviewed preimage bytes.
    #[must_use]
    pub const fn change_set(&self) -> &ShadowChangeSet {
        self.change_set
    }

    /// Returns only the operation indexes known to require restoration.
    #[must_use]
    pub const fn restore_indexes(&self) -> &[u32] {
        self.restore_indexes
    }
}

/// Platform effect boundary for an exact controlled-write transaction.
pub trait AtomicWriteDriver {
    /// Freshly observes every exact target in change-set order without changing it.
    fn observe(
        &mut self,
        change_set: &ShadowChangeSet,
    ) -> Result<Vec<CurrentWriteTarget>, WriteDriverError>;

    /// Applies exact postimages using an opaque consumed-grant authorization.
    fn apply(&mut self, authorization: WriteApplyAuthorization<'_>) -> WriteApplyReport;

    /// Restores exact reviewed preimages using a fresh bounded authorization.
    fn restore(&mut self, authorization: WriteRestoreAuthorization<'_>) -> WriteRestoreReport;
}

/// Runs final revalidation, consumes one grant, applies, verifies, and restores on known failure.
pub fn execute_write_transaction<D: AtomicWriteDriver>(
    issuer: &mut GrantIssuer,
    policy: &PolicyEngine,
    change_set: &ShadowChangeSet,
    approval: &WriteApprovalReceipt,
    request: WriteTransactionRequest,
    driver: &mut D,
) -> Result<WriteTransactionResult, WriteTransactionError> {
    validate_identifier(&request.transaction_id)?;
    let current = driver
        .observe(change_set)
        .map_err(|_| WriteTransactionError::ObservationFailed)?;
    revalidate_before_apply(issuer, change_set, approval, &current, request.now_epoch_ms)?;
    let context = policy_context(&approval.grant, request.now_epoch_ms)?;
    let consumption = issuer.consume_for_execution(&approval.grant.grant_id, policy, &context)?;
    let mut ledger = WriteReceiptLedger::new(
        &request.transaction_id,
        change_set,
        &approval.grant.grant_id,
        request.now_epoch_ms,
    )?;
    let report = driver.apply(WriteApplyAuthorization {
        transaction_id: &request.transaction_id,
        consumed_grant_sha256: &consumption.consumed_grant_sha256,
        change_set,
    });

    let disposition = classify_apply_report(change_set.operations().len(), &report).unwrap_or(
        ApplyDisposition::Uncertain {
            failure_code: WriteTransactionError::DriverReportInvalid.code().to_owned(),
        },
    );
    let outcome = match disposition {
        ApplyDisposition::Success => {
            for index in 0..change_set.operations().len() {
                ledger.transition(
                    index,
                    WriteOperationStatus::Applied,
                    None,
                    request.now_epoch_ms,
                )?;
            }
            match driver.observe(change_set) {
                Ok(postimages) if observations_match_postimages(change_set, &postimages) => {
                    for index in 0..change_set.operations().len() {
                        ledger.transition(
                            index,
                            WriteOperationStatus::Verified,
                            None,
                            request.now_epoch_ms,
                        )?;
                    }
                    WriteTransactionOutcome::Committed
                }
                _ => restore_after_failure(
                    driver,
                    change_set,
                    &request,
                    &mut ledger,
                    all_indexes(change_set.operations().len())?,
                    "write.verify.postimage_mismatch",
                )?,
            }
        }
        ApplyDisposition::KnownFailure {
            applied_indexes,
            failure_index,
            failure_code,
        } => {
            for index in &applied_indexes {
                let index = usize::try_from(*index)
                    .map_err(|_| WriteTransactionError::DriverReportInvalid)?;
                ledger.transition(
                    index,
                    WriteOperationStatus::Applied,
                    None,
                    request.now_epoch_ms,
                )?;
                ledger.transition(
                    index,
                    WriteOperationStatus::Failed,
                    Some(&failure_code),
                    request.now_epoch_ms,
                )?;
            }
            let failure_index = usize::try_from(failure_index)
                .map_err(|_| WriteTransactionError::DriverReportInvalid)?;
            ledger.transition(
                failure_index,
                WriteOperationStatus::Failed,
                Some(&failure_code),
                request.now_epoch_ms,
            )?;
            for index in (failure_index + 1)..change_set.operations().len() {
                ledger.transition(
                    index,
                    WriteOperationStatus::Superseded,
                    None,
                    request.now_epoch_ms,
                )?;
            }
            if applied_indexes.is_empty() {
                WriteTransactionOutcome::FailedNoChange
            } else {
                restore_after_failure(
                    driver,
                    change_set,
                    &request,
                    &mut ledger,
                    applied_indexes,
                    &failure_code,
                )?
            }
        }
        ApplyDisposition::Uncertain { failure_code } => {
            ledger.fail_every_nonterminal(&failure_code, request.now_epoch_ms)?;
            WriteTransactionOutcome::Uncertain
        }
    };

    let result = WriteTransactionResult {
        transaction_id: request.transaction_id.clone(),
        outcome,
        grant_consumption: consumption,
        receipts: ledger.receipts,
        verification_requirements: change_set
            .permitted_verification()
            .iter()
            .map(|verification_id| SeparateVerificationRequirement {
                verification_id: verification_id.clone(),
                executed: false,
                requires_separate_command_grant: true,
            })
            .collect(),
    };
    verify_write_receipts(&result.receipts)?;
    Ok(result)
}

/// Recomputes one complete per-operation receipt chain from its retained fields.
pub fn verify_write_receipts(
    receipts: &[WriteOperationReceipt],
) -> Result<(), WriteTransactionError> {
    if receipts.is_empty() {
        return Err(WriteTransactionError::ReceiptIntegrity);
    }
    let transaction_id = receipts[0].transaction_id.as_str();
    let grant_id = &receipts[0].grant_id;
    let mut previous = ZERO_SHA256.to_owned();
    let mut states = BTreeMap::<String, WriteOperationStatus>::new();
    let mut identities = BTreeMap::<String, (String, String)>::new();
    for (index, receipt) in receipts.iter().enumerate() {
        if receipt.transaction_id != transaction_id
            || &receipt.grant_id != grant_id
            || receipt.sequence != u32::try_from(index + 1).unwrap_or(u32::MAX)
            || receipt.previous_receipt_sha256 != previous
            || receipt.preimage_sha256.len() != 64
            || receipt.postimage_sha256.len() != 64
        {
            return Err(WriteTransactionError::ReceiptIntegrity);
        }
        let mut candidate = receipt.clone();
        candidate.receipt_sha256 = ZERO_SHA256.to_owned();
        if canonical_sha256(&candidate)? != receipt.receipt_sha256 {
            return Err(WriteTransactionError::ReceiptIntegrity);
        }
        if let Some((preimage, postimage)) = identities.get(&receipt.operation_id) {
            if preimage != &receipt.preimage_sha256 || postimage != &receipt.postimage_sha256 {
                return Err(WriteTransactionError::ReceiptIntegrity);
            }
        } else {
            identities.insert(
                receipt.operation_id.clone(),
                (
                    receipt.preimage_sha256.clone(),
                    receipt.postimage_sha256.clone(),
                ),
            );
        }
        match states.get(&receipt.operation_id).copied() {
            None if receipt.status == WriteOperationStatus::Proposed => {}
            Some(current) if current.allows(receipt.status) => {}
            _ => return Err(WriteTransactionError::ReceiptIntegrity),
        }
        if matches!(receipt.status, WriteOperationStatus::Failed) != receipt.failure_code.is_some()
        {
            return Err(WriteTransactionError::ReceiptIntegrity);
        }
        previous = receipt.receipt_sha256.clone();
        states.insert(receipt.operation_id.clone(), receipt.status);
    }
    if states.values().any(|status| {
        !matches!(
            status,
            WriteOperationStatus::Verified
                | WriteOperationStatus::Failed
                | WriteOperationStatus::RolledBack
                | WriteOperationStatus::Superseded
        )
    }) {
        return Err(WriteTransactionError::ReceiptIntegrity);
    }
    Ok(())
}

/// Exact input for a fresh rollback proposal after a committed transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RollbackProposalRequest {
    /// New rollback change-set identity.
    pub change_set_id: String,
    /// Fresh observation time.
    pub observed_at_epoch_ms: u64,
    /// Complete new rollback review narrative.
    pub review: WriteReviewNarrative,
    /// Exact separately authorized verification labels for the rollback.
    pub permitted_verification: Vec<String>,
}

/// Builds a new authority-free rollback request only when no later target change exists.
pub fn propose_fresh_rollback(
    original: &ShadowChangeSet,
    current: &[CurrentWriteTarget],
    request: RollbackProposalRequest,
) -> Result<ShadowChangeSetRequest, WriteTransactionError> {
    validate_identifier(&request.change_set_id)?;
    if current.len() != original.operations().len()
        || !observations_match_postimages(original, current)
    {
        return Err(WriteTransactionError::PreapplyDenied);
    }
    let operations = original
        .operations()
        .iter()
        .zip(current)
        .map(|(operation, observation)| ShadowWriteDraft {
            operation_id: format!("rollback:{}", operation.operation_id()),
            target: observation.target.clone(),
            observed_bytes: observation.bytes.clone(),
            proposed_bytes: operation.preimage_bytes().to_vec(),
            expected_postimage_sha256: operation.preimage_sha256().to_owned(),
            syntax: operation.syntax(),
            line_endings: operation.line_endings(),
            generated_file: operation.generated_file(),
            allow_generated_file: operation.generated_file(),
        })
        .collect();
    Ok(ShadowChangeSetRequest {
        change_set_id: request.change_set_id,
        observed_at_epoch_ms: request.observed_at_epoch_ms,
        operations,
        review: request.review,
        permitted_verification: request.permitted_verification,
    })
}

enum ApplyDisposition {
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

fn classify_apply_report(
    operation_count: usize,
    report: &WriteApplyReport,
) -> Result<ApplyDisposition, WriteTransactionError> {
    if report.uncertain {
        let code = report
            .failure_code
            .as_deref()
            .unwrap_or(WriteDriverError::Uncertain.code());
        validate_failure_code(code)?;
        return Ok(ApplyDisposition::Uncertain {
            failure_code: code.to_owned(),
        });
    }
    let complete = all_indexes(operation_count)?;
    if report.failure_index.is_none()
        && report.failure_code.is_none()
        && report.applied_indexes == complete
    {
        return Ok(ApplyDisposition::Success);
    }
    let (Some(failure_index), Some(failure_code)) =
        (report.failure_index, report.failure_code.as_deref())
    else {
        return Err(WriteTransactionError::DriverReportInvalid);
    };
    validate_failure_code(failure_code)?;
    let failure = usize::try_from(failure_index)
        .ok()
        .filter(|index| *index < operation_count)
        .ok_or(WriteTransactionError::DriverReportInvalid)?;
    let expected_prefix = all_indexes(failure)?;
    if report.applied_indexes != expected_prefix || (report.atomic && !expected_prefix.is_empty()) {
        return Err(WriteTransactionError::DriverReportInvalid);
    }
    Ok(ApplyDisposition::KnownFailure {
        applied_indexes: report.applied_indexes.clone(),
        failure_index,
        failure_code: failure_code.to_owned(),
    })
}

fn restore_after_failure<D: AtomicWriteDriver>(
    driver: &mut D,
    change_set: &ShadowChangeSet,
    request: &WriteTransactionRequest,
    ledger: &mut WriteReceiptLedger<'_>,
    indexes: Vec<u32>,
    failure_code: &str,
) -> Result<WriteTransactionOutcome, WriteTransactionError> {
    for index in &indexes {
        let index =
            usize::try_from(*index).map_err(|_| WriteTransactionError::DriverReportInvalid)?;
        if ledger.current(index) == Some(WriteOperationStatus::Applied) {
            ledger.transition(
                index,
                WriteOperationStatus::Failed,
                Some(failure_code),
                request.now_epoch_ms,
            )?;
        }
    }
    let report = driver.restore(WriteRestoreAuthorization {
        transaction_id: &request.transaction_id,
        change_set,
        restore_indexes: &indexes,
    });
    if report.uncertain
        || report.failure_code.is_some()
        || report.restored_indexes != indexes
        || driver.observe(change_set).map_or(true, |values| {
            !observations_match_preimages(change_set, &values)
        })
    {
        return Ok(WriteTransactionOutcome::Uncertain);
    }
    for index in indexes {
        ledger.transition(
            usize::try_from(index).map_err(|_| WriteTransactionError::DriverReportInvalid)?,
            WriteOperationStatus::RolledBack,
            None,
            request.now_epoch_ms,
        )?;
    }
    Ok(WriteTransactionOutcome::Restored)
}

fn policy_context(
    grant: &CapabilityGrant,
    now_epoch_ms: u64,
) -> Result<PolicyEvaluationContext, WriteTransactionError> {
    if grant.status != GrantStatus::Issued
        || grant.operation != OperationBinding::new(GrantOperation::WorkspaceWrite)
    {
        return Err(WriteTransactionError::InvalidInput);
    }
    Ok(PolicyEvaluationContext {
        actor_id: grant.actor_id.clone(),
        session_id: grant.session_id.clone(),
        task_id: grant.task_id.clone(),
        action_id: grant
            .action_id
            .clone()
            .ok_or(WriteTransactionError::InvalidInput)?,
        action_kind: grant
            .action_kind
            .ok_or(WriteTransactionError::InvalidInput)?,
        tool_id: grant
            .tool_id
            .clone()
            .ok_or(WriteTransactionError::InvalidInput)?,
        tool_version: grant
            .tool_version
            .clone()
            .ok_or(WriteTransactionError::InvalidInput)?,
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

fn observations_match_postimages(
    change_set: &ShadowChangeSet,
    observations: &[CurrentWriteTarget],
) -> bool {
    observations.len() == change_set.operations().len()
        && observations
            .iter()
            .zip(change_set.operations())
            .all(|(observation, operation)| {
                same_target_path(&observation.target, operation.target())
                    && observation.bytes == operation.proposed_bytes()
                    && target_matches_bytes(&observation.target, &observation.bytes)
                    && hex_sha256(&observation.bytes) == operation.expected_postimage_sha256()
            })
}

fn observations_match_preimages(
    change_set: &ShadowChangeSet,
    observations: &[CurrentWriteTarget],
) -> bool {
    observations.len() == change_set.operations().len()
        && observations
            .iter()
            .zip(change_set.operations())
            .all(|(observation, operation)| {
                same_target_path(&observation.target, operation.target())
                    && observation.bytes == operation.preimage_bytes()
                    && target_matches_bytes(&observation.target, &observation.bytes)
                    && hex_sha256(&observation.bytes) == operation.preimage_sha256()
            })
}

fn same_target_path(left: &GrantTarget, right: &GrantTarget) -> bool {
    left.workspace_id() == right.workspace_id()
        && left.authorization_id() == right.authorization_id()
        && left.adapter_instance_id() == right.adapter_instance_id()
        && left.platform() == right.platform()
        && left.path_components() == right.path_components()
        && left.object_kind() == right.object_kind()
}

fn target_matches_bytes(target: &GrantTarget, bytes: &[u8]) -> bool {
    target.preimage().is_some_and(|preimage| {
        preimage.byte_len() == u64::try_from(bytes.len()).unwrap_or(u64::MAX)
            && hex_bytes(preimage.content_sha256()) == hex_sha256(bytes)
    })
}

fn all_indexes(count: usize) -> Result<Vec<u32>, WriteTransactionError> {
    (0..count)
        .map(|index| u32::try_from(index).map_err(|_| WriteTransactionError::InvalidInput))
        .collect()
}

fn validate_identifier(value: &str) -> Result<(), WriteTransactionError> {
    if value.is_empty()
        || value.len() > MAX_IDENTIFIER_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
    {
        return Err(WriteTransactionError::InvalidInput);
    }
    Ok(())
}

fn validate_failure_code(value: &str) -> Result<(), WriteTransactionError> {
    if value.is_empty()
        || value.len() > MAX_FAILURE_CODE_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
    {
        return Err(WriteTransactionError::DriverReportInvalid);
    }
    Ok(())
}

struct WriteReceiptLedger<'operation> {
    transaction_id: &'operation str,
    operations: &'operation ShadowChangeSet,
    grant_id: &'operation GrantId,
    current: BTreeMap<usize, WriteOperationStatus>,
    receipts: Vec<WriteOperationReceipt>,
}

impl<'operation> WriteReceiptLedger<'operation> {
    fn new(
        transaction_id: &'operation str,
        operations: &'operation ShadowChangeSet,
        grant_id: &'operation GrantId,
        occurred_at_epoch_ms: u64,
    ) -> Result<Self, WriteTransactionError> {
        let mut ledger = Self {
            transaction_id,
            operations,
            grant_id,
            current: BTreeMap::new(),
            receipts: Vec::new(),
        };
        for index in 0..operations.operations().len() {
            ledger.emit(
                index,
                WriteOperationStatus::Proposed,
                None,
                occurred_at_epoch_ms,
            )?;
            ledger.transition(
                index,
                WriteOperationStatus::Approved,
                None,
                occurred_at_epoch_ms,
            )?;
        }
        Ok(ledger)
    }

    fn current(&self, index: usize) -> Option<WriteOperationStatus> {
        self.current.get(&index).copied()
    }

    fn transition(
        &mut self,
        index: usize,
        status: WriteOperationStatus,
        failure_code: Option<&str>,
        occurred_at_epoch_ms: u64,
    ) -> Result<(), WriteTransactionError> {
        let current = self
            .current(index)
            .ok_or(WriteTransactionError::ReceiptIntegrity)?;
        if !current.allows(status)
            || matches!(status, WriteOperationStatus::Failed) != failure_code.is_some()
        {
            return Err(WriteTransactionError::ReceiptIntegrity);
        }
        if let Some(code) = failure_code {
            validate_failure_code(code)?;
        }
        self.emit(index, status, failure_code, occurred_at_epoch_ms)
    }

    fn fail_every_nonterminal(
        &mut self,
        failure_code: &str,
        occurred_at_epoch_ms: u64,
    ) -> Result<(), WriteTransactionError> {
        for index in 0..self.operations.operations().len() {
            if let Some(WriteOperationStatus::Approved | WriteOperationStatus::Applied) =
                self.current(index)
            {
                self.transition(
                    index,
                    WriteOperationStatus::Failed,
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
        status: WriteOperationStatus,
        failure_code: Option<&str>,
        occurred_at_epoch_ms: u64,
    ) -> Result<(), WriteTransactionError> {
        let operation = self
            .operations
            .operations()
            .get(index)
            .ok_or(WriteTransactionError::ReceiptIntegrity)?;
        let sequence = u32::try_from(self.receipts.len() + 1)
            .map_err(|_| WriteTransactionError::ReceiptIntegrity)?;
        let previous = self.receipts.last().map_or_else(
            || ZERO_SHA256.to_owned(),
            |receipt| receipt.receipt_sha256.clone(),
        );
        let mut receipt = WriteOperationReceipt {
            transaction_id: self.transaction_id.to_owned(),
            sequence,
            operation_id: operation.operation_id().to_owned(),
            status,
            preimage_sha256: operation.preimage_sha256().to_owned(),
            postimage_sha256: operation.expected_postimage_sha256().to_owned(),
            grant_id: self.grant_id.clone(),
            occurred_at_epoch_ms,
            failure_code: failure_code.map(str::to_owned),
            previous_receipt_sha256: previous,
            receipt_sha256: ZERO_SHA256.to_owned(),
        };
        receipt.receipt_sha256 = canonical_sha256(&receipt)?;
        self.current.insert(index, status);
        self.receipts.push(receipt);
        Ok(())
    }
}

fn canonical_sha256(value: &impl Serialize) -> Result<String, WriteTransactionError> {
    serde_json::to_vec(value)
        .map(|bytes| hex_sha256(&bytes))
        .map_err(|_| WriteTransactionError::ReceiptIntegrity)
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

    use super::*;
    use crate::{
        grants::SessionReadGrantRequest,
        policy::{PolicyDocument, ScopeRules, ToolPolicyBinding},
        test_target::scope,
        write_approval::{
            ShadowWriteDraft, WriteApprovalDecision, WriteGrantRequest, WriteLineEndings,
            WriteSyntax, build_shadow_change_set, issue_write_grant, render_write_preview,
        },
    };
    use agentmage_kernel_contracts::{
        ActionId, ActionKind, ActorId, ApprovalId, DataSensitivity, GrantNonce, SessionId, TaskId,
        ToolId,
    };

    fn rules<T: Ord>(values: impl IntoIterator<Item = T>) -> ScopeRules<T> {
        ScopeRules {
            allowed: values.into_iter().collect(),
            denied: BTreeSet::new(),
        }
    }

    fn target(path: &[String], bytes: &[u8]) -> GrantTarget {
        let content: [u8; 32] = Sha256::digest(bytes).into();
        let object: [u8; 32] = Sha256::digest(path.join("/").as_bytes()).into();
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
                "object_identity_sha256": object
            },
            "preimage": {"byte_len": bytes.len(), "content_sha256": content}
        }))
        .expect("synthetic exact transaction target")
    }

    fn review(label: &str) -> WriteReviewNarrative {
        WriteReviewNarrative {
            rationale: format!("Apply the exact {label} fixture change"),
            behavior_change: format!("The {label} fixture uses the reviewed value"),
            verification_plan: vec!["Run the focused transaction checks".to_owned()],
            risks: vec!["A fixture consumer may expect the previous value".to_owned()],
            rollback: "Restore every exact reviewed preimage in a new transaction".to_owned(),
            unverified_assumptions: vec!["No external consumer was inspected".to_owned()],
        }
    }

    struct Fixture {
        issuer: GrantIssuer,
        policy: PolicyEngine,
        change_set: ShadowChangeSet,
        approval: WriteApprovalReceipt,
    }

    fn fixture(operation_count: usize) -> Fixture {
        let paths: Vec<Vec<String>> = (0..operation_count)
            .map(|index| vec!["src".to_owned(), format!("fixture-{index}.json")])
            .collect();
        let preimages: Vec<Vec<u8>> = (0..operation_count)
            .map(|index| format!("{{\"value\":{index}}}\n").into_bytes())
            .collect();
        let postimages: Vec<Vec<u8>> = (0..operation_count)
            .map(|index| format!("{{\"value\":{}}}\n", index + 10).into_bytes())
            .collect();
        let targets: Vec<_> = paths
            .iter()
            .zip(&preimages)
            .map(|(path, bytes)| target(path, bytes))
            .collect();
        let action_id = ActionId::from_raw("action-write-transaction");
        let tool_id = ToolId::from_raw("workspace.write");
        let policy = PolicyEngine::new(PolicyDocument {
            schema_version: 1,
            revision: 1,
            actors: rules([ActorId::from_raw("actor-local")]),
            tasks: rules([TaskId::from_raw("task-write")]),
            actions: rules([action_id.clone()]),
            tools: rules([ToolPolicyBinding {
                tool_id: tool_id.clone(),
                tool_version: "1.0.0".to_owned(),
            }]),
            operations: rules([OperationBinding::new(GrantOperation::WorkspaceWrite)]),
            targets: rules(targets.clone()),
            denied_argument_sha256s: BTreeSet::new(),
            denied_preimage_sha256s: BTreeSet::new(),
            network_scopes: ScopeRules::deny_all(),
            credential_scopes: ScopeRules::deny_all(),
            publication_scopes: ScopeRules::deny_all(),
        })
        .expect("exact write policy");
        let mut issuer = GrantIssuer::new();
        let parent = issuer
            .issue_session_read(SessionReadGrantRequest {
                grant_id: GrantId::from_raw("grant-parent-transaction"),
                actor_id: ActorId::from_raw("actor-local"),
                session_id: SessionId::from_raw("session-write"),
                task_id: TaskId::from_raw("task-write"),
                targets: vec![scope(&[])],
                excluded_targets: vec![scope(&["private"])],
                sensitivity: DataSensitivity::Restricted,
                issued_at_epoch_ms: 1_000,
                expires_at_epoch_ms: 100_000,
                nonce: GrantNonce::from_raw("nonce-parent-transaction"),
                maximum_derived_operations: 2,
                preview_sha256: "a".repeat(64),
                policy_sha256: policy.policy_sha256().to_owned(),
            })
            .expect("session parent");
        let operations = targets
            .into_iter()
            .zip(preimages)
            .zip(postimages)
            .enumerate()
            .map(
                |(index, ((target, observed_bytes), proposed_bytes))| ShadowWriteDraft {
                    operation_id: format!("operation-{index}"),
                    target,
                    observed_bytes,
                    expected_postimage_sha256: hex_sha256(&proposed_bytes),
                    proposed_bytes,
                    syntax: WriteSyntax::Json,
                    line_endings: WriteLineEndings::Lf,
                    generated_file: false,
                    allow_generated_file: false,
                },
            )
            .collect();
        let change_set = build_shadow_change_set(
            &parent,
            ShadowChangeSetRequest {
                change_set_id: "change-set-transaction".to_owned(),
                observed_at_epoch_ms: 2_000,
                operations,
                review: review("write"),
                permitted_verification: vec!["cargo-test-write-transaction".to_owned()],
            },
        )
        .expect("shadow transaction");
        let preview = render_write_preview(&change_set).expect("transaction preview");
        let approval = issue_write_grant(
            &mut issuer,
            &change_set,
            &preview,
            &WriteApprovalDecision {
                approval_id: ApprovalId::from_raw("approval-write-transaction"),
                approved_change_set_sha256: preview.change_set_sha256.clone(),
                approved_preview_sha256: preview.preview_sha256.clone(),
                approved_at_epoch_ms: 3_000,
                expires_at_epoch_ms: 30_000,
                permitted_verification: preview.permitted_verification.clone(),
                user_confirmed: true,
            },
            WriteGrantRequest {
                parent_grant_id: parent.grant_id,
                grant_id: GrantId::from_raw("grant-write-transaction"),
                action_id,
                action_kind: ActionKind::DeterministicTool,
                tool_id,
                tool_version: "1.0.0".to_owned(),
                nonce: GrantNonce::from_raw("nonce-write-transaction"),
                policy_sha256: policy.policy_sha256().to_owned(),
            },
        )
        .expect("write grant");
        Fixture {
            issuer,
            policy,
            change_set,
            approval,
        }
    }

    #[derive(Clone, Copy)]
    enum DriverMode {
        Success,
        FailAt(usize),
        CorruptPostimage,
        Uncertain,
        MalformedAtomicPartial,
        RestoreFails,
    }

    struct MemoryDriver {
        paths: Vec<Vec<String>>,
        bytes: Vec<Vec<u8>>,
        mode: DriverMode,
        apply_calls: usize,
        restore_calls: usize,
    }

    impl MemoryDriver {
        fn new(change_set: &ShadowChangeSet, mode: DriverMode) -> Self {
            Self {
                paths: change_set
                    .operations()
                    .iter()
                    .map(|operation| {
                        operation
                            .target()
                            .path_components()
                            .iter()
                            .map(|component| component.as_str().to_owned())
                            .collect()
                    })
                    .collect(),
                bytes: change_set
                    .operations()
                    .iter()
                    .map(|operation| operation.preimage_bytes().to_vec())
                    .collect(),
                mode,
                apply_calls: 0,
                restore_calls: 0,
            }
        }

        fn observations(&self) -> Vec<CurrentWriteTarget> {
            self.paths
                .iter()
                .zip(&self.bytes)
                .map(|(path, bytes)| CurrentWriteTarget {
                    target: target(path, bytes),
                    bytes: bytes.clone(),
                })
                .collect()
        }
    }

    impl AtomicWriteDriver for MemoryDriver {
        fn observe(
            &mut self,
            _change_set: &ShadowChangeSet,
        ) -> Result<Vec<CurrentWriteTarget>, WriteDriverError> {
            Ok(self.observations())
        }

        fn apply(&mut self, authorization: WriteApplyAuthorization<'_>) -> WriteApplyReport {
            self.apply_calls += 1;
            let operations = authorization.change_set().operations();
            match self.mode {
                DriverMode::Success | DriverMode::RestoreFails => {
                    for (bytes, operation) in self.bytes.iter_mut().zip(operations) {
                        *bytes = operation.proposed_bytes().to_vec();
                    }
                    if matches!(self.mode, DriverMode::RestoreFails) {
                        self.bytes[0].push(b' ');
                    }
                    WriteApplyReport {
                        atomic: true,
                        applied_indexes: all_indexes(operations.len()).expect("bounded indexes"),
                        failure_index: None,
                        failure_code: None,
                        uncertain: false,
                    }
                }
                DriverMode::FailAt(failure) => {
                    for (bytes, operation) in self.bytes.iter_mut().zip(operations).take(failure) {
                        *bytes = operation.proposed_bytes().to_vec();
                    }
                    WriteApplyReport {
                        atomic: false,
                        applied_indexes: all_indexes(failure).expect("bounded prefix"),
                        failure_index: Some(u32::try_from(failure).expect("bounded failure")),
                        failure_code: Some("write.driver.injected_failure".to_owned()),
                        uncertain: false,
                    }
                }
                DriverMode::CorruptPostimage => {
                    for (bytes, operation) in self.bytes.iter_mut().zip(operations) {
                        *bytes = operation.proposed_bytes().to_vec();
                    }
                    self.bytes[0].push(b' ');
                    WriteApplyReport {
                        atomic: true,
                        applied_indexes: all_indexes(operations.len()).expect("bounded indexes"),
                        failure_index: None,
                        failure_code: None,
                        uncertain: false,
                    }
                }
                DriverMode::Uncertain => {
                    self.bytes[0] = operations[0].proposed_bytes().to_vec();
                    WriteApplyReport {
                        atomic: false,
                        applied_indexes: Vec::new(),
                        failure_index: None,
                        failure_code: Some(WriteDriverError::Uncertain.code().to_owned()),
                        uncertain: true,
                    }
                }
                DriverMode::MalformedAtomicPartial => WriteApplyReport {
                    atomic: true,
                    applied_indexes: vec![0],
                    failure_index: Some(1),
                    failure_code: Some("write.driver.invalid_atomic_claim".to_owned()),
                    uncertain: false,
                },
            }
        }

        fn restore(&mut self, authorization: WriteRestoreAuthorization<'_>) -> WriteRestoreReport {
            self.restore_calls += 1;
            if matches!(self.mode, DriverMode::RestoreFails) {
                return WriteRestoreReport {
                    restored_indexes: Vec::new(),
                    failure_code: Some(WriteDriverError::RestoreFailed.code().to_owned()),
                    uncertain: true,
                };
            }
            for index in authorization.restore_indexes() {
                let index = usize::try_from(*index).expect("bounded restore index");
                self.bytes[index] = authorization.change_set().operations()[index]
                    .preimage_bytes()
                    .to_vec();
            }
            WriteRestoreReport {
                restored_indexes: authorization.restore_indexes().to_vec(),
                failure_code: None,
                uncertain: false,
            }
        }
    }

    fn execute(fixture: &mut Fixture, driver: &mut MemoryDriver) -> WriteTransactionResult {
        execute_write_transaction(
            &mut fixture.issuer,
            &fixture.policy,
            &fixture.change_set,
            &fixture.approval,
            WriteTransactionRequest {
                transaction_id: "write-transaction-0001".to_owned(),
                now_epoch_ms: 4_000,
            },
            driver,
        )
        .expect("reconciled transaction result")
    }

    #[test]
    fn successful_atomic_write_consumes_once_verifies_and_requires_separate_checks() {
        let mut fixture = fixture(3);
        let mut driver = MemoryDriver::new(&fixture.change_set, DriverMode::Success);
        let result = execute(&mut fixture, &mut driver);

        assert_eq!(result.outcome, WriteTransactionOutcome::Committed);
        assert_eq!(driver.apply_calls, 1);
        assert_eq!(driver.restore_calls, 0);
        assert!(observations_match_postimages(
            &fixture.change_set,
            &driver.observations()
        ));
        assert_eq!(
            fixture
                .issuer
                .current(&fixture.approval.grant.grant_id)
                .map(|grant| grant.status),
            Some(GrantStatus::Consumed)
        );
        assert!(
            result
                .receipts
                .iter()
                .any(|receipt| { receipt.status == WriteOperationStatus::Verified })
        );
        assert_eq!(
            result.verification_requirements,
            [SeparateVerificationRequirement {
                verification_id: "cargo-test-write-transaction".to_owned(),
                executed: false,
                requires_separate_command_grant: true,
            }]
        );
        assert_eq!(verify_write_receipts(&result.receipts), Ok(()));
    }

    #[test]
    fn ordered_partial_failure_restores_changed_prefix_and_supersedes_later_work() {
        let mut fixture = fixture(3);
        let mut driver = MemoryDriver::new(&fixture.change_set, DriverMode::FailAt(1));
        let result = execute(&mut fixture, &mut driver);

        assert_eq!(result.outcome, WriteTransactionOutcome::Restored);
        assert_eq!(driver.restore_calls, 1);
        assert!(observations_match_preimages(
            &fixture.change_set,
            &driver.observations()
        ));
        assert!(result.receipts.iter().any(|receipt| {
            receipt.operation_id == "operation-0"
                && receipt.status == WriteOperationStatus::RolledBack
        }));
        assert!(result.receipts.iter().any(|receipt| {
            receipt.operation_id == "operation-1" && receipt.status == WriteOperationStatus::Failed
        }));
        assert!(result.receipts.iter().any(|receipt| {
            receipt.operation_id == "operation-2"
                && receipt.status == WriteOperationStatus::Superseded
        }));
    }

    #[test]
    fn preeffect_failure_changes_nothing_and_postimage_mismatch_restores_everything() {
        let mut no_change = fixture(3);
        let mut failed_driver = MemoryDriver::new(&no_change.change_set, DriverMode::FailAt(0));
        let failed = execute(&mut no_change, &mut failed_driver);
        assert_eq!(failed.outcome, WriteTransactionOutcome::FailedNoChange);
        assert_eq!(failed_driver.restore_calls, 0);
        assert!(observations_match_preimages(
            &no_change.change_set,
            &failed_driver.observations()
        ));

        let mut mismatch = fixture(3);
        let mut corrupt_driver =
            MemoryDriver::new(&mismatch.change_set, DriverMode::CorruptPostimage);
        let restored = execute(&mut mismatch, &mut corrupt_driver);
        assert_eq!(restored.outcome, WriteTransactionOutcome::Restored);
        assert!(observations_match_preimages(
            &mismatch.change_set,
            &corrupt_driver.observations()
        ));
    }

    #[test]
    fn uncertain_malformed_and_failed_restoration_never_claim_commit_or_retry() {
        for mode in [
            DriverMode::Uncertain,
            DriverMode::MalformedAtomicPartial,
            DriverMode::RestoreFails,
        ] {
            let mut fixture = fixture(3);
            let mut driver = MemoryDriver::new(&fixture.change_set, mode);
            let result = execute(&mut fixture, &mut driver);
            assert_eq!(result.outcome, WriteTransactionOutcome::Uncertain);
            assert_eq!(
                fixture
                    .issuer
                    .current(&fixture.approval.grant.grant_id)
                    .map(|grant| grant.status),
                Some(GrantStatus::Consumed)
            );
        }
    }

    #[test]
    fn changed_preimage_invalidates_before_driver_apply() {
        let mut fixture = fixture(2);
        let mut driver = MemoryDriver::new(&fixture.change_set, DriverMode::Success);
        driver.bytes[0].push(b' ');
        assert_eq!(
            execute_write_transaction(
                &mut fixture.issuer,
                &fixture.policy,
                &fixture.change_set,
                &fixture.approval,
                WriteTransactionRequest {
                    transaction_id: "write-transaction-stale".to_owned(),
                    now_epoch_ms: 4_000,
                },
                &mut driver,
            ),
            Err(WriteTransactionError::PreapplyDenied)
        );
        assert_eq!(driver.apply_calls, 0);
        assert_eq!(
            fixture
                .issuer
                .current(&fixture.approval.grant.grant_id)
                .map(|grant| grant.status),
            Some(GrantStatus::Invalidated)
        );
    }

    #[test]
    fn rollback_is_a_fresh_reversed_request_and_later_user_change_refuses_it() {
        let fixture = fixture(2);
        let mut driver = MemoryDriver::new(&fixture.change_set, DriverMode::Success);
        for (bytes, operation) in driver.bytes.iter_mut().zip(fixture.change_set.operations()) {
            *bytes = operation.proposed_bytes().to_vec();
        }
        let current = driver.observations();
        let rollback = propose_fresh_rollback(
            &fixture.change_set,
            &current,
            RollbackProposalRequest {
                change_set_id: "change-set-rollback".to_owned(),
                observed_at_epoch_ms: 5_000,
                review: review("rollback"),
                permitted_verification: vec!["cargo-test-rollback".to_owned()],
            },
        )
        .expect("fresh rollback request");
        assert_eq!(
            rollback.operations[0].proposed_bytes,
            fixture.change_set.operations()[0].preimage_bytes()
        );
        assert_eq!(
            rollback.operations[0].observed_bytes,
            fixture.change_set.operations()[0].proposed_bytes()
        );

        driver.bytes[0].push(b' ');
        assert_eq!(
            propose_fresh_rollback(
                &fixture.change_set,
                &driver.observations(),
                RollbackProposalRequest {
                    change_set_id: "change-set-rollback-stale".to_owned(),
                    observed_at_epoch_ms: 6_000,
                    review: review("rollback"),
                    permitted_verification: Vec::new(),
                },
            ),
            Err(WriteTransactionError::PreapplyDenied)
        );
    }

    #[test]
    fn transition_matrix_and_receipt_tampering_fail_closed() {
        for current in WriteOperationStatus::ALL {
            for next in WriteOperationStatus::ALL {
                let expected = matches!(
                    (current, next),
                    (
                        WriteOperationStatus::Proposed,
                        WriteOperationStatus::Approved | WriteOperationStatus::Superseded
                    ) | (
                        WriteOperationStatus::Approved,
                        WriteOperationStatus::Applied
                            | WriteOperationStatus::Failed
                            | WriteOperationStatus::Superseded
                    ) | (
                        WriteOperationStatus::Applied,
                        WriteOperationStatus::Verified | WriteOperationStatus::Failed
                    ) | (
                        WriteOperationStatus::Failed,
                        WriteOperationStatus::RolledBack
                    )
                );
                assert_eq!(current.allows(next), expected);
            }
        }

        let mut fixture = fixture(2);
        let mut driver = MemoryDriver::new(&fixture.change_set, DriverMode::Success);
        let result = execute(&mut fixture, &mut driver);
        let mut changed = result.receipts.clone();
        changed[0].postimage_sha256 = "f".repeat(64);
        assert_eq!(
            verify_write_receipts(&changed),
            Err(WriteTransactionError::ReceiptIntegrity)
        );
        assert_eq!(
            verify_write_receipts(&result.receipts[..1]),
            Err(WriteTransactionError::ReceiptIntegrity)
        );
        let mut reordered = result.receipts;
        reordered.swap(0, 1);
        assert_eq!(
            verify_write_receipts(&reordered),
            Err(WriteTransactionError::ReceiptIntegrity)
        );
    }
}
