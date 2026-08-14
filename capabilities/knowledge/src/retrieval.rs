//! Deterministic source-traceable knowledge retrieval without semantic components.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use agentmage_kernel_contracts::{WorkspacePath, WorkspaceScopePath};
use sha2::{Digest, Sha256};

use crate::ObsidianSourceRange;

const MAX_DOCUMENTS: usize = 100_000;
const MAX_FRAGMENTS_PER_DOCUMENT: usize = 100_000;
const MAX_QUERY_ITEMS: usize = 64;
const MAX_QUERY_ITEM_BYTES: usize = 256;
const MAX_RESULT_COUNT: u32 = 1_000;
const MAX_CONTEXT_BYTES: u64 = 4 * 1024 * 1024;
const MAX_FRAGMENT_BYTES: usize = 1024 * 1024;
const MAX_SYNTHESIS_BYTES: usize = 1024 * 1024;

/// Closed source file type used by deterministic metadata filters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum KnowledgeFileType {
    /// Markdown source.
    Markdown,
    /// Plain text source.
    Text,
    /// JSON source.
    Json,
}

/// Closed authority class used by the fixed ranking table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum KnowledgeSourceAuthority {
    /// User-owned canonical Markdown record.
    CanonicalMarkdown,
    /// Direct observed supporting evidence.
    DirectEvidence,
    /// Source-traceable derived projection.
    DerivedProjection,
}

/// Closed searchable fragment class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum KnowledgeSourceFragmentKind {
    /// User-visible title.
    Title,
    /// Canonical field or frontmatter property.
    Field,
    /// Tag value.
    Tag,
    /// Link or relationship value.
    Link,
    /// Date or timestamp value.
    Date,
    /// Task value.
    Task,
    /// Heading value.
    Heading,
    /// Other bounded metadata.
    Metadata,
    /// Raw bounded body excerpt.
    Body,
}

/// One exact source fragment and optional normalized fact identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeSourceFragment {
    /// Fragment class.
    pub kind: KnowledgeSourceFragmentKind,
    /// Exact searchable text.
    pub text: String,
    /// Exact source range.
    pub source_range: ObsidianSourceRange,
    /// Optional normalized fact key used only for contradiction detection.
    pub fact_key: Option<String>,
}

/// One complete source document supplied from an approved source adapter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeSourceDocument {
    /// Approved root containing the source path.
    pub root: WorkspaceScopePath,
    /// Canonical relative source path.
    pub path: WorkspacePath,
    /// Source file type.
    pub file_type: KnowledgeFileType,
    /// Authority class.
    pub authority: KnowledgeSourceAuthority,
    /// Exact observed content digest.
    pub content_sha256: String,
    /// Digest currently expected by the source of truth.
    pub current_content_sha256: String,
    /// ISO date on which the source was last verified.
    pub verified_on: String,
    /// ISO date represented by the source.
    pub source_date: String,
    /// Optional normalized note kind such as `handoff`.
    pub note_kind: Option<String>,
    /// Whether source metadata explicitly classifies this document historical.
    pub historical: bool,
    /// Whether policy denied retrieval from this source.
    pub denied: bool,
    /// Complete bounded source fragments.
    pub fragments: Vec<KnowledgeSourceFragment>,
}

/// Complete deterministic context query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeContextQuery {
    /// Exact case-insensitive terms, all required for a match.
    pub terms: Vec<String>,
    /// Exact case-insensitive phrases, all required for a match.
    pub phrases: Vec<String>,
    /// Approved roots, at least one.
    pub roots: Vec<WorkspaceScopePath>,
    /// Optional inclusive earliest source date.
    pub date_from: Option<String>,
    /// Optional inclusive latest source date.
    pub date_to: Option<String>,
    /// ISO date anchoring the fixed verification-recency score bands.
    pub as_of_date: String,
    /// Allowed file types, at least one.
    pub file_types: BTreeSet<KnowledgeFileType>,
    /// Allowed authority classes, at least one.
    pub authorities: BTreeSet<KnowledgeSourceAuthority>,
    /// Whether explicitly historical sources may enter.
    pub include_historical: bool,
    /// Maximum number of returned citations.
    pub max_results: u32,
    /// Maximum assembled context bytes.
    pub max_context_bytes: u64,
}

/// Per-hit freshness state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnowledgeFreshness {
    /// Observed and current content digests agree.
    Current,
    /// The source changed after observation.
    Stale,
}

/// Overall retrieval evidence state preserved through rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnowledgeEvidenceState {
    /// Current observed evidence was returned.
    Observed,
    /// At least one returned citation is stale.
    Stale,
    /// Current sources assert different values for one normalized fact.
    Conflicting,
    /// No admissible matching evidence exists.
    UnknownBlocked,
}

/// One explicit factor in a deterministic score trace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeScoreFactor {
    /// Stable factor identifier.
    pub factor: String,
    /// Signed integer contribution.
    pub points: i32,
}

/// One ranked source citation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeRetrievalHit {
    /// Canonical source path.
    pub path: WorkspacePath,
    /// Source content digest.
    pub content_sha256: String,
    /// Exact source range.
    pub source_range: ObsidianSourceRange,
    /// Fragment class.
    pub fragment_kind: KnowledgeSourceFragmentKind,
    /// Bounded exact cited text.
    pub text: String,
    /// Source authority.
    pub authority: KnowledgeSourceAuthority,
    /// Current or stale state.
    pub freshness: KnowledgeFreshness,
    /// Deterministic integer score.
    pub score: i32,
    /// Complete deterministic score trace.
    pub score_factors: Vec<KnowledgeScoreFactor>,
    /// Stable citation digest derived from path, hash, range, and fragment class.
    pub citation_sha256: String,
}

/// One deduplicated context entry with its citation preserved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeContextEntry {
    /// Exact bounded context text.
    pub text: String,
    /// Citation identity for the text.
    pub citation_sha256: String,
    /// Exact source path.
    pub path: WorkspacePath,
    /// Exact source range.
    pub source_range: ObsidianSourceRange,
}

/// Complete deterministic retrieval result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeRetrievalResult {
    /// Overall evidence state.
    pub evidence_state: KnowledgeEvidenceState,
    /// Stable ranked citations.
    pub hits: Vec<KnowledgeRetrievalHit>,
    /// Deduplicated budgeted context preserving citations.
    pub context: Vec<KnowledgeContextEntry>,
    /// Sorted normalized fact keys with contradictory current values.
    pub conflicting_fact_keys: Vec<String>,
    /// Number of denied matching source documents.
    pub denied_source_count: u64,
    /// Number of matched candidates omitted by result or byte budgets.
    pub omitted_candidate_count: u64,
    /// Fixed false semantic-component marker.
    pub semantic_components_used: bool,
}

/// Source-traceable input envelope for a later bounded synthesis implementation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeSynthesisEnvelope {
    /// Retrieval evidence state that synthesis may not change.
    pub evidence_state: KnowledgeEvidenceState,
    /// Exact bounded context supplied to synthesis.
    pub context: Vec<KnowledgeContextEntry>,
    /// Complete admissible citation set.
    pub citation_sha256: Vec<String>,
    /// Contradictory fact keys that synthesis must preserve.
    pub conflicting_fact_keys: Vec<String>,
}

/// One proposed synthesis output before deterministic rendering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeAnswerDraft {
    /// Evidence state copied exactly from the synthesis envelope.
    pub evidence_state: KnowledgeEvidenceState,
    /// Bounded answer body; empty for unknown or blocked evidence.
    pub text: String,
    /// Unique citations selected from the envelope.
    pub citation_sha256: Vec<String>,
}

/// Deterministically validated final knowledge answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnowledgeRenderedAnswer {
    /// Preserved evidence state.
    pub evidence_state: KnowledgeEvidenceState,
    /// Stable user-visible state label.
    pub state_label: String,
    /// Validated answer body or fixed blocked response.
    pub text: String,
    /// Validated source citation identities.
    pub citation_sha256: Vec<String>,
    /// Contradictory fact keys preserved for final display.
    pub conflicting_fact_keys: Vec<String>,
}

/// Content-free deterministic retrieval failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnowledgeRetrievalError {
    /// Query, source, metadata, path, digest, range, or budget is invalid.
    InvalidInput,
    /// Arithmetic or fixed resource bound was exceeded.
    ResourceLimit,
    /// Synthesis attempted to change evidence state or cite unavailable evidence.
    EvidenceDrift,
}

/// Creates the exact evidence-preserving boundary consumed by later synthesis.
#[must_use]
pub fn prepare_knowledge_synthesis(
    result: &KnowledgeRetrievalResult,
) -> KnowledgeSynthesisEnvelope {
    KnowledgeSynthesisEnvelope {
        evidence_state: result.evidence_state,
        context: result.context.clone(),
        citation_sha256: result
            .hits
            .iter()
            .map(|hit| hit.citation_sha256.clone())
            .collect(),
        conflicting_fact_keys: result.conflicting_fact_keys.clone(),
    }
}

/// Validates a synthesis draft and renders it without weakening evidence state.
pub fn render_knowledge_answer(
    envelope: &KnowledgeSynthesisEnvelope,
    draft: &KnowledgeAnswerDraft,
) -> Result<KnowledgeRenderedAnswer, KnowledgeRetrievalError> {
    if draft.evidence_state != envelope.evidence_state
        || draft.text.len() > MAX_SYNTHESIS_BYTES
        || draft.text.chars().any(char::is_control)
    {
        return Err(KnowledgeRetrievalError::EvidenceDrift);
    }
    let available = envelope
        .citation_sha256
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let selected = draft
        .citation_sha256
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if selected.len() != draft.citation_sha256.len()
        || !selected.is_subset(&available)
        || draft
            .citation_sha256
            .iter()
            .any(|value| !valid_sha256(value))
    {
        return Err(KnowledgeRetrievalError::EvidenceDrift);
    }
    let blocked = envelope.evidence_state == KnowledgeEvidenceState::UnknownBlocked;
    if blocked && (!draft.text.is_empty() || !draft.citation_sha256.is_empty())
        || !blocked && (draft.text.is_empty() || draft.citation_sha256.is_empty())
    {
        return Err(KnowledgeRetrievalError::EvidenceDrift);
    }
    Ok(KnowledgeRenderedAnswer {
        evidence_state: envelope.evidence_state,
        state_label: evidence_state_label(envelope.evidence_state).to_owned(),
        text: if blocked {
            "No admissible evidence is available.".to_owned()
        } else {
            draft.text.clone()
        },
        citation_sha256: draft.citation_sha256.clone(),
        conflicting_fact_keys: envelope.conflicting_fact_keys.clone(),
    })
}

/// Runs exact lexical and metadata retrieval with a fixed ranking decision table.
pub fn retrieve_knowledge(
    query: &KnowledgeContextQuery,
    documents: &[KnowledgeSourceDocument],
) -> Result<KnowledgeRetrievalResult, KnowledgeRetrievalError> {
    validate_query(query)?;
    if documents.len() > MAX_DOCUMENTS {
        return Err(KnowledgeRetrievalError::ResourceLimit);
    }
    for document in documents {
        validate_document(document)?;
        if date_ordinal(&document.verified_on)? > date_ordinal(&query.as_of_date)? {
            return Err(KnowledgeRetrievalError::InvalidInput);
        }
    }
    let normalized_terms = normalize_items(&query.terms)?;
    let normalized_phrases = normalize_items(&query.phrases)?;
    let mut candidates = Vec::new();
    let mut denied_source_count = 0_u64;
    let mut facts = BTreeMap::<String, BTreeSet<String>>::new();
    for document in documents {
        if !query
            .roots
            .iter()
            .any(|root| root.contains_path(&document.path))
            || !query.file_types.contains(&document.file_type)
            || !query.authorities.contains(&document.authority)
            || (!query.include_historical && document.historical)
            || query
                .date_from
                .as_ref()
                .is_some_and(|date| &document.source_date < date)
            || query
                .date_to
                .as_ref()
                .is_some_and(|date| &document.source_date > date)
        {
            continue;
        }
        let document_matches = document
            .fragments
            .iter()
            .any(|fragment| lexical_match(&fragment.text, &normalized_terms, &normalized_phrases));
        if document.denied {
            if document_matches {
                denied_source_count += 1;
            }
            continue;
        }
        for fragment in &document.fragments {
            if !lexical_match(&fragment.text, &normalized_terms, &normalized_phrases)
                || crate::domain::secret_candidate(&fragment.text)
            {
                continue;
            }
            let freshness = if document.content_sha256 == document.current_content_sha256 {
                KnowledgeFreshness::Current
            } else {
                KnowledgeFreshness::Stale
            };
            if freshness == KnowledgeFreshness::Current
                && let Some(key) = &fragment.fact_key
            {
                facts
                    .entry(key.clone())
                    .or_default()
                    .insert(fragment.text.clone());
            }
            candidates.push(build_hit(
                document,
                fragment,
                freshness,
                &normalized_terms,
                &normalized_phrases,
                &query.as_of_date,
            )?);
        }
    }
    candidates.sort_by(|left, right| {
        Reverse(left.score)
            .cmp(&Reverse(right.score))
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.source_range.cmp(&right.source_range))
            .then_with(|| left.citation_sha256.cmp(&right.citation_sha256))
    });
    let conflicting_fact_keys = facts
        .into_iter()
        .filter_map(|(key, values)| (values.len() > 1).then_some(key))
        .collect::<Vec<_>>();
    let candidate_count = candidates.len();
    candidates.truncate(query.max_results as usize);
    let context = assemble_context(&candidates, query.max_context_bytes)?;
    let hits = candidates
        .into_iter()
        .filter(|hit| {
            context
                .iter()
                .any(|entry| entry.citation_sha256 == hit.citation_sha256)
        })
        .collect::<Vec<_>>();
    let omitted_candidate_count = u64::try_from(candidate_count.saturating_sub(hits.len()))
        .map_err(|_| KnowledgeRetrievalError::ResourceLimit)?;
    let evidence_state = if hits.is_empty() {
        KnowledgeEvidenceState::UnknownBlocked
    } else if !conflicting_fact_keys.is_empty() {
        KnowledgeEvidenceState::Conflicting
    } else if hits
        .iter()
        .any(|hit| hit.freshness == KnowledgeFreshness::Stale)
    {
        KnowledgeEvidenceState::Stale
    } else {
        KnowledgeEvidenceState::Observed
    };
    Ok(KnowledgeRetrievalResult {
        evidence_state,
        hits,
        context,
        conflicting_fact_keys,
        denied_source_count,
        omitted_candidate_count,
        semantic_components_used: false,
    })
}

fn validate_query(query: &KnowledgeContextQuery) -> Result<(), KnowledgeRetrievalError> {
    if query.terms.is_empty() && query.phrases.is_empty()
        || query.terms.len().saturating_add(query.phrases.len()) > MAX_QUERY_ITEMS
        || query.roots.is_empty()
        || query.file_types.is_empty()
        || query.authorities.is_empty()
        || query.max_results == 0
        || query.max_results > MAX_RESULT_COUNT
        || query.max_context_bytes == 0
        || query.max_context_bytes > MAX_CONTEXT_BYTES
    {
        return Err(KnowledgeRetrievalError::InvalidInput);
    }
    date_ordinal(&query.as_of_date)?;
    if let Some(date) = &query.date_from {
        date_ordinal(date)?;
    }
    if let Some(date) = &query.date_to {
        date_ordinal(date)?;
    }
    if query
        .date_from
        .as_ref()
        .zip(query.date_to.as_ref())
        .is_some_and(|(from, to)| from > to)
    {
        return Err(KnowledgeRetrievalError::InvalidInput);
    }
    Ok(())
}

fn validate_document(document: &KnowledgeSourceDocument) -> Result<(), KnowledgeRetrievalError> {
    if !document.root.contains_path(&document.path)
        || !valid_sha256(&document.content_sha256)
        || !valid_sha256(&document.current_content_sha256)
        || document.fragments.len() > MAX_FRAGMENTS_PER_DOCUMENT
        || document.note_kind.as_ref().is_some_and(|value| {
            value.is_empty()
                || value.len() > 64
                || !value
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
    {
        return Err(KnowledgeRetrievalError::InvalidInput);
    }
    date_ordinal(&document.verified_on)?;
    date_ordinal(&document.source_date)?;
    for fragment in &document.fragments {
        if fragment.text.is_empty()
            || fragment.text.len() > MAX_FRAGMENT_BYTES
            || !valid_range(fragment.source_range)
            || fragment.fact_key.as_ref().is_some_and(|value| {
                value.is_empty()
                    || value.len() > 128
                    || crate::domain::secret_candidate(value)
                    || !value.bytes().all(|byte| {
                        byte.is_ascii_lowercase()
                            || byte.is_ascii_digit()
                            || matches!(byte, b'.' | b'-' | b'_')
                    })
            })
        {
            return Err(KnowledgeRetrievalError::InvalidInput);
        }
    }
    Ok(())
}

fn normalize_items(items: &[String]) -> Result<Vec<String>, KnowledgeRetrievalError> {
    items
        .iter()
        .map(|item| {
            if item.is_empty()
                || item.len() > MAX_QUERY_ITEM_BYTES
                || item.chars().any(char::is_control)
                || crate::domain::secret_candidate(item)
            {
                return Err(KnowledgeRetrievalError::InvalidInput);
            }
            Ok(item.to_lowercase())
        })
        .collect()
}

fn lexical_match(text: &str, terms: &[String], phrases: &[String]) -> bool {
    let normalized = text.to_lowercase();
    terms.iter().all(|term| normalized.contains(term))
        && phrases.iter().all(|phrase| normalized.contains(phrase))
}

fn build_hit(
    document: &KnowledgeSourceDocument,
    fragment: &KnowledgeSourceFragment,
    freshness: KnowledgeFreshness,
    terms: &[String],
    phrases: &[String],
    as_of_date: &str,
) -> Result<KnowledgeRetrievalHit, KnowledgeRetrievalError> {
    let phrase_count =
        i32::try_from(phrases.len()).map_err(|_| KnowledgeRetrievalError::ResourceLimit)?;
    let term_count =
        i32::try_from(terms.len()).map_err(|_| KnowledgeRetrievalError::ResourceLimit)?;
    let mut factors = vec![
        KnowledgeScoreFactor {
            factor: "exact-phrase".to_owned(),
            points: phrase_count.saturating_mul(100),
        },
        KnowledgeScoreFactor {
            factor: "exact-term".to_owned(),
            points: term_count.saturating_mul(20),
        },
        KnowledgeScoreFactor {
            factor: "fragment-kind".to_owned(),
            points: fragment_kind_points(fragment.kind),
        },
        KnowledgeScoreFactor {
            factor: "source-authority".to_owned(),
            points: authority_points(document.authority),
        },
        KnowledgeScoreFactor {
            factor: "freshness".to_owned(),
            points: match freshness {
                KnowledgeFreshness::Current => 25,
                KnowledgeFreshness::Stale => -50,
            },
        },
    ];
    if document.note_kind.as_deref() == Some("handoff") {
        factors.push(KnowledgeScoreFactor {
            factor: "handoff-note".to_owned(),
            points: 30,
        });
    }
    if document.historical {
        factors.push(KnowledgeScoreFactor {
            factor: "historical-source".to_owned(),
            points: -25,
        });
    }
    let age = date_ordinal(as_of_date)? - date_ordinal(&document.verified_on)?;
    factors.push(KnowledgeScoreFactor {
        factor: "verification-recency".to_owned(),
        points: if age <= 30 {
            15
        } else if age <= 180 {
            8
        } else {
            0
        },
    });
    let score = factors
        .iter()
        .try_fold(0_i32, |sum, factor| sum.checked_add(factor.points))
        .ok_or(KnowledgeRetrievalError::ResourceLimit)?;
    Ok(KnowledgeRetrievalHit {
        path: document.path.clone(),
        content_sha256: document.content_sha256.clone(),
        source_range: fragment.source_range,
        fragment_kind: fragment.kind,
        text: fragment.text.clone(),
        authority: document.authority,
        freshness,
        score,
        score_factors: factors,
        citation_sha256: citation_digest(document, fragment)?,
    })
}

fn assemble_context(
    candidates: &[KnowledgeRetrievalHit],
    max_context_bytes: u64,
) -> Result<Vec<KnowledgeContextEntry>, KnowledgeRetrievalError> {
    let mut used = 0_u64;
    let mut seen = BTreeSet::new();
    let mut context = Vec::new();
    for hit in candidates {
        if !seen.insert(hit.text.clone()) {
            continue;
        }
        let bytes =
            u64::try_from(hit.text.len()).map_err(|_| KnowledgeRetrievalError::ResourceLimit)?;
        let Some(next) = used.checked_add(bytes) else {
            return Err(KnowledgeRetrievalError::ResourceLimit);
        };
        if next > max_context_bytes {
            continue;
        }
        used = next;
        context.push(KnowledgeContextEntry {
            text: hit.text.clone(),
            citation_sha256: hit.citation_sha256.clone(),
            path: hit.path.clone(),
            source_range: hit.source_range,
        });
    }
    Ok(context)
}

fn citation_digest(
    document: &KnowledgeSourceDocument,
    fragment: &KnowledgeSourceFragment,
) -> Result<String, KnowledgeRetrievalError> {
    let bytes = serde_json::to_vec(&(
        &document.path,
        &document.content_sha256,
        fragment.source_range.start_line,
        fragment.source_range.start_column,
        fragment.source_range.end_line,
        fragment.source_range.end_column,
        fragment_kind_id(fragment.kind),
    ))
    .map_err(|_| KnowledgeRetrievalError::ResourceLimit)?;
    Ok(sha256(&bytes))
}

fn sha256(value: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(value) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

const fn fragment_kind_points(kind: KnowledgeSourceFragmentKind) -> i32 {
    match kind {
        KnowledgeSourceFragmentKind::Title => 30,
        KnowledgeSourceFragmentKind::Field => 26,
        KnowledgeSourceFragmentKind::Tag => 24,
        KnowledgeSourceFragmentKind::Link => 18,
        KnowledgeSourceFragmentKind::Date => 20,
        KnowledgeSourceFragmentKind::Task => 22,
        KnowledgeSourceFragmentKind::Heading => 28,
        KnowledgeSourceFragmentKind::Metadata => 16,
        KnowledgeSourceFragmentKind::Body => 10,
    }
}

const fn authority_points(authority: KnowledgeSourceAuthority) -> i32 {
    match authority {
        KnowledgeSourceAuthority::CanonicalMarkdown => 40,
        KnowledgeSourceAuthority::DirectEvidence => 30,
        KnowledgeSourceAuthority::DerivedProjection => 0,
    }
}

const fn fragment_kind_id(kind: KnowledgeSourceFragmentKind) -> &'static str {
    match kind {
        KnowledgeSourceFragmentKind::Title => "title",
        KnowledgeSourceFragmentKind::Field => "field",
        KnowledgeSourceFragmentKind::Tag => "tag",
        KnowledgeSourceFragmentKind::Link => "link",
        KnowledgeSourceFragmentKind::Date => "date",
        KnowledgeSourceFragmentKind::Task => "task",
        KnowledgeSourceFragmentKind::Heading => "heading",
        KnowledgeSourceFragmentKind::Metadata => "metadata",
        KnowledgeSourceFragmentKind::Body => "body",
    }
}

const fn evidence_state_label(state: KnowledgeEvidenceState) -> &'static str {
    match state {
        KnowledgeEvidenceState::Observed => "Observed",
        KnowledgeEvidenceState::Stale => "Stale",
        KnowledgeEvidenceState::Conflicting => "Conflicting",
        KnowledgeEvidenceState::UnknownBlocked => "Unknown/Blocked",
    }
}

fn valid_range(range: ObsidianSourceRange) -> bool {
    range.start_line > 0
        && range.start_column > 0
        && range.end_line >= range.start_line
        && range.end_column > 0
        && (range.end_line > range.start_line || range.end_column > range.start_column)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn date_ordinal(value: &str) -> Result<i64, KnowledgeRetrievalError> {
    if value.len() != 10
        || value.as_bytes().get(4) != Some(&b'-')
        || value.as_bytes().get(7) != Some(&b'-')
    {
        return Err(KnowledgeRetrievalError::InvalidInput);
    }
    let year = parse_date_part(&value[0..4])?;
    let month = parse_date_part(&value[5..7])?;
    let day = parse_date_part(&value[8..10])?;
    if year == 0 || !(1..=12).contains(&month) {
        return Err(KnowledgeRetrievalError::InvalidInput);
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_lengths = [
        31_i64,
        28 + i64::from(leap),
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let month_index =
        usize::try_from(month - 1).map_err(|_| KnowledgeRetrievalError::InvalidInput)?;
    if day == 0 || day > month_lengths[month_index] {
        return Err(KnowledgeRetrievalError::InvalidInput);
    }
    let prior_year = year - 1;
    let prior_month_days: i64 = month_lengths[..month_index].iter().sum();
    Ok(prior_year * 365 + prior_year / 4 - prior_year / 100
        + prior_year / 400
        + prior_month_days
        + day)
}

fn parse_date_part(value: &str) -> Result<i64, KnowledgeRetrievalError> {
    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(KnowledgeRetrievalError::InvalidInput);
    }
    value
        .parse::<i64>()
        .map_err(|_| KnowledgeRetrievalError::InvalidInput)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath, WorkspaceScopePath};

    use super::{
        KnowledgeAnswerDraft, KnowledgeContextQuery, KnowledgeEvidenceState, KnowledgeFileType,
        KnowledgeFreshness, KnowledgeRetrievalError, KnowledgeSourceAuthority,
        KnowledgeSourceDocument, KnowledgeSourceFragment, KnowledgeSourceFragmentKind,
        prepare_knowledge_synthesis, render_knowledge_answer, retrieve_knowledge,
    };
    use crate::ObsidianSourceRange;

    fn workspace() -> WorkspaceId {
        WorkspaceId::from_raw("workspace-retrieval")
    }

    fn root() -> WorkspaceScopePath {
        WorkspaceScopePath::new(workspace(), ["vault"]).expect("valid test root")
    }

    fn path(name: &str) -> WorkspacePath {
        WorkspacePath::new(workspace(), ["vault", name]).expect("valid test path")
    }

    fn source_range(line: u32) -> ObsidianSourceRange {
        ObsidianSourceRange {
            start_line: line,
            start_column: 1,
            end_line: line,
            end_column: 20,
        }
    }

    fn fragment(
        kind: KnowledgeSourceFragmentKind,
        text: &str,
        line: u32,
        fact_key: Option<&str>,
    ) -> KnowledgeSourceFragment {
        KnowledgeSourceFragment {
            kind,
            text: text.to_owned(),
            source_range: source_range(line),
            fact_key: fact_key.map(str::to_owned),
        }
    }

    fn document(
        name: &str,
        authority: KnowledgeSourceAuthority,
        note_kind: Option<&str>,
        current: bool,
        fragments: Vec<KnowledgeSourceFragment>,
    ) -> KnowledgeSourceDocument {
        KnowledgeSourceDocument {
            root: root(),
            path: path(name),
            file_type: KnowledgeFileType::Markdown,
            authority,
            content_sha256: "a".repeat(64),
            current_content_sha256: if current {
                "a".repeat(64)
            } else {
                "b".repeat(64)
            },
            verified_on: "2026-08-10".to_owned(),
            source_date: "2026-08-09".to_owned(),
            note_kind: note_kind.map(str::to_owned),
            historical: false,
            denied: false,
            fragments,
        }
    }

    fn query(term: &str) -> KnowledgeContextQuery {
        KnowledgeContextQuery {
            terms: vec![term.to_owned()],
            phrases: Vec::new(),
            roots: vec![root()],
            date_from: None,
            date_to: None,
            as_of_date: "2026-08-14".to_owned(),
            file_types: [KnowledgeFileType::Markdown].into_iter().collect(),
            authorities: [
                KnowledgeSourceAuthority::CanonicalMarkdown,
                KnowledgeSourceAuthority::DirectEvidence,
                KnowledgeSourceAuthority::DerivedProjection,
            ]
            .into_iter()
            .collect(),
            include_historical: false,
            max_results: 20,
            max_context_bytes: 16_384,
        }
    }

    #[test]
    fn fixed_ranking_prefers_current_canonical_handoff_and_exposes_trace() {
        let documents = vec![
            document(
                "stale.md",
                KnowledgeSourceAuthority::DerivedProjection,
                None,
                false,
                vec![fragment(
                    KnowledgeSourceFragmentKind::Body,
                    "needle stale",
                    1,
                    None,
                )],
            ),
            document(
                "evidence.md",
                KnowledgeSourceAuthority::DirectEvidence,
                None,
                true,
                vec![fragment(
                    KnowledgeSourceFragmentKind::Body,
                    "needle evidence",
                    1,
                    None,
                )],
            ),
            document(
                "handoff.md",
                KnowledgeSourceAuthority::CanonicalMarkdown,
                Some("handoff"),
                true,
                vec![fragment(
                    KnowledgeSourceFragmentKind::Body,
                    "needle current",
                    1,
                    None,
                )],
            ),
        ];
        let result = retrieve_knowledge(&query("needle"), &documents).expect("retrieval succeeds");

        assert_eq!(result.hits[0].path, path("handoff.md"));
        assert_eq!(result.hits[1].path, path("evidence.md"));
        assert_eq!(result.hits[2].path, path("stale.md"));
        assert!(
            result.hits[0]
                .score_factors
                .iter()
                .any(|factor| factor.factor == "handoff-note")
        );
        assert_eq!(result.evidence_state, KnowledgeEvidenceState::Stale);
        assert!(!result.semantic_components_used);
    }

    #[test]
    fn searches_every_fragment_kind_and_applies_metadata_filters() {
        let kinds = [
            KnowledgeSourceFragmentKind::Title,
            KnowledgeSourceFragmentKind::Field,
            KnowledgeSourceFragmentKind::Tag,
            KnowledgeSourceFragmentKind::Link,
            KnowledgeSourceFragmentKind::Date,
            KnowledgeSourceFragmentKind::Task,
            KnowledgeSourceFragmentKind::Heading,
            KnowledgeSourceFragmentKind::Metadata,
            KnowledgeSourceFragmentKind::Body,
        ];
        let fragments = kinds
            .into_iter()
            .enumerate()
            .map(|(index, kind)| fragment(kind, &format!("needle {index}"), index as u32 + 1, None))
            .collect();
        let mut source = document(
            "all.md",
            KnowledgeSourceAuthority::CanonicalMarkdown,
            None,
            true,
            fragments,
        );
        source.source_date = "2026-07-01".to_owned();
        let mut exact_query = query("needle");
        exact_query.date_from = Some("2026-07-01".to_owned());
        exact_query.date_to = Some("2026-07-01".to_owned());

        let result = retrieve_knowledge(&exact_query, &[source]).expect("retrieval succeeds");
        let actual: BTreeSet<_> = result.hits.iter().map(|hit| hit.fragment_kind).collect();
        assert_eq!(actual, kinds.into_iter().collect());
        assert_eq!(result.hits.len(), 9);
    }

    #[test]
    fn ties_are_stable_and_citations_are_content_bound() {
        let first = document(
            "a.md",
            KnowledgeSourceAuthority::CanonicalMarkdown,
            None,
            true,
            vec![fragment(
                KnowledgeSourceFragmentKind::Body,
                "needle a",
                2,
                None,
            )],
        );
        let second = document(
            "b.md",
            KnowledgeSourceAuthority::CanonicalMarkdown,
            None,
            true,
            vec![fragment(
                KnowledgeSourceFragmentKind::Body,
                "needle b",
                2,
                None,
            )],
        );
        let result = retrieve_knowledge(&query("needle"), &[second, first.clone()])
            .expect("retrieval succeeds");
        assert_eq!(result.hits[0].path, path("a.md"));
        assert_eq!(result.hits[0].citation_sha256.len(), 64);

        let mut changed = first;
        changed.content_sha256 = "c".repeat(64);
        changed.current_content_sha256 = "c".repeat(64);
        let changed_result =
            retrieve_knowledge(&query("needle"), &[changed]).expect("retrieval succeeds");
        assert_ne!(
            result.hits[0].citation_sha256,
            changed_result.hits[0].citation_sha256
        );
    }

    #[test]
    fn contradictions_and_staleness_remain_explicit() {
        let open = document(
            "open.md",
            KnowledgeSourceAuthority::CanonicalMarkdown,
            None,
            true,
            vec![fragment(
                KnowledgeSourceFragmentKind::Field,
                "status open",
                1,
                Some("task.status"),
            )],
        );
        let closed = document(
            "closed.md",
            KnowledgeSourceAuthority::DirectEvidence,
            None,
            true,
            vec![fragment(
                KnowledgeSourceFragmentKind::Field,
                "status closed",
                1,
                Some("task.status"),
            )],
        );
        let result =
            retrieve_knowledge(&query("status"), &[open, closed]).expect("retrieval succeeds");
        assert_eq!(result.evidence_state, KnowledgeEvidenceState::Conflicting);
        assert_eq!(result.conflicting_fact_keys, ["task.status"]);

        let stale = document(
            "stale.md",
            KnowledgeSourceAuthority::CanonicalMarkdown,
            None,
            false,
            vec![fragment(
                KnowledgeSourceFragmentKind::Body,
                "status old",
                1,
                None,
            )],
        );
        let stale_result =
            retrieve_knowledge(&query("status"), &[stale]).expect("retrieval succeeds");
        assert_eq!(stale_result.evidence_state, KnowledgeEvidenceState::Stale);
        assert_eq!(stale_result.hits[0].freshness, KnowledgeFreshness::Stale);
    }

    #[test]
    fn denied_absent_and_budget_exhaustion_block_without_guessing() {
        let mut denied = document(
            "denied.md",
            KnowledgeSourceAuthority::CanonicalMarkdown,
            None,
            true,
            vec![fragment(
                KnowledgeSourceFragmentKind::Body,
                "needle denied",
                1,
                None,
            )],
        );
        denied.denied = true;
        let denied_result =
            retrieve_knowledge(&query("needle"), &[denied]).expect("retrieval succeeds");
        assert_eq!(
            denied_result.evidence_state,
            KnowledgeEvidenceState::UnknownBlocked
        );
        assert_eq!(denied_result.denied_source_count, 1);

        let source = document(
            "large.md",
            KnowledgeSourceAuthority::CanonicalMarkdown,
            None,
            true,
            vec![fragment(
                KnowledgeSourceFragmentKind::Body,
                "needle context",
                1,
                None,
            )],
        );
        let mut tiny = query("needle");
        tiny.max_context_bytes = 1;
        let tiny_result = retrieve_knowledge(&tiny, &[source]).expect("retrieval succeeds");
        assert_eq!(
            tiny_result.evidence_state,
            KnowledgeEvidenceState::UnknownBlocked
        );
        assert_eq!(tiny_result.omitted_candidate_count, 1);
    }

    #[test]
    fn context_is_deduplicated_and_preserves_the_highest_ranked_citation() {
        let canonical = document(
            "canonical.md",
            KnowledgeSourceAuthority::CanonicalMarkdown,
            None,
            true,
            vec![fragment(
                KnowledgeSourceFragmentKind::Body,
                "needle same",
                1,
                None,
            )],
        );
        let derived = document(
            "derived.md",
            KnowledgeSourceAuthority::DerivedProjection,
            None,
            true,
            vec![fragment(
                KnowledgeSourceFragmentKind::Body,
                "needle same",
                1,
                None,
            )],
        );
        let result = retrieve_knowledge(&query("needle"), &[derived, canonical])
            .expect("retrieval succeeds");
        assert_eq!(result.context.len(), 1);
        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.hits[0].path, path("canonical.md"));
        assert_eq!(result.omitted_candidate_count, 1);
    }

    #[test]
    fn adversarial_inputs_are_inert_or_rejected_and_secrets_never_enter_context() {
        let literal = document(
            "literal.md",
            KnowledgeSourceAuthority::CanonicalMarkdown,
            None,
            true,
            vec![
                fragment(
                    KnowledgeSourceFragmentKind::Body,
                    "literal .* value",
                    1,
                    None,
                ),
                fragment(KnowledgeSourceFragmentKind::Body, "ordinary value", 2, None),
                fragment(
                    KnowledgeSourceFragmentKind::Body,
                    "needle password=hidden",
                    3,
                    None,
                ),
            ],
        );
        let literal_result = retrieve_knowledge(&query(".*"), std::slice::from_ref(&literal))
            .expect("literal query succeeds");
        assert_eq!(literal_result.hits.len(), 1);
        assert_eq!(literal_result.hits[0].text, "literal .* value");

        let secret_result =
            retrieve_knowledge(&query("needle"), &[literal]).expect("secret omission succeeds");
        assert!(secret_result.hits.is_empty());
        assert_eq!(
            secret_result.evidence_state,
            KnowledgeEvidenceState::UnknownBlocked
        );

        let mut oversized = query("needle");
        oversized.terms = vec!["x".repeat(257)];
        assert_eq!(
            retrieve_knowledge(&oversized, &[]),
            Err(KnowledgeRetrievalError::InvalidInput)
        );
    }

    #[test]
    fn approved_roots_exclude_unrelated_workspace_canaries() {
        let source = document(
            "approved.md",
            KnowledgeSourceAuthority::CanonicalMarkdown,
            None,
            true,
            vec![fragment(
                KnowledgeSourceFragmentKind::Body,
                "needle approved",
                1,
                None,
            )],
        );
        let canary_root =
            WorkspaceScopePath::new(workspace(), ["other"]).expect("valid canary root");
        let canary = KnowledgeSourceDocument {
            root: canary_root,
            path: WorkspacePath::new(workspace(), ["other", "canary.md"])
                .expect("valid canary path"),
            fragments: vec![fragment(
                KnowledgeSourceFragmentKind::Body,
                "needle canary",
                1,
                None,
            )],
            ..source.clone()
        };
        let result =
            retrieve_knowledge(&query("needle"), &[canary, source]).expect("retrieval succeeds");
        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.hits[0].path, path("approved.md"));
    }

    #[test]
    fn labeled_question_corpus_meets_exact_top_one_and_citation_expectations() {
        let corpus = vec![
            document(
                "handoff.md",
                KnowledgeSourceAuthority::CanonicalMarkdown,
                Some("handoff"),
                true,
                vec![fragment(
                    KnowledgeSourceFragmentKind::Field,
                    "release owner alice",
                    1,
                    None,
                )],
            ),
            document(
                "roadmap.md",
                KnowledgeSourceAuthority::CanonicalMarkdown,
                None,
                true,
                vec![fragment(
                    KnowledgeSourceFragmentKind::Task,
                    "next gate diagnostics",
                    1,
                    None,
                )],
            ),
            document(
                "evidence.md",
                KnowledgeSourceAuthority::DirectEvidence,
                None,
                true,
                vec![fragment(
                    KnowledgeSourceFragmentKind::Date,
                    "verification date 2026-08-10",
                    1,
                    None,
                )],
            ),
        ];
        let cases = [
            ("release owner", path("handoff.md")),
            ("next gate", path("roadmap.md")),
            ("verification date", path("evidence.md")),
        ];
        let mut correct = 0_u32;
        for (phrase, expected) in cases {
            let mut case_query = query("unused");
            case_query.terms.clear();
            case_query.phrases = vec![phrase.to_owned()];
            let result = retrieve_knowledge(&case_query, &corpus).expect("corpus query succeeds");
            assert_eq!(result.hits.len(), 1);
            assert_eq!(
                result.context[0].citation_sha256,
                result.hits[0].citation_sha256
            );
            if result.hits[0].path == expected {
                correct += 1;
            }
        }
        assert_eq!(correct, 3, "fixed corpus top-1 precision must remain 100%");
    }

    #[test]
    fn synthesis_and_rendering_preserve_state_citations_conflicts_and_blocked_answers() {
        let source = document(
            "answer.md",
            KnowledgeSourceAuthority::CanonicalMarkdown,
            None,
            true,
            vec![fragment(
                KnowledgeSourceFragmentKind::Body,
                "needle observed",
                1,
                None,
            )],
        );
        let result = retrieve_knowledge(&query("needle"), &[source]).expect("retrieval succeeds");
        let envelope = prepare_knowledge_synthesis(&result);
        let draft = KnowledgeAnswerDraft {
            evidence_state: KnowledgeEvidenceState::Observed,
            text: "The bounded answer is observed.".to_owned(),
            citation_sha256: vec![result.hits[0].citation_sha256.clone()],
        };
        let rendered = render_knowledge_answer(&envelope, &draft).expect("render succeeds");
        assert_eq!(rendered.evidence_state, KnowledgeEvidenceState::Observed);
        assert_eq!(rendered.state_label, "Observed");
        assert_eq!(rendered.citation_sha256, draft.citation_sha256);

        let mut drifted = draft.clone();
        drifted.evidence_state = KnowledgeEvidenceState::Conflicting;
        assert_eq!(
            render_knowledge_answer(&envelope, &drifted),
            Err(KnowledgeRetrievalError::EvidenceDrift)
        );
        drifted = draft;
        drifted.citation_sha256 = vec!["f".repeat(64)];
        assert_eq!(
            render_knowledge_answer(&envelope, &drifted),
            Err(KnowledgeRetrievalError::EvidenceDrift)
        );

        let unknown = retrieve_knowledge(&query("absent"), &[]).expect("retrieval succeeds");
        let unknown_envelope = prepare_knowledge_synthesis(&unknown);
        let unknown_rendered = render_knowledge_answer(
            &unknown_envelope,
            &KnowledgeAnswerDraft {
                evidence_state: KnowledgeEvidenceState::UnknownBlocked,
                text: String::new(),
                citation_sha256: Vec::new(),
            },
        )
        .expect("blocked answer renders");
        assert_eq!(unknown_rendered.state_label, "Unknown/Blocked");
        assert_eq!(
            unknown_rendered.text,
            "No admissible evidence is available."
        );
        assert!(unknown_rendered.citation_sha256.is_empty());
    }
}
