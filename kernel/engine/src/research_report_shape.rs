//! Closed untrusted report drafts; validation alone establishes no source evidence.
use std::collections::BTreeSet;

use agentmage_kernel_contracts::RuntimeArtifactRef;
use serde::{Deserialize, Serialize};

use super::ResearchSourceSpan;
use crate::persistence::detect_secret_classes;

const MAX_DRAFT_BYTES: usize = 128 * 1024;
const MAX_SOURCES: usize = 20;
const MAX_CLAIMS: usize = 64;
const MAX_SPANS: usize = 128;
const MAX_TEXT_BYTES: usize = 4096;
const MAX_CONFLICTS: usize = 32;
const MAX_UNRESOLVED: usize = 32;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Bounded untrusted claims and complete canonical bundle references.
pub struct ResearchReportDraft {
    /// Only draft schema one is supported.
    pub schema_version: u16,
    /// Descriptive report identifier, not a runtime or authority identity.
    pub report_id: String,
    /// Full source bundles; provider metadata cannot replace them.
    pub sources: Vec<RuntimeArtifactRef>,
    /// Exact quotations referring to the source table.
    pub spans: Vec<ResearchSourceSpan>,
    /// Observations and explicitly limited interpretations.
    pub claims: Vec<ResearchReportClaimDraft>,
    /// Proposed unresolved conflicts between observed claims.
    pub conflicts: Vec<ResearchReportConflictDraft>,
    /// Explicit unresolved questions, not fabricated evidence of network failure.
    pub unresolved: Vec<String>,
}

// The first component supports only observed excerpts and explicitly labelled
// model interpretation. A deterministic-derivation label is not accepted until
// an actual named verifier and its receipt are implemented at the same owner.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
/// No model-selected deterministic-derivation or completion label is admitted.
pub enum ResearchReportClaimDraft {
    /// Exact excerpt presence, not proof of the publisher proposition.
    Observed {
        /// Unique identifier within this draft.
        claim_id: String,
        /// Index of the exact supporting quotation.
        span_index: u16,
    },
    /// Interpretation with source links and a mandatory visible limitation.
    ModelInference {
        /// Unique identifier within this draft.
        claim_id: String,
        /// Inert proposed interpretation.
        text: String,
        /// Sorted unique supporting quotation indexes.
        span_indexes: Vec<u16>,
        /// Explicit uncertainty; never a permission request or grant.
        limitation: String,
    },
}

impl ResearchReportClaimDraft {
    /// Existing evidence vocabulary; an observation confirms excerpt presence only.
    #[must_use]
    pub const fn evidence_label(&self) -> crate::web_research_safety::EvidenceLabel {
        match self {
            Self::Observed { .. } => crate::web_research_safety::EvidenceLabel::Observed,
            Self::ModelInference { .. } => {
                crate::web_research_safety::EvidenceLabel::ModelInference
            }
        }
    }

    pub(super) fn id(&self) -> &str {
        match self {
            Self::Observed { claim_id, .. } | Self::ModelInference { claim_id, .. } => claim_id,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Model-proposed conflict; canonical checking does not adjudicate its truth.
pub struct ResearchReportConflictDraft {
    /// Links observed claims only. The existence of a semantic contradiction is
    /// model-proposed and unresolved, never mechanically adjudicated as truth.
    pub claim_ids: Vec<String>,
    /// Visible unresolved limitation.
    pub limitation: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Content-free refusal of an untrusted report shape.
pub enum ResearchReportShapeError {
    /// Unknown schema or inconsistent identities, indexes or text.
    Invalid,
    /// Count, text or serialized-size ceiling exceeded.
    Limit,
    /// Detected secret material cannot enter a report.
    Secret,
}

impl ResearchReportDraft {
    /// Decodes only the bounded closed shape; no provenance or authority is granted.
    pub fn decode(bytes: &[u8]) -> Result<Self, ResearchReportShapeError> {
        if bytes.is_empty() || bytes.len() > MAX_DRAFT_BYTES {
            return Err(ResearchReportShapeError::Limit);
        }
        let draft: Self =
            serde_json::from_slice(bytes).map_err(|_| ResearchReportShapeError::Invalid)?;
        draft.validate()?;
        Ok(draft)
    }

    pub(super) fn validate(&self) -> Result<(), ResearchReportShapeError> {
        use ResearchReportShapeError as E;
        if self.schema_version != 1 || !report_id(&self.report_id) {
            return Err(E::Invalid);
        }
        if self.sources.is_empty()
            || self.sources.len() > MAX_SOURCES
            || self.claims.is_empty()
            || self.claims.len() > MAX_CLAIMS
            || self.spans.is_empty()
            || self.spans.len() > MAX_SPANS
            || self.conflicts.len() > MAX_CONFLICTS
            || self.unresolved.len() > MAX_UNRESOLVED
        {
            return Err(E::Limit);
        }
        let mut source_ids = BTreeSet::new();
        for source in &self.sources {
            // Shape bounds only. Reference/manifest/lifecycle verification must
            // use the existing canonical reader, never a model-provided manifest.
            if source.schema_version != agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION
                || source.artifact_id.as_str().is_empty()
                || source.artifact_id.as_str().len() > 256
                || source.byte_size == 0
                || source.byte_size > 16 * 1024
                || source.media_type != "application/json"
                || !report_digest(&source.manifest_sha256)
                || !report_digest(&source.payload_sha256)
            {
                return Err(E::Invalid);
            }
            if !source_ids.insert(source.artifact_id.clone()) {
                return Err(E::Invalid);
            }
        }
        let mut source_used = vec![false; self.sources.len()];
        for span in &self.spans {
            let Some(used) = source_used.get_mut(usize::from(span.source_index)) else {
                return Err(E::Invalid);
            };
            *used = true;
            if span.start_byte >= span.end_byte
                || span.body_sha256.len() != 64
                || !span
                    .body_sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(E::Invalid);
            }
            report_text(&span.excerpt, 1024)?;
        }
        if source_used.contains(&false) {
            return Err(E::Invalid);
        }
        let mut claim_ids = BTreeSet::new();
        let mut observed_ids = BTreeSet::new();
        let mut spans_used = vec![false; self.spans.len()];
        for claim in &self.claims {
            if !report_id(claim.id()) || !claim_ids.insert(claim.id()) {
                return Err(E::Invalid);
            }
            match claim {
                ResearchReportClaimDraft::Observed {
                    claim_id,
                    span_index,
                } => {
                    observed_ids.insert(claim_id.as_str());
                    let Some(used) = spans_used.get_mut(usize::from(*span_index)) else {
                        return Err(E::Invalid);
                    };
                    *used = true;
                }
                ResearchReportClaimDraft::ModelInference {
                    text,
                    span_indexes,
                    limitation,
                    ..
                } => {
                    report_text(text, MAX_TEXT_BYTES)?;
                    report_text(limitation, MAX_TEXT_BYTES)?;
                    if span_indexes.is_empty()
                        || span_indexes.len() > MAX_SOURCES
                        || span_indexes.windows(2).any(|w| w[0] >= w[1])
                    {
                        return Err(E::Invalid);
                    }
                    for index in span_indexes {
                        let Some(used) = spans_used.get_mut(usize::from(*index)) else {
                            return Err(E::Invalid);
                        };
                        *used = true;
                    }
                }
            }
        }
        if spans_used.contains(&false) {
            return Err(E::Invalid);
        }
        let mut conflict_sets = BTreeSet::new();
        for conflict in &self.conflicts {
            report_text(&conflict.limitation, MAX_TEXT_BYTES)?;
            if conflict.claim_ids.len() < 2
                || conflict.claim_ids.len() > MAX_SOURCES
                || conflict.claim_ids.windows(2).any(|w| w[0] >= w[1])
                || conflict
                    .claim_ids
                    .iter()
                    .any(|id| !observed_ids.contains(id.as_str()))
                || !conflict_sets.insert(&conflict.claim_ids)
            {
                return Err(E::Invalid);
            }
        }
        for unresolved in &self.unresolved {
            report_text(unresolved, MAX_TEXT_BYTES)?;
        }
        // Validate bounded typed callers as well as the bounded wire decoder.
        if serde_json::to_vec(self).map_err(|_| E::Invalid)?.len() > MAX_DRAFT_BYTES {
            return Err(E::Limit);
        }
        Ok(())
    }
}

fn report_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        && detect_secret_classes("research_identifier", value.as_bytes()).is_empty()
}

fn report_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn report_text(value: &str, maximum: usize) -> Result<(), ResearchReportShapeError> {
    if value.trim().is_empty() || value.len() > maximum {
        return Err(ResearchReportShapeError::Limit);
    }
    if value
        .chars()
        .any(|c| c.is_control() && !matches!(c, '\t' | '\n' | '\r'))
    {
        return Err(ResearchReportShapeError::Invalid);
    }
    if !detect_secret_classes("research_report", value.as_bytes()).is_empty() {
        return Err(ResearchReportShapeError::Secret);
    }
    Ok(())
}

#[cfg(test)]
mod shape_tests {
    use super::*;

    fn draft() -> ResearchReportDraft {
        ResearchReportDraft {
            schema_version: 1,
            report_id: "report-1".into(),
            sources: vec![RuntimeArtifactRef {
                schema_version: agentmage_kernel_contracts::CONTRACT_SCHEMA_VERSION,
                artifact_id: agentmage_kernel_contracts::RuntimeArtifactId::from_raw("bundle-1"),
                manifest_sha256: "a".repeat(64),
                payload_sha256: "b".repeat(64),
                byte_size: 123,
                media_type: "application/json".into(),
            }],
            spans: vec![ResearchSourceSpan {
                source_index: 0,
                body_sha256: "c".repeat(64),
                start_byte: 0,
                end_byte: 6,
                excerpt: "source".into(),
            }],
            claims: vec![ResearchReportClaimDraft::Observed {
                claim_id: "observed-1".into(),
                span_index: 0,
            }],
            conflicts: vec![],
            unresolved: vec!["Publication time is unknown.".into()],
        }
    }

    #[test]
    fn exact_shape_roundtrip_does_not_establish_canonical_evidence() {
        let original = draft();
        original.validate().unwrap();
        let bytes = serde_json::to_vec(&original).unwrap();
        assert!(ResearchReportDraft::decode(&bytes).unwrap() == original);
        // No store, native producer, receipt or runtime grant was involved.
        // This remains input validation only, never accepted report evidence.
    }

    #[test]
    fn unknown_authority_fields_and_unexecuted_derivation_refuse() {
        let original = serde_json::to_value(draft()).unwrap();
        for (level, field, value) in [
            (0, "approved", serde_json::json!(true)),
            (0, "schema_version", serde_json::json!(2)),
            (1, "approved", serde_json::json!(true)),
            (1, "kind", serde_json::json!("deterministic_derivation")),
        ] {
            let mut changed = original.clone();
            if level == 0 {
                changed[field] = value;
            } else {
                changed["claims"][0][field] = value;
            }
            assert!(ResearchReportDraft::decode(&serde_json::to_vec(&changed).unwrap()).is_err());
        }
    }

    #[test]
    fn duplicate_unused_and_out_of_range_identities_refuse() {
        type ShapeMutation = Box<dyn Fn(&mut ResearchReportDraft)>;
        let cases: Vec<ShapeMutation> = vec![
            Box::new(|d| d.sources.push(d.sources[0].clone())),
            Box::new(|d| d.claims.push(d.claims[0].clone())),
            Box::new(|d| d.spans.push(d.spans[0].clone())),
            Box::new(|d| d.spans[0].source_index = u16::MAX),
            Box::new(|d| d.spans[0].start_byte = d.spans[0].end_byte),
            Box::new(|d| d.spans[0].body_sha256 = "C".repeat(64)),
            Box::new(|d| {
                d.claims[0] = ResearchReportClaimDraft::Observed {
                    claim_id: "observed-1".into(),
                    span_index: u16::MAX,
                }
            }),
            Box::new(|d| {
                d.conflicts.push(ResearchReportConflictDraft {
                    claim_ids: vec!["observed-1".into(), "missing-claim".into()],
                    limitation: "This is an unverified proposed conflict.".into(),
                })
            }),
        ];
        for change in cases {
            let mut candidate = draft();
            change(&mut candidate);
            assert!(candidate.validate().is_err());
        }
    }

    #[test]
    fn inference_needs_bounded_citations_and_visible_nonsecret_limitation() {
        let inference =
            |limitation: &str, indexes: Vec<u16>| ResearchReportClaimDraft::ModelInference {
                claim_id: "inference-1".into(),
                text: "This interpretation is proposed, not an observed quotation.".into(),
                span_indexes: indexes,
                limitation: limitation.into(),
            };
        let mut candidate = draft();
        candidate.claims = vec![inference(
            "Source truth was not independently verified.",
            vec![0],
        )];
        assert!(candidate.validate().is_ok());
        for (limitation, indexes) in [
            ("".to_owned(), vec![0]),
            ("Unknown".to_owned(), vec![]),
            ("Unknown".to_owned(), vec![0, 0]),
            ("Unknown".to_owned(), vec![u16::MAX]),
            (format!("Bearer {}", "x".repeat(32)), vec![0]),
        ] {
            candidate.claims = vec![inference(&limitation, indexes)];
            assert!(candidate.validate().is_err());
        }
    }

    #[test]
    fn outer_and_typed_work_limits_refuse_without_truncation() {
        assert_eq!(
            ResearchReportDraft::decode(&vec![b' '; MAX_DRAFT_BYTES + 1]).err(),
            Some(ResearchReportShapeError::Limit)
        );
        let mut candidate = draft();
        candidate.unresolved = vec!["Unknown".into(); MAX_UNRESOLVED + 1];
        assert_eq!(candidate.validate(), Err(ResearchReportShapeError::Limit));
        candidate = draft();
        candidate.unresolved[0] = "x".repeat(MAX_TEXT_BYTES + 1);
        assert_eq!(candidate.validate(), Err(ResearchReportShapeError::Limit));
        candidate = draft();
        candidate.spans[0].excerpt = "source\0".into();
        assert_eq!(candidate.validate(), Err(ResearchReportShapeError::Invalid));
    }

    #[test]
    fn token_shaped_identifiers_cannot_bypass_report_text_filtering() {
        let token = format!("ghp_{}", "x".repeat(24));
        let mut candidate = draft();
        candidate.report_id = token.clone();
        assert!(candidate.validate().is_err());
        candidate = draft();
        candidate.claims[0] = ResearchReportClaimDraft::Observed {
            claim_id: token,
            span_index: 0,
        };
        assert!(candidate.validate().is_err());
    }
}
