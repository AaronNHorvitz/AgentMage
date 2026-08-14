//! Deterministic restart reconciliation and opaque resume permits.

use std::collections::{BTreeMap, BTreeSet};

use agentmage_kernel_contracts::{
    AgentRestartSnapshot, AgentStateKind, AuthorityTransactionId, AuthorityTransactionState,
    GrantId, GrantStatus, OperationOutcome, PolicyId, Receipt, ReceiptId, RepositorySnapshotId,
    TaskId, to_canonical_json,
};
use sha2::{Digest, Sha256};

const MAX_AUTHORITY_TRANSACTIONS: usize = 1_024;
const MAX_GRANTS: usize = 2_048;
const MAX_RECEIPTS: usize = 4_096;
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Stable reason persisted restart state cannot authorize another agent transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestartReconciliationError {
    /// Expected context or persisted snapshot shape is malformed or oversized.
    InvalidSnapshot,
    /// Task, repository, policy, profile, state, or revision differs from expectation.
    ContextMismatch,
    /// Unconsumed authority remains pending and must be revoked or re-approved.
    PendingAuthority,
    /// A consumed grant lacks one exact terminal transaction and receipt.
    ConsumedGrantUnresolved,
    /// A launched or explicitly uncertain effect cannot be resumed safely.
    UncertainEffect,
    /// Receipt identity, ordering, digest, or terminal binding is inconsistent.
    ReceiptMismatch,
}

/// Exact immutable context expected by one interrupted agent-state restoration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestartExpectation {
    /// Exact current task identity.
    pub task_id: TaskId,
    /// Exact current repository snapshot identity.
    pub repository_snapshot_id: RepositorySnapshotId,
    /// Exact deterministic policy identity.
    pub policy_id: PolicyId,
    /// Lowercase SHA-256 digest of the expected policy revision.
    pub policy_sha256: String,
    /// Exact selected configuration profile identity.
    pub selected_profile_id: String,
    /// Lowercase SHA-256 digest of the selected profile revision.
    pub selected_profile_sha256: String,
    /// Exact interrupted agent state.
    pub agent_state: AgentStateKind,
    /// Exact interrupted agent-state revision.
    pub agent_state_revision: u64,
}

/// One-use proof that persisted restart facts were reconciled for an exact state revision.
///
/// Fields are private and this type has no public constructor, clone implementation, or
/// deserializer. Only [`RestartReconciler::reconcile`] can create it.
#[derive(Debug, PartialEq, Eq)]
pub struct RestartPermit {
    agent_state: AgentStateKind,
    agent_state_revision: u64,
}

impl RestartPermit {
    pub(crate) const fn agent_state(&self) -> AgentStateKind {
        self.agent_state
    }

    pub(crate) const fn agent_state_revision(&self) -> u64 {
        self.agent_state_revision
    }
}

/// Frozen deterministic restart reconciler for one exact interrupted context.
pub struct RestartReconciler {
    expected: RestartExpectation,
}

impl RestartReconciler {
    /// Creates a reconciler for one valid active interrupted state.
    pub fn new(expected: RestartExpectation) -> Result<Self, RestartReconciliationError> {
        if !valid_expectation(&expected) {
            return Err(RestartReconciliationError::InvalidSnapshot);
        }
        Ok(Self { expected })
    }

    /// Reconciles every persisted identity, authority revision, grant, and receipt.
    pub fn reconcile(
        &self,
        snapshot: &AgentRestartSnapshot,
    ) -> Result<RestartPermit, RestartReconciliationError> {
        validate_snapshot_shape(snapshot)?;
        if snapshot.task_id != self.expected.task_id
            || snapshot.repository_snapshot_id != self.expected.repository_snapshot_id
            || snapshot.policy_id != self.expected.policy_id
            || snapshot.policy_sha256 != self.expected.policy_sha256
            || snapshot.selected_profile_id != self.expected.selected_profile_id
            || snapshot.selected_profile_sha256 != self.expected.selected_profile_sha256
            || snapshot.agent_state != self.expected.agent_state
            || snapshot.agent_state_revision != self.expected.agent_state_revision
        {
            return Err(RestartReconciliationError::ContextMismatch);
        }

        let transactions = transaction_map(snapshot)?;
        let grants = grant_map(snapshot)?;
        let receipts = validate_receipt_chain(snapshot, &transactions)?;
        validate_terminal_bindings(&transactions, &grants, &receipts)?;

        if snapshot
            .grants
            .iter()
            .any(|grant| grant.status == GrantStatus::Uncertain)
            || snapshot.authority_transactions.iter().any(|transaction| {
                transaction.uncertain_effect
                    || matches!(
                        transaction.state,
                        AuthorityTransactionState::LaunchCommitted
                            | AuthorityTransactionState::Reconciling
                    )
                    || transaction.outcome == Some(OperationOutcome::Uncertain)
            })
        {
            return Err(RestartReconciliationError::UncertainEffect);
        }
        if snapshot.authority_transactions.iter().any(|transaction| {
            matches!(
                transaction.state,
                AuthorityTransactionState::GrantConsumed
                    | AuthorityTransactionState::AttemptRecorded
            )
        }) || snapshot.grants.iter().any(|grant| {
            grant.status == GrantStatus::Consumed
                && !terminal_transaction_for_grant(&transactions, &grant.grant_id)
        }) {
            return Err(RestartReconciliationError::ConsumedGrantUnresolved);
        }
        if snapshot
            .authority_transactions
            .iter()
            .any(|transaction| transaction.state == AuthorityTransactionState::Prepared)
            || snapshot
                .grants
                .iter()
                .any(|grant| grant.status == GrantStatus::Issued)
        {
            return Err(RestartReconciliationError::PendingAuthority);
        }

        Ok(RestartPermit {
            agent_state: snapshot.agent_state,
            agent_state_revision: snapshot.agent_state_revision,
        })
    }
}

fn valid_expectation(value: &RestartExpectation) -> bool {
    value.agent_state_revision > 0
        && !value.agent_state.is_terminal()
        && valid_id(value.task_id.as_str())
        && valid_id(value.repository_snapshot_id.as_str())
        && valid_id(value.policy_id.as_str())
        && valid_sha256(&value.policy_sha256)
        && valid_id(&value.selected_profile_id)
        && valid_sha256(&value.selected_profile_sha256)
}

fn validate_snapshot_shape(
    snapshot: &AgentRestartSnapshot,
) -> Result<(), RestartReconciliationError> {
    if snapshot.schema_version != agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION
        || snapshot.agent_state_revision == 0
        || snapshot.agent_state.is_terminal()
        || snapshot.authority_transactions.len() > MAX_AUTHORITY_TRANSACTIONS
        || snapshot.grants.len() > MAX_GRANTS
        || snapshot.receipts.len() > MAX_RECEIPTS
        || !valid_id(snapshot.task_id.as_str())
        || !valid_id(snapshot.repository_snapshot_id.as_str())
        || !valid_id(snapshot.policy_id.as_str())
        || !valid_sha256(&snapshot.policy_sha256)
        || !valid_id(&snapshot.selected_profile_id)
        || !valid_sha256(&snapshot.selected_profile_sha256)
    {
        return Err(RestartReconciliationError::InvalidSnapshot);
    }
    Ok(())
}

fn transaction_map(
    snapshot: &AgentRestartSnapshot,
) -> Result<
    BTreeMap<&AuthorityTransactionId, &agentmage_kernel_contracts::AuthorityTransactionRecord>,
    RestartReconciliationError,
> {
    let mut values = BTreeMap::new();
    for transaction in &snapshot.authority_transactions {
        if transaction.schema_version != agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION
            || transaction.task_id != snapshot.task_id
            || transaction.revision == 0
            || !valid_id(transaction.authority_transaction_id.as_str())
            || values
                .insert(&transaction.authority_transaction_id, transaction)
                .is_some()
        {
            return Err(RestartReconciliationError::InvalidSnapshot);
        }
    }
    Ok(values)
}

fn grant_map(
    snapshot: &AgentRestartSnapshot,
) -> Result<
    BTreeMap<&GrantId, &agentmage_kernel_contracts::CapabilityGrant>,
    RestartReconciliationError,
> {
    let mut values = BTreeMap::new();
    for grant in &snapshot.grants {
        if grant.schema_version != agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION
            || grant.task_id != snapshot.task_id
            || grant.revision == 0
            || grant.policy_sha256 != snapshot.policy_sha256
            || !valid_id(grant.grant_id.as_str())
            || values.insert(&grant.grant_id, grant).is_some()
        {
            return Err(RestartReconciliationError::InvalidSnapshot);
        }
    }
    Ok(values)
}

fn validate_receipt_chain<'a>(
    snapshot: &'a AgentRestartSnapshot,
    transactions: &BTreeMap<
        &AuthorityTransactionId,
        &agentmage_kernel_contracts::AuthorityTransactionRecord,
    >,
) -> Result<BTreeMap<&'a ReceiptId, &'a Receipt>, RestartReconciliationError> {
    let mut values = BTreeMap::new();
    let mut transaction_receipts = BTreeSet::new();
    let mut previous = ZERO_SHA256.to_owned();
    for (index, receipt) in snapshot.receipts.iter().enumerate() {
        let expected_sequence = u64::try_from(index)
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or(RestartReconciliationError::ReceiptMismatch)?;
        if receipt.schema_version != agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION
            || receipt.task_id != snapshot.task_id
            || receipt.sequence != expected_sequence
            || receipt.previous_receipt_sha256 != previous
            || !valid_id(receipt.receipt_id.as_str())
            || !valid_sha256(&receipt.receipt_sha256)
            || receipt_digest(receipt)? != receipt.receipt_sha256
            || values.insert(&receipt.receipt_id, receipt).is_some()
            || !transaction_receipts.insert(&receipt.authority_transaction_id)
            || !transactions.contains_key(&receipt.authority_transaction_id)
        {
            return Err(RestartReconciliationError::ReceiptMismatch);
        }
        previous.clone_from(&receipt.receipt_sha256);
    }
    Ok(values)
}

fn validate_terminal_bindings(
    transactions: &BTreeMap<
        &AuthorityTransactionId,
        &agentmage_kernel_contracts::AuthorityTransactionRecord,
    >,
    grants: &BTreeMap<&GrantId, &agentmage_kernel_contracts::CapabilityGrant>,
    receipts: &BTreeMap<&ReceiptId, &Receipt>,
) -> Result<(), RestartReconciliationError> {
    for transaction in transactions.values() {
        if transaction.state == AuthorityTransactionState::Terminal {
            let receipt_id = transaction
                .receipt_id
                .as_ref()
                .ok_or(RestartReconciliationError::ReceiptMismatch)?;
            let receipt = receipts
                .get(receipt_id)
                .ok_or(RestartReconciliationError::ReceiptMismatch)?;
            let grant = grants
                .get(&transaction.grant_id)
                .ok_or(RestartReconciliationError::ConsumedGrantUnresolved)?;
            if transaction.receipt_sha256.as_deref() != Some(receipt.receipt_sha256.as_str())
                || transaction.outcome != Some(receipt.outcome)
                || receipt.authority_transaction_id != transaction.authority_transaction_id
                || receipt.grant_id != transaction.grant_id
                || if transaction.uncertain_effect
                    || transaction.outcome == Some(OperationOutcome::Uncertain)
                {
                    grant.status != GrantStatus::Uncertain
                } else {
                    grant.status != GrantStatus::Consumed
                }
            {
                return Err(RestartReconciliationError::ReceiptMismatch);
            }
        } else if transaction.receipt_id.is_some() || transaction.receipt_sha256.is_some() {
            return Err(RestartReconciliationError::ReceiptMismatch);
        }
    }
    for receipt in receipts.values() {
        let transaction = transactions
            .get(&receipt.authority_transaction_id)
            .ok_or(RestartReconciliationError::ReceiptMismatch)?;
        if transaction.state != AuthorityTransactionState::Terminal {
            return Err(RestartReconciliationError::ReceiptMismatch);
        }
    }
    Ok(())
}

fn terminal_transaction_for_grant(
    transactions: &BTreeMap<
        &AuthorityTransactionId,
        &agentmage_kernel_contracts::AuthorityTransactionRecord,
    >,
    grant_id: &GrantId,
) -> bool {
    transactions.values().any(|transaction| {
        transaction.grant_id == *grant_id
            && transaction.state == AuthorityTransactionState::Terminal
    })
}

fn receipt_digest(receipt: &Receipt) -> Result<String, RestartReconciliationError> {
    let mut candidate = receipt.clone();
    candidate.receipt_sha256 = ZERO_SHA256.to_owned();
    let bytes =
        to_canonical_json(&candidate).map_err(|_| RestartReconciliationError::ReceiptMismatch)?;
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}")
            .map_err(|_| RestartReconciliationError::ReceiptMismatch)?;
    }
    Ok(encoded)
}

fn valid_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::{
        RestartExpectation, RestartReconciler, RestartReconciliationError, receipt_digest,
    };
    use crate::agent_state::{AgentStateController, AgentStateError, legal_transition};
    use agentmage_kernel_contracts::{
        ActionId, ActionKind, ActorId, AgentRestartSnapshot, AgentStateKind, ApprovalId,
        AuthorityTransactionId, AuthorityTransactionRecord, AuthorityTransactionState,
        CapabilityGrant, CorrelationId, DataSensitivity, GrantClass, GrantId, GrantNonce,
        GrantOperation, GrantStatus, OperationAttemptId, OperationBinding, OperationOutcome,
        PolicyId, Receipt, ReceiptId, RepositorySnapshotId, SessionId, TaskId, ToolCallId, ToolId,
        from_json, to_canonical_json,
    };

    const POLICY_SHA256: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    const PROFILE_SHA256: &str = "2222222222222222222222222222222222222222222222222222222222222222";
    const OTHER_SHA256: &str = "3333333333333333333333333333333333333333333333333333333333333333";

    fn expectation() -> RestartExpectation {
        RestartExpectation {
            task_id: TaskId::from_raw("task-0001"),
            repository_snapshot_id: RepositorySnapshotId::from_raw("snapshot-0001"),
            policy_id: PolicyId::from_raw("policy-0001"),
            policy_sha256: POLICY_SHA256.to_owned(),
            selected_profile_id: "profile-0001".to_owned(),
            selected_profile_sha256: PROFILE_SHA256.to_owned(),
            agent_state: AgentStateKind::Validation,
            agent_state_revision: 4,
        }
    }

    fn snapshot() -> AgentRestartSnapshot {
        let expected = expectation();
        AgentRestartSnapshot {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            task_id: expected.task_id,
            repository_snapshot_id: expected.repository_snapshot_id,
            policy_id: expected.policy_id,
            policy_sha256: expected.policy_sha256,
            selected_profile_id: expected.selected_profile_id,
            selected_profile_sha256: expected.selected_profile_sha256,
            agent_state: expected.agent_state,
            agent_state_revision: expected.agent_state_revision,
            authority_transactions: Vec::new(),
            grants: Vec::new(),
            receipts: Vec::new(),
        }
    }

    fn grant(status: GrantStatus) -> CapabilityGrant {
        CapabilityGrant {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            grant_id: GrantId::from_raw("grant-0001"),
            revision: 2,
            grant_class: GrantClass::Operation,
            actor_id: ActorId::from_raw("actor-0001"),
            approval_id: Some(ApprovalId::from_raw("approval-0001")),
            session_id: SessionId::from_raw("session-0001"),
            task_id: TaskId::from_raw("task-0001"),
            action_id: Some(ActionId::from_raw("action-0001")),
            action_kind: Some(ActionKind::DeterministicTool),
            operation: OperationBinding::new(GrantOperation::WorkspaceRead),
            tool_id: Some(ToolId::from_raw("tool-0001")),
            tool_version: Some("1.0.0".to_owned()),
            targets: Vec::new(),
            excluded_targets: Vec::new(),
            sensitivity: DataSensitivity::Operational,
            argument_sha256: OTHER_SHA256.to_owned(),
            preimages: Vec::new(),
            expected_side_effects: Vec::new(),
            rollback_description: "No state change".to_owned(),
            issued_at_epoch_ms: 1,
            expires_at_epoch_ms: 10,
            nonce: GrantNonce::from_raw("nonce-0001"),
            use_limit: 1,
            use_count: u32::from(status == GrantStatus::Consumed),
            parent_grant_id: None,
            parent_grant_sha256: None,
            preview_sha256: OTHER_SHA256.to_owned(),
            policy_sha256: POLICY_SHA256.to_owned(),
            status,
        }
    }

    fn transaction(
        state: AuthorityTransactionState,
        outcome: Option<OperationOutcome>,
        uncertain_effect: bool,
    ) -> AuthorityTransactionRecord {
        AuthorityTransactionRecord {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            authority_transaction_id: AuthorityTransactionId::from_raw("transaction-0001"),
            revision: 6,
            operation_attempt_id: OperationAttemptId::from_raw("attempt-0001"),
            approval_id: ApprovalId::from_raw("approval-0001"),
            grant_id: GrantId::from_raw("grant-0001"),
            operation: OperationBinding::new(GrantOperation::WorkspaceRead),
            correlation_id: CorrelationId::from_raw("correlation-0001"),
            session_id: SessionId::from_raw("session-0001"),
            task_id: TaskId::from_raw("task-0001"),
            action_id: ActionId::from_raw("action-0001"),
            tool_call_id: ToolCallId::from_raw("call-0001"),
            state,
            consumed_grant_sha256: (state != AuthorityTransactionState::Prepared)
                .then(|| OTHER_SHA256.to_owned()),
            result_sha256: outcome.map(|_| OTHER_SHA256.to_owned()),
            outcome,
            uncertain_effect,
            receipt_id: None,
            receipt_sha256: None,
            occurred_at: "1970-01-01T00:00:01Z".to_owned(),
        }
    }

    fn completed_snapshot() -> AgentRestartSnapshot {
        let mut current = snapshot();
        let mut receipt = Receipt {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            receipt_id: ReceiptId::from_raw("receipt-0001"),
            sequence: 1,
            correlation_id: CorrelationId::from_raw("correlation-0001"),
            authority_transaction_id: AuthorityTransactionId::from_raw("transaction-0001"),
            operation_attempt_id: OperationAttemptId::from_raw("attempt-0001"),
            approval_id: ApprovalId::from_raw("approval-0001"),
            grant_id: GrantId::from_raw("grant-0001"),
            session_id: SessionId::from_raw("session-0001"),
            task_id: TaskId::from_raw("task-0001"),
            action_id: ActionId::from_raw("action-0001"),
            tool_call_id: Some(ToolCallId::from_raw("call-0001")),
            operation: OperationBinding::new(GrantOperation::WorkspaceRead),
            outcome: OperationOutcome::Succeeded,
            operation_sha256: OTHER_SHA256.to_owned(),
            evidence: Vec::new(),
            error: None,
            previous_receipt_sha256: super::ZERO_SHA256.to_owned(),
            receipt_sha256: super::ZERO_SHA256.to_owned(),
            occurred_at: "1970-01-01T00:00:01Z".to_owned(),
        };
        receipt.receipt_sha256 = receipt_digest(&receipt).expect("receipt digest");
        let mut terminal = transaction(
            AuthorityTransactionState::Terminal,
            Some(OperationOutcome::Succeeded),
            false,
        );
        terminal.receipt_id = Some(receipt.receipt_id.clone());
        terminal.receipt_sha256 = Some(receipt.receipt_sha256.clone());
        current.authority_transactions.push(terminal);
        current.grants.push(grant(GrantStatus::Consumed));
        current.receipts.push(receipt);
        current
    }

    #[test]
    fn task_12_2_1_5_exact_reconciliation_unlocks_one_restored_controller() {
        let reconciler = RestartReconciler::new(expectation()).expect("valid expectation");
        let current = snapshot();
        let encoded = to_canonical_json(&current).expect("snapshot serializes");
        assert_eq!(
            from_json::<AgentRestartSnapshot>(&encoded),
            Ok(current.clone())
        );
        let permit = reconciler.reconcile(&current).expect("safe restart");
        let mut controller = AgentStateController::restore(AgentStateKind::Validation, 4)
            .expect("restored controller");
        assert_eq!(
            controller.transition(AgentStateKind::Approval),
            Err(AgentStateError::RestartRequired)
        );
        controller
            .resume_after_restart(permit)
            .expect("permit matches");
        controller
            .transition(AgentStateKind::Approval)
            .expect("transition after reconciliation");
        assert_eq!(controller.revision(), 5);
    }

    #[test]
    fn task_12_2_1_5_every_current_context_identity_must_match() {
        let reconciler = RestartReconciler::new(expectation()).expect("valid expectation");
        let mut candidates = Vec::new();
        let mut task = snapshot();
        task.task_id = TaskId::from_raw("task-other");
        candidates.push(task);
        let mut repository = snapshot();
        repository.repository_snapshot_id = RepositorySnapshotId::from_raw("snapshot-other");
        candidates.push(repository);
        let mut policy_id = snapshot();
        policy_id.policy_id = PolicyId::from_raw("policy-other");
        candidates.push(policy_id);
        let mut policy_revision = snapshot();
        policy_revision.policy_sha256 = OTHER_SHA256.to_owned();
        candidates.push(policy_revision);
        let mut profile = snapshot();
        profile.selected_profile_id = "profile-other".to_owned();
        candidates.push(profile);
        let mut profile_revision = snapshot();
        profile_revision.selected_profile_sha256 = OTHER_SHA256.to_owned();
        candidates.push(profile_revision);
        let mut state = snapshot();
        state.agent_state = AgentStateKind::Approval;
        candidates.push(state);
        let mut revision = snapshot();
        revision.agent_state_revision = 5;
        candidates.push(revision);
        for candidate in candidates {
            assert_eq!(
                reconciler.reconcile(&candidate),
                Err(RestartReconciliationError::ContextMismatch)
            );
        }
    }

    #[test]
    fn task_12_2_1_5_pending_or_consumed_authority_blocks_resume() {
        let reconciler = RestartReconciler::new(expectation()).expect("valid expectation");
        let mut pending = snapshot();
        pending.grants.push(grant(GrantStatus::Issued));
        assert_eq!(
            reconciler.reconcile(&pending),
            Err(RestartReconciliationError::PendingAuthority)
        );

        let mut consumed = snapshot();
        consumed.grants.push(grant(GrantStatus::Consumed));
        consumed.authority_transactions.push(transaction(
            AuthorityTransactionState::AttemptRecorded,
            None,
            false,
        ));
        assert_eq!(
            reconciler.reconcile(&consumed),
            Err(RestartReconciliationError::ConsumedGrantUnresolved)
        );
    }

    #[test]
    fn task_12_2_1_5_completed_consumed_grant_and_receipt_reconcile_exactly() {
        let reconciler = RestartReconciler::new(expectation()).expect("valid expectation");
        assert!(reconciler.reconcile(&completed_snapshot()).is_ok());
    }

    #[test]
    fn task_12_2_1_5_launched_reconciling_or_uncertain_effects_never_resume() {
        let reconciler = RestartReconciler::new(expectation()).expect("valid expectation");
        for state in [
            AuthorityTransactionState::LaunchCommitted,
            AuthorityTransactionState::Reconciling,
        ] {
            let mut current = snapshot();
            current.grants.push(grant(GrantStatus::Consumed));
            current
                .authority_transactions
                .push(transaction(state, None, false));
            assert_eq!(
                reconciler.reconcile(&current),
                Err(RestartReconciliationError::UncertainEffect)
            );
        }
        let mut uncertain = snapshot();
        uncertain.grants.push(grant(GrantStatus::Uncertain));
        assert_eq!(
            reconciler.reconcile(&uncertain),
            Err(RestartReconciliationError::UncertainEffect)
        );

        let mut terminal_uncertain = completed_snapshot();
        terminal_uncertain.grants[0].status = GrantStatus::Uncertain;
        terminal_uncertain.authority_transactions[0].outcome = Some(OperationOutcome::Uncertain);
        terminal_uncertain.authority_transactions[0].uncertain_effect = true;
        terminal_uncertain.receipts[0].outcome = OperationOutcome::Uncertain;
        terminal_uncertain.receipts[0].receipt_sha256 = super::ZERO_SHA256.to_owned();
        terminal_uncertain.receipts[0].receipt_sha256 =
            receipt_digest(&terminal_uncertain.receipts[0]).expect("receipt digest");
        terminal_uncertain.authority_transactions[0].receipt_sha256 =
            Some(terminal_uncertain.receipts[0].receipt_sha256.clone());
        assert_eq!(
            reconciler.reconcile(&terminal_uncertain),
            Err(RestartReconciliationError::UncertainEffect)
        );
    }

    #[test]
    fn task_12_2_1_5_receipt_replay_tampering_and_permit_mismatch_fail_closed() {
        let reconciler = RestartReconciler::new(expectation()).expect("valid expectation");
        let mut duplicate = completed_snapshot();
        duplicate.receipts.push(duplicate.receipts[0].clone());
        assert_eq!(
            reconciler.reconcile(&duplicate),
            Err(RestartReconciliationError::ReceiptMismatch)
        );
        let mut tampered = completed_snapshot();
        tampered.receipts[0].operation_sha256 = POLICY_SHA256.to_owned();
        assert_eq!(
            reconciler.reconcile(&tampered),
            Err(RestartReconciliationError::ReceiptMismatch)
        );

        let permit = reconciler.reconcile(&snapshot()).expect("safe restart");
        let mut wrong = AgentStateController::restore(AgentStateKind::Validation, 5)
            .expect("restored controller");
        assert_eq!(
            wrong.resume_after_restart(permit),
            Err(AgentStateError::RestartPermitMismatch)
        );
        assert_eq!(
            wrong.transition(AgentStateKind::Approval),
            Err(AgentStateError::RestartRequired)
        );
    }

    #[test]
    fn d027_s12_restart_before_and_after_every_state_edge_reconciles_exactly() {
        const STATES: [AgentStateKind; 17] = [
            AgentStateKind::Observation,
            AgentStateKind::Proposal,
            AgentStateKind::Validation,
            AgentStateKind::Clarification,
            AgentStateKind::Approval,
            AgentStateKind::Execution,
            AgentStateKind::Verification,
            AgentStateKind::Checkpoint,
            AgentStateKind::Success,
            AgentStateKind::NoOp,
            AgentStateKind::Blocked,
            AgentStateKind::Declined,
            AgentStateKind::Stalled,
            AgentStateKind::Exhausted,
            AgentStateKind::Uncertain,
            AgentStateKind::Cancelled,
            AgentStateKind::Failed,
        ];
        let mut before_interruptions = 0_u64;
        let mut after_interruptions = 0_u64;
        let mut edge_index = 0_u64;

        for from in STATES.into_iter().filter(|state| !state.is_terminal()) {
            for to in STATES {
                if !legal_transition(from, to) {
                    continue;
                }
                edge_index += 1;
                let revision = 100 + edge_index * 2;
                let mut before_expectation = expectation();
                before_expectation.agent_state = from;
                before_expectation.agent_state_revision = revision;
                let mut before_snapshot = snapshot();
                before_snapshot.agent_state = from;
                before_snapshot.agent_state_revision = revision;
                let before_reconciler =
                    RestartReconciler::new(before_expectation).expect("before expectation");
                let permit = before_reconciler
                    .reconcile(&before_snapshot)
                    .expect("before state reconciles");
                let mut controller =
                    AgentStateController::restore(from, revision).expect("active source restores");
                assert_eq!(
                    controller.transition(to),
                    Err(AgentStateError::RestartRequired)
                );
                controller
                    .resume_after_restart(permit)
                    .expect("before permit matches");
                if to.is_success() {
                    assert_eq!(
                        controller.transition(to),
                        Err(AgentStateError::VerifierRequired)
                    );
                    assert_eq!(controller.current(), from);
                } else {
                    controller
                        .transition(to)
                        .expect("ordinary edge after resume");
                    assert_eq!(controller.current(), to);
                }
                before_interruptions += 1;

                let after_revision = revision + 1;
                if to.is_terminal() {
                    assert_eq!(
                        AgentStateController::restore(to, after_revision),
                        Err(AgentStateError::InvalidRestoredState)
                    );
                    for next in STATES {
                        assert!(!legal_transition(to, next));
                    }
                } else {
                    let mut after_expectation = expectation();
                    after_expectation.agent_state = to;
                    after_expectation.agent_state_revision = after_revision;
                    let mut after_snapshot = snapshot();
                    after_snapshot.agent_state = to;
                    after_snapshot.agent_state_revision = after_revision;
                    let after_reconciler =
                        RestartReconciler::new(after_expectation).expect("after expectation");
                    let after_permit = after_reconciler
                        .reconcile(&after_snapshot)
                        .expect("after state reconciles");
                    let mut restored = AgentStateController::restore(to, after_revision)
                        .expect("active destination restores");
                    restored
                        .resume_after_restart(after_permit)
                        .expect("after permit matches");
                    assert_eq!(restored.current(), to);
                }
                after_interruptions += 1;
            }
        }

        assert_eq!(edge_index, 52);
        assert_eq!(before_interruptions, 52);
        assert_eq!(after_interruptions, 52);
    }
}
