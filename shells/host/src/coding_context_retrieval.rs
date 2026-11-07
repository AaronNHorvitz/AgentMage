// SPDX-License-Identifier: Apache-2.0
//! Task-based ordering within the existing nonessential repository context tier.

use std::collections::BTreeSet;

use super::*;

const MAX_TERMS: usize = 128;
const MAX_TERM_BYTES: usize = 64;

// Bounded distinct words: repetition cannot buy priority. Delimiters split paths
// and snake_case identifiers; non-ASCII bytes are delimiters, never sliced text.
fn terms(text: &str, maximum_bytes: usize) -> BTreeSet<String> {
    let mut result = BTreeSet::new();
    let mut word = Vec::new();
    let mut overlong = false;
    for byte in text.bytes().take(maximum_bytes).chain([b' ']) {
        if byte.is_ascii_alphanumeric() {
            if word.len() < MAX_TERM_BYTES {
                word.push(byte.to_ascii_lowercase());
            } else {
                overlong = true;
            }
        } else {
            if !overlong && word.len() >= 3 && word.iter().any(u8::is_ascii_alphabetic) {
                result.insert(String::from_utf8(word.clone()).expect("ASCII word"));
                if result.len() == MAX_TERMS {
                    break;
                }
            }
            word.clear();
            overlong = false;
        }
    }
    result
}

fn matches(query: &BTreeSet<String>, text: &str) -> usize {
    if query.is_empty() {
        return 0;
    }
    // Store only matching terms, so long source files cannot fill the term set
    // with unrelated words before reaching a relevant identifier.
    let mut found = BTreeSet::new();
    let mut word = Vec::new();
    let mut overlong = false;
    for byte in text.bytes().take(65_536).chain([b' ']) {
        if byte.is_ascii_alphanumeric() {
            if word.len() < MAX_TERM_BYTES {
                word.push(byte.to_ascii_lowercase());
            } else {
                overlong = true;
            }
        } else {
            if !overlong
                && let Ok(term) = std::str::from_utf8(&word)
                && query.contains(term)
            {
                found.insert(term.to_owned());
                if found.len() == query.len() {
                    break;
                }
            }
            word.clear();
            overlong = false;
        }
    }
    found.len()
}

fn body_matches(query: &BTreeSet<String>, excerpt: &str) -> usize {
    // Native sources wrap their text as JSON. Match decoded text so escaped
    // line breaks and indentation cannot become prefixes of identifiers.
    // The producer caps source bytes, not serialized JSON bytes: escaping
    // expands an otherwise admitted source. The decoded scan remains bounded.
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(excerpt)
        && value["schema_version"] == 1
        && value["untrusted_repository_source"] == true
        && let Some(content) = value["content"].as_str()
    {
        return matches(query, content);
    }
    matches(query, excerpt)
}

pub(super) fn rank_sources(
    objective: &str,
    sources: &[CodingContextSource],
    reserved: &[ContextItemCandidate],
) -> Vec<CodingContextSource> {
    let mut identities = reserved
        .iter()
        .map(|item| &item.item_id)
        .collect::<BTreeSet<_>>();
    if sources
        .iter()
        .any(|source| !identities.insert(&source.item_id))
    {
        // Leave conflicting identities for the existing composer to refuse.
        // A different score must never disguise an original duplicate.
        return sources.to_vec();
    }
    let query = terms(objective, 8_192);
    sources
        .iter()
        .cloned()
        .map(|mut source| {
            // Never promote an instruction, evidence, denied/stale source or
            // essential item. Rank affects allocation only, never authority.
            if source.kind == ContextItemKind::Supporting
                && source.admission == ContextAdmission::Eligible
                && !source.essential
                && source
                    .item_id
                    .strip_prefix("repository-source-")
                    .is_some_and(|id| {
                        id.len() == 16
                            && id
                                .bytes()
                                .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
                    })
                && let Some(path) = source.source_id.strip_prefix("agentmage:workspace-file:")
            {
                let path_matches = matches(&query, path);
                let body_matches = body_matches(&query, &source.bounded_excerpt);
                // The existing composer sorts within this tier by item_id.
                // Path overlap dominates body overlap. Keep original identity in
                // the tie-break without copying an unbounded caller label.
                source.item_id = format!(
                    "repository-source-{:03}-{:03}-{}",
                    MAX_TERMS - path_matches,
                    MAX_TERMS - body_matches,
                    sha256(source.item_id.as_bytes()),
                );
            }
            source
        })
        .collect()
}
