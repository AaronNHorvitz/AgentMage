//! Dependency-ready workflow supervision over the existing reusable runtime coordinator.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use agentmage_kernel_contracts::{
    AgentStateKind, CanonicalStepExecutionPolicy, CanonicalWorkflowDefinition,
    CanonicalWorkflowFailureClass, PlanStepId, RuntimeEvent, RuntimeEventKind, RuntimeOutcome,
    RuntimeRunRequest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    runtime_coordinator::{verify_runtime_outcome, verify_runtime_run_request},
    runtime_event::RuntimeEventSequence,
    workflow_definition::{WorkflowStepPolicyBinding, admit_closed_workflow},
};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_SUPERVISOR_STEPS: usize = 256;

/// Exact non-authoritative input for one dependency-ready coordinator attempt.
#[derive(Clone, Copy, Debug)]
pub struct WorkflowAttemptRequest<'a> {
    /// Stable workflow execution identity.
    pub execution_id: &'a str,
    /// Closed workflow step identity.
    pub step_id: &'a str,
    /// Existing plan-step identity bound by the admitted policy.
    pub plan_step_id: &'a PlanStepId,
    /// Exact admitted policy digest.
    pub policy_sha256: &'a str,
    /// One-based fresh attempt ordinal.
    pub attempt_ordinal: u32,
    /// Prior attempt identity only for a fresh successor.
    pub prior_attempt_id: Option<&'a str>,
    /// Already verified dependency step identities.
    pub completed_dependencies: &'a [String],
}

/// Resource accounting not already carried by the canonical runtime outcome.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowAttemptResources {
    /// Exact context tokens consumed by the coordinator attempt.
    pub context_tokens: u64,
    /// Maximum resident bytes attributed to the attempt.
    pub memory_bytes: u64,
    /// Policy-defined cost minor units consumed by the attempt.
    pub cost_minor_units: u64,
}

/// One exact terminal run produced by the reusable runtime coordinator.
#[derive(Clone, Debug)]
pub struct VerifiedWorkflowStepRun {
    /// Fresh supervisor attempt identity.
    pub attempt_id: String,
    /// One-based ordinal requested by the supervisor.
    pub attempt_ordinal: u32,
    /// Exact current preflight policy digest.
    pub preflight_policy_sha256: String,
    /// Whether all registered preflights were freshly observed before start.
    pub preflight_current: bool,
    /// Host-framed request consumed by the coordinator.
    pub request: RuntimeRunRequest,
    /// Complete coordinator-owned event stream.
    pub events: Vec<RuntimeEvent>,
    /// Canonical verifier-owned terminal outcome.
    pub outcome: RuntimeOutcome,
    /// Deterministic supervisor failure class for a non-success.
    pub failure_class: Option<CanonicalWorkflowFailureClass>,
    /// Whether a prior effect remains uncertain and therefore non-retryable.
    pub uncertain_effect: bool,
    /// Exact additional resource accounting.
    pub resources: WorkflowAttemptResources,
}

/// Closed result from the injected coordinator composition boundary.
#[derive(Clone, Debug)]
pub enum WorkflowAttemptPortResult {
    /// One coordinator run reached a terminal outcome.
    Terminal(Box<VerifiedWorkflowStepRun>),
    /// Execution stopped at a verified safe boundary before another attempt began.
    Interrupted {
        /// Stable content-free interruption code.
        reason_code: String,
    },
}

/// Factory/driver for exact reusable-runtime coordinators.
///
/// Implementations may compose model, context, tool, approval, journal, checkpoint, and verifier
/// ports, but this trait grants no such authority to the supervisor itself.
pub trait VerifiedWorkflowAttemptPort {
    /// Revalidates preflight/policy/current-state bindings, composes one fresh coordinator, and
    /// drives it to a terminal or safe interruption boundary.
    fn run_attempt(
        &mut self,
        request: WorkflowAttemptRequest<'_>,
    ) -> Result<WorkflowAttemptPortResult, WorkflowSupervisorError>;
}

/// Closed supervisor terminal state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowSupervisorState {
    /// Every step completed from current verifier evidence.
    Succeeded,
    /// Execution stopped at a verified resumable step boundary.
    Interrupted,
    /// Current dependency, policy, preflight, or verification evidence blocked progress.
    Blocked,
    /// An unsafe or inconsistent runtime boundary failed closed.
    Failed,
    /// Trusted cancellation terminated the workflow.
    Cancelled,
    /// A declared attempt, no-progress, or resource budget was exhausted.
    Exhausted,
    /// A potentially completed effect requires reconciliation and cannot be replayed.
    Uncertain,
}

/// One content-free attempt retained in workflow order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowAttemptLineage {
    /// Closed workflow step identity.
    pub step_id: String,
    /// Existing plan-step identity.
    pub plan_step_id: PlanStepId,
    /// Fresh supervisor attempt identity.
    pub attempt_id: String,
    /// One-based attempt ordinal.
    pub attempt_ordinal: u32,
    /// Exact coordinator run identity.
    pub run_id: String,
    /// Tool-call identities observed in this run.
    pub tool_call_ids: Vec<String>,
    /// Grant identities observed in this run.
    pub grant_ids: Vec<String>,
    /// Receipt identities observed in this run.
    pub receipt_ids: Vec<String>,
    /// Canonical outcome digest.
    pub outcome_sha256: String,
    /// Verifier-owned terminal state.
    pub state: AgentStateKind,
    /// Exact resource usage attributed to this attempt.
    pub budget_usage: WorkflowSupervisorBudgetLedger,
}

/// Exact aggregate budgets consumed across completed coordinator attempts.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowSupervisorBudgetLedger {
    /// Total model turns.
    pub turns: u64,
    /// Total context tokens.
    pub tokens: u64,
    /// Total tool calls.
    pub tool_calls: u64,
    /// Total attempts.
    pub attempts: u64,
    /// Total output bytes.
    pub output_bytes: u64,
    /// Maximum attempt resident memory.
    pub peak_memory_bytes: u64,
    /// Total policy-defined cost minor units.
    pub cost_minor_units: u64,
}

/// One deterministic terminal diagnosis with no model-authored text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowSupervisorDiagnosis {
    /// Stable reason code.
    pub reason_code: String,
    /// Exact failed or interrupted step when one was selected.
    pub step_id: Option<String>,
    /// Stable safe-next-action code.
    pub safe_next_action_code: String,
    /// Whether a possibly completed effect requires reconciliation.
    pub uncertain_effect: bool,
}

/// Durable safe-boundary supervisor checkpoint.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowSupervisorCheckpoint {
    /// Exact workflow execution identity.
    pub execution_id: String,
    /// Immutable workflow definition digest.
    pub definition_sha256: String,
    /// Ordered policy digests in definition order.
    pub policy_sha256s: Vec<String>,
    /// Verified completed workflow steps.
    pub completed_step_ids: Vec<String>,
    /// Complete immutable attempt lineage.
    pub attempts: Vec<WorkflowAttemptLineage>,
    /// Current aggregate budget ledger.
    pub budgets: WorkflowSupervisorBudgetLedger,
    /// Digest of this checkpoint with this field zeroed.
    pub checkpoint_sha256: String,
}

/// Complete interface-neutral supervisor result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowSupervisorResult {
    /// Exact workflow execution identity.
    pub execution_id: String,
    /// Terminal supervisor state.
    pub state: WorkflowSupervisorState,
    /// Verified completed steps in definition order.
    pub completed_step_ids: Vec<String>,
    /// Complete attempt lineage.
    pub attempts: Vec<WorkflowAttemptLineage>,
    /// Aggregate budget ledger.
    pub budgets: WorkflowSupervisorBudgetLedger,
    /// Content-free terminal or interruption diagnosis.
    pub diagnosis: Option<WorkflowSupervisorDiagnosis>,
    /// Resumable checkpoint only for a safe interruption.
    pub checkpoint: Option<WorkflowSupervisorCheckpoint>,
}

/// Stable fail-closed supervisor error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowSupervisorError {
    /// Definition, policy, execution identity, or checkpoint input was invalid.
    InvalidInput,
    /// The workflow/policy admission boundary rejected the graph.
    DefinitionDenied,
    /// The coordinator adapter failed before producing a truthful terminal result.
    RuntimeFailed,
    /// Runtime request, events, outcome, preflight, or resource evidence was inconsistent.
    EvidenceDenied,
}

impl WorkflowSupervisorError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "workflow.supervisor.input-invalid",
            Self::DefinitionDenied => "workflow.supervisor.definition-denied",
            Self::RuntimeFailed => "workflow.supervisor.runtime-failed",
            Self::EvidenceDenied => "workflow.supervisor.evidence-denied",
        }
    }
}

impl std::fmt::Display for WorkflowSupervisorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for WorkflowSupervisorError {}

/// Runs or resumes one immutable admitted workflow through injected reusable coordinators.
pub fn supervise_verified_workflow<P: VerifiedWorkflowAttemptPort>(
    execution_id: &str,
    definition: &CanonicalWorkflowDefinition,
    policies: &[CanonicalStepExecutionPolicy],
    checkpoint: Option<&WorkflowSupervisorCheckpoint>,
    port: &mut P,
) -> Result<WorkflowSupervisorResult, WorkflowSupervisorError> {
    if !valid_identifier(execution_id)
        || definition.steps.is_empty()
        || definition.steps.len() > MAX_SUPERVISOR_STEPS
        || policies.len() != definition.steps.len()
    {
        return Err(WorkflowSupervisorError::InvalidInput);
    }
    let bindings = definition
        .steps
        .iter()
        .zip(policies)
        .map(|(step, policy)| WorkflowStepPolicyBinding {
            step_id: &step.step_id,
            policy,
        })
        .collect::<Vec<_>>();
    let admitted = admit_closed_workflow(definition, &bindings)
        .map_err(|_| WorkflowSupervisorError::DefinitionDenied)?;
    let policy_by_step = definition
        .steps
        .iter()
        .zip(policies)
        .map(|(step, policy)| (step.step_id.as_str(), policy))
        .collect::<BTreeMap<_, _>>();

    let mut completed = Vec::new();
    let mut attempts = Vec::new();
    let mut budgets = WorkflowSupervisorBudgetLedger::default();
    if let Some(checkpoint) = checkpoint {
        verify_checkpoint(
            checkpoint,
            execution_id,
            admitted.definition_sha256(),
            policies,
            &definition
                .steps
                .iter()
                .map(|step| step.step_id.as_str())
                .collect::<Vec<_>>(),
        )?;
        completed = checkpoint.completed_step_ids.clone();
        attempts = checkpoint.attempts.clone();
        budgets = checkpoint.budgets.clone();
    }

    let mut identities = IdentityLedger::from_attempts(&attempts)?;
    loop {
        if completed.len() == definition.steps.len() {
            return Ok(result(
                execution_id,
                WorkflowSupervisorState::Succeeded,
                completed,
                attempts,
                budgets,
                None,
                None,
            ));
        }
        let Some(step) = definition.steps.iter().find(|step| {
            !completed.contains(&step.step_id)
                && step
                    .depends_on
                    .iter()
                    .all(|dependency| completed.contains(dependency))
        }) else {
            return Ok(terminal(
                execution_id,
                WorkflowSupervisorState::Blocked,
                completed,
                attempts,
                budgets,
                "workflow.supervisor.dependency-open",
                None,
                "workflow.safe-next.resolve-dependency",
                false,
            ));
        };
        let policy = policy_by_step[step.step_id.as_str()];
        let step_attempts = attempts
            .iter()
            .filter(|attempt| attempt.step_id == step.step_id)
            .count();
        let mut no_progress = 0_u64;
        loop {
            let ordinal = u32::try_from(step_attempts)
                .ok()
                .and_then(|value| value.checked_add(u32::try_from(no_progress).ok()?))
                .and_then(|value| value.checked_add(1))
                .ok_or(WorkflowSupervisorError::EvidenceDenied)?;
            if u64::from(ordinal) > policy.budgets.attempts {
                return Ok(terminal(
                    execution_id,
                    WorkflowSupervisorState::Exhausted,
                    completed,
                    attempts,
                    budgets,
                    "workflow.supervisor.attempt-budget-exhausted",
                    Some(step.step_id.clone()),
                    "workflow.safe-next.replan",
                    false,
                ));
            }
            let prior_attempt_id = attempts
                .iter()
                .rev()
                .find(|attempt| attempt.step_id == step.step_id)
                .map(|attempt| attempt.attempt_id.as_str());
            let request = WorkflowAttemptRequest {
                execution_id,
                step_id: &step.step_id,
                plan_step_id: &policy.plan_step_id,
                policy_sha256: &policy.policy_sha256,
                attempt_ordinal: ordinal,
                prior_attempt_id,
                completed_dependencies: &step.depends_on,
            };
            let run = match port.run_attempt(request)? {
                WorkflowAttemptPortResult::Interrupted { reason_code } => {
                    if !valid_code(&reason_code) {
                        return Err(WorkflowSupervisorError::EvidenceDenied);
                    }
                    let checkpoint = seal_checkpoint(
                        execution_id,
                        admitted.definition_sha256(),
                        policies,
                        &completed,
                        &attempts,
                        &budgets,
                    )?;
                    return Ok(result(
                        execution_id,
                        WorkflowSupervisorState::Interrupted,
                        completed,
                        attempts,
                        budgets,
                        Some(WorkflowSupervisorDiagnosis {
                            reason_code,
                            step_id: Some(step.step_id.clone()),
                            safe_next_action_code: "workflow.safe-next.resume-checkpoint"
                                .to_owned(),
                            uncertain_effect: false,
                        }),
                        Some(checkpoint),
                    ));
                }
                WorkflowAttemptPortResult::Terminal(run) => *run,
            };
            let lineage = validate_run(step.step_id.as_str(), policy, ordinal, &run, &identities)?;
            consume_budgets(&mut budgets, policy, &lineage, &attempts)?;
            identities.insert(&lineage)?;
            attempts.push(lineage);

            if matches!(
                run.outcome.state,
                AgentStateKind::Success | AgentStateKind::NoOp
            ) {
                completed.push(step.step_id.clone());
                break;
            }
            let failure = run
                .failure_class
                .ok_or(WorkflowSupervisorError::EvidenceDenied)?;
            if run.uncertain_effect || failure == CanonicalWorkflowFailureClass::UncertainEffect {
                return Ok(terminal(
                    execution_id,
                    WorkflowSupervisorState::Uncertain,
                    completed,
                    attempts,
                    budgets,
                    "workflow.supervisor.effect-uncertain",
                    Some(step.step_id.clone()),
                    "workflow.safe-next.reconcile-effect",
                    true,
                ));
            }
            if run.outcome.state == AgentStateKind::Cancelled
                || failure == CanonicalWorkflowFailureClass::Cancellation
            {
                return Ok(terminal(
                    execution_id,
                    WorkflowSupervisorState::Cancelled,
                    completed,
                    attempts,
                    budgets,
                    "workflow.supervisor.cancelled",
                    Some(step.step_id.clone()),
                    "workflow.safe-next.none",
                    false,
                ));
            }
            let retry_eligible = failure == CanonicalWorkflowFailureClass::Transient
                && policy.effect_class.permits_automatic_retry()
                && policy.retry_class.is_automatic()
                && policy
                    .effect_class
                    .permitted_retry_classes()
                    .contains(&policy.retry_class)
                && u64::from(ordinal) < policy.budgets.attempts;
            if retry_eligible {
                no_progress = no_progress.saturating_add(1);
                if no_progress >= policy.budgets.no_progress_events {
                    return Ok(terminal(
                        execution_id,
                        WorkflowSupervisorState::Exhausted,
                        completed,
                        attempts,
                        budgets,
                        "workflow.supervisor.no-progress-exhausted",
                        Some(step.step_id.clone()),
                        "workflow.safe-next.replan",
                        false,
                    ));
                }
                continue;
            }
            let (state, reason, next) = match failure.default_disposition() {
                agentmage_kernel_contracts::CanonicalFailureDisposition::TerminalResourceExhausted => (
                    WorkflowSupervisorState::Exhausted,
                    "workflow.supervisor.resource-exhausted",
                    "workflow.safe-next.reduce-scope",
                ),
                agentmage_kernel_contracts::CanonicalFailureDisposition::TerminalCancelled => (
                    WorkflowSupervisorState::Cancelled,
                    "workflow.supervisor.cancelled",
                    "workflow.safe-next.none",
                ),
                agentmage_kernel_contracts::CanonicalFailureDisposition::ReconcileThenDecide => (
                    WorkflowSupervisorState::Uncertain,
                    "workflow.supervisor.reconciliation-required",
                    "workflow.safe-next.reconcile-effect",
                ),
                agentmage_kernel_contracts::CanonicalFailureDisposition::AwaitDependency => (
                    WorkflowSupervisorState::Blocked,
                    "workflow.supervisor.dependency-blocked",
                    "workflow.safe-next.resolve-dependency",
                ),
                agentmage_kernel_contracts::CanonicalFailureDisposition::AwaitFreshApproval => (
                    WorkflowSupervisorState::Blocked,
                    "workflow.supervisor.fresh-approval-required",
                    "workflow.safe-next.request-approval",
                ),
                _ => (
                    WorkflowSupervisorState::Failed,
                    "workflow.supervisor.attempt-failed",
                    "workflow.safe-next.inspect-diagnostic",
                ),
            };
            return Ok(terminal(
                execution_id,
                state,
                completed,
                attempts,
                budgets,
                reason,
                Some(step.step_id.clone()),
                next,
                state == WorkflowSupervisorState::Uncertain,
            ));
        }
    }
}

fn validate_run(
    step_id: &str,
    policy: &CanonicalStepExecutionPolicy,
    ordinal: u32,
    run: &VerifiedWorkflowStepRun,
    identities: &IdentityLedger,
) -> Result<WorkflowAttemptLineage, WorkflowSupervisorError> {
    if !valid_identifier(&run.attempt_id)
        || run.attempt_ordinal != ordinal
        || !run.preflight_current
        || run.preflight_policy_sha256 != policy.policy_sha256
        || run.uncertain_effect
            && matches!(
                run.outcome.state,
                AgentStateKind::Success | AgentStateKind::NoOp
            )
    {
        return Err(WorkflowSupervisorError::EvidenceDenied);
    }
    verify_runtime_run_request(&run.request)
        .map_err(|_| WorkflowSupervisorError::EvidenceDenied)?;
    verify_runtime_outcome(&run.outcome, &run.request)
        .map_err(|_| WorkflowSupervisorError::EvidenceDenied)?;
    let mut sequence = RuntimeEventSequence::new();
    for event in &run.events {
        if event.run_id != run.request.run_id
            || event.task_id != run.request.task.task_id
            || event.policy_id != run.request.policy_id
        {
            return Err(WorkflowSupervisorError::EvidenceDenied);
        }
        sequence
            .push(event)
            .map_err(|_| WorkflowSupervisorError::EvidenceDenied)?;
    }
    if !sequence.is_terminal() || run.events.is_empty() {
        return Err(WorkflowSupervisorError::EvidenceDenied);
    }
    let terminal_matches = run.events.last().is_some_and(|event| {
        matches!(
            &event.kind,
            RuntimeEventKind::RunTerminal { state, outcome_sha256 }
                if *state == run.outcome.state && outcome_sha256 == &run.outcome.outcome_sha256
        )
    });
    if !terminal_matches
        || matches!(
            run.outcome.state,
            AgentStateKind::Success | AgentStateKind::NoOp
        ) == run.failure_class.is_some()
    {
        return Err(WorkflowSupervisorError::EvidenceDenied);
    }
    let mut tool_call_ids = BTreeSet::new();
    let mut grant_ids = BTreeSet::new();
    let mut receipt_ids = BTreeSet::new();
    for event in &run.events {
        match &event.kind {
            RuntimeEventKind::ToolRequested { tool_call_id, .. }
            | RuntimeEventKind::ToolStarted { tool_call_id, .. }
            | RuntimeEventKind::AttemptStarted { tool_call_id, .. }
            | RuntimeEventKind::AttemptEnded { tool_call_id, .. } => {
                tool_call_ids.insert(tool_call_id.as_str().to_owned());
            }
            RuntimeEventKind::PermissionDecided {
                grant_id: Some(grant_id),
                ..
            } => {
                grant_ids.insert(grant_id.as_str().to_owned());
            }
            RuntimeEventKind::ToolCompleted {
                tool_call_id,
                receipt_id,
                ..
            } => {
                tool_call_ids.insert(tool_call_id.as_str().to_owned());
                receipt_ids.insert(receipt_id.as_str().to_owned());
            }
            RuntimeEventKind::ToolFailed {
                tool_call_id,
                receipt_id: Some(receipt_id),
                ..
            } => {
                tool_call_ids.insert(tool_call_id.as_str().to_owned());
                receipt_ids.insert(receipt_id.as_str().to_owned());
            }
            RuntimeEventKind::ToolFailed {
                tool_call_id,
                receipt_id: None,
                ..
            } => {
                tool_call_ids.insert(tool_call_id.as_str().to_owned());
            }
            _ => {}
        }
    }
    receipt_ids.extend(
        run.outcome
            .receipt_ids
            .iter()
            .map(|identity| identity.as_str().to_owned()),
    );
    let lineage = WorkflowAttemptLineage {
        step_id: step_id.to_owned(),
        plan_step_id: policy.plan_step_id.clone(),
        attempt_id: run.attempt_id.clone(),
        attempt_ordinal: ordinal,
        run_id: run.request.run_id.as_str().to_owned(),
        tool_call_ids: tool_call_ids.into_iter().collect(),
        grant_ids: grant_ids.into_iter().collect(),
        receipt_ids: receipt_ids.into_iter().collect(),
        outcome_sha256: run.outcome.outcome_sha256.clone(),
        state: run.outcome.state,
        budget_usage: attempt_budget_usage(policy, run)?,
    };
    identities.ensure_fresh(&lineage)?;
    Ok(lineage)
}

fn consume_budgets(
    ledger: &mut WorkflowSupervisorBudgetLedger,
    policy: &CanonicalStepExecutionPolicy,
    current: &WorkflowAttemptLineage,
    prior_attempts: &[WorkflowAttemptLineage],
) -> Result<(), WorkflowSupervisorError> {
    let mut step_usage = WorkflowSupervisorBudgetLedger::default();
    for attempt in prior_attempts
        .iter()
        .filter(|attempt| attempt.step_id == current.step_id)
        .chain(std::iter::once(current))
    {
        add_budget_usage(&mut step_usage, &attempt.budget_usage)?;
    }
    if step_usage.turns > policy.budgets.turns
        || step_usage.tokens > policy.budgets.tokens
        || step_usage.tool_calls > policy.budgets.tool_calls
        || step_usage.attempts > policy.budgets.attempts
        || step_usage.output_bytes > policy.budgets.output_bytes
        || step_usage.peak_memory_bytes > policy.budgets.memory_bytes
        || step_usage.cost_minor_units > policy.budgets.cost_minor_units
    {
        return Err(WorkflowSupervisorError::EvidenceDenied);
    }
    add_budget_usage(ledger, &current.budget_usage)
}

fn attempt_budget_usage(
    policy: &CanonicalStepExecutionPolicy,
    run: &VerifiedWorkflowStepRun,
) -> Result<WorkflowSupervisorBudgetLedger, WorkflowSupervisorError> {
    let output_bytes = run
        .outcome
        .output
        .as_ref()
        .map_or(0, |output| match output {
            agentmage_kernel_contracts::RuntimeOutput::Inline { payload } => {
                payload.bytes.len() as u64
            }
            agentmage_kernel_contracts::RuntimeOutput::Artifact { reference } => {
                reference.byte_size
            }
        });
    let request = &run.request;
    if u64::from(request.limits.max_turns) > policy.budgets.turns
        || u64::from(request.limits.max_tool_calls) > policy.budgets.tool_calls
        || request.limits.max_elapsed_ms > policy.budgets.duration_ms
        || request.limits.max_output_bytes > policy.budgets.output_bytes
        || u64::from(run.outcome.turn_count) > policy.budgets.turns
        || u64::from(run.outcome.tool_call_count) > policy.budgets.tool_calls
        || run.resources.context_tokens > policy.budgets.tokens
        || run.resources.memory_bytes > policy.budgets.memory_bytes
        || run.resources.cost_minor_units > policy.budgets.cost_minor_units
        || output_bytes > policy.budgets.output_bytes
    {
        return Err(WorkflowSupervisorError::EvidenceDenied);
    }
    Ok(WorkflowSupervisorBudgetLedger {
        turns: u64::from(run.outcome.turn_count),
        tokens: run.resources.context_tokens,
        tool_calls: u64::from(run.outcome.tool_call_count),
        attempts: 1,
        output_bytes,
        peak_memory_bytes: run.resources.memory_bytes,
        cost_minor_units: run.resources.cost_minor_units,
    })
}

fn add_budget_usage(
    ledger: &mut WorkflowSupervisorBudgetLedger,
    usage: &WorkflowSupervisorBudgetLedger,
) -> Result<(), WorkflowSupervisorError> {
    ledger.turns = ledger
        .turns
        .checked_add(usage.turns)
        .ok_or(WorkflowSupervisorError::EvidenceDenied)?;
    ledger.tokens = ledger
        .tokens
        .checked_add(usage.tokens)
        .ok_or(WorkflowSupervisorError::EvidenceDenied)?;
    ledger.tool_calls = ledger
        .tool_calls
        .checked_add(usage.tool_calls)
        .ok_or(WorkflowSupervisorError::EvidenceDenied)?;
    ledger.attempts = ledger
        .attempts
        .checked_add(usage.attempts)
        .ok_or(WorkflowSupervisorError::EvidenceDenied)?;
    ledger.output_bytes = ledger
        .output_bytes
        .checked_add(usage.output_bytes)
        .ok_or(WorkflowSupervisorError::EvidenceDenied)?;
    ledger.peak_memory_bytes = ledger.peak_memory_bytes.max(usage.peak_memory_bytes);
    ledger.cost_minor_units = ledger
        .cost_minor_units
        .checked_add(usage.cost_minor_units)
        .ok_or(WorkflowSupervisorError::EvidenceDenied)?;
    Ok(())
}

#[derive(Default)]
struct IdentityLedger {
    attempts: BTreeSet<String>,
    runs: BTreeSet<String>,
    tool_calls: BTreeSet<String>,
    grants: BTreeSet<String>,
    receipts: BTreeSet<String>,
}

impl IdentityLedger {
    fn from_attempts(attempts: &[WorkflowAttemptLineage]) -> Result<Self, WorkflowSupervisorError> {
        let mut result = Self::default();
        for attempt in attempts {
            result.insert(attempt)?;
        }
        Ok(result)
    }

    fn ensure_fresh(
        &self,
        attempt: &WorkflowAttemptLineage,
    ) -> Result<(), WorkflowSupervisorError> {
        if self.attempts.contains(&attempt.attempt_id)
            || self.runs.contains(&attempt.run_id)
            || attempt
                .tool_call_ids
                .iter()
                .any(|id| self.tool_calls.contains(id))
            || attempt.grant_ids.iter().any(|id| self.grants.contains(id))
            || attempt
                .receipt_ids
                .iter()
                .any(|id| self.receipts.contains(id))
        {
            return Err(WorkflowSupervisorError::EvidenceDenied);
        }
        Ok(())
    }

    fn insert(&mut self, attempt: &WorkflowAttemptLineage) -> Result<(), WorkflowSupervisorError> {
        self.ensure_fresh(attempt)?;
        self.attempts.insert(attempt.attempt_id.clone());
        self.runs.insert(attempt.run_id.clone());
        self.tool_calls
            .extend(attempt.tool_call_ids.iter().cloned());
        self.grants.extend(attempt.grant_ids.iter().cloned());
        self.receipts.extend(attempt.receipt_ids.iter().cloned());
        Ok(())
    }
}

fn seal_checkpoint(
    execution_id: &str,
    definition_sha256: &str,
    policies: &[CanonicalStepExecutionPolicy],
    completed_step_ids: &[String],
    attempts: &[WorkflowAttemptLineage],
    budgets: &WorkflowSupervisorBudgetLedger,
) -> Result<WorkflowSupervisorCheckpoint, WorkflowSupervisorError> {
    let mut checkpoint = WorkflowSupervisorCheckpoint {
        execution_id: execution_id.to_owned(),
        definition_sha256: definition_sha256.to_owned(),
        policy_sha256s: policies
            .iter()
            .map(|policy| policy.policy_sha256.clone())
            .collect(),
        completed_step_ids: completed_step_ids.to_vec(),
        attempts: attempts.to_vec(),
        budgets: budgets.clone(),
        checkpoint_sha256: ZERO_SHA256.to_owned(),
    };
    checkpoint.checkpoint_sha256 = checkpoint_digest(&checkpoint)?;
    Ok(checkpoint)
}

fn verify_checkpoint(
    checkpoint: &WorkflowSupervisorCheckpoint,
    execution_id: &str,
    definition_sha256: &str,
    policies: &[CanonicalStepExecutionPolicy],
    ordered_steps: &[&str],
) -> Result<(), WorkflowSupervisorError> {
    if checkpoint.execution_id != execution_id
        || checkpoint.definition_sha256 != definition_sha256
        || checkpoint.policy_sha256s
            != policies
                .iter()
                .map(|policy| policy.policy_sha256.clone())
                .collect::<Vec<_>>()
        || !checkpoint
            .completed_step_ids
            .iter()
            .map(String::as_str)
            .eq(ordered_steps
                .iter()
                .copied()
                .take(checkpoint.completed_step_ids.len()))
        || checkpoint.checkpoint_sha256 != checkpoint_digest(checkpoint)?
    {
        return Err(WorkflowSupervisorError::EvidenceDenied);
    }
    IdentityLedger::from_attempts(&checkpoint.attempts)?;
    let policy_by_step = ordered_steps
        .iter()
        .copied()
        .zip(policies)
        .collect::<BTreeMap<_, _>>();
    let mut cursor = 0_usize;
    for completed_step in &checkpoint.completed_step_ids {
        let policy = policy_by_step
            .get(completed_step.as_str())
            .copied()
            .ok_or(WorkflowSupervisorError::EvidenceDenied)?;
        let group_start = cursor;
        while checkpoint
            .attempts
            .get(cursor)
            .is_some_and(|attempt| attempt.step_id == *completed_step)
        {
            let attempt = &checkpoint.attempts[cursor];
            let expected_ordinal = u32::try_from(cursor - group_start + 1)
                .map_err(|_| WorkflowSupervisorError::EvidenceDenied)?;
            if attempt.plan_step_id != policy.plan_step_id
                || attempt.attempt_ordinal != expected_ordinal
                || !valid_identifier(&attempt.attempt_id)
                || !valid_identifier(&attempt.run_id)
                || attempt.tool_call_ids.iter().any(|id| !valid_identifier(id))
                || attempt.grant_ids.iter().any(|id| !valid_identifier(id))
                || attempt.receipt_ids.iter().any(|id| !valid_identifier(id))
                || attempt.outcome_sha256.len() != 64
                || !attempt
                    .outcome_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
                || attempt.budget_usage.attempts != 1
            {
                return Err(WorkflowSupervisorError::EvidenceDenied);
            }
            cursor += 1;
        }
        if cursor == group_start
            || !checkpoint.attempts[cursor - 1].state.is_success()
            || checkpoint.attempts[group_start..cursor - 1]
                .iter()
                .any(|attempt| attempt.state.is_success())
        {
            return Err(WorkflowSupervisorError::EvidenceDenied);
        }
    }
    if cursor != checkpoint.attempts.len() {
        return Err(WorkflowSupervisorError::EvidenceDenied);
    }
    let mut rebuilt_budgets = WorkflowSupervisorBudgetLedger::default();
    for (index, attempt) in checkpoint.attempts.iter().enumerate() {
        let policy = policy_by_step
            .get(attempt.step_id.as_str())
            .copied()
            .ok_or(WorkflowSupervisorError::EvidenceDenied)?;
        consume_budgets(
            &mut rebuilt_budgets,
            policy,
            attempt,
            &checkpoint.attempts[..index],
        )?;
    }
    if rebuilt_budgets != checkpoint.budgets {
        return Err(WorkflowSupervisorError::EvidenceDenied);
    }
    Ok(())
}

fn checkpoint_digest(
    checkpoint: &WorkflowSupervisorCheckpoint,
) -> Result<String, WorkflowSupervisorError> {
    let mut candidate = checkpoint.clone();
    candidate.checkpoint_sha256 = ZERO_SHA256.to_owned();
    serde_json::to_vec(&candidate)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| WorkflowSupervisorError::EvidenceDenied)
}

#[allow(clippy::too_many_arguments)]
fn terminal(
    execution_id: &str,
    state: WorkflowSupervisorState,
    completed_step_ids: Vec<String>,
    attempts: Vec<WorkflowAttemptLineage>,
    budgets: WorkflowSupervisorBudgetLedger,
    reason_code: &str,
    step_id: Option<String>,
    safe_next_action_code: &str,
    uncertain_effect: bool,
) -> WorkflowSupervisorResult {
    result(
        execution_id,
        state,
        completed_step_ids,
        attempts,
        budgets,
        Some(WorkflowSupervisorDiagnosis {
            reason_code: reason_code.to_owned(),
            step_id,
            safe_next_action_code: safe_next_action_code.to_owned(),
            uncertain_effect,
        }),
        None,
    )
}

fn result(
    execution_id: &str,
    state: WorkflowSupervisorState,
    completed_step_ids: Vec<String>,
    attempts: Vec<WorkflowAttemptLineage>,
    budgets: WorkflowSupervisorBudgetLedger,
    diagnosis: Option<WorkflowSupervisorDiagnosis>,
    checkpoint: Option<WorkflowSupervisorCheckpoint>,
) -> WorkflowSupervisorResult {
    WorkflowSupervisorResult {
        execution_id: execution_id.to_owned(),
        state,
        completed_step_ids,
        attempts,
        budgets,
        diagnosis,
        checkpoint,
    }
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.is_ascii()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn valid_code(value: &str) -> bool {
    valid_identifier(value) && value.contains('.')
}

fn sha256(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use agentmage_kernel_contracts::{
        ApprovalId, AuthorityClass, BudgetLimit, BudgetResource, CONTRACT_SCHEMA_VERSION,
        CanonicalApprovalRequirement, CanonicalDiagnosticDisclosure, CanonicalEffectClass,
        CanonicalExecutionBudgets, CanonicalIdempotencyRequirement, CanonicalRetryClass,
        CanonicalSchemaBinding, CanonicalVerificationRequirement, CanonicalWorkflowStep,
        ContextSensitivity, CorrelationId, DataSensitivity, EvidenceId, EvidenceKind,
        EvidenceReference, GrantId, GrantOperation, ModelRunId, PlanId, PolicyId, ReceiptId,
        RepositorySnapshotId, RollbackPlan, RuntimeArtifactId, RuntimeEventId,
        RuntimeEventRetention, RuntimeEventRetentionKind, RuntimeOperationId,
        RuntimePayloadReference, RuntimePermissionDisposition, RuntimeRunId, RuntimeRunLimits,
        RuntimeSessionMode, RuntimeTurnId, SessionId, StopCondition, StopConditionKind, Task,
        TaskId, TaskStatus, ToolCallId, ToolCatalogId, WorkPacket, WorkPacketId, WorkPacketState,
        WorkspaceId,
    };

    use super::*;
    use crate::{
        engineering_records::canonical_record_sha256,
        model_codec::tests_support::profile,
        runtime_coordinator::{
            runtime_tool_catalog_sha256, seal_runtime_outcome, seal_runtime_run_request,
        },
        runtime_event::{runtime_event_persistence, seal_runtime_event},
    };

    const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[derive(Clone, Copy)]
    enum Script {
        Success,
        Transient,
        Uncertain,
        Cancelled,
        Resource,
        StalePreflight,
        Oversize,
        FalseSuccess,
        ReusedSuccess,
        Interrupt,
    }

    struct ScriptedPort {
        scripts: VecDeque<Script>,
        selected_steps: Vec<String>,
        requests: Vec<(String, u32, Option<String>)>,
        next_run: u32,
    }

    impl ScriptedPort {
        fn new(scripts: impl IntoIterator<Item = Script>) -> Self {
            Self {
                scripts: scripts.into_iter().collect(),
                selected_steps: Vec::new(),
                requests: Vec::new(),
                next_run: 1,
            }
        }
    }

    impl VerifiedWorkflowAttemptPort for ScriptedPort {
        fn run_attempt(
            &mut self,
            request: WorkflowAttemptRequest<'_>,
        ) -> Result<WorkflowAttemptPortResult, WorkflowSupervisorError> {
            self.selected_steps.push(request.step_id.to_owned());
            self.requests.push((
                request.step_id.to_owned(),
                request.attempt_ordinal,
                request.prior_attempt_id.map(str::to_owned),
            ));
            let script = self
                .scripts
                .pop_front()
                .ok_or(WorkflowSupervisorError::RuntimeFailed)?;
            if matches!(script, Script::Interrupt) {
                return Ok(WorkflowAttemptPortResult::Interrupted {
                    reason_code: "workflow.interrupted.safe-boundary".to_owned(),
                });
            }
            let suffix = if matches!(script, Script::ReusedSuccess) {
                self.next_run.saturating_sub(1)
            } else {
                let suffix = self.next_run;
                self.next_run += 1;
                suffix
            };
            let (state, failure_class, uncertain_effect) = match script {
                Script::Success | Script::FalseSuccess | Script::ReusedSuccess => {
                    (AgentStateKind::Success, None, false)
                }
                Script::Transient => (
                    AgentStateKind::Failed,
                    Some(CanonicalWorkflowFailureClass::Transient),
                    false,
                ),
                Script::Uncertain => (
                    AgentStateKind::Uncertain,
                    Some(CanonicalWorkflowFailureClass::UncertainEffect),
                    true,
                ),
                Script::Cancelled => (
                    AgentStateKind::Cancelled,
                    Some(CanonicalWorkflowFailureClass::Cancellation),
                    false,
                ),
                Script::Resource | Script::Oversize => (
                    AgentStateKind::Exhausted,
                    Some(CanonicalWorkflowFailureClass::Resource),
                    false,
                ),
                Script::StalePreflight => (
                    AgentStateKind::Blocked,
                    Some(CanonicalWorkflowFailureClass::Preflight),
                    false,
                ),
                Script::Interrupt => unreachable!(),
            };
            let mut run = fixture_run(request, suffix, state, failure_class, uncertain_effect);
            if matches!(script, Script::StalePreflight) {
                run.preflight_current = false;
            }
            if matches!(script, Script::Oversize) {
                run.resources.memory_bytes = 1_000_001;
            }
            if matches!(script, Script::FalseSuccess) {
                run.failure_class = Some(CanonicalWorkflowFailureClass::Verification);
            }
            Ok(WorkflowAttemptPortResult::Terminal(Box::new(run)))
        }
    }

    #[test]
    fn three_dependency_steps_complete_in_definition_order() {
        let (definition, policies) = workflow();
        let mut port = ScriptedPort::new([Script::Success, Script::Success, Script::Success]);
        let result =
            supervise_verified_workflow("execution-0001", &definition, &policies, None, &mut port)
                .expect("workflow succeeds");

        assert_eq!(result.state, WorkflowSupervisorState::Succeeded);
        assert_eq!(result.completed_step_ids, ["step-1", "step-2", "step-3"]);
        assert_eq!(port.selected_steps, result.completed_step_ids);
        assert_eq!(result.attempts.len(), 3);
        assert_eq!(result.budgets.attempts, 3);
        emit_evidence("STORY_23_6_SUCCESS", &result);
    }

    #[test]
    fn transient_read_retries_with_fresh_lineage_and_prior_attempt_binding() {
        let (definition, policies) = workflow();
        let mut port = ScriptedPort::new([
            Script::Transient,
            Script::Success,
            Script::Success,
            Script::Success,
        ]);
        let result =
            supervise_verified_workflow("execution-retry", &definition, &policies, None, &mut port)
                .expect("fresh retry succeeds");

        assert_eq!(result.state, WorkflowSupervisorState::Succeeded);
        assert_eq!(result.attempts.len(), 4);
        assert_eq!(result.attempts[0].attempt_ordinal, 1);
        assert_eq!(result.attempts[1].attempt_ordinal, 2);
        assert_ne!(result.attempts[0].attempt_id, result.attempts[1].attempt_id);
        assert_ne!(result.attempts[0].run_id, result.attempts[1].run_id);
        assert_ne!(
            result.attempts[0].tool_call_ids,
            result.attempts[1].tool_call_ids
        );
        assert_ne!(result.attempts[0].grant_ids, result.attempts[1].grant_ids);
        assert_ne!(
            result.attempts[0].receipt_ids,
            result.attempts[1].receipt_ids
        );
        assert_eq!(port.requests[1].2.as_deref(), Some("attempt-1"));
        emit_evidence("STORY_23_6_RETRY", &result);
    }

    #[test]
    fn uncertainty_cancellation_and_resource_failure_never_false_complete() {
        let (definition, policies) = workflow();
        let mut diagnoses = Vec::new();
        let started = std::time::Instant::now();
        for (script, expected) in [
            (Script::Uncertain, WorkflowSupervisorState::Uncertain),
            (Script::Cancelled, WorkflowSupervisorState::Cancelled),
            (Script::Resource, WorkflowSupervisorState::Exhausted),
        ] {
            let mut port = ScriptedPort::new([script]);
            let result = supervise_verified_workflow(
                "execution-terminal",
                &definition,
                &policies,
                None,
                &mut port,
            )
            .expect("terminal diagnosis is truthful");
            assert_eq!(result.state, expected);
            assert!(result.completed_step_ids.is_empty());
            assert!(result.diagnosis.is_some());
            diagnoses.push(result);
        }
        let terminal_diagnosis_latency_us = started.elapsed().as_micros();
        assert!(terminal_diagnosis_latency_us < 1_000_000);
        eprintln!(
            "STORY_50_3_TERMINAL_DIAGNOSIS_LATENCY_US={terminal_diagnosis_latency_us};CEILING_US=1000000"
        );
        emit_evidence("STORY_23_6_TERMINALS", &diagnoses);
    }

    #[test]
    fn stale_preflight_oversize_evidence_and_false_success_fail_closed() {
        let (definition, policies) = workflow();
        for script in [
            Script::StalePreflight,
            Script::Oversize,
            Script::FalseSuccess,
        ] {
            let mut port = ScriptedPort::new([script]);
            assert_eq!(
                supervise_verified_workflow(
                    "execution-denied",
                    &definition,
                    &policies,
                    None,
                    &mut port,
                ),
                Err(WorkflowSupervisorError::EvidenceDenied),
            );
        }
    }

    #[test]
    fn retry_reusing_authority_bearing_identities_is_denied() {
        let (definition, policies) = workflow();
        let mut port = ScriptedPort::new([Script::Transient, Script::ReusedSuccess]);
        assert_eq!(
            supervise_verified_workflow(
                "execution-reuse-denied",
                &definition,
                &policies,
                None,
                &mut port,
            ),
            Err(WorkflowSupervisorError::EvidenceDenied),
        );
    }

    #[test]
    fn repeated_transient_state_exhausts_no_progress_budget() {
        let (definition, mut policies) = workflow();
        for policy in &mut policies {
            policy.budgets.no_progress_events = 2;
            policy.policy_sha256 = ZERO_SHA256.to_owned();
            policy.policy_sha256 = canonical_record_sha256(policy).expect("policy seals");
        }
        let mut definition = definition;
        for (step, policy) in definition.steps.iter_mut().zip(&policies) {
            step.budgets = policy.budgets.clone();
        }
        definition.definition_sha256 = ZERO_SHA256.to_owned();
        definition.definition_sha256 =
            canonical_record_sha256(&definition).expect("definition seals");
        let mut port = ScriptedPort::new([Script::Transient, Script::Transient]);
        let result = supervise_verified_workflow(
            "execution-stalled",
            &definition,
            &policies,
            None,
            &mut port,
        )
        .expect("no progress is a typed terminal result");
        assert_eq!(result.state, WorkflowSupervisorState::Exhausted);
        assert_eq!(result.attempts.len(), 2);
        assert_eq!(
            result.diagnosis.expect("diagnosis").reason_code,
            "workflow.supervisor.no-progress-exhausted",
        );
    }

    #[test]
    fn safe_interruption_resumes_without_replaying_completed_steps_and_rejects_tampering() {
        let (definition, policies) = workflow();
        let mut first = ScriptedPort::new([Script::Success, Script::Interrupt]);
        let interrupted = supervise_verified_workflow(
            "execution-resume",
            &definition,
            &policies,
            None,
            &mut first,
        )
        .expect("safe interruption checkpoints");
        assert_eq!(interrupted.state, WorkflowSupervisorState::Interrupted);
        assert_eq!(interrupted.completed_step_ids, ["step-1"]);
        let checkpoint = interrupted.checkpoint.expect("checkpoint retained");

        let mut resumed = ScriptedPort::new([Script::Success, Script::Success]);
        resumed.next_run = 10;
        let completed = supervise_verified_workflow(
            "execution-resume",
            &definition,
            &policies,
            Some(&checkpoint),
            &mut resumed,
        )
        .expect("checkpoint resumes");
        assert_eq!(completed.state, WorkflowSupervisorState::Succeeded);
        assert_eq!(resumed.selected_steps, ["step-2", "step-3"]);
        emit_evidence("STORY_23_6_RESTART", &completed);

        let mut forged_ledger = checkpoint.clone();
        forged_ledger.budgets.tokens = 0;
        forged_ledger.checkpoint_sha256 =
            checkpoint_digest(&forged_ledger).expect("forged checkpoint rehashes");
        let mut unused = ScriptedPort::new([]);
        assert_eq!(
            supervise_verified_workflow(
                "execution-resume",
                &definition,
                &policies,
                Some(&forged_ledger),
                &mut unused,
            ),
            Err(WorkflowSupervisorError::EvidenceDenied),
        );

        let mut tampered = checkpoint;
        tampered.completed_step_ids.push("step-2".to_owned());
        let mut unused = ScriptedPort::new([]);
        assert_eq!(
            supervise_verified_workflow(
                "execution-resume",
                &definition,
                &policies,
                Some(&tampered),
                &mut unused,
            ),
            Err(WorkflowSupervisorError::EvidenceDenied),
        );
    }

    fn workflow() -> (
        CanonicalWorkflowDefinition,
        Vec<CanonicalStepExecutionPolicy>,
    ) {
        let budgets = CanonicalExecutionBudgets {
            turns: 4,
            tokens: 1_000,
            duration_ms: 100_000,
            tool_calls: 4,
            attempts: 3,
            no_progress_events: 3,
            output_bytes: 4_096,
            memory_bytes: 1_000_000,
            cost_minor_units: 100,
        };
        let mut policies = (1..=3)
            .map(|index| CanonicalStepExecutionPolicy {
                schema_version: CONTRACT_SCHEMA_VERSION,
                policy_id: format!("step-policy-{index}"),
                plan_id: PlanId::from_raw("plan-supervisor"),
                plan_step_id: PlanStepId::from_raw(format!("plan-supervisor:step:{index:04}")),
                plan_revision: 1,
                preflight_policy_id: format!("preflight-policy-{index}"),
                required_preflight_ids: vec![format!("workspace-current-{index}")],
                side_effect_policy_id: format!("effect-policy-{index}"),
                effect_class: CanonicalEffectClass::ReadOnly,
                approval_policy_id: format!("approval-policy-{index}"),
                approval_requirement: CanonicalApprovalRequirement::NotRequired,
                idempotency_policy_id: format!("idempotency-policy-{index}"),
                idempotency_key_requirement: CanonicalIdempotencyRequirement::NotApplicable,
                verifier_policy_id: format!("verifier-policy-{index}"),
                verification_requirement:
                    CanonicalVerificationRequirement::VerifierEvidenceRequired,
                required_verifier_ids: vec![format!("verifier-{index}")],
                deferral_reason_code: None,
                retry_policy_id: format!("retry-policy-{index}"),
                retry_class: CanonicalRetryClass::RecoverableRead,
                budget_policy_id: format!("budget-policy-{index}"),
                budgets: budgets.clone(),
                diagnostic_policy_id: format!("diagnostic-policy-{index}"),
                diagnostic_disclosure: CanonicalDiagnosticDisclosure::ContentFreeCodes,
                recorded_at: "2026-08-31T12:00:00Z".to_owned(),
                policy_sha256: ZERO_SHA256.to_owned(),
            })
            .collect::<Vec<_>>();
        for policy in &mut policies {
            policy.policy_sha256 = canonical_record_sha256(policy).expect("policy seals");
        }
        let mut definition = CanonicalWorkflowDefinition {
            schema_version: CONTRACT_SCHEMA_VERSION,
            workflow_id: "verified-supervisor-fixture".to_owned(),
            workflow_version: 1,
            input_schema: CanonicalSchemaBinding {
                schema_id: "workflow-input".to_owned(),
                schema_version: 1,
                schema_sha256: SHA.to_owned(),
            },
            output_schema: CanonicalSchemaBinding {
                schema_id: "workflow-output".to_owned(),
                schema_version: 1,
                schema_sha256: SHA.to_owned(),
            },
            steps: (1..=3)
                .map(|index| CanonicalWorkflowStep {
                    step_id: format!("step-{index}"),
                    depends_on: if index == 1 {
                        Vec::new()
                    } else {
                        vec![format!("step-{}", index - 1)]
                    },
                    model_role: Some("worker".to_owned()),
                    tool_id: None,
                    effect_class: CanonicalEffectClass::ReadOnly,
                    retry_class: CanonicalRetryClass::RecoverableRead,
                    verifier_ids: vec![format!("verifier-{index}")],
                    budgets: budgets.clone(),
                })
                .collect(),
            entry_step_ids: vec!["step-1".to_owned()],
            definition_sha256: ZERO_SHA256.to_owned(),
        };
        definition.definition_sha256 =
            canonical_record_sha256(&definition).expect("definition seals");
        (definition, policies)
    }

    fn fixture_run(
        supervisor_request: WorkflowAttemptRequest<'_>,
        suffix: u32,
        state: AgentStateKind,
        failure_class: Option<CanonicalWorkflowFailureClass>,
        uncertain_effect: bool,
    ) -> VerifiedWorkflowStepRun {
        let request = runtime_request(suffix);
        let mut stream = FixtureStream::new(&request, suffix);
        let turn_id = RuntimeTurnId::from_raw(format!("turn-{suffix}"));
        let operation_id = RuntimeOperationId::from_raw(format!("operation-{suffix}"));
        let model_run_id = ModelRunId::from_raw(format!("model-run-{suffix}"));
        let tool_call_id = ToolCallId::from_raw(format!("tool-call-{suffix}"));
        let approval_id = ApprovalId::from_raw(format!("approval-{suffix}"));
        let grant_id = GrantId::from_raw(format!("grant-{suffix}"));
        let receipt_id = ReceiptId::from_raw(format!("receipt-{suffix}"));
        let mut events = vec![stream.event(
            RuntimeEventKind::RunStarted {
                request_sha256: request.request_sha256.clone(),
            },
            None,
            None,
        )];
        if state == AgentStateKind::Cancelled {
            let cancellation_id = agentmage_kernel_contracts::CancellationId::from_raw(format!(
                "cancellation-{suffix}"
            ));
            events.push(stream.event(
                RuntimeEventKind::CancellationRequested {
                    cancellation_id: cancellation_id.clone(),
                },
                None,
                None,
            ));
            events.push(stream.event(
                RuntimeEventKind::CancellationObserved { cancellation_id },
                None,
                None,
            ));
        } else {
            events.push(stream.event(RuntimeEventKind::TurnStarted, Some(&turn_id), None));
            events.push(stream.event(
                RuntimeEventKind::ModelRequested {
                    model_run_id: model_run_id.clone(),
                    request_sha256: SHA.to_owned(),
                },
                Some(&turn_id),
                None,
            ));
            events.push(stream.event(
                RuntimeEventKind::ModelCompleted {
                    model_run_id,
                    result_sha256: "b".repeat(64),
                },
                Some(&turn_id),
                None,
            ));
            events.push(stream.event(
                RuntimeEventKind::ToolRequested {
                    tool_call_id: tool_call_id.clone(),
                    arguments_sha256: "c".repeat(64),
                },
                Some(&turn_id),
                Some(&operation_id),
            ));
            events.push(stream.event(
                RuntimeEventKind::PermissionRequested {
                    approval_id: approval_id.clone(),
                    operation: GrantOperation::WorkspaceRead,
                    preview_sha256: "d".repeat(64),
                    expires_at_epoch_ms: 10_000,
                },
                Some(&turn_id),
                Some(&operation_id),
            ));
            events.push(stream.event(
                RuntimeEventKind::PermissionDecided {
                    approval_id,
                    disposition: RuntimePermissionDisposition::Allow,
                    grant_id: Some(grant_id),
                    decision_sha256: "e".repeat(64),
                },
                Some(&turn_id),
                Some(&operation_id),
            ));
            events.push(stream.event(
                RuntimeEventKind::ToolStarted {
                    tool_call_id: tool_call_id.clone(),
                    authority_sha256: "f".repeat(64),
                },
                Some(&turn_id),
                Some(&operation_id),
            ));
            let artifact_id = RuntimeArtifactId::from_raw(format!("artifact-{suffix}"));
            events.push(stream.event(
                RuntimeEventKind::ArtifactCreated {
                    artifact_id,
                    manifest_sha256: "f".repeat(64),
                },
                Some(&turn_id),
                Some(&operation_id),
            ));
            events.push(stream.event(
                RuntimeEventKind::ToolCompleted {
                    tool_call_id,
                    receipt_id: receipt_id.clone(),
                    result_sha256: SHA.to_owned(),
                },
                Some(&turn_id),
                Some(&operation_id),
            ));
            events.push(stream.event(
                RuntimeEventKind::TurnCompleted {
                    outcome_sha256: "b".repeat(64),
                },
                Some(&turn_id),
                None,
            ));
        }
        let prior = events.last().expect("prior event");
        let outcome = seal_runtime_outcome(
            RuntimeOutcome {
                schema_version: CONTRACT_SCHEMA_VERSION,
                run_id: request.run_id.clone(),
                session_id: request.session_id.clone(),
                task_id: request.task.task_id.clone(),
                request_sha256: request.request_sha256.clone(),
                state,
                turn_count: u32::from(state != AgentStateKind::Cancelled),
                model_call_count: u32::from(state != AgentStateKind::Cancelled),
                tool_call_count: u32::from(state != AgentStateKind::Cancelled),
                prior_event_id: prior.event_id.clone(),
                prior_event_sha256: prior.event_sha256.clone(),
                evidence: if state.is_success() {
                    vec![evidence(suffix)]
                } else {
                    Vec::new()
                },
                receipt_ids: if state == AgentStateKind::Cancelled {
                    Vec::new()
                } else {
                    vec![receipt_id]
                },
                unresolved_codes: if state.is_success() {
                    Vec::new()
                } else {
                    vec!["workflow.fixture.failure".to_owned()]
                },
                output: None,
                answer_evidence: None,
                outcome_sha256: ZERO_SHA256.to_owned(),
            },
            &request,
        )
        .expect("outcome seals");
        events.push(stream.event(
            RuntimeEventKind::RunTerminal {
                state,
                outcome_sha256: outcome.outcome_sha256.clone(),
            },
            None,
            None,
        ));
        VerifiedWorkflowStepRun {
            attempt_id: format!("attempt-{suffix}"),
            attempt_ordinal: supervisor_request.attempt_ordinal,
            preflight_policy_sha256: supervisor_request.policy_sha256.to_owned(),
            preflight_current: true,
            request,
            events,
            outcome,
            failure_class,
            uncertain_effect,
            resources: WorkflowAttemptResources {
                context_tokens: 10,
                memory_bytes: 1_000,
                cost_minor_units: 1,
            },
        }
    }

    struct FixtureStream<'a> {
        request: &'a RuntimeRunRequest,
        suffix: u32,
        sequence: u64,
        previous_sha256: String,
        causation_event_id: Option<RuntimeEventId>,
    }

    impl<'a> FixtureStream<'a> {
        fn new(request: &'a RuntimeRunRequest, suffix: u32) -> Self {
            Self {
                request,
                suffix,
                sequence: 0,
                previous_sha256: ZERO_SHA256.to_owned(),
                causation_event_id: None,
            }
        }

        fn event(
            &mut self,
            kind: RuntimeEventKind,
            turn_id: Option<&RuntimeTurnId>,
            operation_id: Option<&RuntimeOperationId>,
        ) -> RuntimeEvent {
            let event_id =
                RuntimeEventId::from_raw(format!("event-{}-{}", self.suffix, self.sequence));
            let payload_reference = match &kind {
                RuntimeEventKind::ArtifactCreated { artifact_id, .. } => {
                    Some(RuntimePayloadReference {
                        artifact_id: artifact_id.clone(),
                        sha256: SHA.to_owned(),
                        byte_size: 32,
                        media_type: "application/json".to_owned(),
                    })
                }
                _ => None,
            };
            let event = seal_runtime_event(RuntimeEvent {
                schema_version: CONTRACT_SCHEMA_VERSION,
                event_id: event_id.clone(),
                run_id: self.request.run_id.clone(),
                session_id: self.request.session_id.clone(),
                task_id: self.request.task.task_id.clone(),
                turn_id: turn_id.cloned(),
                operation_id: operation_id.cloned(),
                correlation_id: CorrelationId::from_raw(format!("correlation-{}", self.suffix)),
                causation_event_id: self.causation_event_id.clone(),
                sequence: self.sequence,
                occurred_at_epoch_ms: 1_000 + self.sequence,
                sensitivity: ContextSensitivity::Internal,
                retention: RuntimeEventRetention {
                    kind: RuntimeEventRetentionKind::Ephemeral,
                    expires_at_epoch_ms: None,
                },
                persistence: runtime_event_persistence(&kind),
                policy_id: self.request.policy_id.clone(),
                payload_reference,
                kind,
                previous_event_sha256: self.previous_sha256.clone(),
                event_sha256: ZERO_SHA256.to_owned(),
            })
            .expect("event seals");
            self.sequence += 1;
            self.previous_sha256.clone_from(&event.event_sha256);
            self.causation_event_id = Some(event_id);
            event
        }
    }

    fn runtime_request(suffix: u32) -> RuntimeRunRequest {
        let profile = profile("workflow-supervisor");
        let tool_catalog_id = ToolCatalogId::from_raw(format!("catalog-{suffix}"));
        seal_runtime_run_request(RuntimeRunRequest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            run_id: RuntimeRunId::from_raw(format!("runtime-run-{suffix}")),
            session_id: SessionId::from_raw(format!("session-{suffix}")),
            mode: RuntimeSessionMode::EphemeralReadOnly,
            task: Task {
                schema_version: CONTRACT_SCHEMA_VERSION,
                task_id: TaskId::from_raw(format!("task-{suffix}")),
                session_id: SessionId::from_raw(format!("session-{suffix}")),
                objective: "Verify one dependency-ready workflow step".to_owned(),
                acceptance_criteria: vec!["Current verifier evidence admits completion".to_owned()],
                constraints: vec!["Read only".to_owned()],
                status: TaskStatus::Ready,
            },
            work_packet: packet(suffix),
            workspace_id: WorkspaceId::from_raw("workspace-supervisor"),
            workspace_snapshot_sha256: SHA.to_owned(),
            repository_snapshot_id: RepositorySnapshotId::from_raw("snapshot-supervisor"),
            repository_snapshot_sha256: "b".repeat(64),
            context_budget: profile.context.clone(),
            model_profile: profile,
            tool_catalog_sha256: runtime_tool_catalog_sha256(&tool_catalog_id, &[])
                .expect("catalog digest"),
            tool_catalog_id,
            visible_tools: Vec::new(),
            policy_id: PolicyId::from_raw(format!("runtime-policy-{suffix}")),
            policy_sha256: "c".repeat(64),
            limits: RuntimeRunLimits {
                max_turns: 4,
                max_model_calls: 4,
                max_tool_calls: 4,
                max_repeated_tool_calls: 4,
                max_tool_call_depth: 1,
                max_no_progress_turns: 2,
                max_context_refreshes: 4,
                max_events: 128,
                max_elapsed_ms: 100_000,
                max_output_bytes: 4_096,
            },
            event_cursor: None,
            request_sha256: ZERO_SHA256.to_owned(),
        })
        .expect("request seals")
    }

    fn packet(suffix: u32) -> WorkPacket {
        WorkPacket {
            schema_version: CONTRACT_SCHEMA_VERSION,
            work_packet_id: WorkPacketId::from_raw(format!("packet-{suffix}")),
            task_id: TaskId::from_raw(format!("task-{suffix}")),
            revision: 1,
            objective: "Verify one dependency-ready workflow step".to_owned(),
            reason: "Exercise the interface-neutral workflow supervisor".to_owned(),
            owner: "supervisor-fixture".to_owned(),
            authoritative_evidence: Vec::new(),
            mutable_files: Vec::new(),
            protected_files: vec!["fixture.txt".to_owned()],
            expected_output: "Current verifier evidence".to_owned(),
            acceptance_checks: vec!["Current postconditions pass".to_owned()],
            required_evidence: vec![EvidenceKind::Observation],
            required_capability_class: AuthorityClass::Observe,
            budgets: vec![
                BudgetLimit {
                    resource: BudgetResource::PlanSteps,
                    limit: 4,
                },
                BudgetLimit {
                    resource: BudgetResource::ToolCallDepth,
                    limit: 1,
                },
                BudgetLimit {
                    resource: BudgetResource::ModelCalls,
                    limit: 4,
                },
                BudgetLimit {
                    resource: BudgetResource::ToolCalls,
                    limit: 4,
                },
                BudgetLimit {
                    resource: BudgetResource::InputBytes,
                    limit: 4_096,
                },
                BudgetLimit {
                    resource: BudgetResource::OutputBytes,
                    limit: 4_096,
                },
                BudgetLimit {
                    resource: BudgetResource::ElapsedMilliseconds,
                    limit: 100_000,
                },
                BudgetLimit {
                    resource: BudgetResource::MemoryBytes,
                    limit: 1_000_000,
                },
                BudgetLimit {
                    resource: BudgetResource::DiskBytes,
                    limit: 1_000_000,
                },
                BudgetLimit {
                    resource: BudgetResource::ProcessCount,
                    limit: 1,
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
                description: "Read-only execution has no state change".to_owned(),
            },
            sensitivity: DataSensitivity::Ephemeral,
            last_verification_date: "2026-08-31".to_owned(),
            next_action: None,
            next_review: None,
            status_reason: None,
            disposition: None,
            completion_evidence: Vec::new(),
            superseding_work: None,
            validation_issues: Vec::new(),
            plan_id: Some(PlanId::from_raw("plan-supervisor")),
            state: WorkPacketState::Active,
        }
    }

    fn evidence(suffix: u32) -> EvidenceReference {
        EvidenceReference {
            schema_version: CONTRACT_SCHEMA_VERSION,
            evidence_id: EvidenceId::from_raw(format!("evidence-{suffix}")),
            kind: EvidenceKind::Validation,
            source_id: "supervisor-fixture".to_owned(),
            object_id: "fixture.txt".to_owned(),
            fragment: Some("postcondition:current".to_owned()),
            content_sha256: SHA.to_owned(),
            observed_revision: Some("snapshot-supervisor".to_owned()),
        }
    }

    fn emit_evidence(name: &str, value: &impl Serialize) {
        if std::env::var_os("AGENTMAGE_STORY_23_6_EVIDENCE").is_some() {
            println!(
                "{name}={}",
                serde_json::to_string(value).expect("evidence serializes")
            );
        }
    }
}
