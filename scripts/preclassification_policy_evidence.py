#!/usr/bin/env python3
"""Task 12.2.2.2 preclassification-policy evidence specification."""

from pathlib import Path

try:
    from scripts.revision_evidence import RevisionEvidenceSpec, run_cli
except ModuleNotFoundError:
    from revision_evidence import RevisionEvidenceSpec, run_cli

ROOT = Path(__file__).resolve().parents[1]
SPEC = RevisionEvidenceSpec(
    root=ROOT,
    report_path=ROOT / "artifacts/sprints/sprint-12/story-12.2/preclassification-policy.json",
    artifact_id="task-12-2-2-2-preclassification-policy",
    case_pattern=r"`(PRE-\d{2})`",
    case_ids=tuple(f"PRE-{index:02}" for index in range(1, 7)),
    source_paths=(
        "docs/verification/task-12-2-2-2-preclassification-policy-results.md",
        "kernel/contracts/src/classification.rs",
        "kernel/engine/src/preclassification_policy.rs",
        "kernel/engine/src/lib.rs",
        "scripts/revision_evidence.py",
        "scripts/preclassification_policy_evidence.py",
        "tests/test_preclassification_policy_evidence.py",
    ),
    document_fragments=(
        "Pass for deterministic static and typed-fact checks before semantic classification.",
        "four static checks followed by nine typed current-fact checks",
        "Invoke no semantic classifier when any deterministic check denies",
        "The clearance is not authority.",
        "manual fuzzing",
    ),
    source_fragments={
        "kernel/contracts/src/classification.rs": ("pub struct DeterministicPolicyFacts",),
        "kernel/engine/src/preclassification_policy.rs": (
            "const STATIC_CHECK_ORDER: [StaticPolicyCheckKind; 4]",
            "pub struct PreclassificationClearance",
            "pub struct PreclassificationPolicyGate",
            "pub fn evaluate(",
        ),
        "kernel/engine/src/lib.rs": ("pub mod preclassification_policy;",),
    },
    command_specs=(
        (("cargo", "test", "-p", "agentmage-kernel-engine", "task_12_2_2_2", "--locked"), "6 passed; 0 failed"),
        (("cargo", "clippy", "-p", "agentmage-kernel-contracts", "-p", "agentmage-kernel-engine", "--all-targets", "--locked", "--", "-D", "warnings"), "Finished `dev` profile"),
        (("python3", "-m", "unittest", "tests.test_preclassification_policy_evidence"), "Ran 4 tests"),
        (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
    ),
    claims={
        "focused_case_count": 6,
        "ordered_static_check_count": 4,
        "ordered_typed_fact_check_count": 9,
        "total_ordered_check_count": 13,
        "deterministic_denial_reason_count": 14,
        "semantic_calls_after_deterministic_denial": 0,
        "clearance_public_constructors": 0,
        "clearance_authority_methods": 0,
        "model_executions": 0,
        "classifier_executions": 0,
        "tool_executions": 0,
        "platform_executions": 0,
        "external_network_used": False,
        "manual_fuzzing_executed": False,
        "release_support": False,
    },
    limitations=(
        "This task evaluates typed supplied facts; platform-specific secret, path, executable-content, destination, repository, credential, and network collectors remain later adapter work.",
        "The clearance is not authority. Advisory restriction and escalation enforcement begins in Task 12.2.2.3.",
        "Tests use synthetic content-free facts with no model, classifier, tool, worker, platform effect, private user data, or external network operation.",
        "Cross-platform execution, packaging, release acceptance, and manual fuzzing remain later gates.",
    ),
    status="pass-preclassification-policy",
    task_ids=("12.2.2.2",),
    label="Task 12.2.2.2 preclassification policy",
)


if __name__ == "__main__":
    raise SystemExit(run_cli(SPEC))
