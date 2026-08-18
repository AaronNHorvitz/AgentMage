use agentmage_capability_repository_map::{
    ChangeAlternativeDimension, ChangeAlternativeOption, ChangeAlternativeSet, ChangeClarification,
    ChangeHypothesis, ChangeImpactState, ChangeImpactSurfaceKind, ChangeIntentError,
    ChangeIntentInput, ChangeIntentRecord, ChangeIntentStatus, ChangePlanError, ChangePlanInput,
    ChangePlanRecord, ChangePlanStatus, ChangeReviewKind, ChangeRiskDomain, ChangeSurfaceExclusion,
    ChangeUnknownDimension, ChangeWorkKind, DeepRepositoryIndex, GitTrackedState, HypothesisCheck,
    HypothesisCheckResult, HypothesisRecord, HypothesisStatus, MinimalChangeImpactReport,
    PlannedValidation, PlannedValidationKind, RegressionTestDisposition, RegressionTestPlan,
    RepositoryFactKind, RepositoryFileInput, RepositoryMapInput, ReproductionExecutionStatus,
    ReproductionInput, ReproductionRecord, ReproductionStep, ReproductionStepKind,
    build_change_plan, build_deep_repository_index, build_minimal_change_impact,
    build_repository_map, normalize_change_intent, seal_hypotheses, seal_reproduction,
    verify_change_plan,
};
use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};
use serde::Deserialize;
use sha2::{Digest, Sha256};

const CORPUS: &str = include_str!("../../../docs/verification/sprint-44-planning-corpus.json");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_version: u16,
    record_type: String,
    description: String,
    instruction_cases: Vec<InstructionCase>,
    golden: Golden,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InstructionCase {
    id: String,
    path: Vec<String>,
    source: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Golden {
    proposed_paths: Vec<String>,
    evidence_backed_surfaces: Vec<String>,
    required_reviews: Vec<String>,
    plan_status: String,
    rejected_instruction_fact_count: usize,
    rejected_instruction_citation_count: usize,
    mutation_authority: bool,
}

fn hash(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}

fn source(path: Vec<String>, content: &str) -> RepositoryFileInput {
    RepositoryFileInput {
        path,
        size_bytes: content.len() as u64,
        content_sha256: hash(content.as_bytes()),
        content: Some(content.as_bytes().to_vec()),
        object_kind: agentmage_capability_repository_map::RepositoryObjectKind::RegularFile,
        git_state: GitTrackedState::TrackedClean,
        policy_excluded: false,
        generated: false,
        vendored: false,
    }
}

struct Fixture {
    corpus: Corpus,
    index: DeepRepositoryIndex,
    intent: ChangeIntentRecord,
    impact: MinimalChangeImpactReport,
    test_fact_id: String,
}

fn fixture() -> Fixture {
    let corpus: Corpus = serde_json::from_str(CORPUS).expect("corpus parses");
    assert_eq!(corpus.schema_version, 1);
    assert_eq!(corpus.record_type, "sprint_44_planning_corpus");
    assert!(!corpus.description.is_empty());
    assert!(
        corpus
            .instruction_cases
            .iter()
            .all(|case| !case.id.is_empty())
    );

    let mut files = vec![
        source(
            vec!["src".to_owned(), "lib.rs".to_owned()],
            "pub fn calculate() -> u64 { 1 }\n",
        ),
        source(
            vec!["tests".to_owned(), "calculate_test.rs".to_owned()],
            "fn calculation_fails() {}\n",
        ),
    ];
    files.extend(
        corpus
            .instruction_cases
            .iter()
            .map(|case| source(case.path.clone(), &case.source)),
    );
    let map = build_repository_map(RepositoryMapInput {
        workspace_id: WorkspaceId::from_raw("workspace-sprint-44-corpus"),
        repository_sha256: "a".repeat(64),
        worktree_sha256: "b".repeat(64),
        branch: Some("agentmage/tasks/sprint-44".to_owned()),
        commit_id: "c".repeat(40),
        policy_sha256: "d".repeat(64),
        freshness_sha256: "e".repeat(64),
        files,
    })
    .expect("map");
    let index = build_deep_repository_index(&map, Vec::new(), Vec::new()).expect("index");
    let definition = index
        .facts
        .iter()
        .find(|fact| fact.label == "calculate")
        .expect("definition")
        .fact_id
        .clone();
    let test_fact_id = index
        .facts
        .iter()
        .find(|fact| fact.kind == RepositoryFactKind::Test)
        .expect("test")
        .fact_id
        .clone();
    let mut target_fact_ids = vec![definition, test_fact_id.clone()];
    target_fact_ids.sort();
    let mut surface_exclusions = ChangeImpactSurfaceKind::ALL
        .into_iter()
        .filter(|surface| {
            !matches!(
                surface,
                ChangeImpactSurfaceKind::Files | ChangeImpactSurfaceKind::Tests
            )
        })
        .map(|surface| ChangeSurfaceExclusion {
            surface,
            rationale: "The fixed corpus establishes no impact on this surface".to_owned(),
        })
        .collect::<Vec<_>>();
    surface_exclusions.sort_by_key(|item| item.surface);
    let intent = normalize_change_intent(
        &index,
        ChangeIntentInput {
            intent_id: "intent-sprint-44".to_owned(),
            requested_behavior: "Return the expected fixture value".to_owned(),
            current_behavior: "The cited function returns the failing fixture value".to_owned(),
            current_behavior_fact_ids: target_fact_ids.clone(),
            target_fact_ids,
            users: vec!["library callers".to_owned()],
            acceptance_checks: vec!["The exact regression fixture passes".to_owned()],
            exclusions: vec!["No public interface change".to_owned()],
            risks: vec![ChangeRiskDomain::Reliability],
            rollback: "Restore both exact preimages".to_owned(),
            rollback_reversible: true,
            surface_exclusions,
            clarifications: Vec::new(),
        },
    )
    .expect("intent");
    let impact = build_minimal_change_impact(&index, &intent).expect("impact");
    Fixture {
        corpus,
        index,
        intent,
        impact,
        test_fact_id,
    }
}

fn reproduction(index: &DeepRepositoryIndex) -> ReproductionRecord {
    seal_reproduction(
        index,
        ReproductionInput {
            reproduction_id: "reproduction-sprint-44".to_owned(),
            environment_sha256: "1".repeat(64),
            inputs_sha256: "2".repeat(64),
            steps: vec![ReproductionStep {
                sequence: 1,
                kind: ReproductionStepKind::RunRegisteredTest,
                description: "Run the exact registered regression fixture".to_owned(),
                evidence_fact_ids: Vec::new(),
                separate_grant_required: true,
                write_authority: false,
            }],
            expected_result_sha256: "3".repeat(64),
            observed_result_sha256: Some("4".repeat(64)),
            log_sha256s: vec!["5".repeat(64)],
            failure_signature_observed: true,
            execution_status: ReproductionExecutionStatus::Completed,
            unsafe_reason: None,
        },
    )
    .expect("reproduction")
}

fn hypotheses(index: &DeepRepositoryIndex, reproduction: &ReproductionRecord) -> HypothesisRecord {
    seal_hypotheses(
        index,
        Some(reproduction),
        HypothesisRecord {
            hypotheses: vec![
                ChangeHypothesis {
                    hypothesis_id: "hypothesis-local".to_owned(),
                    explanation: "The cited calculation selects the old value".to_owned(),
                    checks: vec![HypothesisCheck {
                        check_id: "check-local".to_owned(),
                        description: "Correlate the exact result and retained log".to_owned(),
                        evidence_fact_ids: Vec::new(),
                        log_sha256s: vec!["5".repeat(64)],
                        result: HypothesisCheckResult::Passed,
                    }],
                    status: HypothesisStatus::Supported,
                    rejection_reason: None,
                },
                ChangeHypothesis {
                    hypothesis_id: "hypothesis-platform".to_owned(),
                    explanation: "The pinned environment changes the result".to_owned(),
                    checks: vec![HypothesisCheck {
                        check_id: "check-platform".to_owned(),
                        description: "Compare the pinned environment identity".to_owned(),
                        evidence_fact_ids: Vec::new(),
                        log_sha256s: Vec::new(),
                        result: HypothesisCheckResult::Failed,
                    }],
                    status: HypothesisStatus::Rejected,
                    rejection_reason: Some("The same pinned environment reproduces".to_owned()),
                },
            ],
            selected_hypothesis_id: Some("hypothesis-local".to_owned()),
            state_trace_fact_ids: Vec::new(),
            correlated_log_sha256s: vec!["5".repeat(64)],
            hypothesis_sha256: String::new(),
        },
    )
    .expect("hypotheses")
}

fn alternatives() -> Vec<ChangeAlternativeSet> {
    ChangeAlternativeDimension::ALL
        .into_iter()
        .map(|dimension| ChangeAlternativeSet {
            dimension,
            options: vec![
                ChangeAlternativeOption {
                    option_id: "option-minimal".to_owned(),
                    description: "Use only the two exact cited files".to_owned(),
                    tradeoffs: vec!["Keeps the evidence boundary small".to_owned()],
                    evidence_fact_ids: Vec::new(),
                    irreversible: false,
                },
                ChangeAlternativeOption {
                    option_id: "option-wide".to_owned(),
                    description: "Restructure adjacent uncited code".to_owned(),
                    tradeoffs: vec!["Expands beyond established evidence".to_owned()],
                    evidence_fact_ids: Vec::new(),
                    irreversible: dimension == ChangeAlternativeDimension::Irreversibility,
                },
            ],
            selected_option_id: "option-minimal".to_owned(),
            decision_id: format!("decision-{dimension:?}").to_ascii_lowercase(),
            rationale: "The selected option is the smallest evidence-supported change".to_owned(),
            alternative_sha256: String::new(),
        })
        .collect()
}

fn plan(fixture: &Fixture) -> ChangePlanRecord {
    let reproduction = reproduction(&fixture.index);
    build_change_plan(
        &fixture.index,
        &fixture.intent,
        &fixture.impact,
        ChangePlanInput {
            plan_id: "plan-sprint-44".to_owned(),
            work_kind: ChangeWorkKind::Defect,
            reproduction: Some(reproduction.clone()),
            regression_test: RegressionTestPlan {
                disposition: RegressionTestDisposition::FailingTestRetained,
                test_fact_ids: vec![fixture.test_fact_id.clone()],
                rationale: None,
                regression_sha256: String::new(),
            },
            hypotheses: hypotheses(&fixture.index, &reproduction),
            alternatives: alternatives(),
            validations: vec![PlannedValidation {
                validation_id: "validation-regression".to_owned(),
                kind: PlannedValidationKind::Unit,
                description: "Run the exact retained regression fixture".to_owned(),
                acceptance_checks: fixture.intent.input.acceptance_checks.clone(),
                separate_grant_required: true,
                write_authority: false,
            }],
        },
    )
    .expect("plan")
}

fn path_text(path: &WorkspacePath) -> String {
    path.components()
        .iter()
        .map(|component| component.as_str())
        .collect::<Vec<_>>()
        .join("/")
}

fn surface_text(surface: ChangeImpactSurfaceKind) -> &'static str {
    match surface {
        ChangeImpactSurfaceKind::Files => "files",
        ChangeImpactSurfaceKind::Callers => "callers",
        ChangeImpactSurfaceKind::Data => "data",
        ChangeImpactSurfaceKind::Permissions => "permissions",
        ChangeImpactSurfaceKind::Tests => "tests",
        ChangeImpactSurfaceKind::Documentation => "documentation",
        ChangeImpactSurfaceKind::Configuration => "configuration",
        ChangeImpactSurfaceKind::Migrations => "migrations",
        ChangeImpactSurfaceKind::Interfaces => "interfaces",
        ChangeImpactSurfaceKind::Dependencies => "dependencies",
    }
}

#[test]
fn fictional_corpus_plan_matches_the_minimal_scope_golden() {
    let fixture = fixture();
    let plan = plan(&fixture);
    let golden = &fixture.corpus.golden;
    assert!(verify_change_plan(
        &fixture.index,
        &fixture.intent,
        &fixture.impact,
        &plan
    ));
    assert_eq!(
        fixture
            .impact
            .proposed_paths
            .iter()
            .map(path_text)
            .collect::<Vec<_>>(),
        golden.proposed_paths
    );
    assert_eq!(
        fixture
            .impact
            .surfaces
            .iter()
            .filter(|surface| surface.state == ChangeImpactState::EvidenceBacked)
            .map(|surface| surface_text(surface.kind).to_owned())
            .collect::<Vec<_>>(),
        golden.evidence_backed_surfaces
    );
    assert_eq!(
        plan.required_reviews
            .iter()
            .map(|review| match review {
                ChangeReviewKind::Security => "security",
                ChangeReviewKind::Privacy => "privacy",
                ChangeReviewKind::Data => "data",
                ChangeReviewKind::Accessibility => "accessibility",
                ChangeReviewKind::Performance => "performance",
                ChangeReviewKind::Migration => "migration",
                ChangeReviewKind::Rollback => "rollback",
            })
            .collect::<Vec<_>>(),
        golden.required_reviews
    );
    assert_eq!(plan.status, ChangePlanStatus::ReadyForImplementationReview);
    assert_eq!(golden.plan_status, "ready_for_implementation_review");
    assert_eq!(plan.mutation_authority, golden.mutation_authority);
}

#[test]
fn hostile_repository_instructions_are_rejected_without_content_disclosure() {
    let fixture = fixture();
    assert_eq!(
        fixture
            .intent
            .rejected_repository_instruction_fact_ids
            .len(),
        fixture.corpus.golden.rejected_instruction_fact_count
    );
    assert_eq!(
        fixture
            .intent
            .rejected_repository_instruction_citation_ids
            .len(),
        fixture.corpus.golden.rejected_instruction_citation_count
    );
    let encoded = serde_json::to_string(&(&fixture.intent, &fixture.impact, plan(&fixture)))
        .expect("records serialize");
    assert!(!encoded.contains("CANARY_"));
    for case in &fixture.corpus.instruction_cases {
        assert!(!encoded.contains(&case.source));
    }

    let instruction = fixture
        .index
        .facts
        .iter()
        .find(|fact| fact.kind == RepositoryFactKind::Instruction)
        .expect("instruction fact")
        .fact_id
        .clone();
    let mut broadened = fixture.intent.input.clone();
    broadened.target_fact_ids.push(instruction);
    broadened.target_fact_ids.sort();
    assert_eq!(
        normalize_change_intent(&fixture.index, broadened),
        Err(ChangeIntentError::EvidenceInvalid)
    );
}

#[test]
fn valid_ambiguous_contradictory_overbroad_and_missing_requests_are_distinct() {
    let fixture = fixture();
    assert_eq!(fixture.intent.status, ChangeIntentStatus::ReadyForPlanning);

    let mut ambiguous = fixture.intent.input.clone();
    ambiguous.clarifications.push(ChangeClarification {
        question_id: "question-sibling".to_owned(),
        dimension: ChangeUnknownDimension::Scope,
        question: "Should the uncited sibling behavior change?".to_owned(),
        evidence_fact_ids: Vec::new(),
        material: true,
    });
    assert_eq!(
        normalize_change_intent(&fixture.index, ambiguous)
            .expect("ambiguous intent is retained")
            .status,
        ChangeIntentStatus::ClarificationRequired
    );

    let mut contradictory = fixture.intent.input.clone();
    contradictory.requested_behavior = contradictory.current_behavior.clone();
    assert_eq!(
        normalize_change_intent(&fixture.index, contradictory),
        Err(ChangeIntentError::IntentInvalid)
    );

    let mut overbroad = fixture.intent.input.clone();
    overbroad.users = (0..65).map(|index| format!("user-{index:02}")).collect();
    assert_eq!(
        normalize_change_intent(&fixture.index, overbroad),
        Err(ChangeIntentError::IntentInvalid)
    );

    let mut missing = fixture.intent.input.clone();
    missing.acceptance_checks.clear();
    assert_eq!(
        normalize_change_intent(&fixture.index, missing),
        Err(ChangeIntentError::IntentInvalid)
    );
}

#[test]
fn tests_outside_the_minimal_target_and_forged_success_fail_closed() {
    let fixture = fixture();
    let reproduction = reproduction(&fixture.index);
    let mut intent_input = fixture.intent.input.clone();
    intent_input
        .target_fact_ids
        .retain(|identity| identity != &fixture.test_fact_id);
    intent_input
        .current_behavior_fact_ids
        .retain(|identity| identity != &fixture.test_fact_id);
    intent_input
        .surface_exclusions
        .push(ChangeSurfaceExclusion {
            surface: ChangeImpactSurfaceKind::Tests,
            rationale: "No test is declared in this intentionally incomplete candidate".to_owned(),
        });
    intent_input
        .surface_exclusions
        .sort_by_key(|item| item.surface);
    let intent = normalize_change_intent(&fixture.index, intent_input).expect("narrow intent");
    let impact = build_minimal_change_impact(&fixture.index, &intent).expect("narrow impact");
    assert_eq!(
        build_change_plan(
            &fixture.index,
            &intent,
            &impact,
            ChangePlanInput {
                plan_id: "plan-unexplained-test".to_owned(),
                work_kind: ChangeWorkKind::Defect,
                reproduction: Some(reproduction.clone()),
                regression_test: RegressionTestPlan {
                    disposition: RegressionTestDisposition::FailingTestRetained,
                    test_fact_ids: vec![fixture.test_fact_id.clone()],
                    rationale: None,
                    regression_sha256: String::new(),
                },
                hypotheses: hypotheses(&fixture.index, &reproduction),
                alternatives: alternatives(),
                validations: vec![PlannedValidation {
                    validation_id: "validation-regression".to_owned(),
                    kind: PlannedValidationKind::Unit,
                    description: "Run the unexplained regression fixture".to_owned(),
                    acceptance_checks: intent.input.acceptance_checks.clone(),
                    separate_grant_required: true,
                    write_authority: false,
                }],
            }
        ),
        Err(ChangePlanError::RegressionInvalid)
    );

    let mut forged = plan(&fixture);
    forged.mutation_authority = true;
    forged.status = ChangePlanStatus::ReadyForImplementationReview;
    assert!(!verify_change_plan(
        &fixture.index,
        &fixture.intent,
        &fixture.impact,
        &forged
    ));
}

#[test]
fn ten_thousand_change_plan_mutations_have_zero_verified_acceptance() {
    let fixture = fixture();
    let baseline = plan(&fixture);
    for sequence in 0_u32..10_000 {
        let mut changed = baseline.clone();
        let mutation = hash(&sequence.to_be_bytes());
        match sequence % 10 {
            0 => changed.index_sha256 = mutation,
            1 => changed.intent_sha256 = mutation,
            2 => changed.impact_sha256 = mutation,
            3 => changed.input.plan_id.push('x'),
            4 => changed.input.validations[0].write_authority = true,
            5 => changed.input.validations[0].separate_grant_required = false,
            6 => changed.input.alternatives[0].selected_option_id = "option-wide".to_owned(),
            7 => changed.required_reviews.clear(),
            8 => changed.mutation_authority = true,
            _ => changed.plan_sha256 = mutation,
        }
        assert!(
            !verify_change_plan(&fixture.index, &fixture.intent, &fixture.impact, &changed),
            "mutation {sequence} must fail closed"
        );
    }
}
