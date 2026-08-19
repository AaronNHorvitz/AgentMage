use std::collections::BTreeMap;

use agentmage_kernel_engine::command_runner::{
    CommandBounds, CommandRisk, CommandSpec, CommandWorkingDirectory,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_version: u16,
    corpus_id: String,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    case_id: String,
    executable: String,
    arguments: Vec<String>,
    environment: BTreeMap<String, String>,
    accepted: bool,
}

#[test]
fn retained_command_injection_corpus_fails_closed() {
    let corpus: Corpus = serde_json::from_str(include_str!(
        "../../../docs/verification/sprint-41-command-injection-corpus.json"
    ))
    .expect("closed command corpus");
    assert_eq!(corpus.schema_version, 1);
    assert_eq!(corpus.corpus_id, "sprint-41-command-injection-v1");
    // Exact counts: silent growth or category loss must fail the corpus, not pass it.
    assert_eq!(corpus.cases.len(), 44);
    assert_eq!(corpus.cases.iter().filter(|case| case.accepted).count(), 1);
    let identifiers = corpus
        .cases
        .iter()
        .map(|case| case.case_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(identifiers.len(), corpus.cases.len());
    for case in corpus.cases {
        let result = CommandSpec::seal(
            format!("fixture.{}", case.case_id),
            "1.0.0",
            case.executable,
            "1".repeat(64),
            case.arguments,
            CommandWorkingDirectory::EmptyScratch,
            case.environment,
            CommandRisk::Low,
            CommandBounds::new(1_000, 1_024, 1_024, 32 * 1024 * 1024, 4, 100)
                .expect("fixed bounds"),
        );
        assert_eq!(result.is_ok(), case.accepted, "{}", case.case_id);
    }
}
