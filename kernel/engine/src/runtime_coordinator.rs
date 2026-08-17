//! Reusable runtime request admission, outcome verification, and coordinator composition.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, ContractPayload, EvidenceReference, MAX_CONTRACT_JSON_BYTES,
    ModelRuntimeKind, RuntimeApprovalChallenge, RuntimeApprovalDisposition,
    RuntimeApprovalResponse, RuntimeOutcome, RuntimeOutput, RuntimeRunLimits, RuntimeRunRequest,
    RuntimeSessionMode, RuntimeToolReference, TaskStatus, WorkPacketState,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::model_runtime::{ModelAdmissionCatalog, ModelUsePurpose};
use crate::runtime_hardening::RuntimeHardeningLimits;

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_TEXT_BYTES: usize = 4_096;
const MAX_LIST_ITEMS: usize = 128;
const MAX_TURNS: u32 = 4_096;
const MAX_MODEL_CALLS: u32 = 16_384;
const MAX_TOOL_CALLS: u32 = 65_536;
const MAX_CONTEXT_REFRESHES: u32 = 16_384;
const MAX_RUNTIME_EVENTS: u32 = 1_000_000;
const MAX_ELAPSED_MS: u64 = 604_800_000;
const MAX_OUTPUT_BYTES: u64 = 32 * 1024 * 1024;

/// Stable fail-closed reason for a reusable-runtime request or outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeCoordinatorError {
    /// The contract version is unsupported.
    VersionMismatch,
    /// A required identity, digest, text, collection, or payload is malformed.
    InvalidValue,
    /// Task, packet, snapshot, profile, catalog, policy, or cursor bindings disagree.
    BindingMismatch,
    /// One declared run ceiling is zero, inconsistent, or outside the closed maximum.
    InvalidLimits,
    /// The request or outcome canonical digest does not match its bytes.
    DigestMismatch,
    /// The selected profile is malformed or unavailable for its exact declared purpose.
    ModelProfileDenied,
    /// The work packet failed its existing closed validator.
    WorkPacketDenied,
    /// An output placement or terminal-state relationship is not legal for the selected mode.
    OutcomeDenied,
    /// A protected approval challenge or response is stale, mismatched, or malformed.
    ApprovalDenied,
    /// Canonical serialization failed.
    Serialization,
}

impl RuntimeCoordinatorError {
    /// Returns a stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::VersionMismatch => "runtime.coordinator.version_mismatch",
            Self::InvalidValue => "runtime.coordinator.value_invalid",
            Self::BindingMismatch => "runtime.coordinator.binding_mismatch",
            Self::InvalidLimits => "runtime.coordinator.limits_invalid",
            Self::DigestMismatch => "runtime.coordinator.digest_mismatch",
            Self::ModelProfileDenied => "runtime.coordinator.model_profile_denied",
            Self::WorkPacketDenied => "runtime.coordinator.work_packet_denied",
            Self::OutcomeDenied => "runtime.coordinator.outcome_denied",
            Self::ApprovalDenied => "runtime.coordinator.approval_denied",
            Self::Serialization => "runtime.coordinator.serialization_failed",
        }
    }
}

/// Seals one protected approval challenge with its canonical digest.
pub fn seal_runtime_approval_challenge(
    mut challenge: RuntimeApprovalChallenge,
) -> Result<RuntimeApprovalChallenge, RuntimeCoordinatorError> {
    challenge.schema_version = CONTRACT_SCHEMA_VERSION;
    challenge.challenge_sha256 = ZERO_SHA256.to_owned();
    validate_approval_challenge_shape(&challenge)?;
    challenge.challenge_sha256 = canonical_sha256(&challenge)?;
    Ok(challenge)
}

/// Verifies one protected approval challenge's shape and canonical digest.
pub fn verify_runtime_approval_challenge(
    challenge: &RuntimeApprovalChallenge,
) -> Result<(), RuntimeCoordinatorError> {
    validate_approval_challenge_shape(challenge)?;
    let mut preimage = challenge.clone();
    preimage.challenge_sha256 = ZERO_SHA256.to_owned();
    if canonical_sha256(&preimage)? != challenge.challenge_sha256 {
        return Err(RuntimeCoordinatorError::DigestMismatch);
    }
    Ok(())
}

/// Verifies one client approval response against the exact current challenge and clock.
pub fn verify_runtime_approval_response(
    challenge: &RuntimeApprovalChallenge,
    response: &RuntimeApprovalResponse,
    now_epoch_ms: u64,
) -> Result<(), RuntimeCoordinatorError> {
    verify_runtime_approval_challenge(challenge)?;
    if response.schema_version != CONTRACT_SCHEMA_VERSION
        || response.run_id != challenge.run_id
        || response.approval_id != challenge.approval_id
        || response.challenge_sha256 != challenge.challenge_sha256
        || now_epoch_ms >= challenge.expires_at_epoch_ms
        || !valid_sha256(&response.challenge_sha256)
        || match response.disposition {
            RuntimeApprovalDisposition::Allow => {
                response.grant_id.as_ref() != Some(&challenge.proposed_grant_id)
            }
            RuntimeApprovalDisposition::Deny => response.grant_id.is_some(),
        }
    {
        return Err(RuntimeCoordinatorError::ApprovalDenied);
    }
    Ok(())
}

/// Seals one statically valid runtime request with its canonical SHA-256 digest.
pub fn seal_runtime_run_request(
    mut request: RuntimeRunRequest,
) -> Result<RuntimeRunRequest, RuntimeCoordinatorError> {
    request.schema_version = CONTRACT_SCHEMA_VERSION;
    request.request_sha256 = ZERO_SHA256.to_owned();
    validate_runtime_run_request_shape(&request)?;
    request.request_sha256 = canonical_sha256(&request)?;
    Ok(request)
}

/// Verifies one runtime request's shape, relationships, and canonical digest.
pub fn verify_runtime_run_request(
    request: &RuntimeRunRequest,
) -> Result<(), RuntimeCoordinatorError> {
    validate_runtime_run_request_shape(request)?;
    let mut preimage = request.clone();
    preimage.request_sha256 = ZERO_SHA256.to_owned();
    if canonical_sha256(&preimage)? != request.request_sha256 {
        return Err(RuntimeCoordinatorError::DigestMismatch);
    }
    Ok(())
}

/// Seals one terminal outcome after verifying it against the admitted request.
pub fn seal_runtime_outcome(
    mut outcome: RuntimeOutcome,
    request: &RuntimeRunRequest,
) -> Result<RuntimeOutcome, RuntimeCoordinatorError> {
    verify_runtime_run_request(request)?;
    outcome.schema_version = CONTRACT_SCHEMA_VERSION;
    outcome.outcome_sha256 = ZERO_SHA256.to_owned();
    validate_runtime_outcome_shape(&outcome, request)?;
    outcome.outcome_sha256 = canonical_sha256(&outcome)?;
    Ok(outcome)
}

/// Verifies one terminal outcome against the exact admitted request and canonical digest.
pub fn verify_runtime_outcome(
    outcome: &RuntimeOutcome,
    request: &RuntimeRunRequest,
) -> Result<(), RuntimeCoordinatorError> {
    verify_runtime_run_request(request)?;
    validate_runtime_outcome_shape(outcome, request)?;
    let mut preimage = outcome.clone();
    preimage.outcome_sha256 = ZERO_SHA256.to_owned();
    if canonical_sha256(&preimage)? != outcome.outcome_sha256 {
        return Err(RuntimeCoordinatorError::DigestMismatch);
    }
    Ok(())
}

/// Computes the canonical digest for an exact visible tool catalog.
pub fn runtime_tool_catalog_sha256(
    tool_catalog_id: &agentmage_kernel_contracts::ToolCatalogId,
    visible_tools: &[RuntimeToolReference],
) -> Result<String, RuntimeCoordinatorError> {
    #[derive(Serialize)]
    struct CatalogPreimage<'a> {
        tool_catalog_id: &'a agentmage_kernel_contracts::ToolCatalogId,
        visible_tools: &'a [RuntimeToolReference],
    }
    canonical_sha256(&CatalogPreimage {
        tool_catalog_id,
        visible_tools,
    })
}

fn validate_runtime_run_request_shape(
    request: &RuntimeRunRequest,
) -> Result<(), RuntimeCoordinatorError> {
    if request.schema_version != CONTRACT_SCHEMA_VERSION {
        return Err(RuntimeCoordinatorError::VersionMismatch);
    }
    if !valid_identifier(request.run_id.as_str())
        || !valid_identifier(request.session_id.as_str())
        || !valid_identifier(request.workspace_id.as_str())
        || !valid_identifier(request.repository_snapshot_id.as_str())
        || !valid_identifier(request.tool_catalog_id.as_str())
        || !valid_identifier(request.policy_id.as_str())
        || !valid_sha256(&request.workspace_snapshot_sha256)
        || !valid_sha256(&request.repository_snapshot_sha256)
        || !valid_sha256(&request.tool_catalog_sha256)
        || !valid_sha256(&request.policy_sha256)
        || !valid_sha256(&request.request_sha256)
        || !valid_task(request)
        || !valid_visible_tools(&request.visible_tools)
    {
        return Err(RuntimeCoordinatorError::InvalidValue);
    }
    if request.task.session_id != request.session_id
        || request.work_packet.task_id != request.task.task_id
        || request
            .event_cursor
            .as_ref()
            .is_some_and(|cursor| cursor.run_id != request.run_id)
        || (request.mode == RuntimeSessionMode::EphemeralReadOnly && request.event_cursor.is_some())
    {
        return Err(RuntimeCoordinatorError::BindingMismatch);
    }
    if !crate::work_packet::validate_packet(&request.work_packet).is_empty()
        || !matches!(
            request.work_packet.state,
            WorkPacketState::Validated | WorkPacketState::Planned | WorkPacketState::Active
        )
    {
        return Err(RuntimeCoordinatorError::WorkPacketDenied);
    }
    validate_profile_and_context(request)?;
    validate_limits(request)?;
    RuntimeHardeningLimits::from_request(request)
        .map_err(|_| RuntimeCoordinatorError::InvalidLimits)?;
    if request.tool_catalog_sha256
        != runtime_tool_catalog_sha256(&request.tool_catalog_id, &request.visible_tools)?
    {
        return Err(RuntimeCoordinatorError::BindingMismatch);
    }
    if let Some(cursor) = &request.event_cursor
        && (!valid_identifier(cursor.event_id.as_str()) || !valid_sha256(&cursor.event_sha256))
    {
        return Err(RuntimeCoordinatorError::InvalidValue);
    }
    Ok(())
}

fn validate_approval_challenge_shape(
    challenge: &RuntimeApprovalChallenge,
) -> Result<(), RuntimeCoordinatorError> {
    if challenge.schema_version != CONTRACT_SCHEMA_VERSION {
        return Err(RuntimeCoordinatorError::VersionMismatch);
    }
    if !valid_identifier(challenge.run_id.as_str())
        || !valid_identifier(challenge.task_id.as_str())
        || !valid_identifier(challenge.turn_id.as_str())
        || !valid_identifier(challenge.operation_id.as_str())
        || !valid_identifier(challenge.tool_call_id.as_str())
        || !valid_identifier(challenge.approval_id.as_str())
        || !valid_identifier(challenge.proposed_grant_id.as_str())
        || !valid_sha256(&challenge.preview_sha256)
        || challenge.expires_at_epoch_ms == 0
        || !valid_sha256(&challenge.challenge_sha256)
    {
        return Err(RuntimeCoordinatorError::ApprovalDenied);
    }
    Ok(())
}

fn validate_profile_and_context(
    request: &RuntimeRunRequest,
) -> Result<(), RuntimeCoordinatorError> {
    let purpose = if request.model_profile.runtime.kind == ModelRuntimeKind::DeterministicFake {
        ModelUsePurpose::ContractTest
    } else {
        ModelUsePurpose::Product
    };
    ModelAdmissionCatalog::new(vec![request.model_profile.clone()])
        .and_then(|catalog| catalog.admit(&request.model_profile, purpose))
        .map_err(|_| RuntimeCoordinatorError::ModelProfileDenied)?;
    let selected = &request.model_profile.context;
    let requested = &request.context_budget;
    if requested.max_context_tokens == 0
        || requested.max_context_tokens > selected.max_context_tokens
        || requested.max_input_bytes == 0
        || requested.max_input_bytes > selected.max_input_bytes
        || requested.max_messages == 0
        || requested.max_messages > selected.max_messages
        || requested.token_counter != selected.token_counter
        || requested.token_counter_sha256 != selected.token_counter_sha256
    {
        return Err(RuntimeCoordinatorError::BindingMismatch);
    }
    Ok(())
}

fn validate_limits(request: &RuntimeRunRequest) -> Result<(), RuntimeCoordinatorError> {
    validate_runtime_run_limits(&request.limits)
}

/// Validates reusable-runtime ceilings independently before a complete run is framed.
pub fn validate_runtime_run_limits(
    limits: &RuntimeRunLimits,
) -> Result<(), RuntimeCoordinatorError> {
    let model_call_ceiling = limits
        .max_turns
        .checked_mul(4)
        .ok_or(RuntimeCoordinatorError::InvalidLimits)?;
    if limits.max_turns == 0
        || limits.max_turns > MAX_TURNS
        || limits.max_model_calls == 0
        || limits.max_model_calls > MAX_MODEL_CALLS
        || limits.max_model_calls > model_call_ceiling
        || limits.max_tool_calls > MAX_TOOL_CALLS
        || !(1..=16).contains(&limits.max_repeated_tool_calls)
        || limits.max_tool_call_depth > 32
        || limits.max_no_progress_turns == 0
        || limits.max_no_progress_turns > limits.max_turns
        || limits.max_context_refreshes == 0
        || limits.max_context_refreshes > MAX_CONTEXT_REFRESHES
        || limits.max_events == 0
        || limits.max_events > MAX_RUNTIME_EVENTS
        || limits.max_elapsed_ms == 0
        || limits.max_elapsed_ms > MAX_ELAPSED_MS
        || limits.max_output_bytes == 0
        || limits.max_output_bytes > MAX_OUTPUT_BYTES
    {
        return Err(RuntimeCoordinatorError::InvalidLimits);
    }
    Ok(())
}

fn validate_runtime_outcome_shape(
    outcome: &RuntimeOutcome,
    request: &RuntimeRunRequest,
) -> Result<(), RuntimeCoordinatorError> {
    if outcome.schema_version != CONTRACT_SCHEMA_VERSION {
        return Err(RuntimeCoordinatorError::VersionMismatch);
    }
    if outcome.run_id != request.run_id
        || outcome.session_id != request.session_id
        || outcome.task_id != request.task.task_id
        || outcome.request_sha256 != request.request_sha256
    {
        return Err(RuntimeCoordinatorError::BindingMismatch);
    }
    if !outcome.state.is_terminal()
        || outcome.turn_count > request.limits.max_turns
        || outcome.model_call_count > request.limits.max_model_calls
        || outcome.tool_call_count > request.limits.max_tool_calls
        || !valid_identifier(outcome.prior_event_id.as_str())
        || !valid_sha256(&outcome.prior_event_sha256)
        || !valid_sha256(&outcome.outcome_sha256)
        || !valid_evidence(&outcome.evidence)
        || !unique_identifiers(outcome.receipt_ids.iter().map(|value| value.as_str()))
        || !valid_codes(&outcome.unresolved_codes)
        || !valid_output(outcome.output.as_ref(), request)?
        || (outcome.state.is_success() && outcome.evidence.is_empty())
    {
        return Err(RuntimeCoordinatorError::OutcomeDenied);
    }
    Ok(())
}

fn valid_task(request: &RuntimeRunRequest) -> bool {
    let task = &request.task;
    task.schema_version == CONTRACT_SCHEMA_VERSION
        && valid_identifier(task.task_id.as_str())
        && valid_identifier(task.session_id.as_str())
        && matches!(task.status, TaskStatus::Ready | TaskStatus::Running)
        && valid_text(&task.objective)
        && valid_texts(&task.acceptance_criteria, true)
        && valid_texts(&task.constraints, false)
}

fn valid_visible_tools(tools: &[RuntimeToolReference]) -> bool {
    if tools.len() > MAX_LIST_ITEMS {
        return false;
    }
    let mut prior: Option<(&str, &str)> = None;
    let mut identities = BTreeSet::new();
    for tool in tools {
        let key = (tool.tool_id.as_str(), tool.tool_version.as_str());
        if !valid_identifier(key.0)
            || !valid_identifier(key.1)
            || !valid_sha256(&tool.definition_sha256)
            || prior.is_some_and(|value| value >= key)
            || !identities.insert((key.0.to_owned(), key.1.to_owned()))
        {
            return false;
        }
        prior = Some(key);
    }
    true
}

fn valid_evidence(evidence: &[EvidenceReference]) -> bool {
    if evidence.len() > 4_096 {
        return false;
    }
    let mut identities = BTreeSet::new();
    evidence.iter().all(|item| {
        item.schema_version == CONTRACT_SCHEMA_VERSION
            && valid_identifier(item.evidence_id.as_str())
            && valid_text(&item.source_id)
            && valid_text(&item.object_id)
            && item.fragment.as_ref().is_none_or(|value| valid_text(value))
            && valid_sha256(&item.content_sha256)
            && item
                .observed_revision
                .as_ref()
                .is_none_or(|value| valid_text(value))
            && identities.insert(item.evidence_id.as_str())
    })
}

fn valid_output(
    output: Option<&RuntimeOutput>,
    request: &RuntimeRunRequest,
) -> Result<bool, RuntimeCoordinatorError> {
    Ok(match output {
        None => true,
        Some(RuntimeOutput::Inline { payload }) => valid_payload(payload, request),
        Some(RuntimeOutput::Artifact { reference }) => {
            request.mode != RuntimeSessionMode::EphemeralReadOnly
                && valid_identifier(reference.artifact_id.as_str())
                && valid_sha256(&reference.sha256)
                && reference.byte_size > 0
                && reference.byte_size <= request.limits.max_output_bytes
                && valid_media_type(&reference.media_type)
        }
    })
}

fn valid_payload(payload: &ContractPayload, request: &RuntimeRunRequest) -> bool {
    valid_identifier(payload.schema.schema_id.as_str())
        && payload.schema.schema_version > 0
        && valid_sha256(&payload.schema.schema_sha256)
        && valid_media_type(&payload.media_type)
        && u64::try_from(payload.bytes.len())
            .is_ok_and(|bytes| bytes > 0 && bytes <= request.limits.max_output_bytes)
        && payload.sha256 == sha256(&payload.bytes)
}

fn valid_codes(values: &[String]) -> bool {
    values.len() <= MAX_LIST_ITEMS
        && values.windows(2).all(|pair| pair[0] < pair[1])
        && values.iter().all(|value| {
            !value.is_empty()
                && value.len() <= MAX_IDENTIFIER_BYTES
                && value.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'.' | b'_' | b'-')
                })
        })
}

fn valid_texts(values: &[String], required: bool) -> bool {
    (!required || !values.is_empty())
        && values.len() <= MAX_LIST_ITEMS
        && values.iter().all(|value| valid_text(value))
}

fn valid_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_TEXT_BYTES
}

fn valid_media_type(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'+' | b'.' | b'-'))
}

fn unique_identifiers<'a>(values: impl Iterator<Item = &'a str>) -> bool {
    let mut seen = BTreeSet::new();
    let mut count = 0_usize;
    for value in values {
        count += 1;
        if count > 4_096 || !valid_identifier(value) || !seen.insert(value) {
            return false;
        }
    }
    true
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_IDENTIFIER_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn canonical_sha256(value: &impl Serialize) -> Result<String, RuntimeCoordinatorError> {
    let bytes = serde_json::to_vec(value).map_err(|_| RuntimeCoordinatorError::Serialization)?;
    if bytes.len() > MAX_CONTRACT_JSON_BYTES {
        return Err(RuntimeCoordinatorError::Serialization);
    }
    Ok(sha256(&bytes))
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        RuntimeCoordinatorError, runtime_tool_catalog_sha256, seal_runtime_approval_challenge,
        seal_runtime_outcome, seal_runtime_run_request, sha256, verify_runtime_approval_response,
        verify_runtime_outcome, verify_runtime_run_request,
    };
    use agentmage_kernel_contracts::{
        AgentStateKind, ApprovalId, AuthorityClass, BudgetLimit, BudgetResource,
        CONTRACT_SCHEMA_VERSION, ContractPayload, DataSensitivity, EvidenceId, EvidenceKind,
        EvidenceReference, GrantId, GrantOperation, PlanId, ReceiptId, RepositorySnapshotId,
        RollbackPlan, RuntimeApprovalChallenge, RuntimeApprovalDisposition,
        RuntimeApprovalResponse, RuntimeEventCursor, RuntimeEventId, RuntimeOperationId,
        RuntimeOutcome, RuntimeOutput, RuntimeRunId, RuntimeRunLimits, RuntimeRunRequest,
        RuntimeSessionMode, RuntimeTurnId, SchemaId, SchemaReference, SessionId, StopCondition,
        StopConditionKind, Task, TaskId, TaskStatus, ToolCallId, ToolCatalogId, WorkPacket,
        WorkPacketId, WorkPacketState, WorkspaceId, from_json, to_canonical_json,
    };

    const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn stop_conditions() -> Vec<StopCondition> {
        [
            StopConditionKind::AcceptanceSatisfied,
            StopConditionKind::UserDecisionRequired,
            StopConditionKind::PolicyDenied,
            StopConditionKind::Error,
            StopConditionKind::Cancelled,
            StopConditionKind::BudgetExhausted,
            StopConditionKind::UncertainResult,
        ]
        .into_iter()
        .map(|kind| StopCondition {
            kind,
            description: format!("Stop for {kind:?}"),
        })
        .collect()
    }

    fn packet() -> WorkPacket {
        WorkPacket {
            schema_version: CONTRACT_SCHEMA_VERSION,
            work_packet_id: WorkPacketId::from_raw("packet-0001"),
            task_id: TaskId::from_raw("task-0001"),
            revision: 1,
            objective: "Inspect one synthetic fixture".to_owned(),
            reason: "Verify the reusable runtime".to_owned(),
            owner: "fixture-user".to_owned(),
            authoritative_evidence: Vec::new(),
            mutable_files: Vec::new(),
            protected_files: vec!["fixtures/input.txt".to_owned()],
            expected_output: "A grounded bounded answer".to_owned(),
            acceptance_checks: vec!["Cite the observed fixture".to_owned()],
            required_evidence: vec![EvidenceKind::Observation],
            required_capability_class: AuthorityClass::Observe,
            budgets: vec![
                BudgetLimit {
                    resource: BudgetResource::PlanSteps,
                    limit: 4,
                },
                BudgetLimit {
                    resource: BudgetResource::ModelCalls,
                    limit: 4,
                },
                BudgetLimit {
                    resource: BudgetResource::ToolCalls,
                    limit: 4,
                },
            ],
            stop_conditions: stop_conditions(),
            rollback: RollbackPlan {
                reversible: true,
                description: "No state change is permitted".to_owned(),
            },
            sensitivity: DataSensitivity::Ephemeral,
            last_verification_date: "2026-08-16".to_owned(),
            next_action: None,
            next_review: None,
            status_reason: None,
            disposition: None,
            completion_evidence: Vec::new(),
            superseding_work: None,
            validation_issues: Vec::new(),
            plan_id: Some(PlanId::from_raw("plan-0001")),
            state: WorkPacketState::Active,
        }
    }

    fn request() -> RuntimeRunRequest {
        let model_profile = crate::model_codec::tests_support::profile("runtime");
        let tool_catalog_id = ToolCatalogId::from_raw("catalog-0001");
        let visible_tools = Vec::new();
        RuntimeRunRequest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            run_id: RuntimeRunId::from_raw("runtime-run-0001"),
            session_id: SessionId::from_raw("session-0001"),
            mode: RuntimeSessionMode::EphemeralReadOnly,
            task: Task {
                schema_version: CONTRACT_SCHEMA_VERSION,
                task_id: TaskId::from_raw("task-0001"),
                session_id: SessionId::from_raw("session-0001"),
                objective: "Inspect one synthetic fixture".to_owned(),
                acceptance_criteria: vec!["Cite the observed fixture".to_owned()],
                constraints: vec!["Read only".to_owned()],
                status: TaskStatus::Ready,
            },
            work_packet: packet(),
            workspace_id: WorkspaceId::from_raw("workspace-0001"),
            workspace_snapshot_sha256: SHA.to_owned(),
            repository_snapshot_id: RepositorySnapshotId::from_raw("snapshot-0001"),
            repository_snapshot_sha256: "b".repeat(64),
            context_budget: model_profile.context.clone(),
            model_profile,
            tool_catalog_sha256: runtime_tool_catalog_sha256(&tool_catalog_id, &visible_tools)
                .expect("catalog digest"),
            tool_catalog_id,
            visible_tools,
            policy_id: agentmage_kernel_contracts::PolicyId::from_raw("policy-0001"),
            policy_sha256: "c".repeat(64),
            limits: RuntimeRunLimits {
                max_turns: 4,
                max_model_calls: 8,
                max_tool_calls: 4,
                max_repeated_tool_calls: 2,
                max_tool_call_depth: 1,
                max_no_progress_turns: 2,
                max_context_refreshes: 4,
                max_events: 64,
                max_elapsed_ms: 10_000,
                max_output_bytes: 4_096,
            },
            event_cursor: None,
            request_sha256: "0".repeat(64),
        }
    }

    fn evidence() -> EvidenceReference {
        EvidenceReference {
            schema_version: CONTRACT_SCHEMA_VERSION,
            evidence_id: EvidenceId::from_raw("evidence-0001"),
            kind: EvidenceKind::Observation,
            source_id: "fixture".to_owned(),
            object_id: "fixture/input.txt".to_owned(),
            fragment: Some("line:1".to_owned()),
            content_sha256: SHA.to_owned(),
            observed_revision: Some("snapshot-0001".to_owned()),
        }
    }

    fn output() -> RuntimeOutput {
        let bytes = b"Grounded fixture answer".to_vec();
        RuntimeOutput::Inline {
            payload: ContractPayload {
                schema: SchemaReference {
                    schema_id: SchemaId::from_raw("runtime.answer"),
                    schema_version: 1,
                    schema_sha256: SHA.to_owned(),
                },
                media_type: "text/plain".to_owned(),
                sha256: sha256(&bytes),
                bytes,
            },
        }
    }

    fn outcome(request: &RuntimeRunRequest) -> RuntimeOutcome {
        RuntimeOutcome {
            schema_version: CONTRACT_SCHEMA_VERSION,
            run_id: request.run_id.clone(),
            session_id: request.session_id.clone(),
            task_id: request.task.task_id.clone(),
            request_sha256: request.request_sha256.clone(),
            state: AgentStateKind::Success,
            turn_count: 2,
            model_call_count: 2,
            tool_call_count: 1,
            prior_event_id: RuntimeEventId::from_raw("event-0012"),
            prior_event_sha256: "d".repeat(64),
            evidence: vec![evidence()],
            receipt_ids: vec![ReceiptId::from_raw("receipt-0001")],
            unresolved_codes: Vec::new(),
            output: Some(output()),
            outcome_sha256: "0".repeat(64),
        }
    }

    #[test]
    fn story_23_4_request_and_outcome_are_closed_hash_bound_contracts() {
        let request = seal_runtime_run_request(request()).expect("request seals");
        verify_runtime_run_request(&request).expect("request verifies");
        let bytes = to_canonical_json(&request).expect("request serializes");
        assert_eq!(from_json::<RuntimeRunRequest>(&bytes), Ok(request.clone()));

        let outcome = seal_runtime_outcome(outcome(&request), &request).expect("outcome seals");
        verify_runtime_outcome(&outcome, &request).expect("outcome verifies");
        let bytes = to_canonical_json(&outcome).expect("outcome serializes");
        assert_eq!(from_json::<RuntimeOutcome>(&bytes), Ok(outcome));
    }

    #[test]
    fn story_23_4_approval_response_must_echo_the_challenged_grant_identity() {
        let challenge = seal_runtime_approval_challenge(RuntimeApprovalChallenge {
            schema_version: CONTRACT_SCHEMA_VERSION,
            run_id: RuntimeRunId::from_raw("runtime-run-approval"),
            task_id: TaskId::from_raw("task-approval"),
            turn_id: RuntimeTurnId::from_raw("turn-approval"),
            operation_id: RuntimeOperationId::from_raw("operation-approval"),
            tool_call_id: ToolCallId::from_raw("tool-call-approval"),
            approval_id: ApprovalId::from_raw("approval-runtime"),
            proposed_grant_id: GrantId::from_raw("grant-runtime"),
            operation: GrantOperation::WorkspaceRead,
            preview_sha256: "d".repeat(64),
            expires_at_epoch_ms: 10_000,
            challenge_sha256: "0".repeat(64),
        })
        .expect("challenge seals");
        let response = RuntimeApprovalResponse {
            schema_version: CONTRACT_SCHEMA_VERSION,
            run_id: challenge.run_id.clone(),
            approval_id: challenge.approval_id.clone(),
            disposition: RuntimeApprovalDisposition::Allow,
            challenge_sha256: challenge.challenge_sha256.clone(),
            grant_id: Some(challenge.proposed_grant_id.clone()),
        };
        verify_runtime_approval_response(&challenge, &response, 1_000)
            .expect("exact grant response verifies");

        let mut substituted = response;
        substituted.grant_id = Some(GrantId::from_raw("grant-substituted"));
        assert_eq!(
            verify_runtime_approval_response(&challenge, &substituted, 1_000),
            Err(RuntimeCoordinatorError::ApprovalDenied)
        );
    }

    #[test]
    fn story_23_4_request_rejects_binding_catalog_context_cursor_and_limit_drift() {
        let mut cases = Vec::new();

        let mut changed = request();
        changed.task.session_id = SessionId::from_raw("session-other");
        cases.push((changed, RuntimeCoordinatorError::BindingMismatch));

        let mut changed = request();
        changed.tool_catalog_sha256 = "f".repeat(64);
        cases.push((changed, RuntimeCoordinatorError::BindingMismatch));

        let mut changed = request();
        changed.context_budget.max_context_tokens += 1;
        cases.push((changed, RuntimeCoordinatorError::BindingMismatch));

        let mut changed = request();
        changed.event_cursor = Some(RuntimeEventCursor {
            run_id: changed.run_id.clone(),
            event_id: RuntimeEventId::from_raw("event-prior"),
            sequence: 3,
            event_sha256: SHA.to_owned(),
        });
        cases.push((changed, RuntimeCoordinatorError::BindingMismatch));

        let mut changed = request();
        changed.limits.max_no_progress_turns = 0;
        cases.push((changed, RuntimeCoordinatorError::InvalidLimits));

        for (candidate, expected) in cases {
            assert_eq!(seal_runtime_run_request(candidate), Err(expected));
        }
    }

    #[test]
    fn story_23_4_request_and_outcome_tampering_fail_closed() {
        let request = seal_runtime_run_request(request()).expect("request seals");
        let mut changed_request = request.clone();
        changed_request.repository_snapshot_sha256 = "e".repeat(64);
        assert_eq!(
            verify_runtime_run_request(&changed_request),
            Err(RuntimeCoordinatorError::DigestMismatch)
        );

        let sealed_outcome =
            seal_runtime_outcome(outcome(&request), &request).expect("outcome seals");
        let mut changed_outcome = sealed_outcome.clone();
        changed_outcome.turn_count += 1;
        assert_eq!(
            verify_runtime_outcome(&changed_outcome, &request),
            Err(RuntimeCoordinatorError::DigestMismatch)
        );

        let mut false_success = outcome(&request);
        false_success.evidence.clear();
        assert_eq!(
            seal_runtime_outcome(false_success, &request),
            Err(RuntimeCoordinatorError::OutcomeDenied)
        );

        let mut oversized = outcome(&request);
        let RuntimeOutput::Inline { payload } = oversized.output.as_mut().expect("output") else {
            panic!("inline fixture")
        };
        payload.bytes = vec![b'x'; 4_097];
        payload.sha256 = sha256(&payload.bytes);
        assert_eq!(
            seal_runtime_outcome(oversized, &request),
            Err(RuntimeCoordinatorError::OutcomeDenied)
        );
    }
}
