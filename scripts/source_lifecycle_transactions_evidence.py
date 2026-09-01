#!/usr/bin/env python3
"""Build and validate Sub-task 11.2.1.3 source-lifecycle transaction evidence."""

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
RAW_PATH: Final = EVIDENCE_DIR / "source-lifecycle-transactions-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "source-lifecycle-transactions-report.json"
MIGRATION_PATH: Final = ROOT / "kernel/engine/migrations/operational-store/0014-source-lifecycle-transactions.sql"
MODULE_PATH: Final = ROOT / "kernel/engine/src/source_lifecycle.rs"
COMMANDS: Final = (
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "source_lifecycle::tests", "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "runtime_artifact::tests::publication_deduplicates_without_broadening_owner_or_reference_state",
        "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "operational_store::tests::version_one_upgrades_through_eighteen_with_exact_history",
        "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "operational_store::tests::seeded_crash_recovery_campaign_never_repeats_a_completed_transition",
        "--locked",
    ),
    (
        "cargo", "clippy", "-p", "agentmage-kernel-engine", "--all-targets",
        "--all-features", "--locked", "--", "-D", "warnings",
    ),
)
MARKERS: Final = (
    "refresh_hold_expiry_delete_and_collection_are_atomic_and_reference_safe ... ok",
    "stale_revision_and_active_hold_roll_back_without_runtime_reference_drift ... ok",
    "publication_deduplicates_without_broadening_owner_or_reference_state ... ok",
    "version_one_upgrades_through_eighteen_with_exact_history ... ok",
    "seeded_crash_recovery_campaign_never_repeats_a_completed_transition ... ok",
)
REQUIRED_MIGRATION_FRAGMENTS: Final = (
    "CREATE TABLE source_materialization_states",
    "CREATE TABLE source_materialization_events",
    "CREATE TABLE source_dependencies",
    "CREATE TRIGGER source_dependencies_cycle_insert",
    "CREATE TABLE source_refreshes",
    "CREATE TRIGGER source_refreshes_identity_insert",
    "CREATE TABLE source_retention_deadlines",
    "CREATE TABLE source_retention_holds",
    "CREATE TABLE source_retention_hold_events",
    "CREATE TABLE source_released_payloads",
    "CREATE VIEW current_source_sections",
    "CREATE VIEW current_source_extractions",
    "CREATE VIEW current_source_cache_inputs",
    "CREATE VIEW current_source_lexical_indexes",
    "CREATE VIEW current_source_context_dispositions",
)
REQUIRED_API_FRAGMENTS: Final = (
    "pub fn register_source_dependency",
    "pub fn refresh_source",
    "pub fn register_source_retention_deadline",
    "pub fn apply_source_hold",
    "pub fn release_source_hold",
    "pub fn release_source",
    "pub fn expire_sources",
    "pub fn delete_source",
    "pub fn collect_source_garbage",
    "pub fn verify_source_lifecycle",
    "transaction_with_behavior(TransactionBehavior::Immediate)",
    "reconcile_runtime_artifacts(store, payloads, now_epoch_ms)",
)
TRUTH: Final = {
    "synthetic_data_only": True,
    "atomic_refresh_complete": True,
    "transitive_invalidation_complete": True,
    "atomic_expiry_complete": True,
    "user_and_legal_hold_projection_complete": True,
    "logical_release_and_deletion_complete": True,
    "reference_safe_physical_collection_complete": True,
    "new_payload_authority_created": False,
    "typed_source_publication_complete": False,
    "exhaustive_source_crash_campaign_complete": False,
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
        "record_type": "agentmage-source-lifecycle-transactions-evidence",
        "story_id": "11.2",
        "task_id": "11.2.1.3",
        "generated_on": "2026-08-30",
        "status": "pass-local-atomic-source-lifecycle",
        "operational_store_schema_version": 17,
        "physical_payload_authority": {
            "migration": "0007-runtime-artifacts.sql",
            "payload_table": "runtime_payloads",
            "artifact_table": "runtime_artifacts",
            "new_payload_tables": [],
        },
        "transaction_contract": {
            "refresh_invalidates_dependency_closure": True,
            "stale_derivatives_excluded_from_current_views": True,
            "dependency_cycles_rejected": True,
            "hold_events_are_revisioned_and_hash_chained": True,
            "held_expiry_is_preserved": True,
            "release_updates_source_and_runtime_references_together": True,
            "deletion_precedes_existing_payload_reconciliation": True,
            "shared_live_payloads_survive_collection": True,
            "stale_revisions_roll_back_without_reference_drift": True,
        },
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "artifacts": [
            artifact("kernel/engine/migrations/operational-store/0007-runtime-artifacts.sql"),
            artifact("kernel/engine/migrations/operational-store/0014-source-lifecycle-transactions.sql"),
            artifact("kernel/engine/src/runtime_artifact.rs"),
            artifact("kernel/engine/src/source_lifecycle.rs"),
            artifact("kernel/engine/src/operational_store.rs"),
            artifact("docs/verification/story-11-2-source-lifecycle-transactions-evidence.md"),
            artifact("scripts/source_lifecycle_transactions_evidence.py"),
            artifact("tests/test_source_lifecycle_transactions_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "product_truth": dict(TRUTH),
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def validate_sources(migration: str, module: str) -> list[str]:
    failures = [
        f"migration missing lifecycle constraint: {fragment}"
        for fragment in REQUIRED_MIGRATION_FRAGMENTS
        if fragment not in migration
    ]
    failures.extend(
        f"source lifecycle API missing: {fragment}"
        for fragment in REQUIRED_API_FRAGMENTS
        if fragment not in module
    )
    for prohibited in (
        "CREATE TABLE source_payloads",
        "CREATE TABLE source_artifact_payloads",
        "CREATE TABLE source_content",
        "payload BLOB",
        "source_bytes BLOB",
    ):
        if prohibited in migration:
            failures.append(f"migration creates a prohibited source-byte authority: {prohibited}")
    return failures


def validate_raw(value: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in value]
    for prohibited in ("test result: FAILED", "error: could not compile", "warning:"):
        if prohibited in value:
            failures.append(f"raw results contain prohibited marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    if value != expected_report():
        return ["source-lifecycle transaction report is stale, incomplete, reordered, or widened"]
    if value.get("product_truth") != TRUTH:
        return ["source-lifecycle transaction product truth was widened"]
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
        failures = validate_sources(
            MIGRATION_PATH.read_text(encoding="utf-8"),
            MODULE_PATH.read_text(encoding="utf-8"),
        ) + validate_raw(raw)
        if failures:
            for failure in failures:
                print(f"source-lifecycle evidence build failed: {failure}", file=sys.stderr)
            return 1
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")

    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        migration = MIGRATION_PATH.read_text(encoding="utf-8")
        module = MODULE_PATH.read_text(encoding="utf-8")
    except (OSError, json.JSONDecodeError) as error:
        print(f"source-lifecycle evidence validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_sources(migration, module) + validate_raw(raw) + validate_report(report)
    if failures:
        for failure in failures:
            print(f"source-lifecycle evidence validation failed: {failure}", file=sys.stderr)
        return 1
    print("Sub-task 11.2.1.3 atomic source lifecycle transactions validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
