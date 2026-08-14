#!/usr/bin/env python3
"""Task 12.2.2.4 classifier-failure evidence specification."""

from pathlib import Path

try:
    from scripts.revision_evidence import RevisionEvidenceSpec, run_cli
except ModuleNotFoundError:
    from revision_evidence import RevisionEvidenceSpec, run_cli

ROOT = Path(__file__).resolve().parents[1]
SPEC = RevisionEvidenceSpec(
    root=ROOT,
    report_path=ROOT / "artifacts/sprints/sprint-12/story-12.2/classifier-failure.json",
    artifact_id="task-12-2-2-4-classifier-failure",
    case_pattern=r"`(CFL-\d{2})`",
    case_ids=tuple(f"CFL-{index:02}" for index in range(1, 7)),
    source_paths=(
        "docs/verification/task-12-2-2-4-classifier-failure-results.md",
        "kernel/contracts/src/classification.rs",
        "kernel/engine/src/preclassification_policy.rs",
        "kernel/engine/src/advisory_policy.rs",
        "scripts/revision_evidence.py",
        "scripts/classifier_failure_evidence.py",
        "tests/test_classifier_failure_evidence.py",
    ),
    document_fragments=(
        "Pass for deterministic narrower, user-decision, isolation, or blocked classifier failure handling.",
        "all seven incomplete states",
        "Truncated, unavailable, timed-out, and malformed output",
        "Classifier-proposed dispositions are ignored by failure mapping",
        "manual fuzzing",
    ),
    source_fragments={
        "kernel/contracts/src/classification.rs": ("pub enum AdvisoryClassifierStatus",),
        "kernel/engine/src/preclassification_policy.rs": ("pub struct PreclassificationClearance",),
        "kernel/engine/src/advisory_policy.rs": (
            "pub enum ClassifierFailureAction",
            "pub struct ClassifierFailureDecision",
            "pub fn map_failure(",
        ),
    },
    command_specs=(
        (("cargo", "test", "-p", "agentmage-kernel-engine", "task_12_2_2_4", "--locked"), "6 passed; 0 failed"),
        (("cargo", "clippy", "-p", "agentmage-kernel-contracts", "-p", "agentmage-kernel-engine", "--all-targets", "--locked", "--", "-D", "warnings"), "Finished `dev` profile"),
        (("python3", "-m", "unittest", "tests.test_classifier_failure_evidence"), "Ran 4 tests"),
        (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
    ),
    claims={
        "focused_case_count": 6,
        "mapped_incomplete_status_count": 7,
        "narrow_status_count": 1,
        "isolate_status_count": 1,
        "user_decision_status_count": 1,
        "blocked_status_count": 4,
        "valid_confidence_minimum": 0,
        "valid_confidence_maximum": 10000,
        "classifier_failure_authority_methods": 0,
        "model_executions": 0,
        "classifier_runtime_executions": 0,
        "tool_executions": 0,
        "platform_executions": 0,
        "external_network_used": False,
        "manual_fuzzing_executed": False,
        "release_support": False,
    },
    limitations=(
        "This is a typed decision contract; live UI presentation and transition into persisted BLOCKED or user-decision workflow states remain later integration work.",
        "Task 12.2.2.5 adds reclassification triggers before successive trust boundaries.",
        "Tests use synthetic content-free facts and classifier records with no model, classifier runtime, tool, worker, platform effect, private user data, or external network operation.",
        "Cross-platform execution, packaging, release acceptance, and manual fuzzing remain later gates.",
    ),
    status="pass-classifier-failure-mapping",
    task_ids=("12.2.2.4",),
    label="Task 12.2.2.4 classifier failure",
)


if __name__ == "__main__":
    raise SystemExit(run_cli(SPEC))
