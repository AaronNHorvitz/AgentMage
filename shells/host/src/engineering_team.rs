//! Host-owned composition boundary for one approved durable Team campaign.

use agentmage_kernel_contracts::{CampaignId, EngineeringPlanHandoff, SessionId, TeamCampaign};

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
