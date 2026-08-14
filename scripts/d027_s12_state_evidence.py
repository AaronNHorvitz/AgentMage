#!/usr/bin/env python3
"""Task 12.2.4.1 D027-S12-STATE evidence specification."""

from pathlib import Path

try:
    from scripts.revision_evidence import RevisionEvidenceSpec, run_cli
except ModuleNotFoundError:
    from revision_evidence import RevisionEvidenceSpec, run_cli

ROOT = Path(__file__).resolve().parents[1]
SPEC = RevisionEvidenceSpec(
    root=ROOT,
    report_path=ROOT / "artifacts/sprints/sprint-12/story-12.2/d027-s12-state.json",
    artifact_id="task-12-2-4-1-d027-s12-state",
    case_pattern=r"`(D027-STATE-\d{2})`",
    case_ids=tuple(f"D027-STATE-{index:02}" for index in range(1, 6)),
    source_paths=(
        "docs/verification/task-12-2-4-1-d027-state-results.md",
        "kernel/engine/src/agent_state.rs",
        "kernel/engine/src/agent_ceiling.rs",
        "kernel/engine/src/agent_proposal.rs",
        "kernel/engine/src/agent_verifier.rs",
        "scripts/revision_evidence.py",
        "scripts/d027_s12_state_evidence.py",
        "tests/test_d027_s12_state_evidence.py",
    ),
    document_fragments=(
        "Pass for `D027-S12-STATE`; no unverified success was admitted.",
        "all 289 pairs as exactly 52 legal and 237 illegal edges",
        "all nine terminal states sticky",
        "Reject exact replay and competing proposal",
        "Reject all five advisory sources for both success dispositions",
    ),
    source_fragments={
        "kernel/engine/src/agent_state.rs": (
            "d027_s12_state_every_graph_pair_and_controller_disposition_is_exact",
            "assert_eq!(legal_edges, 52);",
            "assert_eq!(illegal_edges, 237);",
            "assert_eq!(ordinary_admissions, 48);",
            "assert_eq!(verifier_gated, 34);",
        ),
        "kernel/engine/src/agent_ceiling.rs": (
            "d027_s12_state_every_ceiling_has_one_sticky_terminal_result",
        ),
        "kernel/engine/src/agent_proposal.rs": (
            "d027_s12_state_duplicate_and_competing_proposals_never_replace_admission",
        ),
        "kernel/engine/src/agent_verifier.rs": (
            "d027_s12_state_false_completion_sources_produce_no_verified_success",
        ),
    },
    command_specs=(
        (("cargo", "test", "-p", "agentmage-kernel-engine", "d027_s12_state_", "--locked"), "4 passed; 0 failed"),
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
        (("python3", "-m", "unittest", "tests.test_d027_s12_state_evidence"), "Ran 4 tests"),
        (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
    ),
    claims={
        "focused_case_count": 5,
        "state_count": 17,
        "ordered_state_pair_count": 289,
        "legal_edge_count": 52,
        "illegal_edge_count": 237,
        "ordinary_admission_count": 48,
        "ordinary_verifier_gated_count": 34,
        "terminal_state_count": 9,
        "ceiling_count": 13,
        "proposal_replay_admissions": 0,
        "competing_proposal_admissions": 0,
        "advisory_completion_candidate_count": 10,
        "unverified_success_count": 0,
        "external_network_used": False,
        "manual_fuzzing_executed": False,
        "release_support": False,
    },
    limitations=(
        "This campaign proves the in-process deterministic state boundary; encrypted persistence wiring and the full crash-before-and-after campaign remain Task 12.2.4.4.",
        "Tests use synthetic identifiers and records with no production model, classifier, tool, worker, private user data, or external network operation.",
        "Cross-platform execution, packaging, release acceptance, and manual fuzzing remain later tasks and gates.",
    ),
    status="pass-d027-s12-state",
    task_ids=("12.2.4.1",),
    label="Task 12.2.4.1 D027-S12-STATE",
)


if __name__ == "__main__":
    raise SystemExit(run_cli(SPEC))
