// SPDX-License-Identifier: Apache-2.0
//! One controlled endpoint phase, not a second agent loop or native admission.

use agentmage_kernel_contracts::{
    BudgetResource, EndpointModelRequest, EndpointModelResult, ModelOperationControl,
    ModelOperationStop,
};
use sha2::{Digest, Sha256};

use crate::runtime_hardening::RuntimeResourceLedger;

use super::{RuntimePortFailure, valid_sha256};

mod context;
pub use context::{
    RuntimeEndpointContextPacket, RuntimeEndpointMessage, RuntimeEndpointMessageRole,
};

/// A single prepared endpoint dispatch. The implementation retains its original
/// credentials, body and disclosure approval; the kernel owns no socket or key.
/// Text is provisional until the complete result and delivered stream agree.
pub trait RuntimeEndpointModelPort {
    /// The transport's closed, content-free failure type.
    type Error;

    /// Describes the immutable request this exact port will dispatch.
    fn request(&self) -> &EndpointModelRequest;

    /// Consumes the prepared request once under the original borrowed control.
    /// Implementations must poll control during waits and after text callbacks.
    /// Callbacks are synchronous and cannot be preempted by this contract.
    fn dispatch(
        self,
        control: &dyn ModelOperationControl,
        text: &mut dyn FnMut(&str),
    ) -> Result<EndpointModelResult, Self::Error>;
}

/// A refusal by the phase owner, distinct from the credential-owning transport.
#[derive(Debug, PartialEq, Eq)]
pub enum RuntimeEndpointPhaseFailure<E> {
    /// The borrowed run owner stopped before admission or after dispatch.
    Stopped(ModelOperationStop),
    /// The credential-owning dependency refused or failed. Never auto-retried.
    Dependency(E),
    /// The returned descriptor or resource observation does not match this phase.
    InvalidResult,
    /// One resource ceiling was exceeded.
    ResourceExhausted,
}

/// Content-free observations, scoped to this single model phase only.
/// Prepared bytes are not bytes sent, provider tokens, or rendered prompt tokens.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct RuntimeEndpointPhaseUsage {
    /// Number of admitted dispatch attempts, including failed dispatches.
    pub dispatch_attempts: u64,
    /// Frozen request-body bytes admitted for the attempt, not network usage.
    pub prepared_body_bytes: u64,
    /// Completed decoded text bytes; null when no accepted completion exists.
    pub completed_response_text_bytes: Option<u64>,
    /// Local completed dispatch time rounded up to milliseconds; null on failure.
    pub completed_dispatch_elapsed_ms: Option<u64>,
}

/// Kernel-owned single-use phase over an already approved, frozen transport
/// request. It owns no credential, socket, source path, grant or native profile.
/// The caller retains disclosure approval and all subsequent effect boundaries.
pub struct RuntimeEndpointModelPhase {
    request: EndpointModelRequest,
    context: RuntimeEndpointContextPacket,
    resources: RuntimeResourceLedger,
    attempted: bool,
    completed: bool,
}

impl RuntimeEndpointModelPhase {
    /// Frames one prepared request against independently admitted original
    /// messages. This validates no destination and approves no disclosure.
    pub fn new(
        request: &EndpointModelRequest,
        context: RuntimeEndpointContextPacket,
    ) -> Result<Self, RuntimePortFailure> {
        let limits = request.limits();
        if &request.context != context.messages()
            || !valid_sha256(&request.body().request_body_sha256)
            || !valid_sha256(&request.context.serialized_messages_sha256)
            || request.context.message_count == 0
            || request.context.serialized_messages_bytes == 0
            || request.context.serialized_messages_bytes > request.body().request_body_bytes
            || limits.requested_max_output_tokens == 0
            || limits.idle_timeout.is_zero()
            || limits.idle_timeout > limits.total_timeout
            || limits.response_text_limit_bytes > limits.response_body_limit_bytes
            || limits.response_body_limit_bytes > limits.response_wire_limit_bytes
        {
            return Err(RuntimePortFailure::Invalid);
        }
        let resources = RuntimeResourceLedger::for_endpoint_model(request)
            .map_err(|_| RuntimePortFailure::Invalid)?;
        Ok(Self {
            request: request.clone(),
            context,
            resources,
            attempted: false,
            completed: false,
        })
    }

    /// Independently checked context facts, without source text or token claims.
    #[must_use]
    pub const fn context(&self) -> &RuntimeEndpointContextPacket {
        &self.context
    }

    /// Invokes the supplied single-use model port under the original borrowed
    /// control. No reconstruction, retry, tool dispatch or cancellation ack.
    /// Even a failed dispatch consumes this phase; re-entry never calls the port.
    pub fn execute<P: RuntimeEndpointModelPort>(
        &mut self,
        control: &dyn ModelOperationControl,
        port: P,
        mut text: impl FnMut(&str),
    ) -> Result<EndpointModelResult, RuntimeEndpointPhaseFailure<P::Error>> {
        if self.attempted {
            return Err(RuntimeEndpointPhaseFailure::ResourceExhausted);
        }
        // Latch single-use before observing any external dependency.
        self.attempted = true;
        control
            .remaining_ms()
            .map_err(RuntimeEndpointPhaseFailure::Stopped)?;
        if port.request() != &self.request {
            return Err(RuntimeEndpointPhaseFailure::InvalidResult);
        }
        self.resources
            .consume(
                BudgetResource::InputBytes,
                self.request.body().request_body_bytes as u64,
            )
            .and_then(|()| self.resources.consume(BudgetResource::ModelCalls, 1))
            .map_err(|_| RuntimeEndpointPhaseFailure::ResourceExhausted)?;
        // Hash/count at the consumer boundary independently of the transport.
        // No plaintext is retained here. Once a callback refuses, suppress all
        // subsequent text even if a broken port continues calling it.
        let mut response_hash = Sha256::new();
        let mut response_bytes = 0_usize;
        let mut stream_failure = None;
        let result = port
            .dispatch(control, &mut |fragment| {
                if stream_failure.is_some() {
                    return;
                }
                if let Err(stop) = control.remaining_ms() {
                    stream_failure = Some(RuntimeEndpointPhaseFailure::Stopped(stop));
                    return;
                }
                let Some(bytes) = response_bytes
                    .checked_add(fragment.len())
                    .filter(|bytes| *bytes <= self.request.limits().response_text_limit_bytes)
                else {
                    stream_failure = Some(RuntimeEndpointPhaseFailure::ResourceExhausted);
                    return;
                };
                response_hash.update(fragment.as_bytes());
                response_bytes = bytes;
                text(fragment);
                if let Err(stop) = control.remaining_ms() {
                    stream_failure = Some(RuntimeEndpointPhaseFailure::Stopped(stop));
                }
            })
            .map_err(RuntimeEndpointPhaseFailure::Dependency)?;
        if let Some(failure) = stream_failure {
            return Err(failure);
        }
        control
            .remaining_ms()
            .map_err(RuntimeEndpointPhaseFailure::Stopped)?;
        if result.request != self.request
            || !valid_sha256(&result.response_text_sha256)
            || result.resources.dispatch_elapsed > self.request.limits().total_timeout
        {
            return Err(RuntimeEndpointPhaseFailure::InvalidResult);
        }
        if result.response_text_bytes != response_bytes
            || result.response_text_sha256
                != response_hash
                    .finalize()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
        {
            return Err(RuntimeEndpointPhaseFailure::InvalidResult);
        }
        let elapsed = u64::try_from(
            result
                .resources
                .dispatch_elapsed
                .as_nanos()
                .div_ceil(1_000_000),
        )
        .map_err(|_| RuntimeEndpointPhaseFailure::InvalidResult)?;
        // Commit result accounting atomically. A rejected result cannot create a
        // partial completed observation, while the admitted attempt stays spent.
        let mut resources = self.resources.clone();
        resources
            .consume(
                BudgetResource::OutputBytes,
                result.response_text_bytes as u64,
            )
            .and_then(|()| resources.consume(BudgetResource::ElapsedMilliseconds, elapsed))
            .map_err(|_| RuntimeEndpointPhaseFailure::ResourceExhausted)?;
        self.resources = resources;
        self.completed = true;
        Ok(result)
    }

    /// Inspects only known phase observations. Unknown memory and tokens are not
    /// projected from the ledger's empty native counters as observed zeroes.
    #[must_use]
    pub fn usage(&self) -> RuntimeEndpointPhaseUsage {
        let usage = self.resources.durable_usage();
        RuntimeEndpointPhaseUsage {
            dispatch_attempts: usage.model_calls,
            prepared_body_bytes: usage.input_bytes,
            completed_response_text_bytes: self.completed.then_some(usage.output_bytes),
            completed_dispatch_elapsed_ms: self.completed.then_some(usage.elapsed_ms),
        }
    }
}

#[cfg(test)]
mod tests;
