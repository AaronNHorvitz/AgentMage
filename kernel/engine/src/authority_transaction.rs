//! Kernel-owned authority-transaction ordering and recovery.

use std::{collections::BTreeMap, fmt, fmt::Write as _};

use agentmage_kernel_contracts::{
    ApprovalId, AuthorityTransactionId, AuthorityTransactionRecord, AuthorityTransactionState,
    ContractError, ErrorCategory, ErrorId, GrantId, OperationAttemptId, OperationBinding,
    OperationOutcome, Receipt, ReceiptId, RetryDisposition, StateChange, ToolCall,
    to_canonical_json,
};
use sha2::{Digest, Sha256};

use crate::{
    grants::GrantIssuer,
    policy::{PolicyEngine, PolicyEvaluationContext},
    tooling::ToolRegistry,
};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Stable reason an authority transaction cannot advance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorityTransactionError {
    /// Transaction or attempt identity is malformed or already retained.
    InvalidIdentity,
    /// The proposed tool call is not exactly registered and valid.
    InvalidToolCall,
    /// An internal transaction transition violated the fixed ordering contract.
    InvalidTransition,
    /// Canonical hashing or receipt construction failed before state changed.
    IntegrityFailure,
    /// A deterministic test interruption stopped the transaction at one write boundary.
    SimulatedCrash,
}

impl AuthorityTransactionError {
    /// Returns a stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidIdentity => "authority.transaction.identity_invalid",
            Self::InvalidToolCall => "authority.transaction.tool_invalid",
            Self::InvalidTransition => "authority.transaction.transition_invalid",
            Self::IntegrityFailure => "authority.transaction.integrity_failed",
            Self::SimulatedCrash => "authority.transaction.simulated_crash",
        }
    }
}

/// Exact production request for one kernel-owned authority transaction.
///
/// Fields are private so callers cannot select test-only cancellation or fault
/// boundaries. Construction validates identity shape and the call/context
/// identity binding before any transaction state can be retained.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorityTransactionRequest {
    transaction_id: AuthorityTransactionId,
    attempt_id: OperationAttemptId,
    approval_id: ApprovalId,
    grant_id: GrantId,
    call: ToolCall,
    context: PolicyEvaluationContext,
    cancellation: CancellationPoint,
    occurred_at_epoch_ms: u64,
    occurred_at: String,
}

impl AuthorityTransactionRequest {
    /// Creates one exact non-cancellable production transaction request.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        transaction_id: AuthorityTransactionId,
        attempt_id: OperationAttemptId,
        approval_id: ApprovalId,
        grant_id: GrantId,
        call: ToolCall,
        context: PolicyEvaluationContext,
        occurred_at_epoch_ms: u64,
        occurred_at: impl Into<String>,
    ) -> Result<Self, AuthorityTransactionError> {
        validate_identifier(transaction_id.as_str())?;
        validate_identifier(attempt_id.as_str())?;
        validate_identifier(approval_id.as_str())?;
        validate_identifier(grant_id.as_str())?;
        if call.action_id != context.action_id
            || call.tool_id != context.tool_id
            || call.tool_version != context.tool_version
        {
            return Err(AuthorityTransactionError::InvalidToolCall);
        }
        let occurred_at = occurred_at.into();
        if occurred_at.is_empty() || occurred_at.len() > 64 {
            return Err(AuthorityTransactionError::InvalidIdentity);
        }
        Ok(Self {
            transaction_id,
            attempt_id,
            approval_id,
            grant_id,
            call,
            context,
            cancellation: CancellationPoint::None,
            occurred_at_epoch_ms,
            occurred_at,
        })
    }
}

/// Kernel-issued proof that one exact grant has been consumed for one attempt.
///
/// The permit has no public constructor, does not implement `Clone`, `Copy`,
/// serialization, or deserialization, and is consumed by [`EffectDriver`]. It
/// borrows the transaction request and therefore cannot outlive the coordinator
/// call that issued it.
///
/// A caller cannot forge a permit:
///
/// ```compile_fail
/// use agentmage_kernel_engine::authority_transaction::EffectAuthorization;
/// let _ = EffectAuthorization::new();
/// ```
///
/// A permit cannot be duplicated:
///
/// ```compile_fail
/// use agentmage_kernel_engine::authority_transaction::EffectAuthorization;
/// fn duplicate(permit: EffectAuthorization<'_>) {
///     let _copy = permit.clone();
/// }
/// ```
///
/// A consumed permit cannot be reused:
///
/// ```compile_fail
/// use agentmage_kernel_engine::authority_transaction::{EffectAuthorization, EffectDriver};
/// fn reuse(driver: &mut impl EffectDriver, permit: EffectAuthorization<'_>) {
///     let _ = driver.execute(permit);
///     let _ = driver.execute(permit);
/// }
/// ```
pub struct EffectAuthorization<'transaction> {
    transaction_id: &'transaction AuthorityTransactionId,
    attempt_id: &'transaction OperationAttemptId,
    consumed_grant_sha256: &'transaction str,
    operation: OperationBinding,
    call: &'transaction ToolCall,
}

impl EffectAuthorization<'_> {
    /// Returns the exact authority-transaction identity.
    #[must_use]
    pub const fn transaction_id(&self) -> &AuthorityTransactionId {
        self.transaction_id
    }

    /// Returns the exact non-replayable operation-attempt identity.
    #[must_use]
    pub const fn attempt_id(&self) -> &OperationAttemptId {
        self.attempt_id
    }

    /// Returns the digest of the exact consumed grant record.
    #[must_use]
    pub const fn consumed_grant_sha256(&self) -> &str {
        self.consumed_grant_sha256
    }

    /// Returns the one canonical operation authorized for this attempt.
    #[must_use]
    pub const fn operation(&self) -> OperationBinding {
        self.operation
    }

    /// Returns the exact validated tool call bound to this attempt.
    #[must_use]
    pub const fn call(&self) -> &ToolCall {
        self.call
    }
}

impl fmt::Debug for EffectAuthorization<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EffectAuthorization")
            .field("transaction_id", self.transaction_id)
            .field("attempt_id", self.attempt_id)
            .field("operation", &self.operation)
            .field("tool_id", &self.call.tool_id)
            .field("tool_call_id", &self.call.tool_call_id)
            .finish_non_exhaustive()
    }
}

/// One bounded effect result returned to the authority-transaction reconciler.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectResult {
    outcome: OperationOutcome,
    result_sha256: String,
    state_change: StateChange,
}

impl EffectResult {
    /// Creates a result whose identity is the digest of bounded redacted material.
    #[must_use]
    pub fn from_redacted_material(
        outcome: OperationOutcome,
        material: &[u8],
        state_change: StateChange,
    ) -> Self {
        Self {
            outcome,
            result_sha256: sha256_hex(material),
            state_change,
        }
    }

    /// Returns the terminal operation outcome reported by the driver.
    #[must_use]
    pub const fn outcome(&self) -> OperationOutcome {
        self.outcome
    }

    /// Returns the content-free result identity.
    #[must_use]
    pub fn result_sha256(&self) -> &str {
        &self.result_sha256
    }

    /// Returns the driver's conservative state-change classification.
    #[must_use]
    pub const fn state_change(&self) -> StateChange {
        self.state_change
    }
}

/// Result of crossing one mediated platform or capability effect boundary.
#[derive(Debug, Default)]
pub struct EffectLaunch {
    results: Option<Vec<EffectResult>>,
}

impl EffectLaunch {
    /// Reports that the effect boundary was not crossed.
    #[must_use]
    pub const fn failed() -> Self {
        Self { results: None }
    }

    /// Reports one completed, bounded effect result.
    #[must_use]
    pub fn completed(result: EffectResult) -> Self {
        Self {
            results: Some(vec![result]),
        }
    }

    #[cfg(test)]
    fn completed_results(results: Vec<EffectResult>) -> Self {
        Self {
            results: Some(results),
        }
    }
}

/// Effect adapter invoked only after the kernel records and consumes authority.
///
/// The authorization is passed by value. Implementations cannot launch through
/// this interface without receiving the opaque permit from
/// [`AuthorityTransactionCoordinator::execute_effect`].
pub trait EffectDriver {
    /// Performs the one exact effect described by the consumed authorization.
    fn execute(&mut self, authorization: EffectAuthorization<'_>) -> EffectLaunch;
}

/// In-memory Phase 4 owner of transaction revisions and terminal receipts.
///
/// The coordinator exposes only the mediated effect boundary. Phase 7 must
/// replace this in-memory journal with one crash-durable encrypted transaction.
#[derive(Debug, Default)]
pub struct AuthorityTransactionCoordinator {
    histories: BTreeMap<AuthorityTransactionId, Vec<AuthorityTransactionRecord>>,
    receipts: Vec<Receipt>,
}

impl AuthorityTransactionCoordinator {
    /// Creates an empty coordinator with no transaction, grant, or launch authority.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the current revision for one exact transaction identity.
    #[must_use]
    pub fn current(
        &self,
        transaction_id: &AuthorityTransactionId,
    ) -> Option<&AuthorityTransactionRecord> {
        self.histories
            .get(transaction_id)
            .and_then(|history| history.last())
    }

    /// Returns the immutable revision history for one exact transaction.
    #[must_use]
    pub fn history(
        &self,
        transaction_id: &AuthorityTransactionId,
    ) -> Option<&[AuthorityTransactionRecord]> {
        self.histories.get(transaction_id).map(Vec::as_slice)
    }

    /// Returns retained terminal receipts in sequence order.
    #[must_use]
    pub fn receipts(&self) -> &[Receipt] {
        &self.receipts
    }

    /// Validates, consumes, records, and executes one exact effect transaction.
    pub fn execute_effect<D: EffectDriver>(
        &mut self,
        registry: &ToolRegistry,
        issuer: &mut GrantIssuer,
        policy: &PolicyEngine,
        request: AuthorityTransactionRequest,
        driver: &mut D,
    ) -> Result<Receipt, AuthorityTransactionError> {
        self.run(registry, issuer, policy, &request, driver, None)
    }

    fn run<D: EffectDriver>(
        &mut self,
        registry: &ToolRegistry,
        issuer: &mut GrantIssuer,
        policy: &PolicyEngine,
        request: &AuthorityTransactionRequest,
        driver: &mut D,
        fault: Option<FaultPoint>,
    ) -> Result<Receipt, AuthorityTransactionError> {
        validate_identifier(request.transaction_id.as_str())?;
        validate_identifier(request.attempt_id.as_str())?;
        validate_identifier(request.approval_id.as_str())?;
        validate_identifier(request.grant_id.as_str())?;
        if self.histories.contains_key(&request.transaction_id) {
            return Err(AuthorityTransactionError::InvalidIdentity);
        }
        let definition = registry
            .validate_arguments(&request.call)
            .map_err(|_| AuthorityTransactionError::InvalidToolCall)?;
        let prepared = AuthorityTransactionRecord {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            authority_transaction_id: request.transaction_id.clone(),
            revision: 1,
            operation_attempt_id: request.attempt_id.clone(),
            approval_id: request.approval_id.clone(),
            grant_id: request.grant_id.clone(),
            operation: definition.required_grant.operation,
            correlation_id: request.call.correlation_id.clone(),
            session_id: request.context.session_id.clone(),
            task_id: request.context.task_id.clone(),
            action_id: request.call.action_id.clone(),
            tool_call_id: request.call.tool_call_id.clone(),
            state: AuthorityTransactionState::Prepared,
            consumed_grant_sha256: None,
            result_sha256: None,
            outcome: None,
            uncertain_effect: false,
            receipt_id: None,
            receipt_sha256: None,
            occurred_at: request.occurred_at.clone(),
        };
        self.append_initial(prepared)?;
        if fault == Some(FaultPoint::PreparedStored) {
            return Err(AuthorityTransactionError::SimulatedCrash);
        }
        if request.cancellation == CancellationPoint::BeforeConsume {
            return self.terminalize(
                &request.transaction_id,
                OperationOutcome::Cancelled,
                stable_result_sha256("cancelled-before-consume"),
                false,
                "authority.transaction.cancelled_before_consume",
            );
        }

        let Some(grant) = issuer.current(&request.grant_id) else {
            return self.terminalize(
                &request.transaction_id,
                OperationOutcome::Denied,
                stable_result_sha256("grant-not-found"),
                false,
                "authority.transaction.grant_not_found",
            );
        };
        if grant.approval_id.as_ref() != Some(&request.approval_id)
            || grant.operation != definition.required_grant.operation
            || grant.tool_id.as_ref() != Some(&request.call.tool_id)
            || grant.tool_version.as_deref() != Some(request.call.tool_version.as_str())
            || grant.action_id.as_ref() != Some(&request.call.action_id)
        {
            return self.terminalize(
                &request.transaction_id,
                OperationOutcome::Denied,
                stable_result_sha256("authority-binding-mismatch"),
                false,
                "authority.transaction.binding_mismatch",
            );
        }

        let consumed =
            match issuer.consume_for_execution(&request.grant_id, policy, &request.context) {
                Ok(consumed) => consumed,
                Err(error) => {
                    return self.terminalize(
                        &request.transaction_id,
                        OperationOutcome::Denied,
                        stable_result_sha256(error.code()),
                        false,
                        error.code(),
                    );
                }
            };
        self.append_state(
            &request.transaction_id,
            AuthorityTransactionState::GrantConsumed,
            Some(consumed.consumed_grant_sha256.clone()),
            None,
            None,
            false,
        )?;
        if fault == Some(FaultPoint::GrantConsumed) {
            return Err(AuthorityTransactionError::SimulatedCrash);
        }

        self.append_state(
            &request.transaction_id,
            AuthorityTransactionState::AttemptRecorded,
            None,
            None,
            None,
            false,
        )?;
        if fault == Some(FaultPoint::AttemptRecorded) {
            return Err(AuthorityTransactionError::SimulatedCrash);
        }
        if request.cancellation == CancellationPoint::BeforeLaunch {
            return self.terminalize(
                &request.transaction_id,
                OperationOutcome::Cancelled,
                stable_result_sha256("cancelled-before-launch"),
                false,
                "authority.transaction.cancelled_before_launch",
            );
        }

        self.append_state(
            &request.transaction_id,
            AuthorityTransactionState::LaunchCommitted,
            None,
            None,
            None,
            false,
        )?;
        if fault == Some(FaultPoint::LaunchBoundaryCommitted) {
            return Err(AuthorityTransactionError::SimulatedCrash);
        }

        let authorization = EffectAuthorization {
            transaction_id: &request.transaction_id,
            attempt_id: &request.attempt_id,
            consumed_grant_sha256: &consumed.consumed_grant_sha256,
            operation: definition.required_grant.operation,
            call: &request.call,
        };
        let launched = driver.execute(authorization);
        if fault == Some(FaultPoint::WorkerReturned) {
            return Err(AuthorityTransactionError::SimulatedCrash);
        }
        let results = match launched.results {
            None => {
                return self.terminalize(
                    &request.transaction_id,
                    OperationOutcome::Failed,
                    stable_result_sha256("worker-launch-failed"),
                    false,
                    "authority.transaction.launch_failed",
                );
            }
            Some(results) => results,
        };
        let result = if request.cancellation == CancellationPoint::AfterLaunch {
            EffectResult::from_redacted_material(
                OperationOutcome::Cancelled,
                b"cancelled-after-launch",
                StateChange::Uncertain,
            )
        } else {
            reconcile_results(&results)
        };
        let uncertain = result.state_change == StateChange::Uncertain
            || result.outcome == OperationOutcome::Uncertain;
        self.append_state(
            &request.transaction_id,
            AuthorityTransactionState::Reconciling,
            None,
            Some(result.result_sha256.clone()),
            None,
            uncertain,
        )?;
        if fault == Some(FaultPoint::ResultReconciled) {
            return Err(AuthorityTransactionError::SimulatedCrash);
        }
        if uncertain {
            issuer
                .mark_execution_uncertain(
                    &request.grant_id,
                    &consumed.consumed_grant_sha256,
                    request.occurred_at_epoch_ms,
                )
                .map_err(|_| AuthorityTransactionError::InvalidTransition)?;
        }
        self.terminalize(
            &request.transaction_id,
            result.outcome,
            result.result_sha256,
            uncertain,
            outcome_code(result.outcome, uncertain),
        )
    }

    #[cfg(test)]
    fn recover(
        &mut self,
        issuer: &mut GrantIssuer,
        transaction_id: &AuthorityTransactionId,
        occurred_at_epoch_ms: u64,
    ) -> Result<Receipt, AuthorityTransactionError> {
        let current = self
            .current(transaction_id)
            .cloned()
            .ok_or(AuthorityTransactionError::InvalidIdentity)?;
        match current.state {
            AuthorityTransactionState::Prepared => self.terminalize(
                transaction_id,
                OperationOutcome::Failed,
                stable_result_sha256("recovered-before-consume"),
                false,
                "authority.transaction.recovered_before_consume",
            ),
            AuthorityTransactionState::GrantConsumed
            | AuthorityTransactionState::AttemptRecorded => self.terminalize(
                transaction_id,
                OperationOutcome::Failed,
                stable_result_sha256("recovered-before-launch"),
                false,
                "authority.transaction.recovered_before_launch",
            ),
            AuthorityTransactionState::LaunchCommitted | AuthorityTransactionState::Reconciling => {
                let consumed = current
                    .consumed_grant_sha256
                    .as_deref()
                    .ok_or(AuthorityTransactionError::InvalidTransition)?;
                issuer
                    .mark_execution_uncertain(&current.grant_id, consumed, occurred_at_epoch_ms)
                    .map_err(|_| AuthorityTransactionError::InvalidTransition)?;
                self.terminalize(
                    transaction_id,
                    OperationOutcome::Uncertain,
                    stable_result_sha256("recovered-after-launch"),
                    true,
                    "authority.transaction.recovered_uncertain",
                )
            }
            AuthorityTransactionState::Terminal => self
                .receipts
                .iter()
                .find(|receipt| {
                    current.receipt_id.as_ref() == Some(&receipt.receipt_id)
                        && current.receipt_sha256.as_deref()
                            == Some(receipt.receipt_sha256.as_str())
                })
                .cloned()
                .ok_or(AuthorityTransactionError::IntegrityFailure),
        }
    }

    fn append_initial(
        &mut self,
        record: AuthorityTransactionRecord,
    ) -> Result<(), AuthorityTransactionError> {
        if record.revision != 1
            || record.state != AuthorityTransactionState::Prepared
            || self
                .histories
                .contains_key(&record.authority_transaction_id)
        {
            return Err(AuthorityTransactionError::InvalidTransition);
        }
        self.histories
            .insert(record.authority_transaction_id.clone(), vec![record]);
        Ok(())
    }

    fn append_state(
        &mut self,
        transaction_id: &AuthorityTransactionId,
        state: AuthorityTransactionState,
        consumed_grant_sha256: Option<String>,
        result_sha256: Option<String>,
        outcome: Option<OperationOutcome>,
        uncertain_effect: bool,
    ) -> Result<(), AuthorityTransactionError> {
        let current = self
            .current(transaction_id)
            .cloned()
            .ok_or(AuthorityTransactionError::InvalidTransition)?;
        if !valid_transition(current.state, state) {
            return Err(AuthorityTransactionError::InvalidTransition);
        }
        let mut next = current;
        next.revision = next
            .revision
            .checked_add(1)
            .ok_or(AuthorityTransactionError::IntegrityFailure)?;
        next.state = state;
        if consumed_grant_sha256.is_some() {
            next.consumed_grant_sha256 = consumed_grant_sha256;
        }
        if result_sha256.is_some() {
            next.result_sha256 = result_sha256;
        }
        next.outcome = outcome;
        next.uncertain_effect = uncertain_effect;
        self.histories
            .get_mut(transaction_id)
            .ok_or(AuthorityTransactionError::InvalidTransition)?
            .push(next);
        Ok(())
    }

    fn terminalize(
        &mut self,
        transaction_id: &AuthorityTransactionId,
        outcome: OperationOutcome,
        result_sha256: String,
        uncertain_effect: bool,
        error_code: &str,
    ) -> Result<Receipt, AuthorityTransactionError> {
        let current = self
            .current(transaction_id)
            .cloned()
            .ok_or(AuthorityTransactionError::InvalidTransition)?;
        if !valid_transition(current.state, AuthorityTransactionState::Terminal) {
            return Err(AuthorityTransactionError::InvalidTransition);
        }
        let sequence = u64::try_from(self.receipts.len())
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or(AuthorityTransactionError::IntegrityFailure)?;
        let receipt_id = ReceiptId::from_raw(format!(
            "receipt-{}",
            current.authority_transaction_id.as_str()
        ));
        let previous_receipt_sha256 = self.receipts.last().map_or_else(
            || ZERO_SHA256.to_owned(),
            |value| value.receipt_sha256.clone(),
        );
        let error = if outcome == OperationOutcome::Succeeded {
            None
        } else {
            Some(ContractError {
                schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                error_id: ErrorId::from_raw(format!(
                    "error-{}",
                    current.authority_transaction_id.as_str()
                )),
                code: error_code.to_owned(),
                category: if outcome == OperationOutcome::Denied {
                    ErrorCategory::Policy
                } else {
                    ErrorCategory::Dependency
                },
                message: "Authority transaction reached a non-success terminal outcome".to_owned(),
                field_path: Vec::new(),
                retry: RetryDisposition::Never,
                caused_by: None,
            })
        };
        let mut receipt = Receipt {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            receipt_id: receipt_id.clone(),
            sequence,
            correlation_id: current.correlation_id.clone(),
            authority_transaction_id: current.authority_transaction_id.clone(),
            operation_attempt_id: current.operation_attempt_id.clone(),
            approval_id: current.approval_id.clone(),
            grant_id: current.grant_id.clone(),
            session_id: current.session_id.clone(),
            task_id: current.task_id.clone(),
            action_id: current.action_id.clone(),
            tool_call_id: Some(current.tool_call_id.clone()),
            operation: current.operation,
            outcome,
            operation_sha256: operation_sha256(current.operation)?,
            evidence: Vec::new(),
            error,
            previous_receipt_sha256,
            receipt_sha256: ZERO_SHA256.to_owned(),
            occurred_at: current.occurred_at.clone(),
        };
        receipt.receipt_sha256 = receipt_sha256(&receipt)?;

        let mut terminal = current;
        terminal.revision = terminal
            .revision
            .checked_add(1)
            .ok_or(AuthorityTransactionError::IntegrityFailure)?;
        terminal.state = AuthorityTransactionState::Terminal;
        terminal.result_sha256 = Some(result_sha256);
        terminal.outcome = Some(outcome);
        terminal.uncertain_effect = uncertain_effect;
        terminal.receipt_id = Some(receipt_id);
        terminal.receipt_sha256 = Some(receipt.receipt_sha256.clone());

        self.histories
            .get_mut(transaction_id)
            .ok_or(AuthorityTransactionError::InvalidTransition)?
            .push(terminal);
        self.receipts.push(receipt.clone());
        Ok(receipt)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CancellationPoint {
    None,
    BeforeConsume,
    BeforeLaunch,
    AfterLaunch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FaultPoint {
    PreparedStored,
    GrantConsumed,
    AttemptRecorded,
    LaunchBoundaryCommitted,
    WorkerReturned,
    ResultReconciled,
}

fn reconcile_results(results: &[EffectResult]) -> EffectResult {
    let Some(first) = results.first() else {
        return EffectResult::from_redacted_material(
            OperationOutcome::Uncertain,
            b"missing-worker-result",
            StateChange::Uncertain,
        );
    };
    if results.iter().all(|candidate| {
        candidate.outcome == first.outcome
            && candidate.result_sha256 == first.result_sha256
            && candidate.state_change == first.state_change
    }) {
        return first.clone();
    }
    EffectResult::from_redacted_material(
        OperationOutcome::Uncertain,
        b"conflicting-worker-results",
        StateChange::Uncertain,
    )
}

fn valid_transition(from: AuthorityTransactionState, to: AuthorityTransactionState) -> bool {
    matches!(
        (from, to),
        (
            AuthorityTransactionState::Prepared,
            AuthorityTransactionState::GrantConsumed | AuthorityTransactionState::Terminal
        ) | (
            AuthorityTransactionState::GrantConsumed,
            AuthorityTransactionState::AttemptRecorded | AuthorityTransactionState::Terminal
        ) | (
            AuthorityTransactionState::AttemptRecorded,
            AuthorityTransactionState::LaunchCommitted | AuthorityTransactionState::Terminal
        ) | (
            AuthorityTransactionState::LaunchCommitted,
            AuthorityTransactionState::Reconciling | AuthorityTransactionState::Terminal
        ) | (
            AuthorityTransactionState::Reconciling,
            AuthorityTransactionState::Terminal
        )
    )
}

fn operation_sha256(
    operation: agentmage_kernel_contracts::OperationBinding,
) -> Result<String, AuthorityTransactionError> {
    serde_json::to_vec(&operation)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| AuthorityTransactionError::IntegrityFailure)
}

fn receipt_sha256(receipt: &Receipt) -> Result<String, AuthorityTransactionError> {
    to_canonical_json(receipt)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| AuthorityTransactionError::IntegrityFailure)
}

fn stable_result_sha256(value: &str) -> String {
    sha256_hex(value.as_bytes())
}

fn sha256_hex(value: &[u8]) -> String {
    let digest = Sha256::digest(value);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

fn validate_identifier(value: &str) -> Result<(), AuthorityTransactionError> {
    if value.is_empty()
        || value.len() > 128
        || value.contains('*')
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
    {
        return Err(AuthorityTransactionError::InvalidIdentity);
    }
    Ok(())
}

fn outcome_code(outcome: OperationOutcome, uncertain: bool) -> &'static str {
    if uncertain {
        return "authority.transaction.effect_uncertain";
    }
    match outcome {
        OperationOutcome::Succeeded => "authority.transaction.succeeded",
        OperationOutcome::Denied => "authority.transaction.denied",
        OperationOutcome::Failed => "authority.transaction.failed",
        OperationOutcome::Cancelled => "authority.transaction.cancelled",
        OperationOutcome::TimedOut => "authority.transaction.timed_out",
        OperationOutcome::Uncertain => "authority.transaction.effect_uncertain",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{
        AuthorityTransactionCoordinator, AuthorityTransactionError, AuthorityTransactionRequest,
        CancellationPoint, EffectAuthorization, EffectDriver, EffectLaunch, EffectResult,
        FaultPoint, valid_transition,
    };
    use crate::{
        grants::{DerivedOperationGrantRequest, GrantIssuer, SessionReadGrantRequest},
        policy::{
            PolicyEngine, PolicyEvaluationContext, StrictLocalReadOnlyScope, ToolPolicyBinding,
        },
        tooling::{Tool, ToolRegistry},
    };
    use agentmage_kernel_contracts::{
        ActionId, ActionKind, ActorId, ApprovalId, AuthorityTransactionId, CapabilityGrant,
        ContractPayload, CorrelationId, DataSensitivity, GrantId, GrantNonce, GrantOperation,
        GrantPreimage, GrantSideEffect, GrantStatus, GrantTarget, OperationAttemptId,
        OperationBinding, OperationOutcome, RequiredGrantTemplate, SchemaId, SchemaReference,
        SessionId, StateChange, TaskId, ToolCall, ToolCallId, ToolDefinition, ToolId,
        ToolRiskLevel, WorkspaceId,
    };

    struct FixtureTool(ToolDefinition);

    impl Tool for FixtureTool {
        fn definition(&self) -> &ToolDefinition {
            &self.0
        }
    }

    #[derive(Default)]
    struct FakeDriver {
        launches: usize,
        launch: Option<EffectLaunch>,
        observed_attempt: Option<String>,
    }

    impl EffectDriver for FakeDriver {
        fn execute(&mut self, authorization: EffectAuthorization<'_>) -> EffectLaunch {
            self.launches += 1;
            self.observed_attempt = Some(format!(
                "{}:{}:{}:{}:{}",
                authorization.transaction_id().as_str(),
                authorization.attempt_id().as_str(),
                authorization.consumed_grant_sha256(),
                authorization.call().tool_id.as_str(),
                authorization.call().tool_call_id.as_str()
            ));
            self.launch.take().unwrap_or_else(EffectLaunch::failed)
        }
    }

    struct Fixture {
        registry: ToolRegistry,
        issuer: GrantIssuer,
        policy: PolicyEngine,
        grant: CapabilityGrant,
        call: ToolCall,
        context: PolicyEvaluationContext,
        transaction_id: AuthorityTransactionId,
        attempt_id: OperationAttemptId,
        approval_id: ApprovalId,
    }

    fn target(path: &[&str]) -> GrantTarget {
        GrantTarget {
            workspace_id: WorkspaceId::from_raw("workspace-0001"),
            path_components: path.iter().map(|value| (*value).to_owned()).collect(),
        }
    }

    fn schema() -> SchemaReference {
        SchemaReference {
            schema_id: SchemaId::from_raw("fixture.input"),
            schema_version: 1,
            schema_sha256: "1".repeat(64),
        }
    }

    fn fixture() -> Fixture {
        let operation = OperationBinding::new(GrantOperation::WorkspaceRead);
        let tool_id = ToolId::from_raw("fixture.read");
        let mut registry = ToolRegistry::new();
        registry
            .register_tool(Box::new(FixtureTool(ToolDefinition {
                schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                tool_id: tool_id.clone(),
                tool_version: "1.0.0".to_owned(),
                display_name: "Fixture reader".to_owned(),
                description: "Reads one synthetic fixture".to_owned(),
                input_schema: schema(),
                output_schema: schema(),
                risk_level: ToolRiskLevel::Low,
                declared_effects: vec![operation],
                required_grant: RequiredGrantTemplate {
                    operation,
                    target_scope: "workspace-file".to_owned(),
                    single_use: true,
                },
                timeout_ms: 1_000,
            })))
            .expect("tool must register");
        let actor_id = ActorId::from_raw("actor-local-0001");
        let session_id = SessionId::from_raw("session-0001");
        let task_id = TaskId::from_raw("task-0001");
        let action_id = ActionId::from_raw("action-0001");
        let operation_target = target(&["src", "fixture.txt"]);
        let policy = PolicyEngine::strict_local_read_only(StrictLocalReadOnlyScope {
            revision: 1,
            actors: BTreeSet::from([actor_id.clone()]),
            tasks: BTreeSet::from([task_id.clone()]),
            actions: BTreeSet::from([action_id.clone()]),
            tools: BTreeSet::from([ToolPolicyBinding {
                tool_id: tool_id.clone(),
                tool_version: "1.0.0".to_owned(),
            }]),
            targets: BTreeSet::from([operation_target.clone()]),
        })
        .expect("policy must build");
        let approval_id = ApprovalId::from_raw("approval-0001");
        let mut issuer = GrantIssuer::new();
        let parent = issuer
            .issue_session_read(SessionReadGrantRequest {
                grant_id: GrantId::from_raw("grant-parent-0001"),
                actor_id: actor_id.clone(),
                session_id: session_id.clone(),
                task_id: task_id.clone(),
                targets: vec![target(&[])],
                excluded_targets: vec![target(&["private"])],
                sensitivity: DataSensitivity::Ephemeral,
                issued_at_epoch_ms: 1_000,
                expires_at_epoch_ms: 60_000,
                nonce: GrantNonce::from_raw("nonce-parent-0001"),
                maximum_derived_operations: 1,
                preview_sha256: "2".repeat(64),
                policy_sha256: policy.policy_sha256().to_owned(),
            })
            .expect("parent must issue");
        let call = ToolCall {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            tool_call_id: ToolCallId::from_raw("call-0001"),
            correlation_id: CorrelationId::from_raw("correlation-0001"),
            action_id: action_id.clone(),
            tool_id: tool_id.clone(),
            tool_version: "1.0.0".to_owned(),
            arguments: ContractPayload {
                schema: schema(),
                media_type: "application/json".to_owned(),
                bytes: b"{}".to_vec(),
                sha256: super::sha256_hex(b"{}"),
            },
        };
        let grant = issuer
            .derive_operation(
                &parent.grant_id,
                DerivedOperationGrantRequest {
                    grant_id: GrantId::from_raw("grant-operation-0001"),
                    approval_id: approval_id.clone(),
                    action_id: action_id.clone(),
                    action_kind: ActionKind::DeterministicTool,
                    operation,
                    tool_id: tool_id.clone(),
                    tool_version: "1.0.0".to_owned(),
                    targets: vec![operation_target],
                    argument_sha256: call.arguments.sha256.clone(),
                    preimages: vec![GrantPreimage {
                        target_index: 0,
                        content_sha256: "3".repeat(64),
                        observed_revision: Some("fixture-v1".to_owned()),
                    }],
                    expected_side_effects: vec![GrantSideEffect {
                        operation,
                        target_indexes: vec![0],
                        details_sha256: "4".repeat(64),
                    }],
                    rollback_description: "No state change is permitted".to_owned(),
                    issued_at_epoch_ms: 2_000,
                    expires_at_epoch_ms: 30_000,
                    nonce: GrantNonce::from_raw("nonce-operation-0001"),
                    preview_sha256: "5".repeat(64),
                    policy_sha256: policy.policy_sha256().to_owned(),
                },
            )
            .expect("grant must derive");
        let context = PolicyEvaluationContext {
            actor_id,
            session_id,
            task_id,
            action_id,
            action_kind: ActionKind::DeterministicTool,
            tool_id,
            tool_version: "1.0.0".to_owned(),
            targets: grant.targets.clone(),
            argument_sha256: grant.argument_sha256.clone(),
            preimages: grant.preimages.clone(),
            expected_side_effects: grant.expected_side_effects.clone(),
            preview_sha256: grant.preview_sha256.clone(),
            now_epoch_ms: 3_000,
            network_scope: None,
            credential_scope: None,
            publication_scope: None,
        };
        Fixture {
            registry,
            issuer,
            policy,
            grant,
            call,
            context,
            transaction_id: AuthorityTransactionId::from_raw("transaction-0001"),
            attempt_id: OperationAttemptId::from_raw("attempt-0001"),
            approval_id,
        }
    }

    fn request(fixture: &Fixture) -> AuthorityTransactionRequest {
        AuthorityTransactionRequest {
            transaction_id: fixture.transaction_id.clone(),
            attempt_id: fixture.attempt_id.clone(),
            approval_id: fixture.approval_id.clone(),
            grant_id: fixture.grant.grant_id.clone(),
            call: fixture.call.clone(),
            context: fixture.context.clone(),
            cancellation: CancellationPoint::None,
            occurred_at_epoch_ms: 4_000,
            occurred_at: "1970-01-01T00:00:04Z".to_owned(),
        }
    }

    fn success() -> EffectResult {
        EffectResult {
            outcome: OperationOutcome::Succeeded,
            result_sha256: "6".repeat(64),
            state_change: StateChange::NotChanged,
        }
    }

    fn states(
        coordinator: &AuthorityTransactionCoordinator,
        transaction_id: &AuthorityTransactionId,
    ) -> Vec<agentmage_kernel_contracts::AuthorityTransactionState> {
        coordinator
            .history(transaction_id)
            .expect("history")
            .iter()
            .map(|record| record.state)
            .collect()
    }

    #[test]
    fn transition_matrix_allows_only_the_declared_forward_edges() {
        use agentmage_kernel_contracts::AuthorityTransactionState::{
            AttemptRecorded, GrantConsumed, LaunchCommitted, Prepared, Reconciling, Terminal,
        };

        let states = [
            Prepared,
            GrantConsumed,
            AttemptRecorded,
            LaunchCommitted,
            Reconciling,
            Terminal,
        ];
        let allowed = [
            (Prepared, GrantConsumed),
            (Prepared, Terminal),
            (GrantConsumed, AttemptRecorded),
            (GrantConsumed, Terminal),
            (AttemptRecorded, LaunchCommitted),
            (AttemptRecorded, Terminal),
            (LaunchCommitted, Reconciling),
            (LaunchCommitted, Terminal),
            (Reconciling, Terminal),
        ];

        for from in states {
            for to in states {
                assert_eq!(
                    valid_transition(from, to),
                    allowed.contains(&(from, to)),
                    "unexpected transition result: {from:?} -> {to:?}"
                );
            }
        }
    }

    #[test]
    fn missing_or_stale_grant_never_reaches_worker_launch() {
        let mut missing_fixture = fixture();
        let missing = GrantId::from_raw("grant-missing");
        let mut candidate = request(&missing_fixture);
        candidate.grant_id = missing;
        let mut coordinator = AuthorityTransactionCoordinator::new();
        let mut driver = FakeDriver::default();
        let receipt = coordinator
            .run(
                &missing_fixture.registry,
                &mut missing_fixture.issuer,
                &missing_fixture.policy,
                &candidate,
                &mut driver,
                None,
            )
            .expect("denial must close");
        assert_eq!(receipt.outcome, OperationOutcome::Denied);
        assert_eq!(driver.launches, 0);

        let mut changed_policy_fixture = fixture();
        let changed_policy = PolicyEngine::strict_local_read_only(StrictLocalReadOnlyScope {
            revision: 2,
            actors: BTreeSet::from([changed_policy_fixture.context.actor_id.clone()]),
            tasks: BTreeSet::from([changed_policy_fixture.context.task_id.clone()]),
            actions: BTreeSet::from([changed_policy_fixture.context.action_id.clone()]),
            tools: BTreeSet::from([ToolPolicyBinding {
                tool_id: changed_policy_fixture.context.tool_id.clone(),
                tool_version: changed_policy_fixture.context.tool_version.clone(),
            }]),
            targets: BTreeSet::from([changed_policy_fixture.context.targets[0].clone()]),
        })
        .expect("changed policy must build");
        let mut coordinator = AuthorityTransactionCoordinator::new();
        let mut driver = FakeDriver::default();
        let owned_request = request(&changed_policy_fixture);
        let receipt = coordinator
            .run(
                &changed_policy_fixture.registry,
                &mut changed_policy_fixture.issuer,
                &changed_policy,
                &owned_request,
                &mut driver,
                None,
            )
            .expect("changed policy denial closes");
        assert_eq!(receipt.outcome, OperationOutcome::Denied);
        assert_eq!(driver.launches, 0);
        assert_eq!(
            states(&coordinator, &missing_fixture.transaction_id),
            [
                agentmage_kernel_contracts::AuthorityTransactionState::Prepared,
                agentmage_kernel_contracts::AuthorityTransactionState::Terminal,
            ]
        );

        let mut stale_fixture = fixture();
        stale_fixture.context.preimages[0].content_sha256 = "f".repeat(64);
        let mut coordinator = AuthorityTransactionCoordinator::new();
        let mut driver = FakeDriver::default();
        let owned_request = request(&stale_fixture);
        let receipt = coordinator
            .run(
                &stale_fixture.registry,
                &mut stale_fixture.issuer,
                &stale_fixture.policy,
                &owned_request,
                &mut driver,
                None,
            )
            .expect("stale preimage denial closes");
        assert_eq!(receipt.outcome, OperationOutcome::Denied);
        assert_eq!(driver.launches, 0);
    }

    #[test]
    fn exact_success_uses_fixed_order_and_binds_receipt_to_every_identity() {
        let mut fixture = fixture();
        let mut coordinator = AuthorityTransactionCoordinator::new();
        let mut driver = FakeDriver {
            launch: Some(EffectLaunch::completed(success())),
            ..FakeDriver::default()
        };
        let owned_request = AuthorityTransactionRequest::new(
            fixture.transaction_id.clone(),
            fixture.attempt_id.clone(),
            fixture.approval_id.clone(),
            fixture.grant.grant_id.clone(),
            fixture.call.clone(),
            fixture.context.clone(),
            4_000,
            "1970-01-01T00:00:04Z",
        )
        .expect("public request must validate");
        let receipt = coordinator
            .execute_effect(
                &fixture.registry,
                &mut fixture.issuer,
                &fixture.policy,
                owned_request,
                &mut driver,
            )
            .expect("exact run succeeds");
        assert_eq!(receipt.outcome, OperationOutcome::Succeeded);
        assert_eq!(receipt.authority_transaction_id, fixture.transaction_id);
        assert_eq!(receipt.operation_attempt_id, fixture.attempt_id);
        assert_eq!(receipt.approval_id, fixture.approval_id);
        assert_eq!(receipt.grant_id, fixture.grant.grant_id);
        assert_eq!(receipt.operation, fixture.grant.operation);
        assert_eq!(driver.launches, 1);
        assert_eq!(states(&coordinator, &fixture.transaction_id).len(), 6);
    }

    #[test]
    fn production_request_rejects_call_context_identity_drift() {
        let fixture = fixture();
        let mut context = fixture.context.clone();
        context.tool_version = "2.0.0".to_owned();
        assert_eq!(
            AuthorityTransactionRequest::new(
                fixture.transaction_id,
                fixture.attempt_id,
                fixture.approval_id,
                fixture.grant.grant_id,
                fixture.call,
                context,
                4_000,
                "1970-01-01T00:00:04Z",
            ),
            Err(AuthorityTransactionError::InvalidToolCall)
        );
    }

    #[test]
    fn consumed_launch_failure_is_terminal_and_non_replayable() {
        let mut fixture = fixture();
        let mut coordinator = AuthorityTransactionCoordinator::new();
        let mut driver = FakeDriver {
            launch: Some(EffectLaunch::failed()),
            ..FakeDriver::default()
        };
        let owned_request = request(&fixture);
        let receipt = coordinator
            .run(
                &fixture.registry,
                &mut fixture.issuer,
                &fixture.policy,
                &owned_request,
                &mut driver,
                None,
            )
            .expect("launch failure closes");
        assert_eq!(receipt.outcome, OperationOutcome::Failed);
        assert_eq!(driver.launches, 1);
        assert_eq!(
            fixture
                .issuer
                .current(&fixture.grant.grant_id)
                .expect("grant retained")
                .status,
            GrantStatus::Consumed
        );

        let transaction_id = AuthorityTransactionId::from_raw("transaction-replay");
        let attempt_id = OperationAttemptId::from_raw("attempt-replay");
        let mut replay = request(&fixture);
        replay.transaction_id = transaction_id;
        replay.attempt_id = attempt_id;
        let replay_receipt = coordinator
            .run(
                &fixture.registry,
                &mut fixture.issuer,
                &fixture.policy,
                &replay,
                &mut driver,
                None,
            )
            .expect("replay denial closes");
        assert_eq!(replay_receipt.outcome, OperationOutcome::Denied);
        assert_eq!(driver.launches, 1);
    }

    #[test]
    fn cancellation_timeout_and_conflicting_duplicates_have_exact_semantics() {
        for (point, expected_launches, expected_status) in [
            (CancellationPoint::BeforeConsume, 0, GrantStatus::Issued),
            (CancellationPoint::BeforeLaunch, 0, GrantStatus::Consumed),
        ] {
            let mut fixture = fixture();
            let mut request = request(&fixture);
            request.cancellation = point;
            let mut coordinator = AuthorityTransactionCoordinator::new();
            let mut driver = FakeDriver::default();
            let receipt = coordinator
                .run(
                    &fixture.registry,
                    &mut fixture.issuer,
                    &fixture.policy,
                    &request,
                    &mut driver,
                    None,
                )
                .expect("cancellation closes");
            assert_eq!(receipt.outcome, OperationOutcome::Cancelled);
            assert_eq!(driver.launches, expected_launches);
            assert_eq!(
                fixture
                    .issuer
                    .current(&fixture.grant.grant_id)
                    .expect("grant retained")
                    .status,
                expected_status
            );
        }

        let mut after_launch_fixture = fixture();
        let mut after_launch = request(&after_launch_fixture);
        after_launch.cancellation = CancellationPoint::AfterLaunch;
        let mut coordinator = AuthorityTransactionCoordinator::new();
        let mut driver = FakeDriver {
            launch: Some(EffectLaunch::completed(success())),
            ..FakeDriver::default()
        };
        let receipt = coordinator
            .run(
                &after_launch_fixture.registry,
                &mut after_launch_fixture.issuer,
                &after_launch_fixture.policy,
                &after_launch,
                &mut driver,
                None,
            )
            .expect("post-launch cancellation closes");
        assert_eq!(receipt.outcome, OperationOutcome::Cancelled);
        assert_eq!(driver.launches, 1);
        assert_eq!(
            after_launch_fixture
                .issuer
                .current(&after_launch_fixture.grant.grant_id)
                .expect("grant retained")
                .status,
            GrantStatus::Uncertain
        );

        for (results, expected) in [
            (
                vec![EffectResult {
                    outcome: OperationOutcome::TimedOut,
                    result_sha256: "7".repeat(64),
                    state_change: StateChange::Uncertain,
                }],
                OperationOutcome::TimedOut,
            ),
            (vec![success(), success()], OperationOutcome::Succeeded),
            (
                vec![
                    success(),
                    EffectResult {
                        outcome: OperationOutcome::Failed,
                        result_sha256: "8".repeat(64),
                        state_change: StateChange::Uncertain,
                    },
                ],
                OperationOutcome::Uncertain,
            ),
        ] {
            let mut fixture = fixture();
            let mut coordinator = AuthorityTransactionCoordinator::new();
            let mut driver = FakeDriver {
                launch: Some(EffectLaunch::completed_results(results)),
                ..FakeDriver::default()
            };
            let owned_request = request(&fixture);
            let receipt = coordinator
                .run(
                    &fixture.registry,
                    &mut fixture.issuer,
                    &fixture.policy,
                    &owned_request,
                    &mut driver,
                    None,
                )
                .expect("result closes");
            assert_eq!(receipt.outcome, expected);
        }
    }

    #[test]
    fn every_crash_boundary_recovers_without_replay_or_hidden_effects() {
        for fault in [
            FaultPoint::PreparedStored,
            FaultPoint::GrantConsumed,
            FaultPoint::AttemptRecorded,
            FaultPoint::LaunchBoundaryCommitted,
            FaultPoint::WorkerReturned,
            FaultPoint::ResultReconciled,
        ] {
            let mut fixture = fixture();
            let mut coordinator = AuthorityTransactionCoordinator::new();
            let mut driver = FakeDriver {
                launch: Some(EffectLaunch::completed(success())),
                ..FakeDriver::default()
            };
            let owned_request = request(&fixture);
            assert_eq!(
                coordinator.run(
                    &fixture.registry,
                    &mut fixture.issuer,
                    &fixture.policy,
                    &owned_request,
                    &mut driver,
                    Some(fault),
                ),
                Err(AuthorityTransactionError::SimulatedCrash)
            );
            let receipt = coordinator
                .recover(&mut fixture.issuer, &fixture.transaction_id, 5_000)
                .expect("recovery closes");
            let launched = matches!(
                fault,
                FaultPoint::WorkerReturned | FaultPoint::ResultReconciled
            );
            assert_eq!(
                receipt.outcome,
                if matches!(
                    fault,
                    FaultPoint::LaunchBoundaryCommitted
                        | FaultPoint::WorkerReturned
                        | FaultPoint::ResultReconciled
                ) {
                    OperationOutcome::Uncertain
                } else {
                    OperationOutcome::Failed
                }
            );
            assert_eq!(driver.launches, usize::from(launched));
            let expected_grant_status = match fault {
                FaultPoint::PreparedStored => GrantStatus::Issued,
                FaultPoint::GrantConsumed | FaultPoint::AttemptRecorded => GrantStatus::Consumed,
                FaultPoint::LaunchBoundaryCommitted
                | FaultPoint::WorkerReturned
                | FaultPoint::ResultReconciled => GrantStatus::Uncertain,
            };
            assert_eq!(
                fixture
                    .issuer
                    .current(&fixture.grant.grant_id)
                    .expect("grant retained")
                    .status,
                expected_grant_status
            );
            assert_eq!(
                coordinator
                    .current(&fixture.transaction_id)
                    .expect("terminal")
                    .state,
                agentmage_kernel_contracts::AuthorityTransactionState::Terminal
            );
        }
    }

    #[test]
    fn receipt_sequence_is_hash_chained_and_terminal_recovery_is_idempotent() {
        let mut fixture = fixture();
        let mut coordinator = AuthorityTransactionCoordinator::new();
        let missing = GrantId::from_raw("grant-missing");
        let mut first_request = request(&fixture);
        first_request.grant_id = missing;
        let mut driver = FakeDriver::default();
        let first = coordinator
            .run(
                &fixture.registry,
                &mut fixture.issuer,
                &fixture.policy,
                &first_request,
                &mut driver,
                None,
            )
            .expect("first denial closes");

        let transaction_id = AuthorityTransactionId::from_raw("transaction-0002");
        let attempt_id = OperationAttemptId::from_raw("attempt-0002");
        let mut second_request = request(&fixture);
        second_request.transaction_id = transaction_id.clone();
        second_request.attempt_id = attempt_id;
        let mut driver = FakeDriver {
            launch: Some(EffectLaunch::completed(success())),
            ..FakeDriver::default()
        };
        let second = coordinator
            .run(
                &fixture.registry,
                &mut fixture.issuer,
                &fixture.policy,
                &second_request,
                &mut driver,
                None,
            )
            .expect("second transaction closes");
        assert_eq!(second.sequence, 2);
        assert_eq!(second.previous_receipt_sha256, first.receipt_sha256);
        assert_eq!(
            coordinator
                .recover(&mut fixture.issuer, &transaction_id, 6_000)
                .expect("terminal recovery is idempotent"),
            second
        );
        assert_eq!(coordinator.receipts().len(), 2);
    }
}
