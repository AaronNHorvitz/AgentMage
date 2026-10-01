//! Authenticated bounded Linux IPC projection of the shared runtime transport.
//!
//! Over this channel a run is cancelled only through a job control request,
//! which the host decides through its durable job ledger under the client
//! scope it derives from the authenticated peer (Decision 0120). Suspension and
//! resumption are job control requests too; a step names where a suspended run
//! stopped (Decision 0122). The catalog host answers documentation pack
//! requests (Decision 0130), memory requests (Decision 0131) and extension
//! requests (Decision 0132) over the same channel. Wire 15 carries a run's
//! recipe, its declared plan and an extension list's issuer identity
//! (Decision 0133).

use agentmage_kernel_contracts::{
    CancellationId, RuntimeApprovalResponse, RuntimeArtifactRef, RuntimeEventCursor, RuntimeRunId,
    RuntimeRunRequest, SessionId,
};
use agentmage_kernel_engine::job_control::JobControlRequest;
use agentmage_kernel_engine::runtime_artifact::{RuntimeArtifactPage, RuntimeArtifactState};
use agentmage_platform_linux::{LinuxAuthenticatedIpcSession, LinuxPeerIdentity};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::coding_action_history::EndedRunActionHistories;
use crate::coding_doc_packs::{DocPackAnswer, DocPackRequest};
use crate::coding_extensions::{ExtensionAnswer, ExtensionRequest};
use crate::coding_memory::{MemoryAnswer, MemoryRequest};
use crate::runtime_transport::{
    RuntimeClientScope, RuntimeJobControl, RuntimeJobStatus, RuntimePrepareInput,
    RuntimeRunDeclarations, RuntimeTransportError, RuntimeTransportPort, RuntimeTransportStep,
};

const WIRE_VERSION: u16 = 15;
/// The wire version this build speaks, as named in a support bundle.
pub const RUNTIME_IPC_WIRE_VERSION: u16 = WIRE_VERSION;
const MAX_WIRE_BYTES: usize = 4 * 1024 * 1024;
/// Largest encoded run declarations; the rest of a frame is envelope.
const MAX_DECLARATION_BYTES: usize = MAX_WIRE_BYTES - 64 * 1024;

/// One closed operation accepted by the host-owned runtime service.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
// Authenticated IPC admits one bounded request frame at a time; keeping the
// closed serde shape inline avoids a second internal representation.
#[allow(clippy::large_enum_variant)]
enum RuntimeIpcRequest {
    Prepare {
        input: RuntimePrepareInput,
    },
    Start {
        request: RuntimeRunRequest,
    },
    Advance {
        run_id: RuntimeRunId,
        request_sha256: String,
        after_event_cursor: Option<RuntimeEventCursor>,
        response: Option<RuntimeApprovalResponse>,
    },
    JobStatus {
        run_id: RuntimeRunId,
        request_sha256: String,
    },
    ControlJob {
        run_id: RuntimeRunId,
        request_sha256: String,
        request: JobControlRequest,
    },
    ReadArtifactPage {
        run_id: RuntimeRunId,
        request_sha256: String,
        reference: RuntimeArtifactRef,
        offset: u64,
        maximum_bytes: u32,
    },
    ReleaseArtifact {
        run_id: RuntimeRunId,
        request_sha256: String,
        reference: RuntimeArtifactRef,
    },
    RunDeclarations {
        run_id: RuntimeRunId,
        request_sha256: String,
    },
    EndedRunActionHistories {
        run_id: RuntimeRunId,
    },
    DocPack {
        request: DocPackRequest,
    },
    Memory {
        request: MemoryRequest,
    },
    Extension {
        request: ExtensionRequest,
    },
    RevokeSessionPreauthorization {
        session_id: SessionId,
        preauthorization_sha256: String,
    },
    Release {
        run_id: RuntimeRunId,
        request_sha256: String,
    },
    Shutdown,
}

/// One closed response from the host-owned runtime service.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case", deny_unknown_fields)]
// Responses are bounded by MAX_WIRE_BYTES and exchanged synchronously.
#[allow(clippy::large_enum_variant)]
enum RuntimeIpcResponse {
    Prepared {
        request: RuntimeRunRequest,
    },
    Step {
        step: RuntimeTransportStep,
    },
    ArtifactPage {
        page: RuntimeArtifactPage,
    },
    ArtifactState {
        state: RuntimeArtifactState,
    },
    RunDeclarations {
        declarations: RuntimeRunDeclarations,
    },
    EndedRunActionHistories {
        histories: EndedRunActionHistories,
    },
    DocPack {
        answer: DocPackAnswer,
    },
    Memory {
        answer: MemoryAnswer,
    },
    Extension {
        answer: ExtensionAnswer,
    },
    JobStatus {
        status: RuntimeJobStatus,
    },
    JobControl {
        control: RuntimeJobControl,
    },
    PreauthorizationRevoked,
    Released,
    Shutdown,
    Error {
        error: RuntimeTransportError,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeIpcEnvelope<T> {
    version: u16,
    payload: T,
}

enum DevelopmentIpcProbe {
    None,
    StaleApproval,
    ReplayApproval(Option<RuntimeApprovalResponse>),
    ExpiredCursor(Option<RuntimeEventCursor>),
    ArtifactIntegrity,
}

/// Client-side adapter that contains transport authority but no runtime or effect authority.
pub struct LinuxRuntimeIpcClient {
    session: LinuxAuthenticatedIpcSession,
    last_error: Option<RuntimeTransportError>,
    development_probe: DevelopmentIpcProbe,
}

impl LinuxRuntimeIpcClient {
    /// Binds an already authenticated one-use Linux IPC session.
    #[must_use]
    pub const fn new(session: LinuxAuthenticatedIpcSession) -> Self {
        Self {
            session,
            last_error: None,
            development_probe: DevelopmentIpcProbe::None,
        }
    }

    /// Corrupts exactly one approval digest for the executable development acceptance probe.
    ///
    /// This does not alter a host challenge, grant, or policy decision. The host must reject the
    /// resulting stale response before launching an effect.
    #[must_use]
    pub(crate) fn with_stale_approval_probe(mut self) -> Self {
        self.development_probe = DevelopmentIpcProbe::StaleApproval;
        self
    }

    /// Replays one already accepted approval for executable rejection testing.
    #[must_use]
    pub(crate) fn with_replayed_approval_probe(mut self) -> Self {
        self.development_probe = DevelopmentIpcProbe::ReplayApproval(None);
        self
    }

    /// Reuses an aged exact cursor after the live replay window advances past it.
    #[must_use]
    pub(crate) fn with_expired_cursor_probe(mut self) -> Self {
        self.development_probe = DevelopmentIpcProbe::ExpiredCursor(None);
        self
    }

    /// Corrupts one returned artifact page after the authenticated host response.
    #[must_use]
    pub(crate) fn with_artifact_integrity_probe(mut self) -> Self {
        self.development_probe = DevelopmentIpcProbe::ArtifactIntegrity;
        self
    }

    /// Returns the last exact transport refusal observed from the host or local channel.
    #[must_use]
    pub const fn last_error(&self) -> Option<RuntimeTransportError> {
        self.last_error
    }

    fn exchange(
        &mut self,
        request: RuntimeIpcRequest,
    ) -> Result<RuntimeIpcResponse, RuntimeTransportError> {
        let result = self.exchange_inner(request);
        if let Err(error) = &result
            && self.last_error.is_none()
        {
            self.last_error = Some(*error);
        }
        result
    }

    fn exchange_inner(
        &mut self,
        request: RuntimeIpcRequest,
    ) -> Result<RuntimeIpcResponse, RuntimeTransportError> {
        let bytes = serde_json::to_vec(&RuntimeIpcEnvelope {
            version: WIRE_VERSION,
            payload: request,
        })
        .map_err(|_| RuntimeTransportError::RequestDenied)?;
        self.session
            .write_frame(&bytes, MAX_WIRE_BYTES)
            .map_err(|_| RuntimeTransportError::RuntimeFailed)?;
        let bytes = self
            .session
            .read_frame(MAX_WIRE_BYTES)
            .map_err(|_| RuntimeTransportError::RuntimeFailed)?;
        let response: RuntimeIpcEnvelope<RuntimeIpcResponse> = serde_json::from_slice(&bytes)
            .map_err(|_| RuntimeTransportError::RuntimeEvidenceDenied)?;
        if response.version != WIRE_VERSION {
            return Err(RuntimeTransportError::RuntimeEvidenceDenied);
        }
        match response.payload {
            RuntimeIpcResponse::Error { error } => Err(error),
            response => Ok(response),
        }
    }

    /// Requests graceful shutdown after all runs have been released.
    pub fn shutdown(&mut self) -> Result<(), RuntimeTransportError> {
        match self.exchange(RuntimeIpcRequest::Shutdown)? {
            RuntimeIpcResponse::Shutdown => Ok(()),
            _ => Err(RuntimeTransportError::RuntimeEvidenceDenied),
        }
    }
}

impl RuntimeTransportPort for LinuxRuntimeIpcClient {
    fn prepare(
        &mut self,
        input: RuntimePrepareInput,
    ) -> Result<RuntimeRunRequest, RuntimeTransportError> {
        self.last_error = None;
        match self.exchange(RuntimeIpcRequest::Prepare { input })? {
            RuntimeIpcResponse::Prepared { request } => Ok(request),
            _ => Err(RuntimeTransportError::RuntimeEvidenceDenied),
        }
    }

    fn start(
        &mut self,
        request: RuntimeRunRequest,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
        match self.exchange(RuntimeIpcRequest::Start { request })? {
            RuntimeIpcResponse::Step { step } => Ok(step),
            _ => Err(RuntimeTransportError::RuntimeEvidenceDenied),
        }
    }

    fn advance(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        after_event_cursor: Option<&RuntimeEventCursor>,
        response: Option<&RuntimeApprovalResponse>,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
        let mut after_event_cursor = after_event_cursor.cloned();
        let mut response = response.cloned();
        match &mut self.development_probe {
            DevelopmentIpcProbe::StaleApproval => {
                if let Some(response) = response.as_mut() {
                    response.challenge_sha256 = "0".repeat(64);
                    self.development_probe = DevelopmentIpcProbe::None;
                }
            }
            DevelopmentIpcProbe::ReplayApproval(saved) => match saved {
                None => {
                    if response.is_some() {
                        *saved = response.clone();
                    }
                }
                Some(previous) => {
                    response = Some(previous.clone());
                    self.development_probe = DevelopmentIpcProbe::None;
                }
            },
            DevelopmentIpcProbe::ExpiredCursor(saved) => match saved {
                None => *saved = after_event_cursor.clone(),
                Some(previous)
                    if after_event_cursor
                        .as_ref()
                        .is_some_and(|cursor| cursor.sequence >= 40) =>
                {
                    after_event_cursor = Some(previous.clone());
                    self.development_probe = DevelopmentIpcProbe::None;
                }
                Some(_) => {}
            },
            DevelopmentIpcProbe::ArtifactIntegrity => {}
            DevelopmentIpcProbe::None => {}
        }
        match self.exchange(RuntimeIpcRequest::Advance {
            run_id: run_id.clone(),
            request_sha256: request_sha256.to_owned(),
            after_event_cursor,
            response,
        })? {
            RuntimeIpcResponse::Step { step } => Ok(step),
            _ => Err(RuntimeTransportError::RuntimeEvidenceDenied),
        }
    }

    /// Refused: over this channel a run is cancelled only through
    /// [`RuntimeTransportPort::control_job`].
    fn cancel(
        &mut self,
        _run_id: &RuntimeRunId,
        _request_sha256: &str,
        _cancellation_id: CancellationId,
        _after_event_cursor: Option<&RuntimeEventCursor>,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
        Err(RuntimeTransportError::RequestDenied)
    }

    fn job_status(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
    ) -> Result<RuntimeJobStatus, RuntimeTransportError> {
        match self.exchange(RuntimeIpcRequest::JobStatus {
            run_id: run_id.clone(),
            request_sha256: request_sha256.to_owned(),
        })? {
            RuntimeIpcResponse::JobStatus { status } => Ok(status),
            _ => Err(RuntimeTransportError::RuntimeEvidenceDenied),
        }
    }

    fn control_job(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        request: &JobControlRequest,
    ) -> Result<RuntimeJobControl, RuntimeTransportError> {
        match self.exchange(RuntimeIpcRequest::ControlJob {
            run_id: run_id.clone(),
            request_sha256: request_sha256.to_owned(),
            request: request.clone(),
        })? {
            RuntimeIpcResponse::JobControl { control } => Ok(control),
            _ => Err(RuntimeTransportError::RuntimeEvidenceDenied),
        }
    }

    fn read_artifact_page(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        reference: &RuntimeArtifactRef,
        offset: u64,
        maximum_bytes: u32,
    ) -> Result<RuntimeArtifactPage, RuntimeTransportError> {
        match self.exchange(RuntimeIpcRequest::ReadArtifactPage {
            run_id: run_id.clone(),
            request_sha256: request_sha256.to_owned(),
            reference: reference.clone(),
            offset,
            maximum_bytes,
        })? {
            RuntimeIpcResponse::ArtifactPage { mut page } => {
                if matches!(
                    self.development_probe,
                    DevelopmentIpcProbe::ArtifactIntegrity
                ) && let Some(byte) = page.bytes.first_mut()
                {
                    *byte ^= 1;
                    self.development_probe = DevelopmentIpcProbe::None;
                }
                Ok(page)
            }
            _ => Err(RuntimeTransportError::RuntimeEvidenceDenied),
        }
    }

    fn release_artifact(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        reference: &RuntimeArtifactRef,
    ) -> Result<RuntimeArtifactState, RuntimeTransportError> {
        match self.exchange(RuntimeIpcRequest::ReleaseArtifact {
            run_id: run_id.clone(),
            request_sha256: request_sha256.to_owned(),
            reference: reference.clone(),
        })? {
            RuntimeIpcResponse::ArtifactState { state } => Ok(state),
            _ => Err(RuntimeTransportError::RuntimeEvidenceDenied),
        }
    }

    fn run_declarations(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
    ) -> Result<RuntimeRunDeclarations, RuntimeTransportError> {
        match self.exchange(RuntimeIpcRequest::RunDeclarations {
            run_id: run_id.clone(),
            request_sha256: request_sha256.to_owned(),
        })? {
            RuntimeIpcResponse::RunDeclarations { declarations } => Ok(declarations),
            _ => Err(RuntimeTransportError::RuntimeEvidenceDenied),
        }
    }

    fn ended_run_action_histories(
        &mut self,
        run_id: &RuntimeRunId,
    ) -> Result<EndedRunActionHistories, RuntimeTransportError> {
        match self.exchange(RuntimeIpcRequest::EndedRunActionHistories {
            run_id: run_id.clone(),
        })? {
            RuntimeIpcResponse::EndedRunActionHistories { histories } => Ok(histories),
            _ => Err(RuntimeTransportError::RuntimeEvidenceDenied),
        }
    }

    fn doc_pack(
        &mut self,
        request: DocPackRequest,
    ) -> Result<DocPackAnswer, RuntimeTransportError> {
        match self.exchange(RuntimeIpcRequest::DocPack { request })? {
            RuntimeIpcResponse::DocPack { answer } => Ok(answer),
            _ => Err(RuntimeTransportError::RuntimeEvidenceDenied),
        }
    }

    fn memory(&mut self, request: MemoryRequest) -> Result<MemoryAnswer, RuntimeTransportError> {
        match self.exchange(RuntimeIpcRequest::Memory { request })? {
            RuntimeIpcResponse::Memory { answer } => Ok(answer),
            _ => Err(RuntimeTransportError::RuntimeEvidenceDenied),
        }
    }

    fn extension(
        &mut self,
        request: ExtensionRequest,
    ) -> Result<ExtensionAnswer, RuntimeTransportError> {
        match self.exchange(RuntimeIpcRequest::Extension { request })? {
            RuntimeIpcResponse::Extension { answer } => Ok(answer),
            _ => Err(RuntimeTransportError::RuntimeEvidenceDenied),
        }
    }

    fn revoke_session_preauthorization(
        &mut self,
        session_id: &SessionId,
        preauthorization_sha256: &str,
    ) -> Result<(), RuntimeTransportError> {
        match self.exchange(RuntimeIpcRequest::RevokeSessionPreauthorization {
            session_id: session_id.clone(),
            preauthorization_sha256: preauthorization_sha256.to_owned(),
        })? {
            RuntimeIpcResponse::PreauthorizationRevoked => Ok(()),
            _ => Err(RuntimeTransportError::RuntimeEvidenceDenied),
        }
    }

    fn release(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
    ) -> Result<(), RuntimeTransportError> {
        match self.exchange(RuntimeIpcRequest::Release {
            run_id: run_id.clone(),
            request_sha256: request_sha256.to_owned(),
        })? {
            RuntimeIpcResponse::Released => Ok(()),
            _ => Err(RuntimeTransportError::RuntimeEvidenceDenied),
        }
    }
}

/// Serves one authenticated client until explicit shutdown or transport failure.
pub fn serve_linux_runtime_ipc<P: RuntimeTransportPort>(
    session: &mut LinuxAuthenticatedIpcSession,
    runtime: &mut P,
) -> Result<(), RuntimeTransportError> {
    let client = client_scope(session.peer().identity());
    loop {
        let bytes = session
            .read_frame(MAX_WIRE_BYTES)
            .map_err(|_| RuntimeTransportError::RuntimeFailed)?;
        let request = serde_json::from_slice::<RuntimeIpcEnvelope<RuntimeIpcRequest>>(&bytes)
            .ok()
            .filter(|request| request.version == WIRE_VERSION)
            .map(|request| request.payload);
        let (response, shutdown) = answer(runtime, &client, request);
        let bytes = serde_json::to_vec(&RuntimeIpcEnvelope {
            version: WIRE_VERSION,
            payload: response,
        })
        .map_err(|_| RuntimeTransportError::RuntimeEvidenceDenied)?;
        session
            .write_frame(&bytes, MAX_WIRE_BYTES)
            .map_err(|_| RuntimeTransportError::RuntimeFailed)?;
        if shutdown {
            return Ok(());
        }
    }
}

/// The job control scope of one authenticated peer (Decision 0120): a digest
/// of the kernel-observed user, process, process start time and executable
/// digest. A restarted client process is a new client.
fn client_scope(peer: &LinuxPeerIdentity) -> RuntimeClientScope {
    let mut digest = Sha256::new();
    digest.update(b"agentmage.runtime-ipc.client-scope.v1\0");
    digest.update(peer.uid().to_be_bytes());
    digest.update(peer.pid().to_be_bytes());
    digest.update(peer.start_time_ticks().to_be_bytes());
    digest.update(peer.executable_sha256());
    let digest = digest.finalize();
    let mut scope = String::from("peer-");
    for byte in &digest[..16] {
        scope.push_str(&format!("{byte:02x}"));
    }
    RuntimeClientScope::derived(scope)
}

/// Answers one decoded request from `client`; `true` ends the service after
/// the answer. A refused or oversized answer is an error response, never a
/// transport failure.
fn answer<P: RuntimeTransportPort>(
    runtime: &mut P,
    client: &RuntimeClientScope,
    request: Option<RuntimeIpcRequest>,
) -> (RuntimeIpcResponse, bool) {
    match request {
        Some(RuntimeIpcRequest::Prepare { input }) => match runtime.prepare(input) {
            Ok(request) => (RuntimeIpcResponse::Prepared { request }, false),
            Err(error) => (RuntimeIpcResponse::Error { error }, false),
        },
        Some(RuntimeIpcRequest::Start { request }) => match runtime.start(request) {
            Ok(step) => (RuntimeIpcResponse::Step { step }, false),
            Err(error) => (RuntimeIpcResponse::Error { error }, false),
        },
        Some(RuntimeIpcRequest::Advance {
            run_id,
            request_sha256,
            after_event_cursor,
            response,
        }) => match runtime.advance(
            &run_id,
            &request_sha256,
            after_event_cursor.as_ref(),
            response.as_ref(),
        ) {
            Ok(step) => (RuntimeIpcResponse::Step { step }, false),
            Err(error) => (RuntimeIpcResponse::Error { error }, false),
        },
        Some(RuntimeIpcRequest::JobStatus {
            run_id,
            request_sha256,
        }) => match runtime.job_status(&run_id, &request_sha256) {
            Ok(status) => (RuntimeIpcResponse::JobStatus { status }, false),
            Err(error) => (RuntimeIpcResponse::Error { error }, false),
        },
        Some(RuntimeIpcRequest::ControlJob {
            run_id,
            request_sha256,
            request,
        }) => match runtime.control_job_for_client(client, &run_id, &request_sha256, &request) {
            Ok(control) => (RuntimeIpcResponse::JobControl { control }, false),
            Err(error) => (RuntimeIpcResponse::Error { error }, false),
        },
        Some(RuntimeIpcRequest::ReadArtifactPage {
            run_id,
            request_sha256,
            reference,
            offset,
            maximum_bytes,
        }) => match runtime.read_artifact_page(
            &run_id,
            &request_sha256,
            &reference,
            offset,
            maximum_bytes,
        ) {
            Ok(page) => (RuntimeIpcResponse::ArtifactPage { page }, false),
            Err(error) => (RuntimeIpcResponse::Error { error }, false),
        },
        Some(RuntimeIpcRequest::ReleaseArtifact {
            run_id,
            request_sha256,
            reference,
        }) => match runtime.release_artifact(&run_id, &request_sha256, &reference) {
            Ok(state) => (RuntimeIpcResponse::ArtifactState { state }, false),
            Err(error) => (RuntimeIpcResponse::Error { error }, false),
        },
        Some(RuntimeIpcRequest::RunDeclarations {
            run_id,
            request_sha256,
        }) => match runtime.run_declarations(&run_id, &request_sha256) {
            // An oversized answer is refused rather than ending the service.
            Ok(declarations) if declarations_fit(&declarations) => {
                (RuntimeIpcResponse::RunDeclarations { declarations }, false)
            }
            Ok(_) => (
                RuntimeIpcResponse::Error {
                    error: RuntimeTransportError::CapacityExceeded,
                },
                false,
            ),
            Err(error) => (RuntimeIpcResponse::Error { error }, false),
        },
        Some(RuntimeIpcRequest::EndedRunActionHistories { run_id }) => {
            match runtime.ended_run_action_histories(&run_id) {
                // Three full chains stay well inside the bound; anything
                // larger is refused rather than ending the service.
                Ok(histories) if answer_fits(&histories) => (
                    RuntimeIpcResponse::EndedRunActionHistories { histories },
                    false,
                ),
                Ok(_) => (
                    RuntimeIpcResponse::Error {
                        error: RuntimeTransportError::CapacityExceeded,
                    },
                    false,
                ),
                Err(error) => (RuntimeIpcResponse::Error { error }, false),
            }
        }
        Some(RuntimeIpcRequest::DocPack { request }) => match runtime.doc_pack(request) {
            // A listing or search stays well inside the bound; anything
            // larger is refused rather than ending the service.
            Ok(answer) if answer_fits(&answer) => (RuntimeIpcResponse::DocPack { answer }, false),
            Ok(_) => (
                RuntimeIpcResponse::Error {
                    error: RuntimeTransportError::CapacityExceeded,
                },
                false,
            ),
            Err(error) => (RuntimeIpcResponse::Error { error }, false),
        },
        Some(RuntimeIpcRequest::Memory { request }) => match runtime.memory(request) {
            // A listing larger than the bound is refused rather than ending
            // the service.
            Ok(answer) if answer_fits(&answer) => (RuntimeIpcResponse::Memory { answer }, false),
            Ok(_) => (
                RuntimeIpcResponse::Error {
                    error: RuntimeTransportError::CapacityExceeded,
                },
                false,
            ),
            Err(error) => (RuntimeIpcResponse::Error { error }, false),
        },
        Some(RuntimeIpcRequest::Extension { request }) => match runtime.extension(request) {
            Ok(answer) if answer_fits(&answer) => (RuntimeIpcResponse::Extension { answer }, false),
            Ok(_) => (
                RuntimeIpcResponse::Error {
                    error: RuntimeTransportError::CapacityExceeded,
                },
                false,
            ),
            Err(error) => (RuntimeIpcResponse::Error { error }, false),
        },
        Some(RuntimeIpcRequest::RevokeSessionPreauthorization {
            session_id,
            preauthorization_sha256,
        }) => {
            match runtime.revoke_session_preauthorization(&session_id, &preauthorization_sha256) {
                Ok(()) => (RuntimeIpcResponse::PreauthorizationRevoked, false),
                Err(error) => (RuntimeIpcResponse::Error { error }, false),
            }
        }
        Some(RuntimeIpcRequest::Release {
            run_id,
            request_sha256,
        }) => match runtime.release(&run_id, &request_sha256) {
            Ok(()) => (RuntimeIpcResponse::Released, false),
            Err(error) => (RuntimeIpcResponse::Error { error }, false),
        },
        Some(RuntimeIpcRequest::Shutdown) => (RuntimeIpcResponse::Shutdown, true),
        None => (
            RuntimeIpcResponse::Error {
                error: RuntimeTransportError::RequestDenied,
            },
            false,
        ),
    }
}

fn declarations_fit(declarations: &RuntimeRunDeclarations) -> bool {
    answer_fits(declarations)
}

fn answer_fits(answer: &impl Serialize) -> bool {
    serde_json::to_vec(answer).is_ok_and(|bytes| bytes.len() <= MAX_DECLARATION_BYTES)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_client() -> RuntimeClientScope {
        client_scope(&LinuxPeerIdentity::new(1_000, 4_242, 77, [9; 32]))
    }

    /// A local-only receipt with one eligible strict-local route.
    fn fixture_route_receipt() -> crate::coding_route::RunRouteReceipt {
        use crate::coding_route::{
            DEVELOPMENT_ROUTED_DATA, RunRouteAudit, RunRouteMode, RunRouteReceipt,
        };
        let mut receipt = RunRouteReceipt {
            schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
            request_id: "run-declarations".to_owned(),
            route_policy_sha256: "4".repeat(64),
            considered_routes: vec![RunRouteAudit {
                route_id: "route-local".to_owned(),
                candidate_sha256: "5".repeat(64),
                eligible: true,
                reason_code: "model-gateway.route.eligible".to_owned(),
            }],
            selected_route_id: Some("route-local".to_owned()),
            fallback_used: false,
            fallback_policy_sha256: None,
            disclosure_class: Some(agentmage_kernel_contracts::CanonicalEndpointClass::StrictLocal),
            mode: RunRouteMode::LocalOnly,
            transmitted_data: DEVELOPMENT_ROUTED_DATA.to_vec(),
            hybrid_grant_sha256: None,
            provider_id: None,
            reason_code: "model-gateway.route.qualified-selected".to_owned(),
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt.computed_sha256().unwrap();
        receipt
    }

    /// One kept entry of the given kind, refused by a person without authority.
    fn fixture_history(
        kind: agentmage_kernel_engine::action_history::ActionKind,
    ) -> crate::coding_action_history::RunActionHistory {
        use agentmage_kernel_engine::action_history::{
            ActionAuthorization, ActionOutcome, ActionRecordDraft,
        };
        let mut recorder = crate::coding_action_history::RunActionRecorder::new();
        recorder.record(Some(ActionRecordDraft {
            action_kind: kind,
            action_id: "operation:0123456789abcdef0123456789abcdef".to_owned(),
            authorization: ActionAuthorization::Unauthorized {},
            effect_sha256: "2".repeat(64),
            outcome: ActionOutcome::Denied,
            reason_code: "runtime.coding.user-denied".to_owned(),
            evidence_sha256s: vec!["3".repeat(64)],
            recorded_at_epoch_ms: 1,
            retain_until_epoch_ms: 2,
        }));
        recorder.declare().unwrap()
    }

    #[test]
    fn run_declarations_cross_the_wire_exactly_and_only_at_the_current_version() {
        use agentmage_kernel_engine::action_history::ActionKind;
        let declarations = RuntimeRunDeclarations {
            schema_version: crate::runtime_transport::RUN_DECLARATIONS_SCHEMA_VERSION,
            run_id: RuntimeRunId::from_raw("run-declarations"),
            request_sha256: "1".repeat(64),
            recoverability: None,
            context_inspections: Some(Vec::new()),
            effect_history: Some(fixture_history(ActionKind::FileWrite)),
            job_control_history: Some(fixture_history(ActionKind::JobControl)),
            route_receipt: Some(fixture_route_receipt()),
            route_history: Some(fixture_history(ActionKind::ModelRoute)),
            recipe_plan: None,
        };
        let request = RuntimeIpcEnvelope {
            version: WIRE_VERSION,
            payload: RuntimeIpcRequest::RunDeclarations {
                run_id: declarations.run_id.clone(),
                request_sha256: declarations.request_sha256.clone(),
            },
        };
        let bytes = serde_json::to_vec(&request).unwrap();
        let decoded: RuntimeIpcEnvelope<RuntimeIpcRequest> =
            serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded.version, 15);
        assert_eq!(decoded.payload, request.payload);
        let response = RuntimeIpcResponse::RunDeclarations {
            declarations: declarations.clone(),
        };
        let bytes = serde_json::to_vec(&response).unwrap();
        assert_eq!(
            serde_json::from_slice::<RuntimeIpcResponse>(&bytes).unwrap(),
            response
        );
        assert!(declarations_fit(&declarations));
        // A field the closed contract does not name is refused, at the top
        // and inside each history (Decision 0127), also through the frame.
        let mut extra = serde_json::to_value(&declarations).unwrap();
        extra["complete"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<RuntimeRunDeclarations>(extra).is_err());
        let text = String::from_utf8(bytes).unwrap();
        for nested in [
            text.replacen(
                "{\"kind\":\"unauthorized\"}",
                "{\"kind\":\"unauthorized\",\"grant_id\":\"grant-1\"}",
                1,
            ),
            text.replacen("\"state\":\"kept\"", "\"state\":\"kept\",\"x\":1", 1),
            text.replacen("\"head\":{", "\"head\":{\"x\":1,", 1),
            text.replacen(
                "\"job_control_history\":{",
                "\"job_control_history\":{\"x\":1,",
                1,
            ),
            // Decision 0128: the route receipt parses closed, with every
            // member required, and so does each audit inside it.
            text.replacen("\"route_receipt\":{", "\"route_receipt\":{\"x\":1,", 1),
            text.replacen("\"provider_id\":null,", "", 1),
            text.replacen("\"eligible\":true,", "\"eligible\":true,\"x\":1,", 1),
            text.replacen("\"route_history\":{", "\"route_history\":{\"x\":1,", 1),
        ] {
            assert_ne!(nested, text);
            assert!(serde_json::from_str::<RuntimeIpcResponse>(&nested).is_err());
        }
    }

    #[test]
    fn an_ended_run_answer_crosses_the_wire_closed_and_only_at_the_current_version() {
        // Decision 0129: the stored histories of an ended run cross the wire
        // closed, and a transport without the operation refuses it.
        use crate::coding_action_history::{
            ENDED_RUN_HISTORIES_SCHEMA_VERSION, EndedRunActionHistories, StoredRunChain,
        };
        use agentmage_kernel_engine::action_history::ActionKind;
        let history = fixture_history(ActionKind::JobControl);
        let histories = EndedRunActionHistories {
            schema_version: ENDED_RUN_HISTORIES_SCHEMA_VERSION,
            run_id: "run-ended".to_owned(),
            effects: None,
            job_control: Some(StoredRunChain {
                records: history.records.clone(),
                head: history.head.clone(),
                complete: true,
                closed: true,
            }),
            routes: None,
        };
        let request = RuntimeIpcEnvelope {
            version: WIRE_VERSION,
            payload: RuntimeIpcRequest::EndedRunActionHistories {
                run_id: RuntimeRunId::from_raw("run-ended"),
            },
        };
        let bytes = serde_json::to_vec(&request).unwrap();
        let decoded: RuntimeIpcEnvelope<RuntimeIpcRequest> =
            serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded.version, 15);
        assert_eq!(decoded.payload, request.payload);
        let response = RuntimeIpcResponse::EndedRunActionHistories {
            histories: histories.clone(),
        };
        let bytes = serde_json::to_vec(&response).unwrap();
        assert_eq!(
            serde_json::from_slice::<RuntimeIpcResponse>(&bytes).unwrap(),
            response
        );
        let text = String::from_utf8(bytes).unwrap();
        for nested in [
            text.replacen("\"closed\":true", "\"closed\":true,\"x\":1", 1),
            text.replacen("\"effects\":null,", "", 1),
            text.replacen(
                "\"run_id\":\"run-ended\"",
                "\"run_id\":\"run-ended\",\"x\":1",
                1,
            ),
        ] {
            assert_ne!(nested, text);
            assert!(serde_json::from_str::<RuntimeIpcResponse>(&nested).is_err());
        }
        // A port without stored histories refuses the operation, and the
        // service keeps answering.
        let mut port = DeclaringPort {
            session_id_bytes: 0,
            released: 0,
        };
        assert_eq!(
            answer(
                &mut port,
                &fixture_client(),
                Some(RuntimeIpcRequest::EndedRunActionHistories {
                    run_id: RuntimeRunId::from_raw("run-ended"),
                })
            ),
            (
                RuntimeIpcResponse::Error {
                    error: RuntimeTransportError::RequestDenied,
                },
                false
            )
        );
        assert!(answer_fits(&histories));
    }

    #[test]
    fn the_largest_chunk_of_documentation_text_fits_one_frame() {
        // Decision 0132: the client sends only documentation text, which JSON
        // escaping at most doubles, so the largest chunk fits one frame; text
        // with other control characters would not, and is refused before any
        // host is launched.
        use crate::coding_doc_packs::MAX_DOC_PACK_CHUNK_BYTES;
        let frame = |text: String| {
            serde_json::to_vec(&RuntimeIpcEnvelope {
                version: WIRE_VERSION,
                payload: RuntimeIpcRequest::DocPack {
                    request: DocPackRequest::ImportChunk {
                        path: format!("{}/notes.txt", "d".repeat(4_000)),
                        offset: u64::MAX,
                        text,
                    },
                },
            })
            .unwrap()
            .len()
        };
        let escaped = "\"\\\n\r\t".repeat(MAX_DOC_PACK_CHUNK_BYTES / 5);
        assert!(escaped.len() <= MAX_DOC_PACK_CHUNK_BYTES);
        assert!(agentmage_capability_knowledge::doc_pack_text_allowed(
            &escaped
        ));
        assert!(frame(escaped) <= MAX_WIRE_BYTES);
        let control = "\u{1}".repeat(MAX_DOC_PACK_CHUNK_BYTES);
        assert!(!agentmage_capability_knowledge::doc_pack_text_allowed(
            &control
        ));
        assert!(frame(control) > MAX_WIRE_BYTES);
    }

    #[test]
    fn a_documentation_pack_request_crosses_the_wire_closed() {
        // Decision 0130: documentation pack requests and answers cross the
        // wire closed; a port that is not a catalog host refuses them, and an
        // oversized answer is refused without ending the service.
        use crate::coding_doc_packs::{DocPackHitView, DocPackRefusal};
        use agentmage_capability_knowledge::DocPackVersion;
        let request = RuntimeIpcEnvelope {
            version: WIRE_VERSION,
            payload: RuntimeIpcRequest::DocPack {
                request: DocPackRequest::Search {
                    terms: vec!["cache".to_owned()],
                    pack_id: None,
                    include_history: false,
                },
            },
        };
        let bytes = serde_json::to_vec(&request).unwrap();
        let decoded: RuntimeIpcEnvelope<RuntimeIpcRequest> =
            serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded.version, 15);
        assert_eq!(decoded.payload, request.payload);
        let text = String::from_utf8(bytes).unwrap();
        for nested in [
            text.replacen(
                "\"include_history\":false",
                "\"include_history\":false,\"x\":1",
                1,
            ),
            text.replacen("\"pack_id\":null,", "", 1),
            text.replacen("\"operation\":\"search\"", "\"operation\":\"upload\"", 1),
        ] {
            assert_ne!(nested, text);
            assert!(
                serde_json::from_str::<RuntimeIpcEnvelope<RuntimeIpcRequest>>(&nested).is_err()
            );
        }
        let hit = DocPackHitView {
            pack_id: "build-tool-guide".to_owned(),
            version: DocPackVersion {
                major: 1,
                minor: 0,
                patch: 0,
            },
            historical: false,
            path: "guide/cache.md".to_owned(),
            start_line: 1,
            end_line: 1,
            heading: true,
            text: "# Build cache".to_owned(),
            citation_sha256: "c".repeat(64),
        };
        let response = RuntimeIpcResponse::DocPack {
            answer: DocPackAnswer::Found {
                hits: vec![hit],
                omitted: 0,
                retention: Vec::new(),
            },
        };
        let bytes = serde_json::to_vec(&response).unwrap();
        assert_eq!(
            serde_json::from_slice::<RuntimeIpcResponse>(&bytes).unwrap(),
            response
        );
        let text = String::from_utf8(bytes).unwrap();
        for nested in [
            text.replacen("\"heading\":true", "\"heading\":true,\"x\":1", 1),
            text.replacen("\"omitted\":0", "\"omitted\":0,\"x\":1", 1),
            text.replacen("\"answer\":\"found\"", "\"answer\":\"uploaded\"", 1),
        ] {
            assert_ne!(nested, text);
            assert!(serde_json::from_str::<RuntimeIpcResponse>(&nested).is_err());
        }
        let mut port = DeclaringPort {
            session_id_bytes: 0,
            released: 0,
        };
        assert_eq!(
            answer(
                &mut port,
                &fixture_client(),
                Some(RuntimeIpcRequest::DocPack {
                    request: DocPackRequest::List {},
                })
            ),
            (
                RuntimeIpcResponse::Error {
                    error: RuntimeTransportError::RequestDenied,
                },
                false
            )
        );
        struct CatalogPort(DocPackAnswer);
        impl RuntimeTransportPort for CatalogPort {
            fn prepare(
                &mut self,
                _input: RuntimePrepareInput,
            ) -> Result<RuntimeRunRequest, RuntimeTransportError> {
                Err(RuntimeTransportError::RequestDenied)
            }
            fn start(
                &mut self,
                _request: RuntimeRunRequest,
            ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
                Err(RuntimeTransportError::RequestDenied)
            }
            fn advance(
                &mut self,
                _run_id: &RuntimeRunId,
                _request_sha256: &str,
                _after_event_cursor: Option<&RuntimeEventCursor>,
                _response: Option<&RuntimeApprovalResponse>,
            ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
                Err(RuntimeTransportError::RequestDenied)
            }
            fn cancel(
                &mut self,
                _run_id: &RuntimeRunId,
                _request_sha256: &str,
                _cancellation_id: CancellationId,
                _after_event_cursor: Option<&RuntimeEventCursor>,
            ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
                Err(RuntimeTransportError::RequestDenied)
            }
            fn release(
                &mut self,
                _run_id: &RuntimeRunId,
                _request_sha256: &str,
            ) -> Result<(), RuntimeTransportError> {
                Err(RuntimeTransportError::RequestDenied)
            }
            fn doc_pack(
                &mut self,
                _request: DocPackRequest,
            ) -> Result<DocPackAnswer, RuntimeTransportError> {
                Ok(self.0.clone())
            }
        }
        let refused = DocPackAnswer::Refused {
            refusal: DocPackRefusal::NotFound,
        };
        assert_eq!(
            answer(
                &mut CatalogPort(refused.clone()),
                &fixture_client(),
                Some(RuntimeIpcRequest::DocPack {
                    request: DocPackRequest::List {},
                })
            ),
            (RuntimeIpcResponse::DocPack { answer: refused }, false)
        );
        let oversized = DocPackAnswer::Found {
            hits: vec![
                DocPackHitView {
                    pack_id: "build-tool-guide".to_owned(),
                    version: DocPackVersion {
                        major: 1,
                        minor: 0,
                        patch: 0,
                    },
                    historical: false,
                    path: "guide/cache.md".to_owned(),
                    start_line: 1,
                    end_line: 1,
                    heading: false,
                    text: "x".repeat(MAX_WIRE_BYTES),
                    citation_sha256: "c".repeat(64),
                };
                1
            ],
            omitted: 0,
            retention: Vec::new(),
        };
        assert_eq!(
            answer(
                &mut CatalogPort(oversized),
                &fixture_client(),
                Some(RuntimeIpcRequest::DocPack {
                    request: DocPackRequest::List {},
                })
            ),
            (
                RuntimeIpcResponse::Error {
                    error: RuntimeTransportError::CapacityExceeded,
                },
                false
            )
        );
    }

    #[test]
    fn a_memory_request_crosses_the_wire_closed() {
        // Decision 0131: memory requests and answers cross the wire closed; a
        // port that is not a catalog host refuses them, and an oversized
        // listing is refused without ending the service.
        use crate::coding_memory::{
            MemoryItemView, MemoryRefusal, MemorySourceView, MemoryStatusView, MemoryTypeView,
        };
        let request = RuntimeIpcEnvelope {
            version: WIRE_VERSION,
            payload: RuntimeIpcRequest::Memory {
                request: MemoryRequest::RevokeSource {
                    workspace_id: "workspace-a".to_owned(),
                    source_id: "doc-pack:guide:1.0.0".to_owned(),
                    object_id: None,
                },
            },
        };
        let bytes = serde_json::to_vec(&request).unwrap();
        let decoded: RuntimeIpcEnvelope<RuntimeIpcRequest> =
            serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded.version, 15);
        assert_eq!(decoded.payload, request.payload);
        let text = String::from_utf8(bytes).unwrap();
        for nested in [
            text.replacen("\"object_id\":null", "\"object_id\":null,\"x\":1", 1),
            text.replacen(",\"object_id\":null", "", 1),
            text.replacen(
                "\"operation\":\"revoke-source\"",
                "\"operation\":\"forget\"",
                1,
            ),
        ] {
            assert_ne!(nested, text);
            assert!(
                serde_json::from_str::<RuntimeIpcEnvelope<RuntimeIpcRequest>>(&nested).is_err()
            );
        }
        let item = MemoryItemView {
            memory_id: "memory-0123456789abcdef".to_owned(),
            memory_type: Some(MemoryTypeView::Semantic),
            workspace_id: "workspace-a".to_owned(),
            status: MemoryStatusView::Approved,
            content: Some("The cache lives in build/cache.".to_owned()),
            sources: vec![MemorySourceView {
                source_id: "doc-pack:guide:1.0.0".to_owned(),
                object_id: "path:guide:cache.md".to_owned(),
                content_sha256: "c".repeat(64),
            }],
            created_at: "2026-10-01T00:00:00Z".to_owned(),
            decided_at: "2026-10-01T00:00:00Z".to_owned(),
        };
        let response = RuntimeIpcResponse::Memory {
            answer: MemoryAnswer::Listed {
                items: vec![item.clone()],
                catalog_revision: 1,
            },
        };
        let bytes = serde_json::to_vec(&response).unwrap();
        assert_eq!(
            serde_json::from_slice::<RuntimeIpcResponse>(&bytes).unwrap(),
            response
        );
        let text = String::from_utf8(bytes).unwrap();
        for nested in [
            text.replacen(
                "\"status\":\"approved\"",
                "\"status\":\"approved\",\"x\":1",
                1,
            ),
            text.replacen(
                "\"catalog_revision\":1",
                "\"catalog_revision\":1,\"x\":1",
                1,
            ),
            text.replacen("\"status\":\"approved\"", "\"status\":\"forgotten\"", 1),
            text.replacen("\"answer\":\"listed\"", "\"answer\":\"forgotten\"", 1),
        ] {
            assert_ne!(nested, text);
            assert!(serde_json::from_str::<RuntimeIpcResponse>(&nested).is_err());
        }
        let list = || {
            Some(RuntimeIpcRequest::Memory {
                request: MemoryRequest::List { workspace_id: None },
            })
        };
        let mut port = DeclaringPort {
            session_id_bytes: 0,
            released: 0,
        };
        assert_eq!(
            answer(&mut port, &fixture_client(), list()),
            (
                RuntimeIpcResponse::Error {
                    error: RuntimeTransportError::RequestDenied,
                },
                false
            )
        );
        struct MemoryPort(MemoryAnswer);
        impl RuntimeTransportPort for MemoryPort {
            fn prepare(
                &mut self,
                _input: RuntimePrepareInput,
            ) -> Result<RuntimeRunRequest, RuntimeTransportError> {
                Err(RuntimeTransportError::RequestDenied)
            }
            fn start(
                &mut self,
                _request: RuntimeRunRequest,
            ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
                Err(RuntimeTransportError::RequestDenied)
            }
            fn advance(
                &mut self,
                _run_id: &RuntimeRunId,
                _request_sha256: &str,
                _after_event_cursor: Option<&RuntimeEventCursor>,
                _response: Option<&RuntimeApprovalResponse>,
            ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
                Err(RuntimeTransportError::RequestDenied)
            }
            fn cancel(
                &mut self,
                _run_id: &RuntimeRunId,
                _request_sha256: &str,
                _cancellation_id: CancellationId,
                _after_event_cursor: Option<&RuntimeEventCursor>,
            ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
                Err(RuntimeTransportError::RequestDenied)
            }
            fn release(
                &mut self,
                _run_id: &RuntimeRunId,
                _request_sha256: &str,
            ) -> Result<(), RuntimeTransportError> {
                Err(RuntimeTransportError::RequestDenied)
            }
            fn memory(
                &mut self,
                _request: MemoryRequest,
            ) -> Result<MemoryAnswer, RuntimeTransportError> {
                Ok(self.0.clone())
            }
        }
        let refused = MemoryAnswer::Refused {
            refusal: MemoryRefusal::NotFound,
        };
        assert_eq!(
            answer(&mut MemoryPort(refused.clone()), &fixture_client(), list()),
            (RuntimeIpcResponse::Memory { answer: refused }, false)
        );
        let oversized = MemoryAnswer::Listed {
            items: vec![
                MemoryItemView {
                    content: Some("x".repeat(MAX_WIRE_BYTES)),
                    ..item
                };
                1
            ],
            catalog_revision: 1,
        };
        assert_eq!(
            answer(&mut MemoryPort(oversized), &fixture_client(), list()),
            (
                RuntimeIpcResponse::Error {
                    error: RuntimeTransportError::CapacityExceeded,
                },
                false
            )
        );
    }

    #[test]
    fn an_extension_request_crosses_the_wire_closed() {
        // Decision 0132: extension requests and answers cross wire 14 closed;
        // a port that is not a catalog host refuses them, and an oversized
        // listing is refused without ending the service.
        use crate::coding_extensions::{
            ExtensionKeyRole, ExtensionKeyView, ExtensionRefusal, ExtensionScopeView,
        };
        let request = RuntimeIpcEnvelope {
            version: WIRE_VERSION,
            payload: RuntimeIpcRequest::Extension {
                request: ExtensionRequest::Uninstall {
                    workspace_id: "workspace-a".to_owned(),
                    package_id: "sample-formatter".to_owned(),
                },
            },
        };
        let bytes = serde_json::to_vec(&request).unwrap();
        let decoded: RuntimeIpcEnvelope<RuntimeIpcRequest> =
            serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded.version, 15);
        assert_eq!(decoded.payload, request.payload);
        let text = String::from_utf8(bytes).unwrap();
        for nested in [
            text.replacen(
                "\"package_id\":\"sample-formatter\"",
                "\"package_id\":\"sample-formatter\",\"x\":1",
                1,
            ),
            text.replacen(",\"package_id\":\"sample-formatter\"", "", 1),
            text.replacen("\"operation\":\"uninstall\"", "\"operation\":\"enable\"", 1),
        ] {
            assert_ne!(nested, text);
            assert!(
                serde_json::from_str::<RuntimeIpcEnvelope<RuntimeIpcRequest>>(&nested).is_err()
            );
        }
        let scope = ExtensionScopeView {
            workspace_id: "workspace-a".to_owned(),
            keys: vec![ExtensionKeyView {
                role: ExtensionKeyRole::RevocationIssuer,
                key_id: "agentmage-sample-issuer".to_owned(),
                key_sha256: "c".repeat(64),
            }],
            extensions: Vec::new(),
            revocations: None,
        };
        let response = RuntimeIpcResponse::Extension {
            answer: ExtensionAnswer::Listed {
                scopes: vec![scope.clone()],
                catalog_revision: 1,
            },
        };
        let bytes = serde_json::to_vec(&response).unwrap();
        assert_eq!(
            serde_json::from_slice::<RuntimeIpcResponse>(&bytes).unwrap(),
            response
        );
        let text = String::from_utf8(bytes).unwrap();
        for nested in [
            text.replacen(
                "\"role\":\"revocation-issuer\"",
                "\"role\":\"revocation-issuer\",\"x\":1",
                1,
            ),
            text.replacen("\"revocations\":null", "\"revocations\":null,\"x\":1", 1),
            text.replacen(",\"revocations\":null", "", 1),
            text.replacen("\"role\":\"revocation-issuer\"", "\"role\":\"owner\"", 1),
            text.replacen("\"answer\":\"listed\"", "\"answer\":\"enabled\"", 1),
        ] {
            assert_ne!(nested, text);
            assert!(serde_json::from_str::<RuntimeIpcResponse>(&nested).is_err());
        }
        let list = || {
            Some(RuntimeIpcRequest::Extension {
                request: ExtensionRequest::List { workspace_id: None },
            })
        };
        let mut port = DeclaringPort {
            session_id_bytes: 0,
            released: 0,
        };
        assert_eq!(
            answer(&mut port, &fixture_client(), list()),
            (
                RuntimeIpcResponse::Error {
                    error: RuntimeTransportError::RequestDenied,
                },
                false
            )
        );
        struct ExtensionPort(ExtensionAnswer);
        impl RuntimeTransportPort for ExtensionPort {
            fn prepare(
                &mut self,
                _input: RuntimePrepareInput,
            ) -> Result<RuntimeRunRequest, RuntimeTransportError> {
                Err(RuntimeTransportError::RequestDenied)
            }
            fn start(
                &mut self,
                _request: RuntimeRunRequest,
            ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
                Err(RuntimeTransportError::RequestDenied)
            }
            fn advance(
                &mut self,
                _run_id: &RuntimeRunId,
                _request_sha256: &str,
                _after_event_cursor: Option<&RuntimeEventCursor>,
                _response: Option<&RuntimeApprovalResponse>,
            ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
                Err(RuntimeTransportError::RequestDenied)
            }
            fn cancel(
                &mut self,
                _run_id: &RuntimeRunId,
                _request_sha256: &str,
                _cancellation_id: CancellationId,
                _after_event_cursor: Option<&RuntimeEventCursor>,
            ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
                Err(RuntimeTransportError::RequestDenied)
            }
            fn release(
                &mut self,
                _run_id: &RuntimeRunId,
                _request_sha256: &str,
            ) -> Result<(), RuntimeTransportError> {
                Err(RuntimeTransportError::RequestDenied)
            }
            fn extension(
                &mut self,
                _request: ExtensionRequest,
            ) -> Result<ExtensionAnswer, RuntimeTransportError> {
                Ok(self.0.clone())
            }
        }
        let refused = ExtensionAnswer::Refused {
            refusal: ExtensionRefusal::RevocationsStale,
        };
        assert_eq!(
            answer(
                &mut ExtensionPort(refused.clone()),
                &fixture_client(),
                list()
            ),
            (RuntimeIpcResponse::Extension { answer: refused }, false)
        );
        let oversized = ExtensionAnswer::Listed {
            scopes: vec![
                ExtensionScopeView {
                    workspace_id: "x".repeat(MAX_WIRE_BYTES),
                    ..scope
                };
                1
            ],
            catalog_revision: 1,
        };
        assert_eq!(
            answer(&mut ExtensionPort(oversized), &fixture_client(), list()),
            (
                RuntimeIpcResponse::Error {
                    error: RuntimeTransportError::CapacityExceeded,
                },
                false
            )
        );
    }

    /// Answers declarations of a chosen size and counts releases.
    struct DeclaringPort {
        session_id_bytes: usize,
        released: usize,
    }

    impl RuntimeTransportPort for DeclaringPort {
        fn prepare(
            &mut self,
            _input: RuntimePrepareInput,
        ) -> Result<RuntimeRunRequest, RuntimeTransportError> {
            Err(RuntimeTransportError::RequestDenied)
        }

        fn start(
            &mut self,
            _request: RuntimeRunRequest,
        ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
            Err(RuntimeTransportError::RequestDenied)
        }

        fn advance(
            &mut self,
            _run_id: &RuntimeRunId,
            _request_sha256: &str,
            _after_event_cursor: Option<&RuntimeEventCursor>,
            _response: Option<&RuntimeApprovalResponse>,
        ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
            Err(RuntimeTransportError::RequestDenied)
        }

        fn cancel(
            &mut self,
            _run_id: &RuntimeRunId,
            _request_sha256: &str,
            _cancellation_id: CancellationId,
            _after_event_cursor: Option<&RuntimeEventCursor>,
        ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
            Err(RuntimeTransportError::RequestDenied)
        }

        fn run_declarations(
            &mut self,
            run_id: &RuntimeRunId,
            request_sha256: &str,
        ) -> Result<RuntimeRunDeclarations, RuntimeTransportError> {
            let report = crate::coding_recoverability::RecoverabilityReport {
                schema_version: 1,
                session_id: "s".repeat(self.session_id_bytes),
                task_id: "task-declarations".to_owned(),
                run_id: Some(run_id.as_str().to_owned()),
                assessments: Vec::new(),
                revert_order: Vec::new(),
                fully_recoverable: true,
                requires_reconciliation: false,
                report_sha256: "0".repeat(64),
            };
            Ok(RuntimeRunDeclarations {
                schema_version: crate::runtime_transport::RUN_DECLARATIONS_SCHEMA_VERSION,
                run_id: run_id.clone(),
                request_sha256: request_sha256.to_owned(),
                recoverability: Some(report),
                context_inspections: None,
                effect_history: None,
                job_control_history: None,
                route_receipt: None,
                route_history: None,
                recipe_plan: None,
            })
        }

        fn release(
            &mut self,
            _run_id: &RuntimeRunId,
            _request_sha256: &str,
        ) -> Result<(), RuntimeTransportError> {
            self.released += 1;
            Ok(())
        }
    }

    #[test]
    fn oversized_run_declarations_are_refused_and_the_service_keeps_answering() {
        // Review V3 of 8fbd2bc6: an answer over the declaration bound is a
        // capacity refusal, not a transport failure, and later requests are
        // still answered.
        let declare = || {
            Some(RuntimeIpcRequest::RunDeclarations {
                run_id: RuntimeRunId::from_raw("run-declarations"),
                request_sha256: "1".repeat(64),
            })
        };
        let mut port = DeclaringPort {
            session_id_bytes: MAX_DECLARATION_BYTES,
            released: 0,
        };
        assert_eq!(
            answer(&mut port, &fixture_client(), declare()),
            (
                RuntimeIpcResponse::Error {
                    error: RuntimeTransportError::CapacityExceeded,
                },
                false
            )
        );
        assert_eq!(
            answer(
                &mut port,
                &fixture_client(),
                Some(RuntimeIpcRequest::Release {
                    run_id: RuntimeRunId::from_raw("run-declarations"),
                    request_sha256: "1".repeat(64),
                })
            ),
            (RuntimeIpcResponse::Released, false)
        );
        assert_eq!(port.released, 1);
        // Just within the bound, the same answer crosses the wire.
        port.session_id_bytes = 0;
        let (RuntimeIpcResponse::RunDeclarations { declarations }, false) =
            answer(&mut port, &fixture_client(), declare())
        else {
            panic!("a small declaration is answered");
        };
        let envelope = serde_json::to_vec(&declarations).unwrap().len();
        port.session_id_bytes = MAX_DECLARATION_BYTES - envelope;
        let (RuntimeIpcResponse::RunDeclarations { declarations }, false) =
            answer(&mut port, &fixture_client(), declare())
        else {
            panic!("a declaration at the bound is answered");
        };
        assert_eq!(
            serde_json::to_vec(&declarations).unwrap().len(),
            MAX_DECLARATION_BYTES
        );
        port.session_id_bytes += 1;
        assert!(matches!(
            answer(&mut port, &fixture_client(), declare()),
            (
                RuntimeIpcResponse::Error {
                    error: RuntimeTransportError::CapacityExceeded
                },
                false
            )
        ));
        // A request the closed contract cannot decode is refused, not fatal;
        // only an explicit shutdown ends the service.
        assert_eq!(
            answer(&mut port, &fixture_client(), None),
            (
                RuntimeIpcResponse::Error {
                    error: RuntimeTransportError::RequestDenied,
                },
                false
            )
        );
        assert_eq!(
            answer(
                &mut port,
                &fixture_client(),
                Some(RuntimeIpcRequest::Shutdown)
            ),
            (RuntimeIpcResponse::Shutdown, true)
        );
    }

    fn fixture_status() -> RuntimeJobStatus {
        RuntimeJobStatus {
            schema_version: 1,
            run_id: RuntimeRunId::from_raw("run-job-control"),
            request_sha256: "1".repeat(64),
            job: agentmage_kernel_engine::job_control::JobObservation {
                job_id: "run-job-control".to_owned(),
                phase: agentmage_kernel_engine::job_control::JobPhase::Running,
                revision: 1,
                cancellation_requested: false,
                head_sha256: "2".repeat(64),
            },
        }
    }

    fn fixture_control_request() -> JobControlRequest {
        JobControlRequest {
            schema_version: 1,
            job_id: "run-job-control".to_owned(),
            request_id: "cancel-fixture-0".to_owned(),
            action: agentmage_kernel_engine::job_control::JobControlAction::Cancel,
            observed_revision: 1,
        }
    }

    #[test]
    fn job_control_crosses_the_wire_without_a_client_scope_or_a_direct_cancel() {
        // Decision 0120: a client asks for status and names a control request;
        // it never names its scope, and a direct cancellation is not an
        // operation of this wire.
        for request in [
            RuntimeIpcRequest::JobStatus {
                run_id: RuntimeRunId::from_raw("run-job-control"),
                request_sha256: "1".repeat(64),
            },
            RuntimeIpcRequest::ControlJob {
                run_id: RuntimeRunId::from_raw("run-job-control"),
                request_sha256: "1".repeat(64),
                request: fixture_control_request(),
            },
        ] {
            let bytes = serde_json::to_vec(&RuntimeIpcEnvelope {
                version: WIRE_VERSION,
                payload: request.clone(),
            })
            .unwrap();
            let decoded: RuntimeIpcEnvelope<RuntimeIpcRequest> =
                serde_json::from_slice(&bytes).unwrap();
            assert_eq!(decoded.version, 15);
            assert_eq!(decoded.payload, request);
        }
        let mut scoped = serde_json::to_value(RuntimeIpcRequest::ControlJob {
            run_id: RuntimeRunId::from_raw("run-job-control"),
            request_sha256: "1".repeat(64),
            request: fixture_control_request(),
        })
        .unwrap();
        scoped["client_scope"] = serde_json::Value::String("peer-chosen".to_owned());
        assert!(serde_json::from_value::<RuntimeIpcRequest>(scoped.clone()).is_err());
        let mut nested = serde_json::to_value(fixture_control_request()).unwrap();
        nested["client_scope"] = serde_json::Value::String("peer-chosen".to_owned());
        assert!(serde_json::from_value::<JobControlRequest>(nested).is_err());
        let cancel = serde_json::json!({
            "operation": "cancel",
            "run_id": "run-job-control",
            "request_sha256": "1".repeat(64),
            "cancellation_id": "cancel-fixture",
            "after_event_cursor": null,
        });
        assert!(serde_json::from_value::<RuntimeIpcRequest>(cancel).is_err());
        for response in [
            RuntimeIpcResponse::JobStatus {
                status: fixture_status(),
            },
            RuntimeIpcResponse::JobControl {
                control: RuntimeJobControl {
                    decision: agentmage_kernel_engine::job_control::JobControlDecision::Applied {
                        revision: 2,
                        phase: agentmage_kernel_engine::job_control::JobPhase::Cancelling,
                    },
                    status: fixture_status(),
                },
            },
        ] {
            let bytes = serde_json::to_vec(&response).unwrap();
            assert_eq!(
                serde_json::from_slice::<RuntimeIpcResponse>(&bytes).unwrap(),
                response
            );
        }
        let mut extra = serde_json::to_value(fixture_status()).unwrap();
        extra["job"]["owner_id"] = serde_json::Value::String("owner".to_owned());
        assert!(serde_json::from_value::<RuntimeJobStatus>(extra).is_err());
    }

    /// Records the scope of each control request it decides.
    #[derive(Default)]
    struct ScopeRecordingPort {
        scopes: Vec<String>,
    }

    impl RuntimeTransportPort for ScopeRecordingPort {
        fn prepare(
            &mut self,
            _input: RuntimePrepareInput,
        ) -> Result<RuntimeRunRequest, RuntimeTransportError> {
            Err(RuntimeTransportError::RequestDenied)
        }

        fn start(
            &mut self,
            _request: RuntimeRunRequest,
        ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
            Err(RuntimeTransportError::RequestDenied)
        }

        fn advance(
            &mut self,
            _run_id: &RuntimeRunId,
            _request_sha256: &str,
            _after_event_cursor: Option<&RuntimeEventCursor>,
            _response: Option<&RuntimeApprovalResponse>,
        ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
            Err(RuntimeTransportError::RequestDenied)
        }

        fn cancel(
            &mut self,
            _run_id: &RuntimeRunId,
            _request_sha256: &str,
            _cancellation_id: CancellationId,
            _after_event_cursor: Option<&RuntimeEventCursor>,
        ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
            Err(RuntimeTransportError::RequestDenied)
        }

        fn job_status(
            &mut self,
            _run_id: &RuntimeRunId,
            _request_sha256: &str,
        ) -> Result<RuntimeJobStatus, RuntimeTransportError> {
            Ok(fixture_status())
        }

        fn control_job(
            &mut self,
            _run_id: &RuntimeRunId,
            _request_sha256: &str,
            _request: &JobControlRequest,
        ) -> Result<RuntimeJobControl, RuntimeTransportError> {
            panic!("the serving host decides only for the scope it derived")
        }

        fn control_job_for_client(
            &mut self,
            client: &RuntimeClientScope,
            _run_id: &RuntimeRunId,
            _request_sha256: &str,
            _request: &JobControlRequest,
        ) -> Result<RuntimeJobControl, RuntimeTransportError> {
            self.scopes.push(client.as_str().to_owned());
            Ok(RuntimeJobControl {
                decision: agentmage_kernel_engine::job_control::JobControlDecision::Applied {
                    revision: 2,
                    phase: agentmage_kernel_engine::job_control::JobPhase::Cancelling,
                },
                status: fixture_status(),
            })
        }

        fn release(
            &mut self,
            _run_id: &RuntimeRunId,
            _request_sha256: &str,
        ) -> Result<(), RuntimeTransportError> {
            Err(RuntimeTransportError::RequestDenied)
        }
    }

    #[test]
    fn the_host_decides_job_control_under_the_scope_it_derived() {
        let control = || {
            Some(RuntimeIpcRequest::ControlJob {
                run_id: RuntimeRunId::from_raw("run-job-control"),
                request_sha256: "1".repeat(64),
                request: fixture_control_request(),
            })
        };
        let status = || {
            Some(RuntimeIpcRequest::JobStatus {
                run_id: RuntimeRunId::from_raw("run-job-control"),
                request_sha256: "1".repeat(64),
            })
        };
        let mut port = ScopeRecordingPort::default();
        let other = client_scope(&LinuxPeerIdentity::new(1_000, 4_243, 77, [9; 32]));
        assert!(matches!(
            answer(&mut port, &fixture_client(), control()),
            (RuntimeIpcResponse::JobControl { .. }, false)
        ));
        assert!(matches!(
            answer(&mut port, &other, control()),
            (RuntimeIpcResponse::JobControl { .. }, false)
        ));
        assert_eq!(port.scopes, [fixture_client().as_str(), other.as_str()]);
        assert_eq!(
            answer(&mut port, &fixture_client(), status()),
            (
                RuntimeIpcResponse::JobStatus {
                    status: fixture_status()
                },
                false
            )
        );
        // A host without job ledgers refuses both and keeps answering.
        let mut declaring = DeclaringPort {
            session_id_bytes: 0,
            released: 0,
        };
        for request in [control(), status()] {
            assert_eq!(
                answer(&mut declaring, &fixture_client(), request),
                (
                    RuntimeIpcResponse::Error {
                        error: RuntimeTransportError::JobControlUnavailable,
                    },
                    false
                )
            );
        }
    }

    #[test]
    fn a_suspended_step_crosses_the_wire_and_names_only_its_boundary() {
        // Decision 0122: a step names where a suspended run stopped. Every
        // other step encodes exactly as before, and the point is closed.
        let cursor = RuntimeEventCursor {
            run_id: RuntimeRunId::from_raw("run-suspension"),
            event_id: agentmage_kernel_contracts::RuntimeEventId::from_raw("event-suspension"),
            sequence: 9,
            event_sha256: "3".repeat(64),
        };
        let point = agentmage_kernel_engine::runtime_loop::RuntimeSuspensionPoint {
            checkpoint_id: agentmage_kernel_contracts::SessionCheckpointId::from_raw(
                "checkpoint-suspension",
            ),
            checkpoint_sha256: "4".repeat(64),
            event_cursor: cursor,
        };
        let running = RuntimeTransportStep {
            run_id: RuntimeRunId::from_raw("run-suspension"),
            request_sha256: "1".repeat(64),
            events: Vec::new(),
            artifacts: Vec::new(),
            approval: None,
            outcome: None,
            suspended: None,
        };
        let encoded = serde_json::to_value(&running).unwrap();
        assert!(encoded.get("suspended").is_none());
        assert_eq!(
            serde_json::from_value::<RuntimeTransportStep>(encoded).unwrap(),
            running
        );
        let suspended = RuntimeTransportStep {
            suspended: Some(point),
            ..running
        };
        let response = RuntimeIpcResponse::Step {
            step: suspended.clone(),
        };
        let bytes = serde_json::to_vec(&RuntimeIpcEnvelope {
            version: WIRE_VERSION,
            payload: response.clone(),
        })
        .unwrap();
        let decoded: RuntimeIpcEnvelope<RuntimeIpcResponse> =
            serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded.version, 15);
        assert_eq!(decoded.payload, response);
        let mut extra = serde_json::to_value(&suspended).unwrap();
        extra["suspended"]["resumable"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<RuntimeTransportStep>(extra).is_err());
    }

    #[test]
    fn each_authenticated_client_process_has_its_own_ledger_scope() {
        let base = LinuxPeerIdentity::new(1_000, 4_242, 77, [9; 32]);
        let scope = client_scope(&base);
        assert_eq!(scope, client_scope(&base));
        assert_eq!(scope.as_str().len(), 5 + 32);
        assert!(scope.as_str().starts_with("peer-"));
        assert!(
            scope.as_str()[5..]
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        );
        for other in [
            LinuxPeerIdentity::new(1_001, 4_242, 77, [9; 32]),
            LinuxPeerIdentity::new(1_000, 4_243, 77, [9; 32]),
            LinuxPeerIdentity::new(1_000, 4_242, 78, [9; 32]),
            LinuxPeerIdentity::new(1_000, 4_242, 77, [8; 32]),
        ] {
            assert_ne!(client_scope(&other), scope);
        }
        // The client cannot shape the scope: it is a valid ledger identity
        // whatever the peer.
        let mut ledger = agentmage_kernel_engine::job_control::JobControlLedger::create(
            "run-job-control",
            "owner-fixture",
        )
        .unwrap();
        ledger
            .observe_owner(agentmage_kernel_engine::job_control::JobOwnerEvent::Started)
            .unwrap();
        assert!(
            ledger
                .control(scope.as_str(), &fixture_control_request())
                .is_ok()
        );
    }
}
