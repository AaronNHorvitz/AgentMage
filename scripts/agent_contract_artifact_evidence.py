#!/usr/bin/env python3
"""Task 12.2.3.1 agent-contract-artifact evidence specification."""

from pathlib import Path

try:
    from scripts.revision_evidence import RevisionEvidenceSpec, run_cli
except ModuleNotFoundError:
    from revision_evidence import RevisionEvidenceSpec, run_cli

ROOT = Path(__file__).resolve().parents[1]
SPEC = RevisionEvidenceSpec(
    root=ROOT,
    report_path=ROOT / "artifacts/sprints/sprint-12/story-12.2/agent-contract-reference.json",
    artifact_id="task-12-2-3-1-agent-contract-reference",
    case_pattern=r"`(ART-\d{2})`",
    case_ids=tuple(f"ART-{index:02}" for index in range(1, 6)),
    source_paths=(
        "docs/verification/task-12-2-3-1-agent-contract-artifacts-results.md",
        "docs/architecture/agent-state-contract-reference.md",
        "kernel/contracts/src/agent_state.rs",
        "kernel/contracts/src/agent_proposal.rs",
        "kernel/contracts/src/agent_restart.rs",
        "kernel/contracts/src/agent_verifier.rs",
        "kernel/engine/src/agent_ceiling.rs",
        "kernel/engine/src/agent_state.rs",
        "kernel/engine/src/agent_verifier.rs",
        "scripts/revision_evidence.py",
        "scripts/agent_contract_artifact_evidence.py",
        "tests/test_agent_contract_artifact_evidence.py",
    ),
    document_fragments=(
        "Pass for the versioned agent-state, proposal, ceiling, verifier, and restart contract reference.",
        "exactly eight active and nine terminal states",
        "exactly 52 legal edges",
        "verifier-only success",
        "Manual fuzzing",
    ),
    source_fragments={
        "docs/architecture/agent-state-contract-reference.md": (
            "## Legal Transition Graph",
            "**17 states**, **289 ordered state pairs**, and **52 legal edges**",
            "## Proposal Identity",
            "## Completion Verification",
            "## Restart Reconciliation",
        ),
        "kernel/contracts/src/agent_state.rs": ("pub enum AgentStateKind",),
        "kernel/engine/src/agent_state.rs": ("pub const fn legal_transition",),
        "kernel/engine/src/agent_verifier.rs": ("pub struct VerifierRegistry",),
    },
    command_specs=(
        (("cargo", "test", "-p", "agentmage-kernel-engine", "task_12_2_1_", "--locked"), "30 passed; 0 failed"),
        (("python3", "-m", "unittest", "tests.test_agent_contract_artifact_evidence"), "Ran 4 tests"),
        (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
        (("npm", "run", "docs:mermaid"), "Validated 59 Mermaid block(s)."),
    ),
    claims={
        "focused_case_count": 5,
        "active_state_count": 8,
        "terminal_state_count": 9,
        "total_state_count": 17,
        "ordered_state_pair_count": 289,
        "legal_transition_count": 52,
        "runtime_ceiling_count": 13,
        "proposal_identity_field_count": 12,
        "ordinary_success_routes": 0,
        "external_network_used": False,
        "manual_fuzzing_executed": False,
        "release_support": False,
    },
    limitations=(
        "The document describes completed contracts and focused deterministic tests, not full product integration or release support.",
        "Encrypted agent-state persistence, production workers, live UI, and cross-platform packaging remain later tasks.",
        "No private user data or external network operation is used.",
        "Manual fuzzing remains deferred to its assigned final gate.",
    ),
    status="pass-agent-contract-reference",
    task_ids=("12.2.3.1",),
    label="Task 12.2.3.1 agent contract artifacts",
)


if __name__ == "__main__":
    raise SystemExit(run_cli(SPEC))
