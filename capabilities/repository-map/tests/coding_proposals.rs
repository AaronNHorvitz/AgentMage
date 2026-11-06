// SPDX-License-Identifier: Apache-2.0
//! Snapshot-only coding contracts; no model or filesystem-effect claim.

use agentmage_capability_repository_map::coding_proposal::{
    CODING_PROPOSAL_INSTRUCTION, CodingProposalError as Error, CodingSource, check_coding_proposal,
    prepare_coding_proposal,
};
use serde_json::json;

fn source(path: &str, text: &str) -> CodingSource {
    CodingSource {
        path: path.into(),
        content: text.into(),
    }
}

#[test]
fn selects_named_definition_with_exact_ranges_and_explicit_omissions() {
    let text = format!(
        "{}\ndef clamp(value):\n    return min(value, 100)\n\n{}",
        "# unrelated source padding\n".repeat(200),
        "# trailing padding\n".repeat(200)
    );
    let sources = [source("limits.py", &text)];
    let context = prepare_coding_proposal("Fix clamp to cap value at 50", &sources, 2048).unwrap();
    assert!(context.message_bytes() <= 2048);
    assert_eq!(
        context.message_bytes(),
        context.user_message().len() + CODING_PROPOSAL_INSTRUCTION.len()
    );
    assert!(context.outlines()[0].symbols.contains(&"clamp".into()));
    assert_eq!(context.outlines()[0].bytes, text.len());
    assert!(
        context
            .excerpts()
            .iter()
            .any(|e| e.text.contains("return min(value, 100)"))
    );
    assert!(
        context
            .excerpts()
            .iter()
            .map(|e| e.text.len())
            .sum::<usize>()
            < text.len()
    );
    for excerpt in context.excerpts() {
        assert_eq!(&text[excerpt.start..excerpt.end], excerpt.text);
    }
    let changes = check_coding_proposal(
        &context,
        &sources,
        &json!({"edits":[{
            "path":"limits.py","old":"min(value, 100)","new":"min(value, 50)"
        }]})
        .to_string(),
    )
    .unwrap();
    assert_eq!(
        changes[0].content,
        text.replace("min(value, 100)", "min(value, 50)")
    );
    assert_ne!(changes[0].before_sha256, changes[0].after_sha256);
    assert_eq!(sources[0].content, text); // never changes the supplied snapshot
}

#[test]
fn deterministic_context_and_original_source_edits_across_files() {
    let sources = [source("z.txt", "alpha beta"), source("a.txt", "first last")];
    let context = prepare_coding_proposal("Update these small files", &sources, 4096).unwrap();
    let reversed = [sources[1].clone(), sources[0].clone()];
    assert_eq!(
        context.user_message(),
        prepare_coding_proposal("Update these small files", &reversed, 4096)
            .unwrap()
            .user_message()
    );
    let changes = check_coding_proposal(
        &context,
        &sources,
        &json!({"edits":[
            {"path":"z.txt","old":"alpha","new":"beta"},
            {"path":"z.txt","old":"beta","new":"gamma"},
            {"path":"a.txt","old":"first","new":"new"}
        ]})
        .to_string(),
    )
    .unwrap();
    assert_eq!(changes[0].path, "a.txt");
    assert_eq!(changes[0].content, "new last");
    assert_eq!(changes[1].content, "beta gamma"); // no sequential rematching
}

#[test]
fn refuses_sources_paths_and_budgets_without_ambient_access() {
    for path in [
        "/outside",
        "../file",
        "a/../file",
        "./file",
        "a//b",
        "a/",
        ".git/config",
        "a/.GIT/config",
        "a\\b",
        "bad\npath",
        "",
        "$(touch outside)",
    ] {
        assert!(matches!(
            prepare_coding_proposal("task", &[source(path, "bytes")], 4096),
            Err(Error::Source)
        ));
    }
    for sources in [
        vec![],
        vec![source("x", "")],
        vec![source("x", "nul\0bytes")],
        vec![source("x", "bytes"), source("x", "other")],
        vec![source("x", &"x".repeat(131073))],
        (0..17)
            .map(|n| source(&format!("file{n}"), "bytes"))
            .collect(),
        vec![
            source("a", &"x".repeat(65536)),
            source("b", &"x".repeat(65536)),
            source("c", "extra"),
        ],
    ] {
        assert!(matches!(
            prepare_coding_proposal("task", &sources, 4096),
            Err(Error::Source)
        ));
    }
    let sources = [source("x", "hello")];
    for (task, budget) in [
        (" ", 4096),
        ("bad\0task", 4096),
        ("task", 2047),
        ("task", 32769),
    ] {
        assert!(matches!(
            prepare_coding_proposal(task, &sources, budget),
            Err(Error::Context)
        ));
    }
    assert!(matches!(
        prepare_coding_proposal(&"t".repeat(4097), &sources, 4096),
        Err(Error::Context)
    ));
}

#[test]
fn counts_json_escaping_and_never_slices_unicode_or_long_lines() {
    let sources = [source(
        "x.txt",
        &format!(
            "visible\n{}\n{}",
            "é😀\"\\".repeat(3000),
            "tail\n".repeat(60)
        ),
    )];
    let context = prepare_coding_proposal("tail", &sources, 2048).unwrap();
    assert!(context.message_bytes() <= 2048);
    for excerpt in context.excerpts() {
        assert_eq!(
            &sources[0].content[excerpt.start..excerpt.end],
            excerpt.text
        );
    }
    let impossible = [source("x.txt", &"é😀\"\\".repeat(3000))];
    assert!(matches!(
        prepare_coding_proposal("task", &impossible, 2048),
        Err(Error::Context)
    ));
}

#[test]
fn refuses_malformed_duplicate_unknown_nested_and_excessive_json() {
    let sources = [source("x", "hello")];
    let context = prepare_coding_proposal("task", &sources, 4096).unwrap();
    let nested = format!("{{\"edits\":{}0{}}}", "[".repeat(2000), "]".repeat(2000));
    for answer in [
        "null",
        "[]",
        "{}",
        "{\"edits\":[]}",
        "{\"edits\":[],\"edits\":[]}",
        "{\"edits\":[[\"x\",\"hello\",\"bye\"]]}",
        "[[{\"path\":\"x\",\"old\":\"hello\",\"new\":\"bye\"}]]",
        "{\"edits\":[{\"path\":\"x\",\"old\":\"hello\",\"new\":\"a\",\"new\":\"b\"}]}",
        "{\"edits\":[{\"path\":\"x\",\"old\":\"hello\",\"new\":\"bye\",\"command\":\"run\"}]}",
        "{\"edits\":[{\"path\":\"x\",\"old\":\"hello\",\"new\":{}}]}",
        "{\"edits\":[{\"path\":\"x\",\"old\":\"hello\",\"new\":\"bye\"}]} true",
        &nested,
        &" ".repeat(65537),
        &json!({"edits": vec![json!({"path":"x","old":"hello","new":"bye"});33]}).to_string(),
    ] {
        assert!(
            matches!(
                check_coding_proposal(&context, &sources, answer),
                Err(Error::Format)
            ),
            "{answer:.100}"
        );
    }
}

#[test]
fn refuses_ambiguous_overlapping_invisible_and_out_of_scope_edits_atomically() {
    let sources = [source("x", "aaa\nhello\nworld\n")];
    let context = prepare_coding_proposal("task", &sources, 4096).unwrap();
    for edits in [
        json!([{"path":"outside","old":"hello","new":"bye"}]),
        json!([{"path":"x","old":"aa","new":"b"}]), // overlapping occurrences
        json!([{"path":"x","old":"","new":"b"}]),
        json!([{"path":"x","old":"hello","new":"hello"}]),
        json!([{"path":"x","old":"hello","new":"\u{0}"}]),
        json!([{"path":"x","old":"hello","new":"bye"},{"path":"x","old":"ello","new":"oo"}]),
        json!([{"path":"x","old":"hello","new":"bye"},{"path":"outside","old":"x","new":"y"}]),
        json!([{"path":"x","old":"hello","new":"world"},{"path":"x","old":"world","new":"hello"},
               {"path":"x","old":"aaa","new":"aaa"}]),
    ] {
        assert!(matches!(
            check_coding_proposal(&context, &sources, &json!({"edits":edits}).to_string()),
            Err(Error::Edit)
        ));
        assert_eq!(sources[0].content, "aaa\nhello\nworld\n");
    }
    let content = format!("{}\nsecret_original_preimage", "padding data\n".repeat(500));
    let hidden = [source("x", &content)];
    let limited = prepare_coding_proposal("padding", &hidden, 2048).unwrap();
    assert!(
        !limited
            .excerpts()
            .iter()
            .any(|e| e.text.contains("secret_original_preimage"))
    );
    assert!(matches!(
        check_coding_proposal(
            &limited,
            &hidden,
            &json!({"edits":[{
                "path":"x","old":"secret_original_preimage","new":"stolen"
            }]})
            .to_string()
        ),
        Err(Error::Edit)
    ));
}

#[test]
fn refuses_stale_complete_snapshot_and_net_noop() {
    let sources = [source("x", "hello world")];
    let context = prepare_coding_proposal("task", &sources, 4096).unwrap();
    let answer = json!({"edits":[{"path":"x","old":"hello","new":"bye"}]}).to_string();
    assert!(matches!(
        check_coding_proposal(&context, &[source("x", "hello world!")], &answer),
        Err(Error::Stale)
    ));
    assert!(matches!(
        check_coding_proposal(&context, &[source("different", "hello world")], &answer),
        Err(Error::Stale)
    ));
    let sources = [source("x", "abcd")];
    let context = prepare_coding_proposal("task", &sources, 4096).unwrap();
    let noop =
        json!({"edits":[{"path":"x","old":"a","new":"ab"},{"path":"x","old":"bc","new":"c"}]})
            .to_string();
    assert!(matches!(
        check_coding_proposal(&context, &sources, &noop),
        Err(Error::Edit)
    ));
}

#[test]
fn refuses_candidate_growth_before_allocating_whole_replacements() {
    let content = format!("{}\nmarker", "x\n".repeat(65000));
    let sources = [source("x", &content)];
    let context = prepare_coding_proposal("marker", &sources, 2048).unwrap();
    let answer = json!({"edits":[{"path":"x","old":"marker","new":"z".repeat(2000)}]}).to_string();
    assert!(matches!(
        check_coding_proposal(&context, &sources, &answer),
        Err(Error::Edit)
    ));
}
