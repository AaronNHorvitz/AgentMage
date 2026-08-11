use std::collections::BTreeSet;

use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_version: u32,
    corpus_id: String,
    seed_algorithm: String,
    case_count: usize,
    cases_per_class: usize,
    case_classes: Vec<String>,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    schema_version: u32,
    case_id: String,
    case_class: String,
    input_mode: String,
    components: Option<Vec<String>>,
    serialized_json_hex: Option<String>,
    expected_outcome: String,
    expected_code: Option<String>,
    expected_component_index: Option<usize>,
    escape_attempt: bool,
    admitted_escape_count: u32,
}

fn decode_hex(value: &str) -> Vec<u8> {
    assert!(value.len().is_multiple_of(2), "hex input must be paired");
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).expect("hex must be ASCII");
            u8::from_str_radix(pair, 16).expect("hex must decode")
        })
        .collect()
}

#[test]
fn generated_path_corpus_fails_closed_without_authority_or_observation() {
    let corpus: Corpus =
        serde_json::from_str(include_str!("../../../fixtures/paths/v1/corpus.json"))
            .expect("published path corpus must parse");
    assert_eq!(corpus.schema_version, 1);
    assert_eq!(corpus.corpus_id, "agentmage-canonical-paths-v1");
    assert_eq!(corpus.seed_algorithm, "closed-class-counter-v1");
    assert_eq!(corpus.case_count, 640);
    assert_eq!(corpus.cases_per_class, 64);
    assert_eq!(corpus.case_classes.len(), 10);
    assert_eq!(corpus.cases.len(), corpus.case_count);

    let mut case_ids = BTreeSet::new();
    let mut case_controls = BTreeSet::new();
    let workspace_id = WorkspaceId::from_raw("workspace-corpus-0001");
    let mut rejected_escape_attempts = 0;

    for case in corpus.cases {
        assert_eq!(case.schema_version, 1);
        assert!(case_ids.insert(case.case_id));
        assert_eq!(case.admitted_escape_count, 0);
        match case.input_mode.as_str() {
            "components" => {
                assert!(case.serialized_json_hex.is_none());
                let components = case.components.expect("component case has components");
                let result = WorkspacePath::new(workspace_id.clone(), components);
                if case.expected_outcome == "accepted" {
                    let path = result.expect("valid control must be accepted");
                    assert!(case.expected_code.is_none());
                    assert!(case.expected_component_index.is_none());
                    if case.case_class == "case_collision_candidate" {
                        let spelling = path.components()[0].as_str().to_owned();
                        assert!(case_controls.insert(spelling));
                    }
                } else {
                    let error = result.expect_err("invalid path must be rejected");
                    assert_eq!(Some(error.kind().code()), case.expected_code.as_deref());
                    assert_eq!(error.component_index(), case.expected_component_index);
                    if case.escape_attempt {
                        rejected_escape_attempts += 1;
                    }
                }
            }
            "serialized_json_hex" => {
                assert!(case.components.is_none());
                let input = decode_hex(
                    case.serialized_json_hex
                        .as_deref()
                        .expect("serialized case has bytes"),
                );
                assert!(serde_json::from_slice::<WorkspacePath>(&input).is_err());
                assert_eq!(case.expected_code.as_deref(), Some("serde.invalid_utf8"));
                if case.escape_attempt {
                    rejected_escape_attempts += 1;
                }
            }
            other => panic!("unknown path corpus input mode: {other}"),
        }
    }
    assert_eq!(case_controls.len(), 64);
    assert_eq!(rejected_escape_attempts, 512);
}
