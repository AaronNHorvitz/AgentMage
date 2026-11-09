// SPDX-License-Identifier: Apache-2.0
//! Trusted prepared-effect startup and return of the original native owner.

use super::*;
use agentmage_host::{
    coding_endpoint::CodingEndpointConnection,
    coding_endpoint_edit_previews::{CodingEndpointAcceptancePort, CodingEndpointEditPreviews},
    coding_endpoint_prepared_startup::{
        CodingEndpointPrepared, CodingEndpointPreparedClock, CodingEndpointPreparedInput,
    },
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

/// Trusted startup for repeated strict model edits over one original repository.
/// Sources are a fixed consented selection, re-read freshly for each model turn.
/// This is ephemeral library composition, not executable activation or recovery.
pub struct CodingEndpointStrictHttpRun<V, C> {
    /// Original narrowed edit startup, using the factory's fresh identities.
    pub startup: agentmage_host::coding_endpoint_prepared_startup::CodingEndpointPreparedStartup,
    /// Original live creation owner; a path or serialized receipt cannot supply it.
    pub repository: LinuxCodingRepository,
    /// Original destination-approved, credential-owning transport.
    pub connection: CodingEndpointConnection<'static>,
    /// Original continuous control, retained through every advance and approval.
    pub control: Arc<dyn ModelOperationControl + Send + Sync>,
    /// One original epoch clock; the factory shares it without requiring Clone.
    pub clock: C,
    /// Independent trusted acceptance checks, never model narration.
    pub verifier: V,
    /// Exact selected paths under prior read and endpoint-disclosure consent.
    pub paths: Vec<String>,
    /// Deterministic excerpt byte allocation.
    pub source_budget: usize,
    /// Request actual endpoint usage observations.
    pub include_usage: bool,
}

/// Run strict source capture, generation, original native diff and exact approval
/// through the existing retained HTTP coordinator. Construction is inert; each
/// advance requires disclosure consent and each edit its own protected approval.
/// One cumulative ledger and shared original clock cover the entire run. Stop,
/// release and shutdown discard ephemeral ownership; no fresh run resumes it.
/// General edit/test/retry, approved Git and durable recovery remain separate.
pub fn serve_coding_endpoint_strict_edits<V, C>(
    listener: TcpListener,
    tokens: ApiTokens,
    stop: Arc<AtomicBool>,
    access: CodingEndpointObservationAccess,
    factory: impl FnOnce(RuntimeRunId, SessionId) -> Result<CodingEndpointStrictHttpRun<V, C>, ApiError>,
) -> Result<(), ApiError>
where
    V: RuntimeVerifierPort + Send + 'static,
    C: RuntimeClock + Send + 'static,
{
    serve_coding_endpoint_edits_and_tests_inner(
        listener,
        tokens,
        stop,
        access,
        false,
        |id, session| factory(id, session).map(|run| (run, None)),
        |_, verifier| Ok(verifier),
    )
    .map(|_| ())
}

/// Explicit fixed-test coding composition over the same local API. The factory
/// must use an EditAndTest startup and the original live repository. Its fixed
/// validation definition is never selected by a model or deserialized HTTP field.
/// Every advance still requires disclosure consent and every edit/test its exact
/// approval. The supplied independent verifier alone can accept completion.
pub fn serve_coding_endpoint_edits_and_tests<V, C>(
    listener: TcpListener,
    tokens: ApiTokens,
    stop: Arc<AtomicBool>,
    access: CodingEndpointObservationAccess,
    factory: impl FnOnce(
        RuntimeRunId,
        SessionId,
    ) -> Result<
        (
            CodingEndpointStrictHttpRun<V, C>,
            agentmage_host::coding_endpoint_edit_previews::CodingEndpointValidation,
        ),
        ApiError,
    >,
) -> Result<(), ApiError>
where
    V: RuntimeVerifierPort + Send + 'static,
    C: RuntimeClock + Send + 'static,
{
    serve_coding_endpoint_edits_and_tests_inner(
        listener,
        tokens,
        stop,
        access,
        false,
        |id, session| factory(id, session).map(|(run, validation)| (run, Some(validation))),
        |_, verifier| Ok(verifier),
    )
    .map(|_| ())
}

/// Serve fixed-test coding with an explicit independent checker over the SAME
/// original owner's records. The original owner checks linked validation and
/// current sealed inputs before and after dispatch. The registry still checks
/// the candidate. The factory must bind its checker to the final narrowed
/// request; this grants no effect or disclosure permission and adds no recovery.
pub fn serve_coding_endpoint_edits_and_checked_tests<V, C>(
    listener: TcpListener,
    tokens: ApiTokens,
    stop: Arc<AtomicBool>,
    access: CodingEndpointObservationAccess,
    factory: impl FnOnce(
        RuntimeRunId,
        SessionId,
    ) -> Result<
        (
            CodingEndpointStrictHttpRun<V, C>,
            agentmage_host::coding_endpoint_edit_previews::CodingEndpointValidation,
        ),
        ApiError,
    >,
) -> Result<(), ApiError>
where
    V: CodingEndpointAcceptancePort + Send + 'static,
    C: RuntimeClock + Send + 'static,
{
    serve_coding_endpoint_edits_and_tests_inner(
        listener,
        tokens,
        stop,
        access,
        false,
        |id, session| factory(id, session).map(|(run, validation)| (run, Some(validation))),
        |previews, verifier| {
            previews
                .completion_verifier(verifier)
                .map_err(|_| ApiError::Configuration)
        },
    )
    .map(|_| ())
}

/// Return the original strict-edit repository after explicit checked terminal
/// release and joined HTTP shutdown. Live, uncertain and faulted runs refuse
/// release; stopping without release returns `None`. Pending decisions are
/// abandoned, while edit lineage and observations survive for separately
/// approved original-checkout apply. No run, grant or crash state is resumed.
pub fn serve_coding_endpoint_strict_edits_returning_repository<V, C>(
    listener: TcpListener,
    tokens: ApiTokens,
    stop: Arc<AtomicBool>,
    access: CodingEndpointObservationAccess,
    factory: impl FnOnce(RuntimeRunId, SessionId) -> Result<CodingEndpointStrictHttpRun<V, C>, ApiError>,
) -> Result<Option<LinuxCodingRepository>, ApiError>
where
    V: RuntimeVerifierPort + Send + 'static,
    C: RuntimeClock + Send + 'static,
{
    serve_coding_endpoint_edits_and_tests_inner(
        listener,
        tokens,
        stop,
        access,
        true,
        |id, session| factory(id, session).map(|run| (run, None)),
        |_, verifier| Ok(verifier),
    )
}

/// Return the SAME original combined edit/test repository after exact terminal
/// release and joined shutdown. Completion still uses its original-owner
/// checker. All authority and release checks match the strict-edit return path;
/// nothing is returned without explicit successful release. Later apply needs
/// a fresh original diff and exact approval. This is process-local continuation.
pub fn serve_coding_endpoint_edits_and_checked_tests_returning_repository<V, C>(
    listener: TcpListener,
    tokens: ApiTokens,
    stop: Arc<AtomicBool>,
    access: CodingEndpointObservationAccess,
    factory: impl FnOnce(
        RuntimeRunId,
        SessionId,
    ) -> Result<
        (
            CodingEndpointStrictHttpRun<V, C>,
            agentmage_host::coding_endpoint_edit_previews::CodingEndpointValidation,
        ),
        ApiError,
    >,
) -> Result<Option<LinuxCodingRepository>, ApiError>
where
    V: CodingEndpointAcceptancePort + Send + 'static,
    C: RuntimeClock + Send + 'static,
{
    serve_coding_endpoint_edits_and_tests_inner(
        listener,
        tokens,
        stop,
        access,
        true,
        |id, session| factory(id, session).map(|(run, validation)| (run, Some(validation))),
        |previews, verifier| {
            previews
                .completion_verifier(verifier)
                .map_err(|_| ApiError::Configuration)
        },
    )
}

fn serve_coding_endpoint_edits_and_tests_inner<V, C, W>(
    listener: TcpListener,
    tokens: ApiTokens,
    stop: Arc<AtomicBool>,
    access: CodingEndpointObservationAccess,
    return_repository: bool,
    factory: impl FnOnce(
        RuntimeRunId,
        SessionId,
    ) -> Result<
        (
            CodingEndpointStrictHttpRun<V, C>,
            Option<agentmage_host::coding_endpoint_edit_previews::CodingEndpointValidation>,
        ),
        ApiError,
    >,
    select_verifier: impl FnOnce(
        &CodingEndpointEditPreviews<CodingEndpointPreparedClock<C>>,
        V,
    ) -> Result<W, ApiError>,
) -> Result<Option<LinuxCodingRepository>, ApiError>
where
    W: RuntimeVerifierPort + Send + 'static,
    C: RuntimeClock + Send + 'static,
{
    use agentmage_host::{
        coding_endpoint_edit_generation::CodingEndpointStrictEdits,
        coding_endpoint_edit_previews::CodingEndpointEditPreviews,
        coding_endpoint_prepared_startup::CodingEndpointPreparedClock,
    };
    let admit = |id: RuntimeRunId,
                 session: SessionId,
                 runs: &mut agentmage_host::coding_endpoint_owner::CodingEndpointRuns<
        'static,
        _,
        _,
        _,
        _,
    >| {
        let (input, validation) = factory(id.clone(), session.clone())?;
        let request = input.startup.request().clone();
        if request.run_id != id || request.session_id != session {
            return Err(ApiError::Configuration);
        }
        let registry = if validation.is_some() {
            agentmage_platform_linux::linux_coding_endpoint_coding_registry()
        } else {
            agentmage_platform_linux::linux_coding_endpoint_edit_registry()
        }
        .map_err(|_| ApiError::Configuration)?;
        // Preserve the narrowed catalog rather than exposing the full registry.
        let mut registry = registry;
        input.startup.guidance().restrict_registry(&mut registry);
        let clock = CodingEndpointPreparedClock::new(input.clock);
        let (previews, boundary) = if let Some(validation) = validation {
            CodingEndpointEditPreviews::new_coding(
                input.startup,
                input.repository,
                input.control,
                clock.clone(),
                validation,
            )
        } else {
            CodingEndpointEditPreviews::new(
                input.startup,
                input.repository,
                input.control,
                clock.clone(),
            )
        }
        .map_err(|_| ApiError::Configuration)?;
        let verifier = select_verifier(&previews, input.verifier)?;
        let port = CodingEndpointStrictEdits::new(
            previews,
            input.connection,
            input.paths,
            input.source_budget,
            input.include_usage,
        )
        .map_err(|_| ApiError::Configuration)?;
        runs.admit_turn_port(request.clone(), port, registry, boundary, verifier, clock)
            .map_err(|_| ApiError::Configuration)?;
        Ok(request)
    };
    let boundary = if return_repository {
        serve_admitted_coding_endpoint_with_turns_returning_tool(
            listener, tokens, stop, access, admit,
        )?
    } else {
        serve_admitted_coding_endpoint_with_turns(listener, tokens, stop, access, admit)?;
        None
    };
    boundary
        .map(|boundary| {
            boundary
                .try_into_repository()
                .map_err(|_| ApiError::Configuration)
        })
        .transpose()
}
