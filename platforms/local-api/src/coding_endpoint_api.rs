// SPDX-License-Identifier: Apache-2.0
//! Dedicated authenticated HTTP projection of one original endpoint run owner.

use super::*;
use agentmage_host::{
    coding_endpoint::CodingEndpointConnection,
    coding_endpoint_owner::{
        CodingEndpointAdmission, CodingEndpointOwnerError, CodingEndpointRuns,
    },
};
use agentmage_kernel_contracts::{
    BoundaryKind, CONTRACT_SCHEMA_VERSION, CancellationId, CancellationReason, CancellationSignal,
    CorrelationId, ModelCancellationProbe, ModelRuntimeFailure, RuntimeApprovalResponse,
    RuntimeEndpointRunRequest, RuntimeRunId, SessionId,
};
use agentmage_kernel_engine::{
    runtime_loop::{
        RuntimeClock, RuntimeEndpointCoordinatorStep, RuntimeEndpointTurnPort, RuntimeToolBoundary,
        RuntimeVerifierPort,
    },
    tooling::ToolRegistry,
};
use std::sync::TryLockError;

/// Trusted host composition, never deserialized from HTTP. The host must obtain
/// source-read consent, capture snapshots and construct its real permission,
/// tool and verifier ports. The HTTP adapter grants none of that authority.
pub struct CodingEndpointHttpRun<T, V, C> {
    /// Sealed request using the exact run/session identities supplied to the factory.
    pub request: RuntimeEndpointRunRequest,
    /// Original approved immutable configuration, retained by the coordinator.
    pub connection: CodingEndpointConnection<'static>,
    /// Exact model-visible catalog supported by the supplied boundary.
    pub registry: ToolRegistry,
    /// Already consented original snapshots. Never opened by this adapter.
    pub sources: Vec<CodingSource>,
    /// Existing deterministic source allocation, in bytes.
    pub source_budget: usize,
    /// Request usage observations from the selected endpoint.
    pub include_usage: bool,
    /// Original scoped permission/grant and tool owner.
    pub tool_boundary: T,
    /// Original independent deterministic completion checks.
    pub verifier: V,
    /// Original run clock, retained across every HTTP operation.
    pub clock: C,
}

/// Trusted service-startup delegation policy. Wire input cannot change it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodingEndpointObservationAccess {
    /// Only the person-facing control credential may inspect the run.
    ControlOnly,
    /// The ordinary client credential may inspect the request and event pages.
    /// This shares potentially sensitive task and tool observations, never
    /// advancement, disclosure consent, approvals, cancellation or release.
    DelegateObservations,
}

impl CodingEndpointObservationAccess {
    // Only private launch configuration calls this; HTTP/model input cannot
    // change the service policy. Sharing covers all holders of client.token.
    pub(super) fn from_launch(share: bool) -> Self {
        if share {
            eprintln!(
                "Run observations shared with client.token holders: the complete task, tool catalog and canonical events may contain source or test data. Model calls, approvals, cancellation and release still require control.token. Choose trusted observers and their destinations."
            );
            Self::DelegateObservations
        } else {
            Self::ControlOnly
        }
    }
}

// Check before opening private launch configuration as well as immediately
// before admission. Loading configuration must not precede listener/stop refusal.
pub(super) fn check_start(listener: &TcpListener, stop: &AtomicBool) -> Result<(), ApiError> {
    if stop.load(Ordering::Acquire)
        || !listener
            .local_addr()
            .map_err(|_| ApiError::Io)?
            .ip()
            .is_loopback()
    {
        return Err(ApiError::Configuration);
    }
    Ok(())
}

struct Session<
    T: RuntimeToolBoundary,
    V: RuntimeVerifierPort,
    C: RuntimeClock,
    P: RuntimeEndpointTurnPort,
> {
    runs: CodingEndpointRuns<'static, T, V, C, P>,
    returned: Option<T>,
}

enum TerminalToolDisposition<T> {
    Discard,
    Return,
    RepositoryReview(fn(T) -> Result<LinuxCodingRepository, ApiError>),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    run_id: RuntimeRunId,
    request_sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Advance {
    run_id: RuntimeRunId,
    request_sha256: String,
    approve_model_context: bool,
    response: Option<RuntimeApprovalResponse>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Events {
    run_id: RuntimeRunId,
    request_sha256: String,
    // Index into the original immutable canonical prefix, not a resume cursor.
    after: usize,
}

struct Control<'a> {
    requested: &'a AtomicBool,
    stopping: &'a AtomicBool,
    request: &'a RuntimeEndpointRunRequest,
    correlation_id: &'a CorrelationId,
}
impl ModelCancellationProbe for Control<'_> {
    fn observe(&self) -> Result<Option<CancellationSignal>, ModelRuntimeFailure> {
        let stopping = self.stopping.load(Ordering::Acquire);
        Ok(
            (stopping || self.requested.load(Ordering::Acquire)).then(|| CancellationSignal {
                schema_version: CONTRACT_SCHEMA_VERSION,
                cancellation_id: CancellationId::from_raw(format!(
                    "cancel:{}",
                    self.request.run_id.as_str()
                )),
                correlation_id: self.correlation_id.clone(),
                task_id: self.request.task.task_id.clone(),
                reason: if stopping {
                    CancellationReason::Shutdown
                } else {
                    CancellationReason::UserRequested
                },
                requested_by: BoundaryKind::Shell,
            }),
        )
    }
}

fn identity(prefix: &str) -> Result<String, ApiError> {
    let mut bytes = [0_u8; 32];
    if rustix::rand::getrandom(&mut bytes, rustix::rand::GetRandomFlags::empty()) != Ok(32) {
        return Err(ApiError::Io);
    }
    Ok(format!(
        "{prefix}-{}",
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    ))
}

/// Hosts one ephemeral configured-endpoint run through the ordinary bounded
/// HTTP/authentication workers. The trusted factory executes once, before
/// serving, with fresh service-issued identities. It performs no model/tool
/// work on behalf of this function. Its immutable configuration and all ports
/// remain in the same retained coordinator until terminal/fault release.
///
/// Every route requires the control credential. Each advance requires explicit
/// disclosure consent; approval responses still undergo the coordinator's exact
/// challenge checks. Cancellation is latched independently of the busy owner.
/// No manual settings, workspace, native runtime or effect routes are exposed.
/// This adds no executable, native filesystem composition or durable recovery.
pub fn serve_coding_endpoint<T, V, C>(
    listener: TcpListener,
    tokens: ApiTokens,
    stop: Arc<AtomicBool>,
    factory: impl FnOnce(RuntimeRunId, SessionId) -> Result<CodingEndpointHttpRun<T, V, C>, ApiError>,
) -> Result<(), ApiError>
where
    T: RuntimeToolBoundary + Send + 'static,
    V: RuntimeVerifierPort + Send + 'static,
    C: RuntimeClock + Send + 'static,
{
    serve_coding_endpoint_with_observation_access(
        listener,
        tokens,
        stop,
        CodingEndpointObservationAccess::ControlOnly,
        factory,
    )
}

/// Hosts the same retained coordinator with an explicit startup observation
/// policy. Delegation shares task/catalog metadata and canonical tool events
/// with holders of the ordinary client credential. It grants no model/tool
/// execution or control authority. The default entry point remains control-only.
pub fn serve_coding_endpoint_with_observation_access<T, V, C>(
    listener: TcpListener,
    tokens: ApiTokens,
    stop: Arc<AtomicBool>,
    access: CodingEndpointObservationAccess,
    factory: impl FnOnce(RuntimeRunId, SessionId) -> Result<CodingEndpointHttpRun<T, V, C>, ApiError>,
) -> Result<(), ApiError>
where
    T: RuntimeToolBoundary + Send + 'static,
    V: RuntimeVerifierPort + Send + 'static,
    C: RuntimeClock + Send + 'static,
{
    serve_admitted_coding_endpoint(
        listener,
        tokens,
        stop,
        access,
        |run_id, session_id, runs| {
            let input = factory(run_id.clone(), session_id.clone())?;
            if input.request.run_id != run_id || input.request.session_id != session_id {
                return Err(ApiError::Configuration);
            }
            let request = input.request;
            runs.admit(CodingEndpointAdmission {
                request: request.clone(),
                connection: input.connection,
                registry: input.registry,
                sources: &input.sources,
                source_budget: input.source_budget,
                include_usage: input.include_usage,
                tool_boundary: input.tool_boundary,
                verifier: input.verifier,
                clock: input.clock,
            })
            .map_err(|_| ApiError::Configuration)?;
            Ok(request)
        },
    )
}

/// Hosts an original trusted admission directly in the same retained owner
/// served by the authenticated endpoint API. The factory runs once with fresh
/// service-issued identities. It may consume an original native preparation via
/// its trusted startup factory; no live ports are extracted or reconstructed.
/// The returned request must match both identities and an actually held run.
/// This grants no inspection, model disclosure, effect or recovery authority.
/// All HTTP routes, the explicit startup observation policy, per-advance
/// disclosure consent and exact approval checks use the existing service path.
pub fn serve_admitted_coding_endpoint<T, V, C>(
    listener: TcpListener,
    tokens: ApiTokens,
    stop: Arc<AtomicBool>,
    access: CodingEndpointObservationAccess,
    factory: impl FnOnce(
        RuntimeRunId,
        SessionId,
        &mut CodingEndpointRuns<'static, T, V, C>,
    ) -> Result<RuntimeEndpointRunRequest, ApiError>,
) -> Result<(), ApiError>
where
    T: RuntimeToolBoundary + Send + 'static,
    V: RuntimeVerifierPort + Send + 'static,
    C: RuntimeClock + Send + 'static,
{
    serve_admitted_endpoint_inner(
        listener,
        tokens,
        stop,
        access,
        factory,
        TerminalToolDisposition::Discard,
    )
    .map(|_| ())
}

/// Hosts the original admitted run, retaining its original tool owner after an
/// exact control-authorized terminal release. Only a known, fully published,
/// non-uncertain terminal can return the boundary. Refused releases leave the
/// run held for inspection; no wire argument can select a different policy.
///
/// Returns the boundary only after serving ends and all HTTP workers join. A
/// successful stop without an explicit successful release returns `None`.
/// Faults, uncertainty and interrupted live runs are never recovered this way.
/// Service errors may drop process-held state; this is not durable recovery.
/// The caller must abandon old native previews through their original owner's
/// methods and obtain fresh admission and approval before later work. Only the
/// original boundary returns, including its existing native authority; no grant
/// is renewed. The old run, budget, verifier and endpoint connection are dropped.
/// Existing service entry points retain their ordinary discard behavior.
pub fn serve_admitted_coding_endpoint_returning_tool<T, V, C>(
    listener: TcpListener,
    tokens: ApiTokens,
    stop: Arc<AtomicBool>,
    access: CodingEndpointObservationAccess,
    factory: impl FnOnce(
        RuntimeRunId,
        SessionId,
        &mut CodingEndpointRuns<'static, T, V, C>,
    ) -> Result<RuntimeEndpointRunRequest, ApiError>,
) -> Result<Option<T>, ApiError>
where
    T: RuntimeToolBoundary + Send + 'static,
    V: RuntimeVerifierPort + Send + 'static,
    C: RuntimeClock + Send + 'static,
{
    serve_admitted_endpoint_inner(
        listener,
        tokens,
        stop,
        access,
        factory,
        TerminalToolDisposition::Return,
    )
}

/// Trusted custom turn selection through the SAME endpoint HTTP service.
/// The factory supplies a live process-local turn port and original boundaries;
/// wire input cannot select them. Disclosure consent, exact approvals, retention,
/// cancellation and worker joining are identical to ordinary admitted runs.
/// This entry point discards ephemeral owners at shutdown; it grants no recovery.
pub fn serve_admitted_coding_endpoint_with_turns<T, V, C, P>(
    listener: TcpListener,
    tokens: ApiTokens,
    stop: Arc<AtomicBool>,
    access: CodingEndpointObservationAccess,
    factory: impl FnOnce(
        RuntimeRunId,
        SessionId,
        &mut CodingEndpointRuns<'static, T, V, C, P>,
    ) -> Result<RuntimeEndpointRunRequest, ApiError>,
) -> Result<(), ApiError>
where
    T: RuntimeToolBoundary + Send + 'static,
    V: RuntimeVerifierPort + Send + 'static,
    C: RuntimeClock + Send + 'static,
    P: RuntimeEndpointTurnPort + Send + 'static,
{
    serve_admitted_endpoint_inner(
        listener,
        tokens,
        stop,
        access,
        factory,
        TerminalToolDisposition::Discard,
    )
    .map(|_| ())
}

/// Return the original tool after checked terminal release of a custom-turn
/// run and joined shutdown. Uses the same terminal, uncertainty, authentication
/// and worker checks as `serve_admitted_coding_endpoint_returning_tool`.
/// Without explicit successful release returns `None`. The old turn port,
/// verifier, connection and budgets are discarded; no crash recovery is added.
pub fn serve_admitted_coding_endpoint_with_turns_returning_tool<T, V, C, P>(
    listener: TcpListener,
    tokens: ApiTokens,
    stop: Arc<AtomicBool>,
    access: CodingEndpointObservationAccess,
    factory: impl FnOnce(
        RuntimeRunId,
        SessionId,
        &mut CodingEndpointRuns<'static, T, V, C, P>,
    ) -> Result<RuntimeEndpointRunRequest, ApiError>,
) -> Result<Option<T>, ApiError>
where
    T: RuntimeToolBoundary + Send + 'static,
    V: RuntimeVerifierPort + Send + 'static,
    C: RuntimeClock + Send + 'static,
    P: RuntimeEndpointTurnPort + Send + 'static,
{
    serve_admitted_endpoint_inner(
        listener,
        tokens,
        stop,
        access,
        factory,
        TerminalToolDisposition::Return,
    )
}

pub(super) fn serve_admitted_coding_endpoint_with_repository_review<T, V, C, P>(
    listener: TcpListener,
    tokens: ApiTokens,
    stop: Arc<AtomicBool>,
    access: CodingEndpointObservationAccess,
    factory: impl FnOnce(
        RuntimeRunId,
        SessionId,
        &mut CodingEndpointRuns<'static, T, V, C, P>,
    ) -> Result<RuntimeEndpointRunRequest, ApiError>,
    convert: fn(T) -> Result<LinuxCodingRepository, ApiError>,
) -> Result<(), ApiError>
where
    T: RuntimeToolBoundary + Send + 'static,
    V: RuntimeVerifierPort + Send + 'static,
    C: RuntimeClock + Send + 'static,
    P: RuntimeEndpointTurnPort + Send + 'static,
{
    serve_admitted_endpoint_inner(
        listener,
        tokens,
        stop,
        access,
        factory,
        TerminalToolDisposition::RepositoryReview(convert),
    )
    .map(|_| ())
}

fn serve_admitted_endpoint_inner<T, V, C, P>(
    listener: TcpListener,
    tokens: ApiTokens,
    stop: Arc<AtomicBool>,
    access: CodingEndpointObservationAccess,
    factory: impl FnOnce(
        RuntimeRunId,
        SessionId,
        &mut CodingEndpointRuns<'static, T, V, C, P>,
    ) -> Result<RuntimeEndpointRunRequest, ApiError>,
    disposition: TerminalToolDisposition<T>,
) -> Result<Option<T>, ApiError>
where
    T: RuntimeToolBoundary + Send + 'static,
    V: RuntimeVerifierPort + Send + 'static,
    C: RuntimeClock + Send + 'static,
    P: RuntimeEndpointTurnPort + Send + 'static,
{
    check_start(&listener, &stop)?;
    let run_id = RuntimeRunId::from_raw(identity("endpoint-run")?);
    let session_id = SessionId::from_raw(identity("endpoint-session")?);
    let mut runs = CodingEndpointRuns::default();
    let request = factory(run_id.clone(), session_id.clone(), &mut runs)?;
    if request.run_id != run_id || request.session_id != session_id {
        return Err(ApiError::Configuration);
    }
    let correlation_id = runs
        .correlation_id(&request)
        .map_err(|_| ApiError::Configuration)?
        .clone();
    let session = Mutex::new(Session {
        runs,
        returned: None,
    });
    let cancelled = AtomicBool::new(false);
    // This state receives only the original released owner. No endpoint, run,
    // source selection, pending approval or grant is reconstructed here.
    let review = Mutex::new(State::default());
    serve_connections(
        listener,
        stop,
        |socket, address, stop| {
            let req = match super::request(socket, address, &tokens, stop) {
                Ok(req) => req,
                Err(status) => return error(socket, status),
            };
            if matches!(&disposition, TerminalToolDisposition::RepositoryReview(_))
                && matches!(
                    (req.method.as_str(), req.path.as_str()),
                    ("GET", "/v1/code/repository/apply")
                        | ("POST", "/v1/code/repository/apply/prepare")
                        | ("POST", "/v1/code/repository/apply/decide")
                )
            {
                // Reuse the original control authentication, explicit read
                // consent and exact native diff approval. No other manual
                // tool, creation, command or fresh model run is exposed.
                return coding_repository_api::handle_repository(socket, &req, &review, stop);
            }
            let observation = matches!(
                (req.method.as_str(), req.path.as_str()),
                ("GET", "/v1/code/endpoint/request") | ("POST", "/v1/code/endpoint/events")
            );
            if !(req.control
                || access == CodingEndpointObservationAccess::DelegateObservations && observation)
            {
                return error(socket, 403);
            }
            let binding_matches = |id: &RuntimeRunId, digest: &str| {
                id == &request.run_id && digest == request.request_sha256
            };
            let invalid = |socket: &mut TcpStream| {
                response(socket, 409, json!({"error":"code.endpoint.binding"}))
            };
            match (req.method.as_str(), req.path.as_str()) {
                ("GET", "/v1/code/endpoint/request") => response(
                    socket,
                    200,
                    json!({
                        "request":request, "retention":"process-only", "configuration":"fixed-for-service"
                    }),
                ),
                ("POST", "/v1/code/endpoint/cancel") => {
                    let Ok(input) = object_json::<Binding>(&req.body) else {
                        return error(socket, 400);
                    };
                    if !binding_matches(&input.run_id, &input.request_sha256) {
                        return invalid(socket);
                    }
                    cancelled.store(true, Ordering::Release);
                    response(socket, 200, json!({"cancellation":"requested"}))
                }
                ("POST", "/v1/code/endpoint/advance") => {
                    let Ok(input) = object_json::<Advance>(&req.body) else {
                        return error(socket, 400);
                    };
                    if !binding_matches(&input.run_id, &input.request_sha256) {
                        return invalid(socket);
                    }
                    if !input.approve_model_context && !cancelled.load(Ordering::Acquire) {
                        return error(socket, 403);
                    }
                    let mut guard = match session.try_lock() {
                        Ok(guard) => guard,
                        Err(TryLockError::WouldBlock) => return error(socket, 409),
                        Err(TryLockError::Poisoned(_)) => return error(socket, 500),
                    };
                    let control = Control {
                        requested: &cancelled,
                        stopping: stop,
                        request: &request,
                        correlation_id: &correlation_id,
                    };
                    match guard
                        .runs
                        .advance(&request, input.response.as_ref(), Some(&control))
                    {
                        Ok(RuntimeEndpointCoordinatorStep::AwaitingApproval { challenge }) => {
                            response(
                                socket,
                                200,
                                json!({"state":"awaiting_approval","challenge":challenge}),
                            )
                        }
                        Ok(RuntimeEndpointCoordinatorStep::Complete { outcome }) => {
                            response(socket, 200, json!({"state":"terminal","outcome":outcome}))
                        }
                        Err(e) => owner_error(socket, e),
                    }
                }
                ("POST", "/v1/code/endpoint/events") | ("POST", "/v1/code/endpoint/release") => {
                    let after = if req.path.ends_with("/events") {
                        let Ok(input) = object_json::<Events>(&req.body) else {
                            return error(socket, 400);
                        };
                        if !binding_matches(&input.run_id, &input.request_sha256) {
                            return invalid(socket);
                        }
                        input.after
                    } else {
                        let Ok(input) = object_json::<Binding>(&req.body) else {
                            return error(socket, 400);
                        };
                        if !binding_matches(&input.run_id, &input.request_sha256) {
                            return invalid(socket);
                        }
                        0
                    };
                    let mut guard = match session.try_lock() {
                        Ok(guard) => guard,
                        Err(TryLockError::WouldBlock) => return error(socket, 409),
                        Err(TryLockError::Poisoned(_)) => return error(socket, 500),
                    };
                    if req.path.ends_with("/release") {
                        let released = match &disposition {
                            TerminalToolDisposition::Discard => guard.runs.release(&request),
                            TerminalToolDisposition::Return => {
                                guard.runs.release_with_tool_boundary(&request).map(|tool| {
                                    guard.returned = Some(tool);
                                })
                            }
                            TerminalToolDisposition::RepositoryReview(convert) => {
                                // Check publication state before consuming the run. A
                                // busy or poisoned review owner must not discard it.
                                let mut review = match review.try_lock() {
                                    Ok(review) => review,
                                    Err(TryLockError::WouldBlock) => return error(socket, 409),
                                    Err(TryLockError::Poisoned(_)) => return error(socket, 500),
                                };
                                if review.repository.is_some() {
                                    // This single-run service already published its
                                    // original owner; repeated release stays refused.
                                    return owner_error(socket, CodingEndpointOwnerError::NotHeld);
                                }
                                match guard.runs.release_with_tool_boundary(&request) {
                                    Ok(tool) => {
                                        let repository = match convert(tool) {
                                            Ok(repository) => repository,
                                            // Release already consumed the terminal run.
                                            // Never disguise extraction refusal as an empty
                                            // socket reply or imply that retry restores it.
                                            Err(_) => {
                                                return response(
                                                    socket,
                                                    409,
                                                    json!({
                                                        "error":"code.endpoint.original-review-unavailable",
                                                        "released":true,
                                                        "original_review_available":false,
                                                        "message":"The run was released, but original review is unavailable. Inspect state and disk; do not repeat release or approval."
                                                    }),
                                                );
                                            }
                                        };
                                        review.repository = Some(Arc::new(Mutex::new(repository)));
                                        Ok(())
                                    }
                                    Err(error) => Err(error),
                                }
                            }
                        };
                        return match released {
                            Ok(()) => response(socket, 200, json!({"released":true})),
                            Err(e) => owner_error(socket, e),
                        };
                    }
                    match guard.runs.events(&request) {
                        Ok(events) if after <= events.len() => {
                            // One original event per page bounds duplication and never
                            // omits, reframes or manufactures canonical events.
                            let end = events.len().min(after.saturating_add(1));
                            response(
                                socket,
                                200,
                                json!({"events":&events[after..end],"next":end,"total":events.len(),"retention":"process-only"}),
                            )
                        }
                        Ok(_) => invalid(socket),
                        Err(e) => owner_error(socket, e),
                    }
                }
                _ => error(socket, 404),
            }
        },
        || {
            review
                .lock()
                .map_err(|_| ApiError::Io)?
                .cancel_active(true)
                .map(|_| ())
        },
    )?;
    let session = session.into_inner().map_err(|_| ApiError::Io)?;
    Ok(session.returned)
}

fn owner_error(socket: &mut TcpStream, error: CodingEndpointOwnerError) -> Result<(), ApiError> {
    // Do not expose dependency diagnostics, private source or credentials.
    let code = match error {
        CodingEndpointOwnerError::StillRunning => "code.endpoint.still-running",
        CodingEndpointOwnerError::NotHeld => "code.endpoint.not-held",
        CodingEndpointOwnerError::Faulted => "code.endpoint.faulted",
        _ => "code.endpoint.refused",
    };
    response(socket, 409, json!({"error":code}))
}

#[cfg(test)]
#[path = "coding_endpoint_release_tests.rs"]
mod tests;
