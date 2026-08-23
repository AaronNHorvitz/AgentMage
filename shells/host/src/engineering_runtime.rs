//! Caller-neutral composition of Engineering RPC over encrypted host-owned state.

use agentmage_kernel_contracts::{EngineeringRpcRequest, EngineeringRpcResponse};
use agentmage_kernel_engine::engineering_persistence::SqlCipherEngineeringStore;
use agentmage_kernel_engine::persistent_supervisor::{
    PersistentSupervisorError, PersistentTaskSupervisor,
};
use agentmage_kernel_engine::verified_artifact::{
    ArtifactUploadSpec, VerifiedArtifactError, VerifiedArtifactUploads,
};

/// Stable content-free Engineering Runtime service failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineeringRuntimeError {
    /// Session lifecycle or durable event persistence failed closed.
    Supervisor(PersistentSupervisorError),
    /// Exact artifact capture or retrieval failed closed.
    Artifact(VerifiedArtifactError),
}

impl EngineeringRuntimeError {
    /// Returns one stable content-free code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Supervisor(error) => error.code(),
            Self::Artifact(error) => error.code(),
        }
    }
}

/// Interface-neutral boundary consumed by authenticated local clients.
pub trait EngineeringRuntimePort {
    /// Executes one validated closed RPC operation.
    fn handle(
        &mut self,
        request: EngineeringRpcRequest,
    ) -> Result<EngineeringRpcResponse, EngineeringRuntimeError>;
}

/// Rust-owned durable Engineering Runtime service.
pub struct EngineeringRuntimeService {
    supervisor: PersistentTaskSupervisor<SqlCipherEngineeringStore>,
    artifacts: VerifiedArtifactUploads<SqlCipherEngineeringStore>,
}

impl EngineeringRuntimeService {
    /// Composes session supervision and exact artifact transfer over one SQLCipher store.
    #[must_use]
    pub fn new(store: SqlCipherEngineeringStore) -> Self {
        Self {
            supervisor: PersistentTaskSupervisor::new(store.clone()),
            artifacts: VerifiedArtifactUploads::new(store),
        }
    }
}

impl EngineeringRuntimePort for EngineeringRuntimeService {
    fn handle(
        &mut self,
        request: EngineeringRpcRequest,
    ) -> Result<EngineeringRpcResponse, EngineeringRuntimeError> {
        use EngineeringRpcRequest::{
            BeginArtifact, CancelArtifact, CancelSession, CommitArtifact, CreateSession,
            ListSessions, OpenSession, PauseSession, ReadArtifactRange, ReplayEvents,
            ResumeSession, UploadArtifactChunk,
        };
        match request {
            CreateSession {
                session_id,
                title,
                mode,
                correlation_id,
                occurred_at_epoch_ms,
            } => self
                .supervisor
                .create_session(
                    session_id,
                    title,
                    mode,
                    correlation_id,
                    occurred_at_epoch_ms,
                )
                .map(|snapshot| EngineeringRpcResponse::Session { snapshot })
                .map_err(EngineeringRuntimeError::Supervisor),
            ListSessions => self
                .supervisor
                .list_sessions()
                .map(|sessions| EngineeringRpcResponse::Sessions { sessions })
                .map_err(EngineeringRuntimeError::Supervisor),
            OpenSession { session_id } => self
                .supervisor
                .open_session(&session_id)
                .map(|snapshot| EngineeringRpcResponse::Session { snapshot })
                .map_err(EngineeringRuntimeError::Supervisor),
            BeginArtifact {
                upload_id,
                session_id,
                source_kind,
                display_name,
                media_type,
                total_bytes,
                expected_sha256,
            } => {
                self.artifacts
                    .begin(ArtifactUploadSpec {
                        upload_id: upload_id.clone(),
                        session_id,
                        source_kind,
                        display_name,
                        media_type,
                        total_bytes,
                        expected_sha256,
                    })
                    .map_err(EngineeringRuntimeError::Artifact)?;
                Ok(EngineeringRpcResponse::ArtifactUploadStarted { upload_id })
            }
            UploadArtifactChunk { chunk } => {
                let upload_id = chunk.upload_id.clone();
                let sequence = chunk.sequence;
                self.artifacts
                    .append(chunk)
                    .map_err(EngineeringRuntimeError::Artifact)?;
                Ok(EngineeringRpcResponse::ArtifactChunkAccepted {
                    upload_id,
                    sequence,
                })
            }
            CommitArtifact {
                upload_id,
                completed_at_epoch_ms,
            } => self
                .artifacts
                .commit(&upload_id, completed_at_epoch_ms)
                .map(|capture| EngineeringRpcResponse::ArtifactCaptured { capture })
                .map_err(EngineeringRuntimeError::Artifact),
            CancelArtifact { upload_id } => {
                if !self.artifacts.cancel(&upload_id) {
                    return Err(EngineeringRuntimeError::Artifact(
                        VerifiedArtifactError::NotFound,
                    ));
                }
                Ok(EngineeringRpcResponse::ArtifactUploadCancelled { upload_id })
            }
            ReadArtifactRange {
                session_id,
                artifact_id,
                offset,
                length,
            } => self
                .artifacts
                .read_range(&session_id, &artifact_id, offset, length)
                .map(|range| EngineeringRpcResponse::ArtifactRange { range })
                .map_err(EngineeringRuntimeError::Artifact),
            ReplayEvents {
                session_id,
                after_sequence,
            } => self
                .supervisor
                .replay(&session_id, after_sequence)
                .map(|events| EngineeringRpcResponse::Events { events })
                .map_err(EngineeringRuntimeError::Supervisor),
            PauseSession {
                session_id,
                correlation_id,
                occurred_at_epoch_ms,
            } => self
                .supervisor
                .pause(&session_id, correlation_id, occurred_at_epoch_ms)
                .map(|event| EngineeringRpcResponse::LifecycleEvent { event })
                .map_err(EngineeringRuntimeError::Supervisor),
            ResumeSession {
                session_id,
                correlation_id,
                occurred_at_epoch_ms,
            } => self
                .supervisor
                .resume(&session_id, correlation_id, occurred_at_epoch_ms)
                .map(|event| EngineeringRpcResponse::LifecycleEvent { event })
                .map_err(EngineeringRuntimeError::Supervisor),
            CancelSession {
                session_id,
                correlation_id,
                occurred_at_epoch_ms,
            } => self
                .supervisor
                .cancel(&session_id, correlation_id, occurred_at_epoch_ms)
                .map(|event| EngineeringRpcResponse::LifecycleEvent { event })
                .map_err(EngineeringRuntimeError::Supervisor),
        }
    }
}
