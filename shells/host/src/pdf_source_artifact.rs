//! Runtime-owned PDF preparation, context accounting, and common artifact-tool projection.

use std::collections::{BTreeMap, BTreeSet};

use agentmage_capability_knowledge::PdfStructuredSourceExtractor;
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

const MAX_PREPARED_PDF_SOURCES: usize = 1_024;
const MAX_PDF_CONTEXT_RECORDS: usize = 4_096;

/// One exact prepared-PDF retention decision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PreparedPdfRetention {
    /// The projection exists only in this bounded host instance.
    Ephemeral,
    /// Original bytes already exist in the existing encrypted runtime-artifact authority.
    PolicyPersisted {
        /// Path-free content-addressed artifact reference.
        artifact: RuntimeArtifactRef,
        /// Exact policy revision authorizing persistence.
        policy_sha256: String,
    },
}

/// Digest-sealed content-free PDF preparation record used for restart and invalidation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedPdfSourceManifest {
    /// Contract schema version.
    pub schema_version: u16,
    /// Stable logical source identity.
    pub source_id: String,
    /// Exact declared media type.
    pub media_type: String,
    /// Complete original byte count.
    pub source_bytes: u64,
    /// Content address of the complete original PDF.
    pub source_sha256: String,
    /// Exact extractor implementation identity.
    pub extractor_sha256: String,
    /// Digest of the complete canonical projection.
    pub extraction_sha256: String,
    /// Number of canonical sections.
    pub section_count: u32,
    /// Number of canonical text bytes.
    pub output_bytes: u64,
    /// Whether extraction omitted no required content.
    pub extraction_complete: bool,
    /// Complete visible warnings.
    pub warnings: Vec<StructuredSourceWarning>,
    /// Classification applied before context or tool projection.
    pub classification: ArtifactClassification,
    /// Protected origin metadata digest.
    pub protected_origin_sha256: String,
    /// Existing artifact-store binding or bounded memory-only retention.
    pub retention: PreparedPdfRetention,
    /// Monotonic logical revision.
    pub revision: u64,
    /// Digest of this record with this field zeroed.
    pub manifest_sha256: String,
}

/// Result of one exact PDF admission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PdfSourceAdmissionOutcome {
    /// Common artifact-protocol manifest.
    pub manifest: ArtifactManifest,
    /// Complete restart record.
    pub prepared_manifest: PreparedPdfSourceManifest,
    /// True only when exact source, extractor, policy, and classification identities matched.
    pub cache_hit: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PreparedPdfSource {
    extraction: StructuredSourceExtraction,
    artifact_manifest: ArtifactManifest,
    prepared_manifest: PreparedPdfSourceManifest,
}

/// Bounded PDF host service. It retains projections, never original PDF bytes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PdfSourceArtifactService {
    sources: BTreeMap<String, PreparedPdfSource>,
}

fn sha256(value: &[u8]) -> String {
    Sha256::digest(value)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn manifest_sha256(
    manifest: &PreparedPdfSourceManifest,
) -> Result<String, StructuredSourceExtractionError> {
    let mut candidate = manifest.clone();
    candidate.manifest_sha256 = "0".repeat(64);
    serde_json::to_vec(&candidate)
        .map(|bytes| sha256(&bytes))
        .map_err(|_| StructuredSourceExtractionError::InvalidInput)
}

fn retention_valid(
    retention: &PreparedPdfRetention,
    request: &StructuredSourceExtractionRequest,
    source_bytes: u64,
) -> bool {
    match retention {
        PreparedPdfRetention::Ephemeral => true,
        PreparedPdfRetention::PolicyPersisted {
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

impl PdfSourceArtifactService {
    /// Prepares one exact PDF without granting persistence authority.
    pub fn admit(
        &mut self,
        request: &StructuredSourceExtractionRequest,
        source: &[u8],
        classification: ArtifactClassification,
        protected_origin_sha256: &str,
        cancelled: &mut dyn FnMut() -> bool,
    ) -> Result<PdfSourceAdmissionOutcome, StructuredSourceExtractionError> {
        self.admit_with_retention(
            request,
            source,
            classification,
            protected_origin_sha256,
            PreparedPdfRetention::Ephemeral,
            cancelled,
        )
    }

    /// Prepares one exact PDF with a caller-verified existing runtime-artifact binding.
    pub fn admit_with_retention(
        &mut self,
        request: &StructuredSourceExtractionRequest,
        source: &[u8],
        classification: ArtifactClassification,
        protected_origin_sha256: &str,
        retention: PreparedPdfRetention,
        cancelled: &mut dyn FnMut() -> bool,
    ) -> Result<PdfSourceAdmissionOutcome, StructuredSourceExtractionError> {
        let source_bytes = u64::try_from(source.len())
            .map_err(|_| StructuredSourceExtractionError::ResourceLimit)?;
        if !valid_sha256(protected_origin_sha256)
            || !retention_valid(&retention, request, source_bytes)
        {
            return Err(StructuredSourceExtractionError::InvalidInput);
        }
        let extractor = PdfStructuredSourceExtractor;
        let extractor_sha256 = extractor.extractor_sha256();
        if let Some(current) = self.sources.get(&request.source_id)
            && current.prepared_manifest.source_sha256 == request.source_sha256
            && current.prepared_manifest.extractor_sha256 == extractor_sha256
            && current.prepared_manifest.classification == classification
            && current.prepared_manifest.protected_origin_sha256 == protected_origin_sha256
            && current.prepared_manifest.retention == retention
        {
            if cancelled() {
                return Err(StructuredSourceExtractionError::Cancelled);
            }
            return Ok(PdfSourceAdmissionOutcome {
                manifest: current.artifact_manifest.clone(),
                prepared_manifest: current.prepared_manifest.clone(),
                cache_hit: true,
            });
        }
        if self.sources.len() >= MAX_PREPARED_PDF_SOURCES
            && !self.sources.contains_key(&request.source_id)
        {
            return Err(StructuredSourceExtractionError::ResourceLimit);
        }
        let extraction = extractor.extract(request, source, cancelled)?;
        let revision = self
            .sources
            .get(&request.source_id)
            .map_or(1, |current| current.prepared_manifest.revision + 1);
        let extraction_sha256 = sha256(
            &serde_json::to_vec(&extraction)
                .map_err(|_| StructuredSourceExtractionError::InvalidInput)?,
        );
        let mut prepared_manifest = PreparedPdfSourceManifest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            source_id: request.source_id.clone(),
            media_type: request.media_type.clone(),
            source_bytes,
            source_sha256: request.source_sha256.clone(),
            extractor_sha256,
            extraction_sha256,
            section_count: u32::try_from(extraction.sections.len())
                .map_err(|_| StructuredSourceExtractionError::ResourceLimit)?,
            output_bytes: extraction.output_bytes,
            extraction_complete: extraction.extraction_complete,
            warnings: extraction.warnings.clone(),
            classification,
            protected_origin_sha256: protected_origin_sha256.to_owned(),
            retention,
            revision,
            manifest_sha256: "0".repeat(64),
        };
        prepared_manifest.manifest_sha256 = manifest_sha256(&prepared_manifest)?;
        let artifact_manifest = artifact_manifest(&prepared_manifest);
        self.sources.insert(
            request.source_id.clone(),
            PreparedPdfSource {
                extraction,
                artifact_manifest: artifact_manifest.clone(),
                prepared_manifest: prepared_manifest.clone(),
            },
        );
        Ok(PdfSourceAdmissionOutcome {
            manifest: artifact_manifest,
            prepared_manifest,
            cache_hit: false,
        })
    }

    /// Reconstructs one persisted projection only when exact bytes reproduce the sealed record.
    pub fn restore_persisted(
        &mut self,
        expected: &PreparedPdfSourceManifest,
        request: &StructuredSourceExtractionRequest,
        source: &[u8],
        cancelled: &mut dyn FnMut() -> bool,
    ) -> Result<(), StructuredSourceExtractionError> {
        if expected.schema_version != CONTRACT_SCHEMA_VERSION
            || expected.manifest_sha256 != manifest_sha256(expected)?
            || expected.source_id != request.source_id
            || expected.media_type != request.media_type
            || expected.source_sha256 != request.source_sha256
            || expected.source_bytes != source.len() as u64
            || expected.revision == 0
            || !matches!(
                expected.retention,
                PreparedPdfRetention::PolicyPersisted { .. }
            )
            || self.sources.contains_key(&expected.source_id)
            || !retention_valid(&expected.retention, request, expected.source_bytes)
        {
            return Err(StructuredSourceExtractionError::InvalidInput);
        }
        let extractor = PdfStructuredSourceExtractor;
        if expected.extractor_sha256 != extractor.extractor_sha256() {
            return Err(StructuredSourceExtractionError::InvalidInput);
        }
        let extraction = extractor.extract(request, source, cancelled)?;
        let extraction_sha256 = sha256(
            &serde_json::to_vec(&extraction)
                .map_err(|_| StructuredSourceExtractionError::InvalidInput)?,
        );
        if extraction_sha256 != expected.extraction_sha256
            || extraction.sections.len() as u32 != expected.section_count
            || extraction.output_bytes != expected.output_bytes
            || extraction.extraction_complete != expected.extraction_complete
            || extraction.warnings != expected.warnings
        {
            return Err(StructuredSourceExtractionError::InvalidInput);
        }
        let artifact_manifest = artifact_manifest(expected);
        self.sources.insert(
            expected.source_id.clone(),
            PreparedPdfSource {
                extraction,
                artifact_manifest,
                prepared_manifest: expected.clone(),
            },
        );
        Ok(())
    }

    /// Returns a content-free exact restart manifest.
    #[must_use]
    pub fn prepared_manifest(&self, source_id: &str) -> Option<PreparedPdfSourceManifest> {
        self.sources
            .get(source_id)
            .map(|source| source.prepared_manifest.clone())
    }

    /// Composes PDF sections and warnings through the canonical context manager.
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
                let empty = section.content.is_empty();
                let record_index = records.len();
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
                    add_candidate(
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
                    add_candidate(
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
            if records.len() > MAX_PDF_CONTEXT_RECORDS {
                return Err(SourcePreparationError::ResourceLimit);
            }
        }
        let packet = compose_context(
            packet_id,
            &ContextCompositionBudget {
                max_bytes: maximum_bytes,
                max_tokens: plan.source_artifacts.allocated_tokens,
                max_items: MAX_PDF_CONTEXT_RECORDS as u32,
                token_counter_id: binding.token_counter_id.clone(),
            },
            candidates,
        )?;
        for accounting in &packet.accounting {
            let record = candidate_records
                .get(&accounting.item_id)
                .and_then(|index| records.get_mut(*index))
                .ok_or(SourcePreparationError::InvalidInput)?;
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

    /// Deletes a projection and all derived context/tool cache state, never original bytes.
    pub fn delete(&mut self, source_id: &str) -> bool {
        self.sources.remove(source_id).is_some()
    }

    /// Returns the number of current projections.
    #[must_use]
    pub fn len(&self) -> usize {
        self.sources.len()
    }

    /// Returns whether no projection is current.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }
}

#[allow(clippy::too_many_arguments)]
fn add_candidate(
    candidates: &mut Vec<ContextItemCandidate>,
    candidate_records: &mut BTreeMap<String, usize>,
    content_owners: &mut BTreeSet<String>,
    records: &mut [SourceContextRecord],
    prepared: &PreparedPdfSourceManifest,
    sensitivity: ContextSensitivity,
    section_id: &str,
    content: &str,
    digest: String,
    record_index: usize,
    counter: &mut impl ExactSourceTokenCounter,
) -> Result<(), SourcePreparationError> {
    let item_id = format!(
        "pdf-context-{}",
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

fn artifact_manifest(prepared: &PreparedPdfSourceManifest) -> ArtifactManifest {
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
            provenance_id: format!("structured-pdf-{}", prepared.source_id),
            source_kind: "captured_pdf".to_owned(),
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

fn project_source(source: &PreparedPdfSource) -> FakeArtifactSource {
    let mut sections = source
        .extraction
        .sections
        .iter()
        .enumerate()
        .map(|(index, section)| ArtifactSection {
            section_id: section.section_id.clone(),
            title: section_title(section.kind).to_owned(),
            ordinal: index as u32,
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
        let section_id = format!("warning:{}", warning.warning_id);
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
        manifest: source.artifact_manifest.clone(),
        sections,
        fragments,
        redacted: source.artifact_manifest.classification == ArtifactClassification::Restricted,
    }
}

impl ArtifactBackend for PdfSourceArtifactService {
    fn manifests(&self) -> Vec<ArtifactManifest> {
        self.sources
            .values()
            .map(|source| source.artifact_manifest.clone())
            .collect()
    }

    fn source(&self, source_id: &str) -> Option<FakeArtifactSource> {
        self.sources.get(source_id).map(project_source)
    }

    fn backend_class(&self) -> ArtifactBackendClass {
        ArtifactBackendClass::ProductionPreparedSource
    }
}

/// Dispatches a prepared PDF through the common native artifact protocol.
pub fn dispatch_pdf_source_artifact(
    kind: ArtifactToolKind,
    request_bytes: &[u8],
    service: &PdfSourceArtifactService,
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
    use agentmage_capability_read_only::{ArtifactLimits, ArtifactOutcome, ArtifactRequest};
    use agentmage_kernel_contracts::{
        PDF_MEDIA_TYPE, RuntimeArtifactId, WorkspaceId, WorkspacePath,
    };
    use agentmage_kernel_engine::model_orchestration_profile::{
        AllocatedContextPartition, ContextPartitionDisposition, ExactTokenCounterBinding,
    };
    use lopdf::content::{Content, Operation};
    use lopdf::{Document, Object, Stream, dictionary};

    use super::*;

    fn pdf(text: Option<&str>, image: bool) -> Vec<u8> {
        let mut document = Document::with_version("1.7");
        let pages_id = document.new_object_id();
        let font_id = document.add_object(dictionary! {
            "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
        });
        let mut resources = dictionary! { "Font" => dictionary! { "F1" => font_id } };
        if image {
            let image_id = document.add_object(Stream::new(
                dictionary! {
                    "Type" => "XObject", "Subtype" => "Image", "Width" => 1,
                    "Height" => 1, "ColorSpace" => "DeviceGray", "BitsPerComponent" => 8,
                },
                vec![0],
            ));
            resources.set("XObject", dictionary! { "Im1" => image_id });
        }
        let operations = text.map_or_else(Vec::new, |value| {
            vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), 12.into()]),
                Operation::new("Tj", vec![Object::string_literal(value)]),
                Operation::new("ET", vec![]),
            ]
        });
        let content_id = document.add_object(Stream::new(
            dictionary! {},
            Content { operations }.encode().expect("content"),
        ));
        let page_id = document.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id, "Contents" => content_id,
            "Resources" => resources, "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        });
        document.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages", "Kids" => vec![Object::Reference(page_id)], "Count" => 1,
            }),
        );
        let catalog_id =
            document.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        document.trailer.set("Root", catalog_id);
        let mut output = Vec::new();
        document.save_to(&mut output).expect("save");
        output
    }

    fn request(source: &[u8]) -> StructuredSourceExtractionRequest {
        StructuredSourceExtractionRequest {
            schema_version: CONTRACT_SCHEMA_VERSION,
            source_id: "prepared-pdf".to_owned(),
            media_type: PDF_MEDIA_TYPE.to_owned(),
            source_sha256: sha256(source),
            source_path: WorkspacePath::new(
                WorkspaceId::from_raw("pdf-runtime"),
                ["captured", "source.pdf"],
            )
            .expect("path"),
            maximum_sections: 100,
            maximum_output_bytes: 1_024 * 1_024,
        }
    }

    struct Counter(ExactTokenCounterBinding);

    impl ExactSourceTokenCounter for Counter {
        fn binding(&self) -> ExactTokenCounterBinding {
            self.0.clone()
        }

        fn count_tokens(&mut self, content: &str) -> Result<u32, SourcePreparationError> {
            u32::try_from(content.split_whitespace().count())
                .map_err(|_| SourcePreparationError::ResourceLimit)
        }
    }

    fn plan(profile_id: &str, binding: &ExactTokenCounterBinding) -> ModelContextWindowPlan {
        let included = |tokens| AllocatedContextPartition {
            requested_tokens: tokens,
            minimum_tokens: u32::from(tokens > 0),
            allocated_tokens: tokens,
            disposition: ContextPartitionDisposition::Included,
            reason_code: None,
        };
        let mut value = ModelContextWindowPlan {
            schema_version: CONTRACT_SCHEMA_VERSION,
            model_profile_id: profile_id.to_owned(),
            model_manifest_sha256: "1".repeat(64),
            model_runtime_sha256: "2".repeat(64),
            tokenizer_sha256: binding.tokenizer_sha256.clone(),
            token_counter_sha256: binding.token_counter_sha256.clone(),
            total_window_tokens: 69,
            system_and_tool_tokens: 1,
            user_input_tokens: 1,
            source_artifacts: included(64),
            retrieved_context: included(0),
            workflow_recovery_reserve_tokens: 1,
            output_reserve_tokens: 1,
            safety_margin_tokens: 1,
            unallocated_tokens: 0,
            plan_sha256: "0".repeat(64),
        };
        value.plan_sha256 = sha256(&serde_json::to_vec(&value).expect("plan"));
        value
    }

    #[test]
    fn admission_cache_context_tool_and_delete_share_exact_identities() {
        let source = pdf(Some("Page-cited evidence"), false);
        let initial_request = request(&source);
        let mut service = PdfSourceArtifactService::default();
        let first = service
            .admit(
                &initial_request,
                &source,
                ArtifactClassification::Internal,
                &"a".repeat(64),
                &mut || false,
            )
            .expect("admit");
        assert!(!first.cache_hit);
        let second = service
            .admit(
                &initial_request,
                &source,
                ArtifactClassification::Internal,
                &"a".repeat(64),
                &mut || false,
            )
            .expect("cache");
        assert!(second.cache_hit);
        assert_eq!(first.prepared_manifest, second.prepared_manifest);

        let binding = ExactTokenCounterBinding {
            token_counter_id: "pdf-counter".to_owned(),
            token_counter_sha256: "b".repeat(64),
            tokenizer_sha256: "c".repeat(64),
        };
        let context = service
            .compile_context(
                "pdf-context-manifest".to_owned(),
                ContextPacketId::from_raw("pdf-context-packet"),
                &plan("pdf-model-profile-a", &binding),
                8_192,
                &mut Counter(binding.clone()),
                false,
            )
            .expect("context");
        assert!(context.used_tokens > 0);
        assert!(context.records.iter().any(|record| {
            record
                .section_id
                .as_deref()
                .is_some_and(|id| id.contains(":span:"))
                && record.disposition == SourceContextDisposition::Included
        }));
        let second_context = service
            .compile_context(
                "pdf-context-manifest-b".to_owned(),
                ContextPacketId::from_raw("pdf-context-packet-b"),
                &plan("pdf-model-profile-b", &binding),
                8_192,
                &mut Counter(binding),
                false,
            )
            .expect("second model context");
        assert_ne!(
            context.context_window_plan_sha256,
            second_context.context_window_plan_sha256
        );
        assert_ne!(context.manifest_sha256, second_context.manifest_sha256);

        let bytes = serde_json::to_vec(&ArtifactRequest {
            schema_version: 1,
            call_id: "pdf-tool-call".to_owned(),
            source_id: Some(initial_request.source_id.clone()),
            section_id: None,
            range: None,
            query: Some("evidence".to_owned()),
            freshness_sha256: Some(first.prepared_manifest.manifest_sha256.clone()),
            output_identity: "pdf-tool-output".to_owned(),
            limits: ArtifactLimits::default(),
            call_depth: 0,
        })
        .expect("request");
        let result = dispatch_pdf_source_artifact(
            ArtifactToolKind::Search,
            &bytes,
            &service,
            true,
            ArtifactExecutionSignal::Continue,
            &mut ArtifactAttemptLedger::default(),
        )
        .expect("dispatch");
        assert_eq!(result.outcome, ArtifactOutcome::Succeeded);
        assert!(!result.items.is_empty());

        let replacement = pdf(Some("Replacement page evidence"), false);
        let replacement_request = request(&replacement);
        let refreshed = service
            .admit(
                &replacement_request,
                &replacement,
                ArtifactClassification::Internal,
                &"a".repeat(64),
                &mut || false,
            )
            .expect("refresh");
        assert_eq!(refreshed.prepared_manifest.revision, 2);
        assert_ne!(
            first.prepared_manifest.manifest_sha256,
            refreshed.prepared_manifest.manifest_sha256
        );
        let stale = dispatch_pdf_source_artifact(
            ArtifactToolKind::Search,
            &bytes,
            &service,
            true,
            ArtifactExecutionSignal::Continue,
            &mut ArtifactAttemptLedger::default(),
        )
        .expect("stale dispatch result");
        assert_eq!(stale.outcome, ArtifactOutcome::Stale);
        assert!(service.delete(&initial_request.source_id));
        assert!(service.is_empty());
    }

    #[test]
    fn restart_refresh_cancellation_and_no_ocr_degrade_fail_closed() {
        let source = pdf(None, true);
        let request = request(&source);
        let retained = PreparedPdfRetention::PolicyPersisted {
            artifact: RuntimeArtifactRef {
                schema_version: CONTRACT_SCHEMA_VERSION,
                artifact_id: RuntimeArtifactId::from_raw("pdf-artifact"),
                manifest_sha256: "d".repeat(64),
                payload_sha256: request.source_sha256.clone(),
                byte_size: source.len() as u64,
                media_type: PDF_MEDIA_TYPE.to_owned(),
            },
            policy_sha256: "e".repeat(64),
        };
        let mut service = PdfSourceArtifactService::default();
        let admitted = service
            .admit_with_retention(
                &request,
                &source,
                ArtifactClassification::Public,
                &"f".repeat(64),
                retained,
                &mut || false,
            )
            .expect("admit scan");
        assert_eq!(
            admitted.manifest.extraction_state,
            ArtifactExtractionState::Partial
        );
        assert!(
            admitted.prepared_manifest.warnings.iter().any(|warning| {
                warning.reason_code == "pdf.page.scanned-candidate-ocr-required"
            })
        );

        let mut restored = PdfSourceArtifactService::default();
        restored
            .restore_persisted(&admitted.prepared_manifest, &request, &source, &mut || {
                false
            })
            .expect("restore");
        let mut tampered = admitted.prepared_manifest.clone();
        tampered.output_bytes += 1;
        assert_eq!(
            PdfSourceArtifactService::default().restore_persisted(
                &tampered,
                &request,
                &source,
                &mut || false,
            ),
            Err(StructuredSourceExtractionError::InvalidInput)
        );
        assert_eq!(
            PdfSourceArtifactService::default().admit(
                &request,
                &source,
                ArtifactClassification::Public,
                &"f".repeat(64),
                &mut || true,
            ),
            Err(StructuredSourceExtractionError::Cancelled)
        );
    }
}
