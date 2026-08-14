#!/usr/bin/env python3
"""Task 12.2.3.2 policy-reference evidence specification."""

from pathlib import Path

try:
    from scripts.revision_evidence import RevisionEvidenceSpec, run_cli
except ModuleNotFoundError:
    from revision_evidence import RevisionEvidenceSpec, run_cli

ROOT = Path(__file__).resolve().parents[1]
SPEC = RevisionEvidenceSpec(
    root=ROOT,
    report_path=ROOT / "artifacts/sprints/sprint-12/story-12.2/policy-reference.json",
    artifact_id="task-12-2-3-2-policy-reference",
    case_pattern=r"`(POL-ART-\d{2})`",
    case_ids=tuple(f"POL-ART-{index:02}" for index in range(1, 6)),
    source_paths=(
        "docs/verification/task-12-2-3-2-policy-reference-results.md",
        "docs/architecture/deterministic-and-advisory-policy-reference.md",
        "kernel/contracts/src/classification.rs",
        "kernel/engine/src/preclassification_policy.rs",
        "kernel/engine/src/advisory_policy.rs",
        "kernel/engine/src/reclassification.rs",
        "scripts/revision_evidence.py",
        "scripts/policy_reference_evidence.py",
        "tests/test_policy_reference_evidence.py",
    ),
    document_fragments=(
        "Pass for the deterministic-policy and advisory-classifier review reference.",
        "all 28 closed fields and all 13 checks",
        "only five restrictive dispositions",
        "all seven incomplete states",
        "56 directed boundary pairs",
    ),
    source_fragments={
        "docs/architecture/deterministic-and-advisory-policy-reference.md": (
            "## Deterministic Policy Fact Table",
            "## Advisory Classifier Output Schema",
            "## Authority Matrix",
            "## Classifier Failure Map",
            "## Reclassification Trigger Inventory",
        ),
        "kernel/contracts/src/classification.rs": (
            "pub struct DeterministicPolicyFacts",
            "pub struct AdvisoryClassifierResult",
            "pub enum ReclassificationContentKind",
            "pub enum ClassificationBoundary",
        ),
        "kernel/engine/src/preclassification_policy.rs": (
            "pub struct PreclassificationPolicyGate",
        ),
        "kernel/engine/src/advisory_policy.rs": ("pub struct AdvisoryPolicyGate",),
        "kernel/engine/src/reclassification.rs": ("pub struct ReclassificationGate",),
    },
    command_specs=(
        (
            ("cargo", "test", "-p", "agentmage-kernel-contracts", "task_12_2_2_", "--locked"),
            "6 passed; 0 failed",
        ),
        (
            ("cargo", "test", "-p", "agentmage-kernel-engine", "task_12_2_2_", "--locked"),
            "24 passed; 0 failed",
        ),
        (("python3", "-m", "unittest", "tests.test_policy_reference_evidence"), "Ran 4 tests"),
        (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
        (("npm", "run", "docs:mermaid"), "Validated 60 Mermaid block(s)."),
    ),
    claims={
        "focused_case_count": 5,
        "deterministic_fact_field_count": 28,
        "ordered_policy_check_count": 13,
        "advisory_field_count": 10,
        "advisory_disposition_count": 5,
        "incomplete_status_count": 7,
        "prohibited_authority_capability_count": 6,
        "reclassification_content_kind_count": 10,
        "classification_boundary_count": 8,
        "directed_distinct_boundary_pair_count": 56,
        "external_network_used": False,
        "manual_fuzzing_executed": False,
        "release_support": False,
    },
    limitations=(
        "The reference describes completed contracts and focused deterministic tests, not live platform fact collection or production execution.",
        "Typed redaction, narrowing, and isolation transformations remain later owning-sprint integrations.",
        "No private user data or external network operation is used.",
        "Cross-platform packaging, release acceptance, and manual fuzzing remain later tasks and gates.",
    ),
    status="pass-policy-reference",
    task_ids=("12.2.3.2",),
    label="Task 12.2.3.2 policy reference",
)


if __name__ == "__main__":
    raise SystemExit(run_cli(SPEC))
