//! Dependency-aware multi-agent scheduling and serialized integration.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::thread;

use agentmage_kernel_contracts::{
    ActorId, AgentLease, AgentLeaseId, AgentLeaseState, CONTRACT_SCHEMA_VERSION, CampaignId,
    EndpointProfileId, IntegrationId, IntegrationRecord, IntegrationState, ModelProfileId,
    ReviewFinding, ReviewOutcome, SessionId, TaskId, TeamCampaign, TeamCampaignState,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_TEAM_WORKERS: u8 = 5;
const MAX_TEAM_TASKS: usize = 10_000;

/// Immutable scheduler input for one bounded implementation task.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TeamTaskSpec {
    /// Exact task identity.
    pub task_id: TaskId,
    /// Dependencies that must already be integrated.
    pub dependencies: Vec<TaskId>,
    /// Exact worker identity.
    pub agent_id: ActorId,
    /// Exact worker session.
    pub session_id: SessionId,
    /// Exact selected model profile.
    pub model_profile_id: ModelProfileId,
    /// Exact selected endpoint profile.
    pub endpoint_profile_id: EndpointProfileId,
    /// Immutable starting commit.
    pub base_commit: String,
    /// AgentMage-owned worktree identity.
    pub worktree_id: String,
    /// AgentMage-owned branch.
    pub branch: String,
    /// Exact non-overlapping path leases.
    pub path_leases: Vec<String>,
    /// Exact shared test-resource leases.
    pub test_resource_leases: Vec<String>,
    /// Maximum correction attempts.
    pub correction_limit: u8,
}

/// Candidate returned by one isolated implementation worker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TeamWorkerCandidate {
    /// Exact task implemented.
    pub task_id: TaskId,
    /// Exact candidate commit.
    pub candidate_commit: String,
    /// Digest of deterministic local-gate evidence.
    pub gate_evidence_sha256: String,
}

/// Complete independent review result for one immutable candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TeamReviewResult {
    /// Closed review outcome.
    pub outcome: ReviewOutcome,
    /// Structured findings in stable order.
    pub findings: Vec<ReviewFinding>,
}

/// One successful serialized integration outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TeamIntegrationResult {
    /// Exact resulting campaign head.
    pub campaign_head: String,
    /// Digest of post-integration deterministic gates.
    pub gate_evidence_sha256: String,
}

/// Stable scheduler, worker, review, or integration failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MultiAgentError {
    /// Campaign or task graph input is invalid.
    InvalidInput,
    /// Task dependencies contain a cycle or unknown task.
    DependencyInvalid,
    /// A path or test-resource lease conflicts.
    LeaseConflict,
    /// An implementation worker failed before a candidate passed local gates.
    WorkerFailed,
    /// Independent review failed or remained disputed.
    ReviewFailed,
    /// Correction attempts were exhausted.
    CorrectionExhausted,
    /// Serialized integration failed or returned an invalid head.
    IntegrationFailed,
    /// A worker thread failed without a typed result.
    WorkerPanicked,
    /// A durable checkpoint contains an in-flight effect that cannot be replayed safely.
    RecoveryUncertain,
}

impl MultiAgentError {
    /// Returns one stable redacted error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "multi-agent.input.invalid",
            Self::DependencyInvalid => "multi-agent.dependency.invalid",
            Self::LeaseConflict => "multi-agent.lease.conflict",
            Self::WorkerFailed => "multi-agent.worker.failed",
            Self::ReviewFailed => "multi-agent.review.failed",
            Self::CorrectionExhausted => "multi-agent.correction.exhausted",
            Self::IntegrationFailed => "multi-agent.integration.failed",
            Self::WorkerPanicked => "multi-agent.worker.panicked",
            Self::RecoveryUncertain => "multi-agent.recovery.uncertain",
        }
    }
}

/// Isolated implementation boundary invoked concurrently by the scheduler.
pub trait TeamWorkerPort: Send + Sync {
    /// Implements one exact lease and returns an immutable candidate commit.
    fn implement(&self, lease: &AgentLease) -> Result<TeamWorkerCandidate, MultiAgentError>;

    /// Corrects structured review findings in the same worker lineage.
    fn correct(
        &self,
        lease: &AgentLease,
        candidate: &TeamWorkerCandidate,
        findings: &[ReviewFinding],
    ) -> Result<TeamWorkerCandidate, MultiAgentError>;
}

/// Independent immutable-candidate review boundary.
pub trait TeamReviewPort {
    /// Reviews the cumulative immutable candidate against its exact lease.
    fn review(
        &mut self,
        lease: &AgentLease,
        candidate: &TeamWorkerCandidate,
    ) -> Result<TeamReviewResult, MultiAgentError>;
}

/// Sole serialized integration boundary.
pub trait TeamIntegrationPort {
    /// Integrates one reviewed candidate against the exact current campaign head.
    fn integrate(
        &mut self,
        campaign_id: &CampaignId,
        campaign_head: &str,
        lease: &AgentLease,
        candidate: &TeamWorkerCandidate,
    ) -> Result<TeamIntegrationResult, MultiAgentError>;
}

/// Result of one complete dependency-ordered Team campaign.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TeamCampaignResult {
    /// Exact final campaign head.
    pub campaign_head: String,
    /// Leases in deterministic task order with final states.
    pub leases: Vec<AgentLease>,
    /// Serialized integration records.
    pub integrations: Vec<IntegrationRecord>,
    /// Maximum workers observed in one concurrent wave.
    pub maximum_concurrent_workers: u8,
}

/// Seals one durable Team campaign projection after validating its complete lifecycle state.
pub fn seal_team_campaign(mut campaign: TeamCampaign) -> Result<TeamCampaign, MultiAgentError> {
    campaign.campaign_sha256 = ZERO_SHA256.to_owned();
    validate_team_campaign(&campaign)?;
    campaign.campaign_sha256 = canonical_sha256(&campaign)?;
    Ok(campaign)
}

/// Verifies one durable Team campaign projection and its canonical digest.
pub fn verify_team_campaign(campaign: &TeamCampaign) -> Result<(), MultiAgentError> {
    validate_team_campaign(campaign)?;
    let mut candidate = campaign.clone();
    candidate.campaign_sha256 = ZERO_SHA256.to_owned();
    if canonical_sha256(&candidate)? != campaign.campaign_sha256 {
        return Err(MultiAgentError::InvalidInput);
    }
    Ok(())
}

fn validate_team_campaign(campaign: &TeamCampaign) -> Result<(), MultiAgentError> {
    if campaign.schema_version != CONTRACT_SCHEMA_VERSION
        || campaign.campaign_id.as_str().is_empty()
        || campaign.coordinator_session_id.as_str().is_empty()
        || campaign.objective.is_empty()
        || campaign.objective.len() > 64 * 1024
        || campaign.approved_plan_id.is_empty()
        || !valid_sha256(&campaign.approved_plan_sha256)
        || !valid_branch(&campaign.campaign_branch)
        || !valid_commit(&campaign.starting_commit)
        || !valid_commit(&campaign.campaign_head)
        || campaign.max_workers == 0
        || campaign.max_workers > MAX_TEAM_WORKERS
        || campaign.task_ids.is_empty()
        || campaign.task_ids.len() > MAX_TEAM_TASKS
        || !unique(campaign.task_ids.iter().map(TaskId::as_str))
        || campaign.leases.len() > campaign.task_ids.len()
        || campaign.integrations.len() > campaign.task_ids.len()
        || campaign
            .reason_codes
            .iter()
            .any(|reason| !valid_identifier(reason))
        || campaign
            .leases
            .iter()
            .any(|lease| verify_agent_lease(lease).is_err())
        || campaign
            .integrations
            .iter()
            .any(|record| verify_integration_record(record).is_err())
    {
        return Err(MultiAgentError::InvalidInput);
    }
    let task_ids = campaign.task_ids.iter().collect::<BTreeSet<_>>();
    if campaign
        .leases
        .iter()
        .any(|lease| !task_ids.contains(&lease.task_id))
        || campaign
            .integrations
            .iter()
            .any(|record| record.campaign_id != campaign.campaign_id)
    {
        return Err(MultiAgentError::InvalidInput);
    }
    match campaign.state {
        TeamCampaignState::Success => {
            if campaign.campaign_head == campaign.starting_commit
                || campaign.leases.len() != campaign.task_ids.len()
                || campaign.integrations.len() != campaign.task_ids.len()
                || campaign
                    .leases
                    .iter()
                    .any(|lease| lease.state != AgentLeaseState::Merged)
                || campaign
                    .integrations
                    .iter()
                    .any(|record| record.state != IntegrationState::Integrated)
                || !campaign.reason_codes.is_empty()
                || campaign.final_evidence.is_empty()
            {
                return Err(MultiAgentError::InvalidInput);
            }
        }
        TeamCampaignState::Blocked | TeamCampaignState::Failed => {
            if campaign.reason_codes.is_empty() || !campaign.final_evidence.is_empty() {
                return Err(MultiAgentError::InvalidInput);
            }
        }
        TeamCampaignState::Cancelled => {
            if campaign.reason_codes.is_empty() {
                return Err(MultiAgentError::InvalidInput);
            }
        }
        TeamCampaignState::Planned
        | TeamCampaignState::Ready
        | TeamCampaignState::Running
        | TeamCampaignState::Paused => {
            if !campaign.final_evidence.is_empty() {
                return Err(MultiAgentError::InvalidInput);
            }
        }
    }
    Ok(())
}

fn verify_agent_lease(lease: &AgentLease) -> Result<(), MultiAgentError> {
    let mut candidate = lease.clone();
    let expected = candidate.lease_sha256.clone();
    candidate.lease_sha256 = ZERO_SHA256.to_owned();
    if !valid_sha256(&expected) || canonical_sha256(&candidate)? != expected {
        return Err(MultiAgentError::InvalidInput);
    }
    Ok(())
}

fn verify_integration_record(record: &IntegrationRecord) -> Result<(), MultiAgentError> {
    let mut candidate = record.clone();
    let expected = candidate.integration_sha256.clone();
    candidate.integration_sha256 = ZERO_SHA256.to_owned();
    if !valid_sha256(&expected) || canonical_sha256(&candidate)? != expected {
        return Err(MultiAgentError::InvalidInput);
    }
    Ok(())
}

/// Deterministic kernel-owned Team coordinator.
pub struct TeamCampaignCoordinator<W, R, I>
where
    W: TeamWorkerPort,
    R: TeamReviewPort,
    I: TeamIntegrationPort,
{
    campaign_id: CampaignId,
    campaign_head: String,
    max_workers: u8,
    tasks: BTreeMap<TaskId, TeamTaskSpec>,
    worker: Arc<W>,
    reviewer: R,
    integrator: I,
    completed: BTreeSet<TaskId>,
    admitted: BTreeSet<TaskId>,
    final_leases: BTreeMap<TaskId, AgentLease>,
    integrations: Vec<IntegrationRecord>,
    maximum_concurrent_workers: u8,
}

impl<W, R, I> TeamCampaignCoordinator<W, R, I>
where
    W: TeamWorkerPort,
    R: TeamReviewPort,
    I: TeamIntegrationPort,
{
    /// Validates and freezes one campaign scheduler.
    pub fn new(
        campaign_id: CampaignId,
        campaign_head: String,
        max_workers: u8,
        tasks: Vec<TeamTaskSpec>,
        worker: W,
        reviewer: R,
        integrator: I,
    ) -> Result<Self, MultiAgentError> {
        if campaign_id.as_str().is_empty()
            || !valid_commit(&campaign_head)
            || max_workers == 0
            || max_workers > MAX_TEAM_WORKERS
            || tasks.is_empty()
            || tasks.len() > MAX_TEAM_TASKS
        {
            return Err(MultiAgentError::InvalidInput);
        }
        let mut indexed = BTreeMap::new();
        for task in tasks {
            validate_task(&task)?;
            if indexed.insert(task.task_id.clone(), task).is_some() {
                return Err(MultiAgentError::InvalidInput);
            }
        }
        validate_dependencies(&indexed)?;
        Ok(Self {
            campaign_id,
            campaign_head,
            max_workers,
            tasks: indexed,
            worker: Arc::new(worker),
            reviewer,
            integrator,
            completed: BTreeSet::new(),
            admitted: BTreeSet::new(),
            final_leases: BTreeMap::new(),
            integrations: Vec::new(),
            maximum_concurrent_workers: 0,
        })
    }

    /// Restores one scheduler only from an effect-complete integrated wave boundary.
    pub fn resume(
        campaign_id: CampaignId,
        max_workers: u8,
        tasks: Vec<TeamTaskSpec>,
        checkpoint: TeamCampaignResult,
        worker: W,
        reviewer: R,
        integrator: I,
    ) -> Result<Self, MultiAgentError> {
        let mut coordinator = Self::new(
            campaign_id.clone(),
            checkpoint.campaign_head.clone(),
            max_workers,
            tasks,
            worker,
            reviewer,
            integrator,
        )?;
        if checkpoint.maximum_concurrent_workers > max_workers
            || checkpoint.leases.len() != checkpoint.integrations.len()
            || !unique(checkpoint.leases.iter().map(|lease| lease.task_id.as_str()))
            || checkpoint.leases.iter().any(|lease| {
                lease.campaign_id != campaign_id
                    || lease.state != AgentLeaseState::Merged
                    || verify_agent_lease(lease).is_err()
                    || !coordinator.tasks.contains_key(&lease.task_id)
            })
            || checkpoint.integrations.iter().any(|record| {
                record.campaign_id != campaign_id
                    || record.state != IntegrationState::Integrated
                    || verify_integration_record(record).is_err()
            })
        {
            return Err(MultiAgentError::RecoveryUncertain);
        }
        let lease_ids = checkpoint
            .leases
            .iter()
            .map(|lease| &lease.lease_id)
            .collect::<BTreeSet<_>>();
        if checkpoint
            .integrations
            .iter()
            .any(|record| !lease_ids.contains(&record.lease_id))
        {
            return Err(MultiAgentError::RecoveryUncertain);
        }
        coordinator.completed = checkpoint
            .leases
            .iter()
            .map(|lease| lease.task_id.clone())
            .collect();
        coordinator.admitted = coordinator.completed.clone();
        coordinator.final_leases = checkpoint
            .leases
            .into_iter()
            .map(|lease| (lease.task_id.clone(), lease))
            .collect();
        coordinator.integrations = checkpoint.integrations;
        coordinator.maximum_concurrent_workers = checkpoint.maximum_concurrent_workers;
        Ok(coordinator)
    }

    /// Executes all dependency-ready waves, reviews, corrections, and integrations.
    pub fn run(self) -> Result<TeamCampaignResult, MultiAgentError> {
        self.run_with_progress(|_| Ok(()))
    }

    /// Executes one campaign while synchronously checkpointing every durable boundary.
    pub fn run_with_progress<F>(
        mut self,
        mut checkpoint: F,
    ) -> Result<TeamCampaignResult, MultiAgentError>
    where
        F: FnMut(&TeamCampaignResult) -> Result<(), MultiAgentError>,
    {
        let mut completed = std::mem::take(&mut self.completed);
        let mut admitted = std::mem::take(&mut self.admitted);
        let mut final_leases = std::mem::take(&mut self.final_leases);
        let mut integrations = std::mem::take(&mut self.integrations);
        let mut maximum_concurrent_workers = self.maximum_concurrent_workers;
        while completed.len() < self.tasks.len() {
            let wave = self.next_wave(&completed, &admitted)?;
            if wave.is_empty() {
                return Err(MultiAgentError::DependencyInvalid);
            }
            maximum_concurrent_workers = maximum_concurrent_workers.max(wave.len() as u8);
            let leases = wave
                .iter()
                .map(|task| lease_for(&self.campaign_id, task))
                .collect::<Result<Vec<_>, _>>()?;
            for lease in &leases {
                admitted.insert(lease.task_id.clone());
                final_leases.insert(lease.task_id.clone(), lease.clone());
            }
            checkpoint(&campaign_result(
                &self.campaign_head,
                &final_leases,
                &integrations,
                maximum_concurrent_workers,
            ))?;
            let candidates = thread::scope(|scope| {
                let handles = leases
                    .iter()
                    .map(|lease| {
                        let worker = Arc::clone(&self.worker);
                        scope.spawn(move || worker.implement(lease))
                    })
                    .collect::<Vec<_>>();
                handles
                    .into_iter()
                    .map(|handle| {
                        handle
                            .join()
                            .unwrap_or(Err(MultiAgentError::WorkerPanicked))
                    })
                    .collect::<Vec<_>>()
            });

            for (mut lease, candidate) in leases.into_iter().zip(candidates) {
                let mut candidate = match candidate {
                    Ok(candidate) => candidate,
                    Err(error) => {
                        lease.state = AgentLeaseState::Failed;
                        reseal_lease(&mut lease)?;
                        final_leases.insert(lease.task_id.clone(), lease);
                        checkpoint(&campaign_result(
                            &self.campaign_head,
                            &final_leases,
                            &integrations,
                            maximum_concurrent_workers,
                        ))?;
                        return Err(error);
                    }
                };
                if candidate.task_id != lease.task_id || !valid_commit(&candidate.candidate_commit)
                {
                    return Err(MultiAgentError::WorkerFailed);
                }
                lease.state = AgentLeaseState::Reviewing;
                lease.candidate_commit = Some(candidate.candidate_commit.clone());
                reseal_lease(&mut lease)?;
                final_leases.insert(lease.task_id.clone(), lease.clone());
                checkpoint(&campaign_result(
                    &self.campaign_head,
                    &final_leases,
                    &integrations,
                    maximum_concurrent_workers,
                ))?;
                loop {
                    let review = self.reviewer.review(&lease, &candidate)?;
                    match review.outcome {
                        ReviewOutcome::Pass if review.findings.is_empty() => break,
                        ReviewOutcome::ChangesRequired if !review.findings.is_empty() => {
                            if lease.correction_count >= lease.correction_limit {
                                return Err(MultiAgentError::CorrectionExhausted);
                            }
                            lease.state = AgentLeaseState::Correcting;
                            lease.correction_count += 1;
                            reseal_lease(&mut lease)?;
                            final_leases.insert(lease.task_id.clone(), lease.clone());
                            checkpoint(&campaign_result(
                                &self.campaign_head,
                                &final_leases,
                                &integrations,
                                maximum_concurrent_workers,
                            ))?;
                            candidate =
                                self.worker.correct(&lease, &candidate, &review.findings)?;
                            if candidate.task_id != lease.task_id
                                || !valid_commit(&candidate.candidate_commit)
                            {
                                return Err(MultiAgentError::WorkerFailed);
                            }
                            lease.state = AgentLeaseState::Reviewing;
                            lease.candidate_commit = Some(candidate.candidate_commit.clone());
                            reseal_lease(&mut lease)?;
                            final_leases.insert(lease.task_id.clone(), lease.clone());
                            checkpoint(&campaign_result(
                                &self.campaign_head,
                                &final_leases,
                                &integrations,
                                maximum_concurrent_workers,
                            ))?;
                        }
                        ReviewOutcome::Blocked | ReviewOutcome::Disputed => {
                            return Err(MultiAgentError::ReviewFailed);
                        }
                        _ => return Err(MultiAgentError::ReviewFailed),
                    }
                }
                lease.state = AgentLeaseState::IntegrationQueued;
                reseal_lease(&mut lease)?;
                final_leases.insert(lease.task_id.clone(), lease.clone());
                checkpoint(&campaign_result(
                    &self.campaign_head,
                    &final_leases,
                    &integrations,
                    maximum_concurrent_workers,
                ))?;
                let prior_head = self.campaign_head.clone();
                lease.state = AgentLeaseState::Integrating;
                reseal_lease(&mut lease)?;
                final_leases.insert(lease.task_id.clone(), lease.clone());
                checkpoint(&campaign_result(
                    &self.campaign_head,
                    &final_leases,
                    &integrations,
                    maximum_concurrent_workers,
                ))?;
                let integrated = self.integrator.integrate(
                    &self.campaign_id,
                    &prior_head,
                    &lease,
                    &candidate,
                )?;
                if !valid_commit(&integrated.campaign_head)
                    || integrated.campaign_head == prior_head
                    || !valid_sha256(&integrated.gate_evidence_sha256)
                {
                    return Err(MultiAgentError::IntegrationFailed);
                }
                self.campaign_head = integrated.campaign_head.clone();
                lease.state = AgentLeaseState::Merged;
                reseal_lease(&mut lease)?;
                integrations.push(seal_integration(IntegrationRecord {
                    schema_version: CONTRACT_SCHEMA_VERSION,
                    integration_id: IntegrationId::from_raw(format!(
                        "integration-{}-{}",
                        self.campaign_id.as_str(),
                        integrations.len() + 1
                    )),
                    campaign_id: self.campaign_id.clone(),
                    lease_id: lease.lease_id.clone(),
                    prior_campaign_head: prior_head,
                    candidate_commit: candidate.candidate_commit,
                    resulting_campaign_head: Some(self.campaign_head.clone()),
                    state: IntegrationState::Integrated,
                    gate_evidence_sha256: Some(integrated.gate_evidence_sha256),
                    reason_codes: Vec::new(),
                    integration_sha256: ZERO_SHA256.to_owned(),
                })?);
                completed.insert(lease.task_id.clone());
                final_leases.insert(lease.task_id.clone(), lease);
                checkpoint(&campaign_result(
                    &self.campaign_head,
                    &final_leases,
                    &integrations,
                    maximum_concurrent_workers,
                ))?;
            }
        }
        Ok(campaign_result(
            &self.campaign_head,
            &final_leases,
            &integrations,
            maximum_concurrent_workers,
        ))
    }

    fn next_wave<'a>(
        &'a self,
        completed: &BTreeSet<TaskId>,
        admitted: &BTreeSet<TaskId>,
    ) -> Result<Vec<&'a TeamTaskSpec>, MultiAgentError> {
        let mut selected = Vec::new();
        let mut paths: Vec<&str> = Vec::new();
        let mut resources = BTreeSet::new();
        for task in self.tasks.values() {
            if selected.len() >= usize::from(self.max_workers)
                || admitted.contains(&task.task_id)
                || !task
                    .dependencies
                    .iter()
                    .all(|dependency| completed.contains(dependency))
            {
                continue;
            }
            if task
                .path_leases
                .iter()
                .any(|path| paths.iter().any(|held| path_overlaps(path.as_str(), held)))
                || task
                    .test_resource_leases
                    .iter()
                    .any(|resource| resources.contains(resource))
            {
                continue;
            }
            paths.extend(task.path_leases.iter().map(String::as_str));
            resources.extend(task.test_resource_leases.iter().cloned());
            selected.push(task);
        }
        Ok(selected)
    }
}

fn campaign_result(
    campaign_head: &str,
    leases: &BTreeMap<TaskId, AgentLease>,
    integrations: &[IntegrationRecord],
    maximum_concurrent_workers: u8,
) -> TeamCampaignResult {
    TeamCampaignResult {
        campaign_head: campaign_head.to_owned(),
        leases: leases.values().cloned().collect(),
        integrations: integrations.to_vec(),
        maximum_concurrent_workers,
    }
}

fn lease_for(campaign_id: &CampaignId, task: &TeamTaskSpec) -> Result<AgentLease, MultiAgentError> {
    let mut lease = AgentLease {
        schema_version: CONTRACT_SCHEMA_VERSION,
        campaign_id: campaign_id.clone(),
        lease_id: AgentLeaseId::from_raw(format!(
            "lease-{}-{}",
            campaign_id.as_str(),
            task.task_id.as_str()
        )),
        task_id: task.task_id.clone(),
        agent_id: task.agent_id.clone(),
        session_id: task.session_id.clone(),
        model_profile_id: task.model_profile_id.clone(),
        endpoint_profile_id: task.endpoint_profile_id.clone(),
        base_commit: task.base_commit.clone(),
        worktree_id: task.worktree_id.clone(),
        branch: task.branch.clone(),
        path_leases: task.path_leases.clone(),
        test_resource_leases: task.test_resource_leases.clone(),
        state: AgentLeaseState::Implementing,
        correction_limit: task.correction_limit,
        correction_count: 0,
        candidate_commit: None,
        lease_sha256: ZERO_SHA256.to_owned(),
    };
    reseal_lease(&mut lease)?;
    Ok(lease)
}

fn reseal_lease(lease: &mut AgentLease) -> Result<(), MultiAgentError> {
    lease.lease_sha256 = ZERO_SHA256.to_owned();
    lease.lease_sha256 = canonical_sha256(lease)?;
    Ok(())
}

fn seal_integration(mut record: IntegrationRecord) -> Result<IntegrationRecord, MultiAgentError> {
    record.integration_sha256 = ZERO_SHA256.to_owned();
    record.integration_sha256 = canonical_sha256(&record)?;
    Ok(record)
}

fn validate_task(task: &TeamTaskSpec) -> Result<(), MultiAgentError> {
    if task.task_id.as_str().is_empty()
        || task.agent_id.as_str().is_empty()
        || task.session_id.as_str().is_empty()
        || task.model_profile_id.as_str().is_empty()
        || task.endpoint_profile_id.as_str().is_empty()
        || !valid_commit(&task.base_commit)
        || !valid_identifier(&task.worktree_id)
        || !valid_branch(&task.branch)
        || task.path_leases.is_empty()
        || task.path_leases.len() > 64
        || task.test_resource_leases.len() > 64
        || task.correction_limit > 10
        || !unique(task.dependencies.iter().map(TaskId::as_str))
        || !unique(task.path_leases.iter().map(String::as_str))
        || !unique(task.test_resource_leases.iter().map(String::as_str))
        || task.path_leases.iter().any(|path| !valid_lease_path(path))
        || task
            .test_resource_leases
            .iter()
            .any(|resource| !valid_identifier(resource))
    {
        return Err(MultiAgentError::InvalidInput);
    }
    Ok(())
}

fn validate_dependencies(tasks: &BTreeMap<TaskId, TeamTaskSpec>) -> Result<(), MultiAgentError> {
    for task in tasks.values() {
        if task
            .dependencies
            .iter()
            .any(|dependency| dependency == &task.task_id || !tasks.contains_key(dependency))
        {
            return Err(MultiAgentError::DependencyInvalid);
        }
    }
    let mut complete = BTreeSet::new();
    loop {
        let before = complete.len();
        for task in tasks.values() {
            if task
                .dependencies
                .iter()
                .all(|dependency| complete.contains(dependency))
            {
                complete.insert(task.task_id.clone());
            }
        }
        if complete.len() == tasks.len() {
            return Ok(());
        }
        if complete.len() == before {
            return Err(MultiAgentError::DependencyInvalid);
        }
    }
}

fn path_overlaps(left: &str, right: &str) -> bool {
    left == right
        || left
            .strip_prefix(right)
            .is_some_and(|suffix| suffix.starts_with('/'))
        || right
            .strip_prefix(left)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn valid_lease_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 1024
        && !value.starts_with('/')
        && !value.contains('\0')
        && !value.split('/').any(|part| matches!(part, "" | "." | ".."))
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

fn valid_branch(value: &str) -> bool {
    valid_lease_path(value)
        && !value.starts_with('-')
        && !value.ends_with('.')
        && !value.contains("..")
        && !value.contains("@{")
}

fn valid_commit(value: &str) -> bool {
    (value.len() == 40 || value.len() == 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn unique<'a>(mut values: impl Iterator<Item = &'a str>) -> bool {
    let mut seen = BTreeSet::new();
    values.all(|value| seen.insert(value))
}

fn canonical_sha256<T: Serialize>(value: &T) -> Result<String, MultiAgentError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| MultiAgentError::InvalidInput)
}

fn sha256(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    let digest = digest.finalize();
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU8, Ordering};
    use std::sync::{Arc, Barrier, Mutex};

    use super::{
        MultiAgentError, TeamCampaignCoordinator, TeamIntegrationPort, TeamIntegrationResult,
        TeamReviewPort, TeamReviewResult, TeamTaskSpec, TeamWorkerCandidate, TeamWorkerPort,
        ZERO_SHA256, reseal_lease, seal_team_campaign, verify_team_campaign,
    };
    use agentmage_kernel_contracts::{
        ActorId, AgentLease, CampaignId, EndpointProfileId, EvidenceId, EvidenceKind,
        EvidenceReference, ModelProfileId, ReviewOutcome, SessionId, TaskId, TeamCampaign,
        TeamCampaignState,
    };

    struct ConcurrentWorker {
        barrier: Arc<Barrier>,
        active: Arc<AtomicU8>,
        maximum: Arc<AtomicU8>,
    }

    impl TeamWorkerPort for ConcurrentWorker {
        fn implement(&self, lease: &AgentLease) -> Result<TeamWorkerCandidate, MultiAgentError> {
            let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
            self.maximum.fetch_max(active, Ordering::SeqCst);
            self.barrier.wait();
            self.active.fetch_sub(1, Ordering::SeqCst);
            Ok(TeamWorkerCandidate {
                task_id: lease.task_id.clone(),
                candidate_commit: candidate_commit(lease.task_id.as_str()),
                gate_evidence_sha256: "a".repeat(64),
            })
        }

        fn correct(
            &self,
            _lease: &AgentLease,
            candidate: &TeamWorkerCandidate,
            _findings: &[agentmage_kernel_contracts::ReviewFinding],
        ) -> Result<TeamWorkerCandidate, MultiAgentError> {
            Ok(candidate.clone())
        }
    }

    #[derive(Default)]
    struct PassingReviewer;

    impl TeamReviewPort for PassingReviewer {
        fn review(
            &mut self,
            _lease: &AgentLease,
            _candidate: &TeamWorkerCandidate,
        ) -> Result<TeamReviewResult, MultiAgentError> {
            Ok(TeamReviewResult {
                outcome: ReviewOutcome::Pass,
                findings: Vec::new(),
            })
        }
    }

    #[derive(Default)]
    struct SerializedIntegrator {
        active: bool,
        order: Arc<Mutex<Vec<String>>>,
    }

    impl TeamIntegrationPort for SerializedIntegrator {
        fn integrate(
            &mut self,
            _campaign_id: &CampaignId,
            _campaign_head: &str,
            lease: &AgentLease,
            _candidate: &TeamWorkerCandidate,
        ) -> Result<TeamIntegrationResult, MultiAgentError> {
            assert!(!self.active);
            self.active = true;
            self.order
                .lock()
                .unwrap()
                .push(lease.task_id.as_str().to_owned());
            self.active = false;
            Ok(TeamIntegrationResult {
                campaign_head: candidate_commit(&format!("integrated-{}", lease.task_id.as_str())),
                gate_evidence_sha256: "b".repeat(64),
            })
        }
    }

    fn task(id: &str) -> TeamTaskSpec {
        TeamTaskSpec {
            task_id: TaskId::from_raw(id),
            dependencies: Vec::new(),
            agent_id: ActorId::from_raw(format!("agent-{id}")),
            session_id: SessionId::from_raw(format!("session-{id}")),
            model_profile_id: ModelProfileId::from_raw("model-qualified"),
            endpoint_profile_id: EndpointProfileId::from_raw("endpoint-qualified"),
            base_commit: "1".repeat(40),
            worktree_id: format!("worktree-{id}"),
            branch: format!("agentmage/team/{id}"),
            path_leases: vec![format!("owned/{id}")],
            test_resource_leases: vec![format!("test-{id}")],
            correction_limit: 2,
        }
    }

    fn candidate_commit(seed: &str) -> String {
        super::sha256(seed.as_bytes())[..40].to_owned()
    }

    #[test]
    fn three_workers_run_concurrently_and_integrate_serially() {
        let maximum = Arc::new(AtomicU8::new(0));
        let order = Arc::new(Mutex::new(Vec::new()));
        let coordinator = TeamCampaignCoordinator::new(
            CampaignId::from_raw("campaign-concurrent"),
            "0".repeat(40),
            5,
            vec![task("task-a"), task("task-b"), task("task-c")],
            ConcurrentWorker {
                barrier: Arc::new(Barrier::new(3)),
                active: Arc::new(AtomicU8::new(0)),
                maximum: Arc::clone(&maximum),
            },
            PassingReviewer,
            SerializedIntegrator {
                active: false,
                order: Arc::clone(&order),
            },
        )
        .unwrap();
        let mut progress = Vec::new();
        let result = coordinator
            .run_with_progress(|snapshot| {
                progress.push(snapshot.clone());
                Ok(())
            })
            .unwrap();
        assert_eq!(maximum.load(Ordering::SeqCst), 3);
        assert_eq!(result.maximum_concurrent_workers, 3);
        assert_eq!(result.integrations.len(), 3);
        assert_eq!(order.lock().unwrap().len(), 3);
        assert_eq!(progress.first().unwrap().leases.len(), 3);
        assert!(progress.first().unwrap().leases.iter().all(|lease| {
            lease.state == agentmage_kernel_contracts::AgentLeaseState::Implementing
        }));
        assert!(
            progress.last().unwrap().leases.iter().all(|lease| {
                lease.state == agentmage_kernel_contracts::AgentLeaseState::Merged
            })
        );
        assert_eq!(progress.last().unwrap(), &result);
    }

    #[test]
    fn integrated_wave_resumes_without_replaying_and_inflight_state_is_refused() {
        let mut second = task("task-b");
        second.dependencies = vec![TaskId::from_raw("task-a")];
        let tasks = vec![task("task-a"), second];
        let worker = || ConcurrentWorker {
            barrier: Arc::new(Barrier::new(1)),
            active: Arc::new(AtomicU8::new(0)),
            maximum: Arc::new(AtomicU8::new(0)),
        };
        let campaign_id = CampaignId::from_raw("campaign-resume");
        let coordinator = TeamCampaignCoordinator::new(
            campaign_id.clone(),
            "0".repeat(40),
            1,
            tasks.clone(),
            worker(),
            PassingReviewer,
            SerializedIntegrator::default(),
        )
        .unwrap();
        let mut boundary = None;
        assert_eq!(
            coordinator.run_with_progress(|snapshot| {
                if snapshot.leases.len() == 1
                    && snapshot.leases.iter().all(|lease| {
                        lease.state == agentmage_kernel_contracts::AgentLeaseState::Merged
                    })
                {
                    boundary = Some(snapshot.clone());
                    return Err(MultiAgentError::WorkerFailed);
                }
                Ok(())
            }),
            Err(MultiAgentError::WorkerFailed)
        );
        let boundary = boundary.unwrap();
        let resumed = TeamCampaignCoordinator::resume(
            campaign_id.clone(),
            1,
            tasks.clone(),
            boundary.clone(),
            worker(),
            PassingReviewer,
            SerializedIntegrator::default(),
        )
        .unwrap()
        .run()
        .unwrap();
        assert_eq!(resumed.leases.len(), 2);
        assert_eq!(resumed.integrations.len(), 2);

        let mut uncertain = boundary;
        uncertain.leases[0].state = agentmage_kernel_contracts::AgentLeaseState::Integrating;
        reseal_lease(&mut uncertain.leases[0]).unwrap();
        assert!(matches!(
            TeamCampaignCoordinator::resume(
                campaign_id,
                1,
                tasks,
                uncertain,
                worker(),
                PassingReviewer,
                SerializedIntegrator::default(),
            ),
            Err(MultiAgentError::RecoveryUncertain)
        ));
    }

    #[test]
    fn dependency_cycle_and_overlapping_paths_fail_before_workers_start() {
        let mut first = task("task-a");
        let mut second = task("task-b");
        first.dependencies = vec![second.task_id.clone()];
        second.dependencies = vec![first.task_id.clone()];
        let result = TeamCampaignCoordinator::new(
            CampaignId::from_raw("campaign-cycle"),
            "0".repeat(40),
            2,
            vec![first, second],
            ConcurrentWorker {
                barrier: Arc::new(Barrier::new(1)),
                active: Arc::new(AtomicU8::new(0)),
                maximum: Arc::new(AtomicU8::new(0)),
            },
            PassingReviewer,
            SerializedIntegrator::default(),
        );
        assert!(matches!(result, Err(MultiAgentError::DependencyInvalid)));
    }

    #[test]
    fn successful_campaign_projection_binds_every_lease_integration_and_plan() {
        let coordinator = TeamCampaignCoordinator::new(
            CampaignId::from_raw("campaign-durable"),
            "0".repeat(40),
            1,
            vec![task("task-a")],
            ConcurrentWorker {
                barrier: Arc::new(Barrier::new(1)),
                active: Arc::new(AtomicU8::new(0)),
                maximum: Arc::new(AtomicU8::new(0)),
            },
            PassingReviewer,
            SerializedIntegrator::default(),
        )
        .unwrap();
        let result = coordinator.run().unwrap();
        let campaign = seal_team_campaign(TeamCampaign {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            campaign_id: CampaignId::from_raw("campaign-durable"),
            coordinator_session_id: SessionId::from_raw("session-team-coordinator"),
            objective: "Implement the approved Plan".to_owned(),
            approved_plan_id: "approval-team-plan".to_owned(),
            approved_plan_sha256: "a".repeat(64),
            campaign_branch: "agentmage/team/campaign-durable".to_owned(),
            starting_commit: "0".repeat(40),
            campaign_head: result.campaign_head,
            max_workers: 1,
            state: TeamCampaignState::Success,
            task_ids: vec![TaskId::from_raw("task-a")],
            leases: result.leases,
            integrations: result.integrations,
            reason_codes: Vec::new(),
            final_evidence: vec![EvidenceReference {
                schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                evidence_id: EvidenceId::from_raw("evidence-team-final"),
                kind: EvidenceKind::Validation,
                source_id: "team-final-verifier".to_owned(),
                object_id: "campaign-durable".to_owned(),
                fragment: None,
                content_sha256: "b".repeat(64),
                observed_revision: Some("final".to_owned()),
            }],
            campaign_sha256: ZERO_SHA256.to_owned(),
        })
        .unwrap();
        verify_team_campaign(&campaign).unwrap();
        let mut substituted = campaign;
        substituted.approved_plan_sha256 = "c".repeat(64);
        assert_eq!(
            verify_team_campaign(&substituted),
            Err(MultiAgentError::InvalidInput)
        );
    }
}
