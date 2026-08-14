#!/usr/bin/env python3
"""Story 12.3 D027-S12-SCOPE evidence specification."""

from pathlib import Path

try:
    from scripts.revision_evidence import RevisionEvidenceSpec, run_cli
except ModuleNotFoundError:
    from revision_evidence import RevisionEvidenceSpec, run_cli


ROOT = Path(__file__).resolve().parents[1]
SPEC = RevisionEvidenceSpec(
    root=ROOT,
    report_path=ROOT / "artifacts/sprints/sprint-12/story-12.3/d027-s12-scope.json",
    artifact_id="story-12-3-d027-s12-scope",
    case_pattern=r"`(D027-SCOPE-\d{2})`",
    case_ids=tuple(f"D027-SCOPE-{index:02}" for index in range(1, 13)),
    source_paths=(
        "docs/verification/task-12-3-d027-scope-results.md",
        "requirements/planning-scope-decisions.json",
        "scripts/planning_scope.py",
        "scripts/status_model.py",
        "scripts/validate_docs.py",
        "scripts/verify_clean_traceability.py",
        "scripts/d027_s12_scope_evidence.py",
        "tests/test_planning_scope.py",
        "tests/test_status_model.py",
        "tests/test_clean_traceability.py",
        "tests/test_traceability_report.py",
        "tests/test_d027_s12_scope_evidence.py",
    ),
    document_fragments=(
        "Pass for `D027-S12-SCOPE`",
        "241 stable requirement / 30 normative",
        "current normative-map result to **31**",
        "Reject mutation, deletion, duplication, reordering, and renumbering",
        "Manual fuzzing remains deferred",
    ),
    source_fragments={
        "scripts/planning_scope.py": (
            "EXPECTED_DECISIONS: Final = (\"ADR-0027\", \"ADR-0040\")",
            "current requirement identities do not match accepted decisions",
            "current normative mappings do not match the ordered post-0040 snapshot",
            "required negative-control registry is missing, reordered, or changed",
        ),
        "scripts/status_model.py": (
            "current model set must preserve evaluated historical candidates",
            "must remain disabled before admission",
        ),
        "scripts/verify_clean_traceability.py": (
            '("python3", "scripts/planning_scope.py"),',
        ),
        "tests/test_planning_scope.py": (
            "test_preserved_requirement_mutation_is_rejected",
            "test_unapproved_addition_is_rejected_even_when_reported_count_is_edited",
            "test_corrupt_supersession_boundary_is_rejected_precisely",
            "test_corrupt_generated_registry_linkage_is_rejected_precisely",
        ),
    },
    command_specs=(
        (("python3", "scripts/planning_scope.py"), "Decision 0027 baseline 241/30"),
        (("python3", "-m", "unittest", "tests.test_planning_scope"), "Ran 14 tests"),
        (
            (
                "python3",
                "-m",
                "unittest",
                "tests.test_status_model",
                "tests.test_documentation_controls",
                "tests.test_product_ci",
                "tests.test_clean_traceability",
                "tests.test_traceability_report",
            ),
            "OK",
        ),
        (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
        (("npm", "run", "docs:mermaid"), "Validated"),
        (("python3", "scripts/validate_docs.py"), "all policy invariants"),
        (("python3", "scripts/requirement_registry.py", "--check"), "Requirement registry is current"),
        (("python3", "scripts/additions_only.py"), "Additions-only baseline passed"),
        (("python3", "scripts/requirement_coverage.py"), "Requirement coverage passed"),
        (("npm", "run", "schemas:check"), "tests 3"),
        (("python3", "scripts/policy_expectations.py", "--check"), "Policy expectations are current"),
        (("python3", "scripts/traceability_report.py", "--check"), "Traceability report is current"),
        (("python3", "scripts/verify_clean_traceability.py"), "Verified 31 normative statements"),
        (("python3", "-m", "unittest", "tests.test_d027_s12_scope_evidence"), "Ran 4 tests"),
    ),
    claims={
        "pre_0027_stable_requirement_count": 229,
        "pre_0027_normative_mapping_count": 26,
        "decision_0027_stable_requirement_count": 241,
        "decision_0027_normative_mapping_count": 30,
        "current_stable_requirement_count": 241,
        "current_normative_mapping_count": 31,
        "epic_count": 17,
        "sprint_count": 169,
        "preserved_requirement_count": 229,
        "appended_requirement_count": 12,
        "negative_control_count": 11,
        "negative_control_failure_count": 11,
        "external_network_used": False,
        "manual_fuzzing_executed": False,
        "release_support": False,
    },
    limitations=(
        "This campaign proves planning-integrity behavior and does not enable a model or complete a product capability.",
        "Decision 0040's current 31st mapping remains separate from the immutable Decision 0027 30-mapping result.",
        "Manual fuzzing remains deferred to its separately approved final campaign.",
    ),
    status="pass-d027-s12-scope",
    task_ids=("12.3.1", "12.3.2"),
    label="Story 12.3 D027-S12-SCOPE",
)


if __name__ == "__main__":
    raise SystemExit(run_cli(SPEC))
