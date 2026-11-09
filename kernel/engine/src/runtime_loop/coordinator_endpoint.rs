// SPDX-License-Identifier: Apache-2.0
//! Endpoint model dispatch inside the existing coordinator and its authority boundaries.

use super::endpoint::{
    RuntimeEndpointContextPacket, RuntimeEndpointModelPhase, RuntimeEndpointModelPort,
    RuntimeEndpointPhaseFailure, RuntimeEndpointTurnResult,
};
use super::*;
use agentmage_kernel_contracts::{
    EndpointModelFinish, EndpointModelRequest, EndpointModelResult, RuntimeEndpointRunRequest,
    SchemaReference,
};

/// Current coordinator observations used to prepare an approved endpoint request.
/// Preparing context grants no disclosure or tool authority.
pub struct RuntimeEndpointPreparationInput<'a> {
    /// Original sealed endpoint run, never a substituted native request.
    pub request: &'a RuntimeEndpointRunRequest,
    /// Current turn ordinal under the coordinator's exclusive mutable owner.
    pub turn: u32,
    /// Completed calls actually observed by this coordinator, including a
    /// failed validation explicitly admitted for feedback by the trusted tool.
    /// Inspect each corresponding result's outcome; completion is not success.
    pub completed_tool_calls: &'a [ToolCall],
    /// Actual tool results, separate from untrusted model text.
    pub tool_results: &'a [ToolResult],
    /// Current grounded evidence.
    pub evidence: &'a [EvidenceReference],
    /// Pre-effect refusals; these are neither receipts nor successful results.
    pub rejected_tool_calls: &'a [RuntimeToolRejection],
}

trait PreparedDispatch {
    fn request(&self) -> &EndpointModelRequest;
    fn dispatch(
        self: Box<Self>,
        control: &dyn ModelOperationControl,
        text: &mut dyn FnMut(&str),
    ) -> Result<EndpointModelResult, RuntimePortFailure>;
}
struct Prepared<P>(P);
impl<P: RuntimeEndpointModelPort<Error = RuntimePortFailure>> PreparedDispatch for Prepared<P> {
    fn request(&self) -> &EndpointModelRequest {
        self.0.request()
    }
    fn dispatch(
        self: Box<Self>,
        control: &dyn ModelOperationControl,
        text: &mut dyn FnMut(&str),
    ) -> Result<EndpointModelResult, RuntimePortFailure> {
        self.0.dispatch(control, text)
    }
}
impl RuntimeEndpointModelPort for Box<dyn PreparedDispatch + '_> {
    type Error = RuntimePortFailure;
    fn request(&self) -> &EndpointModelRequest {
        self.as_ref().request()
    }
    fn dispatch(
        self,
        control: &dyn ModelOperationControl,
        text: &mut dyn FnMut(&str),
    ) -> Result<EndpointModelResult, Self::Error> {
        PreparedDispatch::dispatch(self, control, text)
    }
}

/// One immutable credential-owning request and independently admitted original context.
/// The transport retains its credentials and disclosure approval; the kernel
/// never constructs or reads either. The trusted context owner fixes the
/// completion payload schema before dispatch.
pub struct RuntimePreparedEndpointTurn<'a> {
    context: RuntimeEndpointContextPacket,
    port: Box<dyn PreparedDispatch + 'a>,
    completion_schema: SchemaReference,
    tool_preparation: Option<Box<ToolPreparation<'a>>>,
    completion_only: bool,
    completion_check: Option<Box<CompletionCheck<'a>>>,
}

/// Original coordinator identities for interpreting a completed answer. This
/// input carries no permission, grant or executable authority.
pub struct RuntimeEndpointToolPreparationInput<'a> {
    /// Complete original sealed request.
    pub request: &'a RuntimeEndpointRunRequest,
    /// Fully received, accounted, normally terminated untrusted model text.
    pub text: &'a str,
    /// Current turn identity.
    pub turn_id: &'a RuntimeTurnId,
    /// Next operation identity, fixed by the coordinator.
    pub operation_id: &'a RuntimeOperationId,
    /// Next call identity, fixed by the coordinator.
    pub tool_call_id: &'a ToolCallId,
    /// Original run correlation identity.
    pub correlation_id: &'a CorrelationId,
    /// Next action identity, fixed by the coordinator.
    pub action_id: &'a ActionId,
}

/// Inert proposal produced by trusted interpretation. The registry, cumulative
/// budgets and original tool permission boundary must still admit it.
pub struct RuntimeEndpointToolProposal {
    /// Registered model-proposable tool.
    pub tool_id: ToolId,
    /// Exact registered version.
    pub tool_version: String,
    /// Untrusted argument values; the registry owns their payload schema.
    pub arguments: serde_json::Value,
}

type ToolPreparation<'a> = dyn FnOnce(
        RuntimeEndpointToolPreparationInput<'_>,
        &dyn ModelOperationControl,
    ) -> Result<RuntimeEndpointToolProposal, RuntimePortFailure>
    + 'a;
type CompletionCheck<'a> =
    dyn FnOnce(&dyn ModelOperationControl) -> Result<(), RuntimePortFailure> + 'a;
impl<'a> RuntimePreparedEndpointTurn<'a> {
    /// Retains a single-use prepared transport, with no dispatch or authority grant.
    pub fn new<P: RuntimeEndpointModelPort<Error = RuntimePortFailure> + 'a>(
        context: RuntimeEndpointContextPacket,
        port: P,
        completion_schema: SchemaReference,
    ) -> Self {
        Self {
            context,
            port: Box::new(Prepared(port)),
            completion_schema,
            tool_preparation: None,
            completion_only: false,
            completion_check: None,
        }
    }

    /// Narrow this turn to a completion candidate for the original verifier.
    /// Tool-shaped answers terminate with a recorded refusal before admission.
    /// No tool interpreter is invoked, even if one was attached to this turn.
    /// This process-local restriction cannot be selected by model or wire data.
    #[must_use]
    pub fn completion_only(mut self) -> Self {
        self.completion_only = true;
        self
    }

    /// Recheck process-held completion prerequisites after a normally completed,
    /// accounted model response and before calling the independent verifier.
    /// This single-use trusted callback grants no authority and cannot accept a
    /// task. It must only inspect previously consented state under the original
    /// control. Tool proposals, invalid claim envelopes, malformed/duplicate-key
    /// JSON and incomplete responses never invoke it. The verifier still owns
    /// schema semantics. No model or wire field can install or skip this check.
    #[must_use]
    pub fn with_completion_check<F>(mut self, check: F) -> Self
    where
        F: FnOnce(&dyn ModelOperationControl) -> Result<(), RuntimePortFailure> + 'a,
    {
        self.completion_check = Some(Box::new(check));
        self
    }

    /// Opt into trusted interpretation of a normally completed answer. Called
    /// once under the ORIGINAL borrowed control, after model accounting and
    /// before tool admission. Refusal is terminal; no partial answer or model
    /// failure enters this callback. It may prepare an inert preview under prior
    /// source-read consent, but MUST NOT execute effects or confer authority.
    /// No wire/model field can select or construct this process-local callback.
    /// A completion-only restriction takes precedence over this callback.
    #[must_use]
    pub fn with_tool_preparation<F>(mut self, prepare: F) -> Self
    where
        F: FnOnce(
                RuntimeEndpointToolPreparationInput<'_>,
                &dyn ModelOperationControl,
            ) -> Result<RuntimeEndpointToolProposal, RuntimePortFailure>
            + 'a,
    {
        self.tool_preparation = Some(Box::new(prepare));
        self
    }
}

/// Context and credential owner for each endpoint turn. The same borrowed
/// control must bound preparation and later dispatch. A prepared port is owned
/// and single-use, so no request can be replaced between framing and dispatch.
pub trait RuntimeEndpointTurnPort {
    /// The actual selected transport/model, without native provenance.
    fn model_binding(&self) -> RuntimeModelBinding<'_>;
    /// Prepare original messages and the exact disclosure-approved request.
    fn prepare<'a>(
        &mut self,
        input: RuntimeEndpointPreparationInput<'_>,
        control: &dyn ModelOperationControl,
    ) -> Result<RuntimePreparedEndpointTurn<'a>, RuntimePortFailure>
    where
        Self: 'a;
}

/// Explicit endpoint family adapter. It contains no native profile or fallback.
pub struct RuntimeEndpointDriver<P>(pub P);

/// Family boundary used by the reusable coordinator. Existing native ports
/// retain their full exact-profile contract through the blanket implementation.
/// Endpoint implementations opt in through `RuntimeEndpointDriver`.
pub trait RuntimeCoordinatorModelPort {
    /// Identity available at the actual model boundary.
    fn model_binding(&self) -> RuntimeModelBinding<'_>;
    /// Original native port, or refusal without entering one.
    fn native(&self) -> Result<&dyn RuntimeModelPort, RuntimeLoopError>;
    /// Original mutable native port, or refusal without entering one.
    fn native_mut(&mut self) -> Result<&mut dyn RuntimeModelPort, RuntimeLoopError>;
    /// Endpoint request preparation; native ports refuse before context dispatch.
    fn prepare_endpoint<'a>(
        &mut self,
        _input: RuntimeEndpointPreparationInput<'_>,
        _control: &dyn ModelOperationControl,
    ) -> Result<RuntimePreparedEndpointTurn<'a>, RuntimePortFailure>
    where
        Self: 'a,
    {
        Err(RuntimePortFailure::Invalid)
    }
}
impl<M: RuntimeModelPort> RuntimeCoordinatorModelPort for M {
    fn model_binding(&self) -> RuntimeModelBinding<'_> {
        RuntimeModelPort::model_binding(self)
    }
    fn native(&self) -> Result<&dyn RuntimeModelPort, RuntimeLoopError> {
        Ok(self)
    }
    fn native_mut(&mut self) -> Result<&mut dyn RuntimeModelPort, RuntimeLoopError> {
        Ok(self)
    }
}
impl<P: RuntimeEndpointTurnPort> RuntimeCoordinatorModelPort for RuntimeEndpointDriver<P> {
    fn model_binding(&self) -> RuntimeModelBinding<'_> {
        self.0.model_binding()
    }
    fn native(&self) -> Result<&dyn RuntimeModelPort, RuntimeLoopError> {
        Err(RuntimeLoopError::ModelBinding)
    }
    fn native_mut(&mut self) -> Result<&mut dyn RuntimeModelPort, RuntimeLoopError> {
        Err(RuntimeLoopError::ModelBinding)
    }
    fn prepare_endpoint<'a>(
        &mut self,
        input: RuntimeEndpointPreparationInput<'_>,
        control: &dyn ModelOperationControl,
    ) -> Result<RuntimePreparedEndpointTurn<'a>, RuntimePortFailure>
    where
        Self: 'a,
    {
        self.0.prepare(input, control)
    }
}

/// Unavailable native context for endpoint composition; never invents a native packet.
pub struct RuntimeNoNativeContext;
impl RuntimeContextPort for RuntimeNoNativeContext {
    fn build_context(
        &mut self,
        _: &RuntimeRunRequest,
        _: ContextPacketId,
        _: u32,
        _: &[ToolCall],
        _: &[ToolResult],
        _: &[EvidenceReference],
    ) -> Result<ModelContextPacket, RuntimePortFailure> {
        Err(RuntimePortFailure::Invalid)
    }
}

pub(super) struct OwnedEndpointClaim {
    turn: RuntimeEndpointTurnResult,
    text: String,
    schema: SchemaReference,
}
impl OwnedEndpointClaim {
    pub(super) fn claim(
        &self,
        scope: RuntimeRunScope<'_>,
    ) -> Result<RuntimeCompletionClaim<'_>, RuntimePortFailure> {
        RuntimeCompletionClaim::endpoint(scope, &self.turn, &self.text, self.schema.clone())
    }
}

impl<P: RuntimeEndpointTurnPort, T: RuntimeToolBoundary, V: RuntimeVerifierPort, C: RuntimeClock>
    ReusableRuntimeCoordinator<RuntimeEndpointDriver<P>, RuntimeNoNativeContext, T, V, C>
{
    /// Composes one read-only endpoint run through the existing coordinator.
    /// The owner must keep one instance for this run; no path or wire metadata
    /// restores its clock, accepted turns or authority after destruction.
    pub fn new_endpoint(
        request: RuntimeEndpointRunRequest,
        port: P,
        registry: ToolRegistry,
        tool_boundary: T,
        verifier: V,
        clock: C,
    ) -> Result<Self, RuntimeLoopError> {
        Self::compose(
            request.into(),
            RuntimeEndpointDriver(port),
            RuntimeNoNativeContext,
            registry,
            tool_boundary,
            verifier,
            clock,
            None,
            None,
            None,
            None,
            None,
        )
    }

    /// Consumes a terminal endpoint coordinator and returns its ORIGINAL tool
    /// owner for separately admitted later work. No run, budget, approval or
    /// pending decision is resumed. Export events before consuming this owner.
    /// Running, suspended, uncertain or reconciliation-required runs refuse and
    /// return the same coordinator unchanged. No dependency is entered here.
    pub fn try_into_endpoint_tool_boundary(self: Box<Self>) -> Result<T, Box<Self>> {
        let terminal = self.endpoint_outcome.as_ref().is_some_and(|endpoint| {
            self.outcome.as_ref() == Some(&endpoint.outcome)
                && endpoint.outcome.state != AgentStateKind::Uncertain
                && self.events.last().is_some_and(|event| {
                    matches!(&event.kind, RuntimeEventKind::RunTerminal { state, outcome_sha256 }
                        if *state == endpoint.outcome.state
                            && outcome_sha256 == &endpoint.outcome.outcome_sha256)
                })
        });
        if !terminal
            || self.pending.is_some()
            || self.suspended
            || self.correctness_reconciliation_required
        {
            return Err(self);
        }
        Ok(self.tool_boundary)
    }
}

/// Endpoint boundary retaining its complete terminal envelope. Native boundary
/// types and their concrete layout remain unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeEndpointCoordinatorStep {
    /// The existing permission challenge, awaiting an exact user decision.
    AwaitingApproval {
        /// Immutable challenge to display and bind into the response.
        challenge: RuntimeApprovalChallenge,
    },
    /// Whole endpoint outcome after verifier checks and terminal publication.
    Complete {
        /// Distinct provenance; never a native model assignment.
        outcome: crate::runtime_coordinator::RuntimeEndpointOutcome,
    },
}

// This wire form contains only an inert tool request. The coordinator derives
// its call identity; model text cannot select a run, grant, policy or receipt.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct EndpointToolProposal {
    tool_call: EndpointToolArguments,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct EndpointToolArguments {
    tool_id: ToolId,
    tool_version: String,
    arguments: serde_json::Value,
}

impl<
    M: RuntimeCoordinatorModelPort,
    X: RuntimeContextPort,
    T: RuntimeToolBoundary,
    V: RuntimeVerifierPort,
    C: RuntimeClock,
> ReusableRuntimeCoordinator<M, X, T, V, C>
{
    /// Advances the SAME approval/turn/tool/verifier loop with an endpoint
    /// terminal envelope. Native APIs refuse endpoint instances so they cannot
    /// accidentally discard or mislabel endpoint answer provenance.
    pub fn run_endpoint_until_boundary(
        &mut self,
        response: Option<&RuntimeApprovalResponse>,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<RuntimeEndpointCoordinatorStep, RuntimeLoopError> {
        if self.request.endpoint().is_none() {
            return Err(RuntimeLoopError::UnsupportedMode);
        }
        match self.run_until_boundary_impl(response, cancellation, None)? {
            RuntimeSuspendableStep::Boundary(RuntimeCoordinatorStep::AwaitingApproval {
                challenge,
            }) => Ok(RuntimeEndpointCoordinatorStep::AwaitingApproval { challenge }),
            RuntimeSuspendableStep::Boundary(RuntimeCoordinatorStep::Complete { outcome }) => {
                let Some(endpoint) = self.endpoint_outcome.as_ref() else {
                    self.correctness_reconciliation_required = true;
                    return Err(RuntimeLoopError::InvalidBoundaryResult);
                };
                if endpoint.outcome != outcome {
                    self.correctness_reconciliation_required = true;
                    return Err(RuntimeLoopError::InvalidBoundaryResult);
                }
                Ok(RuntimeEndpointCoordinatorStep::Complete {
                    outcome: endpoint.clone(),
                })
            }
            RuntimeSuspendableStep::Suspended { .. } => Err(RuntimeLoopError::UnsupportedMode),
        }
    }

    pub(super) fn run_endpoint_turn(
        &mut self,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<(), RuntimeLoopError> {
        let request = self
            .request
            .endpoint()
            .ok_or(RuntimeLoopError::UnsupportedMode)?
            .clone();
        // Full turn closure, verifier and terminal space is reserved before any
        // external preparation or dispatch. Tool admission has its own reserve.
        if self.remaining_events() < 7 {
            self.transition_terminal(AgentStateKind::Exhausted)?;
            return self.finish_terminal(
                AgentStateKind::Exhausted,
                vec!["runtime.budget.exhausted".into()],
                None,
            );
        }
        if self
            .resources
            .consume(BudgetResource::PlanSteps, 1)
            .is_err()
        {
            self.transition_terminal(AgentStateKind::Exhausted)?;
            return self.finish_terminal(
                AgentStateKind::Exhausted,
                vec!["runtime.budget.exhausted".into()],
                None,
            );
        }
        self.turn_count += 1;
        self.context_refresh_count += 1;
        let turn_id = RuntimeTurnId::from_raw(derived_id(
            "turn",
            request.run_id.as_str(),
            u64::from(self.turn_count),
        ));
        self.active_turn = Some(turn_id.clone());
        self.emit(RuntimeEventKind::TurnStarted, Some(&turn_id), None)?;
        let (prepared, stop) = {
            let control = RuntimePhaseControl::for_run_limits(
                &mut self.clock,
                &request.task.task_id,
                request.limits.max_elapsed_ms,
                &self.correlation_id,
                self.started_at_epoch_ms
                    .ok_or(RuntimeLoopError::InvalidBoundaryResult)?,
                self.events
                    .last()
                    .map_or(0, |event| event.occurred_at_epoch_ms),
                &self.last_phase_observed_at,
                cancellation,
            );
            let prepared = control.check().and_then(|_| {
                self.model.prepare_endpoint(
                    RuntimeEndpointPreparationInput {
                        request: &request,
                        turn: self.turn_count,
                        completed_tool_calls: &self.completed_tool_calls,
                        tool_results: &self.tool_results,
                        evidence: &self.evidence,
                        rejected_tool_calls: &self.rejected_tool_calls,
                    },
                    &control,
                )
            });
            if prepared.is_ok() {
                let _ = control.check();
            }
            (prepared, control.into_stop())
        };
        self.latch_phase_cancellation(stop.as_ref());
        if let Some(stop) = stop {
            if let Err(error) = &prepared
                && *error != stop.port_failure()
            {
                return self.finish_model_failure(&turn_id, *error);
            }
            return self.finish_phase_stop(stop, false);
        }
        let prepared = match prepared {
            Ok(value) => value,
            Err(error) => return self.finish_model_failure(&turn_id, error),
        };
        let RuntimePreparedEndpointTurn {
            context,
            port,
            completion_schema,
            tool_preparation,
            completion_only,
            completion_check,
        } = prepared;
        let descriptor = port.request().clone();
        let mut phase = match RuntimeEndpointModelPhase::for_run(&request, &descriptor, context) {
            Ok(phase) => phase,
            Err(error) => return self.finish_model_failure(&turn_id, error),
        };
        let mut admitted = self.resources.clone();
        for (resource, amount) in [
            (
                BudgetResource::InputBytes,
                descriptor.body().request_body_bytes as u64,
            ),
            (BudgetResource::ModelCalls, 1),
        ] {
            if admitted.consume(resource, amount).is_err() {
                return self.finish_budget_exhaustion(&turn_id);
            }
        }
        let mut envelope = admitted.clone();
        for (resource, amount) in [
            (
                BudgetResource::OutputBytes,
                descriptor.limits().response_text_limit_bytes as u64,
            ),
            (
                BudgetResource::ElapsedMilliseconds,
                u64::try_from(
                    descriptor
                        .limits()
                        .total_timeout
                        .as_nanos()
                        .div_ceil(1_000_000),
                )
                .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?,
            ),
        ] {
            if envelope.consume(resource, amount).is_err() {
                return self.finish_budget_exhaustion(&turn_id);
            }
        }
        // The static phase bound is not a renewed deadline. The original clock
        // remains borrowed during dispatch and all synchronous text callbacks.
        if self.stop_before_phase(cancellation, false)? {
            return Ok(());
        }
        let model_run_id = ModelRunId::from_raw(derived_id(
            "model-run",
            request.run_id.as_str(),
            u64::from(self.turn_count),
        ));
        self.emit(
            RuntimeEventKind::ModelRequested {
                model_run_id: model_run_id.clone(),
                request_sha256: observation_sha256(&descriptor)?,
            },
            Some(&turn_id),
            None,
        )?;
        // Event admission above changes this SAME ledger; spend only the actual
        // model attempt/input below, without replacing its event observations.
        let before_attempt = self.resources.clone();
        self.resources
            .consume(
                BudgetResource::InputBytes,
                descriptor.body().request_body_bytes as u64,
            )
            .and_then(|()| self.resources.consume(BudgetResource::ModelCalls, 1))
            .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
        self.model_call_count += 1;
        let mut text = String::new();
        let (accepted, stop) = {
            let control = RuntimePhaseControl::for_run_limits(
                &mut self.clock,
                &request.task.task_id,
                request.limits.max_elapsed_ms,
                &self.correlation_id,
                self.started_at_epoch_ms
                    .ok_or(RuntimeLoopError::InvalidBoundaryResult)?,
                self.events
                    .last()
                    .map_or(0, |event| event.occurred_at_epoch_ms),
                &self.last_phase_observed_at,
                cancellation,
            );
            let result = RuntimeEndpointTurnResult::execute_phase(
                &mut phase,
                turn_id.clone(),
                &control,
                port,
                |part| text.push_str(part),
            );
            (result, control.into_stop())
        };
        if phase.usage().dispatch_attempts == 0 {
            self.resources = before_attempt;
            self.model_call_count -= 1;
        }
        self.latch_phase_cancellation(stop.as_ref());
        let accepted = match accepted {
            Ok(value) => value,
            Err(error) => {
                self.emit(
                    RuntimeEventKind::ModelFailed {
                        model_run_id,
                        failure_code: "runtime.model.incomplete.unknown".into(),
                    },
                    Some(&turn_id),
                    None,
                )?;
                if let Some(stop) = stop {
                    if let RuntimeEndpointPhaseFailure::Dependency(error) = &error
                        && *error != stop.port_failure()
                    {
                        return self.finish_model_failure(&turn_id, *error);
                    }
                    return self.finish_phase_stop(stop, false);
                }
                let failure = match error {
                    RuntimeEndpointPhaseFailure::Dependency(error) => error,
                    RuntimeEndpointPhaseFailure::ResourceExhausted => {
                        RuntimePortFailure::ResourceExhausted
                    }
                    RuntimeEndpointPhaseFailure::Stopped(_) => RuntimePortFailure::Cancelled,
                    RuntimeEndpointPhaseFailure::InvalidResult => RuntimePortFailure::Invalid,
                };
                return self.finish_model_failure(&turn_id, failure);
            }
        };
        for (resource, amount) in [
            (
                BudgetResource::OutputBytes,
                accepted.result().response_text_bytes as u64,
            ),
            (
                BudgetResource::ElapsedMilliseconds,
                phase
                    .usage()
                    .completed_dispatch_elapsed_ms
                    .ok_or(RuntimeLoopError::InvalidBoundaryResult)?,
            ),
        ] {
            self.resources
                .consume(resource, amount)
                .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
        }
        self.emit(
            RuntimeEventKind::ModelCompleted {
                model_run_id,
                result_sha256: observation_sha256(accepted.result())?,
            },
            Some(&turn_id),
            None,
        )?;
        if accepted.result().finish != EndpointModelFinish::Stop {
            return self.finish_model_failure(&turn_id, RuntimePortFailure::Invalid);
        }
        if self.stop_before_phase(cancellation, false)? {
            return Ok(());
        }
        self.state
            .transition(AgentStateKind::Proposal)
            .map_err(|_| RuntimeLoopError::State)?;
        self.state
            .transition(AgentStateKind::Validation)
            .map_err(|_| RuntimeLoopError::State)?;
        if completion_only
            && (serde_json::from_str::<serde_json::Value>(&text)
                .is_ok_and(|value| value.get("tool_call").is_some())
                || serde_json::from_str::<EndpointToolProposal>(&text).is_ok())
        {
            return self.finish_refused_proposal(&turn_id, "runtime.proposal.completion-only");
        }
        let tool_preparation = if completion_only {
            None
        } else {
            tool_preparation
        };
        let interpreted = if let Some(prepare) = tool_preparation {
            let closing_sha256 = observation_sha256(&accepted)?;
            if self.tool_budget_exhausted(&turn_id, &closing_sha256)? {
                return Ok(());
            }
            let operation_id = RuntimeOperationId::from_raw(derived_id(
                "operation",
                request.run_id.as_str(),
                u64::from(self.tool_call_count) + 1,
            ));
            let tool_call_id = ToolCallId::from_raw(derived_id(
                "endpoint-call",
                request.run_id.as_str(),
                u64::from(self.turn_count),
            ));
            let action_id = runtime_action_id(&request.run_id, self.tool_call_count + 1);
            let (proposal, stop) = {
                let control = RuntimePhaseControl::for_run_limits(
                    &mut self.clock,
                    &request.task.task_id,
                    request.limits.max_elapsed_ms,
                    &self.correlation_id,
                    self.started_at_epoch_ms
                        .ok_or(RuntimeLoopError::InvalidBoundaryResult)?,
                    self.events.last().map_or(0, |e| e.occurred_at_epoch_ms),
                    &self.last_phase_observed_at,
                    cancellation,
                );
                let proposal = control.check().and_then(|_| {
                    prepare(
                        RuntimeEndpointToolPreparationInput {
                            request: &request,
                            text: &text,
                            turn_id: &turn_id,
                            operation_id: &operation_id,
                            tool_call_id: &tool_call_id,
                            correlation_id: &self.correlation_id,
                            action_id: &action_id,
                        },
                        &control,
                    )
                });
                if proposal.is_ok() {
                    let _ = control.check();
                }
                (proposal, control.into_stop())
            };
            self.latch_phase_cancellation(stop.as_ref());
            if let Some(stop) = stop {
                if let Err(error) = &proposal
                    && *error != stop.port_failure()
                {
                    return self.finish_model_failure(&turn_id, *error);
                }
                return self.finish_phase_stop(stop, false);
            }
            match proposal {
                Ok(proposal) => Some(EndpointToolProposal {
                    tool_call: EndpointToolArguments {
                        tool_id: proposal.tool_id,
                        tool_version: proposal.tool_version,
                        arguments: proposal.arguments,
                    },
                }),
                Err(_) => {
                    return self
                        .finish_refused_proposal(&turn_id, "runtime.proposal.preparation-refused");
                }
            }
        } else {
            None
        };
        let was_interpreted = interpreted.is_some();
        // Completion-only turns never enter tool admission, including through
        // serde's positional struct encoding. Keep the fallback closed even if
        // the proposal parser later accepts another representation.
        if let Some(tool) = interpreted.or_else(|| {
            if completion_only {
                None
            } else {
                serde_json::from_str::<EndpointToolProposal>(&text).ok()
            }
        }) {
            if !was_interpreted && !crate::tooling::is_closed_json(text.as_bytes()) {
                return self
                    .finish_refused_proposal(&turn_id, "runtime.proposal.arguments-invalid");
            }
            let closing_sha256 = observation_sha256(&accepted)?;
            if self.tool_budget_exhausted(&turn_id, &closing_sha256)? {
                return Ok(());
            }
            if !self
                .registry
                .is_model_proposable(&tool.tool_call.tool_id, &tool.tool_call.tool_version)
            {
                return self.finish_refused_proposal(&turn_id, "runtime.proposal.tool-not-offered");
            }
            // Only argument values are model-authored. The exact registry owns
            // the schema and the coordinator owns the encoded bytes and digest.
            // Model errors never reach request_tool's trusted-envelope fault.
            let definition = self
                .registry
                .get_tool(&tool.tool_call.tool_id, &tool.tool_call.tool_version)
                .ok_or(RuntimeLoopError::InvalidBoundaryResult)?;
            let bytes = serde_json::to_vec(&tool.tool_call.arguments)
                .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
            let arguments = ContractPayload {
                schema: definition.input_schema.clone(),
                media_type: "application/json".into(),
                sha256: sha256(&bytes),
                bytes,
            };
            let tool_call_id = ToolCallId::from_raw(derived_id(
                "endpoint-call",
                request.run_id.as_str(),
                u64::from(self.turn_count),
            ));
            let checked = ToolCall {
                schema_version: CONTRACT_SCHEMA_VERSION,
                tool_call_id: tool_call_id.clone(),
                correlation_id: self.correlation_id.clone(),
                action_id: runtime_action_id(&request.run_id, self.tool_call_count + 1),
                tool_id: tool.tool_call.tool_id.clone(),
                tool_version: tool.tool_call.tool_version.clone(),
                arguments: arguments.clone(),
            };
            if self.registry.validate_arguments(&checked).is_err() {
                return self
                    .finish_refused_proposal(&turn_id, "runtime.proposal.arguments-invalid");
            }
            return self.request_tool(
                turn_id,
                RequestedToolCall {
                    tool_call_id,
                    tool_id: tool.tool_call.tool_id,
                    tool_version: tool.tool_call.tool_version,
                    arguments,
                    origin: ProposalOrigin::Model,
                    closing_sha256,
                },
                cancellation,
            );
        }
        if serde_json::from_str::<serde_json::Value>(&text)
            .is_ok_and(|value| value.get("tool_call").is_some())
        {
            return self.finish_refused_proposal(&turn_id, "runtime.proposal.arguments-invalid");
        }
        // Checked completion turns require one unambiguous JSON value before
        // inspecting prerequisites. The verifier still owns schema semantics.
        // Keep legacy clients' completion handling unchanged when no check is
        // installed; model text cannot select or disable this restriction.
        if completion_check.is_some() && !crate::tooling::is_closed_json(text.as_bytes()) {
            return self.finish_invalid_proposal(&turn_id);
        }
        let owned = OwnedEndpointClaim {
            turn: accepted,
            text,
            schema: completion_schema,
        };
        let claim = match owned.claim(self.request.scope()?) {
            Ok(claim) => claim,
            Err(_) => return self.finish_invalid_proposal(&turn_id),
        };
        if let Some(check) = completion_check {
            let (checked, stop) = {
                let control = RuntimePhaseControl::for_run_limits(
                    &mut self.clock,
                    &request.task.task_id,
                    request.limits.max_elapsed_ms,
                    &self.correlation_id,
                    self.started_at_epoch_ms
                        .ok_or(RuntimeLoopError::InvalidBoundaryResult)?,
                    self.events.last().map_or(0, |e| e.occurred_at_epoch_ms),
                    &self.last_phase_observed_at,
                    cancellation,
                );
                let checked = control.check().and_then(|_| check(&control));
                // Always observe control again, including after refusal.
                // Preserve the original stop/failure precedence below.
                let _ = control.check();
                (checked, control.into_stop())
            };
            self.latch_phase_cancellation(stop.as_ref());
            if let Some(stop) = stop {
                if let Err(error) = &checked
                    && *error != stop.port_failure()
                {
                    return self.finish_model_failure(&turn_id, *error);
                }
                return self.finish_phase_stop(stop, false);
            }
            if let Err(error) = checked {
                if error == RuntimePortFailure::Invalid {
                    return self.finish_refused_proposal(
                        &turn_id,
                        "runtime.proposal.completion-check-refused",
                    );
                }
                return self.finish_model_failure(&turn_id, error);
            }
        }
        for state in [
            AgentStateKind::Approval,
            AgentStateKind::Execution,
            AgentStateKind::Verification,
        ] {
            self.state
                .transition(state)
                .map_err(|_| RuntimeLoopError::State)?;
        }
        let postconditions = runtime_postconditions_from_task(&request.task);
        let registry = VerifierRegistry::new(VerifierContext {
            verifier_id: self.verifier.verifier_id().clone(),
            task_id: request.task.task_id.clone(),
            proposal_id: claim.proposal_id().clone(),
            repository_snapshot_id: request.repository_snapshot_id.clone(),
            state_revision: self.state.revision(),
            postconditions: postconditions.clone(),
        })
        .map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
        let candidate = self
            .verifier
            .verify_completion(RuntimeVerificationInput {
                request: self.request.scope()?,
                proposal: claim,
                state_revision: self.state.revision(),
                postconditions: &postconditions,
                evidence: &self.evidence,
                tool_results: &self.tool_results,
                receipt_ids: &self.receipt_ids,
            })
            .map_err(RuntimeLoopError::Dependency)?;
        let completion = match registry.verify(&candidate) {
            Ok(value) => value,
            Err(_) => {
                self.transition_terminal(AgentStateKind::Failed)?;
                self.close_turn(&turn_id, observation_sha256(&owned.turn)?)?;
                return self.finish_terminal(
                    AgentStateKind::Failed,
                    vec!["runtime.verification.failed".into()],
                    None,
                );
            }
        };
        if self.stop_before_phase(cancellation, false)? {
            return Ok(());
        }
        for item in candidate
            .postconditions
            .iter()
            .flat_map(|result| &result.evidence)
        {
            if !self
                .evidence
                .iter()
                .any(|old| old.evidence_id == item.evidence_id)
            {
                self.evidence.push(item.clone());
            }
        }
        self.evidence
            .sort_by(|a, b| a.evidence_id.as_str().cmp(b.evidence_id.as_str()));
        let claim = owned
            .claim(self.request.scope()?)
            .map_err(RuntimeLoopError::Dependency)?;
        let payload = claim
            .payload()
            .ok_or(RuntimeLoopError::InvalidBoundaryResult)?
            .clone();
        let closing_sha256 = claim.proposal_sha256().to_owned();
        if self.stop_before_phase(cancellation, false)? {
            return Ok(());
        }
        self.state
            .complete(&completion)
            .map_err(|_| RuntimeLoopError::State)?;
        self.endpoint_claim = Some(owned);
        self.close_turn(&turn_id, closing_sha256)?;
        self.finish_terminal(
            self.state.current(),
            Vec::new(),
            Some(RuntimeOutput::Inline { payload }),
        )
    }
}

fn observation_sha256(value: &impl serde::Serialize) -> Result<String, RuntimeLoopError> {
    let bytes = serde_json::to_vec(value).map_err(|_| RuntimeLoopError::InvalidBoundaryResult)?;
    if bytes.len() > agentmage_kernel_contracts::MAX_CONTRACT_JSON_BYTES {
        return Err(RuntimeLoopError::InvalidBoundaryResult);
    }
    Ok(sha256(&bytes))
}
