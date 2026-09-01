//! Production adapter from runtime-owned prepared sources to the common artifact dispatcher.

use agentmage_capability_read_only::{
    ArtifactAttemptLedger, ArtifactBackend, ArtifactBackendClass, ArtifactClassification,
    ArtifactDispatchError, ArtifactExecutionSignal, ArtifactExtractionState, ArtifactFragment,
    ArtifactFreshness, ArtifactManifest, ArtifactProvenance, ArtifactRange, ArtifactResult,
    ArtifactSearchHit, ArtifactSection, ArtifactToolKind, FakeArtifactSource, dispatch_artifact,
};
use agentmage_kernel_contracts::ContextSensitivity;
use agentmage_kernel_engine::source_preparation::{
    PreparedExtractionState, PreparedSourceLifecycle, SourcePreparationService, SourceSectionKind,
};

/// Read-only projection of the runtime-owned prepared-source service.
pub struct NativeSourceArtifactBackend<'a> {
    service: &'a SourcePreparationService,
}

impl<'a> NativeSourceArtifactBackend<'a> {
    /// Binds one existing prepared-source service without copying or acquiring authority.
    #[must_use]
    pub const fn new(service: &'a SourcePreparationService) -> Self {
        Self { service }
    }
}

impl ArtifactBackend for NativeSourceArtifactBackend<'_> {
    fn manifests(&self) -> Vec<ArtifactManifest> {
        self.service
            .manifests()
            .into_iter()
            .map(|manifest| project_manifest(&manifest))
            .collect()
    }

    fn source(&self, source_id: &str) -> Option<FakeArtifactSource> {
        let view = self.service.source(source_id)?;
        let sections = view
            .sections
            .iter()
            .map(|section| ArtifactSection {
                section_id: section.section_id.clone(),
                title: section_title(section.kind).to_owned(),
                ordinal: section.ordinal,
                byte_len: section.content.len() as u64,
            })
            .collect();
        let mut fragments = Vec::with_capacity(view.lines.len() * 2 + view.sections.len());
        for line in &view.lines {
            fragments.push(ArtifactFragment::Byte {
                start: line.source_start_byte,
                end_exclusive: line.source_end_byte_exclusive,
                content: line.content.clone(),
            });
            fragments.push(ArtifactFragment::Line {
                start: line.line_number,
                end_exclusive: line.line_number.saturating_add(1),
                content: line.content.clone(),
            });
        }
        fragments.extend(
            view.sections
                .iter()
                .map(|section| ArtifactFragment::Section {
                    section_id: section.section_id.clone(),
                    content: section.content.clone(),
                }),
        );
        Some(FakeArtifactSource {
            manifest: project_manifest(&view.manifest),
            sections,
            fragments,
            redacted: view.manifest.sensitivity == ContextSensitivity::Restricted,
        })
    }

    fn backend_class(&self) -> ArtifactBackendClass {
        ArtifactBackendClass::ProductionPreparedSource
    }

    fn search(
        &self,
        source_id: &str,
        query: &str,
        maximum_items: usize,
    ) -> Option<Vec<ArtifactSearchHit>> {
        let view = self.service.source(source_id)?;
        self.service
            .search(
                source_id,
                &view.manifest.manifest_sha256,
                query,
                maximum_items.saturating_add(1),
            )
            .ok()
            .map(|hits| {
                hits.into_iter()
                    .map(|hit| ArtifactSearchHit {
                        fragment: ArtifactFragment::Line {
                            start: hit.line_number,
                            end_exclusive: hit.line_number.saturating_add(1),
                            content: hit.content,
                        },
                        match_start: hit.match_start,
                        match_end_exclusive: hit.match_end_exclusive,
                    })
                    .collect()
            })
    }

    fn range(
        &self,
        source_id: &str,
        range: &ArtifactRange,
        _maximum_units: u64,
    ) -> Option<Vec<ArtifactFragment>> {
        let view = self.service.source(source_id)?;
        let fragments = match range {
            ArtifactRange::Byte {
                start,
                end_exclusive,
            } => self
                .service
                .read_range(
                    source_id,
                    &view.manifest.manifest_sha256,
                    usize::try_from(*start).ok()?,
                    usize::try_from(*end_exclusive).ok()?,
                )
                .ok()
                .map(|content| {
                    vec![ArtifactFragment::Byte {
                        start: *start,
                        end_exclusive: *end_exclusive,
                        content,
                    }]
                })
                .unwrap_or_default(),
            ArtifactRange::Line {
                start,
                end_exclusive,
            } => {
                let selected = view
                    .lines
                    .iter()
                    .filter(|line| line.line_number >= *start && line.line_number < *end_exclusive)
                    .map(|line| line.content.as_str())
                    .collect::<Vec<_>>();
                if selected.is_empty() {
                    Vec::new()
                } else {
                    vec![ArtifactFragment::Line {
                        start: *start,
                        end_exclusive: *end_exclusive,
                        content: selected.join("\n"),
                    }]
                }
            }
            ArtifactRange::Section { section_id } => view
                .sections
                .iter()
                .find(|section| section.section_id == *section_id)
                .map(|section| {
                    vec![ArtifactFragment::Section {
                        section_id: section.section_id.clone(),
                        content: section.content.clone(),
                    }]
                })
                .unwrap_or_default(),
            ArtifactRange::Page { .. }
            | ArtifactRange::Sheet { .. }
            | ArtifactRange::Cell { .. } => Vec::new(),
        };
        Some(fragments)
    }

    fn read(
        &self,
        source_id: &str,
        section_id: Option<&str>,
        maximum_items: usize,
    ) -> Option<Vec<ArtifactFragment>> {
        let view = self.service.source(source_id)?;
        Some(
            view.sections
                .into_iter()
                .filter(|section| {
                    section_id.map_or(
                        section.kind != SourceSectionKind::DocumentRoot,
                        |expected| section.section_id == expected,
                    )
                })
                .take(maximum_items.saturating_add(1))
                .map(|section| ArtifactFragment::Section {
                    section_id: section.section_id,
                    content: section.content,
                })
                .collect(),
        )
    }

    fn log_errors(&self, source_id: &str, maximum_items: usize) -> Option<Vec<ArtifactFragment>> {
        let view = self.service.source(source_id)?;
        self.service
            .log_diagnostics(
                source_id,
                &view.manifest.manifest_sha256,
                maximum_items.saturating_add(1),
            )
            .ok()
            .map(|sections| {
                sections
                    .into_iter()
                    .map(|section| ArtifactFragment::Section {
                        section_id: section.section_id,
                        content: section.content,
                    })
                    .collect()
            })
    }
}

/// Dispatches one source-artifact request through the common closed capability protocol.
pub fn dispatch_native_source_artifact(
    kind: ArtifactToolKind,
    request_bytes: &[u8],
    service: &SourcePreparationService,
    workspace_read_authorized: bool,
    signal: ArtifactExecutionSignal,
    ledger: &mut ArtifactAttemptLedger,
) -> Result<ArtifactResult, ArtifactDispatchError> {
    dispatch_artifact(
        kind,
        request_bytes,
        &NativeSourceArtifactBackend::new(service),
        workspace_read_authorized,
        signal,
        ledger,
    )
}

fn project_manifest(
    manifest: &agentmage_kernel_engine::source_preparation::PreparedSourceManifest,
) -> ArtifactManifest {
    let lifecycle_current = manifest.lifecycle == PreparedSourceLifecycle::Current;
    let extraction_state = if lifecycle_current {
        match manifest.extraction_state {
            PreparedExtractionState::Complete => ArtifactExtractionState::Complete,
            PreparedExtractionState::Truncated => ArtifactExtractionState::Partial,
        }
    } else {
        ArtifactExtractionState::Failed
    };
    ArtifactManifest {
        source_id: manifest.source_id.clone(),
        media_type: manifest.media_type.clone(),
        byte_len: manifest.source_bytes,
        payload_sha256: manifest.source_sha256.clone(),
        classification: match manifest.sensitivity {
            ContextSensitivity::Public => ArtifactClassification::Public,
            ContextSensitivity::Internal => ArtifactClassification::Internal,
            ContextSensitivity::Private => ArtifactClassification::Confidential,
            ContextSensitivity::Restricted => ArtifactClassification::Restricted,
        },
        extraction_state,
        provenance: ArtifactProvenance {
            provenance_id: format!("prepared-source-{}", manifest.source_id),
            source_kind: manifest.source_kind.clone(),
            protected_origin_sha256: manifest.protected_origin_sha256.clone(),
        },
        freshness: ArtifactFreshness {
            manifest_sha256: manifest.manifest_sha256.clone(),
            observed_sequence: manifest.revision,
        },
        reason_code: match (lifecycle_current, manifest.extraction_state) {
            (false, _) => Some("source.lifecycle.not-current".to_owned()),
            (true, PreparedExtractionState::Truncated) => {
                Some("source.extraction.truncated".to_owned())
            }
            (true, PreparedExtractionState::Complete) => None,
        },
    }
}

const fn section_title(kind: SourceSectionKind) -> &'static str {
    match kind {
        SourceSectionKind::DocumentRoot => "Document root",
        SourceSectionKind::Text => "Text",
        SourceSectionKind::Error => "Error",
        SourceSectionKind::Warning => "Warning",
        SourceSectionKind::StackTrace => "Stack trace",
        SourceSectionKind::TestResult => "Test result",
        SourceSectionKind::Timestamp => "Timestamp",
    }
}

#[cfg(test)]
mod tests {
    use agentmage_capability_read_only::{
        ArtifactItem, ArtifactLimits, ArtifactOutcome, ArtifactRange, ArtifactRequest,
    };
    use agentmage_kernel_contracts::ContextSensitivity;
    use agentmage_kernel_engine::{
        model_orchestration_profile::ExactTokenCounterBinding,
        source_preparation::{
            ExactSourceTokenCounter, PreparedSourceRetention, SourceAdmissionRequest,
            SourceEncoding, SourceMediaFamily, SourcePreparationError, SourcePreparationLimits,
            SourcePreparationService,
        },
    };
    use sha2::{Digest, Sha256};

    use super::*;

    struct Counter;

    impl ExactSourceTokenCounter for Counter {
        fn binding(&self) -> ExactTokenCounterBinding {
            ExactTokenCounterBinding {
                token_counter_id: "native-source-test-counter".to_owned(),
                token_counter_sha256: "c".repeat(64),
                tokenizer_sha256: "d".repeat(64),
            }
        }

        fn count_tokens(&mut self, content: &str) -> Result<u32, SourcePreparationError> {
            u32::try_from(content.split_whitespace().count())
                .map_err(|_| SourcePreparationError::ResourceLimit)
        }
    }

    fn sha256(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    fn service() -> (SourcePreparationService, String) {
        let mut service = SourcePreparationService::new();
        let manifest = service
            .admit(
                SourceAdmissionRequest {
                    source_id: "native-log".to_owned(),
                    request_id: "native-request".to_owned(),
                    source_kind: "paste".to_owned(),
                    protected_origin_sha256: sha256(b"/protected/source.log"),
                    media_type: "text/x-log".to_owned(),
                    media_family: SourceMediaFamily::Log,
                    encoding: SourceEncoding::Auto,
                    sensitivity: ContextSensitivity::Internal,
                    retention: PreparedSourceRetention::Ephemeral,
                    collected_at_epoch_ms: 1,
                    limits: SourcePreparationLimits::default(),
                    bytes: b"2026-01-01 start\nERROR failure secret-like-value\n  at frame\n"
                        .to_vec(),
                },
                &mut Counter,
                &mut || false,
            )
            .expect("prepared source");
        (service, manifest.manifest_sha256)
    }

    fn request(kind: ArtifactToolKind, freshness: &str) -> Vec<u8> {
        serde_json::to_vec(&ArtifactRequest {
            schema_version: 1,
            call_id: format!("native-{}", kind.id()),
            source_id: (kind != ArtifactToolKind::List).then(|| "native-log".to_owned()),
            section_id: None,
            range: (kind == ArtifactToolKind::Range).then_some(ArtifactRange::Line {
                start: 1,
                end_exclusive: 4,
            }),
            query: (kind == ArtifactToolKind::Search).then(|| "failure".to_owned()),
            freshness_sha256: (kind != ArtifactToolKind::List).then(|| freshness.to_owned()),
            output_identity: format!("native-output-{}", kind.id()),
            limits: ArtifactLimits::default(),
            call_depth: 0,
        })
        .expect("request")
    }

    #[test]
    fn story_22_3_all_native_text_log_tools_use_one_production_dispatcher() {
        let (service, freshness) = service();
        let mut ledger = ArtifactAttemptLedger::default();
        for kind in ArtifactToolKind::ALL {
            let result = dispatch_native_source_artifact(
                kind,
                &request(kind, &freshness),
                &service,
                true,
                ArtifactExecutionSignal::Continue,
                &mut ledger,
            )
            .expect("native dispatch");
            assert!(result.verify(kind));
            assert_eq!(result.outcome, ArtifactOutcome::Succeeded);
            assert!(result.production_execution);
            assert!(!result.receipt.parser_launched);
            assert!(!result.receipt.network_accessed);
            assert!(!result.receipt.workspace_mutated);
            if kind == ArtifactToolKind::Search {
                assert_eq!(result.items.len(), 1);
                assert!(matches!(
                    &result.items[0],
                    ArtifactItem::SearchHit {
                        fragment: ArtifactFragment::Line { start: 2, .. },
                        ..
                    }
                ));
            }
            if kind == ArtifactToolKind::Range {
                assert_eq!(result.items.len(), 1);
                assert!(matches!(
                    &result.items[0],
                    ArtifactItem::Content {
                        fragment: ArtifactFragment::Line {
                            start: 1,
                            end_exclusive: 4,
                            ..
                        }
                    }
                ));
            }
            let encoded = serde_json::to_string(&result).expect("result JSON");
            assert!(!encoded.contains("/protected/source.log"));
        }
        assert_eq!(ledger.len(), ArtifactToolKind::ALL.len());
    }

    #[test]
    fn story_22_3_native_log_diagnostics_are_bounded_fresh_and_cancellable() {
        let (service, freshness) = service();
        let mut ledger = ArtifactAttemptLedger::default();
        let kind = ArtifactToolKind::LogErrors;
        let result = dispatch_native_source_artifact(
            kind,
            &request(kind, &freshness),
            &service,
            true,
            ArtifactExecutionSignal::Continue,
            &mut ledger,
        )
        .expect("diagnostics");
        assert_eq!(result.outcome, ArtifactOutcome::Succeeded);
        assert!(!result.items.is_empty());

        let stale = dispatch_native_source_artifact(
            kind,
            &request(kind, &"0".repeat(64)),
            &service,
            true,
            ArtifactExecutionSignal::Continue,
            &mut ArtifactAttemptLedger::default(),
        )
        .expect("stale receipt");
        assert_eq!(stale.outcome, ArtifactOutcome::Stale);

        let cancelled = dispatch_native_source_artifact(
            kind,
            &request(kind, &freshness),
            &service,
            true,
            ArtifactExecutionSignal::Cancelled,
            &mut ArtifactAttemptLedger::default(),
        )
        .expect("cancelled receipt");
        assert_eq!(cancelled.outcome, ArtifactOutcome::Cancelled);
        assert!(cancelled.production_execution);
    }
}
