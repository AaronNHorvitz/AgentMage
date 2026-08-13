#!/usr/bin/env python3
"""Build and validate encrypted-store lifecycle evidence."""

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
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-11/story-11.1/store-lifecycle.json"
SOURCE_PATHS: Final = (
    "kernel/engine/migrations/operational-store/0003-retention-lifecycle.sql",
    "kernel/engine/src/operational_store.rs",
    "platforms/linux/src/secret_service.rs",
    "docs/architecture/durable-authority-store.md",
    "scripts/store_lifecycle_evidence.py",
    "tests/test_store_lifecycle_evidence.py",
)
COMMAND_SPECS: Final = (
    (("cargo", "test", "-p", "agentmage-kernel-engine", "operational_store", "--locked"), "16 passed; 0 failed"),
    (("cargo", "test", "-p", "agentmage-platform-linux", "secret_service", "--locked"), "5 passed; 0 failed; 3 ignored"),
    (("cargo", "clippy", "--workspace", "--all-targets", "--locked", "--", "-D", "warnings"), "Finished `dev` profile"),
    (("python3", "-m", "unittest", "tests.test_store_lifecycle_evidence"), "Ran 4 tests"),
)
CLAIMS: Final = {
    "schema_v3_additive": True,
    "user_and_legal_holds_distinct": True,
    "stale_transition_rejected_atomically": True,
    "held_records_not_expired": True,
    "retention_events_hash_chained_to_state": True,
    "backup_separately_keyed_and_fully_verified": True,
    "restore_writes_only_fresh_candidate": True,
    "wrong_key_or_corruption_leaves_no_candidate": True,
    "occupied_destination_preserved": True,
    "whole_store_key_destroyed_and_absence_verified": True,
    "physical_overwrite_claim": False,
    "plaintext_or_sql_export_implemented": False,
    "json_lines_export_implemented": False,
    "external_network_used": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "Cryptographic erasure is complete-store key destruction; it is not per-record erasure inside the shared-key database and does not erase separately keyed backups.",
    "No physical-overwrite guarantee is made for SSD, copy-on-write, snapshot, or remapped storage.",
    "Restore creates and verifies a fresh candidate but does not perform Sprint 161 atomic live-store swap, rollback-point, or clean-device continuity orchestration.",
    "JSON Lines derivation, export-only authority denial, and deletion/regeneration behavior remain assigned to Sub-task 11.1.1.7.",
    "Live Secret Service key destruction is environment-dependent; macOS, Windows, packaging, crash campaign, canary campaign, and release evidence remain later work.",
]
FRAGMENTS: Final = {
    "kernel/engine/migrations/operational-store/0003-retention-lifecycle.sql": (
        "CHECK(hold_kind IN ('none', 'user', 'legal'))",
        "CREATE TABLE retention_events (",
        "previous_event_sha256 TEXT NOT NULL",
        "state_sha256 TEXT NOT NULL",
    ),
    "kernel/engine/src/operational_store.rs": (
        "pub fn apply_retention_hold(",
        "pub fn release_retention_hold(",
        "pub fn expire_due(",
        "pub fn restore_to_fresh_candidate<",
        "pub fn cryptographic_erase<L: OperationalStoreKeyLifecycle>(",
        "fn verify_retention_lifecycle(",
        "fn encrypted_backup_restores_only_to_a_verified_fresh_candidate()",
        "fn whole_store_cryptographic_erasure_consumes_key_scope_without_overwrite_claim()",
    ),
    "platforms/linux/src/secret_service.rs": (
        "impl OperationalStoreKeyLifecycle for LinuxOperationalStoreKeyProvider",
        "fn destroy_key_and_verify_absent(",
        "Err(error) if error.kind() == LinuxSecretServiceErrorKind::NotFound => Ok(()),",
    ),
}
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class EvidenceError(ValueError):
    """Raised when lifecycle evidence is incomplete or overstated."""


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


def validate_source(path: str, value: str) -> list[str]:
    return [
        f"lifecycle source fragment changed: {path}:{index}"
        for index, fragment in enumerate(FRAGMENTS.get(path, ()), 1)
        if value.count(fragment) != 1
    ]


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
    for path in SOURCE_PATHS:
        data = git_bytes(revision, path)
        if failures := validate_source(path, data.decode()):
            raise EvidenceError("; ".join(failures))
        records.append({"bytes": len(data), "path": path, "sha256": hashlib.sha256(data).hexdigest()})
    return records


def build_report(revision: str) -> dict[str, Any]:
    return {
        "artifact_id": "encrypted-store-lifecycle",
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-bounded-store-lifecycle",
        "task_ids": ["11.1.1.6"],
        "verification_commands": [run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    failures = []
    exact = {
        "artifact_id": "encrypted-store-lifecycle",
        "claims": CLAIMS,
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "status": "pass-bounded-store-lifecycle",
        "task_ids": ["11.1.1.6"],
        "verification_commands": expected_commands(),
    }
    for key, value in exact.items():
        if report.get(key) != value:
            failures.append(f"lifecycle evidence {key} changed")
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("lifecycle evidence revision is invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(SOURCE_PATHS):
        failures.append("lifecycle evidence sources changed")
    elif any(
        not isinstance(item.get("bytes"), int) or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None for item in sources
    ):
        failures.append("lifecycle evidence source records are invalid")
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
    print("store lifecycle evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
