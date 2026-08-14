//! Candidate-neutral model profile, runtime, stream, and proposal contracts.

use crate::{
    CancellationSignal, ContextPacketId, ContractError, ContractPayload, CorrelationId,
    ModelAdapterId, ModelCodecId, ModelManifestId, ModelMessageId, ModelProfileId, ModelRunId,
    ModelStreamId, PlatformArchitecture, PlatformFamily, ProposalId, SchemaReference, SessionId,
    TaskId, ToolCallId, ToolCatalogId, ToolId,
};

/// Functional role for which a model profile may be evaluated independently.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ModelRole {
    /// Conversational response and explanation.
    Dialogue,
    /// Repository-aware coding and planning.
    CodingPlanner,
    /// Selection of one typed tool candidate.
    ToolSelection,
    /// Advisory safety classification with no authority.
    SafetyClassification,
    /// Retrieval embedding generation.
    Embedding,
    /// Retrieval reranking.
    Reranking,
    /// Multimodal interpretation.
    Multimodal,
}

/// Input or output modality declared by an exact profile.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ModelModality {
    /// UTF-8 text.
    Text,
    /// Image input.
    Image,
    /// Audio input or output.
    Audio,
    /// Embedding vectors.
    Embedding,
}

/// Lifecycle state of one exact profile rather than an entire model family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelLifecycleState {
    /// Profile is recorded but has not completed admission.
    Candidate,
    /// Exact profile may run only in its declared isolated evidence scope.
    Evaluating,
    /// Exact profile passed every required gate for its declared capabilities.
    Approved,
    /// Exact profile is supported only within recorded limitations.
    Degraded,
    /// Profile was isolated after identity or behavior drift.
    Quarantined,
    /// Profile failed a mandatory gate and remains historical evidence.
    Rejected,
    /// Profile is retained as evidence but cannot be selected for new work.
    Retired,
}

/// Runtime adapter family behind the common local model contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelRuntimeKind {
    /// Deterministic test-only adapter.
    DeterministicFake,
    /// Native pinned `llama.cpp` process.
    NativeLlamaCpp,
    /// Separately gated Docker Model Runner compatibility process.
    DockerModelRunner,
    /// Apple Silicon Metal `llama.cpp` process.
    MacosMetalLlamaCpp,
}

/// Availability state observed from one runtime process.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelHealthState {
    /// Adapter process is not running.
    Unloaded,
    /// Adapter is starting or verifying an artifact.
    Loading,
    /// Exact selected profile is ready for a bounded request.
    Ready,
    /// Adapter remains alive but cannot accept a request.
    Degraded,
    /// Adapter is quarantined and cannot be selected.
    Quarantined,
    /// Adapter terminated or failed its health contract.
    Failed,
}

/// Terminal disposition of one model run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelRunTerminalState {
    /// Structured candidate bytes were returned and closed by kernel validation.
    Proposed,
    /// Bounded plain text was returned without a closed proposal.
    AdvisoryText,
    /// The caller cancelled the run.
    Cancelled,
    /// The run exceeded its exact deadline.
    TimedOut,
    /// A declared resource ceiling was exceeded.
    ResourceExhausted,
    /// The runtime or codec failed without a usable result.
    Failed,
    /// The response was malformed, partial, stale, or replayed.
    Rejected,
}

/// Role of one bounded message supplied to the model edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelMessageRole {
    /// Fixed system contract.
    System,
    /// User-authored request content.
    User,
    /// Prior model advisory content.
    Assistant,
    /// Typed tool result content.
    Tool,
}

/// Closed non-authoritative proposal class returned by a family codec.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelProposalKind {
    /// Bounded advisory text only.
    Text,
    /// Request for additional evidence.
    EvidenceRequest,
    /// Request for one separately validated tool call.
    ToolCall,
    /// Request for an explicit user answer or decision.
    UserQuestion,
    /// Visible report that the model cannot continue.
    Blocked,
    /// Candidate claim that acceptance may be satisfied.
    CompletionCandidate,
}

/// Capability state for one exact role and profile tuple.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelCapabilityState {
    /// No applicable evaluation has run.
    NotEvaluated,
    /// Evaluation is blocked by a named prerequisite.
    Blocked,
    /// Profile failed the role's mandatory threshold.
    Failed,
    /// Profile passed the exact declared role threshold.
    Passed,
    /// Role does not apply to this profile.
    NotApplicable,
}

/// Exact immutable model artifact and its first-party provenance.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelArtifact {
    /// Stable artifact identity.
    pub artifact_id: String,
    /// First-party publisher or controller.
    pub publisher: String,
    /// Pinned upstream revision.
    pub source_revision: String,
    /// Artifact format such as GGUF.
    pub format: String,
    /// Exact byte length.
    pub bytes: u64,
    /// Lowercase SHA-256 digest of the artifact bytes.
    pub sha256: String,
}

/// One attributable transformation between upstream and runtime artifacts.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelTransformation {
    /// Stable transformation identity.
    pub transformation_id: String,
    /// Exact tool and version.
    pub tool: String,
    /// Lowercase SHA-256 digest of canonical arguments.
    pub arguments_sha256: String,
    /// Lowercase SHA-256 input artifact digest.
    pub input_sha256: String,
    /// Lowercase SHA-256 output artifact digest.
    pub output_sha256: String,
    /// Whether independent reproduction has matched the output.
    pub reproducible: bool,
}

/// Exact closed model-family codec identity.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FamilyCodecIdentity {
    /// Stable codec identity.
    pub codec_id: ModelCodecId,
    /// Exact codec version.
    pub codec_version: String,
    /// Lowercase SHA-256 digest of codec source or package bytes.
    pub codec_sha256: String,
    /// Exact tokenizer identity and revision.
    pub tokenizer: String,
    /// Lowercase SHA-256 tokenizer digest.
    pub tokenizer_sha256: String,
    /// Exact chat-template identity.
    pub template: String,
    /// Lowercase SHA-256 template digest.
    pub template_sha256: String,
    /// Exact tool-protocol version.
    pub tool_protocol_version: String,
    /// Ordered end-token identifiers.
    pub end_tokens: Vec<u32>,
    /// Whether reasoning controls are enabled for this tuple.
    pub reasoning_enabled: bool,
}

/// Exact bounded decoding tuple used for a run.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecodingProfile {
    /// Stable decoding profile identity.
    pub profile_id: String,
    /// Ordered sampler names.
    pub sampler_order: Vec<String>,
    /// Temperature represented as a finite non-negative value.
    pub temperature: f32,
    /// Top-p represented as a finite value in the inclusive unit interval.
    pub top_p: f32,
    /// Top-k candidate count.
    pub top_k: u32,
    /// Repeat penalty represented as a finite positive value.
    pub repeat_penalty: f32,
    /// Exact random seed.
    pub seed: u64,
    /// Maximum generated tokens.
    pub max_output_tokens: u32,
}

/// Exact context limits and accounting method.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextBudget {
    /// Maximum context tokens admitted by this profile.
    pub max_context_tokens: u32,
    /// Maximum input bytes before tokenization.
    pub max_input_bytes: u64,
    /// Maximum messages in one packet.
    pub max_messages: u32,
    /// Exact token-accounting implementation identity.
    pub token_counter: String,
    /// Lowercase SHA-256 digest of the token-counter implementation.
    pub token_counter_sha256: String,
}

/// Minimum and maximum hardware/driver envelope for one exact profile.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HardwareEnvelope {
    /// Platform family.
    pub platform: PlatformFamily,
    /// Processor architecture.
    pub architecture: PlatformArchitecture,
    /// Minimum system memory in bytes.
    pub minimum_system_memory_bytes: u64,
    /// Minimum accelerator memory in bytes, or zero for CPU-only profiles.
    pub minimum_accelerator_memory_bytes: u64,
    /// Required acceleration family.
    pub accelerator: String,
    /// Exact driver-version constraint.
    pub driver_constraint: String,
}

/// One role-specific capability and limitation record.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelCapability {
    /// Evaluated role.
    pub role: ModelRole,
    /// Current role disposition.
    pub state: ModelCapabilityState,
    /// Exact evaluation-profile identity when applicable.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub evaluation_profile: Option<String>,
    /// Lowercase SHA-256 result digest when an evaluation ran.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub result_sha256: Option<String>,
    /// Stable limitation codes.
    pub limitations: Vec<String>,
}

/// Complete candidate-neutral identity for one indivisible model tuple.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExactModelProfile {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable exact-profile identity.
    pub profile_id: ModelProfileId,
    /// Stable manifest identity.
    pub manifest_id: ModelManifestId,
    /// Lowercase SHA-256 digest of canonical manifest bytes.
    pub manifest_sha256: String,
    /// Human-readable profile label.
    pub display_name: String,
    /// Model family label used only at the codec edge.
    pub family: String,
    /// Country or jurisdiction controlling the publisher.
    pub publisher_control: String,
    /// Ordered lineage identifiers from source to selected artifact.
    pub lineage: Vec<String>,
    /// SPDX license expression where available.
    pub license_spdx: String,
    /// Lowercase SHA-256 digest of exact license/use terms.
    pub license_terms_sha256: String,
    /// Selected runtime artifact.
    pub artifact: ModelArtifact,
    /// Ordered attributable transformations.
    pub transformations: Vec<ModelTransformation>,
    /// Closed tokenizer/template/tool protocol codec.
    pub codec: FamilyCodecIdentity,
    /// Exact runtime identity expected by this profile.
    pub runtime: ModelRuntimeIdentity,
    /// Quantization identity, including implementation-specific details.
    pub quantization: String,
    /// Declared modalities.
    pub modalities: Vec<ModelModality>,
    /// Exact bounded context profile.
    pub context: ContextBudget,
    /// Exact bounded decoding profile.
    pub decoding: DecodingProfile,
    /// Supported hardware envelopes, each requiring separate evidence.
    pub hardware: Vec<HardwareEnvelope>,
    /// Role-specific capability records.
    pub capabilities: Vec<ModelCapability>,
    /// Lowercase SHA-256 digest of the governing model policy.
    pub policy_sha256: String,
    /// Lifecycle state of this exact tuple.
    pub lifecycle: ModelLifecycleState,
    /// Whether this profile is currently selectable.
    pub enabled: bool,
    /// Whether this profile may ever be selected as an automatic fallback.
    pub automatic_fallback: bool,
}

/// Exact runtime executable and isolation identity.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRuntimeIdentity {
    /// Stable adapter identity.
    pub adapter_id: ModelAdapterId,
    /// Adapter implementation family.
    pub kind: ModelRuntimeKind,
    /// Exact adapter contract version.
    pub contract_version: u16,
    /// Exact runtime build or image identity.
    pub runtime_build: String,
    /// Lowercase SHA-256 runtime build or image digest.
    pub runtime_sha256: String,
    /// Platform family.
    pub platform: PlatformFamily,
    /// Processor architecture.
    pub architecture: PlatformArchitecture,
}

/// Content-free observation of the runtime isolation boundary.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeIsolationObservation {
    /// Exact adapter identity.
    pub adapter_id: ModelAdapterId,
    /// Exact selected profile identity.
    pub profile_id: ModelProfileId,
    /// Whether the process observed a network interface or route.
    pub network_available: bool,
    /// Whether the process observed workspace content or handles.
    pub workspace_available: bool,
    /// Whether the process observed tool or grant material.
    pub authority_material_available: bool,
    /// Whether the process observed credential material.
    pub credential_material_available: bool,
    /// Lowercase SHA-256 digest of the observation procedure and result.
    pub observation_sha256: String,
}

/// Runtime observation used by the kernel to compare an exact manifest tuple.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelManifestObservation {
    /// Exact profile identity observed by the adapter.
    pub profile_id: ModelProfileId,
    /// Exact canonical manifest digest observed by the adapter.
    pub manifest_sha256: String,
    /// Exact artifact digest observed before load.
    pub artifact_sha256: String,
    /// Exact tokenizer digest observed before load.
    pub tokenizer_sha256: String,
    /// Exact template digest observed before load.
    pub template_sha256: String,
    /// Exact codec digest observed before load.
    pub codec_sha256: String,
    /// Exact runtime identity serving the request.
    pub runtime: ModelRuntimeIdentity,
}

/// Content-free result of loading one exact profile.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelLoadReceipt {
    /// Exact profile identity loaded.
    pub profile_id: ModelProfileId,
    /// Exact manifest digest loaded.
    pub manifest_sha256: String,
    /// Exact adapter identity that loaded it.
    pub adapter_id: ModelAdapterId,
    /// Logical load duration in milliseconds.
    pub elapsed_ms: u64,
    /// Content-free isolation observation after load.
    pub isolation: RuntimeIsolationObservation,
}

/// Content-free result of unloading one exact profile.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelUnloadReceipt {
    /// Exact profile identity unloaded.
    pub profile_id: ModelProfileId,
    /// Exact adapter identity that unloaded it.
    pub adapter_id: ModelAdapterId,
    /// Whether no profile remains loaded.
    pub empty: bool,
    /// Logical unload duration in milliseconds.
    pub elapsed_ms: u64,
}

/// Sink for sequential inert response fragments.
pub trait ModelStreamSink {
    /// Accepts one fragment after adapter-side parsing and before proposal validation.
    fn accept(&mut self, fragment: StreamedModelFragment) -> Result<(), ModelRuntimeFailure>;
}

/// Trusted observation point for cancellation that may arrive during inference.
///
/// Implementations expose no wake handle, execution callback, or authority. A
/// runtime may only poll for an immutable cancellation signal and stop work.
pub trait ModelCancellationProbe: Sync {
    /// Returns the current cancellation signal, if one has been requested.
    fn observe(&self) -> Result<Option<CancellationSignal>, ModelRuntimeFailure>;
}

impl ModelCancellationProbe for CancellationSignal {
    fn observe(&self) -> Result<Option<CancellationSignal>, ModelRuntimeFailure> {
        Ok(Some(self.clone()))
    }
}

/// Candidate-neutral local runtime contract implemented by every adapter.
///
/// This interface carries no workspace handle, tool implementation, grant,
/// credential, network destination, automatic-selection method, or completion
/// authority. Returned records remain untrusted observations until the kernel
/// validates them against the exact admitted tuple.
pub trait LocalModelRuntime {
    /// Returns the immutable adapter identity.
    fn identity(&self) -> &ModelRuntimeIdentity;

    /// Observes the exact manifest tuple before load without changing state.
    fn verify_manifest(
        &self,
        profile: &ExactModelProfile,
    ) -> Result<ModelManifestObservation, ModelRuntimeFailure>;

    /// Loads exactly one previously admitted profile.
    fn load(
        &mut self,
        profile: &ExactModelProfile,
    ) -> Result<ModelLoadReceipt, ModelRuntimeFailure>;

    /// Unloads exactly the named profile.
    fn unload(
        &mut self,
        profile_id: &ModelProfileId,
    ) -> Result<ModelUnloadReceipt, ModelRuntimeFailure>;

    /// Returns a content-free health observation.
    fn health(&self) -> ModelHealth;

    /// Counts one exact context packet using the selected profile tokenizer.
    fn count_tokens(
        &self,
        context: &EncodedModelContext,
    ) -> Result<TokenCountResult, ModelRuntimeFailure>;

    /// Runs one bounded request and emits inert sequential fragments.
    fn stream(
        &mut self,
        request: &ModelRunRequest,
        context: &EncodedModelContext,
        cancellation: Option<&dyn ModelCancellationProbe>,
        sink: &mut dyn ModelStreamSink,
    ) -> Result<ModelRunResult, ModelRuntimeFailure>;

    /// Reports content-free bounded resource accounting.
    fn resources(&self) -> Result<ModelResourceReport, ModelRuntimeFailure>;
}

/// Exact inert bytes produced by one family codec for one context packet.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncodedModelContext {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact codec that encoded the packet.
    pub codec_id: ModelCodecId,
    /// Exact profile selected for the packet.
    pub profile_id: ModelProfileId,
    /// Exact context packet identity.
    pub context_packet_id: ContextPacketId,
    /// Encoded prompt bytes with no executable authority.
    pub bytes: Vec<u8>,
    /// Lowercase SHA-256 digest of the encoded bytes.
    pub sha256: String,
}

/// Model-family edge that owns template encoding and closed proposal decoding.
///
/// The codec receives no workspace handle, tool implementation, grant, credential,
/// endpoint, or execution callback. A decoded proposal remains inert and must pass
/// kernel admission before any later operation can be considered.
pub trait ModelFamilyCodec {
    /// Returns the immutable codec identity.
    fn identity(&self) -> &FamilyCodecIdentity;

    /// Encodes one already bounded context packet for one exact profile.
    fn encode_context(
        &self,
        profile: &ExactModelProfile,
        packet: &ModelContextPacket,
    ) -> Result<EncodedModelContext, ModelRuntimeFailure>;

    /// Decodes one complete response into a closed inert proposal.
    fn decode_proposal(
        &self,
        profile: &ExactModelProfile,
        request: &ModelRunRequest,
        response: &[u8],
    ) -> Result<ClosedModelProposal, ModelRuntimeFailure>;
}

/// One bounded message in an exact model context packet.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelMessage {
    /// Stable message identity.
    pub message_id: ModelMessageId,
    /// Message role.
    pub role: ModelMessageRole,
    /// Schema-bound message payload.
    pub content: ContractPayload,
}

/// Bounded context supplied to one exact selected profile.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelContextPacket {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable context packet identity.
    pub context_packet_id: ContextPacketId,
    /// Owning session.
    pub session_id: SessionId,
    /// Owning user-directed task.
    pub task_id: TaskId,
    /// Exact selected profile.
    pub profile_id: ModelProfileId,
    /// Exact selected manifest digest.
    pub manifest_sha256: String,
    /// Frozen tool-catalog identity visible only as definitions.
    pub tool_catalog_id: ToolCatalogId,
    /// Ordered bounded messages.
    pub messages: Vec<ModelMessage>,
    /// Exact accounted input byte total.
    pub input_bytes: u64,
    /// Exact accounted input token total.
    pub input_tokens: u32,
    /// Lowercase SHA-256 digest of canonical packet bytes.
    pub packet_sha256: String,
}

/// Request for one bounded run through one exact profile and adapter.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRunRequest {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable run identity.
    pub model_run_id: ModelRunId,
    /// Correlation identity shared with fragments and result.
    pub correlation_id: CorrelationId,
    /// Exact context packet identity.
    pub context_packet_id: ContextPacketId,
    /// Exact profile identity.
    pub profile_id: ModelProfileId,
    /// Exact manifest digest.
    pub manifest_sha256: String,
    /// Exact runtime adapter identity.
    pub adapter_id: ModelAdapterId,
    /// Exact decoding profile identity.
    pub decoding_profile_id: String,
    /// Maximum output tokens.
    pub max_output_tokens: u32,
    /// Logical timeout in milliseconds.
    pub timeout_ms: u64,
}

/// One ordered fragment from a bounded response stream.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StreamedModelFragment {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable stream identity.
    pub stream_id: ModelStreamId,
    /// Owning run identity.
    pub model_run_id: ModelRunId,
    /// Correlation identity from the request.
    pub correlation_id: CorrelationId,
    /// Zero-based contiguous sequence number.
    pub sequence: u32,
    /// Exact fragment bytes; no fragment is executable.
    pub bytes: Vec<u8>,
    /// Lowercase SHA-256 digest of the fragment bytes.
    pub sha256: String,
    /// Whether this is the declared final fragment.
    pub terminal: bool,
}

/// Inert model-origin candidate for one typed tool call.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelToolCallCandidate {
    /// Candidate tool-call identity.
    pub tool_call_id: ToolCallId,
    /// Exact frozen tool identity.
    pub tool_id: ToolId,
    /// Exact frozen tool version.
    pub tool_version: String,
    /// Schema-bound candidate arguments.
    pub arguments: ContractPayload,
}

/// Closed model-origin wire candidate before trusted identity binding.
///
/// A model can describe only an inert proposal class, optional typed payload,
/// and optional tool request. The family codec, never the model, creates the
/// proposal, run, context, profile, codec, correlation, and digest bindings.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelProposalWireCandidate {
    /// Contract schema version.
    pub schema_version: u16,
    /// Closed proposal class.
    pub kind: ModelProposalKind,
    /// Optional schema-bound proposal payload.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub payload: Option<ContractPayload>,
    /// Optional model-origin tool request without a trusted call identity.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub tool_call: Option<ModelToolCallWireCandidate>,
}

/// Inert model-origin wire candidate for one typed tool request.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelToolCallWireCandidate {
    /// Exact frozen tool identity requested by the model.
    pub tool_id: ToolId,
    /// Exact frozen tool version requested by the model.
    pub tool_version: String,
    /// Schema-bound candidate arguments.
    pub arguments: ContractPayload,
}

/// Complete closed proposal decoded at a model-family edge.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClosedModelProposal {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable proposal identity.
    pub proposal_id: ProposalId,
    /// Owning model run.
    pub model_run_id: ModelRunId,
    /// Exact context packet used by the run.
    pub context_packet_id: ContextPacketId,
    /// Exact selected profile.
    pub profile_id: ModelProfileId,
    /// Exact selected codec.
    pub codec_id: ModelCodecId,
    /// Correlation identity from the run.
    pub correlation_id: CorrelationId,
    /// Closed proposal class.
    pub kind: ModelProposalKind,
    /// Optional schema-bound proposal payload.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub payload: Option<ContractPayload>,
    /// Optional inert tool-call candidate.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub tool_call: Option<ModelToolCallCandidate>,
    /// Lowercase SHA-256 digest of the canonical complete proposal.
    pub proposal_sha256: String,
}

/// Content-free runtime health observation.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelHealth {
    /// Exact adapter identity.
    pub adapter_id: ModelAdapterId,
    /// Exact profile when loaded.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub profile_id: Option<ModelProfileId>,
    /// Current health state.
    pub state: ModelHealthState,
    /// Stable content-free reason code.
    pub reason_code: String,
    /// Logical observation time in milliseconds.
    pub observed_at_ms: u64,
}

/// Content-free resource observation for one loaded profile or run.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelResourceReport {
    /// Exact adapter identity.
    pub adapter_id: ModelAdapterId,
    /// Exact profile identity.
    pub profile_id: ModelProfileId,
    /// Optional run identity.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub model_run_id: Option<ModelRunId>,
    /// Resident memory in bytes.
    pub resident_memory_bytes: u64,
    /// Accelerator memory in bytes.
    pub accelerator_memory_bytes: u64,
    /// Accounted input tokens.
    pub input_tokens: u32,
    /// Accounted output tokens.
    pub output_tokens: u32,
    /// Logical elapsed duration in milliseconds.
    pub elapsed_ms: u64,
}

/// Exact token-count result before admission of a run.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenCountResult {
    /// Exact profile identity.
    pub profile_id: ModelProfileId,
    /// Exact context packet identity.
    pub context_packet_id: ContextPacketId,
    /// Exact token count.
    pub tokens: u32,
    /// Exact token-counter implementation identity.
    pub counter: String,
    /// Lowercase SHA-256 digest of the counted encoded context bytes.
    pub packet_sha256: String,
}

/// Terminal content-free run result plus an optional inert proposal.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRunResult {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact run identity.
    pub model_run_id: ModelRunId,
    /// Exact stream identity.
    pub stream_id: ModelStreamId,
    /// Correlation identity from the request.
    pub correlation_id: CorrelationId,
    /// Terminal run disposition.
    pub terminal_state: ModelRunTerminalState,
    /// Number of accepted contiguous fragments.
    pub fragment_count: u32,
    /// Lowercase SHA-256 digest of the complete response bytes.
    pub response_sha256: String,
    /// Optional complete inert proposal.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub proposal: Option<ClosedModelProposal>,
    /// Optional typed runtime failure.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub failure: Option<ModelRuntimeFailure>,
    /// Content-free resource report.
    pub resources: ModelResourceReport,
}

/// Typed fail-closed runtime or codec failure.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRuntimeFailure {
    /// Stable content-free failure code.
    pub code: String,
    /// Whether a corrected request may be submitted within its outer budget.
    pub retryable_after_correction: bool,
    /// Whether dependency recovery is required before another request.
    pub dependency_recovery_required: bool,
    /// Optional shared contract error.
    #[serde(deserialize_with = "crate::serialization::deserialize_required_option")]
    pub contract_error: Option<Box<ContractError>>,
}

/// Closed schema references exchanged by a model runtime client.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelClientSchemas {
    /// Exact model-profile schema.
    pub model_profile: SchemaReference,
    /// Bounded model-message schema.
    pub message: SchemaReference,
    /// Role-specific capability schema.
    pub capability: SchemaReference,
    /// Context-packet schema.
    pub context_packet: SchemaReference,
    /// Run-request schema.
    pub run_request: SchemaReference,
    /// Stream-fragment schema.
    pub stream_fragment: SchemaReference,
    /// Closed-proposal schema.
    pub proposal: SchemaReference,
    /// Inert model-origin tool-call schema.
    pub tool_call: SchemaReference,
    /// Existing kernel-owned tool-result schema supplied as model context.
    pub tool_result: SchemaReference,
    /// Run-result schema.
    pub run_result: SchemaReference,
    /// Terminal-claim schema, which must resolve to the closed run-result contract.
    pub terminal_claim: SchemaReference,
    /// Shared correlation-identity schema.
    pub correlation: SchemaReference,
}

#[cfg(test)]
mod tests {
    use super::{
        ClosedModelProposal, ModelLifecycleState, ModelProposalKind, ModelRunTerminalState,
        ModelRuntimeKind,
    };

    #[test]
    fn candidate_runtime_vocabulary_is_closed_and_non_authoritative() {
        assert_eq!(
            ModelRuntimeKind::NativeLlamaCpp,
            ModelRuntimeKind::NativeLlamaCpp
        );
        assert_eq!(
            ModelLifecycleState::Candidate,
            ModelLifecycleState::Candidate
        );
        assert_eq!(ModelProposalKind::ToolCall, ModelProposalKind::ToolCall);
        assert_eq!(
            ModelRunTerminalState::Rejected,
            ModelRunTerminalState::Rejected
        );
        assert!(!std::any::type_name::<ClosedModelProposal>().contains("Grant"));
    }

    #[test]
    fn unknown_proposal_fields_fail_closed() {
        let candidate = r#"{
            "schema_version":2,
            "proposal_id":"proposal-1",
            "model_run_id":"run-1",
            "context_packet_id":"context-1",
            "profile_id":"profile-1",
            "codec_id":"codec-1",
            "correlation_id":"correlation-1",
            "kind":"completion_candidate",
            "payload":null,
            "tool_call":null,
            "proposal_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "grant":true
        }"#;
        assert!(serde_json::from_str::<ClosedModelProposal>(candidate).is_err());
    }
}
