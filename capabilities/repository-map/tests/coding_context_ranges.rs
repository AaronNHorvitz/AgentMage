// SPDX-License-Identifier: MIT OR Apache-2.0
//! Source visibility and exact-edit contracts, without model or tool execution.

use agentmage_capability_repository_map::coding_proposal::{
    CodingProposalError, CodingSource, check_coding_proposal, prepare_coding_proposal,
};
use serde_json::json;

#[test]
fn a_complete_file_can_replace_previously_selected_definitions_within_budget() {
    let text = "import sys\n\ndef alpha():\n    return 1\n\ndef beta():\n    return 2\n";
    let sources = [CodingSource {
        path: "sample.py".into(),
        content: text.into(),
    }];
    let context = prepare_coding_proposal("Update alpha and beta", &sources, 4096).unwrap();
    assert_eq!(context.excerpts().len(), 1);
    let excerpt = &context.excerpts()[0];
    assert_eq!((excerpt.start, excerpt.end), (0, text.len()));
    assert_eq!(excerpt.text, text);
    let changes = check_coding_proposal(
        &context,
        &sources,
        &json!({"edits":[{"path":"sample.py",
            "old":"    return 1\n\ndef beta():\n    return 2\n",
            "new":"    return 3\n\ndef beta():\n    return 4\n"}]})
        .to_string(),
    )
    .unwrap();
    assert_eq!(
        changes[0].content,
        text.replace("return 1", "return 3")
            .replace("return 2", "return 4")
    );
    assert_eq!(sources[0].content, text);
}

#[test]
fn parser_selection_keeps_indentation_and_final_newline_under_a_tight_budget() {
    let definition = "    def chosen(self):\r\n        return 'é😀'\r\n";
    let text = format!(
        "class Example:\r\n{}{}{}",
        "    # padding\r\n".repeat(300),
        definition,
        "    # tail\r\n".repeat(300)
    );
    let sources = [CodingSource {
        path: "sample.py".into(),
        content: text.clone(),
    }];
    let context = prepare_coding_proposal("Change chosen", &sources, 2048).unwrap();
    assert!(context.message_bytes() <= 2048);
    assert!(
        context
            .excerpts()
            .iter()
            .any(|e| e.text.contains(definition))
    );
    assert!(
        context
            .excerpts()
            .iter()
            .map(|e| e.text.len())
            .sum::<usize>()
            < text.len()
    );
    for e in context.excerpts() {
        assert_eq!(&text[e.start..e.end], e.text);
        assert!(e.start == 0 || text.as_bytes()[e.start - 1] == b'\n');
        assert!(e.end == text.len() || text.as_bytes()[e.end - 1] == b'\n');
    }
    let answer = |old: &str| {
        json!({"edits":[{"path":"sample.py", "old":old,
        "new":"    def chosen(self):\r\n        return 'updated'\r\n"}]})
        .to_string()
    };
    let changes = check_coding_proposal(&context, &sources, &answer(definition)).unwrap();
    assert_eq!(
        changes[0].content,
        text.replace(
            definition,
            "    def chosen(self):\r\n        return 'updated'\r\n"
        )
    );
    // Expanded visibility does not authorize whitespace repair or fuzzy matching.
    assert!(matches!(
        check_coding_proposal(
            &context,
            &sources,
            &answer(&definition.replace("\r\n", "\n"))
        ),
        Err(CodingProposalError::Edit)
    ));
}

#[test]
fn merged_ranges_stay_canonical_exact_and_budgeted_with_many_candidates() {
    let text = (0..120)
        .map(|i| format!("def target_{i}():\n    return 'é\\\"😀 {i}'\n\n"))
        .collect::<String>();
    let sources = [CodingSource {
        path: "sample.py".into(),
        content: text.clone(),
    }];
    let context = prepare_coding_proposal("Change target return", &sources, 2048).unwrap();
    let repeat = prepare_coding_proposal("Change target return", &sources, 2048).unwrap();
    assert_eq!(context.user_message(), repeat.user_message());
    assert!(context.message_bytes() <= 2048);
    assert!(context.excerpts().len() <= 64);
    assert!(
        context
            .excerpts()
            .iter()
            .map(|e| e.text.len())
            .sum::<usize>()
            < text.len()
    );
    for e in context.excerpts() {
        assert_eq!(&text[e.start..e.end], e.text);
    }
    for pair in context.excerpts().windows(2) {
        assert!(pair[0].end < pair[1].start);
    }
    let omitted = "def target_119():\n    return 'é\\\"😀 119'\n";
    assert!(!context.excerpts().iter().any(|e| e.text.contains(omitted)));
    assert!(matches!(
        check_coding_proposal(
            &context,
            &sources,
            &json!({"edits":[{"path":"sample.py","old":omitted,"new":"not visible"}]}).to_string()
        ),
        Err(CodingProposalError::Edit)
    ));
}
