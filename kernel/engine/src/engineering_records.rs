//! Deterministic semantic validation for canonical Engineering Runtime records.

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, CanonicalArtifactEnvelope, CanonicalArtifactIngestionResult,
    CanonicalArtifactTransformation, CanonicalCapabilityManifest, CanonicalCaptureState,
    CanonicalContextDeliveryReceipt, CanonicalContextDisposition, CanonicalContextManifest,
    CanonicalDeliveryOutcome, CanonicalModelEndpointProfile, CanonicalModelRouteDecision,
    CanonicalTerminalOutcome, CanonicalTerminalResult, CanonicalToolObservation,
    CanonicalVerificationOutcome, CanonicalVerificationResult, CanonicalWorkflowCheckpoint,
    CanonicalWorkflowDefinition, CanonicalWorkflowLifecycle, CanonicalWorkflowState,
    VersionedContract, to_canonical_json,
};
use sha2::{Digest, Sha256};

const MAX_SHORT_TEXT: usize = 512;
const MAX_IDENTIFIER: usize = 128;
const MAX_LIST: usize = 256;
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Stable semantic validation failure at the trusted runtime boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalRecordError {
    /// Stable machine-readable failure code.
    pub code: &'static str,
    /// Closed top-level field associated with the failure.
    pub field: &'static str,
}

impl std::fmt::Display for CanonicalRecordError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} at {}", self.code, self.field)
    }
}

impl std::error::Error for CanonicalRecordError {}

/// Semantic validator implemented by each canonical top-level record.
pub trait ValidateCanonicalRecord: VersionedContract {
    /// Rejects schema-valid but semantically inconsistent records.
    fn validate_canonical(&self) -> Result<(), CanonicalRecordError>;
}

/// Closed canonical Engineering Runtime record families introduced by Story 1.3.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CanonicalRuntimeRecordKind {
    /// Immutable source artifact envelope.
    ArtifactEnvelope,
    /// Reproducible source-to-derivative transformation.
    ArtifactTransformation,
    /// Terminal artifact-ingestion disposition.
    ArtifactIngestionResult,
    /// Complete model-context accounting manifest.
    ContextManifest,
    /// Immutable workflow graph and policy surface.
    WorkflowDefinition,
    /// Current rebuildable workflow projection.
    WorkflowState,
    /// Terminal observation of one admitted tool attempt.
    ToolObservation,
    /// Current deterministic verifier result.
    VerificationResult,
    /// Runtime-verifier-established terminal result.
    TerminalResult,
}

/// Existing authority that owns one persisted part of a canonical record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanonicalRecordAuthority {
    /// Private content-addressed runtime artifact payload store.
    RuntimeArtifactStore,
    /// SQLCipher operational metadata store.
    OperationalStore,
    /// Append-only canonical runtime event journal.
    RuntimeEventJournal,
    /// Existing request-bound source-artifact service.
    SourceArtifactService,
}

/// Whether a record has a materialized current-state projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanonicalRecordProjection {
    /// History and immutable artifact references are sufficient; no current row is authoritative.
    None,
    /// Current workflow state may be materialized but must rebuild from the runtime journal.
    RebuildableWorkflowState,
}

/// Exact persistence route through existing authorities; this record owns no storage itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CanonicalRecordAuthorityRoute {
    /// Canonical record family.
    pub kind: CanonicalRuntimeRecordKind,
    /// Owner of complete canonical JSON bytes.
    pub payload_authority: CanonicalRecordAuthority,
    /// Owner of immutable identity, digest, lifecycle, retention, and reference metadata.
    pub metadata_authority: CanonicalRecordAuthority,
    /// Owner of ordered correctness history.
    pub history_authority: CanonicalRecordAuthority,
    /// Owner of authoritative source bytes, if this record directly names them.
    pub source_bytes_authority: Option<CanonicalRecordAuthority>,
    /// Optional current-state projection, never a second truth source.
    pub projection: CanonicalRecordProjection,
}

/// Returns the single closed persistence route for one canonical record family.
#[must_use]
pub const fn canonical_record_authority_route(
    kind: CanonicalRuntimeRecordKind,
) -> CanonicalRecordAuthorityRoute {
    CanonicalRecordAuthorityRoute {
        kind,
        payload_authority: CanonicalRecordAuthority::RuntimeArtifactStore,
        metadata_authority: CanonicalRecordAuthority::OperationalStore,
        history_authority: CanonicalRecordAuthority::RuntimeEventJournal,
        source_bytes_authority: match kind {
            CanonicalRuntimeRecordKind::ArtifactEnvelope => {
                Some(CanonicalRecordAuthority::SourceArtifactService)
            }
            CanonicalRuntimeRecordKind::ArtifactTransformation
            | CanonicalRuntimeRecordKind::ArtifactIngestionResult
            | CanonicalRuntimeRecordKind::ContextManifest
            | CanonicalRuntimeRecordKind::WorkflowDefinition
            | CanonicalRuntimeRecordKind::WorkflowState
            | CanonicalRuntimeRecordKind::ToolObservation
            | CanonicalRuntimeRecordKind::VerificationResult
            | CanonicalRuntimeRecordKind::TerminalResult => None,
        },
        projection: match kind {
            CanonicalRuntimeRecordKind::WorkflowState => {
                CanonicalRecordProjection::RebuildableWorkflowState
            }
            CanonicalRuntimeRecordKind::ArtifactEnvelope
            | CanonicalRuntimeRecordKind::ArtifactTransformation
            | CanonicalRuntimeRecordKind::ArtifactIngestionResult
            | CanonicalRuntimeRecordKind::ContextManifest
            | CanonicalRuntimeRecordKind::WorkflowDefinition
            | CanonicalRuntimeRecordKind::ToolObservation
            | CanonicalRuntimeRecordKind::VerificationResult
            | CanonicalRuntimeRecordKind::TerminalResult => CanonicalRecordProjection::None,
        },
    }
}

/// Borrowed canonical record ready for validation and existing-artifact serialization.
#[derive(Clone, Copy, Debug)]
pub enum CanonicalRuntimeRecordRef<'a> {
    /// Artifact envelope record.
    ArtifactEnvelope(&'a CanonicalArtifactEnvelope),
    /// Artifact transformation record.
    ArtifactTransformation(&'a CanonicalArtifactTransformation),
    /// Artifact ingestion result record.
    ArtifactIngestionResult(&'a CanonicalArtifactIngestionResult),
    /// Context manifest record.
    ContextManifest(&'a CanonicalContextManifest),
    /// Workflow definition record.
    WorkflowDefinition(&'a CanonicalWorkflowDefinition),
    /// Workflow state record.
    WorkflowState(&'a CanonicalWorkflowState),
    /// Tool observation record.
    ToolObservation(&'a CanonicalToolObservation),
    /// Verification result record.
    VerificationResult(&'a CanonicalVerificationResult),
    /// Terminal result record.
    TerminalResult(&'a CanonicalTerminalResult),
}

impl CanonicalRuntimeRecordRef<'_> {
    /// Returns the closed record-family identity.
    #[must_use]
    pub const fn kind(self) -> CanonicalRuntimeRecordKind {
        match self {
            Self::ArtifactEnvelope(_) => CanonicalRuntimeRecordKind::ArtifactEnvelope,
            Self::ArtifactTransformation(_) => CanonicalRuntimeRecordKind::ArtifactTransformation,
            Self::ArtifactIngestionResult(_) => CanonicalRuntimeRecordKind::ArtifactIngestionResult,
            Self::ContextManifest(_) => CanonicalRuntimeRecordKind::ContextManifest,
            Self::WorkflowDefinition(_) => CanonicalRuntimeRecordKind::WorkflowDefinition,
            Self::WorkflowState(_) => CanonicalRuntimeRecordKind::WorkflowState,
            Self::ToolObservation(_) => CanonicalRuntimeRecordKind::ToolObservation,
            Self::VerificationResult(_) => CanonicalRuntimeRecordKind::VerificationResult,
            Self::TerminalResult(_) => CanonicalRuntimeRecordKind::TerminalResult,
        }
    }

    /// Returns the existing-authority route for this record.
    #[must_use]
    pub const fn authority_route(self) -> CanonicalRecordAuthorityRoute {
        canonical_record_authority_route(self.kind())
    }

    /// Validates and serializes exact canonical JSON for the existing artifact authority.
    pub fn artifact_payload(self) -> Result<Vec<u8>, CanonicalRecordError> {
        macro_rules! validate_and_encode {
            ($record:expr) => {{
                $record.validate_canonical()?;
                to_canonical_json($record).map_err(|_| error("engineering.record.encode", "record"))
            }};
        }
        match self {
            Self::ArtifactEnvelope(record) => validate_and_encode!(record),
            Self::ArtifactTransformation(record) => validate_and_encode!(record),
            Self::ArtifactIngestionResult(record) => validate_and_encode!(record),
            Self::ContextManifest(record) => validate_and_encode!(record),
            Self::WorkflowDefinition(record) => validate_and_encode!(record),
            Self::WorkflowState(record) => validate_and_encode!(record),
            Self::ToolObservation(record) => validate_and_encode!(record),
            Self::VerificationResult(record) => validate_and_encode!(record),
            Self::TerminalResult(record) => validate_and_encode!(record),
        }
    }
}

/// Returns the SHA-256 of stable compact contract JSON.
///
/// A caller sealing a record must zero its declared seal field first. The helper does not
/// guess which field is authoritative and therefore cannot accidentally bless the wrong one.
pub fn canonical_record_sha256<T: VersionedContract>(
    record: &T,
) -> Result<String, CanonicalRecordError> {
    record_version(record)?;
    let bytes =
        to_canonical_json(record).map_err(|_| error("engineering.record.encode", "record"))?;
    Ok(hex(&Sha256::digest(bytes)))
}

impl ValidateCanonicalRecord for CanonicalArtifactEnvelope {
    fn validate_canonical(&self) -> Result<(), CanonicalRecordError> {
        record_version(self)?;
        identifiers(&[&self.artifact_id, &self.request_id, &self.authority_id])?;
        short(&self.media_type, "media_type")?;
        match self.capture_state {
            CanonicalCaptureState::Captured => {
                self.byte_length
                    .ok_or_else(|| error("engineering.artifact.length_required", "byte_length"))?;
                sha(self.sha256.as_deref().unwrap_or_default(), "sha256")?;
            }
            _ if self.byte_length.is_some() || self.sha256.is_some() => {
                return Err(error(
                    "engineering.artifact.uncaptured_metadata",
                    "capture_state",
                ));
            }
            _ => {}
        }
        timestamp(&self.collected_at, "collected_at")
    }
}

impl ValidateCanonicalRecord for CanonicalArtifactIngestionResult {
    fn validate_canonical(&self) -> Result<(), CanonicalRecordError> {
        record_version(self)?;
        identifiers(&[&self.ingestion_id, &self.artifact_id])?;
        unique_identifiers(&self.transformation_ids, "transformation_ids")?;
        bounded_text_list(&self.warnings, "warnings")?;
        if !self.terminal {
            return Err(error("engineering.ingestion.not_terminal", "terminal"));
        }
        if let Some(source) = &self.source {
            artifact_reference(source, "source")?;
            if source.artifact_id != self.artifact_id {
                return Err(error(
                    "engineering.ingestion.source_identity",
                    "source.artifact_id",
                ));
            }
        }
        if let Some(code) = &self.error_code {
            identifier(code, "error_code")?;
        }
        Ok(())
    }
}

impl ValidateCanonicalRecord for CanonicalArtifactTransformation {
    fn validate_canonical(&self) -> Result<(), CanonicalRecordError> {
        record_version(self)?;
        identifiers(&[
            &self.transformation_id,
            &self.artifact_id,
            &self.transformer_id,
        ])?;
        short(&self.transformer_version, "transformer_version")?;
        sha(&self.input_sha256, "input_sha256")?;
        if let Some(output) = &self.output_sha256 {
            sha(output, "output_sha256")?;
        }
        list(&self.source_ranges, "source_ranges")?;
        for range in &self.source_ranges {
            if range.start_byte > range.end_byte_exclusive {
                return Err(error("engineering.range.reversed", "source_ranges"));
            }
        }
        bounded_text_list(&self.warnings, "warnings")
    }
}

impl ValidateCanonicalRecord for CanonicalContextManifest {
    fn validate_canonical(&self) -> Result<(), CanonicalRecordError> {
        record_version(self)?;
        identifiers(&[
            &self.context_manifest_id,
            &self.session_id,
            &self.turn_id,
            &self.model_profile_id,
        ])?;
        if self.items.len() > 4096 || self.source_artifact_count != self.items.len() as u64 {
            return Err(error(
                "engineering.context.count_invalid",
                "source_artifact_count",
            ));
        }
        let mut artifact_ids = std::collections::BTreeSet::new();
        let mut accounted_tokens = 0_u64;
        for item in &self.items {
            identifier(&item.artifact_id, "items.artifact_id")?;
            if !artifact_ids.insert(item.artifact_id.as_str()) {
                return Err(error(
                    "engineering.context.duplicate_artifact",
                    "items.artifact_id",
                ));
            }
            list(&item.ranges, "items.ranges")?;
            for range in &item.ranges {
                if range.start_byte > range.end_byte_exclusive {
                    return Err(error("engineering.range.reversed", "items.ranges"));
                }
            }
            let admits_content = matches!(
                item.disposition,
                CanonicalContextDisposition::Included
                    | CanonicalContextDisposition::Summarized
                    | CanonicalContextDisposition::Truncated
            );
            if !admits_content && (!item.ranges.is_empty() || item.token_count != 0) {
                return Err(error(
                    "engineering.context.non_admitting_content",
                    "items.disposition",
                ));
            }
            let needs_reason = !matches!(item.disposition, CanonicalContextDisposition::Included);
            if needs_reason != item.reason_code.is_some() {
                return Err(error(
                    "engineering.context.reason_binding",
                    "items.reason_code",
                ));
            }
            if let Some(code) = &item.reason_code {
                identifier(code, "items.reason_code")?;
            }
            if let Some(reason) = &item.reason {
                short(reason, "items.reason")?;
            }
            accounted_tokens = accounted_tokens
                .checked_add(item.token_count)
                .ok_or_else(|| error("engineering.context.token_overflow", "items.token_count"))?;
        }
        if accounted_tokens > self.total_input_tokens {
            return Err(error(
                "engineering.context.token_accounting",
                "total_input_tokens",
            ));
        }
        sha(&self.manifest_sha256, "manifest_sha256")
    }
}

impl ValidateCanonicalRecord for CanonicalContextDeliveryReceipt {
    fn validate_canonical(&self) -> Result<(), CanonicalRecordError> {
        record_version(self)?;
        identifiers(&[
            &self.receipt_id,
            &self.context_manifest_id,
            &self.model_request_id,
            &self.route_decision_id,
        ])?;
        sha(&self.context_manifest_sha256, "context_manifest_sha256")?;
        if self.delivered.len() > 4096 {
            return Err(error("engineering.record.list_too_large", "delivered"));
        }
        for delivered in &self.delivered {
            identifier(&delivered.artifact_id, "delivered.artifact_id")?;
            sha(&delivered.sha256, "delivered.sha256")?;
            if delivered.range.start_byte > delivered.range.end_byte_exclusive {
                return Err(error("engineering.range.reversed", "delivered.range"));
            }
        }
        match self.outcome {
            CanonicalDeliveryOutcome::Delivered
                if !self.required_unseen_artifact_ids.is_empty() =>
            {
                return Err(error(
                    "engineering.context.required_unseen",
                    "required_unseen_artifact_ids",
                ));
            }
            CanonicalDeliveryOutcome::Blocked if self.required_unseen_artifact_ids.is_empty() => {
                return Err(error(
                    "engineering.context.block_without_missing",
                    "required_unseen_artifact_ids",
                ));
            }
            _ => {}
        }
        list(
            &self.required_unseen_artifact_ids,
            "required_unseen_artifact_ids",
        )?;
        sha(&self.receipt_sha256, "receipt_sha256")
    }
}

impl ValidateCanonicalRecord for CanonicalWorkflowDefinition {
    fn validate_canonical(&self) -> Result<(), CanonicalRecordError> {
        record_version(self)?;
        identifier(&self.workflow_id, "workflow_id")?;
        if self.workflow_version == 0
            || self.steps.is_empty()
            || self.steps.len() > MAX_LIST
            || self.entry_step_ids.is_empty()
        {
            return Err(error("engineering.workflow.shape_invalid", "steps"));
        }
        schema_binding(&self.input_schema)?;
        schema_binding(&self.output_schema)?;
        let mut step_ids = std::collections::BTreeSet::new();
        for step in &self.steps {
            identifier(&step.step_id, "steps.step_id")?;
            if !step_ids.insert(step.step_id.as_str()) {
                return Err(error(
                    "engineering.workflow.duplicate_step",
                    "steps.step_id",
                ));
            }
            unique_identifiers(&step.depends_on, "steps.depends_on")?;
            if step
                .depends_on
                .iter()
                .any(|dependency| dependency == &step.step_id)
            {
                return Err(error(
                    "engineering.workflow.self_dependency",
                    "steps.depends_on",
                ));
            }
            if let Some(model_role) = &step.model_role {
                identifier(model_role, "steps.model_role")?;
            }
            if let Some(tool_id) = &step.tool_id {
                identifier(tool_id, "steps.tool_id")?;
            }
            if step.verifier_ids.is_empty() || step.verifier_ids.len() > 32 {
                return Err(error(
                    "engineering.workflow.verifier_required",
                    "steps.verifier_ids",
                ));
            }
            unique_identifiers(&step.verifier_ids, "steps.verifier_ids")?;
            if !step
                .effect_class
                .permitted_retry_classes()
                .contains(&step.retry_class)
            {
                return Err(error(
                    "engineering.workflow.retry_class",
                    "steps.retry_class",
                ));
            }
            positive_budgets(&step.budgets)?;
        }
        for step in &self.steps {
            if step
                .depends_on
                .iter()
                .any(|dependency| !step_ids.contains(dependency.as_str()))
            {
                return Err(error(
                    "engineering.workflow.unknown_dependency",
                    "steps.depends_on",
                ));
            }
        }
        if self
            .entry_step_ids
            .iter()
            .any(|entry| !step_ids.contains(entry.as_str()))
        {
            return Err(error(
                "engineering.workflow.unknown_entry",
                "entry_step_ids",
            ));
        }
        unique_identifiers(&self.entry_step_ids, "entry_step_ids")?;
        reject_workflow_cycle(self)?;
        sha(&self.definition_sha256, "definition_sha256")
    }
}

impl ValidateCanonicalRecord for CanonicalWorkflowState {
    fn validate_canonical(&self) -> Result<(), CanonicalRecordError> {
        record_version(self)?;
        identifier(&self.workflow_id, "workflow_id")?;
        if self.workflow_version == 0 {
            return Err(error(
                "engineering.workflow.version_invalid",
                "workflow_version",
            ));
        }
        unique_identifiers(&self.completed_step_ids, "completed_step_ids")?;
        unique_identifiers(&self.attempt_ids, "attempt_ids")?;
        if let Some(active_step_id) = &self.active_step_id {
            identifier(active_step_id, "active_step_id")?;
            if self.completed_step_ids.contains(active_step_id) {
                return Err(error(
                    "engineering.workflow.active_step_complete",
                    "active_step_id",
                ));
            }
        }
        sha(&self.consumed_budget_sha256, "consumed_budget_sha256")?;
        let terminal = matches!(
            self.state,
            CanonicalWorkflowLifecycle::Succeeded
                | CanonicalWorkflowLifecycle::NoOp
                | CanonicalWorkflowLifecycle::Blocked
                | CanonicalWorkflowLifecycle::Failed
                | CanonicalWorkflowLifecycle::Cancelled
                | CanonicalWorkflowLifecycle::TimedOut
                | CanonicalWorkflowLifecycle::ResourceExhausted
                | CanonicalWorkflowLifecycle::Uncertain
        );
        if terminal != self.terminal_result_id.is_some() {
            return Err(error(
                "engineering.workflow.terminal_binding",
                "terminal_result_id",
            ));
        }
        if terminal && self.active_step_id.is_some() {
            return Err(error(
                "engineering.workflow.terminal_active_step",
                "active_step_id",
            ));
        }
        if let Some(terminal_result_id) = &self.terminal_result_id {
            identifier(terminal_result_id, "terminal_result_id")?;
        }
        Ok(())
    }
}

impl ValidateCanonicalRecord for CanonicalWorkflowCheckpoint {
    fn validate_canonical(&self) -> Result<(), CanonicalRecordError> {
        record_version(self)?;
        identifiers(&[&self.checkpoint_id, &self.workflow_id])?;
        for (value, field) in [
            (&self.state_sha256, "state_sha256"),
            (&self.source_manifest_sha256, "source_manifest_sha256"),
            (&self.policy_sha256, "policy_sha256"),
            (&self.environment_sha256, "environment_sha256"),
            (&self.route_sha256, "route_sha256"),
            (&self.tool_catalog_sha256, "tool_catalog_sha256"),
        ] {
            sha(value, field)?;
        }
        sha_list(&self.receipt_sha256s, "receipt_sha256s")?;
        sha_list(&self.consumed_grant_sha256s, "consumed_grant_sha256s")?;
        timestamp(&self.created_at, "created_at")
    }
}

impl ValidateCanonicalRecord for CanonicalModelEndpointProfile {
    fn validate_canonical(&self) -> Result<(), CanonicalRecordError> {
        record_version(self)?;
        identifiers(&[
            &self.endpoint_profile_id,
            &self.operator_id,
            &self.protocol_codec_id,
        ])?;
        short(&self.endpoint_reference, "endpoint_reference")?;
        for (value, field) in [
            (&self.host_policy_sha256, "host_policy_sha256"),
            (&self.quota_policy_sha256, "quota_policy_sha256"),
            (&self.cost_policy_sha256, "cost_policy_sha256"),
        ] {
            sha(value, field)?;
        }
        if let Some(value) = &self.qualification_sha256 {
            sha(value, "qualification_sha256")?;
        }
        if self.enabled || self.automatic_fallback {
            return Err(error(
                "engineering.endpoint.activation_forbidden",
                "enabled",
            ));
        }
        Ok(())
    }
}

impl ValidateCanonicalRecord for CanonicalModelRouteDecision {
    fn validate_canonical(&self) -> Result<(), CanonicalRecordError> {
        record_version(self)?;
        identifiers(&[&self.route_decision_id, &self.request_id])?;
        sha(&self.policy_sha256, "policy_sha256")?;
        if self.considered_routes.is_empty() || self.considered_routes.len() > 64 {
            return Err(error(
                "engineering.route.considered_required",
                "considered_routes",
            ));
        }
        if self.fallback_used != self.fallback_policy_sha256.is_some() {
            return Err(error(
                "engineering.route.fallback_policy",
                "fallback_policy_sha256",
            ));
        }
        if let Some(value) = &self.fallback_policy_sha256 {
            sha(value, "fallback_policy_sha256")?;
        }
        if let Some(selected) = &self.selected_route_id
            && !self
                .considered_routes
                .iter()
                .any(|route| &route.route_id == selected && route.qualified && route.admitted)
        {
            return Err(error(
                "engineering.route.selection_not_admitted",
                "selected_route_id",
            ));
        }
        short(&self.reason, "reason")?;
        sha(&self.decision_sha256, "decision_sha256")
    }
}

impl ValidateCanonicalRecord for CanonicalToolObservation {
    fn validate_canonical(&self) -> Result<(), CanonicalRecordError> {
        record_version(self)?;
        identifiers(&[
            &self.observation_id,
            &self.tool_call_id,
            &self.attempt_id,
            &self.task_id,
            &self.step_id,
            &self.tool_id,
            &self.authority_id,
        ])?;
        for (value, field) in [
            (&self.tool_schema_sha256, "tool_schema_sha256"),
            (&self.arguments_sha256, "arguments_sha256"),
            (&self.resource_usage_sha256, "resource_usage_sha256"),
            (&self.receipt_sha256, "receipt_sha256"),
        ] {
            sha(value, field)?;
        }
        short(&self.tool_version, "tool_version")?;
        timestamp(&self.started_at, "started_at")?;
        timestamp(&self.completed_at, "completed_at")?;
        if !self.terminal {
            return Err(error("engineering.tool.not_terminal", "terminal"));
        }
        if !self.descendants_cleaned {
            return Err(error(
                "engineering.tool.descendants_unclean",
                "descendants_cleaned",
            ));
        }
        if self.stdout_excerpt.len() > 8192 || self.stderr_excerpt.len() > 8192 {
            return Err(error(
                "engineering.tool.excerpt_too_large",
                "stdout_excerpt",
            ));
        }
        if let Some(signal) = &self.signal {
            short(signal, "signal")?;
        }
        if let Some(stdout) = &self.stdout {
            artifact_reference(stdout, "stdout")?;
        }
        if let Some(stderr) = &self.stderr {
            artifact_reference(stderr, "stderr")?;
        }
        unique_identifiers(&self.generated_artifact_ids, "generated_artifact_ids")?;
        Ok(())
    }
}

impl ValidateCanonicalRecord for CanonicalCapabilityManifest {
    fn validate_canonical(&self) -> Result<(), CanonicalRecordError> {
        record_version(self)?;
        identifiers(&[&self.capability_id, &self.publisher_id])?;
        if self.capability_version == 0 {
            return Err(error(
                "engineering.capability.version_invalid",
                "capability_version",
            ));
        }
        short(&self.title, "title")?;
        short(&self.purpose, "purpose")?;
        schema_binding(&self.input_schema)?;
        schema_binding(&self.output_schema)?;
        positive_budgets(&self.budgets)?;
        if self.verifier_ids.is_empty() || self.verifier_ids.len() > 64 {
            return Err(error(
                "engineering.capability.verifier_required",
                "verifier_ids",
            ));
        }
        for (value, field) in [
            (
                &self.workflow_definition_sha256,
                "workflow_definition_sha256",
            ),
            (&self.fixture_manifest_sha256, "fixture_manifest_sha256"),
            (&self.migration_policy_sha256, "migration_policy_sha256"),
            (&self.removal_policy_sha256, "removal_policy_sha256"),
            (&self.manifest_sha256, "manifest_sha256"),
        ] {
            sha(value, field)?;
        }
        Ok(())
    }
}

impl ValidateCanonicalRecord for CanonicalVerificationResult {
    fn validate_canonical(&self) -> Result<(), CanonicalRecordError> {
        record_version(self)?;
        identifiers(&[
            &self.verification_result_id,
            &self.workflow_id,
            &self.verifier_id,
        ])?;
        if let Some(step_id) = &self.step_id {
            identifier(step_id, "step_id")?;
        }
        short(&self.verifier_version, "verifier_version")?;
        sha(&self.subject_sha256, "subject_sha256")?;
        if self.observed_evidence_sha256s.is_empty() {
            return Err(error(
                "engineering.verification.evidence_required",
                "observed_evidence_sha256s",
            ));
        }
        sha_list(&self.observed_evidence_sha256s, "observed_evidence_sha256s")?;
        if has_duplicates(&self.observed_evidence_sha256s) {
            return Err(error(
                "engineering.verification.duplicate_evidence",
                "observed_evidence_sha256s",
            ));
        }
        unique_identifiers(&self.preserved_invariants, "preserved_invariants")?;
        unique_identifiers(
            &self.prohibited_effects_observed,
            "prohibited_effects_observed",
        )?;
        if self.outcome == CanonicalVerificationOutcome::Passed
            && (!self.current || !self.prohibited_effects_observed.is_empty())
        {
            return Err(error("engineering.verification.false_success", "outcome"));
        }
        sha(&self.result_sha256, "result_sha256")
    }
}

impl ValidateCanonicalRecord for CanonicalTerminalResult {
    fn validate_canonical(&self) -> Result<(), CanonicalRecordError> {
        record_version(self)?;
        identifiers(&[&self.terminal_result_id, &self.workflow_id])?;
        if self.established_by != "agentmage-runtime-verifier" {
            return Err(error(
                "engineering.terminal.invalid_establisher",
                "established_by",
            ));
        }
        let success = matches!(
            self.outcome,
            CanonicalTerminalOutcome::VerifiedSuccess | CanonicalTerminalOutcome::VerifiedNoOp
        );
        unique_identifiers(&self.verification_result_ids, "verification_result_ids")?;
        if success {
            if self.verification_result_ids.is_empty()
                || self.diagnostic_code.is_some()
                || self.safe_next_action.is_some()
            {
                return Err(error("engineering.terminal.false_success", "outcome"));
            }
        } else if self.diagnostic_code.is_none() || self.safe_next_action.is_none() {
            return Err(error(
                "engineering.terminal.missing_recovery",
                "diagnostic_code",
            ));
        }
        if let Some(code) = &self.diagnostic_code {
            identifier(code, "diagnostic_code")?;
        }
        if let Some(action) = &self.safe_next_action {
            short(action, "safe_next_action")?;
        }
        sha(
            &self.last_verified_state_sha256,
            "last_verified_state_sha256",
        )?;
        sha(&self.result_sha256, "result_sha256")
    }
}

/// Expected request, task, session, and content identities at one runtime admission boundary.
#[derive(Clone, Copy, Debug)]
pub struct CanonicalRuntimeIdentityBindings<'a> {
    /// Request identity assigned by the host.
    pub request_id: &'a str,
    /// Task identity assigned by the kernel.
    pub task_id: &'a str,
    /// Persistent session identity assigned by the host.
    pub session_id: &'a str,
    /// SHA-256 of the authoritative source bytes.
    pub source_sha256: &'a str,
    /// SHA-256 of the admitted workflow definition.
    pub workflow_definition_sha256: &'a str,
}

/// One identity-bound terminal slice of the canonical Engineering Runtime record family.
///
/// This is a validation view only. It owns no state and creates no alternate store. The
/// caller supplies the records loaded from their existing authorities and this view proves
/// their cross-record identities agree before they are used together.
#[derive(Clone, Copy, Debug)]
pub struct CanonicalRuntimeRecordSet<'a> {
    /// Host- and kernel-assigned boundary identities.
    pub bindings: CanonicalRuntimeIdentityBindings<'a>,
    /// Authoritative source envelope.
    pub artifact: &'a CanonicalArtifactEnvelope,
    /// Ordered transformations applied to that source.
    pub transformations: &'a [CanonicalArtifactTransformation],
    /// Terminal ingestion result for that source.
    pub ingestion: &'a CanonicalArtifactIngestionResult,
    /// Context manifest that accounts for the source.
    pub context: &'a CanonicalContextManifest,
    /// Admitted workflow definition.
    pub workflow_definition: &'a CanonicalWorkflowDefinition,
    /// Current workflow state.
    pub workflow_state: &'a CanonicalWorkflowState,
    /// Terminal tool observations available to this state.
    pub observations: &'a [CanonicalToolObservation],
    /// Current deterministic verification results.
    pub verifications: &'a [CanonicalVerificationResult],
    /// Runtime-verifier-established terminal result.
    pub terminal_result: &'a CanonicalTerminalResult,
}

impl CanonicalRuntimeRecordSet<'_> {
    /// Validates every record and every cross-record identity without mutation.
    pub fn validate(&self) -> Result<(), CanonicalRecordError> {
        identifier(self.bindings.request_id, "bindings.request_id")?;
        identifier(self.bindings.task_id, "bindings.task_id")?;
        identifier(self.bindings.session_id, "bindings.session_id")?;
        sha(self.bindings.source_sha256, "bindings.source_sha256")?;
        sha(
            self.bindings.workflow_definition_sha256,
            "bindings.workflow_definition_sha256",
        )?;

        self.artifact.validate_canonical()?;
        self.ingestion.validate_canonical()?;
        self.context.validate_canonical()?;
        self.workflow_definition.validate_canonical()?;
        self.workflow_state.validate_canonical()?;
        self.terminal_result.validate_canonical()?;

        if self.artifact.request_id != self.bindings.request_id
            || self.context.session_id != self.bindings.session_id
            || self.artifact.sha256.as_deref() != Some(self.bindings.source_sha256)
            || self.workflow_definition.definition_sha256
                != self.bindings.workflow_definition_sha256
        {
            return Err(error("engineering.record.binding_mismatch", "bindings"));
        }
        if self.ingestion.artifact_id != self.artifact.artifact_id
            || self.ingestion.source.as_ref().is_none_or(|source| {
                source.artifact_id != self.artifact.artifact_id
                    || source.sha256 != self.bindings.source_sha256
                    || Some(source.byte_length) != self.artifact.byte_length
            })
        {
            return Err(error(
                "engineering.record.source_binding",
                "ingestion.source",
            ));
        }

        let mut transformation_ids = Vec::with_capacity(self.transformations.len());
        for transformation in self.transformations {
            transformation.validate_canonical()?;
            if transformation.artifact_id != self.artifact.artifact_id
                || transformation.input_sha256 != self.bindings.source_sha256
            {
                return Err(error(
                    "engineering.record.transformation_binding",
                    "transformations",
                ));
            }
            transformation_ids.push(transformation.transformation_id.as_str());
        }
        if !self
            .ingestion
            .transformation_ids
            .iter()
            .map(String::as_str)
            .eq(transformation_ids)
        {
            return Err(error(
                "engineering.record.transformation_chain",
                "ingestion.transformation_ids",
            ));
        }
        if !self
            .context
            .items
            .iter()
            .any(|item| item.artifact_id == self.artifact.artifact_id)
        {
            return Err(error("engineering.record.context_binding", "context.items"));
        }
        if self.workflow_state.workflow_id != self.workflow_definition.workflow_id
            || self.workflow_state.workflow_version != self.workflow_definition.workflow_version
            || self.workflow_state.terminal_result_id.as_deref()
                != Some(self.terminal_result.terminal_result_id.as_str())
            || self.terminal_result.workflow_id != self.workflow_definition.workflow_id
        {
            return Err(error(
                "engineering.record.workflow_binding",
                "workflow_state",
            ));
        }

        let steps = self
            .workflow_definition
            .steps
            .iter()
            .map(|step| (step.step_id.as_str(), step))
            .collect::<std::collections::BTreeMap<_, _>>();
        let mut observation_ids = std::collections::BTreeSet::new();
        let mut observed_attempt_ids = std::collections::BTreeSet::new();
        for observation in self.observations {
            observation.validate_canonical()?;
            let Some(step) = steps.get(observation.step_id.as_str()) else {
                return Err(error(
                    "engineering.record.observation_step",
                    "observations.step_id",
                ));
            };
            if observation.task_id != self.bindings.task_id
                || step.tool_id.as_deref() != Some(observation.tool_id.as_str())
                || !self
                    .workflow_state
                    .attempt_ids
                    .contains(&observation.attempt_id)
                || !observation_ids.insert(observation.observation_id.as_str())
                || !observed_attempt_ids.insert(observation.attempt_id.as_str())
            {
                return Err(error(
                    "engineering.record.observation_binding",
                    "observations",
                ));
            }
        }
        if !self
            .workflow_state
            .attempt_ids
            .iter()
            .all(|attempt_id| observed_attempt_ids.contains(attempt_id.as_str()))
        {
            return Err(error(
                "engineering.record.missing_observation",
                "workflow_state.attempt_ids",
            ));
        }

        let mut verification_ids = std::collections::BTreeSet::new();
        for verification in self.verifications {
            verification.validate_canonical()?;
            if verification.workflow_id != self.workflow_definition.workflow_id
                || verification
                    .step_id
                    .as_ref()
                    .is_some_and(|step_id| !steps.contains_key(step_id.as_str()))
                || !verification_ids.insert(verification.verification_result_id.as_str())
            {
                return Err(error(
                    "engineering.record.verification_binding",
                    "verifications",
                ));
            }
        }
        if !self
            .terminal_result
            .verification_result_ids
            .iter()
            .all(|identity| verification_ids.contains(identity.as_str()))
        {
            return Err(error(
                "engineering.record.terminal_verification_binding",
                "terminal_result.verification_result_ids",
            ));
        }
        Ok(())
    }
}

/// Validates one durable workflow transition without mutating either state.
///
/// Identity, definition version, sequence, completed steps, and attempt history are
/// monotonic. Terminal states are absorbing. The closed transition table is deliberately
/// exhaustive so adding a lifecycle variant requires an explicit policy decision.
pub fn validate_workflow_transition(
    previous: &CanonicalWorkflowState,
    next: &CanonicalWorkflowState,
) -> Result<(), CanonicalRecordError> {
    previous.validate_canonical()?;
    next.validate_canonical()?;
    if previous.workflow_id != next.workflow_id
        || previous.workflow_version != next.workflow_version
    {
        return Err(error(
            "engineering.workflow.transition_identity",
            "workflow_id",
        ));
    }
    if previous.sequence.checked_add(1) != Some(next.sequence) {
        return Err(error(
            "engineering.workflow.transition_sequence",
            "sequence",
        ));
    }
    if !workflow_transition_allowed(previous.state, next.state) {
        return Err(error("engineering.workflow.transition_invalid", "state"));
    }
    if !is_ordered_prefix(&previous.completed_step_ids, &next.completed_step_ids) {
        return Err(error(
            "engineering.workflow.completed_steps_regressed",
            "completed_step_ids",
        ));
    }
    if !is_ordered_prefix(&previous.attempt_ids, &next.attempt_ids) {
        return Err(error(
            "engineering.workflow.attempts_regressed",
            "attempt_ids",
        ));
    }
    Ok(())
}

fn workflow_transition_allowed(
    from: CanonicalWorkflowLifecycle,
    to: CanonicalWorkflowLifecycle,
) -> bool {
    use CanonicalWorkflowLifecycle as State;
    match from {
        State::Created => matches!(to, State::Validating | State::Cancelled),
        State::Validating => matches!(
            to,
            State::Ready
                | State::WaitingForDependency
                | State::Blocked
                | State::Failed
                | State::Cancelled
                | State::TimedOut
                | State::ResourceExhausted
        ),
        State::Ready => matches!(
            to,
            State::Running | State::Paused | State::Blocked | State::Cancelled
        ),
        State::Running => matches!(
            to,
            State::Verifying
                | State::WaitingForDependency
                | State::WaitingForApproval
                | State::Paused
                | State::Reconciling
                | State::Blocked
                | State::Failed
                | State::Cancelled
                | State::TimedOut
                | State::ResourceExhausted
                | State::Uncertain
        ),
        State::Verifying => matches!(
            to,
            State::Succeeded
                | State::NoOp
                | State::Running
                | State::Blocked
                | State::Failed
                | State::Cancelled
                | State::TimedOut
                | State::ResourceExhausted
                | State::Uncertain
        ),
        State::WaitingForDependency => matches!(
            to,
            State::Ready
                | State::Running
                | State::Paused
                | State::Blocked
                | State::Cancelled
                | State::TimedOut
        ),
        State::WaitingForApproval => matches!(
            to,
            State::Running | State::Paused | State::Blocked | State::Cancelled | State::TimedOut
        ),
        State::Paused => matches!(to, State::Ready | State::Running | State::Cancelled),
        State::Reconciling => matches!(
            to,
            State::Running
                | State::Recovering
                | State::Verifying
                | State::Blocked
                | State::Failed
                | State::Cancelled
                | State::Uncertain
        ),
        State::Recovering => matches!(
            to,
            State::Ready
                | State::Running
                | State::Verifying
                | State::Blocked
                | State::Failed
                | State::Cancelled
                | State::Uncertain
        ),
        State::Succeeded
        | State::NoOp
        | State::Blocked
        | State::Failed
        | State::Cancelled
        | State::TimedOut
        | State::ResourceExhausted
        | State::Uncertain => false,
    }
}

fn record_version<T: VersionedContract>(record: &T) -> Result<(), CanonicalRecordError> {
    if record.schema_version() == CONTRACT_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(error("engineering.record.version", "schema_version"))
    }
}

fn artifact_reference(
    reference: &agentmage_kernel_contracts::CanonicalArtifactReference,
    field: &'static str,
) -> Result<(), CanonicalRecordError> {
    identifier(&reference.artifact_id, field)?;
    sha(&reference.sha256, field)
}

fn bounded_text_list(values: &[String], field: &'static str) -> Result<(), CanonicalRecordError> {
    list(values, field)?;
    for value in values {
        short(value, field)?;
    }
    Ok(())
}

fn unique_identifiers(values: &[String], field: &'static str) -> Result<(), CanonicalRecordError> {
    list(values, field)?;
    for value in values {
        identifier(value, field)?;
    }
    if has_duplicates(values) {
        Err(error("engineering.record.duplicate_identity", field))
    } else {
        Ok(())
    }
}

fn has_duplicates<T: Ord>(values: &[T]) -> bool {
    let mut seen = std::collections::BTreeSet::new();
    values.iter().any(|value| !seen.insert(value))
}

fn is_ordered_prefix<T: PartialEq>(prefix: &[T], values: &[T]) -> bool {
    values.starts_with(prefix)
}

fn reject_workflow_cycle(
    definition: &CanonicalWorkflowDefinition,
) -> Result<(), CanonicalRecordError> {
    let indices = definition
        .steps
        .iter()
        .enumerate()
        .map(|(index, step)| (step.step_id.as_str(), index))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut indegrees = definition
        .steps
        .iter()
        .map(|step| step.depends_on.len())
        .collect::<Vec<_>>();
    let mut dependents = vec![Vec::new(); definition.steps.len()];
    for (step_index, step) in definition.steps.iter().enumerate() {
        for dependency in &step.depends_on {
            let dependency_index = indices[dependency.as_str()];
            dependents[dependency_index].push(step_index);
        }
    }
    let mut ready = indegrees
        .iter()
        .enumerate()
        .filter_map(|(index, indegree)| (*indegree == 0).then_some(index))
        .collect::<Vec<_>>();
    let mut visited = 0_usize;
    while let Some(index) = ready.pop() {
        visited += 1;
        for dependent in &dependents[index] {
            indegrees[*dependent] -= 1;
            if indegrees[*dependent] == 0 {
                ready.push(*dependent);
            }
        }
    }
    if visited == definition.steps.len() {
        Ok(())
    } else {
        Err(error(
            "engineering.workflow.dependency_cycle",
            "steps.depends_on",
        ))
    }
}

fn schema_binding(
    binding: &agentmage_kernel_contracts::CanonicalSchemaBinding,
) -> Result<(), CanonicalRecordError> {
    identifier(&binding.schema_id, "schema_id")?;
    if binding.schema_version == 0 {
        return Err(error(
            "engineering.schema.version_invalid",
            "schema_version",
        ));
    }
    sha(&binding.schema_sha256, "schema_sha256")
}

fn positive_budgets(
    budgets: &agentmage_kernel_contracts::CanonicalExecutionBudgets,
) -> Result<(), CanonicalRecordError> {
    if [
        budgets.turns,
        budgets.tokens,
        budgets.duration_ms,
        budgets.tool_calls,
        budgets.attempts,
        budgets.no_progress_events,
        budgets.output_bytes,
        budgets.memory_bytes,
    ]
    .contains(&0)
    {
        Err(error("engineering.budget.zero", "budgets"))
    } else {
        Ok(())
    }
}

fn identifiers(values: &[&String]) -> Result<(), CanonicalRecordError> {
    for value in values {
        identifier(value, "identifier")?;
    }
    Ok(())
}

fn identifier(value: &str, field: &'static str) -> Result<(), CanonicalRecordError> {
    let mut chars = value.chars();
    if value.is_empty()
        || value.len() > MAX_IDENTIFIER
        || !chars.next().is_some_and(|c| c.is_ascii_alphanumeric())
        || !chars.all(|c| c.is_ascii_alphanumeric() || ". _:-".contains(c))
        || value.contains(' ')
    {
        return Err(error("engineering.record.identifier", field));
    }
    Ok(())
}

fn short(value: &str, field: &'static str) -> Result<(), CanonicalRecordError> {
    if value.is_empty() || value.len() > MAX_SHORT_TEXT {
        Err(error("engineering.record.text_bound", field))
    } else {
        Ok(())
    }
}

fn list<T>(values: &[T], field: &'static str) -> Result<(), CanonicalRecordError> {
    if values.len() > MAX_LIST {
        Err(error("engineering.record.list_too_large", field))
    } else {
        Ok(())
    }
}

fn sha_list(values: &[String], field: &'static str) -> Result<(), CanonicalRecordError> {
    list(values, field)?;
    for value in values {
        sha(value, field)?;
    }
    Ok(())
}

fn sha(value: &str, field: &'static str) -> Result<(), CanonicalRecordError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(error("engineering.record.sha256", field))
    }
}

fn timestamp(value: &str, field: &'static str) -> Result<(), CanonicalRecordError> {
    if value.len() <= 64 && value.contains('T') && (value.ends_with('Z') || value.contains('+')) {
        Ok(())
    } else {
        Err(error("engineering.record.timestamp", field))
    }
}

fn error(code: &'static str, field: &'static str) -> CanonicalRecordError {
    CanonicalRecordError { code, field }
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

/// Returns the all-zero digest used while sealing a canonical record.
#[must_use]
pub const fn zero_sha256() -> &'static str {
    ZERO_SHA256
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentmage_kernel_contracts::{
        CanonicalArtifactOrigin, CanonicalArtifactReference, CanonicalByteRange,
        CanonicalClassification, CanonicalContextItem, CanonicalEffectClass,
        CanonicalExecutionBudgets, CanonicalIngestionDisposition, CanonicalRetryClass,
        CanonicalSchemaBinding, CanonicalStateChange, CanonicalTerminalOutcome,
        CanonicalToolOutcome, CanonicalWorkflowStep,
    };

    fn digest() -> String {
        "a".repeat(64)
    }

    fn artifact() -> CanonicalArtifactEnvelope {
        CanonicalArtifactEnvelope {
            schema_version: CONTRACT_SCHEMA_VERSION,
            artifact_id: "artifact:1".to_owned(),
            request_id: "request:1".to_owned(),
            authority_id: "authority:1".to_owned(),
            origin: CanonicalArtifactOrigin::Paste,
            media_type: "text/plain".to_owned(),
            classification: CanonicalClassification::Internal,
            capture_state: CanonicalCaptureState::Captured,
            byte_length: Some(4),
            sha256: Some(digest()),
            collected_at: "2026-08-29T12:00:00Z".to_owned(),
        }
    }

    fn transformation() -> CanonicalArtifactTransformation {
        CanonicalArtifactTransformation {
            schema_version: CONTRACT_SCHEMA_VERSION,
            transformation_id: "transformation:1".to_owned(),
            artifact_id: "artifact:1".to_owned(),
            transformer_id: "parser:1".to_owned(),
            transformer_version: "1".to_owned(),
            input_sha256: digest(),
            output_sha256: Some(digest()),
            source_ranges: vec![CanonicalByteRange {
                start_byte: 0,
                end_byte_exclusive: 4,
            }],
            warnings: Vec::new(),
            reproducible: true,
        }
    }

    fn ingestion() -> CanonicalArtifactIngestionResult {
        CanonicalArtifactIngestionResult {
            schema_version: CONTRACT_SCHEMA_VERSION,
            ingestion_id: "ingestion:1".to_owned(),
            artifact_id: "artifact:1".to_owned(),
            disposition: CanonicalIngestionDisposition::Parsed,
            source: Some(CanonicalArtifactReference {
                artifact_id: "artifact:1".to_owned(),
                sha256: digest(),
                byte_length: 4,
            }),
            transformation_ids: vec!["transformation:1".to_owned()],
            warnings: Vec::new(),
            error_code: None,
            terminal: true,
        }
    }

    fn context() -> CanonicalContextManifest {
        CanonicalContextManifest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            context_manifest_id: "context:1".to_owned(),
            session_id: "session:1".to_owned(),
            turn_id: "turn:1".to_owned(),
            model_profile_id: "model:1".to_owned(),
            source_artifact_count: 1,
            items: vec![CanonicalContextItem {
                artifact_id: "artifact:1".to_owned(),
                disposition: CanonicalContextDisposition::Included,
                ranges: vec![CanonicalByteRange {
                    start_byte: 0,
                    end_byte_exclusive: 4,
                }],
                token_count: 1,
                reason_code: None,
                reason: None,
            }],
            total_input_tokens: 1,
            reserved_output_tokens: 1,
            safety_margin_tokens: 1,
            manifest_sha256: digest(),
        }
    }

    fn budgets() -> CanonicalExecutionBudgets {
        CanonicalExecutionBudgets {
            turns: 1,
            tokens: 1,
            duration_ms: 1,
            tool_calls: 1,
            attempts: 1,
            no_progress_events: 1,
            output_bytes: 1,
            memory_bytes: 1,
            cost_minor_units: 0,
        }
    }

    fn workflow_definition() -> CanonicalWorkflowDefinition {
        CanonicalWorkflowDefinition {
            schema_version: CONTRACT_SCHEMA_VERSION,
            workflow_id: "workflow:1".to_owned(),
            workflow_version: 1,
            input_schema: CanonicalSchemaBinding {
                schema_id: "schema:input".to_owned(),
                schema_version: 1,
                schema_sha256: digest(),
            },
            output_schema: CanonicalSchemaBinding {
                schema_id: "schema:output".to_owned(),
                schema_version: 1,
                schema_sha256: digest(),
            },
            steps: vec![CanonicalWorkflowStep {
                step_id: "step:1".to_owned(),
                depends_on: Vec::new(),
                model_role: None,
                tool_id: Some("tool:1".to_owned()),
                effect_class: CanonicalEffectClass::ReadOnly,
                retry_class: CanonicalRetryClass::RecoverableRead,
                verifier_ids: vec!["verifier:1".to_owned()],
                budgets: budgets(),
            }],
            entry_step_ids: vec!["step:1".to_owned()],
            definition_sha256: digest(),
        }
    }

    fn workflow_state(state: CanonicalWorkflowLifecycle, sequence: u64) -> CanonicalWorkflowState {
        let terminal = matches!(
            state,
            CanonicalWorkflowLifecycle::Succeeded
                | CanonicalWorkflowLifecycle::NoOp
                | CanonicalWorkflowLifecycle::Blocked
                | CanonicalWorkflowLifecycle::Failed
                | CanonicalWorkflowLifecycle::Cancelled
                | CanonicalWorkflowLifecycle::TimedOut
                | CanonicalWorkflowLifecycle::ResourceExhausted
                | CanonicalWorkflowLifecycle::Uncertain
        );
        CanonicalWorkflowState {
            schema_version: CONTRACT_SCHEMA_VERSION,
            workflow_id: "workflow:1".to_owned(),
            workflow_version: 1,
            sequence,
            state,
            active_step_id: None,
            completed_step_ids: if terminal {
                vec!["step:1".to_owned()]
            } else {
                Vec::new()
            },
            attempt_ids: if terminal {
                vec!["attempt:1".to_owned()]
            } else {
                Vec::new()
            },
            consumed_budget_sha256: digest(),
            terminal_result_id: terminal.then(|| "terminal:1".to_owned()),
        }
    }

    fn observation() -> CanonicalToolObservation {
        CanonicalToolObservation {
            schema_version: CONTRACT_SCHEMA_VERSION,
            observation_id: "observation:1".to_owned(),
            tool_call_id: "tool-call:1".to_owned(),
            attempt_id: "attempt:1".to_owned(),
            task_id: "task:1".to_owned(),
            step_id: "step:1".to_owned(),
            tool_id: "tool:1".to_owned(),
            tool_version: "1".to_owned(),
            tool_schema_sha256: digest(),
            arguments_sha256: digest(),
            authority_id: "authority:1".to_owned(),
            started_at: "2026-08-29T12:00:00Z".to_owned(),
            completed_at: "2026-08-29T12:00:01Z".to_owned(),
            outcome: CanonicalToolOutcome::Succeeded,
            exit_code: Some(0),
            signal: None,
            stdout: None,
            stderr: None,
            stdout_excerpt: String::new(),
            stderr_excerpt: String::new(),
            stdout_truncated: false,
            stderr_truncated: false,
            generated_artifact_ids: Vec::new(),
            state_change: CanonicalStateChange::NotChanged,
            descendants_cleaned: true,
            resource_usage_sha256: digest(),
            retry_disposition: agentmage_kernel_contracts::CanonicalRetryDisposition::NotEligible,
            receipt_sha256: digest(),
            terminal: true,
        }
    }

    fn verification() -> CanonicalVerificationResult {
        CanonicalVerificationResult {
            schema_version: CONTRACT_SCHEMA_VERSION,
            verification_result_id: "verification:1".to_owned(),
            workflow_id: "workflow:1".to_owned(),
            step_id: Some("step:1".to_owned()),
            verifier_id: "verifier:1".to_owned(),
            verifier_version: "1".to_owned(),
            subject_sha256: digest(),
            observed_evidence_sha256s: vec![digest()],
            preserved_invariants: vec!["invariant:1".to_owned()],
            prohibited_effects_observed: Vec::new(),
            outcome: CanonicalVerificationOutcome::Passed,
            current: true,
            result_sha256: digest(),
        }
    }

    fn terminal_result() -> CanonicalTerminalResult {
        CanonicalTerminalResult {
            schema_version: CONTRACT_SCHEMA_VERSION,
            terminal_result_id: "terminal:1".to_owned(),
            workflow_id: "workflow:1".to_owned(),
            outcome: CanonicalTerminalOutcome::VerifiedSuccess,
            verification_result_ids: vec!["verification:1".to_owned()],
            last_verified_state_sha256: digest(),
            diagnostic_code: None,
            safe_next_action: None,
            established_by: "agentmage-runtime-verifier".to_owned(),
            result_sha256: digest(),
        }
    }

    #[test]
    fn capture_metadata_is_present_if_and_only_if_bytes_were_captured() {
        let mut envelope = CanonicalArtifactEnvelope {
            schema_version: CONTRACT_SCHEMA_VERSION,
            artifact_id: "artifact:1".to_owned(),
            request_id: "request:1".to_owned(),
            authority_id: "authority:1".to_owned(),
            origin: CanonicalArtifactOrigin::Paste,
            media_type: "text/plain".to_owned(),
            classification: CanonicalClassification::Internal,
            capture_state: CanonicalCaptureState::Captured,
            byte_length: Some(50_128),
            sha256: Some(digest()),
            collected_at: "2026-08-23T12:00:00Z".to_owned(),
        };
        assert_eq!(envelope.validate_canonical(), Ok(()));
        envelope.capture_state = CanonicalCaptureState::Denied;
        assert_eq!(
            envelope.validate_canonical().unwrap_err().code,
            "engineering.artifact.uncaptured_metadata"
        );
    }

    #[test]
    fn success_and_delivery_records_fail_closed_on_missing_proof() {
        let delivery = CanonicalContextDeliveryReceipt {
            schema_version: CONTRACT_SCHEMA_VERSION,
            receipt_id: "receipt:1".to_owned(),
            context_manifest_id: "context:1".to_owned(),
            context_manifest_sha256: digest(),
            model_request_id: "request:1".to_owned(),
            route_decision_id: "route:1".to_owned(),
            delivered: Vec::new(),
            required_unseen_artifact_ids: vec!["artifact:1".to_owned()],
            outcome: CanonicalDeliveryOutcome::Delivered,
            receipt_sha256: digest(),
        };
        assert_eq!(
            delivery.validate_canonical().unwrap_err().code,
            "engineering.context.required_unseen"
        );

        let result = CanonicalTerminalResult {
            schema_version: CONTRACT_SCHEMA_VERSION,
            terminal_result_id: "terminal:1".to_owned(),
            workflow_id: "workflow:1".to_owned(),
            outcome: CanonicalTerminalOutcome::VerifiedSuccess,
            verification_result_ids: Vec::new(),
            last_verified_state_sha256: digest(),
            diagnostic_code: None,
            safe_next_action: None,
            established_by: "agentmage-runtime-verifier".to_owned(),
            result_sha256: digest(),
        };
        assert_eq!(
            result.validate_canonical().unwrap_err().code,
            "engineering.terminal.false_success"
        );
    }

    #[test]
    fn canonical_digest_is_stable_and_requires_current_schema() {
        let envelope = CanonicalArtifactEnvelope {
            schema_version: CONTRACT_SCHEMA_VERSION,
            artifact_id: "artifact:1".to_owned(),
            request_id: "request:1".to_owned(),
            authority_id: "authority:1".to_owned(),
            origin: CanonicalArtifactOrigin::Paste,
            media_type: "text/plain".to_owned(),
            classification: CanonicalClassification::Internal,
            capture_state: CanonicalCaptureState::Captured,
            byte_length: Some(3),
            sha256: Some(digest()),
            collected_at: "2026-08-23T12:00:00Z".to_owned(),
        };
        assert_eq!(
            canonical_record_sha256(&envelope),
            canonical_record_sha256(&envelope)
        );
        assert_eq!(
            canonical_record_sha256(&envelope).expect("digest").len(),
            64
        );
    }

    #[test]
    fn every_canonical_record_family_has_one_existing_authority_route() {
        use CanonicalRuntimeRecordKind as Kind;

        let kinds = [
            Kind::ArtifactEnvelope,
            Kind::ArtifactTransformation,
            Kind::ArtifactIngestionResult,
            Kind::ContextManifest,
            Kind::WorkflowDefinition,
            Kind::WorkflowState,
            Kind::ToolObservation,
            Kind::VerificationResult,
            Kind::TerminalResult,
        ];
        let unique = kinds.into_iter().collect::<std::collections::BTreeSet<_>>();
        assert_eq!(unique.len(), 9);

        for kind in kinds {
            let route = canonical_record_authority_route(kind);
            assert_eq!(route.kind, kind);
            assert_eq!(
                route.payload_authority,
                CanonicalRecordAuthority::RuntimeArtifactStore
            );
            assert_eq!(
                route.metadata_authority,
                CanonicalRecordAuthority::OperationalStore
            );
            assert_eq!(
                route.history_authority,
                CanonicalRecordAuthority::RuntimeEventJournal
            );
            assert_eq!(
                route.source_bytes_authority,
                (kind == Kind::ArtifactEnvelope)
                    .then_some(CanonicalRecordAuthority::SourceArtifactService)
            );
            assert_eq!(
                route.projection,
                if kind == Kind::WorkflowState {
                    CanonicalRecordProjection::RebuildableWorkflowState
                } else {
                    CanonicalRecordProjection::None
                }
            );
        }
    }

    #[test]
    fn every_canonical_record_validates_before_artifact_serialization() {
        use CanonicalRuntimeRecordKind as Kind;

        let artifact = artifact();
        let transformation = transformation();
        let ingestion = ingestion();
        let context = context();
        let definition = workflow_definition();
        let state = workflow_state(CanonicalWorkflowLifecycle::Succeeded, 5);
        let observation = observation();
        let verification = verification();
        let terminal = terminal_result();
        let records = [
            CanonicalRuntimeRecordRef::ArtifactEnvelope(&artifact),
            CanonicalRuntimeRecordRef::ArtifactTransformation(&transformation),
            CanonicalRuntimeRecordRef::ArtifactIngestionResult(&ingestion),
            CanonicalRuntimeRecordRef::ContextManifest(&context),
            CanonicalRuntimeRecordRef::WorkflowDefinition(&definition),
            CanonicalRuntimeRecordRef::WorkflowState(&state),
            CanonicalRuntimeRecordRef::ToolObservation(&observation),
            CanonicalRuntimeRecordRef::VerificationResult(&verification),
            CanonicalRuntimeRecordRef::TerminalResult(&terminal),
        ];
        let expected = [
            Kind::ArtifactEnvelope,
            Kind::ArtifactTransformation,
            Kind::ArtifactIngestionResult,
            Kind::ContextManifest,
            Kind::WorkflowDefinition,
            Kind::WorkflowState,
            Kind::ToolObservation,
            Kind::VerificationResult,
            Kind::TerminalResult,
        ];

        for (record, kind) in records.into_iter().zip(expected) {
            assert_eq!(record.kind(), kind);
            assert_eq!(record.authority_route().kind, kind);
            let payload = record.artifact_payload().expect("valid canonical record");
            let value: serde_json::Value =
                serde_json::from_slice(&payload).expect("canonical JSON");
            assert_eq!(
                value
                    .get("schema_version")
                    .and_then(serde_json::Value::as_u64),
                Some(u64::from(CONTRACT_SCHEMA_VERSION))
            );
        }

        let mut malformed_context = context.clone();
        malformed_context.source_artifact_count = 2;
        assert_eq!(
            CanonicalRuntimeRecordRef::ContextManifest(&malformed_context)
                .artifact_payload()
                .unwrap_err()
                .code,
            "engineering.context.count_invalid"
        );
    }

    #[test]
    fn runtime_record_set_binds_request_task_session_source_workflow_and_proof() {
        let artifact = artifact();
        let transformations = vec![transformation()];
        let ingestion = ingestion();
        let context = context();
        let definition = workflow_definition();
        let state = workflow_state(CanonicalWorkflowLifecycle::Succeeded, 5);
        let observations = vec![observation()];
        let verifications = vec![verification()];
        let terminal = terminal_result();
        let set = CanonicalRuntimeRecordSet {
            bindings: CanonicalRuntimeIdentityBindings {
                request_id: "request:1",
                task_id: "task:1",
                session_id: "session:1",
                source_sha256: &digest(),
                workflow_definition_sha256: &digest(),
            },
            artifact: &artifact,
            transformations: &transformations,
            ingestion: &ingestion,
            context: &context,
            workflow_definition: &definition,
            workflow_state: &state,
            observations: &observations,
            verifications: &verifications,
            terminal_result: &terminal,
        };
        assert_eq!(set.validate(), Ok(()));

        let wrong_task = CanonicalRuntimeRecordSet {
            bindings: CanonicalRuntimeIdentityBindings {
                task_id: "task:other",
                ..set.bindings
            },
            ..set
        };
        assert_eq!(
            wrong_task.validate().unwrap_err().code,
            "engineering.record.observation_binding"
        );
        let wrong_request = CanonicalRuntimeRecordSet {
            bindings: CanonicalRuntimeIdentityBindings {
                request_id: "request:other",
                ..set.bindings
            },
            ..set
        };
        assert_eq!(
            wrong_request.validate().unwrap_err().code,
            "engineering.record.binding_mismatch"
        );

        let missing_observation = CanonicalRuntimeRecordSet {
            observations: &[],
            ..set
        };
        assert_eq!(
            missing_observation.validate().unwrap_err().code,
            "engineering.record.missing_observation"
        );
    }

    #[test]
    fn identity_content_and_graph_mutations_fail_closed() {
        let mut context = context();
        context.source_artifact_count = 2;
        assert_eq!(
            context.validate_canonical().unwrap_err().code,
            "engineering.context.count_invalid"
        );

        let mut definition = workflow_definition();
        definition.steps.push(CanonicalWorkflowStep {
            step_id: "step:2".to_owned(),
            depends_on: vec!["step:1".to_owned()],
            model_role: None,
            tool_id: Some("tool:1".to_owned()),
            effect_class: CanonicalEffectClass::ReadOnly,
            retry_class: CanonicalRetryClass::RecoverableRead,
            verifier_ids: vec!["verifier:1".to_owned()],
            budgets: budgets(),
        });
        definition.steps[0].depends_on = vec!["step:2".to_owned()];
        assert_eq!(
            definition.validate_canonical().unwrap_err().code,
            "engineering.workflow.dependency_cycle"
        );

        let mut verification = verification();
        verification.observed_evidence_sha256s.push(digest());
        assert_eq!(
            verification.validate_canonical().unwrap_err().code,
            "engineering.verification.duplicate_evidence"
        );
    }

    #[test]
    fn workflow_transition_table_is_closed_and_terminal_states_are_absorbing() {
        use CanonicalWorkflowLifecycle as State;
        let states = [
            State::Created,
            State::Validating,
            State::Ready,
            State::Running,
            State::Verifying,
            State::WaitingForDependency,
            State::WaitingForApproval,
            State::Paused,
            State::Reconciling,
            State::Recovering,
            State::Succeeded,
            State::NoOp,
            State::Blocked,
            State::Failed,
            State::Cancelled,
            State::TimedOut,
            State::ResourceExhausted,
            State::Uncertain,
        ];
        for from in states {
            for to in states {
                let previous = workflow_state(from, 4);
                let next = workflow_state(to, 5);
                assert_eq!(
                    validate_workflow_transition(&previous, &next).is_ok(),
                    workflow_transition_allowed(from, to),
                    "transition {from:?} -> {to:?} diverged from the closed table",
                );
            }
        }

        let previous = workflow_state(State::Running, 4);
        let mut next = workflow_state(State::Verifying, 6);
        assert_eq!(
            validate_workflow_transition(&previous, &next)
                .unwrap_err()
                .code,
            "engineering.workflow.transition_sequence"
        );
        next.sequence = 5;
        next.workflow_id = "workflow:other".to_owned();
        assert_eq!(
            validate_workflow_transition(&previous, &next)
                .unwrap_err()
                .code,
            "engineering.workflow.transition_identity"
        );
    }
}
