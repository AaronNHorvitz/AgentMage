//! Authenticated bounded Linux IPC projection of the shared runtime transport.
//!
//! Over this channel a run is cancelled only through a job control request,
//! which the host decides through its durable job ledger under the client
//! scope it derives from the authenticated peer (Decision 0120). Suspension and
//! resumption are job control requests too; a step names where a suspended run
//! stopped (Decision 0122).

use agentmage_kernel_contracts::{
    CancellationId, RuntimeApprovalResponse, RuntimeArtifactRef, RuntimeEventCursor, RuntimeRunId,
    RuntimeRunRequest, SessionId,
};
use agentmage_kernel_engine::job_control::JobControlRequest;
use agentmage_kernel_engine::runtime_artifact::{RuntimeArtifactPage, RuntimeArtifactState};
use agentmage_platform_linux::{LinuxAuthenticatedIpcSession, LinuxPeerIdentity};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::runtime_transport::{
    RuntimeClientScope, RuntimeJobControl, RuntimeJobStatus, RuntimePrepareInput,
    RuntimeRunDeclarations, RuntimeTransportError, RuntimeTransportPort, RuntimeTransportStep,
};

const WIRE_VERSION: u16 = 8;
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
    serde_json::to_vec(declarations).is_ok_and(|bytes| bytes.len() <= MAX_DECLARATION_BYTES)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_client() -> RuntimeClientScope {
        client_scope(&LinuxPeerIdentity::new(1_000, 4_242, 77, [9; 32]))
    }

    #[test]
    fn run_declarations_cross_the_wire_exactly_and_only_at_the_current_version() {
        let declarations = RuntimeRunDeclarations {
            schema_version: 1,
            run_id: RuntimeRunId::from_raw("run-declarations"),
            request_sha256: "1".repeat(64),
            recoverability: None,
            context_inspections: Some(Vec::new()),
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
        assert_eq!(decoded.version, 8);
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
        // A field the closed contract does not name is refused.
        let mut extra = serde_json::to_value(&declarations).unwrap();
        extra["complete"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<RuntimeRunDeclarations>(extra).is_err());
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
                schema_version: 1,
                run_id: run_id.clone(),
                request_sha256: request_sha256.to_owned(),
                recoverability: Some(report),
                context_inspections: None,
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
            assert_eq!(decoded.version, 8);
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
        assert_eq!(decoded.version, 8);
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
