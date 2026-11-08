// SPDX-License-Identifier: Apache-2.0
//! Explicit endpoint model turns for coordinator composition. No orchestration
//! loop, tools, proposal interpretation, recovery or effect authority lives here.

use std::cell::Cell;

use agentmage_kernel_contracts::{
    BudgetResource, CancellationSignal, CorrelationId, EndpointModelRequest, EndpointModelResult,
    ModelCancellationProbe, RuntimeEndpointRunRequest, RuntimeSessionMode, RuntimeTurnId,
};

use super::{
    RuntimeEndpointContextPacket, RuntimeEndpointModelPhase, RuntimeEndpointModelPort,
    RuntimeEndpointPhaseFailure, RuntimeEndpointRunBinding,
};
use crate::runtime_hardening::RuntimeResourceLedger;
use crate::runtime_loop::{
    RuntimeClock, RuntimePhaseControl, RuntimePhaseStop, RuntimePortFailure, derived_id,
};

/// A failed explicit turn. Every refusal closes this model-turn state; it
/// cannot silently retry a dependency failure or renew a spent run budget.
#[derive(Debug, PartialEq, Eq)]
pub enum RuntimeEndpointTurnFailure<E> {
    /// Run framing, control or cumulative admission refused before dispatch.
    Admission(RuntimePortFailure),
    /// The original single-use stream phase refused or failed.
    Dispatch(RuntimeEndpointPhaseFailure<E>),
}

/// Accepted model completion bound to exactly one run, turn and original
/// context. This is model output, never an approved tool or verified answer.
/// Private fields and no deserializer prevent construction from wire claims.
#[derive(Debug)]
pub struct RuntimeEndpointTurnResult {
    run: RuntimeEndpointRunBinding,
    turn_id: RuntimeTurnId,
    result: EndpointModelResult,
}

impl RuntimeEndpointTurnResult {
    /// Immutable sealed run identity.
    #[must_use]
    pub const fn run(&self) -> &RuntimeEndpointRunBinding {
        &self.run
    }

    /// Coordinator-compatible identity derived from the run and turn ordinal.
    #[must_use]
    pub const fn turn_id(&self) -> &RuntimeTurnId {
        &self.turn_id
    }

    /// Exact accepted request/response descriptor, including original context.
    #[must_use]
    pub const fn result(&self) -> &EndpointModelResult {
        &self.result
    }
}

/// Known cumulative model observations only. These are not network-byte,
/// provider-token, process-memory, tool or durable-continuation observations.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct RuntimeEndpointTurnsUsage {
    /// Explicit turns admitted, including a turn whose dispatch later failed.
    pub turns: u32,
    /// Admitted model attempts, including failed dispatches.
    pub dispatch_attempts: u64,
    /// Complete prepared body bytes spent before dispatch.
    pub prepared_body_bytes: u64,
    /// Text bytes in accepted completions only; incomplete output is excluded.
    pub accepted_response_text_bytes: u64,
    /// Sum of rounded-up dispatch durations in accepted completions only.
    pub accepted_dispatch_elapsed_ms: u64,
}

/// Coordinator model-turn component with a single cumulative ledger and the
/// existing runtime's monotonic clock/cancellation control. The caller supplies
/// each prepared turn explicitly; this component never selects the next action.
/// Initially restricted to ephemeral read-only runs without a resume cursor.
/// It emits no runtime events and cannot replace the full coordinator.
pub struct RuntimeEndpointTurns<C> {
    request: RuntimeEndpointRunRequest,
    resources: RuntimeResourceLedger,
    clock: C,
    started_at: u64,
    last_observed_at: Cell<u64>,
    correlation_id: CorrelationId,
    turns: u32,
    closed: bool,
    cancellation: Option<CancellationSignal>,
}

impl<C: RuntimeClock> RuntimeEndpointTurns<C> {
    /// Verifies the sealed request and starts the one run clock immediately.
    /// No native profile, destination consent, tool permission or resume claim.
    pub fn new(
        request: RuntimeEndpointRunRequest,
        mut clock: C,
    ) -> Result<Self, RuntimePortFailure> {
        crate::runtime_coordinator::verify_runtime_endpoint_run_request(&request)
            .map_err(|_| RuntimePortFailure::Invalid)?;
        if request.mode != RuntimeSessionMode::EphemeralReadOnly || request.event_cursor.is_some() {
            return Err(RuntimePortFailure::Invalid);
        }
        let resources = RuntimeResourceLedger::for_endpoint_run(&request)
            .map_err(|_| RuntimePortFailure::Invalid)?;
        let started_at = clock.now_epoch_ms()?;
        if started_at == 0 {
            return Err(RuntimePortFailure::Invalid);
        }
        let correlation_id =
            CorrelationId::from_raw(derived_id("correlation", request.run_id.as_str(), 0));
        Ok(Self {
            request,
            resources,
            clock,
            started_at,
            last_observed_at: Cell::new(started_at),
            correlation_id,
            turns: 0,
            closed: false,
            cancellation: None,
        })
    }

    /// The same derived correlation identity used by the full coordinator.
    #[must_use]
    pub const fn correlation_id(&self) -> &CorrelationId {
        &self.correlation_id
    }

    /// Exact owner-observed cancellation, when one was latched. Dependency
    /// failure can still decide the failed result; this is not an exit receipt.
    #[must_use]
    pub const fn cancellation(&self) -> Option<&CancellationSignal> {
        self.cancellation.as_ref()
    }

    /// True after any refusal or failed turn. Accepted model output alone does
    /// not finish a task; the containing coordinator decides its next action.
    #[must_use]
    pub const fn is_closed(&self) -> bool {
        self.closed
    }

    /// Observes the original remaining wall time for preparing the next request.
    /// A failed observation closes the state; polling cannot reset the clock.
    pub fn remaining_ms(
        &mut self,
        cancellation: Option<&dyn ModelCancellationProbe>,
    ) -> Result<u64, RuntimePortFailure> {
        if self.closed {
            return Err(RuntimePortFailure::Invalid);
        }
        let control = RuntimePhaseControl::for_run_limits(
            &mut self.clock,
            &self.request.task.task_id,
            self.request.limits.max_elapsed_ms,
            &self.correlation_id,
            self.started_at,
            self.started_at,
            &self.last_observed_at,
            cancellation,
        );
        let result = control.check();
        if let Some(stop) = control.into_stop() {
            self.latch_stop(stop);
        }
        result
    }

    /// Executes exactly one caller-supplied turn with the original context and
    /// single-use model port. Text stays provisional until the result is accepted.
    /// Counts and prepared input are reserved atomically before any dispatch;
    /// accepted output/time are committed atomically after phase validation.
    /// Every failure is terminal for this component, including budget refusal.
    pub fn execute_turn<P: RuntimeEndpointModelPort>(
        &mut self,
        request: &EndpointModelRequest,
        context: RuntimeEndpointContextPacket,
        port: P,
        cancellation: Option<&dyn ModelCancellationProbe>,
        text: impl FnMut(&str),
    ) -> Result<RuntimeEndpointTurnResult, RuntimeEndpointTurnFailure<P::Error>> {
        use RuntimeEndpointTurnFailure::{Admission, Dispatch};
        if self.closed {
            return Err(Admission(RuntimePortFailure::Invalid));
        }
        // No path reopens a refusal, including failures before actual dispatch.
        self.closed = true;
        let mut phase = RuntimeEndpointModelPhase::for_run(&self.request, request, context)
            .map_err(Admission)?;
        if port.request() != request {
            return Err(Admission(RuntimePortFailure::Invalid));
        }
        let limits = &self.request.limits;
        if self.turns >= limits.max_turns
            || self.turns >= limits.max_context_refreshes
            || self.resources.durable_usage().model_calls >= u64::from(limits.max_model_calls)
        {
            return Err(Admission(RuntimePortFailure::ResourceExhausted));
        }
        let control = RuntimePhaseControl::for_run_limits(
            &mut self.clock,
            &self.request.task.task_id,
            self.request.limits.max_elapsed_ms,
            &self.correlation_id,
            self.started_at,
            self.started_at,
            &self.last_observed_at,
            cancellation,
        );
        if let Err(error) = control.check() {
            if let Some(stop) = control.into_stop() {
                self.latch_stop(stop);
            }
            return Err(Admission(error));
        }
        let mut planned = self.resources.clone();
        planned
            .consume(BudgetResource::PlanSteps, 1)
            .map_err(|_| Admission(RuntimePortFailure::ResourceExhausted))?;
        let mut admitted = planned.clone();
        for (resource, amount) in [
            (
                BudgetResource::InputBytes,
                request.body().request_body_bytes as u64,
            ),
            (BudgetResource::ModelCalls, 1),
        ] {
            admitted
                .consume(resource, amount)
                .map_err(|_| Admission(RuntimePortFailure::ResourceExhausted))?;
        }
        // Admit the complete per-dispatch envelope against remaining cumulative
        // capacity. A later provisional stream must not cross the run's output
        // ceiling before its final descriptor is available. Only observed
        // accepted output/time, not this reservation, becomes cumulative use.
        let mut envelope = admitted.clone();
        for (resource, amount) in [
            (
                BudgetResource::OutputBytes,
                request.limits().response_text_limit_bytes as u64,
            ),
            (
                BudgetResource::ElapsedMilliseconds,
                u64::try_from(
                    request
                        .limits()
                        .total_timeout
                        .as_nanos()
                        .div_ceil(1_000_000),
                )
                .map_err(|_| Admission(RuntimePortFailure::Invalid))?,
            ),
        ] {
            envelope
                .consume(resource, amount)
                .map_err(|_| Admission(RuntimePortFailure::ResourceExhausted))?;
        }
        // Keep the reservation spent even if a dependency unwinds and its
        // caller catches the panic. This component was closed before entry.
        self.resources = admitted;
        self.turns += 1;
        let turn_id = RuntimeTurnId::from_raw(derived_id(
            "turn",
            self.request.run_id.as_str(),
            u64::from(self.turns),
        ));
        let result = phase.execute(&control, port, text);
        let stop = control.into_stop();
        // Admission was reserved above, but a second control observation can
        // stop the phase before it admits an attempt. Never report that as a
        // model call or prepared input actually spent by the phase.
        if phase.usage().dispatch_attempts == 0 {
            self.resources = planned;
        }
        if let Some(stop) = stop {
            self.latch_stop(stop);
        }
        let result = result.map_err(Dispatch)?;
        let usage = phase.usage();
        let mut completed = self.resources.clone();
        for (resource, amount) in [
            (
                BudgetResource::OutputBytes,
                usage
                    .completed_response_text_bytes
                    .ok_or(Admission(RuntimePortFailure::Invalid))?,
            ),
            (
                BudgetResource::ElapsedMilliseconds,
                usage
                    .completed_dispatch_elapsed_ms
                    .ok_or(Admission(RuntimePortFailure::Invalid))?,
            ),
        ] {
            completed
                .consume(resource, amount)
                .map_err(|_| Admission(RuntimePortFailure::ResourceExhausted))?;
        }
        let run = phase
            .run_binding()
            .cloned()
            .ok_or(Admission(RuntimePortFailure::Invalid))?;
        self.resources = completed;
        self.closed = false;
        Ok(RuntimeEndpointTurnResult {
            run,
            turn_id,
            result,
        })
    }

    /// Inspects cumulative known model observations without native-only zeros.
    #[must_use]
    pub fn usage(&self) -> RuntimeEndpointTurnsUsage {
        let usage = self.resources.durable_usage();
        RuntimeEndpointTurnsUsage {
            turns: self.turns,
            dispatch_attempts: usage.model_calls,
            prepared_body_bytes: usage.input_bytes,
            accepted_response_text_bytes: usage.output_bytes,
            accepted_dispatch_elapsed_ms: usage.elapsed_ms,
        }
    }

    fn latch_stop(&mut self, stop: RuntimePhaseStop) {
        self.closed = true;
        if let RuntimePhaseStop::Cancelled(signal) = stop {
            self.cancellation = Some(signal);
        }
    }
}
