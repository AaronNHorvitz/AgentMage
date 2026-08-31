#!/usr/bin/env python3
"""Build and validate Sub-task 11.2.2.1 workflow-materialization evidence."""

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
RAW_PATH: Final = EVIDENCE_DIR / "workflow-materialization-schema-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "workflow-materialization-schema-report.json"
MIGRATION_PATH: Final = (
    ROOT / "kernel/engine/migrations/operational-store/0015-workflow-materializations.sql"
)
TABLES: Final = (
    "workflow_plan_step_policies",
    "workflow_attempts",
    "workflow_preflights",
    "workflow_tool_calls",
    "workflow_idempotency_keys",
    "workflow_approvals",
    "workflow_receipts",
    "workflow_verifications",
    "workflow_consumed_budgets",
    "workflow_recovery_decisions",
    "workflow_state_fingerprints",
    "workflow_terminal_diagnostics",
)
COMMANDS: Final = (
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "operational_store::tests::workflow_materializations_bind_existing_run_session_event_and_receipt_authorities",
        "--locked",
    ),
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "operational_store::tests::version_one_upgrades_through_seventeen_with_exact_history",
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
    "workflow_materializations_bind_existing_run_session_event_and_receipt_authorities ... ok",
    "version_one_upgrades_through_seventeen_with_exact_history ... ok",
    "seeded_crash_recovery_campaign_never_repeats_a_completed_transition ... ok",
)
TRUTH: Final = {
    "synthetic_data_only": True,
    "workflow_metadata_materializations_complete": True,
    "typed_publication_api_complete": False,
    "attempt_and_idempotency_invariants_complete": False,
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
        "record_type": "agentmage-workflow-materialization-schema-evidence",
        "story_id": "11.2",
        "task_id": "11.2.2.1",
        "generated_on": "2026-08-30",
        "status": "pass-local-structural-schema",
        "materialization_migration_version": 15,
        "current_operational_store_schema_version": 17,
        "normalized_tables": list(TABLES),
        "existing_authorities": {
            "metadata_store": "operational-store",
            "run_table": "runtime_runs",
            "session_table": "sessions",
            "event_table": "runtime_events",
            "receipt_table": "receipts",
            "new_payload_tables": [],
            "new_event_journals": [],
        },
        "structural_contract": {
            "workflow_family_count": len(TABLES),
            "strict_tables": True,
            "closed_enum_checks": True,
            "bounded_record_json": True,
            "exact_run_session_binding": True,
            "exact_run_sequence_event_binding": True,
            "existing_plan_binding": True,
            "existing_grant_binding": True,
            "existing_receipt_binding": True,
        },
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "artifacts": [
            artifact("kernel/engine/migrations/operational-store/0006-runtime-journal.sql"),
            artifact("kernel/engine/migrations/operational-store/0015-workflow-materializations.sql"),
            artifact("kernel/engine/src/operational_store.rs"),
            artifact("docs/verification/story-11-2-workflow-materialization-schema-evidence.md"),
            artifact("scripts/workflow_materialization_schema_evidence.py"),
            artifact("tests/test_workflow_materialization_schema_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "product_truth": dict(TRUTH),
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def validate_migration(value: str) -> list[str]:
    failures = [
        f"migration missing normalized table: {table}"
        for table in TABLES
        if f"CREATE TABLE {table} (" not in value
    ]
    if value.count(") STRICT;") != len(TABLES):
        failures.append("migration does not define exactly twelve STRICT workflow tables")
    if value.count("record_json BLOB NOT NULL") != len(TABLES):
        failures.append("every workflow family must retain one bounded canonical metadata record")
    if value.count(
        "FOREIGN KEY(run_id, session_id) REFERENCES runtime_runs(run_id, session_id)"
    ) != len(TABLES):
        failures.append("every workflow family must bind the exact existing run and session")
    if value.count("REFERENCES runtime_events(run_id, sequence, event_id)") != len(TABLES):
        failures.append("every workflow family must bind the exact existing runtime event")
    for required in (
        "FOREIGN KEY(plan_id) REFERENCES plans(plan_id)",
        "FOREIGN KEY(grant_id) REFERENCES grant_identities(grant_id)",
        "FOREIGN KEY(receipt_id) REFERENCES receipts(receipt_id)",
    ):
        if required not in value:
            failures.append(f"migration missing existing authority binding: {required}")
    for prohibited in (
        "CREATE TABLE workflow_payload",
        "CREATE TABLE workflow_event_journal",
        "payload BLOB",
        "source_bytes BLOB",
    ):
        if prohibited in value:
            failures.append(f"migration creates a prohibited second authority: {prohibited}")
    return failures


def validate_raw(value: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in value]
    for prohibited in ("test result: FAILED", "error: could not compile", "warning:"):
        if prohibited in value:
            failures.append(f"raw results contain prohibited marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    if value != expected_report():
        return ["workflow-materialization report is stale, incomplete, reordered, or widened"]
    if value.get("product_truth") != TRUTH:
        return ["workflow-materialization product truth was widened"]
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
                print(f"workflow-materialization evidence build failed: {failure}", file=sys.stderr)
            return 1
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")

    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        migration = MIGRATION_PATH.read_text(encoding="utf-8")
    except (OSError, json.JSONDecodeError) as error:
        print(f"workflow-materialization evidence validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_migration(migration) + validate_raw(raw) + validate_report(report)
    if failures:
        for failure in failures:
            print(f"workflow-materialization evidence validation failed: {failure}", file=sys.stderr)
        return 1
    print("Sub-task 11.2.2.1 normalized workflow materializations validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
