//! Authenticated bounded Linux IPC projection of the shared runtime transport.

use agentmage_kernel_contracts::{
    CancellationId, RuntimeApprovalResponse, RuntimeArtifactRef, RuntimeEventCursor, RuntimeRunId,
    RuntimeRunRequest, SessionId,
};
use agentmage_kernel_engine::runtime_artifact::{RuntimeArtifactPage, RuntimeArtifactState};
use agentmage_platform_linux::LinuxAuthenticatedIpcSession;
use serde::{Deserialize, Serialize};

use crate::runtime_transport::{
    RuntimePrepareInput, RuntimeTransportError, RuntimeTransportPort, RuntimeTransportStep,
};

const WIRE_VERSION: u16 = 4;
const MAX_WIRE_BYTES: usize = 4 * 1024 * 1024;

/// One closed operation accepted by the host-owned runtime service.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
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
    Cancel {
        run_id: RuntimeRunId,
        request_sha256: String,
        cancellation_id: CancellationId,
        after_event_cursor: Option<RuntimeEventCursor>,
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
enum RuntimeIpcResponse {
    Prepared { request: RuntimeRunRequest },
    Step { step: RuntimeTransportStep },
    ArtifactPage { page: RuntimeArtifactPage },
    ArtifactState { state: RuntimeArtifactState },
    PreauthorizationRevoked,
    Released,
    Shutdown,
    Error { error: RuntimeTransportError },
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

    fn cancel(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        cancellation_id: CancellationId,
        after_event_cursor: Option<&RuntimeEventCursor>,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError> {
        match self.exchange(RuntimeIpcRequest::Cancel {
            run_id: run_id.clone(),
            request_sha256: request_sha256.to_owned(),
            cancellation_id,
            after_event_cursor: after_event_cursor.cloned(),
        })? {
            RuntimeIpcResponse::Step { step } => Ok(step),
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
            RuntimeIpcResponse::ArtifactPage { page } => Ok(page),
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
    loop {
        let bytes = session
            .read_frame(MAX_WIRE_BYTES)
            .map_err(|_| RuntimeTransportError::RuntimeFailed)?;
        let request = serde_json::from_slice::<RuntimeIpcEnvelope<RuntimeIpcRequest>>(&bytes)
            .ok()
            .filter(|request| request.version == WIRE_VERSION)
            .map(|request| request.payload);
        let (response, shutdown) = match request {
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
            Some(RuntimeIpcRequest::Cancel {
                run_id,
                request_sha256,
                cancellation_id,
                after_event_cursor,
            }) => match runtime.cancel(
                &run_id,
                &request_sha256,
                cancellation_id,
                after_event_cursor.as_ref(),
            ) {
                Ok(step) => (RuntimeIpcResponse::Step { step }, false),
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
            Some(RuntimeIpcRequest::RevokeSessionPreauthorization {
                session_id,
                preauthorization_sha256,
            }) => match runtime
                .revoke_session_preauthorization(&session_id, &preauthorization_sha256)
            {
                Ok(()) => (RuntimeIpcResponse::PreauthorizationRevoked, false),
                Err(error) => (RuntimeIpcResponse::Error { error }, false),
            },
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
        };
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
