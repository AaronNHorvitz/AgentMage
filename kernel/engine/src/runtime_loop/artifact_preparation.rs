//! Pure, invocation-local artifact preparation owned by the existing coordinator.

use super::*;

pub(super) struct PreparedToolArtifact {
    pub(super) manifest: RuntimeArtifactManifest,
}

impl PreparedToolArtifact {
    fn matches(&self, candidate: &RuntimeToolArtifactCandidate) -> bool {
        self.manifest.kind == candidate.kind
            && self.manifest.media_type == candidate.media_type
            && self.manifest.byte_size == candidate.bytes.len() as u64
            && self.manifest.payload_sha256 == sha256(&candidate.bytes)
    }
}

pub(super) struct ToolCompletionBuilder<'a, C> {
    request: &'a RuntimeRunRequest,
    definition: &'a ToolDefinition,
    call: &'a ToolCall,
    started: &'a RuntimeEvent,
    start_observed: &'a Cell<bool>,
    clock: &'a mut C,
    resources: RuntimeResourceLedger,
    artifact_offset: u64,
    artifact_available: bool,
    prepared: Vec<PreparedToolArtifact>,
    receipt: Option<(ReceiptId, String)>,
    observed_at: Option<u64>,
    attempts: u8,
    failed: bool,
    terminal: Option<RuntimeEvent>,
    execution: Option<ToolExecutionObservation>,
}

impl<'a, C: RuntimeClock> ToolCompletionBuilder<'a, C> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        request: &'a RuntimeRunRequest,
        definition: &'a ToolDefinition,
        call: &'a ToolCall,
        started: &'a RuntimeEvent,
        start_observed: &'a Cell<bool>,
        clock: &'a mut C,
        resources: RuntimeResourceLedger,
        artifact_offset: u64,
        artifact_available: bool,
    ) -> Self {
        Self {
            request,
            definition,
            call,
            started,
            start_observed,
            clock,
            resources,
            artifact_offset,
            artifact_available,
            prepared: Vec::new(),
            receipt: None,
            observed_at: None,
            attempts: 0,
            failed: false,
            terminal: None,
            execution: None,
        }
    }

    pub(super) fn matches_return(&self, commit: &RuntimeToolCorrectnessCommit) -> bool {
        !self.failed
            && self.attempts == 1
            && valid_tool_execution(&commit.execution, self.definition, self.call, self.request)
            && tool_execution_observation(&commit.execution)
                .is_ok_and(|observed| self.execution.as_ref() == Some(&observed))
            && self
                .terminal
                .as_ref()
                .is_some_and(|terminal| commit.events == [self.started.clone(), terminal.clone()])
    }

    pub(super) fn into_parts(self) -> (RuntimeResourceLedger, Option<Vec<PreparedToolArtifact>>) {
        let prepared = self.receipt.map(|_| self.prepared);
        (self.resources, prepared)
    }

    fn observation_time(&mut self) -> Result<u64, RuntimePortFailure> {
        if let Some(now) = self.observed_at {
            return Ok(now);
        }
        let now = self.clock.now_epoch_ms()?;
        if now == 0 || now < self.started.occurred_at_epoch_ms {
            return Err(RuntimePortFailure::Invalid);
        }
        self.observed_at = Some(now);
        Ok(now)
    }

    fn prepare(
        &mut self,
        receipt_id: &ReceiptId,
        receipt_sha256: &str,
        candidates: &[RuntimeToolArtifactCandidate],
    ) -> Result<Vec<RuntimeArtifactRef>, RuntimePortFailure> {
        if self.failed
            || self.attempts != 0
            || !self.artifact_available
            || !self.start_observed.get()
            || !valid_identifier(receipt_id.as_str())
            || !valid_sha256(receipt_sha256)
            || candidates.is_empty()
            || candidates.len() > MAX_RUNTIME_TOOL_ARTIFACT_CANDIDATES
            || self.prepared.len() + candidates.len() > MAX_RUNTIME_TOOL_ARTIFACT_CANDIDATES
            || self
                .receipt
                .as_ref()
                .is_some_and(|(id, digest)| id != receipt_id || digest != receipt_sha256)
        {
            return Err(RuntimePortFailure::Invalid);
        }
        let mut resources = self.resources.clone();
        // Bound the whole append before hashing or constructing its manifests.
        for candidate in candidates {
            let bytes = candidate.bytes.len() as u64;
            if candidate.kind == RuntimeArtifactKind::ModelOutput
                || !valid_media_type(&candidate.media_type)
                || bytes == 0
                || bytes > self.request.limits.max_output_bytes
            {
                return Err(RuntimePortFailure::Invalid);
            }
            resources
                .consume(BudgetResource::OutputBytes, bytes)
                .and_then(|()| resources.admit_artifact(bytes))
                .map_err(|_| RuntimePortFailure::ResourceExhausted)?;
        }
        self.check_event_count(self.prepared.len() + candidates.len(), false)?;
        let now = self.observation_time()?;
        let mut prepared = Vec::with_capacity(candidates.len());
        let mut references = Vec::with_capacity(candidates.len());
        for (index, candidate) in candidates.iter().enumerate() {
            let ordinal = self
                .artifact_offset
                .checked_add((self.prepared.len() + index) as u64 + 1)
                .ok_or(RuntimePortFailure::Invalid)?;
            let manifest = prepare_artifact_manifest(
                self.request,
                ordinal,
                now,
                &candidate.bytes,
                &candidate.media_type,
                candidate.kind,
                self.started.turn_id.as_ref(),
                self.started.operation_id.as_ref(),
                Some(receipt_id),
                true,
            )?;
            references
                .push(runtime_artifact_ref(&manifest).map_err(|_| RuntimePortFailure::Invalid)?);
            prepared.push(PreparedToolArtifact { manifest });
        }
        self.resources = resources;
        self.prepared.extend(prepared);
        self.receipt = Some((receipt_id.clone(), receipt_sha256.to_owned()));
        Ok(references)
    }

    fn check_event_count(
        &self,
        candidates: usize,
        output_artifact: bool,
    ) -> Result<(), RuntimePortFailure> {
        // The terminal and every planned artifact need a distinct successor. This
        // is only preparation capacity; normal event-envelope accounting still
        // happens at publication, and later checkpoints keep their own admission.
        let additional = candidates as u64 + 1 + u64::from(output_artifact);
        if u64::from(self.resources.snapshot().event_count) + additional
            > u64::from(self.resources.limits().run_events)
            || self.started.sequence + additional >= u64::from(self.request.limits.max_events)
        {
            return Err(RuntimePortFailure::ResourceExhausted);
        }
        Ok(())
    }

    fn seal(
        &mut self,
        execution: &RuntimeToolExecution,
    ) -> Result<RuntimeEvent, RuntimePortFailure> {
        self.attempts = self.attempts.saturating_add(1);
        if self.failed
            || self.attempts != 1
            || !valid_tool_execution(execution, self.definition, self.call, self.request)
        {
            return Err(RuntimePortFailure::Invalid);
        }
        if let Some((receipt_id, receipt_sha256)) = &self.receipt {
            if receipt_id != &execution.receipt_id
                || receipt_sha256 != &execution.receipt_sha256
                || self.prepared.len() != execution.artifact_candidates.len()
                || !self
                    .prepared
                    .iter()
                    .zip(&execution.artifact_candidates)
                    .all(|(prepared, candidate)| prepared.matches(candidate))
            {
                return Err(RuntimePortFailure::Invalid);
            }
            // Output remains routed by the existing owner after the prepared
            // sequence. Check that it also fits; do not charge it twice.
            let mut after_output = self.resources.clone();
            let output_artifact = if let Some(payload) = &execution.result.output {
                let bytes = payload.bytes.len() as u64;
                after_output
                    .consume(BudgetResource::OutputBytes, bytes)
                    .map_err(|_| RuntimePortFailure::ResourceExhausted)?;
                let artifact = execution.result.outcome != OperationOutcome::Succeeded
                    || payload.bytes.len() > MAX_RUNTIME_INLINE_OUTPUT_BYTES;
                if artifact {
                    after_output
                        .admit_artifact(bytes)
                        .map_err(|_| RuntimePortFailure::ResourceExhausted)?;
                }
                artifact
            } else {
                false
            };
            self.check_event_count(self.prepared.len(), output_artifact)?;
        }
        let now = self.observation_time()?;
        let event = prepare_runtime_event(
            self.request,
            &self.started.correlation_id,
            Some(self.started),
            now,
            runtime_tool_terminal_event(execution, self.call)?,
            self.started.turn_id.as_ref(),
            self.started.operation_id.as_ref(),
            true,
            &mut self.resources,
        )?;
        self.execution =
            Some(tool_execution_observation(execution).map_err(|_| RuntimePortFailure::Invalid)?);
        self.terminal = Some(event.clone());
        Ok(event)
    }
}

impl<C: RuntimeClock> RuntimeToolTerminalBuilder for ToolCompletionBuilder<'_, C> {
    fn prepare_artifacts(
        &mut self,
        receipt_id: &ReceiptId,
        receipt_sha256: &str,
        candidates: &[RuntimeToolArtifactCandidate],
    ) -> Result<Vec<RuntimeArtifactRef>, RuntimePortFailure> {
        let result = self.prepare(receipt_id, receipt_sha256, candidates);
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    fn build_terminal_event(
        &mut self,
        execution: &RuntimeToolExecution,
    ) -> Result<RuntimeEvent, RuntimePortFailure> {
        let result = self.seal(execution);
        if result.is_err() {
            self.failed = true;
        }
        result
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn prepare_artifact_manifest(
    request: &RuntimeRunRequest,
    ordinal: u64,
    created_at_epoch_ms: u64,
    bytes: &[u8],
    media_type: &str,
    kind: RuntimeArtifactKind,
    turn_id: Option<&RuntimeTurnId>,
    operation_id: Option<&RuntimeOperationId>,
    receipt_id: Option<&ReceiptId>,
    retain_preview: bool,
) -> Result<RuntimeArtifactManifest, RuntimePortFailure> {
    seal_runtime_artifact_manifest(RuntimeArtifactManifest {
        schema_version: CONTRACT_SCHEMA_VERSION,
        artifact_id: RuntimeArtifactId::from_raw(derived_id(
            "artifact",
            request.run_id.as_str(),
            ordinal,
        )),
        kind,
        payload_sha256: sha256(bytes),
        byte_size: bytes.len() as u64,
        media_type: media_type.to_owned(),
        sensitivity: runtime_sensitivity(request),
        retention: RuntimeEventRetention {
            kind: RuntimeEventRetentionKind::Session,
            expires_at_epoch_ms: None,
        },
        session_id: request.session_id.clone(),
        task_id: request.task.task_id.clone(),
        producer_run_id: request.run_id.clone(),
        producer_turn_id: turn_id.cloned(),
        producer_operation_id: operation_id.cloned(),
        receipt_id: receipt_id.cloned(),
        policy_id: request.policy_id.clone(),
        policy_sha256: request.policy_sha256.clone(),
        created_at_epoch_ms,
        integrity: RuntimeArtifactIntegrityState::Verified,
        preview: retain_preview
            .then(|| runtime_artifact_preview(bytes))
            .flatten(),
        manifest_sha256: ZERO_SHA256.to_owned(),
    })
    .map_err(|_| RuntimePortFailure::Invalid)
}
