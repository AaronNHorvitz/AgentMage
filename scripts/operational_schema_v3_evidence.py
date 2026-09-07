#!/usr/bin/env python3
"""Build and validate current operational-schema v3 evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import tempfile
from pathlib import Path
from typing import Any, Final

ROOT: Final = Path(__file__).resolve().parents[1]
MIGRATION_PATHS: Final = (
    "kernel/engine/migrations/operational-store/0002-domain-schema.sql",
    "kernel/engine/migrations/operational-store/0003-retention-lifecycle.sql",
)
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-11/story-11.1/operational-schema-v3.json"
SOURCE_PATHS: Final = (
    "docs/architecture/durable-authority-store.md",
    *MIGRATION_PATHS,
    "kernel/engine/src/operational_store.rs",
    "scripts/operational_schema_v3_evidence.py",
    "tests/test_operational_schema_v3_evidence.py",
)
COMMAND_SPECS: Final = (
    (("cargo", "test", "-p", "agentmage-kernel-engine", "operational_store", "--locked"), "20 passed; 0 failed"),
    (("python3", "-m", "unittest", "tests.test_operational_schema_v3_evidence"), "Ran 5 tests"),
    (("npm", "run", "product:lint"), "Structural effect mediation boundary validated."),
    (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
)
V2_TABLES: Final = (
    "actions", "decisions", "evidence", "files", "objectives", "plans",
    "retention", "sessions", "tasks",
)
V2_INDEXES: Final = (
    "actions_task_idx", "actions_transaction_idx", "decisions_task_idx",
    "evidence_task_idx", "files_session_idx", "objectives_session_idx",
    "plans_objective_idx", "retention_disposition_idx", "tasks_plan_idx",
)
SCHEMA_PROFILE: Final = {
    "schema_version": 3,
    "history_versions": [1, 2, 3],
    "existing_authority_table_count": 10,
    "v2_domain_table_count": 9,
    "v3_lifecycle_table_count": 1,
    "total_table_count": 20,
    "v2_index_count": 9,
    "v3_index_count": 1,
    "total_added_index_count": 10,
    "v2_domain_tables": list(V2_TABLES),
    "v3_lifecycle_tables": ["retention_events"],
    "v2_indexes": list(V2_INDEXES),
    "v3_indexes": ["retention_hold_expiry_idx"],
    "migration_1_bytes_preserved": True,
    "migration_2_bytes_preserved": True,
    "upgrade_paths": ["encrypted-v1-through-v3", "encrypted-v2-to-v3"],
}
CLAIMS: Final = {
    "all_migrations_hash_bound": True,
    "encrypted_v1_through_v3_executed": True,
    "encrypted_v2_to_v3_executed": True,
    "existing_v2_retention_preserved": True,
    "existing_v2_retention_event_initialized": True,
    "version_two_failure_rollback_executed": True,
    "version_three_failure_rollback_executed": True,
    "partial_alterations_rejected": True,
    "foreign_key_and_closed_value_constraints_executed": True,
    "historical_v2_artifact_rewritten": False,
    "external_network_used": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "This artifact proves the current encrypted schema and migration chain; typed production writes for every normalized domain table and full domain restart reconstruction remain later work.",
    "The historical operational-schema-v2 artifact remains immutable and describes only its original source revision.",
    "Cross-process crash injection, live platform key services, cross-platform execution, packaging, and release acceptance remain later gates.",
]
V3_FRAGMENTS: Final = (
    "ALTER TABLE retention ADD COLUMN hold_kind",
    "ALTER TABLE retention ADD COLUMN prior_disposition",
    "ALTER TABLE retention ADD COLUMN revision",
    "ALTER TABLE retention ADD COLUMN updated_at_epoch_ms",
    "ALTER TABLE retention ADD COLUMN erased_at_epoch_ms",
    "CREATE TABLE retention_events (",
    "CREATE INDEX retention_hold_expiry_idx",
)
SOURCE_FRAGMENTS: Final = (
    "fn version_one_upgrades_through_eighteen_with_exact_history()",
    "fn version_two_retention_rows_upgrade_to_three_with_initial_event()",
    "fn failed_version_two_migration_rolls_back_without_partial_schema()",
    "fn failed_version_three_migration_rolls_back_all_alterations()",
)
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
CREATE_TABLE = re.compile(r"(?m)^CREATE TABLE ([a-z_]+) \(")
CREATE_INDEX = re.compile(r"(?m)^CREATE INDEX ([a-z_]+)(?: ON|\n)")


class EvidenceError(ValueError):
    """Raised when schema v3 evidence is incomplete or overstated."""


def validate_migrations(v2: str, v3: str) -> list[str]:
    failures = []
    if tuple(sorted(CREATE_TABLE.findall(v2))) != V2_TABLES:
        failures.append("schema v2 table closure changed")
    if tuple(sorted(CREATE_INDEX.findall(v2))) != V2_INDEXES:
        failures.append("schema v2 index closure changed")
    if CREATE_TABLE.findall(v3) != ["retention_events"]:
        failures.append("schema v3 table closure changed")
    if CREATE_INDEX.findall(v3) != ["retention_hold_expiry_idx"]:
        failures.append("schema v3 index closure changed")
    failures.extend(
        f"schema v3 fragment changed: {index}"
        for index, fragment in enumerate(V3_FRAGMENTS, 1)
        if v3.count(fragment) != 1
    )
    if v3.count("ALTER TABLE retention ADD COLUMN") != 5:
        failures.append("schema v3 alteration closure changed")
    return failures


def validate_source(value: str) -> list[str]:
    return [
        f"schema v3 execution fragment changed: {index}"
        for index, fragment in enumerate(SOURCE_FRAGMENTS, 1)
        if value.count(fragment) != 1
    ]


def git_revision(candidate: str) -> str:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"], cwd=ROOT,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, timeout=30, check=False,
    )
    revision = result.stdout.strip()
    if result.returncode or REVISION.fullmatch(revision) is None:
        raise EvidenceError("source revision is unavailable")
    return revision


def git_bytes(revision: str, path: str) -> bytes:
    result = subprocess.run(
        ["git", "show", f"{revision}:{path}"], cwd=ROOT, stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL, timeout=60, check=False,
    )
    if result.returncode or not result.stdout:
        raise EvidenceError("committed source is unavailable")
    return result.stdout


def command_record(arguments: tuple[str, ...], marker: str) -> dict[str, Any]:
    return {
        "command_id": hashlib.sha256("\0".join(arguments).encode()).hexdigest(),
        "expected_marker_sha256": hashlib.sha256(marker.encode()).hexdigest(),
        "exit_code": 0,
        "status": "pass",
    }


def expected_commands() -> list[dict[str, Any]]:
    return [command_record(arguments, marker) for arguments, marker in COMMAND_SPECS]


def run_checked(arguments: tuple[str, ...], marker: str) -> dict[str, Any]:
    result = subprocess.run(
        list(arguments), cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
        timeout=300, check=False, env={**os.environ, "LANG": "C", "LC_ALL": "C"},
    )
    if result.returncode or marker not in result.stdout + result.stderr:
        raise EvidenceError(f"verification command failed: {Path(arguments[0]).name}")
    return command_record(arguments, marker)


def source_records(revision: str) -> list[dict[str, Any]]:
    records = []
    values = {path: git_bytes(revision, path) for path in SOURCE_PATHS}
    if failures := validate_migrations(*(values[path].decode() for path in MIGRATION_PATHS)):
        raise EvidenceError("; ".join(failures))
    if failures := validate_source(values["kernel/engine/src/operational_store.rs"].decode()):
        raise EvidenceError("; ".join(failures))
    for path, data in values.items():
        records.append({"bytes": len(data), "path": path, "sha256": hashlib.sha256(data).hexdigest()})
    return records


def migration_identities(revision: str) -> dict[str, str]:
    return {Path(path).name: hashlib.sha256(git_bytes(revision, path)).hexdigest() for path in MIGRATION_PATHS}


def build_report(revision: str) -> dict[str, Any]:
    return {
        "artifact_id": "encrypted-operational-schema-v3",
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "migration_sha256": migration_identities(revision),
        "private_user_data_used": False,
        "schema_profile": SCHEMA_PROFILE,
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-versioned-schema-and-migration-chain",
        "task_ids": ["11.1.2.1"],
        "verification_commands": [run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    failures = []
    exact = {
        "artifact_id": "encrypted-operational-schema-v3",
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_profile": SCHEMA_PROFILE,
        "schema_version": 1,
        "status": "pass-versioned-schema-and-migration-chain",
        "task_ids": ["11.1.2.1"],
        "verification_commands": expected_commands(),
    }
    for key, value in exact.items():
        if report.get(key) != value:
            failures.append(f"schema v3 evidence {key} changed")
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("schema v3 evidence revision is invalid")
    identities = report.get("migration_sha256")
    if not isinstance(identities, dict) or set(identities) != {Path(path).name for path in MIGRATION_PATHS} or any(
        SHA256.fullmatch(str(value)) is None for value in identities.values()
    ):
        failures.append("schema v3 migration identities changed")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(SOURCE_PATHS):
        failures.append("schema v3 sources changed")
    elif any(
        not isinstance(item.get("bytes"), int) or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None for item in sources
    ):
        failures.append("schema v3 source records are invalid")
    return failures


def validate_committed(report: dict[str, Any]) -> None:
    revision = report["source_revision"]
    for item in report["sources"]:
        if hashlib.sha256(git_bytes(revision, item["path"])).hexdigest() != item["sha256"]:
            raise EvidenceError("committed source binding changed")
    if migration_identities(revision) != report["migration_sha256"]:
        raise EvidenceError("committed migration binding changed")


def write_atomic(report: dict[str, Any]) -> None:
    REPORT_PATH.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=f".{REPORT_PATH.name}.", dir=REPORT_PATH.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
            json.dump(report, stream, indent=2, sort_keys=True)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, REPORT_PATH)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    revision = git_revision(arguments.source_revision)
    if arguments.write:
        report = build_report(revision)
        write_atomic(report)
    else:
        report = json.loads(REPORT_PATH.read_text())
    failures = validate_report(report)
    if failures:
        raise EvidenceError("; ".join(failures))
    validate_committed(report)
    print("operational schema v3 evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
