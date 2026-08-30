#!/usr/bin/env python3
"""Build and validate Sub-task 11.2.1.2 content-deduplication evidence."""

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
RAW_PATH: Final = EVIDENCE_DIR / "source-content-deduplication-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "source-content-deduplication-report.json"
MIGRATION_PATH: Final = ROOT / "kernel/engine/migrations/operational-store/0013-source-content-deduplication.sql"
COMMANDS: Final = (
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "runtime_artifact::tests::publication_deduplicates_without_broadening_owner_or_reference_state",
        "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "operational_store::tests::source_content_deduplication_preserves_every_logical_identity",
        "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "operational_store::tests::version_one_upgrades_through_fifteen_with_exact_history",
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
    "publication_deduplicates_without_broadening_owner_or_reference_state ... ok",
    "source_content_deduplication_preserves_every_logical_identity ... ok",
    "version_one_upgrades_through_fifteen_with_exact_history ... ok",
    "seeded_crash_recovery_campaign_never_repeats_a_completed_transition ... ok",
)
REQUIRED_MIGRATION_FRAGMENTS: Final = (
    "CREATE UNIQUE INDEX source_manifests_physical_identity_idx",
    "CREATE UNIQUE INDEX source_retentions_physical_identity_idx",
    "CREATE TRIGGER source_origins_identity_update",
    "CREATE TRIGGER source_references_identity_update",
    "CREATE TRIGGER source_manifests_identity_insert",
    "CREATE TRIGGER source_manifests_identity_update",
    "CREATE TRIGGER source_provenance_identity_insert",
    "CREATE TRIGGER source_provenance_identity_update",
    "CREATE TRIGGER source_retentions_identity_insert",
    "CREATE TRIGGER source_retentions_identity_update",
)
TRUTH: Final = {
    "synthetic_data_only": True,
    "physical_content_hash_deduplication_complete": True,
    "logical_origin_identity_preserved": True,
    "logical_classification_identity_preserved": True,
    "logical_authority_identity_preserved": True,
    "logical_freshness_identity_preserved": True,
    "logical_retention_identity_preserved": True,
    "new_payload_authority_created": False,
    "refresh_invalidation_lifecycle_complete": False,
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
        "record_type": "agentmage-source-content-deduplication-evidence",
        "story_id": "11.2",
        "task_id": "11.2.1.2",
        "generated_on": "2026-08-30",
        "status": "pass-local-content-deduplication",
        "operational_store_schema_version": 15,
        "physical_payload_authority": {
            "migration": "0007-runtime-artifacts.sql",
            "payload_table": "runtime_payloads",
            "artifact_table": "runtime_artifacts",
            "new_payload_tables": [],
        },
        "deduplication_contract": {
            "physical_payloads_for_equal_bytes": 1,
            "logical_runtime_artifacts_for_distinct_owners": 2,
            "logical_source_manifests_for_distinct_identities": 2,
            "preserved_identity_classes": [
                "origin", "classification", "authority", "freshness", "retention"
            ],
            "physical_reference_identity_is_single_source": True,
            "logical_identity_rows_are_immutable": True,
        },
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "artifacts": [
            artifact("kernel/engine/migrations/operational-store/0007-runtime-artifacts.sql"),
            artifact("kernel/engine/migrations/operational-store/0013-source-content-deduplication.sql"),
            artifact("kernel/engine/src/runtime_artifact.rs"),
            artifact("kernel/engine/src/operational_store.rs"),
            artifact("docs/verification/story-11-2-source-content-deduplication-evidence.md"),
            artifact("scripts/source_content_deduplication_evidence.py"),
            artifact("tests/test_source_content_deduplication_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "product_truth": dict(TRUTH),
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def validate_migration(value: str) -> list[str]:
    failures = [
        f"migration missing deduplication constraint: {fragment}"
        for fragment in REQUIRED_MIGRATION_FRAGMENTS
        if fragment not in value
    ]
    for prohibited in (
        "CREATE TABLE source_payloads",
        "CREATE TABLE source_artifact_payloads",
        "CREATE TABLE source_content",
        "payload BLOB",
        "source_bytes BLOB",
    ):
        if prohibited in value:
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
        return ["source-content-deduplication report is stale, incomplete, reordered, or widened"]
    if value.get("product_truth") != TRUTH:
        return ["source-content-deduplication product truth was widened"]
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
                print(f"source-content-deduplication evidence build failed: {failure}", file=sys.stderr)
            return 1
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")

    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        migration = MIGRATION_PATH.read_text(encoding="utf-8")
    except (OSError, json.JSONDecodeError) as error:
        print(f"source-content-deduplication evidence validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_migration(migration) + validate_raw(raw) + validate_report(report)
    if failures:
        for failure in failures:
            print(f"source-content-deduplication evidence validation failed: {failure}", file=sys.stderr)
        return 1
    print("Sub-task 11.2.1.2 content-hash deduplication validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
