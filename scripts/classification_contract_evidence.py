#!/usr/bin/env python3
"""Task 12.2.2.1 classification-contract evidence specification."""

from pathlib import Path

try:
    from scripts.revision_evidence import RevisionEvidenceSpec, run_cli
except ModuleNotFoundError:
    from revision_evidence import RevisionEvidenceSpec, run_cli

ROOT = Path(__file__).resolve().parents[1]
SPEC = RevisionEvidenceSpec(
    root=ROOT,
    report_path=ROOT / "artifacts/sprints/sprint-12/story-12.2/classification-contracts.json",
    artifact_id="task-12-2-2-1-classification-contracts",
    case_pattern=r"`(CLS-\d{2})`",
    case_ids=tuple(f"CLS-{index:02}" for index in range(1, 7)),
    source_paths=(
        "docs/verification/task-12-2-2-1-classification-contract-results.md",
        "kernel/contracts/src/classification.rs",
        "kernel/contracts/src/lib.rs",
        "kernel/contracts/src/serialization.rs",
        "scripts/revision_evidence.py",
        "scripts/classification_contract_evidence.py",
        "tests/test_classification_contract_evidence.py",
    ),
    document_fragments=(
        "Pass for five separate closed deterministic and advisory classification schemas.",
        "all seven roles and four measured dispositions",
        "only five restrictive or escalating dispositions",
        "has no grant, operation, destination, model-selection, execution, or completion field",
        "manual fuzzing",
    ),
    source_fragments={
        "kernel/contracts/src/classification.rs": (
            "pub struct DataSensitivityAssessment",
            "pub struct ActionRiskAssessment",
            "pub struct ModelCapabilityAssessment",
            "pub struct DeterministicPolicyFacts",
            "pub struct AdvisoryClassifierResult",
        ),
        "kernel/contracts/src/lib.rs": ("pub use classification::{",),
        "kernel/contracts/src/serialization.rs": ("crate::AdvisoryClassifierResult,",),
    },
    command_specs=(
        (("cargo", "test", "-p", "agentmage-kernel-contracts", "task_12_2_2_1", "--locked"), "6 passed; 0 failed"),
        (("cargo", "clippy", "-p", "agentmage-kernel-contracts", "--all-targets", "--locked", "--", "-D", "warnings"), "Finished `dev` profile"),
        (("python3", "-m", "unittest", "tests.test_classification_contract_evidence"), "Ran 4 tests"),
        (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
    ),
    claims={
        "focused_case_count": 6,
        "separate_versioned_schema_count": 5,
        "data_sensitivity_label_count": 4,
        "action_risk_label_count": 4,
        "model_capability_role_count": 7,
        "model_capability_status_count": 4,
        "static_policy_check_kind_count": 4,
        "advisory_classifier_status_count": 8,
        "advisory_disposition_count": 5,
        "advisory_authority_field_count": 0,
        "advisory_completion_field_count": 0,
        "model_executions": 0,
        "classifier_executions": 0,
        "tool_executions": 0,
        "platform_executions": 0,
        "external_network_used": False,
        "manual_fuzzing_executed": False,
        "release_support": False,
    },
    limitations=(
        "This task defines closed wire schemas; deterministic fact collection and policy evaluation begin in Task 12.2.2.2.",
        "Advisory disposition enforcement, failure mapping, and continuous boundary reclassification remain Tasks 12.2.2.3 through 12.2.2.5.",
        "Tests use synthetic content-free records with no model, classifier, tool, worker, platform effect, private user data, or external network operation.",
        "Cross-platform execution, packaging, release acceptance, and manual fuzzing remain later gates.",
    ),
    status="pass-separated-classification-contracts",
    task_ids=("12.2.2.1",),
    label="Task 12.2.2.1 classification contracts",
)


if __name__ == "__main__":
    raise SystemExit(run_cli(SPEC))
