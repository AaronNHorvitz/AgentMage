#!/usr/bin/env python3
"""Task 12.2.1.5 restart-reconciliation evidence specification."""

from pathlib import Path

try:
    from scripts.revision_evidence import RevisionEvidenceSpec, run_cli
except ModuleNotFoundError:
    from revision_evidence import RevisionEvidenceSpec, run_cli

ROOT = Path(__file__).resolve().parents[1]
SPEC = RevisionEvidenceSpec(
    root=ROOT,
    report_path=ROOT / "artifacts/sprints/sprint-12/story-12.2/restart-reconciliation.json",
    artifact_id="task-12-2-1-5-restart-reconciliation",
    case_pattern=r"`(RST-\d{2})`",
    case_ids=tuple(f"RST-{index:02}" for index in range(1, 7)),
    source_paths=(
        "docs/verification/task-12-2-1-5-restart-reconciliation-results.md",
        "kernel/contracts/src/agent_restart.rs",
        "kernel/contracts/src/lib.rs",
        "kernel/contracts/src/serialization.rs",
        "kernel/engine/src/agent_restart.rs",
        "kernel/engine/src/agent_state.rs",
        "kernel/engine/src/lib.rs",
        "scripts/revision_evidence.py",
        "scripts/restart_reconciliation_evidence.py",
        "tests/test_restart_reconciliation_evidence.py",
    ),
    document_fragments=(
        "Pass for exact restart reconciliation before another restored agent-state transition.",
        "Only `RestartReconciler` can construct the opaque, non-cloneable `RestartPermit`.",
        "content-addressed receipt chain",
        "complete crash-before-and-after-every-agent-transition campaign remains Task 12.2.4.4",
        "manual fuzzing",
    ),
    source_fragments={
        "kernel/contracts/src/agent_restart.rs": ("pub struct AgentRestartSnapshot",),
        "kernel/engine/src/agent_restart.rs": (
            "pub struct RestartPermit",
            "pub struct RestartReconciler",
            "pub fn reconcile(",
            "AuthorityTransactionState::LaunchCommitted\n                            | AuthorityTransactionState::Reconciling",
        ),
        "kernel/engine/src/agent_state.rs": (
            "pub fn restore(",
            "pub fn resume_after_restart(",
            "restart_required: true,",
        ),
    },
    command_specs=(
        (("cargo", "test", "-p", "agentmage-kernel-engine", "task_12_2_1_5", "--locked"), "6 passed; 0 failed"),
        (("cargo", "test", "-p", "agentmage-kernel-contracts", "serialization", "--locked"), "5 passed; 0 failed"),
        (("cargo", "clippy", "-p", "agentmage-kernel-contracts", "-p", "agentmage-kernel-engine", "--all-targets", "--locked", "--", "-D", "warnings"), "Finished `dev` profile"),
        (("python3", "-m", "unittest", "tests.test_restart_reconciliation_evidence"), "Ran 4 tests"),
        (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
    ),
    claims={
        "focused_case_count": 6,
        "bound_context_field_count": 8,
        "maximum_authority_transactions": 1024,
        "maximum_grants": 2048,
        "maximum_receipts": 4096,
        "opaque_permit_public_constructors": 0,
        "opaque_permit_clone_implementations": 0,
        "pending_authority_resumes": 0,
        "unresolved_consumed_grant_resumes": 0,
        "uncertain_effect_resumes": 0,
        "model_executions": 0,
        "classifier_executions": 0,
        "tool_executions": 0,
        "platform_executions": 0,
        "external_network_used": False,
        "manual_fuzzing_executed": False,
        "release_support": False,
    },
    limitations=(
        "This proves the in-process agent restart gate over typed persisted facts; wiring the snapshot to the encrypted operational store remains a later integration task.",
        "Existing durable authority recovery resolves interrupted effect transactions before this gate, but the complete crash-before-and-after-every-agent-transition campaign remains Task 12.2.4.4.",
        "Tests use synthetic authority and receipt records with no model, classifier, tool, worker, platform effect, private user data, or external network operation.",
        "Cross-platform execution, packaging, release acceptance, and manual fuzzing remain later gates.",
    ),
    status="pass-restart-reconciliation-gate",
    task_ids=("12.2.1.5",),
    label="Task 12.2.1.5 restart reconciliation",
)


if __name__ == "__main__":
    raise SystemExit(run_cli(SPEC))
