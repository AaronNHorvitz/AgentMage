//! Interface-neutral composition of one ephemeral local coding coordinator.

use agentmage_kernel_contracts::RuntimeRunRequest;
use agentmage_kernel_engine::runtime_loop::{
    ReusableRuntimeCoordinator, RuntimeArtifactPort, RuntimeCheckpointPort, RuntimeClock,
    RuntimeContextPort, RuntimeCorrectnessTransactionPort, RuntimeJournalPort, RuntimeLoopError,
    RuntimeModelPort, RuntimeToolBoundary,
};

use crate::{
    coding_session::CodingSessionProfile, coding_tools::native_coding_runtime_registry,
    coding_verifier::CodingCompletionVerifier,
};

/// Concrete shared coordinator shape used by every ephemeral coding interface.
pub type EphemeralCodingCoordinator<M, X, T, C> =
    ReusableRuntimeCoordinator<M, X, T, CodingCompletionVerifier, C>;

/// Concrete shared coordinator shape used by every durable coding interface.
pub type DurableCodingCoordinator<M, X, T, C> =
    ReusableRuntimeCoordinator<M, X, T, CodingCompletionVerifier, C>;

/// Composes one coding run entirely through existing reusable runtime boundaries.
///
/// The returned coordinator owns no terminal, IDE, workflow, persistence, or MCP type. Native
/// tools are registered directly; every consequential effect remains inside `tool_boundary`.
pub fn compose_ephemeral_coding_coordinator<M, X, T, C>(
    profile: &CodingSessionProfile,
    request: RuntimeRunRequest,
    model: M,
    context: X,
    tool_boundary: T,
    clock: C,
) -> Result<EphemeralCodingCoordinator<M, X, T, C>, RuntimeLoopError>
where
    M: RuntimeModelPort,
    X: RuntimeContextPort,
    T: RuntimeToolBoundary,
    C: RuntimeClock,
{
    let mut registry = native_coding_runtime_registry(
        profile.write_scope().clone(),
        profile.commands().clone(),
        profile.validations().clone(),
    )
    .map_err(|_| RuntimeLoopError::ToolCatalogBinding)?;
    profile.coding_guidance().restrict_registry(&mut registry);
    ReusableRuntimeCoordinator::new(
        request,
        model,
        context,
        registry,
        tool_boundary,
        CodingCompletionVerifier::for_profile(profile),
        clock,
    )
}

/// Composes one resumable coding run through the canonical journal, artifact, and checkpoint ports.
///
/// Persistence does not broaden the coding profile or tool authority. The trusted boundary owns
/// every durable write and reconstructs only a verified safe-boundary continuation.
pub fn compose_durable_coding_coordinator<M, X, T, C>(
    profile: &CodingSessionProfile,
    request: RuntimeRunRequest,
    model: M,
    context: X,
    tool_boundary: T,
    clock: C,
) -> Result<DurableCodingCoordinator<M, X, T, C>, RuntimeLoopError>
where
    M: RuntimeModelPort,
    X: RuntimeContextPort,
    T: RuntimeToolBoundary
        + RuntimeJournalPort
        + RuntimeArtifactPort
        + RuntimeCheckpointPort
        + RuntimeCorrectnessTransactionPort,
    C: RuntimeClock,
{
    let mut registry = native_coding_runtime_registry(
        profile.write_scope().clone(),
        profile.commands().clone(),
        profile.validations().clone(),
    )
    .map_err(|_| RuntimeLoopError::ToolCatalogBinding)?;
    profile.coding_guidance().restrict_registry(&mut registry);
    ReusableRuntimeCoordinator::new_with_durable_state(
        request,
        model,
        context,
        registry,
        tool_boundary,
        CodingCompletionVerifier::for_profile(profile),
        clock,
    )
}
