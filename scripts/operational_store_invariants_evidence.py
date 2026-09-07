#!/usr/bin/env python3
"""Build and validate encrypted operational-store invariant evidence."""

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
REPORT_PATH: Final = (
    ROOT / "artifacts/sprints/sprint-11/story-11.1/store-invariants.json"
)
SOURCE_PATHS: Final = (
    "docs/architecture/durable-authority-store.md",
    "kernel/engine/migrations/operational-store/0002-domain-schema.sql",
    "kernel/engine/src/operational_store.rs",
    "scripts/operational_store_invariants_evidence.py",
    "tests/test_operational_store_invariants_evidence.py",
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
        "23 passed; 0 failed",
    ),
    (
        (
            "cargo",
            "test",
            "-p",
            "agentmage-kernel-engine",
            "--locked",
        ),
        "273 passed; 0 failed",
    ),
    (
        (
            "python3",
            "-m",
            "unittest",
            "tests.test_operational_store_invariants_evidence",
        ),
        "Ran 5 tests",
    ),
    (
        ("npm", "run", "product:lint"),
        "Structural effect mediation boundary validated.",
    ),
)
INVARIANT_PROFILE: Final = {
    "writer_lock_order": [
        "open-keyed-sqlcipher",
        "claim-exclusive-writer",
        "verify-runtime-configuration",
        "migrate",
        "load-authority",
    ],
    "required_pragmas": {
        "busy_timeout": 0,
        "foreign_keys": 1,
        "journal_mode": "wal",
        "locking_mode": "exclusive",
        "secure_delete": 1,
        "synchronous": 2,
        "temp_store": 2,
        "trusted_schema": 0,
        "wal_autocheckpoint": 1,
    },
    "publication_transaction": "immediate",
    "publication_generation": "compare-and-swap",
    "checkpoint_commit": "same-transaction-as-state",
    "migration_transaction": "one-transaction-per-version",
    "migration_history": "sha256-bound",
}
CLAIMS: Final = {
    "exclusive_writer_precedes_migration": True,
    "required_runtime_configuration_verified": True,
    "foreign_keys_enforced": True,
    "wal_and_full_synchronization_enforced": True,
    "atomic_generation_and_checkpoint_rollback_executed": True,
    "migration_rollback_executed": True,
    "second_writer_denied": True,
    "cross_process_stress_campaign_complete": False,
    "public_domain_write_api_implemented": False,
    "external_network_used": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "The second-writer fixture uses two independent SQLCipher connections in one process; multi-process contention and crash campaigns remain later verification tasks.",
    "The publication rollback fixture injects failure at checkpoint insertion after the metadata update and verifies transaction rollback; exhaustive crash points remain Sub-task 11.1.3.3.",
    "This invariant gate does not implement typed domain writes, classification, retention transitions, export, erasure, or application-host composition.",
    "No cross-platform certification or release acceptance is claimed.",
]
REQUIRED_SOURCE_FRAGMENTS: Final = (
    "claim_exclusive_writer(&connection)?;\n    verify_runtime_configuration(&connection)?;\n    migrate(&connection)?;",
    "let transaction = connection\n        .transaction_with_behavior(TransactionBehavior::Immediate)",
    "INSERT INTO checkpoints(\n                 generation, state_sha256, session_checkpoint_sha256",
    "PRAGMA foreign_keys = ON;",
    "PRAGMA synchronous = FULL;",
)
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class StoreInvariantEvidenceError(ValueError):
    """Raised when store-invariant evidence is incomplete or overstated."""


def validate_source(value: str) -> list[str]:
    return [
        f"store invariant source fragment changed: {index}"
        for index, fragment in enumerate(REQUIRED_SOURCE_FRAGMENTS, start=1)
        if value.count(fragment) != 1
    ]


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
        raise StoreInvariantEvidenceError("source revision is unavailable")
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
        raise StoreInvariantEvidenceError("committed source is unavailable")
    return completed.stdout


def validate_committed_store(revision: str) -> str:
    data = git_bytes(revision, "kernel/engine/src/operational_store.rs")
    try:
        value = data.decode("utf-8")
    except UnicodeError as error:
        raise StoreInvariantEvidenceError("committed store source is invalid") from error
    if errors := validate_source(value):
        raise StoreInvariantEvidenceError("; ".join(errors))
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
        raise StoreInvariantEvidenceError(
            f"verification command failed: {Path(arguments[0]).name}"
        )
    return command_record(arguments, marker)


def build_report(revision: str) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "artifact_id": "encrypted-operational-store-invariants",
        "source_revision": revision,
        "task_ids": ["11.1.1.2"],
        "status": "pass-writer-transaction-wal-checkpoint-migration-invariants",
        "store_source_sha256": validate_committed_store(revision),
        "invariant_profile": INVARIANT_PROFILE,
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
        "artifact_id": "encrypted-operational-store-invariants",
        "task_ids": ["11.1.1.2"],
        "status": "pass-writer-transaction-wal-checkpoint-migration-invariants",
        "invariant_profile": INVARIANT_PROFILE,
        "verification_commands": expected_commands(),
        "claims": CLAIMS,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "external_network_used": False,
    }
    for key, value in exact.items():
        if report.get(key) != value:
            failures.append(f"store invariant {key} changed")
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("store invariant source revision is invalid")
    if SHA256.fullmatch(str(report.get("store_source_sha256", ""))) is None:
        failures.append("store invariant source identity is invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(
        SOURCE_PATHS
    ):
        failures.append("store invariant source evidence changed")
    elif any(
        not isinstance(item.get("bytes"), int)
        or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None
        for item in sources
    ):
        failures.append("store invariant source records are invalid")
    else:
        store = sources[SOURCE_PATHS.index("kernel/engine/src/operational_store.rs")]
        if store["sha256"] != report.get("store_source_sha256"):
            failures.append("store invariant source identity changed")
    return failures


def validate_committed_sources(report: dict[str, Any]) -> None:
    revision = str(report["source_revision"])
    for item in report["sources"]:
        if hashlib.sha256(git_bytes(revision, item["path"])).hexdigest() != item["sha256"]:
            raise StoreInvariantEvidenceError("committed source binding changed")
    if validate_committed_store(revision) != report["store_source_sha256"]:
        raise StoreInvariantEvidenceError("committed store binding changed")


def read_report(path: Path) -> dict[str, Any]:
    try:
        report = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise StoreInvariantEvidenceError("evidence report is unavailable") from error
    if not isinstance(report, dict):
        raise StoreInvariantEvidenceError("evidence report is invalid")
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
            raise StoreInvariantEvidenceError("; ".join(errors))
        write_atomic(REPORT_PATH, report)
    report = read_report(REPORT_PATH)
    if errors := validate_report(report):
        raise StoreInvariantEvidenceError("; ".join(errors))
    validate_committed_sources(report)
    print("Encrypted operational-store invariant evidence validated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
