#!/usr/bin/env python3
"""Task 12.2.4.2 D027-S12-POLICY evidence specification."""

from pathlib import Path

try:
    from scripts.revision_evidence import RevisionEvidenceSpec, run_cli
except ModuleNotFoundError:
    from revision_evidence import RevisionEvidenceSpec, run_cli

ROOT = Path(__file__).resolve().parents[1]
SPEC = RevisionEvidenceSpec(
    root=ROOT,
    report_path=ROOT / "artifacts/sprints/sprint-12/story-12.2/d027-s12-policy.json",
    artifact_id="task-12-2-4-2-d027-s12-policy",
    case_pattern=r"`(D027-POLICY-\d{2})`",
    case_ids=tuple(f"D027-POLICY-{index:02}" for index in range(1, 5)),
    source_paths=(
        "docs/verification/task-12-2-4-2-d027-policy-results.md",
        "fixtures/agent-policy/v1/d027-policy-campaign.json",
        "kernel/engine/src/preclassification_policy.rs",
        "scripts/revision_evidence.py",
        "scripts/d027_s12_policy_evidence.py",
        "tests/test_d027_s12_policy_evidence.py",
    ),
    document_fragments=(
        "Pass for `D027-S12-POLICY`; 5,120 seeded fact mutations produced zero decision drift.",
        "exactly 320 mutations in each of 16 required dimensions",
        "identical clearance or first denial",
        "at least one clearance and one denial",
    ),
    source_fragments={
        "fixtures/agent-policy/v1/d027-policy-campaign.json": (
            '"campaign_id": "D027-S12-POLICY"',
            '"mutation_count": 5120',
            '"mutations_per_dimension": 320',
            '"required_outcome": "identical_clearance_or_first_denial"',
        ),
        "kernel/engine/src/preclassification_policy.rs": (
            "d027_s12_policy_5120_seeded_fact_mutations_are_deterministic",
            "const DIMENSION_COUNT: usize = 16;",
            "const MUTATIONS_PER_DIMENSION: usize = 320;",
            "let mut seed = 0xd027_5120_5eed_u64;",
            'assert_eq!(first, repeated, "decision drift at mutation {index}");',
            "assert!(clearance_count > 0);",
            "assert!(denial_count > 0);",
        ),
    },
    command_specs=(
        (("cargo", "test", "-p", "agentmage-kernel-engine", "d027_s12_policy_", "--locked"), "1 passed; 0 failed"),
        (
            (
                "cargo",
                "clippy",
                "-p",
                "agentmage-kernel-engine",
                "--all-targets",
                "--locked",
                "--",
                "-D",
                "warnings",
            ),
            "Finished `dev` profile",
        ),
        (("python3", "-m", "unittest", "tests.test_d027_s12_policy_evidence"), "Ran 4 tests"),
        (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
    ),
    claims={
        "focused_case_count": 4,
        "seed_decimal": 228867283377901,
        "mutation_count": 5120,
        "dimension_count": 16,
        "mutations_per_dimension": 320,
        "policy_evaluation_count": 10240,
        "clearance_paths_present": True,
        "denial_paths_present": True,
        "decision_drift_count": 0,
        "semantic_classifier_calls": 0,
        "external_network_used": False,
        "manual_fuzzing_executed": False,
        "release_support": False,
    },
    limitations=(
        "The campaign uses deterministic synthetic fact records and does not collect live platform facts or invoke a semantic classifier.",
        "It proves in-process policy reproducibility, not production execution, provider integration, or cross-platform packaging.",
        "No private user data or external network operation is used.",
        "Release acceptance and manual fuzzing remain later tasks and gates.",
    ),
    status="pass-d027-s12-policy",
    task_ids=("12.2.4.2",),
    label="Task 12.2.4.2 D027-S12-POLICY",
)


if __name__ == "__main__":
    raise SystemExit(run_cli(SPEC))
