#!/usr/bin/env python3
"""Task 12.2.1.1 persisted agent-state evidence specification."""

from pathlib import Path

try:
    from scripts.revision_evidence import RevisionEvidenceSpec, run_cli
except ModuleNotFoundError:
    from revision_evidence import RevisionEvidenceSpec, run_cli

ROOT = Path(__file__).resolve().parents[1]
SPEC = RevisionEvidenceSpec(
    root=ROOT,
    report_path=ROOT / "artifacts/sprints/sprint-12/story-12.2/agent-state.json",
    artifact_id="task-12-2-1-1-agent-state",
    case_pattern=r"`(AST-\d{2})`",
    case_ids=tuple(f"AST-{index:02}" for index in range(1, 7)),
    source_paths=(
        "docs/verification/task-12-2-1-1-agent-state-results.md",
        "kernel/contracts/src/agent_state.rs",
        "kernel/contracts/src/lib.rs",
        "kernel/engine/src/agent_state.rs",
        "kernel/engine/src/lib.rs",
        "scripts/revision_evidence.py",
        "scripts/agent_state_evidence.py",
        "tests/test_agent_state_evidence.py",
    ),
    document_fragments=(
        "Pass for the closed persisted agent-state and legal-transition contract.",
        "all 289 state pairs",
        "exactly 52 edges",
        "bounded 4,096-transition controller",
        "manual fuzzing",
    ),
    source_fragments={
        "kernel/contracts/src/agent_state.rs": ("pub enum AgentStateKind", "pub struct AgentStateTransition"),
        "kernel/engine/src/agent_state.rs": ("pub struct AgentStateController", "pub const fn legal_transition"),
        "scripts/revision_evidence.py": ("class RevisionEvidenceSpec", "def validate_current"),
    },
    command_specs=(
        (("cargo", "test", "-p", "agentmage-kernel-contracts", "agent_state", "--locked"), "1 passed; 0 failed"),
        (("cargo", "test", "-p", "agentmage-kernel-engine", "task_12_2_1_1", "--locked"), "6 passed; 0 failed"),
        (("cargo", "clippy", "-p", "agentmage-kernel-contracts", "-p", "agentmage-kernel-engine", "--all-targets", "--locked", "--", "-D", "warnings"), "Finished `dev` profile"),
        (("python3", "-m", "unittest", "tests.test_agent_state_evidence"), "Ran 4 tests"),
        (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
    ),
    claims={
        "active_state_count": 8,
        "terminal_state_count": 9,
        "success_state_count": 2,
        "state_pair_count": 289,
        "legal_transition_count": 52,
        "maximum_retained_transitions": 4096,
        "ordinary_success_routes": 0,
        "engine_test_count": 6,
        "contract_test_count": 1,
        "model_executions": 0,
        "tool_executions": 0,
        "platform_executions": 0,
        "external_network_used": False,
        "manual_fuzzing_executed": False,
        "release_support": False,
    },
    limitations=(
        "This task defines in-memory shared contracts and controller behavior; durable store integration begins in later Story 12.2 tasks.",
        "Proposal identity, ceilings, verifier records, restart reconciliation, deterministic policy, and advisory classification remain separate sub-tasks.",
        "No model, tool, grant, worker, platform effect, private user data, or external network operation is used.",
        "Cross-platform execution, packaging, release acceptance, and manual fuzzing remain later gates.",
    ),
    status="pass-agent-state-transition-contract",
    task_ids=("12.2.1.1",),
    label="Task 12.2.1.1 agent state",
)


if __name__ == "__main__":
    raise SystemExit(run_cli(SPEC))
