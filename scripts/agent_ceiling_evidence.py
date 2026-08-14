#!/usr/bin/env python3
"""Task 12.2.1.3 agent-ceiling evidence specification."""

from pathlib import Path

try:
    from scripts.revision_evidence import RevisionEvidenceSpec, run_cli
except ModuleNotFoundError:
    from revision_evidence import RevisionEvidenceSpec, run_cli

ROOT = Path(__file__).resolve().parents[1]
SPEC = RevisionEvidenceSpec(
    root=ROOT,
    report_path=ROOT / "artifacts/sprints/sprint-12/story-12.2/agent-ceilings.json",
    artifact_id="task-12-2-1-3-agent-ceilings",
    case_pattern=r"`(ACL-\d{2})`",
    case_ids=tuple(f"ACL-{index:02}" for index in range(1, 7)),
    source_paths=(
        "docs/verification/task-12-2-1-3-agent-ceiling-results.md",
        "kernel/engine/src/agent_ceiling.rs",
        "kernel/engine/src/lib.rs",
        "scripts/revision_evidence.py",
        "scripts/agent_ceiling_evidence.py",
        "tests/test_agent_ceiling_evidence.py",
    ),
    document_fragments=(
        "Pass for complete deterministic agent ceilings and one sticky terminal result.",
        "all 13 resources",
        "closed 13-resource set",
        "one exact terminal breach",
        "manual fuzzing",
    ),
    source_fragments={
        "kernel/engine/src/agent_ceiling.rs": (
            "pub enum AgentCeilingKind",
            "pub struct AgentCeilingController",
            "pub const ALL_AGENT_CEILINGS: [AgentCeilingKind; 13]",
        ),
        "kernel/engine/src/lib.rs": ("pub mod agent_ceiling;",),
    },
    command_specs=(
        (("cargo", "test", "-p", "agentmage-kernel-engine", "task_12_2_1_3", "--locked"), "6 passed; 0 failed"),
        (("cargo", "clippy", "-p", "agentmage-kernel-engine", "--all-targets", "--locked", "--", "-D", "warnings"), "Finished `dev` profile"),
        (("python3", "-m", "unittest", "tests.test_agent_ceiling_evidence"), "Ran 4 tests"),
        (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
    ),
    claims={
        "focused_case_count": 6,
        "ceiling_resource_count": 13,
        "inclusive_boundary_checks": 13,
        "terminal_state_count_per_run": 1,
        "no_progress_terminal": "STALLED",
        "other_ceiling_terminal": "EXHAUSTED",
        "rejected_usage_increments": 0,
        "profile_failure_class_count": 3,
        "model_executions": 0,
        "tool_executions": 0,
        "platform_executions": 0,
        "external_network_used": False,
        "manual_fuzzing_executed": False,
        "release_support": False,
    },
    limitations=(
        "The controller is in-memory; durable state integration and restart reconciliation remain later Story 12.2 tasks.",
        "Platform collectors that supply observed memory, disk, process, and elapsed-time values remain later integration work.",
        "No model, tool, grant, worker, platform effect, private user data, or external network operation is used.",
        "Cross-platform execution, packaging, release acceptance, and manual fuzzing remain later gates.",
    ),
    status="pass-deterministic-agent-ceilings",
    task_ids=("12.2.1.3",),
    label="Task 12.2.1.3 agent ceilings",
)


if __name__ == "__main__":
    raise SystemExit(run_cli(SPEC))
