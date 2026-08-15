//! Evidence-bound test-generation plans over verified structured test-file changes.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use agentmage_kernel_contracts::WorkspacePath;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    StructuredArtifactClass, StructuredFileChangePlan, StructuredLanguage,
    verify_structured_file_change,
};

const TEST_GENERATION_SCHEMA_VERSION: u16 = 1;
const MAX_CASES: usize = 256;
const MAX_CHANGES: usize = 32;

/// Closed behavior concerns considered for every generated test plan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestConcern {
    /// Public and internal behavior boundaries.
    Boundary,
    /// Empty, extreme, malformed, and limit cases.
    EdgeCase,
    /// Explicit error and partial-failure behavior.
    Failure,
    /// Authorization, denial, and least-privilege behavior.
    Permission,
    /// Persistence, schema, and data-integrity behavior.
    DataChange,
    /// Restoration and concurrent-change behavior.
    Rollback,
}

impl TestConcern {
    /// Complete stable applicability checklist.
    pub const ALL: [Self; 6] = [
        Self::Boundary,
        Self::EdgeCase,
        Self::Failure,
        Self::Permission,
        Self::DataChange,
        Self::Rollback,
    ];
}

/// Closed admitted repository test framework.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryTestFramework {
    /// Rust built-in test harness under Cargo.
    CargoTest,
    /// Python tests collected by Pytest.
    Pytest,
    /// TypeScript or JavaScript tests collected by Vitest.
    Vitest,
    /// JavaScript tests executed by the Node test runner.
    NodeTest,
    /// Go built-in test runner.
    GoTest,
    /// Shell Test Anything Protocol harness.
    ShellTap,
    /// SQL fixture or migration-test harness.
    SqlFixture,
}

/// Exact repository evidence defining the selected test style.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryTestStyle {
    /// Stable style identity.
    pub style_id: String,
    /// Admitted framework.
    pub framework: RepositoryTestFramework,
    /// Exact current example test path.
    pub example_path: WorkspacePath,
    /// SHA-256 of the complete example source.
    pub example_source_sha256: String,
    /// SHA-256 of parsed style metadata such as naming and fixture conventions.
    pub style_evidence_sha256: String,
    /// Exact trusted command-template identity proposed for later validation.
    pub test_command_id: String,
    /// Optional exact trusted formatter-template identity.
    pub formatter_command_id: Option<String>,
}

/// Explicit applicability decision for one required test concern.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestConcernApplicability {
    /// Closed concern.
    pub concern: TestConcern,
    /// Whether the approved change requires at least one case for this concern.
    pub applicable: bool,
    /// Exact evidence-backed rationale digest.
    pub rationale_sha256: String,
}

/// Expected initial state for one proposed generated test.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeneratedTestExpectation {
    /// Test should pass against the proposed postimage.
    PassAfterChange,
    /// Reproduced regression test should fail against the approved base.
    FailOnApprovedBase,
}

/// One untrusted semantic test-case proposal linked to exact source evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedTestCase {
    /// Stable case identity.
    pub case_id: String,
    /// Explicit covered concern.
    pub concern: TestConcern,
    /// Exact test-file path containing the proposed case.
    pub target_path: WorkspacePath,
    /// SHA-256 of the test name rather than raw model-authored content.
    pub test_name_sha256: String,
    /// Exact behavior or requirement evidence identity.
    pub behavior_evidence_sha256: String,
    /// Expected initial state.
    pub expectation: GeneratedTestExpectation,
    /// Classification remains untrusted until separately executed.
    pub untrusted_classification: bool,
}

/// Exact binding to one verified structured test-file change.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedTestChangeBinding {
    /// Exact target test path.
    pub path: WorkspacePath,
    /// Structured language or safe textual dialect.
    pub language: StructuredLanguage,
    /// Exact structured change-plan identity.
    pub structured_plan_sha256: String,
    /// Exact complete test postimage identity.
    pub postimage_sha256: String,
}

/// Input for one complete authority-free test-generation plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestGenerationRequest {
    /// Stable generation identity.
    pub generation_id: String,
    /// Exact approved intent identity.
    pub intent_sha256: String,
    /// Exact approved change-plan identity.
    pub change_plan_sha256: String,
    /// Repository style evidence.
    pub style: RepositoryTestStyle,
    /// Every concern exactly once in enum order.
    pub applicability: Vec<TestConcernApplicability>,
    /// Stable case-id-sorted proposed cases.
    pub cases: Vec<GeneratedTestCase>,
    /// Stable path-sorted verified test-file proposals.
    pub test_changes: Vec<StructuredFileChangePlan>,
}

/// Complete hash-bound test-generation plan with no execution or write authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestGenerationPlan {
    /// Schema version.
    pub schema_version: u16,
    /// Stable generation identity.
    pub generation_id: String,
    /// Exact approved intent identity.
    pub intent_sha256: String,
    /// Exact approved change-plan identity.
    pub change_plan_sha256: String,
    /// Exact repository style evidence.
    pub style: RepositoryTestStyle,
    /// Complete applicability ledger.
    pub applicability: Vec<TestConcernApplicability>,
    /// Stable untrusted semantic case proposals.
    pub cases: Vec<GeneratedTestCase>,
    /// Stable verified source-change bindings.
    pub test_changes: Vec<GeneratedTestChangeBinding>,
    /// Tests remain unverified until a trusted runner produces evidence.
    pub execution_verified: bool,
    /// Later validation requires a separate command grant.
    pub separate_validation_grant_required: bool,
    /// This plan carries no write authority.
    pub mutation_authority: bool,
    /// SHA-256 over every preceding field.
    pub plan_sha256: String,
}

/// Stable content-free test-generation failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestGenerationError {
    /// Input syntax, bounds, identities, ordering, or workspace scope are invalid.
    InvalidInput,
    /// Applicability is incomplete, duplicated, or unsupported by proposed cases.
    ApplicabilityInvalid,
    /// Repository style and proposed language are inconsistent.
    StyleMismatch,
    /// A proposed test-file change is invalid or not classified as a test.
    ChangeInvalid,
}

impl TestGenerationError {
    /// Stable content-free error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidInput => "test-generation.input-invalid",
            Self::ApplicabilityInvalid => "test-generation.applicability-invalid",
            Self::StyleMismatch => "test-generation.style-mismatch",
            Self::ChangeInvalid => "test-generation.change-invalid",
        }
    }
}

/// Builds one evidence-bound test-generation plan without writing or executing tests.
pub fn build_test_generation_plan(
    request: TestGenerationRequest,
) -> Result<TestGenerationPlan, TestGenerationError> {
    validate_request(&request)?;
    let change_by_path = request
        .test_changes
        .iter()
        .map(|change| (change.summary().path.clone(), change))
        .collect::<BTreeMap<_, _>>();
    if change_by_path.len() != request.test_changes.len() {
        return Err(TestGenerationError::ChangeInvalid);
    }
    for change in &request.test_changes {
        if !verify_structured_file_change(change)
            || change.summary().artifact_class != StructuredArtifactClass::Test
            || change.summary().intent_sha256 != request.intent_sha256
            || change.summary().change_plan_sha256 != request.change_plan_sha256
            || !framework_accepts(request.style.framework, change.summary().language)
        {
            return Err(TestGenerationError::ChangeInvalid);
        }
    }
    let applicable = request
        .applicability
        .iter()
        .filter(|entry| entry.applicable)
        .map(|entry| entry.concern)
        .collect::<BTreeSet<_>>();
    let covered = request
        .cases
        .iter()
        .map(|case| case.concern)
        .collect::<BTreeSet<_>>();
    if applicable != covered
        || request
            .cases
            .iter()
            .any(|case| !change_by_path.contains_key(&case.target_path))
    {
        return Err(TestGenerationError::ApplicabilityInvalid);
    }
    let test_changes = request
        .test_changes
        .iter()
        .map(|change| GeneratedTestChangeBinding {
            path: change.summary().path.clone(),
            language: change.summary().language,
            structured_plan_sha256: change.plan_sha256().to_owned(),
            postimage_sha256: change.summary().postimage_sha256.clone(),
        })
        .collect();
    let mut plan = TestGenerationPlan {
        schema_version: TEST_GENERATION_SCHEMA_VERSION,
        generation_id: request.generation_id,
        intent_sha256: request.intent_sha256,
        change_plan_sha256: request.change_plan_sha256,
        style: request.style,
        applicability: request.applicability,
        cases: request.cases,
        test_changes,
        execution_verified: false,
        separate_validation_grant_required: true,
        mutation_authority: false,
        plan_sha256: String::new(),
    };
    plan.plan_sha256 = plan_digest(&plan);
    Ok(plan)
}

/// Verifies plan structure and digest without treating semantic classifications as proof.
#[must_use]
pub fn verify_test_generation_plan(plan: &TestGenerationPlan) -> bool {
    plan.schema_version == TEST_GENERATION_SCHEMA_VERSION
        && valid_identifier(&plan.generation_id)
        && is_sha256(&plan.intent_sha256)
        && is_sha256(&plan.change_plan_sha256)
        && valid_style(&plan.style)
        && valid_applicability(&plan.applicability)
        && valid_cases(&plan.cases, &plan.applicability, &plan.test_changes)
        && !plan.execution_verified
        && plan.separate_validation_grant_required
        && !plan.mutation_authority
        && plan.plan_sha256 == plan_digest(plan)
}

fn validate_request(request: &TestGenerationRequest) -> Result<(), TestGenerationError> {
    if !valid_identifier(&request.generation_id)
        || !is_sha256(&request.intent_sha256)
        || !is_sha256(&request.change_plan_sha256)
        || !valid_style(&request.style)
        || request.cases.is_empty()
        || request.cases.len() > MAX_CASES
        || request.test_changes.is_empty()
        || request.test_changes.len() > MAX_CHANGES
        || request
            .test_changes
            .windows(2)
            .any(|pair| pair[0].summary().path >= pair[1].summary().path)
        || !valid_cases_syntax(&request.cases)
    {
        return Err(TestGenerationError::InvalidInput);
    }
    if !valid_applicability(&request.applicability) {
        return Err(TestGenerationError::ApplicabilityInvalid);
    }
    if request.test_changes.iter().any(|change| {
        change.summary().path.workspace_id() != request.style.example_path.workspace_id()
    }) {
        return Err(TestGenerationError::StyleMismatch);
    }
    Ok(())
}

fn valid_style(style: &RepositoryTestStyle) -> bool {
    valid_identifier(&style.style_id)
        && is_sha256(&style.example_source_sha256)
        && is_sha256(&style.style_evidence_sha256)
        && valid_identifier(&style.test_command_id)
        && style
            .formatter_command_id
            .as_deref()
            .is_none_or(valid_identifier)
}

fn valid_applicability(entries: &[TestConcernApplicability]) -> bool {
    entries.len() == TestConcern::ALL.len()
        && entries
            .iter()
            .map(|entry| entry.concern)
            .eq(TestConcern::ALL)
        && entries
            .iter()
            .all(|entry| is_sha256(&entry.rationale_sha256))
        && entries.iter().any(|entry| entry.applicable)
}

fn valid_cases_syntax(cases: &[GeneratedTestCase]) -> bool {
    cases
        .windows(2)
        .all(|pair| pair[0].case_id < pair[1].case_id)
        && cases.iter().all(|case| {
            valid_identifier(&case.case_id)
                && is_sha256(&case.test_name_sha256)
                && is_sha256(&case.behavior_evidence_sha256)
                && case.untrusted_classification
        })
}

fn valid_cases(
    cases: &[GeneratedTestCase],
    applicability: &[TestConcernApplicability],
    changes: &[GeneratedTestChangeBinding],
) -> bool {
    if cases.is_empty()
        || cases.len() > MAX_CASES
        || !valid_cases_syntax(cases)
        || !valid_applicability(applicability)
        || changes.is_empty()
        || changes.len() > MAX_CHANGES
        || changes.windows(2).any(|pair| pair[0].path >= pair[1].path)
        || changes.iter().any(|change| {
            !is_sha256(&change.structured_plan_sha256) || !is_sha256(&change.postimage_sha256)
        })
    {
        return false;
    }
    let applicable = applicability
        .iter()
        .filter(|entry| entry.applicable)
        .map(|entry| entry.concern)
        .collect::<BTreeSet<_>>();
    let covered = cases
        .iter()
        .map(|case| case.concern)
        .collect::<BTreeSet<_>>();
    let paths = changes
        .iter()
        .map(|change| &change.path)
        .collect::<BTreeSet<_>>();
    applicable == covered && cases.iter().all(|case| paths.contains(&case.target_path))
}

const fn framework_accepts(
    framework: RepositoryTestFramework,
    language: StructuredLanguage,
) -> bool {
    matches!(
        (framework, language),
        (RepositoryTestFramework::CargoTest, StructuredLanguage::Rust)
            | (RepositoryTestFramework::Pytest, StructuredLanguage::Python)
            | (
                RepositoryTestFramework::Vitest,
                StructuredLanguage::TypeScript
                    | StructuredLanguage::Tsx
                    | StructuredLanguage::JavaScript
            )
            | (
                RepositoryTestFramework::NodeTest,
                StructuredLanguage::JavaScript
            )
            | (RepositoryTestFramework::GoTest, StructuredLanguage::Go)
            | (RepositoryTestFramework::ShellTap, StructuredLanguage::Shell)
            | (RepositoryTestFramework::SqlFixture, StructuredLanguage::Sql)
    )
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn plan_digest(plan: &TestGenerationPlan) -> String {
    let mut canonical = plan.clone();
    canonical.plan_sha256.clear();
    sha256_json(&canonical)
}

fn sha256_json(value: &impl Serialize) -> String {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .unwrap_or_else(|_| sha256_hex(b"test-generation-serialization-failed"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use agentmage_kernel_contracts::WorkspaceId;

    use crate::{StructuredEdit, StructuredFileChangeRequest, build_structured_file_change};

    use super::*;

    fn path(parts: &[&str]) -> WorkspacePath {
        WorkspacePath::new(
            WorkspaceId::from_raw("workspace-test-generation"),
            parts.iter().copied(),
        )
        .expect("path")
    }

    fn test_change() -> StructuredFileChangePlan {
        let source = "def test_existing() -> None:\n    assert True\n";
        let replacement = "def test_existing() -> None:\n    assert True\n\n\ndef test_permission_denied() -> None:\n    assert True\n\n\ndef test_rollback_conflict() -> None:\n    assert True\n";
        build_structured_file_change(StructuredFileChangeRequest {
            change_id: "change-generated-tests".to_owned(),
            path: path(&["tests", "test_access.py"]),
            intent_sha256: "1".repeat(64),
            change_plan_sha256: "2".repeat(64),
            language: StructuredLanguage::Python,
            artifact_class: StructuredArtifactClass::Test,
            preimage: source.as_bytes().to_vec(),
            expected_preimage_sha256: sha256_hex(source.as_bytes()),
            edits: vec![StructuredEdit::ReplaceSyntaxNode {
                edit_id: "edit-test-module".to_owned(),
                start_byte: 0,
                end_byte: u64::try_from(source.len()).expect("length"),
                expected_node_sha256: sha256_hex(source.as_bytes()),
                replacement: replacement.to_owned(),
            }],
            additional_review_hooks: Vec::new(),
            generated: false,
            allow_generated: false,
        })
        .expect("test change")
    }

    fn request() -> TestGenerationRequest {
        let target = path(&["tests", "test_access.py"]);
        TestGenerationRequest {
            generation_id: "test-generation-access".to_owned(),
            intent_sha256: "1".repeat(64),
            change_plan_sha256: "2".repeat(64),
            style: RepositoryTestStyle {
                style_id: "pytest-local-style".to_owned(),
                framework: RepositoryTestFramework::Pytest,
                example_path: target.clone(),
                example_source_sha256: "3".repeat(64),
                style_evidence_sha256: "4".repeat(64),
                test_command_id: "pytest-focused".to_owned(),
                formatter_command_id: Some("ruff-format-check".to_owned()),
            },
            applicability: TestConcern::ALL
                .into_iter()
                .map(|concern| TestConcernApplicability {
                    concern,
                    applicable: matches!(concern, TestConcern::Permission | TestConcern::Rollback),
                    rationale_sha256: "5".repeat(64),
                })
                .collect(),
            cases: vec![
                GeneratedTestCase {
                    case_id: "case-permission".to_owned(),
                    concern: TestConcern::Permission,
                    target_path: target.clone(),
                    test_name_sha256: "6".repeat(64),
                    behavior_evidence_sha256: "7".repeat(64),
                    expectation: GeneratedTestExpectation::PassAfterChange,
                    untrusted_classification: true,
                },
                GeneratedTestCase {
                    case_id: "case-rollback".to_owned(),
                    concern: TestConcern::Rollback,
                    target_path: target,
                    test_name_sha256: "8".repeat(64),
                    behavior_evidence_sha256: "9".repeat(64),
                    expectation: GeneratedTestExpectation::PassAfterChange,
                    untrusted_classification: true,
                },
            ],
            test_changes: vec![test_change()],
        }
    }

    #[test]
    fn repository_style_and_all_concern_decisions_bind_verified_test_changes() {
        let plan = build_test_generation_plan(request()).expect("plan");
        assert!(verify_test_generation_plan(&plan));
        assert_eq!(plan.applicability.len(), TestConcern::ALL.len());
        assert_eq!(plan.cases.len(), 2);
        assert!(!plan.execution_verified);
        assert!(plan.separate_validation_grant_required);
        assert!(!plan.mutation_authority);
    }

    #[test]
    fn missing_concerns_cases_style_and_plan_drift_fail_closed() {
        let mut missing = request();
        missing.applicability.pop();
        assert_eq!(
            build_test_generation_plan(missing),
            Err(TestGenerationError::ApplicabilityInvalid)
        );

        let mut uncovered = request();
        uncovered.cases.pop();
        assert_eq!(
            build_test_generation_plan(uncovered),
            Err(TestGenerationError::ApplicabilityInvalid)
        );

        let mut style = request();
        style.style.framework = RepositoryTestFramework::GoTest;
        assert_eq!(
            build_test_generation_plan(style),
            Err(TestGenerationError::ChangeInvalid)
        );

        let mut plan = build_test_generation_plan(request()).expect("plan");
        plan.execution_verified = true;
        assert!(!verify_test_generation_plan(&plan));
    }
}
