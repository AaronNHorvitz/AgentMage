//! Interface-independent ephemeral read-only runtime coordinator.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{
    ActionId, AgentProposal, AgentStateKind, ApprovalId, CONTRACT_SCHEMA_VERSION,
    CancellationSignal, ClosedModelProposal, ContextPacketId, CorrelationId, EvidenceReference,
    ExactModelProfile, GrantId, GrantOperation, LocalModelRuntime, ModelCancellationProbe,
    ModelContextPacket, ModelFamilyCodec, ModelProposalKind, ModelRunId, ModelRunRequest,
    ModelRunResult, ModelRunTerminalState, OperationOutcome, PostconditionId, ReceiptId,
    RuntimeApprovalChallenge, RuntimeApprovalDisposition, RuntimeApprovalResponse, RuntimeEvent,
    RuntimeEventId, RuntimeEventKind, RuntimeEventRetention, RuntimeEventRetentionKind,
    RuntimeOperationId, RuntimeOutcome, RuntimeOutput, RuntimePermissionDisposition,
    RuntimeRunRequest, RuntimeSessionMode, RuntimeToolReference, RuntimeTurnId, StateChange,
    ToolCall, ToolDefinition, ToolResult, VerifierCandidate, VerifierId, to_canonical_json,
};
use sha2::{Digest, Sha256};

use crate::agent_proposal::{ExpectedProposalContext, ProposalAdmissionRegistry};
use crate::agent_state::AgentStateController;
use crate::agent_verifier::{VerifierContext, VerifierRegistry};
use crate::model_runtime::{LocalModelController, ModelRuntimeGateError};
use crate::runtime_coordinator::{
    RuntimeCoordinatorError, runtime_tool_catalog_sha256, seal_runtime_approval_challenge,
    seal_runtime_outcome, verify_runtime_approval_response, verify_runtime_run_request,
};
use crate::runtime_event::{
    RuntimeEventDelivery, RuntimeEventError, RuntimeEventPublisher, RuntimeEventSubscription,
    runtime_event_persistence, seal_runtime_event,
};
use crate::tooling::{
    PreGrantDispatchDisposition, ProposalOrigin, ToolAttemptGuard, ToolDispatcher, ToolRegistry,
};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MIN_RUNTIME_EVENTS: u32 = 6;
const SHA256_BYTES: usize = 64;

/// Closed dependency failure observed by the coordinator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimePortFailure {
    /// The port rejected malformed or mismatched input.
    Invalid,
    /// A required local dependency is unavailable.
    Unavailable,
    /// Cancellation won before a terminal result was available.
    Cancelled,
    /// The bounded operation exceeded its deadline.
    TimedOut,
    /// A declared resource ceiling was exhausted.
    ResourceExhausted,
    /// The port cannot establish whether an operation completed.
    Uncertain,
}

impl RuntimePortFailure {
    /// Returns one stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Invalid => "runtime.port.invalid",
            Self::Unavailable => "runtime.port.unavailable",
            Self::Cancelled => "runtime.port.cancelled",
            Self::TimedOut => "runtime.port.timed_out",
            Self::ResourceExhausted => "runtime.port.resource_exhausted",
            Self::Uncertain => "runtime.port.uncertain",
        }
    }
}

/// Trusted monotonic wall-clock observation used only for deadlines and event timestamps.
pub trait RuntimeClock {
    /// Returns a positive Unix epoch timestamp in milliseconds.
    fn now_epoch_ms(&mut self) -> Result<u64, RuntimePortFailure>;
}

/// Context builder for one exact request and its currently verified observations.
pub trait RuntimeContextPort {
    /// Builds a complete bounded model context without model, tool, grant, or effect authority.
    fn build_context(
        &mut self,
        request: &RuntimeRunRequest,
        context_packet_id: ContextPacketId,
        turn: u32,
        tool_results: &[ToolResult],
        evidence: &[EvidenceReference],
    ) -> Result<ModelContextPacket, RuntimePortFailure>;
}

/// Candidate-neutral model controller used by the reusable coordinator.
pub trait RuntimeModelPort {
    /// Returns the exact profile already admitted by the model controller.
    fn exact_profile(&self) -> &ExactModelProfile;

    /// Executes one bounded request and returns only a validated inert result.
    fn run_model(
        &mut self,
        request: &ModelRunRequest,
        context: &ModelContextPacket,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<ModelRunResult, RuntimePortFailure>;
}

impl<R, C> RuntimeModelPort for LocalModelController<R, C>
where
    R: LocalModelRuntime,
    C: ModelFamilyCodec,
{
    fn exact_profile(&self) -> &ExactModelProfile {
        self.exact_profile()
    }

    fn run_model(
        &mut self,
        request: &ModelRunRequest,
        context: &ModelContextPacket,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<ModelRunResult, RuntimePortFailure> {
        self.health().map_err(map_model_error)?;
        self.count_tokens(context).map_err(map_model_error)?;
        self.stream(request, context, cancellation)
            .map_err(map_model_error)
    }
}

/// Deterministic permission result produced without granting authority to the model or client.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimePermissionEvaluation {
    /// An exact pre-existing grant may be consumed by the trusted tool boundary.
    Allow {
        /// Exact approval identity bound to the grant.
        approval_id: ApprovalId,
        /// Digest of the complete protected preview.
        preview_sha256: String,
        /// Exclusive decision expiration.
        expires_at_epoch_ms: u64,
        /// Exact separately issued grant identity.
        grant_id: GrantId,
        /// Digest of the policy decision.
        decision_sha256: String,
        /// Digest of the authority transaction consumed before worker launch.
        authority_sha256: String,
    },
    /// A protected user decision is required before any worker launch.
    Ask {
        /// Stable approval identity.
        approval_id: ApprovalId,
        /// Exact operation-grant identity proposed by the trusted boundary.
        grant_id: GrantId,
        /// Digest of the complete protected preview.
        preview_sha256: String,
        /// Exclusive decision expiration.
        expires_at_epoch_ms: u64,
    },
    /// Policy or the user declined the operation before effect.
    Deny {
        /// Exact approval or policy-decision identity.
        approval_id: ApprovalId,
        /// Exact proposed grant identity retained even though no grant is issued.
        grant_id: GrantId,
        /// Digest of the complete protected preview.
        preview_sha256: String,
        /// Exclusive decision expiration.
        expires_at_epoch_ms: u64,
        /// Digest of the policy or user decision.
        decision_sha256: String,
        /// Stable content-free denial reason.
        reason_code: String,
    },
}

/// One terminal tool execution returned after exact authority consumption.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeToolExecution {
    /// Canonical operation receipt identity.
    pub receipt_id: ReceiptId,
    /// Digest of the complete canonical receipt.
    pub receipt_sha256: String,
    /// Exact terminal tool result.
    pub result: ToolResult,
}

/// Trusted grant, policy, and worker boundary used only after registry validation.
pub trait RuntimeToolBoundary {
    /// Evaluates one exact validated call without launching a worker.
    fn evaluate(
        &mut self,
        request: &RuntimeRunRequest,
        operation_id: &RuntimeOperationId,
        definition: &ToolDefinition,
        call: &ToolCall,
        now_epoch_ms: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure>;

    /// Resolves one protected user response against current grant and policy state.
    fn resolve(
        &mut self,
        request: &RuntimeRunRequest,
        challenge: &RuntimeApprovalChallenge,
        response: &RuntimeApprovalResponse,
        definition: &ToolDefinition,
        call: &ToolCall,
        now_epoch_ms: u64,
    ) -> Result<RuntimePermissionEvaluation, RuntimePortFailure>;

    /// Consumes exact allowed authority and executes one admitted call once.
    fn execute(
        &mut self,
        request: &RuntimeRunRequest,
        evaluation: &RuntimePermissionEvaluation,
        definition: &ToolDefinition,
        call: &ToolCall,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<RuntimeToolExecution, RuntimePortFailure>;
}

/// Exact deterministic completion input exposed to a verifier implementation.
pub struct RuntimeVerificationInput<'a> {
    /// Owning runtime request.
    pub request: &'a RuntimeRunRequest,
    /// Exact inert model proposal under review.
    pub proposal: &'a ClosedModelProposal,
    /// Current state revision to which a proof must bind.
    pub state_revision: u64,
    /// Ordered deterministic postconditions derived from acceptance criteria.
    pub postconditions: &'a [PostconditionId],
    /// Current grounded evidence available to deterministic checks.
    pub evidence: &'a [EvidenceReference],
}

/// Deterministic verifier boundary; model prose and client claims remain ineligible.
pub trait RuntimeVerifierPort {
    /// Returns the exact registered verifier identity.
    fn verifier_id(&self) -> &VerifierId;

    /// Produces one closed candidate for validation by the existing verifier registry.
    fn verify(
        &mut self,
        input: RuntimeVerificationInput<'_>,
    ) -> Result<VerifierCandidate, RuntimePortFailure>;
}

/// Boundary reached by an interface-independent coordinator invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeCoordinatorStep {
    /// The run is paused before effect pending one exact user decision.
    AwaitingApproval {
        /// Immutable challenge every client must display and bind into its response.
        challenge: RuntimeApprovalChallenge,
    },
    /// The run reached one canonical terminal outcome.
    Complete {
        /// Canonical verifier-bound or truthful non-success outcome.
        outcome: RuntimeOutcome,
    },
}

/// Stable construction or invariant failure from the reusable coordinator itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeLoopError {
    /// The submitted runtime contract failed closed verification.
    Contract(RuntimeCoordinatorError),
    /// Durable state or a resume cursor was requested without durable ports.
    UnsupportedMode,
    /// The model port is not bound to the exact request profile.
    ModelBinding,
    /// The frozen request catalog differs from the exact registry.
    ToolCatalogBinding,
    /// The event stream rejected an event or subscriber operation.
    Event(RuntimeEventError),
    /// The deterministic agent-state graph rejected an internal edge.
    State,
    /// A required dependency failed before a truthful terminal result could be formed.
    Dependency(RuntimePortFailure),
    /// A proposal, permission, tool result, or verifier record violated its closed contract.
    InvalidBoundaryResult,
}

impl From<RuntimeCoordinatorError> for RuntimeLoopError {
    fn from(value: RuntimeCoordinatorError) -> Self {
        Self::Contract(value)
    }
}

impl From<RuntimeEventError> for RuntimeLoopError {
    fn from(value: RuntimeEventError) -> Self {
        Self::Event(value)
    }
}

struct PendingApproval {
    challenge: RuntimeApprovalChallenge,
    definition: ToolDefinition,
    call: ToolCall,
    turn_id: RuntimeTurnId,
    operation_id: RuntimeOperationId,
}

/// One reusable, interface-neutral ephemeral runtime coordinator.
pub struct ReusableRuntimeCoordinator<M, X, T, V, C>
where
    M: RuntimeModelPort,
    X: RuntimeContextPort,
    T: RuntimeToolBoundary,
    V: RuntimeVerifierPort,
    C: RuntimeClock,
{
    request: RuntimeRunRequest,
    model: M,
    context: X,
    registry: ToolRegistry,
    tool_boundary: T,
    verifier: V,
    clock: C,
    publisher: RuntimeEventPublisher,
    state: AgentStateController,
    attempt_guard: ToolAttemptGuard,
    events: Vec<RuntimeEvent>,
    tool_results: Vec<ToolResult>,
    evidence: Vec<EvidenceReference>,
    receipt_ids: Vec<ReceiptId>,
    pending: Option<PendingApproval>,
    outcome: Option<RuntimeOutcome>,
    active_turn: Option<RuntimeTurnId>,
    correlation_id: agentmage_kernel_contracts::CorrelationId,
    started_at_epoch_ms: Option<u64>,
    turn_count: u32,
    model_call_count: u32,
    tool_call_count: u32,
    context_refresh_count: u32,
    no_progress_turns: u32,
}

impl<M, X, T, V, C> ReusableRuntimeCoordinator<M, X, T, V, C>
where
    M: RuntimeModelPort,
    X: RuntimeContextPort,
    T: RuntimeToolBoundary,
    V: RuntimeVerifierPort,
    C: RuntimeClock,
{
    /// Composes one exact request with already constructed kernel-owned ports.
    pub fn new(
        request: RuntimeRunRequest,
        model: M,
        context: X,
        registry: ToolRegistry,
        tool_boundary: T,
        verifier: V,
        clock: C,
    ) -> Result<Self, RuntimeLoopError> {
        verify_runtime_run_request(&request)?;
        if request.mode == RuntimeSessionMode::DurableReadOnly || request.event_cursor.is_some() {
            return Err(RuntimeLoopError::UnsupportedMode);
        }
        if model.exact_profile() != &request.model_profile {
            return Err(RuntimeLoopError::ModelBinding);
        }
        let registry_tools = runtime_tool_references(&registry)?;
        if registry_tools != request.visible_tools
            || runtime_tool_catalog_sha256(&request.tool_catalog_id, &registry_tools)?
                != request.tool_catalog_sha256
            || !catalog_allowed_for_mode(request.mode, &registry)
        {
            return Err(RuntimeLoopError::ToolCatalogBinding);
        }
        if request.limits.max_events < MIN_RUNTIME_EVENTS {
            return Err(RuntimeLoopError::Contract(
                RuntimeCoordinatorError::InvalidLimits,
            ));
        }
        let attempt_guard = ToolAttemptGuard::new(
            request.limits.max_repeated_tool_calls,
            request.limits.max_tool_call_depth,
        )
        .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
        let correlation_id =
            CorrelationId::from_raw(derived_id("correlation", request.run_id.as_str(), 0));
        let evidence = request.work_packet.authoritative_evidence.clone();
        Ok(Self {
            request,
            model,
            context,
            registry,
            tool_boundary,
            verifier,
            clock,
            publisher: RuntimeEventPublisher::new(),
            state: AgentStateController::new(),
            attempt_guard,
            events: Vec::new(),
            tool_results: Vec::new(),
            evidence,
            receipt_ids: Vec::new(),
            pending: None,
            outcome: None,
            active_turn: None,
            correlation_id,
            started_at_epoch_ms: None,
            turn_count: 0,
            model_call_count: 0,
            tool_call_count: 0,
            context_refresh_count: 0,
            no_progress_turns: 0,
        })
    }

    /// Registers one bounded ordered event subscriber before or during execution.
    pub fn subscribe_events(
        &self,
        capacity: usize,
    ) -> Result<RuntimeEventSubscription, RuntimeLoopError> {
        self.publisher.subscribe(capacity).map_err(Into::into)
    }

    /// Returns the exact verified in-memory event history for this ephemeral run.
    #[must_use]
    pub fn events(&self) -> &[RuntimeEvent] {
        &self.events
    }

    /// Returns the canonical outcome after terminal completion.
    #[must_use]
    pub const fn outcome(&self) -> Option<&RuntimeOutcome> {
        self.outcome.as_ref()
    }

    /// Runs until a protected approval or canonical terminal outcome is reached.
    pub fn run_until_boundary(
        &mut self,
        response: Option<&RuntimeApprovalResponse>,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<RuntimeCoordinatorStep, RuntimeLoopError> {
        if let Some(outcome) = &self.outcome {
            if response.is_some() {
                return Err(RuntimeLoopError::Contract(
                    RuntimeCoordinatorError::ApprovalDenied,
                ));
            }
            return Ok(RuntimeCoordinatorStep::Complete {
                outcome: outcome.clone(),
            });
        }
        if self.events.is_empty() {
            if response.is_some() {
                return Err(RuntimeLoopError::Contract(
                    RuntimeCoordinatorError::ApprovalDenied,
                ));
            }
            self.start()?;
        }
        if self.pending.is_some() {
            let Some(response) = response else {
                return Ok(RuntimeCoordinatorStep::AwaitingApproval {
                    challenge: self
                        .pending
                        .as_ref()
                        .expect("pending checked")
                        .challenge
                        .clone(),
                });
            };
            if let Some(signal) = observe_cancellation(cancellation)? {
                self.cancel(signal)?;
                return Ok(RuntimeCoordinatorStep::Complete {
                    outcome: self.outcome.clone().expect("cancellation is terminal"),
                });
            }
            self.resume_pending(response, cancellation)?;
        } else if response.is_some() {
            return Err(RuntimeLoopError::Contract(
                RuntimeCoordinatorError::ApprovalDenied,
            ));
        }

        loop {
            if let Some(outcome) = &self.outcome {
                return Ok(RuntimeCoordinatorStep::Complete {
                    outcome: outcome.clone(),
                });
            }
            if let Some(challenge) = self.pending.as_ref().map(|value| value.challenge.clone()) {
                return Ok(RuntimeCoordinatorStep::AwaitingApproval { challenge });
            }
            if let Some(signal) = observe_cancellation(cancellation)? {
                self.cancel(signal)?;
                continue;
            }
            if self.turn_count >= self.request.limits.max_turns
                || self.model_call_count >= self.request.limits.max_model_calls
                || self.context_refresh_count >= self.request.limits.max_context_refreshes
                || self.remaining_events() < MIN_RUNTIME_EVENTS
                || self.elapsed_limit_reached()?
            {
                self.transition_terminal(AgentStateKind::Exhausted)?;
                self.finish_terminal(
                    AgentStateKind::Exhausted,
                    vec!["runtime.budget.exhausted".to_owned()],
                    None,
                )?;
                continue;
            }
            self.run_turn(cancellation)?;
        }
    }

    fn start(&mut self) -> Result<(), RuntimeLoopError> {
        let now = self
            .clock
            .now_epoch_ms()
            .map_err(RuntimeLoopError::Dependency)?;
        if now == 0 {
            return Err(RuntimeLoopError::Dependency(RuntimePortFailure::Invalid));
        }
        self.started_at_epoch_ms = Some(now);
        self.emit_at(
            now,
            RuntimeEventKind::RunStarted {
                request_sha256: self.request.request_sha256.clone(),
            },
            None,
            None,
        )?;
        Ok(())
    }

    fn run_turn(
        &mut self,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<(), RuntimeLoopError> {
        self.turn_count += 1;
        self.context_refresh_count += 1;
        let turn_id = RuntimeTurnId::from_raw(derived_id(
            "turn",
            self.request.run_id.as_str(),
            u64::from(self.turn_count),
        ));
        self.active_turn = Some(turn_id.clone());
        self.emit(RuntimeEventKind::TurnStarted, Some(&turn_id), None)?;

        let context_packet_id = ContextPacketId::from_raw(derived_id(
            "context",
            self.request.run_id.as_str(),
            u64::from(self.context_refresh_count),
        ));
        let context = match self.context.build_context(
            &self.request,
            context_packet_id,
            self.turn_count,
            &self.tool_results,
            &self.evidence,
        ) {
            Ok(context) if valid_context_packet(&context, &self.request) => context,
            _ => {
                self.transition_terminal(AgentStateKind::Failed)?;
                self.close_turn(&turn_id, sha256(b"runtime.context.failed"))?;
                return self.finish_terminal(
                    AgentStateKind::Failed,
                    vec!["runtime.context.failed".to_owned()],
                    None,
                );
            }
        };

        self.model_call_count += 1;
        let model_run_id = ModelRunId::from_raw(derived_id(
            "model-run",
            self.request.run_id.as_str(),
            u64::from(self.model_call_count),
        ));
        let model_request = ModelRunRequest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            model_run_id: model_run_id.clone(),
            correlation_id: self.correlation_id.clone(),
            context_packet_id: context.context_packet_id.clone(),
            profile_id: self.request.model_profile.profile_id.clone(),
            manifest_sha256: self.request.model_profile.manifest_sha256.clone(),
            adapter_id: self.request.model_profile.runtime.adapter_id.clone(),
            decoding_profile_id: self.request.model_profile.decoding.profile_id.clone(),
            max_output_tokens: self.request.model_profile.decoding.max_output_tokens,
            timeout_ms: self.request.limits.max_elapsed_ms,
        };
        let request_sha256 = contract_sha256(&model_request)?;
        self.emit(
            RuntimeEventKind::ModelRequested {
                model_run_id: model_run_id.clone(),
                request_sha256,
            },
            Some(&turn_id),
            None,
        )?;
        let result = match self.model.run_model(&model_request, &context, cancellation) {
            Ok(result) => result,
            Err(error) => {
                self.emit(
                    RuntimeEventKind::ModelFailed {
                        model_run_id,
                        failure_code: error.code().to_owned(),
                    },
                    Some(&turn_id),
                    None,
                )?;
                return self.finish_model_failure(&turn_id, error);
            }
        };
        if !valid_model_result(&result, &model_request) {
            self.emit(
                RuntimeEventKind::ModelFailed {
                    model_run_id,
                    failure_code: "runtime.model.result_invalid".to_owned(),
                },
                Some(&turn_id),
                None,
            )?;
            return self.finish_model_failure(&turn_id, RuntimePortFailure::Invalid);
        }
        let result_sha256 = contract_sha256(&result)?;
        match result.terminal_state {
            ModelRunTerminalState::Proposed => {
                self.emit(
                    RuntimeEventKind::ModelCompleted {
                        model_run_id,
                        result_sha256,
                    },
                    Some(&turn_id),
                    None,
                )?;
                let Some(proposal) = result.proposal else {
                    return self.finish_invalid_proposal(&turn_id);
                };
                self.process_proposal(turn_id, context, proposal, cancellation)
            }
            ModelRunTerminalState::Cancelled => {
                self.emit(
                    RuntimeEventKind::ModelFailed {
                        model_run_id,
                        failure_code: "runtime.model.cancelled".to_owned(),
                    },
                    Some(&turn_id),
                    None,
                )?;
                self.finish_model_failure(&turn_id, RuntimePortFailure::Cancelled)
            }
            ModelRunTerminalState::TimedOut => {
                self.emit(
                    RuntimeEventKind::ModelFailed {
                        model_run_id,
                        failure_code: "runtime.model.timed_out".to_owned(),
                    },
                    Some(&turn_id),
                    None,
                )?;
                self.finish_model_failure(&turn_id, RuntimePortFailure::TimedOut)
            }
            ModelRunTerminalState::ResourceExhausted => {
                self.emit(
                    RuntimeEventKind::ModelFailed {
                        model_run_id,
                        failure_code: "runtime.model.resource_exhausted".to_owned(),
                    },
                    Some(&turn_id),
                    None,
                )?;
                self.finish_model_failure(&turn_id, RuntimePortFailure::ResourceExhausted)
            }
            ModelRunTerminalState::AdvisoryText
            | ModelRunTerminalState::Failed
            | ModelRunTerminalState::Rejected => {
                self.emit(
                    RuntimeEventKind::ModelFailed {
                        model_run_id,
                        failure_code: "runtime.model.no_typed_proposal".to_owned(),
                    },
                    Some(&turn_id),
                    None,
                )?;
                self.finish_model_failure(&turn_id, RuntimePortFailure::Invalid)
            }
        }
    }

    fn process_proposal(
        &mut self,
        turn_id: RuntimeTurnId,
        context: ModelContextPacket,
        proposal: ClosedModelProposal,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<(), RuntimeLoopError> {
        let expected = ExpectedProposalContext {
            session_id: self.request.session_id.clone(),
            task_id: self.request.task.task_id.clone(),
            turn: u64::from(self.turn_count),
            model_run_id: proposal.model_run_id.clone(),
            context_packet_id: context.context_packet_id,
            repository_snapshot_id: self.request.repository_snapshot_id.clone(),
            tool_catalog_id: self.request.tool_catalog_id.clone(),
            policy_id: self.request.policy_id.clone(),
            correlation_id: self.correlation_id.clone(),
        };
        let candidate = AgentProposal {
            schema_version: CONTRACT_SCHEMA_VERSION,
            proposal_id: proposal.proposal_id.clone(),
            session_id: self.request.session_id.clone(),
            task_id: self.request.task.task_id.clone(),
            turn: u64::from(self.turn_count),
            model_run_id: proposal.model_run_id.clone(),
            context_packet_id: proposal.context_packet_id.clone(),
            repository_snapshot_id: self.request.repository_snapshot_id.clone(),
            tool_catalog_id: self.request.tool_catalog_id.clone(),
            policy_id: self.request.policy_id.clone(),
            correlation_id: proposal.correlation_id.clone(),
            proposal_sha256: proposal.proposal_sha256.clone(),
        };
        ProposalAdmissionRegistry::new(expected)
            .and_then(|mut registry| registry.admit(&candidate))
            .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
        self.state
            .transition(AgentStateKind::Proposal)
            .map_err(|_| RuntimeLoopError::State)?;
        self.state
            .transition(AgentStateKind::Validation)
            .map_err(|_| RuntimeLoopError::State)?;

        match proposal.kind {
            ModelProposalKind::Text | ModelProposalKind::CompletionCandidate => {
                self.verify_completion(turn_id, proposal)
            }
            ModelProposalKind::ToolCall => self.propose_tool(turn_id, proposal, cancellation),
            ModelProposalKind::EvidenceRequest
            | ModelProposalKind::UserQuestion
            | ModelProposalKind::Blocked => {
                self.state
                    .transition(AgentStateKind::Blocked)
                    .map_err(|_| RuntimeLoopError::State)?;
                self.close_turn(&turn_id, proposal.proposal_sha256)?;
                self.finish_terminal(
                    AgentStateKind::Blocked,
                    vec!["runtime.proposal.blocked".to_owned()],
                    proposal
                        .payload
                        .map(|payload| RuntimeOutput::Inline { payload }),
                )
            }
        }
    }

    fn verify_completion(
        &mut self,
        turn_id: RuntimeTurnId,
        proposal: ClosedModelProposal,
    ) -> Result<(), RuntimeLoopError> {
        let Some(payload) = proposal.payload.clone() else {
            return self.finish_invalid_proposal(&turn_id);
        };
        if !valid_output_payload(&payload, self.request.limits.max_output_bytes) {
            return self.finish_invalid_proposal(&turn_id);
        }
        self.state
            .transition(AgentStateKind::Approval)
            .map_err(|_| RuntimeLoopError::State)?;
        self.state
            .transition(AgentStateKind::Execution)
            .map_err(|_| RuntimeLoopError::State)?;
        self.state
            .transition(AgentStateKind::Verification)
            .map_err(|_| RuntimeLoopError::State)?;
        let postconditions = runtime_postconditions(&self.request);
        let verifier_context = VerifierContext {
            verifier_id: self.verifier.verifier_id().clone(),
            task_id: self.request.task.task_id.clone(),
            proposal_id: proposal.proposal_id.clone(),
            repository_snapshot_id: self.request.repository_snapshot_id.clone(),
            state_revision: self.state.revision(),
            postconditions: postconditions.clone(),
        };
        let registry = VerifierRegistry::new(verifier_context)
            .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
        let candidate = self
            .verifier
            .verify(RuntimeVerificationInput {
                request: &self.request,
                proposal: &proposal,
                state_revision: self.state.revision(),
                postconditions: &postconditions,
                evidence: &self.evidence,
            })
            .map_err(RuntimeLoopError::Dependency)?;
        let completion = match registry.verify(&candidate) {
            Ok(completion) => completion,
            Err(_) => {
                self.state
                    .transition(AgentStateKind::Failed)
                    .map_err(|_| RuntimeLoopError::State)?;
                self.close_turn(&turn_id, proposal.proposal_sha256)?;
                return self.finish_terminal(
                    AgentStateKind::Failed,
                    vec!["runtime.verification.failed".to_owned()],
                    Some(RuntimeOutput::Inline { payload }),
                );
            }
        };
        self.state
            .complete(&completion)
            .map_err(|_| RuntimeLoopError::State)?;
        for item in candidate
            .postconditions
            .iter()
            .flat_map(|result| result.evidence.iter())
        {
            if !self
                .evidence
                .iter()
                .any(|existing| existing.evidence_id == item.evidence_id)
            {
                self.evidence.push(item.clone());
            }
        }
        self.evidence
            .sort_by(|left, right| left.evidence_id.as_str().cmp(right.evidence_id.as_str()));
        let terminal = self.state.current();
        self.close_turn(&turn_id, proposal.proposal_sha256)?;
        self.finish_terminal(
            terminal,
            Vec::new(),
            Some(RuntimeOutput::Inline { payload }),
        )
    }

    fn propose_tool(
        &mut self,
        turn_id: RuntimeTurnId,
        proposal: ClosedModelProposal,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<(), RuntimeLoopError> {
        if self.tool_call_count >= self.request.limits.max_tool_calls || self.remaining_events() < 7
        {
            self.state
                .transition(AgentStateKind::Exhausted)
                .map_err(|_| RuntimeLoopError::State)?;
            self.close_turn(&turn_id, proposal.proposal_sha256)?;
            return self.finish_terminal(
                AgentStateKind::Exhausted,
                vec!["runtime.tool_budget.exhausted".to_owned()],
                None,
            );
        }
        let Some(candidate) = proposal.tool_call else {
            return self.finish_invalid_proposal(&turn_id);
        };
        self.tool_call_count += 1;
        let call = ToolCall {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: candidate.tool_call_id,
            correlation_id: self.correlation_id.clone(),
            action_id: ActionId::from_raw(derived_id(
                "action",
                self.request.run_id.as_str(),
                u64::from(self.tool_call_count),
            )),
            tool_id: candidate.tool_id,
            tool_version: candidate.tool_version,
            arguments: candidate.arguments,
        };
        let definition = self
            .registry
            .validate_arguments(&call)
            .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?
            .clone();
        let receipt = ToolDispatcher::new(&self.registry).dispatch(ProposalOrigin::Model, &call);
        if receipt.disposition != PreGrantDispatchDisposition::GrantRequired {
            return Err(RuntimeLoopError::InvalidBoundaryResult);
        }
        self.attempt_guard
            .record_attempt(&call, 0)
            .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
        let operation_id = RuntimeOperationId::from_raw(derived_id(
            "operation",
            self.request.run_id.as_str(),
            u64::from(self.tool_call_count),
        ));
        self.emit(
            RuntimeEventKind::ToolRequested {
                tool_call_id: call.tool_call_id.clone(),
                arguments_sha256: call.arguments.sha256.clone(),
            },
            Some(&turn_id),
            Some(&operation_id),
        )?;
        self.state
            .transition(AgentStateKind::Approval)
            .map_err(|_| RuntimeLoopError::State)?;
        let now = self
            .clock
            .now_epoch_ms()
            .map_err(RuntimeLoopError::Dependency)?;
        let evaluation = self
            .tool_boundary
            .evaluate(&self.request, &operation_id, &definition, &call, now)
            .map_err(RuntimeLoopError::Dependency)?;
        self.process_permission(
            evaluation,
            definition,
            call,
            turn_id,
            operation_id,
            cancellation,
            false,
            now,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn process_permission(
        &mut self,
        evaluation: RuntimePermissionEvaluation,
        definition: ToolDefinition,
        call: ToolCall,
        turn_id: RuntimeTurnId,
        operation_id: RuntimeOperationId,
        cancellation: Option<&dyn ModelCancellationProbe>,
        request_already_emitted: bool,
        now_epoch_ms: u64,
    ) -> Result<(), RuntimeLoopError> {
        if !valid_permission_evaluation(&evaluation, now_epoch_ms) {
            return Err(RuntimeLoopError::InvalidBoundaryResult);
        }
        let challenge = permission_challenge(
            &self.request,
            &turn_id,
            &operation_id,
            &definition,
            &call,
            &evaluation,
        )?;
        if !request_already_emitted {
            self.emit(
                RuntimeEventKind::PermissionRequested {
                    approval_id: challenge.approval_id.clone(),
                    operation: challenge.operation,
                    preview_sha256: challenge.preview_sha256.clone(),
                    expires_at_epoch_ms: challenge.expires_at_epoch_ms,
                },
                Some(&turn_id),
                Some(&operation_id),
            )?;
        }
        match &evaluation {
            RuntimePermissionEvaluation::Ask { .. } => {
                if request_already_emitted {
                    return Err(RuntimeLoopError::InvalidBoundaryResult);
                }
                self.pending = Some(PendingApproval {
                    challenge,
                    definition,
                    call,
                    turn_id,
                    operation_id,
                });
                Ok(())
            }
            RuntimePermissionEvaluation::Deny {
                approval_id,
                decision_sha256,
                reason_code,
                ..
            } => {
                self.emit(
                    RuntimeEventKind::PermissionDecided {
                        approval_id: approval_id.clone(),
                        disposition: RuntimePermissionDisposition::Deny,
                        grant_id: None,
                        decision_sha256: decision_sha256.clone(),
                    },
                    Some(&turn_id),
                    Some(&operation_id),
                )?;
                self.state
                    .transition(AgentStateKind::Declined)
                    .map_err(|_| RuntimeLoopError::State)?;
                self.close_turn(&turn_id, sha256(reason_code.as_bytes()))?;
                self.finish_terminal(AgentStateKind::Declined, vec![reason_code.clone()], None)
            }
            RuntimePermissionEvaluation::Allow {
                approval_id,
                grant_id,
                decision_sha256,
                authority_sha256,
                ..
            } => {
                self.emit(
                    RuntimeEventKind::PermissionDecided {
                        approval_id: approval_id.clone(),
                        disposition: RuntimePermissionDisposition::Allow,
                        grant_id: Some(grant_id.clone()),
                        decision_sha256: decision_sha256.clone(),
                    },
                    Some(&turn_id),
                    Some(&operation_id),
                )?;
                self.state
                    .transition(AgentStateKind::Execution)
                    .map_err(|_| RuntimeLoopError::State)?;
                self.emit(
                    RuntimeEventKind::ToolStarted {
                        tool_call_id: call.tool_call_id.clone(),
                        authority_sha256: authority_sha256.clone(),
                    },
                    Some(&turn_id),
                    Some(&operation_id),
                )?;
                let execution = self
                    .tool_boundary
                    .execute(&self.request, &evaluation, &definition, &call, cancellation)
                    .map_err(RuntimeLoopError::Dependency)?;
                self.complete_tool(execution, definition, call, turn_id, operation_id)
            }
        }
    }

    fn resume_pending(
        &mut self,
        response: &RuntimeApprovalResponse,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<(), RuntimeLoopError> {
        let pending = self.pending.take().expect("pending checked by caller");
        let now = self
            .clock
            .now_epoch_ms()
            .map_err(RuntimeLoopError::Dependency)?;
        verify_runtime_approval_response(&pending.challenge, response, now)?;
        let evaluation = self
            .tool_boundary
            .resolve(
                &self.request,
                &pending.challenge,
                response,
                &pending.definition,
                &pending.call,
                now,
            )
            .map_err(RuntimeLoopError::Dependency)?;
        let resolved_challenge = permission_challenge(
            &self.request,
            &pending.turn_id,
            &pending.operation_id,
            &pending.definition,
            &pending.call,
            &evaluation,
        )?;
        if resolved_challenge != pending.challenge {
            return Err(RuntimeLoopError::InvalidBoundaryResult);
        }
        match (&response.disposition, &evaluation) {
            (
                RuntimeApprovalDisposition::Allow,
                RuntimePermissionEvaluation::Allow {
                    approval_id,
                    grant_id,
                    ..
                },
            ) if approval_id == &pending.challenge.approval_id
                && Some(grant_id) == response.grant_id.as_ref() => {}
            (
                RuntimeApprovalDisposition::Deny,
                RuntimePermissionEvaluation::Deny { approval_id, .. },
            ) if approval_id == &pending.challenge.approval_id => {}
            _ => return Err(RuntimeLoopError::InvalidBoundaryResult),
        }
        self.process_permission(
            evaluation,
            pending.definition,
            pending.call,
            pending.turn_id,
            pending.operation_id,
            cancellation,
            true,
            now,
        )
    }

    fn complete_tool(
        &mut self,
        execution: RuntimeToolExecution,
        definition: ToolDefinition,
        call: ToolCall,
        turn_id: RuntimeTurnId,
        operation_id: RuntimeOperationId,
    ) -> Result<(), RuntimeLoopError> {
        if !valid_tool_execution(&execution, &definition, &call, &self.request) {
            return Err(RuntimeLoopError::InvalidBoundaryResult);
        }
        let prior_evidence = self.evidence.len();
        match execution.result.outcome {
            OperationOutcome::Succeeded => {
                self.emit(
                    RuntimeEventKind::ToolCompleted {
                        tool_call_id: call.tool_call_id,
                        receipt_id: execution.receipt_id.clone(),
                        result_sha256: contract_sha256(&execution.result)?,
                    },
                    Some(&turn_id),
                    Some(&operation_id),
                )?;
                self.receipt_ids.push(execution.receipt_id);
                for item in &execution.result.evidence {
                    if !self
                        .evidence
                        .iter()
                        .any(|existing| existing.evidence_id == item.evidence_id)
                    {
                        self.evidence.push(item.clone());
                    }
                }
                self.evidence.sort_by(|left, right| {
                    left.evidence_id.as_str().cmp(right.evidence_id.as_str())
                });
                self.tool_results.push(execution.result);
                self.state
                    .transition(AgentStateKind::Verification)
                    .map_err(|_| RuntimeLoopError::State)?;
                self.state
                    .transition(AgentStateKind::Checkpoint)
                    .map_err(|_| RuntimeLoopError::State)?;
                if self.evidence.len() == prior_evidence {
                    self.no_progress_turns += 1;
                } else {
                    self.no_progress_turns = 0;
                }
                self.close_turn(&turn_id, execution.receipt_sha256)?;
                if self.no_progress_turns >= self.request.limits.max_no_progress_turns {
                    self.state
                        .transition(AgentStateKind::Stalled)
                        .map_err(|_| RuntimeLoopError::State)?;
                    self.finish_terminal(
                        AgentStateKind::Stalled,
                        vec!["runtime.no_progress.exhausted".to_owned()],
                        None,
                    )
                } else {
                    self.state
                        .transition(AgentStateKind::Observation)
                        .map_err(|_| RuntimeLoopError::State)?;
                    Ok(())
                }
            }
            OperationOutcome::Denied => self.finish_tool_non_success(
                execution,
                call,
                turn_id,
                operation_id,
                AgentStateKind::Failed,
                "runtime.tool.denied_after_launch",
            ),
            OperationOutcome::Cancelled => self.finish_tool_non_success(
                execution,
                call,
                turn_id,
                operation_id,
                AgentStateKind::Cancelled,
                "runtime.tool.cancelled",
            ),
            OperationOutcome::TimedOut => self.finish_tool_non_success(
                execution,
                call,
                turn_id,
                operation_id,
                AgentStateKind::Exhausted,
                "runtime.tool.timed_out",
            ),
            OperationOutcome::Failed => self.finish_tool_non_success(
                execution,
                call,
                turn_id,
                operation_id,
                AgentStateKind::Failed,
                "runtime.tool.failed",
            ),
            OperationOutcome::Uncertain => self.finish_tool_non_success(
                execution,
                call,
                turn_id,
                operation_id,
                AgentStateKind::Uncertain,
                "runtime.tool.uncertain",
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn finish_tool_non_success(
        &mut self,
        execution: RuntimeToolExecution,
        call: ToolCall,
        turn_id: RuntimeTurnId,
        operation_id: RuntimeOperationId,
        terminal: AgentStateKind,
        code: &str,
    ) -> Result<(), RuntimeLoopError> {
        self.emit(
            RuntimeEventKind::ToolFailed {
                tool_call_id: call.tool_call_id,
                receipt_id: Some(execution.receipt_id.clone()),
                failure_code: code.to_owned(),
            },
            Some(&turn_id),
            Some(&operation_id),
        )?;
        self.receipt_ids.push(execution.receipt_id);
        self.transition_terminal(terminal)?;
        self.close_turn(&turn_id, execution.receipt_sha256)?;
        self.finish_terminal(terminal, vec![code.to_owned()], None)
    }

    fn finish_invalid_proposal(&mut self, turn_id: &RuntimeTurnId) -> Result<(), RuntimeLoopError> {
        self.transition_terminal(AgentStateKind::Failed)?;
        self.close_turn(turn_id, sha256(b"runtime.proposal.invalid"))?;
        self.finish_terminal(
            AgentStateKind::Failed,
            vec!["runtime.proposal.invalid".to_owned()],
            None,
        )
    }

    fn finish_model_failure(
        &mut self,
        turn_id: &RuntimeTurnId,
        failure: RuntimePortFailure,
    ) -> Result<(), RuntimeLoopError> {
        let state = match failure {
            RuntimePortFailure::Cancelled => AgentStateKind::Cancelled,
            RuntimePortFailure::TimedOut | RuntimePortFailure::ResourceExhausted => {
                AgentStateKind::Exhausted
            }
            RuntimePortFailure::Uncertain => AgentStateKind::Uncertain,
            RuntimePortFailure::Invalid | RuntimePortFailure::Unavailable => AgentStateKind::Failed,
        };
        self.transition_terminal(state)?;
        self.close_turn(turn_id, sha256(failure.code().as_bytes()))?;
        self.finish_terminal(state, vec![failure.code().to_owned()], None)
    }

    fn cancel(&mut self, signal: CancellationSignal) -> Result<(), RuntimeLoopError> {
        if signal.schema_version != CONTRACT_SCHEMA_VERSION
            || signal.task_id != self.request.task.task_id
            || signal.correlation_id != self.correlation_id
        {
            return Err(RuntimeLoopError::InvalidBoundaryResult);
        }
        let turn_id = self.active_turn.clone();
        let operation_id = self
            .pending
            .as_ref()
            .map(|pending| pending.operation_id.clone());
        self.emit(
            RuntimeEventKind::CancellationRequested {
                cancellation_id: signal.cancellation_id.clone(),
            },
            turn_id.as_ref(),
            operation_id.as_ref(),
        )?;
        self.emit(
            RuntimeEventKind::CancellationObserved {
                cancellation_id: signal.cancellation_id,
            },
            turn_id.as_ref(),
            operation_id.as_ref(),
        )?;
        self.pending = None;
        self.active_turn = None;
        self.transition_terminal(AgentStateKind::Cancelled)?;
        self.finish_terminal(
            AgentStateKind::Cancelled,
            vec!["runtime.cancelled".to_owned()],
            None,
        )
    }

    fn transition_terminal(&mut self, state: AgentStateKind) -> Result<(), RuntimeLoopError> {
        if self.state.current() == state {
            return Ok(());
        }
        self.state
            .transition(state)
            .map(|_| ())
            .map_err(|_| RuntimeLoopError::State)
    }

    fn close_turn(
        &mut self,
        turn_id: &RuntimeTurnId,
        outcome_sha256: String,
    ) -> Result<(), RuntimeLoopError> {
        if self.active_turn.as_ref() != Some(turn_id) {
            return Err(RuntimeLoopError::InvalidBoundaryResult);
        }
        self.emit(
            RuntimeEventKind::TurnCompleted { outcome_sha256 },
            Some(turn_id),
            None,
        )?;
        self.active_turn = None;
        Ok(())
    }

    fn finish_terminal(
        &mut self,
        state: AgentStateKind,
        mut unresolved_codes: Vec<String>,
        output: Option<RuntimeOutput>,
    ) -> Result<(), RuntimeLoopError> {
        if self.outcome.is_some() {
            return Ok(());
        }
        if self.state.current() != state || !state.is_terminal() || self.active_turn.is_some() {
            return Err(RuntimeLoopError::State);
        }
        unresolved_codes.sort();
        unresolved_codes.dedup();
        self.receipt_ids
            .sort_by(|left, right| left.as_str().cmp(right.as_str()));
        self.receipt_ids.dedup();
        let prior = self
            .events
            .last()
            .ok_or(RuntimeLoopError::InvalidBoundaryResult)?;
        let outcome = seal_runtime_outcome(
            RuntimeOutcome {
                schema_version: CONTRACT_SCHEMA_VERSION,
                run_id: self.request.run_id.clone(),
                session_id: self.request.session_id.clone(),
                task_id: self.request.task.task_id.clone(),
                request_sha256: self.request.request_sha256.clone(),
                state,
                turn_count: self.turn_count,
                model_call_count: self.model_call_count,
                tool_call_count: self.tool_call_count,
                prior_event_id: prior.event_id.clone(),
                prior_event_sha256: prior.event_sha256.clone(),
                evidence: self.evidence.clone(),
                receipt_ids: self.receipt_ids.clone(),
                unresolved_codes,
                output,
                outcome_sha256: ZERO_SHA256.to_owned(),
            },
            &self.request,
        )?;
        self.emit(
            RuntimeEventKind::RunTerminal {
                state,
                outcome_sha256: outcome.outcome_sha256.clone(),
            },
            None,
            None,
        )?;
        self.outcome = Some(outcome);
        Ok(())
    }

    fn emit(
        &mut self,
        kind: RuntimeEventKind,
        turn_id: Option<&RuntimeTurnId>,
        operation_id: Option<&RuntimeOperationId>,
    ) -> Result<RuntimeEventDelivery, RuntimeLoopError> {
        let now = self
            .clock
            .now_epoch_ms()
            .map_err(RuntimeLoopError::Dependency)?;
        self.emit_at(now, kind, turn_id, operation_id)
    }

    fn emit_at(
        &mut self,
        occurred_at_epoch_ms: u64,
        kind: RuntimeEventKind,
        turn_id: Option<&RuntimeTurnId>,
        operation_id: Option<&RuntimeOperationId>,
    ) -> Result<RuntimeEventDelivery, RuntimeLoopError> {
        if self.events.len() >= self.request.limits.max_events as usize
            || occurred_at_epoch_ms == 0
            || self
                .events
                .last()
                .is_some_and(|event| occurred_at_epoch_ms < event.occurred_at_epoch_ms)
        {
            return Err(RuntimeLoopError::InvalidBoundaryResult);
        }
        let sequence = self.events.len() as u64;
        let event = seal_runtime_event(RuntimeEvent {
            schema_version: CONTRACT_SCHEMA_VERSION,
            event_id: RuntimeEventId::from_raw(derived_id(
                "event",
                self.request.run_id.as_str(),
                sequence,
            )),
            run_id: self.request.run_id.clone(),
            session_id: self.request.session_id.clone(),
            task_id: self.request.task.task_id.clone(),
            turn_id: turn_id.cloned(),
            operation_id: operation_id.cloned(),
            correlation_id: self.correlation_id.clone(),
            causation_event_id: self.events.last().map(|event| event.event_id.clone()),
            sequence,
            occurred_at_epoch_ms,
            sensitivity: runtime_sensitivity(&self.request),
            retention: RuntimeEventRetention {
                kind: RuntimeEventRetentionKind::Ephemeral,
                expires_at_epoch_ms: None,
            },
            persistence: runtime_event_persistence(&kind),
            policy_id: self.request.policy_id.clone(),
            payload_reference: None,
            kind,
            previous_event_sha256: self.events.last().map_or_else(
                || ZERO_SHA256.to_owned(),
                |event| event.event_sha256.clone(),
            ),
            event_sha256: ZERO_SHA256.to_owned(),
        })?;
        let delivery = self.publisher.publish(event.clone())?;
        self.events.push(event);
        Ok(delivery)
    }

    fn remaining_events(&self) -> u32 {
        self.request
            .limits
            .max_events
            .saturating_sub(self.events.len() as u32)
    }

    fn elapsed_limit_reached(&mut self) -> Result<bool, RuntimeLoopError> {
        let now = self
            .clock
            .now_epoch_ms()
            .map_err(RuntimeLoopError::Dependency)?;
        let started = self
            .started_at_epoch_ms
            .ok_or(RuntimeLoopError::InvalidBoundaryResult)?;
        Ok(now.saturating_sub(started) >= self.request.limits.max_elapsed_ms)
    }
}

/// Computes exact stable tool references for one frozen registry.
pub fn runtime_tool_references(
    registry: &ToolRegistry,
) -> Result<Vec<RuntimeToolReference>, RuntimeLoopError> {
    registry
        .list_tools()
        .into_iter()
        .map(|definition| {
            Ok(RuntimeToolReference {
                tool_id: definition.tool_id.clone(),
                tool_version: definition.tool_version.clone(),
                definition_sha256: contract_sha256(definition)?,
            })
        })
        .collect()
}

fn permission_challenge(
    request: &RuntimeRunRequest,
    turn_id: &RuntimeTurnId,
    operation_id: &RuntimeOperationId,
    definition: &ToolDefinition,
    call: &ToolCall,
    evaluation: &RuntimePermissionEvaluation,
) -> Result<RuntimeApprovalChallenge, RuntimeLoopError> {
    let (approval_id, proposed_grant_id, preview_sha256, expires_at_epoch_ms) = match evaluation {
        RuntimePermissionEvaluation::Allow {
            approval_id,
            grant_id,
            preview_sha256,
            expires_at_epoch_ms,
            ..
        }
        | RuntimePermissionEvaluation::Ask {
            approval_id,
            grant_id,
            preview_sha256,
            expires_at_epoch_ms,
        }
        | RuntimePermissionEvaluation::Deny {
            approval_id,
            grant_id,
            preview_sha256,
            expires_at_epoch_ms,
            ..
        } => (approval_id, grant_id, preview_sha256, *expires_at_epoch_ms),
    };
    Ok(seal_runtime_approval_challenge(RuntimeApprovalChallenge {
        schema_version: CONTRACT_SCHEMA_VERSION,
        run_id: request.run_id.clone(),
        task_id: request.task.task_id.clone(),
        turn_id: turn_id.clone(),
        operation_id: operation_id.clone(),
        tool_call_id: call.tool_call_id.clone(),
        approval_id: approval_id.clone(),
        proposed_grant_id: proposed_grant_id.clone(),
        operation: definition.required_grant.operation.operation(),
        preview_sha256: preview_sha256.clone(),
        expires_at_epoch_ms,
        challenge_sha256: ZERO_SHA256.to_owned(),
    })?)
}

fn runtime_postconditions(request: &RuntimeRunRequest) -> Vec<PostconditionId> {
    request
        .task
        .acceptance_criteria
        .iter()
        .enumerate()
        .map(|(index, value)| {
            PostconditionId::from_raw(format!(
                "postcondition:{index}:{}",
                &sha256(value.as_bytes())[..16]
            ))
        })
        .collect()
}

fn valid_context_packet(packet: &ModelContextPacket, request: &RuntimeRunRequest) -> bool {
    if packet.schema_version != CONTRACT_SCHEMA_VERSION
        || packet.session_id != request.session_id
        || packet.task_id != request.task.task_id
        || packet.profile_id != request.model_profile.profile_id
        || packet.manifest_sha256 != request.model_profile.manifest_sha256
        || packet.tool_catalog_id != request.tool_catalog_id
        || packet.messages.is_empty()
        || packet.messages.len() > request.context_budget.max_messages as usize
        || packet.input_bytes == 0
        || packet.input_bytes > request.context_budget.max_input_bytes
        || packet.input_tokens == 0
        || packet.input_tokens > request.context_budget.max_context_tokens
        || !valid_sha256(&packet.packet_sha256)
    {
        return false;
    }
    let mut message_ids = BTreeSet::new();
    let mut input_bytes = 0_u64;
    for message in &packet.messages {
        let Ok(bytes) = u64::try_from(message.content.bytes.len()) else {
            return false;
        };
        let Some(next_bytes) = input_bytes.checked_add(bytes) else {
            return false;
        };
        input_bytes = next_bytes;
        if !message_ids.insert(message.message_id.as_str())
            || !valid_output_payload(&message.content, request.context_budget.max_input_bytes)
        {
            return false;
        }
    }
    if input_bytes != packet.input_bytes {
        return false;
    }
    let mut preimage = packet.clone();
    preimage.packet_sha256 = ZERO_SHA256.to_owned();
    contract_sha256(&preimage).is_ok_and(|digest| digest == packet.packet_sha256)
}

fn valid_model_result(result: &ModelRunResult, request: &ModelRunRequest) -> bool {
    if result.schema_version != CONTRACT_SCHEMA_VERSION
        || result.model_run_id != request.model_run_id
        || result.correlation_id != request.correlation_id
        || result.fragment_count == 0
        || !valid_identifier(result.stream_id.as_str())
        || !valid_sha256(&result.response_sha256)
        || (matches!(result.terminal_state, ModelRunTerminalState::Proposed)
            != result.proposal.is_some())
        || result.resources.adapter_id != request.adapter_id
        || result.resources.profile_id != request.profile_id
        || result.resources.model_run_id.as_ref() != Some(&request.model_run_id)
    {
        return false;
    }
    result.proposal.as_ref().is_none_or(|proposal| {
        proposal.model_run_id == request.model_run_id
            && proposal.context_packet_id == request.context_packet_id
            && proposal.profile_id == request.profile_id
            && proposal.correlation_id == request.correlation_id
            && crate::model_codec::proposal_digest(proposal)
                .is_ok_and(|digest| digest == proposal.proposal_sha256)
    })
}

fn valid_permission_evaluation(
    evaluation: &RuntimePermissionEvaluation,
    now_epoch_ms: u64,
) -> bool {
    let (approval_id, preview_sha256, expires_at_epoch_ms) = match evaluation {
        RuntimePermissionEvaluation::Allow {
            approval_id,
            preview_sha256,
            expires_at_epoch_ms,
            grant_id,
            decision_sha256,
            authority_sha256,
        } => {
            if !valid_identifier(grant_id.as_str())
                || !valid_sha256(decision_sha256)
                || !valid_sha256(authority_sha256)
            {
                return false;
            }
            (approval_id, preview_sha256, expires_at_epoch_ms)
        }
        RuntimePermissionEvaluation::Ask {
            approval_id,
            grant_id,
            preview_sha256,
            expires_at_epoch_ms,
        } => {
            if !valid_identifier(grant_id.as_str()) {
                return false;
            }
            (approval_id, preview_sha256, expires_at_epoch_ms)
        }
        RuntimePermissionEvaluation::Deny {
            approval_id,
            grant_id,
            preview_sha256,
            expires_at_epoch_ms,
            decision_sha256,
            reason_code,
        } => {
            if !valid_identifier(grant_id.as_str())
                || !valid_sha256(decision_sha256)
                || !valid_code(reason_code)
            {
                return false;
            }
            (approval_id, preview_sha256, expires_at_epoch_ms)
        }
    };
    valid_identifier(approval_id.as_str())
        && valid_sha256(preview_sha256)
        && *expires_at_epoch_ms > now_epoch_ms
}

fn valid_tool_execution(
    execution: &RuntimeToolExecution,
    definition: &ToolDefinition,
    call: &ToolCall,
    request: &RuntimeRunRequest,
) -> bool {
    let result = &execution.result;
    let mut evidence_ids = BTreeSet::new();
    valid_identifier(execution.receipt_id.as_str())
        && valid_sha256(&execution.receipt_sha256)
        && result.schema_version == CONTRACT_SCHEMA_VERSION
        && result.tool_call_id == call.tool_call_id
        && result.correlation_id == call.correlation_id
        && valid_runtime_state_change(
            request.mode,
            definition.required_grant.operation.operation(),
            result.outcome,
            result.state_change,
        )
        && result
            .output
            .as_ref()
            .is_none_or(|payload| valid_output_payload(payload, request.limits.max_output_bytes))
        && result.evidence.iter().all(|evidence| {
            evidence.schema_version == CONTRACT_SCHEMA_VERSION
                && valid_identifier(evidence.evidence_id.as_str())
                && evidence_ids.insert(evidence.evidence_id.as_str())
                && valid_sha256(&evidence.content_sha256)
                && evidence.observed_revision.as_deref()
                    == Some(request.repository_snapshot_id.as_str())
        })
}

fn catalog_allowed_for_mode(mode: RuntimeSessionMode, registry: &ToolRegistry) -> bool {
    registry.list_tools().iter().all(|definition| {
        let operation = definition.required_grant.operation.operation();
        match mode {
            RuntimeSessionMode::EphemeralReadOnly | RuntimeSessionMode::DurableReadOnly => {
                operation == GrantOperation::WorkspaceRead
            }
            RuntimeSessionMode::ControlledWrite => matches!(
                operation,
                GrantOperation::WorkspaceRead
                    | GrantOperation::WorkspaceWrite
                    | GrantOperation::CommandExecute
            ),
        }
    })
}

fn valid_runtime_state_change(
    mode: RuntimeSessionMode,
    operation: GrantOperation,
    outcome: OperationOutcome,
    state_change: StateChange,
) -> bool {
    match (mode, operation, outcome, state_change) {
        (
            RuntimeSessionMode::ControlledWrite,
            GrantOperation::WorkspaceWrite,
            OperationOutcome::Succeeded,
            StateChange::Changed,
        )
        | (
            RuntimeSessionMode::ControlledWrite,
            _,
            OperationOutcome::Uncertain,
            StateChange::Uncertain,
        ) => true,
        (_, _, OperationOutcome::Succeeded, StateChange::NotChanged) => {
            operation != GrantOperation::WorkspaceWrite
        }
        (_, _, OperationOutcome::Denied, StateChange::NotChanged)
        | (_, _, OperationOutcome::Failed, StateChange::NotChanged)
        | (_, _, OperationOutcome::Cancelled, StateChange::NotChanged)
        | (_, _, OperationOutcome::TimedOut, StateChange::NotChanged) => true,
        _ => false,
    }
}

fn valid_output_payload(
    payload: &agentmage_kernel_contracts::ContractPayload,
    maximum_bytes: u64,
) -> bool {
    valid_identifier(payload.schema.schema_id.as_str())
        && payload.schema.schema_version > 0
        && valid_sha256(&payload.schema.schema_sha256)
        && valid_media_type(&payload.media_type)
        && u64::try_from(payload.bytes.len()).is_ok_and(|bytes| bytes > 0 && bytes <= maximum_bytes)
        && payload.sha256 == sha256(&payload.bytes)
}

fn observe_cancellation(
    cancellation: Option<&dyn ModelCancellationProbe>,
) -> Result<Option<CancellationSignal>, RuntimeLoopError> {
    cancellation
        .map(|probe| probe.observe())
        .transpose()
        .map(|value| value.flatten())
        .map_err(|_| RuntimeLoopError::Dependency(RuntimePortFailure::Unavailable))
}

fn map_model_error(error: ModelRuntimeGateError) -> RuntimePortFailure {
    match error {
        ModelRuntimeGateError::NotLoaded | ModelRuntimeGateError::RuntimeFailure => {
            RuntimePortFailure::Unavailable
        }
        ModelRuntimeGateError::ProfileInvalid
        | ModelRuntimeGateError::ProfileNotRegistered
        | ModelRuntimeGateError::ProfileChanged
        | ModelRuntimeGateError::ProfileUnavailable
        | ModelRuntimeGateError::AutomaticFallbackProhibited
        | ModelRuntimeGateError::RuntimeMismatch
        | ModelRuntimeGateError::ManifestObservationMismatch
        | ModelRuntimeGateError::IsolationViolation
        | ModelRuntimeGateError::AlreadyLoaded
        | ModelRuntimeGateError::RequestMismatch
        | ModelRuntimeGateError::TokenCountMismatch
        | ModelRuntimeGateError::StreamInvalid
        | ModelRuntimeGateError::ResultMismatch => RuntimePortFailure::Invalid,
    }
}

fn runtime_sensitivity(
    request: &RuntimeRunRequest,
) -> agentmage_kernel_contracts::ContextSensitivity {
    match request.work_packet.sensitivity {
        agentmage_kernel_contracts::DataSensitivity::Ephemeral => {
            agentmage_kernel_contracts::ContextSensitivity::Private
        }
        agentmage_kernel_contracts::DataSensitivity::Operational
        | agentmage_kernel_contracts::DataSensitivity::Durable => {
            agentmage_kernel_contracts::ContextSensitivity::Internal
        }
        agentmage_kernel_contracts::DataSensitivity::Restricted => {
            agentmage_kernel_contracts::ContextSensitivity::Restricted
        }
    }
}

fn contract_sha256<T: agentmage_kernel_contracts::VersionedContract>(
    value: &T,
) -> Result<String, RuntimeLoopError> {
    let bytes = to_canonical_json(value).map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
    Ok(sha256(&bytes))
}

fn derived_id(prefix: &str, run_id: &str, sequence: u64) -> String {
    format!(
        "{prefix}:{}",
        &sha256(format!("{run_id}:{sequence}").as_bytes())[..32]
    )
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn valid_code(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn valid_media_type(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'+' | b'.' | b'-'))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == SHA256_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
#[path = "runtime_loop_tests.rs"]
mod tests;
