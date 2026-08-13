#!/usr/bin/env python3
"""Build and validate classification and retention decision-table evidence."""

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
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-11/story-11.1/classification-retention-table.json"
TABLE_PATH: Final = ROOT / "docs/architecture/data-classification-retention-table.md"
PERSISTENCE_PATH: Final = ROOT / "kernel/engine/src/persistence.rs"
STORE_PATH: Final = ROOT / "kernel/engine/src/operational_store.rs"
SOURCE_PATHS: Final = (
    "docs/architecture/data-classification-retention-table.md",
    "kernel/engine/src/persistence.rs",
    "kernel/engine/src/operational_store.rs",
    "scripts/classification_retention_table.py",
    "tests/test_classification_retention_table.py",
)
EXPECTED_IDS: Final = (
    "DC-01", "DC-02", "DC-03", "DC-04", "DC-05", "DC-06", "DC-07", "DC-08",
    "RT-01", "RT-02", "RT-03", "RT-04", "RT-05",
    "LC-01", "LC-02", "LC-03", "LC-04", "LC-05", "LC-06",
    "DE-01", "BK-01", "RS-01", "ER-01", "ER-02",
)
TABLE_FRAGMENTS: Final = (
    "Restricted persistence remains unavailable",
    "Application-host wiring remains open",
    "Never read as startup authority",
    "Not per-record; separately keyed backups require their own erasure",
    "No physical-overwrite claim",
)
PERSISTENCE_FRAGMENTS: Final = (
    "pub enum PersistenceSensitivity {",
    "pub enum PersistenceRetentionIntent {",
    "pub enum PersistenceFieldHandling {",
    "pub enum EphemeralContentClass {",
    "Some(PersistenceOutcome::DeniedRestricted)",
    "let encryption = if prepared.is_some() {",
)
STORE_FRAGMENTS: Final = (
    "pub enum RetentionHoldKind {",
    "pub fn apply_retention_hold(",
    "pub fn release_retention_hold(",
    "pub fn expire_due(",
    "pub fn export_json_lines(",
    "pub fn restore_to_fresh_candidate<",
    "pub fn cryptographic_erase<L: OperationalStoreKeyLifecycle>(",
)
COMMAND_SPECS: Final = (
    (("python3", "-m", "unittest", "tests.test_classification_retention_table"), "Ran 4 tests"),
    (("cargo", "test", "-p", "agentmage-kernel-engine", "persistence", "--locked"), "test result: ok."),
    (("cargo", "test", "-p", "agentmage-kernel-engine", "operational_store", "--locked"), "test result: ok."),
    (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
)
CLAIMS: Final = {
    "classification_rows_closed": 8,
    "retention_assignment_rows_closed": 5,
    "lifecycle_transition_rows_closed": 6,
    "export_backup_restore_erasure_rows_closed": 5,
    "kernel_and_table_fragments_bound": True,
    "current_and_target_behavior_distinguished": True,
    "limitations_preserved": True,
    "external_network_used": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "The table describes implemented kernel boundaries but does not make the application host, adapters, or normalized domain writes complete.",
    "Restricted persistence and application wiring of the PRD retention defaults remain unavailable.",
    "Per-record cryptographic erasure is not claimed, separately keyed backups require separate erasure, and physical overwrite is not claimed.",
    "The JSON Lines derivative is a content-free audit view and is neither startup authority nor a full portability archive.",
    "Secret-canary, crash-point, live key-service, cross-platform, packaging, and release campaigns remain later work.",
]
ID = re.compile(r"`([A-Z]{2}-\d{2})`")
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class EvidenceError(ValueError):
    """Raised when decision-table evidence is incomplete or overstated."""


def validate(table: str, persistence: str, store: str) -> list[str]:
    failures = []
    if tuple(ID.findall(table)) != EXPECTED_IDS:
        failures.append("classification-retention row closure changed")
    for label, value, fragments in (
        ("table", table, TABLE_FRAGMENTS),
        ("persistence", persistence, PERSISTENCE_FRAGMENTS),
        ("store", store, STORE_FRAGMENTS),
    ):
        failures.extend(
            f"classification-retention {label} fragment changed: {index}"
            for index, fragment in enumerate(fragments, 1)
            if value.count(fragment) != 1
        )
    return failures


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
    values: dict[str, str] = {}
    records = []
    for path in SOURCE_PATHS:
        data = git_bytes(revision, path)
        values[path] = data.decode()
        records.append({"bytes": len(data), "path": path, "sha256": hashlib.sha256(data).hexdigest()})
    failures = validate(
        values[str(TABLE_PATH.relative_to(ROOT))],
        values[str(PERSISTENCE_PATH.relative_to(ROOT))],
        values[str(STORE_PATH.relative_to(ROOT))],
    )
    if failures:
        raise EvidenceError("; ".join(failures))
    return records


def build_report(revision: str) -> dict[str, Any]:
    return {
        "artifact_id": "classification-retention-decision-table",
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "row_count": len(EXPECTED_IDS),
        "row_ids": list(EXPECTED_IDS),
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-current-kernel-decision-table",
        "task_ids": ["11.1.2.2"],
        "verification_commands": [run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    failures = []
    exact = {
        "artifact_id": "classification-retention-decision-table",
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "row_count": len(EXPECTED_IDS),
        "row_ids": list(EXPECTED_IDS),
        "schema_version": 1,
        "status": "pass-current-kernel-decision-table",
        "task_ids": ["11.1.2.2"],
        "verification_commands": expected_commands(),
    }
    for key, value in exact.items():
        if report.get(key) != value:
            failures.append(f"classification-retention evidence {key} changed")
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("classification-retention evidence revision is invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(SOURCE_PATHS):
        failures.append("classification-retention evidence sources changed")
    elif any(
        not isinstance(item.get("bytes"), int) or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None for item in sources
    ):
        failures.append("classification-retention evidence source records are invalid")
    return failures


def validate_committed(report: dict[str, Any]) -> None:
    for item in report["sources"]:
        if hashlib.sha256(git_bytes(report["source_revision"], item["path"])).hexdigest() != item["sha256"]:
            raise EvidenceError("committed source binding changed")


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
    print("classification-retention evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
