//! Closed interface-independent runtime event contracts.

use crate::{
    AgentStateKind, ApprovalId, CancellationId, ContextSensitivity, CorrelationId, GrantId,
    GrantOperation, ModelRunId, PolicyId, ReceiptId, RuntimeArtifactId, RuntimeEventId,
    RuntimeOperationId, RuntimeRunId, RuntimeTurnId, SessionCheckpointId, SessionId, TaskId,
    ToolCallId,
};

/// Closed retention class for one runtime event or referenced payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeEventRetentionKind {
    /// Keep only in process memory for the current run.
    Ephemeral,
    /// Retain under the owning session lifecycle.
    Session,
    /// Retain until the exact policy-selected expiration.
    UntilExpiration,
    /// Retain until an explicit user release decision.
    UserHold,
}

/// Exact retention assignment carried by one runtime event.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeEventRetention {
    /// Closed retention class.
    pub kind: RuntimeEventRetentionKind,
    /// Exclusive expiration for `UntilExpiration`; absent for every other class.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub expires_at_epoch_ms: Option<u64>,
}

/// Persistence and delivery class assigned independently from event content.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeEventPersistenceClass {
    /// Correctness-bearing event retained with its canonical transaction.
    Correctness,
    /// Ordered progress eligible for bounded asynchronous batch persistence.
    Progress,
    /// Content-free local metric eligible only for the optional metrics projection.
    Metric,
}

/// Verified reference to a large payload stored outside the event envelope.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePayloadReference {
    /// Stable runtime artifact identity.
    pub artifact_id: RuntimeArtifactId,
    /// Lowercase SHA-256 digest of the exact payload bytes.
    pub sha256: String,
    /// Exact payload size in bytes.
    pub byte_size: u64,
    /// Closed or policy-approved media type.
    pub media_type: String,
}

/// User-visible deterministic disposition projected from policy and grant state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RuntimePermissionDisposition {
    /// Current policy and an exact consumable grant permit dispatch.
    Allow,
    /// A protected user decision is required before any effect may begin.
    Ask,
    /// The exact operation is prohibited, invalid, or outside current scope.
    Deny,
}

/// Closed content-minimized runtime event family.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeEventKind {
    /// One exact reusable-runtime run began.
    RunStarted {
        /// Digest of the exact admitted runtime request.
        request_sha256: String,
    },
    /// One ordered turn began.
    TurnStarted,
    /// One ordered turn ended without claiming run completion.
    TurnCompleted {
        /// Digest of the exact turn result projection.
        outcome_sha256: String,
    },
    /// A bounded model request was admitted for execution.
    ModelRequested {
        /// Exact model-run identity.
        model_run_id: ModelRunId,
        /// Digest of the bounded request supplied to the model adapter.
        request_sha256: String,
    },
    /// A model run produced a validated typed result.
    ModelCompleted {
        /// Exact model-run identity.
        model_run_id: ModelRunId,
        /// Digest of the typed result, never raw model text.
        result_sha256: String,
    },
    /// A model run ended without a valid typed result.
    ModelFailed {
        /// Exact model-run identity.
        model_run_id: ModelRunId,
        /// Stable content-free failure code.
        failure_code: String,
    },
    /// A typed tool call was proposed and validated structurally.
    ToolRequested {
        /// Exact tool-call identity.
        tool_call_id: ToolCallId,
        /// Digest of the validated arguments.
        arguments_sha256: String,
    },
    /// A separately authorized tool worker crossed its launch boundary.
    ToolStarted {
        /// Exact tool-call identity.
        tool_call_id: ToolCallId,
        /// Digest of the consumed authority transaction.
        authority_sha256: String,
    },
    /// A tool worker returned one reconciled terminal result.
    ToolCompleted {
        /// Exact tool-call identity.
        tool_call_id: ToolCallId,
        /// Canonical receipt identity.
        receipt_id: ReceiptId,
        /// Digest of the reconciled result.
        result_sha256: String,
    },
    /// A tool worker failed or produced an uncertain terminal result.
    ToolFailed {
        /// Exact tool-call identity.
        tool_call_id: ToolCallId,
        /// Optional receipt identity when a terminal receipt was available.
        #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
        receipt_id: Option<ReceiptId>,
        /// Stable content-free failure code.
        failure_code: String,
    },
    /// Policy determined that one operation requires an explicit decision.
    PermissionRequested {
        /// Exact protected approval identity.
        approval_id: ApprovalId,
        /// Closed operation awaiting a decision.
        operation: GrantOperation,
        /// Digest of the complete protected preview.
        preview_sha256: String,
        /// Exclusive approval expiration.
        expires_at_epoch_ms: u64,
    },
    /// Policy and grant state produced one visible operation disposition.
    PermissionDecided {
        /// Exact approval identity considered by the decision.
        approval_id: ApprovalId,
        /// Deterministic visible disposition.
        disposition: RuntimePermissionDisposition,
        /// Exact grant identity only for an allowed operation.
        #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
        grant_id: Option<GrantId>,
        /// Digest of the complete policy decision.
        decision_sha256: String,
    },
    /// A file object was observed without mutation.
    FileObserved {
        /// Digest of the held workspace-object identity, never a raw path.
        object_identity_sha256: String,
        /// Digest of the observed content or metadata.
        observation_sha256: String,
    },
    /// A separately granted file change was reconciled.
    FileModified {
        /// Digest of the held workspace-object identity, never a raw path.
        object_identity_sha256: String,
        /// Digest of the verified postcondition.
        postcondition_sha256: String,
        /// Canonical write receipt identity.
        receipt_id: ReceiptId,
    },
    /// A content-addressed runtime artifact became available by verified reference.
    ArtifactCreated {
        /// Exact artifact identity, matching the top-level payload reference.
        artifact_id: RuntimeArtifactId,
        /// Digest of the immutable artifact manifest.
        manifest_sha256: String,
    },
    /// A supplied source artifact crossed admission with an immutable manifest.
    SourceAdmitted {
        /// Exact admitted source artifact identity.
        source_artifact_id: RuntimeArtifactId,
        /// Digest of the immutable source manifest.
        manifest_sha256: String,
    },
    /// One deterministic extraction attempt began for an admitted source.
    ExtractionStarted {
        /// Exact admitted source artifact identity.
        source_artifact_id: RuntimeArtifactId,
        /// Exact extraction identity.
        extraction_id: String,
        /// Digest of the extractor, source revision, and limits.
        input_sha256: String,
    },
    /// One extraction completed with a referenced result.
    ExtractionCompleted {
        /// Exact admitted source artifact identity.
        source_artifact_id: RuntimeArtifactId,
        /// Exact extraction identity.
        extraction_id: String,
        /// Digest of the complete extraction result manifest.
        result_sha256: String,
    },
    /// One extraction stopped with a stable content-free reason.
    ExtractionBlocked {
        /// Exact admitted source artifact identity.
        source_artifact_id: RuntimeArtifactId,
        /// Exact extraction identity.
        extraction_id: String,
        /// Stable content-free blocked reason.
        reason_code: String,
    },
    /// One exact extracted section became index-addressable.
    SectionIndexed {
        /// Exact admitted source artifact identity.
        source_artifact_id: RuntimeArtifactId,
        /// Exact successful extraction identity.
        extraction_id: String,
        /// Stable section identity.
        section_id: String,
        /// Digest of the exact source locator and index record.
        locator_sha256: String,
    },
    /// One source/context inclusion decision became current.
    ContextDisposition {
        /// Exact context-manifest identity.
        context_manifest_id: String,
        /// Exact source artifact governed by the decision.
        source_artifact_id: RuntimeArtifactId,
        /// Digest of the complete disposition and accounting record.
        disposition_sha256: String,
    },
    /// One exact current preflight observation was admitted for an attempt.
    PreflightObserved {
        /// Exact attempt identity.
        attempt_id: String,
        /// Exact registered preflight identity.
        preflight_id: String,
        /// Digest of the bounded preflight observation.
        observation_sha256: String,
    },
    /// One admitted tool attempt crossed its unique start commitment.
    AttemptStarted {
        /// Exact attempt identity.
        attempt_id: String,
        /// Exact tool-call identity.
        tool_call_id: ToolCallId,
        /// Digest of the complete prepared attempt.
        prepared_sha256: String,
    },
    /// One admitted attempt acquired a reconciled terminal observation.
    AttemptEnded {
        /// Exact attempt identity.
        attempt_id: String,
        /// Exact tool-call identity.
        tool_call_id: ToolCallId,
        /// Exact closed observation identity.
        observation_id: String,
        /// Digest of the closed terminal observation.
        observation_sha256: String,
    },
    /// One deterministic verifier result was observed for an ended attempt.
    VerificationObserved {
        /// Exact attempt identity.
        attempt_id: String,
        /// Exact verification-result identity.
        verification_id: String,
        /// Digest of the complete verification result.
        result_sha256: String,
    },
    /// One retry eligibility decision was recorded for an ended attempt.
    RetryDecided {
        /// Exact attempt identity.
        attempt_id: String,
        /// Whether a fresh successor attempt is eligible.
        eligible: bool,
        /// Digest of the complete retry decision.
        decision_sha256: String,
    },
    /// One deterministic recovery decision was recorded.
    RecoveryDecided {
        /// Exact recovery decision identity.
        recovery_id: String,
        /// Digest of the complete recovery decision.
        decision_sha256: String,
    },
    /// One terminal non-success diagnosis became available by verified reference.
    TerminalDiagnostic {
        /// Exact terminal diagnosis identity.
        diagnostic_id: String,
        /// Digest of the complete diagnostic record.
        diagnostic_sha256: String,
    },
    /// One safe-boundary checkpoint was committed.
    CheckpointCommitted {
        /// Exact checkpoint identity.
        checkpoint_id: SessionCheckpointId,
        /// Digest of the canonical checkpoint.
        checkpoint_sha256: String,
    },
    /// Cancellation was requested for the exact run or operation.
    CancellationRequested {
        /// Stable cancellation-tree identity.
        cancellation_id: CancellationId,
    },
    /// Cancellation and owned-descendant cleanup were observed.
    CancellationObserved {
        /// Stable cancellation-tree identity.
        cancellation_id: CancellationId,
    },
    /// Content-free user-visible progress suitable for coalescing.
    Progress {
        /// Stable progress code.
        code: String,
    },
    /// Content-free local metric suitable for optional batching.
    Metric {
        /// Stable metric identity.
        name: String,
        /// Integer metric value in the metric's documented unit.
        value: i64,
    },
    /// The run reached one terminal state backed by its canonical outcome digest.
    RunTerminal {
        /// Exact terminal state from the persisted agent-state contract.
        state: AgentStateKind,
        /// Digest of the canonical runtime outcome.
        outcome_sha256: String,
    },
}

/// One immutable hash-chained event emitted by the reusable runtime.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeEvent {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable event identity.
    pub event_id: RuntimeEventId,
    /// Exact owning runtime run.
    pub run_id: RuntimeRunId,
    /// Exact owning session.
    pub session_id: SessionId,
    /// Exact owning task.
    pub task_id: TaskId,
    /// Exact turn identity when the event occurs inside a turn.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub turn_id: Option<RuntimeTurnId>,
    /// Exact operation identity when the event concerns one operation.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub operation_id: Option<RuntimeOperationId>,
    /// Correlation identity shared by the originating request and derived records.
    pub correlation_id: CorrelationId,
    /// Earlier event that directly caused this event; absent only for `RunStarted`.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub causation_event_id: Option<RuntimeEventId>,
    /// Strictly increasing zero-based sequence within the run.
    pub sequence: u64,
    /// Trusted wall-clock observation in Unix epoch milliseconds.
    pub occurred_at_epoch_ms: u64,
    /// Content sensitivity assigned before projection or persistence.
    pub sensitivity: ContextSensitivity,
    /// Exact retention assignment.
    pub retention: RuntimeEventRetention,
    /// Persistence and projection class.
    pub persistence: RuntimeEventPersistenceClass,
    /// Exact policy identity used for this event.
    pub policy_id: PolicyId,
    /// Optional verified reference for a large payload kept outside the envelope.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub payload_reference: Option<RuntimePayloadReference>,
    /// Closed content-minimized event payload.
    pub kind: RuntimeEventKind,
    /// Digest of the previous event, or all zeroes for sequence zero.
    pub previous_event_sha256: String,
    /// Digest of this canonical event with this field set to all zeroes.
    pub event_sha256: String,
}
