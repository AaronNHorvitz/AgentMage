//! Deterministic semantic validation for canonical Engineering Runtime records.

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, CanonicalArtifactEnvelope, CanonicalArtifactIngestionResult,
    CanonicalArtifactTransformation, CanonicalCapabilityManifest, CanonicalCaptureState,
    CanonicalContextDeliveryReceipt, CanonicalContextManifest, CanonicalDeliveryOutcome,
    CanonicalModelEndpointProfile, CanonicalModelRouteDecision, CanonicalTerminalOutcome,
    CanonicalTerminalResult, CanonicalToolObservation, CanonicalVerificationOutcome,
    CanonicalVerificationResult, CanonicalWorkflowCheckpoint, CanonicalWorkflowDefinition,
    CanonicalWorkflowLifecycle, CanonicalWorkflowState, VersionedContract, to_canonical_json,
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
        list(&self.transformation_ids, "transformation_ids")?;
        list(&self.warnings, "warnings")?;
        if !self.terminal {
            return Err(error("engineering.ingestion.not_terminal", "terminal"));
        }
        if let Some(source) = &self.source {
            identifier(&source.artifact_id, "source.artifact_id")?;
            sha(&source.sha256, "source.sha256")?;
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
        list(&self.warnings, "warnings")
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
        if self.items.len() > 4096 || self.source_artifact_count < self.items.len() as u64 {
            return Err(error(
                "engineering.context.count_invalid",
                "source_artifact_count",
            ));
        }
        for item in &self.items {
            identifier(&item.artifact_id, "items.artifact_id")?;
            list(&item.ranges, "items.ranges")?;
            if let Some(reason) = &item.reason {
                short(reason, "items.reason")?;
            }
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
            list(&step.depends_on, "steps.depends_on")?;
            if step.verifier_ids.is_empty() || step.verifier_ids.len() > 32 {
                return Err(error(
                    "engineering.workflow.verifier_required",
                    "steps.verifier_ids",
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
        list(&self.completed_step_ids, "completed_step_ids")?;
        list(&self.attempt_ids, "attempt_ids")?;
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
        sha(&self.subject_sha256, "subject_sha256")?;
        if self.observed_evidence_sha256s.is_empty() {
            return Err(error(
                "engineering.verification.evidence_required",
                "observed_evidence_sha256s",
            ));
        }
        sha_list(&self.observed_evidence_sha256s, "observed_evidence_sha256s")?;
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
        sha(
            &self.last_verified_state_sha256,
            "last_verified_state_sha256",
        )?;
        sha(&self.result_sha256, "result_sha256")
    }
}

fn record_version<T: VersionedContract>(record: &T) -> Result<(), CanonicalRecordError> {
    if record.schema_version() == CONTRACT_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(error("engineering.record.version", "schema_version"))
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
    use agentmage_kernel_contracts::{CanonicalArtifactOrigin, CanonicalClassification};

    fn digest() -> String {
        "a".repeat(64)
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
}
