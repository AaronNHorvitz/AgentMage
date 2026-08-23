//! Caller-neutral composition of Engineering RPC over encrypted host-owned state.

use agentmage_kernel_contracts::{
    ActorId, ApprovalId, ArtifactCaptureResult, ArtifactSourceKind, ArtifactUploadChunk,
    ArtifactUploadId, CONTRACT_SCHEMA_VERSION, ContextDeliveryReceipt, ContextPacketId,
    EndpointProfileId, EngineeringEventKind, EngineeringRpcRequest, EngineeringRpcResponse,
    EngineeringSessionMode, EngineeringTerminalState, ModelProfileId, RouteDecisionId,
    RuntimeArtifactId, RuntimeRunId, RuntimeRunRequest, SessionId, VerifiedModelTurnResult,
};
use agentmage_kernel_engine::engineering_execution::{
    seal_runtime_binding, verify_runtime_binding,
};
use agentmage_kernel_engine::engineering_mode::{
    EngineeringModeOperation, enforce_engineering_mode,
};
use agentmage_kernel_engine::engineering_persistence::SqlCipherEngineeringStore;
use agentmage_kernel_engine::engineering_plan::{
    seal_plan_approval, seal_plan_handoff, verify_plan_approval,
};
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
    /// Exact owning Verified Chat session.
    pub session_id: SessionId,
    /// Immutable kernel-enforced mode of the owning session.
    pub mode: EngineeringSessionMode,
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
    /// The immutable session mode denies this operation family.
    ModeDenied,
    /// Exact Plan approval or replay validation failed closed.
    PlanApprovalFailed,
    /// Approved-Plan handoff validation or target-session creation failed closed.
    PlanHandoffFailed,
    /// Approved-Plan runtime request binding failed closed.
    RuntimeBindingFailed,
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
            Self::ModeDenied => "engineering.mode.denied",
            Self::PlanApprovalFailed => "engineering.plan.approval.failed",
            Self::PlanHandoffFailed => "engineering.plan.handoff.failed",
            Self::RuntimeBindingFailed => "engineering.runtime.binding.failed",
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
    approval_actor: ActorId,
}

impl EngineeringRuntimeService {
    /// Composes session supervision and exact artifact transfer over one SQLCipher store.
    #[must_use]
    pub fn new(store: SqlCipherEngineeringStore, approval_actor: ActorId) -> Self {
        Self {
            supervisor: PersistentTaskSupervisor::new(store.clone()),
            artifacts: VerifiedArtifactUploads::new(store),
            model: None,
            approval_actor,
        }
    }

    /// Installs one qualified model executor from trusted host composition.
    pub fn install_model(&mut self, model: Box<dyn EngineeringModelPort>) {
        self.model = Some(model);
    }

    #[allow(clippy::too_many_arguments)]
    fn approve_plan(
        &mut self,
        session_id: SessionId,
        plan_artifact_id: RuntimeArtifactId,
        plan_sha256: String,
        approval_id: ApprovalId,
        correlation_id: agentmage_kernel_contracts::CorrelationId,
        occurred_at_epoch_ms: u64,
    ) -> Result<EngineeringRpcResponse, EngineeringRuntimeError> {
        let snapshot = self
            .supervisor
            .open_session(&session_id)
            .map_err(EngineeringRuntimeError::Supervisor)?;
        if snapshot.terminal.is_some() || snapshot.mode != EngineeringSessionMode::Plan {
            return Err(EngineeringRuntimeError::PlanApprovalFailed);
        }
        enforce_engineering_mode(snapshot.mode, EngineeringModeOperation::PlanArtifact)
            .map_err(|_| EngineeringRuntimeError::ModeDenied)?;
        let capture = snapshot
            .artifacts
            .iter()
            .find(|capture| capture.artifact_id == plan_artifact_id)
            .ok_or(EngineeringRuntimeError::PlanApprovalFailed)?;
        if capture.source_kind != ArtifactSourceKind::Generated
            || capture.display_name != "Verified Chat plan draft"
            || capture.media_type != "text/markdown"
            || capture.source_sha256 != plan_sha256
        {
            return Err(EngineeringRuntimeError::PlanApprovalFailed);
        }
        let approval = seal_plan_approval(
            approval_id,
            session_id.clone(),
            plan_artifact_id.clone(),
            plan_sha256,
            self.approval_actor.clone(),
            occurred_at_epoch_ms,
        )
        .map_err(|_| EngineeringRuntimeError::PlanApprovalFailed)?;
        let events = self
            .supervisor
            .replay(&session_id, None)
            .map_err(EngineeringRuntimeError::Supervisor)?;
        for event in events {
            if let EngineeringEventKind::PlanApproved { approval: existing } = &event.kind
                && existing.plan_artifact_id == plan_artifact_id
            {
                verify_plan_approval(existing)
                    .map_err(|_| EngineeringRuntimeError::PlanApprovalFailed)?;
                if existing.as_ref() == &approval {
                    return Ok(EngineeringRpcResponse::PlanApproved {
                        approval: existing.clone(),
                        event: Box::new(event),
                    });
                }
                return Err(EngineeringRuntimeError::PlanApprovalFailed);
            }
        }
        let event = self
            .supervisor
            .record(
                &session_id,
                correlation_id,
                occurred_at_epoch_ms,
                EngineeringEventKind::PlanApproved {
                    approval: Box::new(approval.clone()),
                },
            )
            .map_err(EngineeringRuntimeError::Supervisor)?;
        Ok(EngineeringRpcResponse::PlanApproved {
            approval: Box::new(approval),
            event: Box::new(event),
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn create_session_from_approved_plan(
        &mut self,
        source_session_id: SessionId,
        plan_artifact_id: RuntimeArtifactId,
        plan_sha256: String,
        approval_id: ApprovalId,
        approval_sha256: String,
        target_session_id: SessionId,
        title: String,
        target_mode: EngineeringSessionMode,
        correlation_id: agentmage_kernel_contracts::CorrelationId,
        occurred_at_epoch_ms: u64,
    ) -> Result<EngineeringRpcResponse, EngineeringRuntimeError> {
        if !matches!(
            target_mode,
            EngineeringSessionMode::Agent | EngineeringSessionMode::Team
        ) {
            return Err(EngineeringRuntimeError::PlanHandoffFailed);
        }
        let source = self
            .supervisor
            .open_session(&source_session_id)
            .map_err(EngineeringRuntimeError::Supervisor)?;
        if source.mode != EngineeringSessionMode::Plan || source.terminal.is_some() {
            return Err(EngineeringRuntimeError::PlanHandoffFailed);
        }
        let capture = source
            .artifacts
            .iter()
            .find(|capture| capture.artifact_id == plan_artifact_id)
            .ok_or(EngineeringRuntimeError::PlanHandoffFailed)?;
        if capture.source_kind != ArtifactSourceKind::Generated
            || capture.display_name != "Verified Chat plan draft"
            || capture.media_type != "text/markdown"
            || capture.source_sha256 != plan_sha256
        {
            return Err(EngineeringRuntimeError::PlanHandoffFailed);
        }
        let approval = self
            .supervisor
            .replay(&source_session_id, None)
            .map_err(EngineeringRuntimeError::Supervisor)?
            .into_iter()
            .find_map(|event| match event.kind {
                EngineeringEventKind::PlanApproved { approval }
                    if approval.approval_id == approval_id =>
                {
                    Some(approval)
                }
                _ => None,
            })
            .ok_or(EngineeringRuntimeError::PlanHandoffFailed)?;
        verify_plan_approval(&approval).map_err(|_| EngineeringRuntimeError::PlanHandoffFailed)?;
        if approval.session_id != source_session_id
            || approval.plan_artifact_id != plan_artifact_id
            || approval.plan_sha256 != plan_sha256
            || approval.approval_sha256 != approval_sha256
            || approval.approved_by != self.approval_actor
        {
            return Err(EngineeringRuntimeError::PlanHandoffFailed);
        }
        let handoff = seal_plan_handoff(
            source_session_id,
            target_session_id.clone(),
            target_mode,
            plan_artifact_id,
            plan_sha256,
            approval_id,
            approval_sha256,
            self.approval_actor.clone(),
            occurred_at_epoch_ms,
        )
        .map_err(|_| EngineeringRuntimeError::PlanHandoffFailed)?;
        match self.supervisor.open_session(&target_session_id) {
            Ok(snapshot) => {
                let events = self
                    .supervisor
                    .replay(&target_session_id, None)
                    .map_err(EngineeringRuntimeError::Supervisor)?;
                if events.len() == 1
                    && matches!(
                        &events[0].kind,
                        EngineeringEventKind::SessionCreatedFromPlan { handoff: existing }
                            if existing.as_ref() == &handoff
                    )
                {
                    return Ok(EngineeringRpcResponse::Session { snapshot });
                }
                return Err(EngineeringRuntimeError::PlanHandoffFailed);
            }
            Err(PersistentSupervisorError::NotFound) => {}
            Err(error) => return Err(EngineeringRuntimeError::Supervisor(error)),
        }
        self.supervisor
            .create_session_from_plan(
                target_session_id,
                title,
                target_mode,
                correlation_id,
                occurred_at_epoch_ms,
                handoff,
            )
            .map(|snapshot| EngineeringRpcResponse::Session { snapshot })
            .map_err(EngineeringRuntimeError::Supervisor)
    }

    fn bind_approved_plan_runtime(
        &mut self,
        session_id: SessionId,
        run_request: RuntimeRunRequest,
        correlation_id: agentmage_kernel_contracts::CorrelationId,
        occurred_at_epoch_ms: u64,
    ) -> Result<EngineeringRpcResponse, EngineeringRuntimeError> {
        if occurred_at_epoch_ms == 0 {
            return Err(EngineeringRuntimeError::RuntimeBindingFailed);
        }
        let snapshot = self
            .supervisor
            .open_session(&session_id)
            .map_err(EngineeringRuntimeError::Supervisor)?;
        if snapshot.mode != EngineeringSessionMode::Agent || snapshot.terminal.is_some() {
            return Err(EngineeringRuntimeError::RuntimeBindingFailed);
        }
        enforce_engineering_mode(snapshot.mode, EngineeringModeOperation::LocalEffect)
            .map_err(|_| EngineeringRuntimeError::ModeDenied)?;
        let events = self
            .supervisor
            .replay(&session_id, None)
            .map_err(EngineeringRuntimeError::Supervisor)?;
        let handoff = match events.first().map(|event| &event.kind) {
            Some(EngineeringEventKind::SessionCreatedFromPlan { handoff }) => handoff.clone(),
            _ => return Err(EngineeringRuntimeError::RuntimeBindingFailed),
        };
        let source = self
            .supervisor
            .open_session(&handoff.source_session_id)
            .map_err(EngineeringRuntimeError::Supervisor)?;
        let plan = source
            .artifacts
            .iter()
            .find(|capture| capture.artifact_id == handoff.plan_artifact_id)
            .ok_or(EngineeringRuntimeError::RuntimeBindingFailed)?;
        if source.mode != EngineeringSessionMode::Plan
            || plan.source_kind != ArtifactSourceKind::Generated
            || plan.source_sha256 != handoff.plan_sha256
        {
            return Err(EngineeringRuntimeError::RuntimeBindingFailed);
        }
        let binding = seal_runtime_binding(&handoff, &run_request)
            .map_err(|_| EngineeringRuntimeError::RuntimeBindingFailed)?;
        verify_runtime_binding(&binding, &handoff, &run_request)
            .map_err(|_| EngineeringRuntimeError::RuntimeBindingFailed)?;
        for event in events.into_iter().skip(1) {
            if let EngineeringEventKind::RuntimeBound { binding: existing } = &event.kind {
                if existing.as_ref() == &binding {
                    return Ok(EngineeringRpcResponse::RuntimeBound {
                        binding: existing.clone(),
                        event: Box::new(event),
                    });
                }
                return Err(EngineeringRuntimeError::RuntimeBindingFailed);
            }
        }
        let event = self
            .supervisor
            .record(
                &session_id,
                correlation_id,
                occurred_at_epoch_ms,
                EngineeringEventKind::RuntimeBound {
                    binding: Box::new(binding.clone()),
                },
            )
            .map_err(EngineeringRuntimeError::Supervisor)?;
        Ok(EngineeringRpcResponse::RuntimeBound {
            binding: Box::new(binding),
            event: Box::new(event),
        })
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
        enforce_engineering_mode(snapshot.mode, EngineeringModeOperation::ArtifactIngestion)
            .map_err(|_| EngineeringRuntimeError::ModeDenied)?;
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
        enforce_engineering_mode(snapshot.mode, EngineeringModeOperation::ModelInference)
            .map_err(|_| EngineeringRuntimeError::ModeDenied)?;
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
                session_id: session_id.clone(),
                mode: snapshot.mode,
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
        let plan_artifact = if snapshot.mode == EngineeringSessionMode::Plan {
            enforce_engineering_mode(snapshot.mode, EngineeringModeOperation::PlanArtifact)
                .map_err(|_| EngineeringRuntimeError::ModeDenied)?;
            Some(self.persist_generated(
                &session_id,
                &prompt_artifact_id,
                &format!("plan-{}", correlation_id.as_str()),
                "Verified Chat plan draft",
                "text/markdown",
                output_text.as_bytes(),
                occurred_at_epoch_ms,
            )?)
        } else {
            None
        };
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
            turn: Box::new(VerifiedModelTurnResult {
                schema_version: CONTRACT_SCHEMA_VERSION,
                session_id,
                run_id,
                context,
                output_sha256: sha256(output_text.as_bytes()),
                output_text,
                terminal: EngineeringTerminalState::Success,
            }),
            plan_artifact,
        })
    }
}

impl EngineeringRuntimePort for EngineeringRuntimeService {
    fn handle(
        &mut self,
        request: EngineeringRpcRequest,
    ) -> Result<EngineeringRpcResponse, EngineeringRuntimeError> {
        use EngineeringRpcRequest::{
            ApprovePlan, BeginArtifact, BindApprovedPlanRuntime, CancelArtifact, CancelSession,
            CommitArtifact, CreateSession, CreateSessionFromApprovedPlan, ExecuteVerifiedTurn,
            IngestArtifact, ListSessions, OpenSession, PauseSession, ReadArtifactRange,
            ReplayEvents, ResumeSession, UploadArtifactChunk,
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
            ApprovePlan {
                session_id,
                plan_artifact_id,
                plan_sha256,
                approval_id,
                correlation_id,
                occurred_at_epoch_ms,
            } => self.approve_plan(
                session_id,
                plan_artifact_id,
                plan_sha256,
                approval_id,
                correlation_id,
                occurred_at_epoch_ms,
            ),
            CreateSessionFromApprovedPlan {
                source_session_id,
                plan_artifact_id,
                plan_sha256,
                approval_id,
                approval_sha256,
                target_session_id,
                title,
                target_mode,
                correlation_id,
                occurred_at_epoch_ms,
            } => self.create_session_from_approved_plan(
                source_session_id,
                plan_artifact_id,
                plan_sha256,
                approval_id,
                approval_sha256,
                target_session_id,
                title,
                target_mode,
                correlation_id,
                occurred_at_epoch_ms,
            ),
            BindApprovedPlanRuntime {
                session_id,
                run_request,
                correlation_id,
                occurred_at_epoch_ms,
            } => self.bind_approved_plan_runtime(
                session_id,
                *run_request,
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
        ActorId, ApprovalId, ArtifactSourceKind, ArtifactUploadChunk, ArtifactUploadId,
        CloudSynchronizationMarker, CorrelationId, EndpointProfileId, EngineeringEventKind,
        EngineeringRpcRequest, EngineeringRpcResponse, EngineeringSessionMode, ModelProfileId,
        RouteDecisionId, SessionId, StorageFilesystemClass, StrictLocalStorageObservation,
    };
    use agentmage_kernel_engine::engineering_persistence::SqlCipherEngineeringStore;
    use agentmage_kernel_engine::operational_store::{
        OperationalStore, OperationalStoreKeyError, OperationalStoreKeyProvider,
    };
    use agentmage_kernel_engine::runtime_coordinator::seal_runtime_run_request;

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
        expected_mode: EngineeringSessionMode,
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
                || input.mode != self.expected_mode
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
        let mut runtime = EngineeringRuntimeService::new(
            adapter.clone(),
            ActorId::from_raw("user-exact-turn-fixture"),
        );
        runtime.install_model(Box::new(ExactSentinelModel {
            expected_sha256: source_sha256.clone(),
            expected_mode: EngineeringSessionMode::Ask,
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
        let EngineeringRpcResponse::VerifiedTurnCompleted {
            turn,
            plan_artifact,
        } = turn
        else {
            panic!("turn response expected");
        };
        assert!(plan_artifact.is_none());
        assert_eq!(turn.output_text, "exact-context-sentinels-verified");
        assert_eq!(turn.context.inline_bytes, capture.byte_length);
        drop(runtime);

        let mut reopened =
            EngineeringRuntimeService::new(adapter, ActorId::from_raw("user-exact-turn-fixture"));
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
    fn plan_turn_persists_exact_hash_bound_markdown_without_implementation_authority() {
        let (directory, adapter) = store();
        let session_id = SessionId::from_raw("session-plan-turn-fixture");
        let upload_id = ArtifactUploadId::from_raw("upload-plan-turn-fixture");
        let source = b"produce a bounded implementation plan".to_vec();
        let source_sha256 = sha256(&source);
        let expected_output = "exact-context-sentinels-verified";
        let mut runtime = EngineeringRuntimeService::new(
            adapter.clone(),
            ActorId::from_raw("user-plan-turn-fixture"),
        );
        runtime.install_model(Box::new(ExactSentinelModel {
            expected_sha256: source_sha256.clone(),
            expected_mode: EngineeringSessionMode::Plan,
            sentinels: Vec::new(),
        }));
        runtime
            .handle(EngineeringRpcRequest::CreateSession {
                session_id: session_id.clone(),
                title: "Durable plan turn".to_owned(),
                mode: EngineeringSessionMode::Plan,
                correlation_id: CorrelationId::from_raw("correlation-plan-create"),
                occurred_at_epoch_ms: 100,
            })
            .unwrap();
        runtime
            .handle(EngineeringRpcRequest::BeginArtifact {
                upload_id: upload_id.clone(),
                session_id: session_id.clone(),
                source_kind: ArtifactSourceKind::Paste,
                display_name: "Plan request".to_owned(),
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
                    bytes: source,
                    chunk_sha256: source_sha256,
                    final_chunk: true,
                },
            })
            .unwrap();
        let EngineeringRpcResponse::ArtifactCaptured { capture } = runtime
            .handle(EngineeringRpcRequest::CommitArtifact {
                upload_id,
                completed_at_epoch_ms: 110,
            })
            .unwrap()
        else {
            panic!("capture response expected");
        };
        let EngineeringRpcResponse::VerifiedTurnCompleted {
            turn,
            plan_artifact: Some(plan_artifact),
        } = runtime
            .handle(EngineeringRpcRequest::ExecuteVerifiedTurn {
                session_id: session_id.clone(),
                prompt_artifact_id: capture.artifact_id,
                correlation_id: CorrelationId::from_raw("correlation-plan-turn"),
                occurred_at_epoch_ms: 120,
            })
            .unwrap()
        else {
            panic!("Plan mode must return a durable plan artifact");
        };
        assert_eq!(turn.output_text, expected_output);
        assert_eq!(plan_artifact.media_type, "text/markdown");
        assert_eq!(
            plan_artifact.source_sha256,
            sha256(expected_output.as_bytes())
        );
        assert_eq!(plan_artifact.byte_length, expected_output.len() as u64);
        let approval_request = EngineeringRpcRequest::ApprovePlan {
            session_id: session_id.clone(),
            plan_artifact_id: plan_artifact.artifact_id.clone(),
            plan_sha256: plan_artifact.source_sha256.clone(),
            approval_id: ApprovalId::from_raw("approval-plan-turn-fixture"),
            correlation_id: CorrelationId::from_raw("correlation-plan-approval"),
            occurred_at_epoch_ms: 130,
        };
        let EngineeringRpcResponse::PlanApproved { approval, event } =
            runtime.handle(approval_request.clone()).unwrap()
        else {
            panic!("Plan approval response expected");
        };
        assert_eq!(approval.plan_sha256, plan_artifact.source_sha256);
        assert_eq!(approval.plan_artifact_id, plan_artifact.artifact_id);
        assert!(matches!(
            event.kind,
            EngineeringEventKind::PlanApproved { .. }
        ));
        let EngineeringRpcResponse::PlanApproved {
            approval: replayed_approval,
            event: replayed_event,
        } = runtime.handle(approval_request).unwrap()
        else {
            panic!("idempotent Plan approval response expected");
        };
        assert_eq!(replayed_approval, approval);
        assert_eq!(replayed_event, event);
        let target_session_id = SessionId::from_raw("session-agent-from-plan-fixture");
        let handoff_request = EngineeringRpcRequest::CreateSessionFromApprovedPlan {
            source_session_id: session_id.clone(),
            plan_artifact_id: plan_artifact.artifact_id.clone(),
            plan_sha256: plan_artifact.source_sha256.clone(),
            approval_id: approval.approval_id.clone(),
            approval_sha256: approval.approval_sha256.clone(),
            target_session_id: target_session_id.clone(),
            title: "Agent from approved Plan".to_owned(),
            target_mode: EngineeringSessionMode::Agent,
            correlation_id: CorrelationId::from_raw("correlation-plan-handoff"),
            occurred_at_epoch_ms: 140,
        };
        let EngineeringRpcResponse::Session { snapshot: target } =
            runtime.handle(handoff_request.clone()).unwrap()
        else {
            panic!("approved Plan handoff session expected");
        };
        assert_eq!(target.session_id, target_session_id);
        assert_eq!(target.mode, EngineeringSessionMode::Agent);
        assert!(target.artifacts.is_empty());
        let EngineeringRpcResponse::Session {
            snapshot: replayed_target,
        } = runtime.handle(handoff_request).unwrap()
        else {
            panic!("idempotent approved Plan handoff expected");
        };
        assert_eq!(replayed_target, target);
        let (_, mut run_request) = crate::coding_run::tests::fixture_profile_and_request();
        run_request.session_id = target_session_id.clone();
        run_request.task.session_id = target_session_id.clone();
        run_request.task.objective = expected_output.to_owned();
        run_request.work_packet.objective = expected_output.to_owned();
        run_request.request_sha256 = "0".repeat(64);
        let run_request = seal_runtime_run_request(run_request).unwrap();
        let binding_request = EngineeringRpcRequest::BindApprovedPlanRuntime {
            session_id: target_session_id.clone(),
            run_request: Box::new(run_request.clone()),
            correlation_id: CorrelationId::from_raw("correlation-runtime-binding"),
            occurred_at_epoch_ms: 150,
        };
        let EngineeringRpcResponse::RuntimeBound { binding, event } =
            runtime.handle(binding_request.clone()).unwrap()
        else {
            panic!("approved Plan runtime binding expected");
        };
        assert_eq!(binding.session_id, target_session_id);
        assert_eq!(binding.plan_sha256, plan_artifact.source_sha256);
        assert_eq!(binding.request_sha256, run_request.request_sha256);
        assert!(matches!(
            event.kind,
            EngineeringEventKind::RuntimeBound { .. }
        ));
        let EngineeringRpcResponse::RuntimeBound {
            binding: replayed_binding,
            event: replayed_binding_event,
        } = runtime.handle(binding_request).unwrap()
        else {
            panic!("idempotent runtime binding expected");
        };
        assert_eq!(replayed_binding, binding);
        assert_eq!(replayed_binding_event, event);
        let EngineeringRpcResponse::Events {
            events: target_events,
        } = runtime
            .handle(EngineeringRpcRequest::ReplayEvents {
                session_id: target_session_id.clone(),
                after_sequence: None,
            })
            .unwrap()
        else {
            panic!("target events expected");
        };
        assert!(matches!(
            target_events.as_slice(),
            [agentmage_kernel_contracts::EngineeringEvent {
                kind: EngineeringEventKind::SessionCreatedFromPlan { handoff },
                ..
            }, agentmage_kernel_contracts::EngineeringEvent {
                kind: EngineeringEventKind::RuntimeBound { binding: durable },
                ..
            }] if handoff.approval_sha256 == approval.approval_sha256
                && handoff.plan_sha256 == plan_artifact.source_sha256
                && durable.as_ref() == binding.as_ref()
        ));
        let mut substituted_request = run_request;
        substituted_request.task.objective = "substituted plan".to_owned();
        substituted_request.work_packet.objective = "substituted plan".to_owned();
        substituted_request.request_sha256 = "0".repeat(64);
        let substituted_request = seal_runtime_run_request(substituted_request).unwrap();
        assert_eq!(
            runtime.handle(EngineeringRpcRequest::BindApprovedPlanRuntime {
                session_id: target_session_id,
                run_request: Box::new(substituted_request),
                correlation_id: CorrelationId::from_raw("correlation-runtime-substitution"),
                occurred_at_epoch_ms: 151,
            }),
            Err(EngineeringRuntimeError::RuntimeBindingFailed)
        );
        assert_eq!(
            runtime.handle(EngineeringRpcRequest::CreateSessionFromApprovedPlan {
                source_session_id: session_id.clone(),
                plan_artifact_id: plan_artifact.artifact_id.clone(),
                plan_sha256: plan_artifact.source_sha256.clone(),
                approval_id: approval.approval_id.clone(),
                approval_sha256: "e".repeat(64),
                target_session_id: SessionId::from_raw("session-agent-substitution"),
                title: "Substituted handoff".to_owned(),
                target_mode: EngineeringSessionMode::Agent,
                correlation_id: CorrelationId::from_raw("correlation-handoff-substitution"),
                occurred_at_epoch_ms: 141,
            }),
            Err(EngineeringRuntimeError::PlanHandoffFailed)
        );
        assert_eq!(
            runtime.handle(EngineeringRpcRequest::ApprovePlan {
                session_id: session_id.clone(),
                plan_artifact_id: plan_artifact.artifact_id.clone(),
                plan_sha256: "f".repeat(64),
                approval_id: ApprovalId::from_raw("approval-plan-turn-substitution"),
                correlation_id: CorrelationId::from_raw("correlation-plan-substitution"),
                occurred_at_epoch_ms: 131,
            }),
            Err(EngineeringRuntimeError::PlanApprovalFailed)
        );

        drop(runtime);
        let mut reopened =
            EngineeringRuntimeService::new(adapter, ActorId::from_raw("user-plan-turn-fixture"));
        let EngineeringRpcResponse::Session { snapshot } = reopened
            .handle(EngineeringRpcRequest::OpenSession { session_id })
            .unwrap()
        else {
            panic!("session response expected");
        };
        assert_eq!(snapshot.mode, EngineeringSessionMode::Plan);
        assert_eq!(snapshot.artifacts.len(), 2);
        assert_eq!(snapshot.artifacts[1], plan_artifact);
        assert!(snapshot.terminal.is_none());
        let EngineeringRpcResponse::Events { events } = reopened
            .handle(EngineeringRpcRequest::ReplayEvents {
                session_id: snapshot.session_id,
                after_sequence: None,
            })
            .unwrap()
        else {
            panic!("events response expected");
        };
        assert!(matches!(
            events.last().map(|event| &event.kind),
            Some(EngineeringEventKind::PlanApproved { approval: durable })
                if durable.as_ref() == approval.as_ref()
        ));
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn multi_range_ingestion_is_durable_and_idempotent() {
        let (directory, adapter) = store();
        let session_id = SessionId::from_raw("session-large-ingestion-fixture");
        let upload_id = ArtifactUploadId::from_raw("upload-large-ingestion-fixture");
        let source = vec![b'x'; 1_200_000];
        let source_sha256 = sha256(&source);
        let mut runtime = EngineeringRuntimeService::new(
            adapter.clone(),
            ActorId::from_raw("user-ingestion-fixture"),
        );
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

        let mut reopened =
            EngineeringRuntimeService::new(adapter, ActorId::from_raw("user-ingestion-fixture"));
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
