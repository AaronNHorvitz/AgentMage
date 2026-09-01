#!/usr/bin/env python3
"""Build and validate Sub-task 11.2.1.1 source-materialization schema evidence."""

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
RAW_PATH: Final = EVIDENCE_DIR / "source-materialization-schema-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "source-materialization-schema-report.json"
MIGRATION_PATH: Final = ROOT / "kernel/engine/migrations/operational-store/0012-source-artifact-materializations.sql"
TABLES: Final = (
    "source_manifests",
    "source_origins",
    "source_references",
    "source_extractions",
    "source_sections",
    "source_provenance",
    "source_lexical_indexes",
    "source_context_dispositions",
    "source_cache_inputs",
    "source_retentions",
    "source_lifecycle_events",
)
COMMANDS: Final = (
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "operational_store::tests::version_eighteen_schema_matches_fixture_snapshot_and_is_relational",
        "--locked",
    ),
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "operational_store::tests::source_materializations_bind_one_existing_encrypted_payload_without_new_byte_store",
        "--locked",
    ),
    (
        "cargo",
        "test",
        "-p",
        "agentmage-kernel-engine",
        "operational_store::tests::version_one_upgrades_through_eighteen_with_exact_history",
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
    "version_eighteen_schema_matches_fixture_snapshot_and_is_relational ... ok",
    "source_materializations_bind_one_existing_encrypted_payload_without_new_byte_store ... ok",
    "version_one_upgrades_through_eighteen_with_exact_history ... ok",
    "seeded_crash_recovery_campaign_never_repeats_a_completed_transition ... ok",
)
TRUTH: Final = {
    "synthetic_data_only": True,
    "schema_materializations_complete": True,
    "typed_publication_api_complete": False,
    "content_deduplication_complete": False,
    "refresh_invalidation_lifecycle_complete": False,
    "full_crash_campaign_complete": False,
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
        "record_type": "agentmage-source-materialization-schema-evidence",
        "story_id": "11.2",
        "task_id": "11.2.1.1",
        "generated_on": "2026-08-30",
        "status": "pass-local-structural-schema",
        "materialization_migration_version": 12,
        "current_operational_store_schema_version": 18,
        "normalized_tables": list(TABLES),
        "payload_authority": {
            "migration": "0007-runtime-artifacts.sql",
            "payload_table": "runtime_payloads",
            "artifact_table": "runtime_artifacts",
            "new_payload_tables": [],
            "source_rows_store_raw_source_bytes": False,
        },
        "structural_contract": {
            "source_family_count": len(TABLES),
            "strict_tables": True,
            "closed_enum_checks": True,
            "bounded_record_json": True,
            "foreign_keys_enabled": True,
            "manifest_provenance_cycle_is_deferred_and_transactional": True,
            "physical_artifact_payload_and_size_are_jointly_bound": True,
            "section_parent_stays_in_same_extraction": True,
            "context_section_stays_in_same_source": True,
            "retention_lifecycle_family_is_closed": True,
        },
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "artifacts": [
            artifact("kernel/engine/migrations/operational-store/0007-runtime-artifacts.sql"),
            artifact("kernel/engine/migrations/operational-store/0012-source-artifact-materializations.sql"),
            artifact("kernel/engine/src/operational_store.rs"),
            artifact("docs/verification/story-11-2-source-materialization-schema-evidence.md"),
            artifact("scripts/source_materialization_schema_evidence.py"),
            artifact("tests/test_source_materialization_schema_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "product_truth": dict(TRUTH),
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def validate_migration(value: str) -> list[str]:
    failures = [f"migration missing normalized table: {table}" for table in TABLES if f"CREATE TABLE {table} (" not in value]
    for prohibited in (
        "CREATE TABLE source_payloads",
        "CREATE TABLE source_artifact_payloads",
        "payload BLOB",
        "source_bytes BLOB",
    ):
        if prohibited in value:
            failures.append(f"migration creates a prohibited source-byte authority: {prohibited}")
    if "REFERENCES runtime_artifacts(artifact_id, payload_sha256, byte_size)" not in value:
        failures.append("migration does not bind source materializations to the existing artifact tuple")
    return failures


def validate_raw(value: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in value]
    for prohibited in ("test result: FAILED", "error: could not compile", "warning:"):
        if prohibited in value:
            failures.append(f"raw results contain prohibited marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    if value != expected_report():
        return ["source-materialization report is stale, incomplete, reordered, or widened"]
    if value.get("product_truth") != TRUTH:
        return ["source-materialization product truth was widened"]
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
                print(f"source-materialization evidence build failed: {failure}", file=sys.stderr)
            return 1
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")

    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        migration = MIGRATION_PATH.read_text(encoding="utf-8")
    except (OSError, json.JSONDecodeError) as error:
        print(f"source-materialization evidence validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_migration(migration) + validate_raw(raw) + validate_report(report)
    if failures:
        for failure in failures:
            print(f"source-materialization evidence validation failed: {failure}", file=sys.stderr)
        return 1
    print("Sub-task 11.2.1.1 normalized source materializations validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
