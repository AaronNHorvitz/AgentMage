#!/usr/bin/env python3
"""Task 12.2.4.4 D027-S12-RESTART evidence specification."""

from pathlib import Path

try:
    from scripts.revision_evidence import RevisionEvidenceSpec, run_cli
except ModuleNotFoundError:
    from revision_evidence import RevisionEvidenceSpec, run_cli

ROOT = Path(__file__).resolve().parents[1]
SPEC = RevisionEvidenceSpec(
    root=ROOT,
    report_path=ROOT / "artifacts/sprints/sprint-12/story-12.2/d027-s12-restart.json",
    artifact_id="task-12-2-4-4-d027-s12-restart",
    case_pattern=r"`(D027-RESTART-\d{2})`",
    case_ids=tuple(f"D027-RESTART-{index:02}" for index in range(1, 7)),
    source_paths=(
        "docs/verification/task-12-2-4-4-d027-restart-results.md",
        "fixtures/agent-policy/v1/d027-restart-campaign.json",
        "kernel/engine/src/agent_restart.rs",
        "kernel/engine/src/agent_verifier.rs",
        "kernel/engine/src/authority_transaction.rs",
        "kernel/engine/src/operational_store.rs",
        "scripts/revision_evidence.py",
        "scripts/d027_s12_restart_evidence.py",
        "tests/test_d027_s12_restart_evidence.py",
    ),
    document_fragments=(
        "Pass for `D027-S12-RESTART`; every declared interruption reconciled with no replay, duplicate effect, duplicate receipt, or false success.",
        "before and after all 52 legal edges",
        "around prepare and grant-consumption persistence",
        "around launch commitment and worker return without relaunch",
        "before publication and after hash-chained terminal receipt",
        "Lose unapplied proofs safely",
    ),
    source_fragments={
        "fixtures/agent-policy/v1/d027-restart-campaign.json": (
            '"campaign_id": "D027-S12-RESTART"',
            '"interruption_positions": 104',
            '"grant_replay": 0',
            '"duplicate_effect": 0',
            '"false_success": 0',
        ),
        "kernel/engine/src/agent_restart.rs": (
            "d027_s12_restart_before_and_after_every_state_edge_reconciles_exactly",
            "assert_eq!(before_interruptions, 52);",
            "assert_eq!(after_interruptions, 52);",
        ),
        "kernel/engine/src/agent_verifier.rs": (
            "d027_s12_restart_verifier_proof_loss_never_creates_false_success",
        ),
        "kernel/engine/src/authority_transaction.rs": (
            "d027_s12_restart_grant_worker_receipt_and_terminal_boundaries_reconcile",
            "fn encrypted_restart_recovery_never_replays_and_publishes_one_receipt() {",
            "fn receipt_sequence_is_hash_chained_and_terminal_recovery_is_idempotent() {",
        ),
        "kernel/engine/src/operational_store.rs": (
            "pub struct DurableAuthorityRuntime",
        ),
    },
    command_specs=(
        (("cargo", "test", "-p", "agentmage-kernel-engine", "d027_s12_restart_", "--locked"), "3 passed; 0 failed"),
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
        (("python3", "-m", "unittest", "tests.test_d027_s12_restart_evidence"), "Ran 4 tests"),
        (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
    ),
    claims={
        "focused_case_count": 6,
        "legal_state_edge_count": 52,
        "state_interruption_position_count": 104,
        "authority_fault_point_count": 6,
        "encrypted_authority_recovery_count": 6,
        "verifier_disposition_count": 2,
        "verifier_interruption_position_count": 4,
        "grant_replay_count": 0,
        "duplicate_effect_count": 0,
        "duplicate_receipt_count": 0,
        "false_success_count": 0,
        "external_network_used": False,
        "manual_fuzzing_executed": False,
        "release_support": False,
    },
    limitations=(
        "Agent-state snapshots are exercised as typed restart records; direct storage of the complete agent snapshot remains a later operational integration.",
        "Authority transaction cases do use the encrypted local operational store, but no production model, provider, platform worker, or external service.",
        "No private user data or external network operation is used.",
        "Cross-platform packaging, release acceptance, and manual fuzzing remain later tasks and gates.",
    ),
    status="pass-d027-s12-restart",
    task_ids=("12.2.4.4",),
    label="Task 12.2.4.4 D027-S12-RESTART",
)


if __name__ == "__main__":
    raise SystemExit(run_cli(SPEC))
