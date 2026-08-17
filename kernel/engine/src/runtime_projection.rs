//! Optional bounded transcript, metric, and diagnostic projections.

use std::fmt::Write as _;

use agentmage_kernel_contracts::{
    AgentStateKind, CONTRACT_SCHEMA_VERSION, ContextSensitivity, ConversationAttachmentReference,
    ConversationId, ConversationTurn, ConversationTurnId, ConversationTurnRole, DataSensitivity,
    PolicyId, RuntimeEvent, RuntimeEventId, RuntimeEventKind, RuntimeEventPersistenceClass,
    RuntimeOutcome, RuntimeOutput, RuntimeRunId, RuntimeRunRequest, SessionId, TaskId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    conversation_library::verify_conversation_turn,
    runtime_coordinator::{verify_runtime_outcome, verify_runtime_run_request},
    runtime_event::verify_runtime_event,
};

const PROJECTION_SCHEMA_VERSION: u16 = 1;
const MAX_TRANSCRIPT_BYTES: u64 = 2 * 1024 * 1024;
const MAX_METRIC_RECORDS: usize = 4_096;
const MAX_DIAGNOSTIC_RECORDS: usize = 4_096;
const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// Stable failure from optional projection construction or bounded collection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeProjectionError {
    /// Projection policy is malformed or exceeds hard limits.
    InvalidPolicy,
    /// The source request, outcome, or event failed canonical verification.
    InvalidSource,
    /// Session, task, run, or policy bindings differ.
    BindingMismatch,
    /// The requested optional projection is disabled.
    ProjectionDisabled,
    /// Safe mode prohibits transcript or metric collection.
    SafeModeDenied,
    /// Restricted content cannot enter an optional projection.
    Restricted,
    /// A declared cumulative collection ceiling was reached.
    ResourceLimit,
    /// Deterministic projection encoding failed.
    Serialization,
}

impl RuntimeProjectionError {
    /// Returns one stable content-free failure code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidPolicy => "runtime_projection.policy.invalid",
            Self::InvalidSource => "runtime_projection.source.invalid",
            Self::BindingMismatch => "runtime_projection.binding.mismatch",
            Self::ProjectionDisabled => "runtime_projection.disabled",
            Self::SafeModeDenied => "runtime_projection.safe_mode.denied",
            Self::Restricted => "runtime_projection.restricted",
            Self::ResourceLimit => "runtime_projection.resource_limit",
            Self::Serialization => "runtime_projection.serialization.failed",
        }
    }
}

/// Exact local-only policy for optional runtime projections.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeProjectionPolicy {
    /// Owning runtime run.
    pub run_id: RuntimeRunId,
    /// Owning local session.
    pub session_id: SessionId,
    /// Owning task.
    pub task_id: TaskId,
    /// Exact deterministic policy identity.
    pub policy_id: PolicyId,
    /// Digest of the exact policy revision.
    pub policy_sha256: String,
    /// Whether exact user-visible transcript turns may be projected.
    pub transcript_enabled: bool,
    /// Whether content-free integer metrics may be projected.
    pub metrics_enabled: bool,
    /// Whether content-free local diagnostics may be projected.
    pub diagnostics_enabled: bool,
    /// Whether recovery-only safe mode is active.
    pub safe_mode: bool,
    /// Cumulative original bytes admitted to transcript projection.
    pub max_transcript_bytes: u64,
    /// Maximum content-free metric records.
    pub max_metric_records: usize,
    /// Maximum content-free diagnostic records.
    pub max_diagnostic_records: usize,
}

/// Caller-supplied conversation placement for one verified runtime exchange.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeTranscriptPlacement {
    /// Existing canonical conversation identity.
    pub conversation_id: ConversationId,
    /// One-based ordinal assigned to the user turn.
    pub first_ordinal: u64,
    /// Trusted user-turn time.
    pub user_created_at_epoch_ms: u64,
    /// Trusted assistant-turn time, no earlier than the user turn.
    pub assistant_created_at_epoch_ms: u64,
    /// Deterministic local date shared by this exchange.
    pub local_date: String,
    /// Turn-level persistence sensitivity selected before projection.
    pub sensitivity: DataSensitivity,
    /// Whether exact text may be retained in the encrypted conversation store.
    pub retain_text: bool,
}

/// Two immutable conversation turns derived from one verified runtime exchange.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeTranscriptProjection {
    /// Projection schema version.
    pub schema_version: u16,
    /// Exact sealed runtime request digest.
    pub request_sha256: String,
    /// Exact sealed runtime outcome digest.
    pub outcome_sha256: String,
    /// User-authored objective turn.
    pub user_turn: ConversationTurn,
    /// Assistant terminal-output turn.
    pub assistant_turn: ConversationTurn,
    /// Original source bytes charged to the cumulative projection ceiling.
    pub source_bytes: u64,
    /// Fixed false marker: transcript prose is never correctness authority.
    pub carries_authority: bool,
    /// Digest of this projection with this field set to zeroes.
    pub projection_sha256: String,
}

/// Content-free metric derived only from a verified `metric` event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeMetricProjection {
    /// Projection schema version.
    pub schema_version: u16,
    /// Exact source run.
    pub run_id: RuntimeRunId,
    /// Exact source event.
    pub event_id: RuntimeEventId,
    /// Exact source event digest.
    pub event_sha256: String,
    /// Approved content-free metric name.
    pub name: String,
    /// Integer metric value in the metric contract's documented unit.
    pub value: i64,
    /// Trusted source-event time.
    pub occurred_at_epoch_ms: u64,
    /// Fixed false marker: metrics are not completion or operation authority.
    pub carries_authority: bool,
    /// Digest of this projection with this field set to zeroes.
    pub projection_sha256: String,
}

/// Closed source class for one content-free runtime diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeDiagnosticKind {
    /// Local model boundary failure.
    ModelFailure,
    /// Tool or effect boundary failure.
    ToolFailure,
    /// Canonical terminal runtime state.
    Terminal,
}

/// Content-free diagnostic derived only from a verified canonical event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeDiagnosticProjection {
    /// Projection schema version.
    pub schema_version: u16,
    /// Exact source run.
    pub run_id: RuntimeRunId,
    /// Exact source event.
    pub event_id: RuntimeEventId,
    /// Exact source event digest.
    pub event_sha256: String,
    /// Closed diagnostic source class.
    pub kind: RuntimeDiagnosticKind,
    /// Stable content-free status or failure code.
    pub reason_code: String,
    /// Trusted source-event time.
    pub occurred_at_epoch_ms: u64,
    /// Fixed false marker: diagnostics are not completion or operation authority.
    pub carries_authority: bool,
    /// Digest of this projection with this field set to zeroes.
    pub projection_sha256: String,
}

/// Counts returned by one nonblocking in-memory projection collection step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RuntimeProjectionCollection {
    /// Transcript exchanges added.
    pub transcripts_added: usize,
    /// Metric records added.
    pub metrics_added: usize,
    /// Diagnostic records added.
    pub diagnostics_added: usize,
}

/// Bounded local collector that owns no persistence, export, or effect authority.
#[derive(Debug)]
pub struct RuntimeProjectionCollector {
    policy: RuntimeProjectionPolicy,
    transcript_bytes: u64,
    transcripts: Vec<RuntimeTranscriptProjection>,
    metrics: Vec<RuntimeMetricProjection>,
    diagnostics: Vec<RuntimeDiagnosticProjection>,
}

impl RuntimeProjectionCollector {
    /// Binds one validated optional projection policy to one exact verified request.
    pub fn new(
        policy: RuntimeProjectionPolicy,
        request: &RuntimeRunRequest,
    ) -> Result<Self, RuntimeProjectionError> {
        validate_policy(&policy)?;
        verify_runtime_run_request(request).map_err(|_| RuntimeProjectionError::InvalidSource)?;
        if !policy_matches_request(&policy, request) {
            return Err(RuntimeProjectionError::BindingMismatch);
        }
        Ok(Self {
            policy,
            transcript_bytes: 0,
            transcripts: Vec::new(),
            metrics: Vec::new(),
            diagnostics: Vec::new(),
        })
    }

    /// Projects and collects one verified user/assistant exchange without persistence.
    pub fn collect_transcript(
        &mut self,
        request: &RuntimeRunRequest,
        outcome: &RuntimeOutcome,
        placement: &RuntimeTranscriptPlacement,
    ) -> Result<RuntimeProjectionCollection, RuntimeProjectionError> {
        let projection = project_runtime_transcript(&self.policy, request, outcome, placement)?;
        if self.transcripts.iter().any(|retained| {
            retained.request_sha256 == projection.request_sha256
                && retained.outcome_sha256 == projection.outcome_sha256
        }) {
            return Err(RuntimeProjectionError::InvalidSource);
        }
        let next_bytes = self
            .transcript_bytes
            .checked_add(projection.source_bytes)
            .ok_or(RuntimeProjectionError::ResourceLimit)?;
        if next_bytes > self.policy.max_transcript_bytes {
            return Err(RuntimeProjectionError::ResourceLimit);
        }
        self.transcript_bytes = next_bytes;
        self.transcripts.push(projection);
        Ok(RuntimeProjectionCollection {
            transcripts_added: 1,
            ..RuntimeProjectionCollection::default()
        })
    }

    /// Derives optional metric and diagnostic records from one verified event.
    pub fn collect_event(
        &mut self,
        event: &RuntimeEvent,
    ) -> Result<RuntimeProjectionCollection, RuntimeProjectionError> {
        verify_projection_event(&self.policy, event)?;
        let metric = project_runtime_metric(&self.policy, event)?;
        let diagnostic = project_runtime_diagnostic(&self.policy, event)?;
        if metric.as_ref().is_some_and(|candidate| {
            self.metrics
                .iter()
                .any(|retained| retained.event_id == candidate.event_id)
        }) || diagnostic.as_ref().is_some_and(|candidate| {
            self.diagnostics
                .iter()
                .any(|retained| retained.event_id == candidate.event_id)
        }) {
            return Err(RuntimeProjectionError::InvalidSource);
        }
        if metric.is_some() && self.metrics.len() >= self.policy.max_metric_records {
            return Err(RuntimeProjectionError::ResourceLimit);
        }
        if diagnostic.is_some() && self.diagnostics.len() >= self.policy.max_diagnostic_records {
            return Err(RuntimeProjectionError::ResourceLimit);
        }
        let result = RuntimeProjectionCollection {
            metrics_added: usize::from(metric.is_some()),
            diagnostics_added: usize::from(diagnostic.is_some()),
            ..RuntimeProjectionCollection::default()
        };
        if let Some(metric) = metric {
            self.metrics.push(metric);
        }
        if let Some(diagnostic) = diagnostic {
            self.diagnostics.push(diagnostic);
        }
        Ok(result)
    }

    /// Returns collected transcript projections in canonical collection order.
    #[must_use]
    pub fn transcripts(&self) -> &[RuntimeTranscriptProjection] {
        &self.transcripts
    }

    /// Returns collected content-free metric records in event order.
    #[must_use]
    pub fn metrics(&self) -> &[RuntimeMetricProjection] {
        &self.metrics
    }

    /// Returns collected content-free diagnostics in event order.
    #[must_use]
    pub fn diagnostics(&self) -> &[RuntimeDiagnosticProjection] {
        &self.diagnostics
    }
}

/// Projects one verified request/outcome pair into canonical conversation-turn contracts.
pub fn project_runtime_transcript(
    policy: &RuntimeProjectionPolicy,
    request: &RuntimeRunRequest,
    outcome: &RuntimeOutcome,
    placement: &RuntimeTranscriptPlacement,
) -> Result<RuntimeTranscriptProjection, RuntimeProjectionError> {
    validate_policy(policy)?;
    if !policy.transcript_enabled {
        return Err(RuntimeProjectionError::ProjectionDisabled);
    }
    if policy.safe_mode {
        return Err(RuntimeProjectionError::SafeModeDenied);
    }
    verify_runtime_run_request(request).map_err(|_| RuntimeProjectionError::InvalidSource)?;
    verify_runtime_outcome(outcome, request).map_err(|_| RuntimeProjectionError::InvalidSource)?;
    if !policy_matches_request(policy, request) {
        return Err(RuntimeProjectionError::BindingMismatch);
    }
    validate_placement(placement)?;

    let user_bytes = request.task.objective.as_bytes();
    let (assistant_text, assistant_sha256, assistant_bytes, attachments) =
        assistant_projection(outcome, placement.retain_text)?;
    let source_bytes = u64::try_from(user_bytes.len())
        .ok()
        .and_then(|bytes| bytes.checked_add(assistant_bytes))
        .ok_or(RuntimeProjectionError::ResourceLimit)?;
    if source_bytes > policy.max_transcript_bytes {
        return Err(RuntimeProjectionError::ResourceLimit);
    }
    let assistant_ordinal = placement
        .first_ordinal
        .checked_add(1)
        .ok_or(RuntimeProjectionError::InvalidSource)?;
    let mut citations = outcome
        .evidence
        .iter()
        .map(|evidence| evidence.evidence_id.as_str().to_owned())
        .collect::<Vec<_>>();
    citations.sort();
    citations.dedup();
    let mut assistant_sources = outcome
        .evidence
        .iter()
        .map(|evidence| evidence.content_sha256.clone())
        .chain([outcome.outcome_sha256.clone(), assistant_sha256.clone()])
        .collect::<Vec<_>>();
    assistant_sources.sort();
    assistant_sources.dedup();
    let mut receipt_ids = outcome.receipt_ids.clone();
    receipt_ids.sort_by(|left, right| left.as_str().cmp(right.as_str()));
    receipt_ids.dedup();

    let user_turn = ConversationTurn {
        schema_version: CONTRACT_SCHEMA_VERSION,
        turn_id: transcript_turn_id(&request.run_id, "user"),
        conversation_id: placement.conversation_id.clone(),
        ordinal: placement.first_ordinal,
        role: ConversationTurnRole::User,
        sensitivity: placement.sensitivity,
        created_at_epoch_ms: placement.user_created_at_epoch_ms,
        local_date: placement.local_date.clone(),
        text: placement
            .retain_text
            .then(|| request.task.objective.clone()),
        text_sha256: sha256(user_bytes),
        attachments: Vec::new(),
        grant_ids: Vec::new(),
        receipt_ids: Vec::new(),
        checkpoint_id: None,
        citation_ids: Vec::new(),
        source_sha256: vec![request.request_sha256.clone()],
    };
    let assistant_turn = ConversationTurn {
        schema_version: CONTRACT_SCHEMA_VERSION,
        turn_id: transcript_turn_id(&request.run_id, "assistant"),
        conversation_id: placement.conversation_id.clone(),
        ordinal: assistant_ordinal,
        role: ConversationTurnRole::Assistant,
        sensitivity: placement.sensitivity,
        created_at_epoch_ms: placement.assistant_created_at_epoch_ms,
        local_date: placement.local_date.clone(),
        text: assistant_text,
        text_sha256: assistant_sha256,
        attachments,
        grant_ids: Vec::new(),
        receipt_ids,
        checkpoint_id: None,
        citation_ids: citations,
        source_sha256: assistant_sources,
    };
    verify_conversation_turn(&user_turn).map_err(|_| RuntimeProjectionError::InvalidSource)?;
    verify_conversation_turn(&assistant_turn).map_err(|_| RuntimeProjectionError::InvalidSource)?;
    let mut projection = RuntimeTranscriptProjection {
        schema_version: PROJECTION_SCHEMA_VERSION,
        request_sha256: request.request_sha256.clone(),
        outcome_sha256: outcome.outcome_sha256.clone(),
        user_turn,
        assistant_turn,
        source_bytes,
        carries_authority: false,
        projection_sha256: ZERO_SHA256.to_owned(),
    };
    projection.projection_sha256 = canonical_sha256(&projection)?;
    Ok(projection)
}

/// Verifies one retained transcript projection and its self-digest.
pub fn verify_runtime_transcript_projection(
    projection: &RuntimeTranscriptProjection,
) -> Result<(), RuntimeProjectionError> {
    let assistant_ordinal = projection.user_turn.ordinal.checked_add(1);
    if projection.schema_version != PROJECTION_SCHEMA_VERSION
        || projection.carries_authority
        || !valid_sha256(&projection.request_sha256)
        || !valid_sha256(&projection.outcome_sha256)
        || projection.source_bytes == 0
        || projection.user_turn.role != ConversationTurnRole::User
        || projection.assistant_turn.role != ConversationTurnRole::Assistant
        || projection.user_turn.conversation_id != projection.assistant_turn.conversation_id
        || assistant_ordinal != Some(projection.assistant_turn.ordinal)
        || projection.user_turn.turn_id == projection.assistant_turn.turn_id
        || projection.user_turn.source_sha256 != [projection.request_sha256.clone()]
        || !projection
            .assistant_turn
            .source_sha256
            .contains(&projection.outcome_sha256)
        || !projection.user_turn.attachments.is_empty()
        || !projection.user_turn.grant_ids.is_empty()
        || !projection.user_turn.receipt_ids.is_empty()
        || projection.user_turn.checkpoint_id.is_some()
        || !projection.user_turn.citation_ids.is_empty()
        || !projection.assistant_turn.grant_ids.is_empty()
        || projection.assistant_turn.checkpoint_id.is_some()
        || !valid_sha256(&projection.projection_sha256)
    {
        return Err(RuntimeProjectionError::InvalidSource);
    }
    verify_conversation_turn(&projection.user_turn)
        .map_err(|_| RuntimeProjectionError::InvalidSource)?;
    verify_conversation_turn(&projection.assistant_turn)
        .map_err(|_| RuntimeProjectionError::InvalidSource)?;
    let mut candidate = projection.clone();
    candidate.projection_sha256 = ZERO_SHA256.to_owned();
    if canonical_sha256(&candidate)? != projection.projection_sha256 {
        return Err(RuntimeProjectionError::InvalidSource);
    }
    Ok(())
}

/// Verifies one retained content-free metric projection.
pub fn verify_runtime_metric_projection(
    projection: &RuntimeMetricProjection,
) -> Result<(), RuntimeProjectionError> {
    if !valid_id(projection.run_id.as_str())
        || !valid_id(projection.event_id.as_str())
        || !valid_code(&projection.name)
        || projection.occurred_at_epoch_ms == 0
    {
        return Err(RuntimeProjectionError::InvalidSource);
    }
    verify_projection_digest(
        projection.schema_version,
        projection.carries_authority,
        &projection.event_sha256,
        &projection.projection_sha256,
        projection,
        |candidate| candidate.projection_sha256 = ZERO_SHA256.to_owned(),
    )
}

/// Verifies one retained content-free diagnostic projection.
pub fn verify_runtime_diagnostic_projection(
    projection: &RuntimeDiagnosticProjection,
) -> Result<(), RuntimeProjectionError> {
    if !valid_id(projection.run_id.as_str())
        || !valid_id(projection.event_id.as_str())
        || !valid_code(&projection.reason_code)
        || projection.occurred_at_epoch_ms == 0
    {
        return Err(RuntimeProjectionError::InvalidSource);
    }
    verify_projection_digest(
        projection.schema_version,
        projection.carries_authority,
        &projection.event_sha256,
        &projection.projection_sha256,
        projection,
        |candidate| candidate.projection_sha256 = ZERO_SHA256.to_owned(),
    )
}

fn project_runtime_metric(
    policy: &RuntimeProjectionPolicy,
    event: &RuntimeEvent,
) -> Result<Option<RuntimeMetricProjection>, RuntimeProjectionError> {
    if !policy.metrics_enabled
        || policy.safe_mode
        || event.sensitivity == ContextSensitivity::Restricted
    {
        return Ok(None);
    }
    let RuntimeEventKind::Metric { name, value } = &event.kind else {
        return Ok(None);
    };
    if event.persistence != RuntimeEventPersistenceClass::Metric {
        return Err(RuntimeProjectionError::InvalidSource);
    }
    let mut projection = RuntimeMetricProjection {
        schema_version: PROJECTION_SCHEMA_VERSION,
        run_id: event.run_id.clone(),
        event_id: event.event_id.clone(),
        event_sha256: event.event_sha256.clone(),
        name: name.clone(),
        value: *value,
        occurred_at_epoch_ms: event.occurred_at_epoch_ms,
        carries_authority: false,
        projection_sha256: ZERO_SHA256.to_owned(),
    };
    projection.projection_sha256 = canonical_sha256(&projection)?;
    Ok(Some(projection))
}

fn project_runtime_diagnostic(
    policy: &RuntimeProjectionPolicy,
    event: &RuntimeEvent,
) -> Result<Option<RuntimeDiagnosticProjection>, RuntimeProjectionError> {
    if !policy.diagnostics_enabled || event.sensitivity == ContextSensitivity::Restricted {
        return Ok(None);
    }
    let (kind, reason_code) = match &event.kind {
        RuntimeEventKind::ModelFailed { failure_code, .. } => {
            (RuntimeDiagnosticKind::ModelFailure, failure_code.clone())
        }
        RuntimeEventKind::ToolFailed { failure_code, .. } => {
            (RuntimeDiagnosticKind::ToolFailure, failure_code.clone())
        }
        RuntimeEventKind::RunTerminal { state, .. } if state.is_terminal() => (
            RuntimeDiagnosticKind::Terminal,
            terminal_reason_code(*state).to_owned(),
        ),
        RuntimeEventKind::RunTerminal { .. } => {
            return Err(RuntimeProjectionError::InvalidSource);
        }
        _ => return Ok(None),
    };
    let mut projection = RuntimeDiagnosticProjection {
        schema_version: PROJECTION_SCHEMA_VERSION,
        run_id: event.run_id.clone(),
        event_id: event.event_id.clone(),
        event_sha256: event.event_sha256.clone(),
        kind,
        reason_code,
        occurred_at_epoch_ms: event.occurred_at_epoch_ms,
        carries_authority: false,
        projection_sha256: ZERO_SHA256.to_owned(),
    };
    projection.projection_sha256 = canonical_sha256(&projection)?;
    Ok(Some(projection))
}

fn verify_projection_event(
    policy: &RuntimeProjectionPolicy,
    event: &RuntimeEvent,
) -> Result<(), RuntimeProjectionError> {
    verify_runtime_event(event).map_err(|_| RuntimeProjectionError::InvalidSource)?;
    if event.run_id != policy.run_id
        || event.session_id != policy.session_id
        || event.task_id != policy.task_id
        || event.policy_id != policy.policy_id
    {
        return Err(RuntimeProjectionError::BindingMismatch);
    }
    Ok(())
}

fn assistant_projection(
    outcome: &RuntimeOutcome,
    retain_text: bool,
) -> Result<
    (
        Option<String>,
        String,
        u64,
        Vec<ConversationAttachmentReference>,
    ),
    RuntimeProjectionError,
> {
    match &outcome.output {
        Some(RuntimeOutput::Inline { payload }) => {
            let bytes = u64::try_from(payload.bytes.len())
                .map_err(|_| RuntimeProjectionError::ResourceLimit)?;
            let text = if retain_text && textual_media_type(&payload.media_type) {
                Some(
                    std::str::from_utf8(&payload.bytes)
                        .map_err(|_| RuntimeProjectionError::InvalidSource)?
                        .to_owned(),
                )
            } else {
                None
            };
            Ok((text, payload.sha256.clone(), bytes, Vec::new()))
        }
        Some(RuntimeOutput::Artifact { reference }) => Ok((
            None,
            reference.sha256.clone(),
            reference.byte_size,
            vec![ConversationAttachmentReference {
                reference_id: reference.artifact_id.as_str().to_owned(),
                display_name: "Runtime output".to_owned(),
                content_sha256: reference.sha256.clone(),
                media_type: reference.media_type.clone(),
            }],
        )),
        None => Ok((None, sha256(&[]), 0, Vec::new())),
    }
}

fn validate_policy(policy: &RuntimeProjectionPolicy) -> Result<(), RuntimeProjectionError> {
    if !valid_id(policy.run_id.as_str())
        || !valid_id(policy.session_id.as_str())
        || !valid_id(policy.task_id.as_str())
        || !valid_id(policy.policy_id.as_str())
        || !valid_sha256(&policy.policy_sha256)
        || policy.max_transcript_bytes == 0
        || policy.max_transcript_bytes > MAX_TRANSCRIPT_BYTES
        || policy.max_metric_records == 0
        || policy.max_metric_records > MAX_METRIC_RECORDS
        || policy.max_diagnostic_records == 0
        || policy.max_diagnostic_records > MAX_DIAGNOSTIC_RECORDS
    {
        return Err(RuntimeProjectionError::InvalidPolicy);
    }
    Ok(())
}

fn policy_matches_request(policy: &RuntimeProjectionPolicy, request: &RuntimeRunRequest) -> bool {
    policy.run_id == request.run_id
        && policy.session_id == request.session_id
        && policy.task_id == request.task.task_id
        && policy.policy_id == request.policy_id
        && policy.policy_sha256 == request.policy_sha256
}

fn validate_placement(
    placement: &RuntimeTranscriptPlacement,
) -> Result<(), RuntimeProjectionError> {
    if !placement
        .conversation_id
        .as_str()
        .starts_with("conversation-")
        || placement.first_ordinal == 0
        || placement.user_created_at_epoch_ms == 0
        || placement.assistant_created_at_epoch_ms < placement.user_created_at_epoch_ms
        || !valid_local_date(&placement.local_date)
        || matches!(
            placement.sensitivity,
            DataSensitivity::Ephemeral | DataSensitivity::Restricted
        )
    {
        return Err(if placement.sensitivity == DataSensitivity::Restricted {
            RuntimeProjectionError::Restricted
        } else {
            RuntimeProjectionError::InvalidSource
        });
    }
    Ok(())
}

fn verify_projection_digest<T: Clone + Serialize>(
    schema_version: u16,
    carries_authority: bool,
    event_sha256: &str,
    projection_sha256: &str,
    projection: &T,
    clear: impl FnOnce(&mut T),
) -> Result<(), RuntimeProjectionError> {
    if schema_version != PROJECTION_SCHEMA_VERSION
        || carries_authority
        || !valid_sha256(event_sha256)
        || !valid_sha256(projection_sha256)
    {
        return Err(RuntimeProjectionError::InvalidSource);
    }
    let mut candidate = projection.clone();
    clear(&mut candidate);
    if canonical_sha256(&candidate)? != projection_sha256 {
        return Err(RuntimeProjectionError::InvalidSource);
    }
    Ok(())
}

const fn terminal_reason_code(state: AgentStateKind) -> &'static str {
    match state {
        AgentStateKind::Success => "runtime.terminal.success",
        AgentStateKind::NoOp => "runtime.terminal.no_op",
        AgentStateKind::Blocked => "runtime.terminal.blocked",
        AgentStateKind::Declined => "runtime.terminal.declined",
        AgentStateKind::Stalled => "runtime.terminal.stalled",
        AgentStateKind::Exhausted => "runtime.terminal.exhausted",
        AgentStateKind::Uncertain => "runtime.terminal.uncertain",
        AgentStateKind::Cancelled => "runtime.terminal.cancelled",
        AgentStateKind::Failed => "runtime.terminal.failed",
        AgentStateKind::Observation
        | AgentStateKind::Proposal
        | AgentStateKind::Validation
        | AgentStateKind::Clarification
        | AgentStateKind::Approval
        | AgentStateKind::Execution
        | AgentStateKind::Verification
        | AgentStateKind::Checkpoint => "runtime.terminal.invalid",
    }
}

fn transcript_turn_id(run_id: &RuntimeRunId, role: &str) -> ConversationTurnId {
    let digest = sha256(format!("runtime-transcript-v1\n{}\n{role}\n", run_id.as_str()).as_bytes());
    ConversationTurnId::from_raw(format!("turn-runtime-{}", &digest[..32]))
}

fn textual_media_type(value: &str) -> bool {
    value.starts_with("text/") || matches!(value, "application/json" | "application/markdown")
}

fn valid_local_date(value: &str) -> bool {
    value.len() == 10
        && value.as_bytes()[4] == b'-'
        && value.as_bytes()[7] == b'-'
        && value
            .bytes()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn valid_code(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'_' | b'-' | b':')
        })
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn canonical_sha256<T: Serialize>(value: &T) -> Result<String, RuntimeProjectionError> {
    let bytes = serde_json::to_vec(value).map_err(|_| RuntimeProjectionError::Serialization)?;
    Ok(sha256(&bytes))
}

fn sha256(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut encoded, "{byte:02x}")
            .expect("writing a SHA-256 digest to a String cannot fail");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        model_codec::tests_support::profile,
        runtime_coordinator::{
            runtime_tool_catalog_sha256, seal_runtime_outcome, seal_runtime_run_request,
        },
        runtime_event::{runtime_event_persistence, seal_runtime_event},
    };
    use agentmage_kernel_contracts::{
        AuthorityClass, BudgetLimit, BudgetResource, ContractPayload, EvidenceId, EvidenceKind,
        EvidenceReference, GrantId, PlanId, ReceiptId, RepositorySnapshotId, RollbackPlan,
        RuntimeEventRetention, RuntimeEventRetentionKind, RuntimeRunLimits, RuntimeSessionMode,
        SchemaId, SchemaReference, StopCondition, StopConditionKind, Task, TaskStatus,
        ToolCatalogId, WorkPacket, WorkPacketId, WorkPacketState, WorkspaceId,
    };

    const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

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
            reason: "Verify optional projections".to_owned(),
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
            sensitivity: DataSensitivity::Operational,
            last_verification_date: "2026-08-17".to_owned(),
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
        let model_profile = profile("projection");
        let tool_catalog_id = ToolCatalogId::from_raw("catalog-0001");
        let visible_tools = Vec::new();
        seal_runtime_run_request(RuntimeRunRequest {
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
            workspace_snapshot_sha256: HASH_A.to_owned(),
            repository_snapshot_id: RepositorySnapshotId::from_raw("snapshot-0001"),
            repository_snapshot_sha256: "b".repeat(64),
            context_budget: model_profile.context.clone(),
            model_profile,
            tool_catalog_sha256: runtime_tool_catalog_sha256(&tool_catalog_id, &visible_tools)
                .expect("catalog hashes"),
            tool_catalog_id,
            visible_tools,
            policy_id: PolicyId::from_raw("policy-0001"),
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
            request_sha256: ZERO_SHA256.to_owned(),
        })
        .expect("request seals")
    }

    fn outcome(request: &RuntimeRunRequest) -> RuntimeOutcome {
        let bytes = b"Grounded fixture answer".to_vec();
        seal_runtime_outcome(
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
                evidence: vec![EvidenceReference {
                    schema_version: CONTRACT_SCHEMA_VERSION,
                    evidence_id: EvidenceId::from_raw("evidence-0001"),
                    kind: EvidenceKind::Observation,
                    source_id: "fixture".to_owned(),
                    object_id: "fixture/input.txt".to_owned(),
                    fragment: Some("line:1".to_owned()),
                    content_sha256: HASH_A.to_owned(),
                    observed_revision: Some("snapshot-0001".to_owned()),
                }],
                receipt_ids: vec![ReceiptId::from_raw("receipt-0001")],
                unresolved_codes: Vec::new(),
                output: Some(RuntimeOutput::Inline {
                    payload: ContractPayload {
                        schema: SchemaReference {
                            schema_id: SchemaId::from_raw("runtime.answer"),
                            schema_version: 1,
                            schema_sha256: HASH_A.to_owned(),
                        },
                        media_type: "text/plain".to_owned(),
                        sha256: sha256(&bytes),
                        bytes,
                    },
                }),
                outcome_sha256: ZERO_SHA256.to_owned(),
            },
            request,
        )
        .expect("outcome seals")
    }

    fn policy(request: &RuntimeRunRequest) -> RuntimeProjectionPolicy {
        RuntimeProjectionPolicy {
            run_id: request.run_id.clone(),
            session_id: request.session_id.clone(),
            task_id: request.task.task_id.clone(),
            policy_id: request.policy_id.clone(),
            policy_sha256: request.policy_sha256.clone(),
            transcript_enabled: true,
            metrics_enabled: true,
            diagnostics_enabled: true,
            safe_mode: false,
            max_transcript_bytes: 4_096,
            max_metric_records: 4,
            max_diagnostic_records: 4,
        }
    }

    fn placement() -> RuntimeTranscriptPlacement {
        RuntimeTranscriptPlacement {
            conversation_id: ConversationId::from_raw("conversation-runtime-0001"),
            first_ordinal: 1,
            user_created_at_epoch_ms: 1_000,
            assistant_created_at_epoch_ms: 2_000,
            local_date: "2026-08-17".to_owned(),
            sensitivity: DataSensitivity::Operational,
            retain_text: true,
        }
    }

    fn event(
        request: &RuntimeRunRequest,
        sequence: u64,
        sensitivity: ContextSensitivity,
        kind: RuntimeEventKind,
    ) -> RuntimeEvent {
        let persistence = runtime_event_persistence(&kind);
        seal_runtime_event(RuntimeEvent {
            schema_version: CONTRACT_SCHEMA_VERSION,
            event_id: RuntimeEventId::from_raw(format!("event-{sequence:04}")),
            run_id: request.run_id.clone(),
            session_id: request.session_id.clone(),
            task_id: request.task.task_id.clone(),
            turn_id: None,
            operation_id: None,
            correlation_id: agentmage_kernel_contracts::CorrelationId::from_raw("correlation-0001"),
            causation_event_id: Some(RuntimeEventId::from_raw(format!(
                "event-{:04}",
                sequence.saturating_sub(1)
            ))),
            sequence,
            occurred_at_epoch_ms: 2_000 + sequence,
            sensitivity,
            retention: RuntimeEventRetention {
                kind: RuntimeEventRetentionKind::Session,
                expires_at_epoch_ms: None,
            },
            persistence,
            policy_id: request.policy_id.clone(),
            payload_reference: None,
            kind,
            previous_event_sha256: HASH_A.to_owned(),
            event_sha256: ZERO_SHA256.to_owned(),
        })
        .expect("event seals")
    }

    #[test]
    fn story_50_2_verified_runtime_exchange_projects_valid_non_authoritative_turns() {
        let request = request();
        let outcome = outcome(&request);
        let projection =
            project_runtime_transcript(&policy(&request), &request, &outcome, &placement())
                .expect("transcript projects");
        verify_runtime_transcript_projection(&projection).expect("projection verifies");
        assert_eq!(
            projection.user_turn.text.as_deref(),
            Some("Inspect one synthetic fixture")
        );
        assert_eq!(
            projection.assistant_turn.text.as_deref(),
            Some("Grounded fixture answer")
        );
        assert_eq!(projection.assistant_turn.citation_ids, ["evidence-0001"]);
        assert_eq!(projection.assistant_turn.receipt_ids.len(), 1);
        assert!(!projection.carries_authority);

        let mut changed = projection;
        changed.assistant_turn.text = Some("forged".to_owned());
        assert_eq!(
            verify_runtime_transcript_projection(&changed),
            Err(RuntimeProjectionError::InvalidSource)
        );

        let projection =
            project_runtime_transcript(&policy(&request), &request, &outcome, &placement())
                .expect("transcript projects again");
        let mut forged_authority = projection.clone();
        forged_authority
            .assistant_turn
            .grant_ids
            .push(GrantId::from_raw("grant-forged"));
        forged_authority.projection_sha256 = ZERO_SHA256.to_owned();
        forged_authority.projection_sha256 =
            canonical_sha256(&forged_authority).expect("projection rehashes");
        assert_eq!(
            verify_runtime_transcript_projection(&forged_authority),
            Err(RuntimeProjectionError::InvalidSource)
        );

        let mut broken_link = projection;
        broken_link.assistant_turn.source_sha256 = vec![HASH_A.to_owned()];
        broken_link.projection_sha256 = ZERO_SHA256.to_owned();
        broken_link.projection_sha256 =
            canonical_sha256(&broken_link).expect("projection rehashes");
        assert_eq!(
            verify_runtime_transcript_projection(&broken_link),
            Err(RuntimeProjectionError::InvalidSource)
        );
    }

    #[test]
    fn story_50_2_transcript_projection_obeys_opt_in_safe_mode_sensitivity_and_budget() {
        let request = request();
        let outcome = outcome(&request);
        let mut candidate_policy = policy(&request);
        candidate_policy.transcript_enabled = false;
        assert_eq!(
            project_runtime_transcript(&candidate_policy, &request, &outcome, &placement()),
            Err(RuntimeProjectionError::ProjectionDisabled)
        );

        candidate_policy.transcript_enabled = true;
        candidate_policy.safe_mode = true;
        assert_eq!(
            project_runtime_transcript(&candidate_policy, &request, &outcome, &placement()),
            Err(RuntimeProjectionError::SafeModeDenied)
        );

        candidate_policy.safe_mode = false;
        let mut restricted = placement();
        restricted.sensitivity = DataSensitivity::Restricted;
        assert_eq!(
            project_runtime_transcript(&candidate_policy, &request, &outcome, &restricted),
            Err(RuntimeProjectionError::Restricted)
        );

        candidate_policy.max_transcript_bytes = 1;
        assert_eq!(
            project_runtime_transcript(&candidate_policy, &request, &outcome, &placement()),
            Err(RuntimeProjectionError::ResourceLimit)
        );
    }

    #[test]
    fn story_50_2_metrics_and_diagnostics_are_content_free_event_bound_and_bounded() {
        let request = request();
        let mut collector =
            RuntimeProjectionCollector::new(policy(&request), &request).expect("collector");
        let metric = event(
            &request,
            1,
            ContextSensitivity::Internal,
            RuntimeEventKind::Metric {
                name: "runtime.queue.depth".to_owned(),
                value: 7,
            },
        );
        assert_eq!(
            collector
                .collect_event(&metric)
                .expect("metric collects")
                .metrics_added,
            1
        );
        assert_eq!(collector.metrics()[0].name, "runtime.queue.depth");
        verify_runtime_metric_projection(&collector.metrics()[0]).expect("metric verifies");
        let mut malformed_metric = collector.metrics()[0].clone();
        malformed_metric.name = "not/a/content/free/code".to_owned();
        malformed_metric.projection_sha256 = ZERO_SHA256.to_owned();
        malformed_metric.projection_sha256 =
            canonical_sha256(&malformed_metric).expect("metric rehashes");
        assert_eq!(
            verify_runtime_metric_projection(&malformed_metric),
            Err(RuntimeProjectionError::InvalidSource)
        );

        let failure = event(
            &request,
            2,
            ContextSensitivity::Private,
            RuntimeEventKind::ModelFailed {
                model_run_id: agentmage_kernel_contracts::ModelRunId::from_raw("model-run-0001"),
                failure_code: "runtime.model.timed_out".to_owned(),
            },
        );
        assert_eq!(
            collector
                .collect_event(&failure)
                .expect("diagnostic collects")
                .diagnostics_added,
            1
        );
        assert_eq!(
            collector.diagnostics()[0].reason_code,
            "runtime.model.timed_out"
        );
        verify_runtime_diagnostic_projection(&collector.diagnostics()[0])
            .expect("diagnostic verifies");
        let mut malformed_diagnostic = collector.diagnostics()[0].clone();
        malformed_diagnostic.reason_code = "secret\nvalue".to_owned();
        malformed_diagnostic.projection_sha256 = ZERO_SHA256.to_owned();
        malformed_diagnostic.projection_sha256 =
            canonical_sha256(&malformed_diagnostic).expect("diagnostic rehashes");
        assert_eq!(
            verify_runtime_diagnostic_projection(&malformed_diagnostic),
            Err(RuntimeProjectionError::InvalidSource)
        );

        assert_eq!(
            collector.collect_event(&metric),
            Err(RuntimeProjectionError::InvalidSource)
        );
        assert_eq!(collector.metrics().len(), 1);
    }

    #[test]
    fn story_50_2_safe_mode_and_restricted_events_never_collect_optional_telemetry() {
        let request = request();
        let mut safe_policy = policy(&request);
        safe_policy.safe_mode = true;
        let mut collector =
            RuntimeProjectionCollector::new(safe_policy, &request).expect("safe collector");
        let metric = event(
            &request,
            1,
            ContextSensitivity::Internal,
            RuntimeEventKind::Metric {
                name: "runtime.queue.depth".to_owned(),
                value: 7,
            },
        );
        assert_eq!(
            collector.collect_event(&metric),
            Ok(RuntimeProjectionCollection::default())
        );

        let terminal = event(
            &request,
            2,
            ContextSensitivity::Internal,
            RuntimeEventKind::RunTerminal {
                state: AgentStateKind::Failed,
                outcome_sha256: "d".repeat(64),
            },
        );
        assert_eq!(
            collector
                .collect_event(&terminal)
                .expect("safe diagnostic collects")
                .diagnostics_added,
            1
        );

        let restricted = event(
            &request,
            3,
            ContextSensitivity::Restricted,
            RuntimeEventKind::ToolFailed {
                tool_call_id: agentmage_kernel_contracts::ToolCallId::from_raw("call-0001"),
                receipt_id: None,
                failure_code: "runtime.tool.failed".to_owned(),
            },
        );
        assert_eq!(
            collector.collect_event(&restricted),
            Ok(RuntimeProjectionCollection::default())
        );
        assert_eq!(collector.diagnostics().len(), 1);
    }

    #[test]
    fn story_50_2_collection_overage_and_binding_drift_are_non_mutating() {
        let request = request();
        let mut bounded_policy = policy(&request);
        bounded_policy.max_metric_records = 1;
        let mut collector =
            RuntimeProjectionCollector::new(bounded_policy, &request).expect("collector");
        let first = event(
            &request,
            1,
            ContextSensitivity::Internal,
            RuntimeEventKind::Metric {
                name: "runtime.queue.depth".to_owned(),
                value: 1,
            },
        );
        collector.collect_event(&first).expect("first metric");
        let second = event(
            &request,
            2,
            ContextSensitivity::Internal,
            RuntimeEventKind::Metric {
                name: "runtime.queue.depth".to_owned(),
                value: 2,
            },
        );
        assert_eq!(
            collector.collect_event(&second),
            Err(RuntimeProjectionError::ResourceLimit)
        );
        assert_eq!(collector.metrics().len(), 1);

        let mut foreign = second;
        foreign.session_id = SessionId::from_raw("session-foreign");
        foreign.event_sha256 = ZERO_SHA256.to_owned();
        foreign = seal_runtime_event(foreign).expect("foreign event reseals");
        assert_eq!(
            collector.collect_event(&foreign),
            Err(RuntimeProjectionError::BindingMismatch)
        );
        assert_eq!(collector.metrics().len(), 1);
    }
}
