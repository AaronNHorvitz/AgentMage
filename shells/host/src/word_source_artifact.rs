//! Runtime-owned DOCX preparation and common native artifact-tool projection.

use std::collections::BTreeMap;

use agentmage_capability_knowledge::WordStructuredSourceExtractor;
use agentmage_capability_read_only::{
    ArtifactAttemptLedger, ArtifactBackend, ArtifactBackendClass, ArtifactClassification,
    ArtifactDispatchError, ArtifactExecutionSignal, ArtifactExtractionState, ArtifactFragment,
    ArtifactFreshness, ArtifactManifest, ArtifactProvenance, ArtifactResult, ArtifactSection,
    ArtifactToolKind, FakeArtifactSource, dispatch_artifact,
};
use agentmage_kernel_contracts::{
    StructuredSourceExtraction, StructuredSourceExtractionError, StructuredSourceExtractionRequest,
    StructuredSourceExtractor, StructuredSourceSectionKind,
};
use sha2::{Digest, Sha256};

const MAX_PREPARED_WORD_SOURCES: usize = 1_024;

/// One content-free terminal result for a DOCX preparation attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordSourceAdmissionOutcome {
    /// Exact current artifact manifest.
    pub manifest: ArtifactManifest,
    /// True only when the exact source/extractor projection was already prepared.
    pub cache_hit: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PreparedWordSource {
    extraction: StructuredSourceExtraction,
    manifest: ArtifactManifest,
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

fn manifest_sha256(
    request: &StructuredSourceExtractionRequest,
    extractor_sha256: &str,
    revision: u64,
) -> String {
    sha256(
        format!(
            "{}\n{}\n{}\n{}\n{revision}",
            request.source_id, request.media_type, request.source_sha256, extractor_sha256,
        )
        .as_bytes(),
    )
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
        if !valid_id(source_kind) || !valid_sha256(protected_origin_sha256) {
            return Err(StructuredSourceExtractionError::InvalidInput);
        }
        let extractor = WordStructuredSourceExtractor;
        let extractor_sha256 = extractor.extractor_sha256();
        if let Some(current) = self.sources.get(&request.source_id)
            && current.extraction.source_sha256 == request.source_sha256
            && current.extraction.extractor_sha256 == extractor_sha256
        {
            if cancelled() {
                return Err(StructuredSourceExtractionError::Cancelled);
            }
            return Ok(WordSourceAdmissionOutcome {
                manifest: current.manifest.clone(),
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
        let manifest = ArtifactManifest {
            source_id: request.source_id.clone(),
            media_type: request.media_type.clone(),
            byte_len: source.len() as u64,
            payload_sha256: request.source_sha256.clone(),
            classification,
            extraction_state: ArtifactExtractionState::Complete,
            provenance: ArtifactProvenance {
                provenance_id: format!("structured-word-{}", request.source_id),
                source_kind: source_kind.to_owned(),
                protected_origin_sha256: protected_origin_sha256.to_owned(),
            },
            freshness: ArtifactFreshness {
                manifest_sha256: manifest_sha256(request, &extractor_sha256, revision),
                observed_sequence: revision,
            },
            reason_code: None,
        };
        self.sources.insert(
            request.source_id.clone(),
            PreparedWordSource {
                extraction,
                manifest: manifest.clone(),
            },
        );
        Ok(WordSourceAdmissionOutcome {
            manifest,
            cache_hit: false,
        })
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

fn section_title(kind: StructuredSourceSectionKind) -> &'static str {
    match kind {
        StructuredSourceSectionKind::Document => "Document",
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
    FakeArtifactSource {
        manifest: source.manifest.clone(),
        sections: source
            .extraction
            .sections
            .iter()
            .map(|section| ArtifactSection {
                section_id: section.section_id.clone(),
                title: section_title(section.kind).to_owned(),
                ordinal: section.ordinal,
                byte_len: section.content.len() as u64,
            })
            .collect(),
        fragments: source
            .extraction
            .sections
            .iter()
            .filter(|section| !section.content.is_empty())
            .map(|section| ArtifactFragment::Section {
                section_id: section.section_id.clone(),
                content: section.content.clone(),
            })
            .collect(),
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
        CONTRACT_SCHEMA_VERSION, DOCX_MEDIA_TYPE, WorkspaceId, WorkspacePath,
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
}
