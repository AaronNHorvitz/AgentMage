//! Exact runtime-request framing for one bounded coding session.

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, PolicyId, RuntimeRunId, RuntimeRunRequest, RuntimeSessionMode,
    SessionId, Task, TaskId, TaskStatus, WorkPacket,
};
use agentmage_kernel_engine::runtime_coordinator::seal_runtime_run_request;

use crate::coding_session::CodingSessionProfile;

/// User- and policy-owned inputs needed to frame one ephemeral coding run.
pub struct CodingRunRequestInput {
    /// Fresh runtime run identity.
    pub run_id: RuntimeRunId,
    /// Owning authenticated local session identity.
    pub session_id: SessionId,
    /// Exact current user objective.
    pub objective: String,
    /// Exact criteria whose deterministic evidence permits completion.
    pub acceptance_criteria: Vec<String>,
    /// User-visible exclusions and operating constraints.
    pub constraints: Vec<String>,
    /// Current validated, planned, or active descriptive work packet.
    pub work_packet: WorkPacket,
    /// Exact run policy identity composed by the trusted authority layer.
    pub policy_id: PolicyId,
    /// Digest of that complete policy revision.
    pub policy_sha256: String,
}

/// Stable content-free coding-run framing refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodingRunRequestError {
    /// Task, packet, profile, or policy bindings disagree.
    BindingDenied,
    /// The common runtime request contract rejected the final envelope.
    ContractDenied,
}

impl CodingRunRequestError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::BindingDenied => "runtime.coding-run.binding-denied",
            Self::ContractDenied => "runtime.coding-run.contract-denied",
        }
    }
}

/// Seals one interface-neutral ephemeral controlled-write runtime request.
pub fn build_ephemeral_coding_run_request(
    profile: &CodingSessionProfile,
    input: CodingRunRequestInput,
) -> Result<RuntimeRunRequest, CodingRunRequestError> {
    let task_id = TaskId::from_raw(profile.worktree().task_id.clone());
    if input.objective.is_empty()
        || input.acceptance_criteria.is_empty()
        || input.work_packet.task_id != task_id
        || input.work_packet.objective != input.objective
        || input.work_packet.acceptance_checks != input.acceptance_criteria
        || input.policy_sha256.len() != 64
        || !input
            .policy_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(CodingRunRequestError::BindingDenied);
    }
    let request = RuntimeRunRequest {
        schema_version: CONTRACT_SCHEMA_VERSION,
        run_id: input.run_id,
        session_id: input.session_id.clone(),
        mode: RuntimeSessionMode::ControlledWrite,
        task: Task {
            schema_version: CONTRACT_SCHEMA_VERSION,
            task_id,
            session_id: input.session_id,
            objective: input.objective,
            acceptance_criteria: input.acceptance_criteria,
            constraints: input.constraints,
            status: TaskStatus::Ready,
        },
        work_packet: input.work_packet,
        workspace_id: profile.write_scope().workspace_id().clone(),
        workspace_snapshot_sha256: profile.worktree().record_sha256.clone(),
        repository_snapshot_id: profile.repository_snapshot_id().clone(),
        repository_snapshot_sha256: profile.repository_snapshot_sha256().to_owned(),
        model_profile: profile.model_profile().clone(),
        context_budget: profile.model_profile().context.clone(),
        tool_catalog_id: profile.tool_catalog_id().clone(),
        tool_catalog_sha256: profile.tool_catalog_sha256().to_owned(),
        visible_tools: profile.visible_tools().to_vec(),
        policy_id: input.policy_id,
        policy_sha256: input.policy_sha256,
        limits: profile.limits().clone(),
        event_cursor: None,
        request_sha256: "0".repeat(64),
    };
    seal_runtime_run_request(request).map_err(|_| CodingRunRequestError::ContractDenied)
}

#[cfg(test)]
pub(crate) mod tests {
    use agentmage_kernel_contracts::{
        AuthorityClass, BudgetLimit, BudgetResource, DataSensitivity, EvidenceKind, PlanId,
        RollbackPlan, StopCondition, StopConditionKind, WorkPacketId, WorkPacketState,
    };
    use agentmage_kernel_engine::runtime_coordinator::verify_runtime_run_request;

    use super::*;
    use crate::coding_session::{CodingSessionProfile, tests::input};

    #[test]
    fn story_48_2_run_request_binds_profile_task_packet_policy_and_ephemeral_mode() {
        let (profile, request) = fixture_profile_and_request();
        verify_runtime_run_request(&request).expect("runtime request verifies");
        assert_eq!(request.mode, RuntimeSessionMode::ControlledWrite);
        assert!(request.event_cursor.is_none());
        assert_eq!(request.task.task_id.as_str(), profile.worktree().task_id);
        assert_eq!(request.workspace_id, *profile.write_scope().workspace_id());
        assert_eq!(request.visible_tools, profile.visible_tools());
        assert_eq!(request.policy_sha256, "b".repeat(64));
    }

    #[test]
    fn story_48_2_run_request_rejects_task_packet_and_policy_drift() {
        let profile = CodingSessionProfile::build(input()).expect("coding profile");
        let mut packet = fixture_work_packet(&profile);
        packet.objective = "another objective".to_owned();
        assert_eq!(
            build_ephemeral_coding_run_request(
                &profile,
                input_for(packet, "Inspect and repair one bounded fixture")
            ),
            Err(CodingRunRequestError::BindingDenied)
        );

        let mut wrong_task = fixture_work_packet(&profile);
        wrong_task.task_id = TaskId::from_raw("another-task");
        assert_eq!(
            build_ephemeral_coding_run_request(
                &profile,
                input_for(wrong_task, "Inspect and repair one bounded fixture")
            ),
            Err(CodingRunRequestError::BindingDenied)
        );

        let mut malformed_policy = input_for(
            fixture_work_packet(&profile),
            "Inspect and repair one bounded fixture",
        );
        malformed_policy.policy_sha256 = "B".repeat(64);
        assert_eq!(
            build_ephemeral_coding_run_request(&profile, malformed_policy),
            Err(CodingRunRequestError::BindingDenied)
        );
    }

    pub(crate) fn fixture_profile_and_request() -> (CodingSessionProfile, RuntimeRunRequest) {
        let profile = CodingSessionProfile::build(input()).expect("coding profile");
        let packet = fixture_work_packet(&profile);
        let request = build_ephemeral_coding_run_request(
            &profile,
            input_for(packet, "Inspect and repair one bounded fixture"),
        )
        .expect("coding request");
        (profile, request)
    }

    fn input_for(packet: WorkPacket, objective: &str) -> CodingRunRequestInput {
        CodingRunRequestInput {
            run_id: RuntimeRunId::from_raw("coding-run-0001"),
            session_id: SessionId::from_raw("coding-session-0001"),
            objective: objective.to_owned(),
            acceptance_criteria: vec!["The focused fixture passes".to_owned()],
            constraints: vec!["No network and no unrelated changes".to_owned()],
            work_packet: packet,
            policy_id: PolicyId::from_raw("coding-policy-0001"),
            policy_sha256: "b".repeat(64),
        }
    }

    pub(crate) fn fixture_work_packet(profile: &CodingSessionProfile) -> WorkPacket {
        WorkPacket {
            schema_version: CONTRACT_SCHEMA_VERSION,
            work_packet_id: WorkPacketId::from_raw("coding-packet-0001"),
            task_id: TaskId::from_raw(profile.worktree().task_id.clone()),
            revision: 1,
            objective: "Inspect and repair one bounded fixture".to_owned(),
            reason: "Exercise the shared coding runtime".to_owned(),
            owner: "fixture-user".to_owned(),
            authoritative_evidence: Vec::new(),
            mutable_files: vec!["src/lib.rs".to_owned()],
            protected_files: vec![".git".to_owned()],
            expected_output: "One evidence-backed coding result".to_owned(),
            acceptance_checks: vec!["The focused fixture passes".to_owned()],
            required_evidence: vec![EvidenceKind::Validation],
            required_capability_class: AuthorityClass::LocalWrite,
            budgets: vec![
                BudgetLimit {
                    resource: BudgetResource::ModelCalls,
                    limit: 16,
                },
                BudgetLimit {
                    resource: BudgetResource::ToolCalls,
                    limit: 32,
                },
            ],
            stop_conditions: [
                StopConditionKind::AcceptanceSatisfied,
                StopConditionKind::UserDecisionRequired,
                StopConditionKind::PolicyDenied,
                StopConditionKind::Error,
                StopConditionKind::Cancelled,
                StopConditionKind::BudgetExhausted,
                StopConditionKind::UncertainResult,
            ]
            .into_iter()
            .map(|kind| StopCondition {
                kind,
                description: format!("Stop for {kind:?}"),
            })
            .collect(),
            rollback: RollbackPlan {
                reversible: true,
                description: "Revert only exact approved bytes through a fresh request".to_owned(),
            },
            sensitivity: DataSensitivity::Ephemeral,
            last_verification_date: "2026-08-17".to_owned(),
            next_action: None,
            next_review: None,
            status_reason: None,
            disposition: None,
            completion_evidence: Vec::new(),
            superseding_work: None,
            validation_issues: Vec::new(),
            plan_id: Some(PlanId::from_raw("coding-plan-0001")),
            state: WorkPacketState::Active,
        }
    }
}
