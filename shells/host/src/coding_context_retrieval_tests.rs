// SPDX-License-Identifier: Apache-2.0

use super::tests::{FixtureCounter, request};
use super::*;
use crate::coding_session::{CodingSessionProfile, tests::input};
use agentmage_kernel_engine::runtime_coordinator::seal_runtime_run_request;

fn ranked_sources(objective: &str, sources: &[CodingContextSource]) -> Vec<CodingContextSource> {
    retrieval::rank_sources(objective, sources, &[])
}

fn retrieval_source(label: &str, path: &str, text: &str) -> CodingContextSource {
    CodingContextSource::new(
        format!("repository-source-{}", &sha256(label.as_bytes())[..16]),
        ContextItemKind::Supporting,
        ContextSensitivity::Private,
        ContextAdmission::Eligible,
        false,
        format!("agentmage:workspace-file:{path}"),
        "frozen-revision",
        sha256(text.as_bytes()),
        text,
    )
    .unwrap()
}

#[test]
fn retrieval_prioritizes_task_paths_then_distinct_body_matches_deterministically() {
    let sources = vec![
        retrieval_source("first", "src/unrelated.py", &"refund refund ".repeat(100)),
        retrieval_source("second", "src/helper.py", "refund invoice"),
        retrieval_source("last", "src/refund.py", "pass"),
    ];
    let ranked = ranked_sources("Fix REFUND for invoice", &sources);
    let mut sorted = ranked.clone();
    sorted.sort_by(|a, b| a.item_id.cmp(&b.item_id));
    assert_eq!(sorted[0].source_id, sources[2].source_id);
    assert_eq!(sorted[1].source_id, sources[1].source_id);
    assert_eq!(sorted[2].source_id, sources[0].source_id);
    let mut reversed = sources.clone();
    reversed.reverse();
    let mut reversed = ranked_sources("Fix REFUND for invoice", &reversed);
    reversed.sort_by(|a, b| a.item_id.cmp(&b.item_id));
    assert_eq!(sorted, reversed);
    // A rank changes allocation identity only. Complete provenance and authority
    // classification remain byte-for-byte the original supplied values.
    for (mut selected, original) in ranked.into_iter().zip(&sources) {
        assert_ne!(selected.item_id, original.item_id);
        selected.item_id.clone_from(&original.item_id);
        assert_eq!(&selected, original);
    }
}

#[test]
fn retrieval_cannot_promote_authority_denied_stale_or_nonrepository_sources() {
    let source = retrieval_source("original", "src/refund.py", "refund");
    let mut excluded = Vec::new();
    for admission in [ContextAdmission::Denied, ContextAdmission::Stale] {
        let mut candidate = source.clone();
        candidate.admission = admission;
        excluded.push(candidate);
    }
    for kind in [ContextItemKind::Evidence, ContextItemKind::Correction] {
        let mut candidate = source.clone();
        candidate.kind = kind;
        excluded.push(candidate);
    }
    let mut essential = source.clone();
    essential.essential = true;
    excluded.push(essential);
    let mut other = source;
    other.source_id = "session:refund".into();
    excluded.push(other);
    for (index, source) in excluded.iter_mut().enumerate() {
        source.item_id = format!(
            "repository-source-{}",
            &sha256(index.to_string().as_bytes())[..16]
        );
    }
    let mut original_label = retrieval_source("label", "src/refund.py", "refund");
    original_label.item_id = "coding-system-1".into();
    excluded.push(original_label);
    assert_eq!(ranked_sources("refund", &excluded), excluded);
}

#[test]
fn retrieval_does_not_hide_duplicate_original_candidate_identities() {
    let first = retrieval_source("same", "src/refund.py", "refund");
    let mut second = retrieval_source("same", "src/other.py", "unrelated");
    second.item_id.clone_from(&first.item_id);
    let sources = vec![first, second];
    assert_eq!(ranked_sources("refund", &sources), sources);
    let profile = CodingSessionProfile::build(input()).unwrap();
    let mut owner =
        CodingContextPort::for_profile(&profile, vec![], FixtureCounter("fixture-counter-v1"))
            .unwrap();
    let reserved = owner.candidate(&sources[0]).unwrap();
    assert_eq!(
        retrieval::rank_sources("refund", &sources[..1], &[reserved]),
        sources[..1]
    );
    let mut context =
        CodingContextPort::for_profile(&profile, sources, FixtureCounter("fixture-counter-v1"))
            .unwrap();
    assert_eq!(
        context.build_context(
            &request(&profile),
            ContextPacketId::from_raw("duplicate"),
            1,
            &[],
            &[],
            &[]
        ),
        Err(RuntimePortFailure::ResourceExhausted)
    );
}

#[test]
fn retrieval_bounds_query_and_scans_late_source_matches_without_word_flooding() {
    let mut content = (0..300).map(|n| format!("unused{n} ")).collect::<String>();
    content.push_str("refund");
    let sources = [
        retrieval_source("one", "src/one.py", &content),
        retrieval_source("two", "src/two.py", "no relevant word"),
    ];
    let ranked = ranked_sources("refund", &sources);
    assert!(ranked[0].item_id < ranked[1].item_id);
    let query = format!("{} refund", " ".repeat(8192));
    let bounded = ranked_sources(&query, &sources);
    let empty = ranked_sources("12345 中文 😀", &sources);
    assert_eq!(bounded, empty);
    let overlong = ranked_sources(&format!("{}refund", "a".repeat(65)), &sources);
    assert_eq!(overlong, empty);
    let many = (0..128).map(|n| format!("query{n} ")).collect::<String>();
    assert_eq!(ranked_sources(&(many + "refund"), &sources), empty);
}

#[test]
fn retrieval_changes_actual_bounded_packets_and_preserves_all_omission_identities() {
    let profile = CodingSessionProfile::build(input()).unwrap();
    let mut request = request(&profile);
    request.task.objective = "Repair refund".into();
    request = seal_runtime_run_request(request).unwrap();
    // Each source fits alone, but all five overflow the existing packet budget.
    let mut sources = (0..4)
        .map(|i| {
            retrieval_source(
                &format!("noise-{i}"),
                &format!("src/noise{i}.py"),
                &format!("{i} {}", "noise ".repeat(8000)),
            )
        })
        .collect::<Vec<_>>();
    sources.push(retrieval_source(
        "target",
        "src/refund.py",
        &"refund ".repeat(6800),
    ));
    sources[4].item_id = "repository-source-ffffffffffffffff".into();
    let originals = sources.clone();
    let started = std::time::Instant::now();
    let mut context =
        CodingContextPort::for_profile(&profile, sources, FixtureCounter("fixture-counter-v1"))
            .unwrap();
    let packet = context
        .build_context(
            &request,
            ContextPacketId::from_raw("ranked-packet"),
            1,
            &[],
            &[],
            &[],
        )
        .unwrap();
    assert!(
        packet
            .messages
            .iter()
            .any(|m| m.content.bytes == originals[4].bounded_excerpt.as_bytes())
    );
    assert_eq!(context.supporting_sources, originals);
    assert_eq!(packet.messages[0].role, ModelMessageRole::System);
    let accounting: serde_json::Value =
        serde_json::from_slice(&packet.messages.last().unwrap().content.bytes).unwrap();
    let omitted = accounting["omitted_sources"].as_array().unwrap();
    assert!(!omitted.is_empty());
    for source in &originals {
        let included = packet
            .messages
            .iter()
            .any(|m| m.content.bytes == source.bounded_excerpt.as_bytes());
        let missing = omitted
            .iter()
            .filter(|v| {
                v["source_id"] == source.source_id && v["content_sha256"] == source.content_sha256
            })
            .count();
        assert_eq!(missing, usize::from(!included));
    }
    let mut reversed = originals.clone();
    reversed.reverse();
    let mut second =
        CodingContextPort::for_profile(&profile, reversed, FixtureCounter("fixture-counter-v1"))
            .unwrap();
    assert_eq!(
        packet,
        second
            .build_context(
                &request,
                ContextPacketId::from_raw("ranked-packet"),
                1,
                &[],
                &[],
                &[]
            )
            .unwrap()
    );
    assert!(packet.input_bytes <= request.context_budget.max_input_bytes);
    assert!(
        packet.input_tokens
            <= request.context_budget.max_context_tokens
                - request.model_profile.decoding.max_output_tokens
    );
    // Same source bytes and budget with the original identity ordering. The
    // alternate source namespace disables only this new repository heuristic.
    let mut control = originals.clone();
    for source in &mut control {
        source.source_id =
            source
                .source_id
                .replacen("agentmage:workspace-file:", "fixture:unranked:", 1);
    }
    let mut control =
        CodingContextPort::for_profile(&profile, control, FixtureCounter("fixture-counter-v1"))
            .unwrap();
    let control = control
        .build_context(
            &request,
            ContextPacketId::from_raw("unranked-control"),
            1,
            &[],
            &[],
            &[],
        )
        .unwrap();
    assert!(
        !control
            .messages
            .iter()
            .any(|m| m.content.bytes == originals[4].bounded_excerpt.as_bytes())
    );
    eprintln!(
        "retrieval_fixture sources={} supplied_bytes={} included_sources={} omitted_sources={} packet_bytes={} fixture_tokens={} elapsed_us={}",
        originals.len(),
        originals
            .iter()
            .map(|s| s.bounded_excerpt.len())
            .sum::<usize>(),
        originals.len() - omitted.len(),
        omitted.len(),
        packet.input_bytes,
        packet.input_tokens,
        started.elapsed().as_micros()
    );
}

#[test]
fn retrieval_matches_decoded_production_source_lines_and_indentation() {
    for content in [
        "x = 1\nrefund = 2\n",
        "package c\nfunc F() {\n\trefund := 1\n}\n",
        "\"refund\"",
        "\\refund",
        "\rrefund",
    ] {
        let wrapped = |content: &str| {
            serde_json::to_string(&serde_json::json!({
                "schema_version":1,"untrusted_repository_source":true,
                "path":"src/helper.py","content_sha256":sha256(content.as_bytes()),"content":content
            }))
            .unwrap()
        };
        let relevant = retrieval_source("relevant", "src/helper.py", &wrapped(content));
        let unrelated = retrieval_source("unrelated", "src/other.py", &wrapped("nothing"));
        let ranked = ranked_sources("refund", &[relevant.clone(), unrelated]);
        assert!(ranked[0].item_id < ranked[1].item_id, "{content:?}");
        assert!(ranked[0].item_id.starts_with("repository-source-128-127-"));
        assert_eq!(ranked[0].bounded_excerpt, relevant.bounded_excerpt);
    }
    // JSON escapes are decoded as characters, including a Unicode ASCII escape.
    let escaped = retrieval_source(
        "escaped",
        "src/helper.py",
        r#"{"schema_version":1,"untrusted_repository_source":true,"content":"\u0072efund"}"#,
    );
    assert!(
        ranked_sources("refund", &[escaped])[0]
            .item_id
            .starts_with("repository-source-128-127-")
    );
}
