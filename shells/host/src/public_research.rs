//! Bounded public-search preparation and claim-level citation verification.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAX_QUERY_BYTES: usize = 2_048;
const MAX_DOMAINS: usize = 32;
const MAX_RESULTS: u16 = 100;
const MAX_TOTAL_BYTES: u64 = 8 * 1_048_576;
const MAX_QUOTE_WORDS: u16 = 25;

/// Closed preferred source class in descending authority order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicSourceType {
    /// Official first-party documentation or record.
    PrimaryDocumentation,
    /// Original research publication or dataset.
    OriginalResearch,
    /// Government, standards-body, court, or regulator record.
    AuthoritativeRecord,
    /// Named secondary analysis.
    SecondaryAnalysis,
}

/// Exact public-search request handed to a separately owned network provider.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicSearchRequest {
    /// Stable request identity.
    pub request_id: String,
    /// User-approved public query.
    pub query: String,
    /// Sorted optional domain allowlist.
    pub domains: Vec<String>,
    /// Maximum age in days, or zero when no recency filter was requested.
    pub recency_days: u16,
    /// Sorted allowed source classes.
    pub source_types: Vec<PublicSourceType>,
    /// Maximum returned results.
    pub max_results: u16,
    /// Maximum aggregate response bytes.
    pub max_total_bytes: u64,
}

/// Untrusted result returned by the separately owned public-search provider.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicSearchCandidate {
    /// Provider result identity.
    pub result_id: String,
    /// Exact source class.
    pub source_type: PublicSourceType,
    /// Source title.
    pub title: String,
    /// Direct public HTTPS URL.
    pub direct_url: String,
    /// Publisher identity.
    pub publisher: String,
    /// Publication timestamp in Unix milliseconds when known.
    pub published_at_epoch_ms: Option<u64>,
    /// Access timestamp in Unix milliseconds.
    pub accessed_at_epoch_ms: u64,
    /// Exact bounded excerpt bytes.
    pub excerpt: String,
    /// True only when the proposed claim is an inference rather than source text.
    pub inference: bool,
}

/// Verified claim-level citation containing no network or execution authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicCitation {
    /// Provider result identity.
    pub result_id: String,
    /// Source title.
    pub title: String,
    /// Direct public HTTPS URL.
    pub direct_url: String,
    /// Publisher identity.
    pub publisher: String,
    /// Exact source class.
    pub source_type: PublicSourceType,
    /// Publication timestamp when supplied.
    pub published_at_epoch_ms: Option<u64>,
    /// Exact access point.
    pub accessed_at_epoch_ms: u64,
    /// Digest of the bounded excerpt, not retained excerpt content.
    pub excerpt_sha256: String,
    /// Number of quoted words permitted from this excerpt.
    pub quotation_word_limit: u16,
    /// Whether downstream prose must visibly label the claim as inference.
    pub inference_label_required: bool,
    /// True only when the source satisfies the caller's recency constraint.
    pub fresh: bool,
}

/// Stable preparation or result-verification refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicResearchError {
    /// Query identity or limits are invalid.
    InvalidRequest,
    /// A returned source is outside the exact declared scope.
    SourceDenied,
    /// A result exceeds byte, count, or quotation bounds.
    LimitExceeded,
}

/// Validates one request without performing a network action.
pub fn prepare_public_search(
    request: PublicSearchRequest,
) -> Result<PublicSearchRequest, PublicResearchError> {
    if !valid_id(&request.request_id)
        || request.query.trim().is_empty()
        || request.query.len() > MAX_QUERY_BYTES
        || request.domains.len() > MAX_DOMAINS
        || !strictly_sorted(&request.domains)
        || request.domains.iter().any(|domain| !valid_domain(domain))
        || request.source_types.is_empty()
        || !strictly_sorted(&request.source_types)
        || request.max_results == 0
        || request.max_results > MAX_RESULTS
        || request.max_total_bytes == 0
        || request.max_total_bytes > MAX_TOTAL_BYTES
    {
        return Err(PublicResearchError::InvalidRequest);
    }
    Ok(request)
}

/// Verifies and authority-ranks provider results into content-minimized citations.
pub fn verify_public_results(
    request: &PublicSearchRequest,
    candidates: Vec<PublicSearchCandidate>,
    now_epoch_ms: u64,
) -> Result<Vec<PublicCitation>, PublicResearchError> {
    prepare_public_search(request.clone())?;
    if now_epoch_ms == 0 || candidates.len() > request.max_results as usize {
        return Err(PublicResearchError::LimitExceeded);
    }
    let total = candidates.iter().try_fold(0_u64, |sum, candidate| {
        sum.checked_add(candidate.excerpt.len() as u64)
    });
    if total.is_none_or(|bytes| bytes > request.max_total_bytes) {
        return Err(PublicResearchError::LimitExceeded);
    }
    let mut identities = BTreeSet::new();
    let mut citations = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        if !identities.insert(candidate.result_id.clone())
            || !valid_id(&candidate.result_id)
            || candidate.title.trim().is_empty()
            || candidate.publisher.trim().is_empty()
            || !request.source_types.contains(&candidate.source_type)
            || !allowed_url(&candidate.direct_url, &request.domains)
            || candidate.accessed_at_epoch_ms == 0
            || candidate.accessed_at_epoch_ms > now_epoch_ms
            || candidate.excerpt.is_empty()
        {
            return Err(PublicResearchError::SourceDenied);
        }
        let fresh = candidate.published_at_epoch_ms.is_some_and(|published| {
            published <= now_epoch_ms
                && (request.recency_days == 0
                    || now_epoch_ms.saturating_sub(published)
                        <= u64::from(request.recency_days) * 86_400_000)
        });
        citations.push(PublicCitation {
            result_id: candidate.result_id,
            title: candidate.title,
            direct_url: candidate.direct_url,
            publisher: candidate.publisher,
            source_type: candidate.source_type,
            published_at_epoch_ms: candidate.published_at_epoch_ms,
            accessed_at_epoch_ms: candidate.accessed_at_epoch_ms,
            excerpt_sha256: sha256(candidate.excerpt.as_bytes()),
            quotation_word_limit: MAX_QUOTE_WORDS,
            inference_label_required: candidate.inference,
            fresh,
        });
    }
    citations.sort_by(|left, right| {
        left.source_type
            .cmp(&right.source_type)
            .then(right.fresh.cmp(&left.fresh))
            .then(left.direct_url.cmp(&right.direct_url))
            .then(left.result_id.cmp(&right.result_id))
    });
    Ok(citations)
}

fn allowed_url(url: &str, domains: &[String]) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    !host.is_empty()
        && !url.contains('@')
        && !url.bytes().any(|byte| byte.is_ascii_control())
        && (domains.is_empty()
            || domains
                .iter()
                .any(|domain| host == domain || host.ends_with(&format!(".{domain}"))))
}

fn valid_domain(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && !value.starts_with('.')
        && !value.ends_with('.')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn strictly_sorted<T: Ord>(values: &[T]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> PublicSearchRequest {
        PublicSearchRequest {
            request_id: "research-1".to_owned(),
            query: "bounded evidence".to_owned(),
            domains: vec!["example.gov".to_owned()],
            recency_days: 30,
            source_types: vec![
                PublicSourceType::PrimaryDocumentation,
                PublicSourceType::OriginalResearch,
            ],
            max_results: 4,
            max_total_bytes: 4_096,
        }
    }

    fn candidate(source_type: PublicSourceType, id: &str) -> PublicSearchCandidate {
        PublicSearchCandidate {
            result_id: id.to_owned(),
            source_type,
            title: "Public record".to_owned(),
            direct_url: format!("https://docs.example.gov/{id}"),
            publisher: "Example".to_owned(),
            published_at_epoch_ms: Some(1_900_000),
            accessed_at_epoch_ms: 2_000_000,
            excerpt: "bounded source excerpt".to_owned(),
            inference: false,
        }
    }

    #[test]
    fn sprint_82_primary_sources_rank_first_and_citations_are_content_minimized() {
        let result = verify_public_results(
            &request(),
            vec![
                candidate(PublicSourceType::OriginalResearch, "result-2"),
                candidate(PublicSourceType::PrimaryDocumentation, "result-1"),
            ],
            2_100_000,
        )
        .expect("results");
        assert_eq!(result[0].result_id, "result-1");
        assert_eq!(result[0].quotation_word_limit, 25);
        assert_eq!(result[0].excerpt_sha256.len(), 64);
        assert!(!result[0].inference_label_required);
    }

    #[test]
    fn sprint_82_domain_protocol_identity_and_scope_attacks_fail_closed() {
        for url in [
            "http://docs.example.gov/x",
            "https://example.com/x",
            "https://user@docs.example.gov/x",
        ] {
            let mut value = candidate(PublicSourceType::PrimaryDocumentation, "result-1");
            value.direct_url = url.to_owned();
            assert_eq!(
                verify_public_results(&request(), vec![value], 2_100_000),
                Err(PublicResearchError::SourceDenied)
            );
        }
    }

    #[test]
    fn sprint_82_limits_duplicates_and_unrequested_sources_fail_closed() {
        let value = candidate(PublicSourceType::PrimaryDocumentation, "result-1");
        assert_eq!(
            verify_public_results(&request(), vec![value.clone(), value], 2_100_000),
            Err(PublicResearchError::SourceDenied)
        );
        let mut denied = candidate(PublicSourceType::SecondaryAnalysis, "result-2");
        denied.excerpt = "x".repeat(5_000);
        assert_eq!(
            verify_public_results(&request(), vec![denied], 2_100_000),
            Err(PublicResearchError::LimitExceeded)
        );
    }

    #[test]
    fn sprint_82_inference_and_staleness_remain_visible() {
        let mut value = candidate(PublicSourceType::PrimaryDocumentation, "result-1");
        value.inference = true;
        value.published_at_epoch_ms = Some(1);
        let result = verify_public_results(&request(), vec![value], 2_700_000_000)
            .expect("visible limitation");
        assert!(result[0].inference_label_required);
        assert!(!result[0].fresh);
    }
}
