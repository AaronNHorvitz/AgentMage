//! Deterministic comparison gate for structural, lexical, semantic, and hybrid retrieval.

use std::collections::BTreeSet;

use agentmage_kernel_contracts::WorkspacePath;

const MAX_CASES: usize = 10_000;
const MAX_RANKED_PATHS: usize = 1_000;
const BASIS_POINTS: u64 = 10_000;

/// Retrieval task family used to preserve structural/lexical primacy for symbols.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RetrievalTaskClass {
    /// Exact code symbol, path, or structural relationship.
    CodeSymbol,
    /// Conceptual or prose-oriented knowledge question.
    ConceptProse,
}

/// Closed benchmark mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RetrievalBenchmarkMode {
    /// Structural map only.
    Structural,
    /// Exact lexical and metadata retrieval.
    Lexical,
    /// Approved local semantic retrieval only.
    Semantic,
    /// Policy-controlled structural, lexical, and semantic combination.
    Hybrid,
}

/// Final release behavior selected by the comparison gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetrievalReleaseBehavior {
    /// Deterministic structural and lexical retrieval remains the complete behavior.
    DeterministicOnly,
    /// Optional hybrid retrieval is eligible only for the exact measured profile and scope.
    HybridEligible,
}

/// One labeled question with all four mode results under identical limits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetrievalBenchmarkCase {
    /// Stable case identity.
    pub case_id: String,
    /// Declared task family.
    pub task_class: RetrievalTaskClass,
    /// Complete expected canonical source set.
    pub expected_paths: Vec<WorkspacePath>,
    /// Ranked structural-only source paths.
    pub structural_paths: Vec<WorkspacePath>,
    /// Ranked lexical-only source paths.
    pub lexical_paths: Vec<WorkspacePath>,
    /// Ranked semantic-only source paths.
    pub semantic_paths: Vec<WorkspacePath>,
    /// Ranked hybrid source paths.
    pub hybrid_paths: Vec<WorkspacePath>,
    /// Structural execution time in microseconds.
    pub structural_latency_us: u64,
    /// Lexical execution time in microseconds.
    pub lexical_latency_us: u64,
    /// Semantic execution time in microseconds.
    pub semantic_latency_us: u64,
    /// Hybrid execution time in microseconds.
    pub hybrid_latency_us: u64,
    /// Structural peak working bytes.
    pub structural_peak_bytes: u64,
    /// Lexical peak working bytes.
    pub lexical_peak_bytes: u64,
    /// Semantic peak working bytes.
    pub semantic_peak_bytes: u64,
    /// Hybrid peak working bytes.
    pub hybrid_peak_bytes: u64,
    /// Structural uncertainty or evidence-state failures.
    pub structural_uncertainty_failures: u32,
    /// Lexical uncertainty or evidence-state failures.
    pub lexical_uncertainty_failures: u32,
    /// Semantic uncertainty or evidence-state failures.
    pub semantic_uncertainty_failures: u32,
    /// Hybrid uncertainty or evidence-state failures.
    pub hybrid_uncertainty_failures: u32,
}

/// Integer quality and resource metrics for one mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetrievalBenchmarkMetrics {
    /// Evaluated case count.
    pub case_count: u64,
    /// Micro-averaged path precision in basis points.
    pub precision_bps: u32,
    /// Micro-averaged path recall in basis points.
    pub recall_bps: u32,
    /// Cases with a correct top-one path in basis points.
    pub top_1_bps: u32,
    /// Cases whose complete returned path set exactly matches expected in basis points.
    pub exact_citation_set_bps: u32,
    /// Maximum observed latency in microseconds.
    pub max_latency_us: u64,
    /// Maximum observed peak working bytes.
    pub max_peak_bytes: u64,
    /// Total uncertainty or evidence-state failures.
    pub uncertainty_failures: u64,
}

/// Approved comparison thresholds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticBenefitThresholds {
    /// Minimum hybrid recall gain on concept/prose cases over lexical retrieval.
    pub minimum_concept_recall_gain_bps: u32,
    /// Minimum allowed citation correctness for hybrid results.
    pub minimum_hybrid_citation_bps: u32,
    /// Maximum hybrid latency per case.
    pub maximum_hybrid_latency_us: u64,
    /// Maximum hybrid peak working bytes.
    pub maximum_hybrid_peak_bytes: u64,
    /// Maximum admitted uncertainty failures; normally zero.
    pub maximum_uncertainty_failures: u64,
}

/// Complete comparison report with explicit gains, regressions, and release decision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetrievalBenchmarkReport {
    /// Exact model-manifest digest under evaluation.
    pub model_manifest_sha256: String,
    /// Exact corpus digest.
    pub corpus_sha256: String,
    /// Exact hardware/environment evidence digest.
    pub hardware_sha256: String,
    /// Shared result limit used by every mode.
    pub result_limit: u32,
    /// Structural metrics.
    pub structural: RetrievalBenchmarkMetrics,
    /// Lexical metrics.
    pub lexical: RetrievalBenchmarkMetrics,
    /// Semantic metrics.
    pub semantic: RetrievalBenchmarkMetrics,
    /// Hybrid metrics.
    pub hybrid: RetrievalBenchmarkMetrics,
    /// Hybrid-minus-lexical concept/prose recall basis points.
    pub concept_recall_gain_bps: i32,
    /// Hybrid-minus-lexical overall precision basis points.
    pub precision_delta_bps: i32,
    /// Hybrid-minus-lexical exact citation-set basis points.
    pub citation_delta_bps: i32,
    /// Whether code-symbol hybrid recall was no worse than lexical recall.
    pub code_symbol_non_regression: bool,
    /// Exact release behavior selected by thresholds.
    pub release_behavior: RetrievalReleaseBehavior,
    /// Stable reasons retaining deterministic-only behavior.
    pub blockers: Vec<String>,
}

/// Closed benchmark validation failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetrievalBenchmarkError {
    /// Identity, threshold, case, ranking, or resource input is invalid.
    InvalidInput,
    /// Fixed resource or arithmetic bound was exceeded.
    ResourceLimit,
}

/// Compares all modes using the same labeled cases, hardware, and result limit.
pub fn benchmark_retrieval(
    model_manifest_sha256: String,
    corpus_sha256: String,
    hardware_sha256: String,
    result_limit: u32,
    thresholds: &SemanticBenefitThresholds,
    cases: &[RetrievalBenchmarkCase],
) -> Result<RetrievalBenchmarkReport, RetrievalBenchmarkError> {
    if !valid_sha256(&model_manifest_sha256)
        || !valid_sha256(&corpus_sha256)
        || !valid_sha256(&hardware_sha256)
        || result_limit == 0
        || result_limit as usize > MAX_RANKED_PATHS
        || cases.is_empty()
        || cases.len() > MAX_CASES
        || thresholds.minimum_concept_recall_gain_bps > BASIS_POINTS as u32
        || thresholds.minimum_hybrid_citation_bps > BASIS_POINTS as u32
        || thresholds.maximum_hybrid_latency_us == 0
        || thresholds.maximum_hybrid_peak_bytes == 0
    {
        return Err(RetrievalBenchmarkError::InvalidInput);
    }
    validate_cases(cases, result_limit)?;
    let structural = metrics(cases, RetrievalBenchmarkMode::Structural, None)?;
    let lexical = metrics(cases, RetrievalBenchmarkMode::Lexical, None)?;
    let semantic = metrics(cases, RetrievalBenchmarkMode::Semantic, None)?;
    let hybrid = metrics(cases, RetrievalBenchmarkMode::Hybrid, None)?;
    let concept_lexical = metrics(
        cases,
        RetrievalBenchmarkMode::Lexical,
        Some(RetrievalTaskClass::ConceptProse),
    )?;
    let concept_hybrid = metrics(
        cases,
        RetrievalBenchmarkMode::Hybrid,
        Some(RetrievalTaskClass::ConceptProse),
    )?;
    let code_lexical = metrics(
        cases,
        RetrievalBenchmarkMode::Lexical,
        Some(RetrievalTaskClass::CodeSymbol),
    )?;
    let code_hybrid = metrics(
        cases,
        RetrievalBenchmarkMode::Hybrid,
        Some(RetrievalTaskClass::CodeSymbol),
    )?;
    let concept_recall_gain_bps = delta(concept_hybrid.recall_bps, concept_lexical.recall_bps);
    let precision_delta_bps = delta(hybrid.precision_bps, lexical.precision_bps);
    let citation_delta_bps = delta(
        hybrid.exact_citation_set_bps,
        lexical.exact_citation_set_bps,
    );
    let code_symbol_non_regression = code_hybrid.recall_bps >= code_lexical.recall_bps;
    let mut blockers = Vec::new();
    if concept_recall_gain_bps < thresholds.minimum_concept_recall_gain_bps as i32 {
        blockers.push("concept-recall-gain-below-threshold".to_owned());
    }
    if hybrid.exact_citation_set_bps < thresholds.minimum_hybrid_citation_bps {
        blockers.push("hybrid-citation-correctness-below-threshold".to_owned());
    }
    if precision_delta_bps < 0 || citation_delta_bps < 0 {
        blockers.push("hybrid-grounding-regression".to_owned());
    }
    if !code_symbol_non_regression {
        blockers.push("code-symbol-regression".to_owned());
    }
    if hybrid.max_latency_us > thresholds.maximum_hybrid_latency_us {
        blockers.push("hybrid-latency-budget-exceeded".to_owned());
    }
    if hybrid.max_peak_bytes > thresholds.maximum_hybrid_peak_bytes {
        blockers.push("hybrid-memory-budget-exceeded".to_owned());
    }
    if hybrid.uncertainty_failures > thresholds.maximum_uncertainty_failures {
        blockers.push("hybrid-uncertainty-failure".to_owned());
    }
    let release_behavior = if blockers.is_empty() {
        RetrievalReleaseBehavior::HybridEligible
    } else {
        RetrievalReleaseBehavior::DeterministicOnly
    };
    Ok(RetrievalBenchmarkReport {
        model_manifest_sha256,
        corpus_sha256,
        hardware_sha256,
        result_limit,
        structural,
        lexical,
        semantic,
        hybrid,
        concept_recall_gain_bps,
        precision_delta_bps,
        citation_delta_bps,
        code_symbol_non_regression,
        release_behavior,
        blockers,
    })
}

fn validate_cases(
    cases: &[RetrievalBenchmarkCase],
    result_limit: u32,
) -> Result<(), RetrievalBenchmarkError> {
    let mut identities = BTreeSet::new();
    let mut has_concept = false;
    let mut has_code = false;
    for case in cases {
        if case.case_id.is_empty()
            || case.case_id.len() > 128
            || !case.case_id.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'-' | b'_' | b'.')
            })
            || !identities.insert(case.case_id.clone())
            || case.expected_paths.is_empty()
            || !unique(&case.expected_paths)
        {
            return Err(RetrievalBenchmarkError::InvalidInput);
        }
        has_concept |= case.task_class == RetrievalTaskClass::ConceptProse;
        has_code |= case.task_class == RetrievalTaskClass::CodeSymbol;
        for paths in [
            &case.structural_paths,
            &case.lexical_paths,
            &case.semantic_paths,
            &case.hybrid_paths,
        ] {
            if paths.len() > result_limit as usize || !unique(paths) {
                return Err(RetrievalBenchmarkError::InvalidInput);
            }
        }
    }
    if !has_concept || !has_code {
        return Err(RetrievalBenchmarkError::InvalidInput);
    }
    Ok(())
}

fn metrics(
    cases: &[RetrievalBenchmarkCase],
    mode: RetrievalBenchmarkMode,
    filter: Option<RetrievalTaskClass>,
) -> Result<RetrievalBenchmarkMetrics, RetrievalBenchmarkError> {
    let selected = cases
        .iter()
        .filter(|case| filter.is_none_or(|value| value == case.task_class))
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return Err(RetrievalBenchmarkError::InvalidInput);
    }
    let mut returned = 0_u64;
    let mut expected = 0_u64;
    let mut relevant = 0_u64;
    let mut top_one = 0_u64;
    let mut exact_sets = 0_u64;
    let mut max_latency = 0_u64;
    let mut max_memory = 0_u64;
    let mut uncertainty = 0_u64;
    for case in &selected {
        let paths = mode_paths(case, mode);
        let expected_set = case.expected_paths.iter().collect::<BTreeSet<_>>();
        let actual_set = paths.iter().collect::<BTreeSet<_>>();
        returned = returned
            .checked_add(paths.len() as u64)
            .ok_or(RetrievalBenchmarkError::ResourceLimit)?;
        expected = expected
            .checked_add(case.expected_paths.len() as u64)
            .ok_or(RetrievalBenchmarkError::ResourceLimit)?;
        relevant = relevant
            .checked_add(actual_set.intersection(&expected_set).count() as u64)
            .ok_or(RetrievalBenchmarkError::ResourceLimit)?;
        top_one += u64::from(
            paths
                .first()
                .is_some_and(|path| expected_set.contains(path)),
        );
        exact_sets += u64::from(actual_set == expected_set);
        max_latency = max_latency.max(mode_latency(case, mode));
        max_memory = max_memory.max(mode_memory(case, mode));
        uncertainty = uncertainty
            .checked_add(u64::from(mode_uncertainty(case, mode)))
            .ok_or(RetrievalBenchmarkError::ResourceLimit)?;
    }
    let count = selected.len() as u64;
    Ok(RetrievalBenchmarkMetrics {
        case_count: count,
        precision_bps: ratio(relevant, returned)?,
        recall_bps: ratio(relevant, expected)?,
        top_1_bps: ratio(top_one, count)?,
        exact_citation_set_bps: ratio(exact_sets, count)?,
        max_latency_us: max_latency,
        max_peak_bytes: max_memory,
        uncertainty_failures: uncertainty,
    })
}

fn mode_paths(case: &RetrievalBenchmarkCase, mode: RetrievalBenchmarkMode) -> &[WorkspacePath] {
    match mode {
        RetrievalBenchmarkMode::Structural => &case.structural_paths,
        RetrievalBenchmarkMode::Lexical => &case.lexical_paths,
        RetrievalBenchmarkMode::Semantic => &case.semantic_paths,
        RetrievalBenchmarkMode::Hybrid => &case.hybrid_paths,
    }
}

const fn mode_latency(case: &RetrievalBenchmarkCase, mode: RetrievalBenchmarkMode) -> u64 {
    match mode {
        RetrievalBenchmarkMode::Structural => case.structural_latency_us,
        RetrievalBenchmarkMode::Lexical => case.lexical_latency_us,
        RetrievalBenchmarkMode::Semantic => case.semantic_latency_us,
        RetrievalBenchmarkMode::Hybrid => case.hybrid_latency_us,
    }
}

const fn mode_memory(case: &RetrievalBenchmarkCase, mode: RetrievalBenchmarkMode) -> u64 {
    match mode {
        RetrievalBenchmarkMode::Structural => case.structural_peak_bytes,
        RetrievalBenchmarkMode::Lexical => case.lexical_peak_bytes,
        RetrievalBenchmarkMode::Semantic => case.semantic_peak_bytes,
        RetrievalBenchmarkMode::Hybrid => case.hybrid_peak_bytes,
    }
}

const fn mode_uncertainty(case: &RetrievalBenchmarkCase, mode: RetrievalBenchmarkMode) -> u32 {
    match mode {
        RetrievalBenchmarkMode::Structural => case.structural_uncertainty_failures,
        RetrievalBenchmarkMode::Lexical => case.lexical_uncertainty_failures,
        RetrievalBenchmarkMode::Semantic => case.semantic_uncertainty_failures,
        RetrievalBenchmarkMode::Hybrid => case.hybrid_uncertainty_failures,
    }
}

fn unique(paths: &[WorkspacePath]) -> bool {
    paths.iter().collect::<BTreeSet<_>>().len() == paths.len()
}

fn ratio(numerator: u64, denominator: u64) -> Result<u32, RetrievalBenchmarkError> {
    if denominator == 0 {
        return Ok(u32::from(numerator == 0) * BASIS_POINTS as u32);
    }
    let value = numerator
        .checked_mul(BASIS_POINTS)
        .ok_or(RetrievalBenchmarkError::ResourceLimit)?
        / denominator;
    u32::try_from(value).map_err(|_| RetrievalBenchmarkError::ResourceLimit)
}

fn delta(left: u32, right: u32) -> i32 {
    left as i32 - right as i32
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::{WorkspaceId, WorkspacePath};

    use super::{
        RetrievalBenchmarkCase, RetrievalBenchmarkError, RetrievalReleaseBehavior,
        RetrievalTaskClass, SemanticBenefitThresholds, benchmark_retrieval,
    };

    fn path(name: &str) -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-benchmark"),
            ["vault", name],
        )
        .expect("valid path")
    }

    fn case(
        id: &str,
        class: RetrievalTaskClass,
        expected: &str,
        structural: Vec<WorkspacePath>,
        lexical: Vec<WorkspacePath>,
        semantic: Vec<WorkspacePath>,
        hybrid: Vec<WorkspacePath>,
    ) -> RetrievalBenchmarkCase {
        RetrievalBenchmarkCase {
            case_id: id.to_owned(),
            task_class: class,
            expected_paths: vec![path(expected)],
            structural_paths: structural,
            lexical_paths: lexical,
            semantic_paths: semantic,
            hybrid_paths: hybrid,
            structural_latency_us: 10,
            lexical_latency_us: 20,
            semantic_latency_us: 80,
            hybrid_latency_us: 100,
            structural_peak_bytes: 100,
            lexical_peak_bytes: 200,
            semantic_peak_bytes: 800,
            hybrid_peak_bytes: 1_000,
            structural_uncertainty_failures: 0,
            lexical_uncertainty_failures: 0,
            semantic_uncertainty_failures: 0,
            hybrid_uncertainty_failures: 0,
        }
    }

    fn corpus() -> Vec<RetrievalBenchmarkCase> {
        vec![
            case(
                "concept-1",
                RetrievalTaskClass::ConceptProse,
                "concept.md",
                Vec::new(),
                Vec::new(),
                vec![path("concept.md")],
                vec![path("concept.md")],
            ),
            case(
                "symbol-1",
                RetrievalTaskClass::CodeSymbol,
                "symbol.md",
                vec![path("symbol.md")],
                vec![path("symbol.md")],
                Vec::new(),
                vec![path("symbol.md")],
            ),
        ]
    }

    fn thresholds() -> SemanticBenefitThresholds {
        SemanticBenefitThresholds {
            minimum_concept_recall_gain_bps: 5_000,
            minimum_hybrid_citation_bps: 10_000,
            maximum_hybrid_latency_us: 200,
            maximum_hybrid_peak_bytes: 2_000,
            maximum_uncertainty_failures: 0,
        }
    }

    #[test]
    fn complete_comparison_can_make_exact_hybrid_profile_eligible() {
        let report = benchmark_retrieval(
            "a".repeat(64),
            "b".repeat(64),
            "c".repeat(64),
            3,
            &thresholds(),
            &corpus(),
        )
        .expect("benchmark succeeds");

        assert_eq!(
            report.release_behavior,
            RetrievalReleaseBehavior::HybridEligible
        );
        assert!(report.blockers.is_empty());
        assert_eq!(report.concept_recall_gain_bps, 10_000);
        assert_eq!(report.hybrid.exact_citation_set_bps, 10_000);
        assert_eq!(report.lexical.exact_citation_set_bps, 5_000);
        assert!(report.code_symbol_non_regression);
        assert_eq!(report.hybrid.max_latency_us, 100);
        assert_eq!(report.hybrid.max_peak_bytes, 1_000);
    }

    #[test]
    fn quality_resource_uncertainty_and_code_regressions_retain_deterministic_behavior() {
        let mut cases = corpus();
        cases[0].hybrid_paths = vec![path("wrong.md")];
        cases[0].hybrid_latency_us = 500;
        cases[0].hybrid_peak_bytes = 5_000;
        cases[0].hybrid_uncertainty_failures = 1;
        cases[1].hybrid_paths.clear();
        let report = benchmark_retrieval(
            "a".repeat(64),
            "b".repeat(64),
            "c".repeat(64),
            3,
            &thresholds(),
            &cases,
        )
        .expect("benchmark succeeds");

        assert_eq!(
            report.release_behavior,
            RetrievalReleaseBehavior::DeterministicOnly
        );
        assert!(
            report
                .blockers
                .contains(&"concept-recall-gain-below-threshold".to_owned())
        );
        assert!(
            report
                .blockers
                .contains(&"hybrid-citation-correctness-below-threshold".to_owned())
        );
        assert!(
            report
                .blockers
                .contains(&"hybrid-grounding-regression".to_owned())
        );
        assert!(
            report
                .blockers
                .contains(&"code-symbol-regression".to_owned())
        );
        assert!(
            report
                .blockers
                .contains(&"hybrid-latency-budget-exceeded".to_owned())
        );
        assert!(
            report
                .blockers
                .contains(&"hybrid-memory-budget-exceeded".to_owned())
        );
        assert!(
            report
                .blockers
                .contains(&"hybrid-uncertainty-failure".to_owned())
        );
    }

    #[test]
    fn corpus_requires_both_task_classes_unique_cases_paths_and_shared_bounds() {
        let mut cases = corpus();
        cases.pop();
        assert_eq!(
            benchmark_retrieval(
                "a".repeat(64),
                "b".repeat(64),
                "c".repeat(64),
                3,
                &thresholds(),
                &cases,
            ),
            Err(RetrievalBenchmarkError::InvalidInput)
        );

        let mut duplicate = corpus();
        duplicate[1].case_id = duplicate[0].case_id.clone();
        assert_eq!(
            benchmark_retrieval(
                "a".repeat(64),
                "b".repeat(64),
                "c".repeat(64),
                3,
                &thresholds(),
                &duplicate,
            ),
            Err(RetrievalBenchmarkError::InvalidInput)
        );

        let mut duplicate_path = corpus();
        duplicate_path[1].hybrid_paths.push(path("symbol.md"));
        assert_eq!(
            benchmark_retrieval(
                "a".repeat(64),
                "b".repeat(64),
                "c".repeat(64),
                3,
                &thresholds(),
                &duplicate_path,
            ),
            Err(RetrievalBenchmarkError::InvalidInput)
        );
    }
}
