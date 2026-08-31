//! Fail-closed reconciliation for durable workflow checkpoints and restart observations.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, CanonicalWorkflowCheckpoint, CanonicalWorkflowLifecycle,
    CanonicalWorkflowState, RuntimeArtifactRef, RuntimeEventCursor, RuntimeResumeBinding,
    SessionCheckpoint,
};

use crate::context_management::verify_checkpoint;
use crate::engineering_records::{ValidateCanonicalRecord, canonical_record_sha256};
use crate::runtime_artifact::verify_runtime_resume_binding;

const MAX_RESUME_IDENTITIES: usize = 256;

/// One identity family that changed after a durable workflow checkpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorkflowResumeDrift {
    /// The canonical workflow-state digest changed.
    WorkflowState,
    /// The authoritative source manifest changed.
    SourceManifest,
    /// The effective workflow plan or current step changed.
    Plan,
    /// The verifier/evidence identity set changed.
    VerificationEvidence,
    /// The admitted tool catalog changed.
    ToolCatalog,
    /// The selected model route changed.
    ModelRoute,
    /// The deterministic policy changed.
    Policy,
    /// The captured runtime environment changed.
    Environment,
    /// The ordered runtime journal advanced, rewound, or forked.
    Journal,
    /// A terminal receipt set changed.
    Receipt,
    /// A consumed-grant set changed.
    ConsumedGrant,
    /// A required immutable runtime artifact changed.
    Artifact,
}

/// Effect truth observed independently before restart dispatch is considered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkflowEffectObservation {
    /// No effect was admitted for the checkpointed step.
    NoEffect,
    /// Preflight proved that an admitted effect did not occur.
    VerifiedNotApplied,
    /// Postcondition and terminal receipt prove that the effect completed.
    VerifiedApplied {
        /// Digest of the independently verified terminal receipt.
        receipt_sha256: String,
    },
    /// The effect may have happened and must not be replayed.
    Uncertain {
        /// Stable content-free evidence identity for later reconciliation.
        evidence_sha256: String,
    },
}

/// Current trusted facts compared with one durable workflow checkpoint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkflowResumeObservation {
    /// Current canonical workflow state.
    pub workflow_state: CanonicalWorkflowState,
    /// Current authoritative source-manifest digest.
    pub source_manifest_sha256: String,
    /// Current exact plan identity.
    pub plan_id: String,
    /// Current exact plan revision.
    pub plan_revision: u32,
    /// Current exact active-step identity.
    pub plan_step_id: String,
    /// Current admitted tool-catalog digest.
    pub tool_catalog_sha256: String,
    /// Current selected model-route digest.
    pub route_sha256: String,
    /// Current deterministic policy digest.
    pub policy_sha256: String,
    /// Current trusted environment digest.
    pub environment_sha256: String,
    /// Current exact journal head.
    pub event_cursor: RuntimeEventCursor,
    /// Current terminal receipt digests in stable order.
    pub receipt_sha256s: Vec<String>,
    /// Current consumed-grant digests in stable order.
    pub consumed_grant_sha256s: Vec<String>,
    /// Current verifier/evidence identities in stable order.
    pub evidence_ids: Vec<String>,
    /// Current immutable artifact references in stable identity order.
    pub artifacts: Vec<RuntimeArtifactRef>,
    /// Whether the workflow budget is exhausted at this checkpoint.
    pub budget_exhausted: bool,
    /// Independently observed effect truth.
    pub effect: WorkflowEffectObservation,
}

/// One closed safe action selected after restart reconciliation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowResumeAction {
    /// Continue from the exact verified checkpoint without replaying prior effects.
    ResumeVerifiedCheckpoint,
    /// Publish terminal verification for an effect that already completed.
    FinalizeVerifiedEffect,
    /// Keep the task blocked while an uncertain effect is reconciled.
    ReconcileUncertainEffect,
    /// Replan from current observations and publish a fresh checkpoint.
    ReplanAndRecheckpoint,
    /// Stop because the declared workflow budget is exhausted.
    StopBudgetExhausted,
    /// Preserve the already terminal workflow without dispatching more work.
    PreserveTerminalResult,
}

/// Content-free restart result; it carries no dispatch or capability authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkflowResumeDecision {
    /// The only safe next action selected by the reconciliation boundary.
    pub action: WorkflowResumeAction,
    /// Stable content-free reason for the decision.
    pub reason_code: &'static str,
    /// Exact sorted identity families that became stale.
    pub drift: Vec<WorkflowResumeDrift>,
    /// Verified receipt to finalize without replay, when applicable.
    pub verified_receipt_sha256: Option<String>,
    /// Whether another effect dispatch is permitted after this decision.
    pub dispatch_permitted: bool,
}

/// Verifies one durable checkpoint against current identities and selects exactly one safe action.
///
/// This boundary is deliberately non-authoritative. A `ResumeVerifiedCheckpoint` result still
/// requires the ordinary kernel authority transaction before a later caller can dispatch work.
pub fn reconcile_workflow_resume(
    session_checkpoint: &SessionCheckpoint,
    workflow_checkpoint: &CanonicalWorkflowCheckpoint,
    binding: &RuntimeResumeBinding,
    current: &WorkflowResumeObservation,
) -> WorkflowResumeDecision {
    if durable_records_valid(session_checkpoint, workflow_checkpoint, binding, current).is_err() {
        return decision(
            WorkflowResumeAction::ReplanAndRecheckpoint,
            "workflow.resume.durable_state_invalid",
            Vec::new(),
            None,
            false,
        );
    }

    if workflow_terminal(current.workflow_state.state) {
        return decision(
            WorkflowResumeAction::PreserveTerminalResult,
            "workflow.resume.already_terminal",
            Vec::new(),
            None,
            false,
        );
    }

    if current.budget_exhausted {
        return decision(
            WorkflowResumeAction::StopBudgetExhausted,
            "workflow.resume.budget_exhausted",
            Vec::new(),
            None,
            false,
        );
    }

    let drift = resume_drift(session_checkpoint, workflow_checkpoint, binding, current);
    if !drift.is_empty() {
        return decision(
            WorkflowResumeAction::ReplanAndRecheckpoint,
            "workflow.resume.identity_drift",
            drift,
            None,
            false,
        );
    }

    match &current.effect {
        WorkflowEffectObservation::Uncertain { .. } => decision(
            WorkflowResumeAction::ReconcileUncertainEffect,
            "workflow.resume.effect_uncertain",
            Vec::new(),
            None,
            false,
        ),
        WorkflowEffectObservation::VerifiedApplied { receipt_sha256 } => decision(
            WorkflowResumeAction::FinalizeVerifiedEffect,
            "workflow.resume.effect_already_applied",
            Vec::new(),
            Some(receipt_sha256.clone()),
            false,
        ),
        WorkflowEffectObservation::NoEffect | WorkflowEffectObservation::VerifiedNotApplied => {
            decision(
                WorkflowResumeAction::ResumeVerifiedCheckpoint,
                "workflow.resume.checkpoint_current",
                Vec::new(),
                None,
                true,
            )
        }
    }
}

fn durable_records_valid(
    session_checkpoint: &SessionCheckpoint,
    workflow_checkpoint: &CanonicalWorkflowCheckpoint,
    binding: &RuntimeResumeBinding,
    current: &WorkflowResumeObservation,
) -> Result<(), ()> {
    verify_checkpoint(session_checkpoint).map_err(|_| ())?;
    workflow_checkpoint.validate_canonical().map_err(|_| ())?;
    current
        .workflow_state
        .validate_canonical()
        .map_err(|_| ())?;
    verify_runtime_resume_binding(binding).map_err(|_| ())?;
    if workflow_checkpoint.workflow_id != current.workflow_state.workflow_id
        || workflow_checkpoint.journal_sequence != binding.event_cursor.sequence
        || binding.checkpoint_id != session_checkpoint.checkpoint_id
        || binding.checkpoint_sha256 != session_checkpoint.checkpoint_sha256
        || binding.session_id != session_checkpoint.session_id
        || binding.task_id != session_checkpoint.task_id
        || current.event_cursor.run_id != binding.run_id
        || !valid_identifier(&current.plan_id)
        || current.plan_revision == 0
        || !valid_identifier(&current.plan_step_id)
        || !valid_sha256_list(&current.receipt_sha256s)
        || !valid_sha256_list(&current.consumed_grant_sha256s)
        || !valid_identifier_list(&current.evidence_ids)
        || !valid_artifacts(&current.artifacts)
    {
        return Err(());
    }
    match &current.effect {
        WorkflowEffectObservation::NoEffect | WorkflowEffectObservation::VerifiedNotApplied => {}
        WorkflowEffectObservation::VerifiedApplied { receipt_sha256 } => {
            if !valid_sha256(receipt_sha256) || !current.receipt_sha256s.contains(receipt_sha256) {
                return Err(());
            }
        }
        WorkflowEffectObservation::Uncertain { evidence_sha256 } => {
            if !valid_sha256(evidence_sha256) {
                return Err(());
            }
        }
    }
    Ok(())
}

fn resume_drift(
    session_checkpoint: &SessionCheckpoint,
    workflow_checkpoint: &CanonicalWorkflowCheckpoint,
    binding: &RuntimeResumeBinding,
    current: &WorkflowResumeObservation,
) -> Vec<WorkflowResumeDrift> {
    let mut drift = BTreeSet::new();
    if workflow_checkpoint.state_sha256
        != canonical_record_sha256(&current.workflow_state).unwrap_or_default()
    {
        drift.insert(WorkflowResumeDrift::WorkflowState);
    }
    if workflow_checkpoint.source_manifest_sha256 != current.source_manifest_sha256 {
        drift.insert(WorkflowResumeDrift::SourceManifest);
    }
    if session_checkpoint.plan_id.as_str() != current.plan_id
        || session_checkpoint.plan_revision != current.plan_revision
        || session_checkpoint.plan_step_id.as_str() != current.plan_step_id
    {
        drift.insert(WorkflowResumeDrift::Plan);
    }
    if workflow_checkpoint.tool_catalog_sha256 != current.tool_catalog_sha256 {
        drift.insert(WorkflowResumeDrift::ToolCatalog);
    }
    if workflow_checkpoint.route_sha256 != current.route_sha256 {
        drift.insert(WorkflowResumeDrift::ModelRoute);
    }
    if workflow_checkpoint.policy_sha256 != current.policy_sha256
        || session_checkpoint.policy_sha256 != current.policy_sha256
    {
        drift.insert(WorkflowResumeDrift::Policy);
    }
    if workflow_checkpoint.environment_sha256 != current.environment_sha256
        || session_checkpoint.workspace_state_sha256 != current.environment_sha256
    {
        drift.insert(WorkflowResumeDrift::Environment);
    }
    if binding.event_cursor != current.event_cursor {
        drift.insert(WorkflowResumeDrift::Journal);
    }
    if workflow_checkpoint.receipt_sha256s != current.receipt_sha256s {
        drift.insert(WorkflowResumeDrift::Receipt);
    }
    if workflow_checkpoint.consumed_grant_sha256s != current.consumed_grant_sha256s {
        drift.insert(WorkflowResumeDrift::ConsumedGrant);
    }
    if session_checkpoint
        .evidence_ids
        .iter()
        .map(|identity| identity.as_str())
        .ne(current.evidence_ids.iter().map(String::as_str))
    {
        drift.insert(WorkflowResumeDrift::VerificationEvidence);
    }
    if binding.artifacts != current.artifacts {
        drift.insert(WorkflowResumeDrift::Artifact);
    }
    drift.into_iter().collect()
}

fn decision(
    action: WorkflowResumeAction,
    reason_code: &'static str,
    drift: Vec<WorkflowResumeDrift>,
    verified_receipt_sha256: Option<String>,
    dispatch_permitted: bool,
) -> WorkflowResumeDecision {
    WorkflowResumeDecision {
        action,
        reason_code,
        drift,
        verified_receipt_sha256,
        dispatch_permitted,
    }
}

const fn workflow_terminal(state: CanonicalWorkflowLifecycle) -> bool {
    matches!(
        state,
        CanonicalWorkflowLifecycle::Succeeded
            | CanonicalWorkflowLifecycle::NoOp
            | CanonicalWorkflowLifecycle::Blocked
            | CanonicalWorkflowLifecycle::Denied
            | CanonicalWorkflowLifecycle::Failed
            | CanonicalWorkflowLifecycle::Cancelled
            | CanonicalWorkflowLifecycle::TimedOut
            | CanonicalWorkflowLifecycle::ResourceExhausted
            | CanonicalWorkflowLifecycle::Uncertain
    )
}

fn valid_sha256_list(values: &[String]) -> bool {
    values.len() <= MAX_RESUME_IDENTITIES
        && values.iter().all(|value| valid_sha256(value))
        && values.windows(2).all(|pair| pair[0] < pair[1])
}

fn valid_identifier_list(values: &[String]) -> bool {
    values.len() <= MAX_RESUME_IDENTITIES
        && values.iter().all(|value| valid_identifier(value))
        && values.windows(2).all(|pair| pair[0] < pair[1])
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.'))
}

fn valid_artifacts(artifacts: &[RuntimeArtifactRef]) -> bool {
    artifacts.len() <= MAX_RESUME_IDENTITIES
        && artifacts.iter().all(|artifact| {
            artifact.schema_version == CONTRACT_SCHEMA_VERSION
                && !artifact.artifact_id.as_str().is_empty()
                && valid_sha256(&artifact.manifest_sha256)
                && valid_sha256(&artifact.payload_sha256)
                && artifact.byte_size > 0
                && !artifact.media_type.is_empty()
                && artifact.media_type.len() <= 256
                && !artifact.media_type.chars().any(char::is_control)
        })
        && artifacts
            .windows(2)
            .all(|pair| pair[0].artifact_id < pair[1].artifact_id)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{
        CONTRACT_SCHEMA_VERSION, CanonicalWorkflowCheckpoint, CanonicalWorkflowLifecycle,
        CanonicalWorkflowState, ModelProfileId, PlanId, PlanStepId, PolicyId, RepositorySnapshotId,
        RuntimeEventCursor, RuntimeEventId, RuntimeResumeBinding, RuntimeRunId, SessionCheckpoint,
        SessionCheckpointId, SessionId, TaskId, WorkspaceId,
    };

    use super::{
        WorkflowEffectObservation, WorkflowResumeAction, WorkflowResumeDrift,
        WorkflowResumeObservation, reconcile_workflow_resume,
    };
    use crate::context_management::finalize_checkpoint;
    use crate::engineering_records::canonical_record_sha256;
    use crate::runtime_artifact::seal_runtime_resume_binding;

    fn hash(byte: char) -> String {
        byte.to_string().repeat(64)
    }

    struct Fixture {
        session_checkpoint: SessionCheckpoint,
        workflow_checkpoint: CanonicalWorkflowCheckpoint,
        binding: RuntimeResumeBinding,
        current: WorkflowResumeObservation,
    }

    fn fixture() -> Fixture {
        let state = CanonicalWorkflowState {
            schema_version: CONTRACT_SCHEMA_VERSION,
            workflow_id: "workflow-resume-1".to_owned(),
            workflow_version: 1,
            sequence: 7,
            state: CanonicalWorkflowLifecycle::Running,
            active_step_id: Some("step-resume-1".to_owned()),
            completed_step_ids: vec!["step-resume-0".to_owned()],
            attempt_ids: vec!["attempt-resume-1".to_owned()],
            consumed_budget_sha256: hash('1'),
            terminal_result_id: None,
        };
        let source_manifest_sha256 = hash('4');
        let policy_sha256 = hash('5');
        let environment_sha256 = hash('6');
        let route_sha256 = hash('7');
        let tool_catalog_sha256 = hash('8');
        let receipt_sha256s = vec![hash('9')];
        let consumed_grant_sha256s = vec![hash('a')];
        let event_cursor = RuntimeEventCursor {
            run_id: RuntimeRunId::from_raw("run-resume-1"),
            event_id: RuntimeEventId::from_raw("event-resume-7"),
            sequence: 7,
            event_sha256: hash('b'),
        };
        let session_checkpoint = finalize_checkpoint(SessionCheckpoint {
            schema_version: CONTRACT_SCHEMA_VERSION,
            checkpoint_id: SessionCheckpointId::from_raw("session-checkpoint-resume-1"),
            session_id: SessionId::from_raw("session-resume-1"),
            task_id: TaskId::from_raw("task-resume-1"),
            objective_sha256: hash('2'),
            plan_id: PlanId::from_raw("plan-resume-1"),
            plan_revision: 1,
            plan_step_id: PlanStepId::from_raw("step-resume-1"),
            next_action_sha256: hash('3'),
            workspace_id: WorkspaceId::from_raw("workspace-resume-1"),
            workspace_state_sha256: environment_sha256.clone(),
            repository_snapshot_id: RepositorySnapshotId::from_raw("repository-resume-1"),
            repository_branch: "main".to_owned(),
            repository_map_sha256: hash('c'),
            files: Vec::new(),
            instruction_sha256: hash('d'),
            permission_profile_id: "permission-resume-1".to_owned(),
            permission_profile_sha256: hash('e'),
            policy_id: PolicyId::from_raw("policy-resume-1"),
            policy_sha256: policy_sha256.clone(),
            model_profile_id: ModelProfileId::from_raw("model-resume-1"),
            model_manifest_sha256: hash('f'),
            model_runtime_sha256: hash('0'),
            evidence_ids: vec![agentmage_kernel_contracts::EvidenceId::from_raw(
                "verification-evidence-resume-1",
            )],
            citation_set_sha256: hash('1'),
            blockers: Vec::new(),
            context_packet_sha256: hash('2'),
            action_id: None,
            action_state: None,
            consumed_grant_id: None,
            receipt_id: None,
            receipt_sha256: None,
            ephemeral: false,
            checkpoint_sha256: hash('0'),
        })
        .expect("session checkpoint");
        let workflow_checkpoint = CanonicalWorkflowCheckpoint {
            schema_version: CONTRACT_SCHEMA_VERSION,
            checkpoint_id: "workflow-checkpoint-resume-1".to_owned(),
            workflow_id: state.workflow_id.clone(),
            state_sha256: canonical_record_sha256(&state).expect("state digest"),
            journal_sequence: event_cursor.sequence,
            source_manifest_sha256: source_manifest_sha256.clone(),
            policy_sha256: policy_sha256.clone(),
            environment_sha256: environment_sha256.clone(),
            route_sha256: route_sha256.clone(),
            tool_catalog_sha256: tool_catalog_sha256.clone(),
            receipt_sha256s: receipt_sha256s.clone(),
            consumed_grant_sha256s: consumed_grant_sha256s.clone(),
            created_at: "2026-08-31T00:00:00Z".to_owned(),
        };
        let binding = seal_runtime_resume_binding(RuntimeResumeBinding {
            schema_version: CONTRACT_SCHEMA_VERSION,
            checkpoint_id: session_checkpoint.checkpoint_id.clone(),
            checkpoint_sha256: session_checkpoint.checkpoint_sha256.clone(),
            session_id: session_checkpoint.session_id.clone(),
            task_id: session_checkpoint.task_id.clone(),
            run_id: event_cursor.run_id.clone(),
            event_cursor: event_cursor.clone(),
            artifacts: Vec::new(),
            binding_sha256: hash('0'),
        })
        .expect("runtime resume binding");
        let current = WorkflowResumeObservation {
            workflow_state: state,
            source_manifest_sha256,
            plan_id: "plan-resume-1".to_owned(),
            plan_revision: 1,
            plan_step_id: "step-resume-1".to_owned(),
            tool_catalog_sha256,
            route_sha256,
            policy_sha256,
            environment_sha256,
            event_cursor,
            receipt_sha256s,
            consumed_grant_sha256s,
            evidence_ids: vec!["verification-evidence-resume-1".to_owned()],
            artifacts: Vec::new(),
            budget_exhausted: false,
            effect: WorkflowEffectObservation::NoEffect,
        };
        Fixture {
            session_checkpoint,
            workflow_checkpoint,
            binding,
            current,
        }
    }

    fn decide(fixture: &Fixture) -> super::WorkflowResumeDecision {
        reconcile_workflow_resume(
            &fixture.session_checkpoint,
            &fixture.workflow_checkpoint,
            &fixture.binding,
            &fixture.current,
        )
    }

    #[test]
    fn story_11_3_current_checkpoint_is_the_only_dispatch_permitting_result() {
        let fixture = fixture();
        let decision = decide(&fixture);
        assert_eq!(
            decision.action,
            WorkflowResumeAction::ResumeVerifiedCheckpoint
        );
        assert_eq!(decision.reason_code, "workflow.resume.checkpoint_current");
        assert!(decision.dispatch_permitted);
        assert!(decision.drift.is_empty());
        assert_eq!(decision.verified_receipt_sha256, None);
    }

    #[test]
    fn story_11_3_every_identity_family_invalidates_resume_transitively() {
        let mut fixture = fixture();
        fixture.current.workflow_state.sequence += 1;
        fixture.current.source_manifest_sha256 = hash('a');
        fixture.current.plan_id = "plan-resume-changed".to_owned();
        fixture.current.plan_step_id = "step-resume-changed".to_owned();
        fixture.current.tool_catalog_sha256 = hash('d');
        fixture.current.route_sha256 = hash('e');
        fixture.current.policy_sha256 = hash('f');
        fixture.current.environment_sha256 = hash('0');
        fixture.current.event_cursor.event_sha256 = hash('1');
        fixture.current.receipt_sha256s = vec![hash('2')];
        fixture.current.consumed_grant_sha256s = vec![hash('3')];
        fixture.current.evidence_ids = vec!["verification-evidence-changed".to_owned()];
        let decision = decide(&fixture);
        assert_eq!(decision.action, WorkflowResumeAction::ReplanAndRecheckpoint);
        assert_eq!(
            decision.drift,
            vec![
                WorkflowResumeDrift::WorkflowState,
                WorkflowResumeDrift::SourceManifest,
                WorkflowResumeDrift::Plan,
                WorkflowResumeDrift::VerificationEvidence,
                WorkflowResumeDrift::ToolCatalog,
                WorkflowResumeDrift::ModelRoute,
                WorkflowResumeDrift::Policy,
                WorkflowResumeDrift::Environment,
                WorkflowResumeDrift::Journal,
                WorkflowResumeDrift::Receipt,
                WorkflowResumeDrift::ConsumedGrant,
            ]
        );
        assert!(!decision.dispatch_permitted);
    }

    #[test]
    fn story_11_3_uncertain_or_completed_effect_never_replays() {
        let mut fixture = fixture();
        fixture.current.effect = WorkflowEffectObservation::Uncertain {
            evidence_sha256: hash('c'),
        };
        let uncertain = decide(&fixture);
        assert_eq!(
            uncertain.action,
            WorkflowResumeAction::ReconcileUncertainEffect
        );
        assert!(!uncertain.dispatch_permitted);

        fixture.current.effect = WorkflowEffectObservation::VerifiedApplied {
            receipt_sha256: fixture.current.receipt_sha256s[0].clone(),
        };
        let applied = decide(&fixture);
        assert_eq!(applied.action, WorkflowResumeAction::FinalizeVerifiedEffect);
        assert_eq!(
            applied.verified_receipt_sha256,
            Some(fixture.current.receipt_sha256s[0].clone())
        );
        assert!(!applied.dispatch_permitted);
    }

    #[test]
    fn story_11_3_budget_and_terminal_state_are_absorbing() {
        let mut fixture = fixture();
        fixture.current.budget_exhausted = true;
        let exhausted = decide(&fixture);
        assert_eq!(exhausted.action, WorkflowResumeAction::StopBudgetExhausted);
        assert!(!exhausted.dispatch_permitted);

        fixture.current.budget_exhausted = false;
        fixture.current.workflow_state.state = CanonicalWorkflowLifecycle::Succeeded;
        fixture.current.workflow_state.active_step_id = None;
        fixture.current.workflow_state.terminal_result_id = Some("terminal-resume-1".to_owned());
        fixture.workflow_checkpoint.state_sha256 =
            canonical_record_sha256(&fixture.current.workflow_state).expect("terminal state");
        let terminal = decide(&fixture);
        assert_eq!(
            terminal.action,
            WorkflowResumeAction::PreserveTerminalResult
        );
        assert!(!terminal.dispatch_permitted);
    }

    #[test]
    fn story_11_3_malformed_or_mismatched_durable_records_fail_closed() {
        let mut invalid_binding_fixture = fixture();
        invalid_binding_fixture.binding.checkpoint_sha256 = hash('f');
        let decision = decide(&invalid_binding_fixture);
        assert_eq!(decision.action, WorkflowResumeAction::ReplanAndRecheckpoint);
        assert_eq!(
            decision.reason_code,
            "workflow.resume.durable_state_invalid"
        );
        assert!(!decision.dispatch_permitted);

        let mut invalid_effect_fixture = fixture();
        invalid_effect_fixture.current.effect = WorkflowEffectObservation::VerifiedApplied {
            receipt_sha256: hash('f'),
        };
        let decision = decide(&invalid_effect_fixture);
        assert_eq!(
            decision.reason_code,
            "workflow.resume.durable_state_invalid"
        );
        assert!(!decision.dispatch_permitted);
    }
}
