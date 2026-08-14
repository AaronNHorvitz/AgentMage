#!/usr/bin/env python3
"""Task 12.2.2.3 advisory-policy evidence specification."""

from pathlib import Path

try:
    from scripts.revision_evidence import RevisionEvidenceSpec, run_cli
except ModuleNotFoundError:
    from revision_evidence import RevisionEvidenceSpec, run_cli

ROOT = Path(__file__).resolve().parents[1]
SPEC = RevisionEvidenceSpec(
    root=ROOT,
    report_path=ROOT / "artifacts/sprints/sprint-12/story-12.2/advisory-policy.json",
    artifact_id="task-12-2-2-3-advisory-policy",
    case_pattern=r"`(ADV-\d{2})`",
    case_ids=tuple(f"ADV-{index:02}" for index in range(1, 7)),
    source_paths=(
        "docs/verification/task-12-2-2-3-advisory-policy-results.md",
        "kernel/contracts/src/classification.rs",
        "kernel/engine/src/preclassification_policy.rs",
        "kernel/engine/src/advisory_policy.rs",
        "kernel/engine/src/lib.rs",
        "scripts/revision_evidence.py",
        "scripts/advisory_policy_evidence.py",
        "tests/test_advisory_policy_evidence.py",
    ),
    document_fragments=(
        "Pass for authority-reducing advisory classification over exact deterministic clearance.",
        "only `deny`, `narrow`, `redact`, `isolate`, and `escalate`",
        "an empty advisory preserves rather than broadens",
        "has no grant, operation, destination-selection, model-switch, execution, denial-override, or completion method",
        "manual fuzzing",
    ),
    source_fragments={
        "kernel/contracts/src/classification.rs": (
            "pub enum AdvisoryClassifierDisposition",
            "pub struct AdvisoryClassifierResult",
        ),
        "kernel/engine/src/preclassification_policy.rs": ("matches_advisory_identity",),
        "kernel/engine/src/advisory_policy.rs": (
            "pub struct AdvisoryBoundaryDecision",
            "pub struct AdvisoryPolicyGate",
            "fn normalize_dispositions(",
        ),
        "kernel/engine/src/lib.rs": ("pub mod advisory_policy;",),
    },
    command_specs=(
        (("cargo", "test", "-p", "agentmage-kernel-engine", "task_12_2_2_3", "--locked"), "6 passed; 0 failed"),
        (("cargo", "clippy", "-p", "agentmage-kernel-contracts", "-p", "agentmage-kernel-engine", "--all-targets", "--locked", "--", "-D", "warnings"), "Finished `dev` profile"),
        (("python3", "-m", "unittest", "tests.test_advisory_policy_evidence"), "Ran 4 tests"),
        (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
    ),
    claims={
        "focused_case_count": 6,
        "allowed_advisory_disposition_count": 5,
        "incomplete_status_count": 7,
        "prohibited_field_attack_count": 6,
        "classifier_created_grants": 0,
        "classifier_denial_overrides": 0,
        "classifier_destination_selections": 0,
        "classifier_model_switches": 0,
        "classifier_executions": 0,
        "classifier_completions": 0,
        "model_executions": 0,
        "classifier_runtime_executions": 0,
        "tool_executions": 0,
        "platform_executions": 0,
        "external_network_used": False,
        "manual_fuzzing_executed": False,
        "release_support": False,
    },
    limitations=(
        "Task 12.2.2.4 assigns explicit narrower, user-decision, or blocked outcomes to incomplete classifier states; this task only prevents them from changing clearance.",
        "Tool-set subtraction and scope/redaction/isolation materialization remain later typed product-boundary integrations.",
        "Tests use synthetic content-free facts and classifier records with no model, classifier runtime, tool, worker, platform effect, private user data, or external network operation.",
        "Cross-platform execution, packaging, release acceptance, and manual fuzzing remain later gates.",
    ),
    status="pass-advisory-authority-reduction",
    task_ids=("12.2.2.3",),
    label="Task 12.2.2.3 advisory policy",
)


if __name__ == "__main__":
    raise SystemExit(run_cli(SPEC))
