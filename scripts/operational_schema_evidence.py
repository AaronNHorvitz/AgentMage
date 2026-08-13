#!/usr/bin/env python3
"""Build and validate Sprint 11 operational-schema evidence."""

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
MIGRATION_PATH: Final = (
    "kernel/engine/migrations/operational-store/0002-domain-schema.sql"
)
REPORT_PATH: Final = (
    ROOT / "artifacts/sprints/sprint-11/story-11.1/operational-schema-v2.json"
)
SOURCE_PATHS: Final = (
    "docs/architecture/durable-authority-store.md",
    MIGRATION_PATH,
    "kernel/engine/src/operational_store.rs",
    "scripts/operational_schema_evidence.py",
    "tests/test_operational_schema_evidence.py",
)
COMMAND_SPECS: Final = (
    (
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "operational_store::tests",
            "--locked",
        ),
        "10 passed; 0 failed",
    ),
    (
        (
            "python3",
            "-m",
            "unittest",
            "tests.test_operational_schema_evidence",
        ),
        "Ran 5 tests",
    ),
    (
        ("npm", "run", "product:lint"),
        "Structural effect mediation boundary validated.",
    ),
    (
        ("npm", "run", "docs:lint"),
        "Summary: 0 issues in 0 files",
    ),
)
V2_TABLES: Final = (
    "actions",
    "decisions",
    "evidence",
    "files",
    "objectives",
    "plans",
    "retention",
    "sessions",
    "tasks",
)
V2_INDEXES: Final = (
    "actions_task_idx",
    "actions_transaction_idx",
    "decisions_task_idx",
    "evidence_task_idx",
    "files_session_idx",
    "objectives_session_idx",
    "plans_objective_idx",
    "retention_disposition_idx",
    "tasks_plan_idx",
)
SCHEMA_PROFILE: Final = {
    "schema_version": 2,
    "history_versions": [1, 2],
    "existing_authority_table_count": 10,
    "added_domain_table_count": 9,
    "total_table_count": 19,
    "added_index_count": 9,
    "domain_tables": list(V2_TABLES),
    "domain_indexes": list(V2_INDEXES),
    "migration_1_bytes_preserved": True,
    "upgrade_path": "encrypted-v1-to-v2-additive-transaction",
}
CLAIMS: Final = {
    "normalized_domain_schema_implemented": True,
    "version_one_upgrade_executed": True,
    "version_two_failure_rollback_executed": True,
    "migration_history_hash_bound": True,
    "foreign_key_and_closed_value_constraints_executed": True,
    "public_domain_write_api_implemented": False,
    "domain_state_in_authority_digest": False,
    "retention_lifecycle_implemented": False,
    "external_network_used": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "Schema v2 adds structural domain tables only; public typed domain records, transactional write APIs, and restart reconstruction remain later Sprint 11 work.",
    "Domain rows are not yet part of the authority-state digest because no production boundary can write them; the existing grant, transaction, receipt, and checkpoint digest remains unchanged.",
    "The retention table constrains policy shape but does not implement expiry, hold transitions, erasure, export, backup policy, or restore reconciliation.",
    "This evidence uses local encrypted SQLCipher fixtures and claims no application-host composition, cross-platform certification, or release support.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
CREATE_TABLE = re.compile(r"(?m)^CREATE TABLE ([a-z_]+) \(")
CREATE_INDEX = re.compile(r"(?m)^CREATE INDEX ([a-z_]+) ON ")


class OperationalSchemaEvidenceError(ValueError):
    """Raised when operational-schema evidence is incomplete or overstated."""


def validate_migration(value: str) -> list[str]:
    failures: list[str] = []
    if tuple(sorted(CREATE_TABLE.findall(value))) != V2_TABLES:
        failures.append("operational schema table closure changed")
    if tuple(sorted(CREATE_INDEX.findall(value))) != V2_INDEXES:
        failures.append("operational schema index closure changed")
    if value.count(" STRICT;") != len(V2_TABLES):
        failures.append("operational schema strict-table closure changed")
    for fragment, count in (
        ("FOREIGN KEY(session_id) REFERENCES sessions(session_id)", 2),
        ("FOREIGN KEY(objective_id) REFERENCES objectives(objective_id)", 1),
        ("FOREIGN KEY(parent_task_id, plan_id) REFERENCES tasks(task_id, plan_id)", 1),
        ("FOREIGN KEY(action_id, task_id) REFERENCES actions(action_id, task_id)", 1),
        (
            "FOREIGN KEY(transaction_id) REFERENCES transaction_identities(transaction_id)",
            1,
        ),
        ("CHECK(legal_hold IN (0, 1))", 1),
    ):
        if value.count(fragment) != count:
            failures.append(f"operational schema relationship changed: {fragment}")
    return failures


def git_revision(candidate: str) -> str:
    completed = subprocess.run(
        ["git", "rev-parse", "--verify", f"{candidate}^{{commit}}"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
        timeout=30,
        check=False,
    )
    revision = completed.stdout.strip()
    if completed.returncode != 0 or REVISION.fullmatch(revision) is None:
        raise OperationalSchemaEvidenceError("source revision is unavailable")
    return revision


def git_bytes(revision: str, path: str) -> bytes:
    completed = subprocess.run(
        ["git", "show", f"{revision}:{path}"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        timeout=60,
        check=False,
    )
    if completed.returncode != 0 or not completed.stdout:
        raise OperationalSchemaEvidenceError("committed source is unavailable")
    return completed.stdout


def validate_committed_migration(revision: str) -> str:
    data = git_bytes(revision, MIGRATION_PATH)
    try:
        value = data.decode("utf-8")
    except UnicodeError as error:
        raise OperationalSchemaEvidenceError("committed migration is invalid") from error
    if errors := validate_migration(value):
        raise OperationalSchemaEvidenceError("; ".join(errors))
    return hashlib.sha256(data).hexdigest()


def source_records(revision: str) -> list[dict[str, Any]]:
    return [
        {
            "path": path,
            "bytes": len(data := git_bytes(revision, path)),
            "sha256": hashlib.sha256(data).hexdigest(),
        }
        for path in SOURCE_PATHS
    ]


def command_record(arguments: tuple[str, ...], marker: str) -> dict[str, Any]:
    return {
        "command_id": hashlib.sha256("\0".join(arguments).encode()).hexdigest(),
        "exit_code": 0,
        "expected_marker_sha256": hashlib.sha256(marker.encode()).hexdigest(),
        "status": "pass",
    }


def expected_commands() -> list[dict[str, Any]]:
    return [command_record(arguments, marker) for arguments, marker in COMMAND_SPECS]


def run_checked(arguments: tuple[str, ...], marker: str) -> dict[str, Any]:
    completed = subprocess.run(
        list(arguments),
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        timeout=300,
        check=False,
        env={**os.environ, "LANG": "C", "LC_ALL": "C"},
    )
    if completed.returncode != 0 or marker not in completed.stdout + completed.stderr:
        raise OperationalSchemaEvidenceError(
            f"verification command failed: {Path(arguments[0]).name}"
        )
    return command_record(arguments, marker)


def build_report(revision: str) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "artifact_id": "encrypted-operational-schema-v2",
        "source_revision": revision,
        "task_ids": ["11.1.1.1"],
        "status": "pass-normalized-schema-and-migration-boundary",
        "migration_sha256": validate_committed_migration(revision),
        "schema_profile": SCHEMA_PROFILE,
        "verification_commands": [
            run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS
        ],
        "claims": CLAIMS,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "external_network_used": False,
        "sources": source_records(revision),
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    exact = {
        "schema_version": 1,
        "artifact_id": "encrypted-operational-schema-v2",
        "task_ids": ["11.1.1.1"],
        "status": "pass-normalized-schema-and-migration-boundary",
        "schema_profile": SCHEMA_PROFILE,
        "verification_commands": expected_commands(),
        "claims": CLAIMS,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "external_network_used": False,
    }
    for key, value in exact.items():
        if report.get(key) != value:
            failures.append(f"operational schema {key} changed")
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("operational schema source revision is invalid")
    if SHA256.fullmatch(str(report.get("migration_sha256", ""))) is None:
        failures.append("operational schema migration identity is invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        failures.append("operational schema source evidence changed")
    elif any(
        not isinstance(item.get("bytes"), int)
        or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None
        for item in sources
    ):
        failures.append("operational schema source records are invalid")
    else:
        migration = sources[SOURCE_PATHS.index(MIGRATION_PATH)]
        if migration["sha256"] != report.get("migration_sha256"):
            failures.append("operational schema migration identity changed")
    return failures


def validate_committed_sources(report: dict[str, Any]) -> None:
    revision = str(report["source_revision"])
    for item in report["sources"]:
        if hashlib.sha256(git_bytes(revision, item["path"])).hexdigest() != item["sha256"]:
            raise OperationalSchemaEvidenceError("committed source binding changed")
    if validate_committed_migration(revision) != report["migration_sha256"]:
        raise OperationalSchemaEvidenceError("committed migration binding changed")


def read_report(path: Path) -> dict[str, Any]:
    try:
        report = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise OperationalSchemaEvidenceError("evidence report is unavailable") from error
    if not isinstance(report, dict):
        raise OperationalSchemaEvidenceError("evidence report is invalid")
    return report


def write_atomic(path: Path, report: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
            json.dump(report, stream, indent=2, sort_keys=True)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--source-revision", default="HEAD")
    arguments = parser.parse_args()
    if arguments.write:
        report = build_report(git_revision(arguments.source_revision))
        if errors := validate_report(report):
            raise OperationalSchemaEvidenceError("; ".join(errors))
        write_atomic(REPORT_PATH, report)
    report = read_report(REPORT_PATH)
    if errors := validate_report(report):
        raise OperationalSchemaEvidenceError("; ".join(errors))
    validate_committed_sources(report)
    print("Encrypted operational schema v2 evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
