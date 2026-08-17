//! Interface-independent reusable runtime coordinator.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{
    ActionId, AgentProposal, AgentStateKind, ApprovalId, BudgetResource, CONTRACT_SCHEMA_VERSION,
    CancellationSignal, ClosedModelProposal, ContextPacketId, CorrelationId, EvidenceReference,
    ExactModelProfile, GrantId, GrantOperation, LocalModelRuntime, ModelCancellationProbe,
    ModelContextPacket, ModelFamilyCodec, ModelProposalKind, ModelRunId, ModelRunRequest,
    ModelRunResult, ModelRunTerminalState, OperationOutcome, PostconditionId, ReceiptId,
    RuntimeApprovalChallenge, RuntimeApprovalDisposition, RuntimeApprovalResponse,
    RuntimeArtifactId, RuntimeArtifactIntegrityState, RuntimeArtifactKind, RuntimeArtifactManifest,
    RuntimeArtifactPreview, RuntimeArtifactRef, RuntimeContinuationState, RuntimeEvent,
    RuntimeEventCursor, RuntimeEventId, RuntimeEventKind, RuntimeEventRetention,
    RuntimeEventRetentionKind, RuntimeOperationId, RuntimeOutcome, RuntimeOutput,
    RuntimePayloadReference, RuntimePermissionDisposition, RuntimeResumeBinding, RuntimeRunRequest,
    RuntimeSessionMode, RuntimeToolAttemptState, RuntimeToolReference, RuntimeTurnId,
    SessionCheckpoint, StateChange, ToolCall, ToolDefinition, ToolResult, VerifierCandidate,
    VerifierId, to_canonical_json,
};
use sha2::{Digest, Sha256};

use crate::agent_proposal::{ExpectedProposalContext, ProposalAdmissionRegistry};
use crate::agent_state::AgentStateController;
use crate::agent_verifier::{VerifierContext, VerifierRegistry};
use crate::context_management::verify_checkpoint;
use crate::model_runtime::{LocalModelController, ModelRuntimeGateError};
use crate::runtime_artifact::{
    MAX_RUNTIME_ARTIFACT_PREVIEW_BYTES, RUNTIME_CONTINUATION_MEDIA_TYPE,
    encode_runtime_continuation_state, runtime_artifact_ref, runtime_payload_reference,
    seal_runtime_artifact_manifest, seal_runtime_continuation_state, verify_runtime_artifact_ref,
    verify_runtime_continuation_state, verify_runtime_resume_binding,
};
use crate::runtime_coordinator::{
    RuntimeCoordinatorError, runtime_tool_catalog_sha256, seal_runtime_approval_challenge,
    seal_runtime_outcome, seal_runtime_run_request, verify_runtime_approval_response,
    verify_runtime_run_request,
};
use crate::runtime_event::{
    RuntimeEventBatch, RuntimeEventBatchLimits, RuntimeEventDelivery, RuntimeEventError,
    RuntimeEventPublisher, RuntimeEventSubscription, replay_runtime_events,
    runtime_event_persistence, seal_runtime_event,
};
use crate::runtime_hardening::{RuntimeResourceLedger, RuntimeResourceSnapshot};
use crate::tooling::{
    PreGrantDispatchDisposition, ProposalOrigin, ToolAttemptGuard, ToolDispatcher, ToolRegistry,
};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MIN_RUNTIME_EVENTS: u32 = 6;
const SHA256_BYTES: usize = 64;
/// Maximum terminal or tool payload retained inline once a runtime artifact port is active.
pub const MAX_RUNTIME_INLINE_OUTPUT_BYTES: usize = 64 * 1024;

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

/// Optional durable journal boundary implemented by a trusted runtime host.
///
/// The model, client, and native tool definitions never receive this port. Implementations must
/// retain the canonical event envelope only, leaving transcript and artifact bytes in their
/// separately classified stores.
pub trait RuntimeJournalPort {
    /// Accepts one already sealed event under bounded queue and durability rules.
    fn append_runtime_event(&mut self, event: &RuntimeEvent) -> Result<(), RuntimePortFailure>;

    /// Flushes every previously accepted deferred progress event.
    fn flush_runtime_events(&mut self) -> Result<(), RuntimePortFailure>;

    /// Returns one complete verified durable history for cursor and resume reconciliation.
    fn load_runtime_events(
        &mut self,
        run_id: &agentmage_kernel_contracts::RuntimeRunId,
    ) -> Result<Vec<RuntimeEvent>, RuntimePortFailure>;
}

/// Optional private content-addressed artifact boundary implemented by a trusted runtime host.
///
/// Implementations receive only a sealed path-free manifest and bounded bytes. The model, client,
/// tool registry, and renderer never receive native payload-store authority.
pub trait RuntimeArtifactPort {
    /// Publishes one exact immutable payload and returns its complete path-free reference.
    fn publish_runtime_artifact(
        &mut self,
        manifest: RuntimeArtifactManifest,
        payload: &[u8],
    ) -> Result<RuntimeArtifactRef, RuntimePortFailure>;
}

/// Immutable safe-boundary material supplied to the trusted checkpoint host.
pub struct RuntimeCheckpointCommit<'a> {
    /// Exact current runtime request.
    pub request: &'a RuntimeRunRequest,
    /// Sealed coordinator state represented by the continuation artifact.
    pub continuation: &'a RuntimeContinuationState,
    /// Exact continuation artifact reference.
    pub continuation_artifact: &'a RuntimeArtifactRef,
    /// Last journal event committed before checkpoint publication.
    pub event_cursor: &'a RuntimeEventCursor,
    /// Complete sorted artifact set required by this checkpoint.
    pub artifacts: &'a [RuntimeArtifactRef],
}

/// Trusted result of one atomic checkpoint and resume-binding publication.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeCheckpointPublication {
    /// Canonical metadata-only session checkpoint.
    pub checkpoint: SessionCheckpoint,
    /// Exact cursor and artifact set atomically bound to that checkpoint.
    pub binding: RuntimeResumeBinding,
}

/// Complete verified material loaded by the trusted host for one restart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeResumeSnapshot {
    /// Current metadata-only session checkpoint.
    pub checkpoint: SessionCheckpoint,
    /// Exact persisted cursor and artifact set.
    pub binding: RuntimeResumeBinding,
    /// Safe-boundary coordinator continuation decoded from private artifact storage.
    pub continuation: RuntimeContinuationState,
    /// Exact artifact from which `continuation` was decoded.
    pub continuation_artifact: RuntimeArtifactRef,
}

/// Optional atomic checkpoint and private resume boundary implemented by a trusted host.
pub trait RuntimeCheckpointPort {
    /// Commits one verified safe boundary after its continuation artifact event is durable.
    fn commit_runtime_checkpoint(
        &mut self,
        input: RuntimeCheckpointCommit<'_>,
    ) -> Result<RuntimeCheckpointPublication, RuntimePortFailure>;

    /// Loads the one current checkpoint and decoded continuation for an exact resume request.
    fn load_runtime_checkpoint(
        &mut self,
        request: &RuntimeRunRequest,
    ) -> Result<Option<RuntimeResumeSnapshot>, RuntimePortFailure>;
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
    /// Ordered successful tool results observed by this coordinator.
    pub tool_results: &'a [ToolResult],
    /// Canonical effect receipts accumulated by this coordinator.
    pub receipt_ids: &'a [ReceiptId],
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

#[derive(Clone, Copy)]
struct RuntimeJournalHooks<T> {
    append: fn(&mut T, &RuntimeEvent) -> Result<(), RuntimePortFailure>,
    flush: fn(&mut T) -> Result<(), RuntimePortFailure>,
    load: fn(
        &mut T,
        &agentmage_kernel_contracts::RuntimeRunId,
    ) -> Result<Vec<RuntimeEvent>, RuntimePortFailure>,
}

#[derive(Clone, Copy)]
struct RuntimeArtifactHooks<T> {
    publish: fn(
        &mut T,
        RuntimeArtifactManifest,
        &[u8],
    ) -> Result<RuntimeArtifactRef, RuntimePortFailure>,
}

#[derive(Clone, Copy)]
struct RuntimeCheckpointHooks<T> {
    commit: for<'a> fn(
        &mut T,
        RuntimeCheckpointCommit<'a>,
    ) -> Result<RuntimeCheckpointPublication, RuntimePortFailure>,
    load:
        fn(&mut T, &RuntimeRunRequest) -> Result<Option<RuntimeResumeSnapshot>, RuntimePortFailure>,
}

/// One reusable, interface-neutral runtime coordinator.
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
    resources: RuntimeResourceLedger,
    journal: Option<RuntimeJournalHooks<T>>,
    artifact: Option<RuntimeArtifactHooks<T>>,
    checkpoint: Option<RuntimeCheckpointHooks<T>>,
    state: AgentStateController,
    attempt_guard: ToolAttemptGuard,
    events: Vec<RuntimeEvent>,
    tool_results: Vec<ToolResult>,
    evidence: Vec<EvidenceReference>,
    receipt_ids: Vec<ReceiptId>,
    artifact_references: Vec<RuntimeArtifactRef>,
    tool_attempts: Vec<RuntimeToolAttemptState>,
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
        Self::compose(
            request,
            model,
            context,
            registry,
            tool_boundary,
            verifier,
            clock,
            None,
            None,
            None,
        )
    }

    /// Composes one durable request with the same trusted host boundary used for native effects.
    pub fn new_with_journal(
        request: RuntimeRunRequest,
        model: M,
        context: X,
        registry: ToolRegistry,
        tool_boundary: T,
        verifier: V,
        clock: C,
    ) -> Result<Self, RuntimeLoopError>
    where
        T: RuntimeJournalPort,
    {
        Self::compose(
            request,
            model,
            context,
            registry,
            tool_boundary,
            verifier,
            clock,
            Some(RuntimeJournalHooks {
                append: append_runtime_event::<T>,
                flush: flush_runtime_events::<T>,
                load: load_runtime_events::<T>,
            }),
            None,
            None,
        )
    }

    /// Composes one persisted request with both canonical journal and private artifact ports.
    pub fn new_with_persistence(
        request: RuntimeRunRequest,
        model: M,
        context: X,
        registry: ToolRegistry,
        tool_boundary: T,
        verifier: V,
        clock: C,
    ) -> Result<Self, RuntimeLoopError>
    where
        T: RuntimeJournalPort + RuntimeArtifactPort,
    {
        Self::compose(
            request,
            model,
            context,
            registry,
            tool_boundary,
            verifier,
            clock,
            Some(RuntimeJournalHooks {
                append: append_runtime_event::<T>,
                flush: flush_runtime_events::<T>,
                load: load_runtime_events::<T>,
            }),
            Some(RuntimeArtifactHooks {
                publish: publish_runtime_artifact::<T>,
            }),
            None,
        )
    }

    /// Composes one fully durable request with journal, artifact, checkpoint, and resume ports.
    pub fn new_with_durable_state(
        request: RuntimeRunRequest,
        model: M,
        context: X,
        registry: ToolRegistry,
        tool_boundary: T,
        verifier: V,
        clock: C,
    ) -> Result<Self, RuntimeLoopError>
    where
        T: RuntimeJournalPort + RuntimeArtifactPort + RuntimeCheckpointPort,
    {
        Self::compose(
            request,
            model,
            context,
            registry,
            tool_boundary,
            verifier,
            clock,
            Some(RuntimeJournalHooks {
                append: append_runtime_event::<T>,
                flush: flush_runtime_events::<T>,
                load: load_runtime_events::<T>,
            }),
            Some(RuntimeArtifactHooks {
                publish: publish_runtime_artifact::<T>,
            }),
            Some(RuntimeCheckpointHooks {
                commit: commit_runtime_checkpoint::<T>,
                load: load_runtime_checkpoint::<T>,
            }),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn compose(
        request: RuntimeRunRequest,
        model: M,
        context: X,
        registry: ToolRegistry,
        tool_boundary: T,
        verifier: V,
        clock: C,
        journal: Option<RuntimeJournalHooks<T>>,
        artifact: Option<RuntimeArtifactHooks<T>>,
        checkpoint: Option<RuntimeCheckpointHooks<T>>,
    ) -> Result<Self, RuntimeLoopError> {
        verify_runtime_run_request(&request)?;
        if request.mode == RuntimeSessionMode::DurableReadOnly && journal.is_none() {
            return Err(RuntimeLoopError::UnsupportedMode);
        }
        if request.mode == RuntimeSessionMode::EphemeralReadOnly && journal.is_some() {
            return Err(RuntimeLoopError::UnsupportedMode);
        }
        if artifact.is_some()
            && (journal.is_none() || request.mode == RuntimeSessionMode::EphemeralReadOnly)
        {
            return Err(RuntimeLoopError::UnsupportedMode);
        }
        if checkpoint.is_some() && (journal.is_none() || artifact.is_none()) {
            return Err(RuntimeLoopError::UnsupportedMode);
        }
        if request.event_cursor.is_some() && checkpoint.is_none() {
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
        let resources = RuntimeResourceLedger::new(&request)
            .map_err(|_| RuntimeLoopError::Contract(RuntimeCoordinatorError::InvalidLimits))?;
        let correlation_id =
            CorrelationId::from_raw(derived_id("correlation", request.run_id.as_str(), 0));
        let evidence = request.work_packet.authoritative_evidence.clone();
        let mut coordinator = Self {
            request,
            model,
            context,
            registry,
            tool_boundary,
            verifier,
            clock,
            publisher: RuntimeEventPublisher::new(),
            resources,
            journal,
            artifact,
            checkpoint,
            state: AgentStateController::new(),
            attempt_guard,
            events: Vec::new(),
            tool_results: Vec::new(),
            evidence,
            receipt_ids: Vec::new(),
            artifact_references: Vec::new(),
            tool_attempts: Vec::new(),
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
        };
        if coordinator.request.event_cursor.is_some() {
            coordinator.restore_runtime_checkpoint()?;
        }
        Ok(coordinator)
    }

    /// Registers one bounded ordered event subscriber before or during execution.
    pub fn subscribe_events(
        &self,
        capacity: usize,
    ) -> Result<RuntimeEventSubscription, RuntimeLoopError> {
        self.resources
            .validate_subscription_capacity(capacity)
            .map_err(|_| RuntimeLoopError::Event(RuntimeEventError::SubscriberLimit))?;
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

    /// Returns exact verified artifacts published by this invocation in creation order.
    #[must_use]
    pub fn artifact_references(&self) -> &[RuntimeArtifactRef] {
        &self.artifact_references
    }

    /// Returns content-free request-bound resource accounting for diagnostics and verification.
    #[must_use]
    pub const fn resource_snapshot(&self) -> &RuntimeResourceSnapshot {
        self.resources.snapshot()
    }

    /// Replays one verified bounded event page after an exact client reconnect cursor.
    pub fn replay_events(
        &self,
        after: Option<&RuntimeEventCursor>,
        limits: RuntimeEventBatchLimits,
    ) -> Result<RuntimeEventBatch, RuntimeLoopError> {
        replay_runtime_events(&self.events, after, limits).map_err(Into::into)
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
        self.resources
            .consume(BudgetResource::PlanSteps, 1)
            .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
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
        if self
            .resources
            .consume(BudgetResource::InputBytes, context.input_bytes)
            .is_err()
        {
            return self.finish_budget_exhaustion(&turn_id);
        }

        self.resources
            .consume(BudgetResource::ModelCalls, 1)
            .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
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
        let model_memory = result
            .resources
            .resident_memory_bytes
            .checked_add(result.resources.accelerator_memory_bytes);
        if model_memory.is_none_or(|bytes| self.resources.observe_memory_peak(bytes).is_err())
            || self
                .resources
                .consume(
                    BudgetResource::ElapsedMilliseconds,
                    result.resources.elapsed_ms,
                )
                .is_err()
        {
            self.emit(
                RuntimeEventKind::ModelFailed {
                    model_run_id,
                    failure_code: "runtime.budget.exhausted".to_owned(),
                },
                Some(&turn_id),
                None,
            )?;
            return self.finish_model_failure(&turn_id, RuntimePortFailure::ResourceExhausted);
        }
        let result_sha256 = contract_sha256(&result)?;
        match result.terminal_state {
            ModelRunTerminalState::Proposed => {
                self.emit(
                    RuntimeEventKind::ModelCompleted {
                        model_run_id: model_run_id.clone(),
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
                        model_run_id: model_run_id.clone(),
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
                        model_run_id: model_run_id.clone(),
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
                        model_run_id: model_run_id.clone(),
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
                let output = match proposal.payload {
                    Some(payload) => {
                        let Some(output) = self.route_runtime_output(
                            payload,
                            RuntimeArtifactKind::ModelOutput,
                            &turn_id,
                            None,
                            None,
                        )?
                        else {
                            return self.finish_budget_exhaustion(&turn_id);
                        };
                        Some(output)
                    }
                    None => None,
                };
                self.state
                    .transition(AgentStateKind::Blocked)
                    .map_err(|_| RuntimeLoopError::State)?;
                self.close_turn(&turn_id, proposal.proposal_sha256)?;
                self.finish_terminal(
                    AgentStateKind::Blocked,
                    vec!["runtime.proposal.blocked".to_owned()],
                    output,
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
                tool_results: &self.tool_results,
                receipt_ids: &self.receipt_ids,
            })
            .map_err(RuntimeLoopError::Dependency)?;
        let completion = match registry.verify(&candidate) {
            Ok(completion) => completion,
            Err(_) => {
                let Some(output) = self.route_runtime_output(
                    payload,
                    RuntimeArtifactKind::ModelOutput,
                    &turn_id,
                    None,
                    None,
                )?
                else {
                    return self.finish_budget_exhaustion(&turn_id);
                };
                self.state
                    .transition(AgentStateKind::Failed)
                    .map_err(|_| RuntimeLoopError::State)?;
                self.close_turn(&turn_id, proposal.proposal_sha256)?;
                return self.finish_terminal(
                    AgentStateKind::Failed,
                    vec!["runtime.verification.failed".to_owned()],
                    Some(output),
                );
            }
        };
        let Some(output) = self.route_runtime_output(
            payload,
            RuntimeArtifactKind::ModelOutput,
            &turn_id,
            None,
            None,
        )?
        else {
            return self.finish_budget_exhaustion(&turn_id);
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
        self.finish_terminal(terminal, Vec::new(), Some(output))
    }

    fn propose_tool(
        &mut self,
        turn_id: RuntimeTurnId,
        proposal: ClosedModelProposal,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<(), RuntimeLoopError> {
        let required_events = if self.checkpoint.is_some() { 9 } else { 7 };
        if self.tool_call_count >= self.request.limits.max_tool_calls
            || self.remaining_events() < required_events
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
        self.resources
            .consume(BudgetResource::ToolCalls, 1)
            .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
        self.tool_call_count += 1;
        let call = ToolCall {
            schema_version: CONTRACT_SCHEMA_VERSION,
            tool_call_id: candidate.tool_call_id,
            correlation_id: self.correlation_id.clone(),
            action_id: runtime_action_id(&self.request.run_id, self.tool_call_count),
            tool_id: candidate.tool_id,
            tool_version: candidate.tool_version,
            arguments: candidate.arguments,
        };
        let definition = self
            .registry
            .validate_arguments(&call)
            .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?
            .clone();
        let process_attempts = u64::from(
            definition.required_grant.operation.operation() == GrantOperation::CommandExecute,
        );
        if self
            .resources
            .consume(
                BudgetResource::InputBytes,
                u64::try_from(call.arguments.bytes.len())
                    .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?,
            )
            .is_err()
            || self
                .resources
                .consume(BudgetResource::ProcessCount, process_attempts)
                .is_err()
        {
            return self.finish_budget_exhaustion(&turn_id);
        }
        let receipt = ToolDispatcher::new(&self.registry).dispatch(ProposalOrigin::Model, &call);
        if receipt.disposition != PreGrantDispatchDisposition::GrantRequired {
            return Err(RuntimeLoopError::InvalidBoundaryResult);
        }
        let attempt = self
            .attempt_guard
            .record_attempt(&call, 0)
            .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
        self.tool_attempts.push(RuntimeToolAttemptState {
            schema_version: CONTRACT_SCHEMA_VERSION,
            sequence: attempt.sequence,
            tool_call_id: call.tool_call_id.clone(),
            semantic_sha256: attempt.semantic_sha256,
            occurrence: attempt.occurrence,
            call_depth: attempt.call_depth,
        });
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
                self.resources
                    .record_denial()
                    .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
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
                let result_sha256 = contract_sha256(&execution.result)?;
                let mut output_exhausted = false;
                if let Some(payload) = execution.result.output.clone() {
                    let artifact_kind = tool_artifact_kind(&definition, &payload.media_type);
                    output_exhausted = self
                        .route_runtime_output(
                            payload,
                            artifact_kind,
                            &turn_id,
                            Some(&operation_id),
                            Some(&execution.receipt_id),
                        )?
                        .is_none();
                }
                self.emit(
                    RuntimeEventKind::ToolCompleted {
                        tool_call_id: call.tool_call_id,
                        receipt_id: execution.receipt_id.clone(),
                        result_sha256,
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
                if output_exhausted {
                    self.transition_terminal(AgentStateKind::Exhausted)?;
                    self.finish_terminal(
                        AgentStateKind::Exhausted,
                        vec!["runtime.budget.exhausted".to_owned()],
                        None,
                    )
                } else if self.no_progress_turns >= self.request.limits.max_no_progress_turns {
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
                    self.persist_safe_boundary()
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

    fn persist_safe_boundary(&mut self) -> Result<(), RuntimeLoopError> {
        let Some(commit) = self.checkpoint.as_ref().map(|hooks| hooks.commit) else {
            return Ok(());
        };
        if self.state.current() != AgentStateKind::Observation
            || self.active_turn.is_some()
            || self.pending.is_some()
            || self.outcome.is_some()
            || self.remaining_events() < 2
        {
            return Err(RuntimeLoopError::InvalidBoundaryResult);
        }
        let continuation_cursor = runtime_event_cursor(
            self.events
                .last()
                .ok_or(RuntimeLoopError::InvalidBoundaryResult)?,
        );
        let continuation = seal_runtime_continuation_state(RuntimeContinuationState {
            schema_version: CONTRACT_SCHEMA_VERSION,
            request_sha256: runtime_base_request_sha256(&self.request)?,
            run_id: self.request.run_id.clone(),
            session_id: self.request.session_id.clone(),
            task_id: self.request.task.task_id.clone(),
            event_cursor: continuation_cursor,
            agent_state: self.state.current(),
            agent_state_revision: self.state.revision(),
            state_transitions: self.state.transitions().to_vec(),
            turn_count: self.turn_count,
            model_call_count: self.model_call_count,
            tool_call_count: self.tool_call_count,
            context_refresh_count: self.context_refresh_count,
            no_progress_turns: self.no_progress_turns,
            resources: self.resources.durable_usage(),
            tool_attempts: self.tool_attempts.clone(),
            tool_results: self.tool_results.clone(),
            evidence: self.evidence.clone(),
            receipt_ids: self.receipt_ids.clone(),
            artifacts: self.artifact_references.clone(),
            continuation_sha256: ZERO_SHA256.to_owned(),
        })
        .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
        let payload = encode_runtime_continuation_state(&continuation)
            .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
        if self
            .resources
            .admit_artifact(
                u64::try_from(payload.len())
                    .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?,
            )
            .is_err()
        {
            self.transition_terminal(AgentStateKind::Exhausted)?;
            return self.finish_terminal(
                AgentStateKind::Exhausted,
                vec!["runtime.budget.exhausted".to_owned()],
                None,
            );
        }
        let continuation_artifact = self.publish_artifact_bytes(
            &payload,
            RUNTIME_CONTINUATION_MEDIA_TYPE,
            RuntimeArtifactKind::Report,
            None,
            None,
            None,
            false,
        )?;
        let event_cursor = runtime_event_cursor(
            self.events
                .last()
                .ok_or(RuntimeLoopError::InvalidBoundaryResult)?,
        );
        let mut artifacts = self.artifact_references.clone();
        artifacts.sort_by(|left, right| left.artifact_id.as_str().cmp(right.artifact_id.as_str()));
        let publication = commit(
            &mut self.tool_boundary,
            RuntimeCheckpointCommit {
                request: &self.request,
                continuation: &continuation,
                continuation_artifact: &continuation_artifact,
                event_cursor: &event_cursor,
                artifacts: &artifacts,
            },
        )
        .map_err(RuntimeLoopError::Dependency)?;
        validate_runtime_checkpoint_publication(
            &self.request,
            &continuation,
            &continuation_artifact,
            &event_cursor,
            &artifacts,
            &publication,
        )?;
        self.emit(
            RuntimeEventKind::CheckpointCommitted {
                checkpoint_id: publication.checkpoint.checkpoint_id,
                checkpoint_sha256: publication.checkpoint.checkpoint_sha256,
            },
            None,
            None,
        )?;
        Ok(())
    }

    fn restore_runtime_checkpoint(&mut self) -> Result<(), RuntimeLoopError> {
        let requested_cursor = self
            .request
            .event_cursor
            .clone()
            .ok_or(RuntimeLoopError::UnsupportedMode)?;
        let journal = self
            .journal
            .as_ref()
            .ok_or(RuntimeLoopError::UnsupportedMode)?;
        let checkpoint = self
            .checkpoint
            .as_ref()
            .ok_or(RuntimeLoopError::UnsupportedMode)?;
        let events = (journal.load)(&mut self.tool_boundary, &self.request.run_id)
            .map_err(RuntimeLoopError::Dependency)?;
        let snapshot = (checkpoint.load)(&mut self.tool_boundary, &self.request)
            .map_err(RuntimeLoopError::Dependency)?
            .ok_or(RuntimeLoopError::UnsupportedMode)?;
        validate_runtime_resume_snapshot(&self.request, &requested_cursor, &events, &snapshot)?;

        let mut restored_state = AgentStateController::new();
        for expected in &snapshot.continuation.state_transitions {
            let admitted = restored_state
                .transition(expected.to)
                .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
            if admitted != expected {
                return Err(RuntimeLoopError::InvalidBoundaryResult);
            }
        }
        let restored_guard = ToolAttemptGuard::restore(
            self.request.limits.max_repeated_tool_calls,
            self.request.limits.max_tool_call_depth,
            &snapshot.continuation.tool_attempts,
        )
        .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
        let mut restored_resources =
            RuntimeResourceLedger::restore(&self.request, &snapshot.continuation.resources)
                .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
        let event_bytes = events.iter().try_fold(0_u64, |total, event| {
            let bytes = to_canonical_json(event)
                .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?
                .len();
            total
                .checked_add(
                    u64::try_from(bytes).map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?,
                )
                .ok_or(RuntimeLoopError::InvalidBoundaryResult)
        })?;
        let artifact_bytes =
            snapshot
                .binding
                .artifacts
                .iter()
                .try_fold(0_u64, |total, artifact| {
                    total
                        .checked_add(artifact.byte_size)
                        .ok_or(RuntimeLoopError::InvalidBoundaryResult)
                })?;
        restored_resources
            .reconcile_durable_projection(
                u32::try_from(events.len()).map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?,
                event_bytes,
                u32::try_from(snapshot.binding.artifacts.len())
                    .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?,
                artifact_bytes,
            )
            .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
        for event in &events {
            self.publisher.publish(event.clone())?;
        }

        self.started_at_epoch_ms = events.first().map(|event| event.occurred_at_epoch_ms);
        self.events = events;
        self.state = restored_state;
        self.attempt_guard = restored_guard;
        self.resources = restored_resources;
        self.tool_results = snapshot.continuation.tool_results;
        self.evidence = snapshot.continuation.evidence;
        self.receipt_ids = snapshot.continuation.receipt_ids;
        self.artifact_references = snapshot.binding.artifacts;
        self.tool_attempts = snapshot.continuation.tool_attempts;
        self.turn_count = snapshot.continuation.turn_count;
        self.model_call_count = snapshot.continuation.model_call_count;
        self.tool_call_count = snapshot.continuation.tool_call_count;
        self.context_refresh_count = snapshot.continuation.context_refresh_count;
        self.no_progress_turns = snapshot.continuation.no_progress_turns;
        Ok(())
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
        self.resources
            .record_parser_failure()
            .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
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

    fn finish_budget_exhaustion(
        &mut self,
        turn_id: &RuntimeTurnId,
    ) -> Result<(), RuntimeLoopError> {
        self.transition_terminal(AgentStateKind::Exhausted)?;
        self.close_turn(turn_id, sha256(b"runtime.budget.exhausted"))?;
        self.finish_terminal(
            AgentStateKind::Exhausted,
            vec!["runtime.budget.exhausted".to_owned()],
            None,
        )
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
        if let Some(journal) = self.journal.as_ref() {
            (journal.flush)(&mut self.tool_boundary).map_err(RuntimeLoopError::Dependency)?;
        }
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
        self.emit_at_with_payload(occurred_at_epoch_ms, kind, turn_id, operation_id, None)
    }

    fn emit_at_with_payload(
        &mut self,
        occurred_at_epoch_ms: u64,
        kind: RuntimeEventKind,
        turn_id: Option<&RuntimeTurnId>,
        operation_id: Option<&RuntimeOperationId>,
        payload_reference: Option<RuntimePayloadReference>,
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
                kind: if self.journal.is_some() {
                    RuntimeEventRetentionKind::Session
                } else {
                    RuntimeEventRetentionKind::Ephemeral
                },
                expires_at_epoch_ms: None,
            },
            persistence: runtime_event_persistence(&kind),
            policy_id: self.request.policy_id.clone(),
            payload_reference,
            kind,
            previous_event_sha256: self.events.last().map_or_else(
                || ZERO_SHA256.to_owned(),
                |event| event.event_sha256.clone(),
            ),
            event_sha256: ZERO_SHA256.to_owned(),
        })?;
        let event_bytes = to_canonical_json(&event)
            .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?
            .len();
        self.resources
            .admit_event(
                u64::try_from(event_bytes).map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?,
            )
            .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
        if let Some(journal) = self.journal.as_ref() {
            (journal.append)(&mut self.tool_boundary, &event)
                .map_err(RuntimeLoopError::Dependency)?;
        }
        let delivery = self.publisher.publish(event.clone())?;
        self.events.push(event);
        Ok(delivery)
    }

    fn route_runtime_output(
        &mut self,
        payload: agentmage_kernel_contracts::ContractPayload,
        kind: RuntimeArtifactKind,
        turn_id: &RuntimeTurnId,
        operation_id: Option<&RuntimeOperationId>,
        receipt_id: Option<&ReceiptId>,
    ) -> Result<Option<RuntimeOutput>, RuntimeLoopError> {
        let payload_bytes = u64::try_from(payload.bytes.len())
            .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
        if self
            .resources
            .consume(BudgetResource::OutputBytes, payload_bytes)
            .is_err()
        {
            return Ok(None);
        }
        if payload.bytes.len() <= MAX_RUNTIME_INLINE_OUTPUT_BYTES || self.artifact.is_none() {
            return Ok(Some(RuntimeOutput::Inline { payload }));
        }
        if self.resources.admit_artifact(payload_bytes).is_err() {
            return Ok(None);
        }
        let payload_reference =
            runtime_payload_reference_from_artifact(&self.publish_artifact_bytes(
                &payload.bytes,
                &payload.media_type,
                kind,
                Some(turn_id),
                operation_id,
                receipt_id,
                true,
            )?)?;
        Ok(Some(RuntimeOutput::Artifact {
            reference: payload_reference,
        }))
    }

    #[allow(clippy::too_many_arguments)]
    fn publish_artifact_bytes(
        &mut self,
        bytes: &[u8],
        media_type: &str,
        kind: RuntimeArtifactKind,
        turn_id: Option<&RuntimeTurnId>,
        operation_id: Option<&RuntimeOperationId>,
        receipt_id: Option<&ReceiptId>,
        retain_preview: bool,
    ) -> Result<RuntimeArtifactRef, RuntimeLoopError> {
        let publish = self
            .artifact
            .as_ref()
            .ok_or(RuntimeLoopError::UnsupportedMode)?
            .publish;
        let created_at_epoch_ms = self
            .clock
            .now_epoch_ms()
            .map_err(RuntimeLoopError::Dependency)?;
        let artifact_id = RuntimeArtifactId::from_raw(derived_id(
            "artifact",
            self.request.run_id.as_str(),
            self.artifact_references.len() as u64 + 1,
        ));
        let manifest = seal_runtime_artifact_manifest(RuntimeArtifactManifest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            artifact_id: artifact_id.clone(),
            kind,
            payload_sha256: sha256(bytes),
            byte_size: bytes.len() as u64,
            media_type: media_type.to_owned(),
            sensitivity: runtime_sensitivity(&self.request),
            retention: RuntimeEventRetention {
                kind: RuntimeEventRetentionKind::Session,
                expires_at_epoch_ms: None,
            },
            session_id: self.request.session_id.clone(),
            task_id: self.request.task.task_id.clone(),
            producer_run_id: self.request.run_id.clone(),
            producer_turn_id: turn_id.cloned(),
            producer_operation_id: operation_id.cloned(),
            receipt_id: receipt_id.cloned(),
            policy_id: self.request.policy_id.clone(),
            policy_sha256: self.request.policy_sha256.clone(),
            created_at_epoch_ms,
            integrity: RuntimeArtifactIntegrityState::Verified,
            preview: retain_preview
                .then(|| runtime_artifact_preview(bytes))
                .flatten(),
            manifest_sha256: ZERO_SHA256.to_owned(),
        })
        .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
        let expected =
            runtime_artifact_ref(&manifest).map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
        let reference = publish(&mut self.tool_boundary, manifest.clone(), bytes)
            .map_err(RuntimeLoopError::Dependency)?;
        verify_runtime_artifact_ref(&reference, &manifest)
            .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
        if reference != expected {
            return Err(RuntimeLoopError::InvalidBoundaryResult);
        }
        let payload_reference = runtime_payload_reference(&manifest)
            .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
        self.emit_at_with_payload(
            created_at_epoch_ms,
            RuntimeEventKind::ArtifactCreated {
                artifact_id,
                manifest_sha256: manifest.manifest_sha256,
            },
            turn_id,
            operation_id,
            Some(payload_reference.clone()),
        )?;
        self.artifact_references.push(reference.clone());
        Ok(reference)
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

fn append_runtime_event<T: RuntimeJournalPort>(
    port: &mut T,
    event: &RuntimeEvent,
) -> Result<(), RuntimePortFailure> {
    port.append_runtime_event(event)
}

fn flush_runtime_events<T: RuntimeJournalPort>(port: &mut T) -> Result<(), RuntimePortFailure> {
    port.flush_runtime_events()
}

fn load_runtime_events<T: RuntimeJournalPort>(
    port: &mut T,
    run_id: &agentmage_kernel_contracts::RuntimeRunId,
) -> Result<Vec<RuntimeEvent>, RuntimePortFailure> {
    port.load_runtime_events(run_id)
}

fn publish_runtime_artifact<T: RuntimeArtifactPort>(
    port: &mut T,
    manifest: RuntimeArtifactManifest,
    payload: &[u8],
) -> Result<RuntimeArtifactRef, RuntimePortFailure> {
    port.publish_runtime_artifact(manifest, payload)
}

fn commit_runtime_checkpoint<T: RuntimeCheckpointPort>(
    port: &mut T,
    input: RuntimeCheckpointCommit<'_>,
) -> Result<RuntimeCheckpointPublication, RuntimePortFailure> {
    port.commit_runtime_checkpoint(input)
}

fn load_runtime_checkpoint<T: RuntimeCheckpointPort>(
    port: &mut T,
    request: &RuntimeRunRequest,
) -> Result<Option<RuntimeResumeSnapshot>, RuntimePortFailure> {
    port.load_runtime_checkpoint(request)
}

fn runtime_payload_reference_from_artifact(
    reference: &RuntimeArtifactRef,
) -> Result<RuntimePayloadReference, RuntimeLoopError> {
    if reference.byte_size == 0
        || !valid_identifier(reference.artifact_id.as_str())
        || !valid_sha256(&reference.payload_sha256)
        || !valid_media_type(&reference.media_type)
    {
        return Err(RuntimeLoopError::InvalidBoundaryResult);
    }
    Ok(RuntimePayloadReference {
        artifact_id: reference.artifact_id.clone(),
        sha256: reference.payload_sha256.clone(),
        byte_size: reference.byte_size,
        media_type: reference.media_type.clone(),
    })
}

fn runtime_base_request_sha256(request: &RuntimeRunRequest) -> Result<String, RuntimeLoopError> {
    let mut base = request.clone();
    base.event_cursor = None;
    base.request_sha256 = ZERO_SHA256.to_owned();
    seal_runtime_run_request(base)
        .map(|sealed| sealed.request_sha256)
        .map_err(RuntimeLoopError::Contract)
}

fn runtime_event_cursor(event: &RuntimeEvent) -> RuntimeEventCursor {
    RuntimeEventCursor {
        run_id: event.run_id.clone(),
        event_id: event.event_id.clone(),
        sequence: event.sequence,
        event_sha256: event.event_sha256.clone(),
    }
}

fn validate_runtime_checkpoint_publication(
    request: &RuntimeRunRequest,
    continuation: &RuntimeContinuationState,
    continuation_artifact: &RuntimeArtifactRef,
    event_cursor: &RuntimeEventCursor,
    artifacts: &[RuntimeArtifactRef],
    publication: &RuntimeCheckpointPublication,
) -> Result<(), RuntimeLoopError> {
    verify_checkpoint(&publication.checkpoint)
        .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
    verify_runtime_continuation_state(continuation)
        .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
    verify_runtime_resume_binding(&publication.binding)
        .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
    let checkpoint = &publication.checkpoint;
    let binding = &publication.binding;
    let mut expected_artifacts = continuation.artifacts.clone();
    expected_artifacts.push(continuation_artifact.clone());
    expected_artifacts
        .sort_by(|left, right| left.artifact_id.as_str().cmp(right.artifact_id.as_str()));
    let mut evidence_ids = continuation
        .evidence
        .iter()
        .map(|evidence| evidence.evidence_id.clone())
        .collect::<Vec<_>>();
    evidence_ids.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    if continuation.request_sha256 != runtime_base_request_sha256(request)?
        || continuation_artifact.media_type != RUNTIME_CONTINUATION_MEDIA_TYPE
        || expected_artifacts != artifacts
        || binding.artifacts != artifacts
        || checkpoint.ephemeral
        || checkpoint.session_id != request.session_id
        || checkpoint.task_id != request.task.task_id
        || checkpoint.objective_sha256 != sha256(request.task.objective.as_bytes())
        || request.work_packet.plan_id.as_ref() != Some(&checkpoint.plan_id)
        || checkpoint.plan_revision != request.work_packet.revision
        || checkpoint.workspace_id != request.workspace_id
        || checkpoint.workspace_state_sha256 != request.workspace_snapshot_sha256
        || checkpoint.repository_snapshot_id != request.repository_snapshot_id
        || checkpoint.repository_map_sha256 != request.repository_snapshot_sha256
        || checkpoint.permission_profile_id != request.policy_id.as_str()
        || checkpoint.permission_profile_sha256 != request.policy_sha256
        || checkpoint.policy_id != request.policy_id
        || checkpoint.policy_sha256 != request.policy_sha256
        || checkpoint.model_profile_id != request.model_profile.profile_id
        || checkpoint.model_manifest_sha256 != request.model_profile.manifest_sha256
        || checkpoint.model_runtime_sha256 != request.model_profile.runtime.runtime_sha256
        || checkpoint.evidence_ids != evidence_ids
        || checkpoint.context_packet_sha256 != continuation.continuation_sha256
        || checkpoint.action_id.is_some()
        || checkpoint.action_state.is_some()
        || checkpoint.consumed_grant_id.is_some()
        || checkpoint.receipt_id.is_some()
        || checkpoint.receipt_sha256.is_some()
        || binding.checkpoint_id != checkpoint.checkpoint_id
        || binding.checkpoint_sha256 != checkpoint.checkpoint_sha256
        || binding.session_id != request.session_id
        || binding.task_id != request.task.task_id
        || binding.run_id != request.run_id
        || &binding.event_cursor != event_cursor
    {
        return Err(RuntimeLoopError::InvalidBoundaryResult);
    }
    Ok(())
}

fn validate_runtime_resume_snapshot(
    request: &RuntimeRunRequest,
    requested_cursor: &RuntimeEventCursor,
    events: &[RuntimeEvent],
    snapshot: &RuntimeResumeSnapshot,
) -> Result<(), RuntimeLoopError> {
    if events.is_empty() || events.len() > request.limits.max_events as usize {
        return Err(RuntimeLoopError::InvalidBoundaryResult);
    }
    let last = events
        .last()
        .ok_or(RuntimeLoopError::InvalidBoundaryResult)?;
    if &runtime_event_cursor(last) != requested_cursor {
        return Err(RuntimeLoopError::UnsupportedMode);
    }
    validate_runtime_checkpoint_publication(
        request,
        &snapshot.continuation,
        &snapshot.continuation_artifact,
        &snapshot.binding.event_cursor,
        &snapshot.binding.artifacts,
        &RuntimeCheckpointPublication {
            checkpoint: snapshot.checkpoint.clone(),
            binding: snapshot.binding.clone(),
        },
    )?;
    let RuntimeEventKind::RunStarted { request_sha256 } = &events[0].kind else {
        return Err(RuntimeLoopError::InvalidBoundaryResult);
    };
    if request_sha256 != &snapshot.continuation.request_sha256 {
        return Err(RuntimeLoopError::InvalidBoundaryResult);
    }
    let continuation_index = usize::try_from(snapshot.continuation.event_cursor.sequence)
        .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
    let binding_index = usize::try_from(snapshot.binding.event_cursor.sequence)
        .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
    let expected_binding_index = continuation_index
        .checked_add(1)
        .ok_or(RuntimeLoopError::InvalidBoundaryResult)?;
    if binding_index != expected_binding_index
        || continuation_index >= events.len()
        || binding_index >= events.len()
        || runtime_event_cursor(&events[continuation_index]) != snapshot.continuation.event_cursor
        || runtime_event_cursor(&events[binding_index]) != snapshot.binding.event_cursor
        || !matches!(
            events[continuation_index].kind,
            RuntimeEventKind::TurnCompleted { .. }
        )
    {
        return Err(RuntimeLoopError::InvalidBoundaryResult);
    }
    let RuntimeEventKind::ArtifactCreated {
        artifact_id,
        manifest_sha256,
    } = &events[binding_index].kind
    else {
        return Err(RuntimeLoopError::InvalidBoundaryResult);
    };
    let expected_payload =
        runtime_payload_reference_from_artifact(&snapshot.continuation_artifact)?;
    if artifact_id != &snapshot.continuation_artifact.artifact_id
        || manifest_sha256 != &snapshot.continuation_artifact.manifest_sha256
        || events[binding_index].turn_id.is_some()
        || events[binding_index].operation_id.is_some()
        || events[binding_index].payload_reference.as_ref() != Some(&expected_payload)
    {
        return Err(RuntimeLoopError::InvalidBoundaryResult);
    }
    match events.len().saturating_sub(binding_index) {
        1 => {}
        2 => {
            let RuntimeEventKind::CheckpointCommitted {
                checkpoint_id,
                checkpoint_sha256,
            } = &events[binding_index + 1].kind
            else {
                return Err(RuntimeLoopError::InvalidBoundaryResult);
            };
            if checkpoint_id != &snapshot.checkpoint.checkpoint_id
                || checkpoint_sha256 != &snapshot.checkpoint.checkpoint_sha256
            {
                return Err(RuntimeLoopError::InvalidBoundaryResult);
            }
        }
        _ => return Err(RuntimeLoopError::InvalidBoundaryResult),
    }
    Ok(())
}

fn tool_artifact_kind(definition: &ToolDefinition, media_type: &str) -> RuntimeArtifactKind {
    if definition.tool_id.as_str().contains("validation") {
        RuntimeArtifactKind::TestLog
    } else {
        match definition.required_grant.operation.operation() {
            GrantOperation::CommandExecute => RuntimeArtifactKind::StandardOutput,
            GrantOperation::WorkspaceWrite if media_type == "text/x-diff" => {
                RuntimeArtifactKind::Patch
            }
            GrantOperation::WorkspaceWrite => RuntimeArtifactKind::Report,
            _ => RuntimeArtifactKind::Report,
        }
    }
}

fn runtime_artifact_preview(bytes: &[u8]) -> Option<RuntimeArtifactPreview> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut end = text.len().min(MAX_RUNTIME_ARTIFACT_PREVIEW_BYTES);
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    if end == 0 {
        return None;
    }
    let preview = &text[..end];
    Some(RuntimeArtifactPreview {
        text: preview.to_owned(),
        byte_size: end as u32,
        truncated: end < bytes.len(),
        sha256: sha256(preview.as_bytes()),
    })
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

/// Derives the exact one-based action identity the coordinator will assign to a tool call.
///
/// Trusted session-policy composition can enumerate the request's bounded tool-call budget before
/// model execution without granting the model any control over an action identity.
#[must_use]
pub fn runtime_action_id(
    run_id: &agentmage_kernel_contracts::RuntimeRunId,
    sequence: u32,
) -> ActionId {
    ActionId::from_raw(derived_id("action", run_id.as_str(), u64::from(sequence)))
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
        || result.resources.output_tokens > request.max_output_tokens
        || result.resources.elapsed_ms > request.timeout_ms
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
