#!/usr/bin/env python3
"""Build and validate bounded Sprint 11 crash/canary result evidence."""

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
REPORT_PATH: Final = ROOT / "artifacts/sprints/sprint-11/story-11.1/crash-canary-results.json"
DOCUMENT_PATH: Final = ROOT / "docs/verification/sprint-11-crash-canary-results.md"
AUTHORITY_PATH: Final = ROOT / "kernel/engine/src/authority_transaction.rs"
STORE_PATH: Final = ROOT / "kernel/engine/src/operational_store.rs"
PERSISTENCE_PATH: Final = ROOT / "kernel/engine/src/persistence.rs"
SOURCE_PATHS: Final = (
    "docs/verification/sprint-11-crash-canary-results.md",
    "kernel/engine/src/authority_transaction.rs",
    "kernel/engine/src/operational_store.rs",
    "kernel/engine/src/persistence.rs",
    "scripts/crash_canary_result_evidence.py",
    "tests/test_crash_canary_result_evidence.py",
)
CRASH_POINT_IDS: Final = ("CP-01", "CP-02", "CP-03", "CP-04", "CP-05", "CP-06")
CANARY_SURFACES: Final = (
    "sqlcipher-main",
    "sqlcipher-wal",
    "sqlcipher-shm",
    "encrypted-backup",
    "derived-json-lines",
    "content-free-receipts",
    "debug-and-stable-errors",
)
DOCUMENT_FRAGMENTS: Final = (
    "**Not sufficient for:** `S-011-ST01`, `S-011-RT01`, or Sprint 11 closure",
    "| `CP-01` | Prepared stored | Failed | Issued | 0 | 0 |",
    "| `CP-06` | Result reconciled | Succeeded | Consumed | 1 | 0 |",
    "and canonical-record bytes of a schema-v3 test store. Direct insertion is",
    "Sub-task `11.1.3.2` still must place distinct synthetic canaries in every field",
    "Sub-task `11.1.3.3` still must run at least 100 seeded crash resumes",
    "Manual fuzzing remains separately deferred and is not executed",
)
AUTHORITY_FRAGMENTS: Final = (
    "fn encrypted_restart_recovery_never_replays_and_publishes_one_receipt() {\n        for fault in [\n            FaultPoint::PreparedStored,",
    "FaultPoint::PreparedStored\n                | FaultPoint::GrantConsumed\n                | FaultPoint::AttemptRecorded => OperationOutcome::Failed,",
    "FaultPoint::ResultReconciled => OperationOutcome::Succeeded,",
    "assert_eq!(runtime.receipts().len(), 1);",
    "assert_eq!(replay_driver.launches, 0);",
)
STORE_FRAGMENTS: Final = (
    "fn synthetic_canary_is_absent_from_encrypted_and_derived_artifacts()",
    "fn assert_artifacts_exclude_canary(",
    ".backup(&backup, &observation(), &mut TestKey([62; 32]))",
    ".export_json_lines(&export, &observation())",
    "assert_artifacts_exclude_canary(&path, &backup, &export, canary);\n        drop(store);",
    'OperationalStore::open(&backup, &observation(), &mut TestKey([62; 32]))',
)
PERSISTENCE_FRAGMENTS: Final = (
    "pub enum EphemeralContentClass {",
    "Some(PersistenceOutcome::DeniedSecret)",
    "fn every_raw_content_class_is_ephemeral_without_value_or_digest_receipt()",
    "fn persisted_secret_denies_without_value_or_value_digest_in_receipt()",
)
COMMAND_SPECS: Final = (
    (("cargo", "test", "-p", "agentmage-kernel-engine", "encrypted_restart_recovery_never_replays_and_publishes_one_receipt", "--locked"), "1 passed; 0 failed"),
    (("cargo", "test", "-p", "agentmage-kernel-engine", "synthetic_canary_is_absent_from_encrypted_and_derived_artifacts", "--locked"), "1 passed; 0 failed"),
    (("cargo", "test", "-p", "agentmage-kernel-engine", "persistence", "--locked"), "12 passed; 0 failed"),
    (("python3", "-m", "unittest", "tests.test_crash_canary_result_evidence"), "Ran 4 tests"),
    (("npm", "run", "docs:lint"), "Summary: 0 issues in 0 files"),
)
CLAIMS: Final = {
    "deterministic_crash_point_count": 6,
    "encrypted_restart_per_point": True,
    "single_terminal_receipt_per_point": True,
    "recovery_or_replay_driver_launches": 0,
    "canary_surface_count": 7,
    "raw_canary_retained_in_evidence": False,
    "encrypted_and_derived_canary_absent": True,
    "production_ingress_every_field_tested": False,
    "temporary_files_logs_model_context_crash_output_tested": False,
    "minimum_100_seed_crash_campaign_complete": False,
    "operating_system_process_kill_used": False,
    "s011_st01_complete": False,
    "s011_rt01_complete": False,
    "manual_fuzzing_executed": False,
    "external_network_used": False,
    "release_support": False,
}
LIMITATIONS: Final = [
    "Crash results cover six deterministic in-process authority checkpoints, not operating-system process kills or every migration, key, backup, restore, expiry, deletion, and transaction boundary.",
    "The canary is directly seeded into one sessions record and does not exercise every field of every production input family.",
    "Temporary files, logs, model context, crash output, and complete sanitization reporting remain Sub-task 11.1.3.2.",
    "The at-least-100-seed crash campaign remains Sub-task 11.1.3.3.",
    "Live Secret Service, cross-platform, packaging, release acceptance, and manually deferred fuzz testing are not claimed.",
]
REVISION = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")


class EvidenceError(ValueError):
    """Raised when crash/canary evidence is incomplete or overstated."""


def validate_sources(document: str, authority: str, store: str, persistence: str) -> list[str]:
    failures = []
    if tuple(re.findall(r"`(CP-\d{2})`", document)) != CRASH_POINT_IDS:
        failures.append("crash-point matrix closure changed")
    for label, value, fragments in (
        ("document", document, DOCUMENT_FRAGMENTS),
        ("authority", authority, AUTHORITY_FRAGMENTS),
        ("store", store, STORE_FRAGMENTS),
        ("persistence", persistence, PERSISTENCE_FRAGMENTS),
    ):
        failures.extend(
            f"crash-canary {label} fragment changed: {index}"
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
    failures = validate_sources(
        values[str(DOCUMENT_PATH.relative_to(ROOT))],
        values[str(AUTHORITY_PATH.relative_to(ROOT))],
        values[str(STORE_PATH.relative_to(ROOT))],
        values[str(PERSISTENCE_PATH.relative_to(ROOT))],
    )
    if failures:
        raise EvidenceError("; ".join(failures))
    return records


def build_report(revision: str) -> dict[str, Any]:
    return {
        "artifact_id": "sprint-11-bounded-crash-canary-results",
        "canary_surfaces": list(CANARY_SURFACES),
        "claims": CLAIMS,
        "crash_point_ids": list(CRASH_POINT_IDS),
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "source_revision": revision,
        "sources": source_records(revision),
        "status": "pass-current-partial-results-open-verification-gates",
        "task_ids": ["11.1.2.4"],
        "verification_commands": [run_checked(arguments, marker) for arguments, marker in COMMAND_SPECS],
    }


def validate_report(report: dict[str, Any]) -> list[str]:
    failures = []
    exact = {
        "artifact_id": "sprint-11-bounded-crash-canary-results",
        "canary_surfaces": list(CANARY_SURFACES),
        "claims": CLAIMS,
        "crash_point_ids": list(CRASH_POINT_IDS),
        "external_network_used": False,
        "limitations": LIMITATIONS,
        "private_user_data_used": False,
        "schema_version": 1,
        "status": "pass-current-partial-results-open-verification-gates",
        "task_ids": ["11.1.2.4"],
        "verification_commands": expected_commands(),
    }
    for key, value in exact.items():
        if report.get(key) != value:
            failures.append(f"crash-canary evidence {key} changed")
    if REVISION.fullmatch(str(report.get("source_revision", ""))) is None:
        failures.append("crash-canary evidence revision is invalid")
    sources = report.get("sources")
    if not isinstance(sources, list) or [item.get("path") for item in sources] != list(SOURCE_PATHS):
        failures.append("crash-canary evidence sources changed")
    elif any(
        not isinstance(item.get("bytes"), int) or item["bytes"] <= 0
        or SHA256.fullmatch(str(item.get("sha256", ""))) is None for item in sources
    ):
        failures.append("crash-canary evidence source records are invalid")
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
    print("bounded crash-canary evidence: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
