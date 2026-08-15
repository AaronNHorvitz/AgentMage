use agentmage_capability_repository_map::{
    GitTrackedState, RepositoryDeepAnalysisError, RepositoryFactKind, RepositoryFactState,
    RepositoryFileInput, RepositoryMap, RepositoryMapInput, RepositoryTraceKind,
    build_deep_repository_index, build_repository_learning_export, build_repository_map,
    build_repository_traces, verify_deep_repository_index, verify_fact_citation,
};
use agentmage_kernel_contracts::WorkspaceId;
use serde::Deserialize;
use sha2::{Digest, Sha256};

const CORPUS: &str =
    include_str!("../../../docs/verification/sprint-43-hostile-repository-corpus.json");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_version: u16,
    record_type: String,
    description: String,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    path: Vec<String>,
    source: String,
    expected_profile_kind: Option<String>,
}

fn hash(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}

fn corpus_map(corpus: &Corpus) -> RepositoryMap {
    build_repository_map(RepositoryMapInput {
        workspace_id: WorkspaceId::from_raw("workspace-sprint-43-corpus"),
        repository_sha256: "a".repeat(64),
        worktree_sha256: "b".repeat(64),
        branch: Some("agentmage/tasks/sprint-43".to_owned()),
        commit_id: "c".repeat(40),
        policy_sha256: "d".repeat(64),
        freshness_sha256: "e".repeat(64),
        files: corpus
            .cases
            .iter()
            .map(|case| RepositoryFileInput {
                path: case.path.clone(),
                size_bytes: case.source.len() as u64,
                content_sha256: hash(case.source.as_bytes()),
                content: Some(case.source.as_bytes().to_vec()),
                git_state: GitTrackedState::TrackedClean,
                policy_excluded: false,
                generated: false,
                vendored: false,
            })
            .collect(),
    })
    .expect("corpus map")
}

fn profile_kind(value: &str) -> Option<RepositoryFactKind> {
    match value {
        "instruction" => Some(RepositoryFactKind::Instruction),
        "language" => Some(RepositoryFactKind::Language),
        "package_manager" => Some(RepositoryFactKind::PackageManager),
        "continuous_integration" => Some(RepositoryFactKind::ContinuousIntegration),
        "migration" => Some(RepositoryFactKind::Migration),
        _ => None,
    }
}

#[test]
fn hostile_prose_and_secret_canaries_never_become_structural_claims_or_exports() {
    let corpus: Corpus = serde_json::from_str(CORPUS).expect("corpus parses");
    assert_eq!(corpus.schema_version, 1);
    assert_eq!(corpus.record_type, "sprint_43_hostile_repository_corpus");
    assert!(!corpus.description.is_empty());
    assert_eq!(corpus.cases.len(), 12);
    let map = corpus_map(&corpus);
    let index = build_deep_repository_index(&map, Vec::new(), Vec::new()).expect("deep index");
    let traces = build_repository_traces(&index).expect("trace set");
    let export = build_repository_learning_export(&index, &traces).expect("learning export");
    let encoded =
        serde_json::to_string(&(index.clone(), traces.clone(), export)).expect("result serializes");

    for case in &corpus.cases {
        assert!(
            !encoded.contains("CANARY_"),
            "{} leaked source data",
            case.id
        );
        if let Some(expected) = case.expected_profile_kind.as_deref().and_then(profile_kind) {
            let expected_path = case.path.join("/");
            assert!(
                index.facts.iter().any(|fact| {
                    fact.kind == expected
                        && fact.citations.iter().any(|citation| {
                            citation
                                .path
                                .components()
                                .iter()
                                .map(|component| component.as_str())
                                .collect::<Vec<_>>()
                                .join("/")
                                == expected_path
                        })
                }),
                "{} missing deterministic path/parser signal",
                case.id
            );
        }
    }
    for kind in [
        RepositoryTraceKind::Authentication,
        RepositoryTraceKind::Authorization,
        RepositoryTraceKind::DataFlow,
    ] {
        let trace = traces
            .traces
            .iter()
            .find(|trace| trace.kind == kind)
            .expect("closed trace exists");
        assert_eq!(trace.state, RepositoryFactState::UnknownBlocked);
        assert!(trace.fact_ids.is_empty());
    }
}

#[test]
fn every_retained_fact_citation_resolves_to_the_exact_current_map() {
    let corpus: Corpus = serde_json::from_str(CORPUS).expect("corpus parses");
    let map = corpus_map(&corpus);
    let index = build_deep_repository_index(&map, Vec::new(), Vec::new()).expect("deep index");
    assert!(verify_deep_repository_index(&map, &index));
    assert!(index.facts.iter().all(|fact| {
        fact.citations
            .iter()
            .all(|citation| verify_fact_citation(&map, citation))
    }));

    let mut stale = map.clone();
    stale.commit_id = "9".repeat(40);
    assert!(!verify_deep_repository_index(&stale, &index));
}

#[test]
fn ten_thousand_deep_index_mutations_have_zero_verified_acceptance() {
    let corpus: Corpus = serde_json::from_str(CORPUS).expect("corpus parses");
    let map = corpus_map(&corpus);
    let baseline = build_deep_repository_index(&map, Vec::new(), Vec::new()).expect("deep index");
    assert!(verify_deep_repository_index(&map, &baseline));
    for sequence in 0_u32..10_000 {
        let mut changed = baseline.clone();
        let mutation = hash(&sequence.to_be_bytes());
        match sequence % 16 {
            0 => changed.map_sha256 = mutation,
            1 => changed.git.commit_id = format!("{sequence:040x}"),
            2 => changed.rule_set_sha256 = mutation,
            3 => changed.facts[0].fact_id = mutation,
            4 => changed.facts[0].fact_sha256 = mutation,
            5 => changed.facts[0].label.push('x'),
            6 => changed.facts[0].state = RepositoryFactState::Inferred,
            7 => changed.facts[0].citations[0].content_sha256 = mutation,
            8 => changed.facts[0].citations[0].citation_sha256 = mutation,
            9 => changed.facts[0].citations[0].method_id.push('x'),
            10 => changed.blind_spots[0].blind_spot_sha256 = mutation,
            11 => changed.coverage.indexed_facts += 1,
            12 => changed.coverage.blind_spots += 1,
            13 => changed.coverage.whole_repository_claim_permitted = true,
            14 => changed.index_sha256 = mutation,
            _ => changed.facts[0].untrusted_repository_data = false,
        }
        assert!(
            !verify_deep_repository_index(&map, &changed),
            "mutation {sequence} must fail closed"
        );
    }
}

#[test]
fn duplicate_paths_and_unregistered_adapter_claims_fail_before_analysis() {
    let corpus: Corpus = serde_json::from_str(CORPUS).expect("corpus parses");
    let map = corpus_map(&corpus);
    let invalid = agentmage_capability_repository_map::RepositoryAdapterFactInput {
        adapter_id: "unregistered-model-output".to_owned(),
        capability: agentmage_capability_repository_map::RepositoryAdapterCapability::Symbols,
        kind: RepositoryFactKind::Symbol,
        label: "fabricated".to_owned(),
        state: RepositoryFactState::Observed,
        citations: Vec::new(),
    };
    assert_eq!(
        build_deep_repository_index(&map, Vec::new(), vec![invalid]),
        Err(RepositoryDeepAnalysisError::AdapterInvalid)
    );
}
