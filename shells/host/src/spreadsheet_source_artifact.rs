//! Runtime-owned spreadsheet preparation and common native artifact-tool projection.

use std::collections::BTreeMap;

use agentmage_capability_knowledge::SpreadsheetStructuredSourceExtractor;
use agentmage_capability_read_only::{
    ArtifactAttemptLedger, ArtifactBackend, ArtifactBackendClass, ArtifactClassification,
    ArtifactDispatchError, ArtifactExecutionSignal, ArtifactExtensionError,
    ArtifactExtractorExtensions, ArtifactFragment, ArtifactFreshness, ArtifactManifest,
    ArtifactProvenance, ArtifactResult, ArtifactSection, ArtifactToolKind, FakeArtifactSource,
    dispatch_artifact,
};
use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, RuntimeArtifactRef, StructuredSourceExtraction,
    StructuredSourceExtractionError, StructuredSourceExtractionRequest, StructuredSourceExtractor,
    StructuredSourceSectionKind, StructuredSourceWarning, XLSX_MEDIA_TYPE,
};
use agentmage_kernel_engine::source_preparation::{
    ExactSourceTokenCounter, SourcePreparationError,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAX_PREPARED_SPREADSHEET_SOURCES: usize = 1_024;

/// Closed lifecycle of one prepared spreadsheet projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreparedSpreadsheetLifecycle {
    /// Projection is current and retrievable.
    Current,
    /// Source or parser identity changed and retrieval is refused.
    Stale,
    /// Projection content was released while content-free lineage remains.
    Released,
    /// Released lineage was explicitly tombstoned.
    Deleted,
}

/// Exact retention binding; this service never stores a second workbook copy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PreparedSpreadsheetRetention {
    /// Projection exists only in bounded process memory.
    Ephemeral,
    /// Original bytes already exist in the encrypted runtime artifact authority.
    PolicyPersisted {
        /// Path-free content-addressed original-payload reference.
        artifact: RuntimeArtifactRef,
        /// Exact policy revision authorizing persistence.
        policy_sha256: String,
    },
}

/// One content-free exact sheet summary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedSpreadsheetSheetSummary {
    /// Exact sheet name.
    pub name: String,
    /// Exact package part.
    pub source_part: String,
    /// One-based workbook order.
    pub sheet_number: u32,
    /// Declared used range when present.
    pub dimension: Option<String>,
    /// Retained populated cell count.
    pub cell_count: u32,
    /// Exact declared visibility.
    pub state: String,
    /// Whether worksheet protection was declared.
    pub protected: bool,
    /// Whether the declared range was intentionally not expanded.
    pub sparse_dimension: bool,
}

/// One bounded content-free lexical-index entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedSpreadsheetLexicalEntry {
    /// Unicode-lowercase alphanumeric term.
    pub term: String,
    /// Canonical section identities containing the term.
    pub section_ids: Vec<String>,
}

/// Digest-sealed prepared workbook manifest; original bytes are not retained by this service.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedSpreadsheetSourceManifest {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable source identity.
    pub source_id: String,
    /// Exact source digest.
    pub source_sha256: String,
    /// Complete source byte count.
    pub source_bytes: u64,
    /// Exact extractor identity.
    pub extractor_sha256: String,
    /// Digest of the complete canonical projection.
    pub extraction_sha256: String,
    /// Canonical section count.
    pub section_count: u32,
    /// Emitted canonical text bytes.
    pub output_bytes: u64,
    /// Complete ordered sheet summaries.
    pub sheets: Vec<PreparedSpreadsheetSheetSummary>,
    /// Complete extraction and policy warnings.
    pub warnings: Vec<StructuredSourceWarning>,
    /// Number of distinct normalized lexical terms.
    pub lexical_term_count: u32,
    /// Digest of the complete content-free lexical index.
    pub lexical_index_sha256: String,
    /// Current prepared-projection lifecycle.
    pub lifecycle: PreparedSpreadsheetLifecycle,
    /// Existing original-byte retention or ephemeral projection-only state.
    pub retention: PreparedSpreadsheetRetention,
    /// Sensitivity fixed before projection.
    pub classification: ArtifactClassification,
    /// Content-free source kind.
    pub source_kind: String,
    /// Digest of protected origin metadata.
    pub protected_origin_sha256: String,
    /// Monotonic logical revision.
    pub revision: u64,
    /// Digest of this complete record with this field zeroed.
    pub manifest_sha256: String,
}

/// One completed workbook admission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpreadsheetSourceAdmissionOutcome {
    /// Common native artifact manifest.
    pub manifest: ArtifactManifest,
    /// Spreadsheet-specific prepared manifest.
    pub prepared_manifest: PreparedSpreadsheetSourceManifest,
    /// True only for exact source, extractor, classification, and origin reuse.
    pub cache_hit: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PreparedSpreadsheetSource {
    extraction: Option<StructuredSourceExtraction>,
    lexical_index: BTreeMap<String, Vec<String>>,
    manifest: ArtifactManifest,
    prepared_manifest: PreparedSpreadsheetSourceManifest,
}

/// Bounded runtime-owned prepared spreadsheet service.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpreadsheetSourceArtifactService {
    sources: BTreeMap<String, PreparedSpreadsheetSource>,
}

/// Exact per-source token accounting for combined spreadsheet context.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpreadsheetContextAccounting {
    /// Stable source identity.
    pub source_id: String,
    /// Exact current prepared-manifest digest.
    pub manifest_sha256: String,
    /// Exact tokens counted from selected candidates.
    pub token_count: u32,
    /// Selected candidate count.
    pub candidate_count: u32,
}

/// Complete bounded combined-artifact accounting.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombinedSpreadsheetContextAccounting {
    /// Stable exact-token-counter identity.
    pub token_counter_id: String,
    /// Digest of the exact counter implementation.
    pub token_counter_sha256: String,
    /// Digest of the exact profile tokenizer.
    pub tokenizer_sha256: String,
    /// Total selected tokens across all sources.
    pub used_tokens: u32,
    /// Caller-supplied hard token ceiling.
    pub maximum_tokens: u32,
    /// Records in caller source order.
    pub sources: Vec<SpreadsheetContextAccounting>,
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && value.is_ascii()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256(value: &[u8]) -> String {
    Sha256::digest(value)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn manifest_sha256(
    manifest: &PreparedSpreadsheetSourceManifest,
) -> Result<String, StructuredSourceExtractionError> {
    let mut candidate = manifest.clone();
    candidate.manifest_sha256 = "0".repeat(64);
    serde_json::to_vec(&candidate)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| StructuredSourceExtractionError::InvalidInput)
}

fn valid_retention(
    retention: &PreparedSpreadsheetRetention,
    request: &StructuredSourceExtractionRequest,
    source_bytes: u64,
) -> bool {
    match retention {
        PreparedSpreadsheetRetention::Ephemeral => true,
        PreparedSpreadsheetRetention::PolicyPersisted {
            artifact,
            policy_sha256,
        } => {
            artifact.schema_version == CONTRACT_SCHEMA_VERSION
                && valid_id(artifact.artifact_id.as_str())
                && valid_sha256(&artifact.manifest_sha256)
                && artifact.payload_sha256 == request.source_sha256
                && artifact.byte_size == source_bytes
                && artifact.media_type == request.media_type
                && valid_sha256(policy_sha256)
        }
    }
}

fn sheet_summaries(
    extraction: &StructuredSourceExtraction,
) -> Result<Vec<PreparedSpreadsheetSheetSummary>, StructuredSourceExtractionError> {
    let mut summaries = Vec::new();
    for section in extraction
        .sections
        .iter()
        .filter(|section| section.kind == StructuredSourceSectionKind::Table)
    {
        let value: serde_json::Value = serde_json::from_str(&section.content)
            .map_err(|_| StructuredSourceExtractionError::Malformed)?;
        let sheet_number = section
            .provenance
            .table
            .ok_or(StructuredSourceExtractionError::Malformed)?;
        let cell_count = extraction
            .sections
            .iter()
            .filter(|candidate| {
                candidate.kind == StructuredSourceSectionKind::TableCell
                    && candidate.provenance.table == Some(sheet_number)
            })
            .count();
        summaries.push(PreparedSpreadsheetSheetSummary {
            name: value["name"]
                .as_str()
                .ok_or(StructuredSourceExtractionError::Malformed)?
                .to_owned(),
            source_part: section.provenance.source_part.clone(),
            sheet_number,
            dimension: value["dimension"].as_str().map(str::to_owned),
            cell_count: u32::try_from(cell_count)
                .map_err(|_| StructuredSourceExtractionError::ResourceLimit)?,
            state: value["state"]
                .as_str()
                .ok_or(StructuredSourceExtractionError::Malformed)?
                .to_owned(),
            protected: value["protected"]
                .as_bool()
                .ok_or(StructuredSourceExtractionError::Malformed)?,
            sparse_dimension: value["sparse_dimension"]
                .as_bool()
                .ok_or(StructuredSourceExtractionError::Malformed)?,
        });
    }
    Ok(summaries)
}

fn lexical_index(extraction: &StructuredSourceExtraction) -> BTreeMap<String, Vec<String>> {
    let mut index = BTreeMap::<String, Vec<String>>::new();
    for section in &extraction.sections {
        for term in section
            .content
            .split(|character: char| !character.is_alphanumeric())
            .filter(|term| !term.is_empty())
            .map(str::to_lowercase)
        {
            let section_ids = index.entry(term).or_default();
            if section_ids.last() != Some(&section.section_id) {
                section_ids.push(section.section_id.clone());
            }
        }
    }
    index
}

impl SpreadsheetSourceArtifactService {
    /// Admits one exact XLSX and retains only its canonical projection and manifests.
    pub fn admit(
        &mut self,
        request: &StructuredSourceExtractionRequest,
        source: &[u8],
        classification: ArtifactClassification,
        source_kind: &str,
        protected_origin_sha256: &str,
        cancelled: &mut dyn FnMut() -> bool,
    ) -> Result<SpreadsheetSourceAdmissionOutcome, StructuredSourceExtractionError> {
        self.admit_with_retention(
            request,
            source,
            classification,
            source_kind,
            protected_origin_sha256,
            PreparedSpreadsheetRetention::Ephemeral,
            cancelled,
        )
    }

    /// Admits one exact XLSX bound to an existing original-payload retention decision.
    #[allow(clippy::too_many_arguments)]
    pub fn admit_with_retention(
        &mut self,
        request: &StructuredSourceExtractionRequest,
        source: &[u8],
        classification: ArtifactClassification,
        source_kind: &str,
        protected_origin_sha256: &str,
        retention: PreparedSpreadsheetRetention,
        cancelled: &mut dyn FnMut() -> bool,
    ) -> Result<SpreadsheetSourceAdmissionOutcome, StructuredSourceExtractionError> {
        if request.media_type != XLSX_MEDIA_TYPE
            || !valid_id(source_kind)
            || !valid_sha256(protected_origin_sha256)
            || !valid_retention(&retention, request, source.len() as u64)
        {
            return Err(StructuredSourceExtractionError::InvalidInput);
        }
        let extractor = SpreadsheetStructuredSourceExtractor;
        let extractor_sha256 = extractor.extractor_sha256();
        if let Some(current) = self.sources.get(&request.source_id)
            && current.prepared_manifest.source_sha256 == request.source_sha256
            && current.prepared_manifest.extractor_sha256 == extractor_sha256
            && current.prepared_manifest.classification == classification
            && current.prepared_manifest.source_kind == source_kind
            && current.prepared_manifest.protected_origin_sha256 == protected_origin_sha256
            && current.prepared_manifest.retention == retention
            && current.prepared_manifest.lifecycle == PreparedSpreadsheetLifecycle::Current
        {
            if cancelled() {
                return Err(StructuredSourceExtractionError::Cancelled);
            }
            return Ok(SpreadsheetSourceAdmissionOutcome {
                manifest: current.manifest.clone(),
                prepared_manifest: current.prepared_manifest.clone(),
                cache_hit: true,
            });
        }
        if self.sources.len() >= MAX_PREPARED_SPREADSHEET_SOURCES
            && !self.sources.contains_key(&request.source_id)
        {
            return Err(StructuredSourceExtractionError::ResourceLimit);
        }
        let extraction = extractor.extract(request, source, cancelled)?;
        let revision = self
            .sources
            .get(&request.source_id)
            .map_or(1, |current| current.prepared_manifest.revision + 1);
        let extraction_sha256 = serde_json::to_vec(&extraction)
            .map(|bytes| sha256(&bytes))
            .map_err(|_| StructuredSourceExtractionError::InvalidInput)?;
        let lexical_index = lexical_index(&extraction);
        let lexical_index_sha256 = serde_json::to_vec(&lexical_index)
            .map(|bytes| sha256(&bytes))
            .map_err(|_| StructuredSourceExtractionError::InvalidInput)?;
        let mut prepared_manifest = PreparedSpreadsheetSourceManifest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            source_id: request.source_id.clone(),
            source_sha256: request.source_sha256.clone(),
            source_bytes: source.len() as u64,
            extractor_sha256,
            extraction_sha256,
            section_count: u32::try_from(extraction.sections.len())
                .map_err(|_| StructuredSourceExtractionError::ResourceLimit)?,
            output_bytes: extraction.output_bytes,
            sheets: sheet_summaries(&extraction)?,
            warnings: extraction.warnings.clone(),
            lexical_term_count: u32::try_from(lexical_index.len())
                .map_err(|_| StructuredSourceExtractionError::ResourceLimit)?,
            lexical_index_sha256,
            lifecycle: PreparedSpreadsheetLifecycle::Current,
            retention,
            classification,
            source_kind: source_kind.to_owned(),
            protected_origin_sha256: protected_origin_sha256.to_owned(),
            revision,
            manifest_sha256: "0".repeat(64),
        };
        prepared_manifest.manifest_sha256 = manifest_sha256(&prepared_manifest)?;
        let manifest = artifact_manifest(&prepared_manifest);
        self.sources.insert(
            request.source_id.clone(),
            PreparedSpreadsheetSource {
                extraction: Some(extraction),
                lexical_index,
                manifest: manifest.clone(),
                prepared_manifest: prepared_manifest.clone(),
            },
        );
        Ok(SpreadsheetSourceAdmissionOutcome {
            manifest,
            prepared_manifest,
            cache_hit: false,
        })
    }

    /// Returns one exact current content-free source manifest.
    #[must_use]
    pub fn prepared_manifest(&self, source_id: &str) -> Option<PreparedSpreadsheetSourceManifest> {
        self.sources
            .get(source_id)
            .map(|source| source.prepared_manifest.clone())
    }

    /// Restores a policy-persisted projection from its exact existing original reference.
    pub fn restore_persisted(
        &mut self,
        expected: &PreparedSpreadsheetSourceManifest,
        request: &StructuredSourceExtractionRequest,
        source: &[u8],
        cancelled: &mut dyn FnMut() -> bool,
    ) -> Result<(), StructuredSourceExtractionError> {
        if expected.lifecycle != PreparedSpreadsheetLifecycle::Current
            || !matches!(
                expected.retention,
                PreparedSpreadsheetRetention::PolicyPersisted { .. }
            )
            || self.sources.contains_key(&expected.source_id)
            || expected.manifest_sha256 != manifest_sha256(expected)?
        {
            return Err(StructuredSourceExtractionError::InvalidInput);
        }
        let admitted = self.admit_with_retention(
            request,
            source,
            expected.classification,
            &expected.source_kind,
            &expected.protected_origin_sha256,
            expected.retention.clone(),
            cancelled,
        )?;
        if admitted.prepared_manifest != *expected {
            self.sources.remove(&expected.source_id);
            return Err(StructuredSourceExtractionError::InvalidInput);
        }
        Ok(())
    }

    /// Marks a current projection stale after source, parser, or policy invalidation.
    pub fn invalidate(
        &mut self,
        source_id: &str,
        expected_manifest_sha256: &str,
    ) -> Result<PreparedSpreadsheetSourceManifest, ArtifactExtensionError> {
        self.transition(
            source_id,
            expected_manifest_sha256,
            PreparedSpreadsheetLifecycle::Current,
            PreparedSpreadsheetLifecycle::Stale,
            false,
        )
    }

    /// Releases derived content while retaining only content-free lineage.
    pub fn release(
        &mut self,
        source_id: &str,
        expected_manifest_sha256: &str,
    ) -> Result<PreparedSpreadsheetSourceManifest, ArtifactExtensionError> {
        let prior = self
            .sources
            .get(source_id)
            .ok_or(ArtifactExtensionError::OutOfRange)?
            .prepared_manifest
            .lifecycle;
        if !matches!(
            prior,
            PreparedSpreadsheetLifecycle::Current | PreparedSpreadsheetLifecycle::Stale
        ) {
            return Err(ArtifactExtensionError::Denied);
        }
        self.transition(
            source_id,
            expected_manifest_sha256,
            prior,
            PreparedSpreadsheetLifecycle::Released,
            true,
        )
    }

    /// Tombstones already released lineage; no source or projection bytes remain.
    pub fn delete(
        &mut self,
        source_id: &str,
        expected_manifest_sha256: &str,
    ) -> Result<PreparedSpreadsheetSourceManifest, ArtifactExtensionError> {
        self.transition(
            source_id,
            expected_manifest_sha256,
            PreparedSpreadsheetLifecycle::Released,
            PreparedSpreadsheetLifecycle::Deleted,
            true,
        )
    }

    fn transition(
        &mut self,
        source_id: &str,
        expected_manifest_sha256: &str,
        expected: PreparedSpreadsheetLifecycle,
        next: PreparedSpreadsheetLifecycle,
        clear: bool,
    ) -> Result<PreparedSpreadsheetSourceManifest, ArtifactExtensionError> {
        let source = self
            .sources
            .get_mut(source_id)
            .ok_or(ArtifactExtensionError::OutOfRange)?;
        if source.prepared_manifest.manifest_sha256 != expected_manifest_sha256
            || source.prepared_manifest.lifecycle != expected
        {
            return Err(ArtifactExtensionError::Stale);
        }
        if clear {
            source.extraction = None;
            source.lexical_index.clear();
        }
        source.prepared_manifest.lifecycle = next;
        source.prepared_manifest.revision = source
            .prepared_manifest
            .revision
            .checked_add(1)
            .ok_or(ArtifactExtensionError::LimitExceeded)?;
        source.prepared_manifest.manifest_sha256 = manifest_sha256(&source.prepared_manifest)
            .map_err(|_| ArtifactExtensionError::Denied)?;
        source.manifest = artifact_manifest(&source.prepared_manifest);
        Ok(source.prepared_manifest.clone())
    }

    /// Returns bounded candidates through the same freshness-bound projection used by native tools.
    pub fn context_candidates(
        &self,
        source_id: &str,
        freshness_sha256: &str,
        maximum_items: usize,
    ) -> Result<Vec<ArtifactFragment>, ArtifactExtensionError> {
        if maximum_items == 0 {
            return Err(ArtifactExtensionError::LimitExceeded);
        }
        let source = self.current(source_id, freshness_sha256)?;
        if source.manifest.classification == ArtifactClassification::Restricted {
            return Err(ArtifactExtensionError::Denied);
        }
        Ok(project_source(source)
            .fragments
            .into_iter()
            .take(maximum_items)
            .collect())
    }

    /// Returns a bounded content-free lexical index for the exact current projection.
    pub fn lexical_index(
        &self,
        source_id: &str,
        freshness_sha256: &str,
        maximum_terms: usize,
    ) -> Result<Vec<PreparedSpreadsheetLexicalEntry>, ArtifactExtensionError> {
        if maximum_terms == 0 {
            return Err(ArtifactExtensionError::LimitExceeded);
        }
        let source = self.current(source_id, freshness_sha256)?;
        if source.manifest.classification == ArtifactClassification::Restricted {
            return Err(ArtifactExtensionError::Denied);
        }
        Ok(source
            .lexical_index
            .iter()
            .take(maximum_terms)
            .map(|(term, section_ids)| PreparedSpreadsheetLexicalEntry {
                term: term.clone(),
                section_ids: section_ids.clone(),
            })
            .collect())
    }

    /// Counts selected candidates across exact current workbook revisions without approximation.
    pub fn account_combined_context(
        &self,
        sources: &[(&str, &str)],
        maximum_candidates_per_source: usize,
        maximum_tokens: u32,
        counter: &mut impl ExactSourceTokenCounter,
    ) -> Result<CombinedSpreadsheetContextAccounting, SourcePreparationError> {
        if sources.is_empty() || maximum_candidates_per_source == 0 || maximum_tokens == 0 {
            return Err(SourcePreparationError::InvalidInput);
        }
        let binding = counter.binding();
        if !valid_id(&binding.token_counter_id)
            || !valid_sha256(&binding.token_counter_sha256)
            || !valid_sha256(&binding.tokenizer_sha256)
        {
            return Err(SourcePreparationError::TokenizerMismatch);
        }
        let mut used_tokens = 0_u32;
        let mut records = Vec::new();
        for (source_id, freshness) in sources {
            let candidates = self
                .context_candidates(source_id, freshness, maximum_candidates_per_source)
                .map_err(|_| SourcePreparationError::Conflict)?;
            let mut source_tokens = 0_u32;
            for candidate in &candidates {
                let content = match candidate {
                    ArtifactFragment::Byte { content, .. }
                    | ArtifactFragment::Line { content, .. }
                    | ArtifactFragment::Page { content, .. }
                    | ArtifactFragment::Sheet { content, .. }
                    | ArtifactFragment::Cell { content, .. }
                    | ArtifactFragment::Section { content, .. } => content,
                };
                source_tokens = source_tokens
                    .checked_add(counter.count_tokens(content)?)
                    .ok_or(SourcePreparationError::ResourceLimit)?;
            }
            used_tokens = used_tokens
                .checked_add(source_tokens)
                .ok_or(SourcePreparationError::ResourceLimit)?;
            if used_tokens > maximum_tokens {
                return Err(SourcePreparationError::ResourceLimit);
            }
            records.push(SpreadsheetContextAccounting {
                source_id: (*source_id).to_owned(),
                manifest_sha256: (*freshness).to_owned(),
                token_count: source_tokens,
                candidate_count: u32::try_from(candidates.len())
                    .map_err(|_| SourcePreparationError::ResourceLimit)?,
            });
        }
        Ok(CombinedSpreadsheetContextAccounting {
            token_counter_id: binding.token_counter_id,
            token_counter_sha256: binding.token_counter_sha256,
            tokenizer_sha256: binding.tokenizer_sha256,
            used_tokens,
            maximum_tokens,
            sources: records,
        })
    }

    fn current(
        &self,
        source_id: &str,
        freshness_sha256: &str,
    ) -> Result<&PreparedSpreadsheetSource, ArtifactExtensionError> {
        let source = self
            .sources
            .get(source_id)
            .ok_or(ArtifactExtensionError::OutOfRange)?;
        if source.prepared_manifest.manifest_sha256 != freshness_sha256
            || source.prepared_manifest.lifecycle != PreparedSpreadsheetLifecycle::Current
            || source.extraction.is_none()
        {
            return Err(ArtifactExtensionError::Stale);
        }
        Ok(source)
    }
}

impl ArtifactExtractorExtensions for SpreadsheetSourceArtifactService {
    fn get_page(
        &self,
        _source_id: &str,
        _freshness_sha256: &str,
        _page_number: u64,
        _limits: agentmage_capability_read_only::ArtifactLimits,
    ) -> Result<ArtifactFragment, ArtifactExtensionError> {
        Err(ArtifactExtensionError::Unsupported)
    }

    fn get_sheet(
        &self,
        source_id: &str,
        freshness_sha256: &str,
        sheet_name: &str,
        limits: agentmage_capability_read_only::ArtifactLimits,
    ) -> Result<Vec<ArtifactFragment>, ArtifactExtensionError> {
        if !valid_id(sheet_name) || limits.items == 0 || limits.range_units == 0 {
            return Err(ArtifactExtensionError::LimitExceeded);
        }
        let source = self.current(source_id, freshness_sha256)?;
        if source.manifest.classification == ArtifactClassification::Restricted {
            return Err(ArtifactExtensionError::Denied);
        }
        let fragments = project_source(source)
            .fragments
            .into_iter()
            .filter(|fragment| match fragment {
                ArtifactFragment::Sheet { name, .. }
                | ArtifactFragment::Cell { sheet: name, .. } => name == sheet_name,
                _ => false,
            })
            .take(limits.items as usize + 1)
            .collect::<Vec<_>>();
        if fragments.is_empty() {
            Err(ArtifactExtensionError::OutOfRange)
        } else if fragments.len() > limits.items as usize {
            Err(ArtifactExtensionError::LimitExceeded)
        } else {
            Ok(fragments)
        }
    }
}

fn artifact_manifest(prepared: &PreparedSpreadsheetSourceManifest) -> ArtifactManifest {
    ArtifactManifest {
        source_id: prepared.source_id.clone(),
        media_type: XLSX_MEDIA_TYPE.to_owned(),
        byte_len: prepared.source_bytes,
        payload_sha256: prepared.source_sha256.clone(),
        classification: prepared.classification,
        extraction_state: agentmage_capability_read_only::ArtifactExtractionState::Complete,
        provenance: ArtifactProvenance {
            provenance_id: format!("structured-spreadsheet-{}", prepared.source_id),
            source_kind: prepared.source_kind.clone(),
            protected_origin_sha256: prepared.protected_origin_sha256.clone(),
        },
        freshness: ArtifactFreshness {
            manifest_sha256: prepared.manifest_sha256.clone(),
            observed_sequence: prepared.revision,
        },
        reason_code: None,
    }
}

fn sheet_name(source: &PreparedSpreadsheetSource, sheet_number: u32) -> Option<&str> {
    source
        .prepared_manifest
        .sheets
        .iter()
        .find(|sheet| sheet.sheet_number == sheet_number)
        .map(|sheet| sheet.name.as_str())
}

fn project_source(source: &PreparedSpreadsheetSource) -> FakeArtifactSource {
    let extraction = source
        .extraction
        .as_ref()
        .expect("current prepared spreadsheet has extraction");
    let sections = extraction
        .sections
        .iter()
        .map(|section| ArtifactSection {
            section_id: section.section_id.clone(),
            title: format!("{:?}", section.kind),
            ordinal: section.ordinal,
            byte_len: section.content.len() as u64,
        })
        .collect();
    let mut fragments = Vec::new();
    for summary in &source.prepared_manifest.sheets {
        let content = serde_json::to_string(summary).unwrap_or_default();
        fragments.push(ArtifactFragment::Sheet {
            name: summary.name.clone(),
            content,
        });
    }
    for section in &extraction.sections {
        if section.kind == StructuredSourceSectionKind::TableCell {
            let Some(table) = section.provenance.table else {
                continue;
            };
            let Some(name) = sheet_name(source, table) else {
                continue;
            };
            fragments.push(ArtifactFragment::Cell {
                sheet: name.to_owned(),
                row: u64::from(section.provenance.row.unwrap_or(0)),
                column: u64::from(section.provenance.cell.unwrap_or(0)),
                content: section.content.clone(),
            });
        }
    }
    FakeArtifactSource {
        manifest: source.manifest.clone(),
        sections,
        fragments,
        redacted: source.manifest.classification == ArtifactClassification::Restricted,
    }
}

impl ArtifactBackend for SpreadsheetSourceArtifactService {
    fn manifests(&self) -> Vec<ArtifactManifest> {
        self.sources
            .values()
            .filter(|source| {
                source.prepared_manifest.lifecycle == PreparedSpreadsheetLifecycle::Current
                    && source.extraction.is_some()
            })
            .map(|source| source.manifest.clone())
            .collect()
    }

    fn source(&self, source_id: &str) -> Option<FakeArtifactSource> {
        self.sources.get(source_id).and_then(|source| {
            (source.prepared_manifest.lifecycle == PreparedSpreadsheetLifecycle::Current
                && source.extraction.is_some())
            .then(|| project_source(source))
        })
    }

    fn backend_class(&self) -> ArtifactBackendClass {
        ArtifactBackendClass::ProductionPreparedSource
    }
}

/// Dispatches a prepared spreadsheet through the common native artifact protocol.
pub fn dispatch_spreadsheet_source_artifact(
    kind: ArtifactToolKind,
    request_bytes: &[u8],
    service: &SpreadsheetSourceArtifactService,
    workspace_read_authorized: bool,
    signal: ArtifactExecutionSignal,
    ledger: &mut ArtifactAttemptLedger,
) -> Result<ArtifactResult, ArtifactDispatchError> {
    dispatch_artifact(
        kind,
        request_bytes,
        service,
        workspace_read_authorized,
        signal,
        ledger,
    )
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use agentmage_capability_read_only::{
        ArtifactItem, ArtifactLimits, ArtifactOutcome, ArtifactRange, ArtifactRequest,
        validate_artifact_request,
    };
    use agentmage_kernel_contracts::{RuntimeArtifactId, WorkspaceId, WorkspacePath};
    use agentmage_kernel_engine::model_orchestration_profile::ExactTokenCounterBinding;
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    use super::*;

    fn package(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = ZipWriter::new(&mut cursor);
            let options = SimpleFileOptions::default()
                .compression_method(CompressionMethod::Stored)
                .unix_permissions(0o644);
            for (name, content) in entries {
                writer.start_file(*name, options).expect("start");
                writer.write_all(content.as_bytes()).expect("write");
            }
            writer.finish().expect("finish");
        }
        cursor.into_inner()
    }

    fn fixture() -> Vec<u8> {
        package(&[
            (
                "[Content_Types].xml",
                "<?xml version=\"1.0\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/></Types>",
            ),
            (
                "xl/workbook.xml",
                "<?xml version=\"1.0\"?><workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><sheets><sheet name=\"Data\" sheetId=\"1\" r:id=\"rId1\"/></sheets></workbook>",
            ),
            (
                "xl/_rels/workbook.xml.rels",
                "<?xml version=\"1.0\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet1.xml\"/></Relationships>",
            ),
            (
                "xl/worksheets/sheet1.xml",
                "<?xml version=\"1.0\"?><worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><dimension ref=\"A1:B2\"/><sheetData><row r=\"1\"><c r=\"A1\" t=\"inlineStr\"><is><t>Name</t></is></c><c r=\"B1\" t=\"inlineStr\"><is><t>Amount</t></is></c></row><row r=\"2\"><c r=\"A2\" t=\"inlineStr\"><is><t>Alice</t></is></c><c r=\"B2\"><v>7</v></c></row></sheetData></worksheet>",
            ),
        ])
    }

    fn extraction_request_for(source: &[u8], source_id: &str) -> StructuredSourceExtractionRequest {
        StructuredSourceExtractionRequest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            source_id: source_id.to_owned(),
            media_type: XLSX_MEDIA_TYPE.to_owned(),
            source_sha256: sha256(source),
            source_path: WorkspacePath::new(
                WorkspaceId::from_raw("workspace-sheet"),
                ["sources", "data.xlsx"],
            )
            .expect("path"),
            maximum_sections: 100,
            maximum_output_bytes: 1_024 * 1_024,
        }
    }

    fn extraction_request(source: &[u8]) -> StructuredSourceExtractionRequest {
        extraction_request_for(source, "spreadsheet-source")
    }

    fn persisted_retention(source: &[u8]) -> PreparedSpreadsheetRetention {
        PreparedSpreadsheetRetention::PolicyPersisted {
            artifact: RuntimeArtifactRef {
                schema_version: CONTRACT_SCHEMA_VERSION,
                artifact_id: RuntimeArtifactId::from_raw("spreadsheet-original"),
                manifest_sha256: "b".repeat(64),
                payload_sha256: sha256(source),
                byte_size: source.len() as u64,
                media_type: XLSX_MEDIA_TYPE.to_owned(),
            },
            policy_sha256: "c".repeat(64),
        }
    }

    struct WordCounter {
        binding: ExactTokenCounterBinding,
    }

    impl ExactSourceTokenCounter for WordCounter {
        fn binding(&self) -> ExactTokenCounterBinding {
            self.binding.clone()
        }

        fn count_tokens(&mut self, content: &str) -> Result<u32, SourcePreparationError> {
            u32::try_from(content.split_whitespace().count())
                .map_err(|_| SourcePreparationError::ResourceLimit)
        }
    }

    fn admit(
        service: &mut SpreadsheetSourceArtifactService,
        classification: ArtifactClassification,
    ) -> SpreadsheetSourceAdmissionOutcome {
        let source = fixture();
        service
            .admit(
                &extraction_request(&source),
                &source,
                classification,
                "attachment",
                &"a".repeat(64),
                &mut || false,
            )
            .expect("admit")
    }

    fn request(
        call_id: &str,
        freshness: &str,
        range: Option<ArtifactRange>,
        query: Option<&str>,
    ) -> Vec<u8> {
        request_with_limits(call_id, freshness, range, query, ArtifactLimits::default())
    }

    fn request_with_limits(
        call_id: &str,
        freshness: &str,
        range: Option<ArtifactRange>,
        query: Option<&str>,
        limits: ArtifactLimits,
    ) -> Vec<u8> {
        serde_json::to_vec(&ArtifactRequest {
            schema_version: 1,
            call_id: call_id.to_owned(),
            source_id: Some("spreadsheet-source".to_owned()),
            section_id: None,
            range,
            query: query.map(str::to_owned),
            freshness_sha256: Some(freshness.to_owned()),
            output_identity: format!("output-{call_id}"),
            limits,
            call_depth: 0,
        })
        .expect("request")
    }

    #[test]
    fn admission_manifest_sheet_summary_and_cache_bind_exact_source() {
        let mut service = SpreadsheetSourceArtifactService::default();
        let first = admit(&mut service, ArtifactClassification::Internal);
        let second = admit(&mut service, ArtifactClassification::Internal);
        assert!(!first.cache_hit);
        assert!(second.cache_hit);
        assert_eq!(first.prepared_manifest, second.prepared_manifest);
        assert_eq!(first.prepared_manifest.sheets.len(), 1);
        assert_eq!(first.prepared_manifest.sheets[0].name, "Data");
        assert_eq!(first.prepared_manifest.sheets[0].cell_count, 4);
        assert_eq!(first.manifest.payload_sha256, sha256(&fixture()));
        assert_eq!(service.manifests(), [first.manifest]);
    }

    #[test]
    fn exact_sheet_cell_search_and_context_use_one_fresh_projection() {
        let mut service = SpreadsheetSourceArtifactService::default();
        let admitted = admit(&mut service, ArtifactClassification::Internal);
        let freshness = &admitted.prepared_manifest.manifest_sha256;
        let sheet = service
            .get_sheet(
                "spreadsheet-source",
                freshness,
                "Data",
                ArtifactLimits::default(),
            )
            .expect("sheet");
        assert_eq!(sheet.len(), 5);
        assert!(sheet.iter().any(|fragment| matches!(fragment,
            ArtifactFragment::Cell { row: 2, column: 1, content, .. } if content.contains("Alice")
        )));
        assert!(
            !service
                .context_candidates("spreadsheet-source", freshness, 3)
                .expect("context")
                .is_empty()
        );
        assert!(
            service
                .lexical_index("spreadsheet-source", freshness, 100)
                .expect("lexical index")
                .iter()
                .any(|entry| entry.term == "alice" && !entry.section_ids.is_empty())
        );

        let mut ledger = ArtifactAttemptLedger::default();
        for (kind, bytes) in [
            (
                ArtifactToolKind::Range,
                request(
                    "sheet",
                    freshness,
                    Some(ArtifactRange::Sheet {
                        name: "Data".to_owned(),
                    }),
                    None,
                ),
            ),
            (
                ArtifactToolKind::Range,
                request(
                    "cell",
                    freshness,
                    Some(ArtifactRange::Cell {
                        sheet: "Data".to_owned(),
                        start_row: 2,
                        start_column: 1,
                        end_row: 2,
                        end_column: 2,
                    }),
                    None,
                ),
            ),
            (
                ArtifactToolKind::Search,
                request("search", freshness, None, Some("Alice")),
            ),
        ] {
            validate_artifact_request(kind, &bytes)
                .unwrap_or_else(|error| panic!("{kind:?} request: {error:?}"));
            let result = dispatch_spreadsheet_source_artifact(
                kind,
                &bytes,
                &service,
                true,
                ArtifactExecutionSignal::Continue,
                &mut ledger,
            )
            .unwrap_or_else(|error| panic!("{kind:?} dispatch: {error:?}"));
            assert_eq!(result.outcome, ArtifactOutcome::Succeeded);
            assert!(result.production_execution);
            assert!(result.verify(kind));
            assert!(!result.items.is_empty());
        }

        let bounded = dispatch_spreadsheet_source_artifact(
            ArtifactToolKind::Range,
            &request_with_limits(
                "range-reference",
                freshness,
                Some(ArtifactRange::Sheet {
                    name: "Data".to_owned(),
                }),
                None,
                ArtifactLimits {
                    output_bytes: 256,
                    ..ArtifactLimits::default()
                },
            ),
            &service,
            true,
            ArtifactExecutionSignal::Continue,
            &mut ledger,
        )
        .expect("content-addressed bounded reference");
        assert_eq!(bounded.outcome, ArtifactOutcome::Truncated);
        assert!(matches!(
            bounded.items.as_slice(),
            [ArtifactItem::LargeResultReference { .. }]
        ));
    }

    #[test]
    fn stale_restricted_missing_and_bounded_sheet_requests_fail_closed() {
        let mut service = SpreadsheetSourceArtifactService::default();
        let admitted = admit(&mut service, ArtifactClassification::Restricted);
        let freshness = &admitted.prepared_manifest.manifest_sha256;
        assert_eq!(
            service.get_sheet(
                "spreadsheet-source",
                &"0".repeat(64),
                "Data",
                ArtifactLimits::default(),
            ),
            Err(ArtifactExtensionError::Stale)
        );
        assert_eq!(
            service.get_sheet(
                "spreadsheet-source",
                freshness,
                "Data",
                ArtifactLimits::default(),
            ),
            Err(ArtifactExtensionError::Denied)
        );
        let mut public = SpreadsheetSourceArtifactService::default();
        let public_admission = admit(&mut public, ArtifactClassification::Public);
        let tiny = ArtifactLimits {
            items: 1,
            ..ArtifactLimits::default()
        };
        assert_eq!(
            public.get_sheet(
                "spreadsheet-source",
                &public_admission.prepared_manifest.manifest_sha256,
                "Data",
                tiny,
            ),
            Err(ArtifactExtensionError::LimitExceeded)
        );
    }

    #[test]
    fn persisted_restart_invalidation_release_delete_and_reattachment_are_exact() {
        let source = fixture();
        let request = extraction_request(&source);
        let retention = persisted_retention(&source);
        let mut service = SpreadsheetSourceArtifactService::default();
        let admitted = service
            .admit_with_retention(
                &request,
                &source,
                ArtifactClassification::Internal,
                "attachment",
                &"a".repeat(64),
                retention,
                &mut || false,
            )
            .expect("persisted admission");

        let mut restarted = SpreadsheetSourceArtifactService::default();
        restarted
            .restore_persisted(&admitted.prepared_manifest, &request, &source, &mut || {
                false
            })
            .expect("exact restart");
        let stale = restarted
            .invalidate(
                "spreadsheet-source",
                &admitted.prepared_manifest.manifest_sha256,
            )
            .expect("invalidate");
        assert_eq!(stale.lifecycle, PreparedSpreadsheetLifecycle::Stale);
        assert_eq!(restarted.manifests(), []);
        assert_eq!(
            restarted.context_candidates("spreadsheet-source", &stale.manifest_sha256, 1,),
            Err(ArtifactExtensionError::Stale)
        );
        let released = restarted
            .release("spreadsheet-source", &stale.manifest_sha256)
            .expect("release");
        assert_eq!(released.lifecycle, PreparedSpreadsheetLifecycle::Released);
        assert!(restarted.sources["spreadsheet-source"].extraction.is_none());
        let deleted = restarted
            .delete("spreadsheet-source", &released.manifest_sha256)
            .expect("delete");
        assert_eq!(deleted.lifecycle, PreparedSpreadsheetLifecycle::Deleted);

        let reattached = restarted
            .admit(
                &request,
                &source,
                ArtifactClassification::Internal,
                "attachment",
                &"a".repeat(64),
                &mut || false,
            )
            .expect("reattach");
        assert!(!reattached.cache_hit);
        assert_eq!(
            reattached.prepared_manifest.lifecycle,
            PreparedSpreadsheetLifecycle::Current
        );
        assert!(reattached.prepared_manifest.revision > deleted.revision);
        assert!(restarted.sources["spreadsheet-source"].extraction.is_some());
    }

    #[test]
    fn combined_artifact_context_uses_exact_counter_and_one_projection_per_source() {
        let source = fixture();
        let mut service = SpreadsheetSourceArtifactService::default();
        let first = admit(&mut service, ArtifactClassification::Internal);
        let second_request = extraction_request_for(&source, "spreadsheet-source-b");
        let second = service
            .admit(
                &second_request,
                &source,
                ArtifactClassification::Internal,
                "attachment",
                &"d".repeat(64),
                &mut || false,
            )
            .expect("second source");
        let mut counter = WordCounter {
            binding: ExactTokenCounterBinding {
                token_counter_id: "fixture-word-counter".to_owned(),
                token_counter_sha256: "e".repeat(64),
                tokenizer_sha256: "f".repeat(64),
            },
        };
        let sources = [
            (
                "spreadsheet-source",
                first.prepared_manifest.manifest_sha256.as_str(),
            ),
            (
                "spreadsheet-source-b",
                second.prepared_manifest.manifest_sha256.as_str(),
            ),
        ];
        let accounting = service
            .account_combined_context(&sources, 3, 100, &mut counter)
            .expect("combined accounting");
        assert_eq!(accounting.sources.len(), 2);
        assert!(accounting.used_tokens > 0);
        assert_eq!(accounting.token_counter_id, "fixture-word-counter");
        assert_eq!(
            service.account_combined_context(&sources, 3, 1, &mut counter),
            Err(SourcePreparationError::ResourceLimit)
        );
    }
}
