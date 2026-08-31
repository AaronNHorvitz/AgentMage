#!/usr/bin/env python3
"""Build and validate the local Story 11.3 durable-resume evidence record."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-11/story-11.3"
RAW_PATH: Final = EVIDENCE_DIR / "durable-resume-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "durable-resume-report.json"
COMMANDS: Final = (
    ("cargo", "test", "-p", "agentmage-kernel-engine", "story_11_3", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "workflow_checkpoint_", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "workflow_projection_failure_retains_no_event_checkpoint_cursor_or_state", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "d027_s12_restart_grant_worker_receipt_and_terminal_boundaries_reconcile", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "session_reconstructs_after_supervisor_and_view_disappear", "--locked"),
    ("cargo", "test", "-p", "agentmage-kernel-engine", "story_21_2_process_stop_matrix_preserves_one_truthful_replay", "--locked"),
    ("cargo", "clippy", "-p", "agentmage-kernel-engine", "--all-targets", "--all-features", "--locked", "--", "-D", "warnings"),
)
MARKERS: Final = (
    "story_11_3_current_checkpoint_is_the_only_dispatch_permitting_result ... ok",
    "story_11_3_every_identity_family_invalidates_resume_transitively ... ok",
    "story_11_3_uncertain_or_completed_effect_never_replays ... ok",
    "story_11_3_budget_and_terminal_state_are_absorbing ... ok",
    "story_11_3_malformed_or_mismatched_durable_records_fail_closed ... ok",
    "workflow_checkpoint_reopens_exactly_and_tamper_blocks_restart ... ok",
    "workflow_projection_failure_retains_no_event_checkpoint_cursor_or_state ... ok",
    "d027_s12_restart_grant_worker_receipt_and_terminal_boundaries_reconcile ... ok",
    "session_reconstructs_after_supervisor_and_view_disappear ... ok",
    "story_21_2_process_stop_matrix_preserves_one_truthful_replay ... ok",
    "Finished `dev` profile",
)
SOURCE_MARKERS: Final = {
    "kernel/engine/src/workflow_resume.rs": (
        "pub fn reconcile_workflow_resume(",
        "WorkflowResumeAction::ReconcileUncertainEffect",
        "WorkflowResumeAction::ReplanAndRecheckpoint",
        "WorkflowResumeAction::StopBudgetExhausted",
        '"workflow.resume.effect_uncertain"',
    ),
    "kernel/engine/src/operational_store.rs": (
        "const SCHEMA_VERSION: i64 = 17;",
        "fn persist_workflow_checkpoint(",
        "fn verify_workflow_checkpoints(",
        "fn workflow_checkpoint_reopens_exactly_and_tamper_blocks_restart()",
        "fn workflow_projection_failure_retains_no_event_checkpoint_cursor_or_state()",
    ),
    "kernel/engine/migrations/operational-store/0017-workflow-checkpoints.sql": (
        "CREATE TABLE workflow_checkpoints",
        "FOREIGN KEY(session_checkpoint_id) REFERENCES session_checkpoints(checkpoint_id)",
        "FOREIGN KEY(run_id, event_sequence, event_id)",
        "CHECK(journal_sequence = event_sequence)",
    ),
    "kernel/engine/src/persistent_supervisor.rs": (
        "fn session_reconstructs_after_supervisor_and_view_disappear()",
    ),
    "kernel/engine/src/runtime_journal.rs": (
        "fn story_21_2_process_stop_matrix_preserves_one_truthful_replay()",
    ),
    "kernel/engine/src/authority_transaction.rs": (
        "fn d027_s12_restart_grant_worker_receipt_and_terminal_boundaries_reconcile()",
    ),
}
RETAINED_PATHS: Final = tuple(SOURCE_MARKERS) + (
    "kernel/engine/fixtures/operational-store/schema-17.json",
    "docs/verification/story-11-3-durable-resume.md",
    "scripts/story_11_3_resume_evidence.py",
    "tests/test_story_11_3_resume_evidence.py",
)
TRUTH: Final = {
    "authoritative_workflow_checkpoint_persisted_atomically": True,
    "verified_restart_reconstructs_checkpoint": True,
    "checkpoint_tamper_blocks_restart": True,
    "identity_drift_invalidates_transitively": True,
    "uncertain_or_completed_effect_replayed": False,
    "exhausted_budget_dispatch_permitted": False,
    "terminal_workflow_dispatch_permitted": False,
    "client_presence_owns_task_lifetime": False,
    "journal_process_stop_matrix_complete": True,
    "authority_crash_reconciliation_complete": True,
    "rv52_story_11_3_local_slice_complete": True,
    "synthetic_data_only": True,
    "native_cross_platform_complete": False,
    "installed_product_complete": False,
    "full_rv52_protocol_complete": False,
    "release_claim": "none",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def artifact(path: str) -> dict[str, Any]:
    absolute = ROOT / path
    return {"path": path, "byte_length": absolute.stat().st_size, "sha256": sha256(absolute)}


def expected_report() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "record_type": "agentmage-story-11-3-durable-resume-evidence",
        "story_id": "11.3",
        "protocol_id": "RV-52",
        "generated_on": "2026-08-31",
        "status": "PASS_LOCAL_DURABLE_RESUME_SCOPE",
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "crash_boundary_coverage": {
            "journal": ["queue-admission", "batch-flush", "correctness-transaction", "subscriber-publication"],
            "authority": ["prepared-stored", "grant-consumed", "attempt-recorded", "launch-boundary-committed", "worker-returned", "result-reconciled"],
            "positions": ["before", "after"],
            "durable_checkpoint": ["atomic-commit", "atomic-rollback", "verified-reopen", "tamper-refusal"],
        },
        "safe_resume_actions": [
            "resume-verified-checkpoint",
            "finalize-verified-effect-without-replay",
            "reconcile-uncertain-effect",
            "replan-and-recheckpoint",
            "stop-budget-exhausted",
            "preserve-terminal-result",
        ],
        "artifacts": [artifact(path) for path in RETAINED_PATHS] + [artifact(RAW_PATH.relative_to(ROOT).as_posix())],
        "product_truth": dict(TRUTH),
        "limitations": [
            "evidence uses deterministic local fixtures and does not claim native installed-product execution",
            "RV-52 scenarios owned by Stories 16.4, 21.4, and 22.5 remain outside this Story 11.3 slice",
            "cross-platform packaging, independent review, and release completion are not claimed",
        ],
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def validate_sources() -> list[str]:
    failures: list[str] = []
    for path in RETAINED_PATHS:
        if not (ROOT / path).is_file():
            failures.append(f"missing retained source: {path}")
    for path, markers in SOURCE_MARKERS.items():
        try:
            content = (ROOT / path).read_text(encoding="utf-8")
        except OSError as error:
            failures.append(f"cannot read source {path}: {error}")
            continue
        failures.extend(f"{path} missing marker: {marker}" for marker in markers if marker not in content)
    return failures


def validate_raw(value: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in value]
    for prohibited in ("test result: FAILED", "error: could not compile", "panicked at", "warning:"):
        if prohibited in value:
            failures.append(f"raw results contain prohibited marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    return [] if value == expected_report() else ["Story 11.3 report is stale, incomplete, reordered, or widened"]


def capture() -> tuple[str, int]:
    output: list[str] = []
    for command in COMMANDS:
        result = subprocess.run(command, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
        output.append(f"$ {' '.join(command)}\n{result.stdout}")
        if result.returncode != 0:
            return "\n".join(output), result.returncode
    return "\n".join(output), 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    arguments = parser.parse_args()
    failures = validate_sources()
    if arguments.write:
        raw, returncode = capture()
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        failures.extend(validate_raw(raw))
        if returncode == 0 and not failures:
            REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")
    else:
        try:
            raw = RAW_PATH.read_text(encoding="utf-8")
            report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            failures.append(f"cannot read retained evidence: {error}")
        else:
            failures.extend(validate_raw(raw))
            failures.extend(validate_report(report))
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print("Story 11.3 durable workflow resume and RV-52 local slice validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
