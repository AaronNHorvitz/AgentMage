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
READER_TEST_PATH: Final = ROOT / "kernel/engine/src/research_reader_migration_tests.rs"
JOB_LEDGER_TEST_PATH: Final = ROOT / "kernel/engine/src/job_ledger_migration_tests.rs"
RUN_HISTORY_TEST_PATH: Final = ROOT / "kernel/engine/src/run_action_history_migration_tests.rs"
FIXTURE_PATH: Final = ROOT / "kernel/engine/fixtures/operational-store/schema-22.json"
COMMANDS: Final = (
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "operational_store::tests::research_migration_tests", "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "operational_store::tests::version_twenty_two_schema_matches_fixture_snapshot_and_is_relational",
        "--locked",
    ),
    (
        "cargo", "test", "-p", "agentmage-kernel-engine",
        "operational_store::tests::version_one_upgrades_through_twenty_two_with_exact_history",
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
    "version_twenty_one_upgrades_to_run_action_histories_preserving_records_and_history ... ok",
    "failed_version_twenty_two_migration_rolls_back_and_stays_retryable ... ok",
    "version_twenty_one_corrupt_history_cannot_add_run_action_histories ... ok",
    "version_twenty_upgrades_to_job_ledgers_preserving_records_and_history ... ok",
    "failed_version_twenty_one_migration_rolls_back_and_stays_retryable ... ok",
    "version_twenty_corrupt_history_cannot_add_job_ledgers ... ok",
    "version_nineteen_backup_refusal_preserves_source_without_creating_candidate ... ok",
    "version_nineteen_reader_epoch_preserves_tables_records_and_history ... ok",
    "version_nineteen_corrupt_history_cannot_advance_reader_epoch ... ok",
    "failed_version_twenty_reader_epoch_keeps_version_nineteen_retryable ... ok",
    "version_eighteen_upgrades_without_losing_existing_records ... ok",
    "failed_version_nineteen_migration_rolls_back_tables_history_and_version ... ok",
    "version_twenty_two_schema_matches_fixture_snapshot_and_is_relational ... ok",
    "version_one_upgrades_through_twenty_two_with_exact_history ... ok",
    "failed_version_two_migration_rolls_back_without_partial_schema ... ok",
    "failed_version_three_migration_rolls_back_all_alterations ... ok",
    "future_schema_and_page_corruption_are_refused ... ok",
    "exclusive_writer_and_encrypted_backup_are_verified ... ok",
    "encrypted_backup_restores_only_to_a_verified_fresh_candidate ... ok",
    "seeded_crash_recovery_campaign_never_repeats_a_completed_transition ... ok",
)
SOURCE_MARKERS: Final = (
    "verify_schema_history_through(connection, 19)?;",
    "verify_schema_history_through(connection, 20)?;",
    "verify_schema_history_through(connection, 21)?;",
    "fn verify_schema_history_through(",
    "../migrations/operational-store/0020-research-draft-readers.sql",
    "../migrations/operational-store/0021-job-control-ledgers.sql",
    "../migrations/operational-store/0022-run-action-histories.sql",
    "const SCHEMA_VERSION: i64 = 22;",
    "../fixtures/operational-store/schema-22.json",
    "fn verify_schema_history(connection: &Connection)",
    "fn failed_version_three_migration_rolls_back_all_alterations()",
    "fn failed_version_two_migration_rolls_back_without_partial_schema()",
    "fn future_schema_and_page_corruption_are_refused()",
    "fn exclusive_writer_and_encrypted_backup_are_verified()",
    "fn encrypted_backup_restores_only_to_a_verified_fresh_candidate()",
    "SeededCrashBoundary::Migration",
)
READER_TEST_MARKERS: Final = (
    "fn version_nineteen_reader_epoch_preserves_tables_records_and_history()",
    "fn version_nineteen_corrupt_history_cannot_advance_reader_epoch()",
    "fn failed_version_twenty_reader_epoch_keeps_version_nineteen_retryable()",
    "fn version_nineteen_backup_refusal_preserves_source_without_creating_candidate()",
    "include!(\"job_ledger_migration_tests.rs\");",
)
JOB_LEDGER_TEST_MARKERS: Final = (
    "fn version_twenty_upgrades_to_job_ledgers_preserving_records_and_history()",
    "fn failed_version_twenty_one_migration_rolls_back_and_stays_retryable()",
    "fn version_twenty_corrupt_history_cannot_add_job_ledgers()",
    "include!(\"run_action_history_migration_tests.rs\");",
)
RUN_HISTORY_TEST_MARKERS: Final = (
    "fn version_twenty_one_upgrades_to_run_action_histories_preserving_records_and_history()",
    "fn failed_version_twenty_two_migration_rolls_back_and_stays_retryable()",
    "fn version_twenty_one_corrupt_history_cannot_add_run_action_histories()",
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
    "03800068d00ea422acc3702e48bb3cae4c736f104680bb57767ce0b8b1fb6db3",
    "1f1e3664bcdd5c740be344f971c35794a5269d01330eb1699157b61bca6489ea",
    "6f27f41680d76642fbaae8efded29934bc2ccb797c75c23d7142986d7414573d",
    "84ed1485d0d2144a00eee86bd5052add809fc18af0c0483348d9fdbe5873ff09",
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
        "generated_on": "2026-09-30",
        "status": "pass-local-migration-compatibility",
        "canonical_store": "operational-store",
        "operational_schema_version": 22,
        "migration_count": 22,
        "commands": [" ".join(command) for command in COMMANDS],
        "required_markers": list(MARKERS),
        "artifacts": [
            artifact("kernel/engine/src/operational_store.rs"),
            artifact("kernel/engine/fixtures/operational-store/schema-18.json"),
            artifact("kernel/engine/fixtures/operational-store/schema-19.json"),
            artifact("kernel/engine/fixtures/operational-store/schema-20.json"),
            artifact("kernel/engine/fixtures/operational-store/schema-21.json"),
            artifact("kernel/engine/fixtures/operational-store/schema-22.json"),
            artifact("kernel/engine/migrations/operational-store/0020-research-draft-readers.sql"),
            artifact("kernel/engine/migrations/operational-store/0021-job-control-ledgers.sql"),
            artifact("kernel/engine/migrations/operational-store/0022-run-action-histories.sql"),
            artifact("kernel/engine/src/job_ledger_store.rs"),
            artifact("kernel/engine/src/run_action_history_store.rs"),
            artifact("kernel/engine/migrations/operational-store/0019-research-budgets.sql"),
            artifact("kernel/engine/src/research_journal.rs"),
            artifact("kernel/engine/src/research_migration_tests.rs"),
            artifact("kernel/engine/src/research_reader_migration_tests.rs"),
            artifact("kernel/engine/src/job_ledger_migration_tests.rs"),
            artifact("kernel/engine/src/run_action_history_migration_tests.rs"),
            artifact("docs/decisions/0087-research-draft-reader-version-barrier.md"),
            artifact("docs/decisions/0118-durable-job-control-ledgers.md"),
            artifact("docs/decisions/0129-durable-run-action-histories.md"),
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


def validate_reader_tests(value: str) -> list[str]:
    return [f"reader test source missing marker: {marker}" for marker in READER_TEST_MARKERS if marker not in value]


def validate_job_ledger_tests(value: str) -> list[str]:
    return [f"job ledger migration test source missing marker: {marker}" for marker in JOB_LEDGER_TEST_MARKERS if marker not in value]


def validate_run_history_tests(value: str) -> list[str]:
    return [f"run action history migration test source missing marker: {marker}" for marker in RUN_HISTORY_TEST_MARKERS if marker not in value]


def validate_fixture(value: Any) -> list[str]:
    failures: list[str] = []
    if not isinstance(value, dict) or value.get("record_type") != "agentmage-operational-store-schema-fixture":
        failures.append("fixture record type is not exact")
        return failures
    if value.get("schema_version") != 22:
        failures.append("fixture schema version is not 22")
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
        failures += validate_reader_tests(READER_TEST_PATH.read_text(encoding="utf-8"))
        failures += validate_job_ledger_tests(JOB_LEDGER_TEST_PATH.read_text(encoding="utf-8"))
        failures += validate_run_history_tests(RUN_HISTORY_TEST_PATH.read_text(encoding="utf-8"))
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
        reader_tests = READER_TEST_PATH.read_text(encoding="utf-8")
        job_ledger_tests = JOB_LEDGER_TEST_PATH.read_text(encoding="utf-8")
        run_history_tests = RUN_HISTORY_TEST_PATH.read_text(encoding="utf-8")
        fixture = json.loads(FIXTURE_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"storage migration compatibility evidence validation failed: {error}", file=sys.stderr)
        return 1
    failures = validate_source(source) + validate_reader_tests(reader_tests) + validate_job_ledger_tests(job_ledger_tests) + validate_run_history_tests(run_history_tests) + validate_fixture(fixture) + validate_raw(raw) + validate_report(report)
    if failures:
        for failure in failures:
            print(f"storage migration compatibility evidence validation failed: {failure}", file=sys.stderr)
        return 1
    print("Sub-task 11.2.3.1 storage migration compatibility validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
