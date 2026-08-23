//! Host-owned composition boundary for one approved durable Team campaign.

use agentmage_kernel_contracts::{
    CampaignId, EngineeringPlanHandoff, EvidenceReference, SessionId, TaskId, TeamCampaign,
    TeamCampaignState,
};
use agentmage_kernel_engine::multi_agent::{
    MultiAgentError, TeamCampaignCoordinator, TeamCampaignResult, TeamIntegrationPort,
    TeamReviewPort, TeamTaskSpec, TeamWorkerPort, seal_team_campaign,
};

/// Exact trusted input supplied to an installed Team campaign executor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineeringTeamInput {
    /// Exact Team coordinator session.
    pub session_id: SessionId,
    /// Exact approved-Plan handoff.
    pub handoff: EngineeringPlanHandoff,
    /// Exact approved Plan bytes loaded from encrypted artifact authority.
    pub plan_bytes: Vec<u8>,
    /// Exact new campaign identity.
    pub campaign_id: CampaignId,
    /// AgentMage-owned campaign branch.
    pub campaign_branch: String,
    /// Exact immutable starting commit.
    pub starting_commit: String,
    /// Configured concurrent worker ceiling.
    pub max_workers: u8,
    /// Last verified durable checkpoint when this campaign is being resumed.
    pub resume_from: Option<Box<TeamCampaign>>,
}

/// Stable refusal from an installed Team campaign executor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineeringTeamError {
    /// Planning, worker, review, correction, integration, or final verification failed closed.
    Failed,
}

/// Host-owned durable checkpoint boundary used during Team execution.
pub trait EngineeringTeamCheckpointPort {
    /// Persists one complete validated campaign projection before execution continues.
    fn checkpoint(&mut self, campaign: &TeamCampaign) -> Result<(), EngineeringTeamError>;
}

impl<F> EngineeringTeamCheckpointPort for F
where
    F: FnMut(&TeamCampaign) -> Result<(), EngineeringTeamError>,
{
    fn checkpoint(&mut self, campaign: &TeamCampaign) -> Result<(), EngineeringTeamError> {
        self(campaign)
    }
}

/// Trusted host composition point for a Team scheduler and its isolated workers.
pub trait EngineeringTeamPort {
    /// Executes one approved campaign and returns only its complete terminal projection.
    fn execute(
        &mut self,
        input: &EngineeringTeamInput,
        checkpoints: &mut dyn EngineeringTeamCheckpointPort,
    ) -> Result<TeamCampaign, EngineeringTeamError>;
}

/// Trusted Plan-to-task-graph boundary for the kernel Team scheduler.
pub trait EngineeringTeamPlannerPort {
    /// Produces the complete immutable task graph for one approved Plan.
    fn plan(
        &mut self,
        input: &EngineeringTeamInput,
    ) -> Result<Vec<TeamTaskSpec>, EngineeringTeamError>;
}

/// Deterministic campaign-wide verification boundary run after every task integrates.
pub trait EngineeringTeamFinalVerifierPort {
    /// Returns complete final evidence only when the integrated campaign passes.
    fn verify(
        &mut self,
        input: &EngineeringTeamInput,
        result: &TeamCampaignResult,
    ) -> Result<Vec<EvidenceReference>, EngineeringTeamError>;
}

/// Kernel-backed Team executor composed from explicit planning, worker, review, and integration ports.
pub struct KernelEngineeringTeamExecutor<P, W, R, I, V> {
    planner: P,
    worker: W,
    reviewer: R,
    integrator: I,
    verifier: V,
}

impl<P, W, R, I, V> KernelEngineeringTeamExecutor<P, W, R, I, V> {
    /// Installs explicit dependencies without discovering models, repositories, or authority.
    pub fn new(planner: P, worker: W, reviewer: R, integrator: I, verifier: V) -> Self {
        Self {
            planner,
            worker,
            reviewer,
            integrator,
            verifier,
        }
    }
}

impl<P, W, R, I, V> EngineeringTeamPort for KernelEngineeringTeamExecutor<P, W, R, I, V>
where
    P: EngineeringTeamPlannerPort,
    W: TeamWorkerPort + Clone + 'static,
    R: TeamReviewPort + Clone + 'static,
    I: TeamIntegrationPort + Clone + 'static,
    V: EngineeringTeamFinalVerifierPort,
{
    fn execute(
        &mut self,
        input: &EngineeringTeamInput,
        checkpoints: &mut dyn EngineeringTeamCheckpointPort,
    ) -> Result<TeamCampaign, EngineeringTeamError> {
        let tasks = self.planner.plan(input)?;
        let task_ids = tasks
            .iter()
            .map(|task| task.task_id.clone())
            .collect::<Vec<_>>();
        let objective = String::from_utf8(input.plan_bytes.clone())
            .map_err(|_| EngineeringTeamError::Failed)?;
        let mut last_result = input.resume_from.as_deref().map_or_else(
            || TeamCampaignResult {
                campaign_head: input.starting_commit.clone(),
                leases: Vec::new(),
                integrations: Vec::new(),
                maximum_concurrent_workers: 0,
            },
            |campaign| TeamCampaignResult {
                campaign_head: campaign.campaign_head.clone(),
                leases: campaign.leases.clone(),
                integrations: campaign.integrations.clone(),
                maximum_concurrent_workers: campaign.maximum_concurrent_workers,
            },
        );
        if let Some(resume) = input.resume_from.as_deref() {
            if resume.task_ids != task_ids
                || !matches!(
                    resume.state,
                    TeamCampaignState::Ready
                        | TeamCampaignState::Running
                        | TeamCampaignState::Paused
                )
            {
                return Err(EngineeringTeamError::Failed);
            }
        } else {
            checkpoints.checkpoint(&build_campaign(
                input,
                &objective,
                &task_ids,
                &last_result,
                TeamCampaignState::Running,
                Vec::new(),
                Vec::new(),
            )?)?;
        }
        let coordinator = if input.resume_from.is_some() {
            TeamCampaignCoordinator::resume(
                input.campaign_id.clone(),
                input.max_workers,
                tasks,
                last_result.clone(),
                self.worker.clone(),
                self.reviewer.clone(),
                self.integrator.clone(),
            )
        } else {
            TeamCampaignCoordinator::new(
                input.campaign_id.clone(),
                input.starting_commit.clone(),
                input.max_workers,
                tasks,
                self.worker.clone(),
                self.reviewer.clone(),
                self.integrator.clone(),
            )
        }
        .map_err(|_| EngineeringTeamError::Failed)?;
        let result = coordinator.run_with_progress(|progress| {
            let campaign = build_campaign(
                input,
                &objective,
                &task_ids,
                progress,
                TeamCampaignState::Running,
                Vec::new(),
                Vec::new(),
            )
            .map_err(|_| MultiAgentError::CheckpointFailed)?;
            checkpoints
                .checkpoint(&campaign)
                .map_err(|_| MultiAgentError::CheckpointFailed)?;
            last_result = progress.clone();
            Ok(())
        });
        let result = match result {
            Ok(result) => result,
            Err(MultiAgentError::CheckpointFailed | MultiAgentError::RecoveryUncertain) => {
                return Err(EngineeringTeamError::Failed);
            }
            Err(error) => {
                let failed = build_campaign(
                    input,
                    &objective,
                    &task_ids,
                    &last_result,
                    TeamCampaignState::Failed,
                    vec![error.code().to_owned()],
                    Vec::new(),
                )?;
                checkpoints.checkpoint(&failed)?;
                return Ok(failed);
            }
        };
        let evidence = match self.verifier.verify(input, &result) {
            Ok(evidence) if !evidence.is_empty() => evidence,
            _ => {
                let failed = build_campaign(
                    input,
                    &objective,
                    &task_ids,
                    &result,
                    TeamCampaignState::Failed,
                    vec!["multi-agent.final-verification.failed".to_owned()],
                    Vec::new(),
                )?;
                checkpoints.checkpoint(&failed)?;
                return Ok(failed);
            }
        };
        let completed = build_campaign(
            input,
            &objective,
            &task_ids,
            &result,
            TeamCampaignState::Success,
            Vec::new(),
            evidence,
        )?;
        checkpoints.checkpoint(&completed)?;
        Ok(completed)
    }
}

fn build_campaign(
    input: &EngineeringTeamInput,
    objective: &str,
    task_ids: &[TaskId],
    result: &TeamCampaignResult,
    state: TeamCampaignState,
    reason_codes: Vec<String>,
    final_evidence: Vec<EvidenceReference>,
) -> Result<TeamCampaign, EngineeringTeamError> {
    seal_team_campaign(TeamCampaign {
        schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
        campaign_id: input.campaign_id.clone(),
        coordinator_session_id: input.session_id.clone(),
        objective: objective.to_owned(),
        approved_plan_id: input.handoff.approval_id.as_str().to_owned(),
        approved_plan_sha256: input.handoff.plan_sha256.clone(),
        campaign_branch: input.campaign_branch.clone(),
        starting_commit: input.starting_commit.clone(),
        campaign_head: result.campaign_head.clone(),
        max_workers: input.max_workers,
        maximum_concurrent_workers: result.maximum_concurrent_workers,
        state,
        task_ids: task_ids.to_vec(),
        leases: result.leases.clone(),
        integrations: result.integrations.clone(),
        reason_codes,
        final_evidence,
        campaign_sha256: "0".repeat(64),
    })
    .map_err(|_| EngineeringTeamError::Failed)
}
