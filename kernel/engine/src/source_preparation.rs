//! Production text/log source admission, extraction, retrieval, and context accounting.

use std::collections::{BTreeMap, BTreeSet};

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, ComposedContextPacket, ContextAdmission, ContextItemCandidate,
    ContextItemKind, ContextPacketId, ContextSensitivity, RuntimeArtifactRef,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::context_management::{
    ContextCompositionBudget, ContextCompositionError, compose_context,
};
use crate::model_orchestration_profile::{
    ExactTokenCounterBinding, ModelContextWindowPlan, verify_model_context_window_plan,
};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_SOURCE_ID_BYTES: usize = 128;
const MAX_MEDIA_TYPE_BYTES: usize = 128;
const MAX_SOURCE_COUNT: usize = 256;
const MAX_SOURCE_BYTES: usize = 25 * 1024 * 1024;
const MAX_TOTAL_MEMORY_BYTES: usize = 64 * 1024 * 1024;
const MAX_LINES: usize = 500_000;
const MAX_SECTIONS: usize = 16_384;
const MAX_CONTEXT_RECORDS: usize = 4_096;
const MAX_ADMISSION_OBSERVATIONS: usize = 1_024;
const MAX_QUERY_BYTES: usize = 4_096;
const REPEAT_VISIBLE_LIMIT: usize = 3;

/// Stable fail-closed source preparation result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourcePreparationError {
    /// An identity, digest, media type, bound, or relationship is malformed.
    InvalidInput,
    /// Source content exceeds one declared or fixed resource ceiling.
    ResourceLimit,
    /// The selected encoding cannot decode the complete source.
    InvalidEncoding,
    /// The source is binary or uses an unsupported media family.
    Unsupported,
    /// Cancellation was observed before a complete current extraction was published.
    Cancelled,
    /// The source identity already exists or an expected revision is stale.
    Conflict,
    /// The source does not exist or is no longer available.
    NotFound,
    /// Exact tokenizer or model-context bindings drifted.
    TokenizerMismatch,
    /// Existing deterministic context composition refused the supplied candidates.
    Context(ContextCompositionError),
    /// Canonical hashing failed.
    Serialization,
}

impl SourcePreparationError {
    /// Returns one stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "source.preparation.invalid",
            Self::ResourceLimit => "source.preparation.resource_limit",
            Self::InvalidEncoding => "source.preparation.encoding_invalid",
            Self::Unsupported => "source.preparation.unsupported",
            Self::Cancelled => "source.preparation.cancelled",
            Self::Conflict => "source.preparation.conflict",
            Self::NotFound => "source.preparation.not_found",
            Self::TokenizerMismatch => "source.preparation.tokenizer_mismatch",
            Self::Context(error) => match error {
                ContextCompositionError::InvalidInput => "source.context.invalid",
                ContextCompositionError::ConflictingIdentity => "source.context.identity_conflict",
                ContextCompositionError::EssentialItemExceedsBudget => {
                    "source.context.essential_over_budget"
                }
                ContextCompositionError::SerializationFailed => {
                    "source.context.serialization_failed"
                }
            },
            Self::Serialization => "source.preparation.serialization_failed",
        }
    }
}

impl From<ContextCompositionError> for SourcePreparationError {
    fn from(error: ContextCompositionError) -> Self {
        Self::Context(error)
    }
}

/// Closed initial source family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceMediaFamily {
    /// Plain user-supplied text.
    PlainText,
    /// Diagnostic or application log text.
    Log,
}

/// Closed decoding selection for one exact source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceEncoding {
    /// Detect only a BOM or strict UTF-8; no implicit lossy fallback.
    Auto,
    /// Strict UTF-8, optionally with a UTF-8 BOM.
    Utf8,
    /// Strict little-endian UTF-16.
    Utf16Le,
    /// Strict big-endian UTF-16.
    Utf16Be,
    /// Explicit ISO-8859-1 fallback where every byte maps exactly once.
    Latin1,
}

/// Closed source retention binding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PreparedSourceRetention {
    /// Bytes exist only in this bounded service instance.
    Ephemeral,
    /// Bytes were separately admitted to the existing encrypted runtime-artifact authority.
    PolicyPersisted {
        /// Exact verified path-free artifact reference.
        artifact: RuntimeArtifactRef,
        /// Exact policy revision authorizing persistence.
        policy_sha256: String,
    },
}

/// Closed current lifecycle projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreparedSourceLifecycle {
    /// Current content may be retrieved and considered for context.
    Current,
    /// A replacement or refresh invalidated this revision.
    Stale,
    /// Retention expiry made the content unavailable.
    Expired,
    /// Retention release made the content unavailable.
    Released,
    /// Deletion made the content unavailable.
    Deleted,
}

/// Closed extraction result visible without source bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreparedExtractionState {
    /// Complete admitted content was decoded and indexed.
    Complete,
    /// Deterministic head/tail limits omitted source lines visibly.
    Truncated,
}

/// Closed structural section family for text and log preparation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceSectionKind {
    /// Complete retained extraction root.
    DocumentRoot,
    /// Ordinary contiguous text.
    Text,
    /// Error, exception, panic, or failure cluster.
    Error,
    /// Warning cluster.
    Warning,
    /// Stack-trace cluster.
    StackTrace,
    /// Test-result cluster.
    TestResult,
    /// Timestamp-led log cluster.
    Timestamp,
}

/// Exact extraction ceilings for one source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SourcePreparationLimits {
    /// Maximum admitted input bytes.
    pub max_input_bytes: u64,
    /// Maximum retained normalized UTF-8 bytes.
    pub max_output_bytes: u64,
    /// Maximum source lines retained after head/tail selection.
    pub max_lines: u32,
    /// Maximum structural sections including the root.
    pub max_sections: u32,
    /// Maximum distinct normalized lexical terms.
    pub max_index_terms: u32,
}

impl Default for SourcePreparationLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 25 * 1024 * 1024,
            max_output_bytes: 8 * 1024 * 1024,
            max_lines: 250_000,
            max_sections: 8_192,
            max_index_terms: 250_000,
        }
    }
}

/// Exact request to admit one immutable text or log source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceAdmissionRequest {
    /// Stable logical source identity.
    pub source_id: String,
    /// Exact request identity that supplied the source.
    pub request_id: String,
    /// Content-free source kind such as `paste` or `request_reference`.
    pub source_kind: String,
    /// Digest of protected path, URI, or origin metadata; never the raw value.
    pub protected_origin_sha256: String,
    /// Declared media type.
    pub media_type: String,
    /// Initial format family.
    pub media_family: SourceMediaFamily,
    /// Exact decoding policy.
    pub encoding: SourceEncoding,
    /// Pre-persistence sensitivity classification.
    pub sensitivity: ContextSensitivity,
    /// Explicit persistence or default memory-only binding.
    pub retention: PreparedSourceRetention,
    /// Trusted collection time.
    pub collected_at_epoch_ms: u64,
    /// Exact extraction ceilings.
    pub limits: SourcePreparationLimits,
    /// Complete supplied bytes.
    pub bytes: Vec<u8>,
}

/// One provenance-preserving normalized line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PreparedSourceLine {
    /// Stable line identity derived without the revision source identity.
    pub line_id: String,
    /// One-based original logical line number.
    pub line_number: u64,
    /// First source byte contributing to the line.
    pub source_start_byte: u64,
    /// First source byte after the line including its line terminator when present.
    pub source_end_byte_exclusive: u64,
    /// Exact normalized inert line content after ANSI removal.
    pub content: String,
    /// Digest of normalized line content and source range.
    pub content_sha256: String,
    /// Whether an earlier equal consecutive line represents this line in collapsed renderings.
    pub collapsed_repeat: bool,
}

/// One canonical structural section with immutable source provenance.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PreparedSourceSection {
    /// Stable section identity.
    pub section_id: String,
    /// Optional parent section identity.
    pub parent_section_id: Option<String>,
    /// Stable zero-based order.
    pub ordinal: u32,
    /// Closed structural kind.
    pub kind: SourceSectionKind,
    /// First source byte contributing to the section.
    pub source_start_byte: u64,
    /// First source byte after the section.
    pub source_end_byte_exclusive: u64,
    /// First one-based source line.
    pub start_line: u64,
    /// First one-based line after the section.
    pub end_line_exclusive: u64,
    /// Exact bounded inert normalized content; empty only for the root.
    pub content: String,
    /// Digest of exact section content and provenance.
    pub content_sha256: String,
    /// Exact tokens from the bound counter.
    pub token_count: u32,
}

/// Content-free immutable source manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PreparedSourceManifest {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable source identity.
    pub source_id: String,
    /// Exact request identity.
    pub request_id: String,
    /// Content-free source kind.
    pub source_kind: String,
    /// Protected origin digest.
    pub protected_origin_sha256: String,
    /// Declared media type.
    pub media_type: String,
    /// Initial format family.
    pub media_family: SourceMediaFamily,
    /// Encoding actually used.
    pub encoding: SourceEncoding,
    /// Sensitivity classification.
    pub sensitivity: ContextSensitivity,
    /// Current lifecycle state.
    pub lifecycle: PreparedSourceLifecycle,
    /// Persistence binding.
    pub retention: PreparedSourceRetention,
    /// Exact extraction ceilings used to produce this immutable revision.
    pub limits: SourcePreparationLimits,
    /// Complete supplied byte size.
    pub source_bytes: u64,
    /// Digest of complete supplied bytes.
    pub source_sha256: String,
    /// Digest of retained normalized text.
    pub extraction_sha256: String,
    /// Complete or visibly truncated extraction state.
    pub extraction_state: PreparedExtractionState,
    /// Total decoded line count before head/tail selection.
    pub total_lines: u64,
    /// Retained line count.
    pub retained_lines: u64,
    /// Omitted line count.
    pub omitted_lines: u64,
    /// Exact token-counter identity.
    pub token_counter_id: String,
    /// Exact token-counter implementation digest.
    pub token_counter_sha256: String,
    /// Exact tokenizer digest.
    pub tokenizer_sha256: String,
    /// Monotonic logical revision.
    pub revision: u64,
    /// Trusted collection time.
    pub collected_at_epoch_ms: u64,
    /// Digest of this manifest with this field zeroed.
    pub manifest_sha256: String,
}

/// Complete immutable prepared source projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedSourceView {
    /// Content-free manifest.
    pub manifest: PreparedSourceManifest,
    /// Retained provenance-preserving lines.
    pub lines: Vec<PreparedSourceLine>,
    /// Canonical root and cluster sections.
    pub sections: Vec<PreparedSourceSection>,
}

/// Closed terminal state for one attempted source admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceAdmissionDisposition {
    /// A complete immutable prepared source was published.
    Admitted,
    /// Decoding or media classification proved the supplied bytes unsupported.
    Unsupported,
    /// Cancellation stopped publication.
    Cancelled,
    /// Validation, bounds, identity, or tokenizer checks refused publication.
    Refused,
}

/// Content-free terminal observation retained for every admission attempt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourceAdmissionObservation {
    /// Stable one-based process-local attempt sequence.
    pub sequence: u64,
    /// Requested source identity.
    pub source_id: String,
    /// Requested request identity.
    pub request_id: String,
    /// Complete supplied byte count for this admission call.
    pub source_bytes: u64,
    /// Digest of complete supplied bytes.
    pub source_sha256: String,
    /// Terminal admission disposition.
    pub disposition: SourceAdmissionDisposition,
    /// Stable content-free terminal reason.
    pub reason_code: String,
    /// Digest of this observation with this field zeroed.
    pub observation_sha256: String,
}

/// Verifies one content-free source admission observation.
pub fn verify_source_admission_observation(
    observation: &SourceAdmissionObservation,
) -> Result<(), SourcePreparationError> {
    if observation.sequence == 0
        || !valid_id(&observation.source_id)
        || !valid_id(&observation.request_id)
        || observation.source_bytes as usize > MAX_SOURCE_BYTES
        || !valid_sha256(&observation.source_sha256)
        || !valid_id(&observation.reason_code)
        || !valid_sha256(&observation.observation_sha256)
    {
        return Err(SourcePreparationError::InvalidInput);
    }
    let mut candidate = observation.clone();
    candidate.observation_sha256 = ZERO_SHA256.to_owned();
    if sha256_json(&candidate)? != observation.observation_sha256 {
        return Err(SourcePreparationError::InvalidInput);
    }
    Ok(())
}

/// One exact lexical hit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourceLexicalHit {
    /// Source identity.
    pub source_id: String,
    /// Stable line identity.
    pub line_id: String,
    /// One-based line number.
    pub line_number: u64,
    /// Exact matched normalized text.
    pub content: String,
    /// Byte offset in the normalized line.
    pub match_start: u64,
    /// First normalized line byte after the match.
    pub match_end_exclusive: u64,
    /// Digest binding query, line, and offsets.
    pub hit_sha256: String,
}

/// Growing-log refresh classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogRefreshDisposition {
    /// The new source is a valid append-only successor.
    Appended,
    /// The candidate is shorter than the observed predecessor.
    Truncated,
    /// The candidate changed bytes inside the prior observed prefix.
    Rotated,
    /// The candidate bytes are identical to the predecessor.
    Unchanged,
}

/// Result of comparing and, when valid, admitting one growing-log successor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LogRefreshResult {
    /// Closed comparison result.
    pub disposition: LogRefreshDisposition,
    /// Predecessor source identity.
    pub prior_source_id: String,
    /// Admitted successor identity only for append or unchanged reuse.
    pub current_source_id: Option<String>,
    /// Number of stable prior line identities preserved exactly.
    pub stable_prior_line_count: u64,
    /// Digest binding both source byte identities and the disposition.
    pub refresh_sha256: String,
}

/// Closed complete disposition for one source section in model context.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceContextDisposition {
    /// Complete section content entered the packet.
    Included,
    /// A checked summary entered context in place of complete section content.
    Summarized,
    /// The source extraction visibly retained a deterministic head/tail subset.
    Truncated,
    /// A lower-priority exact duplicate was excluded.
    Duplicate,
    /// Current source lifecycle is stale.
    Stale,
    /// Source extraction is unsupported.
    Unsupported,
    /// Source bytes are unavailable.
    Unavailable,
    /// Disclosure policy forbids this sensitivity.
    Restricted,
    /// Exact source-artifact budget omitted this section.
    Omitted,
}

/// Content-free accounting for one source or section.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourceContextRecord {
    /// Stable source identity.
    pub source_id: String,
    /// Exact prepared-manifest or refused-admission observation digest.
    pub source_revision_sha256: String,
    /// Optional section identity; absent for source-level non-content states.
    pub section_id: Option<String>,
    /// Optional section content digest.
    pub section_sha256: Option<String>,
    /// Exact context disposition.
    pub disposition: SourceContextDisposition,
    /// Stable reason code for every non-included state.
    pub reason_code: Option<String>,
    /// Exact admitted or considered token count.
    pub token_count: u32,
}

/// Complete source-context accounting bound to an existing composed packet and window plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PreparedSourceContextManifest {
    /// Contract schema version.
    pub schema_version: u16,
    /// Exact context manifest identity.
    pub context_manifest_id: String,
    /// Exact model-context plan digest.
    pub context_window_plan_sha256: String,
    /// Exact token counter identity.
    pub token_counter_id: String,
    /// Exact token-counter implementation digest.
    pub token_counter_sha256: String,
    /// Exact tokenizer digest.
    pub tokenizer_sha256: String,
    /// Allocated source-artifact tokens.
    pub allocated_tokens: u32,
    /// Actual included source-artifact tokens.
    pub used_tokens: u32,
    /// Complete canonical source/section accounting.
    pub records: Vec<SourceContextRecord>,
    /// Existing deterministic composed context packet.
    pub packet: ComposedContextPacket,
    /// Digest of this manifest with this field zeroed.
    pub manifest_sha256: String,
}

/// Exact tokenizer implementation supplied by an admitted model profile.
pub trait ExactSourceTokenCounter {
    /// Returns the exact counter/tokenizer binding.
    fn binding(&self) -> ExactTokenCounterBinding;
    /// Counts the exact UTF-8 content without approximation.
    fn count_tokens(&mut self, content: &str) -> Result<u32, SourcePreparationError>;
}

/// Verifies one content-free prepared-source manifest and its immutable digest.
pub fn verify_prepared_source_manifest(
    manifest: &PreparedSourceManifest,
) -> Result<(), SourcePreparationError> {
    if manifest.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_id(&manifest.source_id)
        || !valid_id(&manifest.request_id)
        || !valid_id(&manifest.source_kind)
        || !valid_sha256(&manifest.protected_origin_sha256)
        || manifest.media_type.is_empty()
        || manifest.media_type.len() > MAX_MEDIA_TYPE_BYTES
        || !matches!(
            (manifest.media_family, manifest.media_type.as_str()),
            (SourceMediaFamily::PlainText, "text/plain")
                | (
                    SourceMediaFamily::Log,
                    "text/x-log" | "text/plain" | "application/log"
                )
        )
        || manifest.encoding == SourceEncoding::Auto
        || !valid_manifest_retention(manifest)
        || manifest.source_bytes as usize > MAX_SOURCE_BYTES
        || manifest.source_bytes > manifest.limits.max_input_bytes
        || !valid_sha256(&manifest.source_sha256)
        || !valid_sha256(&manifest.extraction_sha256)
        || !valid_limits(manifest.limits)
        || manifest.retained_lines > manifest.total_lines
        || manifest.retained_lines > u64::from(manifest.limits.max_lines)
        || manifest.omitted_lines != manifest.total_lines - manifest.retained_lines
        || (manifest.extraction_state == PreparedExtractionState::Complete
            && manifest.omitted_lines != 0)
        || (manifest.extraction_state == PreparedExtractionState::Truncated
            && manifest.omitted_lines == 0)
        || !valid_id(&manifest.token_counter_id)
        || !valid_sha256(&manifest.token_counter_sha256)
        || !valid_sha256(&manifest.tokenizer_sha256)
        || manifest.revision == 0
        || manifest.collected_at_epoch_ms == 0
        || !valid_sha256(&manifest.manifest_sha256)
        || manifest_digest(manifest)? != manifest.manifest_sha256
    {
        return Err(SourcePreparationError::InvalidInput);
    }
    Ok(())
}

/// Verifies complete source/section accounting and its existing composed packet.
pub fn verify_prepared_source_context_manifest(
    manifest: &PreparedSourceContextManifest,
) -> Result<(), SourcePreparationError> {
    crate::context_management::verify_composed_context(&manifest.packet)?;
    if manifest.schema_version != CONTRACT_SCHEMA_VERSION
        || !valid_id(&manifest.context_manifest_id)
        || !valid_sha256(&manifest.context_window_plan_sha256)
        || !valid_id(&manifest.token_counter_id)
        || !valid_sha256(&manifest.token_counter_sha256)
        || !valid_sha256(&manifest.tokenizer_sha256)
        || manifest.allocated_tokens != manifest.packet.max_tokens
        || manifest.used_tokens > manifest.allocated_tokens
        || manifest.used_tokens != manifest.packet.used_tokens
        || manifest.packet.token_counter_id != manifest.token_counter_id
        || manifest.records.is_empty()
        || manifest.records.len() > MAX_CONTEXT_RECORDS
        || !valid_sha256(&manifest.manifest_sha256)
    {
        return Err(SourcePreparationError::InvalidInput);
    }
    let mut prior: Option<(&str, Option<&str>)> = None;
    let mut source_level = BTreeSet::new();
    let mut accounted_included_digests = BTreeSet::new();
    let included_items = manifest
        .packet
        .items
        .iter()
        .map(|item| (item.content_sha256.as_str(), item.token_count))
        .collect::<BTreeMap<_, _>>();
    if included_items.len() != manifest.packet.items.len() {
        return Err(SourcePreparationError::InvalidInput);
    }
    for record in &manifest.records {
        if !valid_id(&record.source_id)
            || !valid_sha256(&record.source_revision_sha256)
            || record
                .section_id
                .as_deref()
                .is_some_and(|value| !valid_id(value))
            || record
                .section_sha256
                .as_deref()
                .is_some_and(|value| !valid_sha256(value))
            || record.section_id.is_some() != record.section_sha256.is_some()
            || (record.disposition == SourceContextDisposition::Included)
                == record.reason_code.is_some()
        {
            return Err(SourcePreparationError::InvalidInput);
        }
        let key = (record.source_id.as_str(), record.section_id.as_deref());
        if prior.is_some_and(|value| value >= key) {
            return Err(SourcePreparationError::InvalidInput);
        }
        prior = Some(key);
        if record.section_id.is_none() && !source_level.insert(record.source_id.as_str()) {
            return Err(SourcePreparationError::InvalidInput);
        }
        if record.section_id.is_some()
            && matches!(
                record.disposition,
                SourceContextDisposition::Included | SourceContextDisposition::Summarized
            )
        {
            let digest = record.section_sha256.as_deref().unwrap_or_default();
            if included_items.get(digest) != Some(&record.token_count)
                || !accounted_included_digests.insert(digest)
            {
                return Err(SourcePreparationError::InvalidInput);
            }
        }
    }
    if source_level.is_empty()
        || included_items.keys().copied().collect::<BTreeSet<_>>() != accounted_included_digests
        || manifest.records.iter().any(|record| {
            record.section_id.is_some() && !source_level.contains(record.source_id.as_str())
        })
    {
        return Err(SourcePreparationError::InvalidInput);
    }
    let mut candidate = manifest.clone();
    candidate.manifest_sha256 = ZERO_SHA256.to_owned();
    if sha256_json(&candidate)? != manifest.manifest_sha256 {
        return Err(SourcePreparationError::InvalidInput);
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PreparedSource {
    view: PreparedSourceView,
    source_bytes: Vec<u8>,
    lexical_index: BTreeMap<String, Vec<String>>,
}

/// Bounded runtime-owned source preparation and retrieval service.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourcePreparationService {
    sources: BTreeMap<String, PreparedSource>,
    retained_bytes: usize,
    observations: Vec<SourceAdmissionObservation>,
}

impl SourcePreparationService {
    /// Creates an empty memory-bounded service.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Admits, decodes, normalizes, sections, indexes, and publishes one immutable source.
    pub fn admit(
        &mut self,
        request: SourceAdmissionRequest,
        counter: &mut impl ExactSourceTokenCounter,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<PreparedSourceManifest, SourcePreparationError> {
        if self.observations.len() >= MAX_ADMISSION_OBSERVATIONS {
            return Err(SourcePreparationError::ResourceLimit);
        }
        let source_id = request.source_id.clone();
        let request_id = request.request_id.clone();
        let source_bytes = request.bytes.len() as u64;
        let source_sha256 = sha256(&request.bytes);
        let result = self.admit_inner(request, counter, cancelled);
        self.record_admission(
            source_id,
            request_id,
            source_bytes,
            source_sha256,
            result.as_ref().map(|_| ()).map_err(|error| *error),
        )?;
        result
    }

    fn admit_inner(
        &mut self,
        request: SourceAdmissionRequest,
        counter: &mut impl ExactSourceTokenCounter,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<PreparedSourceManifest, SourcePreparationError> {
        validate_request(&request, self.retained_bytes, self.sources.len())?;
        if self.sources.contains_key(&request.source_id) {
            return Err(SourcePreparationError::Conflict);
        }
        if cancelled() {
            return Err(SourcePreparationError::Cancelled);
        }
        let prepared = prepare(request, counter, cancelled)?;
        self.retained_bytes = self
            .retained_bytes
            .checked_add(prepared.source_bytes.len())
            .ok_or(SourcePreparationError::ResourceLimit)?;
        let manifest = prepared.view.manifest.clone();
        verify_prepared_source_manifest(&manifest)?;
        self.sources.insert(manifest.source_id.clone(), prepared);
        Ok(manifest)
    }

    /// Returns every terminal admission observation in exact attempt order.
    #[must_use]
    pub fn admission_observations(&self) -> Vec<SourceAdmissionObservation> {
        self.observations.clone()
    }

    /// Admits bounded transport chunks without accepting a partially accumulated source.
    pub fn admit_chunks(
        &mut self,
        mut request: SourceAdmissionRequest,
        chunks: impl IntoIterator<Item = Vec<u8>>,
        counter: &mut impl ExactSourceTokenCounter,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<PreparedSourceManifest, SourcePreparationError> {
        if self.observations.len() >= MAX_ADMISSION_OBSERVATIONS {
            return Err(SourcePreparationError::ResourceLimit);
        }
        if !request.bytes.is_empty() {
            self.record_admission(
                request.source_id.clone(),
                request.request_id.clone(),
                request.bytes.len() as u64,
                sha256(&request.bytes),
                Err(SourcePreparationError::InvalidInput),
            )?;
            return Err(SourcePreparationError::InvalidInput);
        }
        for chunk in chunks {
            if cancelled() {
                self.record_admission(
                    request.source_id.clone(),
                    request.request_id.clone(),
                    request.bytes.len() as u64,
                    sha256(&request.bytes),
                    Err(SourcePreparationError::Cancelled),
                )?;
                return Err(SourcePreparationError::Cancelled);
            }
            let next = request
                .bytes
                .len()
                .checked_add(chunk.len())
                .ok_or(SourcePreparationError::ResourceLimit)?;
            if next > MAX_SOURCE_BYTES || next as u64 > request.limits.max_input_bytes {
                let mut observed = request.bytes.clone();
                observed.extend_from_slice(&chunk);
                self.record_admission(
                    request.source_id.clone(),
                    request.request_id.clone(),
                    observed.len() as u64,
                    sha256(&observed),
                    Err(SourcePreparationError::ResourceLimit),
                )?;
                return Err(SourcePreparationError::ResourceLimit);
            }
            request.bytes.extend_from_slice(&chunk);
        }
        self.admit(request, counter, cancelled)
    }

    /// Returns manifests in stable source-identity order.
    #[must_use]
    pub fn manifests(&self) -> Vec<PreparedSourceManifest> {
        self.sources
            .values()
            .map(|source| source.view.manifest.clone())
            .collect()
    }

    /// Returns one immutable prepared source projection without protected origin metadata.
    #[must_use]
    pub fn source(&self, source_id: &str) -> Option<PreparedSourceView> {
        self.sources
            .get(source_id)
            .map(|source| source.view.clone())
    }

    /// Reconstructs one current policy-persisted source from its existing encrypted artifact.
    ///
    /// The caller must obtain `bytes` from the already-authorized runtime-artifact store. This
    /// service never opens a second store, path, or URI and publishes nothing unless the complete
    /// deterministic projection exactly matches the retained manifest.
    pub fn restore_persisted(
        &mut self,
        expected: &PreparedSourceManifest,
        bytes: Vec<u8>,
        counter: &mut impl ExactSourceTokenCounter,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<(), SourcePreparationError> {
        verify_prepared_source_manifest(expected)?;
        if expected.lifecycle != PreparedSourceLifecycle::Current
            || !matches!(
                expected.retention,
                PreparedSourceRetention::PolicyPersisted { .. }
            )
            || self.sources.contains_key(&expected.source_id)
        {
            return Err(SourcePreparationError::Conflict);
        }
        let prepared = prepare(
            SourceAdmissionRequest {
                source_id: expected.source_id.clone(),
                request_id: expected.request_id.clone(),
                source_kind: expected.source_kind.clone(),
                protected_origin_sha256: expected.protected_origin_sha256.clone(),
                media_type: expected.media_type.clone(),
                media_family: expected.media_family,
                encoding: expected.encoding,
                sensitivity: expected.sensitivity,
                retention: expected.retention.clone(),
                collected_at_epoch_ms: expected.collected_at_epoch_ms,
                limits: expected.limits,
                bytes,
            },
            counter,
            cancelled,
        )?;
        if prepared.view.manifest != *expected {
            return Err(SourcePreparationError::Conflict);
        }
        validate_request_memory(
            prepared.source_bytes.len(),
            self.retained_bytes,
            self.sources.len(),
        )?;
        self.retained_bytes = self
            .retained_bytes
            .checked_add(prepared.source_bytes.len())
            .ok_or(SourcePreparationError::ResourceLimit)?;
        self.sources.insert(expected.source_id.clone(), prepared);
        Ok(())
    }

    /// Returns an exact bounded normalized byte range after current-state and digest checks.
    pub fn read_range(
        &self,
        source_id: &str,
        manifest_sha256: &str,
        start: usize,
        end_exclusive: usize,
    ) -> Result<String, SourcePreparationError> {
        let source = self.current(source_id, manifest_sha256)?;
        let text = rendered_text(&source.view.lines);
        if start > end_exclusive
            || end_exclusive > text.len()
            || !text.is_char_boundary(start)
            || !text.is_char_boundary(end_exclusive)
        {
            return Err(SourcePreparationError::InvalidInput);
        }
        Ok(text[start..end_exclusive].to_owned())
    }

    /// Performs deterministic artifact-local lexical retrieval with exact hit provenance.
    pub fn search(
        &self,
        source_id: &str,
        manifest_sha256: &str,
        query: &str,
        maximum_hits: usize,
    ) -> Result<Vec<SourceLexicalHit>, SourcePreparationError> {
        if query.is_empty() || query.len() > MAX_QUERY_BYTES || maximum_hits == 0 {
            return Err(SourcePreparationError::InvalidInput);
        }
        let source = self.current(source_id, manifest_sha256)?;
        let normalized = normalize_query(query);
        if normalized.is_empty() {
            return Err(SourcePreparationError::InvalidInput);
        }
        let terms = lexical_terms(&normalized);
        let mut candidate_ids: Option<BTreeSet<&str>> = None;
        for term in &terms {
            let Some(ids) = source.lexical_index.get(term) else {
                return Ok(Vec::new());
            };
            let current = ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
            candidate_ids = Some(match candidate_ids {
                None => current,
                Some(existing) => existing.intersection(&current).copied().collect(),
            });
        }
        let candidates = candidate_ids.unwrap_or_default();
        let mut hits = Vec::new();
        for line in &source.view.lines {
            if !candidates.contains(line.line_id.as_str()) {
                continue;
            }
            let lowercase = line.content.to_lowercase();
            if let Some(start) = lowercase.find(&normalized) {
                let end = start + normalized.len();
                hits.push(SourceLexicalHit {
                    source_id: source_id.to_owned(),
                    line_id: line.line_id.clone(),
                    line_number: line.line_number,
                    content: line.content.clone(),
                    match_start: u64::try_from(start)
                        .map_err(|_| SourcePreparationError::ResourceLimit)?,
                    match_end_exclusive: u64::try_from(end)
                        .map_err(|_| SourcePreparationError::ResourceLimit)?,
                    hit_sha256: sha256_json(&(
                        source_id,
                        &line.line_id,
                        query,
                        start,
                        end,
                        &line.content_sha256,
                    ))?,
                });
                if hits.len() == maximum_hits {
                    break;
                }
            }
        }
        Ok(hits)
    }

    /// Returns only exact error/warning/stack/test sections from one current log.
    pub fn log_diagnostics(
        &self,
        source_id: &str,
        manifest_sha256: &str,
        maximum_sections: usize,
    ) -> Result<Vec<PreparedSourceSection>, SourcePreparationError> {
        if maximum_sections == 0 {
            return Err(SourcePreparationError::InvalidInput);
        }
        let source = self.current(source_id, manifest_sha256)?;
        Ok(source
            .view
            .sections
            .iter()
            .filter(|section| {
                matches!(
                    section.kind,
                    SourceSectionKind::Error
                        | SourceSectionKind::Warning
                        | SourceSectionKind::StackTrace
                        | SourceSectionKind::TestResult
                )
            })
            .take(maximum_sections)
            .cloned()
            .collect())
    }

    /// Compares and admits an append-only log successor while preserving prior line identities.
    pub fn refresh_log(
        &mut self,
        prior_source_id: &str,
        mut successor: SourceAdmissionRequest,
        counter: &mut impl ExactSourceTokenCounter,
        cancelled: &mut impl FnMut() -> bool,
    ) -> Result<LogRefreshResult, SourcePreparationError> {
        let prior = self
            .sources
            .get(prior_source_id)
            .ok_or(SourcePreparationError::NotFound)?;
        if prior.view.manifest.media_family != SourceMediaFamily::Log
            || successor.media_family != SourceMediaFamily::Log
            || successor.protected_origin_sha256 != prior.view.manifest.protected_origin_sha256
            || successor.source_id == prior_source_id
        {
            return Err(SourcePreparationError::InvalidInput);
        }
        let disposition = if successor.bytes == prior.source_bytes {
            LogRefreshDisposition::Unchanged
        } else if successor.bytes.len() < prior.source_bytes.len() {
            LogRefreshDisposition::Truncated
        } else if !successor.bytes.starts_with(&prior.source_bytes) {
            LogRefreshDisposition::Rotated
        } else {
            LogRefreshDisposition::Appended
        };
        if disposition == LogRefreshDisposition::Appended
            && prior.view.manifest.lifecycle != PreparedSourceLifecycle::Current
        {
            return Err(SourcePreparationError::Conflict);
        }
        let prior_sha256 = prior.view.manifest.source_sha256.clone();
        let prior_total_lines = prior.view.manifest.total_lines;
        let prior_ends_at_line_boundary =
            prior.source_bytes.ends_with(b"\n") || prior.source_bytes.ends_with(b"\r");
        let candidate_sha256 = sha256(&successor.bytes);
        let current_source_id = match disposition {
            LogRefreshDisposition::Appended => {
                successor.collected_at_epoch_ms = successor
                    .collected_at_epoch_ms
                    .max(prior.view.manifest.collected_at_epoch_ms);
                validate_request(&successor, self.retained_bytes, self.sources.len())?;
                if self.sources.contains_key(&successor.source_id) {
                    return Err(SourcePreparationError::Conflict);
                }
                let current = prepare(successor, counter, cancelled)?;
                let current_id = current.view.manifest.source_id.clone();
                self.retained_bytes = self
                    .retained_bytes
                    .checked_add(current.source_bytes.len())
                    .ok_or(SourcePreparationError::ResourceLimit)?;
                self.sources.insert(current_id.clone(), current);
                let prior = self
                    .sources
                    .get_mut(prior_source_id)
                    .ok_or(SourcePreparationError::NotFound)?;
                prior.view.manifest.lifecycle = PreparedSourceLifecycle::Stale;
                prior.view.manifest.revision = prior
                    .view
                    .manifest
                    .revision
                    .checked_add(1)
                    .ok_or(SourcePreparationError::ResourceLimit)?;
                prior.view.manifest.manifest_sha256 = manifest_digest(&prior.view.manifest)?;
                Some(current_id)
            }
            LogRefreshDisposition::Unchanged => Some(prior_source_id.to_owned()),
            LogRefreshDisposition::Truncated | LogRefreshDisposition::Rotated => None,
        };
        let stable_prior_line_count = match disposition {
            LogRefreshDisposition::Unchanged => prior_total_lines,
            LogRefreshDisposition::Appended => {
                prior_total_lines.saturating_sub(u64::from(!prior_ends_at_line_boundary))
            }
            LogRefreshDisposition::Truncated | LogRefreshDisposition::Rotated => 0,
        };
        Ok(LogRefreshResult {
            disposition,
            prior_source_id: prior_source_id.to_owned(),
            current_source_id,
            stable_prior_line_count,
            refresh_sha256: sha256_json(&(
                prior_source_id,
                prior_sha256,
                candidate_sha256,
                disposition,
                stable_prior_line_count,
            ))?,
        })
    }

    /// Marks one current source stale without deleting retained bytes or provenance.
    pub fn mark_stale(
        &mut self,
        source_id: &str,
        expected_manifest_sha256: &str,
    ) -> Result<(), SourcePreparationError> {
        let source = self
            .sources
            .get_mut(source_id)
            .ok_or(SourcePreparationError::NotFound)?;
        if source.view.manifest.manifest_sha256 != expected_manifest_sha256
            || source.view.manifest.lifecycle != PreparedSourceLifecycle::Current
        {
            return Err(SourcePreparationError::Conflict);
        }
        source.view.manifest.lifecycle = PreparedSourceLifecycle::Stale;
        source.view.manifest.revision = source
            .view
            .manifest
            .revision
            .checked_add(1)
            .ok_or(SourcePreparationError::ResourceLimit)?;
        source.view.manifest.manifest_sha256 = manifest_digest(&source.view.manifest)?;
        Ok(())
    }

    /// Releases retained content after an exact current or stale manifest check.
    pub fn release(
        &mut self,
        source_id: &str,
        expected_manifest_sha256: &str,
    ) -> Result<PreparedSourceManifest, SourcePreparationError> {
        let source = self
            .sources
            .get_mut(source_id)
            .ok_or(SourcePreparationError::NotFound)?;
        if source.view.manifest.manifest_sha256 != expected_manifest_sha256
            || !matches!(
                source.view.manifest.lifecycle,
                PreparedSourceLifecycle::Current | PreparedSourceLifecycle::Stale
            )
        {
            return Err(SourcePreparationError::Conflict);
        }
        self.retained_bytes = self
            .retained_bytes
            .saturating_sub(source.source_bytes.len());
        source.source_bytes.clear();
        source.view.lines.clear();
        source.view.sections.clear();
        source.lexical_index.clear();
        source.view.manifest.lifecycle = PreparedSourceLifecycle::Released;
        source.view.manifest.revision = source
            .view
            .manifest
            .revision
            .checked_add(1)
            .ok_or(SourcePreparationError::ResourceLimit)?;
        source.view.manifest.manifest_sha256 = manifest_digest(&source.view.manifest)?;
        Ok(source.view.manifest.clone())
    }

    /// Expires retained content after an exact current or stale manifest check.
    pub fn expire(
        &mut self,
        source_id: &str,
        expected_manifest_sha256: &str,
    ) -> Result<PreparedSourceManifest, SourcePreparationError> {
        self.release(source_id, expected_manifest_sha256)?;
        let source = self
            .sources
            .get_mut(source_id)
            .ok_or(SourcePreparationError::NotFound)?;
        source.view.manifest.lifecycle = PreparedSourceLifecycle::Expired;
        source.view.manifest.manifest_sha256 = manifest_digest(&source.view.manifest)?;
        Ok(source.view.manifest.clone())
    }

    /// Tombstones one already-released source while retaining only content-free lineage.
    pub fn delete(
        &mut self,
        source_id: &str,
        expected_manifest_sha256: &str,
    ) -> Result<PreparedSourceManifest, SourcePreparationError> {
        let source = self
            .sources
            .get_mut(source_id)
            .ok_or(SourcePreparationError::NotFound)?;
        if source.view.manifest.manifest_sha256 != expected_manifest_sha256
            || !matches!(
                source.view.manifest.lifecycle,
                PreparedSourceLifecycle::Released | PreparedSourceLifecycle::Expired
            )
        {
            return Err(SourcePreparationError::Conflict);
        }
        source.view.manifest.lifecycle = PreparedSourceLifecycle::Deleted;
        source.view.manifest.revision = source
            .view
            .manifest
            .revision
            .checked_add(1)
            .ok_or(SourcePreparationError::ResourceLimit)?;
        source.view.manifest.manifest_sha256 = manifest_digest(&source.view.manifest)?;
        Ok(source.view.manifest.clone())
    }

    /// Compiles complete source/section accounting through the existing context manager.
    pub fn compile_context(
        &self,
        context_manifest_id: String,
        packet_id: ContextPacketId,
        plan: &ModelContextWindowPlan,
        maximum_bytes: u64,
        counter: &mut impl ExactSourceTokenCounter,
        disclose_private: bool,
    ) -> Result<PreparedSourceContextManifest, SourcePreparationError> {
        let binding = counter.binding();
        if !valid_id(&context_manifest_id)
            || verify_model_context_window_plan(plan).is_err()
            || plan.token_counter_sha256 != binding.token_counter_sha256
            || plan.tokenizer_sha256 != binding.tokenizer_sha256
            || !valid_id(&binding.token_counter_id)
            || maximum_bytes == 0
        {
            return Err(SourcePreparationError::TokenizerMismatch);
        }
        let mut records = Vec::new();
        let mut candidates = Vec::new();
        let mut content_owners = BTreeMap::new();
        let mut refused_sources = BTreeMap::new();
        for observation in &self.observations {
            if observation.disposition != SourceAdmissionDisposition::Admitted
                && !self.sources.contains_key(&observation.source_id)
            {
                refused_sources.insert(observation.source_id.as_str(), observation);
            }
        }
        for observation in refused_sources.values() {
            let disposition = if observation.disposition == SourceAdmissionDisposition::Unsupported
            {
                SourceContextDisposition::Unsupported
            } else {
                SourceContextDisposition::Unavailable
            };
            records.push(SourceContextRecord {
                source_id: observation.source_id.clone(),
                source_revision_sha256: observation.observation_sha256.clone(),
                section_id: None,
                section_sha256: None,
                disposition,
                reason_code: Some(observation.reason_code.clone()),
                token_count: 0,
            });
        }
        for source in self.sources.values() {
            let manifest = &source.view.manifest;
            if manifest.lifecycle != PreparedSourceLifecycle::Current {
                let (disposition, reason) = if manifest.lifecycle == PreparedSourceLifecycle::Stale
                {
                    (SourceContextDisposition::Stale, "context.source.stale")
                } else {
                    (
                        SourceContextDisposition::Unavailable,
                        "context.source.unavailable",
                    )
                };
                records.push(source_level_record(manifest, disposition, reason));
                records.extend(source.view.sections.iter().map(|section| {
                    section_record(manifest, section, disposition, Some(reason), 0)
                }));
                continue;
            }
            if manifest.sensitivity == ContextSensitivity::Restricted
                || (manifest.sensitivity == ContextSensitivity::Private && !disclose_private)
            {
                records.push(source_level_record(
                    manifest,
                    SourceContextDisposition::Restricted,
                    "context.source.restricted",
                ));
                records.extend(source.view.sections.iter().map(|section| {
                    section_record(
                        manifest,
                        section,
                        SourceContextDisposition::Restricted,
                        Some("context.source.restricted"),
                        0,
                    )
                }));
                continue;
            }
            let (source_disposition, source_reason) =
                if manifest.extraction_state == PreparedExtractionState::Truncated {
                    (
                        SourceContextDisposition::Truncated,
                        Some("context.source.extraction_truncated".to_owned()),
                    )
                } else {
                    (SourceContextDisposition::Included, None)
                };
            records.push(SourceContextRecord {
                source_id: manifest.source_id.clone(),
                source_revision_sha256: manifest.manifest_sha256.clone(),
                section_id: None,
                section_sha256: None,
                disposition: source_disposition,
                reason_code: source_reason,
                token_count: 0,
            });
            if let Some(root) = source
                .view
                .sections
                .iter()
                .find(|section| section.kind == SourceSectionKind::DocumentRoot)
            {
                records.push(section_record(
                    manifest,
                    root,
                    SourceContextDisposition::Omitted,
                    Some("context.section.container"),
                    0,
                ));
            }
            let sections = source
                .view
                .sections
                .iter()
                .filter(|section| section.kind != SourceSectionKind::DocumentRoot)
                .collect::<Vec<_>>();
            if sections.is_empty() {
                records.push(source_level_record(
                    manifest,
                    SourceContextDisposition::Unavailable,
                    "context.source.no_sections",
                ));
                continue;
            }
            for section in sections {
                if records.len() >= MAX_CONTEXT_RECORDS {
                    return Err(SourcePreparationError::ResourceLimit);
                }
                let duplicate = content_owners
                    .insert(section.content_sha256.as_str(), section.section_id.as_str())
                    .is_some();
                if duplicate {
                    records.push(section_record(
                        manifest,
                        section,
                        SourceContextDisposition::Duplicate,
                        Some("context.section.duplicate"),
                        section.token_count,
                    ));
                    continue;
                }
                let observed_tokens = counter.count_tokens(&section.content)?;
                if observed_tokens != section.token_count
                    || manifest.token_counter_id != binding.token_counter_id
                    || manifest.token_counter_sha256 != binding.token_counter_sha256
                    || manifest.tokenizer_sha256 != binding.tokenizer_sha256
                {
                    return Err(SourcePreparationError::TokenizerMismatch);
                }
                let item_id = format!("source-context-{}", &section.section_id[..16]);
                candidates.push(ContextItemCandidate {
                    item_id,
                    kind: ContextItemKind::Evidence,
                    sensitivity: manifest.sensitivity,
                    admission: ContextAdmission::Eligible,
                    authoritative_evidence: true,
                    essential: false,
                    source_id: manifest.source_id.clone(),
                    source_revision: manifest.manifest_sha256.clone(),
                    content_sha256: section.content_sha256.clone(),
                    bounded_excerpt: section.content.clone(),
                    token_count: section.token_count,
                });
                records.push(section_record(
                    manifest,
                    section,
                    SourceContextDisposition::Included,
                    None,
                    section.token_count,
                ));
            }
        }
        let packet = compose_context(
            packet_id,
            &ContextCompositionBudget {
                max_bytes: maximum_bytes,
                max_tokens: plan.source_artifacts.allocated_tokens,
                max_items: u32::try_from(MAX_CONTEXT_RECORDS)
                    .map_err(|_| SourcePreparationError::ResourceLimit)?,
                token_counter_id: binding.token_counter_id.clone(),
            },
            candidates,
        )?;
        let included = packet
            .items
            .iter()
            .map(|item| item.content_sha256.as_str())
            .collect::<BTreeSet<_>>();
        for record in &mut records {
            if record.disposition == SourceContextDisposition::Included
                && record
                    .section_sha256
                    .as_deref()
                    .is_some_and(|digest| !included.contains(digest))
            {
                record.disposition = SourceContextDisposition::Omitted;
                record.reason_code = Some("context.section.budget".to_owned());
            }
        }
        records.sort_by(|left, right| {
            (left.source_id.as_str(), left.section_id.as_deref())
                .cmp(&(right.source_id.as_str(), right.section_id.as_deref()))
        });
        let mut manifest = PreparedSourceContextManifest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            context_manifest_id,
            context_window_plan_sha256: plan.plan_sha256.clone(),
            token_counter_id: binding.token_counter_id,
            token_counter_sha256: binding.token_counter_sha256,
            tokenizer_sha256: binding.tokenizer_sha256,
            allocated_tokens: plan.source_artifacts.allocated_tokens,
            used_tokens: packet.used_tokens,
            records,
            packet,
            manifest_sha256: ZERO_SHA256.to_owned(),
        };
        manifest.manifest_sha256 = sha256_json(&manifest)?;
        verify_prepared_source_context_manifest(&manifest)?;
        Ok(manifest)
    }

    fn current(
        &self,
        source_id: &str,
        manifest_sha256: &str,
    ) -> Result<&PreparedSource, SourcePreparationError> {
        let source = self
            .sources
            .get(source_id)
            .ok_or(SourcePreparationError::NotFound)?;
        if source.view.manifest.manifest_sha256 != manifest_sha256
            || source.view.manifest.lifecycle != PreparedSourceLifecycle::Current
        {
            return Err(SourcePreparationError::Conflict);
        }
        Ok(source)
    }

    fn record_admission(
        &mut self,
        source_id: String,
        request_id: String,
        source_bytes: u64,
        source_sha256: String,
        result: Result<(), SourcePreparationError>,
    ) -> Result<(), SourcePreparationError> {
        if self.observations.len() >= MAX_ADMISSION_OBSERVATIONS {
            return Err(SourcePreparationError::ResourceLimit);
        }
        let (disposition, reason_code) = match result {
            Ok(()) => (SourceAdmissionDisposition::Admitted, "source.admitted"),
            Err(SourcePreparationError::Unsupported | SourcePreparationError::InvalidEncoding) => (
                SourceAdmissionDisposition::Unsupported,
                "source.unsupported",
            ),
            Err(SourcePreparationError::Cancelled) => {
                (SourceAdmissionDisposition::Cancelled, "source.cancelled")
            }
            Err(_) => (SourceAdmissionDisposition::Refused, "source.refused"),
        };
        let sequence = u64::try_from(self.observations.len() + 1)
            .map_err(|_| SourcePreparationError::ResourceLimit)?;
        let mut observation = SourceAdmissionObservation {
            sequence,
            source_id,
            request_id,
            source_bytes,
            source_sha256,
            disposition,
            reason_code: reason_code.to_owned(),
            observation_sha256: ZERO_SHA256.to_owned(),
        };
        observation.observation_sha256 = sha256_json(&observation)?;
        verify_source_admission_observation(&observation)?;
        self.observations.push(observation);
        Ok(())
    }
}

#[derive(Clone)]
struct DecodedScalar {
    character: char,
    start: usize,
    end: usize,
}

fn validate_request(
    request: &SourceAdmissionRequest,
    retained_bytes: usize,
    source_count: usize,
) -> Result<(), SourcePreparationError> {
    let limits = request.limits;
    if !valid_id(&request.source_id)
        || !valid_id(&request.request_id)
        || !valid_id(&request.source_kind)
        || !valid_sha256(&request.protected_origin_sha256)
        || request.media_type.is_empty()
        || request.media_type.len() > MAX_MEDIA_TYPE_BYTES
        || request.collected_at_epoch_ms == 0
        || request.bytes.len() > MAX_SOURCE_BYTES
        || request.bytes.len() as u64 > limits.max_input_bytes
        || !valid_limits(limits)
        || validate_request_memory(request.bytes.len(), retained_bytes, source_count).is_err()
        || !valid_retention(request)
    {
        return Err(SourcePreparationError::InvalidInput);
    }
    let expected_family = match request.media_family {
        SourceMediaFamily::PlainText => request.media_type == "text/plain",
        SourceMediaFamily::Log => matches!(
            request.media_type.as_str(),
            "text/x-log" | "text/plain" | "application/log"
        ),
    };
    if !expected_family {
        return Err(SourcePreparationError::Unsupported);
    }
    Ok(())
}

fn valid_limits(limits: SourcePreparationLimits) -> bool {
    limits.max_input_bytes > 0
        && limits.max_input_bytes as usize <= MAX_SOURCE_BYTES
        && limits.max_output_bytes > 0
        && limits.max_output_bytes as usize <= MAX_SOURCE_BYTES
        && limits.max_lines > 0
        && limits.max_lines as usize <= MAX_LINES
        && limits.max_sections > 0
        && limits.max_sections as usize <= MAX_SECTIONS
        && limits.max_index_terms > 0
        && limits.max_index_terms as usize <= MAX_LINES
}

fn validate_request_memory(
    source_bytes: usize,
    retained_bytes: usize,
    source_count: usize,
) -> Result<(), SourcePreparationError> {
    if source_count >= MAX_SOURCE_COUNT
        || retained_bytes.saturating_add(source_bytes) > MAX_TOTAL_MEMORY_BYTES
    {
        Err(SourcePreparationError::ResourceLimit)
    } else {
        Ok(())
    }
}

fn valid_retention(request: &SourceAdmissionRequest) -> bool {
    match &request.retention {
        PreparedSourceRetention::Ephemeral => true,
        PreparedSourceRetention::PolicyPersisted {
            artifact,
            policy_sha256,
        } => {
            artifact.schema_version == CONTRACT_SCHEMA_VERSION
                && valid_id(artifact.artifact_id.as_str())
                && valid_sha256(&artifact.manifest_sha256)
                && artifact.payload_sha256 == sha256(&request.bytes)
                && artifact.byte_size == request.bytes.len() as u64
                && artifact.media_type == request.media_type
                && valid_sha256(policy_sha256)
        }
    }
}

fn valid_manifest_retention(manifest: &PreparedSourceManifest) -> bool {
    match &manifest.retention {
        PreparedSourceRetention::Ephemeral => true,
        PreparedSourceRetention::PolicyPersisted {
            artifact,
            policy_sha256,
        } => {
            artifact.schema_version == CONTRACT_SCHEMA_VERSION
                && valid_id(artifact.artifact_id.as_str())
                && valid_sha256(&artifact.manifest_sha256)
                && artifact.payload_sha256 == manifest.source_sha256
                && artifact.byte_size == manifest.source_bytes
                && artifact.media_type == manifest.media_type
                && valid_sha256(policy_sha256)
        }
    }
}

fn prepare(
    request: SourceAdmissionRequest,
    counter: &mut impl ExactSourceTokenCounter,
    cancelled: &mut impl FnMut() -> bool,
) -> Result<PreparedSource, SourcePreparationError> {
    let source_sha256 = sha256(&request.bytes);
    let (encoding, decoded) = decode_lines(&request.bytes, request.encoding)?;
    if decoded.iter().any(|line| line.content.contains('\0')) {
        return Err(SourcePreparationError::Unsupported);
    }
    let total_lines = decoded.len();
    let retained_indexes = bounded_head_tail_indexes(
        &decoded,
        request.limits.max_lines as usize,
        request.limits.max_output_bytes as usize,
    );
    let omitted_lines = total_lines.saturating_sub(retained_indexes.len());
    let mut lines = Vec::with_capacity(retained_indexes.len());
    let mut prior_content: Option<String> = None;
    let mut repetition = 0_usize;
    for (position, index) in retained_indexes.into_iter().enumerate() {
        if cancelled() {
            return Err(SourcePreparationError::Cancelled);
        }
        let line = &decoded[index];
        let content = strip_ansi(&line.content);
        if prior_content.as_deref() == Some(content.as_str()) {
            repetition += 1;
        } else {
            repetition = 1;
            prior_content = Some(content.clone());
        }
        let line_number =
            u64::try_from(index + 1).map_err(|_| SourcePreparationError::ResourceLimit)?;
        let line_id = format!(
            "line-{}",
            &sha256_json(&(
                &request.protected_origin_sha256,
                line_number,
                line.start,
                line.end,
                &content,
            ))?[..24]
        );
        lines.push(PreparedSourceLine {
            line_id,
            line_number,
            source_start_byte: u64::try_from(line.start)
                .map_err(|_| SourcePreparationError::ResourceLimit)?,
            source_end_byte_exclusive: u64::try_from(line.end)
                .map_err(|_| SourcePreparationError::ResourceLimit)?,
            content_sha256: sha256_json(&(line.start, line.end, &content))?,
            content,
            collapsed_repeat: repetition > REPEAT_VISIBLE_LIMIT,
        });
        if position >= request.limits.max_lines as usize {
            return Err(SourcePreparationError::ResourceLimit);
        }
    }
    let rendered = rendered_text(&lines);
    let binding = counter.binding();
    if !valid_id(&binding.token_counter_id)
        || !valid_sha256(&binding.token_counter_sha256)
        || !valid_sha256(&binding.tokenizer_sha256)
    {
        return Err(SourcePreparationError::TokenizerMismatch);
    }
    let sections = build_sections(
        &request.protected_origin_sha256,
        &lines,
        counter,
        request.limits.max_sections as usize,
    )?;
    let lexical_index = build_lexical_index(&lines, request.limits.max_index_terms as usize)?;
    let mut manifest = PreparedSourceManifest {
        schema_version: CONTRACT_SCHEMA_VERSION,
        source_id: request.source_id,
        request_id: request.request_id,
        source_kind: request.source_kind,
        protected_origin_sha256: request.protected_origin_sha256,
        media_type: request.media_type,
        media_family: request.media_family,
        encoding,
        sensitivity: request.sensitivity,
        lifecycle: PreparedSourceLifecycle::Current,
        retention: request.retention,
        limits: request.limits,
        source_bytes: request.bytes.len() as u64,
        source_sha256,
        extraction_sha256: sha256(rendered.as_bytes()),
        extraction_state: if omitted_lines == 0 {
            PreparedExtractionState::Complete
        } else {
            PreparedExtractionState::Truncated
        },
        total_lines: total_lines as u64,
        retained_lines: lines.len() as u64,
        omitted_lines: omitted_lines as u64,
        token_counter_id: binding.token_counter_id,
        token_counter_sha256: binding.token_counter_sha256,
        tokenizer_sha256: binding.tokenizer_sha256,
        revision: 1,
        collected_at_epoch_ms: request.collected_at_epoch_ms,
        manifest_sha256: ZERO_SHA256.to_owned(),
    };
    manifest.manifest_sha256 = manifest_digest(&manifest)?;
    Ok(PreparedSource {
        view: PreparedSourceView {
            manifest,
            lines,
            sections,
        },
        source_bytes: request.bytes,
        lexical_index,
    })
}

#[derive(Clone)]
struct DecodedLine {
    start: usize,
    end: usize,
    content: String,
}

fn decode(
    bytes: &[u8],
    requested: SourceEncoding,
) -> Result<(SourceEncoding, Vec<DecodedScalar>), SourcePreparationError> {
    let actual = match requested {
        SourceEncoding::Auto if bytes.starts_with(&[0xff, 0xfe]) => SourceEncoding::Utf16Le,
        SourceEncoding::Auto if bytes.starts_with(&[0xfe, 0xff]) => SourceEncoding::Utf16Be,
        SourceEncoding::Auto | SourceEncoding::Utf8 => SourceEncoding::Utf8,
        explicit => explicit,
    };
    match actual {
        SourceEncoding::Utf8 => decode_utf8(bytes).map(|scalars| (actual, scalars)),
        SourceEncoding::Utf16Le => decode_utf16(bytes, true).map(|scalars| (actual, scalars)),
        SourceEncoding::Utf16Be => decode_utf16(bytes, false).map(|scalars| (actual, scalars)),
        SourceEncoding::Latin1 => Ok((
            actual,
            bytes
                .iter()
                .enumerate()
                .map(|(index, byte)| DecodedScalar {
                    character: char::from(*byte),
                    start: index,
                    end: index + 1,
                })
                .collect(),
        )),
        SourceEncoding::Auto => unreachable!(),
    }
}

fn decode_lines(
    bytes: &[u8],
    requested: SourceEncoding,
) -> Result<(SourceEncoding, Vec<DecodedLine>), SourcePreparationError> {
    let actual = match requested {
        SourceEncoding::Auto if bytes.starts_with(&[0xff, 0xfe]) => SourceEncoding::Utf16Le,
        SourceEncoding::Auto if bytes.starts_with(&[0xfe, 0xff]) => SourceEncoding::Utf16Be,
        SourceEncoding::Auto | SourceEncoding::Utf8 => SourceEncoding::Utf8,
        explicit => explicit,
    };
    if actual == SourceEncoding::Utf8 {
        return decode_utf8_lines(bytes).map(|lines| (actual, lines));
    }
    let (_, scalars) = decode(bytes, actual)?;
    decoded_lines(&scalars, bytes.len()).map(|lines| (actual, lines))
}

fn decode_utf8_lines(bytes: &[u8]) -> Result<Vec<DecodedLine>, SourcePreparationError> {
    let offset = usize::from(bytes.starts_with(&[0xef, 0xbb, 0xbf])) * 3;
    let text = std::str::from_utf8(&bytes[offset..])
        .map_err(|_| SourcePreparationError::InvalidEncoding)?;
    if text.is_empty() {
        return Ok(vec![DecodedLine {
            start: offset,
            end: bytes.len(),
            content: String::new(),
        }]);
    }
    let mut lines = Vec::new();
    let mut line_start = offset;
    let mut content_start = 0_usize;
    let mut iterator = text.char_indices().peekable();
    while let Some((index, character)) = iterator.next() {
        if character != '\r' && character != '\n' {
            continue;
        }
        let mut terminator_end = index + character.len_utf8();
        if character == '\r' && iterator.peek().is_some_and(|(_, next)| *next == '\n') {
            let (next_index, next) = iterator.next().expect("peeked newline");
            terminator_end = next_index + next.len_utf8();
        }
        lines.push(DecodedLine {
            start: line_start,
            end: offset + terminator_end,
            content: text[content_start..index].to_owned(),
        });
        line_start = offset + terminator_end;
        content_start = terminator_end;
        if lines.len() > MAX_LINES {
            return Err(SourcePreparationError::ResourceLimit);
        }
    }
    if content_start < text.len() {
        lines.push(DecodedLine {
            start: line_start,
            end: bytes.len(),
            content: text[content_start..].to_owned(),
        });
    }
    Ok(lines)
}

fn decode_utf8(bytes: &[u8]) -> Result<Vec<DecodedScalar>, SourcePreparationError> {
    let offset = usize::from(bytes.starts_with(&[0xef, 0xbb, 0xbf])) * 3;
    let text = std::str::from_utf8(&bytes[offset..])
        .map_err(|_| SourcePreparationError::InvalidEncoding)?;
    Ok(text
        .char_indices()
        .map(|(index, character)| DecodedScalar {
            character,
            start: offset + index,
            end: offset + index + character.len_utf8(),
        })
        .collect())
}

fn decode_utf16(
    bytes: &[u8],
    little_endian: bool,
) -> Result<Vec<DecodedScalar>, SourcePreparationError> {
    let bom = if little_endian {
        [0xff, 0xfe]
    } else {
        [0xfe, 0xff]
    };
    let offset = usize::from(bytes.starts_with(&bom)) * 2;
    if !(bytes.len() - offset).is_multiple_of(2) {
        return Err(SourcePreparationError::InvalidEncoding);
    }
    let units = bytes[offset..]
        .chunks_exact(2)
        .map(|chunk| {
            if little_endian {
                u16::from_le_bytes([chunk[0], chunk[1]])
            } else {
                u16::from_be_bytes([chunk[0], chunk[1]])
            }
        })
        .collect::<Vec<_>>();
    let mut scalars = Vec::new();
    let mut index = 0_usize;
    while index < units.len() {
        let unit = units[index];
        let start = offset + index * 2;
        if (0xd800..=0xdbff).contains(&unit) {
            let Some(low) = units.get(index + 1).copied() else {
                return Err(SourcePreparationError::InvalidEncoding);
            };
            if !(0xdc00..=0xdfff).contains(&low) {
                return Err(SourcePreparationError::InvalidEncoding);
            }
            let value = 0x10000 + (((unit as u32 - 0xd800) << 10) | (low as u32 - 0xdc00));
            let character = char::from_u32(value).ok_or(SourcePreparationError::InvalidEncoding)?;
            scalars.push(DecodedScalar {
                character,
                start,
                end: start + 4,
            });
            index += 2;
        } else if (0xdc00..=0xdfff).contains(&unit) {
            return Err(SourcePreparationError::InvalidEncoding);
        } else {
            scalars.push(DecodedScalar {
                character: char::from_u32(unit as u32)
                    .ok_or(SourcePreparationError::InvalidEncoding)?,
                start,
                end: start + 2,
            });
            index += 1;
        }
    }
    Ok(scalars)
}

fn decoded_lines(
    scalars: &[DecodedScalar],
    byte_len: usize,
) -> Result<Vec<DecodedLine>, SourcePreparationError> {
    if scalars.is_empty() {
        return Ok(vec![DecodedLine {
            start: 0,
            end: byte_len,
            content: String::new(),
        }]);
    }
    let mut lines = Vec::new();
    let mut content = String::new();
    let mut line_start = scalars[0].start;
    let mut index = 0_usize;
    while index < scalars.len() {
        let scalar = &scalars[index];
        if scalar.character == '\r' || scalar.character == '\n' {
            let mut end = scalar.end;
            if scalar.character == '\r'
                && scalars
                    .get(index + 1)
                    .is_some_and(|next| next.character == '\n')
            {
                index += 1;
                end = scalars[index].end;
            }
            lines.push(DecodedLine {
                start: line_start,
                end,
                content: std::mem::take(&mut content),
            });
            line_start = scalars.get(index + 1).map_or(end, |next| next.start);
        } else {
            content.push(scalar.character);
        }
        index += 1;
    }
    let last_end = scalars.last().map_or(byte_len, |scalar| scalar.end);
    if line_start < last_end || !content.is_empty() {
        lines.push(DecodedLine {
            start: line_start,
            end: last_end,
            content,
        });
    }
    if lines.len() > MAX_LINES {
        return Err(SourcePreparationError::ResourceLimit);
    }
    Ok(lines)
}

fn head_tail_indexes(total: usize, maximum: usize) -> Vec<usize> {
    if total <= maximum {
        return (0..total).collect();
    }
    let head = maximum.div_ceil(2);
    let tail = maximum - head;
    (0..head).chain(total - tail..total).collect()
}

fn bounded_head_tail_indexes(
    lines: &[DecodedLine],
    maximum_lines: usize,
    maximum_bytes: usize,
) -> Vec<usize> {
    let initial = lines.len().min(maximum_lines);
    let fits = |count: usize| {
        head_tail_indexes(lines.len(), count)
            .into_iter()
            .try_fold(0_usize, |used, index| {
                used.checked_add(strip_ansi(&lines[index].content).len().saturating_add(1))
            })
            .is_some_and(|used| used <= maximum_bytes)
    };
    if fits(initial) {
        return head_tail_indexes(lines.len(), initial);
    }
    let mut lower = 0_usize;
    let mut upper = initial;
    while lower < upper {
        let middle = lower + (upper - lower).div_ceil(2);
        if fits(middle) {
            lower = middle;
        } else {
            upper = middle - 1;
        }
    }
    head_tail_indexes(lines.len(), lower)
}

fn strip_ansi(content: &str) -> String {
    let chars = content.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(content.len());
    let mut index = 0_usize;
    while index < chars.len() {
        if chars[index] == '\u{1b}' && chars.get(index + 1) == Some(&'[') {
            let mut end = index + 2;
            while end < chars.len() && !matches!(chars[end], '@'..='~') {
                end += 1;
            }
            if end < chars.len() {
                index = end + 1;
                continue;
            }
        }
        output.push(chars[index]);
        index += 1;
    }
    output
}

fn rendered_text(lines: &[PreparedSourceLine]) -> String {
    let mut output = String::new();
    let mut collapsed = 0_usize;
    for line in lines {
        if line.collapsed_repeat {
            collapsed += 1;
            continue;
        }
        if collapsed > 0 {
            output.push_str(&format!("… {collapsed} repeated line(s) omitted …\n"));
            collapsed = 0;
        }
        output.push_str(&line.content);
        output.push('\n');
    }
    if collapsed > 0 {
        output.push_str(&format!("… {collapsed} repeated line(s) omitted …\n"));
    }
    output
}

fn build_sections(
    protected_origin_sha256: &str,
    lines: &[PreparedSourceLine],
    counter: &mut impl ExactSourceTokenCounter,
    maximum: usize,
) -> Result<Vec<PreparedSourceSection>, SourcePreparationError> {
    if maximum == 0 {
        return Err(SourcePreparationError::ResourceLimit);
    }
    let root_id = format!(
        "section-{}",
        &sha256_json(&(protected_origin_sha256, "root"))?[..24]
    );
    let mut sections = vec![PreparedSourceSection {
        section_id: root_id.clone(),
        parent_section_id: None,
        ordinal: 0,
        kind: SourceSectionKind::DocumentRoot,
        source_start_byte: lines.first().map_or(0, |line| line.source_start_byte),
        source_end_byte_exclusive: lines
            .last()
            .map_or(0, |line| line.source_end_byte_exclusive),
        start_line: lines.first().map_or(1, |line| line.line_number),
        end_line_exclusive: lines.last().map_or(1, |line| line.line_number + 1),
        content: String::new(),
        content_sha256: sha256(rendered_text(lines).as_bytes()),
        token_count: counter.count_tokens(&rendered_text(lines))?,
    }];
    let mut start = 0_usize;
    while start < lines.len() {
        if sections.len() >= maximum {
            return Err(SourcePreparationError::ResourceLimit);
        }
        let kind = classify_line(&lines[start].content);
        let mut end = start + 1;
        while end < lines.len() && classify_line(&lines[end].content) == kind {
            end += 1;
        }
        let group = &lines[start..end];
        let content = rendered_text(group);
        let first = &group[0];
        let last = &group[group.len() - 1];
        let content_sha256 = sha256(content.as_bytes());
        let section_id = format!(
            "section-{}",
            &sha256_json(&(
                protected_origin_sha256,
                kind,
                first.source_start_byte,
                last.source_end_byte_exclusive,
                &content_sha256,
            ))?[..24]
        );
        sections.push(PreparedSourceSection {
            section_id,
            parent_section_id: Some(root_id.clone()),
            ordinal: u32::try_from(sections.len())
                .map_err(|_| SourcePreparationError::ResourceLimit)?,
            kind,
            source_start_byte: first.source_start_byte,
            source_end_byte_exclusive: last.source_end_byte_exclusive,
            start_line: first.line_number,
            end_line_exclusive: last.line_number + 1,
            token_count: counter.count_tokens(&content)?,
            content,
            content_sha256,
        });
        start = end;
    }
    Ok(sections)
}

fn classify_line(content: &str) -> SourceSectionKind {
    let trimmed = content.trim();
    let lower = trimmed.to_lowercase();
    if lower.starts_with("test ") && (lower.contains(" ok") || lower.contains("fail")) {
        SourceSectionKind::TestResult
    } else if lower.contains("error")
        || lower.contains("failed")
        || lower.contains("failure")
        || lower.contains("exception")
        || lower.contains("panic")
    {
        SourceSectionKind::Error
    } else if lower.contains("warning") || lower.starts_with("warn") {
        SourceSectionKind::Warning
    } else if trimmed.starts_with("at ") || trimmed.starts_with("File ") {
        SourceSectionKind::StackTrace
    } else if looks_timestamped(trimmed) {
        SourceSectionKind::Timestamp
    } else {
        SourceSectionKind::Text
    }
}

fn looks_timestamped(content: &str) -> bool {
    let bytes = content.as_bytes();
    bytes.len() >= 10
        && bytes[0..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(u8::is_ascii_digit)
}

fn build_lexical_index(
    lines: &[PreparedSourceLine],
    maximum_terms: usize,
) -> Result<BTreeMap<String, Vec<String>>, SourcePreparationError> {
    let mut index: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for line in lines {
        for term in lexical_terms(&line.content) {
            if !index.contains_key(&term) && index.len() >= maximum_terms {
                return Err(SourcePreparationError::ResourceLimit);
            }
            let ids = index.entry(term).or_default();
            if ids.last() != Some(&line.line_id) {
                ids.push(line.line_id.clone());
            }
        }
    }
    Ok(index)
}

fn lexical_terms(content: &str) -> Vec<String> {
    content
        .split(|character: char| !(character.is_alphanumeric() || character == '_'))
        .filter(|term| !term.is_empty())
        .map(str::to_lowercase)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn normalize_query(query: &str) -> String {
    query
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn source_level_record(
    manifest: &PreparedSourceManifest,
    disposition: SourceContextDisposition,
    reason: &str,
) -> SourceContextRecord {
    SourceContextRecord {
        source_id: manifest.source_id.clone(),
        source_revision_sha256: manifest.manifest_sha256.clone(),
        section_id: None,
        section_sha256: None,
        disposition,
        reason_code: Some(reason.to_owned()),
        token_count: 0,
    }
}

fn section_record(
    manifest: &PreparedSourceManifest,
    section: &PreparedSourceSection,
    disposition: SourceContextDisposition,
    reason: Option<&str>,
    token_count: u32,
) -> SourceContextRecord {
    SourceContextRecord {
        source_id: manifest.source_id.clone(),
        source_revision_sha256: manifest.manifest_sha256.clone(),
        section_id: Some(section.section_id.clone()),
        section_sha256: Some(section.content_sha256.clone()),
        disposition,
        reason_code: reason.map(str::to_owned),
        token_count,
    }
}

fn manifest_digest(manifest: &PreparedSourceManifest) -> Result<String, SourcePreparationError> {
    let mut candidate = manifest.clone();
    candidate.manifest_sha256 = ZERO_SHA256.to_owned();
    sha256_json(&candidate)
}

fn sha256_json(value: &impl Serialize) -> Result<String, SourcePreparationError> {
    serde_json::to_vec(value)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| SourcePreparationError::Serialization)
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_SOURCE_ID_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{ContextPacketId, RuntimeArtifactId};

    use super::*;
    use crate::model_orchestration_profile::{
        AllocatedContextPartition, ContextPartitionDisposition,
    };

    struct WordCounter;

    impl ExactSourceTokenCounter for WordCounter {
        fn binding(&self) -> ExactTokenCounterBinding {
            ExactTokenCounterBinding {
                token_counter_id: "story-22-3-word-counter".to_owned(),
                token_counter_sha256: "c".repeat(64),
                tokenizer_sha256: "d".repeat(64),
            }
        }

        fn count_tokens(&mut self, content: &str) -> Result<u32, SourcePreparationError> {
            u32::try_from(content.split_whitespace().count())
                .map_err(|_| SourcePreparationError::ResourceLimit)
        }
    }

    fn request(
        source_id: &str,
        bytes: Vec<u8>,
        family: SourceMediaFamily,
    ) -> SourceAdmissionRequest {
        SourceAdmissionRequest {
            source_id: source_id.to_owned(),
            request_id: "request-22-3".to_owned(),
            source_kind: "paste".to_owned(),
            protected_origin_sha256: sha256(format!("origin:{source_id}").as_bytes()),
            media_type: match family {
                SourceMediaFamily::PlainText => "text/plain",
                SourceMediaFamily::Log => "text/x-log",
            }
            .to_owned(),
            media_family: family,
            encoding: SourceEncoding::Auto,
            sensitivity: ContextSensitivity::Internal,
            retention: PreparedSourceRetention::Ephemeral,
            collected_at_epoch_ms: 10_000,
            limits: SourcePreparationLimits::default(),
            bytes,
        }
    }

    fn plan(source_tokens: u32) -> ModelContextWindowPlan {
        let partition = AllocatedContextPartition {
            requested_tokens: source_tokens,
            minimum_tokens: 1,
            allocated_tokens: source_tokens,
            disposition: ContextPartitionDisposition::Included,
            reason_code: None,
        };
        let retrieved = AllocatedContextPartition {
            requested_tokens: 0,
            minimum_tokens: 0,
            allocated_tokens: 0,
            disposition: ContextPartitionDisposition::Included,
            reason_code: None,
        };
        let mut value = ModelContextWindowPlan {
            schema_version: CONTRACT_SCHEMA_VERSION,
            model_profile_id: "model-profile-22-3".to_owned(),
            model_manifest_sha256: "1".repeat(64),
            model_runtime_sha256: "2".repeat(64),
            tokenizer_sha256: "d".repeat(64),
            token_counter_sha256: "c".repeat(64),
            total_window_tokens: source_tokens + 5,
            system_and_tool_tokens: 1,
            user_input_tokens: 1,
            source_artifacts: partition,
            retrieved_context: retrieved,
            workflow_recovery_reserve_tokens: 1,
            output_reserve_tokens: 1,
            safety_margin_tokens: 1,
            unallocated_tokens: 0,
            plan_sha256: ZERO_SHA256.to_owned(),
        };
        value.plan_sha256 = sha256_json(&value).expect("plan digest");
        value
    }

    #[test]
    fn story_22_3_text_and_log_preparation_preserves_provenance_and_clusters() {
        let bytes = b"\x1b[31mERROR boom\x1b[0m\nrepeat\nrepeat\nrepeat\nrepeat\nwarning: low\n  at frame\ntest unit ... FAILED\n2026-08-31 ready\n".to_vec();
        let mut service = SourcePreparationService::new();
        let manifest = service
            .admit(
                request("log-main", bytes.clone(), SourceMediaFamily::Log),
                &mut WordCounter,
                &mut || false,
            )
            .expect("log admits");
        assert_eq!(manifest.source_sha256, sha256(&bytes));
        assert_eq!(manifest.extraction_state, PreparedExtractionState::Complete);
        let source = service.source("log-main").expect("source view");
        assert!(
            source
                .lines
                .iter()
                .all(|line| !line.content.contains('\u{1b}'))
        );
        assert_eq!(
            source
                .lines
                .iter()
                .filter(|line| line.collapsed_repeat)
                .count(),
            1
        );
        for kind in [
            SourceSectionKind::Error,
            SourceSectionKind::Warning,
            SourceSectionKind::StackTrace,
            SourceSectionKind::TestResult,
            SourceSectionKind::Timestamp,
        ] {
            assert!(source.sections.iter().any(|section| section.kind == kind));
        }
        let hits = service
            .search("log-main", &manifest.manifest_sha256, "error boom", 10)
            .expect("lexical search");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].line_number, 1);
        let diagnostics = service
            .log_diagnostics("log-main", &manifest.manifest_sha256, 10)
            .expect("diagnostics");
        assert_eq!(diagnostics.len(), 4);
        assert!(
            serde_json::to_string(&manifest)
                .expect("manifest JSON")
                .find("ERROR boom")
                .is_none()
        );
    }

    #[test]
    fn story_22_3_declared_encodings_are_exact_and_binary_or_lossy_input_fails() {
        let mut service = SourcePreparationService::new();
        let text = "alpha β\nerror café\n";
        let mut utf16 = vec![0xff, 0xfe];
        for unit in text.encode_utf16() {
            utf16.extend(unit.to_le_bytes());
        }
        let mut utf16_request = request("utf16-source", utf16, SourceMediaFamily::PlainText);
        utf16_request.encoding = SourceEncoding::Auto;
        let manifest = service
            .admit(utf16_request, &mut WordCounter, &mut || false)
            .expect("UTF-16 admits");
        assert_eq!(manifest.encoding, SourceEncoding::Utf16Le);
        assert_eq!(
            service.source("utf16-source").expect("source").lines[1].content,
            "error café"
        );

        let mut latin = request(
            "latin-source",
            b"caf\xe9\n".to_vec(),
            SourceMediaFamily::PlainText,
        );
        latin.encoding = SourceEncoding::Latin1;
        service
            .admit(latin, &mut WordCounter, &mut || false)
            .expect("declared fallback admits");
        assert_eq!(
            service.source("latin-source").expect("source").lines[0].content,
            "café"
        );

        let invalid = request(
            "invalid-source",
            vec![0xff, 0xfe, 0x00],
            SourceMediaFamily::PlainText,
        );
        assert_eq!(
            service.admit(invalid, &mut WordCounter, &mut || false),
            Err(SourcePreparationError::InvalidEncoding)
        );
        let binary = request(
            "binary-source",
            vec![0, 1, 2, 3],
            SourceMediaFamily::PlainText,
        );
        assert_eq!(
            service.admit(binary, &mut WordCounter, &mut || false),
            Err(SourcePreparationError::Unsupported)
        );
    }

    #[test]
    fn story_22_3_cancellation_limits_and_persistence_binding_fail_before_publication() {
        let mut service = SourcePreparationService::new();
        let mut cancelled = true;
        assert_eq!(
            service.admit(
                request(
                    "cancelled-source",
                    b"never retained".to_vec(),
                    SourceMediaFamily::PlainText,
                ),
                &mut WordCounter,
                &mut || std::mem::take(&mut cancelled),
            ),
            Err(SourcePreparationError::Cancelled)
        );
        assert!(service.manifests().is_empty());
        assert_eq!(
            service.admission_observations()[0].disposition,
            SourceAdmissionDisposition::Cancelled
        );

        let mut limited = request(
            "limited-source",
            b"one\ntwo\nthree\nfour\nfive\n".to_vec(),
            SourceMediaFamily::PlainText,
        );
        limited.limits.max_lines = 4;
        let manifest = service
            .admit(limited, &mut WordCounter, &mut || false)
            .expect("head/tail source admits");
        assert_eq!(
            manifest.extraction_state,
            PreparedExtractionState::Truncated
        );
        assert_eq!(manifest.retained_lines, 4);
        assert_eq!(manifest.omitted_lines, 1);

        let bytes = b"persist me".to_vec();
        let mut persisted = request(
            "persisted-source",
            bytes.clone(),
            SourceMediaFamily::PlainText,
        );
        persisted.retention = PreparedSourceRetention::PolicyPersisted {
            artifact: RuntimeArtifactRef {
                schema_version: CONTRACT_SCHEMA_VERSION,
                artifact_id: RuntimeArtifactId::from_raw("runtime-source-22-3"),
                manifest_sha256: "a".repeat(64),
                payload_sha256: sha256(&bytes),
                byte_size: bytes.len() as u64,
                media_type: "text/plain".to_owned(),
            },
            policy_sha256: "b".repeat(64),
        };
        service
            .admit(persisted.clone(), &mut WordCounter, &mut || false)
            .expect("exact existing-artifact binding admits");
        persisted.source_id = "persisted-drift".to_owned();
        if let PreparedSourceRetention::PolicyPersisted { artifact, .. } = &mut persisted.retention
        {
            artifact.payload_sha256 = "f".repeat(64);
        }
        assert_eq!(
            service.admit(persisted, &mut WordCounter, &mut || false),
            Err(SourcePreparationError::InvalidInput)
        );

        let mut streamed = request("streamed-source", Vec::new(), SourceMediaFamily::PlainText);
        streamed.limits.max_input_bytes = 12;
        let streamed_manifest = service
            .admit_chunks(
                streamed,
                [b"stream ".to_vec(), b"input".to_vec()],
                &mut WordCounter,
                &mut || false,
            )
            .expect("bounded chunks admit atomically");
        assert_eq!(streamed_manifest.source_bytes, 12);
        let mut overflow = request("stream-overflow", Vec::new(), SourceMediaFamily::PlainText);
        overflow.limits.max_input_bytes = 4;
        assert_eq!(
            service.admit_chunks(overflow, [b"five!".to_vec()], &mut WordCounter, &mut || {
                false
            },),
            Err(SourcePreparationError::ResourceLimit)
        );
        assert!(service.source("stream-overflow").is_none());
    }

    #[test]
    fn story_22_3_growing_log_refresh_preserves_append_line_identity_and_detects_rotation() {
        let mut service = SourcePreparationService::new();
        let mut prior = request(
            "log-revision-1",
            b"alpha\nbeta\n".to_vec(),
            SourceMediaFamily::Log,
        );
        prior.protected_origin_sha256 = "e".repeat(64);
        let prior_manifest = service
            .admit(prior, &mut WordCounter, &mut || false)
            .expect("prior log");
        let prior_ids = service
            .source("log-revision-1")
            .expect("prior")
            .lines
            .into_iter()
            .map(|line| line.line_id)
            .collect::<Vec<_>>();

        let mut appended = request(
            "log-revision-2",
            b"alpha\nbeta\ngamma\n".to_vec(),
            SourceMediaFamily::Log,
        );
        appended.protected_origin_sha256 = "e".repeat(64);
        let result = service
            .refresh_log("log-revision-1", appended, &mut WordCounter, &mut || false)
            .expect("append refresh");
        assert_eq!(result.disposition, LogRefreshDisposition::Appended);
        assert_eq!(result.stable_prior_line_count, prior_ids.len() as u64);
        assert_eq!(
            service
                .source("log-revision-2")
                .expect("successor")
                .lines
                .iter()
                .take(prior_ids.len())
                .map(|line| &line.line_id)
                .collect::<Vec<_>>(),
            prior_ids.iter().collect::<Vec<_>>()
        );

        for (source_id, bytes, expected) in [
            (
                "log-truncated",
                b"alpha\n".to_vec(),
                LogRefreshDisposition::Truncated,
            ),
            (
                "log-rotated",
                b"changed\nbeta\nmore\n".to_vec(),
                LogRefreshDisposition::Rotated,
            ),
        ] {
            let mut candidate = request(source_id, bytes, SourceMediaFamily::Log);
            candidate.protected_origin_sha256 = "e".repeat(64);
            let result = service
                .refresh_log("log-revision-1", candidate, &mut WordCounter, &mut || false)
                .expect("rotation result");
            assert_eq!(result.disposition, expected);
            assert!(result.current_source_id.is_none());
            assert!(service.source(source_id).is_none());
        }
        let stale_prior = service
            .source("log-revision-1")
            .expect("prior remains as lineage");
        assert_eq!(
            stale_prior.manifest.lifecycle,
            PreparedSourceLifecycle::Stale
        );
        assert_ne!(
            stale_prior.manifest.manifest_sha256,
            prior_manifest.manifest_sha256
        );

        let mut bounded_service = SourcePreparationService::new();
        let mut bounded_prior = request(
            "bounded-log-1",
            (0..20)
                .map(|value| format!("line-{value}\n"))
                .collect::<String>()
                .into_bytes(),
            SourceMediaFamily::Log,
        );
        bounded_prior.protected_origin_sha256 = "f".repeat(64);
        bounded_prior.limits.max_lines = 6;
        bounded_service
            .admit(bounded_prior.clone(), &mut WordCounter, &mut || false)
            .expect("bounded prior");
        let mut bounded_successor = bounded_prior;
        bounded_successor.source_id = "bounded-log-2".to_owned();
        bounded_successor
            .bytes
            .extend_from_slice(b"line-20\nline-21\n");
        let bounded_refresh = bounded_service
            .refresh_log(
                "bounded-log-1",
                bounded_successor,
                &mut WordCounter,
                &mut || false,
            )
            .expect("bounded append does not confuse a moving tail with rotation");
        assert_eq!(bounded_refresh.disposition, LogRefreshDisposition::Appended);
        assert_eq!(bounded_refresh.stable_prior_line_count, 20);
    }

    #[test]
    fn story_22_3_persisted_restart_release_and_delete_are_exact_and_fail_closed() {
        let bytes = b"persisted restart evidence\nERROR retained diagnostic\n".to_vec();
        let mut persisted = request("restart-source", bytes.clone(), SourceMediaFamily::Log);
        persisted.retention = PreparedSourceRetention::PolicyPersisted {
            artifact: RuntimeArtifactRef {
                schema_version: CONTRACT_SCHEMA_VERSION,
                artifact_id: RuntimeArtifactId::from_raw("runtime-source-restart"),
                manifest_sha256: "a".repeat(64),
                payload_sha256: sha256(&bytes),
                byte_size: bytes.len() as u64,
                media_type: "text/x-log".to_owned(),
            },
            policy_sha256: "b".repeat(64),
        };
        let mut first = SourcePreparationService::new();
        let manifest = first
            .admit(persisted, &mut WordCounter, &mut || false)
            .expect("initial persisted admission");
        let expected_view = first.source("restart-source").expect("initial view");

        let mut reopened = SourcePreparationService::new();
        reopened
            .restore_persisted(&manifest, bytes.clone(), &mut WordCounter, &mut || false)
            .expect("exact restart");
        assert_eq!(
            reopened.source("restart-source").expect("restored view"),
            expected_view
        );
        let mut corrupt = SourcePreparationService::new();
        assert!(
            corrupt
                .restore_persisted(
                    &manifest,
                    b"corrupt artifact bytes".to_vec(),
                    &mut WordCounter,
                    &mut || false,
                )
                .is_err()
        );
        assert!(corrupt.manifests().is_empty());
        let mut forged_binding = manifest.clone();
        if let PreparedSourceRetention::PolicyPersisted { artifact, .. } =
            &mut forged_binding.retention
        {
            artifact.payload_sha256 = "f".repeat(64);
        }
        forged_binding.manifest_sha256 = manifest_digest(&forged_binding).expect("forged digest");
        assert_eq!(
            verify_prepared_source_manifest(&forged_binding),
            Err(SourcePreparationError::InvalidInput)
        );

        let released = reopened
            .release("restart-source", &manifest.manifest_sha256)
            .expect("release");
        assert_eq!(released.lifecycle, PreparedSourceLifecycle::Released);
        assert!(
            reopened
                .source("restart-source")
                .expect("released lineage")
                .lines
                .is_empty()
        );
        assert_eq!(
            reopened.read_range("restart-source", &released.manifest_sha256, 0, 0),
            Err(SourcePreparationError::Conflict)
        );
        let deleted = reopened
            .delete("restart-source", &released.manifest_sha256)
            .expect("delete");
        assert_eq!(deleted.lifecycle, PreparedSourceLifecycle::Deleted);
        assert_eq!(
            reopened.delete("restart-source", &released.manifest_sha256),
            Err(SourcePreparationError::Conflict)
        );

        let ephemeral = first
            .admit(
                request(
                    "ephemeral-restart",
                    b"memory only".to_vec(),
                    SourceMediaFamily::PlainText,
                ),
                &mut WordCounter,
                &mut || false,
            )
            .expect("ephemeral admission");
        assert_eq!(
            SourcePreparationService::new().restore_persisted(
                &ephemeral,
                b"memory only".to_vec(),
                &mut WordCounter,
                &mut || false,
            ),
            Err(SourcePreparationError::Conflict)
        );
        let expired = first
            .expire("ephemeral-restart", &ephemeral.manifest_sha256)
            .expect("retention expiry");
        assert_eq!(expired.lifecycle, PreparedSourceLifecycle::Expired);
        let deleted_after_expiry = first
            .delete("ephemeral-restart", &expired.manifest_sha256)
            .expect("expired deletion");
        assert_eq!(
            deleted_after_expiry.lifecycle,
            PreparedSourceLifecycle::Deleted
        );
    }

    #[test]
    #[ignore = "subprocess stop target; invoked only by the Story 22.3 crash/restart test"]
    fn story_22_3_source_preparation_crash_child() {
        if std::env::var_os("AGENTMAGE_STORY_22_3_CRASH_CHILD").is_none() {
            return;
        }
        let mut callbacks = 0_u8;
        let _ = SourcePreparationService::new().admit(
            request(
                "crash-child-source",
                b"first line\nsecond line\nthird line\n".to_vec(),
                SourceMediaFamily::Log,
            ),
            &mut WordCounter,
            &mut || {
                callbacks = callbacks.saturating_add(1);
                if callbacks == 3 {
                    std::process::exit(86);
                }
                false
            },
        );
        panic!("crash child reached normal return");
    }

    #[test]
    fn story_22_3_process_stop_publishes_no_partial_source_and_exact_restart_recovers() {
        let status = std::process::Command::new(std::env::current_exe().expect("test executable"))
            .arg("source_preparation::tests::story_22_3_source_preparation_crash_child")
            .arg("--ignored")
            .arg("--exact")
            .env("AGENTMAGE_STORY_22_3_CRASH_CHILD", "1")
            .status()
            .expect("crash child status");
        assert_eq!(status.code(), Some(86));

        let bytes = b"restart after abrupt stop\n".to_vec();
        let mut persisted = request(
            "crash-restart-source",
            bytes.clone(),
            SourceMediaFamily::Log,
        );
        persisted.retention = PreparedSourceRetention::PolicyPersisted {
            artifact: RuntimeArtifactRef {
                schema_version: CONTRACT_SCHEMA_VERSION,
                artifact_id: RuntimeArtifactId::from_raw("runtime-source-crash-restart"),
                manifest_sha256: "a".repeat(64),
                payload_sha256: sha256(&bytes),
                byte_size: bytes.len() as u64,
                media_type: "text/x-log".to_owned(),
            },
            policy_sha256: "b".repeat(64),
        };
        let mut canonical = SourcePreparationService::new();
        let expected = canonical
            .admit(persisted, &mut WordCounter, &mut || false)
            .expect("canonical persisted manifest");
        let mut reopened = SourcePreparationService::new();
        reopened
            .restore_persisted(&expected, bytes, &mut WordCounter, &mut || false)
            .expect("exact post-stop reconstruction");
        assert_eq!(reopened.manifests(), vec![expected]);
    }

    #[test]
    fn story_22_3_twenty_five_mib_log_is_bounded_and_visibly_truncated() {
        let mut bytes = Vec::with_capacity(MAX_SOURCE_BYTES);
        let mut line = vec![b'x'; 255];
        line.push(b'\n');
        while bytes.len() < MAX_SOURCE_BYTES {
            bytes.extend_from_slice(&line);
        }
        assert_eq!(bytes.len(), MAX_SOURCE_BYTES);
        let started = std::time::Instant::now();
        let mut service = SourcePreparationService::new();
        let manifest = service
            .admit(
                request("maximum-log", bytes, SourceMediaFamily::Log),
                &mut WordCounter,
                &mut || false,
            )
            .expect("declared maximum log");
        assert_eq!(manifest.source_bytes as usize, MAX_SOURCE_BYTES);
        assert_eq!(
            manifest.extraction_state,
            PreparedExtractionState::Truncated
        );
        assert!(manifest.omitted_lines > 0);
        let view = service.source("maximum-log").expect("maximum log view");
        assert!(rendered_text(&view.lines).len() <= manifest.limits.max_output_bytes as usize);
        assert_eq!(view.lines.first().expect("head").line_number, 1);
        assert_eq!(
            view.lines.last().expect("tail").line_number,
            manifest.total_lines
        );
        assert!(started.elapsed() < std::time::Duration::from_secs(60));
    }

    #[test]
    fn story_22_3_context_accounts_every_source_section_duplicate_restriction_and_budget() {
        let mut service = SourcePreparationService::new();
        for source_id in ["source-a", "source-b"] {
            service
                .admit(
                    request(
                        source_id,
                        b"shared evidence\nerror detail\n".to_vec(),
                        SourceMediaFamily::PlainText,
                    ),
                    &mut WordCounter,
                    &mut || false,
                )
                .expect("shared source");
        }
        let mut private = request(
            "source-private",
            b"private evidence\n".to_vec(),
            SourceMediaFamily::PlainText,
        );
        private.sensitivity = ContextSensitivity::Private;
        service
            .admit(private, &mut WordCounter, &mut || false)
            .expect("private source");
        let stale = service
            .admit(
                request(
                    "source-stale",
                    b"stale evidence\n".to_vec(),
                    SourceMediaFamily::PlainText,
                ),
                &mut WordCounter,
                &mut || false,
            )
            .expect("stale source first admits");
        service
            .mark_stale("source-stale", &stale.manifest_sha256)
            .expect("mark stale");
        assert_eq!(
            service.admit(
                request(
                    "source-unsupported",
                    vec![0, 1, 2],
                    SourceMediaFamily::PlainText,
                ),
                &mut WordCounter,
                &mut || false,
            ),
            Err(SourcePreparationError::Unsupported)
        );

        let manifest = service
            .compile_context(
                "context-manifest-22-3".to_owned(),
                ContextPacketId::from_raw("context-packet-22-3"),
                &plan(4),
                1_024,
                &mut WordCounter,
                false,
            )
            .expect("context compiles");
        verify_prepared_source_context_manifest(&manifest).expect("manifest verifies");
        assert_eq!(manifest.allocated_tokens, 4);
        assert!(manifest.used_tokens <= 4);
        for source_id in [
            "source-a",
            "source-b",
            "source-private",
            "source-stale",
            "source-unsupported",
        ] {
            assert!(
                manifest
                    .records
                    .iter()
                    .any(|record| { record.source_id == source_id && record.section_id.is_none() })
            );
        }
        for disposition in [
            SourceContextDisposition::Included,
            SourceContextDisposition::Duplicate,
            SourceContextDisposition::Restricted,
            SourceContextDisposition::Stale,
            SourceContextDisposition::Unsupported,
            SourceContextDisposition::Omitted,
        ] {
            assert!(
                manifest
                    .records
                    .iter()
                    .any(|record| record.disposition == disposition),
                "missing {disposition:?}"
            );
        }
        let expected_sections: usize = service
            .sources
            .values()
            .map(|source| source.view.sections.len() + 1)
            .sum();
        assert_eq!(manifest.records.len(), expected_sections + 1);

        let mut tampered = manifest.clone();
        tampered.records[0].reason_code = Some("invented".to_owned());
        assert_eq!(
            verify_prepared_source_context_manifest(&tampered),
            Err(SourcePreparationError::InvalidInput)
        );
        let mut missing_packet_accounting = manifest.clone();
        let included = missing_packet_accounting
            .records
            .iter_mut()
            .find(|record| {
                record.disposition == SourceContextDisposition::Included
                    && record.section_id.is_some()
            })
            .expect("included section record");
        included.disposition = SourceContextDisposition::Omitted;
        included.reason_code = Some("context.section.forged_omission".to_owned());
        assert_eq!(
            verify_prepared_source_context_manifest(&missing_packet_accounting),
            Err(SourcePreparationError::InvalidInput)
        );
        let mut drifted_plan = plan(4);
        drifted_plan.source_artifacts.allocated_tokens = 3;
        assert_eq!(
            service.compile_context(
                "context-plan-drift".to_owned(),
                ContextPacketId::from_raw("packet-plan-drift"),
                &drifted_plan,
                1_024,
                &mut WordCounter,
                false,
            ),
            Err(SourcePreparationError::TokenizerMismatch)
        );
    }
}
