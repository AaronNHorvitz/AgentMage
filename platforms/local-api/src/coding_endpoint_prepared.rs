// SPDX-License-Identifier: Apache-2.0
//! Trusted prepared-effect startup and return of the original native owner.

use super::*;
use agentmage_host::{
    coding_endpoint::CodingEndpointConnection,
    coding_endpoint_prepared_startup::{CodingEndpointPrepared, CodingEndpointPreparedInput},
};
use agentmage_kernel_contracts::{ModelOperationControl, RuntimeRunId, SessionId};
use agentmage_kernel_engine::runtime_loop::{RuntimeClock, RuntimeVerifierPort};

/// Process-local startup inputs, never deserialized from a request or model.
/// The factory must use its supplied fresh run/session identities when binding
/// the prepared effect. Native admission and instruction narrowing remain with
/// the original prepared owner. This object grants no read or disclosure consent.
pub struct CodingEndpointPreparedHttpRun<V, C> {
    /// Original prepared edit or command, including its live creation owner.
    pub prepared: CodingEndpointPrepared,
    /// Credential-owning connection and its original destination approval.
    pub connection: CodingEndpointConnection<'static>,
    /// Original continuous run control, never renewed between HTTP advances.
    pub control: Arc<dyn ModelOperationControl + Send + Sync>,
    /// Original epoch clock, shared with the native tool by prepared admission.
    pub clock: C,
    /// Trusted independent acceptance checks; model narration is not verification.
    pub verifier: V,
    /// Fresh snapshots under prior source-read and disclosure consent.
    pub sources: Vec<CodingSource>,
    /// Deterministic source allocation in bytes, never a token estimate.
    pub source_budget: usize,
    /// Request actual usage observations from the configured endpoint.
    pub include_usage: bool,
}

/// Serve one original prepared effect through its instruction-narrowed native
/// boundary and the existing authenticated endpoint coordinator. Construction is
/// inert; every advance needs model-disclosure consent and every effect needs
/// its original exact protected approval. The factory runs once before serving.
///
/// Export canonical events before requesting release; return discards the run.
/// After explicit successful terminal release and joined service shutdown,
/// abandon old previews through the original boundary and return the same live
/// repository, retaining edit lineage and validation attempts. Without explicit
/// release return `None`; uncertain/faulted release refuses and stays inspectable.
/// Errors can drop ephemeral ownership. This adds no executable activation, path
/// adoption, automatic cleanup, durable recovery, grant or budget renewal.
pub fn serve_coding_endpoint_prepared<V, C>(
    listener: TcpListener,
    tokens: ApiTokens,
    stop: Arc<AtomicBool>,
    access: CodingEndpointObservationAccess,
    factory: impl FnOnce(
        RuntimeRunId,
        SessionId,
    ) -> Result<CodingEndpointPreparedHttpRun<V, C>, ApiError>,
) -> Result<Option<LinuxCodingRepository>, ApiError>
where
    V: RuntimeVerifierPort + Send + 'static,
    C: RuntimeClock + Send + 'static,
{
    serve_admitted_coding_endpoint_returning_tool(
        listener,
        tokens,
        stop,
        access,
        |run_id, session_id, runs| {
            let input = factory(run_id, session_id)?;
            input
                .prepared
                .admit(
                    runs,
                    CodingEndpointPreparedInput {
                        connection: input.connection,
                        control: input.control,
                        clock: input.clock,
                        verifier: input.verifier,
                        sources: &input.sources,
                        source_budget: input.source_budget,
                        include_usage: input.include_usage,
                    },
                )
                .map_err(|_| ApiError::Configuration)
        },
    )
    .map(|boundary| boundary.map(|tool| tool.into_repository()))
}
