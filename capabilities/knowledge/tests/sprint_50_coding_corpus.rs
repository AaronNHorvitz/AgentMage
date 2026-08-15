use agentmage_capability_knowledge::{
    CodingSkillAdmission, CodingSkillDefinition, CodingSkillFinding,
    assess_coding_skill_definition, built_in_coding_skill_definitions,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Corpus {
    schema_version: u16,
    corpus_id: String,
    skill_ids: Vec<String>,
    definition_mutations: Vec<Mutation>,
    prohibited_operation_attempts: Vec<String>,
    fictional_workflow_dimensions: WorkflowDimensions,
    repository_failure_scenarios: Vec<FailureScenario>,
    expanded_case_counts: CaseCounts,
}

#[derive(Deserialize)]
struct Mutation {
    mutation_id: String,
    expected_finding: String,
}

#[derive(Deserialize)]
struct WorkflowDimensions {
    languages: Vec<String>,
    workflows: Vec<String>,
    source_corpus: String,
    integrated_chat_status: String,
    integrated_cli_status: String,
}

#[derive(Deserialize)]
struct FailureScenario {
    scenario_id: String,
    expected_outcome: String,
    local_contract_test: String,
}

#[derive(Deserialize)]
struct CaseCounts {
    definition_mutations: usize,
    prohibited_operation_attempts: usize,
    fictional_workflow_combinations: usize,
    repository_failure_scenarios: usize,
    total: usize,
}

fn corpus() -> Corpus {
    serde_json::from_str(include_str!(
        "../../../docs/verification/sprint-50-coding-corpus.json"
    ))
    .expect("Sprint 50 corpus must parse")
}

fn mutate(definition: &mut CodingSkillDefinition, mutation: &str) {
    match mutation {
        "vague-completion" => definition.completion_criteria = vec!["Done".to_owned()],
        "hidden-authority" => definition.authority.writes = true,
        "unsupported-tool" => definition.advisory_tools.push("ambient-shell".to_owned()),
        "unsupported-language" => definition.supported_languages.push("fortran".to_owned()),
        "missing-validation" => {
            definition.required_validation.pop();
        }
        "overbroad-scope" => definition.maximum_paths = u16::MAX,
        other => panic!("unregistered mutation: {other}"),
    }
}

fn finding(value: &str) -> CodingSkillFinding {
    match value {
        "vague_completion" => CodingSkillFinding::VagueCompletion,
        "hidden_authority" => CodingSkillFinding::HiddenAuthority,
        "unsupported_tool" => CodingSkillFinding::UnsupportedTool,
        "unsupported_language" => CodingSkillFinding::UnsupportedLanguage,
        "missing_validation" => CodingSkillFinding::MissingValidation,
        "overbroad_scope" => CodingSkillFinding::OverbroadScope,
        other => panic!("unregistered finding: {other}"),
    }
}

#[test]
fn all_definition_mutations_disable_every_coding_skill() {
    let corpus = corpus();
    let definitions = built_in_coding_skill_definitions();
    assert_eq!(corpus.schema_version, 1);
    assert_eq!(corpus.corpus_id, "sprint-50-coding-skill-and-workflow-v1");
    assert_eq!(definitions.len(), corpus.skill_ids.len());
    for (definition, expected_skill) in definitions.into_iter().zip(&corpus.skill_ids) {
        assert_eq!(
            serde_json::to_value(definition.skill).expect("skill serialization"),
            expected_skill.as_str()
        );
        for mutation in &corpus.definition_mutations {
            let mut changed = definition.clone();
            mutate(&mut changed, &mutation.mutation_id);
            let assessment = assess_coding_skill_definition(&changed);
            assert_eq!(assessment.admission, CodingSkillAdmission::Disabled);
            assert!(
                assessment
                    .findings
                    .contains(&finding(&mutation.expected_finding)),
                "{} did not produce {}",
                mutation.mutation_id,
                mutation.expected_finding
            );
            assert!(!assessment.authority_granted);
        }
    }
}

#[test]
fn every_skill_denies_every_autonomous_operation() {
    let corpus = corpus();
    for definition in built_in_coding_skill_definitions() {
        assert_eq!(
            definition.prohibited_operations,
            corpus.prohibited_operation_attempts
        );
        assert!(!assess_coding_skill_definition(&definition).authority_granted);
    }
}

#[test]
fn corpus_dimensions_and_truthful_blockers_are_closed() {
    let corpus = corpus();
    let definition_count = corpus.skill_ids.len() * corpus.definition_mutations.len();
    let operation_count = corpus.skill_ids.len() * corpus.prohibited_operation_attempts.len();
    let workflow_count = corpus.fictional_workflow_dimensions.languages.len()
        * corpus.fictional_workflow_dimensions.workflows.len();
    let failure_count = corpus.repository_failure_scenarios.len();
    assert_eq!(
        definition_count,
        corpus.expanded_case_counts.definition_mutations
    );
    assert_eq!(
        operation_count,
        corpus.expanded_case_counts.prohibited_operation_attempts
    );
    assert_eq!(
        workflow_count,
        corpus.expanded_case_counts.fictional_workflow_combinations
    );
    assert_eq!(
        failure_count,
        corpus.expanded_case_counts.repository_failure_scenarios
    );
    assert_eq!(
        definition_count + operation_count + workflow_count + failure_count,
        corpus.expanded_case_counts.total
    );
    assert_eq!(
        corpus.fictional_workflow_dimensions.source_corpus,
        "docs/verification/sprint-45-coding-corpus.json"
    );
    assert_eq!(
        corpus.fictional_workflow_dimensions.integrated_chat_status,
        "blocked-product-coordinator-absent"
    );
    assert_eq!(
        corpus.fictional_workflow_dimensions.integrated_cli_status,
        "blocked-product-coordinator-absent"
    );
    for scenario in corpus.repository_failure_scenarios {
        assert!(!scenario.scenario_id.is_empty());
        assert!(!scenario.expected_outcome.is_empty());
        assert!(scenario.local_contract_test.contains("::tests::"));
    }
}
