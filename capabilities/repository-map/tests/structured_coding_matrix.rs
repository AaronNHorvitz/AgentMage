use std::fmt::Write;

use agentmage_capability_repository_map::{
    StructuredArtifactClass, StructuredEdit, StructuredEditError, StructuredEditMethod,
    StructuredFileChangeRequest, StructuredLanguage, build_structured_file_change,
    verify_structured_file_change,
};
use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_version: u16,
    corpus_id: String,
    cases: Vec<Case>,
    required_test_concerns: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    case_id: String,
    path: String,
    language: FixtureLanguage,
    artifact_class: StructuredArtifactClass,
    source: String,
    operation: Operation,
    expected: String,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum FixtureLanguage {
    Python,
    TypeScript,
    JavaScript,
    Go,
    Shell,
    Sql,
}

impl From<FixtureLanguage> for StructuredLanguage {
    fn from(value: FixtureLanguage) -> Self {
        match value {
            FixtureLanguage::Python => Self::Python,
            FixtureLanguage::TypeScript => Self::TypeScript,
            FixtureLanguage::JavaScript => Self::JavaScript,
            FixtureLanguage::Go => Self::Go,
            FixtureLanguage::Shell => Self::Shell,
            FixtureLanguage::Sql => Self::Sql,
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Operation {
    RenameIdentifier { old: String, replacement: String },
    ReplaceExactText { old: String, replacement: String },
    ReplaceModule { replacement: String },
}

fn corpus() -> Corpus {
    serde_json::from_str(include_str!(
        "../../../docs/verification/sprint-45-coding-corpus.json"
    ))
    .expect("valid closed corpus")
}

fn path(value: &str) -> WorkspacePath {
    WorkspacePath::new(
        WorkspaceId::from_raw("workspace-sprint-45-corpus"),
        value.split('/'),
    )
    .expect("fixture path")
}

fn edit(case: &Case) -> StructuredEdit {
    match &case.operation {
        Operation::RenameIdentifier { old, replacement } => StructuredEdit::RenameIdentifier {
            edit_id: format!("edit-{}", case.case_id),
            old: old.clone(),
            replacement: replacement.clone(),
        },
        Operation::ReplaceExactText { old, replacement } => StructuredEdit::ReplaceExactText {
            edit_id: format!("edit-{}", case.case_id),
            expected: old.clone(),
            replacement: replacement.clone(),
        },
        Operation::ReplaceModule { replacement } => StructuredEdit::ReplaceSyntaxNode {
            edit_id: format!("edit-{}", case.case_id),
            start_byte: 0,
            end_byte: u64::try_from(case.source.len()).expect("source length"),
            expected_node_sha256: sha256_hex(case.source.as_bytes()),
            replacement: replacement.clone(),
        },
    }
}

fn request(case: &Case) -> StructuredFileChangeRequest {
    StructuredFileChangeRequest {
        change_id: case.case_id.clone(),
        path: path(&case.path),
        intent_sha256: "1".repeat(64),
        change_plan_sha256: "2".repeat(64),
        language: case.language.into(),
        artifact_class: case.artifact_class,
        preimage: case.source.as_bytes().to_vec(),
        expected_preimage_sha256: sha256_hex(case.source.as_bytes()),
        edits: vec![edit(case)],
        additional_review_hooks: Vec::new(),
        generated: false,
        allow_generated: false,
    }
}

#[test]
fn fictional_python_typescript_javascript_go_shell_sql_and_test_goldens_are_exact() {
    let corpus = corpus();
    assert_eq!(corpus.schema_version, 1);
    assert_eq!(corpus.corpus_id, "sprint-45-fictional-multilanguage-v1");
    assert_eq!(corpus.cases.len(), 7);
    assert_eq!(
        corpus.required_test_concerns,
        [
            "boundary",
            "edge_case",
            "failure",
            "permission",
            "data_change",
            "rollback"
        ]
    );

    for case in &corpus.cases {
        let plan = build_structured_file_change(request(case)).expect("fixture plan");
        assert!(verify_structured_file_change(&plan));
        assert_eq!(
            plan.postimage(),
            case.expected.as_bytes(),
            "{}",
            case.case_id
        );
        assert!(!plan.summary().mutation_authority);
        if !matches!(&case.operation, Operation::ReplaceModule { .. }) {
            assert!(!plan.summary().unchanged_spans.is_empty());
        }
        let expected_method = match case.language {
            FixtureLanguage::Go | FixtureLanguage::Shell | FixtureLanguage::Sql => {
                StructuredEditMethod::ExactTextFallback
            }
            FixtureLanguage::Python | FixtureLanguage::TypeScript | FixtureLanguage::JavaScript => {
                StructuredEditMethod::SyntaxTree
            }
        };
        assert!(
            plan.summary()
                .changed_ranges
                .iter()
                .all(|range| range.method == expected_method)
        );
    }
}

#[test]
fn malformed_syntax_duplicate_fallback_and_stale_preimages_never_produce_a_plan() {
    let corpus = corpus();
    let python = &corpus.cases[0];
    let mut malformed = request(python);
    malformed.preimage = b"def broken(:\n".to_vec();
    malformed.expected_preimage_sha256 = sha256_hex(&malformed.preimage);
    assert_eq!(
        build_structured_file_change(malformed),
        Err(StructuredEditError::SyntaxInvalid)
    );

    let go = &corpus.cases[3];
    let mut duplicate = request(go);
    duplicate.preimage = b"package sample\nconst OldTotal = OldTotal\n".to_vec();
    duplicate.expected_preimage_sha256 = sha256_hex(&duplicate.preimage);
    assert_eq!(
        build_structured_file_change(duplicate),
        Err(StructuredEditError::EditTargetInvalid)
    );

    let mut stale = request(python);
    stale.expected_preimage_sha256 = "f".repeat(64);
    assert_eq!(
        build_structured_file_change(stale),
        Err(StructuredEditError::InvalidInput)
    );
}

#[test]
fn unsupported_source_uses_only_unique_exact_text_and_preserves_every_other_byte() {
    let source = "before exact target after\n";
    let expected = "before reviewed target after\n";
    let plan = build_structured_file_change(StructuredFileChangeRequest {
        change_id: "case-unsupported-bounded-fallback".to_owned(),
        path: path("docs/example.rb"),
        intent_sha256: "1".repeat(64),
        change_plan_sha256: "2".repeat(64),
        language: StructuredLanguage::PlainText,
        artifact_class: StructuredArtifactClass::Documentation,
        preimage: source.as_bytes().to_vec(),
        expected_preimage_sha256: sha256_hex(source.as_bytes()),
        edits: vec![StructuredEdit::ReplaceExactText {
            edit_id: "edit-bounded-fallback".to_owned(),
            expected: "exact target".to_owned(),
            replacement: "reviewed target".to_owned(),
        }],
        additional_review_hooks: Vec::new(),
        generated: false,
        allow_generated: false,
    })
    .expect("bounded fallback");
    assert_eq!(plan.postimage(), expected.as_bytes());
    assert_eq!(plan.summary().changed_ranges.len(), 1);
    assert_eq!(plan.summary().unchanged_spans.len(), 2);
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}
