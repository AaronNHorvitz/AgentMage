#!/usr/bin/env python3
"""Task 12.2.4.3 D027-S12-CLASSIFIER evidence specification."""

from pathlib import Path

try:
    from scripts.revision_evidence import RevisionEvidenceSpec, run_cli
except ModuleNotFoundError:
    from revision_evidence import RevisionEvidenceSpec, run_cli

ROOT = Path(__file__).resolve().parents[1]
SPEC = RevisionEvidenceSpec(
    root=ROOT,
    report_path=ROOT / "artifacts/sprints/sprint-12/story-12.2/d027-s12-classifier.json",
    artifact_id="task-12-2-4-3-d027-s12-classifier",
    case_pattern=r"`(D027-CLASS-\d{2})`",
    case_ids=tuple(f"D027-CLASS-{index:02}" for index in range(1, 6)),
    source_paths=(
        "docs/verification/task-12-2-4-3-d027-classifier-results.md",
        "fixtures/agent-policy/v1/d027-classifier-campaign.json",
        "kernel/contracts/src/classification.rs",
        "kernel/engine/src/advisory_policy.rs",
        "scripts/revision_evidence.py",
        "scripts/d027_s12_classifier_evidence.py",
        "tests/test_d027_s12_classifier_evidence.py",
    ),
    document_fragments=(
        "Pass for `D027-S12-CLASSIFIER`; 1,280 advisory outputs produced zero broader authority or completion.",
        "128 outputs in each of ten hostile or valid classes",
        "all seven incomplete statuses",
        "Reject grant, operation, destination, model, execution, and completion fields",
        "Preserve only deny, narrow, redact, isolate, and user escalation",
    ),
    source_fragments={
        "fixtures/agent-policy/v1/d027-classifier-campaign.json": (
            '"campaign_id": "D027-S12-CLASSIFIER"',
            '"output_count": 1280',
            '"outputs_per_class": 128',
            '"broader_authority": 0',
            '"completion": 0',
        ),
        "kernel/contracts/src/classification.rs": (
            "pub struct AdvisoryClassifierResult",
            "pub enum AdvisoryClassifierDisposition",
        ),
        "kernel/engine/src/advisory_policy.rs": (
            "d027_s12_classifier_1280_outputs_never_broaden_authority_or_complete",
            "const OUTPUT_CLASS_COUNT: usize = 10;",
            "const OUTPUTS_PER_CLASS: usize = 128;",
            "assert_eq!(parser_rejections, 256);",
            "assert_eq!(failure_decisions, 896);",
            "assert_eq!(restrictive_decisions, 128);",
        ),
    },
    command_specs=(
        (("cargo", "test", "-p", "agentmage-kernel-engine", "d027_s12_classifier_", "--locked"), "1 passed; 0 failed"),
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
        (("python3", "-m", "unittest", "tests.test_d027_s12_classifier_evidence"), "Ran 4 tests"),
        (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
    ),
    claims={
        "focused_case_count": 5,
        "classifier_output_count": 1280,
        "output_class_count": 10,
        "outputs_per_class": 128,
        "allow_like_parser_rejection_count": 128,
        "authority_field_parser_rejection_count": 128,
        "safe_failure_decision_count": 896,
        "restrictive_decision_count": 128,
        "broader_authority_count": 0,
        "completion_count": 0,
        "model_executions": 0,
        "external_network_used": False,
        "manual_fuzzing_executed": False,
        "release_support": False,
    },
    limitations=(
        "The campaign uses synthetic classifier records and does not run a production model or learned classifier.",
        "It proves the typed in-process advisory boundary, not platform execution, provider integration, or cross-platform packaging.",
        "No private user data or external network operation is used.",
        "Release acceptance and manual fuzzing remain later tasks and gates.",
    ),
    status="pass-d027-s12-classifier",
    task_ids=("12.2.4.3",),
    label="Task 12.2.4.3 D027-S12-CLASSIFIER",
)


if __name__ == "__main__":
    raise SystemExit(run_cli(SPEC))
