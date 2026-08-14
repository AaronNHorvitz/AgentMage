#!/usr/bin/env python3
"""Task 12.2.2.5 reclassification evidence specification."""

from pathlib import Path

try:
    from scripts.revision_evidence import RevisionEvidenceSpec, run_cli
except ModuleNotFoundError:
    from revision_evidence import RevisionEvidenceSpec, run_cli

ROOT = Path(__file__).resolve().parents[1]
SPEC = RevisionEvidenceSpec(
    root=ROOT,
    report_path=ROOT / "artifacts/sprints/sprint-12/story-12.2/reclassification.json",
    artifact_id="task-12-2-2-5-reclassification",
    case_pattern=r"`(RCL-\d{2})`",
    case_ids=tuple(f"RCL-{index:02}" for index in range(1, 7)),
    source_paths=(
        "docs/verification/task-12-2-2-5-reclassification-results.md",
        "kernel/contracts/src/classification.rs",
        "kernel/contracts/src/lib.rs",
        "kernel/contracts/src/serialization.rs",
        "kernel/engine/src/preclassification_policy.rs",
        "kernel/engine/src/advisory_policy.rs",
        "kernel/engine/src/reclassification.rs",
        "kernel/engine/src/lib.rs",
        "scripts/revision_evidence.py",
        "scripts/reclassification_evidence.py",
        "tests/test_reclassification_evidence.py",
    ),
    document_fragments=(
        "Pass for fresh classification before every represented successive trust boundary.",
        "all ten named content classes",
        "all 56 directed pairs among eight distinct boundaries",
        "Only an opaque non-cloneable permit",
        "manual fuzzing",
    ),
    source_fragments={
        "kernel/contracts/src/classification.rs": (
            "pub enum ReclassificationContentKind",
            "pub enum ClassificationBoundary",
            "pub struct ReclassificationRequest",
        ),
        "kernel/engine/src/preclassification_policy.rs": ("matches_reclassification_identity",),
        "kernel/engine/src/reclassification.rs": (
            "pub struct ReclassificationPermit",
            "pub struct ReclassificationGate",
            "pub fn reclassify(",
            "pub fn cross_boundary(",
        ),
        "kernel/engine/src/lib.rs": ("pub mod reclassification;",),
    },
    command_specs=(
        (("cargo", "test", "-p", "agentmage-kernel-engine", "task_12_2_2_5", "--locked"), "6 passed; 0 failed"),
        (("cargo", "clippy", "-p", "agentmage-kernel-contracts", "-p", "agentmage-kernel-engine", "--all-targets", "--locked", "--", "-D", "warnings"), "Finished `dev` profile"),
        (("python3", "-m", "unittest", "tests.test_reclassification_evidence"), "Ran 4 tests"),
        (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
    ),
    claims={
        "focused_case_count": 6,
        "reclassification_content_kind_count": 10,
        "trust_boundary_count": 8,
        "directed_distinct_boundary_pair_count": 56,
        "allowed_crossings_per_permit": 1,
        "advisory_restriction_count": 5,
        "restricted_network_crossings": 0,
        "model_executions": 0,
        "classifier_runtime_executions": 0,
        "tool_executions": 0,
        "platform_executions": 0,
        "external_network_used": False,
        "manual_fuzzing_executed": False,
        "release_support": False,
    },
    limitations=(
        "This task proves the kernel reclassification contract with typed synthetic observations; production adapters must supply current content and revision evidence at their owning later sprints.",
        "Materializing redacted, narrowed, or isolated replacement content requires a new observation and remains later product integration.",
        "No model, classifier runtime, tool, worker, platform effect, private user data, or external network operation is used.",
        "Cross-platform execution, packaging, release acceptance, and manual fuzzing remain later gates.",
    ),
    status="pass-continuous-reclassification",
    task_ids=("12.2.2.5",),
    label="Task 12.2.2.5 reclassification",
)


if __name__ == "__main__":
    raise SystemExit(run_cli(SPEC))
