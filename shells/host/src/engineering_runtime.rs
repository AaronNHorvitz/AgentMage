//! Caller-neutral composition of Engineering RPC over encrypted host-owned state.

use agentmage_kernel_contracts::{
    ArtifactCaptureResult, ArtifactSourceKind, ArtifactUploadChunk, ArtifactUploadId,
    CONTRACT_SCHEMA_VERSION, ContextDeliveryReceipt, ContextPacketId, EndpointProfileId,
    EngineeringEventKind, EngineeringRpcRequest, EngineeringRpcResponse, EngineeringTerminalState,
    ModelProfileId, RouteDecisionId, RuntimeArtifactId, RuntimeRunId, SessionId,
    VerifiedModelTurnResult,
};
use agentmage_kernel_engine::engineering_persistence::SqlCipherEngineeringStore;
use agentmage_kernel_engine::engineering_records::ValidateCanonicalRecord;
use agentmage_kernel_engine::persistent_supervisor::{
    PersistentSupervisorError, PersistentTaskSupervisor,
};
use agentmage_kernel_engine::verified_artifact::{
    ArtifactUploadSpec, MAX_VERIFIED_ARTIFACT_CHUNK_BYTES, MAX_VERIFIED_ARTIFACT_READ_BYTES,
    VerifiedArtifactError, VerifiedArtifactUploads,
};
use agentmage_kernel_engine::verified_context::{
    ContextAdmissionPolicy, VerifiedContextError, admit_context,
};
use sha2::{Digest, Sha256};

use crate::artifact_ingestion::{ArtifactIngestionError, plan_artifact_ingestion};

const MAX_VERIFIED_MODEL_OUTPUT_BYTES: usize = 4 * 1024 * 1024;

/// Exact verified input supplied to one already-qualified model executor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineeringModelInput {
    /// Exact model-delivery receipt.
    pub context: ContextDeliveryReceipt,
    /// Exact prompt bytes loaded from encrypted artifact authority.
    pub prompt_bytes: Vec<u8>,
}

/// Stable content-free model-execution refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineeringModelError {
    /// The already-qualified model executor failed closed.
    Failed,
}

/// Trusted composition boundary for one qualified model and endpoint route.
pub trait EngineeringModelPort {
    /// Exact qualified model profile.
    fn model_profile_id(&self) -> ModelProfileId;
    /// Exact qualified endpoint profile.
    fn endpoint_profile_id(&self) -> EndpointProfileId;
    /// Exact deterministic route decision.
    fn route_decision_id(&self) -> RouteDecisionId;
    /// Executes one bounded model turn over exact verified bytes.
    fn execute(&mut self, input: &EngineeringModelInput) -> Result<String, EngineeringModelError>;
}

/// Stable content-free Engineering Runtime service failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineeringRuntimeError {
    /// Session lifecycle or durable event persistence failed closed.
    Supervisor(PersistentSupervisorError),
    /// Exact artifact capture or retrieval failed closed.
    Artifact(VerifiedArtifactError),
    /// Verified context admission failed closed.
    Context(VerifiedContextError),
    /// No qualified model executor was installed by trusted composition.
    ModelUnavailable,
    /// The qualified model executor failed or returned invalid output.
    ModelFailed,
    /// Artifact ingestion or its semantic record validation failed closed.
    IngestionFailed,
}

impl EngineeringRuntimeError {
    /// Returns one stable content-free code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Supervisor(error) => error.code(),
            Self::Artifact(error) => error.code(),
            Self::Context(error) => error.code(),
            Self::ModelUnavailable => "engineering.model.unavailable",
            Self::ModelFailed => "engineering.model.failed",
            Self::IngestionFailed => "engineering.ingestion.failed",
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
    model: Option<Box<dyn EngineeringModelPort>>,
}

impl EngineeringRuntimeService {
    /// Composes session supervision and exact artifact transfer over one SQLCipher store.
    #[must_use]
    pub fn new(store: SqlCipherEngineeringStore) -> Self {
        Self {
            supervisor: PersistentTaskSupervisor::new(store.clone()),
            artifacts: VerifiedArtifactUploads::new(store),
            model: None,
        }
    }

    /// Installs one qualified model executor from trusted host composition.
    pub fn install_model(&mut self, model: Box<dyn EngineeringModelPort>) {
        self.model = Some(model);
    }

    fn ingest_artifact(
        &mut self,
        session_id: SessionId,
        artifact_id: RuntimeArtifactId,
        completed_at_epoch_ms: u64,
    ) -> Result<EngineeringRpcResponse, EngineeringRuntimeError> {
        if completed_at_epoch_ms == 0 {
            return Err(EngineeringRuntimeError::IngestionFailed);
        }
        let snapshot = self
            .supervisor
            .open_session(&session_id)
            .map_err(EngineeringRuntimeError::Supervisor)?;
        if snapshot.terminal.is_some() {
            return Err(EngineeringRuntimeError::IngestionFailed);
        }
        let capture = snapshot
            .artifacts
            .iter()
            .find(|capture| capture.artifact_id == artifact_id)
            .cloned()
            .ok_or(EngineeringRuntimeError::Artifact(
                VerifiedArtifactError::NotFound,
            ))?;
        let source = self.read_complete_artifact(&session_id, &capture)?;
        let plan = plan_artifact_ingestion(
            &artifact_id,
            &capture.media_type,
            &capture.source_sha256,
            &source,
        )
        .map_err(|error| match error {
            ArtifactIngestionError::InvalidInput
            | ArtifactIngestionError::ParseFailed(_)
            | ArtifactIngestionError::EncodingFailed => EngineeringRuntimeError::IngestionFailed,
        })?;
        plan.result
            .validate_canonical()
            .map_err(|_| EngineeringRuntimeError::IngestionFailed)?;

        let derivative = match (&plan.derivative_media_type, &plan.derivative_bytes) {
            (Some(media_type), Some(bytes)) => Some(self.persist_generated(
                &session_id,
                &artifact_id,
                "derivative",
                "Parsed artifact derivative",
                media_type,
                bytes,
                completed_at_epoch_ms,
            )?),
            (None, None) => None,
            _ => return Err(EngineeringRuntimeError::IngestionFailed),
        };
        let transformation_record = if let Some(transformation) = &plan.transformation {
            transformation
                .validate_canonical()
                .map_err(|_| EngineeringRuntimeError::IngestionFailed)?;
            let bytes = serde_json::to_vec(transformation)
                .map_err(|_| EngineeringRuntimeError::IngestionFailed)?;
            Some(self.persist_generated(
                &session_id,
                &artifact_id,
                "transformation",
                "Artifact transformation provenance",
                "application/vnd.agentmage.transformation+json",
                &bytes,
                completed_at_epoch_ms,
            )?)
        } else {
            None
        };
        let ingestion_bytes = serde_json::to_vec(&plan.result)
            .map_err(|_| EngineeringRuntimeError::IngestionFailed)?;
        let ingestion_record = self.persist_generated(
            &session_id,
            &artifact_id,
            "ingestion",
            "Terminal artifact ingestion record",
            "application/vnd.agentmage.ingestion+json",
            &ingestion_bytes,
            completed_at_epoch_ms,
        )?;
        Ok(EngineeringRpcResponse::ArtifactIngested {
            ingestion: Box::new(plan.result),
            derivative: Box::new(derivative),
            transformation_record: Box::new(transformation_record),
            ingestion_record: Box::new(ingestion_record),
        })
    }

    fn read_complete_artifact(
        &self,
        session_id: &SessionId,
        capture: &ArtifactCaptureResult,
    ) -> Result<Vec<u8>, EngineeringRuntimeError> {
        let capacity = usize::try_from(capture.byte_length)
            .map_err(|_| EngineeringRuntimeError::IngestionFailed)?;
        let mut bytes = Vec::with_capacity(capacity);
        let mut offset = 0_u64;
        while offset < capture.byte_length {
            let length = (capture.byte_length - offset).min(MAX_VERIFIED_ARTIFACT_READ_BYTES);
            let range = self
                .artifacts
                .read_range(session_id, &capture.artifact_id, offset, length)
                .map_err(EngineeringRuntimeError::Artifact)?;
            if range.returned_offset != offset || range.bytes.len() as u64 != length {
                return Err(EngineeringRuntimeError::Artifact(
                    VerifiedArtifactError::IntegrityMismatch,
                ));
            }
            bytes.extend_from_slice(&range.bytes);
            offset += length;
        }
        if bytes.len() as u64 != capture.byte_length || sha256(&bytes) != capture.source_sha256 {
            return Err(EngineeringRuntimeError::Artifact(
                VerifiedArtifactError::IntegrityMismatch,
            ));
        }
        Ok(bytes)
    }

    #[allow(clippy::too_many_arguments)]
    fn persist_generated(
        &mut self,
        session_id: &SessionId,
        source_artifact_id: &RuntimeArtifactId,
        role: &str,
        display_name: &str,
        media_type: &str,
        bytes: &[u8],
        completed_at_epoch_ms: u64,
    ) -> Result<ArtifactCaptureResult, EngineeringRuntimeError> {
        let upload_id = ArtifactUploadId::from_raw(format!(
            "ingest-{}-{}",
            role,
            &sha256(format!("{}:{role}", source_artifact_id.as_str()).as_bytes())[..32]
        ));
        let snapshot = self
            .supervisor
            .open_session(session_id)
            .map_err(EngineeringRuntimeError::Supervisor)?;
        if let Some(existing) = snapshot
            .artifacts
            .iter()
            .find(|capture| capture.upload_id == upload_id)
        {
            if existing.source_sha256 != sha256(bytes)
                || existing.byte_length != bytes.len() as u64
                || existing.media_type != media_type
            {
                return Err(EngineeringRuntimeError::IngestionFailed);
            }
            return Ok(existing.clone());
        }
        let expected_sha256 = sha256(bytes);
        self.artifacts
            .begin(ArtifactUploadSpec {
                upload_id: upload_id.clone(),
                session_id: session_id.clone(),
                source_kind: ArtifactSourceKind::Generated,
                display_name: display_name.to_owned(),
                media_type: media_type.to_owned(),
                total_bytes: bytes.len() as u64,
                expected_sha256,
            })
            .map_err(EngineeringRuntimeError::Artifact)?;
        let chunks = bytes.chunks(MAX_VERIFIED_ARTIFACT_CHUNK_BYTES);
        let chunk_count = chunks.len();
        let mut offset = 0_u64;
        for (index, chunk) in chunks.enumerate() {
            self.artifacts
                .append(ArtifactUploadChunk {
                    schema_version: CONTRACT_SCHEMA_VERSION,
                    upload_id: upload_id.clone(),
                    session_id: session_id.clone(),
                    sequence: index as u32,
                    offset,
                    total_bytes: bytes.len() as u64,
                    bytes: chunk.to_vec(),
                    chunk_sha256: sha256(chunk),
                    final_chunk: index + 1 == chunk_count,
                })
                .map_err(EngineeringRuntimeError::Artifact)?;
            offset += chunk.len() as u64;
        }
        self.artifacts
            .commit(&upload_id, completed_at_epoch_ms)
            .map_err(EngineeringRuntimeError::Artifact)
    }

    fn execute_verified_turn(
        &mut self,
        session_id: agentmage_kernel_contracts::SessionId,
        prompt_artifact_id: agentmage_kernel_contracts::RuntimeArtifactId,
        correlation_id: agentmage_kernel_contracts::CorrelationId,
        occurred_at_epoch_ms: u64,
    ) -> Result<EngineeringRpcResponse, EngineeringRuntimeError> {
        if occurred_at_epoch_ms == 0 || correlation_id.as_str().is_empty() {
            return Err(EngineeringRuntimeError::ModelFailed);
        }
        let snapshot = self
            .supervisor
            .open_session(&session_id)
            .map_err(EngineeringRuntimeError::Supervisor)?;
        if snapshot.terminal.is_some() {
            return Err(EngineeringRuntimeError::ModelFailed);
        }
        let capture = snapshot
            .artifacts
            .iter()
            .find(|capture| capture.artifact_id == prompt_artifact_id)
            .cloned()
            .ok_or(EngineeringRuntimeError::Artifact(
                VerifiedArtifactError::NotFound,
            ))?;
        if capture.byte_length > MAX_VERIFIED_ARTIFACT_READ_BYTES {
            return Err(EngineeringRuntimeError::Context(
                VerifiedContextError::ResourceExceeded,
            ));
        }
        let range = self
            .artifacts
            .read_range(&session_id, &prompt_artifact_id, 0, capture.byte_length)
            .map_err(EngineeringRuntimeError::Artifact)?;
        if range.truncated
            || range.returned_offset != 0
            || range.bytes.len() as u64 != capture.byte_length
            || sha256(&range.bytes) != capture.source_sha256
        {
            return Err(EngineeringRuntimeError::Artifact(
                VerifiedArtifactError::IntegrityMismatch,
            ));
        }
        let model = self
            .model
            .as_mut()
            .ok_or(EngineeringRuntimeError::ModelUnavailable)?;
        let model_profile_id = model.model_profile_id();
        let endpoint_profile_id = model.endpoint_profile_id();
        let route_decision_id = model.route_decision_id();
        let run_id = RuntimeRunId::from_raw(format!(
            "engineering-run-{}-{}",
            session_id.as_str(),
            correlation_id.as_str()
        ));
        let context_packet_id = ContextPacketId::from_raw(format!(
            "engineering-context-{}-{}",
            session_id.as_str(),
            correlation_id.as_str()
        ));
        let context = admit_context(
            context_packet_id.clone(),
            run_id.clone(),
            model_profile_id,
            endpoint_profile_id,
            route_decision_id.clone(),
            std::slice::from_ref(&capture),
            &ContextAdmissionPolicy {
                max_inline_bytes: MAX_VERIFIED_ARTIFACT_READ_BYTES,
                token_limit: MAX_VERIFIED_ARTIFACT_READ_BYTES,
                estimated_bytes_per_token: 1,
            },
        )
        .map_err(EngineeringRuntimeError::Context)?;
        self.supervisor
            .record(
                &session_id,
                correlation_id.clone(),
                occurred_at_epoch_ms,
                EngineeringEventKind::ModelRouteSelected { route_decision_id },
            )
            .map_err(EngineeringRuntimeError::Supervisor)?;
        self.supervisor
            .record(
                &session_id,
                correlation_id.clone(),
                occurred_at_epoch_ms,
                EngineeringEventKind::ContextAdmitted { context_packet_id },
            )
            .map_err(EngineeringRuntimeError::Supervisor)?;
        let output_text = model
            .execute(&EngineeringModelInput {
                context: context.clone(),
                prompt_bytes: range.bytes,
            })
            .map_err(|_| EngineeringRuntimeError::ModelFailed)?;
        if output_text.is_empty()
            || output_text.len() > MAX_VERIFIED_MODEL_OUTPUT_BYTES
            || output_text.contains('\0')
        {
            return Err(EngineeringRuntimeError::ModelFailed);
        }
        self.supervisor
            .record(
                &session_id,
                correlation_id,
                occurred_at_epoch_ms,
                EngineeringEventKind::VerificationCompleted {
                    terminal: EngineeringTerminalState::Success,
                },
            )
            .map_err(EngineeringRuntimeError::Supervisor)?;
        Ok(EngineeringRpcResponse::VerifiedTurnCompleted {
            turn: VerifiedModelTurnResult {
                schema_version: CONTRACT_SCHEMA_VERSION,
                session_id,
                run_id,
                context,
                output_sha256: sha256(output_text.as_bytes()),
                output_text,
                terminal: EngineeringTerminalState::Success,
            },
        })
    }
}

impl EngineeringRuntimePort for EngineeringRuntimeService {
    fn handle(
        &mut self,
        request: EngineeringRpcRequest,
    ) -> Result<EngineeringRpcResponse, EngineeringRuntimeError> {
        use EngineeringRpcRequest::{
            BeginArtifact, CancelArtifact, CancelSession, CommitArtifact, CreateSession,
            ExecuteVerifiedTurn, IngestArtifact, ListSessions, OpenSession, PauseSession,
            ReadArtifactRange, ReplayEvents, ResumeSession, UploadArtifactChunk,
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
            IngestArtifact {
                session_id,
                artifact_id,
                completed_at_epoch_ms,
            } => self.ingest_artifact(session_id, artifact_id, completed_at_epoch_ms),
            ExecuteVerifiedTurn {
                session_id,
                prompt_artifact_id,
                correlation_id,
                occurred_at_epoch_ms,
            } => self.execute_verified_turn(
                session_id,
                prompt_artifact_id,
                correlation_id,
                occurred_at_epoch_ms,
            ),
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

fn sha256(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    let digest = digest.finalize();
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::{Arc, Mutex};

    use agentmage_kernel_contracts::{
        ArtifactSourceKind, ArtifactUploadChunk, ArtifactUploadId, CloudSynchronizationMarker,
        CorrelationId, EndpointProfileId, EngineeringEventKind, EngineeringRpcRequest,
        EngineeringRpcResponse, EngineeringSessionMode, ModelProfileId, RouteDecisionId, SessionId,
        StorageFilesystemClass, StrictLocalStorageObservation,
    };
    use agentmage_kernel_engine::engineering_persistence::SqlCipherEngineeringStore;
    use agentmage_kernel_engine::operational_store::{
        OperationalStore, OperationalStoreKeyError, OperationalStoreKeyProvider,
    };

    use super::{
        EngineeringModelError, EngineeringModelInput, EngineeringModelPort,
        EngineeringRuntimeError, EngineeringRuntimePort, EngineeringRuntimeService, sha256,
    };

    struct TestKey;

    impl OperationalStoreKeyProvider for TestKey {
        fn with_key<T>(
            &mut self,
            operation: impl FnOnce(&[u8]) -> T,
        ) -> Result<T, OperationalStoreKeyError> {
            Ok(operation(&[67; 32]))
        }
    }

    struct ExactSentinelModel {
        expected_sha256: String,
        sentinels: Vec<(usize, Vec<u8>)>,
    }

    impl EngineeringModelPort for ExactSentinelModel {
        fn model_profile_id(&self) -> ModelProfileId {
            ModelProfileId::from_raw("model-exact-sentinel-fixture")
        }

        fn endpoint_profile_id(&self) -> EndpointProfileId {
            EndpointProfileId::from_raw("endpoint-exact-sentinel-fixture")
        }

        fn route_decision_id(&self) -> RouteDecisionId {
            RouteDecisionId::from_raw("route-exact-sentinel-fixture")
        }

        fn execute(
            &mut self,
            input: &EngineeringModelInput,
        ) -> Result<String, EngineeringModelError> {
            if sha256(&input.prompt_bytes) != self.expected_sha256
                || input.context.artifacts.len() != 1
                || input.context.artifacts[0].ranges != vec![(0, input.prompt_bytes.len() as u64)]
                || self.sentinels.iter().any(|(offset, sentinel)| {
                    input.prompt_bytes.get(*offset..offset + sentinel.len())
                        != Some(sentinel.as_slice())
                })
            {
                return Err(EngineeringModelError::Failed);
            }
            Ok("exact-context-sentinels-verified".to_owned())
        }
    }

    fn store() -> (std::path::PathBuf, SqlCipherEngineeringStore) {
        let directory = std::env::temp_dir().join(format!(
            "agentmage-verified-turn-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir(&directory).unwrap();
        let observation = StrictLocalStorageObservation {
            filesystem: StorageFilesystemClass::Local,
            synchronization_marker: None::<CloudSynchronizationMarker>,
            root_identity_sha256: [29; 32],
            symlink_free: true,
        };
        let authority =
            OperationalStore::open(&directory.join("authority.db"), &observation, &mut TestKey)
                .unwrap();
        (
            directory,
            SqlCipherEngineeringStore::new(Arc::new(Mutex::new(authority))),
        )
    }

    #[test]
    fn fifty_thousand_byte_turn_is_exact_durable_and_reopenable() {
        let (directory, adapter) = store();
        let session_id = SessionId::from_raw("session-exact-turn-fixture");
        let upload_id = ArtifactUploadId::from_raw("upload-exact-turn-fixture");
        let mut source = vec![b'x'; 50_128];
        let sentinel_texts = [
            b"BEGIN-7d3a1f".as_slice(),
            b"QUARTER-48e2".as_slice(),
            b"MIDDLE-b591".as_slice(),
            b"THREE-QUARTER-c6".as_slice(),
            b"END-91af8c20".as_slice(),
        ];
        let offsets = [
            0,
            source.len() / 4,
            source.len() / 2,
            source.len() * 3 / 4,
            source.len() - sentinel_texts[4].len(),
        ];
        let sentinels: Vec<_> = offsets
            .into_iter()
            .zip(sentinel_texts)
            .map(|(offset, sentinel)| {
                source[offset..offset + sentinel.len()].copy_from_slice(sentinel);
                (offset, sentinel.to_vec())
            })
            .collect();
        let source_sha256 = sha256(&source);
        let mut runtime = EngineeringRuntimeService::new(adapter.clone());
        runtime.install_model(Box::new(ExactSentinelModel {
            expected_sha256: source_sha256.clone(),
            sentinels,
        }));
        runtime
            .handle(EngineeringRpcRequest::CreateSession {
                session_id: session_id.clone(),
                title: "Exact context turn".to_owned(),
                mode: EngineeringSessionMode::Ask,
                correlation_id: CorrelationId::from_raw("correlation-exact-create"),
                occurred_at_epoch_ms: 10,
            })
            .unwrap();
        runtime
            .handle(EngineeringRpcRequest::BeginArtifact {
                upload_id: upload_id.clone(),
                session_id: session_id.clone(),
                source_kind: ArtifactSourceKind::Paste,
                display_name: "Long exact paste".to_owned(),
                media_type: "text/plain".to_owned(),
                total_bytes: source.len() as u64,
                expected_sha256: source_sha256.clone(),
            })
            .unwrap();
        runtime
            .handle(EngineeringRpcRequest::UploadArtifactChunk {
                chunk: ArtifactUploadChunk {
                    schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                    upload_id: upload_id.clone(),
                    session_id: session_id.clone(),
                    sequence: 0,
                    offset: 0,
                    total_bytes: source.len() as u64,
                    chunk_sha256: source_sha256,
                    final_chunk: true,
                    bytes: source,
                },
            })
            .unwrap();
        let capture = runtime
            .handle(EngineeringRpcRequest::CommitArtifact {
                upload_id,
                completed_at_epoch_ms: 20,
            })
            .unwrap();
        let EngineeringRpcResponse::ArtifactCaptured { capture } = capture else {
            panic!("capture response expected");
        };
        let turn = runtime
            .handle(EngineeringRpcRequest::ExecuteVerifiedTurn {
                session_id: session_id.clone(),
                prompt_artifact_id: capture.artifact_id.clone(),
                correlation_id: CorrelationId::from_raw("correlation-exact-turn"),
                occurred_at_epoch_ms: 30,
            })
            .unwrap();
        let EngineeringRpcResponse::VerifiedTurnCompleted { turn } = turn else {
            panic!("turn response expected");
        };
        assert_eq!(turn.output_text, "exact-context-sentinels-verified");
        assert_eq!(turn.context.inline_bytes, capture.byte_length);
        drop(runtime);

        let mut reopened = EngineeringRuntimeService::new(adapter);
        let snapshot = reopened
            .handle(EngineeringRpcRequest::OpenSession {
                session_id: session_id.clone(),
            })
            .unwrap();
        let EngineeringRpcResponse::Session { snapshot } = snapshot else {
            panic!("session response expected");
        };
        assert_eq!(snapshot.artifacts, vec![capture]);
        let replay = reopened
            .handle(EngineeringRpcRequest::ReplayEvents {
                session_id,
                after_sequence: None,
            })
            .unwrap();
        let EngineeringRpcResponse::Events { events } = replay else {
            panic!("event response expected");
        };
        assert_eq!(events.len(), 5);
        assert!(matches!(
            events.last().map(|event| &event.kind),
            Some(EngineeringEventKind::VerificationCompleted {
                terminal: agentmage_kernel_contracts::EngineeringTerminalState::Success
            })
        ));
        assert_eq!(
            reopened.handle(EngineeringRpcRequest::ExecuteVerifiedTurn {
                session_id: snapshot.session_id,
                prompt_artifact_id: snapshot.artifacts[0].artifact_id.clone(),
                correlation_id: CorrelationId::from_raw("correlation-no-model"),
                occurred_at_epoch_ms: 40,
            }),
            Err(EngineeringRuntimeError::ModelUnavailable)
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn multi_range_ingestion_is_durable_and_idempotent() {
        let (directory, adapter) = store();
        let session_id = SessionId::from_raw("session-large-ingestion-fixture");
        let upload_id = ArtifactUploadId::from_raw("upload-large-ingestion-fixture");
        let source = vec![b'x'; 1_200_000];
        let source_sha256 = sha256(&source);
        let mut runtime = EngineeringRuntimeService::new(adapter.clone());
        runtime
            .handle(EngineeringRpcRequest::CreateSession {
                session_id: session_id.clone(),
                title: "Large ingestion".to_owned(),
                mode: EngineeringSessionMode::Ask,
                correlation_id: CorrelationId::from_raw("correlation-ingestion-create"),
                occurred_at_epoch_ms: 100,
            })
            .unwrap();
        runtime
            .handle(EngineeringRpcRequest::BeginArtifact {
                upload_id: upload_id.clone(),
                session_id: session_id.clone(),
                source_kind: ArtifactSourceKind::File,
                display_name: "large.txt".to_owned(),
                media_type: "text/plain".to_owned(),
                total_bytes: source.len() as u64,
                expected_sha256: source_sha256,
            })
            .unwrap();
        let chunks = source.chunks(256 * 1024);
        let count = chunks.len();
        let mut offset = 0_u64;
        for (index, bytes) in chunks.enumerate() {
            runtime
                .handle(EngineeringRpcRequest::UploadArtifactChunk {
                    chunk: ArtifactUploadChunk {
                        schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                        upload_id: upload_id.clone(),
                        session_id: session_id.clone(),
                        sequence: index as u32,
                        offset,
                        total_bytes: source.len() as u64,
                        bytes: bytes.to_vec(),
                        chunk_sha256: sha256(bytes),
                        final_chunk: index + 1 == count,
                    },
                })
                .unwrap();
            offset += bytes.len() as u64;
        }
        let captured = runtime
            .handle(EngineeringRpcRequest::CommitArtifact {
                upload_id,
                completed_at_epoch_ms: 110,
            })
            .unwrap();
        let EngineeringRpcResponse::ArtifactCaptured { capture } = captured else {
            panic!("capture expected");
        };
        let request = EngineeringRpcRequest::IngestArtifact {
            session_id: session_id.clone(),
            artifact_id: capture.artifact_id,
            completed_at_epoch_ms: 120,
        };
        let first = runtime.handle(request.clone()).unwrap();
        let second = runtime.handle(request).unwrap();
        assert_eq!(first, second);
        let EngineeringRpcResponse::ArtifactIngested {
            ingestion,
            derivative,
            transformation_record,
            ingestion_record,
        } = first
        else {
            panic!("ingestion expected");
        };
        assert_eq!(
            ingestion.disposition,
            agentmage_kernel_contracts::CanonicalIngestionDisposition::Parsed
        );
        assert_eq!(derivative.as_ref().as_ref().unwrap().byte_length, 1_200_000);
        assert!(transformation_record.as_ref().is_some());
        assert_eq!(ingestion_record.source_kind, ArtifactSourceKind::Generated);
        drop(runtime);

        let mut reopened = EngineeringRuntimeService::new(adapter);
        let snapshot = reopened
            .handle(EngineeringRpcRequest::OpenSession {
                session_id: session_id.clone(),
            })
            .unwrap();
        let EngineeringRpcResponse::Session { snapshot } = snapshot else {
            panic!("session expected");
        };
        assert_eq!(snapshot.artifacts.len(), 4);
        let events = reopened
            .handle(EngineeringRpcRequest::ReplayEvents {
                session_id,
                after_sequence: None,
            })
            .unwrap();
        let EngineeringRpcResponse::Events { events } = events else {
            panic!("events expected");
        };
        assert_eq!(events.len(), 5);
        assert!(
            events
                .iter()
                .skip(1)
                .all(|event| matches!(event.kind, EngineeringEventKind::ArtifactCaptured { .. }))
        );
        fs::remove_dir_all(directory).unwrap();
    }
}
