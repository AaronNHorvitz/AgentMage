//! Contracts for Verified Chat and the complete Engineering Runtime boundary.

use crate::{
    ActorId, AgentLeaseId, ArtifactUploadId, CampaignId, CapabilityId, ContextPacketId,
    CorrelationId, EndpointProfileId, EvidenceReference, GrantId, IntegrationId, ModelProfileId,
    ReceiptId, ReviewId, RouteDecisionId, RuntimeArtifactId, RuntimeEventId, RuntimeRunId,
    SessionId, TaskId, ToolCallId, ToolId,
};

/// Kernel-enforced operating mode for one Engineering Runtime session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineeringSessionMode {
    /// Read-only evidence-backed assistance.
    Ask,
    /// Read-only plan construction and approval.
    Plan,
    /// Authorized single-agent effects and verification.
    Agent,
    /// Deterministically coordinated isolated multi-agent execution.
    Team,
}

/// Truthful terminal state shared by sessions, tasks, and campaigns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EngineeringTerminalState {
    /// Every required deterministic postcondition passed.
    Success,
    /// The requested state already held and was deterministically verified.
    NoOp,
    /// A declared prerequisite prevents progress.
    Blocked,
    /// The user declined required authority.
    Declined,
    /// No-progress ceilings were reached.
    Stalled,
    /// A declared resource or attempt budget was exhausted.
    Exhausted,
    /// An effect may have occurred and cannot be reconciled safely.
    Uncertain,
    /// Cancellation and owned cleanup completed.
    Cancelled,
    /// A non-retryable implementation or dependency failure occurred.
    Failed,
}

/// Origin of exact bytes supplied to Verified Chat.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactSourceKind {
    /// Exact plain-text clipboard capture from the AgentMage composer.
    Paste,
    /// Explicit local file selection.
    File,
    /// Explicit current-editor selection.
    EditorSelection,
    /// Output from one terminal tool observation.
    ToolOutput,
    /// Runtime-generated report or derivative artifact.
    Generated,
}

/// Terminal ingestion disposition for one supplied artifact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactCaptureDisposition {
    /// Original bytes were retained exactly and hash-verified.
    CapturedExactly,
    /// A parser produced a complete derivative representation.
    ParsedSuccessfully,
    /// Exact bytes remain available through bounded retrieval.
    AvailableByExactRetrieval,
    /// A parser returned a bounded partial result with warnings.
    PartiallyParsed,
    /// The media type has no enabled parser.
    Unsupported,
    /// The source is encrypted or locked.
    EncryptedOrLocked,
    /// Current policy denied capture or parsing.
    DeniedByPolicy,
    /// A symbolic or native reference could not be resolved exactly.
    FailedToResolve,
    /// The source was omitted with an explicit visible reason.
    OmittedWithVisibleReason,
}

/// One bounded ordered chunk in an authenticated artifact transfer.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactUploadChunk {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact upload attempt.
    pub upload_id: ArtifactUploadId,
    /// Owning durable session.
    pub session_id: SessionId,
    /// Stable zero-based chunk sequence.
    pub sequence: u32,
    /// Exact byte offset of the first byte in this chunk.
    pub offset: u64,
    /// Declared total source bytes.
    pub total_bytes: u64,
    /// Exact chunk bytes.
    pub bytes: Vec<u8>,
    /// SHA-256 of `bytes`.
    pub chunk_sha256: String,
    /// Whether this is the final declared chunk.
    pub final_chunk: bool,
}

/// Host-authoritative terminal receipt for one captured source artifact.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactCaptureResult {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact upload attempt.
    pub upload_id: ArtifactUploadId,
    /// Immutable artifact identity.
    pub artifact_id: RuntimeArtifactId,
    /// Owning session.
    pub session_id: SessionId,
    /// Original source kind.
    pub source_kind: ArtifactSourceKind,
    /// Bounded display name.
    pub display_name: String,
    /// Exact media type.
    pub media_type: String,
    /// Exact authoritative byte count.
    pub byte_length: u64,
    /// Exact line count when meaningful.
    pub line_count: Option<u64>,
    /// SHA-256 of authoritative source bytes.
    pub source_sha256: String,
    /// Terminal ingestion disposition.
    pub disposition: ArtifactCaptureDisposition,
    /// Content-addressed deduplication never changes logical identity.
    pub payload_deduplicated: bool,
    /// Trusted completion time in Unix epoch milliseconds.
    pub completed_at_epoch_ms: u64,
    /// Stable visible warnings without source content.
    pub warning_codes: Vec<String>,
    /// Digest of this receipt with this field zeroed before sealing.
    pub receipt_sha256: String,
}

/// Exact bounded retrieval receipt for one source or derivative artifact.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRangeReceipt {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact artifact read.
    pub artifact_id: RuntimeArtifactId,
    /// Authoritative source digest.
    pub source_sha256: String,
    /// Requested starting byte.
    pub requested_offset: u64,
    /// Requested maximum bytes.
    pub requested_length: u64,
    /// Actual starting byte.
    pub returned_offset: u64,
    /// Exact returned bytes.
    pub bytes: Vec<u8>,
    /// Digest of returned bytes.
    pub returned_sha256: String,
    /// Whether source bytes remain beyond this result.
    pub truncated: bool,
}

/// Model-access state for one source artifact in one turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactCoverageState {
    /// Captured but not admitted to this turn.
    Captured,
    /// Exact content was placed inline.
    AdmittedInline,
    /// Exact retrieval was available but not invoked.
    RetrievalAvailable,
    /// Required exact ranges were retrieved.
    Retrieved,
    /// Only declared ranges were examined.
    PartiallyExamined,
    /// The source was not examined.
    NotExamined,
    /// No qualified parser or modality was available.
    Unsupported,
    /// Policy blocked delivery.
    Blocked,
}

/// Context coverage and provenance for one source artifact.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextArtifactCoverage {
    /// Exact source artifact.
    pub artifact_id: RuntimeArtifactId,
    /// Source digest.
    pub source_sha256: String,
    /// Exact source bytes.
    pub source_bytes: u64,
    /// Current coverage state.
    pub state: ArtifactCoverageState,
    /// Inclusive-exclusive admitted or retrieved byte ranges.
    pub ranges: Vec<(u64, u64)>,
    /// Stable omission, truncation, or parser warnings.
    pub reason_codes: Vec<String>,
}

/// Host-authored proof of what one exact model request could observe.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextDeliveryReceipt {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact context packet.
    pub context_packet_id: ContextPacketId,
    /// Exact owning runtime run.
    pub run_id: RuntimeRunId,
    /// Exact model profile.
    pub model_profile_id: ModelProfileId,
    /// Exact endpoint profile.
    pub endpoint_profile_id: EndpointProfileId,
    /// Exact route decision.
    pub route_decision_id: RouteDecisionId,
    /// Ordered source coverage records.
    pub artifacts: Vec<ContextArtifactCoverage>,
    /// Exact inline-context bytes.
    pub inline_bytes: u64,
    /// Estimated admitted tokens.
    pub estimated_tokens: u64,
    /// Context ceiling applied to this turn.
    pub token_limit: u64,
    /// Digest of the exact model-visible context packet.
    pub context_sha256: String,
    /// Digest of this receipt with this field zeroed before sealing.
    pub receipt_sha256: String,
}

/// Privacy and network class for one exact model endpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointClass {
    /// Inference remains on the current device with no inference egress.
    StrictLocal,
    /// User-controlled private local-network endpoint.
    LocalNetworkPrivate,
    /// User or organization-controlled remote deployment.
    RemotePrivate,
    /// Third-party managed open-weight endpoint.
    RemoteManaged,
}

/// Supported model transport protocol family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointProtocol {
    /// Native llama.cpp process protocol.
    LlamaCppNative,
    /// Ollama native HTTP API.
    Ollama,
    /// OpenAI Chat Completions compatible API.
    OpenAiChat,
    /// OpenAI Responses compatible API.
    OpenAiResponses,
    /// Anthropic Messages compatible API.
    AnthropicMessages,
    /// Hugging Face Text Generation Inference.
    Tgi,
    /// SGLang HTTP protocol.
    Sglang,
    /// Ray Serve LLM protocol profile.
    RayServe,
    /// KServe inference protocol profile.
    Kserve,
    /// Separately gated Docker Model Runner compatibility path.
    DockerModelRunner,
}

/// Exact endpoint identity and privacy contract excluding raw credentials.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelEndpointProfile {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable endpoint profile identity.
    pub endpoint_profile_id: EndpointProfileId,
    /// Route privacy class.
    pub class: EndpointClass,
    /// Wire protocol.
    pub protocol: EndpointProtocol,
    /// Exact operator identity.
    pub operator: String,
    /// Exact deployment owner.
    pub deployment_owner: String,
    /// Normalized base endpoint without embedded credentials.
    pub base_url: String,
    /// Ordered exact allowed host names.
    pub allowed_hosts: Vec<String>,
    /// Whether redirects are prohibited.
    pub redirects_denied: bool,
    /// Whether ambient proxies are prohibited.
    pub ambient_proxies_denied: bool,
    /// Exact model identity and revision.
    pub model_id: String,
    /// Typed credential-broker reference, never a secret value.
    pub credential_reference: Option<String>,
    /// Required region or `local`.
    pub region: String,
    /// Stable logging-policy classification.
    pub logging_policy: String,
    /// Stable retention-policy classification.
    pub retention_policy: String,
    /// Stable training-use classification.
    pub training_use_policy: String,
    /// Maximum admitted context tokens.
    pub max_context_tokens: u64,
    /// Maximum requested output tokens.
    pub max_output_tokens: u64,
    /// Exact qualification revision.
    pub qualification_sha256: String,
    /// Whether current qualification admits use.
    pub qualified: bool,
    /// Digest of this profile with this field zeroed before sealing.
    pub profile_sha256: String,
}

/// Deterministic visible record of one model route and any proposed fallback.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRouteDecision {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable route decision.
    pub route_decision_id: RouteDecisionId,
    /// Owning session.
    pub session_id: SessionId,
    /// Exact selected model.
    pub model_profile_id: ModelProfileId,
    /// Exact selected endpoint.
    pub endpoint_profile_id: EndpointProfileId,
    /// Selected endpoint class.
    pub class: EndpointClass,
    /// Stable policy reason.
    pub reason_code: String,
    /// Whether this decision followed a failed earlier route.
    pub fallback: bool,
    /// Prior endpoint only for explicit fallback review.
    pub prior_endpoint_profile_id: Option<EndpointProfileId>,
    /// Whether a user-visible disclosure was required and accepted.
    pub disclosure_accepted: bool,
    /// Digest of deterministic route policy.
    pub policy_sha256: String,
    /// Digest of this decision with this field zeroed before sealing.
    pub decision_sha256: String,
}

/// Terminal outcome for one admitted tool call.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolObservationTermination {
    /// Tool completed with a successful result.
    Succeeded,
    /// Tool reached a deterministic failure.
    Failed,
    /// Policy or authority denied dispatch.
    Denied,
    /// Cancellation and cleanup completed.
    Cancelled,
    /// The time ceiling terminated execution.
    TimedOut,
    /// An effect cannot be reconciled safely.
    Uncertain,
}

/// Complete durable terminal observation for exactly one tool call.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolObservation {
    /// Contract schema version.
    pub schema_version: u16,
    /// Owning session.
    pub session_id: SessionId,
    /// Owning task.
    pub task_id: TaskId,
    /// Owning runtime run.
    pub run_id: RuntimeRunId,
    /// Exact calling agent.
    pub agent_id: ActorId,
    /// Exact tool call.
    pub tool_call_id: ToolCallId,
    /// Tool identity and version.
    pub tool_id: ToolId,
    /// Exact tool version.
    pub tool_version: String,
    /// Digest of validated arguments.
    pub arguments_sha256: String,
    /// Consumed grant when an effect was admitted.
    pub grant_id: Option<GrantId>,
    /// Trusted start time.
    pub started_at_epoch_ms: u64,
    /// Trusted completion time.
    pub completed_at_epoch_ms: u64,
    /// Closed terminal classification.
    pub termination: ToolObservationTermination,
    /// Process exit code where applicable.
    pub exit_code: Option<i32>,
    /// Process signal where applicable.
    pub signal: Option<i32>,
    /// Exact stdout artifact where output exists.
    pub stdout_artifact_id: Option<RuntimeArtifactId>,
    /// Exact stderr artifact where output exists.
    pub stderr_artifact_id: Option<RuntimeArtifactId>,
    /// Total stdout bytes before any display truncation.
    pub stdout_bytes: u64,
    /// Total stderr bytes before any display truncation.
    pub stderr_bytes: u64,
    /// Whether the display projection omitted bytes.
    pub display_truncated: bool,
    /// Generated artifact identities.
    pub generated_artifact_ids: Vec<RuntimeArtifactId>,
    /// Modified object identity digests, never ambient paths.
    pub modified_object_sha256: Vec<String>,
    /// Whether the complete owned process tree was cleaned.
    pub process_tree_clean: bool,
    /// Stable retry classification.
    pub retry_code: String,
    /// Canonical effect receipt where one exists.
    pub receipt_id: Option<ReceiptId>,
    /// Digest of this observation with this field zeroed before sealing.
    pub observation_sha256: String,
}

/// Versioned executable capability manifest without authority-bearing values.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityManifest {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable capability identity.
    pub capability_id: CapabilityId,
    /// Semantic manifest version.
    pub version: String,
    /// User-visible name.
    pub display_name: String,
    /// Input schema digest.
    pub input_schema_sha256: String,
    /// Output schema digest.
    pub output_schema_sha256: String,
    /// Closed required tool identities.
    pub required_tools: Vec<ToolId>,
    /// Closed required model role codes.
    pub required_model_roles: Vec<String>,
    /// Closed required authority-class codes.
    pub authority_classes: Vec<String>,
    /// Stable ordered workflow-step identities.
    pub workflow_steps: Vec<String>,
    /// Stable deterministic postcondition identities.
    pub postconditions: Vec<String>,
    /// Explicit approval-point identities.
    pub approval_points: Vec<String>,
    /// Maximum runtime turns.
    pub max_turns: u32,
    /// Maximum concurrent workers requested by this capability.
    pub max_workers: u8,
    /// Digest of the immutable manifest with this field zeroed before sealing.
    pub manifest_sha256: String,
}

/// Persistent lifecycle state of one worker lease.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentLeaseState {
    /// Dependencies and resources permit admission.
    Ready,
    /// Exact paths and test resources are held.
    Leased,
    /// Worker is executing in its isolated worktree.
    Implementing,
    /// Deterministic local gates are running.
    Gating,
    /// Independent review is active.
    Reviewing,
    /// Owning worker is correcting structured findings.
    Correcting,
    /// Candidate passed all required gates and review.
    MergeReady,
    /// Candidate waits for serialized integration.
    IntegrationQueued,
    /// Candidate is being integrated.
    Integrating,
    /// Candidate was integrated into the campaign branch.
    Merged,
    /// A prerequisite prevents progress.
    Blocked,
    /// Review results require human reconciliation.
    Disputed,
    /// Worker failed terminally.
    Failed,
    /// Worker cancellation completed.
    Cancelled,
}

/// Exact bounded authority and isolation assignment for one Team worker.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentLease {
    /// Contract schema version.
    pub schema_version: u16,
    /// Owning campaign.
    pub campaign_id: CampaignId,
    /// Stable lease identity.
    pub lease_id: AgentLeaseId,
    /// Exact task.
    pub task_id: TaskId,
    /// Exact worker identity.
    pub agent_id: ActorId,
    /// Exact worker session.
    pub session_id: SessionId,
    /// Exact model profile selected by policy.
    pub model_profile_id: ModelProfileId,
    /// Exact endpoint profile selected by policy.
    pub endpoint_profile_id: EndpointProfileId,
    /// Immutable base commit.
    pub base_commit: String,
    /// AgentMage-owned worktree identity.
    pub worktree_id: String,
    /// AgentMage-owned branch.
    pub branch: String,
    /// Exact authorized path prefixes.
    pub path_leases: Vec<String>,
    /// Exact shared test-resource leases.
    pub test_resource_leases: Vec<String>,
    /// Current lifecycle state.
    pub state: AgentLeaseState,
    /// Maximum correction attempts.
    pub correction_limit: u8,
    /// Current correction count.
    pub correction_count: u8,
    /// Current candidate commit where applicable.
    pub candidate_commit: Option<String>,
    /// Digest of this lease with this field zeroed before sealing.
    pub lease_sha256: String,
}

/// Independent review disposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReviewOutcome {
    /// Candidate satisfies the immutable task and required gates.
    Pass,
    /// Structured findings must be corrected by the owning worker.
    ChangesRequired,
    /// Review cannot proceed because required evidence is unavailable.
    Blocked,
    /// Deterministic evidence and reviewer conclusion conflict.
    Disputed,
}

/// One structured immutable independent-review record.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewFinding {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable review identity.
    pub review_id: ReviewId,
    /// Owning campaign and lease.
    pub campaign_id: CampaignId,
    /// Reviewed lease.
    pub lease_id: AgentLeaseId,
    /// Independent reviewer identity.
    pub reviewer_id: ActorId,
    /// Immutable base commit.
    pub base_commit: String,
    /// Immutable candidate commit.
    pub candidate_commit: String,
    /// Review outcome.
    pub outcome: ReviewOutcome,
    /// Stable severity code.
    pub severity: String,
    /// Stable finding code.
    pub code: String,
    /// Bounded non-secret finding summary.
    pub summary: String,
    /// Digest of complete reviewed diff.
    pub diff_sha256: String,
    /// Digest of this finding with this field zeroed before sealing.
    pub finding_sha256: String,
}

/// Serialized integration lifecycle state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrationState {
    /// Candidate is queued behind earlier accepted work.
    Queued,
    /// Preconditions are being revalidated against current campaign head.
    Revalidating,
    /// Candidate is being integrated into the campaign branch.
    Integrating,
    /// Candidate and post-integration gates passed.
    Integrated,
    /// A stale base, conflict, or failed gate blocked integration.
    Blocked,
    /// Integration failed without modifying the campaign head.
    Failed,
}

/// Immutable record for one serialized integration attempt.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntegrationRecord {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable integration identity.
    pub integration_id: IntegrationId,
    /// Owning campaign.
    pub campaign_id: CampaignId,
    /// Candidate lease.
    pub lease_id: AgentLeaseId,
    /// Campaign head before the attempt.
    pub prior_campaign_head: String,
    /// Exact candidate commit.
    pub candidate_commit: String,
    /// Campaign head after successful integration.
    pub resulting_campaign_head: Option<String>,
    /// Current integration state.
    pub state: IntegrationState,
    /// Digest of required post-integration gate evidence.
    pub gate_evidence_sha256: Option<String>,
    /// Stable failure or blocker codes.
    pub reason_codes: Vec<String>,
    /// Digest of this record with this field zeroed before sealing.
    pub integration_sha256: String,
}

/// Persistent campaign lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamCampaignState {
    /// Campaign is being assembled before admission.
    Planned,
    /// At least one dependency-ready task may be leased.
    Ready,
    /// One or more workers are active.
    Running,
    /// Campaign is paused at a safe boundary.
    Paused,
    /// A required prerequisite prevents progress.
    Blocked,
    /// Cancellation and owned cleanup completed.
    Cancelled,
    /// A terminal failure prevents completion.
    Failed,
    /// Every task and final campaign verifier passed.
    Success,
}

/// Complete durable state projection for one Team campaign.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeamCampaign {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable campaign identity.
    pub campaign_id: CampaignId,
    /// Owning coordinator session.
    pub coordinator_session_id: SessionId,
    /// Bounded project objective.
    pub objective: String,
    /// Immutable approved plan identity.
    pub approved_plan_id: String,
    /// Digest of the approved plan.
    pub approved_plan_sha256: String,
    /// AgentMage-owned campaign branch.
    pub campaign_branch: String,
    /// Exact starting commit.
    pub starting_commit: String,
    /// Current campaign head.
    pub campaign_head: String,
    /// Maximum active implementation workers, at most five for this release.
    pub max_workers: u8,
    /// Current campaign state.
    pub state: TeamCampaignState,
    /// Stable task identities in dependency order.
    pub task_ids: Vec<TaskId>,
    /// Current worker leases.
    pub leases: Vec<AgentLease>,
    /// Ordered accepted integration records.
    pub integrations: Vec<IntegrationRecord>,
    /// Campaign-wide blocker and failure codes.
    pub reason_codes: Vec<String>,
    /// Deterministic final evidence where terminal success is claimed.
    pub final_evidence: Vec<EvidenceReference>,
    /// Digest of this campaign projection with this field zeroed before sealing.
    pub campaign_sha256: String,
}

/// Closed durable Engineering Runtime event kind.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
pub enum EngineeringEventKind {
    /// A session was created.
    SessionCreated,
    /// An artifact transfer began.
    ArtifactTransferStarted {
        /// Exact upload attempt.
        upload_id: ArtifactUploadId,
    },
    /// An ordered artifact chunk was committed.
    ArtifactTransferProgress {
        /// Exact upload attempt.
        upload_id: ArtifactUploadId,
        /// Total exact bytes committed so far.
        bytes: u64,
    },
    /// An artifact became exact and durable.
    ArtifactCaptured {
        /// Exact immutable artifact.
        artifact_id: RuntimeArtifactId,
    },
    /// Context was admitted for one model turn.
    ContextAdmitted {
        /// Exact admitted context packet.
        context_packet_id: ContextPacketId,
    },
    /// A deterministic model route was selected.
    ModelRouteSelected {
        /// Exact deterministic route decision.
        route_decision_id: RouteDecisionId,
    },
    /// One terminal tool observation became durable.
    ToolObserved {
        /// Exact terminal tool call.
        tool_call_id: ToolCallId,
    },
    /// One deterministic verification completed.
    VerificationCompleted {
        /// Deterministically established result.
        terminal: EngineeringTerminalState,
    },
    /// A Team worker entered a new lease state.
    WorkerUpdated {
        /// Exact worker lease.
        lease_id: AgentLeaseId,
        /// New persistent lease state.
        state: AgentLeaseState,
    },
    /// A serialized integration state changed.
    IntegrationUpdated {
        /// Exact serialized integration attempt.
        integration_id: IntegrationId,
        /// New persistent integration state.
        state: IntegrationState,
    },
    /// A task or campaign reached a truthful terminal state.
    Terminal {
        /// Truthful terminal state.
        state: EngineeringTerminalState,
    },
}

/// Ordered, replayable, correlation-bound event for Verified Chat clients.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineeringEvent {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable event identity.
    pub event_id: RuntimeEventId,
    /// Owning session.
    pub session_id: SessionId,
    /// Owning task where applicable.
    pub task_id: Option<TaskId>,
    /// Exact event sequence within the session.
    pub sequence: u64,
    /// Correlation identity for the originating operation.
    pub correlation_id: CorrelationId,
    /// Trusted wall-clock time.
    pub occurred_at_epoch_ms: u64,
    /// Closed event payload.
    pub kind: EngineeringEventKind,
    /// Digest of previous event or zeroes for sequence zero.
    pub previous_event_sha256: String,
    /// Digest of this event with this field zeroed before sealing.
    pub event_sha256: String,
}

/// Reconstructable Rust-owned projection for one Verified Chat session.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineeringSessionSnapshot {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable session identity.
    pub session_id: SessionId,
    /// User-visible bounded title.
    pub title: String,
    /// Kernel-enforced mode.
    pub mode: EngineeringSessionMode,
    /// Selected model profile when one is admitted.
    pub model_profile_id: Option<ModelProfileId>,
    /// Selected endpoint profile when one is admitted.
    pub endpoint_profile_id: Option<EndpointProfileId>,
    /// Current active task when one exists.
    pub active_task_id: Option<TaskId>,
    /// Captured artifact receipts in stable order.
    pub artifacts: Vec<ArtifactCaptureResult>,
    /// Last durable event sequence, or `None` before the first event.
    pub last_event_sequence: Option<u64>,
    /// Current terminal state only after a terminal event.
    pub terminal: Option<EngineeringTerminalState>,
    /// Digest of the complete snapshot with this field zeroed before sealing.
    pub snapshot_sha256: String,
}

#[cfg(test)]
mod tests {
    use super::{ArtifactSourceKind, EngineeringSessionMode, EngineeringTerminalState};

    #[test]
    fn operating_modes_and_terminal_states_are_closed() {
        assert_eq!(EngineeringSessionMode::Ask, EngineeringSessionMode::Ask);
        assert_ne!(EngineeringSessionMode::Team, EngineeringSessionMode::Agent);
        assert_eq!(
            EngineeringTerminalState::NoOp,
            EngineeringTerminalState::NoOp
        );
        assert_ne!(
            EngineeringTerminalState::Uncertain,
            EngineeringTerminalState::Success
        );
        assert_eq!(ArtifactSourceKind::Paste, ArtifactSourceKind::Paste);
    }
}
