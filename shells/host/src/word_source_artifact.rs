//! Runtime-owned DOCX preparation and common native artifact-tool projection.

use std::collections::{BTreeMap, BTreeSet};

use agentmage_capability_knowledge::WordStructuredSourceExtractor;
use agentmage_capability_read_only::{
    ArtifactAttemptLedger, ArtifactBackend, ArtifactBackendClass, ArtifactClassification,
    ArtifactDispatchError, ArtifactExecutionSignal, ArtifactExtractionState, ArtifactFragment,
    ArtifactFreshness, ArtifactManifest, ArtifactProvenance, ArtifactResult, ArtifactSection,
    ArtifactToolKind, FakeArtifactSource, dispatch_artifact,
};
use agentmage_kernel_contracts::{
    CONTRACT_SCHEMA_VERSION, ContextAdmission, ContextItemCandidate, ContextItemKind,
    ContextOmissionReason, ContextPacketId, ContextSensitivity, RuntimeArtifactRef,
    StructuredSourceExtraction, StructuredSourceExtractionError, StructuredSourceExtractionRequest,
    StructuredSourceExtractor, StructuredSourceSectionKind, StructuredSourceWarning,
};
use agentmage_kernel_engine::context_management::{ContextCompositionBudget, compose_context};
use agentmage_kernel_engine::model_orchestration_profile::{
    ModelContextWindowPlan, verify_model_context_window_plan,
};
use agentmage_kernel_engine::source_preparation::{
    ExactSourceTokenCounter, PreparedSourceContextManifest, SourceContextDisposition,
    SourceContextRecord, SourcePreparationError, verify_prepared_source_context_manifest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAX_PREPARED_WORD_SOURCES: usize = 1_024;
const MAX_WORD_CONTEXT_RECORDS: usize = 4_096;

/// One content-free terminal result for a DOCX preparation attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordSourceAdmissionOutcome {
    /// Exact current artifact manifest.
    pub manifest: ArtifactManifest,
    /// Complete content-free prepared-source manifest for retention and restart.
    pub prepared_manifest: PreparedWordSourceManifest,
    /// True only when the exact source/extractor projection was already prepared.
    pub cache_hit: bool,
}

/// Closed DOCX projection retention binding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PreparedWordSourceRetention {
    /// The projection exists only in this bounded service instance.
    Ephemeral,
    /// Original DOCX bytes already exist in the encrypted runtime-artifact authority.
    PolicyPersisted {
        /// Exact path-free content-addressed payload reference.
        artifact: RuntimeArtifactRef,
        /// Exact policy revision authorizing persistence.
        policy_sha256: String,
    },
}

/// Content-free, digest-sealed DOCX preparation record used for exact restart.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedWordSourceManifest {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable logical source identity.
    pub source_id: String,
    /// Exact declared media type.
    pub media_type: String,
    /// Complete original DOCX byte count.
    pub source_bytes: u64,
    /// Content address of the complete original DOCX.
    pub source_sha256: String,
    /// Exact extractor implementation identity.
    pub extractor_sha256: String,
    /// Hard canonical-section ceiling used for extraction.
    pub maximum_sections: u32,
    /// Hard emitted-content byte ceiling used for extraction.
    pub maximum_output_bytes: u64,
    /// Digest of the complete canonical extraction projection.
    pub extraction_sha256: String,
    /// Number of canonical sections.
    pub section_count: u32,
    /// Number of emitted canonical text bytes.
    pub output_bytes: u64,
    /// Whether no section or content was omitted by extraction bounds.
    pub extraction_complete: bool,
    /// Complete ordered fidelity and policy warnings.
    pub warnings: Vec<StructuredSourceWarning>,
    /// Sensitivity classification applied before native projection.
    pub classification: ArtifactClassification,
    /// Content-free source kind such as attachment or generated.
    pub source_kind: String,
    /// Digest of protected origin metadata; never a raw path or URI.
    pub protected_origin_sha256: String,
    /// Existing artifact-authority binding, or bounded process-memory retention.
    pub retention: PreparedWordSourceRetention,
    /// Monotonic logical source revision.
    pub revision: u64,
    /// Digest of this complete record with this field zeroed.
    pub manifest_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PreparedWordSource {
    extraction: StructuredSourceExtraction,
    manifest: ArtifactManifest,
    prepared_manifest: PreparedWordSourceManifest,
}

/// Bounded runtime-owned prepared DOCX service; source package bytes are never retained here.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WordSourceArtifactService {
    sources: BTreeMap<String, PreparedWordSource>,
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

fn prepared_manifest_sha256(
    manifest: &PreparedWordSourceManifest,
) -> Result<String, StructuredSourceExtractionError> {
    let mut candidate = manifest.clone();
    candidate.manifest_sha256 = "0".repeat(64);
    serde_json::to_vec(&candidate)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| StructuredSourceExtractionError::InvalidInput)
}

fn valid_retention(
    retention: &PreparedWordSourceRetention,
    request: &StructuredSourceExtractionRequest,
    source_len: u64,
) -> bool {
    match retention {
        PreparedWordSourceRetention::Ephemeral => true,
        PreparedWordSourceRetention::PolicyPersisted {
            artifact,
            policy_sha256,
        } => {
            artifact.schema_version == CONTRACT_SCHEMA_VERSION
                && valid_id(artifact.artifact_id.as_str())
                && valid_sha256(&artifact.manifest_sha256)
                && artifact.payload_sha256 == request.source_sha256
                && artifact.byte_size == source_len
                && artifact.media_type == request.media_type
                && valid_sha256(policy_sha256)
        }
    }
}

impl WordSourceArtifactService {
    /// Prepares one exact captured DOCX through the shared extractor contract.
    ///
    /// The service retains only the canonical projection and content-free manifest. Original DOCX
    /// bytes remain owned by the runtime artifact store and are neither copied nor persisted here.
    pub fn admit(
        &mut self,
        request: &StructuredSourceExtractionRequest,
        source: &[u8],
        classification: ArtifactClassification,
        source_kind: &str,
        protected_origin_sha256: &str,
        cancelled: &mut dyn FnMut() -> bool,
    ) -> Result<WordSourceAdmissionOutcome, StructuredSourceExtractionError> {
        self.admit_with_retention(
            request,
            source,
            classification,
            source_kind,
            protected_origin_sha256,
            PreparedWordSourceRetention::Ephemeral,
            cancelled,
        )
    }

    /// Prepares one exact captured DOCX with an existing runtime-artifact retention binding.
    #[allow(clippy::too_many_arguments)]
    pub fn admit_with_retention(
        &mut self,
        request: &StructuredSourceExtractionRequest,
        source: &[u8],
        classification: ArtifactClassification,
        source_kind: &str,
        protected_origin_sha256: &str,
        retention: PreparedWordSourceRetention,
        cancelled: &mut dyn FnMut() -> bool,
    ) -> Result<WordSourceAdmissionOutcome, StructuredSourceExtractionError> {
        if !valid_id(source_kind) || !valid_sha256(protected_origin_sha256) {
            return Err(StructuredSourceExtractionError::InvalidInput);
        }
        if !valid_retention(&retention, request, source.len() as u64) {
            return Err(StructuredSourceExtractionError::InvalidInput);
        }
        let extractor = WordStructuredSourceExtractor;
        let extractor_sha256 = extractor.extractor_sha256();
        if let Some(current) = self.sources.get(&request.source_id)
            && current.extraction.source_sha256 == request.source_sha256
            && current.extraction.extractor_sha256 == extractor_sha256
            && current.prepared_manifest.classification == classification
            && current.prepared_manifest.source_kind == source_kind
            && current.prepared_manifest.protected_origin_sha256 == protected_origin_sha256
            && current.prepared_manifest.retention == retention
        {
            if cancelled() {
                return Err(StructuredSourceExtractionError::Cancelled);
            }
            return Ok(WordSourceAdmissionOutcome {
                manifest: current.manifest.clone(),
                prepared_manifest: current.prepared_manifest.clone(),
                cache_hit: true,
            });
        }
        if self.sources.len() >= MAX_PREPARED_WORD_SOURCES
            && !self.sources.contains_key(&request.source_id)
        {
            return Err(StructuredSourceExtractionError::ResourceLimit);
        }
        let extraction = extractor.extract(request, source, cancelled)?;
        let revision = self.sources.get(&request.source_id).map_or(1, |current| {
            current.manifest.freshness.observed_sequence + 1
        });
        let extraction_sha256 = serde_json::to_vec(&extraction)
            .map(|bytes| sha256(&bytes))
            .map_err(|_| StructuredSourceExtractionError::InvalidInput)?;
        let mut prepared_manifest = PreparedWordSourceManifest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            source_id: request.source_id.clone(),
            media_type: request.media_type.clone(),
            source_bytes: source.len() as u64,
            source_sha256: request.source_sha256.clone(),
            extractor_sha256,
            maximum_sections: request.maximum_sections,
            maximum_output_bytes: request.maximum_output_bytes,
            extraction_sha256,
            section_count: extraction.sections.len() as u32,
            output_bytes: extraction.output_bytes,
            extraction_complete: extraction.extraction_complete,
            warnings: extraction.warnings.clone(),
            classification,
            source_kind: source_kind.to_owned(),
            protected_origin_sha256: protected_origin_sha256.to_owned(),
            retention,
            revision,
            manifest_sha256: "0".repeat(64),
        };
        prepared_manifest.manifest_sha256 = prepared_manifest_sha256(&prepared_manifest)?;
        let manifest = artifact_manifest(&prepared_manifest);
        self.sources.insert(
            request.source_id.clone(),
            PreparedWordSource {
                extraction,
                manifest: manifest.clone(),
                prepared_manifest: prepared_manifest.clone(),
            },
        );
        Ok(WordSourceAdmissionOutcome {
            manifest,
            prepared_manifest,
            cache_hit: false,
        })
    }

    /// Reconstructs one exact persisted projection from bytes supplied by the existing artifact store.
    ///
    /// No path or storage authority is accepted. Publication occurs only when fresh extraction
    /// reproduces the complete digest-sealed prepared manifest byte-for-byte.
    pub fn restore_persisted(
        &mut self,
        expected: &PreparedWordSourceManifest,
        request: &StructuredSourceExtractionRequest,
        source: &[u8],
        cancelled: &mut dyn FnMut() -> bool,
    ) -> Result<(), StructuredSourceExtractionError> {
        if expected.schema_version != CONTRACT_SCHEMA_VERSION
            || expected.manifest_sha256 != prepared_manifest_sha256(expected)?
            || !valid_id(&expected.source_id)
            || !valid_id(&expected.source_kind)
            || !valid_sha256(&expected.source_sha256)
            || !valid_sha256(&expected.extractor_sha256)
            || !valid_sha256(&expected.extraction_sha256)
            || !valid_sha256(&expected.protected_origin_sha256)
            || expected.revision == 0
            || !matches!(
                expected.retention,
                PreparedWordSourceRetention::PolicyPersisted { .. }
            )
            || expected.source_id != request.source_id
            || expected.media_type != request.media_type
            || expected.source_sha256 != request.source_sha256
            || expected.source_bytes != source.len() as u64
            || expected.maximum_sections != request.maximum_sections
            || expected.maximum_output_bytes != request.maximum_output_bytes
            || self.sources.contains_key(&expected.source_id)
        {
            return Err(StructuredSourceExtractionError::InvalidInput);
        }
        let extractor = WordStructuredSourceExtractor;
        if expected.extractor_sha256 != extractor.extractor_sha256()
            || !valid_retention(&expected.retention, request, source.len() as u64)
        {
            return Err(StructuredSourceExtractionError::InvalidInput);
        }
        let extraction = extractor.extract(request, source, cancelled)?;
        let extraction_sha256 = serde_json::to_vec(&extraction)
            .map(|bytes| sha256(&bytes))
            .map_err(|_| StructuredSourceExtractionError::InvalidInput)?;
        if extraction_sha256 != expected.extraction_sha256
            || extraction.sections.len() as u32 != expected.section_count
            || extraction.output_bytes != expected.output_bytes
            || extraction.extraction_complete != expected.extraction_complete
            || extraction.warnings != expected.warnings
        {
            return Err(StructuredSourceExtractionError::InvalidInput);
        }
        let manifest = artifact_manifest(expected);
        self.sources.insert(
            expected.source_id.clone(),
            PreparedWordSource {
                extraction,
                manifest,
                prepared_manifest: expected.clone(),
            },
        );
        Ok(())
    }

    /// Returns one complete content-free manifest for checkpoint persistence.
    #[must_use]
    pub fn prepared_manifest(&self, source_id: &str) -> Option<PreparedWordSourceManifest> {
        self.sources
            .get(source_id)
            .map(|source| source.prepared_manifest.clone())
    }

    /// Composes minimized DOCX sections through the existing context manager with full accounting.
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
        if self.sources.is_empty()
            || !valid_id(&context_manifest_id)
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
        let mut candidate_records = BTreeMap::new();
        let mut content_owners = BTreeSet::new();
        for source in self.sources.values() {
            let prepared = &source.prepared_manifest;
            let sensitivity = context_sensitivity(prepared.classification);
            let restricted = sensitivity == ContextSensitivity::Restricted
                || (sensitivity == ContextSensitivity::Private && !disclose_private);
            records.push(SourceContextRecord {
                source_id: prepared.source_id.clone(),
                source_revision_sha256: prepared.manifest_sha256.clone(),
                section_id: None,
                section_sha256: None,
                disposition: if restricted {
                    SourceContextDisposition::Restricted
                } else if prepared.extraction_complete {
                    SourceContextDisposition::Included
                } else {
                    SourceContextDisposition::Truncated
                },
                reason_code: if restricted {
                    Some("context.source.restricted".to_owned())
                } else if prepared.extraction_complete {
                    None
                } else {
                    Some("context.source.extraction_truncated".to_owned())
                },
                token_count: 0,
            });
            for section in &source.extraction.sections {
                let digest = sha256(section.content.as_bytes());
                let record_index = records.len();
                let empty = section.content.is_empty();
                records.push(SourceContextRecord {
                    source_id: prepared.source_id.clone(),
                    source_revision_sha256: prepared.manifest_sha256.clone(),
                    section_id: Some(section.section_id.clone()),
                    section_sha256: Some(digest.clone()),
                    disposition: if restricted {
                        SourceContextDisposition::Restricted
                    } else if empty {
                        SourceContextDisposition::Omitted
                    } else {
                        SourceContextDisposition::Included
                    },
                    reason_code: if restricted {
                        Some("context.source.restricted".to_owned())
                    } else if empty {
                        Some("context.section.container".to_owned())
                    } else {
                        None
                    },
                    token_count: 0,
                });
                if !restricted && !empty {
                    add_context_candidate(
                        &mut candidates,
                        &mut candidate_records,
                        &mut content_owners,
                        &mut records,
                        prepared,
                        sensitivity,
                        &section.section_id,
                        &section.content,
                        digest,
                        record_index,
                        counter,
                    )?;
                }
            }
            for warning in &source.extraction.warnings {
                let section_id = format!("warning:{}", warning.warning_id);
                let content = warning_content(warning);
                let digest = sha256(content.as_bytes());
                let record_index = records.len();
                records.push(SourceContextRecord {
                    source_id: prepared.source_id.clone(),
                    source_revision_sha256: prepared.manifest_sha256.clone(),
                    section_id: Some(section_id.clone()),
                    section_sha256: Some(digest.clone()),
                    disposition: if restricted {
                        SourceContextDisposition::Restricted
                    } else {
                        SourceContextDisposition::Included
                    },
                    reason_code: restricted.then(|| "context.source.restricted".to_owned()),
                    token_count: 0,
                });
                if !restricted {
                    add_context_candidate(
                        &mut candidates,
                        &mut candidate_records,
                        &mut content_owners,
                        &mut records,
                        prepared,
                        sensitivity,
                        &section_id,
                        &content,
                        digest,
                        record_index,
                        counter,
                    )?;
                }
            }
            if records.len() > MAX_WORD_CONTEXT_RECORDS {
                return Err(SourcePreparationError::ResourceLimit);
            }
        }
        let packet = compose_context(
            packet_id,
            &ContextCompositionBudget {
                max_bytes: maximum_bytes,
                max_tokens: plan.source_artifacts.allocated_tokens,
                max_items: MAX_WORD_CONTEXT_RECORDS as u32,
                token_counter_id: binding.token_counter_id.clone(),
            },
            candidates,
        )?;
        for accounting in &packet.accounting {
            let Some(record) = candidate_records
                .get(&accounting.item_id)
                .and_then(|index| records.get_mut(*index))
            else {
                return Err(SourcePreparationError::InvalidInput);
            };
            record.token_count = accounting.token_count;
            if !accounting.included {
                let (disposition, reason) = match accounting.omission {
                    Some(ContextOmissionReason::Duplicate) => (
                        SourceContextDisposition::Duplicate,
                        "context.section.duplicate",
                    ),
                    Some(ContextOmissionReason::Budget) => {
                        (SourceContextDisposition::Omitted, "context.section.budget")
                    }
                    Some(ContextOmissionReason::Denied) => (
                        SourceContextDisposition::Restricted,
                        "context.section.restricted",
                    ),
                    Some(ContextOmissionReason::Stale) => {
                        (SourceContextDisposition::Stale, "context.section.stale")
                    }
                    None => return Err(SourcePreparationError::InvalidInput),
                };
                record.disposition = disposition;
                record.reason_code = Some(reason.to_owned());
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
            manifest_sha256: "0".repeat(64),
        };
        manifest.manifest_sha256 = sha256(
            &serde_json::to_vec(&manifest).map_err(|_| SourcePreparationError::InvalidInput)?,
        );
        verify_prepared_source_context_manifest(&manifest)?;
        Ok(manifest)
    }

    /// Deletes one prepared projection and its cache identity without touching original bytes.
    pub fn delete(&mut self, source_id: &str) -> bool {
        self.sources.remove(source_id).is_some()
    }

    /// Returns the number of current prepared projections.
    #[must_use]
    pub fn len(&self) -> usize {
        self.sources.len()
    }

    /// Returns whether no current prepared projections exist.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }
}

#[allow(clippy::too_many_arguments)]
fn add_context_candidate(
    candidates: &mut Vec<ContextItemCandidate>,
    candidate_records: &mut BTreeMap<String, usize>,
    content_owners: &mut BTreeSet<String>,
    records: &mut [SourceContextRecord],
    prepared: &PreparedWordSourceManifest,
    sensitivity: ContextSensitivity,
    section_id: &str,
    content: &str,
    digest: String,
    record_index: usize,
    counter: &mut impl ExactSourceTokenCounter,
) -> Result<(), SourcePreparationError> {
    let item_id = format!(
        "word-context-{}",
        &sha256(format!("{}:{section_id}", prepared.source_id).as_bytes())[..24]
    );
    let token_count = counter.count_tokens(content)?;
    if !content_owners.insert(digest.clone()) {
        let record = records
            .get_mut(record_index)
            .ok_or(SourcePreparationError::InvalidInput)?;
        record.disposition = SourceContextDisposition::Duplicate;
        record.reason_code = Some("context.section.duplicate".to_owned());
        record.token_count = token_count;
        return Ok(());
    }
    candidates.push(ContextItemCandidate {
        item_id: item_id.clone(),
        kind: ContextItemKind::Evidence,
        sensitivity,
        admission: ContextAdmission::Eligible,
        authoritative_evidence: true,
        essential: false,
        source_id: prepared.source_id.clone(),
        source_revision: prepared.manifest_sha256.clone(),
        content_sha256: digest,
        bounded_excerpt: content.to_owned(),
        token_count,
    });
    if candidate_records.insert(item_id, record_index).is_some() {
        return Err(SourcePreparationError::Conflict);
    }
    Ok(())
}

const fn context_sensitivity(classification: ArtifactClassification) -> ContextSensitivity {
    match classification {
        ArtifactClassification::Public => ContextSensitivity::Public,
        ArtifactClassification::Internal => ContextSensitivity::Internal,
        ArtifactClassification::Confidential => ContextSensitivity::Private,
        ArtifactClassification::Restricted => ContextSensitivity::Restricted,
    }
}

fn warning_content(warning: &StructuredSourceWarning) -> String {
    format!(
        "reason_code={}; source_part={}; original_remains_authoritative={}",
        warning.reason_code,
        warning.source_part.as_deref().unwrap_or("none"),
        warning.original_remains_authoritative,
    )
}

fn artifact_manifest(prepared: &PreparedWordSourceManifest) -> ArtifactManifest {
    ArtifactManifest {
        source_id: prepared.source_id.clone(),
        media_type: prepared.media_type.clone(),
        byte_len: prepared.source_bytes,
        payload_sha256: prepared.source_sha256.clone(),
        classification: prepared.classification,
        extraction_state: if prepared.extraction_complete {
            ArtifactExtractionState::Complete
        } else {
            ArtifactExtractionState::Partial
        },
        provenance: ArtifactProvenance {
            provenance_id: format!("structured-word-{}", prepared.source_id),
            source_kind: prepared.source_kind.clone(),
            protected_origin_sha256: prepared.protected_origin_sha256.clone(),
        },
        freshness: ArtifactFreshness {
            manifest_sha256: prepared.manifest_sha256.clone(),
            observed_sequence: prepared.revision,
        },
        reason_code: (!prepared.extraction_complete)
            .then(|| "structured-source.extraction.partial".to_owned()),
    }
}

fn section_title(kind: StructuredSourceSectionKind) -> &'static str {
    match kind {
        StructuredSourceSectionKind::Document => "Document",
        StructuredSourceSectionKind::Page => "Page",
        StructuredSourceSectionKind::Paragraph => "Paragraph",
        StructuredSourceSectionKind::Heading => "Heading",
        StructuredSourceSectionKind::ListItem => "List item",
        StructuredSourceSectionKind::Table => "Table",
        StructuredSourceSectionKind::TableRow => "Table row",
        StructuredSourceSectionKind::TableCell => "Table cell",
        StructuredSourceSectionKind::Header => "Header",
        StructuredSourceSectionKind::Footer => "Footer",
        StructuredSourceSectionKind::Note => "Note",
        StructuredSourceSectionKind::Comment => "Comment",
        StructuredSourceSectionKind::TrackedInsertion => "Tracked insertion",
        StructuredSourceSectionKind::TrackedDeletion => "Tracked deletion",
        StructuredSourceSectionKind::Link => "Link",
        StructuredSourceSectionKind::Image => "Image",
        StructuredSourceSectionKind::Relationship => "Relationship",
        StructuredSourceSectionKind::Unsupported => "Unsupported structure",
    }
}

fn project_source(source: &PreparedWordSource) -> FakeArtifactSource {
    let mut sections = source
        .extraction
        .sections
        .iter()
        .map(|section| ArtifactSection {
            section_id: section.section_id.clone(),
            title: section_title(section.kind).to_owned(),
            ordinal: section.ordinal,
            byte_len: section.content.len() as u64,
        })
        .collect::<Vec<_>>();
    let mut fragments = source
        .extraction
        .sections
        .iter()
        .filter(|section| !section.content.is_empty())
        .map(|section| ArtifactFragment::Section {
            section_id: section.section_id.clone(),
            content: section.content.clone(),
        })
        .collect::<Vec<_>>();
    for (index, warning) in source.extraction.warnings.iter().enumerate() {
        let section_id = format!("warning: {}", warning.warning_id);
        let content = warning_content(warning);
        sections.push(ArtifactSection {
            section_id: section_id.clone(),
            title: "Extraction warning".to_owned(),
            ordinal: source.extraction.sections.len() as u32 + index as u32,
            byte_len: content.len() as u64,
        });
        fragments.push(ArtifactFragment::Section {
            section_id,
            content,
        });
    }
    FakeArtifactSource {
        manifest: source.manifest.clone(),
        sections,
        fragments,
        redacted: source.manifest.classification == ArtifactClassification::Restricted,
    }
}

impl ArtifactBackend for WordSourceArtifactService {
    fn manifests(&self) -> Vec<ArtifactManifest> {
        self.sources
            .values()
            .map(|source| source.manifest.clone())
            .collect()
    }

    fn source(&self, source_id: &str) -> Option<FakeArtifactSource> {
        self.sources.get(source_id).map(project_source)
    }

    fn backend_class(&self) -> ArtifactBackendClass {
        ArtifactBackendClass::ProductionPreparedSource
    }
}

/// Dispatches one prepared DOCX through the common Story 16.2 native artifact protocol.
pub fn dispatch_word_source_artifact(
    kind: ArtifactToolKind,
    request_bytes: &[u8],
    service: &WordSourceArtifactService,
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
    use super::*;
    use agentmage_capability_read_only::{ArtifactLimits, ArtifactOutcome, ArtifactRequest};
    use agentmage_kernel_contracts::{
        CONTRACT_SCHEMA_VERSION, ContextPacketId, DOCX_MEDIA_TYPE, RuntimeArtifactId, WorkspaceId,
        WorkspacePath,
    };
    use agentmage_kernel_engine::model_orchestration_profile::{
        AllocatedContextPartition, ContextPartitionDisposition, ExactTokenCounterBinding,
        ModelContextWindowPlan,
    };

    fn push_u16(output: &mut Vec<u8>, value: u16) {
        output.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u32(output: &mut Vec<u8>, value: u32) {
        output.extend_from_slice(&value.to_le_bytes());
    }

    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = u32::MAX;
        for byte in bytes {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                crc = (crc >> 1) ^ (0xedb8_8320 & 0_u32.wrapping_sub(crc & 1));
            }
        }
        !crc
    }

    fn stored_zip(entries: &[(String, String)]) -> Vec<u8> {
        let mut output = Vec::new();
        let mut central = Vec::new();
        for (name, content) in entries {
            let name = name.as_bytes();
            let content = content.as_bytes();
            let offset = output.len() as u32;
            let checksum = crc32(content);
            push_u32(&mut output, 0x0403_4b50);
            push_u16(&mut output, 20);
            push_u16(&mut output, 0);
            push_u16(&mut output, 0);
            push_u16(&mut output, 0);
            push_u16(&mut output, 0);
            push_u32(&mut output, checksum);
            push_u32(&mut output, content.len() as u32);
            push_u32(&mut output, content.len() as u32);
            push_u16(&mut output, name.len() as u16);
            push_u16(&mut output, 0);
            output.extend_from_slice(name);
            output.extend_from_slice(content);

            push_u32(&mut central, 0x0201_4b50);
            push_u16(&mut central, 20);
            push_u16(&mut central, 20);
            push_u16(&mut central, 0);
            push_u16(&mut central, 0);
            push_u16(&mut central, 0);
            push_u16(&mut central, 0);
            push_u32(&mut central, checksum);
            push_u32(&mut central, content.len() as u32);
            push_u32(&mut central, content.len() as u32);
            push_u16(&mut central, name.len() as u16);
            push_u16(&mut central, 0);
            push_u16(&mut central, 0);
            push_u16(&mut central, 0);
            push_u16(&mut central, 0);
            push_u32(&mut central, 0);
            push_u32(&mut central, offset);
            central.extend_from_slice(name);
        }
        let central_offset = output.len() as u32;
        let central_size = central.len() as u32;
        output.extend_from_slice(&central);
        push_u32(&mut output, 0x0605_4b50);
        push_u16(&mut output, 0);
        push_u16(&mut output, 0);
        push_u16(&mut output, entries.len() as u16);
        push_u16(&mut output, entries.len() as u16);
        push_u32(&mut output, central_size);
        push_u32(&mut output, central_offset);
        push_u16(&mut output, 0);
        output
    }

    fn package(text: &str, active: bool) -> Vec<u8> {
        let mut entries = vec![
            ("[Content_Types].xml".to_owned(), "<Types/>".to_owned()),
            ("_rels/.rels".to_owned(), "<Relationships/>".to_owned()),
            (
                "word/document.xml".to_owned(),
                format!(
                    "<w:document xmlns:w=\"w\"><w:body><w:p><w:r><w:t>{text}</w:t></w:r></w:p></w:body></w:document>"
                ),
            ),
        ];
        if active {
            entries.push(("word/vbaProject.bin".to_owned(), "inert".to_owned()));
        }
        stored_zip(&entries)
    }

    fn extraction_request(source: &[u8]) -> StructuredSourceExtractionRequest {
        StructuredSourceExtractionRequest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            source_id: "prepared-word".to_owned(),
            media_type: DOCX_MEDIA_TYPE.to_owned(),
            source_sha256: sha256(source),
            source_path: WorkspacePath::new(
                WorkspaceId::from_raw("word-runtime"),
                ["captured", "source.docx"],
            )
            .expect("path"),
            maximum_sections: 100,
            maximum_output_bytes: 1_024 * 1_024,
        }
    }

    fn tool_request(kind: ArtifactToolKind, freshness: &str) -> Vec<u8> {
        serde_json::to_vec(&ArtifactRequest {
            schema_version: 1,
            call_id: format!("word-{}", kind.id()),
            source_id: (kind != ArtifactToolKind::List).then(|| "prepared-word".to_owned()),
            section_id: None,
            range: None,
            query: (kind == ArtifactToolKind::Search).then(|| "Evidence".to_owned()),
            freshness_sha256: (kind != ArtifactToolKind::List).then(|| freshness.to_owned()),
            output_identity: format!("word-output-{}", kind.id()),
            limits: ArtifactLimits::default(),
            call_depth: 0,
        })
        .expect("request")
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

    fn context_plan(
        profile_id: &str,
        binding: &ExactTokenCounterBinding,
        source_tokens: u32,
    ) -> ModelContextWindowPlan {
        let included = |tokens: u32| AllocatedContextPartition {
            requested_tokens: tokens,
            minimum_tokens: u32::from(tokens > 0),
            allocated_tokens: tokens,
            disposition: ContextPartitionDisposition::Included,
            reason_code: None,
        };
        let mut plan = ModelContextWindowPlan {
            schema_version: CONTRACT_SCHEMA_VERSION,
            model_profile_id: profile_id.to_owned(),
            model_manifest_sha256: "1".repeat(64),
            model_runtime_sha256: "2".repeat(64),
            tokenizer_sha256: binding.tokenizer_sha256.clone(),
            token_counter_sha256: binding.token_counter_sha256.clone(),
            total_window_tokens: source_tokens + 5,
            system_and_tool_tokens: 1,
            user_input_tokens: 1,
            source_artifacts: included(source_tokens),
            retrieved_context: included(0),
            workflow_recovery_reserve_tokens: 1,
            output_reserve_tokens: 1,
            safety_margin_tokens: 1,
            unallocated_tokens: 0,
            plan_sha256: "0".repeat(64),
        };
        plan.plan_sha256 = sha256(&serde_json::to_vec(&plan).expect("plan"));
        plan
    }

    fn payload_reference(source: &[u8]) -> RuntimeArtifactRef {
        RuntimeArtifactRef {
            schema_version: CONTRACT_SCHEMA_VERSION,
            artifact_id: RuntimeArtifactId::from_raw("persisted-word-payload"),
            manifest_sha256: "b".repeat(64),
            payload_sha256: sha256(source),
            byte_size: source.len() as u64,
            media_type: DOCX_MEDIA_TYPE.to_owned(),
        }
    }

    #[test]
    fn prepared_word_uses_common_dispatcher_without_parser_or_effect_bypass() {
        let source = package("Evidence", false);
        let mut service = WordSourceArtifactService::default();
        let outcome = service
            .admit(
                &extraction_request(&source),
                &source,
                ArtifactClassification::Internal,
                "attachment",
                &"a".repeat(64),
                &mut || false,
            )
            .expect("admit");
        assert!(!outcome.cache_hit);
        let cached = service
            .admit(
                &extraction_request(&source),
                &source,
                ArtifactClassification::Internal,
                "attachment",
                &"a".repeat(64),
                &mut || false,
            )
            .expect("cache");
        assert!(cached.cache_hit);
        let mut ledger = ArtifactAttemptLedger::default();
        for kind in [
            ArtifactToolKind::List,
            ArtifactToolKind::Metadata,
            ArtifactToolKind::Read,
            ArtifactToolKind::Sections,
            ArtifactToolKind::Search,
        ] {
            let result = dispatch_word_source_artifact(
                kind,
                &tool_request(kind, &outcome.manifest.freshness.manifest_sha256),
                &service,
                true,
                ArtifactExecutionSignal::Continue,
                &mut ledger,
            )
            .unwrap_or_else(|error| panic!("{kind:?}: {error:?}"));
            assert_eq!(result.outcome, ArtifactOutcome::Succeeded);
            assert!(result.production_execution);
            assert!(!result.receipt.parser_launched);
            assert!(!result.receipt.network_accessed);
            assert!(!result.receipt.workspace_mutated);
        }
    }

    #[test]
    fn cancellation_invalidation_delete_and_reattach_are_truthful() {
        let first = package("First", false);
        let second = package("Second", false);
        let active = package("Unsafe", true);
        let mut service = WordSourceArtifactService::default();
        assert_eq!(
            service.admit(
                &extraction_request(&first),
                &first,
                ArtifactClassification::Internal,
                "attachment",
                &"a".repeat(64),
                &mut || true,
            ),
            Err(StructuredSourceExtractionError::Cancelled)
        );
        let first_outcome = service
            .admit(
                &extraction_request(&first),
                &first,
                ArtifactClassification::Internal,
                "attachment",
                &"a".repeat(64),
                &mut || false,
            )
            .expect("first");
        let second_outcome = service
            .admit(
                &extraction_request(&second),
                &second,
                ArtifactClassification::Internal,
                "attachment",
                &"a".repeat(64),
                &mut || false,
            )
            .expect("invalidate");
        assert_ne!(
            first_outcome.manifest.payload_sha256,
            second_outcome.manifest.payload_sha256
        );
        assert_eq!(second_outcome.manifest.freshness.observed_sequence, 2);
        assert!(service.delete("prepared-word"));
        assert!(service.is_empty());
        let reattached = service
            .admit(
                &extraction_request(&first),
                &first,
                ArtifactClassification::Internal,
                "attachment",
                &"a".repeat(64),
                &mut || false,
            )
            .expect("reattach");
        assert!(!reattached.cache_hit);
        assert_eq!(reattached.manifest.freshness.observed_sequence, 1);
        assert_eq!(
            service.admit(
                &extraction_request(&active),
                &active,
                ArtifactClassification::Internal,
                "attachment",
                &"a".repeat(64),
                &mut || false,
            ),
            Err(StructuredSourceExtractionError::Quarantined)
        );
    }

    #[test]
    fn persisted_manifest_restarts_only_from_exact_existing_payload_reference() {
        let source = package("Restart evidence", false);
        let request = extraction_request(&source);
        let retention = PreparedWordSourceRetention::PolicyPersisted {
            artifact: payload_reference(&source),
            policy_sha256: "c".repeat(64),
        };
        let mut before_restart = WordSourceArtifactService::default();
        let admitted = before_restart
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
        let serialized = serde_json::to_vec(&admitted.prepared_manifest).expect("serialize");
        assert!(
            !serialized
                .windows(b"Restart evidence".len())
                .any(|window| window == b"Restart evidence")
        );
        assert_eq!(
            serde_json::from_slice::<PreparedWordSourceManifest>(&serialized).expect("deserialize"),
            admitted.prepared_manifest
        );
        assert_eq!(
            before_restart.prepared_manifest("prepared-word"),
            Some(admitted.prepared_manifest.clone())
        );
        assert_eq!(
            admitted.prepared_manifest.section_count as usize
                + admitted.prepared_manifest.warnings.len(),
            before_restart
                .source("prepared-word")
                .expect("native projection")
                .sections
                .len()
        );
        assert!(!admitted.prepared_manifest.warnings.is_empty());

        let mut cancelled_restart = WordSourceArtifactService::default();
        assert_eq!(
            cancelled_restart.restore_persisted(
                &admitted.prepared_manifest,
                &request,
                &source,
                &mut || true,
            ),
            Err(StructuredSourceExtractionError::Cancelled)
        );
        assert!(cancelled_restart.is_empty());

        let mut after_restart = WordSourceArtifactService::default();
        after_restart
            .restore_persisted(&admitted.prepared_manifest, &request, &source, &mut || {
                false
            })
            .expect("exact restart");
        assert_eq!(
            after_restart.prepared_manifest("prepared-word"),
            Some(admitted.prepared_manifest.clone())
        );
        assert_eq!(
            after_restart
                .source("prepared-word")
                .expect("restored source")
                .manifest,
            admitted.manifest
        );

        let mut corrupt = source.clone();
        corrupt[0] ^= 1;
        let mut refused = WordSourceArtifactService::default();
        assert_eq!(
            refused.restore_persisted(&admitted.prepared_manifest, &request, &corrupt, &mut || {
                false
            },),
            Err(StructuredSourceExtractionError::InvalidInput)
        );
        assert!(refused.is_empty());

        let mut mismatched_retention = admitted.prepared_manifest.clone();
        if let PreparedWordSourceRetention::PolicyPersisted { artifact, .. } =
            &mut mismatched_retention.retention
        {
            artifact.payload_sha256 = "e".repeat(64);
        }
        mismatched_retention.manifest_sha256 =
            prepared_manifest_sha256(&mismatched_retention).unwrap();
        assert_eq!(
            refused.restore_persisted(&mismatched_retention, &request, &source, &mut || false),
            Err(StructuredSourceExtractionError::InvalidInput)
        );
        assert!(refused.is_empty());

        let mut stale_extractor = admitted.prepared_manifest.clone();
        stale_extractor.extractor_sha256 = "d".repeat(64);
        stale_extractor.manifest_sha256 = prepared_manifest_sha256(&stale_extractor).unwrap();
        assert_eq!(
            refused.restore_persisted(&stale_extractor, &request, &source, &mut || false),
            Err(StructuredSourceExtractionError::InvalidInput)
        );
        assert!(refused.is_empty());
    }

    #[test]
    fn word_sections_have_complete_multi_profile_and_combined_budget_accounting() {
        let first = package("Alpha evidence has several tokens", false);
        let second = package("Beta evidence also has several tokens", false);
        let mut first_request = extraction_request(&first);
        first_request.source_id = "prepared-word-a".to_owned();
        let mut second_request = extraction_request(&second);
        second_request.source_id = "prepared-word-b".to_owned();
        let mut service = WordSourceArtifactService::default();
        for (request, source) in [
            (&first_request, first.as_slice()),
            (&second_request, second.as_slice()),
        ] {
            service
                .admit(
                    request,
                    source,
                    ArtifactClassification::Internal,
                    "attachment",
                    &"a".repeat(64),
                    &mut || false,
                )
                .expect("admit combined source");
        }

        for (index, profile) in ["small-profile", "large-profile"].into_iter().enumerate() {
            let binding = ExactTokenCounterBinding {
                token_counter_id: format!("word-counter-{index}"),
                token_counter_sha256: format!("{:064x}", index + 3),
                tokenizer_sha256: format!("{:064x}", index + 5),
            };
            let plan = context_plan(profile, &binding, if index == 0 { 4 } else { 64 });
            let manifest = service
                .compile_context(
                    format!("word-context-manifest-{index}"),
                    ContextPacketId::from_raw(format!("word-context-packet-{index}")),
                    &plan,
                    4_096,
                    &mut WordCounter {
                        binding: binding.clone(),
                    },
                    false,
                )
                .unwrap_or_else(|error| panic!("{profile} context accounting: {error:?}"));
            assert_eq!(manifest.context_window_plan_sha256, plan.plan_sha256);
            assert_eq!(
                manifest
                    .records
                    .iter()
                    .filter(|record| record.section_id.is_none())
                    .count(),
                2
            );
            assert!(manifest.records.iter().any(|record| {
                record.disposition == SourceContextDisposition::Included
                    && record.section_id.is_some()
            }));
            assert!(manifest.records.iter().any(|record| {
                matches!(
                    record.disposition,
                    SourceContextDisposition::Duplicate | SourceContextDisposition::Omitted
                )
            }));
            assert_eq!(manifest.used_tokens, manifest.packet.used_tokens);
            let expected_records = service
                .sources
                .values()
                .map(|source| {
                    1 + source.extraction.sections.len() + source.extraction.warnings.len()
                })
                .sum::<usize>();
            let expected_candidate_digests = service
                .sources
                .values()
                .flat_map(|source| {
                    source
                        .extraction
                        .sections
                        .iter()
                        .filter(|section| !section.content.is_empty())
                        .map(|section| sha256(section.content.as_bytes()))
                        .chain(
                            source
                                .extraction
                                .warnings
                                .iter()
                                .map(|warning| sha256(warning_content(warning).as_bytes())),
                        )
                })
                .collect::<BTreeSet<_>>();
            assert_eq!(manifest.records.len(), expected_records);
            assert_eq!(
                manifest.packet.accounting.len(),
                expected_candidate_digests.len()
            );
        }

        let binding = ExactTokenCounterBinding {
            token_counter_id: "word-counter-drift".to_owned(),
            token_counter_sha256: "7".repeat(64),
            tokenizer_sha256: "8".repeat(64),
        };
        let plan = context_plan("drift-profile", &binding, 32);
        let mut drifted = binding.clone();
        drifted.tokenizer_sha256 = "9".repeat(64);
        assert!(matches!(
            service.compile_context(
                "word-context-drift".to_owned(),
                ContextPacketId::from_raw("word-context-drift-packet"),
                &plan,
                4_096,
                &mut WordCounter { binding: drifted },
                false,
            ),
            Err(SourcePreparationError::TokenizerMismatch)
        ));
    }

    #[test]
    fn common_word_projection_is_identical_for_every_supported_client() {
        let source = package("Client-neutral evidence", false);
        let request = extraction_request(&source);
        let mut service = WordSourceArtifactService::default();
        service
            .admit(
                &request,
                &source,
                ArtifactClassification::Internal,
                "attachment",
                &"a".repeat(64),
                &mut || false,
            )
            .expect("admit client-neutral source");
        let binding = ExactTokenCounterBinding {
            token_counter_id: "word-client-counter".to_owned(),
            token_counter_sha256: "a".repeat(64),
            tokenizer_sha256: "b".repeat(64),
        };
        let plan = context_plan("word-client-profile", &binding, 32);
        let mut expected = None;
        for client in ["terminal", "headless", "native-chat"] {
            let prepared = service
                .prepared_manifest("prepared-word")
                .expect("prepared manifest");
            let native = service.source("prepared-word").expect("native projection");
            let context = service
                .compile_context(
                    "word-client-context".to_owned(),
                    ContextPacketId::from_raw("word-client-packet"),
                    &plan,
                    4_096,
                    &mut WordCounter {
                        binding: binding.clone(),
                    },
                    false,
                )
                .unwrap_or_else(|error| panic!("{client}: {error:?}"));
            let snapshot = sha256(
                &serde_json::to_vec(&(prepared, native.manifest, native.sections, context))
                    .expect("client snapshot"),
            );
            if let Some(expected) = &expected {
                assert_eq!(&snapshot, expected, "{client}");
            } else {
                expected = Some(snapshot);
            }
        }
    }
}
