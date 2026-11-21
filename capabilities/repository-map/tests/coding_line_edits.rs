// SPDX-License-Identifier: MIT OR Apache-2.0
//! Line-addressed proposals resolve original bytes before the shared strict check.

use agentmage_capability_repository_map::coding_proposal::{
    CodingProposalError as Error, CodingSource, check_coding_proposal, prepare_coding_proposal,
};
use serde_json::{Value, json};

fn source(text: &str) -> CodingSource {
    CodingSource {
        path: "sample.txt".into(),
        content: text.into(),
    }
}

fn edit(first: usize, last: usize, new: &str) -> Value {
    json!({"path":"sample.txt", "start_line":first, "end_line":last, "new":new})
}

#[test]
fn every_range_resolves_exact_utf8_bytes_with_lf_crlf_and_no_final_newline() {
    for text in [
        "é\n\tsecond\r\n😀 final",
        "é\r\n\tsecond\n😀 final\n",
        "one\rline",
    ] {
        let sources = [source(text)];
        let context = prepare_coding_proposal("Change sample", &sources, 4096).unwrap();
        let lines: Vec<_> = text.split_inclusive('\n').collect();
        for first in 1..=lines.len() {
            for last in first..=lines.len() {
                let answer = json!({"edits":[edit(first, last, "updated\r\n")]});
                let changes =
                    check_coding_proposal(&context, &sources, &answer.to_string()).unwrap();
                let expected = format!(
                    "{}updated\r\n{}",
                    lines[..first - 1].concat(),
                    lines[last..].concat()
                );
                assert_eq!(changes[0].content.as_bytes(), expected.as_bytes());
                let legacy = json!({"edits":[{"path":"sample.txt", "old":lines[first-1..last].concat(), "new":"updated\r\n"}]});
                assert_eq!(
                    serde_json::to_value(&changes).unwrap(),
                    serde_json::to_value(
                        check_coding_proposal(&context, &sources, &legacy.to_string()).unwrap()
                    )
                    .unwrap()
                );
            }
        }
        assert_eq!(sources[0].content, text);
    }
}

#[test]
fn insert_delete_and_mixed_edits_use_original_lines_in_any_order() {
    let sources = [source("first\nsecond\nthird\nlast")];
    let context = prepare_coding_proposal("Change sample", &sources, 4096).unwrap();
    let edits = vec![
        edit(1, 1, "first\ninserted\n"),
        edit(2, 2, ""),
        json!({"path":"sample.txt","old":"third","new":"replaced"}),
    ];
    for order in [edits.clone(), edits.into_iter().rev().collect()] {
        let answer = json!({"edits":order});
        let changes = check_coding_proposal(&context, &sources, &answer.to_string()).unwrap();
        assert_eq!(changes[0].content, "first\ninserted\nreplaced\nlast");
    }
    assert_eq!(sources[0].content, "first\nsecond\nthird\nlast");
}

#[test]
fn addressed_repeated_text_still_requires_a_unique_preimage_in_the_whole_file() {
    let sources = [source("same\nunique\nsame\n")];
    let context = prepare_coding_proposal("Change sample", &sources, 4096).unwrap();
    for first in [1, 3] {
        assert!(matches!(
            check_coding_proposal(
                &context,
                &sources,
                &json!({"edits":[edit(first, first, "different\n")]}).to_string()
            ),
            Err(Error::Edit)
        ));
    }
    let changes = check_coding_proposal(
        &context,
        &sources,
        &json!({"edits":[edit(1, 2, "different\nunique\n")]}).to_string(),
    )
    .unwrap();
    assert_eq!(changes[0].content, "different\nunique\nsame\n");
}

#[test]
fn refuses_zero_reversed_outside_phantom_and_stale_lines_without_effects() {
    let sources = [source("first\nsecond\n")];
    let context = prepare_coding_proposal("Change sample", &sources, 4096).unwrap();
    for (first, last) in [
        (0, 0),
        (0, 1),
        (2, 1),
        (3, 3),
        (1, 3),
        (usize::MAX, usize::MAX),
    ] {
        assert!(matches!(
            check_coding_proposal(
                &context,
                &sources,
                &json!({"edits":[edit(first,last,"updated")]}).to_string()
            ),
            Err(Error::Edit)
        ));
    }
    assert!(matches!(
        check_coding_proposal(
            &context,
            &[source("changed\nsecond\n")],
            &json!({"edits":[edit(1,1,"updated")]}).to_string()
        ),
        Err(Error::Stale)
    ));
    assert_eq!(sources[0].content, "first\nsecond\n");
}

#[test]
fn rejects_unknown_mixed_null_duplicate_positional_and_noninteger_fields() {
    let sources = [source("first\nsecond\n")];
    let context = prepare_coding_proposal("Change sample", &sources, 4096).unwrap();
    let mut bad: Vec<String> = [
        json!({"path":"sample.txt","start_line":1,"end_line":1,"old":"first\n","new":"x"}),
        json!({"path":"sample.txt","start_line":null,"end_line":1,"new":"x"}),
        json!({"path":"sample.txt","start_line":1,"new":"x"}),
        json!({"path":"sample.txt","old":null,"new":"x"}),
        json!(["sample.txt", null, 1, 1, "x"]),
    ]
    .into_iter()
    .map(|v| json!({"edits":[v]}).to_string())
    .collect();
    for value in [
        json!(-1),
        json!(1.5),
        json!("1"),
        json!(true),
        json!({}),
        json!([]),
    ] {
        let mut e = edit(1, 1, "x");
        e["start_line"] = value;
        bad.push(json!({"edits":[e]}).to_string());
    }
    bad.push(
        r#"{"edits":[{"path":"sample.txt","start_line":1,"start_line":2,"end_line":2,"new":"x"}]}"#
            .into(),
    );
    bad.push(r#"{"edits":[{"path":"sample.txt","old":null,"old":null,"new":"x"}]}"#.into());
    bad.push(r#"{"edits":[{"path":"sample.txt","start_line":18446744073709551616,"end_line":1,"new":"x"}]}"#.into());
    for answer in bad {
        assert!(
            matches!(
                check_coding_proposal(&context, &sources, &answer),
                Err(Error::Format)
            ),
            "{answer}"
        );
    }
}

#[test]
fn one_invalid_edit_refuses_the_whole_batch_including_overlap_and_no_change() {
    let sources = [source("first\nsecond\nlast\n")];
    let context = prepare_coding_proposal("Change sample", &sources, 4096).unwrap();
    let mut outside = edit(3, 3, "updated");
    outside["path"] = json!("../outside");
    for bad in [
        edit(2, 3, "overlap"),
        edit(3, 3, "last\n"),
        edit(3, 3, "nul\0"),
        outside,
        json!({"path":"sample.txt","old":"second","new":"collision"}),
    ] {
        for reverse in [false, true] {
            let mut edits = vec![edit(1, 2, "changed\n"), bad.clone()];
            if reverse {
                edits.reverse();
            }
            assert!(matches!(
                check_coding_proposal(&context, &sources, &json!({"edits":edits}).to_string()),
                Err(Error::Edit)
            ));
        }
    }
    assert_eq!(sources[0].content, "first\nsecond\nlast\n");
}

#[test]
fn unique_but_omitted_lines_and_ranges_crossing_gaps_are_refused() {
    let text = (1..=500)
        .map(|n| format!("line_{n:04}\n"))
        .collect::<String>();
    let sources = [source(&text)];
    let context = prepare_coding_proposal("line_0250", &sources, 2048).unwrap();
    assert!(
        context
            .excerpts()
            .iter()
            .any(|e| e.text.contains("line_0250"))
    );
    assert!(
        context
            .excerpts()
            .iter()
            .map(|e| e.text.len())
            .sum::<usize>()
            < text.len()
    );
    for n in 1..=500 {
        let start = (n - 1) * 10;
        if context
            .excerpts()
            .iter()
            .any(|e| e.start <= start && start + 10 <= e.end)
        {
            continue;
        }
        assert!(matches!(
            check_coding_proposal(
                &context,
                &sources,
                &json!({"edits":[edit(n,n,"updated\n")]}).to_string()
            ),
            Err(Error::Edit)
        ));
    }
    assert!(matches!(
        check_coding_proposal(
            &context,
            &sources,
            &json!({"edits":[edit(1,500,"updated\n")]}).to_string()
        ),
        Err(Error::Edit)
    ));
}

#[test]
fn line_edits_retain_batch_size_and_edit_count_ceilings() {
    let limit = 128 * 1024;
    let sources = [source(&format!("first\n{}", "p\n".repeat((limit - 6) / 2)))];
    assert_eq!(sources[0].content.len(), limit);
    let context = prepare_coding_proposal("first", &sources, 4096).unwrap();
    for (new, allowed) in [("same!\n", true), ("extra!\n", false)] {
        let result = check_coding_proposal(
            &context,
            &sources,
            &json!({"edits":[edit(1,1,new)]}).to_string(),
        );
        assert_eq!(result.is_ok(), allowed);
        if !allowed {
            assert!(matches!(result, Err(Error::Edit)));
        }
    }
    let text = (1..=33)
        .map(|n| format!("unique_{n}\n"))
        .collect::<String>();
    let sources = [source(&text)];
    let context = prepare_coding_proposal("Change sample", &sources, 4096).unwrap();
    for count in [32, 33] {
        let answer =
            json!({"edits":(1..=count).map(|n|edit(n,n,"replacement\n")).collect::<Vec<_>>()});
        let result = check_coding_proposal(&context, &sources, &answer.to_string());
        assert_eq!(result.is_ok(), count == 32);
        if count == 33 {
            assert!(matches!(result, Err(Error::Format)));
        }
    }
}

#[test]
fn insertion_beside_repeated_blank_lines_preserves_unique_context_and_line_endings() {
    for ending in ["\n", "\r\n"] {
        let text = ["first = 1", "", "target = 2", "", "last = 3", ""].join(ending);
        let sources = [source(&text)];
        let context = prepare_coding_proposal("Insert beside target", &sources, 4096).unwrap();
        let blank = json!({"edits":[edit(2, 2, &format!("inserted = 4{ending}"))]});
        assert!(matches!(
            check_coding_proposal(&context, &sources, &blank.to_string()),
            Err(Error::Edit)
        ));
        // The author selects a unique range before submission. The checker never
        // widens a repeated answer. Keep the context and its exact line endings.
        let replacement = format!("{ending}inserted = 4{ending}target = 2{ending}");
        let anchored = json!({"edits":[edit(2, 3, &replacement)]});
        let changes = check_coding_proposal(&context, &sources, &anchored.to_string()).unwrap();
        assert_eq!(
            changes[0].content,
            [
                "first = 1",
                "",
                "inserted = 4",
                "target = 2",
                "",
                "last = 3",
                ""
            ]
            .join(ending)
        );
        // Prompt guidance is not output repair: a missing final line ending
        // remains missing, even though the original range ended with one.
        let joined = json!({"edits":[edit(2, 3, replacement.trim_end_matches(['\r', '\n']))]});
        let changes = check_coding_proposal(&context, &sources, &joined.to_string()).unwrap();
        assert_eq!(
            changes[0].content,
            format!(
                "first = 1{ending}{ending}inserted = 4{ending}target = 2{ending}last = 3{ending}"
            )
        );
        assert_eq!(sources[0].content, text);
    }
}

#[test]
fn insertions_at_first_middle_and_final_lines_select_nonempty_original_anchors() {
    let original = ["one", "two", "three", "four", "five", "six", "seven"];
    for ending in ["\n", "\r\n"] {
        for final_ending in [false, true] {
            let suffix = if final_ending { ending } else { "" };
            let text = format!("{}{suffix}", original.join(ending));
            let sources = [source(&text)];
            let context = prepare_coding_proposal("Insert a line", &sources, 4096).unwrap();
            for n in [1, 4, 7] {
                for before in [true, false] {
                    let mut expected_lines = original.to_vec();
                    expected_lines.insert(if before { n - 1 } else { n }, "inserted");
                    let expected = format!("{}{suffix}", expected_lines.join(ending));
                    let anchor = original[n - 1];
                    let replacement_ending = if n == original.len() { suffix } else { ending };
                    let replacement = if before {
                        format!("inserted{ending}{anchor}{replacement_ending}")
                    } else {
                        format!("{anchor}{ending}inserted{replacement_ending}")
                    };
                    let anchored = edit(n, n, &replacement);
                    let changes = check_coding_proposal(
                        &context,
                        &sources,
                        &json!({"edits":[anchored.clone()]}).to_string(),
                    )
                    .unwrap();
                    assert_eq!(changes[0].content.as_bytes(), expected.as_bytes());
                    // A reversed insertion range is never an empty selection.
                    // One invalid edit refuses the batch, regardless of order.
                    for reverse in [false, true] {
                        let mut edits = vec![anchored.clone(), edit(n + 1, n, "inserted")];
                        if reverse {
                            edits.reverse();
                        }
                        assert!(matches!(
                            check_coding_proposal(
                                &context,
                                &sources,
                                &json!({"edits":edits}).to_string()
                            ),
                            Err(Error::Edit)
                        ));
                    }
                }
            }
            assert!(matches!(
                check_coding_proposal(
                    &context,
                    &sources,
                    &json!({"edits":[edit(8, 8, "inserted")]}).to_string()
                ),
                Err(Error::Edit)
            ));
            assert_eq!(sources[0].content.as_bytes(), text.as_bytes());
        }
    }
}
