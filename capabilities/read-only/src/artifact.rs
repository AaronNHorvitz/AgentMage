//! Closed source-artifact protocol and deterministic in-memory review backend.

use std::collections::BTreeSet;
use std::fmt::Write;

use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, GrantOperation, OperationBinding, RequiredGrantTemplate, SchemaId,
    SchemaReference, ToolDefinition, ToolId, ToolRiskLevel,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::protocol::parse_closed_json;

/// Exact version of the initial source-artifact protocol.
pub const ARTIFACT_TOOL_VERSION: &str = "1.0.0";
/// Shared closed input schema identity.
pub const ARTIFACT_INPUT_SCHEMA_ID: &str = "agentmage.artifact.input";
/// Shared closed output schema identity.
pub const ARTIFACT_OUTPUT_SCHEMA_ID: &str = "agentmage.artifact.output";
/// Maximum request bytes accepted before dispatch.
pub const MAX_ARTIFACT_REQUEST_BYTES: usize = 64 * 1024;
/// Hard maximum result items.
pub const MAX_ARTIFACT_ITEMS: u32 = 1_000;
/// Hard maximum source bytes inspected by one attempt.
pub const MAX_ARTIFACT_INPUT_BYTES: u64 = 16 * 1024 * 1024;
/// Hard maximum returned bytes.
pub const MAX_ARTIFACT_OUTPUT_BYTES: u64 = 2 * 1024 * 1024;
/// Hard maximum coordinate span.
pub const MAX_ARTIFACT_RANGE: u64 = 1_000_000;
/// Hard maximum task units charged to one attempt.
pub const MAX_ARTIFACT_TASKS: u32 = 1_000;
/// Hard memory ceiling declared by a request.
pub const MAX_ARTIFACT_MEMORY_BYTES: u64 = 32 * 1024 * 1024;
/// Hard wall-clock ceiling declared by a request.
pub const MAX_ARTIFACT_TIME_MS: u64 = 15_000;
/// Hard nested dispatcher depth.
pub const MAX_ARTIFACT_CALL_DEPTH: u8 = 8;

/// Canonical Draft 2020-12 schema bytes bound into all seven definitions.
pub const ARTIFACT_INPUT_SCHEMA_JSON: &str = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","$id":"agentmage.artifact.input","type":"object","additionalProperties":false,"required":["schema_version","call_id","source_id","section_id","range","query","freshness_sha256","output_identity","limits","call_depth"],"properties":{"schema_version":{"const":1},"call_id":{"type":"string","minLength":1,"maxLength":255},"source_id":{"type":["string","null"],"maxLength":255},"section_id":{"type":["string","null"],"maxLength":255},"range":{"type":["object","null"]},"query":{"type":["string","null"],"maxLength":4096},"freshness_sha256":{"type":["string","null"],"pattern":"^[0-9a-f]{64}$"},"output_identity":{"type":"string","minLength":1,"maxLength":255},"limits":{"type":"object","additionalProperties":false,"required":["items","input_bytes","range_units","output_bytes","time_ms","memory_bytes","tasks"],"properties":{"items":{"type":"integer","minimum":1,"maximum":1000},"input_bytes":{"type":"integer","minimum":1,"maximum":16777216},"range_units":{"type":"integer","minimum":1,"maximum":1000000},"output_bytes":{"type":"integer","minimum":1,"maximum":2097152},"time_ms":{"type":"integer","minimum":1,"maximum":15000},"memory_bytes":{"type":"integer","minimum":1,"maximum":33554432},"tasks":{"type":"integer","minimum":1,"maximum":1000}}},"call_depth":{"type":"integer","minimum":0,"maximum":8}}}"#;
/// Canonical output schema bytes bound into all seven definitions.
pub const ARTIFACT_OUTPUT_SCHEMA_JSON: &str = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","$id":"agentmage.artifact.output","type":"object","additionalProperties":false,"required":["schema_version","tool","call_id","output_identity","outcome","source_id","provenance_id","freshness_sha256","items","observed_items","observed_bytes","output_bytes","truncated","production_execution","receipt"],"properties":{"schema_version":{"const":1},"tool":{"type":"string"},"call_id":{"type":"string"},"output_identity":{"type":"string"},"outcome":{"enum":["succeeded","no_result","denied","stale","restricted","unsupported","out_of_range","cancelled","timed_out","failed","truncated"]},"source_id":{"type":["string","null"]},"provenance_id":{"type":["string","null"]},"freshness_sha256":{"type":["string","null"]},"items":{"type":"array","maxItems":1000},"observed_items":{"type":"integer","minimum":0},"observed_bytes":{"type":"integer","minimum":0},"output_bytes":{"type":"integer","minimum":0,"maximum":2097152},"truncated":{"type":"boolean"},"production_execution":{"type":"boolean"},"receipt":{"type":"object"}}}"#;

/// Closed text/log operation catalog. Later page and sheet extensions remain absent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArtifactToolKind {
    /// List admitted source manifests.
    List,
    /// Read one content-free manifest projection.
    Metadata,
    /// Read a bounded default or named section.
    Read,
    /// Read one exact coordinate range.
    Range,
    /// List structural sections.
    Sections,
    /// Search extracted text lexically.
    Search,
    /// Read bounded error, warning, stack, and test-failure clusters from prepared logs.
    LogErrors,
}

impl ArtifactToolKind {
    /// Stable registry order.
    pub const ALL: [Self; 7] = [
        Self::List,
        Self::Metadata,
        Self::Read,
        Self::Range,
        Self::Sections,
        Self::Search,
        Self::LogErrors,
    ];

    /// Exact public tool identity.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::List => "artifact.list",
            Self::Metadata => "artifact.metadata",
            Self::Read => "artifact.read",
            Self::Range => "artifact.range",
            Self::Sections => "artifact.sections",
            Self::Search => "artifact.search",
            Self::LogErrors => "artifact.get_log_errors",
        }
    }

    const fn display_name(self) -> &'static str {
        match self {
            Self::List => "List source artifacts",
            Self::Metadata => "Read source-artifact metadata",
            Self::Read => "Read source artifact",
            Self::Range => "Read source-artifact range",
            Self::Sections => "List source-artifact sections",
            Self::Search => "Search source artifact",
            Self::LogErrors => "Read prepared log diagnostics",
        }
    }
}

/// Caller-selected limits, each checked against a fixed product ceiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactLimits {
    /// Maximum returned items.
    pub items: u32,
    /// Maximum inspected source bytes.
    pub input_bytes: u64,
    /// Maximum requested coordinate span.
    pub range_units: u64,
    /// Maximum serialized item bytes.
    pub output_bytes: u64,
    /// Maximum declared wall time.
    pub time_ms: u64,
    /// Maximum declared working memory.
    pub memory_bytes: u64,
    /// Maximum charged logical tasks.
    pub tasks: u32,
}

impl Default for ArtifactLimits {
    fn default() -> Self {
        Self {
            items: 100,
            input_bytes: 4 * 1024 * 1024,
            range_units: 100_000,
            output_bytes: 1024 * 1024,
            time_ms: 5_000,
            memory_bytes: 8 * 1024 * 1024,
            tasks: 100,
        }
    }
}

/// Exact coordinate range supported by the artifact protocol.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ArtifactRange {
    /// Zero-based inclusive-exclusive byte offsets.
    Byte {
        /// First included byte.
        start: u64,
        /// First excluded byte.
        end_exclusive: u64,
    },
    /// One-based inclusive-exclusive line offsets.
    Line {
        /// First included line.
        start: u64,
        /// First excluded line.
        end_exclusive: u64,
    },
    /// One-based inclusive-exclusive page numbers.
    Page {
        /// First included page.
        start: u64,
        /// First excluded page.
        end_exclusive: u64,
    },
    /// One exact sheet.
    Sheet {
        /// Exact sheet name.
        name: String,
    },
    /// One-indexed inclusive cell rectangle.
    Cell {
        /// Exact sheet name.
        sheet: String,
        /// First included row.
        start_row: u64,
        /// First included column.
        start_column: u64,
        /// Last included row.
        end_row: u64,
        /// Last included column.
        end_column: u64,
    },
    /// One exact structural section.
    Section {
        /// Exact section identity.
        section_id: String,
    },
}

/// Closed request shared by the seven exact identities.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRequest {
    /// Schema version; only one is admitted.
    pub schema_version: u16,
    /// Unique attempt identity used for replay refusal.
    pub call_id: String,
    /// Exact source identity, absent only for list.
    pub source_id: Option<String>,
    /// Exact section identity for section reads.
    pub section_id: Option<String>,
    /// Exact coordinate range for range reads.
    pub range: Option<ArtifactRange>,
    /// Exact case-sensitive lexical query for search.
    pub query: Option<String>,
    /// Required current manifest freshness except for list.
    pub freshness_sha256: Option<String>,
    /// Caller-selected result identity bound into the receipt.
    pub output_identity: String,
    /// Exact execution ceilings.
    pub limits: ArtifactLimits,
    /// Current nested dispatcher depth.
    pub call_depth: u8,
}

/// Source sensitivity available to the protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactClassification {
    /// Public content.
    Public,
    /// Internal content.
    Internal,
    /// Confidential content requiring an exact restricted grant not supplied here.
    Confidential,
    /// Restricted content never disclosed by the workspace-read protocol.
    Restricted,
}

/// Terminal extraction state projected without launching a parser.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactExtractionState {
    /// Extraction is complete.
    Complete,
    /// Extraction is partial and omissions must remain visible.
    Partial,
    /// Source format is unsupported.
    Unsupported,
    /// Extraction failed.
    Failed,
}

/// Freshness binding for one immutable source projection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactFreshness {
    /// Immutable manifest digest.
    pub manifest_sha256: String,
    /// Trusted observation sequence.
    pub observed_sequence: u64,
}

/// Content-free source provenance.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactProvenance {
    /// Stable provenance identity.
    pub provenance_id: String,
    /// Stable source kind such as upload, paste, or generated.
    pub source_kind: String,
    /// Digest of protected origin metadata; never the raw path or URI.
    pub protected_origin_sha256: String,
}

/// Content-free manifest projection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactManifest {
    /// Stable source identity.
    pub source_id: String,
    /// Closed media type.
    pub media_type: String,
    /// Source byte size.
    pub byte_len: u64,
    /// Source payload digest.
    pub payload_sha256: String,
    /// Sensitivity classification.
    pub classification: ArtifactClassification,
    /// Extraction state.
    pub extraction_state: ArtifactExtractionState,
    /// Exact provenance.
    pub provenance: ArtifactProvenance,
    /// Exact freshness.
    pub freshness: ArtifactFreshness,
    /// Stable content-free reason for non-complete extraction.
    pub reason_code: Option<String>,
}

/// One structural section projection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactSection {
    /// Stable section identity.
    pub section_id: String,
    /// Bounded inert title.
    pub title: String,
    /// Stable order within its source.
    pub ordinal: u32,
    /// Exact byte size of extracted section content.
    pub byte_len: u64,
}

/// One typed extracted fragment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ArtifactFragment {
    /// Exact byte slice.
    Byte {
        /// First included byte.
        start: u64,
        /// First excluded byte.
        end_exclusive: u64,
        /// Exact inert extracted text.
        content: String,
    },
    /// Exact line slice.
    Line {
        /// First included line.
        start: u64,
        /// First excluded line.
        end_exclusive: u64,
        /// Exact inert extracted text.
        content: String,
    },
    /// Exact page.
    Page {
        /// One-based page number.
        number: u64,
        /// Exact inert extracted text.
        content: String,
    },
    /// Exact sheet.
    Sheet {
        /// Exact sheet name.
        name: String,
        /// Exact inert extracted text.
        content: String,
    },
    /// Exact cell.
    Cell {
        /// Exact sheet name.
        sheet: String,
        /// One-based row.
        row: u64,
        /// One-based column.
        column: u64,
        /// Exact inert extracted text.
        content: String,
    },
    /// Exact structural section.
    Section {
        /// Exact section identity.
        section_id: String,
        /// Exact inert extracted text.
        content: String,
    },
}

/// One exact backend search match before protocol result shaping.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactSearchHit {
    /// Exact typed fragment containing the match.
    pub fragment: ArtifactFragment,
    /// Zero-based normalized fragment byte offset.
    pub match_start: u64,
    /// First normalized fragment byte after the match.
    pub match_end_exclusive: u64,
}

/// Synthetic source record used only by the in-memory review backend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FakeArtifactSource {
    /// Content-free manifest.
    pub manifest: ArtifactManifest,
    /// Exact sections.
    pub sections: Vec<ArtifactSection>,
    /// Extracted coordinate fragments.
    pub fragments: Vec<ArtifactFragment>,
    /// Whether secret redaction makes all content undisclosable.
    pub redacted: bool,
}

/// Closed backend class recorded in every terminal receipt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactBackendClass {
    /// Deterministic in-memory review projection.
    ReviewProjection,
    /// Runtime-owned already-prepared production source service.
    ProductionPreparedSource,
}

/// Read-only backend port over already-admitted, already-prepared source projections.
pub trait ArtifactBackend {
    /// Returns stable manifest order.
    fn manifests(&self) -> Vec<ArtifactManifest>;
    /// Returns one source projection.
    fn source(&self, source_id: &str) -> Option<FakeArtifactSource>;
    /// Returns the closed backend class; neither class grants authority or launches a parser.
    fn backend_class(&self) -> ArtifactBackendClass;
    /// Returns deterministic prepared lexical matches.
    fn search(
        &self,
        source_id: &str,
        query: &str,
        _maximum_items: usize,
    ) -> Option<Vec<ArtifactSearchHit>> {
        self.source(source_id).map(|source| {
            source
                .fragments
                .into_iter()
                .filter_map(|fragment| {
                    let start = fragment_content(&fragment).find(query)?;
                    Some(ArtifactSearchHit {
                        match_start: start as u64,
                        match_end_exclusive: (start + query.len()) as u64,
                        fragment,
                    })
                })
                .collect()
        })
    }
    /// Returns deterministic exact coordinate fragments.
    fn range(
        &self,
        source_id: &str,
        range: &ArtifactRange,
        _maximum_units: u64,
    ) -> Option<Vec<ArtifactFragment>> {
        self.source(source_id).map(|source| {
            source
                .fragments
                .into_iter()
                .filter(|fragment| range_matches(range, fragment))
                .collect()
        })
    }
    /// Returns the default prepared reading or one exact named section.
    fn read(
        &self,
        source_id: &str,
        section_id: Option<&str>,
        _maximum_items: usize,
    ) -> Option<Vec<ArtifactFragment>> {
        self.source(source_id).map(|source| {
            source
                .fragments
                .into_iter()
                .filter(|fragment| match (section_id, fragment) {
                    (Some(expected), ArtifactFragment::Section { section_id, .. }) => {
                        expected == section_id
                    }
                    (None, _) => true,
                    _ => false,
                })
                .collect()
        })
    }
    /// Returns bounded prepared diagnostic fragments for the admitted log source.
    fn log_errors(&self, source_id: &str, maximum_items: usize) -> Option<Vec<ArtifactFragment>> {
        self.source(source_id).map(|source| {
            source
                .fragments
                .into_iter()
                .filter(diagnostic_fragment)
                .take(maximum_items.saturating_add(1))
                .collect()
        })
    }
}

/// Reserved structured-document extractor ports for later native stories.
///
/// These methods are deliberately not part of [`ArtifactToolKind`] and have no implementation in
/// this crate. Defining their typed boundary now prevents later extractors from bypassing source,
/// freshness, or limit identity when the operations are eventually admitted.
pub trait ArtifactExtractorExtensions {
    /// Reads one exact page through a future admitted extractor.
    fn get_page(
        &self,
        source_id: &str,
        freshness_sha256: &str,
        page_number: u64,
        limits: ArtifactLimits,
    ) -> Result<ArtifactFragment, ArtifactExtensionError>;

    /// Reads one exact sheet through a future admitted extractor.
    fn get_sheet(
        &self,
        source_id: &str,
        freshness_sha256: &str,
        sheet_name: &str,
        limits: ArtifactLimits,
    ) -> Result<Vec<ArtifactFragment>, ArtifactExtensionError>;
}

/// Stable refusal family reserved for future extractor extension ports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactExtensionError {
    /// The extractor family is not admitted.
    Unsupported,
    /// The source freshness binding changed.
    Stale,
    /// Policy prohibits the requested projection.
    Denied,
    /// A declared hard bound would be exceeded.
    LimitExceeded,
    /// The exact coordinate does not exist.
    OutOfRange,
}

/// Deterministic in-memory fake backend.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FakeArtifactBackend {
    sources: Vec<FakeArtifactSource>,
}

impl FakeArtifactBackend {
    /// Builds a canonical fake backend, refusing duplicate or unordered identities.
    pub fn new(mut sources: Vec<FakeArtifactSource>) -> Result<Self, ArtifactDispatchError> {
        sources.sort_by(|a, b| a.manifest.source_id.cmp(&b.manifest.source_id));
        if sources.iter().any(|source| !valid_source(source))
            || sources
                .windows(2)
                .any(|pair| pair[0].manifest.source_id == pair[1].manifest.source_id)
        {
            return Err(ArtifactDispatchError::InvalidProjection);
        }
        Ok(Self { sources })
    }
}

impl ArtifactBackend for FakeArtifactBackend {
    fn manifests(&self) -> Vec<ArtifactManifest> {
        self.sources
            .iter()
            .map(|source| source.manifest.clone())
            .collect()
    }

    fn source(&self, source_id: &str) -> Option<FakeArtifactSource> {
        self.sources
            .binary_search_by(|source| source.manifest.source_id.as_str().cmp(source_id))
            .ok()
            .map(|index| self.sources[index].clone())
    }

    fn backend_class(&self) -> ArtifactBackendClass {
        ArtifactBackendClass::ReviewProjection
    }
}

/// Out-of-band attempt condition owned by the trusted dispatcher.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ArtifactExecutionSignal {
    /// Run normally.
    #[default]
    Continue,
    /// Cancellation was observed.
    Cancelled,
    /// Trusted supervisor deadline elapsed.
    TimedOut,
    /// Isolated worker failed.
    Crashed,
}

/// Closed result state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactOutcome {
    /// Complete bounded success.
    Succeeded,
    /// Valid request found nothing.
    NoResult,
    /// Policy denied execution.
    Denied,
    /// Freshness binding differs.
    Stale,
    /// Classification prohibits disclosure.
    Restricted,
    /// Extraction state or coordinate is unsupported.
    Unsupported,
    /// Requested coordinate does not exist.
    OutOfRange,
    /// Attempt was cancelled.
    Cancelled,
    /// Attempt timed out.
    TimedOut,
    /// Projection or worker failed.
    Failed,
    /// Declared item or output bound truncated results.
    Truncated,
}

/// Typed result item.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ArtifactItem {
    /// Content-free manifest.
    Manifest {
        /// Exact content-free manifest.
        manifest: ArtifactManifest,
    },
    /// Structural section.
    Section {
        /// Exact section projection.
        section: ArtifactSection,
    },
    /// Exact extracted content with a typed locator.
    Content {
        /// Exact typed fragment.
        fragment: ArtifactFragment,
    },
    /// Exact lexical hit.
    SearchHit {
        /// Exact fragment containing the match.
        fragment: ArtifactFragment,
        /// Zero-based match start within fragment content.
        match_start: u64,
        /// First excluded byte within fragment content.
        match_end_exclusive: u64,
    },
    /// Reference replacing a result too large for inline output.
    LargeResultReference {
        /// Stable non-authoritative reference identity.
        reference_id: String,
        /// Exact pre-truncation item count.
        item_count: u32,
        /// Digest binding the omitted result identity and count.
        content_sha256: String,
    },
}

/// Content-free receipt produced for every launched attempt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactReceipt {
    /// Exact call identity.
    pub call_id: String,
    /// Exact tool identity.
    pub tool: String,
    /// Exact source when one was selected.
    pub source_id: Option<String>,
    /// Exact provenance when one was selected.
    pub provenance_id: Option<String>,
    /// Exact freshness when one was selected.
    pub freshness_sha256: Option<String>,
    /// Caller-selected output identity.
    pub output_identity: String,
    /// Terminal outcome.
    pub outcome: ArtifactOutcome,
    /// Exact limits used.
    pub limits: ArtifactLimits,
    /// Whether the attempt read the runtime-owned production prepared-source service.
    pub production_execution: bool,
    /// Always false: source parsing occurs before tool dispatch.
    pub parser_launched: bool,
    /// Always false.
    pub network_accessed: bool,
    /// Always false.
    pub workspace_mutated: bool,
    /// Digest binding every preceding receipt field.
    pub receipt_sha256: String,
}

/// Hash-bound artifact execution result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactResult {
    /// Result schema version.
    pub schema_version: u16,
    /// Exact tool.
    pub tool: String,
    /// Exact call identity.
    pub call_id: String,
    /// Caller-selected output identity.
    pub output_identity: String,
    /// Terminal result state.
    pub outcome: ArtifactOutcome,
    /// Exact source when selected.
    pub source_id: Option<String>,
    /// Exact provenance when selected.
    pub provenance_id: Option<String>,
    /// Exact freshness when selected.
    pub freshness_sha256: Option<String>,
    /// Deterministically ordered result items.
    pub items: Vec<ArtifactItem>,
    /// Items inspected before termination.
    pub observed_items: u32,
    /// Source bytes inspected before termination.
    pub observed_bytes: u64,
    /// Serialized bytes of `items`.
    pub output_bytes: u64,
    /// Whether a declared limit stopped output.
    pub truncated: bool,
    /// Whether this result used the runtime-owned production prepared-source service.
    pub production_execution: bool,
    /// One terminal receipt.
    pub receipt: ArtifactReceipt,
}

impl ArtifactResult {
    /// Verifies result, receipt, backend identity, and read-only safety invariants.
    #[must_use]
    pub fn verify(&self, kind: ArtifactToolKind) -> bool {
        self.schema_version == 1
            && self.tool == kind.id()
            && self.truncated == (self.outcome == ArtifactOutcome::Truncated)
            && self.output_bytes
                == serde_json::to_vec(&self.items).map_or(u64::MAX, |bytes| bytes.len() as u64)
            && self.output_bytes <= MAX_ARTIFACT_OUTPUT_BYTES
            && self.receipt.call_id == self.call_id
            && self.receipt.tool == self.tool
            && self.receipt.source_id == self.source_id
            && self.receipt.provenance_id == self.provenance_id
            && self.receipt.freshness_sha256 == self.freshness_sha256
            && self.receipt.output_identity == self.output_identity
            && self.receipt.outcome == self.outcome
            && self.receipt.production_execution == self.production_execution
            && !self.receipt.parser_launched
            && !self.receipt.network_accessed
            && !self.receipt.workspace_mutated
            && receipt_digest(&self.receipt) == self.receipt.receipt_sha256
    }
}

/// Pre-launch dispatcher refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactDispatchError {
    /// Request bytes or closed shape are malformed.
    Malformed,
    /// Request conflicts with the selected operation or a hard ceiling.
    Denied,
    /// No current workspace-read authority exists.
    Unauthorized,
    /// Call identity was already launched.
    RepeatedCall,
    /// Backend projection is malformed.
    InvalidProjection,
}

impl ArtifactDispatchError {
    /// Stable content-free diagnostic code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Malformed => "artifact.request.malformed",
            Self::Denied => "artifact.request.denied",
            Self::Unauthorized => "artifact.authority.denied",
            Self::RepeatedCall => "artifact.call.repeated",
            Self::InvalidProjection => "artifact.projection.invalid",
        }
    }
}

/// Process-local exact call ledger used to prove single launch semantics.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ArtifactAttemptLedger {
    launched: BTreeSet<String>,
}

impl ArtifactAttemptLedger {
    /// Number of unique launched attempts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.launched.len()
    }

    /// Whether no attempt was launched.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.launched.is_empty()
    }
}

/// Returns all seven declarative definitions in stable order.
#[must_use]
pub fn artifact_tool_definitions() -> Vec<ToolDefinition> {
    ArtifactToolKind::ALL
        .into_iter()
        .map(artifact_tool_definition)
        .collect()
}

/// Returns one exact non-executable definition.
#[must_use]
pub fn artifact_tool_definition(kind: ArtifactToolKind) -> ToolDefinition {
    let operation = OperationBinding::new(GrantOperation::WorkspaceRead);
    ToolDefinition {
        schema_version: CONTRACT_SCHEMA_VERSION,
        tool_id: ToolId::from_raw(kind.id()),
        tool_version: ARTIFACT_TOOL_VERSION.to_owned(),
        display_name: kind.display_name().to_owned(),
        description: "Inspects one already-admitted immutable source-artifact projection"
            .to_owned(),
        input_schema: schema(ARTIFACT_INPUT_SCHEMA_ID, ARTIFACT_INPUT_SCHEMA_JSON),
        output_schema: schema(ARTIFACT_OUTPUT_SCHEMA_ID, ARTIFACT_OUTPUT_SCHEMA_JSON),
        risk_level: ToolRiskLevel::Low,
        declared_effects: vec![operation],
        required_grant: RequiredGrantTemplate {
            operation,
            target_scope: "exact-admitted-source-artifact-projection".to_owned(),
            single_use: true,
        },
        timeout_ms: MAX_ARTIFACT_TIME_MS,
    }
}

/// Resolves only an exact admitted identity and version.
#[must_use]
pub fn artifact_tool_kind(tool_id: &ToolId, version: &str) -> Option<ArtifactToolKind> {
    (version == ARTIFACT_TOOL_VERSION)
        .then(|| {
            ArtifactToolKind::ALL
                .into_iter()
                .find(|kind| kind.id() == tool_id.as_str())
        })
        .flatten()
}

/// Parses and validates a closed request without entering dispatch.
pub fn validate_artifact_request(
    kind: ArtifactToolKind,
    bytes: &[u8],
) -> Result<ArtifactRequest, ArtifactDispatchError> {
    if bytes.is_empty() || bytes.len() > MAX_ARTIFACT_REQUEST_BYTES {
        return Err(ArtifactDispatchError::Malformed);
    }
    let request: ArtifactRequest =
        parse_closed_json(bytes).map_err(|()| ArtifactDispatchError::Malformed)?;
    valid_request(kind, &request)
        .then_some(request)
        .ok_or(ArtifactDispatchError::Denied)
}

/// Dispatches one authorized prepared-source attempt and emits exactly one terminal receipt.
///
/// Invalid, unauthorized, and repeated calls fail before launch and therefore produce no receipt.
pub fn dispatch_artifact(
    kind: ArtifactToolKind,
    request_bytes: &[u8],
    backend: &impl ArtifactBackend,
    workspace_read_authorized: bool,
    signal: ArtifactExecutionSignal,
    ledger: &mut ArtifactAttemptLedger,
) -> Result<ArtifactResult, ArtifactDispatchError> {
    let request = validate_artifact_request(kind, request_bytes)?;
    if !workspace_read_authorized {
        return Err(ArtifactDispatchError::Unauthorized);
    }
    if !valid_backend(backend) {
        return Err(ArtifactDispatchError::InvalidProjection);
    }
    let production_execution =
        backend.backend_class() == ArtifactBackendClass::ProductionPreparedSource;
    if !ledger.launched.insert(request.call_id.clone()) {
        return Err(ArtifactDispatchError::RepeatedCall);
    }
    if signal != ArtifactExecutionSignal::Continue {
        let outcome = match signal {
            ArtifactExecutionSignal::Cancelled => ArtifactOutcome::Cancelled,
            ArtifactExecutionSignal::TimedOut => ArtifactOutcome::TimedOut,
            ArtifactExecutionSignal::Crashed => ArtifactOutcome::Failed,
            ArtifactExecutionSignal::Continue => unreachable!(),
        };
        return Ok(build_result(
            kind,
            &request,
            outcome,
            None,
            Vec::new(),
            (0, 0),
            production_execution,
        ));
    }
    execute(kind, &request, backend)
}

fn execute(
    kind: ArtifactToolKind,
    request: &ArtifactRequest,
    backend: &impl ArtifactBackend,
) -> Result<ArtifactResult, ArtifactDispatchError> {
    if kind == ArtifactToolKind::List {
        let items = backend
            .manifests()
            .into_iter()
            .map(|manifest| ArtifactItem::Manifest { manifest })
            .collect();
        return Ok(bounded_result(
            kind,
            request,
            None,
            items,
            0,
            backend.backend_class() == ArtifactBackendClass::ProductionPreparedSource,
        ));
    }
    let source_id = request.source_id.as_deref().expect("validated source");
    let Some(source) = backend.source(source_id) else {
        return Ok(build_result(
            kind,
            request,
            ArtifactOutcome::NoResult,
            None,
            Vec::new(),
            (0, 0),
            backend.backend_class() == ArtifactBackendClass::ProductionPreparedSource,
        ));
    };
    if source.manifest.classification == ArtifactClassification::Restricted
        || source.manifest.classification == ArtifactClassification::Confidential
    {
        return Ok(build_result(
            kind,
            request,
            ArtifactOutcome::Restricted,
            Some(&source),
            Vec::new(),
            (0, 0),
            backend.backend_class() == ArtifactBackendClass::ProductionPreparedSource,
        ));
    }
    if request.freshness_sha256.as_deref()
        != Some(source.manifest.freshness.manifest_sha256.as_str())
    {
        return Ok(build_result(
            kind,
            request,
            ArtifactOutcome::Stale,
            Some(&source),
            Vec::new(),
            (0, 0),
            backend.backend_class() == ArtifactBackendClass::ProductionPreparedSource,
        ));
    }
    if source.redacted {
        return Ok(build_result(
            kind,
            request,
            ArtifactOutcome::Denied,
            Some(&source),
            Vec::new(),
            (0, 0),
            backend.backend_class() == ArtifactBackendClass::ProductionPreparedSource,
        ));
    }
    if matches!(
        source.manifest.extraction_state,
        ArtifactExtractionState::Unsupported
    ) {
        return Ok(build_result(
            kind,
            request,
            ArtifactOutcome::Unsupported,
            Some(&source),
            Vec::new(),
            (0, 0),
            backend.backend_class() == ArtifactBackendClass::ProductionPreparedSource,
        ));
    }
    if matches!(
        source.manifest.extraction_state,
        ArtifactExtractionState::Failed
    ) {
        return Ok(build_result(
            kind,
            request,
            ArtifactOutcome::Failed,
            Some(&source),
            Vec::new(),
            (0, 0),
            backend.backend_class() == ArtifactBackendClass::ProductionPreparedSource,
        ));
    }
    let observed = source.manifest.byte_len.min(request.limits.input_bytes);
    let items = match kind {
        ArtifactToolKind::Metadata => vec![ArtifactItem::Manifest {
            manifest: source.manifest.clone(),
        }],
        ArtifactToolKind::Sections => source
            .sections
            .iter()
            .cloned()
            .map(|section| ArtifactItem::Section { section })
            .collect(),
        ArtifactToolKind::Read => {
            let section = request.section_id.as_deref();
            backend
                .read(source_id, section, request.limits.items as usize)
                .ok_or(ArtifactDispatchError::InvalidProjection)?
                .into_iter()
                .map(|fragment| ArtifactItem::Content { fragment })
                .collect()
        }
        ArtifactToolKind::Range => backend
            .range(
                source_id,
                request.range.as_ref().expect("validated range"),
                request.limits.range_units,
            )
            .ok_or(ArtifactDispatchError::InvalidProjection)?
            .into_iter()
            .map(|fragment| ArtifactItem::Content { fragment })
            .collect(),
        ArtifactToolKind::Search => backend
            .search(
                source_id,
                request.query.as_deref().expect("validated query"),
                request.limits.items as usize,
            )
            .ok_or(ArtifactDispatchError::InvalidProjection)?
            .into_iter()
            .map(|hit| ArtifactItem::SearchHit {
                fragment: hit.fragment,
                match_start: hit.match_start,
                match_end_exclusive: hit.match_end_exclusive,
            })
            .collect(),
        ArtifactToolKind::LogErrors => backend
            .log_errors(source_id, request.limits.items as usize)
            .ok_or(ArtifactDispatchError::InvalidProjection)?
            .into_iter()
            .map(|fragment| ArtifactItem::Content { fragment })
            .collect(),
        ArtifactToolKind::List => unreachable!(),
    };
    if kind == ArtifactToolKind::Range && items.is_empty() {
        return Ok(build_result(
            kind,
            request,
            ArtifactOutcome::OutOfRange,
            Some(&source),
            Vec::new(),
            (
                u32::try_from(source.fragments.len()).unwrap_or(u32::MAX),
                observed,
            ),
            backend.backend_class() == ArtifactBackendClass::ProductionPreparedSource,
        ));
    }
    Ok(bounded_result(
        kind,
        request,
        Some(&source),
        items,
        observed,
        backend.backend_class() == ArtifactBackendClass::ProductionPreparedSource,
    ))
}

fn bounded_result(
    kind: ArtifactToolKind,
    request: &ArtifactRequest,
    source: Option<&FakeArtifactSource>,
    mut items: Vec<ArtifactItem>,
    observed_bytes: u64,
    production_execution: bool,
) -> ArtifactResult {
    let original_count = items.len();
    items.truncate(request.limits.items as usize);
    let mut truncated = items.len() != original_count;
    while serde_json::to_vec(&items).map_or(usize::MAX, |bytes| bytes.len())
        > request.limits.output_bytes as usize
    {
        truncated = true;
        if items.pop().is_none() {
            break;
        }
    }
    if truncated && items.is_empty() && original_count > 0 {
        let digest = sha256_hex(
            format!("{}:{}:{original_count}", kind.id(), request.output_identity).as_bytes(),
        );
        let reference = ArtifactItem::LargeResultReference {
            reference_id: format!("artifact-result:{}", &digest[..16]),
            item_count: u32::try_from(original_count).unwrap_or(u32::MAX),
            content_sha256: digest,
        };
        if serde_json::to_vec(&[&reference]).map_or(usize::MAX, |bytes| bytes.len())
            <= request.limits.output_bytes as usize
        {
            items.push(reference);
        }
    }
    let outcome = if truncated {
        ArtifactOutcome::Truncated
    } else if items.is_empty() {
        ArtifactOutcome::NoResult
    } else {
        ArtifactOutcome::Succeeded
    };
    build_result(
        kind,
        request,
        outcome,
        source,
        items,
        (original_count as u32, observed_bytes),
        production_execution,
    )
}

fn build_result(
    kind: ArtifactToolKind,
    request: &ArtifactRequest,
    outcome: ArtifactOutcome,
    source: Option<&FakeArtifactSource>,
    items: Vec<ArtifactItem>,
    observed: (u32, u64),
    production_execution: bool,
) -> ArtifactResult {
    let (observed_items, observed_bytes) = observed;
    let source_id = source
        .map(|value| value.manifest.source_id.clone())
        .or_else(|| request.source_id.clone());
    let provenance_id = source.map(|value| value.manifest.provenance.provenance_id.clone());
    let freshness_sha256 = source.map(|value| value.manifest.freshness.manifest_sha256.clone());
    let mut receipt = ArtifactReceipt {
        call_id: request.call_id.clone(),
        tool: kind.id().to_owned(),
        source_id: source_id.clone(),
        provenance_id: provenance_id.clone(),
        freshness_sha256: freshness_sha256.clone(),
        output_identity: request.output_identity.clone(),
        outcome,
        limits: request.limits,
        production_execution,
        parser_launched: false,
        network_accessed: false,
        workspace_mutated: false,
        receipt_sha256: "0".repeat(64),
    };
    receipt.receipt_sha256 = receipt_digest(&receipt);
    ArtifactResult {
        schema_version: 1,
        tool: kind.id().to_owned(),
        call_id: request.call_id.clone(),
        output_identity: request.output_identity.clone(),
        outcome,
        source_id,
        provenance_id,
        freshness_sha256,
        output_bytes: serde_json::to_vec(&items).map_or(u64::MAX, |bytes| bytes.len() as u64),
        items,
        observed_items,
        observed_bytes,
        truncated: outcome == ArtifactOutcome::Truncated,
        production_execution,
        receipt,
    }
}

fn valid_request(kind: ArtifactToolKind, request: &ArtifactRequest) -> bool {
    let limits = request.limits;
    let common = request.schema_version == 1
        && valid_identity(&request.call_id)
        && valid_identity(&request.output_identity)
        && limits.items > 0
        && limits.items <= MAX_ARTIFACT_ITEMS
        && limits.input_bytes > 0
        && limits.input_bytes <= MAX_ARTIFACT_INPUT_BYTES
        && limits.range_units > 0
        && limits.range_units <= MAX_ARTIFACT_RANGE
        && limits.output_bytes > 0
        && limits.output_bytes <= MAX_ARTIFACT_OUTPUT_BYTES
        && limits.time_ms > 0
        && limits.time_ms <= MAX_ARTIFACT_TIME_MS
        && limits.memory_bytes > 0
        && limits.memory_bytes <= MAX_ARTIFACT_MEMORY_BYTES
        && limits.tasks > 0
        && limits.tasks <= MAX_ARTIFACT_TASKS
        && request.call_depth <= MAX_ARTIFACT_CALL_DEPTH
        && request.source_id.as_deref().is_none_or(valid_identity)
        && request.section_id.as_deref().is_none_or(valid_identity)
        && request.freshness_sha256.as_deref().is_none_or(valid_sha256);
    if !common
        || request
            .range
            .as_ref()
            .is_some_and(|range| !valid_range(range, limits.range_units))
    {
        return false;
    }
    match kind {
        ArtifactToolKind::List => {
            request.source_id.is_none()
                && request.section_id.is_none()
                && request.range.is_none()
                && request.query.is_none()
                && request.freshness_sha256.is_none()
        }
        ArtifactToolKind::Metadata | ArtifactToolKind::Sections | ArtifactToolKind::LogErrors => {
            request.source_id.is_some()
                && request.freshness_sha256.is_some()
                && request.section_id.is_none()
                && request.range.is_none()
                && request.query.is_none()
        }
        ArtifactToolKind::Read => {
            request.source_id.is_some()
                && request.freshness_sha256.is_some()
                && request.range.is_none()
                && request.query.is_none()
        }
        ArtifactToolKind::Range => {
            request.source_id.is_some()
                && request.freshness_sha256.is_some()
                && request.section_id.is_none()
                && request.range.is_some()
                && request.query.is_none()
        }
        ArtifactToolKind::Search => {
            request.source_id.is_some()
                && request.freshness_sha256.is_some()
                && request.section_id.is_none()
                && request.range.is_none()
                && request
                    .query
                    .as_deref()
                    .is_some_and(|query| !query.is_empty() && query.len() <= 4096)
        }
    }
}

fn valid_backend(backend: &impl ArtifactBackend) -> bool {
    let manifests = backend.manifests();
    if manifests
        .windows(2)
        .any(|pair| pair[0].source_id >= pair[1].source_id)
    {
        return false;
    }
    manifests.iter().all(|manifest| {
        backend
            .source(&manifest.source_id)
            .is_some_and(|source| source.manifest == *manifest && valid_source(&source))
    })
}

fn diagnostic_fragment(fragment: &ArtifactFragment) -> bool {
    let content = fragment_content(fragment).trim().to_lowercase();
    content.contains("error")
        || content.contains("warning")
        || content.contains("failed")
        || content.contains("failure")
        || content.contains("exception")
        || content.contains("panic")
        || content.starts_with("at ")
        || content.starts_with("file ")
        || (content.starts_with("test ") && (content.contains(" ok") || content.contains("fail")))
}

fn valid_range(range: &ArtifactRange, maximum: u64) -> bool {
    match range {
        ArtifactRange::Byte {
            start,
            end_exclusive,
        }
        | ArtifactRange::Line {
            start,
            end_exclusive,
        }
        | ArtifactRange::Page {
            start,
            end_exclusive,
        } => {
            *start < *end_exclusive
                && end_exclusive - start <= maximum
                && !matches!(
                    range,
                    ArtifactRange::Line { start: 0, .. } | ArtifactRange::Page { start: 0, .. }
                )
        }
        ArtifactRange::Sheet { name } => valid_identity(name),
        ArtifactRange::Cell {
            sheet,
            start_row,
            start_column,
            end_row,
            end_column,
        } => {
            valid_identity(sheet)
                && *start_row > 0
                && *start_column > 0
                && start_row <= end_row
                && start_column <= end_column
                && (end_row - start_row + 1).saturating_mul(end_column - start_column + 1)
                    <= maximum
        }
        ArtifactRange::Section { section_id } => valid_identity(section_id),
    }
}

fn valid_source(source: &FakeArtifactSource) -> bool {
    let manifest = &source.manifest;
    valid_identity(&manifest.source_id)
        && valid_identity(&manifest.provenance.provenance_id)
        && valid_sha256(&manifest.payload_sha256)
        && valid_sha256(&manifest.provenance.protected_origin_sha256)
        && valid_sha256(&manifest.freshness.manifest_sha256)
        && (manifest.extraction_state == ArtifactExtractionState::Complete
            || manifest.reason_code.as_deref().is_some_and(valid_identity))
        && source
            .sections
            .windows(2)
            .all(|pair| pair[0].ordinal < pair[1].ordinal)
        && source
            .sections
            .iter()
            .all(|section| valid_identity(&section.section_id))
}

fn range_matches(range: &ArtifactRange, fragment: &ArtifactFragment) -> bool {
    match (range, fragment) {
        (
            ArtifactRange::Byte {
                start,
                end_exclusive,
            },
            ArtifactFragment::Byte {
                start: actual_start,
                end_exclusive: actual_end,
                ..
            },
        )
        | (
            ArtifactRange::Line {
                start,
                end_exclusive,
            },
            ArtifactFragment::Line {
                start: actual_start,
                end_exclusive: actual_end,
                ..
            },
        ) => actual_start >= start && actual_end <= end_exclusive,
        (
            ArtifactRange::Page {
                start,
                end_exclusive,
            },
            ArtifactFragment::Page { number, .. },
        ) => number >= start && number.saturating_add(1) <= *end_exclusive,
        (ArtifactRange::Sheet { name }, ArtifactFragment::Sheet { name: actual, .. }) => {
            name == actual
        }
        (
            ArtifactRange::Cell {
                sheet,
                start_row,
                start_column,
                end_row,
                end_column,
            },
            ArtifactFragment::Cell {
                sheet: actual_sheet,
                row,
                column,
                ..
            },
        ) => {
            sheet == actual_sheet
                && row >= start_row
                && row <= end_row
                && column >= start_column
                && column <= end_column
        }
        (
            ArtifactRange::Section { section_id },
            ArtifactFragment::Section {
                section_id: actual, ..
            },
        ) => section_id == actual,
        _ => false,
    }
}

fn fragment_content(fragment: &ArtifactFragment) -> &str {
    match fragment {
        ArtifactFragment::Byte { content, .. }
        | ArtifactFragment::Line { content, .. }
        | ArtifactFragment::Page { content, .. }
        | ArtifactFragment::Sheet { content, .. }
        | ArtifactFragment::Cell { content, .. }
        | ArtifactFragment::Section { content, .. } => content,
    }
}

fn valid_identity(value: &str) -> bool {
    !value.is_empty() && value.len() <= 255 && !value.chars().any(char::is_control)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn receipt_digest(receipt: &ArtifactReceipt) -> String {
    let mut copy = receipt.clone();
    copy.receipt_sha256 = "0".repeat(64);
    sha256_hex(&serde_json::to_vec(&copy).expect("receipt serialization"))
}

fn schema(id: &str, bytes: &str) -> SchemaReference {
    SchemaReference {
        schema_id: SchemaId::from_raw(id),
        schema_version: 1,
        schema_sha256: sha256_hex(bytes.as_bytes()),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("String write");
    }
    output
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{GrantOperation, ToolId};

    use super::*;

    fn digest(character: char) -> String {
        character.to_string().repeat(64)
    }

    fn source(
        id: &str,
        classification: ArtifactClassification,
        extraction_state: ArtifactExtractionState,
    ) -> FakeArtifactSource {
        FakeArtifactSource {
            manifest: ArtifactManifest {
                source_id: id.to_owned(),
                media_type: "text/plain".to_owned(),
                byte_len: 32,
                payload_sha256: digest('a'),
                classification,
                extraction_state,
                provenance: ArtifactProvenance {
                    provenance_id: format!("provenance-{id}"),
                    source_kind: "upload".to_owned(),
                    protected_origin_sha256: digest('b'),
                },
                freshness: ArtifactFreshness {
                    manifest_sha256: digest('c'),
                    observed_sequence: 7,
                },
                reason_code: (extraction_state != ArtifactExtractionState::Complete)
                    .then(|| "artifact.extraction.not-complete".to_owned()),
            },
            sections: vec![ArtifactSection {
                section_id: "intro".to_owned(),
                title: "Introduction".to_owned(),
                ordinal: 1,
                byte_len: 11,
            }],
            fragments: vec![
                ArtifactFragment::Byte {
                    start: 0,
                    end_exclusive: 11,
                    content: "hello world".to_owned(),
                },
                ArtifactFragment::Line {
                    start: 1,
                    end_exclusive: 2,
                    content: "hello world".to_owned(),
                },
                ArtifactFragment::Line {
                    start: 2,
                    end_exclusive: 3,
                    content: "ERROR: deterministic fixture failure".to_owned(),
                },
                ArtifactFragment::Page {
                    number: 1,
                    content: "hello page".to_owned(),
                },
                ArtifactFragment::Sheet {
                    name: "Sheet1".to_owned(),
                    content: "hello sheet".to_owned(),
                },
                ArtifactFragment::Cell {
                    sheet: "Sheet1".to_owned(),
                    row: 1,
                    column: 1,
                    content: "hello cell".to_owned(),
                },
                ArtifactFragment::Section {
                    section_id: "intro".to_owned(),
                    content: "hello intro".to_owned(),
                },
            ],
            redacted: false,
        }
    }

    fn request(kind: ArtifactToolKind, call_id: &str) -> ArtifactRequest {
        ArtifactRequest {
            schema_version: 1,
            call_id: call_id.to_owned(),
            source_id: (kind != ArtifactToolKind::List).then(|| "source-1".to_owned()),
            section_id: None,
            range: (kind == ArtifactToolKind::Range).then_some(ArtifactRange::Byte {
                start: 0,
                end_exclusive: 12,
            }),
            query: (kind == ArtifactToolKind::Search).then(|| "hello".to_owned()),
            freshness_sha256: (kind != ArtifactToolKind::List).then(|| digest('c')),
            output_identity: format!("output-{call_id}"),
            limits: ArtifactLimits::default(),
            call_depth: 0,
        }
    }

    fn dispatch(
        kind: ArtifactToolKind,
        request: &ArtifactRequest,
        backend: &FakeArtifactBackend,
        ledger: &mut ArtifactAttemptLedger,
    ) -> Result<ArtifactResult, ArtifactDispatchError> {
        dispatch_artifact(
            kind,
            &serde_json::to_vec(request).expect("request"),
            backend,
            true,
            ArtifactExecutionSignal::Continue,
            ledger,
        )
    }

    #[test]
    fn catalog_is_exact_closed_read_only_and_extensions_are_unregistered() {
        let definitions = artifact_tool_definitions();
        assert_eq!(definitions.len(), 7);
        for (kind, definition) in ArtifactToolKind::ALL.into_iter().zip(definitions) {
            assert_eq!(definition.tool_id.as_str(), kind.id());
            assert_eq!(definition.tool_version, ARTIFACT_TOOL_VERSION);
            assert_eq!(
                definition.input_schema.schema_id.as_str(),
                ARTIFACT_INPUT_SCHEMA_ID
            );
            assert_eq!(
                definition.input_schema.schema_sha256,
                sha256_hex(ARTIFACT_INPUT_SCHEMA_JSON.as_bytes())
            );
            assert_eq!(
                definition.output_schema.schema_id.as_str(),
                ARTIFACT_OUTPUT_SCHEMA_ID
            );
            assert_eq!(
                definition.output_schema.schema_sha256,
                sha256_hex(ARTIFACT_OUTPUT_SCHEMA_JSON.as_bytes())
            );
            assert_eq!(
                definition.required_grant.operation.operation(),
                GrantOperation::WorkspaceRead
            );
            assert!(definition.required_grant.single_use);
        }
        for schema_json in [ARTIFACT_INPUT_SCHEMA_JSON, ARTIFACT_OUTPUT_SCHEMA_JSON] {
            let schema: serde_json::Value = serde_json::from_str(schema_json).expect("JSON schema");
            assert_eq!(
                schema["$schema"],
                "https://json-schema.org/draft/2020-12/schema"
            );
            assert_eq!(schema["additionalProperties"], false);
        }
        for future in ["artifact.get_page", "artifact.get_sheet"] {
            assert!(artifact_tool_kind(&ToolId::from_raw(future), ARTIFACT_TOOL_VERSION).is_none());
        }
        assert!(artifact_tool_kind(&ToolId::from_raw("artifact.list"), "2.0.0").is_none());
    }

    #[test]
    fn all_seven_fake_operations_are_deterministic_typed_and_receipted() {
        let backend = FakeArtifactBackend::new(vec![source(
            "source-1",
            ArtifactClassification::Internal,
            ArtifactExtractionState::Complete,
        )])
        .expect("backend");
        let mut first = Vec::new();
        let mut second = Vec::new();
        for (round, target) in [("a", &mut first), ("b", &mut second)] {
            let mut ledger = ArtifactAttemptLedger::default();
            for kind in ArtifactToolKind::ALL {
                let result = dispatch(
                    kind,
                    &request(kind, &format!("{round}-{}", kind.id())),
                    &backend,
                    &mut ledger,
                )
                .expect("dispatch");
                assert!(result.verify(kind));
                assert_eq!(result.outcome, ArtifactOutcome::Succeeded);
                assert!(!result.receipt.parser_launched);
                assert!(!result.receipt.network_accessed);
                assert!(!result.receipt.workspace_mutated);
                let mut normalized = result;
                normalized.call_id.clear();
                normalized.output_identity.clear();
                normalized.receipt.call_id.clear();
                normalized.receipt.output_identity.clear();
                normalized.receipt.receipt_sha256.clear();
                target.push(normalized);
            }
            assert_eq!(ledger.len(), 7);
        }
        assert_eq!(first, second);
    }

    #[test]
    fn byte_line_page_sheet_cell_section_and_search_projections_are_exact() {
        let backend = FakeArtifactBackend::new(vec![source(
            "source-1",
            ArtifactClassification::Internal,
            ArtifactExtractionState::Complete,
        )])
        .expect("backend");
        let ranges = [
            ArtifactRange::Byte {
                start: 0,
                end_exclusive: 12,
            },
            ArtifactRange::Line {
                start: 1,
                end_exclusive: 2,
            },
            ArtifactRange::Page {
                start: 1,
                end_exclusive: 2,
            },
            ArtifactRange::Sheet {
                name: "Sheet1".to_owned(),
            },
            ArtifactRange::Cell {
                sheet: "Sheet1".to_owned(),
                start_row: 1,
                start_column: 1,
                end_row: 1,
                end_column: 1,
            },
            ArtifactRange::Section {
                section_id: "intro".to_owned(),
            },
        ];
        let mut ledger = ArtifactAttemptLedger::default();
        for (index, range) in ranges.into_iter().enumerate() {
            let mut value = request(ArtifactToolKind::Range, &format!("range-{index}"));
            value.range = Some(range);
            let result =
                dispatch(ArtifactToolKind::Range, &value, &backend, &mut ledger).expect("range");
            assert_eq!(result.items.len(), 1);
        }
        let search = dispatch(
            ArtifactToolKind::Search,
            &request(ArtifactToolKind::Search, "search"),
            &backend,
            &mut ledger,
        )
        .expect("search");
        assert_eq!(search.items.len(), 6);
    }

    #[test]
    fn malformed_extra_oversized_unauthorized_and_repeat_fail_before_launch() {
        let backend = FakeArtifactBackend::new(vec![source(
            "source-1",
            ArtifactClassification::Internal,
            ArtifactExtractionState::Complete,
        )])
        .expect("backend");
        let mut ledger = ArtifactAttemptLedger::default();
        for bytes in [
            b"{}".as_slice(),
            br#"{"schema_version":1,"extra":true}"#,
            &[b'x'; MAX_ARTIFACT_REQUEST_BYTES + 1],
        ] {
            assert!(
                dispatch_artifact(
                    ArtifactToolKind::List,
                    bytes,
                    &backend,
                    true,
                    ArtifactExecutionSignal::Continue,
                    &mut ledger
                )
                .is_err()
            );
        }
        let value = request(ArtifactToolKind::Metadata, "once");
        let bytes = serde_json::to_vec(&value).expect("request");
        assert_eq!(
            dispatch_artifact(
                ArtifactToolKind::Metadata,
                &bytes,
                &backend,
                false,
                ArtifactExecutionSignal::Continue,
                &mut ledger
            ),
            Err(ArtifactDispatchError::Unauthorized)
        );
        dispatch_artifact(
            ArtifactToolKind::Metadata,
            &bytes,
            &backend,
            true,
            ArtifactExecutionSignal::Continue,
            &mut ledger,
        )
        .expect("first");
        assert_eq!(
            dispatch_artifact(
                ArtifactToolKind::Metadata,
                &bytes,
                &backend,
                true,
                ArtifactExecutionSignal::Continue,
                &mut ledger
            ),
            Err(ArtifactDispatchError::RepeatedCall)
        );
        assert_eq!(ledger.len(), 1);
    }

    #[test]
    fn stale_restricted_unsupported_redacted_missing_and_range_fail_closed() {
        let sources = vec![
            source(
                "source-1",
                ArtifactClassification::Internal,
                ArtifactExtractionState::Complete,
            ),
            source(
                "restricted",
                ArtifactClassification::Restricted,
                ArtifactExtractionState::Complete,
            ),
            source(
                "unsupported",
                ArtifactClassification::Internal,
                ArtifactExtractionState::Unsupported,
            ),
            {
                let mut value = source(
                    "redacted",
                    ArtifactClassification::Internal,
                    ArtifactExtractionState::Complete,
                );
                value.redacted = true;
                value
            },
        ];
        let backend = FakeArtifactBackend::new(sources).expect("backend");
        let mut ledger = ArtifactAttemptLedger::default();
        let mut expected = Vec::new();
        for (id, freshness, outcome) in [
            ("source-1", digest('d'), ArtifactOutcome::Stale),
            ("restricted", digest('c'), ArtifactOutcome::Restricted),
            ("unsupported", digest('c'), ArtifactOutcome::Unsupported),
            ("redacted", digest('c'), ArtifactOutcome::Denied),
            ("missing", digest('c'), ArtifactOutcome::NoResult),
        ] {
            let mut value = request(ArtifactToolKind::Metadata, &format!("case-{id}"));
            value.source_id = Some(id.to_owned());
            value.freshness_sha256 = Some(freshness);
            let result = dispatch(ArtifactToolKind::Metadata, &value, &backend, &mut ledger)
                .expect("closed result");
            assert_eq!(result.outcome, outcome);
            assert!(result.items.is_empty() || outcome == ArtifactOutcome::Succeeded);
            assert!(result.verify(ArtifactToolKind::Metadata));
            expected.push(outcome);
        }
        let mut invalid = request(ArtifactToolKind::Range, "invalid-range");
        invalid.range = Some(ArtifactRange::Line {
            start: 0,
            end_exclusive: 1,
        });
        assert_eq!(
            dispatch(ArtifactToolKind::Range, &invalid, &backend, &mut ledger),
            Err(ArtifactDispatchError::Denied)
        );
    }

    #[test]
    fn cancellation_timeout_and_crash_each_get_one_terminal_receipt() {
        let backend = FakeArtifactBackend::new(vec![source(
            "source-1",
            ArtifactClassification::Internal,
            ArtifactExtractionState::Complete,
        )])
        .expect("backend");
        let mut ledger = ArtifactAttemptLedger::default();
        for (index, (signal, outcome)) in [
            (
                ArtifactExecutionSignal::Cancelled,
                ArtifactOutcome::Cancelled,
            ),
            (ArtifactExecutionSignal::TimedOut, ArtifactOutcome::TimedOut),
            (ArtifactExecutionSignal::Crashed, ArtifactOutcome::Failed),
        ]
        .into_iter()
        .enumerate()
        {
            let value = request(ArtifactToolKind::Metadata, &format!("signal-{index}"));
            let result = dispatch_artifact(
                ArtifactToolKind::Metadata,
                &serde_json::to_vec(&value).expect("request"),
                &backend,
                true,
                signal,
                &mut ledger,
            )
            .expect("terminal result");
            assert_eq!(result.outcome, outcome);
            assert!(result.verify(ArtifactToolKind::Metadata));
        }
        assert_eq!(ledger.len(), 3);
    }

    #[test]
    fn every_tool_rejects_the_closed_schema_and_repeat_matrix_before_a_second_launch() {
        let backend = FakeArtifactBackend::new(vec![source(
            "source-1",
            ArtifactClassification::Internal,
            ArtifactExtractionState::Complete,
        )])
        .expect("backend");
        let mut ledger = ArtifactAttemptLedger::default();
        for kind in ArtifactToolKind::ALL {
            let value = request(kind, &format!("matrix-{}", kind.id()));
            let valid = serde_json::to_vec(&value).expect("valid request");
            assert!(validate_artifact_request(kind, &valid).is_ok());
            assert_eq!(
                validate_artifact_request(kind, b"{}"),
                Err(ArtifactDispatchError::Malformed)
            );
            assert_eq!(
                validate_artifact_request(kind, b"{"),
                Err(ArtifactDispatchError::Malformed)
            );

            let mut extra = serde_json::to_value(&value).expect("value");
            extra
                .as_object_mut()
                .expect("object")
                .insert("extra".to_owned(), serde_json::Value::Bool(true));
            assert_eq!(
                validate_artifact_request(kind, &serde_json::to_vec(&extra).expect("extra")),
                Err(ArtifactDispatchError::Malformed)
            );

            let valid_text = String::from_utf8(valid.clone()).expect("UTF-8 request");
            let duplicate = valid_text.replacen(
                "\"schema_version\":1",
                "\"schema_version\":1,\"schema_version\":1",
                1,
            );
            assert_eq!(
                validate_artifact_request(kind, duplicate.as_bytes()),
                Err(ArtifactDispatchError::Malformed)
            );
            assert_eq!(
                validate_artifact_request(kind, &[b'x'; MAX_ARTIFACT_REQUEST_BYTES + 1]),
                Err(ArtifactDispatchError::Malformed)
            );

            let mut out_of_budget = value.clone();
            out_of_budget.limits.tasks = 0;
            assert_eq!(
                validate_artifact_request(
                    kind,
                    &serde_json::to_vec(&out_of_budget).expect("out of budget")
                ),
                Err(ArtifactDispatchError::Denied)
            );

            dispatch_artifact(
                kind,
                &valid,
                &backend,
                true,
                ArtifactExecutionSignal::Continue,
                &mut ledger,
            )
            .expect("one launch");
            assert_eq!(
                dispatch_artifact(
                    kind,
                    &valid,
                    &backend,
                    true,
                    ArtifactExecutionSignal::Continue,
                    &mut ledger,
                ),
                Err(ArtifactDispatchError::RepeatedCall)
            );
        }
        assert_eq!(ledger.len(), ArtifactToolKind::ALL.len());
    }

    #[test]
    fn cancellation_timeout_and_crash_are_receipted_for_every_tool() {
        let backend = FakeArtifactBackend::new(vec![source(
            "source-1",
            ArtifactClassification::Internal,
            ArtifactExtractionState::Complete,
        )])
        .expect("backend");
        let mut ledger = ArtifactAttemptLedger::default();
        for kind in ArtifactToolKind::ALL {
            for (suffix, signal, outcome) in [
                (
                    "cancel",
                    ArtifactExecutionSignal::Cancelled,
                    ArtifactOutcome::Cancelled,
                ),
                (
                    "timeout",
                    ArtifactExecutionSignal::TimedOut,
                    ArtifactOutcome::TimedOut,
                ),
                (
                    "crash",
                    ArtifactExecutionSignal::Crashed,
                    ArtifactOutcome::Failed,
                ),
            ] {
                let value = request(kind, &format!("{}-{suffix}", kind.id()));
                let result = dispatch_artifact(
                    kind,
                    &serde_json::to_vec(&value).expect("request"),
                    &backend,
                    true,
                    signal,
                    &mut ledger,
                )
                .expect("terminal result");
                assert_eq!(result.outcome, outcome);
                assert!(result.verify(kind));
            }
        }
        assert_eq!(ledger.len(), ArtifactToolKind::ALL.len() * 3);
    }

    #[test]
    fn result_limits_truncate_or_return_large_reference_without_hidden_omission() {
        let backend = FakeArtifactBackend::new(vec![source(
            "source-1",
            ArtifactClassification::Internal,
            ArtifactExtractionState::Complete,
        )])
        .expect("backend");
        let mut value = request(ArtifactToolKind::Search, "limited");
        value.limits.items = 2;
        let mut ledger = ArtifactAttemptLedger::default();
        let result =
            dispatch(ArtifactToolKind::Search, &value, &backend, &mut ledger).expect("limited");
        assert_eq!(result.outcome, ArtifactOutcome::Truncated);
        assert_eq!(result.items.len(), 2);
        assert!(result.verify(ArtifactToolKind::Search));

        let mut outside = request(ArtifactToolKind::Range, "outside");
        outside.range = Some(ArtifactRange::Byte {
            start: 100,
            end_exclusive: 101,
        });
        let result = dispatch(ArtifactToolKind::Range, &outside, &backend, &mut ledger)
            .expect("out of range");
        assert_eq!(result.outcome, ArtifactOutcome::OutOfRange);
        assert!(result.verify(ArtifactToolKind::Range));
    }
}
