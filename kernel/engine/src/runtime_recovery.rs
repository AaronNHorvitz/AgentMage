//! Deterministic, non-authoritative runtime recovery planning.

use agentmage_kernel_contracts::AgentStateKind;
use serde::{Deserialize, Serialize};

use crate::{agent_restart::RestartReconciliationError, write_recovery::WriteRecoveryInstruction};

/// Exact runtime boundary at which durable execution was interrupted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeRecoveryBoundary {
    /// A model invocation had not produced an admitted proposal.
    Model,
    /// Context assembly or refresh was in progress.
    Context,
    /// The runtime was waiting for or validating a user decision.
    Approval,
    /// A controlled workspace write was being supervised.
    Write,
    /// A bounded command was being supervised.
    Command,
    /// Deterministic validation was in progress.
    Validation,
    /// Canonical event publication or flush was in progress.
    Event,
    /// Private artifact publication or verification was in progress.
    Artifact,
    /// Atomic checkpoint publication was in progress.
    Checkpoint,
    /// Terminal outcome publication or rendering was in progress.
    Terminal,
}

impl RuntimeRecoveryBoundary {
    /// Every closed runtime recovery boundary.
    pub const ALL: [Self; 10] = [
        Self::Model,
        Self::Context,
        Self::Approval,
        Self::Write,
        Self::Command,
        Self::Validation,
        Self::Event,
        Self::Artifact,
        Self::Checkpoint,
        Self::Terminal,
    ];
}

/// Integrity state of one durable recovery domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeRecoveryEvidenceState {
    /// The complete current domain was freshly verified.
    Verified,
    /// Required durable evidence is absent.
    Missing,
    /// Evidence belongs to a stale request, snapshot, policy, or revision.
    Stale,
    /// Evidence failed schema, digest, ordering, or ownership verification.
    Invalid,
}

/// Result class produced by the existing restart reconciler.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeRestartReconciliation {
    /// Exact persisted restart facts were reconciled successfully.
    Verified,
    /// Persisted shape or bounds were invalid.
    InvalidSnapshot,
    /// Immutable task, repository, policy, profile, state, or revision changed.
    ContextMismatch,
    /// Unconsumed authority remains pending.
    PendingAuthority,
    /// A consumed grant lacks one exact terminal transaction and receipt.
    ConsumedGrantUnresolved,
    /// A launched or explicitly uncertain effect cannot be resumed safely.
    UncertainEffect,
    /// Receipt identity, ordering, digest, or terminal binding is inconsistent.
    ReceiptMismatch,
}

impl RuntimeRestartReconciliation {
    /// Reduces an existing restart-reconciler result without retaining its opaque permit.
    #[must_use]
    pub fn from_result<T>(result: &Result<T, RestartReconciliationError>) -> Self {
        match result {
            Ok(_) => Self::Verified,
            Err(RestartReconciliationError::InvalidSnapshot) => Self::InvalidSnapshot,
            Err(RestartReconciliationError::ContextMismatch) => Self::ContextMismatch,
            Err(RestartReconciliationError::PendingAuthority) => Self::PendingAuthority,
            Err(RestartReconciliationError::ConsumedGrantUnresolved) => {
                Self::ConsumedGrantUnresolved
            }
            Err(RestartReconciliationError::UncertainEffect) => Self::UncertainEffect,
            Err(RestartReconciliationError::ReceiptMismatch) => Self::ReceiptMismatch,
        }
    }
}

/// Durable authority state observed after restart reconciliation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeRecoveryAuthorityState {
    /// No operation authority exists for the interrupted boundary.
    None,
    /// Unconsumed authority existed and must not survive restart.
    Issued,
    /// Consumed authority has one exact verified terminal transaction and receipt.
    ConsumedResolved,
    /// Consumed authority lacks an exact verified terminal binding.
    ConsumedUnresolved,
    /// Authority or its effect is explicitly uncertain.
    Uncertain,
}

/// Consequential-effect state observed independently after interruption.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeRecoveryEffectState {
    /// No consequential effect crossed its launch boundary.
    NotStarted,
    /// Fresh observation proves that no canonical state changed.
    FailedNoChangeVerified,
    /// Fresh observation and receipts prove the exact effect completed.
    CompletedVerified,
    /// Completion or resulting state cannot be established safely.
    Uncertain,
}

/// Complete content-free facts used to choose one recovery action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeRecoveryObservation {
    /// Boundary at which the interruption occurred.
    pub boundary: RuntimeRecoveryBoundary,
    /// Last durably recorded agent state.
    pub agent_state: AgentStateKind,
    /// Result class from the existing exact restart reconciler.
    pub restart: RuntimeRestartReconciliation,
    /// Canonical event-journal integrity.
    pub journal: RuntimeRecoveryEvidenceState,
    /// Private artifact-set integrity.
    pub artifacts: RuntimeRecoveryEvidenceState,
    /// Atomic checkpoint and resume-binding integrity.
    pub checkpoint: RuntimeRecoveryEvidenceState,
    /// Current operation-authority state.
    pub authority: RuntimeRecoveryAuthorityState,
    /// Current consequential-effect state.
    pub effect: RuntimeRecoveryEffectState,
    /// Existing deterministic write-recovery instruction, when a write was interrupted.
    pub write_instruction: Option<WriteRecoveryInstruction>,
    /// Whether a complete verifier-bound terminal outcome was freshly verified.
    pub terminal_outcome_verified: bool,
}

/// Closed next action selected after one interrupted runtime observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeSafeNextAction {
    /// Resume only the exact verified safe-boundary continuation.
    ResumeVerifiedCheckpoint,
    /// Build a new proposal without reusing prior operation authority.
    BuildFreshProposal,
    /// Invalidate prior pending authority and obtain a new user decision.
    RequestFreshApproval,
    /// Follow the separately produced deterministic write-recovery instruction.
    FollowWriteRecoveryInstruction,
    /// Observe and reconcile a possibly completed effect without replaying it.
    ReconcileUncertainEffect,
    /// Return the already verified terminal outcome without executing again.
    ReturnVerifiedTerminal,
    /// Stop active execution and expose only bounded recovery diagnostics.
    EnterSafeMode,
}

/// One deterministic recovery decision carrying no operation authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRecoveryDecision {
    /// The sole permitted next action.
    pub action: RuntimeSafeNextAction,
    /// Stable content-free explanation code.
    pub reason_code: String,
    /// Exact write instruction to follow, only for its corresponding action.
    pub write_instruction: Option<WriteRecoveryInstruction>,
    /// Whether the exact safe-boundary continuation may be resumed.
    pub resume_allowed: bool,
    /// Whether any prior consequential effect may be replayed; always false.
    pub replay_effect_allowed: bool,
    /// Whether a later consequential effect requires separate fresh authority.
    pub requires_fresh_grant: bool,
}

/// Selects exactly one safe, non-authoritative action from freshly observed facts.
#[must_use]
pub fn plan_runtime_recovery(observation: &RuntimeRecoveryObservation) -> RuntimeRecoveryDecision {
    if let Some(reason) = invalid_observation_reason(observation) {
        return decision(RuntimeSafeNextAction::EnterSafeMode, reason, None, false);
    }

    if observation.checkpoint != RuntimeRecoveryEvidenceState::Verified {
        return safe_mode_for_evidence("checkpoint", observation.checkpoint);
    }
    if observation.journal != RuntimeRecoveryEvidenceState::Verified {
        return safe_mode_for_evidence("journal", observation.journal);
    }
    if observation.artifacts != RuntimeRecoveryEvidenceState::Verified {
        return safe_mode_for_evidence("artifact", observation.artifacts);
    }

    match observation.restart {
        RuntimeRestartReconciliation::InvalidSnapshot => {
            return safe_mode("runtime_recovery.restart.invalid_snapshot");
        }
        RuntimeRestartReconciliation::ContextMismatch => {
            return safe_mode("runtime_recovery.restart.context_mismatch");
        }
        RuntimeRestartReconciliation::ReceiptMismatch => {
            return safe_mode("runtime_recovery.restart.receipt_mismatch");
        }
        RuntimeRestartReconciliation::PendingAuthority => {
            return decision(
                RuntimeSafeNextAction::RequestFreshApproval,
                "runtime_recovery.restart.pending_authority_invalidated",
                None,
                true,
            );
        }
        RuntimeRestartReconciliation::ConsumedGrantUnresolved => {
            return reconcile("runtime_recovery.restart.consumed_grant_unresolved");
        }
        RuntimeRestartReconciliation::UncertainEffect => {
            return reconcile("runtime_recovery.restart.effect_uncertain");
        }
        RuntimeRestartReconciliation::Verified => {}
    }

    if matches!(
        observation.authority,
        RuntimeRecoveryAuthorityState::ConsumedUnresolved
            | RuntimeRecoveryAuthorityState::Uncertain
    ) {
        return reconcile("runtime_recovery.authority.unresolved");
    }
    if observation.effect == RuntimeRecoveryEffectState::Uncertain {
        return reconcile("runtime_recovery.effect.uncertain");
    }

    if observation.agent_state.is_terminal() {
        if observation.terminal_outcome_verified {
            return decision(
                RuntimeSafeNextAction::ReturnVerifiedTerminal,
                "runtime_recovery.terminal.verified",
                None,
                false,
            );
        }
        return safe_mode("runtime_recovery.terminal.unverified");
    }

    if observation.authority == RuntimeRecoveryAuthorityState::Issued {
        return decision(
            RuntimeSafeNextAction::RequestFreshApproval,
            "runtime_recovery.approval.fresh_decision_required",
            None,
            true,
        );
    }

    if let Some(instruction) = observation.write_instruction {
        return match instruction {
            WriteRecoveryInstruction::BlockUncertainState => {
                reconcile("runtime_recovery.write.uncertain")
            }
            WriteRecoveryInstruction::NoActionComplete => decision(
                RuntimeSafeNextAction::ResumeVerifiedCheckpoint,
                "runtime_recovery.write.complete",
                None,
                false,
            ),
            WriteRecoveryInstruction::BuildFreshProposal => decision(
                RuntimeSafeNextAction::BuildFreshProposal,
                "runtime_recovery.write.fresh_proposal",
                None,
                true,
            ),
            other => decision(
                RuntimeSafeNextAction::FollowWriteRecoveryInstruction,
                "runtime_recovery.write.follow_existing_plan",
                Some(other),
                other != WriteRecoveryInstruction::PreserveConflictForReview,
            ),
        };
    }

    if observation.authority == RuntimeRecoveryAuthorityState::ConsumedResolved {
        return decision(
            RuntimeSafeNextAction::ResumeVerifiedCheckpoint,
            "runtime_recovery.effect.resolved_continue_verification",
            None,
            false,
        );
    }

    match observation.boundary {
        RuntimeRecoveryBoundary::Approval => decision(
            RuntimeSafeNextAction::RequestFreshApproval,
            "runtime_recovery.approval.fresh_decision_required",
            None,
            true,
        ),
        RuntimeRecoveryBoundary::Write | RuntimeRecoveryBoundary::Command => decision(
            RuntimeSafeNextAction::BuildFreshProposal,
            "runtime_recovery.effect.not_started_fresh_proposal",
            None,
            true,
        ),
        RuntimeRecoveryBoundary::Terminal => {
            safe_mode("runtime_recovery.terminal.boundary_without_terminal_state")
        }
        RuntimeRecoveryBoundary::Model
        | RuntimeRecoveryBoundary::Context
        | RuntimeRecoveryBoundary::Validation
        | RuntimeRecoveryBoundary::Event
        | RuntimeRecoveryBoundary::Artifact
        | RuntimeRecoveryBoundary::Checkpoint => decision(
            RuntimeSafeNextAction::ResumeVerifiedCheckpoint,
            "runtime_recovery.safe_boundary.verified",
            None,
            false,
        ),
    }
}

fn invalid_observation_reason(observation: &RuntimeRecoveryObservation) -> Option<&'static str> {
    if observation.terminal_outcome_verified && !observation.agent_state.is_terminal() {
        return Some("runtime_recovery.observation.nonterminal_with_terminal_proof");
    }
    if observation.agent_state.is_terminal()
        && observation.authority == RuntimeRecoveryAuthorityState::Issued
    {
        return Some("runtime_recovery.observation.terminal_with_pending_authority");
    }
    if observation.authority == RuntimeRecoveryAuthorityState::ConsumedResolved
        && observation.effect == RuntimeRecoveryEffectState::NotStarted
    {
        return Some("runtime_recovery.observation.consumed_without_effect_result");
    }
    if matches!(
        observation.effect,
        RuntimeRecoveryEffectState::FailedNoChangeVerified
            | RuntimeRecoveryEffectState::CompletedVerified
    ) && !matches!(
        observation.authority,
        RuntimeRecoveryAuthorityState::ConsumedResolved
            | RuntimeRecoveryAuthorityState::ConsumedUnresolved
            | RuntimeRecoveryAuthorityState::Uncertain
    ) {
        return Some("runtime_recovery.observation.effect_without_consumed_authority");
    }
    if observation.write_instruction.is_some()
        && observation.boundary != RuntimeRecoveryBoundary::Write
    {
        return Some("runtime_recovery.observation.write_plan_wrong_boundary");
    }
    None
}

fn safe_mode_for_evidence(
    domain: &str,
    state: RuntimeRecoveryEvidenceState,
) -> RuntimeRecoveryDecision {
    let suffix = match state {
        RuntimeRecoveryEvidenceState::Verified => "verified",
        RuntimeRecoveryEvidenceState::Missing => "missing",
        RuntimeRecoveryEvidenceState::Stale => "stale",
        RuntimeRecoveryEvidenceState::Invalid => "invalid",
    };
    safe_mode(&format!("runtime_recovery.{domain}.{suffix}"))
}

fn safe_mode(reason_code: &str) -> RuntimeRecoveryDecision {
    decision(
        RuntimeSafeNextAction::EnterSafeMode,
        reason_code,
        None,
        false,
    )
}

fn reconcile(reason_code: &str) -> RuntimeRecoveryDecision {
    decision(
        RuntimeSafeNextAction::ReconcileUncertainEffect,
        reason_code,
        None,
        false,
    )
}

fn decision(
    action: RuntimeSafeNextAction,
    reason_code: &str,
    write_instruction: Option<WriteRecoveryInstruction>,
    requires_fresh_grant: bool,
) -> RuntimeRecoveryDecision {
    RuntimeRecoveryDecision {
        action,
        reason_code: reason_code.to_owned(),
        write_instruction,
        resume_allowed: action == RuntimeSafeNextAction::ResumeVerifiedCheckpoint,
        replay_effect_allowed: false,
        requires_fresh_grant,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(boundary: RuntimeRecoveryBoundary) -> RuntimeRecoveryObservation {
        RuntimeRecoveryObservation {
            boundary,
            agent_state: AgentStateKind::Observation,
            restart: RuntimeRestartReconciliation::Verified,
            journal: RuntimeRecoveryEvidenceState::Verified,
            artifacts: RuntimeRecoveryEvidenceState::Verified,
            checkpoint: RuntimeRecoveryEvidenceState::Verified,
            authority: RuntimeRecoveryAuthorityState::None,
            effect: RuntimeRecoveryEffectState::NotStarted,
            write_instruction: None,
            terminal_outcome_verified: false,
        }
    }

    #[test]
    fn story_50_2_every_boundary_selects_one_closed_safe_action() {
        let expected = [
            RuntimeSafeNextAction::ResumeVerifiedCheckpoint,
            RuntimeSafeNextAction::ResumeVerifiedCheckpoint,
            RuntimeSafeNextAction::RequestFreshApproval,
            RuntimeSafeNextAction::BuildFreshProposal,
            RuntimeSafeNextAction::BuildFreshProposal,
            RuntimeSafeNextAction::ResumeVerifiedCheckpoint,
            RuntimeSafeNextAction::ResumeVerifiedCheckpoint,
            RuntimeSafeNextAction::ResumeVerifiedCheckpoint,
            RuntimeSafeNextAction::ResumeVerifiedCheckpoint,
            RuntimeSafeNextAction::ReturnVerifiedTerminal,
        ];
        for (boundary, expected_action) in RuntimeRecoveryBoundary::ALL.into_iter().zip(expected) {
            let mut value = observation(boundary);
            if boundary == RuntimeRecoveryBoundary::Terminal {
                value.agent_state = AgentStateKind::Success;
                value.terminal_outcome_verified = true;
            }
            let result = plan_runtime_recovery(&value);
            assert_eq!(result.action, expected_action, "boundary {boundary:?}");
            assert!(!result.replay_effect_allowed);
            assert_eq!(
                result.resume_allowed,
                result.action == RuntimeSafeNextAction::ResumeVerifiedCheckpoint
            );
        }
    }

    #[test]
    fn story_50_2_restart_failures_never_fall_through_to_resume() {
        let cases = [
            (
                RuntimeRestartReconciliation::InvalidSnapshot,
                RuntimeSafeNextAction::EnterSafeMode,
            ),
            (
                RuntimeRestartReconciliation::ContextMismatch,
                RuntimeSafeNextAction::EnterSafeMode,
            ),
            (
                RuntimeRestartReconciliation::PendingAuthority,
                RuntimeSafeNextAction::RequestFreshApproval,
            ),
            (
                RuntimeRestartReconciliation::ConsumedGrantUnresolved,
                RuntimeSafeNextAction::ReconcileUncertainEffect,
            ),
            (
                RuntimeRestartReconciliation::UncertainEffect,
                RuntimeSafeNextAction::ReconcileUncertainEffect,
            ),
            (
                RuntimeRestartReconciliation::ReceiptMismatch,
                RuntimeSafeNextAction::EnterSafeMode,
            ),
        ];
        for (restart, expected) in cases {
            let mut value = observation(RuntimeRecoveryBoundary::Event);
            value.restart = restart;
            let result = plan_runtime_recovery(&value);
            assert_eq!(result.action, expected, "restart {restart:?}");
            assert!(!result.resume_allowed);
            assert!(!result.replay_effect_allowed);
        }
    }

    #[test]
    fn story_50_2_restart_result_mapping_is_exhaustive() {
        assert_eq!(
            RuntimeRestartReconciliation::from_result(&Ok::<_, RestartReconciliationError>(())),
            RuntimeRestartReconciliation::Verified
        );
        let cases = [
            (
                RestartReconciliationError::InvalidSnapshot,
                RuntimeRestartReconciliation::InvalidSnapshot,
            ),
            (
                RestartReconciliationError::ContextMismatch,
                RuntimeRestartReconciliation::ContextMismatch,
            ),
            (
                RestartReconciliationError::PendingAuthority,
                RuntimeRestartReconciliation::PendingAuthority,
            ),
            (
                RestartReconciliationError::ConsumedGrantUnresolved,
                RuntimeRestartReconciliation::ConsumedGrantUnresolved,
            ),
            (
                RestartReconciliationError::UncertainEffect,
                RuntimeRestartReconciliation::UncertainEffect,
            ),
            (
                RestartReconciliationError::ReceiptMismatch,
                RuntimeRestartReconciliation::ReceiptMismatch,
            ),
        ];
        for (error, expected) in cases {
            assert_eq!(
                RuntimeRestartReconciliation::from_result(&Err::<(), _>(error)),
                expected
            );
        }
    }

    #[test]
    fn story_50_2_missing_stale_or_invalid_durable_evidence_enters_safe_mode() {
        for state in [
            RuntimeRecoveryEvidenceState::Missing,
            RuntimeRecoveryEvidenceState::Stale,
            RuntimeRecoveryEvidenceState::Invalid,
        ] {
            for domain in ["checkpoint", "journal", "artifacts"] {
                let mut value = observation(RuntimeRecoveryBoundary::Checkpoint);
                match domain {
                    "checkpoint" => value.checkpoint = state,
                    "journal" => value.journal = state,
                    "artifacts" => value.artifacts = state,
                    _ => unreachable!(),
                }
                let result = plan_runtime_recovery(&value);
                assert_eq!(result.action, RuntimeSafeNextAction::EnterSafeMode);
                assert!(!result.resume_allowed);
                assert!(!result.replay_effect_allowed);
            }
        }
    }

    #[test]
    fn story_50_2_consumed_or_uncertain_effect_is_never_replayed() {
        let cases = [
            (
                RuntimeRecoveryAuthorityState::ConsumedUnresolved,
                RuntimeRecoveryEffectState::Uncertain,
            ),
            (
                RuntimeRecoveryAuthorityState::Uncertain,
                RuntimeRecoveryEffectState::Uncertain,
            ),
            (
                RuntimeRecoveryAuthorityState::ConsumedResolved,
                RuntimeRecoveryEffectState::CompletedVerified,
            ),
            (
                RuntimeRecoveryAuthorityState::ConsumedResolved,
                RuntimeRecoveryEffectState::FailedNoChangeVerified,
            ),
        ];
        for (authority, effect) in cases {
            let mut value = observation(RuntimeRecoveryBoundary::Command);
            value.authority = authority;
            value.effect = effect;
            let result = plan_runtime_recovery(&value);
            assert!(!result.replay_effect_allowed);
            assert_ne!(result.action, RuntimeSafeNextAction::BuildFreshProposal);
            if authority == RuntimeRecoveryAuthorityState::ConsumedResolved {
                assert_eq!(
                    result.action,
                    RuntimeSafeNextAction::ResumeVerifiedCheckpoint
                );
            } else {
                assert_eq!(
                    result.action,
                    RuntimeSafeNextAction::ReconcileUncertainEffect
                );
            }
        }
    }

    #[test]
    fn story_50_2_terminal_result_requires_exact_proof_and_no_pending_authority() {
        let mut value = observation(RuntimeRecoveryBoundary::Terminal);
        value.agent_state = AgentStateKind::Success;
        assert_eq!(
            plan_runtime_recovery(&value).action,
            RuntimeSafeNextAction::EnterSafeMode
        );

        value.terminal_outcome_verified = true;
        assert_eq!(
            plan_runtime_recovery(&value).action,
            RuntimeSafeNextAction::ReturnVerifiedTerminal
        );

        value.authority = RuntimeRecoveryAuthorityState::Issued;
        assert_eq!(
            plan_runtime_recovery(&value).action,
            RuntimeSafeNextAction::EnterSafeMode
        );
    }

    #[test]
    fn story_50_2_write_recovery_plan_is_composed_without_gaining_authority() {
        let cases = [
            (
                WriteRecoveryInstruction::BuildFreshProposal,
                RuntimeSafeNextAction::BuildFreshProposal,
                None,
            ),
            (
                WriteRecoveryInstruction::RestoreWorkspaceIdentity,
                RuntimeSafeNextAction::FollowWriteRecoveryInstruction,
                Some(WriteRecoveryInstruction::RestoreWorkspaceIdentity),
            ),
            (
                WriteRecoveryInstruction::PreserveConflictForReview,
                RuntimeSafeNextAction::FollowWriteRecoveryInstruction,
                Some(WriteRecoveryInstruction::PreserveConflictForReview),
            ),
            (
                WriteRecoveryInstruction::RestoreSecretStore,
                RuntimeSafeNextAction::FollowWriteRecoveryInstruction,
                Some(WriteRecoveryInstruction::RestoreSecretStore),
            ),
            (
                WriteRecoveryInstruction::VerifyCanonicalState,
                RuntimeSafeNextAction::FollowWriteRecoveryInstruction,
                Some(WriteRecoveryInstruction::VerifyCanonicalState),
            ),
            (
                WriteRecoveryInstruction::PublishDerivedIndex,
                RuntimeSafeNextAction::FollowWriteRecoveryInstruction,
                Some(WriteRecoveryInstruction::PublishDerivedIndex),
            ),
            (
                WriteRecoveryInstruction::PersistTerminalReceipt,
                RuntimeSafeNextAction::FollowWriteRecoveryInstruction,
                Some(WriteRecoveryInstruction::PersistTerminalReceipt),
            ),
            (
                WriteRecoveryInstruction::ProposeStagingCleanup,
                RuntimeSafeNextAction::FollowWriteRecoveryInstruction,
                Some(WriteRecoveryInstruction::ProposeStagingCleanup),
            ),
            (
                WriteRecoveryInstruction::BlockUncertainState,
                RuntimeSafeNextAction::ReconcileUncertainEffect,
                None,
            ),
            (
                WriteRecoveryInstruction::NoActionComplete,
                RuntimeSafeNextAction::ResumeVerifiedCheckpoint,
                None,
            ),
        ];
        for (instruction, expected_action, expected_instruction) in cases {
            let mut value = observation(RuntimeRecoveryBoundary::Write);
            value.write_instruction = Some(instruction);
            let result = plan_runtime_recovery(&value);
            assert_eq!(
                result.action, expected_action,
                "instruction {instruction:?}"
            );
            assert_eq!(result.write_instruction, expected_instruction);
            assert!(!result.replay_effect_allowed);
            assert_eq!(
                result.requires_fresh_grant,
                !matches!(
                    instruction,
                    WriteRecoveryInstruction::PreserveConflictForReview
                        | WriteRecoveryInstruction::BlockUncertainState
                        | WriteRecoveryInstruction::NoActionComplete
                )
            );
        }
    }

    #[test]
    fn story_50_2_incoherent_observations_fail_closed_before_recovery() {
        let mut cases = Vec::new();

        let mut nonterminal_proof = observation(RuntimeRecoveryBoundary::Event);
        nonterminal_proof.terminal_outcome_verified = true;
        cases.push(nonterminal_proof);

        let mut consumed_without_result = observation(RuntimeRecoveryBoundary::Command);
        consumed_without_result.authority = RuntimeRecoveryAuthorityState::ConsumedResolved;
        cases.push(consumed_without_result);

        let mut effect_without_authority = observation(RuntimeRecoveryBoundary::Command);
        effect_without_authority.effect = RuntimeRecoveryEffectState::CompletedVerified;
        cases.push(effect_without_authority);

        let mut misplaced_write_plan = observation(RuntimeRecoveryBoundary::Model);
        misplaced_write_plan.write_instruction = Some(WriteRecoveryInstruction::NoActionComplete);
        cases.push(misplaced_write_plan);

        for value in cases {
            let result = plan_runtime_recovery(&value);
            assert_eq!(result.action, RuntimeSafeNextAction::EnterSafeMode);
            assert!(!result.resume_allowed);
            assert!(!result.replay_effect_allowed);
        }
    }
}
