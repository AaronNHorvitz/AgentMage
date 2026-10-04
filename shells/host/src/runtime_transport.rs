//! Caller-neutral transport contract for authenticated shared-runtime clients.

use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use agentmage_kernel_contracts::{
    CancellationId, RuntimeApprovalChallenge, RuntimeApprovalResponse, RuntimeArtifactRef,
    RuntimeEvent, RuntimeEventCursor, RuntimeEventKind, RuntimeOutcome, RuntimeRunId,
    RuntimeRunRequest, SessionId,
};
use agentmage_kernel_engine::context_inspection::ContextInspection;
use agentmage_kernel_engine::job_control::{JobControlDecision, JobControlRequest, JobObservation};
use agentmage_kernel_engine::runtime_artifact::{RuntimeArtifactPage, RuntimeArtifactState};
use agentmage_kernel_engine::runtime_coordinator::seal_runtime_run_request;
use agentmage_kernel_engine::runtime_loop::RuntimeSuspensionPoint;

use crate::coding_action_history::{EndedRunActionHistories, RunActionHistory};
use crate::coding_recoverability::RecoverabilityReport;
use crate::coding_route::RunRouteReceipt;

const PREAUTHORIZATION_SCHEMA_VERSION: u16 = 1;
const MAX_PREAUTHORIZED_PATHS: usize = 64;
const MAX_PREAUTHORIZED_COMMANDS: usize = 32;
const MAX_PREAUTHORIZED_OPERATIONS: u32 = 256;
const MAX_PREAUTHORIZATION_LIFETIME_MS: u64 = 24 * 60 * 60 * 1_000;

/// One exact registered command or validation template admitted by direct user preauthorization.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePreauthorizedCommand {
    /// Exact immutable template identity.
    pub template_id: String,
    /// Exact immutable template version.
    pub template_version: String,
    /// Exact registered template digest.
    pub template_sha256: String,
}

/// Direct-user-approved bounded authority envelope for one local coding session.
///
/// This object is descriptive authority input only. Each admitted operation still requires a
/// fresh exact kernel grant and normal single-use consumption at the Linux effect boundary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSessionPreauthorization {
    /// Closed contract schema.
    pub schema_version: u16,
    /// Stable contract identity.
    pub preauthorization_id: String,
    /// Exact workspace identity selected by the user.
    pub workspace_id: String,
    /// Canonical relative component vectors that may be written.
    pub writable_paths: Vec<Vec<String>>,
    /// Exact registered command or validation templates that may execute.
    pub command_templates: Vec<RuntimePreauthorizedCommand>,
    /// Whether descriptor-bound reads and Git inspection within this workspace are admitted.
    pub allow_workspace_reads: bool,
    /// Inclusive maximum operation grants derived from this contract.
    pub maximum_operations: u32,
    /// Trusted direct-approval time.
    pub approved_at_epoch_ms: u64,
    /// Exclusive contract expiration.
    pub expires_at_epoch_ms: u64,
    /// Trusted revocation time; present contracts are inert.
    pub revoked_at_epoch_ms: Option<u64>,
    /// Canonical digest with this field set to zeroes.
    pub preauthorization_sha256: String,
}

impl RuntimeSessionPreauthorization {
    /// Seals and validates one direct-user-approved contract.
    pub fn seal(mut self) -> Result<Self, RuntimeTransportError> {
        self.schema_version = PREAUTHORIZATION_SCHEMA_VERSION;
        self.preauthorization_sha256 = "0".repeat(64);
        validate_preauthorization(&self)?;
        self.preauthorization_sha256 = canonical_sha256(&self)?;
        Ok(self)
    }

    /// Revalidates shape, digest, current workspace, expiry and revocation.
    pub fn verify(
        &self,
        workspace_id: &str,
        now_epoch_ms: u64,
    ) -> Result<(), RuntimeTransportError> {
        validate_preauthorization(self)?;
        let mut candidate = self.clone();
        candidate.preauthorization_sha256 = "0".repeat(64);
        if canonical_sha256(&candidate)? != self.preauthorization_sha256
            || self.workspace_id != workspace_id
            || now_epoch_ms < self.approved_at_epoch_ms
            || now_epoch_ms >= self.expires_at_epoch_ms
            || self.revoked_at_epoch_ms.is_some()
        {
            return Err(RuntimeTransportError::RequestDenied);
        }
        Ok(())
    }
}

fn validate_preauthorization(
    contract: &RuntimeSessionPreauthorization,
) -> Result<(), RuntimeTransportError> {
    let valid_id = |value: &str| {
        !value.is_empty()
            && value.len() <= 128
            && value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')
            })
    };
    let valid_sha = |value: &str| {
        value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    };
    let valid_component = |value: &str| {
        !value.is_empty()
            && value.len() <= 255
            && !matches!(value, "." | "..")
            && !value.contains('/')
            && !value.contains('\0')
    };
    if contract.schema_version != PREAUTHORIZATION_SCHEMA_VERSION
        || !valid_id(&contract.preauthorization_id)
        || !valid_id(&contract.workspace_id)
        || contract.writable_paths.len() > MAX_PREAUTHORIZED_PATHS
        || contract.command_templates.len() > MAX_PREAUTHORIZED_COMMANDS
        || contract.writable_paths.iter().any(|path| {
            path.is_empty()
                || path.len() > 64
                || path.iter().any(|component| !valid_component(component))
        })
        || contract
            .writable_paths
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        || contract.command_templates.iter().any(|command| {
            !valid_id(&command.template_id)
                || !valid_id(&command.template_version)
                || !valid_sha(&command.template_sha256)
        })
        || contract.command_templates.windows(2).any(|pair| {
            (
                &pair[0].template_id,
                &pair[0].template_version,
                &pair[0].template_sha256,
            ) >= (
                &pair[1].template_id,
                &pair[1].template_version,
                &pair[1].template_sha256,
            )
        })
        || contract.maximum_operations == 0
        || contract.maximum_operations > MAX_PREAUTHORIZED_OPERATIONS
        || contract.approved_at_epoch_ms == 0
        || contract.expires_at_epoch_ms <= contract.approved_at_epoch_ms
        || contract.expires_at_epoch_ms - contract.approved_at_epoch_ms
            > MAX_PREAUTHORIZATION_LIFETIME_MS
        || contract
            .revoked_at_epoch_ms
            .is_some_and(|revoked| revoked < contract.approved_at_epoch_ms)
        || !valid_sha(&contract.preauthorization_sha256)
    {
        return Err(RuntimeTransportError::RequestDenied);
    }
    Ok(())
}

fn canonical_sha256(value: &impl Serialize) -> Result<String, RuntimeTransportError> {
    let bytes = serde_json::to_vec(value).map_err(|_| RuntimeTransportError::RequestDenied)?;
    let digest: [u8; 32] = Sha256::digest(bytes).into();
    Ok(hex(&digest))
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}

/// Decodes one transport frame or record exactly (Decisions 0135 and 0136):
/// the bytes decode into the types, and their re-encoding, read back as JSON,
/// must equal the bytes read as JSON. A member the types do not name is
/// refused at any position, also where a type's own attributes would ignore
/// it, such as beside the tag of a variant without members. A member repeated
/// in any object is refused, inside a map as well as a struct. Numbers are
/// compared after the same encoding and reading, so a 32-bit value written as
/// its shortest decimal decodes exactly.
#[must_use]
pub fn decode_exact<T>(bytes: &[u8]) -> Option<T>
where
    T: serde::de::DeserializeOwned + Serialize,
{
    let received = serde_json::from_slice::<UniqueMembers>(bytes).ok()?.0;
    let decoded = serde_json::from_slice::<T>(bytes).ok()?;
    let reencoded = serde_json::to_vec(&decoded).ok()?;
    (serde_json::from_slice::<serde_json::Value>(&reencoded).ok()? == received).then_some(decoded)
}

/// The JSON text of `value` with the first member of the object at `pointer`
/// written twice, for tests of exact decoding.
#[cfg(test)]
pub(crate) fn with_first_member_repeated(value: &serde_json::Value, pointer: &str) -> Vec<u8> {
    fn write(value: &serde_json::Value, here: &str, pointer: &str, text: &mut String) {
        match value {
            serde_json::Value::Object(members) => {
                text.push('{');
                for (index, (name, member)) in members.iter().enumerate() {
                    let times = if index == 0 && here == pointer { 2 } else { 1 };
                    for time in 0..times {
                        if index > 0 || time > 0 {
                            text.push(',');
                        }
                        text.push_str(&serde_json::to_string(name).unwrap());
                        text.push(':');
                        write(member, &format!("{here}/{name}"), pointer, text);
                    }
                }
                text.push('}');
            }
            serde_json::Value::Array(items) => {
                text.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        text.push(',');
                    }
                    write(item, &format!("{here}/{index}"), pointer, text);
                }
                text.push(']');
            }
            other => text.push_str(&serde_json::to_string(other).unwrap()),
        }
    }
    let mut text = String::new();
    write(value, "", pointer, &mut text);
    text.into_bytes()
}

/// A JSON value read so that an object naming one member twice is refused
/// (Decision 0136). A JSON value, like a map in the types, keeps only the
/// last of two equal names, so neither can refuse it.
struct UniqueMembers(serde_json::Value);

impl<'de> Deserialize<'de> for UniqueMembers {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(UniqueMembersVisitor).map(Self)
    }
}

struct UniqueMembersVisitor;

impl<'de> serde::de::Visitor<'de> for UniqueMembersVisitor {
    type Value = serde_json::Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value whose objects name each member once")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(serde_json::Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(value.into())
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(value.into())
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E> {
        Ok(serde_json::Number::from_f64(value).map_or(serde_json::Value::Null, Into::into))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(serde_json::Value::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(serde_json::Value::String(value))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(serde_json::Value::Null)
    }

    fn visit_seq<A>(self, mut items: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(UniqueMembers(item)) = items.next_element()? {
            values.push(item);
        }
        Ok(serde_json::Value::Array(values))
    }

    fn visit_map<A>(self, mut members: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut object = serde_json::Map::new();
        while let Some(name) = members.next_key::<String>()? {
            let UniqueMembers(member) = members.next_value()?;
            if object.insert(name, member).is_some() {
                return Err(serde::de::Error::custom("repeated member"));
            }
        }
        Ok(serde_json::Value::Object(object))
    }
}

/// Trusted inputs from one authenticated runtime preparation request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePrepareInput {
    /// Whether the host must reconstruct one exact safe-boundary durable run.
    pub resume: bool,
    /// Explicit session consent to retain exact model exchanges for checked continuity.
    pub record_session: bool,
    /// Development-only probe that installs one intentionally undrained bounded subscriber.
    pub slow_subscriber_probe: bool,
    /// Optional direct-user-approved bounded session authority contract.
    pub preauthorization: Option<RuntimeSessionPreauthorization>,
    /// Exact pre-existing Engineering session for approved-Plan execution, when applicable.
    pub engineering_session_id: Option<SessionId>,
    /// Exact selected profile identity.
    pub profile_id: String,
    /// Digest of the exact picker entry displayed to the user.
    pub expected_entry_sha256: String,
    /// Exact selected local workspace identity.
    pub workspace_id: String,
    /// Selected absolute local workspace root, consumed only by the trusted host factory.
    pub workspace_root: String,
    /// Bounded user objective retained as inert task input.
    pub prompt: String,
    /// A recipe the run is held to (Decision 0133): the host instantiates
    /// its plan against the workspace's validation templates and the tool
    /// boundary refuses each write outside it. Absent for most runs, and
    /// then not encoded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe: Option<crate::coding_recipe::RuntimeRecipeRequest>,
}

/// One verified coordinator boundary returned to a transport-only client.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeTransportStep {
    /// Exact active runtime run.
    pub run_id: RuntimeRunId,
    /// Digest of the exact admitted runtime request.
    pub request_sha256: String,
    /// Ordered verified events after the caller's supplied cursor.
    pub events: Vec<RuntimeEvent>,
    /// Complete verified artifact-reference set currently owned by the coordinator.
    pub artifacts: Vec<RuntimeArtifactRef>,
    /// Exact protected challenge only while the coordinator is waiting.
    pub approval: Option<RuntimeApprovalChallenge>,
    /// Canonical outcome only after terminal completion.
    pub outcome: Option<RuntimeOutcome>,
    /// Where the run stopped, only while it is suspended at a committed safe
    /// boundary (Decision 0122).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suspended: Option<RuntimeSuspensionPoint>,
}

/// The request that continues `request`'s run from the checkpoint whose
/// commit event is at `cursor` (Decision 0122): the same run and base request,
/// bound to that cursor. Host and client derive it independently.
pub fn resumed_run_request(
    request: &RuntimeRunRequest,
    cursor: &RuntimeEventCursor,
) -> Result<RuntimeRunRequest, RuntimeTransportError> {
    if cursor.run_id != request.run_id {
        return Err(RuntimeTransportError::RequestDenied);
    }
    let mut resumed = request.clone();
    resumed.event_cursor = Some(cursor.clone());
    seal_runtime_run_request(resumed).map_err(|_| RuntimeTransportError::RequestDenied)
}

/// Whether `event` is the commit event of the checkpoint the run stopped at.
pub fn is_suspension_event(event: Option<&RuntimeEvent>, point: &RuntimeSuspensionPoint) -> bool {
    let Some(event) = event else {
        return false;
    };
    event.run_id == point.event_cursor.run_id
        && event.event_id == point.event_cursor.event_id
        && event.sequence == point.event_cursor.sequence
        && event.event_sha256 == point.event_cursor.event_sha256
        && matches!(
            &event.kind,
            RuntimeEventKind::CheckpointCommitted {
                checkpoint_id,
                checkpoint_sha256,
            } if *checkpoint_id == point.checkpoint_id
                && *checkpoint_sha256 == point.checkpoint_sha256
        )
}

/// Current run declarations schema: schema 2 adds each run's action
/// histories (Decision 0127), schema 3 its model route receipt and route
/// history (Decision 0128), schema 4 the recipe plan it was held to
/// (Decision 0133), and schema 5 the recoverability of its whole session
/// (Decision 0143).
pub const RUN_DECLARATIONS_SCHEMA_VERSION: u16 = 5;

/// Host declarations about one ended run, read before the run is released
/// (Decision 0116). They describe the run and grant nothing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRunDeclarations {
    /// Declarations schema version.
    pub schema_version: u16,
    /// Exact ended run.
    pub run_id: RuntimeRunId,
    /// Digest of the exact admitted runtime request.
    pub request_sha256: String,
    /// Recoverability of this run's effects, absent when the host cannot
    /// declare it completely.
    pub recoverability: Option<RecoverabilityReport>,
    /// Recoverability of every effect of the run's session, from the stored
    /// records of each of its runs; absent when the host cannot declare the
    /// whole session completely (Decision 0143).
    pub session_recoverability: Option<RecoverabilityReport>,
    /// Content-free view of each context composed for a model call, in order,
    /// absent when the host cannot show every one.
    pub context_inspections: Option<Vec<ContextInspection>>,
    /// Action history of the run's tool calls, file writes and commands, kept
    /// by the tool boundary; absent when it could not keep every entry.
    pub effect_history: Option<RunActionHistory>,
    /// Action history of the run's job control, kept by the host service;
    /// absent when the service cannot declare every decided request.
    pub job_control_history: Option<RunActionHistory>,
    /// The router's receipt for the run's model requests, absent when the
    /// composition was not routed by this host (Decision 0128).
    pub route_receipt: Option<RunRouteReceipt>,
    /// Action history of the run's model routes, kept by the routing owner;
    /// absent with the receipt or when its entry could not be kept.
    pub route_history: Option<RunActionHistory>,
    /// The recipe plan the run's writes were held to, absent when the run
    /// had no recipe (Decision 0133).
    pub recipe_plan: Option<agentmage_kernel_engine::engineering_recipe::RecipePlan>,
}

/// Reconciled control state of one held run's job, replayed from the host's
/// durable job ledger (Decision 0120). The job identity is the run identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeJobStatus {
    /// Status schema version.
    pub schema_version: u16,
    /// Exact held run.
    pub run_id: RuntimeRunId,
    /// Digest of the exact admitted runtime request.
    pub request_sha256: String,
    /// Replayed job state.
    pub job: JobObservation,
}

impl RuntimeJobStatus {
    /// Whether this status is a well-formed answer about exactly this run.
    #[must_use]
    pub fn describes(&self, request: &RuntimeRunRequest) -> bool {
        self.schema_version == 1
            && self.run_id == request.run_id
            && self.request_sha256 == request.request_sha256
            && self.job.job_id == request.run_id.as_str()
            && is_lower_sha256(&self.job.head_sha256)
    }
}

/// The host's decision on one job control request and the job state after it
/// (Decision 0120).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeJobControl {
    /// Decision the durable ledger recorded, or its original decision for a
    /// retry from the same client.
    pub decision: JobControlDecision,
    /// Job state after the decision.
    pub status: RuntimeJobStatus,
}

impl RuntimeJobControl {
    /// Whether this is a well-formed answer to `control` about exactly this run.
    #[must_use]
    pub fn answers(&self, request: &RuntimeRunRequest, control: &JobControlRequest) -> bool {
        let decided = match self.decision {
            JobControlDecision::Applied { revision, .. } => revision,
            JobControlDecision::Refused { revision, .. } => revision,
        };
        control.job_id == request.run_id.as_str()
            && self.status.describes(request)
            && decided <= self.status.job.revision
    }
}

/// The authenticated client a job control request came from (Decision 0120).
///
/// Only the serving host derives it, from the peer its IPC endpoint
/// authenticated; a client never names a scope. Code outside this crate cannot
/// construct one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeClientScope(String);

impl RuntimeClientScope {
    pub(crate) const fn derived(scope: String) -> Self {
        Self(scope)
    }

    /// The scope under which the job ledger records the client's requests.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Stable content-free refusal from a shared runtime transport boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTransportError {
    /// The preparation or runtime request was invalid, stale, or substituted.
    RequestDenied,
    /// The selected run is absent or no longer active.
    RunUnavailable,
    /// The supplied event cursor does not identify the exact current stream.
    EventCursorDenied,
    /// The supplied exact cursor is older than the live service replay window.
    EventCursorExpired,
    /// The approval response did not match the one pending challenge.
    ApprovalDenied,
    /// The reusable runtime failed closed.
    RuntimeFailed,
    /// The coordinator exposed a malformed or inconsistent event/outcome boundary.
    RuntimeEvidenceDenied,
    /// The bounded active/prepared run ceiling was reached.
    CapacityExceeded,
    /// The run has no durable job ledger this caller can use (Decision 0120).
    JobControlUnavailable,
    /// The run's recipe could not be instantiated for its workspace
    /// (Decision 0133).
    RecipeDenied,
}

impl RuntimeTransportError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::RequestDenied => "host.runtime.request_denied",
            Self::RunUnavailable => "host.runtime.run_unavailable",
            Self::EventCursorDenied => "host.runtime.event_cursor_denied",
            Self::EventCursorExpired => "host.runtime.event_cursor_expired",
            Self::ApprovalDenied => "host.runtime.approval_denied",
            Self::RuntimeFailed => "host.runtime.failed",
            Self::RuntimeEvidenceDenied => "host.runtime.evidence_denied",
            Self::CapacityExceeded => "host.runtime.capacity_exceeded",
            Self::JobControlUnavailable => "host.runtime.job_control_unavailable",
            Self::RecipeDenied => "host.runtime.recipe_denied",
        }
    }
}

impl fmt::Display for RuntimeTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for RuntimeTransportError {}

/// Host-owned runtime operations exposed to one authenticated local client.
pub trait RuntimeTransportPort {
    /// Frames one exact request from current trusted profile, workspace, policy, and task state.
    fn prepare(
        &mut self,
        input: RuntimePrepareInput,
    ) -> Result<RuntimeRunRequest, RuntimeTransportError>;

    /// Starts one exact previously framed request and returns its first coordinator boundary.
    fn start(
        &mut self,
        request: RuntimeRunRequest,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError>;

    /// Advances or replays one exact run from a verified event cursor.
    fn advance(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        after_event_cursor: Option<&RuntimeEventCursor>,
        response: Option<&RuntimeApprovalResponse>,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError>;

    /// Cancels one exact run and returns its truthful terminal boundary.
    fn cancel(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
        cancellation_id: CancellationId,
        after_event_cursor: Option<&RuntimeEventCursor>,
    ) -> Result<RuntimeTransportStep, RuntimeTransportError>;

    /// Reads one bounded verified artifact page from an active terminal run.
    fn read_artifact_page(
        &mut self,
        _run_id: &RuntimeRunId,
        _request_sha256: &str,
        _reference: &RuntimeArtifactRef,
        _offset: u64,
        _maximum_bytes: u32,
    ) -> Result<RuntimeArtifactPage, RuntimeTransportError> {
        Err(RuntimeTransportError::RequestDenied)
    }

    /// Releases one exact terminal-run artifact through its canonical lifecycle owner.
    fn release_artifact(
        &mut self,
        _run_id: &RuntimeRunId,
        _request_sha256: &str,
        _reference: &RuntimeArtifactRef,
    ) -> Result<RuntimeArtifactState, RuntimeTransportError> {
        Err(RuntimeTransportError::RequestDenied)
    }

    /// Reads the host's declarations about one ended run before it is released
    /// (Decision 0116).
    fn run_declarations(
        &mut self,
        _run_id: &RuntimeRunId,
        _request_sha256: &str,
    ) -> Result<RuntimeRunDeclarations, RuntimeTransportError> {
        Err(RuntimeTransportError::RequestDenied)
    }

    /// Reads the stored action histories of one ended run from the host's
    /// operational store (Decision 0129); the catalog host serves it
    /// (Decision 0130).
    fn ended_run_action_histories(
        &mut self,
        _run_id: &RuntimeRunId,
    ) -> Result<EndedRunActionHistories, RuntimeTransportError> {
        Err(RuntimeTransportError::RequestDenied)
    }

    /// Answers one documentation pack request of the catalog host
    /// (Decision 0130).
    fn doc_pack(
        &mut self,
        _request: crate::coding_doc_packs::DocPackRequest,
    ) -> Result<crate::coding_doc_packs::DocPackAnswer, RuntimeTransportError> {
        Err(RuntimeTransportError::RequestDenied)
    }

    /// Answers one memory request of the catalog host (Decision 0131).
    fn memory(
        &mut self,
        _request: crate::coding_memory::MemoryRequest,
    ) -> Result<crate::coding_memory::MemoryAnswer, RuntimeTransportError> {
        Err(RuntimeTransportError::RequestDenied)
    }

    /// Answers one extension request of the catalog host (Decision 0132).
    fn extension(
        &mut self,
        _request: crate::coding_extensions::ExtensionRequest,
    ) -> Result<crate::coding_extensions::ExtensionAnswer, RuntimeTransportError> {
        Err(RuntimeTransportError::RequestDenied)
    }

    /// Answers one route grant request of the catalog host (Decision 0144).
    fn route_grant(
        &mut self,
        _request: crate::coding_route_grants::RouteGrantRequest,
    ) -> Result<crate::coding_route_grants::RouteGrantAnswer, RuntimeTransportError> {
        Err(RuntimeTransportError::RequestDenied)
    }

    /// Answers one folder admission request of the standalone evidence host
    /// (Decision 0150).
    fn folder(
        &mut self,
        _request: crate::standalone_folder::FolderRequest,
    ) -> Result<crate::standalone_folder::FolderAnswer, RuntimeTransportError> {
        Err(RuntimeTransportError::RequestDenied)
    }

    /// Reads the reconciled control state of one held run's job from the
    /// durable job ledger (Decision 0120).
    fn job_status(
        &mut self,
        _run_id: &RuntimeRunId,
        _request_sha256: &str,
    ) -> Result<RuntimeJobStatus, RuntimeTransportError> {
        Err(RuntimeTransportError::JobControlUnavailable)
    }

    /// Asks the serving host to decide one control of a held run's job. The
    /// host records it under the client it authenticated; the client names no
    /// scope (Decision 0120).
    fn control_job(
        &mut self,
        _run_id: &RuntimeRunId,
        _request_sha256: &str,
        _request: &JobControlRequest,
    ) -> Result<RuntimeJobControl, RuntimeTransportError> {
        Err(RuntimeTransportError::JobControlUnavailable)
    }

    /// Decides one job control request for the client scope the serving owner
    /// derived from its authenticated peer. Only a job owner implements it.
    fn control_job_for_client(
        &mut self,
        _client: &RuntimeClientScope,
        _run_id: &RuntimeRunId,
        _request_sha256: &str,
        _request: &JobControlRequest,
    ) -> Result<RuntimeJobControl, RuntimeTransportError> {
        Err(RuntimeTransportError::JobControlUnavailable)
    }

    /// Revokes one exact direct-user session preauthorization before any later operation.
    fn revoke_session_preauthorization(
        &mut self,
        _session_id: &SessionId,
        _preauthorization_sha256: &str,
    ) -> Result<(), RuntimeTransportError> {
        Err(RuntimeTransportError::RequestDenied)
    }

    /// Discards one unstarted request or removes one already terminal run.
    fn release(
        &mut self,
        run_id: &RuntimeRunId,
        request_sha256: &str,
    ) -> Result<(), RuntimeTransportError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract() -> RuntimeSessionPreauthorization {
        RuntimeSessionPreauthorization {
            schema_version: 1,
            preauthorization_id: "preauthorization-fixture".to_owned(),
            workspace_id: "workspace-fixture".to_owned(),
            writable_paths: vec![vec!["src".to_owned(), "lib.rs".to_owned()]],
            command_templates: vec![RuntimePreauthorizedCommand {
                template_id: "validation-unit".to_owned(),
                template_version: "1.0.0".to_owned(),
                template_sha256: "a".repeat(64),
            }],
            allow_workspace_reads: true,
            maximum_operations: 4,
            approved_at_epoch_ms: 1_000,
            expires_at_epoch_ms: 61_000,
            revoked_at_epoch_ms: None,
            preauthorization_sha256: "0".repeat(64),
        }
        .seal()
        .expect("session contract seals")
    }

    #[test]
    fn bounded_session_preauthorization_is_exact_expiring_and_revocable() {
        let contract = contract();
        contract
            .verify("workspace-fixture", 10_000)
            .expect("exact active contract verifies");

        let mut changed = contract.clone();
        changed.maximum_operations += 1;
        assert_eq!(
            changed.verify("workspace-fixture", 10_000),
            Err(RuntimeTransportError::RequestDenied)
        );
        assert_eq!(
            contract.verify("workspace-other", 10_000),
            Err(RuntimeTransportError::RequestDenied)
        );
        assert_eq!(
            contract.verify("workspace-fixture", 61_000),
            Err(RuntimeTransportError::RequestDenied)
        );

        let mut revoked = contract;
        revoked.revoked_at_epoch_ms = Some(20_000);
        revoked.preauthorization_sha256 = "0".repeat(64);
        let revoked = revoked
            .seal()
            .expect("revoked contract remains well formed");
        assert_eq!(
            revoked.verify("workspace-fixture", 20_000),
            Err(RuntimeTransportError::RequestDenied)
        );
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Sample {
        values: std::collections::BTreeMap<String, u32>,
        items: Vec<SampleItem>,
        ratio: f32,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct SampleItem {
        name: String,
    }

    #[test]
    fn a_member_repeated_in_any_object_is_refused() {
        // Decision 0136 (review F2 of `4bef629b`): the types keep the last of
        // two equal names inside a map, and so does a JSON value, so only
        // reading the frame with unique members refuses the repetition.
        let decode = |text: &str| decode_exact::<Sample>(text.as_bytes());
        let base = r#"{"values":{"a":1,"b":2},"items":[{"name":"x"}],"ratio":0.5}"#;
        assert!(decode(base).is_some());
        let in_map = r#"{"values":{"a":1,"a":2},"items":[{"name":"x"}],"ratio":0.5}"#;
        assert_eq!(
            serde_json::from_str::<Sample>(in_map).unwrap().values["a"],
            2
        );
        assert_eq!(decode(in_map), None);
        for repeated in [
            r#"{"values":{"a":1},"items":[{"name":"x","name":"x"}],"ratio":0.5}"#,
            r#"{"values":{"a":1},"items":[],"ratio":0.5,"ratio":0.5}"#,
            r#"{"values":{"a":1,"b":2,"a":1},"items":[],"ratio":0.5}"#,
        ] {
            assert_eq!(decode(repeated), None, "{repeated}");
        }
        // One name in different objects is not a repetition.
        let distinct = r#"{"values":{"name":1},"items":[{"name":"x"},{"name":"y"}],"ratio":0.5}"#;
        assert!(decode(distinct).is_some());
    }

    #[test]
    fn a_32_bit_value_decodes_exactly_as_the_runtime_writes_it() {
        // Decision 0136 (review F1 of `4bef629b`): a 32-bit value is written
        // as its shortest decimal, which is read back as a different 64-bit
        // number than the 32-bit value widened. The comparison reads both
        // sides from text, so every finite value the runtime writes decodes.
        let sample = |ratio: f32| Sample {
            values: std::collections::BTreeMap::new(),
            items: Vec::new(),
            ratio,
        };
        let mut values = vec![
            0.95,
            0.7,
            0.2,
            1.1,
            0.1,
            0.0,
            1.0,
            f32::MIN_POSITIVE,
            f32::MAX,
            f32::MIN,
            f32::EPSILON,
            1.0e-45,
        ];
        values.extend(
            (0..=u32::MAX)
                .step_by(65_537)
                .map(f32::from_bits)
                .filter(|value| value.is_finite()),
        );
        for value in values {
            let bytes = serde_json::to_vec(&sample(value)).unwrap();
            assert_eq!(
                decode_exact::<Sample>(&bytes),
                Some(sample(value)),
                "{value}"
            );
        }
        // The widened value is not what the frame says, which is what the
        // previous comparison held against the runtime's own frames.
        assert_ne!(
            serde_json::to_value(sample(0.95)).unwrap()["ratio"],
            serde_json::json!(0.95)
        );
        // A number written otherwise than the runtime writes it is refused,
        // even where the types alone read the same 32-bit value from it.
        for other in [
            r#"{"values":{},"items":[],"ratio":0.950000001}"#,
            r#"{"values":{},"items":[],"ratio":1}"#,
        ] {
            assert!(serde_json::from_str::<Sample>(other).is_ok());
            assert_eq!(decode_exact::<Sample>(other.as_bytes()), None, "{other}");
        }
        assert_eq!(
            serde_json::from_str::<Sample>(r#"{"values":{},"items":[],"ratio":0.950000001}"#)
                .unwrap()
                .ratio,
            0.95
        );
    }
}
