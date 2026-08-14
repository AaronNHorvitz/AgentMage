#!/usr/bin/env python3
"""Task 12.2.1.2 proposal-identity evidence specification."""

from pathlib import Path

try:
    from scripts.revision_evidence import RevisionEvidenceSpec, run_cli
except ModuleNotFoundError:
    from revision_evidence import RevisionEvidenceSpec, run_cli

ROOT = Path(__file__).resolve().parents[1]
SPEC = RevisionEvidenceSpec(
    root=ROOT,
    report_path=ROOT / "artifacts/sprints/sprint-12/story-12.2/proposal-identity.json",
    artifact_id="task-12-2-1-2-proposal-identity",
    case_pattern=r"`(PIV-\d{2})`",
    case_ids=tuple(f"PIV-{index:02}" for index in range(1, 7)),
    source_paths=(
        "docs/verification/task-12-2-1-2-proposal-identity-results.md",
        "kernel/contracts/src/agent_proposal.rs",
        "kernel/contracts/src/ids.rs",
        "kernel/contracts/src/serialization.rs",
        "kernel/engine/src/agent_proposal.rs",
        "scripts/revision_evidence.py",
        "scripts/proposal_identity_evidence.py",
        "tests/test_proposal_identity_evidence.py",
    ),
    document_fragments=(
        "Pass for exact proposal identity binding and inert rejection.",
        "each ownership, model/context, evaluation-context, and correlation substitution",
        "at most one exact candidate",
        "cannot represent an operation",
        "manual fuzzing",
    ),
    source_fragments={
        "kernel/contracts/src/agent_proposal.rs": ("pub struct AgentProposal", "pub proposal_sha256: String"),
        "kernel/engine/src/agent_proposal.rs": ("pub struct ProposalAdmissionRegistry", "pub fn admit"),
        "kernel/contracts/src/serialization.rs": ("crate::AgentProposal,",),
    },
    command_specs=(
        (("cargo", "test", "-p", "agentmage-kernel-engine", "task_12_2_1_2", "--locked"), "6 passed; 0 failed"),
        (("cargo", "test", "-p", "agentmage-kernel-contracts", "serialization", "--locked"), "5 passed; 0 failed"),
        (("cargo", "clippy", "-p", "agentmage-kernel-contracts", "-p", "agentmage-kernel-engine", "--all-targets", "--locked", "--", "-D", "warnings"), "Finished `dev` profile"),
        (("python3", "-m", "unittest", "tests.test_proposal_identity_evidence"), "Ran 4 tests"),
        (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
    ),
    claims={
        "focused_case_count": 6,
        "bound_identity_count": 11,
        "identity_substitution_count": 8,
        "parser_rejection_class_count": 5,
        "turn_rejection_count": 2,
        "maximum_admitted_proposals_per_turn": 1,
        "replay_admissions": 0,
        "competing_proposal_admissions": 0,
        "authority_fields": 0,
        "model_executions": 0,
        "tool_executions": 0,
        "platform_executions": 0,
        "external_network_used": False,
        "manual_fuzzing_executed": False,
        "release_support": False,
    },
    limitations=(
        "The proposal digest identifies separately validated closed bytes; the model codec and payload validator remain Sprint 13 work.",
        "Durable proposal persistence and restart reconciliation remain later Story 12.2 tasks.",
        "No model, tool, grant, worker, platform effect, private user data, or external network operation is used.",
        "Cross-platform execution, packaging, release acceptance, and manual fuzzing remain later gates.",
    ),
    status="pass-exact-proposal-identity-boundary",
    task_ids=("12.2.1.2",),
    label="Task 12.2.1.2 proposal identity",
)


if __name__ == "__main__":
    raise SystemExit(run_cli(SPEC))
