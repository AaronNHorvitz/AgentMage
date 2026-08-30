#!/usr/bin/env python3
"""Build and validate Sub-task 11.2.2.2 workflow-attempt invariant evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any, Final


ROOT: Final = Path(__file__).resolve().parents[1]
EVIDENCE_DIR: Final = ROOT / "artifacts/sprints/sprint-11/story-11.2"
RAW_PATH: Final = EVIDENCE_DIR / "workflow-attempt-invariants-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "workflow-attempt-invariants-report.json"
MIGRATION_PATH: Final = (
    ROOT / "kernel/engine/migrations/operational-store/0016-workflow-attempt-invariants.sql"
)
COMMANDS: Final = (
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "operational_store::tests::workflow_attempt_chain_receipt_and_uncertainty_invariants_fail_closed",
        "--locked",
    ),
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "operational_store::tests::version_one_upgrades_through_sixteen_with_exact_history",
        "--locked",
    ),
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "operational_store::tests::seeded_crash_recovery_campaign_never_repeats_a_completed_transition",
        "--locked",
    ),
    (
        "cargo",
        "clippy",
        "-p",
        "agentmage-kernel-engine",
        "--all-targets",
        "--all-features",
        "--locked",
        "--",
        "-D",
        "warnings",
    ),
)
MARKERS: Final = (
    "workflow_attempt_chain_receipt_and_uncertainty_invariants_fail_closed ... ok",
    "version_one_upgrades_through_sixteen_with_exact_history ... ok",
    "seeded_crash_recovery_campaign_never_repeats_a_completed_transition ... ok",
)
UNIQUE_INDEXES: Final = (
    "workflow_attempt_step_ordinal_uq",
    "workflow_idempotency_key_uq",
    "workflow_receipt_identity_uq",
    "workflow_receipt_attempt_uq",
)
TRIGGERS: Final = (
    "workflow_attempt_insert_started",
    "workflow_attempt_insert_chain",
    "workflow_receipt_exact_existing_authority",
    "workflow_attempt_terminal_transition",
    "workflow_attempt_delete_forbidden",
    "workflow_receipt_update_forbidden",
    "workflow_receipt_delete_forbidden",
)
TRUTH: Final = {
    "synthetic_data_only": True,
    "workflow_metadata_materializations_complete": True,
    "attempt_and_idempotency_invariants_complete": True,
    "atomic_event_projection_checkpoint_commit_complete": False,
    "full_workflow_crash_campaign_complete": False,
    "story_completion_claim": False,
    "sprint_completion_claim": False,
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
        "record_type": "agentmage-workflow-attempt-invariants-evidence",
        "story_id": "11.2",
        "task_id": "11.2.2.2",
        "generated_on": "2026-08-30",
        "status": "pass-local-storage-invariants",
        "operational_store_schema_version": 16,
        "unique_indexes": list(UNIQUE_INDEXES),
        "append_only_triggers": list(TRIGGERS),
        "invariant_contract": {
            "unique_attempt_identity": True,
            "unique_step_attempt_ordinal": True,
            "unique_idempotency_key_digest": True,
            "contiguous_terminal_predecessor_chain": True,
            "attempts_start_without_receipt": True,
            "one_terminal_transition": True,
            "exact_existing_receipt_identity_and_digest": True,
            "exact_attempt_receipt_outcome_binding": True,
            "uncertainty_is_terminal_and_immutable": True,
        },
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "artifacts": [
            artifact("kernel/engine/migrations/operational-store/0016-workflow-attempt-invariants.sql"),
            artifact("kernel/engine/src/operational_store.rs"),
            artifact("docs/verification/story-11-2-workflow-attempt-invariants-evidence.md"),
            artifact("scripts/workflow_attempt_invariants_evidence.py"),
            artifact("tests/test_workflow_attempt_invariants_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "product_truth": dict(TRUTH),
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def validate_migration(value: str) -> list[str]:
    failures: list[str] = []
    for index in UNIQUE_INDEXES:
        if f"CREATE UNIQUE INDEX {index}" not in value:
            failures.append(f"migration missing unique invariant: {index}")
    for trigger in TRIGGERS:
        if f"CREATE TRIGGER {trigger}" not in value:
            failures.append(f"migration missing append-only invariant: {trigger}")
    for required in (
        "prior.attempt_ordinal = NEW.attempt_ordinal - 1",
        "prior.state <> 'started'",
        "terminal_receipt.receipt_id = NEW.receipt_id",
        "terminal_receipt.receipt_sha256 = NEW.receipt_sha256",
        "terminal_receipt.outcome = NEW.state",
        "OLD.state <> 'started'",
        "authority_receipt.receipt_sha256 = NEW.receipt_sha256",
        "attempt.attempt_id = NEW.attempt_id",
        "UPDATE workflow_attempts",
        "workflow_attempt_invariant_migration_guard",
    ):
        if required not in value:
            failures.append(f"migration missing fail-closed clause: {required}")
    if "UPDATE workflow_attempts SET state = 'succeeded'" in value:
        failures.append("migration rewrites an attempt as success")
    return failures


def validate_raw(value: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in value]
    for prohibited in ("test result: FAILED", "error: could not compile", "warning:"):
        if prohibited in value:
            failures.append(f"raw results contain prohibited marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    if value != expected_report():
        return ["workflow-attempt report is stale, incomplete, reordered, or widened"]
    if value.get("product_truth") != TRUTH:
        return ["workflow-attempt product truth was widened"]
    return []


def capture() -> tuple[str, int]:
    chunks: list[str] = []
    for command in COMMANDS:
        result = subprocess.run(
            command,
            cwd=ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            check=False,
        )
        chunks.append(f"$ {' '.join(command)}\n{result.stdout.rstrip(chr(10))}\n")
        if result.returncode != 0:
            return "".join(chunks), result.returncode
    return "".join(chunks), 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    if args.write:
        raw, returncode = capture()
        if returncode != 0:
            sys.stderr.write(raw)
            return 1
        failures = validate_migration(MIGRATION_PATH.read_text(encoding="utf-8")) + validate_raw(raw)
        if failures:
            for failure in failures:
                print(f"workflow-attempt evidence build failed: {failure}", file=sys.stderr)
            return 1
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")

    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        migration = MIGRATION_PATH.read_text(encoding="utf-8")
    except (OSError, json.JSONDecodeError) as error:
        print(f"workflow-attempt evidence validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_migration(migration) + validate_raw(raw) + validate_report(report)
    if failures:
        for failure in failures:
            print(f"workflow-attempt evidence validation failed: {failure}", file=sys.stderr)
        return 1
    print("Sub-task 11.2.2.2 workflow attempt invariants validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
