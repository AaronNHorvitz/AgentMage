#!/usr/bin/env python3
"""Build and validate Sub-task 11.2.3.1 migration-compatibility evidence."""

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
RAW_PATH: Final = EVIDENCE_DIR / "storage-migration-compatibility-results.log"
REPORT_PATH: Final = EVIDENCE_DIR / "storage-migration-compatibility-report.json"
SOURCE_PATH: Final = ROOT / "kernel/engine/src/operational_store.rs"
FIXTURE_PATH: Final = ROOT / "kernel/engine/fixtures/operational-store/schema-18.json"
COMMANDS: Final = (
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "operational_store::tests::version_eighteen_schema_matches_fixture_snapshot_and_is_relational",
        "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "operational_store::tests::version_one_upgrades_through_eighteen_with_exact_history",
        "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "operational_store::tests::failed_version_", "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "operational_store::tests::future_schema_and_page_corruption_are_refused", "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "operational_store::tests::exclusive_writer_and_encrypted_backup_are_verified", "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "operational_store::tests::encrypted_backup_restores_only_to_a_verified_fresh_candidate",
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
    "version_eighteen_schema_matches_fixture_snapshot_and_is_relational ... ok",
    "version_one_upgrades_through_eighteen_with_exact_history ... ok",
    "failed_version_two_migration_rolls_back_without_partial_schema ... ok",
    "failed_version_three_migration_rolls_back_all_alterations ... ok",
    "future_schema_and_page_corruption_are_refused ... ok",
    "exclusive_writer_and_encrypted_backup_are_verified ... ok",
    "encrypted_backup_restores_only_to_a_verified_fresh_candidate ... ok",
    "seeded_crash_recovery_campaign_never_repeats_a_completed_transition ... ok",
)
SOURCE_MARKERS: Final = (
    "const SCHEMA_VERSION: i64 = 18;",
    "../fixtures/operational-store/schema-18.json",
    "fn verify_schema_history(connection: &Connection)",
    "fn failed_version_three_migration_rolls_back_all_alterations()",
    "fn failed_version_two_migration_rolls_back_without_partial_schema()",
    "fn future_schema_and_page_corruption_are_refused()",
    "fn exclusive_writer_and_encrypted_backup_are_verified()",
    "fn encrypted_backup_restores_only_to_a_verified_fresh_candidate()",
    "SeededCrashBoundary::Migration",
)
MIGRATION_SHA256: Final = (
    "da2acbccbe4e37e1920ce3941b68716de1a095c45bdbcfe37b39ef239cf45a7d",
    "ac79d84633a58a69204edcf06bd6e9a0f67a0308509a3f9449a1e8599805a3a2",
    "7bbac20f36ed80a1b07bea98641d98c3f894500a11dc7ef7193a1eb29cbc40b6",
    "2ba8afc6ed5aed817e4b88046e6f28fc1b0b3c7aec98b7479daea34cebb43089",
    "035b79e685c570210c67f51cfcd466a12131e93281a33c76768184a816995586",
    "1d42813be51e882bdb0ee17e36ab4c6e498f57a19c3a33e432bbd54e1863547a",
    "266f7eba307f96257baae58cc31b9df9c9a8ff40492f5ac274b9d4171e3883a0",
    "19700923fefef5eddf8112984d38caec8b81ea5b463860db866ab9116915f2f5",
    "5eb6d2b0a312addaaa6c59ace373ba0e9bf9e8551e38ed82ce2f4fcdbccb5cd8",
    "2fe0ada1916efceab682f03d37cda75645755b6873074d17631fcf8991bc0588",
    "75aa2c2c9ca3879dd3c87ac3a1468a4b3933e82eaa843baa9bb222240c6a2e7d",
    "5821bc16e3f91dd88faadba445f663a00794c402c0472940380062c5a5051f4e",
    "7ec4613d128bb70d1be686cff7216f328b1968a6f5032ba9e83d92c2d3481c5e",
    "78146e92fda7e4f463cab1c751455d99ce7ff95de2bce5829fe768092f671fe7",
    "4caabc1a09d7cdedc69ffb231d03a467d9c5dd1aa0f706d917682c17f8aaf223",
    "3f2730d0f49d60ce70bf8bb9f8980e32bae74f31ece544e3d99a839918d20c0d",
    "24a591be13fd7fd97531ca99a494979f66ffd6bdf2d12536eb9619177395797d",
    "563d3643a23b63bee80a40aee12c8cf5fb7f017b52d4f9f95d6ac65bacf26630",
)
TRUTH: Final = {
    "synthetic_data_only": True,
    "forward_migrations_complete": True,
    "fixture_snapshot_complete": True,
    "schema_hashes_complete": True,
    "rollback_tests_complete": True,
    "interrupted_migration_recovery_complete": True,
    "future_schema_refusal_complete": True,
    "occupied_destination_handling_complete": True,
    "downgrade_record_behavior_complete": False,
    "new_family_lifecycle_coverage_complete": False,
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
        "record_type": "agentmage-storage-migration-compatibility-evidence",
        "story_id": "11.2",
        "task_id": "11.2.3.1",
        "generated_on": "2026-08-30",
        "status": "pass-local-migration-compatibility",
        "canonical_store": "operational-store",
        "operational_schema_version": 18,
        "migration_count": 18,
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "artifacts": [
            artifact("kernel/engine/src/operational_store.rs"),
            artifact("kernel/engine/fixtures/operational-store/schema-18.json"),
            artifact("docs/verification/story-11-2-storage-migration-compatibility-evidence.md"),
            artifact("scripts/storage_migration_compatibility_evidence.py"),
            artifact("tests/test_storage_migration_compatibility_evidence.py"),
            artifact(RAW_PATH.relative_to(ROOT).as_posix()),
        ],
        "product_truth": dict(TRUTH),
    }


def render(value: dict[str, Any]) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"


def validate_source(value: str) -> list[str]:
    return [f"source missing migration compatibility marker: {marker}" for marker in SOURCE_MARKERS if marker not in value]


def validate_fixture(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict) or value.get("record_type") != "agentmage-operational-store-schema-fixture":
        failures.append("fixture record type is not exact")
        return failures
    if value.get("schema_version") != 18:
        failures.append("fixture schema version is not 18")
    migrations = value.get("migrations")
    expected = [
        {"version": index, "sha256": digest}
        for index, digest in enumerate(MIGRATION_SHA256, start=1)
    ]
    if migrations != expected:
        failures.append("fixture migration history or digest ordering drifted")
    tables = value.get("tables")
    if not isinstance(tables, list) or tables != sorted(set(tables)):
        failures.append("fixture table inventory is absent, duplicated, or unordered")
    for table in ("schema_history", "source_manifests", "workflow_attempts", "workflow_receipts"):
        if not isinstance(tables, list) or table not in tables:
            failures.append(f"fixture missing required table: {table}")
    return failures


def validate_raw(value: str) -> list[str]:
    failures = [f"raw results missing marker: {marker}" for marker in MARKERS if marker not in value]
    for prohibited in ("test result: FAILED", "error: could not compile", "warning:"):
        if prohibited in value:
            failures.append(f"raw results contain prohibited marker: {prohibited}")
    return failures


def validate_report(value: Any) -> list[str]:
    if value != expected_report():
        return ["storage migration compatibility report is stale, incomplete, reordered, or widened"]
    if value.get("product_truth") != TRUTH:
        return ["storage migration compatibility product truth was widened"]
    return []


def capture() -> tuple[str, int]:
    chunks: list[str] = []
    for command in COMMANDS:
        result = subprocess.run(
            command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
            text=True, check=False,
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
        failures = validate_source(SOURCE_PATH.read_text(encoding="utf-8"))
        failures += validate_fixture(json.loads(FIXTURE_PATH.read_text(encoding="utf-8")))
        failures += validate_raw(raw)
        if failures:
            for failure in failures:
                print(f"storage migration compatibility evidence build failed: {failure}", file=sys.stderr)
            return 1
        EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
        RAW_PATH.write_text(raw, encoding="utf-8")
        REPORT_PATH.write_text(render(expected_report()), encoding="utf-8")

    try:
        raw = RAW_PATH.read_text(encoding="utf-8")
        report = json.loads(REPORT_PATH.read_text(encoding="utf-8"))
        source = SOURCE_PATH.read_text(encoding="utf-8")
        fixture = json.loads(FIXTURE_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"storage migration compatibility evidence validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_source(source) + validate_fixture(fixture) + validate_raw(raw) + validate_report(report)
    if failures:
        for failure in failures:
            print(f"storage migration compatibility evidence validation failed: {failure}", file=sys.stderr)
        return 1
    print("Sub-task 11.2.3.1 storage migration compatibility validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
